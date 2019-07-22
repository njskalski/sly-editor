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

// this is a collection of functions I expect to use in multiple places

use crate::rich_content::HighlightSettings;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

#[macro_export]
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

#[macro_export]
macro_rules! ifdebug {
    ($($arg:tt)*) => {{
        if DEBUG {
            debug!($($arg)*);
        }
    }}
}

pub fn is_prefix_of(prefix: &Path, path: &Path) -> bool {
    if path.components().count() < prefix.components().count() {
        return false;
    }

    for (idx, pref_it) in prefix.components().enumerate() {
        if pref_it != path.components().skip(idx).next().unwrap() {
            return false;
        }
    }

    true
}

//lazy_static! {
//    static ref EXT_TO_LANG_MAP : HashMap<&'static str, &'static str> = hashmap![
//        "rs" => "rust",
//        "toml" => "toml",
//        "json" => "json",
//        "cpp" => "c++",
//        "cxx" => "c++",
//        "hpp" => "c++",
//        "hxx" => "c++",
//        "c" => "c",
//        "h" => "c",
//        "go" => "go",
//        "ini" => "ini" // in this macro, trailing comma is going to break compilation.
//    ];
//}
//
//// TODO(njskalski): upgrade in 1.0
//pub fn guess_format(path : &Path) -> Option<&'static str> {
//    let extension = path.extension().and_then(OsStr::to_str);
//
//    let x = extension.and_then(|ext| EXT_TO_LANG_MAP.get(ext)).map(|x| *x);
//    x
//}

// TODO(njskalski): this should be somewhere else, but I have no brainpower to plan it now.
pub fn highlight_settings_from_path(path: &Path) -> Option<Rc<HighlightSettings>> {
    let ext = path.extension()?.to_string_lossy();
    let settings = HighlightSettings::new(&ext)?;
    Some(Rc::new(settings))
}

#[cfg(test)]
mod tests {

    use super::*;

    //    #[test]
    //    fn guess_format_test() {
    //        assert_eq!(guess_format(Path::new("/home/someone/rust.rs")), Some("rust"));
    //        assert_eq!(guess_format(Path::new("/home/someone/Cargo.toml")), Some("toml"));
    //        assert_eq!(guess_format(Path::new("/home/someone/some.json")), Some("json"));
    //    }

}
