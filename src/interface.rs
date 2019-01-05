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

use app_state::*;
use cursive;
use cursive::*;
use cursive::views::*;
use cursive::theme;
use cursive::theme::{Theme, Palette, BorderStyle};
use cursive::theme::PaletteColor;
use cursive::theme::Color;
use cursive::theme::BaseColor::*;
use cursive::traits::*;
use settings::load_default_settings;
use settings;
use settings::Settings;
use buffer_state::BufferState;
use buffer_state_observer::BufferStateObserver;

use events::IEvent;
use sly_text_view::SlyTextView;
use fuzzy_query_view::FuzzyQueryView;
use file_view::{self, *};
use std::thread;
use utils;

use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::sync::mpsc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;
use view_handle::ViewHandle;

pub struct Interface {
    state : AppState,
    settings : Rc<Settings>,
    channel : (mpsc::Sender<IEvent>, mpsc::Receiver<IEvent>),
    siv : Cursive,
    done : bool,
    file_bar_visible : bool,
    filedialog_visible : bool,
    bufferlist_visible : bool,
    active_editor : ViewHandle,
}

pub type IChannel = mpsc::Sender<IEvent>;

impl Interface {

    fn get_active_editor(&mut self) -> views::ViewRef<SlyTextView> {
        let id = format!("sly{}", self.active_editor.view_id());
        let editor = self.siv.find_id(&id).unwrap() as views::ViewRef<SlyTextView>;
        editor
    }

