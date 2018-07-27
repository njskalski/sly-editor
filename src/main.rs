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

#[macro_use]
extern crate log;
#[macro_use]
extern crate cursive;
extern crate cursive_tree_view;
#[macro_use]
extern crate serde_derive;

mod fuzzy_index;
mod fuzzy_index_trait;
mod fuzzy_query_view;
mod fuzzy_view_item;
mod syntax;
mod content_type;
mod sly_text_view;
mod content_provider;
mod default_settings;
mod settings;
mod app_state;
mod interface;
mod events;
mod file_view;
mod lazy_dir_tree;
mod color_view_wrapper;
mod utils;

extern crate ignore;
extern crate cpuprofiler;
extern crate serde_json;
extern crate unicode_segmentation;
extern crate unicode_width;
extern crate time;
extern crate ropey;
extern crate clipboard;
extern crate regex;
extern crate stderrlog;
extern crate syntect;
extern crate core;
extern crate enumset;

use std::env;
use std::fs;
use std::rc::Rc;
use std::path::Path;
use app_state::AppState;
use interface::Interface;
use cpuprofiler::PROFILER;
use std::path;

// Reason for it being string is that I want to be able to load filelists from remote locations
fn get_file_list_from_dir(path: &Path) -> Vec<String> {
    let mut file_list: Vec<String> = Vec::new();

    let paths = fs::read_dir(path).unwrap();

    for p in paths {
        match p {
            Ok(dir_entry) => {
                let inner_path = dir_entry.path();
                if inner_path.is_dir() {
                    file_list.append(get_file_list_from_dir(&inner_path).as_mut());
                }
                if inner_path.is_file() {
                    file_list.push(inner_path.to_str().unwrap().to_string());
                }
            }
            Err(e) => {
                debug!("not able to process DirEntry: {:?}", e);
            }
        }
    }
    file_list
}

fn main() {
    stderrlog::new()
        .module(module_path!())
        .verbosity(5)
        .init()
        .unwrap();

    // TODO(njskalski) use proper input parsing library

    let profiling_enabled : bool = {
        let profile_directory_path = Path::new("./profiles");
        if profile_directory_path.exists() && profile_directory_path.is_dir() {
            true
        } else { false }
    };

    if profiling_enabled {
        let profile_file : String = format!("./profiles/sly-{:}.profile", time::now().rfc3339());
        let profile_path : &Path = Path::new(&profile_file);
        if !profile_path.exists() { //with timestamp in name this is probably never true
            fs::File::create(&profile_file);
        }
        PROFILER.lock().unwrap().start(profile_file.clone()).unwrap();
    };

    let args: Vec<String> = env::args().skip(1).collect();
    let mut commandline_args : Vec<String> = vec![];

    let mut directories : Vec<String> = Vec::new();
    let mut files : Vec<String> = Vec::new();

    for mut arg in args {
        if arg.starts_with("--") {
            commandline_args.push(arg);
        } else {
            let (exists, is_directory, is_file) = {
                let path = Path::new(&arg);
                (path.exists(), path.is_dir(), path.is_file())
            };

            //removing tailing slashes (for some reasons rust's path allow them)
            while arg.len() > 1 && arg.chars().last().unwrap() == '/' {
                arg = arg.as_str()[0..arg.len()-1].to_string()
            };

            if !exists {
                info!("{:?} does not exist.", arg);
            }

            if is_directory {
                directories.push(arg);
            } else if is_file {
                files.push(arg);
            } else {
                info!("{:?} is neither a file nor directory. Ignoring.", arg);
            }
        }
    }

    let app_state = AppState::new(directories, files);

    {
        for arg in &commandline_args {
            debug!("not supported argument \"{:?}\"", arg);
        }
    }

    let mut interface = Interface::new(app_state);
    interface.run();
    if profiling_enabled {
        PROFILER.lock().unwrap().stop().unwrap();
    };
    debug!("goodbye!");
}
