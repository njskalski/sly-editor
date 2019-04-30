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
pub mod tests {
    use app_state::AppState;
    use cursive::backend;
    use cursive::backend::puppet::observed::ObservedCell;
    use cursive::backend::puppet::observed::ObservedScreen;
    use cursive::event::Event;
    use cursive::event::Key;
    use cursive::Cursive;
    use cursive::Vec2;
    use dir_tree::TreeNodeRef;
    use dir_tree::TreeNodeVec;
    use events::IChannel;
    use events::IEvent;
    use filesystem::FakeFileSystem;
    use filesystem::*;
    use interface::Interface;
    use ncurses::filter;
    use std::cell::RefCell;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;
    use test_utils::basic_setup::tests::*;
    use test_utils::fake_tree::{fake_dir, fake_file, fake_root};
    use FileSystemType;

    pub struct AdvancedSetup {
        ss: Box<dyn BasicSetupSetupTrait>,
        receiver: mpsc::Receiver<IEvent>,
        screen_sink: crossbeam_channel::Receiver<ObservedScreen>,
        input: crossbeam_channel::Sender<Option<Event>>,
        ichannel: IChannel,
        interface: Interface,
        last_screen: RefCell<Option<ObservedScreen>>,
    }

    impl AdvancedSetup {
        pub fn with_files(files_to_open: Vec<&str>) -> Self {
            let basicSetup = BasicSetupSetupStruct::new();

            let (sender, receiver) = mpsc::channel::<IEvent>();
            let filetree = get_fake_filetree();

            //            dbg!(&filetree);

            let size = Vec2::new(210, 50);

            let backend = backend::puppet::Backend::init(Some(size));
            let sink = backend.stream();
            let input = backend.input();

            let (dirs, _) = filesystem_to_lists(&filetree);

            let files: Vec<PathBuf> =
                files_to_open.iter().map(|p| Path::new(p).to_owned()).collect();

            let filesystem = FakeFileSystem::new();

            fill_filesystem(&filetree, &filesystem);

            let app_state = AppState::new(filesystem, dirs, files, filetree, false);

            let mut siv = Cursive::new(move || backend);

            let mut interface = Interface::new(app_state, siv);
            input.send(Some(Event::Refresh)).unwrap();
            interface.main_step();

            let ichannel = interface.event_sink();

            AdvancedSetup {
                ss: Box::new(basicSetup),
                receiver,
                screen_sink: sink,
                input,
                ichannel: ichannel,
                interface,
                last_screen: RefCell::new(None),
            }
        }

        pub fn new() -> Self {
            Self::with_files(vec![])
        }

        pub fn ichannel(&self) -> &IChannel {
            &self.ichannel
        }

        pub fn step(&mut self) {
            self.refresh();

            while !self.input.is_empty() {
                self.interface.main_step();
            }
        }

        pub fn step2(&mut self) {
            self.step();
            self.step();
        }

        pub fn interface(&mut self) -> &mut Interface {
            &mut self.interface
        }

        pub fn draw_screen(&self, screen: &ObservedScreen) {
            println!("captured screen:");

            print!("x");
            for x in 0..screen.size().x {
                print!("{}", x % 10);
            }
            println!("x");

            for y in 0..screen.size().y {
                print!("{}", y % 10);

                for x in 0..screen.size().x {
                    let pos = Vec2::new(x, y);
                    let cell_op: &Option<ObservedCell> = &screen[&pos];
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
                print!("|");
                println!();
            }

            print!("x");
            for x in 0..screen.size().x {
                print!("-");
            }
            println!("x");
        }

        pub fn type_letters(&self, s: &str) {
            for letter in s.chars() {
                // TODO(njskalski): add shift probably
                self.input.send(Some(Event::Char(letter))).unwrap();
            }
        }

        pub fn last_screen(&self) -> Option<ObservedScreen> {
            while let Ok(screen) = self.screen_sink.try_recv() {
                self.last_screen.replace(Some(screen));
            }

            self.last_screen.borrow().clone()
        }

        pub fn dump_debug(&self) {
            self.last_screen().as_ref().map(|s| {
                self.draw_screen(s);
            });
        }

        //    pub fn head_window(&self) {
        //        let siv = self.interface.siv();
        //        siv.screen().get(0).unwrap().required_size()
        //    }

        pub fn hit_keystroke(&self, key: Key) {
            self.input.send(Some(Event::Key(key))).unwrap();
        }

        pub fn hit_enter(&self) {
            self.hit_keystroke(Key::Enter);
        }

        pub fn refresh(&self) {
            self.input.send(Some(Event::Refresh)).unwrap();
        }

        pub fn input(&self) -> &crossbeam_channel::Sender<Option<Event>> {
            &self.input
        }

        /// Active wait
        //    pub fn wait_workers_finish(&self) {
        //        self.refresh();
        //        while self.interface.has_running_workers() {
        //            self.interface.main_step();
        //        }
        //    }

        pub fn has_running_workers(&self) -> bool {
            self.interface.has_running_workers()
        }
    }

    /*
        <root>
            - directory1 (/home/laura)
                - subdirectory1
                - subdirectory2
                    - file1
                    - file2.txt
                    - file3.rs
                - file4.ini
            - directory2 (/home/bob)
                - .file6.hidden
    */
    pub fn get_fake_filetree() -> TreeNodeRef {
        fake_root(vec![
            fake_dir(
                "/home/laura",
                vec![
                    fake_dir("/home/laura/subdirectory1", vec![]),
                    fake_dir(
                        "/home/laura/subdirectory2",
                        vec![
                            fake_file("/home/laura/subdirectory2/file1"),
                            fake_file("/home/laura/subdirectory2/file2.txt"),
                            fake_file("/home/laura/subdirectory2/file3.rs"),
                        ],
                    ),
                    fake_file("/home/laura/file4.ini"),
                ],
            ),
            fake_dir("/home/bob", vec![fake_file("/home/bob/.file6.hidden")]),
        ])
    }

    fn fill_files(files: &mut Vec<PathBuf>, dirs: TreeNodeVec) {
        for dir in dirs {
            let mut subdirs: TreeNodeVec = vec![];

            for child in dir.children() {
                if child.is_file() {
                    files.push(child.path().unwrap().to_owned());
                } else {
                    subdirs.push(child);
                }
            }

            fill_files(files, subdirs);
        }
    }

    fn filesystem_to_lists(root: &TreeNodeRef) -> (Vec<PathBuf>, Vec<PathBuf>) {
        assert!(root.is_root());

        let dirs: Vec<PathBuf> =
            root.as_ref().children().iter().map(|dir| dir.path().unwrap().to_owned()).collect();

        let mut files: Vec<PathBuf> = vec![];
        fill_files(&mut files, root.as_ref().children());

        (dirs, files)
    }

    fn fill_filesystem(filetree: &TreeNodeRef, fs: &FileSystemType) {
        for c in filetree.children() {
            if c.is_file() {
                let path = c.path().unwrap();
                fs.create_file(path, &format!("mock file content of {:?}", path))
                    .expect(&format!("failed to create file {:?}", path));
            }

            if c.is_dir() {
                let path = c.path().unwrap();
                fs.create_dir_all(path).expect(&format!("failed to create dir {:?}", path));

                fill_filesystem(&c, fs);
            }
        }
    }
}
