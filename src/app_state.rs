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

// this data corresponds to application state.
// a lot of it is internal, and it is not designed to be fully dumped.
// I am however putting some of obviously serializable part states in *S structs.

// Some design decisions:
// - history of file is not to be saved. This is not a versioning system.
// - if editor is closed it asks whether to save changes. If told not to, the changes are lost.
//   there will be no office-like "unsaved versions"
// - plugin states are to be lost in first versions
// - I am heading to MVP.

use ignore::gitignore;
use serde_json;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use buffer_state::BufferOpenMode;
use buffer_state::BufferState;
use buffer_state::BufferStateS;
use buffer_state_observer::BufferStateObserver;
use fuzzy_index::FuzzyIndex;
use fuzzy_index_trait::FuzzyIndexTrait;
use fuzzy_view_item::file_list_to_items;

use content_provider;
use content_provider::RopeBasedContentProvider;
use cursive;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::error;
use std::io;
use std::io::Write;
use std::rc::Rc;

use buffer_id::BufferId;
use buffer_state::ExistPolicy;
use core::borrow::Borrow;
use lazy_dir_tree::LazyTreeNode;
use std::cell::Cell;
use std::collections::VecDeque;
use std::io::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use view_handle::ViewHandle;

pub struct AppState {
    buffers_to_load : VecDeque<PathBuf>,

    index : Arc<RefCell<FuzzyIndex>>, /* because searches are mutating the cache TODO this can
                                       * be solved with
                                       * "interior mutability", as other caches in this
                                       * app */
    dir_and_files_tree :     Rc<LazyTreeNode>,
    get_first_buffer_guard : Cell<bool>,
    directories :            Vec<PathBuf>, /* it's a straigthforward copy of arguments used
                                            * to guess "workspace" parameter for languageserver */
    loaded_buffers : HashMap<BufferId, Rc<RefCell<BufferState>>>,
}

impl AppState {
    /// Returns list of buffers. Rather stable.
    pub fn get_buffers(&self) -> Vec<BufferId> {
        self.loaded_buffers.keys().map(|k| k.clone()).collect()
    }

    /// Returns list of BufferIds associated with given path.
    /// Complexity: O(n), can be optimised later.
    fn get_buffers_for_path(&self, lookup_path : &Path) -> Vec<BufferId> {
        let mut result : Vec<BufferId> = Vec::new();
        for (buffer_id, buffer_state) in &self.loaded_buffers {
            if (**buffer_state).borrow().get_path().map(|state_path| state_path.eq(lookup_path))
                == Some(true)
            {
                result.push(buffer_id.clone());
            }
        }
        result
    }

    /// Returns file index. Rather stable.
    pub fn get_file_index(&self) -> Arc<RefCell<FuzzyIndexTrait>> {
        self.index.clone()
    }

    pub fn get_dir_tree(&self) -> Rc<LazyTreeNode> {
        self.dir_and_files_tree.clone()
    }

    pub fn schedule_file_for_load(&mut self, file_path : PathBuf) {
        self.buffers_to_load.push_back(file_path);
    }

    pub fn buffer_obs(&self, id : &BufferId) -> Option<BufferStateObserver> {
        self.loaded_buffers.get(id).map(|b| BufferStateObserver::new(b.clone()))
    }

    pub fn save_buffer_as(&mut self, id : &BufferId, path : PathBuf) -> Result<(), io::Error> {
        let buffer_ptr = self.loaded_buffers.get(id).unwrap();
        let mut buffer = (**buffer_ptr).borrow_mut();
        buffer.save(Some(path))
    }

    /// As of this time, it does not re-open file that is already opened, just returns buffer id
    /// instead.
    pub fn open_or_get_file(&mut self, path : &Path) -> Result<BufferId, io::Error> {
        let buffers = self.get_buffers_for_path(path);
        if buffers.is_empty() {
            self.open_file(path)
        } else {
            if buffers.len() > 1 {
                debug!(
                    "Returning first of {} buffers corresponding to path {:?}",
                    buffers.len(),
                    &path
                );
            }
            Ok(buffers.first().unwrap().clone())
        }
    }

