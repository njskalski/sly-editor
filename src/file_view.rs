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

// this is going to be a view that acts as save_as or load file view.
// I model it after what typical GUI user is accustomed to, no innovation in here in MVP

use cursive::align::*;
use cursive::direction::*;
use cursive::event::*;
use cursive::theme::*;
use cursive::vec::*;
use cursive::view::*;
use cursive::views::*;
use cursive::*;
use cursive_tree_view::*;

use core::any::Any;
use lazy_dir_tree::LazyTreeNode;
use std::borrow::BorrowMut;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

use color_view_wrapper::{ColorViewWrapper, PrinterModifierType};
use settings::Settings;
use std::env;
use std::path::Path;

use buffer_id::BufferId;
use events::IChannel;
use events::IEvent;
use std::path::PathBuf;

// TODO(njskalski) this view took longer than anticipated to implement, so I rushed to the end
// sacrificing quality a refactor is required.
// TODO(njskalski) implement caching or remove Rcs.
// TODO(njskalski) this file is work-in-progress. Most commented code is to be reused, as
// I will need different variants of file tree / directory tree in many places.
// TODO(njskalski) add support directories outside any of selected directories?
// TODO(njskalski) add opening a proper folder and filling file field while data is provided

#[derive(Debug)]
pub enum FileViewVariant {
    SaveAsFile(BufferId, Option<String>, Option<String>), // directory, filename
    OpenFile(Option<String>),                             //directory
}

impl FileViewVariant {
    pub fn get_buffer_id_op(&self) -> Option<BufferId> {
        match self {
            FileViewVariant::SaveAsFile(buffer_id, folder_op, file_op) => Some(buffer_id.clone()),
            FileViewVariant::OpenFile(folder_op) => None,
        }
    }

    pub fn get_folder_op(&self) -> &Option<String> {
        match self {
            FileViewVariant::SaveAsFile(buffer_id, folder_op, file_op) => folder_op,
            FileViewVariant::OpenFile(folder_op) => folder_op,
        }
    }

    pub fn get_file_op(&self) -> &Option<String> {
        match self {
            FileViewVariant::SaveAsFile(buffer_id, folder_op, file_op) => file_op,
            FileViewVariant::OpenFile(folder_op) => &None,
        }
    }

    pub fn is_open(&self) -> bool {
        match self {
            FileViewVariant::OpenFile(_) => true,
            _ => false,
        }
    }

    pub fn is_save(&self) -> bool {
        match self {
            FileViewVariant::SaveAsFile(..) => true,
            _ => false,
        }
    }

    pub fn get_title(&self) -> &'static str {
        match self {
            FileViewVariant::SaveAsFile(..) => "Save file",
            FileViewVariant::OpenFile(_) => "Open file",
        }
    }
}

pub struct FileView {
    variant : FileViewVariant,
    channel : IChannel,
    mv :      LinearLayout,
}

impl FileView {
    fn get_buffer_id_op(&self) -> Option<BufferId> {
        self.variant.get_buffer_id_op()
    }
}

pub const FILE_VIEW_ID : &'static str = "file_view";
const DIR_TREE_VIEW_ID : &'static str = "fv_dir_tree_view";
const FILE_LIST_VIEW_ID : &'static str = "fv_file_list_view";
const EDIT_VIEW_ID : &'static str = "fv_edit_view";

type TreeViewType = TreeView<Rc<LazyTreeNode>>;

fn get_file_view(siv: &mut Cursive) -> ViewRef<FileView> {
    siv.find_id(FILE_VIEW_ID).unwrap()
}

