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

// the "rich" text format is strongly work in progress, it was written primarily to check what would
// be the cost of integrating syntax highlighting from other editors. I decided to proceed with that
// approach, it's on the May 2018 roadmap.

// TODO(njskalski) secure with accessors after fixing the format.

//TODO(njskalski) NEXT STEPS:
// 1) replace Vec in cache with rpds Vector, this way implementing versioning of colors
// 2) then you can remove Rc from Rich Line (probably) and give indexing again (not sure what for)

use content_provider::RopeBasedContentProvider;
use ropey::Rope;
use std::borrow::Borrow;
use std::cell::Ref;
use std::iter::{ExactSizeIterator, Iterator};
use std::marker::Copy;
use std::ops::{Index, IndexMut};

use cursive::theme::Color;
use std::rc::Rc;

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use syntect::highlighting::{
    HighlightIterator,
    HighlightState,
    Highlighter,
    Style,
    Theme,
    ThemeSet,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

const PARSING_MILESTONE : usize = 10;

#[derive(Debug)]
pub struct RichLine {
    line_no : usize,
    length :  usize,
    body :    Vec<(Color, String)>,
}

//TODO(njskalski): optimise, rethink api. maybe even drop the content.
impl RichLine {
    pub fn new(line_no : usize, body : Vec<(Color, String)>) -> Self {
        let mut len : usize = 0;
        for piece in &body {
            len += piece.1.len()
        }

        RichLine { line_no, length : len, body }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn get_color_at(&self, idx : usize) -> Option<Color> {
        let mut cur_idx : usize = 0;

        for chunk in &self.body {
            if cur_idx + chunk.1.len() > idx {
                return Some(chunk.0);
            }
            cur_idx += chunk.1.len();
        }

        None
    }

    pub fn get_line_no(&self) -> usize {
        self.line_no
    }
}

#[derive(Debug)]
pub struct HighlightSettings {
    theme :      Theme,
    syntax :     SyntaxReference,
    syntax_set : SyntaxSet,
}

//TODO move const strings to settings parameters.
impl HighlightSettings {
    pub fn new(extension : &str) -> Option<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines().clone();
        let ts = &ThemeSet::load_defaults();

        let syntax = syntax_set.find_syntax_by_extension(extension)?.clone();
        let theme = ts.themes["base16-ocean.dark"].clone();

        Some(HighlightSettings {
            theme :      theme,
            syntax :     syntax,
            syntax_set : syntax_set,
        })
    }
}

#[derive(Clone, Debug)]
struct ParseCacheRecord {
    line_to_parse :   usize,
    parse_state :     ParseState,
    highlight_state : HighlightState,
}

impl ParseCacheRecord {
    pub fn new(parse_state : ParseState, highlighter : &Highlighter) -> Self {
        let scope_stack = ScopeStack::new();
        let highlight_state = HighlightState::new(highlighter, scope_stack);

        ParseCacheRecord { line_to_parse : 0, parse_state, highlight_state }
    }
}

#[derive(Clone)]
pub struct RichContent {
    highlight_settings : Rc<HighlightSettings>,
    raw_content :        Rope,
    // the key corresponds to number of next line to parse by ParseState. NOT DONE, bc I don't want
    // negative numbers.
    parse_cache : RefCell<Vec<ParseCacheRecord>>, //TODO is it possible to remove RefCell?
    lines :       RefCell<Vec<Rc<RichLine>>>,
}

impl RichContent {
    pub fn new(settings : Rc<HighlightSettings>, rope : Rope) -> Self {
        RichContent {
            highlight_settings : settings,
            raw_content :        rope,
            //contract: sorted.
            parse_cache : RefCell::new(Vec::new()),
            //contract: max key of parse_cache < len(lines)
            lines : RefCell::new(Vec::new()),
        }
    }

    //TODO(njskalski): it should have some kind of observer, but reference to ContentProvider
    // creates a nightmare of lifetimes escalation. Maybe it can be merged with "drop lines"?
    pub fn update_raw_content(&mut self, rope : Rope) {
        self.raw_content = rope;
    }

    // Drop lines removes cache after lines_to_save. Since it operates on cache, it's non mutable.
    // lines_to_save is *exclusive*. It corresponds to number of lines to save. So 0 means
    // "no lines to save", therefore line #0 is subject to drop as well.
    pub fn drop_lines(&self, lines_to_save : usize) {
        debug!("dropping all lines from (exclusive) {:}", lines_to_save);
        //dropping cache.
        {
            if lines_to_save > 0 {
                // this is index of ParseCacheRecord in cache vector, not in lines.
                let idx_op = self.get_cache_idx(lines_to_save - 1);
                let mut parse_cache = self.parse_cache.borrow_mut();
                match idx_op {
                    Some(idx) => {
                        // split_off is exclusive, our ParseCacheRecord index is inclusive.
                        if parse_cache.len() >= idx + 1 {
                            parse_cache.split_off(idx + 1);
                        }
                    }
                    None => parse_cache.clear(),
                }
            } else {
                let mut parse_cache = self.parse_cache.borrow_mut();
                parse_cache.clear();
            }
        }
        //dropping lines.
        {
            let mut lines = self.lines.borrow_mut();
            //            if lines.len() >= lines_to_save {
            lines.split_off(lines_to_save);
            //            }
        }
    }

    pub fn len_lines(&self) -> usize {
        self.raw_content.len_lines()
    }

    /// Returns index of last valid ParseCacheRecord given the last non-modified line index.
    /// Every ParseCacheRecord with index higher than the one returned is going to be invalid after
    /// line (#line_no + 1) gets modified.
    // TODO unit tests.
    fn get_cache_idx(&self, line_no : usize) -> Option<usize> {
        let parse_cache : Ref<Vec<ParseCacheRecord>> = self.parse_cache.borrow();

        debug!("parse cache len = {}", parse_cache.len());

        if !parse_cache.is_empty() {
            let cache : Option<usize> =
                match parse_cache.binary_search_by(|rec| rec.line_to_parse.cmp(&line_no)) {
                    Ok(idx) => Some(idx),
                    Err(higher_index) => {
                        if higher_index == 0 {
                            None
                        } else {
                            Some(higher_index - 1)
                        }
                    }
                };
            cache
        } else {
            None
        }
    }

    /// Returns last valid ParseCacheRecord given the last non-modified line index.
    /// ParseCacheRecord.line_to_parse > line_no (*strictly higher* guaranteed).
    pub fn get_cache(&self, line_no : usize) -> Option<ParseCacheRecord> {
        let idx_op = self.get_cache_idx(line_no);
        debug!("idx_op : {:?}", &idx_op);
        idx_op.and_then(|x| self.parse_cache.borrow().get(x).map(|cache| cache.clone()))
    }

    pub fn get_line(&self, line_no : usize) -> Option<Rc<RichLine>> {
        debug!("ask for rich line {:}", line_no);

        if self.lines.borrow().len() > line_no {
            return self.lines.borrow().get(line_no).map(|x| x.clone());
        }

        // It could be cached, but escalating <'a> everywhere is just a nightmare.
        let highlighter = Highlighter::new(&self.highlight_settings.theme);

        // see contracts
        let mut parse_cache : ParseCacheRecord = match self.get_cache(line_no) {
            None => {
                debug!("cache miss for line_no {:}", line_no);
                ParseCacheRecord::new(
                    ParseState::new(&self.highlight_settings.syntax),
                    &highlighter,
                )
            }
            Some(x) => {
                debug!("cache HIT for line_no {:} ({:})", line_no, x.line_to_parse);
                x
            }
        };

        let line_limit = std::cmp::min(
            (line_no / PARSING_MILESTONE + 1) * PARSING_MILESTONE,
            self.raw_content.len_lines(),
        );
        let first_line = std::cmp::min(self.lines.borrow().len(), parse_cache.line_to_parse);

        self.drop_lines(first_line);

        debug!("generating lines from [{:}, {:})", first_line, line_limit);
        for current_line_no in (first_line)..(line_limit) {
            let line_str = self.raw_content.line(current_line_no).to_string();
            let ops =
                parse_cache.parse_state.parse_line(&line_str, &self.highlight_settings.syntax_set);

            let ranges : Vec<(Style, &str)> = {
                HighlightIterator::new(
                    &mut parse_cache.highlight_state,
                    &ops[..],
                    &line_str,
                    &highlighter,
                )
                .collect()
            };

            let new_line : Vec<(Color, String)> = ranges
                .into_iter()
                .map(|(style, words)| (simplify_style(&style), words.to_string()))
                .collect();
            let rc_rich_line = Rc::new(RichLine::new(current_line_no, new_line));

            //            debug!("generated line {:?}", rc_rich_line);
            self.lines.borrow_mut().push(rc_rich_line);
        }

        // cache, unless it was a last line.
        if line_limit < self.raw_content.len_lines() {
            let mut copy_to_cache = parse_cache.clone();
            copy_to_cache.line_to_parse = line_limit;

            // just checking.

            if !self.parse_cache.borrow().is_empty() {
                let prev_line = self.parse_cache.borrow().last().unwrap().line_to_parse;
                let new_line = line_limit;
                //                debug!("prev_line {:}, new_line {:}", prev_line, new_line);
                if prev_line >= line_limit {
                    error!("expeced prev_line {:} < new_line {:}", prev_line, new_line);
                }
            }

            debug!("caching {:?}", &copy_to_cache);
            self.parse_cache.borrow_mut().push(copy_to_cache);
        }

        debug!("returning line {:}", line_no);
        self.lines.borrow().get(line_no).map(|x| x.clone())
    }
}

struct RichLinesIterator<'a> {
    content : &'a RichContent,
    line_no : usize,
}

impl<'a> Iterator for RichLinesIterator<'a> {
    type Item = Rc<RichLine>;

    fn next(&mut self) -> Option<Self::Item> {
        let old_line_no = self.line_no;
        self.line_no += 1;
        let line = self.content.get_line(old_line_no);
        line
    }
}

impl<'a> ExactSizeIterator for RichLinesIterator<'a> {
    fn len(&self) -> usize {
        self.content.len_lines()
    }
}

//impl <'a> Index<usize> for RichLinesIterator<'a> {
//    type Output = Rc<RichLine>;
//
//    //panics //TODO format docs.
//    fn index(&self, idx : usize) -> &Rc<RichLine> {
//        self.content.get_line(idx).unwrap()
//    }
//}

fn simplify_style(style : &Style) -> Color {
    Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b)
}

pub mod tests {
    use super::*;

    fn basic_setup() {

    }

}