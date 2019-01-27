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

// TODO(njskalski) limit is ignored now

extern crate fst;
extern crate fst_regex;

use self::fst::map;
use self::fst::{IntoStreamer, Map, Streamer};
use self::fst_regex::Regex;
use std::char::{ToLowercase, ToUppercase};
use std::iter::FromIterator;

use fuzzy_index_trait::*;
use fuzzy_view_item::*;

use interface::InterfaceNotifier;
use serde::de::Unexpected::Option as SerdeOption;
use std::cell::*;
use std::collections::HashMap;
use std::collections::*;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::*;
use std::sync::Arc;
use std::thread;

const MAX_CACHE_SIZE : usize = 30;
pub const HARD_QUERY_LIMIT : usize = 50;
/*
Disclaimer:
index matches a fuzzy query to u64, that can be converted to items. So basically we have
items[index[header]] = Vec<ViewItem>, because MULTIPLE ITEMS can have a SINGLE header.
example: multiple files with SAME NAME
example: mutliple methods with same names from different files.

It is not possible to add items after a fst::Map has been built.
*/
pub struct FuzzyIndex {
    index : Arc<Map>, /* this is Map<String, u64>. It's Arc, because queries are ran in
                       * worker threads. */
    items :       HashMap<u64, Vec<Rc<ViewItem>>>,
    items_sizes : Arc<HashMap<u64, usize>>, /* this field is used by workers to determine
                                             * whether they hit the limit
                                             * of records or not. */
    cache : HashMap<String, FuzzySearchTask>,
    /// used to know in what order clear the cache. Does not contain empty query, which is computed
    /// in cache immediately. Also, cache_order and cache sizes are not synchronized, as
    /// cache_order can contain duplicates in rare situations.
    cache_order : LinkedList<String>,
    inot_op : Option<InterfaceNotifier>,
}

impl FuzzyIndexTrait for FuzzyIndex {
    fn get_results_for(&mut self, query : &String, limit_op : Option<usize>) -> Vec<Rc<ViewItem>> {
        let mut results : Vec<Rc<ViewItem>> = Vec::new();

        // this has no effect if we already had such task in progress.
        self.start_search(query, limit_op, self.inot_op.clone());
        let task = self.cache.get(query).unwrap(); // unwrap always succeeds, see line above

        let result_ids = task.get_result_ids();

        for id in result_ids.iter() {
            assert!(self.items.contains_key(id));
            for item in self.items[id].iter() {
                results.push(item.clone());
            }
        }

        results
    }
}

impl FuzzyIndex {
    pub fn new(word_list : Vec<ViewItem>, inot_op : Option<InterfaceNotifier>) -> FuzzyIndex {
        // TODO we can consume word_list here instead of calling ci.copy() below
        let mut items : HashMap<u64, Vec<Rc<ViewItem>>> = HashMap::new();
        let mut header_to_key : HashMap<String, u64> = HashMap::new();
        let mut key = 0;
        for ci in word_list {
            let header : &String = &ci.get_header();

            if header_to_key.contains_key(header) {
                let id : u64 = header_to_key[header];
                items.get_mut(&id).unwrap().push(Rc::new(ci.clone()));
            } else {
                let mut vec : Vec<Rc<ViewItem>> = Vec::new();
                vec.push(Rc::new(ci.clone()));
                header_to_key.insert(header.clone(), key);
                items.insert(key, vec);
                key += 1;
            }
        }

        let mut header_to_key_sorted : Vec<(String, u64)> =
            header_to_key.iter().map(|item| (item.0.clone(), item.1.clone())).collect();
        header_to_key_sorted.sort();
        let map = Map::from_iter(header_to_key_sorted).unwrap();

        let mut item_sizes : HashMap<u64, usize> = HashMap::new();
        for (k, v) in items.iter() {
            item_sizes.insert(*k, v.len());
        }

        let mut i = FuzzyIndex {
            index :       Arc::new(map),
            items :       items,
            items_sizes : Arc::new(item_sizes),
            cache :       HashMap::new(),
            cache_order : LinkedList::new(),
            inot_op :     inot_op,
        };
        // we do not pass inot_op below, since we don't want to necessarily update interface on
        // creation, but we do want our "" results to be available immediately.
        i.start_search(&"".to_string(), None, None);
        i
    }

