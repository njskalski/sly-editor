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

use std::fmt;
use uid::Id as IdT;

//docs: https://docs.rs/uid/0.1.4/uid/struct.Id.html
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BufferIdIdType(());

type Id = IdT<BufferIdIdType>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferId {
    id: usize,
}

impl BufferId {
    pub fn new() -> Self {
        BufferId { id: Id::new().get() }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn from_string(s: &str) -> Option<Self> {
        s[1..].parse::<usize>().ok().map(|id| BufferId { id })
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "b{}", self.id)
    }
}
