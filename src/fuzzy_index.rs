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
}

impl FuzzyIndexTrait for FuzzyIndex {
    fn get_results_for(&mut self, query : &String, limit : usize) -> Vec<Rc<ViewItem>> {
        let mut results : Vec<Rc<ViewItem>> = Vec::new();

        // this has no effect if we already had such task in progress.
        self.start_search(query, limit);
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
    pub fn new(word_list : Vec<ViewItem>) -> FuzzyIndex {
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
        };
        i.start_search(&"".to_string(), HARD_QUERY_LIMIT);
        i
    }

    fn start_search(&mut self, query : &String, limit : usize) {
        if self.cache.contains_key(query) {
            if self.cache[query].get_limit() >= &limit {
                // we got it.
                return;
            } else {
                // not enough results.
                self.cache.remove(query);
                // linked list has no remove, so cache and cache_order can become desynchronized. I
                // don't rely on them being in sync, so don't worry.
                // self.cache_order.remove(&query);
            }
        }

        let task : FuzzySearchTask = FuzzySearchTask::new(query.clone(), self, limit);
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

struct FuzzySearchTask {
    receiver : mpsc::Receiver<u64>,
    query :    String,
    item_ids : RefCell<Vec<u64>>,
    done :     Cell<bool>,
    limit :    usize,
}

impl FuzzySearchTask {
    pub fn new(query : String, index : &FuzzyIndex, limit : usize) -> FuzzySearchTask {
        let (sender, receiver) = channel::<u64>();
        let item_ids = Vec::new();

        let index_ref_copy = index.index.clone();
        let query_copy = query.clone();
        let items_sizes_ref = index.items_sizes.clone();

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
                if it < limit {
                    it += 1;
                } else {
                    break;
                }
            }
            debug!("3");
        });

        FuzzySearchTask {
            receiver : receiver,
            item_ids : RefCell::new(item_ids),
            done :     Cell::new(false),
            query :    query,
            limit :    limit,
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

    pub fn is_done(&self) -> bool {
        self.done.get()
    }

    pub fn get_query(&self) -> &String {
        &self.query
    }

    pub fn get_limit(&self) -> &usize {
        &self.limit
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
