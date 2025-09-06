//! Module for tracking buffer changes for incremental highlighting
//!
//! This module provides functionality to track which lines have changed in a text buffer
//! to enable efficient incremental syntax highlighting.

use gtk4::{TextIter};

/// Tracks changes in a text buffer for incremental highlighting
pub struct ChangeTracker {
    /// Set of lines that have been modified
    pub changed_lines: std::collections::HashSet<i32>,
    /// The last inserted text
    pub last_inserted_text: String,
    /// The position where the last insertion occurred
    pub last_insert_position: Option<(i32, i32)>, // (line, offset)
}

impl ChangeTracker {
    /// Creates a new change tracker
    pub fn new() -> Self {
        Self {
            changed_lines: std::collections::HashSet::new(),
            last_inserted_text: String::new(),
            last_insert_position: None,
        }
    }

    /// Records an insertion in the buffer
    pub fn record_insertion(&mut self, start_iter: &TextIter, end_iter: &TextIter, text: &str) {
        let start_line = start_iter.line();
        let end_line = end_iter.line();
        
        // Add all affected lines to the changed set
        for line in start_line..=end_line {
            self.changed_lines.insert(line);
        }
        
        self.last_inserted_text = text.to_string();
        self.last_insert_position = Some((start_line, start_iter.line_offset()));
    }

    /// Records a deletion in the buffer
    pub fn record_deletion(&mut self, start_iter: &TextIter, end_iter: &TextIter) {
        let start_line = start_iter.line();
        let end_line = end_iter.line();
        
        // Add all affected lines to the changed set
        for line in start_line..=end_line {
            self.changed_lines.insert(line);
        }
    }

    /// Gets the set of changed lines and clears the tracker
    pub fn take_changed_lines(&mut self) -> std::collections::HashSet<i32> {
        std::mem::take(&mut self.changed_lines)
    }

    /// Checks if there are any pending changes
    pub fn has_changes(&self) -> bool {
        !self.changed_lines.is_empty()
    }
}