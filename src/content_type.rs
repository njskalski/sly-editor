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

use ropey::Rope;
use std::marker::Copy;
use std::borrow::Borrow;
use std::ops::{Index, IndexMut};
use std::iter::{Iterator, ExactSizeIterator};
use content_provider::RopeBasedContentProvider;
use std::cell::Ref;

use cursive::theme::Color;
use std::rc::Rc;

use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter, Style, ThemeSet, Theme};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};
use std::cell::Cell;
use std::collections::HashMap;
use std::cell::RefCell;

const PARSING_MILESTONE : usize = 80;

#[derive(Debug)]
pub struct RichLine {
    length : usize,
    body : Vec<(Color, String)>
}

//TODO(njskalski): optimise, rethink api. maybe even drop the content.
impl RichLine {
    pub fn new(body : Vec<(Color, String)>) -> Self {
        let mut len : usize = 0;
        for piece in &body {
            len += piece.1.len()
        }

        RichLine { length : len , body }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn get_color_at(&self, idx : usize) -> Option<Color> {
        let mut cur_idx : usize = 0;

        for chunk in &self.body {
            if cur_idx + chunk.1.len() > idx {
                return Some(chunk.0)
            }
            cur_idx += chunk.1.len();
        }

        None
    }
}

#[derive(Debug)]
pub struct HighlightSettings {
    theme : Theme,
    syntax : SyntaxReference,
    syntax_set : SyntaxSet,
}

//TODO move const strings to settings parameters.
impl HighlightSettings {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines().clone();
        let ts = &ThemeSet::load_defaults();

        let syntax = syntax_set.find_syntax_by_extension("rb").unwrap().clone();
        let theme = ts.themes["base16-ocean.dark"].clone();

        HighlightSettings { theme, syntax, syntax_set }
    }
}

#[derive(Clone)]
struct ParseCacheRecord {
    line_to_parse : usize,
    parse_state : ParseState,
    highlight_state : HighlightState,
//    scope_stack : ScopeStack,
}

impl ParseCacheRecord {
    pub fn new(parse_state : ParseState, highlighter : &Highlighter) -> Self {

        let scope_stack = ScopeStack::new();
        let highlight_state = HighlightState::new(highlighter, scope_stack);

        ParseCacheRecord { line_to_parse : 0 , parse_state, highlight_state}
    }
}

pub struct RichContent {
    highlight_settings : Rc<HighlightSettings>,
    raw_content: Rope,
    // the key corresponds to number of next line to parse by ParseState. NOT DONE, bc I don't want negative numbers.
    parse_cache: RefCell<Vec<ParseCacheRecord>>, //TODO is it possible to remove RefCell?
    lines : RefCell<Vec<Rc<RichLine>>>,
}

impl RichContent {
    pub fn new(settings : Rc<HighlightSettings>, rope : Rope) -> Self {
        RichContent {
            highlight_settings : settings,
            raw_content : rope,
            //contract: sorted.
            parse_cache : RefCell::new(Vec::new()),
            //contract: max key of parse_cache < len(lines)
            lines : RefCell::new(Vec::new())
        }
    }

    pub fn len_lines(&self) -> usize {
        self.raw_content.len_lines()
    }

    // result.0 > line_no
    pub fn get_cache(&self, line_no : usize) -> Option<ParseCacheRecord> {
        let parse_cache : Ref<Vec<ParseCacheRecord>> = self.parse_cache.borrow();

        if parse_cache.is_empty() {
            return None;
        }

        //TODO do edge cases
        let cache : Option<&ParseCacheRecord> = match parse_cache.binary_search_by(|rec| rec.line_to_parse.cmp(&line_no)) {
            Ok(idx) => parse_cache.get(idx),
            Err(higher_index) => {
                if higher_index == 0 { None } else {
                    parse_cache.get(higher_index - 1)
                }
            }
        };

        cache.map(|x| x.clone())
    }

    pub fn get_line(&self, line_no : usize) -> Option<Rc<RichLine>> {
        if self.lines.borrow().len() > line_no {
            return self.lines.borrow().get(line_no).map(|x| x.clone());
        }

        // It could be cached, but escalating <'a> everywhere is just a nightmare.
        let highlighter = Highlighter::new(&self.highlight_settings.theme);

        // see contracts
        let mut parse_cache : ParseCacheRecord = match self.get_cache(line_no) {
            None => ParseCacheRecord::new(
                ParseState::new(&self.highlight_settings.syntax), &highlighter),
            Some(x) => x
        };

        let line_limit = std::cmp::min(line_no + PARSING_MILESTONE, self.raw_content.len_lines());
        let first_line = std::cmp::min(self.lines.borrow().len(), parse_cache.line_to_parse);

        for current_line in (first_line)..(line_limit) {
            let line_str = self.raw_content.line(current_line).to_string();

            let ops = parse_cache.parse_state.parse_line(
                &line_str,
                &self.highlight_settings.syntax_set
            );

            let ranges: Vec<(Style, &str)> = {
                HighlightIterator::new(&mut parse_cache.highlight_state,
                                       &ops[..],
                                       &line_str,
                                       &highlighter).collect() };

            let new_line: Vec<(Color, String)> = ranges
                .into_iter()
                .map(|(style, words)| (simplify_style(&style), words.to_string()))
                .collect();

            let rc_rich_line = Rc::new(RichLine::new(new_line));

            self.lines.borrow_mut().push(rc_rich_line);

            // cache
            if current_line % PARSING_MILESTONE == 0 {
                let mut copy_to_cache = parse_cache.clone();
                copy_to_cache.line_to_parse = current_line;

                // just checking.

                if !self.parse_cache.borrow().is_empty() {
                    let prev_line = self.parse_cache.borrow().last().unwrap().line_to_parse;
                    let new_line = current_line;
                    if prev_line >= current_line {
                        error!("expeced prev_line {:} < new_line {:}", prev_line, new_line);
                    }
                }

                self.parse_cache.borrow_mut().push(copy_to_cache);
            }
        }

        self.lines.borrow().get(line_no).map(|x| x.clone())
    }
}

struct RichLinesIterator<'a> {
    content : &'a RichContent,
    line_no : usize
}

impl <'a> Iterator for RichLinesIterator<'a> {
    type Item = Rc<RichLine>;

    fn next(&mut self) -> Option<Self::Item> {
        let old_line_no = self.line_no;
        self.line_no += 1;
        let line  = self.content.get_line(old_line_no);
        line
    }
}

impl <'a> ExactSizeIterator for RichLinesIterator<'a> {
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

fn simplify_style(style: &Style) -> Color {
    Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    )
}
