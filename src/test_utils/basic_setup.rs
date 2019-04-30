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
    use cursive::backend;
    use cursive::backend::puppet::observed::ObservedCell;
    use cursive::backend::puppet::observed::ObservedScreen;
    use cursive::event::Event;
    use cursive::event::Key;
    use cursive::Cursive;
    use cursive::Vec2;
    use dir_tree::TreeNodeRef;
    use events::IChannel;
    use events::IEvent;
    use file_dialog::FileDialog;
    use file_dialog::FileDialogVariant;
    use settings::Settings;
    use sly_view::SlyView;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::time::Duration;
    use test_utils::advanced_setup::tests::get_fake_filetree;
    use test_utils::fake_tree::{fake_dir, fake_file, fake_root};
    use view_handle::ViewHandle;

    pub trait BasicSetupSetupTrait {
        fn settings(&self) -> &Rc<Settings>;
        fn filesystem(&self) -> &TreeNodeRef;
    }

    pub struct BasicSetupSetupStruct {
        settings: Rc<Settings>,
        filesystem: TreeNodeRef,
    }

    impl BasicSetupSetupTrait for BasicSetupSetupStruct {
        fn settings(&self) -> &Rc<Settings> {
            &self.settings
        }

        fn filesystem(&self) -> &TreeNodeRef {
            &self.filesystem
        }
    }

    impl BasicSetupSetupStruct {
        pub fn new() -> Self {
            BasicSetupSetupStruct {
                settings: Rc::new(Settings::load_default()),
                filesystem: get_fake_filetree(),
            }
        }
    }

    pub struct BasicSetup {
        ss: Box<dyn BasicSetupSetupTrait>,
        receiver: mpsc::Receiver<IEvent>,
        siv: Cursive,
        screen_sink: crossbeam_channel::Receiver<ObservedScreen>,
        input: crossbeam_channel::Sender<Option<Event>>,
        last_screen: RefCell<Option<ObservedScreen>>,
        handle: ViewHandle,
    }

    impl BasicSetup {
        pub fn new<C, V>(constructor: C) -> Self
        where
            C: FnOnce(&BasicSetupSetupTrait, IChannel) -> V,
            V: SlyView + cursive::view::View,
        {
            let basicSetup = BasicSetupSetupStruct::new();

            let (sender, receiver) = mpsc::channel::<IEvent>();
            let filesystem = get_fake_filetree();

            let view = constructor(&basicSetup, sender);

            //        let dialog = FileDialog::new(sender, variant, filesystem.clone(), settings.clone());
            let handle = view.handle();

            let size = Vec2::new(80, 16); // TODO(njskalski): un-hardcode it.

            let backend = backend::puppet::Backend::init(Some(size));
            let sink = backend.stream();
            let input = backend.input();

            let mut siv = Cursive::new(|| backend);
            siv.add_fullscreen_layer(view);

            siv.focus_id(&handle.to_string());

            input.send(Some(Event::Refresh)).unwrap();
            siv.step();

            BasicSetup {
                ss: Box::new(basicSetup),
                receiver,
                siv,
                screen_sink: sink,
                input,
                last_screen: RefCell::new(None),
                handle,
            }
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

        pub fn last_screen(&self) -> Option<ObservedScreen> {
            while let Ok(screen) = self.screen_sink.recv_timeout(Duration::new(0, 0)) {
                self.last_screen.replace(Some(screen));
            }

            self.last_screen.borrow().clone()
        }

        pub fn dump_debug(&self) {
            self.last_screen().as_ref().map(|s| {
                self.draw_screen(s);
            });
        }

        pub fn hit_keystroke(&mut self, key: Key) {
            self.input.send(Some(Event::Key(key))).unwrap();
            self.siv.step();
        }
    }
}
