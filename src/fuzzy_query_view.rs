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

/*
MVP Requirements:
- selection (done), arrows (done), scrolling
- highlighting selected letters (done)
- two display modes: simple (one-liners) and complex: line with search term and 1-2 lines of
description. Example: search line is filename, description is it's location. (done, but can be upgraded)
- context header (done, but requires styling)
*/

// TODO(njskalski) profile and remove unnecessary copies. (actually profiling never pointed to this part of code so far)
// TODO(njskalski) parallelism is disabled right now. Need to update cursive to enable it.

// (CONCURRENCY) we are calling get_current_items twice here, not sure if the
// results are going to be the same, and I rely on them being in the same order. Clearly
// a possible error. Update: this seems to be resolved, as fst::Map supposedly returns results in alphabetic order.
// Also, I introduced chache via interior mutability (so it can be cleared in non-mut method "draw"), so number of
// calls has been reduced to one per draw-input cycle.

use std::any::Any;
use cursive::view::Selector;
type BoxedCallback<'a> = Box<for<'b> FnMut(&'b mut Any) + 'a>;

use std::rc::Rc;

use either::*;

use cursive::event;
use cursive::event::*;
use cursive::traits::*;
use cursive::vec::Vec2;
use cursive::view::{ScrollBase, View};
use cursive::views::*;
use cursive::{Cursive, Printer};
use cursive::theme::*;

use fuzzy_index::{HARD_QUERY_LIMIT};
use fuzzy_index_trait::FuzzyIndexTrait;
use settings::Settings;
use unicode_segmentation::UnicodeSegmentation as us;

use events::IEvent;
use fuzzy_view_item::*;
use interface::IChannel;
use std::collections::{HashMap, LinkedList};
use std::marker::Sized;
use std::sync::Arc;
use std::cell::RefCell;
use std::cell::Cell;
use std::cmp;

const WIDTH : usize = 100;

pub struct FuzzyQueryView {
    context: String,
    marker: String,
    query: String,
    scrollbase: ScrollBase,
    index: Arc<RefCell<FuzzyIndexTrait>>,
    size: Option<Vec2>,
    needs_relayout: Cell<bool>,
    selected: usize,
    settings: Rc<Settings>,
    channel: IChannel,
    items_cache : RefCell<Option<Rc<Vec<Rc<ViewItem>>>>>
}

impl View for FuzzyQueryView {

    fn layout(&mut self, size : Vec2) {
        self.size = Some(size);
        self.update_view();
    }

    fn draw(&self, printer: &Printer) {
        self.clear_cache();

        //draw context
        printer.print(
            (2, 0),
            &format!("Context : {:?} \tquery: {:?}", &self.context, &self.query),
        );

        debug!("size: {:?}", self.size);
        debug!("items: {:?}", self.get_current_items());

        self.scrollbase.draw(&printer.offset((0,1), printer.focused), |printer, line| {
            let items = self.get_current_items();
            let s = get_string_for_line(items.iter(), line).unwrap();
            debug!("i : {} s : {}", line, s);
            printer.print((0,0), &s);
        });
    }

    fn needs_relayout(&self) -> bool {
        self.needs_relayout.get()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        // one stands for header.
        let height = 1 + match *self.items_cache.borrow() {
            Some(ref items) => count_items_lines(items.iter()),
            None => 0
        };

        Vec2::new(cmp::min(WIDTH, constraint.x),
                  cmp::min(height, constraint.y))
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Char(c) => {
                self.add_letter(c);
                // debug!("hit {:?}", c);
                EventResult::Consumed(None)
            }
            Event::Key(Key::Backspace) => {
                self.backspace();
                // debug!("hit backspace");
                EventResult::Consumed(None)
            }
            Event::Key(Key::Up) => {
                if self.selected > 0 {
                    self.selected -= 1;
                };
                debug!("sel1 {:?}", &self.selected);
                EventResult::Consumed(None)
            }
            Event::Key(Key::Down) => {
                if self.selected + 1 < self.get_current_items().len()
                 {
                    self.selected += 1;
                }

                debug!("sel2 {:?}", &self.selected);
                EventResult::Consumed(None)
            }
            Event::Key(Key::Enter) => {
                let items = self.get_current_items();
                if items.len() > 0 {
                    assert!(items.len() > self.selected);
                    self.channel
                        .send(IEvent::FuzzyQueryBarSelected(
                            self.marker.clone(),
                            items[self.selected].get_marker().clone(),
                        ))
                        .unwrap();
                }
                EventResult::Consumed(None)
            }
            _ => {
                debug!("fuzzy got unhandled event {:?}", &event);
                EventResult::Ignored
            }
        }
    }
}

//TODO tests
fn count_items_lines<I, T>(items : I) -> usize
where
    T: AsRef<ViewItem>,
    I: Iterator<Item = T>
{
    items.fold(0, |acc, x| acc + x.as_ref().get_height_in_lines())
}

