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

// encoding # as cursor anchor

use buffer_state::BufferState;
use cursor_set::Cursor;
use cursor_set::CursorSet;
use serde::de::Unexpected::Str;
use std::borrow::Borrow;

fn get_buffer(s: &str) -> (BufferState, CursorSet) {
    let mut cursors: Vec<usize> = vec![];
    let mut text = String::new();

    for c in s.chars() {
        if c == '#' {
            cursors.push(text.len());
        } else {
            text.push(c);
        }
    }

    let cursors: Vec<Cursor> = cursors.iter().map(|a| (*a).into()).collect();

    (BufferState::from_text(&text), CursorSet::new(cursors))
}

/// TODO add support for the selection, maybe preferred column
fn buffer_cursors_to_text<T: Borrow<BufferState>>(b: T, cs: &CursorSet) -> String {
    let mut output = String::new();
    let mut anchors: Vec<usize> = cs.set().iter().map(|c| c.a).collect();
    anchors.sort();

    let buffer = b.borrow().get_content().get_lines().to_string();

    let mut prev: usize = 0;
    for a in anchors {
        output.push_str(&buffer[prev..std::cmp::min(a, buffer.len())]);
        output.push_str("#");
        prev = a;
    }
    if prev < buffer.len() {
        output.push_str(&buffer[prev..]);
    }

    output
}

fn a_to_c(anchors: Vec<usize>) -> CursorSet {
    CursorSet::new(anchors.iter().map(|a| (*a).into()).collect())
}

#[test]
fn get_buffer_test_1() {
    let (bs, cs) = get_buffer("te#xt");
    let text = bs.get_content().get_lines().to_string();

    assert_eq!(text, "text".to_owned());
    assert_eq!(cs.set().iter().map(|c| c.a).collect::<Vec<usize>>(), vec![2]);
}

#[test]
fn get_buffer_test_2() {
    let (bs, cs) = get_buffer("#text#");
    let text = bs.get_content().get_lines().to_string();

    assert_eq!(text, "text".to_owned());
    assert_eq!(cs.set().iter().map(|c| c.a).collect::<Vec<usize>>(), vec![0, 4]);
}

#[test]
fn buffer_cursors_to_text_1() {
    let cursors = a_to_c(vec![2]);
    let buffer = BufferState::from_text("text");

    let output = buffer_cursors_to_text(&buffer, &cursors);

    assert_eq!(output, "te#xt".to_owned());
}

#[test]
fn buffer_cursors_to_text_2() {
    let cursors = a_to_c(vec![0, 4]);
    let buffer = BufferState::from_text("text");

    let output = buffer_cursors_to_text(&buffer, &cursors);

    assert_eq!(output, "#text#".to_owned());
}

fn apply(input: &str, f: fn(&mut CursorSet, &str) -> ()) -> String {
    let (bs, mut cs) = get_buffer(input);
    f(&mut cs, &input);
    dbg!(&cs);
    buffer_cursors_to_text(&bs, &cs)
}

#[test]
fn one_cursor_move_left() {
    let f : fn(&mut CursorSet, &str) = |c: &mut CursorSet, _| {
        c.move_left();
    };

    assert_eq!(apply("text", f), "text");
    assert_eq!(apply("te#xt", f), "t#ext");
    assert_eq!(apply("t#ext", f), "#text");
    assert_eq!(apply("#text", f), "#text");
    assert_eq!(apply("text\n#", f), "text#\n");
}

#[test]
fn multiple_cursor_move_left() {
    let f : fn(&mut CursorSet, &str) = |c: &mut CursorSet, _| {
        c.move_left();
        c.reduce();
    };

    assert_eq!(apply("te#x#t", f), "t#e#xt");
    assert_eq!(apply("#t#ext", f), "#text");
    assert_eq!(apply("#text\n#", f), "#text#\n");
}


#[test]
fn one_cursor_move_right() {
    let f : fn(&mut CursorSet, &str) = |c: &mut CursorSet, s| {
        let bs = BufferState::from_text(s);
        c.move_right(&bs);
        c.reduce();
    };

    assert_eq!(apply("text", f), "text");
    assert_eq!(apply("te#xt", f), "tex#t");
    assert_eq!(apply("t#ext", f), "te#xt");
    assert_eq!(apply("#text", f), "t#ext");
    assert_eq!(apply("text\n#", f), "text\n#");
}
#[test]
fn multiple_cursor_move_right() {
    let f : fn(&mut CursorSet, &str) = |c: &mut CursorSet, s| {
        let bs = BufferState::from_text(s);
        c.move_right(&bs);
        c.reduce();
    };

    assert_eq!(apply("te#x#t", f), "tex#t#");
    assert_eq!(apply("#t#ext", f), "t#e#xt");
    assert_eq!(apply("#text\n#", f), "t#ext\n#");
    assert_eq!(apply("text#\n#", f), "text\n#");
}