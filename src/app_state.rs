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
//      there will be no office-like "unsaved versions"
// - plugin states are to be lost in first versions
// - I am heading to MVP.

use serde_json;
use std::sync::Arc;
use std::path::Path;
use std::fs;
use ignore::gitignore;
use std::env;

use fuzzy_index::FuzzyIndex;
use fuzzy_view_item::file_list_to_items;

use std::cell::{RefCell, Ref};
use std::rc::Rc;
use std::collections::HashMap;
use content_provider::RopeBasedContentProvider;
use content_provider;
use cursive;
use std::io;

use lazy_dir_tree::LazyTreeNode;


pub enum BufferOpenMode {
    ReadOnly,
    ReadWrite
}

#[derive(Debug, Serialize, Deserialize)]
struct BufferStateS {
    path : Option<String>, //unnamed possible, right?
}

pub struct BufferState {
    ss : BufferStateS,
    modified : bool,
    exists : bool,
    mode : BufferOpenMode,
    content : RopeBasedContentProvider,
    screen_id : Option<cursive::ScreenId> //no screen no buffer, but can be None after load. TODO fix it later
    // also above will be changed. Multiple buffers will be able to share same screen (so identifier
    // will get longer. I might also implement transferring buffers between instances (for working on multiple screens)
}

impl BufferState {
    pub fn submit_edit_events(&mut self, events: Vec<content_provider::EditEvent>)
    {
        self.content.submit_events(events);
        self.modified = true; // TODO modified should be moved to history.
    }

    pub fn get_path_ref(&self) -> &Option<String> {
        &self.ss.path
    }
}

pub struct BufferStateObserver {
    buffer_state : Rc<RefCell<BufferState>>,
}

impl BufferStateObserver {
    fn new(buffer_state : Rc<RefCell<BufferState>>) -> Self {
        BufferStateObserver{ buffer_state : buffer_state }
    }

    /// borrows unmutably content
    pub fn content(&self) -> Ref<RopeBasedContentProvider> {
        Ref::map(self.buffer_state.borrow(), |x| &x.content)
    }

    pub fn is_loaded(&self) -> bool {
        self.buffer_state.borrow().screen_id.is_some()
    }

    pub fn get_screen_id(&self) -> cursive::ScreenId {
        self.buffer_state.borrow().screen_id.unwrap()
    }

    pub fn get_path(&self) -> Option<String> {
        self.buffer_state.borrow().ss.path.clone()
    }
}

pub struct AppState {
    // Map of buffers that has been loaded into memory AND assigned a ScreenID.
    loaded_buffers : HashMap<cursive::ScreenId, Rc<RefCell<BufferState>>>,

    /// List of buffers that have been loaded into memory, but yet have no assigned ScreenIDs
    buffers_to_load : Vec<Rc<RefCell<BufferState>>>,

    index : Arc<RefCell<FuzzyIndex>>, //because searches are mutating the cache TODO this can be solved with "interior mutability", as other caches in this app
    dir_tree: Rc<LazyTreeNode>,
}

fn path_to_reader(path : &String) -> fs::File {
    fs::File::open(path.as_str()).expect(&format!("file {:?} did not exist!", path))
}

impl AppState{

    pub fn get_buffer_for_screen(&mut self, screen_id : &cursive::ScreenId) -> Option<Rc<RefCell<BufferState>>> {
        self.loaded_buffers.get(screen_id).map(|x| x.clone())
    }

    pub fn get_file_index(&self) -> Arc<RefCell<FuzzyIndex>> {
        self.index.clone()
    }

    pub fn get_dir_tree(&self) -> Rc<LazyTreeNode> {
        self.dir_tree.clone()
    }

    // TODO(njskalski) this interface is temporary
    pub fn submit_edit_events_to_buffer(&mut self, screen_id : cursive::ScreenId, events : Vec<content_provider::EditEvent>) {
        self.loaded_buffers[&screen_id].borrow_mut().submit_edit_events(events);
    }

    /// this loads TO SCREEN, not to memory.
    pub fn has_buffers_to_load(&self) -> bool {
        self.buffers_to_load.len() > 0
    }

    pub fn get_buffer_observer(&self, screen_id : &cursive::ScreenId) -> Option<BufferStateObserver> {
        self.loaded_buffers.get(screen_id).map(|x| BufferStateObserver::new(x.clone()))
    }

    pub fn schedule_file_for_load(&mut self, file : &String) {
        self.buffers_to_load.push(path_to_buffer_state(file));
    }

