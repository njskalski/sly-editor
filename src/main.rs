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

// TODO I removed warnings because it took much time to scroll into compile breaking errors.
// However, they should get fixed before beta release, and these directives should be removed.
#![allow(warnings)]
#![allow(unused)]
#![allow(bad_style)]

// TODO(njskalski): when multiple directories are selected, one being an ancestor of another, they
// should get "reduced" (so the ancestor's .gitignore is used for subdirectories).

#[macro_use]
extern crate log;
#[macro_use]
extern crate cursive;
extern crate cursive_tree_view;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;
#[macro_use]
mod utils;
#[macro_use]
mod macros;

mod abstract_clipboard;
mod action;
mod app_state;
mod buffer_id;
mod buffer_index;
mod buffer_state;
mod buffer_state_observer;
mod color_view_wrapper;
mod content_provider;
mod cursor_set;
mod default_settings;
mod dir_tree;
mod events;
mod file_dialog;
mod fuzzy_index;
mod fuzzy_index_trait;
mod fuzzy_query_view;
mod fuzzy_view_item;
mod interface;
mod keyboard_shortcut;
mod lsp_client;
mod overlay_dialog;
mod rich_content;
mod settings;
mod simple_fuzzy_index;
mod sly_text_view;
mod sly_view;
mod test_utils;
mod view_handle;
use dir_tree::TreeNode;

#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
extern crate cpuprofiler;

extern crate clipboard;
extern crate core;
extern crate enumset;
extern crate ignore;
extern crate regex;
extern crate ropey;
extern crate serde_json;
extern crate stderrlog;
extern crate syntect;
extern crate time;
extern crate unicode_segmentation;
extern crate unicode_width;
#[macro_use]
extern crate clap;
extern crate uid;
#[macro_use]
extern crate languageserver_types;
extern crate jsonrpc_core;
#[macro_use]
extern crate human_panic;
extern crate crossbeam_channel;
extern crate ncurses;
extern crate serde;
#[macro_use]
extern crate yaml_rust;
extern crate filesystem;

use app_state::AppState;
use cursive::Cursive;
use filesystem::*;
use interface::Interface;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::env;
use std::fs;
use std::path;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

#[cfg(target_os = "linux")]
use cpuprofiler::PROFILER;
use dir_tree::LazyTreeNode;

#[cfg(target_os = "linux")]
fn start_profiling() {
    let profile_file: String = format!("./sly-{:}.profile", time::now().rfc3339());
    let profile_path: &Path = Path::new(&profile_file);
    if !profile_path.exists() {
        //with timestamp in name this is probably never true
        fs::File::create(&profile_file);
    }
    PROFILER.lock().unwrap().start(profile_file.clone()).unwrap();
}

#[cfg(target_os = "linux")]
fn stop_profiling() {
    PROFILER.lock().unwrap().stop().unwrap();
}

#[cfg(not(target_os = "linux"))]
fn start_profiling() {
    debug!("profiling not available on this platform.");
}

#[cfg(not(target_os = "linux"))]
fn stop_profiling() {}

#[cfg(test)]
pub type FileSystemType = FakeFileSystem;

#[cfg(not(test))]
pub type FileSystemType = OsFileSystem;

fn main() {
    //        setup_panic!();
    stderrlog::new().module(module_path!()).verbosity(5).init().unwrap();

    let yml = clap::load_yaml!("clap.yml");
    let mut app = clap::App::from_yaml(yml)
        .author("Andrzej J Skalski <ajskalski@google.com>")
        .long_version(crate_version!());

    let matches = app.clone().get_matches();

    if matches.is_present("help") {
        app.write_long_help(std::io::stdout().borrow_mut());
        return;
    }

    let profiling_enabled: bool = matches.is_present("profiling") && cfg!(target_os = "linux");
    let git_files_included: bool = matches.is_present("git");

    if profiling_enabled {
        start_profiling();
    }

    let args: Vec<String> = env::args().skip(1).collect();

    let mut directories: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();

    if matches.is_present("files_and_directories") {
        for value in matches.values_of("files_and_directories").unwrap() {
            let path_arg = Path::new(value).to_path_buf();
            let path = match fs::canonicalize(&path_arg) {
                Ok(path) => path,
                _ => {
                    info!("unable to canonicalize \"{:?}\", ignoring.", path_arg);
                    continue;
                }
            };

            if !path.exists() {
                info!("{:?} does not exist, now ignoring.", value);
                continue; // TODO(njskalski) stop ignoring new files.
            }

            if path.is_dir() {
                directories.push(path);
            } else if path.is_file() {
                files.push(path);
            } else {
                info!("{:?} is neither a file nor directory. Ignoring.", value);
            }
        }
    } else {
        // if no directory is specified, we take current directory as "project root".
        match env::current_dir() {
            Ok(path) => directories.push(path),
            Err(e) => debug!("unable to access current directory, because {:?}", e),
        }
    }

    debug!(
        "dirs {:?} \n files {:?}\ngit_files_included = {}",
        &directories, &files, git_files_included
    );

    let dir_file_tree = LazyTreeNode::new(directories.clone(), files.clone()).as_ref();

    let app_state = AppState::new(
        FileSystemType::new(),
        directories,
        files,
        dir_file_tree,
        git_files_included == false,
    );

    let mut siv = Cursive::default();
    let mut interface = Interface::new(app_state, siv);
    interface.main();
    if profiling_enabled {
        stop_profiling();
    };
    debug!("goodbye!");
}