    /// Opens file, not checking if a file is opened in another buffer. Intentionally private.
    fn open_file(&mut self, path : &Path) -> Result<BufferId, io::Error> {
        // TODO(njskalski): add delayed load (promise)
        let buffer = BufferState::open(path, ExistPolicy::MustExist)?;
        let id = (*buffer).borrow().id();
        self.loaded_buffers.insert(id.clone(), buffer);
        Ok(id)
    }

    /// This method is called while constructing interface, to determine content of first edit view.
    pub fn get_first_buffer(&mut self) -> Result<BufferStateObserver, io::Error> {
        if self.get_first_buffer_guard.get() {
            panic!("secondary call to app_state::get_first_buffer!");
        }
        self.get_first_buffer_guard.set(true);

        let buffer : Rc<RefCell<BufferState>> = if self.buffers_to_load.is_empty() {
            /// if there is no buffer to load, we create an unnamed one.
            BufferState::new()
        } else {
            let file_path = self.buffers_to_load.pop_front().unwrap();
            BufferState::open(&file_path, ExistPolicy::CanExist)?
        };

        let id = (*buffer).borrow().id();
        self.loaded_buffers.insert(id.clone(), buffer);

        Ok(self.buffer_obs(&id).unwrap())
    }

    pub fn new(directories : Vec<PathBuf>, files : Vec<PathBuf>, enable_gitignore : bool) -> Self {
        let mut files_to_index : Vec<PathBuf> = files.to_owned();
        debug!("enable_gitignore == {}", enable_gitignore);
        for dir in &directories {
            build_file_index(&mut files_to_index, dir, enable_gitignore, None);
        }

        //        debug!("file index:\n{:?}", &files_to_index);

        let file_index_items = file_list_to_items(&files_to_index);
        let buffers_to_load : VecDeque<PathBuf> = files.iter().map(|x| x.clone()).collect();

        AppState {
            buffers_to_load :        buffers_to_load,
            loaded_buffers :         HashMap::new(),
            index :                  Arc::new(RefCell::new(FuzzyIndex::new(file_index_items))),
            dir_and_files_tree :     Rc::new(LazyTreeNode::new(directories.clone(), files)),
            get_first_buffer_guard : Cell::new(false),
            directories :            directories,
        }
    }

    pub fn directories(&self) -> &Vec<PathBuf> {
        &self.directories
    }
}

/// this method takes into account .git and other directives set in .gitignore. However it only
/// takes into account most recent .gitignore
fn build_file_index(
    mut index : &mut Vec<PathBuf>,
    dir : &Path,
    enable_gitignore : bool,
    gi_op : Option<&gitignore::Gitignore>,
) {
    match fs::read_dir(dir) {
        Ok(read_dir) => {
            let gitignore_op : Option<gitignore::Gitignore> = if enable_gitignore {
                let pathbuf = dir.join(Path::new(".gitignore"));
                let gitignore_path = pathbuf.as_path();
                if gitignore_path.exists() && gitignore_path.is_file() {
                    let (gi, error_op) = gitignore::Gitignore::new(&gitignore_path);
                    if let Some(error) = error_op {
                        info!(
                            "Error while parsing gitignore file {:?} : {:}",
                            gitignore_path, error
                        );
                    }
                    Some(gi)
                } else {
                    None
                }
            } else {
                None
            };

            for entry_res in read_dir {
                match entry_res {
                    Ok(entry) => {
                        let path_buf = entry.path();
                        let path = path_buf.as_path();

                        if enable_gitignore {
                            if path.ends_with(Path::new(".git")) {
                                return;
                            }
                        }

                        if path.is_file() {
                            if let Some(ref gitignore) = &gitignore_op {
                                if gitignore.matched(path, false).is_ignore() {
                                    continue;
                                };
                            };
                            index.push(path.to_path_buf()); //TODO(njskalski): move instead of copy.
                        } else {
                            if let Some(ref gitignore) = &gitignore_op {
                                if gitignore.matched(path, true).is_ignore() {
                                    continue;
                                };
                            };

                            let most_recent_gitignore =
                                if gitignore_op.is_some() { gitignore_op.as_ref() } else { gi_op };
                            build_file_index(
                                &mut index,
                                &path,
                                enable_gitignore,
                                most_recent_gitignore,
                            );
                        }
                    }
                    Err(e) => error!("error listing directory \"{:?}\": {:?}. Skipping.", dir, e),
                } //match
            } //for
        }
        Err(e) => warn!("unable to open dir \"{:?}\".", dir),
    }
}
