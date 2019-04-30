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

use crossbeam_channel;
use cursive::backend::puppet::observed::ObservedCell;
use cursive::backend::puppet::observed::ObservedScreen;
use cursive::backend::puppet::observed_screen_view::ObservedScreenView;
use cursive::Cursive;
use file_dialog::FileDialog;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use buffer_id::BufferId;
use cursive::event::Key;
use file_dialog::FileDialogVariant;
use std::path::Path;
use test_utils::basic_setup::tests::BasicSetup;
use utils::is_prefix_of;

#[test]
fn is_prefix_of_test() {
    assert_eq!(is_prefix_of(Path::new("/"), Path::new("/home/nj/sly/editor")), true);
    assert_eq!(is_prefix_of(Path::new("/home/"), Path::new("/home/nj/sly/editor")), true);
    assert_eq!(is_prefix_of(Path::new("/home/nj"), Path::new("/home/nj/sly/editor")), true);
    assert_eq!(is_prefix_of(Path::new("/home/nj/sly"), Path::new("/home/nj/sly/editor")), true);
    assert_eq!(
        is_prefix_of(Path::new("/home/nj/sly/editor"), Path::new("/home/nj/sly/editor")),
        true
    );
    assert_eq!(is_prefix_of(Path::new("/home/njs/sly"), Path::new("/home/nj/sly/editor")), false);
    assert_eq!(is_prefix_of(Path::new("/etc/"), Path::new("/home/nj/sly/editor")), false);
}

fn basic_setup(variant: FileDialogVariant) -> BasicSetup {
    BasicSetup::new(move |settings, ichannel| {
        FileDialog::new(
            ichannel,
            variant,
            settings.filesystem().clone(),
            settings.settings().clone(),
        )
    })
}

#[test]
fn displays_open_file_header() {
    let mut s = basic_setup(FileDialogVariant::OpenFile(None));
    let mut screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("Open file").len(), 1);
    assert_eq!(screen.find_occurences("Save file").len(), 0);
}

#[test]
fn displays_save_file_header() {
    let mut s = basic_setup(FileDialogVariant::SaveAsFile(BufferId::new(), None, None));
    let screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("Open file").len(), 0);
    assert_eq!(screen.find_occurences("Save file").len(), 1);
}

#[test]
fn displays_root() {
    let mut s = basic_setup(FileDialogVariant::OpenFile(None));
    let screen = s.last_screen().unwrap();

    let hits = screen.find_occurences("▸");
    assert_eq!(hits.len(), 1);

    let hit = hits.first().unwrap().expanded_line(0, 7);
    assert_eq!(hit.to_string(), "▸ <root>".to_owned());
}

#[test]
fn expands_root() {
    let mut s = basic_setup(FileDialogVariant::OpenFile(None));

    {
        let screen = s.last_screen().unwrap();
        assert_eq!(screen.find_occurences("▸ <root>").len(), 1);
    }

    s.hit_keystroke(Key::Enter);

    {
        let screen = s.last_screen().unwrap();
        assert_eq!(screen.find_occurences("▸ <root>").len(), 0);
        assert_eq!(screen.find_occurences("▾ <root>").len(), 1);
    }

    {
        let screen = s.last_screen().unwrap();
        let hits = screen.find_occurences("▸");

        assert_eq!(hits.len(), 2);

        let subnodes: Vec<String> =
            hits.iter().map(|hit| hit.expanded_line(0, 20).to_string().trim().to_owned()).collect();

        let expected = vec!["▸ laura", "▸ bob"];

        assert_eq!(subnodes, expected);
    }
}

#[test]
fn save_file_as_points_to_dir() {
    let mut s = basic_setup(FileDialogVariant::SaveAsFile(
        BufferId::new(),
        Some(Path::new("/home/laura/subdirectory2").to_owned()),
        Some("file2.txt".to_owned()),
    ));

    let screen = s.last_screen().unwrap();

    let hits = screen.find_occurences("▾");
    assert_eq!(hits.len(), 3);

    let subnodes: Vec<String> =
        hits.iter().map(|hit| hit.expanded_line(0, 20).to_string().trim().to_owned()).collect();

    let expected = vec!["▾ <root>", "▾ laura", "▾ subdirectory2"];

    assert_eq!(subnodes, expected);
}
