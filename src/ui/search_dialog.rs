use gtk4::{Dialog, ResponseType, Orientation, Box, Label, Entry, CheckButton, Align};
use gtk4::prelude::*;

/// Creates a search and replace dialog
pub fn create_search_replace_dialog(
    parent: &impl IsA<gtk4::Window>,
    initial_text: &str,
) -> (Dialog, Entry, Entry, CheckButton, CheckButton, CheckButton, Label) {
    let dialog = Dialog::builder()
        .title("Search and Replace")
        .transient_for(parent)
        .modal(true)
        .build();

    dialog.add_button("Find Previous", ResponseType::Other(0));
    dialog.add_button("Find Next", ResponseType::Ok);
    dialog.add_button("Replace", ResponseType::Apply);
    dialog.add_button("Replace All", ResponseType::Other(1));
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
    let search_entry = Entry::builder()
        .text(initial_text)
        .hexpand(true)
        .build();
    search_hbox.append(&search_label);
    search_hbox.append(&search_entry);
    vbox.append(&search_hbox);

    // Replace term entry
    let replace_hbox = Box::new(Orientation::Horizontal, 10);
    let replace_label = Label::new(Some("Replace with:"));
    let replace_entry = Entry::builder()
        .hexpand(true)
        .build();
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

    (dialog, search_entry, replace_entry, match_case_cb, whole_word_cb, regex_cb, status_label)
}