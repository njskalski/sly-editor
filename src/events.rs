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

use buffer_id::BufferId;
use content_provider;
use serde_json as sj;
use std::path::PathBuf;
use std::sync::mpsc;
use view_handle::ViewHandle;

pub type IChannel = mpsc::Sender<IEvent>;

/// Right now I use single queue for all events, like interface, plugins, language server
/// interaction etc. It might be the way it stays, but for now it's just for the sake of discovery
/// of requirements.

#[derive(Serialize, Deserialize, Debug)]
pub enum IEvent {
    //Interface event
    QuitSly,
    ShowFileBar,
    ShowBufferList,
    ShowSaveAs(BufferId, Option<PathBuf>),
    OpenFileDialog,
    SaveCurrentBuffer,
    SaveBufferAs(BufferId, PathBuf), // sent by file_view
    OpenFile(PathBuf),               // path
    FuzzyQueryBarSelected(String, String), /* marker (the word that search ran agains),
                                      * selection (value) */
    CloseWindow,

    // Buffer edit events are now in the same queue, not sure yet if that's final.
    BufferEditEvent(BufferId, Vec<content_provider::EditEvent>),
    EnableLSP,

    Proto(String), //for quick hacking.
}
