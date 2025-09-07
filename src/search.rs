//! Module for search and replace functionality
//!
//! This module provides functions for searching text in buffers, finding
//! matching brackets, and replacing text with support for regex patterns.

use gtk4::TextBuffer;
use gtk4::prelude::*;
use regex::Regex;
use equivalent::Comparable;
use std::cmp::Ordering;
use crate::AppContext;
use std::rc::Rc;
use std::cell::RefCell;

/// Gets the currently selected text or word under cursor
pub fn get_selected_text_or_word(buffer: &TextBuffer) -> String {
    if let Some((start, end)) = buffer.selection_bounds() {
        // Text is selected, return the selected text
        buffer.text(&start, &end, false).to_string()
    } else {
        // No text selected, get the word under cursor
        let insert_mark = buffer.get_insert();
        let cursor_iter = buffer.iter_at_mark(&insert_mark);
        let mut start_iter = cursor_iter.clone();
        let mut end_iter = cursor_iter.clone();

        // Move to word boundaries
        if !start_iter.starts_word() {
            start_iter.backward_word_start();
        }

        if !end_iter.ends_word() {
            end_iter.forward_word_end();
        }

        buffer.text(&start_iter, &end_iter, false).to_string()
    }
}

/// Finds the next occurrence of the search text (advanced version with regex support)
pub fn find_next_advanced(
    buffer: &TextBuffer,
    search_text: &str,
    match_case: bool,
    whole_word: bool,
    use_regex: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    if use_regex {
        find_next_regex(buffer, search_text, match_case)
    } else if whole_word {
        find_next_whole_word(buffer, search_text, match_case)
    } else {
        // Get current cursor position
        let insert_mark = buffer.get_insert();
        let mut cursor_iter = buffer.iter_at_mark(&insert_mark);

        // Move one character forward to avoid matching the same text again
        cursor_iter.forward_char();

        // Search from cursor position forward
        if let Some(match_pos) =
            search_text_in_buffer(buffer, search_text, &cursor_iter, match_case, false)
        {
            return Some(match_pos);
        }

        // If not found, wrap around to the beginning
        let start_iter = buffer.start_iter();
        search_text_in_buffer(buffer, search_text, &start_iter, match_case, false)
    }
}

/// Finds the previous occurrence of the search text (advanced version with regex support)
pub fn find_previous_advanced(
    buffer: &TextBuffer,
    search_text: &str,
    match_case: bool,
    whole_word: bool,
    use_regex: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    if use_regex {
        find_previous_regex(buffer, search_text, match_case)
    } else if whole_word {
        find_previous_whole_word(buffer, search_text, match_case)
    } else {
        // Get current cursor position
        let insert_mark = buffer.get_insert();
        let cursor_iter = buffer.iter_at_mark(&insert_mark);

        // Search from cursor position backward
        if let Some(match_pos) =
            search_text_in_buffer_backward(buffer, search_text, &cursor_iter, match_case, false)
        {
            return Some(match_pos);
        }

        // If not found, wrap around to the end
        let end_iter = buffer.end_iter();
        search_text_in_buffer_backward(buffer, search_text, &end_iter, match_case, false)
    }
}

/// Searches for text in the buffer and returns the match position
fn search_text_in_buffer(
    _buffer: &TextBuffer,
    search_text: &str,
    start_iter: &gtk4::TextIter,
    match_case: bool,
    _whole_word: bool, // We handle whole word separately now
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    let flags = if match_case {
        gtk4::TextSearchFlags::VISIBLE_ONLY
    } else {
        gtk4::TextSearchFlags::VISIBLE_ONLY | gtk4::TextSearchFlags::CASE_INSENSITIVE
    };

    let iter = start_iter.clone();
    while let Some((start_match, end_match)) = iter.forward_search(search_text, flags, None) {
        return Some((start_match, end_match));
    }

    None
}

/// Searches for text in the buffer backward and returns the match position
fn search_text_in_buffer_backward(
    _buffer: &TextBuffer,
    search_text: &str,
    start_iter: &gtk4::TextIter,
    match_case: bool,
    _whole_word: bool, // We handle whole word separately now
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    let flags = if match_case {
        gtk4::TextSearchFlags::VISIBLE_ONLY
    } else {
        gtk4::TextSearchFlags::VISIBLE_ONLY | gtk4::TextSearchFlags::CASE_INSENSITIVE
    };

    let iter = start_iter.clone();
    while let Some((start_match, end_match)) = iter.backward_search(search_text, flags, None) {
        return Some((start_match, end_match));
    }

    None
}

