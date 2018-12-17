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

use syntax;
use ropey::RopeSlice;
use content_type::RichContent;
use content_type::RichLine;

const DEFAULT_BLANK : char = ' ';

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

#[derive(Debug)]
pub struct RopeBasedContentProvider {
    history: Vec<(RopeBasedContent, Option<RichContent>)>,
    current: usize,
}

//now events are applied one after another.
//TODO in some combinations offsets should be recomputed. But I expect no such combinations appear. I should however check it just in case.
fn apply_events(c: &RopeBasedContent, events: &Vec<EditEvent>) -> RopeBasedContent {
    let mut new_lines : Rope = c.lines.clone();
    // let mut offsets = c.offsets.clone();
    for event in events {
        match event {
            &EditEvent::Insert {
                ref offset,
                ref content,
            } => {
                new_lines.insert(*offset, content);
            },
            &EditEvent::Change {
                ref offset,
                ref length,
                ref content
            } => {
                new_lines.remove(*offset..(*offset+*length));
                new_lines.insert(*offset, content);
            },
            _ => debug!("event {:?} not supported yet", event),
        }
    }

    RopeBasedContent {
        lines: new_lines,
        timestamp: time::now(),
    }
}

impl RopeBasedContentProvider {
    pub fn new(reader_op : Option<&mut Read>) -> Self {
        RopeBasedContentProvider {
            history: vec![(RopeBasedContent::new(reader_op), None)],
            current: 0,
        }
    }

    pub fn get_lines(&self) -> &Rope {
        &self.history[self.current].0.lines
    }

    pub fn get_line(&self, line_no : usize) -> RopeSlice { self.history[self.current].0.lines.line(line_no) }

    pub fn len_lines(&self) -> usize { self.history[self.current].0.lines.len_lines() }

    pub fn get_rich_line(&self, line_no : usize) -> Option<Rc<RichLine>> {
        let rich_content_op = &self.history[self.current].1;

        match rich_content_op {
            None => None,
            Some(rich_content) => {
                rich_content.get_line(line_no).map(|rc_line| rc_line.clone())
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current < self.history.len() - 1
    }

    pub fn submit_events(&mut self, events: Vec<EditEvent>) {
        debug!("got events {:?}", events);
        let new_content = apply_events(&self.history[self.current].0, &events);
        self.history.truncate(self.current + 1); //droping redo's
        self.history.push((new_content, None));
        self.current += 1;
    }

    pub fn save<T : io::Write>(&self, writer : T) -> io::Result<()> {
        self.history.last().unwrap().0.lines.write_to(writer)
    }
}
