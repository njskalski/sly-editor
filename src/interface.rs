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
use settings::Settings;

use events::IEvent;
use file_dialog::{self, *};
use fuzzy_query_view::FuzzyQueryView;
use sly_text_view::SlyTextView;
use std::thread;
use utils;

use buffer_id::BufferId;
use core::borrow::Borrow;
use core::borrow::BorrowMut;
use events::IChannel;
use file_dialog::FileDialog;
use fuzzy_query_view::FuzzyQueryResult;
use lsp_client::LspClient;
use overlay_dialog::OverlayDialog;
use sly_view::SlyView;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error;
use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::ops::DerefMut;
use std::path::Path;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use view_handle::ViewHandle;
use std::sync;
use std::sync::atomic::AtomicPtr;

const FILE_BAR_MARKER: &'static str = "file_bar";
const BUFFER_LIST_MARKER: &'static str = "file_bar";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum InterfaceError {
    Undefined,
}

/*
At this moment I have not decided on whether interface holds premise before siv or other way around.
So I expect every method in this object that updates handles to reflect these changes in siv field
and other way around.
*/

pub struct Interface {
    state: AppState,
    settings: Rc<Settings>,
    channel: (mpsc::Sender<IEvent>, mpsc::Receiver<IEvent>),
    siv: Cursive,
    active_editor_handle: ViewHandle,
    inactive_editors: HashMap<BufferId, IdView<SlyTextView>>,
    path_to_buffer_id: HashMap<PathBuf, BufferId>,
    done: bool,
    file_dialog_handle: Option<ViewHandle>,
    file_bar_handle: Option<ViewHandle>,
    buffer_list_handle: Option<ViewHandle>,
    lsp_clients: Vec<LspClient>, //TODO(njskalski): temporary storage to avoid removal
}

fn find_view_with_handle<V>(siv: &mut Cursive, handle_op: &Option<ViewHandle>) -> Option<ViewRef<V>>
where
    V: SlyView + View,
{
    match handle_op {
        Some(handle) => siv.find_id(&handle.to_string()),
        None => None,
    }
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InterfaceError (not defined)")
    }
}

impl Interface {
    pub fn new(mut state: AppState) -> Self {
        let mut siv = Cursive::default();
        let settings = Rc::new(Settings::load_default());

        let palette = settings.get_palette();

        let theme: Theme = Theme { shadow: false, borders: BorderStyle::Simple, palette: palette };

        let channel = mpsc::channel();
        siv.set_theme(theme);

        let buffer_observer = state.get_first_buffer().unwrap(); // TODO(njskalski): panics. Semantics unclear.
        let sly_text_view = SlyTextView::new(settings.clone(), buffer_observer, channel.0.clone());
        let active_editor = sly_text_view.handle().clone();

        siv.add_fullscreen_layer(sly_text_view);

        let mut i = Interface {
            state: state,
            settings: settings,
            channel: channel,
            siv: siv,
            active_editor_handle: active_editor,
            inactive_editors: HashMap::new(),
            path_to_buffer_id: HashMap::new(),
            done: false,
            file_dialog_handle: None,
            file_bar_handle: None,
            buffer_list_handle: None,
            lsp_clients: Vec::new(),
        };

        // let known_actions = vec!["show_everything_bar"];
        //TODO filter unknown actions
        for (event, action) in i.settings.get_keybindings("global") {
            let ch = i.event_sink();
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

    // TODO(njskalski): error handling, borrow instead of copy etc.
    fn replace_current_editor_view(
        &mut self,
        new_editor: IdView<SlyTextView>,
    ) -> IdView<SlyTextView> {
        // removing old view
        let handle = self.active_editor_handle.clone();
        let old_view = self.remove_window::<SlyTextView>(&handle);

        // inserting new view
        self.active_editor_handle = new_editor.handle();
        self.siv.screen_mut().add_layer(new_editor);

        // returning old view
        old_view.unwrap()
    }

    fn remove_window<T>(&mut self, handle: &ViewHandle) -> Option<IdView<T>>
    where
        T: SlyView + View,
    {
        let screen = self.siv.screen_mut();
        let layer_pos = screen.find_layer_from_id(&handle.to_string())?;
        screen.move_to_front(layer_pos);
        // the line above modifies state. So now I prefer method to crash than return None.
        let view_box: Box<View> = screen.pop_layer().unwrap();
        let sly_view_box = view_box.as_boxed_any().downcast::<IdView<T>>().ok().unwrap();
        let sly_view = *sly_view_box;
        Some(sly_view)
    }

    // TODO(njskalski): add proper handling of errrors, it's a total mess now!
    fn create_editor_for_buffer_id(&mut self, buffer_id: &BufferId) -> Result<(), InterfaceError> {
        {
            let old_editor = self.inactive_editors.get(buffer_id);
            if old_editor.is_some() {
                panic!("attempt to re-create editor for buffer_id {}", buffer_id);
            }
        }

        let obs = self.state.buffer_obs(buffer_id).unwrap(); //TODO panics
        let mut view = SlyTextView::new(self.settings.clone(), obs, self.event_sink());
        if self.inactive_editors.insert(buffer_id.clone(), view).is_some() {
            panic!("insertion failed, object already present");
        }

        Ok(())
    }

    /// Flushes event queue processing all events in one take. Does not block.
    fn process_events(&mut self) {
        while let Ok(msg) = self.channel.1.try_recv() {}
    }

    fn process_event(&mut self, ievent: IEvent) {
        debug!("processing event {:?}", &ievent);
        match ievent {
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
            IEvent::FuzzyQueryWorkerProduced(marker) => {
                self.refresh();
            }
            _ => {
                debug!("unhandled IEvent {:?}", &ievent);
            }
        }
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
                            Ok(()) => {}
                            Err(e) => error!("file save failed, because \"{}\"", e),
                        }
                    }
                    Ok(FileDialogResult::FileOpen(path)) => {
                        let buf_id = self.open_and_or_focus_file(path);
                        debug!("buffer_id {:?}", buf_id);
                    }
                    Err(e) => {
                        error!("opening file failed, because \"{}\"", e);
                    }
                }