/// Finds the next occurrence using whole word matching
fn find_next_whole_word(
    buffer: &TextBuffer,
    search_text: &str,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    // Get current cursor position
    let insert_mark = buffer.get_insert();
    let mut cursor_iter = buffer.iter_at_mark(&insert_mark);

    // Move one character forward to avoid matching the same text again
    cursor_iter.forward_char();

    // Search from cursor position forward
    if let Some(match_pos) =
        search_text_in_buffer_whole_word(buffer, search_text, &cursor_iter, match_case)
    {
        return Some(match_pos);
    }

    // If not found, wrap around to the beginning
    let start_iter = buffer.start_iter();
    search_text_in_buffer_whole_word(buffer, search_text, &start_iter, match_case)
}

/// Finds the previous occurrence using whole word matching
fn find_previous_whole_word(
    buffer: &TextBuffer,
    search_text: &str,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    // Get current cursor position
    let insert_mark = buffer.get_insert();
    let cursor_iter = buffer.iter_at_mark(&insert_mark);

    // Search from cursor position backward
    if let Some(match_pos) =
        search_text_in_buffer_whole_word_backward(buffer, search_text, &cursor_iter, match_case)
    {
        return Some(match_pos);
    }

    // If not found, wrap around to the end
    let end_iter = buffer.end_iter();
    search_text_in_buffer_whole_word_backward(buffer, search_text, &end_iter, match_case)
}

/// Searches for whole word text in the buffer and returns the match position
fn search_text_in_buffer_whole_word(
    _buffer: &TextBuffer,
    search_text: &str,
    start_iter: &gtk4::TextIter,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    let flags = if match_case {
        gtk4::TextSearchFlags::VISIBLE_ONLY
    } else {
        gtk4::TextSearchFlags::VISIBLE_ONLY | gtk4::TextSearchFlags::CASE_INSENSITIVE
    };

    let mut iter = start_iter.clone();
    while let Some((start_match, end_match)) = iter.forward_search(search_text, flags, None) {
        // Check if the match is a whole word
        if start_match.starts_word() && end_match.ends_word() {
            return Some((start_match, end_match));
        }
        // Move to the next character to continue searching
        if !iter.forward_char() {
            break;
        }
    }

    None
}

/// Searches for whole word text in the buffer backward and returns the match position
fn search_text_in_buffer_whole_word_backward(
    _buffer: &TextBuffer,
    search_text: &str,
    start_iter: &gtk4::TextIter,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    let flags = if match_case {
        gtk4::TextSearchFlags::VISIBLE_ONLY
    } else {
        gtk4::TextSearchFlags::VISIBLE_ONLY | gtk4::TextSearchFlags::CASE_INSENSITIVE
    };

    let mut iter = start_iter.clone();
    while let Some((start_match, end_match)) = iter.backward_search(search_text, flags, None) {
        // Check if the match is a whole word
        if start_match.starts_word() && end_match.ends_word() {
            return Some((start_match, end_match));
        }
        // Move to the previous character to continue searching
        if !iter.backward_char() {
            break;
        }
    }

    None
}

/// Finds the next occurrence using regex
fn find_next_regex(
    buffer: &TextBuffer,
    pattern: &str,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    // Note: This function would need access to app_context to use the cache
    // For now, we'll keep the original implementation
    match compile_regex(pattern, match_case) {
        Ok(regex) => {
            // Get current cursor position
            let insert_mark = buffer.get_insert();
            let mut cursor_iter = buffer.iter_at_mark(&insert_mark);

            // Move one character forward to avoid matching the same text again
            cursor_iter.forward_char();

            // Get the text from cursor to end
            let text = buffer
                .text(&cursor_iter, &buffer.end_iter(), false)
                .to_string();

            // Search in the text
            if let Some(mat) = regex.find(&text) {
                let start_offset = cursor_iter.offset() + mat.start() as i32;
                let end_offset = cursor_iter.offset() + mat.end() as i32;
                let start_iter = buffer.iter_at_offset(start_offset);
                let end_iter = buffer.iter_at_offset(end_offset);
                return Some((start_iter, end_iter));
            }

            // If not found, wrap around to the beginning
            let start_iter = buffer.start_iter();
            let text = buffer
                .text(&start_iter, &buffer.end_iter(), false)
                .to_string();

            if let Some(mat) = regex.find(&text) {
                let start_offset = start_iter.offset() + mat.start() as i32;
                let end_offset = start_iter.offset() + mat.end() as i32;
                let start_iter = buffer.iter_at_offset(start_offset);
                let end_iter = buffer.iter_at_offset(end_offset);
                return Some((start_iter, end_iter));
            }

            None
        }
        Err(_) => None,
    }
}

