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

// TODO(njskalski) add RefCell<Vec<LazyTreeNode>> cache, refresh "on file change"

use std::fmt;
use std::rc::Rc;
use std::path::*;
use std::cmp::{Ord, PartialOrd, PartialEq, Ordering};

/*
    This enum contains entire paths to filesystem nodes (directories or files), except the root
    node that contains a list.
*/

//TODO(njskalski) add hotloading directories.
//TODO(njskalski) create fourth category for out-of-folders files (second argument of constructor).

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum LazyTreeNode {
    RootNode(Vec<Rc<LazyTreeNode>>),
    DirNode(Rc<PathBuf>),
    FileNode(Rc<PathBuf>),
}

impl fmt::Display for LazyTreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LazyTreeNode::RootNode(_) => {write!(f,"<root>") },
            &LazyTreeNode::DirNode(ref path) => {write!(f, "{}", path.file_name().unwrap().to_string_lossy())},
            &LazyTreeNode::FileNode(ref path) => {write!(f, "{}", path.file_name().unwrap().to_string_lossy())}
        }
    }
}

impl LazyTreeNode {
    pub fn new(directories : Vec<PathBuf>, files : Vec<PathBuf>) -> Self {
        let mut nodes : Vec<Rc<LazyTreeNode>> = directories.into_iter().map(
            |x| Rc::new(LazyTreeNode::DirNode(Rc::new(x)))).chain(
            files.into_iter().map(
                |x| Rc::new(LazyTreeNode::FileNode(Rc::new(x))))).collect();
        nodes.sort();
        LazyTreeNode::RootNode(nodes)
    }

    pub fn is_file(&self) -> bool {
        match self {
            &LazyTreeNode::FileNode(_) => true,
            _ => false
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            &LazyTreeNode::DirNode(_) => true,
            _ => false
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            &LazyTreeNode::RootNode(_) => true,
            _ => false
        }
    }
}