    fn start_search(
        &mut self,
        query : &String,
        limit_op : Option<usize>,
        inotop : Option<InterfaceNotifier>,
    ) {
        // TODO(njskalski): while it is possible to update limit as query runs, resuming query is
        // not implemented yet, so I just restart it now.
        if let Some(ref mut runner) = self.cache.get(query) {
            if let Some(ref inot) = inotop {
                runner.update_inot(inot.clone());
            }

            if let Some(old_limit) = runner.limit() {
                if let Some(new_limit) = limit_op {
                    if old_limit >= new_limit {
                        return; // Old limit bigger than new one, no need to restart.
                    }
                } else {
                    // here should be limit update and resume of thread, but not implemented.
                    // this scenario actually moves ahead with re-starting search.
                }
            } else {
                return; // no limit in the old one? It's going to grab all. No need to restart.
            }
        }

        // We either have no cache or cache is not good enough.

        if self.cache.contains_key(query) {
            self.cache.remove(query);
            // We *should* drop query from cache_order here, but we do not. Here is reason:
            // linked list has no remove, so cache and cache_order can become desynchronized. I
            // don't rely on them being in sync, so don't worry.
            // self.cache_order.remove(&query)
        }

        let task : FuzzySearchTask = FuzzySearchTask::new(query.clone(), self, limit_op, inotop);
        self.cache.insert(query.clone(), task);

        if query.len() > 0 {
            self.cache_order.push_back(query.clone());

            while self.cache.len() - 1 /* >= 0, look above */ > MAX_CACHE_SIZE {
                // -1 and condition above stand for the fact I want to keep empty query computed all
                // the time!
                let oldest_query = self.cache_order.pop_front().unwrap();
                // this doesn't have to succeed, the cache_order can become a little longer than
                // cache, look at the top of the function why (duplicates possible)
                self.cache.remove(&oldest_query);
            }
        }
    }
}

enum FuzzySearchTaskUpdate {
    Inot(InterfaceNotifier),
    NewLimit(usize),
}

// TODO(njskalski): add resume (re-spawning thread) if limit is bigger.
struct FuzzySearchTask {
    receiver :            mpsc::Receiver<u64>,
    query :               String,
    item_ids :            RefCell<Vec<u64>>,
    done :                Cell<bool>,
    limit_op :            Option<usize>,
    update_stram_sender : Sender<FuzzySearchTaskUpdate>,
    has_inot :            bool,
}

impl FuzzySearchTask {
    pub fn new(
        query : String,
        index : &FuzzyIndex,
        mut limit_op : Option<usize>,
        mut inot_op : Option<InterfaceNotifier>,
    ) -> FuzzySearchTask {
        let (sender, receiver) = channel::<u64>();
        let item_ids = Vec::new();

        let has_inot = inot_op.is_some();

        let index_ref_copy = index.index.clone();
        let query_copy = query.clone();
        let items_sizes_ref = index.items_sizes.clone();

        let (update_stream_sender, update_stream_receiver) = channel::<FuzzySearchTaskUpdate>();

        thread::spawn(move || {
            debug!("1");
            let regex = query_to_regex(&query_copy);
            let stream_builder : map::StreamBuilder<Regex> = index_ref_copy.search(regex);
            let mut stream = stream_builder.into_stream();

            debug!("2");
            let mut results : Vec<&ViewItem> = Vec::new();
            let mut it : usize = 0;
            while let Some((header, key)) = stream.next() {
                if sender.send(key).is_err() {
                    debug!("unable to send key in FuzzySearchTask internal worker");
                    return;
                }
                it += items_sizes_ref[&key];

                while let Ok(update) = update_stream_receiver.try_recv() {
                    match update {
                        FuzzySearchTaskUpdate::NewLimit(new_limit) => {
                            if let Some(old_limit) = limit_op {
                                if old_limit < new_limit {
                                    limit_op = Some(new_limit);
                                }
                            }
                        }
                        FuzzySearchTaskUpdate::Inot(inot) => {
                            inot_op = Some(inot);
                        }
                    }
                }

                if let Some(inot) = inot_op {
                    inot.refresh();
                }

                if let Some(limit) = limit_op {
                    if it < limit {
                        it += 1;
                    } else {
                        break;
                    }
                }
            }
            debug!("3");
        });

        FuzzySearchTask {
            receiver :            receiver,
            item_ids :            RefCell::new(item_ids),
            done :                Cell::new(false),
            query :               query,
            limit_op :            limit_op,
            update_stram_sender : update_stream_sender,
            has_inot :            has_inot,
        }
    }

    pub fn get_result_ids(&self) -> Ref<Vec<u64>> {
        while !self.done.get() {
            match self.receiver.try_recv() {
                Ok(string) => {
                    self.item_ids.borrow_mut().push(string);
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    self.done.set(true);
                }
            }
        }

        self.item_ids.borrow()
    }

    /// If runner is done, results in noop.
    //
    // TODO(njskalski): store inot for case of restart
    pub fn update_inot(&self, inot : InterfaceNotifier) {
        self.update_stram_sender.send(FuzzySearchTaskUpdate::Inot(inot)); // ignoring result.
    }

    pub fn has_inot(&self) -> bool {
        self.has_inot
    }

    pub fn is_done(&self) -> bool {
        self.done.get()
    }

    pub fn get_query(&self) -> &String {
        &self.query
    }

    pub fn limit(&self) -> &Option<usize> {
        &self.limit_op
    }
}

fn query_to_regex(query : &String) -> Regex {
    let mut regex_vec : Vec<char> = Vec::new();

    regex_vec.append(&mut vec!['.', '*']);

    for letter in query.chars() {
        regex_vec.push('[');
        for subletter in letter.to_lowercase() {
            regex_vec.push(subletter);
        }

        for subletter in letter.to_uppercase() {
            regex_vec.push(subletter);
        }
        regex_vec.push(']');
        regex_vec.append(&mut vec!['.', '*']);
    }

    let regex_str = String::from_iter(regex_vec);

    debug!("regex str {:?}", regex_str);

    let regex : Regex = Regex::new(&regex_str).unwrap();

    regex
}
