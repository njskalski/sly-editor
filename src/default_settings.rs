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

// TODO(njskalski) use borrowed instead of copied type

use std::io::Read;

const default_settings_string: &'static str = r####"
{
  "keybindings" : {
    "text" : {
      "copy" : ["ctrl","c"],
      "paste" : ["ctrl","v"],
      "undo" : ["ctrl","z"],
      "redo" : ["ctrl","Z"]
    },
    "text_view" : {
      "toggle_syntax_highlighting" : ["ctrl","h"]
    },
    "global" : {
      "show_file_bar" : ["ctrl", "p"],
      "show_buffer_list" : ["ctrl", "o"],
      "command_mode" : ["ctrl", "e"],
      "quit" : ["ctrl", "q"],
      "close_window" : ["esc"],
      "save" : ["ctrl", "s"],
      "save_as" : ["ctrl","w"],
      "open_file_dialog" : ["ctrl", "u"],
      "start_lsp" : ["ctrl", "g"]
    },
    "file_bar" : {
    }
  },
  "performance" : {
    "auto_highlighting" : true,
    "max_files_indexed" : 1000
  },
  "theme" : {
    "text_view" : {
      "background_color" : "#1d1d1d",
      "primary_text_color" : "#e5e5e5",
      "secondary_text_color" : "#7f7f7f"
    },
    "file_view" :{
      "non_selected_background" : "#282C34",
      "selected_background" : "#303540",
      "primary_text_color" : "#e5e5e5"
    },
    "fuzzy_view" : {
      "primary_text_color" : "#e5e5e5",
      "secondary_text_color" : "#7f7f7f",
      "highlighted_text_color" : "#559bd4",
      "background_color" : "#2e2e2e",
      "selected_background_color" : "#1d1d1d"
    }
  }
}
"####;

pub fn get_default_settings() -> String {
    default_settings_string.to_string()
}
