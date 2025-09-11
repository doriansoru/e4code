//! Search dialog UI module
//!
//! This module provides the search and replace dialog functionality for the application.

use gtk4::prelude::*;
use gtk4::{Align, Box, CheckButton, Dialog, Entry, Label, Orientation, ResponseType};
use std::rc::Rc;
use std::cell::RefCell;
use crate::search; // Import the search module

pub const RESPONSE_TYPE_FIND_PREVIOUS: ResponseType = ResponseType::Other(0);
pub const RESPONSE_TYPE_REPLACE_ALL: ResponseType = ResponseType::Other(1);

/// Creates a search and replace dialog
///
/// This function creates a dialog window with controls for searching and
/// replacing text, including options for case sensitivity, whole word matching,
/// and regular expressions.
///
/// # Arguments
///
/// * `parent` - Parent window for the dialog
/// * `initial_text` - Initial text to populate the search field with
/// * `buffer` - The TextBuffer to perform search operations on
///
/// # Returns
///
/// A tuple containing the dialog and its child widgets for further manipulation
pub fn create_search_replace_dialog(
    parent: &impl IsA<gtk4::Window>,
    initial_text: &str,
    buffer: &gtk4::TextBuffer,
) -> (
    Dialog,
    Entry,
    Entry,
    CheckButton,
    CheckButton,
    CheckButton,
    Label,
) {
    let dialog = Dialog::builder()
        .title("Search and Replace")
        .transient_for(parent)
        .modal(true)
        .build();

    dialog.add_button("Find Previous", RESPONSE_TYPE_FIND_PREVIOUS);
    dialog.add_button("Find Next", ResponseType::Ok);
    dialog.add_button("Replace", ResponseType::Apply);
    dialog.add_button("Replace All", RESPONSE_TYPE_REPLACE_ALL);
    dialog.add_button("Cancel", ResponseType::Cancel);

    let content_area = dialog.content_area();
    let vbox = Box::new(Orientation::Vertical, 10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);

    // Search term entry
    let search_hbox = Box::new(Orientation::Horizontal, 10);
    let search_label = Label::new(Some("Find what:"));
    let search_entry = Entry::builder().text(initial_text).hexpand(true).build();
    search_hbox.append(&search_label);
    search_hbox.append(&search_entry);
    vbox.append(&search_hbox);

    // Replace term entry
    let replace_hbox = Box::new(Orientation::Horizontal, 10);
    let replace_label = Label::new(Some("Replace with:"));
    let replace_entry = Entry::builder().hexpand(true).build();
    replace_hbox.append(&replace_label);
    replace_hbox.append(&replace_entry);
    vbox.append(&replace_hbox);

    // Options
    let options_hbox = Box::new(Orientation::Horizontal, 10);
    let match_case_cb = CheckButton::with_label("Match case");
    let whole_word_cb = CheckButton::with_label("Whole word");
    let regex_cb = CheckButton::with_label("Regex");
    options_hbox.append(&match_case_cb);
    options_hbox.append(&whole_word_cb);
    options_hbox.append(&regex_cb);
    vbox.append(&options_hbox);

    // Status label for search results and errors
    let status_label = Label::new(Some(""));
    status_label.set_halign(Align::Start);
    vbox.append(&status_label);

    content_area.append(&vbox);

    // Connect signals for counting occurrences
    connect_search_events(
        buffer,
        &search_entry,
        &match_case_cb,
        &whole_word_cb,
        &regex_cb,
        &status_label,
    );

    (
        dialog,
        search_entry,
        replace_entry,
        match_case_cb,
        whole_word_cb,
        regex_cb,
        status_label,
    )
}

/// Connects signals to update the occurrence count in the status label
pub fn connect_search_events(
    buffer: &gtk4::TextBuffer,
    search_entry: &Entry,
    match_case_cb: &CheckButton,
    whole_word_cb: &CheckButton,
    regex_cb: &CheckButton,
    status_label: &Label,
) {
    let buffer_clone = buffer.clone();
    let search_entry_clone = search_entry.clone();
    let match_case_cb_clone = match_case_cb.clone();
    let whole_word_cb_clone = whole_word_cb.clone();
    let regex_cb_clone = regex_cb.clone();
    let status_label_clone = status_label.clone();

    let update_count = Rc::new(RefCell::new(move || {
        let search_text = search_entry_clone.text().to_string();
        let match_case = match_case_cb_clone.is_active();
        let whole_word = whole_word_cb_clone.is_active();
        let use_regex = regex_cb_clone.is_active();

        let count = search::count_all_occurrences(
            &buffer_clone,
            &search_text,
            match_case,
            whole_word,
            use_regex,
        );
        if search_text.is_empty() {
            status_label_clone.set_text("");
        } else {
            status_label_clone.set_text(&format!("{} occurrences found", count));
        }
    }));

    // Initial count update
    update_count.borrow()();

    // Connect signals
    let update_count_clone_1 = update_count.clone();
    search_entry.connect_changed(move |_e| {
        update_count_clone_1.borrow()();
    });

    let update_count_clone_2 = update_count.clone();
    match_case_cb.connect_toggled(move |_e| {
        update_count_clone_2.borrow()();
    });

    let update_count_clone_3 = update_count.clone();
    whole_word_cb.connect_toggled(move |_e| {
        update_count_clone_3.borrow()();
    });

    let update_count_clone_4 = update_count.clone();
    regex_cb.connect_toggled(move |_e| {
        update_count_clone_4.borrow()();
    });
}
