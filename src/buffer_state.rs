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

use content_provider::EditEvent;
use content_provider::RopeBasedContentProvider;
use view_handle::ViewHandle;

use cursive;
use std::fs;
use std::io;

use std::cell::RefCell;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use buffer_id::BufferId;
use buffer_state_observer::BufferStateObserver;
use std::borrow::Borrow;
use utils::highlight_settings_from_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferOpenMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExistPolicy {
    MustExist,
    CanExist,
    MustNotExist,
}

/// This struct represents serializable part of BufferState.
#[derive(Debug, Serialize, Deserialize)]
pub struct BufferStateS {
    /// Path can be None. This represents a buffer which has no file name set.
    path: Option<PathBuf>,
}

pub struct BufferState {
    id: BufferId,
    ss: BufferStateS,
    modified: bool,
    mode: BufferOpenMode,
    content: RopeBasedContentProvider,
}

impl BufferState {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(BufferState {
            id: BufferId::new(),
            ss: BufferStateS { path: None },
            modified: false,
            content: RopeBasedContentProvider::new(None, None),
            mode: BufferOpenMode::ReadWrite,
        }))
    }

    pub fn modified(&self) -> bool {
        self.modified
    }

    pub fn id(&self) -> BufferId {
        self.id.clone()
    }

    pub fn open(
        file_path: &Path,
        creation_policy: ExistPolicy,
    ) -> Result<Rc<RefCell<Self>>, io::Error> {
        debug!("reading file {:?}, creation_policy = {:?}", file_path, creation_policy);

        if !file_path.exists() && creation_policy == ExistPolicy::MustExist {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("\"{:?}\" not found, and required", &file_path),
            ));
        }

        if file_path.exists() && creation_policy == ExistPolicy::MustNotExist {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("\"{:?}\" found and required not to be there.", &file_path),
            ));
        }

        let mut reader: fs::File = path_to_reader(&file_path);
        let highlight_settings_op = highlight_settings_from_path(file_path);

        Ok(Rc::new(RefCell::new(BufferState {
            id: BufferId::new(),
            ss: BufferStateS { path: Some(file_path.to_owned()) },
            modified: false,
            content: RopeBasedContentProvider::new(Some(&mut reader), highlight_settings_op),
            mode: BufferOpenMode::ReadWrite,
        })))
    }

    pub fn get_content(&self) -> &RopeBasedContentProvider {
        &self.content
    }

    pub fn get_content_mut(&mut self) -> &mut RopeBasedContentProvider {
        &mut self.content
    }

    pub fn submit_edit_events(&mut self, events: Vec<EditEvent>) {
        self.content.submit_events(events);
        self.modified = true; // TODO modified should be moved to history.
    }

    pub fn get_filename(&self) -> Option<OsString> {
        match self.ss.path {
            Some(ref path) => path.file_name().map(|osstr| osstr.to_os_string()),
            None => None,
        }
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        self.ss.path.clone()
    }

    fn proceed_with_save(&mut self, mut file: fs::File) -> Result<(), io::Error> {
        self.content.save(file)
    }

    /// Returns whether file exists. File with no path obviously does not.
    pub fn exists(&self) -> bool {
        self.get_path().map_or(false, |path| path.exists())
    }

    pub fn save(&mut self, path: Option<PathBuf>) -> Result<(), io::Error> {
        if path.is_none() && self.ss.path.is_none() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No path provided."));
        }

        if path == self.ss.path && self.exists() && !self.modified {
            info!("Early exit from BufferState.save - file not modified.");
            return Ok(());
        }

        let final_path: PathBuf = match path {
            Some(p) => p,
            None => self.get_path().unwrap(),
        };

        let mut file = fs::File::create(&final_path)?;
        self.proceed_with_save(file)?;

        self.ss.path = Some(final_path);

        self.modified = false;
        debug!("{:?} saved.", &self.ss.path);
        Ok(())
    }
}

fn path_to_reader(path: &Path) -> fs::File {
    fs::File::open(path).expect(&format!("file {:?} did not exist!", path))
}
