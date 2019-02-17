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

// TODO(njskalski) fix known keys to static constant, fix String to static str etc.
// TODO(njskalski) maybe change names of some traits/structs
// TODO(njskalski) add validation as a comparison between default settings and user overrides.
// TODO(njskalski) add validation if commands are known (plugins must be loaded first)
// TODO(njskalski) parse more keys.

use action::Action;
use cursive;
use cursive::event::{Event, Key};
use cursive::theme;
use default_settings::*;
use fuzzy_view_item::ViewItem;
use log;
use serde_json as sj;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind, Read};
use std::iter::FromIterator;
use std::rc::Rc;

pub type KeybindingsType = HashMap<Event, String>;

fn get_known_keys() -> HashSet<String> {
    let mut known_keys: HashSet<String> = HashSet::new();

    for s in vec!["ctrl", "alt", "shift", "backspace", "delete", "esc"] {
        known_keys.insert(s.to_string());
    }

    let alphabet = (b'A'..b'z' + 1) // Start as u8
        .map(|c| c as char) // Convert all to chars
        .filter(|c| c.is_alphabetic())
        .collect::<Vec<_>>();

    for c in alphabet {
        known_keys.insert(c.to_string());
    }

    known_keys
}

fn color_hex_to_rgb(hex: &str) -> Result<theme::Color, Error> {
    if hex.len() != 7 {
        Err(Error::new(ErrorKind::Other, format!("Error parsing color \"{:?}\".", hex)))
    } else {
        let ro = u8::from_str_radix(&hex[1..3], 16);
        let go = u8::from_str_radix(&hex[3..5], 16);
        let bo = u8::from_str_radix(&hex[5..7], 16);

        //        debug!("parsing color {:?} {:?} {:?}", ro, go, bo);
        match (ro, go, bo) {
            (Ok(r), Ok(g), Ok(b)) => Ok(theme::Color::Rgb(r, g, b)),
            _ => Err(Error::new(ErrorKind::Other, format!("Error parsing color \"{:?}\".", hex))),
        }
    }
}

pub struct Settings {
    tree: sj::Value,
    color_cache: RefCell<HashMap<&'static str, cursive::theme::Color>>,
    auto_highlighting: bool,
    file_index_limit: usize,
}

impl Settings {
    fn get_value(&self, selector: &'static str) -> Option<&sj::Value> {
        let mut ptr: Option<&sj::Value> = Some(&self.tree);
        for lane in selector.split('/') {
            ptr = ptr.map(|subtree| &subtree[lane]);
        }
        ptr
    }

    pub fn get_color(&self, selector: &'static str) -> cursive::theme::Color {
        match self.color_cache.borrow().get(selector) {
            Some(color) => return color.clone(),
            None => (),
        }

        let color = match self.get_value(selector) {
            Some(&sj::Value::String(ref color_string)) => color_hex_to_rgb(color_string.as_str())
                .expect(&format!("failed parsing color {:?} : {:?}", selector, color_string)),
            anything_else => panic!(
                "expected color, got {:?} in path {:?} (or earlier)",
                anything_else, selector
            ),
        };

        self.color_cache.borrow_mut().insert(selector, color);

        color
    }

    pub fn auto_highlighting_enabled(&self) -> bool {
        self.auto_highlighting
    }

    // TODO(njskalski) I decided not to use Cursive's palette mechanism, because most views will be
    // using more than the default number of colors. So this method is obsolete.
    pub fn get_palette(&self) -> theme::Palette {
        let mut palette: theme::Palette = theme::Palette::default();

        palette[theme::PaletteColor::Background] =
            self.get_color("theme/text_view/background_color");
        palette[theme::PaletteColor::View] = palette[theme::PaletteColor::Background];
        palette[theme::PaletteColor::Primary] =
            self.get_color("theme/text_view/primary_text_color");
        palette[theme::PaletteColor::Secondary] =
            self.get_color("theme/text_view/secondary_text_color");

        palette
    }

    pub fn get_colorstyle(
        &self,
        front_selector: &'static str,
        background_selector: &'static str,
    ) -> cursive::theme::ColorStyle {
        cursive::theme::ColorStyle {
            front: cursive::theme::ColorType::Color(self.get_color(front_selector)),
            back: cursive::theme::ColorType::Color(self.get_color(background_selector)),
        }
    }

    pub fn file_index_limit(&self) -> usize {
        self.file_index_limit
    }

