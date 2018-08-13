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

use content_provider::RopeBasedContentProvider;
use content_provider::EditEvent;

use cursive;
use std::fs;
use std::io;

use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

pub enum BufferReadMode {
    ReadOnly,
    ReadWrite
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BufferStateS {
    path : Option<PathBuf>, //unnamed possible, right?
}

pub struct BufferState {
    ss : BufferStateS,
    modified : bool,
    exists : bool,
    mode : BufferReadMode,
    content : RopeBasedContentProvider,
    screen_id : Option<cursive::ScreenId> //no screen no buffer, but can be None after load. TODO fix it later
    // also above will be changed. Multiple buffers will be able to share same screen (so identifier
    // will get longer. I might also implement transferring buffers between instances (for working on multiple screens)
}

impl BufferState {
    pub fn open(file: &String) -> Result<Rc<RefCell<BufferState>>, io::Error> {
        let path = Path::new(file);
        // this also checks for file existence:
        // https://doc.rust-lang.org/std/fs/fn.canonicalize.html
        let canon_path = path.canonicalize()?;

        debug!("reading file {:?}", file);

        if !canon_path.is_file() {
            Err(io::Error::new(io::ErrorKind::InvalidInput, format!("\"{}\" (canonized: \"{:?}\") is not file.", file, canon_path)))
        } else {
            Ok(Rc::new(RefCell::new(BufferState {
                ss : BufferStateS { path : Some(canon_path) },
                modified : false,
                exists : true,
                screen_id : None,
                content : RopeBasedContentProvider::new(Some(&mut path_to_reader(file))),
                mode : BufferReadMode::ReadWrite
            })))
        }
    }

    pub fn set_screen_id(&mut self, screen_id : cursive::ScreenId) {
        self.screen_id = Some(screen_id);
    }

    pub fn get_content(&self) -> &RopeBasedContentProvider {
        &self.content
    }

    pub fn get_screen_id(&self) -> &Option<cursive::ScreenId> {
        &self.screen_id
    }

    pub fn submit_edit_events(&mut self, events: Vec<EditEvent>)
    {
        self.content.submit_events(events);
        self.modified = true; // TODO modified should be moved to history.
    }

    pub fn get_filename(&self) -> Option<OsString> {
        match self.ss.path {
            Some(path) => path.file_name().map(|osstr| osstr.to_os_string()),
            None => None
        }
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        self.ss.path.clone()
    }

    fn proceed_with_save(&mut self, mut file : fs::File) -> Result<(), io::Error> {
        self.content.save(file)
    }

    pub fn save(&mut self, path : Option<String>) -> Result<(), io::Error> {
        if path.is_none() && self.ss.path.is_none() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No path provided."));
        }

        // TODO decide where OsString, where String
        if path == self.ss.path.map(|path| path.to_string_lossy().to_string()) && self.exists && !self.modified {
            info!("Early exit from BufferState.save - file not modified.");
            return Ok(());
        }

        // let final_path = match path {
        //     Some(p) => p.clone(),
        //     None => self.ss.path.unwrap();
        // }

        let final_path : PathBuf = match path {
            Some(ref p) => Path::new(p).to_path_buf(),
            None => self.ss.path.clone().unwrap()
        };

        let mut file = fs::File::create(final_path)?;
        self.proceed_with_save(file)?;

        if path != None {
            self.ss.path = Some(Path::new(&path.unwrap()).to_path_buf())
        }

        self.modified = false;
        self.exists = true;
        debug!("{:?} saved.", &self.ss.path);
        Ok(())
    }
}



fn path_to_reader(path : &String) -> fs::File {
    fs::File::open(path.as_str()).expect(&format!("file {:?} did not exist!", path))
}
