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

use serde_json as sj;
use cursive;
use content_provider;


#[derive(Serialize, Deserialize, Debug)] // ready for json-rpc!
pub enum IEvent { //Interface event
    QuitSly,
    ShowFileBar,
    ShowBufferBar,
    ShowSaveAs, // TODO add buffer, default filename etc.
    SaveBuffer, // TODO add buffer
    SaveBufferAs(String), // TODO add buffer
    FuzzyQueryBarSelected(String, String), // marker (the word that search ran agains), selection (value)
    CloseWindow,

    // Buffer edit events are now in the same queue, not sure yet if that's final.
    BufferEditEvent(cursive::ScreenId, Vec<content_provider::EditEvent>),

    Proto(String), //for quick hacking.
}
