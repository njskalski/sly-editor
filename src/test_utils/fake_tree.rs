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

use std::convert::Into;
use dir_tree::TreeNodeVec;
use std::path::PathBuf;
use std::fmt;
use dir_tree::TreeNode;
use std::path::Path;
use std::rc::Rc;
use dir_tree::TreeNodeRef;

#[derive(Debug, Clone)]
pub enum FakeTreeNode {
    FakeRoot(TreeNodeVec),
    FakeDir(PathBuf, TreeNodeVec),
    FakeFile(PathBuf),
}

impl FakeTreeNode {}

pub fn fake_root(children: TreeNodeVec) -> TreeNodeRef {
    FakeTreeNode::FakeRoot(children).as_ref()
}

pub fn fake_dir(s: &str, children: TreeNodeVec) -> TreeNodeRef {
    FakeTreeNode::FakeDir(s.into(), children).as_ref()
}

pub fn fake_file(s: &str) -> TreeNodeRef {
    FakeTreeNode::FakeFile(s.into()).as_ref()
}

impl fmt::Display for FakeTreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &FakeTreeNode::FakeRoot(_) => write!(f, "<root>"),
            &FakeTreeNode::FakeDir(ref path, _) => {
                write!(f, "{}", path.file_name().unwrap().to_string_lossy())
            }
            &FakeTreeNode::FakeFile(ref path) => {
                write!(f, "{}", path.file_name().unwrap().to_string_lossy())
            }
        }
    }
}

impl TreeNode for FakeTreeNode {
    fn is_file(&self) -> bool {
        match self {
            &FakeTreeNode::FakeFile(_) => true,
            _ => false,
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            &FakeTreeNode::FakeDir(_, _) => true,
            _ => false,
        }
    }

    fn is_root(&self) -> bool {
        match self {
            &FakeTreeNode::FakeRoot(_) => true,
            _ => false,
        }
    }

    fn children(&self) -> Vec<Rc<Box<TreeNode>>> {
        match self {
            &FakeTreeNode::FakeRoot(ref v) => v.clone(),
            &FakeTreeNode::FakeDir(_, ref v) => v.clone(),
            &FakeTreeNode::FakeFile(_) => vec![],
        }
    }

    fn path(&self) -> Option<&Path> {
        match self {
            &FakeTreeNode::FakeRoot(_) => None,
            &FakeTreeNode::FakeDir(ref path, _) => Some(path),
            &FakeTreeNode::FakeFile(ref path) => Some(path),
        }
    }

    fn has_children(&self) -> bool {
        !self.children().is_empty()
    }

    fn as_ref(self) -> TreeNodeRef {
        Rc::new(Box::new(self))
    }
}