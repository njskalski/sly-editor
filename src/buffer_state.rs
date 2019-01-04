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
use view_handle::ViewHandle;

use cursive;
use std::fs;
use std::io;

use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use std::borrow::Borrow;

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
    view_handle : Option<ViewHandle>, //no screen no buffer, but can be None after load. TODO fix it later
}

impl BufferState {
    pub fn open(file_path: PathBuf) -> Result<Rc<RefCell<BufferState>>, io::Error> {
        debug!("reading file {:?}", file_path);

        // TODO(njskalski) add support to new files here?
        if !file_path.exists() {
            Err(io::Error::new(io::ErrorKind::InvalidInput, format!("\"{:?}\" not found.", &file_path)))
        } else if !file_path.is_file() {
            Err(io::Error::new(io::ErrorKind::InvalidInput, format!("\"{:?}\" is not file.", &file_path)))
        } else {
            let mut reader : fs::File = path_to_reader(&file_path);
            Ok(Rc::new(RefCell::new(BufferState {
                ss : BufferStateS { path : Some(file_path) },
                modified : false,
                exists : true,
                view_handle : None,
                content : RopeBasedContentProvider::new(Some(&mut reader)),
                mode : BufferReadMode::ReadWrite,
            })))
        }
    }

    pub fn set_view_handle(&mut self, view_handle : ViewHandle) {
        self.view_handle = Some(view_handle);
    }

    pub fn get_content(&self) -> &RopeBasedContentProvider {
        &self.content
    }

    pub fn get_content_mut(&mut self) -> &mut RopeBasedContentProvider {
        &mut self.content
    }

    pub fn get_view_handle(&self) -> &Option<ViewHandle> {
        &self.view_handle
    }

    pub fn submit_edit_events(&mut self, events: Vec<EditEvent>)
    {
        self.content.submit_events(events);
        self.modified = true; // TODO modified should be moved to history.
    }

    pub fn get_filename(&self) -> Option<OsString> {
        match self.ss.path {
            Some(ref path) => path.file_name().map(|osstr| osstr.to_os_string()),
            None => None
        }
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        self.ss.path.clone()
    }

    fn proceed_with_save(&mut self, mut file : fs::File) -> Result<(), io::Error> {
        self.content.save(file)
    }

    pub fn save(&mut self, path : Option<PathBuf>) -> Result<(), io::Error> {
        if path.is_none() && self.ss.path.is_none() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No path provided."));
        }

        if path == self.ss.path && self.exists && !self.modified {
            info!("Early exit from BufferState.save - file not modified.");
            return Ok(());
        }

        let final_path: PathBuf = match path {
            Some(p) => p,
            None => self.get_path().unwrap()
        };

        let mut file = fs::File::create(&final_path)?;
        self.proceed_with_save(file)?;

        self.ss.path = Some(final_path);

        self.modified = false;
        self.exists = true;
        debug!("{:?} saved.", &self.ss.path);
        Ok(())
    }
}

fn path_to_reader(path : &Path) -> fs::File {
    fs::File::open(path).expect(&format!("file {:?} did not exist!", path))
}
