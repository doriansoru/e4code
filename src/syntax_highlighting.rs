//! Module for syntax highlighting functionality
//!
//! This module provides functions for applying syntax highlighting to text buffers
//! using the syntect library, as well as bracket matching and highlighting.

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{TextBuffer, TextIter, TextTag};
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use std::cell::RefCell;
use std::rc::Rc;

/// Context for syntax highlighting, holding all necessary components.
pub struct SyntaxHighlightingContext {
    /// Syntax set for syntax highlighting
    pub ps: Rc<SyntaxSet>,
    /// Theme set for syntax highlighting
    pub ts: Rc<ThemeSet>,
    /// Current syntax reference
    pub syntax: Rc<SyntaxReference>,
    /// Current theme for syntax highlighting
    pub current_theme: Rc<RefCell<Theme>>,
    /// Function to apply syntax highlighting
    pub highlight_closure: Rc<dyn Fn(TextBuffer)>,
}

impl SyntaxHighlightingContext {
    /// Creates a new `SyntaxHighlightingContext`.
    pub fn new(
        ps: Rc<SyntaxSet>,
        ts: Rc<ThemeSet>,
        syntax: Rc<SyntaxReference>,
        current_theme: Rc<RefCell<Theme>>,
        highlight_closure: Rc<dyn Fn(TextBuffer)>,
    ) -> Self {
        Self {
            ps,
            ts,
            syntax,
            current_theme,
            highlight_closure,
        }
    }
}


