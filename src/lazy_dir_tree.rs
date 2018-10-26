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

// I am not entirely sure this is enum is needed.

use std::fmt;
use std::rc::Rc;
use std::path::*;
use std::cmp::{Ord, PartialOrd, PartialEq, Ordering};

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum LazyTreeNode {
    RootNode(Vec<Rc<String>>), // TODO(njskalski) start here!
    DirNode(Rc<String>), //full path TODO(njskalski) add RefCell<Vec<LazyTreeNode>> cache, refresh "on file change"
    FileNode(Rc<String>),
}

impl fmt::Display for LazyTreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LazyTreeNode::RootNode(_) => {write!(f,"<root>") },
            &LazyTreeNode::DirNode(ref path) => {write!(f, "{}", path.split('/').last().unwrap())},
            &LazyTreeNode::FileNode(ref path) => {write!(f, "{}", path.split('/').last().unwrap())}
            // LazyTreeNode::ExpansionPlaceholder => {write!(f,"...")}
        }
    }


}

// TODO(njskalski) now I just build a RootNode even if there is a single directory open
// because I want to support hot-loading additional directories.
impl LazyTreeNode {
    pub fn new(directories : &Vec<String>) -> Self {
        LazyTreeNode::RootNode(directories.iter().map(|x| Rc::new(x.clone())).collect())
    }
}
