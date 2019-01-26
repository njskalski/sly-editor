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

// this is a collection of functions I expect to use in multiple places

use std::path::Path;
use cursive::backend::puppet::observed::ObservedScreen;
use cursive::backend::puppet::observed::ObservedCell;
use cursive::Vec2;
use crossbeam_channel::Receiver;
use core::time::Duration;

/// this method takes a string representing a path, returns pair of options describing
/// folder path and filename. Does not check if file exists, so it cannot differentiate between
/// files and directories, unless path ends with "/", in which case it is assumed it's a directory.
pub fn path_string_to_pair(path_str : String) -> (Option<String>, Option<String>) {
    if path_str.ends_with('/') {
        (Some(path_str[..path_str.len() - 1].to_string()), None)
    } else {
        match path_str.rfind("/") {
            None => (None, Some(path_str)),
            Some(last_slash) => {
                let folder = &path_str[..last_slash];
                let file = &path_str[last_slash + 1..];

                (
                    if folder.len() > 0 { Some(folder.to_string()) } else { None },
                    if file.len() > 0 { Some(file.to_string()) } else { None },
                )
            }
        }
    }
}

pub fn draw_screen(screen : &ObservedScreen) {
    println!("captured screen:");

    print!("   ");
    for x in 0..screen.size().x {
        print!("{}", x % 10);
    }
    println!();
    println!();

    for y in 0..screen.size().y {
        print!("{}   ", y % 10);

        for x in 0..screen.size().x {
            let pos = Vec2::new(x,y);
            let cell_op : &Option<ObservedCell> = &screen[&pos];
            if cell_op.is_some() {
                let cell = cell_op.as_ref().unwrap();

                if cell.letter.is_continuation() {
                    print!("c");
                    continue;
                } else {
                    let letter = cell.letter.unwrap();
                    if letter == " " {
                        print!(" ");
                    } else {
                        print!("{}", letter);
                    }
                }

            } else {
                print!(".");
            }
        }
        println!();
    }
}

pub fn dump_screens(screen_sink : &mut Receiver<ObservedScreen>) {
    while let Ok(screen) = screen_sink.recv_timeout(Duration::new(1, 0)) {
            draw_screen(&screen);
        }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn path_string_to_pair_works() {
        assert_eq!(path_string_to_pair("/bin/".to_string()), (Some("/bin".to_string()), None));
        assert_eq!(
            path_string_to_pair("/bin/tmp".to_string()),
            (Some("/bin".to_string()), Some("tmp".to_string()))
        );
        assert_eq!(
            path_string_to_pair("/bin/tmp/".to_string()),
            (Some("/bin/tmp".to_string()), None)
        );
    }
}
