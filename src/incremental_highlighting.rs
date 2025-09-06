//! Module for efficient incremental syntax highlighting
//!
//! This module provides functionality for efficiently updating syntax highlighting
//! only for the lines that have changed, rather than re-highlighting the entire buffer.

use crate::syntax_highlighting;
use gtk4::prelude::*;
use gtk4::TextBuffer;
use std::collections::HashSet;

/// Applies incremental syntax highlighting based on changed lines
pub fn apply_incremental_highlighting(
    buffer: &TextBuffer,
    syntax_context: &crate::syntax_highlighting::SyntaxHighlightingContext,
    changed_lines: &HashSet<i32>,
) {
    if changed_lines.is_empty() {
        return;
    }
    
    // Expand the range slightly to ensure context is correct
    let min_line = changed_lines.iter().min().copied().unwrap_or(0).max(0);
    let max_line = changed_lines.iter().max().copied().unwrap_or(0).min(buffer.line_count() - 1);
    
    // Add a few lines of context to ensure highlighting is correct
    let start_line = (min_line - 3).max(0);
    let end_line = (max_line + 3).min(buffer.line_count() - 1);
    
    syntax_highlighting::apply_incremental_syntax_highlighting(
        buffer,
        &syntax_context.syntax,
        &syntax_context.ps,
        &syntax_context.current_theme.borrow(),
        start_line,
        end_line,
    );
}