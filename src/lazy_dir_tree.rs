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

use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};
use std::path::PathBuf;
use std::rc::Rc;
use std::fmt;

// TODO(njskalski) add RefCell<Vec<LazyTreeNode>> cache, refresh "on file change"
// TODO(njskalski) add hotloading directories (but remember to keep tests working!)
// TODO(njskalski) create fourth category for out-of-folders files (second argument of constructor).

pub trait TreeNode<T> : fmt::Debug + fmt::Display + Ord where T : TreeNode<T>
{
    fn is_file(&self) -> bool;
    fn is_dir(&self) -> bool;
    fn is_root(&self) -> bool;

    fn children(&self) -> Vec<Rc<T>>;
    fn path(&self) -> Option<PathBuf>;
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum LazyTreeNode {
    RootNode(Vec<Rc<LazyTreeNode>>),
    DirNode(Rc<PathBuf>),
    FileNode(Rc<PathBuf>),
}

impl LazyTreeNode {
    pub fn new(directories : Vec<PathBuf>, files : Vec<PathBuf>) -> Self {
        let mut nodes : Vec<Rc<LazyTreeNode>> = directories
            .into_iter()
            .map(|x| Rc::new(LazyTreeNode::DirNode(Rc::new(x))))
            .chain(files.into_iter().map(|x| Rc::new(LazyTreeNode::FileNode(Rc::new(x)))))
            .collect();
        nodes.sort();
        LazyTreeNode::RootNode(nodes)
    }
}

impl TreeNode for LazyTreeNode {
    fn is_file(&self) -> bool {
        match self {
            &LazyTreeNode::FileNode(_) => true,
            _ => false,
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            &LazyTreeNode::DirNode(_) => true,
            _ => false,
        }
    }

    fn is_root(&self) -> bool {
        match self {
            &LazyTreeNode::RootNode(_) => true,
            _ => false,
        }
    }

    fn children(&self) -> Vec<Rc<LazyTreeNode>> {
        match self {
            &LazyTreeNode::RootNode(ref children) => vec![],
            &LazyTreeNode::DirNode(ref path) => {
                //                let path = Path::new(&**p);
                let mut contents : Vec<LazyTreeNode> = Vec::new();
                for dir_entry in path.read_dir().expect("read_dir call failed.") {
                    if let Ok(entry) = dir_entry {
                        if let Ok(meta) = entry.metadata() {
                            if
                            /* files_visible && */
                            meta.is_file() {
                                contents.push(LazyTreeNode::FileNode(Rc::new(entry.path())));
                            } else if meta.is_dir() {
                                contents.push(LazyTreeNode::DirNode(Rc::new(entry.path())));
                            }
                        }
                    }
                }
                contents.map(|x| Rc::new(x))
            }
            &LazyTreeNode::FileNode(ref path) => vec![],
        }
    }

    fn path(&self) -> Option<PathBuf> {
        match self {
            &LazyTreeNode::RootNode(_) => None,
            &LazyTreeNode::DirNode(ref path) => Some((**path).clone()),
            &LazyTreeNode::FileNode(ref path) => Some((**path).clone()),
        }
    }
}

impl fmt::Display for LazyTreeNode {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LazyTreeNode::RootNode(_) => write!(f, "<root>"),
            &LazyTreeNode::DirNode(ref path) => {
                write!(f, "{}", path.file_name().unwrap().to_string_lossy())
            }
            &LazyTreeNode::FileNode(ref path) => {
                write!(f, "{}", path.file_name().unwrap().to_string_lossy())
            }
        }
    }
}