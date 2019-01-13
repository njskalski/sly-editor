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
use buffer_state::BufferState;
use buffer_state_observer::BufferStateObserver;
use cursive;
use cursive::theme;
use cursive::theme::BaseColor::*;
use cursive::theme::Color;
use cursive::theme::PaletteColor;
use cursive::theme::{BorderStyle, Palette, Theme};
use cursive::traits::*;
use cursive::views::*;
use cursive::*;
use settings;
use settings::load_default_settings;
use settings::Settings;

use events::IEvent;
use file_dialog::{self, *};
use fuzzy_query_view::FuzzyQueryView;
use sly_text_view::SlyTextView;
use std::thread;
use utils;

use buffer_id::BufferId;
use core::borrow::BorrowMut;
use events::IChannel;
use lsp_client::LspClient;
use overlay_dialog::OverlayDialog;
use sly_view::SlyView;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::mpsc;
use std::sync::Arc;
use view_handle::ViewHandle;

pub struct Interface {
    state :                AppState,
    settings :             Rc<Settings>,
    channel :              (mpsc::Sender<IEvent>, mpsc::Receiver<IEvent>),
    siv :                  Cursive,
    active_editor_handle : ViewHandle,
    done :                 bool,
    file_dialog_handle :   Option<ViewHandle>,
    file_bar_handle :      Option<ViewHandle>,
    buffer_list_handle :   Option<ViewHandle>,
    lsp_clients :          Vec<LspClient>, //TODO(njskalski): temporary storage to avoid removal
}

