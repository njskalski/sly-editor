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

use cursive;
use std::fmt;
use uid;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewHandle {
    view_id : usize,
}

impl ViewHandle {
    pub fn new() -> Self {
        ViewHandle { view_id : uid::Id::<usize>::new().get() }
    }

    pub fn view_id(&self) -> usize {
        self.view_id
    }
}

impl fmt::Display for ViewHandle {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vh{}", self.view_id)
    }
}

impl Into<String> for ViewHandle {
    fn into(self) -> String {
        format!("vh{}", self.view_id)
    }
}