fn get_dir_tree_on_collapse_switch_callback(
    files_visible : bool,
) -> impl Fn(&mut Cursive, usize, bool, usize) -> () {
    move |siv : &mut Cursive, row : usize, is_collapsed : bool, children : usize| {
        // debug!("dir tree on collapse callback at {:}, ic = {:}. children = {:}", row,
        // is_collapsed, children);
        let mut view : ViewRef<TreeViewType> = siv.find_id(DIR_TREE_VIEW_ID).unwrap();
        //the line below looks complicated, but it boils down to copying Rc<LazyTreeNode>, so view
        // borrow can end immediately.
        let item = (*view).borrow_item(row).unwrap().clone();

        if is_collapsed == false {
            let mut dir_vec : Vec<Rc<LazyTreeNode>> = Vec::new();
            let mut file_vec : Vec<Rc<LazyTreeNode>> = Vec::new();

            match *item {
                LazyTreeNode::RootNode(ref children) => {
                    for c in children {
                        match c.as_ref() {
                            &LazyTreeNode::DirNode(_) => dir_vec.push(c.clone()),
                            &LazyTreeNode::FileNode(_) => {
                                if files_visible {
                                    file_vec.push(c.clone());
                                }
                            }
                            &LazyTreeNode::RootNode(_) => panic!("RootNode cannot be embedded."),
                        };
                    }
                }
                LazyTreeNode::DirNode(ref p) => {
                    let path = Path::new(&**p);
                    for dir_entry in path.read_dir().expect("read_dir call failed.") {
                        if let Ok(entry) = dir_entry {
                            if let Ok(meta) = entry.metadata() {
                                if files_visible && meta.is_file() {
                                    let res =
                                        Rc::new(LazyTreeNode::FileNode(Rc::new(entry.path())));
                                    file_vec.push(res);
                                } else if meta.is_dir() {
                                    let res = Rc::new(LazyTreeNode::DirNode(Rc::new(entry.path())));
                                    dir_vec.push(res);
                                }
                            }
                        }
                    }
                }
                _ => {}
            };

            dir_vec.sort();
            if files_visible {
                file_vec.sort();
            }

            for dir in dir_vec.iter() {
                view.insert_container_item(dir.clone(), Placement::LastChild, row);
            }

            for file in file_vec.iter() {
                view.insert_item(file.clone(), Placement::LastChild, row);
            }
        } else {
            // TODO(njskalski) - possible bug in cursive_tree_view: removal of these set_collapsed
            // calls leads to cursive_tree_view::draw crash. It seems like there is an
            // override of "index" variable there. Also, the repository seems outdated,
            // so I guess I should either fork it or abandon use of this view.
            match *item {
                LazyTreeNode::RootNode(_) => {
                    view.set_collapsed(row, false);
                    view.remove_children(row);
                    view.set_collapsed(row, true);
                }
                LazyTreeNode::DirNode(_) => {
                    view.set_collapsed(row, false);
                    view.remove_children(row);
                    view.set_collapsed(row, true);
                }
                _ => panic!("Only RootNode or DirNode can be expanded."),
            }
        }
    }
}

//TODO(njskalski) add files support to work with out-of-fileview project-treeview
fn dir_tree_on_select_callback(siv : &mut Cursive, row : usize) {
    // debug!("dir tree on select callback at {:}", row);
    let mut view : ViewRef<TreeViewType> = siv.find_id(DIR_TREE_VIEW_ID).unwrap();
    //the line below looks complicated, but it boils down to copying Rc<LazyTreeNode>, so view
    // borrow can end immediately.
    let item = (*view).borrow_item(row).unwrap().clone();

    let mut file_list_view : ViewRef<SelectView<Rc<LazyTreeNode>>> =
        siv.find_id(FILE_LIST_VIEW_ID).unwrap();
    file_list_view.clear();

    let mut dir_vec : Vec<Rc<LazyTreeNode>> = Vec::new();
    let mut file_vec : Vec<Rc<LazyTreeNode>> = Vec::new();

    match *item {
        // TODO(njskalski) add the argument files as children of RootNode?
        // LazyTreeNode::RootNode(ref dirs) => {
        //     for d in dirs {
        //         view.insert_container_item(Rc::new(LazyTreeNode::DirNode(d.clone())),
        // Placement::LastChild, row);     };
        // },
        LazyTreeNode::DirNode(ref p) => {
            let path = Path::new(&**p);
            for dir_entry in path.read_dir().expect("read_dir call failed") {
                if let Ok(entry) = dir_entry {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_file() {
                            let res = Rc::new(LazyTreeNode::FileNode(Rc::new(entry.path())));
                            file_vec.push(res);
                        } else if meta.is_dir() {
                            // let res = Rc::new(LazyTreeNode::DirNode(Rc::new(entry.path().
                            // to_str().unwrap(). to_string())));
                            // dir_vec.push(res);
                        }
                    }
                }
            }
        }
        _ => {}
    };

    dir_vec.sort();
    file_vec.sort();

    // for dir in dir_vec.iter() {
    //     file_list_view.add_item(dir.to_string(), dir.clone());
    // }

    for file in file_vec.iter() {
        file_list_view.add_item(file.to_string(), file.clone());
    }
}

