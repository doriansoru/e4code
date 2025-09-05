use gtk4::{TextBuffer};
use gtk4::prelude::*;
use std::collections::HashMap;

// Helper function to detect indentation style
// Returns (is_tab_indent, indent_width_if_spaces)
fn detect_indent_style(buffer: &TextBuffer) -> (bool, usize) {
    let mut tab_lines = 0;
    let mut space_indent_counts = Vec::new();

    let mut start_iter = buffer.start_iter();
    let mut line_count = 0;

    // Removed: && start_iter.is_valid()
    while line_count < 20 {
        let mut end_iter = start_iter.clone();
        end_iter.forward_to_line_end();
        let line_text = buffer.text(&start_iter, &end_iter, false).to_string();

        if !line_text.trim().is_empty() {
            if line_text.starts_with('\t') { 
                tab_lines += 1;
            } else if line_text.starts_with(' ') { // Corrected: single space character
                let leading_spaces = line_text.chars().take_while(|&c| c == ' ').count();
                if leading_spaces > 0 {
                    space_indent_counts.push(leading_spaces);
                }
            }
        }

        line_count += 1;
        if !start_iter.forward_line() { // Check if forward_line was successful
            break;
        }
    }

    if tab_lines > space_indent_counts.len() {
        (true, 0) // Tab indentation
    } else if !space_indent_counts.is_empty() {
        // Determine common space indent width
        let mut counts_map: HashMap<usize, usize> = HashMap::new();
        for &count in &space_indent_counts {
            *counts_map.entry(count).or_insert(0) += 1;
        }

        // Find the most frequent space count
        let mut max_count = 0;
        let mut indent_width = 4; // Default if no clear winner

        for (&width, &count) in &counts_map {
            if count > max_count {
                max_count = count; // Update max_count
                indent_width = width; // Update indent_width
            }
        }
        (false, indent_width) // Space indentation
    } else {
        (false, 4) // Default to 4 spaces
    }
}

pub fn indent_selection(buffer: &TextBuffer) {
    let (is_tab_indent, indent_width) = detect_indent_style(buffer);
    let indent_string = if is_tab_indent {
        "\t".to_string()
    } else {
        " ".repeat(indent_width)
    };

    let mut start_iter;
    let mut end_iter;
    let has_selection = buffer.selection_bounds().is_some();

    if has_selection {
        (start_iter, end_iter) = buffer.selection_bounds().unwrap();
    } else {
        start_iter = buffer.iter_at_mark(&buffer.get_insert());
        start_iter.set_line_offset(0);
        end_iter = start_iter.clone();
        end_iter.forward_to_line_end();
    }

    let start_line = start_iter.line();
    let end_line = end_iter.line(); // Get the line number of the end of the selection

    // Store the original selection marks to restore them later
    let original_selection_start_mark = if has_selection {
        Some(buffer.create_mark(None, &start_iter, false))
    } else {
        None
    };
    let original_selection_end_mark = if has_selection {
        Some(buffer.create_mark(None, &end_iter, false))
    } else {
        None
    };

    buffer.begin_user_action();

    // Iterate from bottom to top
    for current_line_num in (start_line..=end_line).rev() {
        let mut line_start_iter = buffer.iter_at_line(current_line_num).expect("Failed to get iterator for line");
        buffer.insert(&mut line_start_iter, &indent_string);
    }

    buffer.end_user_action();

    // Restore the selection
    if has_selection {
        if let (Some(start_mark), Some(end_mark)) = (original_selection_start_mark, original_selection_end_mark) {
            let new_start_iter = buffer.iter_at_mark(&start_mark);
            let new_end_iter = buffer.iter_at_mark(&end_mark);
            buffer.select_range(&new_start_iter, &new_end_iter);
            buffer.delete_mark(&start_mark); 
            buffer.delete_mark(&end_mark);   
        }
    }
}

pub fn outdent_selection(buffer: &TextBuffer) {
    let (is_tab_indent, indent_width) = detect_indent_style(buffer);
    let indent_prefix_string = if is_tab_indent {
        "\t".to_string()
    } else {
        " ".repeat(indent_width)
    };
    let indent_prefix = indent_prefix_string.as_str(); // Fixed: temporary value dropped while borrowed
    let indent_len = indent_prefix.len();

    let mut start_iter;
    let mut end_iter;
    let has_selection = buffer.selection_bounds().is_some();

    if has_selection {
        (start_iter, end_iter) = buffer.selection_bounds().unwrap();
    } else {
        start_iter = buffer.iter_at_mark(&buffer.get_insert());
        start_iter.set_line_offset(0);
        end_iter = start_iter.clone();
        end_iter.forward_to_line_end();
    }

    let start_line = start_iter.line();
    let end_line = end_iter.line();

    // Store the original selection marks to restore them later
    let original_selection_start_mark = if has_selection {
        Some(buffer.create_mark(None, &start_iter, false))
    } else {
        None
    };
    let original_selection_end_mark = if has_selection {
        Some(buffer.create_mark(None, &end_iter, false))
    } else {
        None
    };

    buffer.begin_user_action();

    // Iterate from bottom to top
    for current_line_num in (start_line..=end_line).rev() {
        let mut line_start_iter = buffer.iter_at_line(current_line_num).expect("Failed to get iterator for line");
        let mut line_end_iter = line_start_iter.clone(); 
        line_end_iter.forward_to_line_end();
        let line_text = buffer.text(&line_start_iter, &line_end_iter, false).to_string();

        if line_text.starts_with(indent_prefix) {
            let mut delete_end = line_start_iter.clone(); 
            delete_end.forward_chars(indent_len as i32);
            buffer.delete(&mut line_start_iter, &mut delete_end);
        } else if !is_tab_indent && line_text.starts_with(' ') {
            // Handle partial space outdent
            let mut spaces_to_remove = 0;
            for (i, c) in line_text.chars().enumerate() {
                if i < indent_width && c == ' ' {
                    spaces_to_remove += 1;
                } else {
                    break;
                }
            }
            if spaces_to_remove > 0 {
                let mut delete_end = line_start_iter.clone(); 
                delete_end.forward_chars(spaces_to_remove as i32);
                buffer.delete(&mut line_start_iter, &mut delete_end);
            }
        }
    }

    buffer.end_user_action();

    // Restore the selection
    if has_selection {
        if let (Some(start_mark), Some(end_mark)) = (original_selection_start_mark, original_selection_end_mark) {
            let new_start_iter = buffer.iter_at_mark(&start_mark);
            let new_end_iter = buffer.iter_at_mark(&end_mark);
            buffer.select_range(&new_start_iter, &new_end_iter);
            buffer.delete_mark(&start_mark); 
            buffer.delete_mark(&end_mark);   
        }
    }
}