    pub fn new(mut state : AppState) -> Self {

        let mut siv = Cursive::default();
        let settings = Rc::new(load_default_settings());

        let palette = settings.get_palette();

        let theme : Theme = Theme{
            shadow : false,
            borders : BorderStyle::Simple,
            palette : palette
        };

        let channel = mpsc::channel();
        siv.set_theme(theme);

        let screen_id = siv.active_screen();
        let buffer_observer = state.get_first_buffer();
        let sly_text_view = SlyTextView::new(settings.clone(), buffer_observer, channel.0.clone());

        let active_editor = ViewHandle::new(screen_id, sly_text_view.uid());

        siv.add_fullscreen_layer(IdView::new(format!("sly{}", sly_text_view.uid()), sly_text_view));

        let mut i = Interface{
            state : state,
            settings : settings,
            channel : channel,
            siv : siv,
            done : false,
            file_bar_visible : false,
            filedialog_visible : false,
            bufferlist_visible : false,
            active_editor : active_editor,
        };

        // let known_actions = vec!["show_everything_bar"];
        //TODO filter unknown actions
        for (event, action) in i.settings.get_keybindings("global") {
            let ch = i.get_event_channel();
            match action.as_str() {
                "show_file_bar" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::ShowFileBar).unwrap(); });
                },
                "quit" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::QuitSly).unwrap(); });
                },
                "show_buffer_list" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::ShowBufferList).unwrap(); });
                },
                "save_as" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::ShowSaveAs).unwrap(); });
                },
                "open_file_dialog" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::OpenFileDialog).unwrap(); });
                },
                "close_window" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::CloseWindow).unwrap(); });
                },
                _ => {
                    debug!("unknown action {:?} bound with event global {:?}", action, event);
                }
            }
        };

        i
    }

    fn process_events(&mut self) {
        while let Ok(msg) = self.channel.1.try_recv() {
            debug!("processing event {:?}", msg);
            match msg {
                IEvent::ShowFileBar => {
                    self.show_file_bar();
                },
                IEvent::FuzzyQueryBarSelected(marker, selection) => {
                    debug!("selected {:?}", &selection);
                    self.close_file_bar();
                },
                IEvent::QuitSly => {
                    self.done = true;
                },
                IEvent::CloseWindow => {
                    self.close_floating_windows();
                },
                IEvent::BufferEditEvent(view_handle, events) => {
                    //TODO now I just send to active editor, ignoring view_handle
                    self.get_active_editor().buffer().submit_edit_events_to_buffer(events);
                },
                IEvent::ShowSaveAs => {
                    self.show_save_as();
                },
                IEvent::OpenFileDialog => {
                    self.show_open_file_dialog();
                },
                IEvent::OpenFile(file_path) => {
                    self.state.schedule_file_for_load(file_path);
                    self.close_filedialog();
                },
                IEvent::SaveBufferAs(file_path) => {
//                    match self.get_active_editor() {
//                        Some(view_handle) => {
//                            // TODO(njskalski) Create a separate buffer on this?
//                            let buffer_state: Rc<RefCell<BufferState>> = self.state.get_buffer_for_screen(&view_handle).unwrap();
//                            buffer_state.borrow_mut().save(Some(file_path));
//                        },
//                        None => debug!("unable to SaveBufferAs - no buffer found")
//                    }
                    debug!("IEvent::SaveBufferAs not implemented");
                    self.close_filedialog();
                },
                IEvent::ShowBufferList => {
                    self.show_buffer_list();
                },
                _ => {
                    debug!("unhandled IEvent {:?}", &msg);
                }
            }
        }
    }

    pub fn run(&mut self) {
        while !self.done {
            self.siv.step();
            self.process_events();
        }
    }

    pub fn close_floating_windows(&mut self) {
        self.close_file_bar();
        self.close_filedialog();
        self.close_buffer_list();
    }

    pub fn get_event_channel(&self) -> IChannel {
        self.channel.0.clone()
    }

    // TODO(njskalski) this assertion is temporary, in use only because the interface is built
    // agile, not pre-designed.
    fn assert_no_file_view(&mut self) {
        assert!(self.siv.find_id::<FileView>(file_view::FILE_VIEW_ID).is_none());
    }

    fn show_save_as(&mut self) {
        if self.filedialog_visible {
            return;
        }

        let path_op= self.get_active_editor().buffer().get_path();

        let (folder_op, file_op) = match path_op  {
                None => (None, None),
                Some(path) => utils::path_string_to_pair(path.to_string_lossy().to_string()) // TODO get rid of path_string_to_pair
        };
        self.show_file_dialog(FileViewVariant::SaveAsFile(folder_op, file_op));
    }

    fn show_open_file_dialog(&mut self) {
        if self.filedialog_visible {
            return;
        }

        self.show_file_dialog(FileViewVariant::OpenFile(None));
    }

    fn show_file_dialog(&mut self, variant : FileViewVariant) {
        if !self.filedialog_visible {
            self.assert_no_file_view();
            let is_save = variant.is_save();
            let file_view = FileView::new(self.get_event_channel(), variant, self.state.get_dir_tree(), &self.settings);
            self.siv.add_layer(IdView::new("filedialog", file_view));
            self.filedialog_visible = true;
        }
    }

    fn close_filedialog(&mut self) {
        if self.siv.focus_id("filedialog").is_ok() {
            self.siv.pop_layer();
            self.filedialog_visible = false;
        }
    }

    fn close_file_bar(&mut self) {
        if self.siv.focus_id("filebar").is_ok() {
            self.siv.pop_layer();
            self.file_bar_visible = false;
        }
    }

    fn show_file_bar(&mut self) {
        if !self.file_bar_visible {
            let ebar = FuzzyQueryView::new(self.state.get_file_index(), "filebar".to_string(), self.get_event_channel(), self.settings.clone());
            self.siv.add_layer(IdView::new("filebar",ebar));
            self.file_bar_visible = true;
        }
    }

    fn show_buffer_list(&mut self) {
        if !self.bufferlist_visible {
            let buffer_list = self.state.get_buffers();
            warn!("buffer list not imlemented yet.");
            self.bufferlist_visible = true;
        }
    }

    fn close_buffer_list(&mut self) {
        if self.siv.focus_id("bufferlist").is_ok() {
            self.siv.pop_layer();
            self.bufferlist_visible = false;
        }
    }
}

fn pair_string_to_pathbuf(folder : String, file : String) -> PathBuf {
    let mut file_path : PathBuf = PathBuf::new();
    file_path.push(folder);
    file_path.push(file);
    file_path
}