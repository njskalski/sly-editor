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

/*
Layout:

VerticalLayout:
    - TextView (title) (80,1)
    - HorizontalLayout (80, 15)
        - DirTree
        - FileSelect
    - (optionally) EditView (80, 1)
*/

const DIR_TREE_VIEW_ID: &'static str = "file_dialog_dir_tree_view";
const FILE_LIST_VIEW_ID: &'static str = "file_dialog_file_list_view";
const EDIT_VIEW_ID: &'static str = "file_dialog_edit_view";

// TODO(njskalski): add any elasticity here. It's all paper calculated
const DIR_TREE_VIEW_SIZE: Vec2 = XY::<usize> { x: 30, y: 15 };
const FILE_LIST_VIEW_SIZE: Vec2 = XY::<usize> { x: 50, y: 15 };
const EDIT_VIEW_SIZE: Vec2 = XY::<usize> { x: 80, y: 1 };

use cursive::align::*;
use cursive::direction::*;
use cursive::event::*;
use cursive::theme::*;
use cursive::vec::*;
use cursive::view::View;
use cursive::view::ViewWrapper;
use cursive::view::*;
use cursive::views::IdView;
use cursive::views::ViewRef;
use cursive::views::*;
use cursive::*;
use cursive_tree_view::*;

use core::any::Any;
use dir_tree::TreeNode;
use dir_tree::TreeNodeRef;
use std::borrow::BorrowMut;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

use color_view_wrapper::{ColorViewWrapper, PrinterModifierType};
use settings::Settings;
use std::env;
use std::path::Path;

use buffer_id::BufferId;
use dir_tree::TreeNodeVec;
use events::IChannel;
use events::IEvent;
use overlay_dialog::OverlayDialog;
use sly_view::SlyView;
use std::borrow::Borrow;
use std::cell::Ref;
use std::error;
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use view_handle::ViewHandle;
use dir_tree::LazyTreeNode;
//use lazy_dir_tree::LazyTreeNode;

// TODO(njskalski) this view took longer than anticipated to implement, so I rushed to the end
// sacrificing quality a refactor is required.
// TODO(njskalski) implement caching or remove Rcs.
// TODO(njskalski) this file is work-in-progress. Most commented code is to be reused, as
// I will need different variants of file tree / directory tree in many places.
// TODO(njskalski) add support directories outside any of selected directories?
// TODO(njskalski) add opening a proper folder and filling file field while data is provided

#[derive(Debug)]
pub enum FileDialogVariant {
    SaveAsFile(BufferId, Option<PathBuf>, Option<String>), // directory, filename
    OpenFile(Option<PathBuf>),                              // directory
}

impl FileDialogVariant {
    pub fn get_buffer_id_op(&self) -> Option<BufferId> {
        match self {
            FileDialogVariant::SaveAsFile(buffer_id, folder_op, file_op) => Some(buffer_id.clone()),
            FileDialogVariant::OpenFile(folder_op) => None,
        }
    }

    pub fn get_folder_op(&self) -> &Option<PathBuf> {
        match self {
            FileDialogVariant::SaveAsFile(buffer_id, folder_op, file_op) => folder_op,
            FileDialogVariant::OpenFile(folder_op) => folder_op,
        }
    }

    pub fn get_file_op(&self) -> &Option<String> {
        match self {
            FileDialogVariant::SaveAsFile(buffer_id, folder_op, file_op) => file_op,
            FileDialogVariant::OpenFile(folder_op) => &None,
        }
    }

    pub fn is_open(&self) -> bool {
        match self {
            FileDialogVariant::OpenFile(_) => true,
            _ => false,
        }
    }

    pub fn is_save(&self) -> bool {
        match self {
            FileDialogVariant::SaveAsFile(..) => true,
            _ => false,
        }
    }

    pub fn get_title(&self) -> &'static str {
        match self {
            FileDialogVariant::SaveAsFile(..) => "Save file",
            FileDialogVariant::OpenFile(_) => "Open file",
        }
    }
}

pub struct FileDialog {
    variant: FileDialogVariant,
    vertical_layout: LinearLayout,
    result: Option<Result<FileDialogResult, FileDialogError>>,
    handle: ViewHandle,
}