impl Interface {
    pub fn new(mut state : AppState) -> Self {
        let mut siv = Cursive::default();
        let settings = Rc::new(load_default_settings());

        let palette = settings.get_palette();

        let theme : Theme =
            Theme { shadow : false, borders : BorderStyle::Simple, palette : palette };

        let channel = mpsc::channel();
        siv.set_theme(theme);

        let buffer_observer = state.get_first_buffer().unwrap(); // TODO(njskalski): panics. Semantics unclear.
        let sly_text_view = SlyTextView::new(settings.clone(), buffer_observer, channel.0.clone());
        let active_editor = sly_text_view.handle().clone();

        let sly_text_view_handle = sly_text_view.handle();
        siv.add_fullscreen_layer(sly_text_view.with_id(sly_text_view_handle));

        let mut i = Interface {
            state,
            settings,
            channel,
            siv,
            active_editor_handle : active_editor,
            done : false,
            file_dialog_handle : None,
            file_bar_handle : None,
            buffer_list_handle : None,
            lsp_clients : Vec::new(),
        };

        // let known_actions = vec!["show_everything_bar"];
        //TODO filter unknown actions
        for (event, action) in i.settings.get_keybindings("global") {
            let ch = i.get_event_sink();
            match action.as_str() {
                "show_file_bar" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::ShowFileBar).unwrap();
                    });
                }
                "quit" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::QuitSly).unwrap();
                    });
                }
                "show_buffer_list" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::ShowBufferList).unwrap();
                    });
                }
                "save" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::SaveCurrentBuffer).unwrap();
                    });
                }
                "open_file_dialog" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::OpenFileDialog).unwrap();
                    });
                }
                "close_window" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::CloseWindow).unwrap();
                    });
                }
                "start_lsp" => {
                    i.siv.add_global_callback(event, move |_| {
                        ch.send(IEvent::EnableLSP).unwrap();
                    });
                }
                _ => {
                    debug!("unknown action {:?} bound with event global {:?}", action, event);
                }
            }
        }

        i
    }

    fn process_events(&mut self) {
        while let Ok(msg) = self.channel.1.try_recv() {
            debug!("processing event {:?}", msg);
            match msg {
                IEvent::ShowFileBar => {
                    self.show_file_bar();
                }
                IEvent::QuitSly => {
                    self.done = true;
                }
                IEvent::CloseWindow => {
                    self.cancel_floating_windows();
                }
                IEvent::BufferEditEvent(view_handle, events) => {
                    //TODO now I just send to active editor, ignoring view_handle
                    self.active_editor().buffer_obs().submit_edit_events_to_buffer(events);
                }
                IEvent::SaveCurrentBuffer => {
                    self.save_current_buffer();
                }
                IEvent::OpenFileDialog => {
                    self.show_open_file_dialog();
                }
                IEvent::ShowBufferList => {
                    self.show_buffer_list();
                }
                IEvent::EnableLSP => {
                    self.enable_lsp();
                }
                _ => {
                    debug!("unhandled IEvent {:?}", &msg);
                }
            }
        }
    }

    fn active_editor(&mut self) -> ViewRef<SlyTextView> {
        let editor = self.siv.find_id(&self.active_editor_handle.to_string()).unwrap()
            as views::ViewRef<SlyTextView>;
        editor
    }

    fn file_dialog(&mut self) -> Option<ViewRef<FileDialog>> {
        match_handle(&mut self.siv, &self.file_dialog_handle)
    }

    fn file_bar(&mut self) -> Option<ViewRef<FuzzyQueryView>> {
        match_handle(&mut self.siv, &self.file_bar_handle)
    }

    fn buffer_list(&mut self) -> Option<ViewRef<FuzzyQueryView>> {
        match_handle(&mut self.siv, &self.buffer_list_handle)
    }

    fn cancel_floating_windows(&mut self) {
        self.file_dialog().map(|mut file_dialog_ref| file_dialog_ref.borrow_mut().cancel());
    }

    /// Main program method
    pub fn main(&mut self) {
        while !self.done {
            // first, let's finish whatever action have been started in a previous frame.
            self.process_dialogs();

            self.process_events();
            self.siv.step();
        }
    }

    fn num_open_dialogs(&self) -> usize {
        (if self.file_dialog_handle.is_some() { 1 } else { 0 })
            + (if self.buffer_list_handle.is_some() { 1 } else { 0 })
            + (if self.file_bar_handle.is_some() { 1 } else { 0 })
    }

    fn remove_window(&mut self, handle : &ViewHandle) {
        self.siv.focus_id(&handle.to_string());
        self.siv.pop_layer();
    }

    /// consumes results of previous dialog choices.
    fn process_dialogs(&mut self) {
        if self.num_open_dialogs() > 1 {
            panic!("unexpected situations - more than one dialog open.")
        }

        if self.file_dialog_handle.is_some() {
            let mut file_dialog = self.file_dialog().unwrap();

            if let Some(result) = file_dialog.get_result() {
                match result {
                    Ok(FileDialogResult::Cancel) => {}
                    Ok(FileDialogResult::FileSave(buffer_id, path)) => {
                        match self.state.save_buffer_as(&buffer_id, path) {
                            Ok(()) => {},
                            Err(e) => error!("file save failed, because \"{}\"", e)
                        }
                    }
                    Ok(FileDialogResult::FileOpen(path)) => {
                        let buf_id = self.state.open_file(path);
                        debug!("buffer_id {:?}", buf_id);
                    }
                    Err(e) => {
                        error!("opening file failed, because \"{}\"", e);
                    }
                }

                let handle = self.file_dialog_handle.take().unwrap();
                self.remove_window(&handle);
            }
        }

        // TODO(njskalski): add processing of file_bar and fuzzy stuff.
    }

    pub fn get_event_sink(&self) -> IChannel {
        self.channel.0.clone()
    }

    fn show_save_as(&mut self) {
        if self.file_dialog_handle.is_some() {
            debug!("show_save_as: not showing file_dialog, because it's already opened.");
            return;
        }

        let id = self.active_editor().buffer_obs().buffer_id();
        let path_op = self.active_editor().buffer_obs().get_path();

        let (folder_op, file_op) = match path_op {
            None => (None, None),
            Some(path) => utils::path_string_to_pair(path.to_string_lossy().to_string()), /* TODO get rid of
                                                                                           * path_string_to_pair */
        };
        self.show_file_dialog(FileDialogVariant::SaveAsFile(id, folder_op, file_op));
    }

    fn show_open_file_dialog(&mut self) {
        if self.file_dialog_handle.is_some() {
            debug!("show_open_file_dialog: not showing file_dialog, because it's already opened.");
            return;
        }

        self.show_file_dialog(FileDialogVariant::OpenFile(None));
    }

    fn show_file_dialog(&mut self, variant : FileDialogVariant) {
        if self.file_dialog_handle.is_some() {
            debug!("show_file_dialog: not showing file_dialog, because it's already opened.");
            return;
        }

        let is_save = variant.is_save();
        let mut file_dialog = FileDialog::new(
            self.get_event_sink(),
            variant,
            self.state.get_dir_tree(),
            &self.settings,
        );

        self.file_dialog_handle = Some(file_dialog.get_mut().handle().clone());
        self.siv.add_layer(file_dialog);
    }

    fn show_file_bar(&mut self) {
        if self.file_bar_handle.is_some() {
            debug!("show_file_bar: not showing file_bar, because it's already opened.");
            return;
        }

        let mut file_bar = FuzzyQueryView::new(
            self.state.get_file_index(),
            "filebar".to_string(),
            self.get_event_sink(),
            self.settings.clone(),
        );

        self.file_bar_handle = Some(file_bar.get_mut().handle().clone());
        self.siv.add_layer(file_bar);
    }

    fn show_buffer_list(&mut self) {
        warn!("buffer list not imlemented yet.");
    }

    fn enable_lsp(&mut self) {
        let lsp = LspClient::new(
            OsStr::new("rls"),
            self.get_event_sink(),
            Some(self.state.directories()),
        );
        self.lsp_clients.push(lsp.unwrap());
    }

    fn save_current_buffer(&mut self) {
        let path = self.active_editor().buffer_obs().get_path();
        if path.is_none() {
            self.show_save_as();
        } else {
            let editor = self.active_editor();
            let mut buffer = editor.buffer_obs().borrow_state();
            let buffer_id = buffer.id();
            debug!("save_current_buffer unimplemented ");
        }
    }
}

fn match_handle<V>(siv : &mut Cursive, handle_op : &Option<ViewHandle>) -> Option<ViewRef<V>>
where
    V : SlyView + View,
{
    match handle_op {
        Some(handle) => siv.find_id(&handle.to_string()),
        None => None,
    }
}