    // This method takes first buffer scheduled for load and assigns it a ScreenId.
    pub fn load_buffer(&mut self, screen_id : cursive::ScreenId) -> BufferStateObserver {
        assert!(!self.loaded_buffers.contains_key(&screen_id));
        let mut buffer = self.buffers_to_load.pop().unwrap();
        buffer.borrow_mut().screen_id = Some(screen_id);
        self.loaded_buffers.insert(screen_id, buffer);
        self.get_buffer_observer(&screen_id).unwrap()
    }



    pub fn new(directories : Vec<String>, files : Vec<String>) -> Self {

        let mut file_index : Vec<String> = Vec::new();
        let mut canonized_directories : Vec<String> = Vec::new();

        for directory in directories {

            let path = Path::new(&directory);
            match path.canonicalize() {
                Ok(canon_path) => {
                    debug!("reading directory: {:?}", canon_path.to_string_lossy());
                    canonized_directories.push(canon_path.to_string_lossy().to_string());
                    build_file_index(&mut file_index, &canon_path, true, None);
                },
                _ => error!("unable to read directory {:?}", directory)
            }
        }

        let dir_tree = LazyTreeNode::new(&canonized_directories);

        let buffers : Vec<_> = files.iter().map(|file| {
            Rc::new(RefCell::new(path_to_buffer_state(file)))
        }).collect();

        let file_index_items = file_list_to_items(&file_index);

        AppState {
            buffers_to_load : buffers,
            loaded_buffers : HashMap::new(),
            index : Arc::new(RefCell::new(FuzzyIndex::new(file_index_items))),
            dir_tree : Rc::new(dir_tree)
        }
    }

    fn empty() -> Self {
        Self::new(Vec::new(), Vec::new())
    }
}

fn path_to_buffer_state(file : &String) -> BufferState {
    let path = Path::new(file);

    // this also checks for file existence:
    // https://doc.rust-lang.org/std/fs/fn.canonicalize.html
    let canon_path = path.canonicalize();

    if canon_path.is_ok() {
        debug!("reading file {:?}", file);
        BufferState {
            ss : BufferStateS { path : Some(canon_path.unwrap().to_string_lossy().to_string()) },
            modified : false,
            exists : true,
            screen_id : None,
            content : RopeBasedContentProvider::new(Some(&mut path_to_reader(file))),
            mode : BufferOpenMode::ReadWrite
        }
    } else {
        let mut current_dir = env::current_dir().unwrap();
        // join semantics is interesting and it does exactly what I want: if new filename
        // does not have an absolute path, it attaches it to current_dir. If it does define
        // absolute path, the current_dir part is dropped.
        // see https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.push
        current_dir.join(path);

        BufferState {
            ss : BufferStateS { path : Some(current_dir.to_string_lossy().to_string()) },
            modified : false,
            exists : false,
            screen_id : None,
            content : RopeBasedContentProvider::new(None),
            mode : BufferOpenMode::ReadWrite
        }
    }
}

/// this method takes into account .git and other directives set in .gitignore. However it only takes into account most recent .gitignore
fn build_file_index(mut index : &mut Vec<String>, dir : &Path, enable_gitignore : bool, gi_op : Option<&gitignore::Gitignore>) {
    match fs::read_dir(dir) {
        Ok(read_dir) => {
            let gitignore_op : Option<gitignore::Gitignore> = if enable_gitignore {
                let pathbuf = dir.join(Path::new("/.gitignore"));
                let gitignore_path = pathbuf.as_path();
                if gitignore_path.exists() && gitignore_path.is_file() {
                    let (gi, error_op) = gitignore::Gitignore::new(&gitignore_path);
                    if let Some(error) = error_op {
                        info!("Error while parsing gitignore file {:?} : {:}", gitignore_path, error);
                    }
                    Some(gi)
                } else { None }
            } else { None };

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
                                if gitignore.matched(path, false).is_ignore() { continue };
                            };
                            match path.to_str() {
                                Some(s) => index.push(s.to_string()),
                                None => error!("unable to parse non-unicode file path: \"{:?}\". Skipping.", path)
                            };
                        } else {
                            if let Some(ref gitignore) = &gitignore_op {
                                if gitignore.matched(path, true).is_ignore() { continue };
                            };

                            let most_recent_gitignore = if gitignore_op.is_some() { gitignore_op.as_ref() } else { gi_op };
                            build_file_index(&mut index, &path, enable_gitignore, most_recent_gitignore);
                        }
                    },
                    Err(e) => error!("error listing directory \"{:?}\": {:?}. Skipping.", dir, e)
                } //match
            } //for
        },
        Err(e) => warn!("unable to open dir \"{:?}\".", dir)
    }
}