impl FileDialog {
    fn get_buffer_id_op(&self) -> Option<BufferId> {
        self.variant.get_buffer_id_op()
    }

    //    fn size(&self) -> Vec2 {
    //        Vec2::new(
    //            DIR_TREE_VIEW_SIZE.x + FILE_LIST_VIEW_SIZE.x,
    //            FILE_LIST_VIEW_SIZE.y + EDIT_VIEW_SIZE.y,
    //        )
    //    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FileDialogResult {
    Cancel,
    FileOpen(PathBuf),
    FileSave(BufferId, PathBuf),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileDialogError;

impl fmt::Display for FileDialogError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FileDialogError (not defined)")
    }
}

impl std::error::Error for FileDialogError {
    fn description(&self) -> &str {
        "FileDialogError (not defined)"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl OverlayDialog<FileDialogResult, FileDialogError> for FileDialog {
    fn is_displayed(&self) -> bool {
        self.result.is_none()
    }

    fn is_finished(&self) -> bool {
        self.result.is_some()
    }

    fn get_result(&self) -> Option<Result<FileDialogResult, FileDialogError>> {
        self.result.clone()
    }

    fn cancel(&mut self) {
        self.result = Some(Ok(FileDialogResult::Cancel))
    }
}

impl SlyView for FileDialog {
    fn handle(&self) -> ViewHandle {
        self.handle.clone()
    }
}

type TreeViewType = TreeView<TreeNodeRef>;
type SelectViewType = SelectView<TreeNodeRef>;

fn get_dir_tree_on_collapse_switch_callback(
    file_dialog_handle: ViewHandle,
    files_visible: bool,
) -> impl Fn(&mut Cursive, usize, bool, usize) -> () {
    move |siv: &mut Cursive, row: usize, is_collapsed: bool, children: usize| {
        // debug!("dir tree on collapse callback at {:}, ic = {:}. children = {:}", row,
        // is_collapsed, children);

        let mut file_dialog = get_file_dialog(siv, &file_dialog_handle);
        let mut tree_view: ViewRef<TreeViewType> = file_dialog.tree_view();
        //the line below looks complicated, but it boils down to copying Rc<LazyTreeNode>, so view
        // borrow can end immediately.
        let item = (*tree_view).borrow_item(row).unwrap().clone();

        if is_collapsed == false {
            let mut dir_vec: Vec<TreeNodeRef> = Vec::new();
            let mut file_vec: Vec<TreeNodeRef> = Vec::new();

            let children = item.children();

            if children.is_empty() {
                return;
            }

            for c in children {
                if c.is_file() {
                    file_vec.push(c);
                } else {
                    assert!(c.is_dir());
                    dir_vec.push(c);
                }
            }

            //            dir_vec.sort();
            for dir in dir_vec.iter() {
                tree_view.insert_container_item(dir.clone(), Placement::LastChild, row);
            }

            if files_visible {
                //                file_vec.sort();

                for file in file_vec.iter() {
                    tree_view.insert_item(file.clone(), Placement::LastChild, row);
                }
            }
        } else {
            // TODO(njskalski) - possible bug in cursive_tree_view: removal of these set_collapsed
            // calls leads to cursive_tree_view::draw crash. It seems like there is an
            // override of "index" variable there. Also, the repository seems outdated,
            // so I guess I should either fork it or abandon use of this view.
            if item.has_children() {
                tree_view.set_collapsed(row, false);
                tree_view.remove_children(row);
                tree_view.set_collapsed(row, true);
            }
        }
    }
}

//TODO(njskalski) add files support to work with out-of-FileDialog project-treeview
fn get_dir_tree_on_select_callback(file_dialog_handle: ViewHandle) -> impl Fn(&mut Cursive, usize) {
    move |siv: &mut Cursive, row: usize| {
        // debug!("dir tree on select callback at {:}", row);
        let mut file_dialog = get_file_dialog(siv, &file_dialog_handle);
        let mut view: ViewRef<TreeViewType> = file_dialog.tree_view();
        //the line below looks complicated, but it boils down to copying Rc<LazyTreeNode>, so view
        // borrow can end immediately.
        let item: TreeNodeRef = (*view).borrow_item(row).unwrap().clone();

        let mut file_list_view: ViewRef<SelectViewType> = file_dialog.file_list_view();
        file_list_view.clear();

        if !item.is_dir() {
            return;
        }

        let mut file_vec = TreeNodeVec::new();
        for c in item.children() {
            if c.is_file() {
                file_vec.push(c);
            }
        }

        //        file_vec.sort();
        for file in file_vec.iter() {
            file_list_view.add_item(file.to_string(), file.clone());
        }
    }
}

fn get_file_dialog(siv: &mut Cursive, file_dialog_handle: &ViewHandle) -> ViewRef<FileDialog> {
    siv.find_id::<FileDialog>(&file_dialog_handle.to_string()).unwrap()
}

fn get_file_list_on_submit(
    file_dialog_handle: ViewHandle,
    is_file_open: bool,
) -> impl Fn(&mut Cursive, &TreeNodeRef) -> () {
    move |siv: &mut Cursive, item: &TreeNodeRef| {
        // TODO(njskalski): for some reason if the line below is uncommented (and shadowing ones
        // are disabled) the unwrap inside get_path_op fails. Investigate why.
        // let mut file_view : ViewRef<FileDialog> =
        // siv.find_id::<FileDialog>(FILE_VIEW_ID).unwrap();
        if is_file_open {
            let mut file_view = get_file_dialog(siv, &file_dialog_handle);

            if item.is_file() {
                file_view.result =
                    Some(Ok(FileDialogResult::FileOpen(item.path().unwrap().to_owned())));
            } else {
                panic!("Expected only FileNodes on file_list.");
            }
        } else {
            siv.focus_id(EDIT_VIEW_ID);
            let mut edit_view: ViewRef<EditView> = siv.find_id::<EditView>(EDIT_VIEW_ID).unwrap();
            edit_view.borrow_mut().set_content(item.to_string());
        }
    }
}

fn get_path_op(siv: &mut Cursive, file_dialog_handle: &ViewHandle) -> Option<PathBuf> {
    let mut file_dialog: ViewRef<FileDialog> = get_file_dialog(siv, file_dialog_handle);
    let mut tree_view: ViewRef<TreeViewType> = file_dialog.tree_view();
    let row = tree_view.row().unwrap();
    let item = tree_view.borrow_item(row).unwrap().clone();

    if item.is_file() {
        panic!("no support for FileNodes in this tree");
    } else if item.is_root() {
        None
    } else {
        assert!(item.is_dir());
        item.path().map(|p| p.to_owned())
    }
}

fn get_on_file_edit_save_submit(file_dialog_handle: ViewHandle) -> impl Fn(&mut Cursive, &str) {
    move |siv: &mut Cursive, file_name: &str| {
        if file_name.len() == 0 {
            return;
        }

        if let Some(mut path) = get_path_op(siv, &file_dialog_handle) {
            let mut file_view: ViewRef<FileDialog> = get_file_dialog(siv, &file_dialog_handle);
            path.push(file_name);
            let buffer_id = file_view.get_buffer_id_op().unwrap();
            file_view.result = Some(Ok(FileDialogResult::FileSave(buffer_id, path)));
        } else {
            return; // no folder selected, no prefix. Just ignoring the "submit" for now.
                    // TODO(njskalski): add a popup? error message?
        };
    }
}

fn is_prefix_of(prefix : &Path, path : &Path) -> bool {
    if path.components().count() < prefix.components().count() {
        return false;
    }

    for (idx, pref_it) in prefix.components().enumerate() {
        if pref_it != path.components().skip(idx).next().unwrap() {
            return false;
        }
    }

    true

//    if prefix_components.len() < path_components.co
}

/// Will expand tree to open directory
pub fn expand_tree(tree_view : &mut TreeViewType, path : &Path) -> bool {
//    let mut tree_view = siv.find_id::<TreeViewType>(DIR_TREE_VIEW_ID).unwrap();

    let mut row_begin = 0;
    let mut row_end = tree_view.len();

    let mut done = false;

    let mut last_expansion : Option<usize> = None;

    let mut i = 0;
    loop {

        if i >= tree_view.len() {
            return false;
        }

        let item = tree_view.borrow_item(i).unwrap().clone();

        if (**item).is_root() {
            tree_view.expand_item(i);
            i += 1;
            continue;
        }

        if (**item).is_dir() {
            let item_path = (**item).path().unwrap();

            if is_prefix_of(item_path, path) {
                tree_view.expand_item(i);
                i += 1;
                continue;
            }
        }

        if (**item).is_file() {
            let item_path = (**item).path().unwrap();

            if item_path == path {
                return true;
            }

            i += 1;
        }
    }

    false
}

impl FileDialog {
    pub fn new<T>(
        ch: IChannel,
        variant: FileDialogVariant,
        root: TreeNodeRef,
        settings_ref: T,
    ) -> IdView<Self>
    where
        T: Deref<Target = Settings>,
    {
        debug!("creating file view with variant {:?}", variant);

        let handle = ViewHandle::new();

        // TODO(njskalski) implement styling with new solution when Cursive updates.

        let settings = settings_ref.borrow();

        let primary_text_color = settings.get_color("theme/file_view/primary_text_color");
        let selected_bg_color = settings.get_color("theme/file_view/selected_background");
        let non_selected_bg_color = settings.get_color("theme/file_view/non_selected_background");

        let printer_to_theme: PrinterModifierType = Rc::new(Box::new(move |p: &Printer| {
            let mut palette = theme::Palette::default();

            let theme = Theme { shadow: false, borders: BorderStyle::None, palette: palette };

            theme
        }));

        let mut vertical_layout = LinearLayout::new(Orientation::Vertical);
        let mut horizontal_layout = LinearLayout::new(Orientation::Horizontal);

        // TODO(njskalski) add a separate theme to disable color inversion effect on edit.

        let title: &'static str = variant.get_title();

        vertical_layout.add_child(ColorViewWrapper::new(
            Layer::new(TextView::new(title)),
            printer_to_theme.clone(),
        ));

        let mut dir_tree: TreeViewType = TreeView::new();

        dir_tree.insert_container_item(root, Placement::LastChild, 0);
        dir_tree.set_collapsed(0, true);

        if let Some(path) = variant.get_folder_op() {
            expand_tree(&mut dir_tree, path);
        }

        dir_tree.set_on_collapse(get_dir_tree_on_collapse_switch_callback(handle.clone(), false));
        dir_tree.set_on_select(get_dir_tree_on_select_callback(handle.clone()));

        horizontal_layout.add_child(ColorViewWrapper::new(
            BoxView::with_fixed_size(DIR_TREE_VIEW_SIZE, IdView::new(DIR_TREE_VIEW_ID, dir_tree)),
            printer_to_theme.clone(),
        ));

        let mut file_select: SelectViewType = SelectView::new().v_align(VAlign::Top);
        file_select.set_on_submit(get_file_list_on_submit(handle.clone(), variant.is_open()));

        horizontal_layout.add_child(ColorViewWrapper::new(
            BoxView::with_fixed_size(FILE_LIST_VIEW_SIZE, file_select.with_id(FILE_LIST_VIEW_ID)),
            printer_to_theme.clone(),
        ));

        vertical_layout.add_child(horizontal_layout);

        let mut edit_view = EditView::new().filler(" ");

        match &variant {
            FileDialogVariant::OpenFile(_) => edit_view.disable(),
            FileDialogVariant::SaveAsFile(buffer_id, ..) => {
                edit_view.set_on_submit(get_on_file_edit_save_submit(handle.clone()));
                variant.get_file_op().clone().map(|file| edit_view.set_content(file));
                vertical_layout.add_child(ColorViewWrapper::new(
                    (BoxView::with_fixed_size(EDIT_VIEW_SIZE, edit_view.with_id(EDIT_VIEW_ID))),
                    printer_to_theme.clone(),
                ));
            }
        };

        let file_view = FileDialog { variant, vertical_layout, result: None, handle };

        IdView::new(file_view.handle(), file_view)
    }

    fn tree_view(&mut self) -> ViewRef<TreeViewType> {
        self.vertical_layout
            .call_on(&view::Selector::Id(DIR_TREE_VIEW_ID), views::IdView::<TreeViewType>::get_mut)
            .unwrap()
    }

    fn file_list_view(&mut self) -> ViewRef<SelectViewType> {
        self.vertical_layout
            .call_on(
                &view::Selector::Id(FILE_LIST_VIEW_ID),
                views::IdView::<SelectViewType>::get_mut,
            )
            .unwrap()
    }

    fn edit_view(&mut self) -> ViewRef<EditView> {
        self.vertical_layout
            .call_on(&view::Selector::Id(EDIT_VIEW_ID), views::IdView::<EditView>::get_mut)
            .unwrap()
    }
}

// TODO(njskalski) maybe just use ViewWrapper?
impl View for FileDialog {
    fn draw(&self, printer: &Printer) {
        self.vertical_layout.draw(&printer);
    }

    fn call_on_any<'a>(&mut self, s: &Selector, cb: Box<FnMut(&mut Any) + 'a>) {
        self.vertical_layout.call_on_any(s, cb); //this view is transparent
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.vertical_layout.on_event(event)
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.vertical_layout.required_size(constraint)
    }

    fn needs_relayout(&self) -> bool {
        self.vertical_layout.needs_relayout()
    }

    fn layout(&mut self, size: Vec2) {
        self.vertical_layout.layout(size)
    }

    fn focus_view(&mut self, sel: &Selector) -> Result<(), ()> {
        self.vertical_layout.focus_view(sel)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.vertical_layout.take_focus(source)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crossbeam_channel;
    use cursive::backend::puppet::observed::ObservedCell;
    use cursive::backend::puppet::observed::ObservedScreen;
    use cursive::backend::puppet::observed_screen_view::ObservedScreenView;
    use cursive::Cursive;
    use file_dialog::FileDialog;
    use std::sync::mpsc;
    use std::sync::mpsc::Receiver;
    use std::time::Duration;

    use test_utils::basic_setup::BasicSetup;


    #[test]
    fn is_prefix_of_test() {
        assert_eq!(is_prefix_of(Path::new("/"), Path::new("/home/nj/sly/editor")), true);
        assert_eq!(is_prefix_of(Path::new("/home/"), Path::new("/home/nj/sly/editor")), true);
        assert_eq!(is_prefix_of(Path::new("/home/nj"), Path::new("/home/nj/sly/editor")), true);
        assert_eq!(is_prefix_of(Path::new("/home/nj/sly"), Path::new("/home/nj/sly/editor")), true);
        assert_eq!(is_prefix_of(Path::new("/home/nj/sly/editor"), Path::new("/home/nj/sly/editor")), true);
        assert_eq!(is_prefix_of(Path::new("/home/njs/sly"), Path::new("/home/nj/sly/editor")), false);
        assert_eq!(is_prefix_of(Path::new("/etc/"), Path::new("/home/nj/sly/editor")), false);
    }

    fn basic_setup(variant: FileDialogVariant) -> BasicSetup {
        BasicSetup::new(move |settings, ichannel | {
            FileDialog::new(ichannel, variant, settings.filesystem().clone(), settings.settings().clone())
            })
    }

    #[test]
    fn displays_open_file_header() {
        let mut s = basic_setup(FileDialogVariant::OpenFile(None));
        let screen = s.last_screen().unwrap();
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
        s.hit_keystroke(Key::Enter);

        {
            let screen = s.last_screen().unwrap();
            assert_eq!(screen.find_occurences("▸ <root>").len(), 0);
            assert_eq!(screen.find_occurences("▾ <root>").len(), 1);
        }

        {
            let screen = s.last_screen().unwrap();
            let hits = screen.find_occurences("▸");
            assert_eq!(hits.len(), 4);

            let subnodes: Vec<String> = hits
                .iter()
                .map(|hit| hit.expanded_line(0, 20).to_string().trim().to_owned())
                .collect();

            let expected = vec!["▸ laura", "▸ subdirectory2", "▸ subdirectory2", "▸ bob"];

            assert_eq!(subnodes, expected);
        }
    }

    #[test]
    fn open_file_points_to_dir() {

    }
}
