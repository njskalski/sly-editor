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

use cursive::backend::puppet::observed::ObservedPieceInterface;
use cursive::event::Event;
use cursive::event::Key;
use cursive::Vec2;
use events::IEvent;
use filesystem::*;
use test_utils::advanced_setup::tests::AdvancedSetup;

#[test]
fn first_interface_test() {
    let mut s = AdvancedSetup::new();

    let screen = s.last_screen().unwrap();

    //        assert_eq!(screen[Vec2::new(0,0)], 1);

    //        s.dump_debug();
}

#[test]
fn save_dialog_displays() {
    let mut s = AdvancedSetup::new();

    s.input().send(Some(Event::CtrlChar('s'))).unwrap();
    s.step();

    //        s.dump_debug();

    let screen = s.last_screen().unwrap();

    assert_eq!(screen.find_occurences("Save file").len(), 1);
    assert_eq!(screen.find_occurences("▸ <root>").len(), 1);
}

#[test]
fn basic_typing() {
    let mut s = AdvancedSetup::new();

    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.type_letters(".txt");
    s.step();

    let screen = s.last_screen().unwrap();
    let piece = screen.piece(Vec2::new(0, 0), Vec2::new(6, 4));
    let text = piece.as_strings();

    assert_eq!(text.len(), 4);

    assert_eq!(text[0].trim(), "1 ↵");
    assert_eq!(text[1].trim(), "2 ↵");
    assert_eq!(text[2].trim(), "3 .txt");
    assert_eq!(text[3].trim(), "");
}

#[test]
fn save_dialog_displays_filelist_on_dir_select() {
    let mut s = AdvancedSetup::new();

    s.type_letters("some text");
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.type_letters("some more text");
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();

    s.input().send(Some(Event::CtrlChar('s'))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Down))).unwrap();
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("file4.ini").len(), 1);
}

#[test]
fn save_new_file_via_dialog() {
    let mut s = AdvancedSetup::new();

    s.type_letters("some text");
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.type_letters("some more text");
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();

    s.input().send(Some(Event::CtrlChar('s'))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Down))).unwrap();
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Tab))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Tab))).unwrap();
    s.step();

    s.type_letters("some_filename.txt");
    s.step();

    let screen = s.last_screen().unwrap();
    // there is dialog
    assert_eq!(screen.find_occurences("Save file").len(), 1);

    // we typed what needs to be typed
    assert_eq!(screen.find_occurences("some_filename.txt").len(), 1);

    // file does not exist yet.
    assert_eq!(s.interface().state().filesystem().is_file("/home/laura/some_filename.txt"), false);

    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();
    // dialog closed
    assert_eq!(screen.find_occurences("Save file").len(), 0);

    //        dbg!(s.interface().state().filesystem());

    // save happened
    assert_eq!(s.interface().state().filesystem().is_file("/home/laura/some_filename.txt"), true);
    assert_eq!(
        s.interface().state().filesystem().read_file("/home/laura/some_filename.txt").unwrap(),
        "some text\nsome more text\n".as_bytes().to_vec()
    );
}

#[test]
fn save_via_ctrls() {
    let mut s = AdvancedSetup::with_files(vec!["/home/laura/file4.ini"]);
    s.step();

    let screen = s.last_screen().unwrap();
    // there is fuzzy
    assert_eq!(screen.find_occurences("mock file content").len(), 1);

    s.hit_enter();
    s.type_letters("edited by laura");
    s.hit_enter();
    s.step();

    s.input().send(Some(Event::CtrlChar('s'))).unwrap();
    s.step();

    assert_eq!(
        s.interface().state().filesystem().read_file("/home/laura/file4.ini").unwrap(),
        "\nedited by laura\nmock file content of \"/home/laura/file4.ini\"".as_bytes().to_vec()
    );
}

#[test]
fn open_via_startup() {
    let mut s = AdvancedSetup::with_files(vec!["/home/laura/file4.ini"]);
    s.step();

    let screen = s.last_screen().unwrap();
    // there is fuzzy
    assert_eq!(screen.find_occurences("mock file content").len(), 1);
}