    pub fn get_keybindings(&self, context: &str) -> KeybindingsType {
        let known_keys = get_known_keys();

        let text_bindings: &sj::map::Map<String, sj::Value> =
            match self.tree["keybindings"][context] {
                sj::Value::Object(ref map) => map,
                _ => panic!("settings/keybindings/text is not an sj::Object!"),
            };

        let mut result: KeybindingsType = HashMap::new();

        for (name, object) in text_bindings.iter() {
            let option_name: &String = name;
            let option_keys: &Vec<sj::Value> = match object {
                &sj::Value::Array(ref items) => items,
                _ => panic!("settings/keybindings/text/{:?} is not an array!", option_name),
            };

            if option_keys.len() == 0 {
                panic!(
                    "settings/keybindings/text/{:?} cannot assign empty key combination.",
                    option_name
                );
            }

            let keys: Vec<&String> = option_keys
                .iter()
                .enumerate()
                .map(|(i, ref value)| {
                    match value {
                        &&sj::Value::String(ref s) => {
                            if !known_keys.contains(s) {
                                panic!(
                                    "settings/keybindings/text/{:?}/#{:?} (0 based) - unknown key \
                                     \"{:?}\"!",
                                    option_name, i, s
                                );
                            };
                            // if i == (option_keys.len() -1) && s.len() != 1 {
                            //     panic!("settings/keybindings/text/{:?}/#{:?} (0 based) - it is
                            // expected (for now) that the last key is always a letter, and got
                            // \"{:?}\"", option_name, i, s); }
                            s
                        }
                        _ => panic!(
                            "settings/keybindings/text/{:?}/#{:?} (0 based) is not a string!",
                            option_name, i
                        ),
                    }
                })
                .collect();

            let ctrl_in = keys.contains(&&"ctrl".to_string());
            let shift_in = keys.contains(&&"shift".to_string());
            let alt_in = keys.contains(&&"alt".to_string());

            let last_str: &String = keys.last().unwrap();
            let letter: char = last_str.chars().last().unwrap();

            let event = match (shift_in, alt_in, ctrl_in, last_str.as_str()) {
                (_, _, _, "esc") => Event::Key(Key::Esc),
                (false, false, true, "c") => Event::Exit, //this is special case
                (false, false, false, _) => Event::Char(letter),
                (false, true, false, _) => Event::AltChar(letter),
                (false, false, true, _) => Event::CtrlChar(letter),
                _ => panic!("unsupported key combination = {:?} (now).", option_keys),
            };

            // debug!("assigning {:?} to action {:?}", event, option_name);
            result.insert(event, option_name.clone());
        }

        result
    }

    pub fn load_default() -> Self {
        let mut default_settings = get_default_settings();
        Self::load(&mut default_settings.as_bytes()).expect("failed loading settings. Parse error?")
    }

    pub fn load(reader: &mut Read) -> Option<Self> {
        let settings_result = sj::from_reader::<_, sj::Value>(reader);

        let tree: Option<sj::Value> = match settings_result {
            Err(some_error) => {
                debug!("{:?}", some_error);
                log::logger().flush();
                None
            }
            Ok(s) => Some(s),
        };

        if tree.is_none() {
            return None;
        }
        let tree = tree.unwrap();

        let auto_highlighting = tree
            .get("performance")
            .and_then(|node| node.get("auto_highlighting"))
            .and_then(|node| node.as_bool());

        if auto_highlighting.is_none() {
            return None;
        };
        let auto_highlighting = auto_highlighting.unwrap();

        let file_index_limit = tree
            .get("performance")
            .and_then(|node| node.get("max_files_indexed"))
            .and_then(|node| node.as_u64())
            .unwrap() as usize;

        debug!("file index limit {}", file_index_limit);

        Some(Settings {
            tree: tree,
            color_cache: RefCell::new(HashMap::new()),
            auto_highlighting: auto_highlighting,
            file_index_limit: file_index_limit,
        })
    }

    pub fn add_text_actions(&self, actions: &mut Vec<Action>) {
        let yaml = load_yaml!("actions/text.yaml");
        let mut text_actions = Action::from_yaml(yaml);
        let keybindings = self.get_keybindings("text");


        actions.append(&mut text_actions);
    }

    // TODO(njskalski): add cache.
    pub fn get_actions(&self, contexts: &Vec<&str>) -> Vec<Rc<ViewItem>> {
        let mut actions: Vec<Action> = vec![];

        self.add_text_actions(&mut actions);

        let mut result: Vec<Rc<ViewItem>> = vec![];

        result
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn default_settings_parses() {
        let settins = Settings::load_default();
    }

    #[test]
    fn get_actions_text() {
        let settings = Settings::load_default();
        settings.get_actions(&vec!["text"]);
    }
}
