use gtk4::{TextBuffer, TextTag};
use gtk4::prelude::*;
use gtk4::gdk;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::Theme;

/// Applies syntax highlighting to a text buffer
pub fn apply_syntax_highlighting(
    buffer: &TextBuffer,
    syntax: &syntect::parsing::SyntaxReference,
    ps: &SyntaxSet,
    theme: &Theme,
) {
    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
    let tag_table = buffer.tag_table();

    // Removes only syntect tags (diagnostics, highlight)
    let mut tags_to_remove = Vec::new();
    tag_table.foreach(|tag| {
        if let Some(name) = tag.name() {
            if name.starts_with("fg_") {
               tags_to_remove.push(tag.clone());
            }
        }
    });
    for tag in tags_to_remove {
        buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
    }

    // Fallback to syntect for non-Rust files
    let mut h = syntect::easy::HighlightLines::new(syntax, theme);
    for (line_num, line) in text.lines().enumerate() {
        if let Ok(ranges) = h.highlight_line(line, ps) {
            let mut current_offset = 0;
            for (style, chunk) in ranges {
                if let (Some(start_iter), Some(end_iter)) = (
                    buffer.iter_at_line_offset(line_num as i32, current_offset as i32),
                    buffer.iter_at_line_offset(line_num as i32, (current_offset + chunk.chars().count()) as i32)
                ) {
                    let tag_name = format!("fg_{:02x}{:02x}{:02x}{:02x}_bg_{:02x}{:02x}{:02x}{:02x}",
                        style.foreground.r, style.foreground.g, style.foreground.b, style.foreground.a,
                        style.background.r, style.background.g, style.background.b, style.background.a
                    );
                    let tag = if let Some(existing_tag) = tag_table.lookup(&tag_name) {
                        existing_tag
                    } else {
                        let new_tag = TextTag::new(Some(&tag_name));
                        // Set foreground color
                        new_tag.set_foreground_rgba(Some(&gdk::RGBA::new(
                            style.foreground.r as f32 / 255.0,
                            style.foreground.g as f32 / 255.0,
                            style.foreground.b as f32 / 255.0,
                            style.foreground.a as f32 / 255.0,
                        )));
                        // Set background color if different from default
                        if style.background.r != 0 || style.background.g != 0 || style.background.b != 0 || style.background.a != 0 {
                            new_tag.set_background_rgba(Some(&gdk::RGBA::new(
                                style.background.r as f32 / 255.0,
                                style.background.g as f32 / 255.0,
                                style.background.b as f32 / 255.0,
                                style.background.a as f32 / 255.0,
                            )));
                        }
                        tag_table.add(&new_tag);
                        new_tag
                    };
                    buffer.apply_tag(&tag, &start_iter, &end_iter);
                }
                current_offset += chunk.chars().count();
            }
        }
    }
}

/// Updates bracket highlighting in a text view
use std::rc::Rc;
use std::cell::RefCell;
use gtk4::TextIter;

pub fn update_bracket_highlighting(
    text_view: &gtk4::TextView,
    find_matching_bracket_fn: fn(&gtk4::TextIter, &TextBuffer) -> Option<gtk4::TextIter>,
    prev_bracket_pos1: &Rc<RefCell<Option<TextIter>>>,
    prev_bracket_pos2: &Rc<RefCell<Option<TextIter>>>,
) {
    let buffer = text_view.buffer();
    let cursor_mark = buffer.get_insert();
    let iter = buffer.iter_at_mark(&cursor_mark);

    // Remove previous bracket highlights
    let tag_table = buffer.tag_table();
    if let Some(tag) = tag_table.lookup("bracket_match") {
        if let Some(prev_pos1) = prev_bracket_pos1.borrow_mut().take() {
            if let Some(prev_pos2) = prev_bracket_pos2.borrow_mut().take() {
                buffer.remove_tag(&tag, &prev_pos1, &{let mut i = prev_pos1.clone(); i.forward_char(); i});
                buffer.remove_tag(&tag, &prev_pos2, &{let mut i = prev_pos2.clone(); i.forward_char(); i});
            }
        }
    }

    if let Some(matching_iter) = find_matching_bracket_fn(&iter, &buffer) {
        let tag = if let Some(tag) = tag_table.lookup("bracket_match") {
            tag
        } else {
            let new_tag = gtk4::TextTag::new(Some("bracket_match"));
            new_tag.set_scale(1.3);
            new_tag.set_weight(700);

            tag_table.add(&new_tag);
            new_tag
        };
        buffer.apply_tag(&tag, &iter, &{let mut i = iter.clone(); i.forward_char(); i});
        buffer.apply_tag(&tag, &matching_iter, &{let mut i = matching_iter.clone(); i.forward_char(); i});

        // Store current bracket positions
        *prev_bracket_pos1.borrow_mut() = Some(iter);
        *prev_bracket_pos2.borrow_mut() = Some(matching_iter);
    } else {
        // No match found, clear stored positions
        *prev_bracket_pos1.borrow_mut() = None;
        *prev_bracket_pos2.borrow_mut() = None;
    }
}