#[test]
fn open_via_fuzzy() {
    let mut s = AdvancedSetup::new();

    s.input().send(Some(Event::CtrlChar('p'))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();
    // there is fuzzy
    assert_eq!(screen.find_occurences("Context").len(), 1);

    s.type_letters(".txt");
    s.step();

    while s.has_running_workers() {
        s.step();
    }

    let screen = s.last_screen().unwrap();

    assert_eq!(screen.find_occurences("Context").len(), 1); // fuzzy still here.
    assert_eq!(screen.find_occurences("txt").len(), 2); //file.txt and query: ".txt".
    assert_eq!(screen.find_occurences("file2.txt").len(), 1);
    s.hit_enter();
    s.step();

    let screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("1 mock file content ").len(), 1); // file content there
    assert_eq!(screen.find_occurences("Context").len(), 0); // fuzzy gone.
}

#[test]
fn open_via_dialog() {
    let mut s = AdvancedSetup::new();
    s.step();

    let screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("mock").len(), 0); // no mock contents.

    s.input().send(Some(Event::CtrlChar('d'))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();
    // there is Dialog
    assert_eq!(screen.find_occurences("Open file").len(), 1);

    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Down))).unwrap();
    s.input().send(Some(Event::Key(Key::Enter))).unwrap();
    s.step();
    s.input().send(Some(Event::Key(Key::Tab))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();

    // there is Dialog
    assert_eq!(screen.find_occurences("▾ laura").len(), 1);
    assert_eq!(screen.find_occurences("file4.ini").len(), 1);

    s.hit_enter();
    s.step();

    let screen = s.last_screen().unwrap();

    // dialog went away
    assert_eq!(screen.find_occurences("Open file").len(), 0);

    // and we have mock content
    assert_eq!(screen.find_occurences("mock file content").len(), 1);
}

#[test]
fn fuzzy_buffer_list_displays() {
    let mut s = AdvancedSetup::new();

    s.input().send(Some(Event::CtrlChar('o'))).unwrap();

    s.step();

    let screen = s.last_screen().unwrap();

    assert_eq!(screen.find_occurences("Context : \"context\"    query: \"\"").len(), 1);
    assert_eq!(screen.find_occurences("<unnamed>").len(), 1);
}

//    #[test]
// TODO(njskalski): this fails now, since not all files passed are loaded. Some are just scheduled.
fn loads_listed_files() {
    let mut s = AdvancedSetup::with_files(vec![
        "/home/laura/subdirectory2/file2.txt",
        "/home/laura/subdirectory2/file3.rs",
    ]);

    s.step();

    let screen = s.last_screen().unwrap();
    assert_eq!(
        screen
            .find_occurences("mock file content of \"/home/laura/subdirectory2/file2.txt\"")
            .len(),
        1
    );

    s.input().send(Some(Event::CtrlChar('o'))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("Context").len(), 1);

    s.dump_debug();
}

//    #[test]
// TODO(njskalski): this will not work prior fixing one above.
fn fuzzy_buffer_list_change_buffers() {
    let mut s = AdvancedSetup::with_files(vec!["/home/laura/subdirectory2/file2.txt"]);

    s.input().send(Some(Event::CtrlChar('o'))).unwrap();
    s.step();

    s.dump_debug();

    //        let screen = s.last_screen().unwrap();
    //
    //        assert_eq!(screen.find_occurences("Context : \"context\"    query: \"\"").len(), 1);
    //        assert_eq!(screen.find_occurences("<unnamed>").len(), 1);
}

#[test]
fn all_commands_bar_displays() {
    let mut s = AdvancedSetup::new();

    s.input().send(Some(Event::CtrlChar('y'))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();

    assert_eq!(screen.find_occurences("Context : \"context\"    query: \"\"").len(), 1);
    /// some text stuff
    assert_eq!(screen.find_occurences("paste").len(), 1);
    assert_eq!(screen.find_occurences("redo").len(), 1);
}

#[test]
fn fuzzy_file_index_displays() {
    let mut s = AdvancedSetup::new();

    s.input().send(Some(Event::CtrlChar('p'))).unwrap();
    s.step();

    let screen = s.last_screen().unwrap();

    assert_eq!(screen.find_occurences("Context : \"context\"    query: \"\"").len(), 1);

    s.type_letters("fi");
    s.step(); // needed to process keystrokes

    let screen = s.last_screen().unwrap();
    assert_eq!(screen.find_occurences("query: \"fi\"").len(), 1);

    println!("fi : {:?}", s.interface().state().get_file_index());

    while s.has_running_workers() {
        s.step();
    }

    // cannot test query results at this time, since the file index is empty without mocking
    // ::path.
    //        s.dump_debug();
}
