/*
Copyright 2018 Google LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/
use ropey::Rope;
use serde_json as sj;
use std::io;
use std::io::{BufRead, Read};
use std::rc::Rc;
use time;
use unicode_segmentation::UnicodeSegmentation;

use crate::rich_content::HighlightSettings;
use crate::rich_content::RichContent;
use crate::rich_content::RichLine;
use ropey::RopeSlice;

const DEFAULT_BLANK: char = ' ';

/// Represents a order to edit a content. Offsets are calculated in CHARS, not bytes.
/// offset is the first character of selection, inclusive.
//TODO(njskalski) secure against overlapping cursors!
#[derive(Debug, Serialize, Deserialize)]
pub enum EditEvent {
    Insert { offset: usize, content: String },
    Change { offset: usize, length: usize, content: String },
}

#[derive(Debug)]
struct RopeBasedContent {
    lines: Rope,
    timestamp: time::Tm,
}

impl RopeBasedContent {
    pub fn new(contents: Option<Vec<u8>>) -> Self {
        let rope: Rope = match contents {
            Some(contents) => Rope::from_reader(&contents[..]).unwrap(),
            None => Rope::new(),
        };

        RopeBasedContent { lines: rope, timestamp: time::now() }
    }

    pub fn save<T: io::Write>(&self, writer: T) -> io::Result<()> {
        self.lines.write_to(writer)
    }
}

pub struct RopeBasedContentProvider {
    history: Vec<RopeBasedContent>,
    current: usize,
    // Contract: we do not version rich content. It doesn't make sense: redrawing screen
    // has a similar complexity to syntax highlighting, provided it's implemented properly.
    rich_content: Option<RichContent>,
    highlight_settings_op: Option<Rc<HighlightSettings>>,
}

// Applies events to RopeBasedContent producing new one, and returning *number of lines common* to
// both new and original contents.
// Now events are applied one after another in order they were issued.
//TODO in some combinations offsets should be recomputed. But I expect no such combinations appear.
// I should however check it just in case.
fn apply_events(c: &RopeBasedContent, events: &Vec<EditEvent>) -> (RopeBasedContent, usize) {
    let mut new_lines: Rope = c.lines.clone();

    // Offset is in CHARS, and since it's common, it's valid in both new and old contents.
    let mut first_change_pos = new_lines.len_chars();

    for event in events {
        match event {
            &EditEvent::Insert { ref offset, ref content } => {
                first_change_pos = std::cmp::min(first_change_pos, *offset);
                new_lines.insert(*offset, content);
            }
            &EditEvent::Change { ref offset, ref length, ref content } => {
                first_change_pos = std::cmp::min(first_change_pos, *offset);
                new_lines.remove(*offset..(*offset + *length));
                new_lines.insert(*offset, content);
            }
            _ => debug!("event {:?} not supported yet", event),
        }
    }

    // If first_change_pos is 0 (literally first character of file), obviously there are no
    // common lines betwen old and new version.
    // In other case (first_change_pos > 0), we ask of line_of_first_change. If it's not the first
    // one, we can save all lines before it.
    let num_common_lines = if first_change_pos == 0 {
        0
    } else {
        let line_of_first_change = c.lines.char_to_line(first_change_pos);
        if line_of_first_change > 0 {
            line_of_first_change - 1
        } else {
            0
        }
    };

    (RopeBasedContent { lines: new_lines, timestamp: time::now() }, num_common_lines)
}

impl RopeBasedContentProvider {
    pub fn new(
        contents: Option<Vec<u8>>,
        highlight_settings_op: Option<Rc<HighlightSettings>>,
    ) -> Self {
        RopeBasedContentProvider {
            history: vec![RopeBasedContent::new(contents)],
            current: 0,
            rich_content: None,
            highlight_settings_op: highlight_settings_op,
        }
    }

    /// Returns whether the content is enabled or not.
    // TODO(njskalski): break down for enabled and available.
    pub fn set_rich_content_enabled(&mut self, enabled: bool) -> bool {
        if !enabled {
            self.rich_content = None;
            false
        } else {
            // This costs O(1), but if content provider changes, it needs
            // update.
            self.rich_content = self
                .highlight_settings_op
                .as_ref()
                .map(|s| RichContent::new(s.clone(), self.get_lines().clone()));
            self.rich_content.is_some()
        }
    }

    pub fn is_rich_content_enabled(&self) -> bool {
        self.rich_content.is_some()
    }

    pub fn get_lines(&self) -> &Rope {
        &self.history[self.current].lines
    }

    pub fn get_line(&self, line_no: usize) -> RopeSlice {
        self.history[self.current].lines.line(line_no)
    }

    pub fn len_lines(&self) -> usize {
        self.history[self.current].lines.len_lines()
    }

    pub fn get_rich_line(&self, line_no: usize) -> Option<Rc<RichLine>> {
        self.rich_content.as_ref().and_then(|rich_content| rich_content.get_line(line_no))
    }

    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current < self.history.len() - 1
    }

    pub fn submit_events(&mut self, events: Vec<EditEvent>) {
        debug!("got events {:?}", events);
        let (new_content, num_common_lines) = apply_events(&self.history[self.current], &events);
        let rope = new_content.lines.clone(); // O(1)

        self.history.truncate(self.current + 1); //droping redo's
        self.history.push(new_content);
        self.current += 1;

        // Dropping outdated lines of RichContent. They will be regenerated on-demand.
        self.rich_content.as_mut().map(|rich_content| {
            rich_content.drop_lines(num_common_lines);
            rich_content.update_raw_content(rope);
        });
    }
}