                let handle = self.file_dialog_handle.take().unwrap();
                self.remove_window::<FileDialog>(&handle);
            }
        }

        if self.file_bar_handle.is_some() {
            let mut file_bar = self.file_bar().unwrap();

            if let Some(result) = file_bar.get_result() {
                match result {
                    Ok(FuzzyQueryResult::Cancel) => {}
                    Ok(FuzzyQueryResult::Selected(_, item_marker)) => {
                        debug!("selected file {:?}", &item_marker);
                        self.open_and_or_focus_file(item_marker);
                    }
                    Err(e) => {
                        error!("opening file failed, because \"{}\"", e);
                    }
                }
                let handle = self.file_bar_handle.take().unwrap();
                self.remove_window::<FuzzyQueryView>(&handle);
            }
        }

        if self.buffer_list_handle.is_some() {
            let mut buffer_list = self.buffer_list().unwrap();

            if let Some(result) = buffer_list.get_result() {
                match result {
                    Ok(FuzzyQueryResult::Cancel) => {}
                    Ok(FuzzyQueryResult::Selected(_, buffer_id_str)) => {
                        debug!("selected buffer {}", &buffer_id_str);
                        self.open_and_or_focus(&BufferId::from_string(&buffer_id_str).unwrap());
                    }
                    Err(e) => {
                        error!("opening buffer failed, because \"{}\"", e);
                    }
                }
                let handle = self.buffer_list_handle.take().unwrap();
                self.remove_window::<FuzzyQueryView>(&handle); // TODO(njskalski): can cache here.
            }
        }

        // TODO(njskalski): add processing of file_bar and fuzzy stuff.
    }

    /// This updates interface and SIV!
    fn open_and_or_focus(&mut self, buffer_id: &BufferId) {
        if !self.inactive_editors.contains_key(buffer_id) {
            self.create_editor_for_buffer_id(buffer_id);
        }

        let new_editor = self.inactive_editors.remove(buffer_id).unwrap();
        let mut old_editor = self.replace_current_editor_view(new_editor);
        let old_editor_buffer_id = old_editor.get_mut().buffer_obs().buffer_id().clone();
        self.inactive_editors.insert(old_editor_buffer_id, old_editor);
    }

    //TODO error handling!
    fn open_and_or_focus_file<T>(&mut self, path: T)
    where
        T: Into<PathBuf>,
    {
        let path_buf: PathBuf = path.into();
        let buffer_id_op = self.path_to_buffer_id.get(&path_buf).map(|x| x.clone());

        let buffer_id = match buffer_id_op {
            Some(buffer_id) => buffer_id,
            None => {
                debug!("file {:?} not opened yet, opening.", &path_buf);
                self.state
                    .open_or_get_file(&path_buf)
                    .expect(&format!("Unable to open file {:?}", &path_buf))
            }
        };

        self.open_and_or_focus(&buffer_id);
    }

    fn active_editor(&mut self) -> ViewRef<SlyTextView> {
        let editor = self.siv.find_id(&self.active_editor_handle.to_string()).unwrap()
            as views::ViewRef<SlyTextView>;
        editor
    }

    fn focus_buffer(&mut self, buffer_id: BufferId) {}

    fn file_dialog(&mut self) -> Option<ViewRef<FileDialog>> {
        find_view_with_handle(&mut self.siv, &self.file_dialog_handle)
    }

    fn file_bar(&mut self) -> Option<ViewRef<FuzzyQueryView>> {
        find_view_with_handle(&mut self.siv, &self.file_bar_handle)
    }

    fn buffer_list(&mut self) -> Option<ViewRef<FuzzyQueryView>> {
        find_view_with_handle(&mut self.siv, &self.buffer_list_handle)
    }

    fn cancel_floating_windows(&mut self) {
        self.file_dialog().map(|mut file_dialog_ref| file_dialog_ref.borrow_mut().cancel());
    }

    /// This makes Cursive blocking .step() call exit early with no errors.
    pub fn refresh(&self) {
        self.siv.cb_sink().send_timeout(Box::new(|s: &mut Cursive| {}), Duration::new(0, 0));
    }

    /// Main program method
    pub fn main(mut self) {
        // there have to be two threads for interface. One will be waiting for Cursive step to end
        // (on input or refresh) and another one able to force that wakeup.

//        let receiver = self.channel.1.borrow();

        let (sender, receiver) = mpsc::channel::<IEvent>();

        let arc = Arc::new(sync::Mutex::new(AtomicPtr::new(&mut self)));

        let arc1 = arc.clone();
        let arc2 = arc1.clone();

        let thread1 = thread::Builder::new()
            .name("IEvent processor".to_owned())
            .spawn(move || {
                match receiver.recv() {
                    Ok(ievent) => {
                        match arc1.lock() {
                            Ok(ref mut interface) => {
                                interface.process_event(ievent);
                                while let Ok(ievent) = receiver.try_recv() {
                                    interface.process_event(ievent);
                                }
                                interface.refresh(); // wakes up thread two after processing all ievents.
                            }
                        }

                    }
                    Err(e) => {
                        debug!("finishing ievent processor on {:?}", e);
                    }
                }
            })
            .unwrap();

        let mut siv = self.siv.borrow_mut();

        let thread2 = thread::Builder::new()
            .name("Cursive processor".to_owned())
            .spawn(move || {
                loop {
                    // first, let's finish whatever action have been started in a previous frame.
                    match arc2.lock() {
                        Ok(ref mut interface) => {
                            interface.process_dialogs();
                        }
                    }

                    if !siv.is_running() {
                        siv.step(); //this blocks on input OR force refresh.
                    };
                }
            })
            .unwrap();

        thread1.join();
        thread2.join();
    }

    fn num_open_dialogs(&self) -> usize {
        (if self.file_dialog_handle.is_some() { 1 } else { 0 })
            + (if self.buffer_list_handle.is_some() { 1 } else { 0 })
            + (if self.file_bar_handle.is_some() { 1 } else { 0 })
    }

    pub fn event_sink(&self) -> IChannel {
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

    fn show_file_dialog(&mut self, variant: FileDialogVariant) {
        if self.file_dialog_handle.is_some() {
            debug!("show_file_dialog: not showing file_dialog, because it's already opened.");
            return;
        }

        let is_save = variant.is_save();
        let mut file_dialog =
            FileDialog::new(self.event_sink(), variant, self.state.get_dir_tree(), &self.settings);

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
            FILE_BAR_MARKER.to_string(),
            self.event_sink(),
            self.settings.clone(),
        );

        self.file_bar_handle = Some(file_bar.get_mut().handle().clone());
        self.siv.add_layer(file_bar);
    }

    fn show_buffer_list(&mut self) {
        if self.file_bar_handle.is_some() {
            debug!("show_buffer_list: not showing file_bar, because it's already opened.");
            return;
        }

        let mut buffer_list = FuzzyQueryView::new(
            self.state.buffer_index(),
            BUFFER_LIST_MARKER.to_string(),
            self.event_sink(),
            self.settings.clone(),
        );

        self.buffer_list_handle = Some(buffer_list.get_mut().handle().clone());
        self.siv.add_layer(buffer_list);
    }

    fn enable_lsp(&mut self) {
        let lsp =
            LspClient::new(OsStr::new("rls"), self.event_sink(), Some(self.state.directories()));
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

impl std::error::Error for InterfaceError {
    fn description(&self) -> &str {
        "InterfaceError (not defined)"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}
