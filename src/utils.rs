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

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

#[macro_export]
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

/// this method takes a string representing a path, returns pair of options describing
/// folder path and filename. Does not check if file exists, so it cannot differentiate between
/// files and directories, unless path ends with "/", in which case it is assumed it's a directory.
pub fn path_string_to_pair(path_str : String) -> (Option<String>, Option<String>) {
    if path_str.ends_with('/') {
        (Some(path_str[..path_str.len() - 1].to_string()), None)
    } else {
        match path_str.rfind("/") {
            None => (None, Some(path_str)),
            Some(last_slash) => {
                let folder = &path_str[..last_slash];
                let file = &path_str[last_slash + 1..];

                (
                    if folder.len() > 0 { Some(folder.to_string()) } else { None },
                    if file.len() > 0 { Some(file.to_string()) } else { None },
                )
            }
        }
    }
}

lazy_static! {
    static ref EXT_TO_LANG_MAP : HashMap<&'static str, &'static str> = hashmap![
        "rs" => "rust",
        "toml" => "toml",
        "json" => "json",
        "cpp" => "c++",
        "cxx" => "c++",
        "hpp" => "c++",
        "hxx" => "c++",
        "c" => "c",
        "h" => "c",
        "ini" => "ini" // in this macro, trailing comma is going to break compilation.
    ];
}

// TODO(njskalski): upgrade in 1.0
pub fn guess_format(path : &Path) -> Option<&'static str> {
    let extension = path.extension().and_then(OsStr::to_str);

    let x = extension.and_then(|ext| EXT_TO_LANG_MAP.get(ext)).map(|x| *x);
    x
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn path_string_to_pair_works() {
        assert_eq!(path_string_to_pair("/bin/".to_string()), (Some("/bin".to_string()), None));
        assert_eq!(
            path_string_to_pair("/bin/tmp".to_string()),
            (Some("/bin".to_string()), Some("tmp".to_string()))
        );
        assert_eq!(
            path_string_to_pair("/bin/tmp/".to_string()),
            (Some("/bin/tmp".to_string()), None)
        );
    }

    #[test]
    fn guess_format_test() {
        assert_eq!(guess_format(&PathBuf::new("/home/someone/rust.rs").as_path()), Some("rust"));
        assert_eq!(guess_format(&PathBuf::new("/home/someone/Cargo.toml").as_path()), Some("toml"));
        assert_eq!(guess_format(&PathBuf::new("/home/someone/some.json").as_path()), Some("json"));
    }

}
