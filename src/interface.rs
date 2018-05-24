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

use app_state::AppState;
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

use events::IEvent;
use sly_text_view::SlyTextView;
use fuzzy_query_view::FuzzyQueryView;
use file_view::*;
use std::thread;

use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::sync::mpsc;
use std::collections::HashMap;

pub struct Interface {
    state : AppState,
    settings : Rc<Settings>,
    channel : (mpsc::Sender<IEvent>, mpsc::Receiver<IEvent>),
    siv : Cursive,
    done : bool,
    file_bar_visible : bool,
    file_view_visible : bool,
    buffer_to_screen : HashMap<String, ScreenId>
}

pub type IChannel = mpsc::Sender<IEvent>;

impl Interface {
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

        assert!(state.has_buffers_to_load()); //TODO it's for debug only

        let screen_id = siv.active_screen();
        let buffer_observer = state.load_buffer(screen_id);

        siv.add_fullscreen_layer(SlyTextView::new(settings.clone(), buffer_observer, channel.0.clone()));

        let mut i = Interface{
            state : state,
            settings : settings,
            channel : channel,
            siv : siv,
            done : false,
            file_bar_visible : false,
            file_view_visible : false,
            buffer_to_screen : HashMap::new()
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
                "show_buffer_bar" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::ShowBufferBar).unwrap(); });
                },
                "save_as" => {
                    i.siv.add_global_callback(event, move |_| { ch.send(IEvent::ShowSaveAs).unwrap(); });
                }
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
                    self.close_file_bar();
                },
                IEvent::BufferEditEvent(screen_id, events) => {
                    self.state.submit_edit_events_to_buffer(screen_id, events);
                },
                IEvent::ShowSaveAs => {
                    self.show_save_as();
                }
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

    pub fn get_event_channel(&self) -> IChannel {
        self.channel.0.clone()
    }

    fn show_save_as(&mut self) {
        if !self.file_view_visible {
            let file_view = FileView::new(FileViewVariant::SaveAsFile, self.state.get_dir_tree(), &self.settings);
            self.siv.add_layer(IdView::new("save_file_view", file_view));
            self.file_view_visible = true;
            debug!("file_view_visible = {:?}", self.file_view_visible);
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
}