//TODO tests
/// Returns Option<(Item, number of lines preceeding items consumed)>
fn get_string_for_line<I, T>(mut items : I, line: usize) -> Option<String>
where
    T: AsRef<ViewItem>,
    I: Iterator<Item = T>
{
    let res : Either<usize, String> = items.fold(Left(0), |acc, item| {
        match acc {
            Right(s) => Right(s),
            Left(l) => {
                let item = item.as_ref();
                if l <= line && line < l + item.get_height_in_lines() {
                    const_assert!(MAX_VIEWITEM_HEIGHT == 2); //if it fails, update the code below
                    let line_idx = line - l;
                    let s : String = if line_idx == 0 {
                        item.get_header().clone()
                    } else {
                        item.get_description().clone().unwrap()
                    };
                    Right(s)
                } else {
                    Left(l + item.get_height_in_lines())
                }
            }
        }
    });

    match res {
        Left(_) => None,
        Right(s) => Some(s)
    }
}

impl FuzzyQueryView {

    fn clear_cache(&self) {
        (*self.items_cache.borrow_mut()) = None;
    }

    //blocking!
    fn get_current_items(&self) -> Rc<Vec<Rc<ViewItem>>> {
        let mut refmut = self.items_cache.borrow_mut();
        if refmut.is_none() {
            *refmut = Some(Rc::new(self.index.borrow_mut().get_results_for(&self.query, HARD_QUERY_LIMIT)));
            self.needs_relayout.set(true);
        }

        let rc : Rc<Vec<Rc<ViewItem>>> = refmut.as_ref().unwrap().clone();
        rc
    }

    fn get_item_colorstyle(&self, selected: bool, highlighted : bool) -> ColorStyle {
        self.settings.get_colorstyle(
            if highlighted { "theme/fuzzy_view/highlighted_text_color" } else { "theme/fuzzy_view/primary_text_color" }
            ,
            if selected { "theme/fuzzy_view/selected_background_color" } else { "theme/fuzzy_view/background_color"}
        )
    }

    fn draw_item(&self, item: &ViewItem, selected: bool, pos: (usize, usize), printer: &Printer) -> usize {
        //drawing header

        let row_width = self.size.unwrap().x;

        let header = us::graphemes(item.get_header().as_str(), true).collect::<Vec<&str>>();
        let query = us::graphemes(self.query.as_str(), true).collect::<Vec<&str>>();
        // debug!("header : {:?}\nquery : {:?}", header, query);
        let mut query_pos = 0;
        for header_pos in 0..header.len() {
            let highlighted = {
                if query_pos >= query.len() {
                    // debug!("query pos {:?}, query len {:?}", query_pos, query.len());
                    false
                } else {
                    if *header[header_pos] == *query[query_pos] {
                        // debug!("matched {:?} with {:?}", (*header[header_pos]).to_string(), (*query[query_pos]).to_string());
                        query_pos += 1;
                        true
                    } else {
                        // debug!("not matched {:?} with {:?}", (*header[header_pos]).to_string(), (*query[query_pos]).to_string());
                        false
                    }
                }
            };

            let colorstyle = self.get_item_colorstyle(selected, highlighted);
            printer.with_color(colorstyle, |printer| {
                printer.print((pos.0 + header_pos, pos.1), header[header_pos]);
            });
        }
        //empty suffix:
        let colorstyle = self.get_item_colorstyle(selected, false);
        for i in header.len()..row_width {
            printer.with_color(colorstyle, |printer| {
                printer.print((pos.0 + i, pos.1), " ");
            });
        };
        //end of drawing header

        //drawing description
        let desc_len = match item.get_description() {
            &Some(ref desc) => {
                for (i, line) in desc.lines().enumerate() {
                    printer.with_color(colorstyle, |printer| {
                        printer.print((pos.0, pos.1 + 1 + i), line);
                        for x in line.len()..row_width {
                            printer.print((pos.0 + x, pos.1 + 1 + i), " ");
                        }
                    });
                }
                desc.lines().count()
            }
            &None => 0,
        };
        desc_len + 1
    }

    pub fn new(index: Arc<RefCell<FuzzyIndexTrait>>, marker: String, channel: IChannel, settings : Rc<Settings>) -> Self {
        let mut res = FuzzyQueryView {
            context: "context".to_string(),
            query: String::new(),
            scrollbase: ScrollBase::new(),
            index: index,
            settings : settings,
            selected: 0,
            channel: channel,
            marker: marker,
            items_cache : RefCell::new(None),
            size : None,
            needs_relayout: Cell::new(false)
        };
        res.update_view();
        res
    }

    fn update_view(&mut self) {
        self.selected = 0; //TODO add following currently highlighted item (if not removed).
        self.try_update_scrollbase();
    }

    fn try_update_scrollbase(&mut self) {
        let items = self.get_current_items(); //just call to update cache, start search if necessary //TODO this is blocking now
        match self.size {
            Some(xy) => self.scrollbase.set_heights(xy.y - 1, count_items_lines(items.iter())),
            None => {}
        };
    }

    fn add_letter(&mut self, letter: char) {
        self.query.push(letter);
        self.update_view();
    }

    fn backspace(&mut self) {
        self.query.pop();
        self.update_view();
    }
}
