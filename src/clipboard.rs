use gtk4::prelude::*;
use gtk4::{TextBuffer, TextView};

/// Copies the selected text from a buffer to the clipboard
pub fn copy_selected_text(buffer: &TextBuffer) {
    if let Some((start, end)) = buffer.selection_bounds() {
        let selected_text = buffer.text(&start, &end, false).to_string();
        if let Some(display) = gtk4::gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&selected_text);
        }
    }
}

/// Cuts the selected text from a buffer and copies it to the clipboard
pub fn cut_selected_text(buffer: &TextBuffer) {
    if let Some((start, end)) = buffer.selection_bounds() {
        let selected_text = buffer.text(&start, &end, false).to_string();
        if let Some(display) = gtk4::gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&selected_text);
        }
        // Delete the selected text
        let mut start_clone = start.clone();
        let mut end_clone = end.clone();
        buffer.delete(&mut start_clone, &mut end_clone);
    }
}

/// Pastes text from the clipboard into a text view at the cursor position
pub fn paste_text_async(text_view: &TextView) {
    let buffer = text_view.buffer();
    if let Some(display) = gtk4::gdk::Display::default() {
        let clipboard = display.clipboard();
        clipboard.read_text_async(None::<&gio::Cancellable>, move |res| {
            if let Ok(Some(text)) = res {
                let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                buffer.insert(&mut iter, &text);
            }
        });
    }
}