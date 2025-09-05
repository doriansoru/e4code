use gtk4::prelude::*;
use gtk4::{Notebook, ScrolledWindow, TextView};

/// Helper function to get the TextView from a given Notebook page widget.
/// This encapsulates the common pattern of traversing the widget hierarchy.
pub fn get_text_view_from_page(page: &gtk4::Widget) -> Option<TextView> {
    // The actual structure is: Box (line_numbers_area + ScrolledWindow (TextView))
    if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
        if let Some(scrolled_window) = text_view_with_line_numbers_box
            .last_child()
            .and_then(|w| w.downcast::<ScrolledWindow>().ok())
        {
            if let Some(text_view) = scrolled_window
                .child()
                .and_then(|w| w.downcast::<TextView>().ok())
            {
                return Some(text_view.clone());
            }
        }
    }
    None
}

/// Helper function to get the TextView from the currently active page of a Notebook.
pub fn get_current_text_view(notebook: &Notebook) -> Option<TextView> {
    if let Some(current_page_num) = notebook.current_page() {
        if let Some(page) = notebook.nth_page(Some(current_page_num)) {
            return get_text_view_from_page(&page);
        }
    }
    None
}
