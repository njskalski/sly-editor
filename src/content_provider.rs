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
use std::io::{BufRead, Read};
use std::io;
use std::rc::Rc;
use time;
use unicode_segmentation::UnicodeSegmentation;
use ropey::Rope;
use serde_json as sj;

use ropey::RopeSlice;
use rich_content::RichContent;
use rich_content::RichLine;
use rich_content::HighlightSettings;

const DEFAULT_BLANK : char = ' ';

// Represents a order to edit a content. Offsets are calculated in CHARS, not bytes.
//TODO(njskalski) secure against overlapping cursors!
#[derive(Debug, Serialize, Deserialize)]
pub enum EditEvent {
    Insert {
        offset: usize,
        content: String,
    },
    Change {
        offset: usize,
        length: usize,
        content: String,
    },
}

#[derive(Debug)]
struct RopeBasedContent {
    lines: Rope,
    timestamp: time::Tm,
}

impl RopeBasedContent {
    pub fn new(reader_op: Option<&mut Read>) -> Self {
        match reader_op {
            Some(reader) => RopeBasedContent {
                lines: Rope::from_reader(reader).expect("failed to build rope from reader"), // TODO(njskalski) error handling
                timestamp: time::now(),
            },
            None => RopeBasedContent {
                lines: Rope::new(),
                timestamp: time::now(),
            }
        }
    }

    pub fn save<T : io::Write>(&self, writer : T) -> io::Result<()> {
        self.lines.write_to(writer)
    }
}

pub struct RopeBasedContentProvider {
    history: Vec<RopeBasedContent>,
    current: usize,
    // Contract: we do not version rich content. It doesn't make sense: redrawing screen
    // has a similar complexity to syntax highlighting, provided it's implemented properly.
    rich_content: Option<RichContent>,
}



// Applies events to RopeBasedContent producing new one, and returning *number of lines common* to
// both new and original contents.
// Now events are applied one after another in order they were issued.
//TODO in some combinations offsets should be recomputed. But I expect no such combinations appear. I should however check it just in case.
fn apply_events(c: &RopeBasedContent, events: &Vec<EditEvent>) -> (RopeBasedContent, usize) {
    let mut new_lines : Rope = c.lines.clone();

    // Offset is in CHARS, and since it's common, it's valid in both new and old contents.
    let mut lowest_common_offset = new_lines.len_chars();

    for event in events {
        match event {
            &EditEvent::Insert {
                ref offset,
                ref content,
            } => {
                lowest_common_offset = std::cmp::min(lowest_common_offset, *offset);
                new_lines.insert(*offset, content);
            },
            &EditEvent::Change {
                ref offset,
                ref length,
                ref content
            } => {
                lowest_common_offset = std::cmp::min(lowest_common_offset, *offset);
                new_lines.remove(*offset..(*offset+*length));
                new_lines.insert(*offset, content);
            },
            _ => debug!("event {:?} not supported yet", event),
        }
    }

    (
        RopeBasedContent {
            lines: new_lines,
            timestamp: time::now(),
        },
        lowest_common_offset
    )
}

impl RopeBasedContentProvider {
    pub fn new(reader_op : Option<&mut Read>) -> Self {
        RopeBasedContentProvider {
            history: vec![RopeBasedContent::new(reader_op)],
            current: 0,
            rich_content: None,
        }
    }

    pub fn set_rich_content_enabled(&mut self, enabled : bool) {
        if !enabled {
            self.rich_content = None
        } else {
            self.rich_content = Some(
                RichContent::new(
                    // TODO(njskalski): this needs decoupling (obviously) from here.
                    Rc::new(HighlightSettings::new()),
                    // This costs O(1), but if content provider changes, it needs update.
                    self.get_lines().clone()
                )
            )
        }
    }

    pub fn is_rich_content_enabled(&self) -> bool {
        self.rich_content.is_some()
    }

    pub fn get_lines(&self) -> &Rope {
        &self.history[self.current].lines
    }

    pub fn get_line(&self, line_no : usize) -> RopeSlice { self.history[self.current].lines.line(line_no) }

    pub fn len_lines(&self) -> usize { self.history[self.current].lines.len_lines() }

    pub fn get_rich_line(&self, line_no : usize) -> Option<Rc<RichLine>> {
        debug!("self rich_content.is_some: {:?}", self.rich_content.is_some());
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
        let (new_content, longest_common_prefix) = apply_events(&self.history[self.current], &events);
        self.history.truncate(self.current + 1); //droping redo's
        self.history.push(new_content);
        self.current += 1;

        // Dropping outdated lines of RichContent. They will be regenerated on-demand.
        self.rich_content.as_mut().map(|rich_content|{
            rich_content.drop_lines_after(longest_common_prefix);
        });
    }

    pub fn save<T : io::Write>(&self, writer : T) -> io::Result<()> {
        self.history.last().unwrap().lines.write_to(writer)
    }
}