/// Applies syntax highlighting to a text buffer
///
/// This function uses the syntect library to apply syntax highlighting to the
/// entire contents of a text buffer based on the provided syntax and theme.
///
/// # Arguments
///
/// * `buffer` - The text buffer to apply syntax highlighting to
/// * `syntax` - Reference to the syntax definition to use
/// * `ps` - Reference to the syntax set
/// * `theme` - Reference to the theme to use for highlighting
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

    // syntect for syntax highlighting
    let mut h = syntect::easy::HighlightLines::new(syntax, theme);
    for (line_num, line) in text.lines().enumerate() {
        if let Ok(ranges) = h.highlight_line(line, ps) {
            let mut current_offset = 0;
            for (style, chunk) in ranges {
                if let (Some(start_iter), Some(end_iter)) = (
                    buffer.iter_at_line_offset(line_num as i32, current_offset as i32),
                    buffer.iter_at_line_offset(
                        line_num as i32,
                        (current_offset + chunk.chars().count()) as i32,
                    ),
                ) {
                    let tag_name = format!(
                        "fg_{:02x}{:02x}{:02x}{:02x}_bg_{:02x}{:02x}{:02x}{:02x}",
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                        style.foreground.a,
                        style.background.r,
                        style.background.g,
                        style.background.b,
                        style.background.a
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
                        if style.background.r != 0
                            || style.background.g != 0
                            || style.background.b != 0
                            || style.background.a != 0
                        {
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

/// Applies incremental syntax highlighting to a specific range of lines in a text buffer
///
/// This function updates syntax highlighting only for the specified range of lines,
/// making it more efficient for handling edits.
///
/// # Arguments
///
/// * `buffer` - The text buffer to apply syntax highlighting to
/// * `syntax` - Reference to the syntax definition to use
/// * `ps` - Reference to the syntax set
/// * `theme` - Reference to the theme to use for highlighting
/// * `start_line` - The first line to highlight (inclusive)
/// * `end_line` - The last line to highlight (inclusive)
pub fn apply_incremental_syntax_highlighting(
    buffer: &TextBuffer,
    syntax: &syntect::parsing::SyntaxReference,
    ps: &SyntaxSet,
    theme: &Theme,
    start_line: i32,
    end_line: i32,
) {
    // Ensure valid line range
    let start_line = start_line.max(0);
    let buffer_line_count = buffer.line_count();
    let end_line = end_line.min(buffer_line_count - 1);
    
    if start_line > end_line {
        return;
    }

    let tag_table = buffer.tag_table();

    // Remove syntect tags from the specified range
    if let (Some(start_iter), Some(end_iter)) = (
        buffer.iter_at_line(start_line),
        if end_line + 1 < buffer_line_count {
            buffer.iter_at_line(end_line + 1)
        } else {
            Some(buffer.end_iter())
        },
    ) {
        let mut tags_to_remove = Vec::new();
        tag_table.foreach(|tag| {
            if let Some(name) = tag.name() {
                if name.starts_with("fg_") {
                    tags_to_remove.push(tag.clone());
                }
            }
        });
        for tag in tags_to_remove {
            buffer.remove_tag(&tag, &start_iter, &end_iter);
        }
    }

    // syntect for syntax highlighting
    let mut h = syntect::easy::HighlightLines::new(syntax, theme);
    for line_num in 0..buffer_line_count {
        // Get the line text
        let line_start = buffer.iter_at_line(line_num).unwrap();
        let line_end = if line_num + 1 < buffer_line_count {
            buffer.iter_at_line(line_num + 1).unwrap()
        } else {
            buffer.end_iter()
        };
        let line_text = buffer.text(&line_start, &line_end, false);
        
        // Only highlight lines in the specified range
        if line_num >= start_line && line_num <= end_line {
            if let Ok(ranges) = h.highlight_line(&line_text, ps) {
                let mut current_offset = 0;
                for (style, chunk) in ranges {
                    if let (Some(start_iter), Some(end_iter)) = (
                        buffer.iter_at_line_offset(line_num, current_offset),
                        buffer.iter_at_line_offset(
                            line_num,
                            current_offset + chunk.chars().count() as i32,
                        ),
                    ) {
                        let tag_name = format!(
                            "fg_{:02x}{:02x}{:02x}{:02x}_bg_{:02x}{:02x}{:02x}{:02x}",
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                            style.foreground.a,
                            style.background.r,
                            style.background.g,
                            style.background.b,
                            style.background.a
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
                            if style.background.r != 0
                                || style.background.g != 0
                                || style.background.b != 0
                                || style.background.a != 0
                            {
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
                    current_offset += chunk.chars().count() as i32;
                }
            }
        } else {
            // For lines outside the range, just parse to maintain state
            let _ = h.highlight_line(&line_text, ps);
        }
    }
}

/// Updates bracket highlighting in a text view
///
/// This function finds matching brackets and applies highlighting to both
/// the bracket at the cursor position and its matching counterpart.
///
/// # Arguments
///
/// * `text_view` - The text view to update bracket highlighting for
/// * `find_matching_bracket_fn` - Function to find matching brackets
/// * `prev_bracket_pos1` - Reference to store the position of the first bracket
/// * `prev_bracket_pos2` - Reference to store the position of the second bracket
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
                buffer.remove_tag(&tag, &prev_pos1, &{
                    let mut i = prev_pos1.clone();
                    i.forward_char();
                    i
                });
                buffer.remove_tag(&tag, &prev_pos2, &{
                    let mut i = prev_pos2.clone();
                    i.forward_char();
                    i
                });
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
        buffer.apply_tag(&tag, &iter, &{
            let mut i = iter.clone();
            i.forward_char();
            i
        });
        buffer.apply_tag(&tag, &matching_iter, &{
            let mut i = matching_iter.clone();
            i.forward_char();
            i
        });

        // Store current bracket positions
        *prev_bracket_pos1.borrow_mut() = Some(iter);
        *prev_bracket_pos2.borrow_mut() = Some(matching_iter);
    } else {
        // No match found, clear stored positions
        *prev_bracket_pos1.borrow_mut() = None;
        *prev_bracket_pos2.borrow_mut() = None;
    }
}

/// Finds matching brackets in a text buffer
///
/// This function looks for a matching bracket for the character at the
/// provided iterator position. It supports parentheses, square brackets,
/// and curly braces.
///
/// # Arguments
///
/// * `iter` - Iterator positioned at the bracket to find a match for
/// * `_buffer` - The text buffer (unused in current implementation)
///
/// # Returns
///
/// An iterator positioned at the matching bracket, or None if no match found
pub fn find_matching_bracket(
    iter: &gtk4::TextIter,
    _buffer: &gtk4::TextBuffer,
) -> Option<gtk4::TextIter> {
    let char_at_iter = iter.char();

    let (open_bracket, close_bracket, forward) = match char_at_iter {
        '(' => (Some('('), Some(')'), true),
        ')' => (Some('('), Some(')'), false),
        '[' => (Some('['), Some(']'), true),
        ']' => (Some('['), Some(']'), false),
        '{' => (Some('{'), Some('}'), true),
        '}' => (Some('{'), Some('}'), false),
        _ => (None, None, false),
    };

    if open_bracket.is_none() {
        return None;
    }

    let mut search_iter = iter.clone();
    let mut stack_depth = 1;

    if forward {
        while search_iter.forward_char() {
            let current_char = search_iter.char();
            if current_char == open_bracket.unwrap() {
                stack_depth += 1;
            } else if current_char == close_bracket.unwrap() {
                stack_depth -= 1;
                if stack_depth == 0 {
                    return Some(search_iter);
                }
            }
        }
    } else {
        while search_iter.backward_char() {
            let current_char = search_iter.char();
            if current_char == close_bracket.unwrap() {
                stack_depth += 1;
            } else if current_char == open_bracket.unwrap() {
                stack_depth -= 1;
                if stack_depth == 0 {
                    return Some(search_iter);
                }
            }
        }
    }

    None
}
