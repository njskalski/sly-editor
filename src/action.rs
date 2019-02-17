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

use fuzzy_view_item::ViewItem;
use keyboard_shortcut::KeyboardShortcut;
use yaml_rust::yaml::Yaml;

#[derive(Clone, Debug)]
pub struct Action {
    marker: String,
    desc: Option<String>,
    keyboard_shortcut: Option<KeyboardShortcut>,
}

impl Action {
    pub fn view_item(&self) -> ViewItem {
        ViewItem::new(
            self.marker.clone(),
            None,
            self.marker.clone(),
            self.keyboard_shortcut.clone(),
        )
    }

    pub fn new(marker: String) -> Self {
        Action { marker, desc: None, keyboard_shortcut: None }
    }

    pub fn with_desc(self, desc: String) -> Self {
        Action { marker: self.marker, desc: Some(desc), keyboard_shortcut: self.keyboard_shortcut }
    }

    pub fn with_ks(self, ks: KeyboardShortcut) -> Self {
        Action { marker: self.marker, desc: self.desc, keyboard_shortcut: Some(ks) }
    }

    pub fn set_ks(&mut self, ks: Option<KeyboardShortcut>) {
        self.keyboard_shortcut = ks;
    }

    pub fn marker(&self) -> &String {
        &self.marker
    }

    pub fn desc(&self) -> Option<&String> {
        self.desc.as_ref()
    }

    pub fn keyboard_shortcut(&self) -> Option<&KeyboardShortcut> {
        self.keyboard_shortcut.as_ref()
    }

    // TODO: any error notification?
    pub fn from_yaml(yaml: &yaml_rust::yaml::Yaml) -> Vec<Action> {
        let mut result: Vec<Action> = vec![];
        //        dbg!(yaml);
        for (marker, item) in yaml["actions"].as_hash().expect("actions is not a hash") {
            let marker = marker.as_str().expect("marker is not a string");
            result.push(Action::new(marker.to_owned()))
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use action::Action;
    use yaml_rust::YamlLoader;

    const basic_yaml: &'static str = r###"
actions:
  copy:
  paste:
  undo:
  redo:
"###;

    #[test]
    fn action_yaml_basic() {
        let yaml = &YamlLoader::load_from_str(basic_yaml).unwrap()[0];
        let actions = Action::from_yaml(&yaml);

        assert_eq!(actions.len(), 4);

        let action_markers: Vec<&str> = actions.iter().map(|a| a.marker().as_str()).collect();
        assert_eq!(action_markers, vec!["copy", "paste", "undo", "redo"]);
    }
}