/// Finds the previous occurrence using regex
fn find_previous_regex(
    buffer: &TextBuffer,
    pattern: &str,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    // Note: This function would need access to app_context to use the cache
    // For now, we'll keep the original implementation
    match compile_regex(pattern, match_case) {
        Ok(regex) => {
            // Get current cursor position
            let insert_mark = buffer.get_insert();
            let cursor_iter = buffer.iter_at_mark(&insert_mark);

            // Get the text from start to cursor
            let text = buffer
                .text(&buffer.start_iter(), &cursor_iter, false)
                .to_string();

            // Find all matches and get the last one
            let mut last_match: Option<regex::Match> = None;
            for mat in regex.find_iter(&text) {
                last_match = Some(mat);
            }

            if let Some(mat) = last_match {
                let start_offset = buffer.start_iter().offset() + mat.start() as i32;
                let end_offset = buffer.start_iter().offset() + mat.end() as i32;
                let start_iter = buffer.iter_at_offset(start_offset);
                let end_iter = buffer.iter_at_offset(end_offset);
                return Some((start_iter, end_iter));
            }

            // If not found, wrap around to the end
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();

            // Find all matches and get the last one
            let mut last_match: Option<regex::Match> = None;
            for mat in regex.find_iter(&text) {
                last_match = Some(mat);
            }

            if let Some(mat) = last_match {
                let start_offset = buffer.start_iter().offset() + mat.start() as i32;
                let end_offset = buffer.start_iter().offset() + mat.end() as i32;
                let start_iter = buffer.iter_at_offset(start_offset);
                let end_iter = buffer.iter_at_offset(end_offset);
                return Some((start_iter, end_iter));
            }

            None
        }
        Err(_) => None,
    }
}

/// Compiles a regex pattern with optional case insensitivity
pub fn compile_regex(pattern: &str, match_case: bool) -> Result<Regex, regex::Error> {
    if match_case {
        Regex::new(pattern)
    } else {
        Regex::new(&format!("(?i){}", pattern))
    }
}

/// Replaces the current selection with replacement text (advanced version with regex support)
pub fn replace_selection_advanced(
    buffer: &TextBuffer,
    search_text: &str,
    replacement_text: &str,
    use_regex: bool,
) {
    if let Some((start, end)) = buffer.selection_bounds() {
        let mut start_mut = start;
        let mut end_mut = end;

        if use_regex {
            // For regex replacement, we need to get the matched text and apply the replacement
            let matched_text = buffer.text(&start, &end, false).to_string();
            match compile_regex(search_text, true) {
                // We don't handle case insensitivity here as it's in the pattern
                Ok(regex) => {
                    if let Some(_mat) = regex.find(&matched_text) {
                        let actual_replacement =
                            regex.replace(&matched_text, replacement_text).to_string();
                        buffer.begin_user_action();
                        buffer.delete(&mut start_mut, &mut end_mut);
                        buffer.insert(&mut start_mut, &actual_replacement);
                        buffer.end_user_action();
                    }
                }
                Err(_) => {
                    // If regex compilation fails, fall back to simple replacement
                    buffer.begin_user_action();
                    buffer.delete(&mut start_mut, &mut end_mut);
                    buffer.insert(&mut start_mut, replacement_text);
                    buffer.end_user_action();
                }
            }
        } else {
            buffer.begin_user_action();
            buffer.delete(&mut start_mut, &mut end_mut);
            buffer.insert(&mut start_mut, replacement_text);
            buffer.end_user_action();
        }
    }
}

/// Replaces all occurrences of search text with replacement text (advanced version with regex support)
pub fn replace_all_advanced(
    buffer: &TextBuffer,
    search_text: &str,
    replacement_text: &str,
    match_case: bool,
    whole_word: bool,
    use_regex: bool,
) -> u32 {
    if use_regex {
        replace_all_regex(buffer, search_text, replacement_text, match_case)
    } else if whole_word {
        replace_all_whole_word(buffer, search_text, replacement_text, match_case)
    } else {
        replace_all_simple(buffer, search_text, replacement_text, match_case)
    }
}

/// Replaces all occurrences using simple string matching
fn replace_all_simple(
    buffer: &TextBuffer,
    search_text: &str,
    replacement_text: &str,
    match_case: bool,
) -> u32 {
    let flags = if match_case {
        gtk4::TextSearchFlags::VISIBLE_ONLY
    } else {
        gtk4::TextSearchFlags::VISIBLE_ONLY | gtk4::TextSearchFlags::CASE_INSENSITIVE
    };

    let mut count = 0;
    let mut matches = Vec::new();

    // First, collect all matches without modifying the buffer
    let mut iter = buffer.start_iter();
    while let Some((start_match, end_match)) = iter.forward_search(search_text, flags, None) {
        // Store the positions as offsets instead of iterators
        let start_offset = start_match.offset();
        let end_offset = end_match.offset();
        matches.push((start_offset, end_offset));

        // Move iterator forward to continue searching
        iter = end_match;
    }

    // Now perform replacements in reverse order to maintain correct positions
    for (start_offset, end_offset) in matches.iter().rev() {
        // Convert offsets back to iterators for this specific operation
        let mut start_iter = buffer.iter_at_offset(*start_offset);
        let mut end_iter = buffer.iter_at_offset(*end_offset);

        buffer.begin_user_action();
        buffer.delete(&mut start_iter, &mut end_iter);
        let mut insert_iter = buffer.iter_at_offset(*start_offset);
        buffer.insert(&mut insert_iter, replacement_text);
        buffer.end_user_action();
        count += 1;
    }

    count
}

