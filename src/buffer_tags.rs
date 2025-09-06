//! Module for managing text buffer tags
//!
//! This module provides functions for setting up standard tags that are used
//! in text buffers for features like syntax highlighting and bracket matching.

use gtk4::prelude::*;
use gtk4::{TextBuffer, TextTag};

/// Sets up the standard tags for a text buffer
/// This includes document highlight and bracket match tags
pub fn setup_buffer_tags(buffer: &TextBuffer) {
    // Add the highlight tag to the buffer's tag table
    let highlight_tag = TextTag::new(Some("document_highlight"));
    highlight_tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(0.0, 0.0, 1.0, 0.3)));
    buffer.tag_table().add(&highlight_tag);

    // Add bracket_match tag
    let bracket_match_tag = TextTag::new(Some("bracket_match"));
    bracket_match_tag.set_weight(700);
    bracket_match_tag.set_scale(1.3);
    buffer.tag_table().add(&bracket_match_tag);
}