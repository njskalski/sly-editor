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

#[cfg(test)]
mod tests {
    use cursive::backend::puppet::observed::ObservedPieceInterface;
    use cursive::event::Event;
    use cursive::event::Key;
    use cursive::Vec2;
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
    fn fuzzy_buffer_list_displays() {
        let mut s = AdvancedSetup::new();

        s.input().send(Some(Event::CtrlChar('o'))).unwrap();

        s.step();

        let screen = s.last_screen().unwrap();

        assert_eq!(screen.find_occurences("Context : \"context\"    query: \"\"").len(), 1);
        assert_eq!(screen.find_occurences("<unnamed>").len(), 1);
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
}
