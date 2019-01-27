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
use std::rc::Rc;
use interface::InterfaceNotifier;

/*
Index became a trait, because a default (fst-based) implementation cannot be modified after initial
construction. That doesn't play well with idea of index being filled with suggestions with delay.
An example of that use case is a fuzzy search in context of symbol. Some editing options will
be available by default, while code navigation options will become available only after successful
code analysis done by remote language server or after a succesful query to on-line service.
I want to display available options immediately, and then expand on them as soon as I get more
information.
*/

pub trait FuzzyIndexTrait {
    //TODO(njskalski) remove mut or write why it's impossible.
    fn get_results_for(
        &mut self,
        query : &String,
        limit_op : Option<usize>,
        inot_op : Option<InterfaceNotifier>,
    ) -> Vec<Rc<ViewItem>>;
}