fn get_file_list_on_submit(is_file_open : bool) -> impl Fn(&mut Cursive, &Rc<LazyTreeNode>) -> () {
    move |siv : &mut Cursive, item : &Rc<LazyTreeNode>| {
        // TODO(njskalski): for some reason if the line below is uncommented (and shadowing ones
        // are disabled) the unwrap inside get_path_op fails. Investigate why.
        // let mut file_view : ViewRef<FileView> = siv.find_id::<FileView>(FILE_VIEW_ID).unwrap();
        if is_file_open {
            let mut file_view : ViewRef<FileView> = siv.find_id::<FileView>(FILE_VIEW_ID).unwrap();
            match item.as_ref() {
                &LazyTreeNode::FileNode(ref path) => {
                    file_view.channel.send(IEvent::OpenFile(path.as_ref().clone()))
                }
                _ => panic!("Expected only FileNodes on file_list."),
            };
        } else {
            siv.focus_id(EDIT_VIEW_ID);
            let mut edit_view : ViewRef<EditView> = siv.find_id::<EditView>(EDIT_VIEW_ID).unwrap();
            edit_view.borrow_mut().set_content(item.to_string());
        }
    }
}

fn get_path_op(siv : &mut Cursive) -> Option<PathBuf> {
    // TODO(njskalski) refactor these uwraps.
    let mut tree_view : ViewRef<TreeViewType> = siv.find_id(DIR_TREE_VIEW_ID).unwrap();
    let row = tree_view.row().unwrap();
    let item = tree_view.borrow_item(row).unwrap().clone();

    let prefix = match *item {
        LazyTreeNode::RootNode(_) => return None, // root selected, no prefix
        LazyTreeNode::DirNode(ref path) => path.as_ref().clone(),
        _ => panic!(), //no support for FileNodes in this tree.
    };

    Some(prefix)
}

fn on_file_edit_save_submit(siv : &mut Cursive, file_name : &str) {
    if file_name.len() == 0 {
        return;
    }

    if let Some(mut path) = get_path_op(siv) {
        let mut file_view : ViewRef<FileView> = get_file_view(siv);
        path.push(file_name);
        let buffer_id = file_view.get_buffer_id_op().unwrap();

        file_view.channel.send(IEvent::SaveBufferAs(buffer_id, path));
    } else {
        return; // no folder selected, no prefix. TODO(njskalski) add panic?
    };
}

// TODO(njskalski) this method doesn't work.
pub fn expand_tree(siv : &mut Cursive, path : &Path) {
    let mut tree_view : ViewRef<TreeViewType> = siv.find_id(DIR_TREE_VIEW_ID).unwrap();

    let mut row_begin = 0;
    let mut row_end = tree_view.len();

    let mut done = false;

    let mut last_expansion : Option<usize> = None;

    while !done {
        let mut expanded = false;
        for i in row_begin..row_end {
            let item = tree_view.borrow_item(i).unwrap().clone();

            match *item {
                LazyTreeNode::DirNode(ref dir) => {
                    if path.starts_with(dir.as_ref()) {
                        let items_in_total = tree_view.len();
                        tree_view.expand_item(i);
                        last_expansion = Some(i);
                        expanded = true;
                        let num_children = tree_view.len() - items_in_total;
                        row_begin = i + 1;
                        row_end = row_begin + num_children;
                        break;
                    }
                }
                _ => {}
            }
        }

        if !expanded && last_expansion.is_some() {
            let last_i = last_expansion.unwrap();

            tree_view.collapse_item(last_i);
            tree_view.set_selected_row(last_i);

            done = true;
        }
    }
}

