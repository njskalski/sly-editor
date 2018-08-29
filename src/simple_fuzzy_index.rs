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

use fuzzy_view_item::*;
use fuzzy_index_trait::FuzzyIndexTrait;
use std::rc::Rc;

// pub trait FuzzyIndexTrait {
//     fn get_results_for(&mut self, query : &String, limit : usize) -> Vec<Rc<ViewItem>>;
// }

pub struct SimpleIndexTrait {
    
}