/// Replaces all occurrences using whole word matching
fn replace_all_whole_word(
    buffer: &TextBuffer,
    search_text: &str,
    replacement_text: &str,
    match_case: bool,
) -> u32 {
    let flags = if match_case {
        gtk4::TextSearchFlags::VISIBLE_ONLY
    } else {
        gtk4::TextSearchFlags::VISIBLE_ONLY | gtk4::TextSearchFlags::CASE_INSENSITIVE
    };

    let mut count = 0;
    let mut matches = Vec::new();

    // First, collect all matches without modifying the buffer
    let mut iter = buffer.start_iter();
    while let Some((start_match, end_match)) = iter.forward_search(search_text, flags, None) {
        // Check if the match is a whole word
        if start_match.starts_word() && end_match.ends_word() {
            // Store the positions as offsets instead of iterators
            let start_offset = start_match.offset();
            let end_offset = end_match.offset();
            matches.push((start_offset, end_offset));
        }

        // Move iterator forward to continue searching
        iter = end_match;
    }

    // Now perform replacements in reverse order to maintain correct positions
    for (start_offset, end_offset) in matches.iter().rev() {
        // Convert offsets back to iterators for this specific operation
        let mut start_iter = buffer.iter_at_offset(*start_offset);
        let mut end_iter = buffer.iter_at_offset(*end_offset);

        buffer.begin_user_action();
        buffer.delete(&mut start_iter, &mut end_iter);
        let mut insert_iter = buffer.iter_at_offset(*start_offset);
        buffer.insert(&mut insert_iter, replacement_text);
        buffer.end_user_action();
        count += 1;
    }

    count
}

/// Replaces all occurrences using regex
fn replace_all_regex(
    buffer: &TextBuffer,
    pattern: &str,
    replacement_text: &str,
    match_case: bool,
) -> u32 {
    // Note: This function would need access to app_context to use the cache
    // For now, we'll keep the original implementation but optimize the loop
    match compile_regex(pattern, match_case) {
        Ok(regex) => {
            let mut count = 0;
            let mut matches = Vec::new();

            // First, collect all matches without modifying the buffer
            let mut iter = buffer.start_iter();
            let end_iter_buffer = buffer.end_iter();

            while iter.compare(&end_iter_buffer) == Ordering::Less {
                let remaining_text = buffer.text(&iter, &end_iter_buffer, false).to_string();
                if let Some(mat) = regex.find(&remaining_text) {
                    let start_offset = iter.offset() + mat.start() as i32;
                    let end_offset = iter.offset() + mat.end() as i32;
                    matches.push((start_offset, end_offset));

                    // Advance iter past the current match to find the next one
                    iter.set_offset(end_offset);
                } else {
                    // No more matches in the remaining text
                    break;
                }
            }

            // Now perform replacements in reverse order to maintain correct positions
            buffer.begin_user_action();
            for (start_offset, end_offset) in matches.iter().rev() {
                let mut start_match_iter = buffer.iter_at_offset(*start_offset);
                let mut end_match_iter = buffer.iter_at_offset(*end_offset);

                let matched_text = buffer.text(&start_match_iter, &end_match_iter, false).to_string();
                let actual_replacement = regex.replace(&matched_text, replacement_text).to_string();

                buffer.delete(&mut start_match_iter, &mut end_match_iter);
                buffer.insert(&mut start_match_iter, &actual_replacement);
                count += 1;
            }
            buffer.end_user_action();

            count
        }
        Err(_) => 0,
    }
}

/// Compiles a regex pattern with optional case insensitivity, using a cache
pub fn compile_regex_with_cache(app_context: &Rc<RefCell<AppContext>>, pattern: &str, match_case: bool) -> Result<Regex, regex::Error> {
    let cache_key = if match_case {
        pattern.to_string()
    } else {
        format!("(?i){}", pattern)
    };
    
    // Try to get from cache first
    {
        let context = app_context.borrow();
        if let Some(cached_regex) = context.regex_cache.borrow().get(&cache_key) {
            return Ok(cached_regex.clone());
        }
    }
    
    // If not in cache, compile and store
    let regex = Regex::new(&cache_key)?;
    app_context.borrow().regex_cache.borrow_mut().insert(cache_key, regex.clone());
    Ok(regex)
}