impl FileView {
    pub fn new(
        ch : IChannel,
        variant : FileViewVariant,
        root : Rc<LazyTreeNode>,
        settings : &Rc<Settings>,
    ) -> IdView<Self> {
        debug!("creating file view with variant {:?}", variant);

        // TODO(njskalski) implement styling with new solution when Cursive updates.
        let primary_text_color = settings.get_color("theme/file_view/primary_text_color");
        let selected_bg_color = settings.get_color("theme/file_view/selected_background");
        let non_selected_bg_color = settings.get_color("theme/file_view/non_selected_background");

        let printer_to_theme : PrinterModifierType = Rc::new(Box::new(move |p : &Printer| {
            let mut palette = theme::Palette::default();

            // if p.focused {
            //     palette[PaletteColor::View] = selected_bg_color;
            // } else {
            //     palette[PaletteColor::View] = non_selected_bg_color;
            // }
            // palette[PaletteColor::Background] = palette[PaletteColor::View];
            // palette[PaletteColor::Shadow] = palette[PaletteColor::View];
            //
            // palette[PaletteColor::Primary] = primary_text_color;
            // palette[PaletteColor::Secondary] = primary_text_color;
            // palette[PaletteColor::Tertiary] = primary_text_color;
            // palette[PaletteColor::TitlePrimary] = primary_text_color;
            // palette[PaletteColor::TitleSecondary] = primary_text_color;
            //
            // palette[PaletteColor::Highlight] = primary_text_color;
            // palette[PaletteColor::HighlightInactive] = primary_text_color;

            let theme = Theme { shadow : false, borders : BorderStyle::None, palette : palette };

            theme
        }));

        let mut vl = LinearLayout::new(Orientation::Vertical);
        let mut hl = LinearLayout::new(Orientation::Horizontal);

        // TODO(njskalski) add a separate theme to disable color inversion effect on edit.

        let title : &'static str = variant.get_title();

        vl.add_child(ColorViewWrapper::new(
            Layer::new(TextView::new(title)),
            printer_to_theme.clone(),
        ));

        let mut dir_tree : TreeViewType = TreeView::new();
        // dir_tree.h_align(HAlign::Left);

        dir_tree.insert_container_item(root, Placement::LastChild, 0);
        // dir_tree.insert_item(Rc::new(LazyTreeNode::ExpansionPlaceholder), Placement::LastChild,
        // 0);
        dir_tree.set_collapsed(0, true);

        dir_tree.set_on_collapse(get_dir_tree_on_collapse_switch_callback(false));
        dir_tree.set_on_select(dir_tree_on_select_callback);

        hl.add_child(ColorViewWrapper::new(
            BoxView::with_fixed_size((30, 15), dir_tree.with_id(DIR_TREE_VIEW_ID)),
            printer_to_theme.clone(),
        ));

        let mut file_select : SelectView<Rc<LazyTreeNode>> = SelectView::new().v_align(VAlign::Top);
        // file_select.set_on_select(file_list_on_select);
        file_select.set_on_submit(get_file_list_on_submit(variant.is_open()));
        hl.add_child(ColorViewWrapper::new(
            BoxView::with_fixed_size((50, 15), file_select.with_id(FILE_LIST_VIEW_ID)),
            printer_to_theme.clone(),
        ));

        vl.add_child(hl);

        let mut edit_view = EditView::new().filler(" ");

        match &variant {
            FileViewVariant::OpenFile(_) => edit_view.disable(),
            FileViewVariant::SaveAsFile(buffer_id, ..) => {
                edit_view.set_on_submit(on_file_edit_save_submit);
                variant.get_file_op().clone().map(|file| edit_view.set_content(file));
                vl.add_child(ColorViewWrapper::new(
                    (BoxView::with_fixed_size((80, 1), edit_view.with_id(EDIT_VIEW_ID))),
                    printer_to_theme.clone(),
                ));
            }
        };

        let file_view = FileView { variant : variant, channel : ch, mv : vl };

        IdView::<FileView>::new(FILE_VIEW_ID, file_view)
    }
}

// TODO(njskalski) maybe just use ViewWrapper?
impl View for FileView {
    fn draw(&self, printer : &Printer) {
        self.mv.draw(&printer);
    }

    fn call_on_any<'a>(&mut self, s : &Selector, cb : Box<FnMut(&mut Any) + 'a>) {
        self.mv.call_on_any(s, cb); //this view is transparent
    }

    fn on_event(&mut self, event : Event) -> EventResult {
        self.mv.on_event(event)
    }

    fn required_size(&mut self, constraint : Vec2) -> Vec2 {
        self.mv.required_size(constraint)
    }

    fn needs_relayout(&self) -> bool {
        self.mv.needs_relayout()
    }

    fn layout(&mut self, size : Vec2) {
        self.mv.layout(size)
    }

    fn focus_view(&mut self, sel : &Selector) -> Result<(), ()> {
        self.mv.focus_view(sel)
    }

    fn take_focus(&mut self, source : Direction) -> bool {
        self.mv.take_focus(source)
    }
}
