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
use content_type::{Color, RichContent, RichLine};
use std::io::BufRead;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use unicode_segmentation::UnicodeSegmentation;
use syntect::util::LinesWithEndings;
use ropey::Rope;

//TODO(njskalski): rewrite into iterator.
// breaks into lines without swallowing trailing whitespaces (which split did)
pub fn break_into_lines(reader: &mut BufRead) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut offset: usize = 0;

    loop {
        let mut buf: String = String::new();
        let number_of_bytes = reader.read_line(&mut buf).expect("read will not fail");
        if number_of_bytes == 0 {
            break;
        }

        result.push(buf);
        offset += number_of_bytes;
    }

    if result.len() == 0 {
        result.push(String::new());
    };

    if UnicodeSegmentation::graphemes(result.last().unwrap().as_str(), true).last().unwrap() == "\n" {
        result.push(String::new());
    }

    result
}

// TODO(njskalski) test it further. There is one broken test below
fn simplify_style(style: &Style) -> Color {
    Color {
        r: style.foreground.r,
        g: style.foreground.g,
        b: style.foreground.b,
    }
}

//TODO(njskalski): optimise
pub fn rope_to_colors(rope: &Rope, line_limit : Option<usize>) -> Vec<RichLine> {
    let ss = SyntaxSet::load_defaults_nonewlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ss.find_syntax_by_extension("rs").unwrap();

    let mut lines = rope.lines();

    let mut result: Vec<RichLine> = Vec::new();

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    for line in lines {
        if line_limit.is_some() && line_limit.unwrap() >= result.len() { break; };

        // as_str on line will fail. See docs if you don't trust me.
        let line_string = line.to_string();
        let ranges: Vec<(Style, &str)> = h.highlight(&line_string, &ss);
        let new_line: Vec<(Color, String)> = ranges
            .into_iter()
            .map(|(style, words)| (simplify_style(&style), words.to_string()))
            .collect();
        result.push(RichLine {
            body: new_line,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_colors_hello_world_rs() {
        let hello = r#"fn main() {
    // Print text to the console
    println!("Hello World!");
}
"#;

        let result = rope_to_colors(hello, None);

        // print!("res {:?}", result);

        assert_eq!(result.get_lines_no(), 5); //4 lines

        let mut all_chars = 0;
        for &ref line in &result.lines {
            for word in &line.body {
                all_chars += word.1.as_bytes().len();
            }
        }

        assert_eq!(all_chars, hello.as_bytes().len());
    }

    #[test]
    fn syntax_colors_hello_world_rs_no_newline() {
        let hello = Rope::from_str(r#"fn main() {
    // Print text to the console
    println!("Hello World!");
}"#);

        let result = rope_to_colors(hello, None);
        assert_eq!(result.get_lines_no(), 4); //4 lines

        let mut all_chars = 0;
        for &ref line in &result.lines {
            for word in &line.body {
                all_chars += word.1.len();
            }
        }

        assert_eq!(all_chars, hello.as_bytes().len());
    }

    //TODO(njskalski) this test is false positive. We have 4 lines and a trailing one.
     #[test]
     fn syntax_colors_does_not_swallow_other_line_endings() {
         let hello = Rope::from_str("fn main() {\r\n\t// Print text to the console\r\n\tprintln!(\"Hello World!\");\r\n}\r\n");

         let result = rope_to_colors(hello, None);
         assert_eq!(result.get_lines_no(), 4); //4 lines

         let mut all_chars = 0;
         for &ref line in &result.lines {
             for word in &line.body {
                 all_chars += word.1.as_bytes().len();
             }
         }

         assert_eq!(all_chars, hello.as_bytes().len());
     }

    #[test]
    fn syntax_colors_tolerates_weird_chars() {
        let hello = Rope::from_str(r#"fn main() {
    // Print text to the console
    println!("Hello みんなさん!");
}"#);

        let result = rope_to_colors(hello, None);
        assert_eq!(result.get_lines_no(), 4); //4 lines

        let mut all_chars = 0;
        for &ref line in &result.lines {
            for word in &line.body {
                all_chars += word.1.len();
            }
        }

        assert_eq!(all_chars, hello.as_bytes().len());
    }
}
