use std::collections::HashMap;
use std::path::PathBuf;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, TextBuffer, Settings, Notebook, TreeStore, TextView, ScrolledWindow, Label, ResponseType};
use gtk4::prelude::TextViewExt;

use gio::SimpleAction;
use std::rc::Rc;
use std::cell::RefCell;
use syntect::highlighting::ThemeSet;
use gtk4::pango;
use regex::Regex;

use crate::settings::{AppSettings, save_settings};
use crate::utils::add_zoom_controllers_to_text_view;
use crate::ui::components::{create_line_numbers_area, create_text_view_with_line_numbers};
use crate::file_operations::{open_file_dialog, open_directory_dialog};
use crate::ui::search_dialog;

pub fn open_file_in_new_tab(
    path: &PathBuf,
    notebook: &Notebook,
    highlight_closure: &Rc<dyn Fn(TextBuffer) + 'static>,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    app: &Application,
    current_font_desc: &Rc<RefCell<pango::FontDescription>>,
    update_font: &Rc<dyn Fn(&pango::FontDescription) + 'static>,
    initial_font_size: &Rc<RefCell<f64>>,
    setup_buffer_connections: &Rc<dyn Fn(&TextBuffer, &TextView)>,
) {
    // Check if the file is already open in a tab
    // Use a block to limit the scope of the immutable borrow
    {
        let buffer_paths_borrowed = buffer_paths.borrow();
        for (buffer, existing_path) in buffer_paths_borrowed.iter() {
            if existing_path == path {
                // File is already open, switch to its tab
                for i in 0..notebook.n_pages() {
                    if let Some(page) = notebook.nth_page(Some(i)) {
                        // The actual structure is: Box (line_numbers_area + ScrolledWindow (TextView))
                        if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                            // Get the second child which should be the ScrolledWindow
                            if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                                if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                                    if text_view.buffer() == *buffer {
                                        notebook.set_current_page(Some(i));
                                        return; // Exit the function as we've switched to the existing tab
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } // `buffer_paths_borrowed` is dropped here, releasing the immutable borrow

    // If the file is not already open, proceed to open it in a new tab
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let new_buffer = gtk4::TextBuffer::builder().text(&content).build();
            // Add the highlight tag to the new buffer's tag table
            let highlight_tag = gtk4::TextTag::new(Some("document_highlight"));
            highlight_tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(0.0, 0.0, 1.0, 0.3)));
            new_buffer.tag_table().add(&highlight_tag);
            // Add bracket_match tag
            let bracket_match_tag = gtk4::TextTag::new(Some("bracket_match"));
            bracket_match_tag.set_weight(700);
            bracket_match_tag.set_scale(1.3);
            new_buffer.tag_table().add(&bracket_match_tag);

            // Now it's safe to mutably borrow buffer_paths
            buffer_paths.borrow_mut().insert(new_buffer.clone(), path.clone());
            let new_text_view = gtk4::TextView::builder()
                .buffer(&new_buffer)
                .hexpand(true)
                .vexpand(true)
                .build();

            add_zoom_controllers_to_text_view(
                &new_text_view,
                current_font_desc.clone(),
                update_font.clone(),
                app.clone(),
                initial_font_size.clone(),
            );

            let scrolled_window = ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Automatic)
                .vscrollbar_policy(gtk4::PolicyType::Automatic)
                .child(&new_text_view)
                .build();

            // Line numbers area for the new tab
            let line_numbers_area = create_line_numbers_area(
                &new_text_view,
                &scrolled_window,
                current_font_desc.clone(),
            );

            let text_view_with_line_numbers_box = create_text_view_with_line_numbers(
                &new_text_view,
                &scrolled_window,
                &line_numbers_area,
            );

            // Connect scrolled_window's vadjustment to redraw line_numbers_area
            let line_numbers_area_clone_for_scroll = line_numbers_area.clone();
            scrolled_window.vadjustment().connect_value_changed(move |_| {
                line_numbers_area_clone_for_scroll.queue_draw();
            });

            // Connect new_buffer's changed signal to redraw line_numbers_area
            let line_numbers_area_clone_for_changed = line_numbers_area.clone();
            new_buffer.connect_changed(move |_| {
                line_numbers_area_clone_for_changed.queue_draw();
            });

            // Connect signals to the new buffer (this will also connect bracket highlighting)
            setup_buffer_connections(&new_buffer, &new_text_view);

            let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("Untitled");
            let tab_label = gtk4::Label::new(Some(filename));
            let page_num = notebook.append_page(&text_view_with_line_numbers_box, Some(&tab_label));
            notebook.set_current_page(Some(page_num));

            highlight_closure(new_buffer.clone());
        },
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            // Show error dialog
            // We need to get a reference to the window for the dialog
            // For now, we'll just print the error to console
            // In a full implementation, we'd pass the window reference to this function
        }
    }
}

pub fn open_directory_in_tree(
    path: &PathBuf,
    tree_store: &TreeStore,
    app_settings: &Rc<RefCell<AppSettings>>,
) {
    crate::file_operations::populate_tree_view(tree_store, path);
    app_settings.borrow_mut().last_opened_directory = Some(path.clone());
    save_settings(&app_settings.borrow());
}

/// Creates a new untitled file tab
pub fn create_new_file_tab(
    notebook: &Notebook,
    highlight_closure: &Rc<dyn Fn(TextBuffer) + 'static>,
    _buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    app: &Application,
    current_font_desc: &Rc<RefCell<pango::FontDescription>>,
    update_font: &Rc<dyn Fn(&pango::FontDescription) + 'static>,
    initial_font_size: &Rc<RefCell<f64>>,
    setup_buffer_connections: &Rc<dyn Fn(&TextBuffer, &TextView)>,
) {
    // Create a new empty buffer
    let new_buffer = gtk4::TextBuffer::builder().text("").build();
    
    // Add the highlight tag to the new buffer's tag table
    let highlight_tag = gtk4::TextTag::new(Some("document_highlight"));
    highlight_tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(0.0, 0.0, 1.0, 0.3)));
    new_buffer.tag_table().add(&highlight_tag);
    
    // Add bracket_match tag
    let bracket_match_tag = gtk4::TextTag::new(Some("bracket_match"));
    bracket_match_tag.set_weight(700);
    bracket_match_tag.set_scale(1.3);
    new_buffer.tag_table().add(&bracket_match_tag);

    let new_text_view = gtk4::TextView::builder()
        .buffer(&new_buffer)
        .hexpand(true)
        .vexpand(true)
        .build();

    add_zoom_controllers_to_text_view(
        &new_text_view,
        current_font_desc.clone(),
        update_font.clone(),
        app.clone(),
        initial_font_size.clone(),
    );

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&new_text_view)
        .build();

    // Line numbers area for the new tab
    let line_numbers_area = create_line_numbers_area(
        &new_text_view,
        &scrolled_window,
        current_font_desc.clone(),
    );

    let text_view_with_line_numbers_box = create_text_view_with_line_numbers(
        &new_text_view,
        &scrolled_window,
        &line_numbers_area,
    );

    // Connect scrolled_window's vadjustment to redraw line_numbers_area
    let line_numbers_area_clone_for_scroll = line_numbers_area.clone();
    scrolled_window.vadjustment().connect_value_changed(move |_| {
        line_numbers_area_clone_for_scroll.queue_draw();
    });

    // Connect new_buffer's changed signal to redraw line_numbers_area
    let line_numbers_area_clone_for_changed = line_numbers_area.clone();
    new_buffer.connect_changed(move |_| {
        line_numbers_area_clone_for_changed.queue_draw();
    });

    // Connect signals to the new buffer (this will also connect bracket highlighting)
    setup_buffer_connections(&new_buffer, &new_text_view);

    // Generate a unique name for the new tab
    let mut tab_name = "Untitled-1".to_string();
    let mut counter = 1;
    loop {
        let mut name_exists = false;
        for i in 0..notebook.n_pages() {
            if let Some(page) = notebook.nth_page(Some(i)) {
                if let Some(label_widget) = notebook.tab_label(&page) {
                    if let Some(label) = label_widget.downcast_ref::<Label>() {
                        if label.text().as_str() == tab_name {
                            name_exists = true;
                            break;
                        }
                    }
                }
            }
        }
        if !name_exists {
            break;
        }
        counter += 1;
        tab_name = format!("Untitled-{}", counter);
    }

    let tab_label = gtk4::Label::new(Some(&tab_name));
    let page_num = notebook.append_page(&text_view_with_line_numbers_box, Some(&tab_label));
    notebook.set_current_page(Some(page_num));

    highlight_closure(new_buffer.clone());
}

/// Checks if a buffer has been modified compared to its file on disk
pub fn is_buffer_modified(buffer: &TextBuffer, file_path: Option<&PathBuf>) -> bool {
    if let Some(path) = file_path {
        if let Ok(content_on_disk) = std::fs::read_to_string(path) {
            let buffer_content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            return buffer_content != content_on_disk;
        }
    }
    // If file doesn't exist on disk or can't be read, consider it modified if it has content
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    !buffer.text(&start, &end, false).is_empty()
}

/// Prompts the user to save changes before closing a file
/// This function uses a callback to handle the response since GTK dialogs are asynchronous
pub fn prompt_save_changes_async<F>(
    parent: &impl IsA<gtk4::Window>,
    buffer: gtk4::TextBuffer,
    file_path: Option<PathBuf>,
    buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    notebook: gtk4::Notebook,
    current_page: u32,
    callback: F,
) where
    F: FnOnce(bool) + 'static, // true if we should proceed, false if we should cancel
{
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .buttons(gtk4::ButtonsType::None)
        .text("Save changes?")
        .secondary_text("The document has been modified. Do you want to save your changes?")
        .build();

    dialog.add_button("Save", gtk4::ResponseType::Yes);
    dialog.add_button("Don't Save", gtk4::ResponseType::No);
    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);

    let parent_clone = parent.clone();
    let callback = std::rc::Rc::new(std::cell::RefCell::new(Some(callback)));
    let buffer_paths_clone = buffer_paths.clone();
    let notebook_clone = notebook.clone();
    let buffer_clone = buffer.clone();
    dialog.connect_response(move |dialog, response| {
        // Take the callback out of the RefCell
        let callback = callback.borrow_mut().take();
        
        match response {
            gtk4::ResponseType::Yes => {
                // User wants to save
                if let Some(path) = &file_path {
                    if let Err(e) = save_buffer_to_file(&parent_clone, &buffer_clone, path) {
                        eprintln!("Error saving file: {}", e);
                        // Show error dialog
                        let error_dialog = gtk4::MessageDialog::builder()
                            .transient_for(&parent_clone)
                            .modal(true)
                            .buttons(gtk4::ButtonsType::Ok)
                            .text("Error saving file")
                            .secondary_text(&format!("Could not save file: {}", e))
                            .build();
                        error_dialog.connect_response(|dialog, _| {
                            dialog.close();
                        });
                        error_dialog.present();
                        dialog.close();
                        if let Some(callback) = callback {
                            callback(false); // Don't proceed
                        }
                        return;
                    }
                    // Remove from buffer_paths map
                    buffer_paths_clone.borrow_mut().remove(&buffer_clone);
                    // Close the tab
                    notebook_clone.remove_page(Some(current_page));
                    dialog.close();
                    if let Some(callback) = callback {
                        callback(true); // Proceed
                    }
                } else {
                    // No path - this is an untitled file, need to show save dialog
                    dialog.close();
                    
                    // Show save dialog
                    let parent_clone2 = parent_clone.clone();
                    let buffer_clone2 = buffer_clone.clone();
                    let buffer_paths_clone2 = buffer_paths_clone.clone();
                    let notebook_clone2 = notebook_clone.clone();
                    
                    crate::file_operations::save_file_dialog(
                        &parent_clone2,
                        buffer_clone2,
                        buffer_paths_clone2,
                        Some(notebook_clone2),
                    );
                    
                    // For untitled files, we call the callback immediately since we can't wait
                    // for the save dialog to complete (it's asynchronous)
                    if let Some(callback) = callback {
                        callback(true); // Proceed
                    }
                }
            },
            gtk4::ResponseType::No => {
                // User doesn't want to save
                // Remove from buffer_paths map
                buffer_paths_clone.borrow_mut().remove(&buffer_clone);
                // Close the tab
                notebook_clone.remove_page(Some(current_page));
                dialog.close();
                if let Some(callback) = callback {
                    callback(true); // Proceed
                }
            },
            gtk4::ResponseType::Cancel | gtk4::ResponseType::DeleteEvent => {
                // User cancelled
                dialog.close();
                if let Some(callback) = callback {
                    callback(false); // Don't proceed
                }
            },
            _ => {
                // Unexpected response
                dialog.close();
                if let Some(callback) = callback {
                    callback(false); // Don't proceed
                }
            }
        }
    });
    
    dialog.present();
}

/// Saves the content of a buffer to a file
fn save_buffer_to_file(
    _parent: &impl IsA<gtk4::Window>,
    buffer: &TextBuffer,
    file_path: &PathBuf,
) -> Result<(), std::io::Error> {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    let content = buffer.text(&start, &end, false).to_string();
    std::fs::write(file_path, content)
}

/// Closes the current tab with save prompt if needed
fn close_current_tab(
    window: &ApplicationWindow,
    notebook: &Notebook,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
) {
    if let Some(current_page) = notebook.current_page() {
        if let Some(page) = notebook.nth_page(Some(current_page)) {
            // Get the buffer from the text view
            if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                    if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                        let buffer = text_view.buffer();
                        
                        // Check if buffer has a path and if it's modified
                        let buffer_paths_borrowed = buffer_paths.borrow();
                        let file_path = buffer_paths_borrowed.get(&buffer).cloned();
                        
                        if is_buffer_modified(&buffer, file_path.as_ref()) {
                            // Drop the borrow before showing dialog
                            drop(buffer_paths_borrowed);
                            
                            // Show save prompt asynchronously
                            prompt_save_changes_async(
                                window,
                                buffer,
                                file_path,
                                buffer_paths.clone(),
                                notebook.clone(),
                                current_page,
                                |_proceed| {
                                    // The callback handles all the logic
                                }
                            );
                        } else {
                            // Not modified, close the tab directly
                            // Drop the borrow
                            drop(buffer_paths_borrowed);
                            
                            // Remove from buffer_paths map
                            buffer_paths.borrow_mut().remove(&buffer);
                            
                            // Close the tab
                            notebook.remove_page(Some(current_page));
                        }
                    }
                }
            }
        }
    }
}

/// Closes all tabs with save prompts if needed, handling each tab individually
fn close_all_tabs_with_prompts(
    window: ApplicationWindow,
    notebook: Notebook,
    buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
) {
    // Create a list of all buffers that are actually modified
    let mut buffers_to_check = Vec::new();
    
    // Collect all buffers and their paths, but only if they are modified
    for i in 0..notebook.n_pages() {
        if let Some(page) = notebook.nth_page(Some(i)) {
            if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                    if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                        let buffer = text_view.buffer();
                        let buffer_paths_borrowed = buffer_paths.borrow();
                        let file_path = buffer_paths_borrowed.get(&buffer).cloned();
                        drop(buffer_paths_borrowed); // Release the borrow
                        
                        // Only add to check list if actually modified
                        if is_buffer_modified(&buffer, file_path.as_ref()) {
                            buffers_to_check.push((buffer, file_path, i));
                        }
                    }
                }
            }
        }
    }
    
    // If no buffers are modified, just close all tabs
    if buffers_to_check.is_empty() {
        // No unsaved changes, close all tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }
        return;
    }
    
    // We need to handle this asynchronously, so we'll process one buffer at a time
    // Create a recursive function to handle each buffer
    fn process_next_buffer(
        window: ApplicationWindow,
        notebook: Notebook,
        buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
        mut buffers_to_check: Vec<(TextBuffer, Option<PathBuf>, u32)>,
    ) {
        if let Some((buffer, file_path, page_index)) = buffers_to_check.pop() {
            let buffer_paths_clone = buffer_paths.clone();
            let notebook_clone = notebook.clone();
            let window_clone = window.clone();
            
            prompt_save_changes_async(
                &window,
                buffer,
                file_path,
                buffer_paths_clone,
                notebook_clone,
                page_index as u32,
                move |proceed| {
                    if proceed {
                        // Continue with the next buffer if there are more
                        process_next_buffer(window_clone, notebook, buffer_paths, buffers_to_check);
                    }
                    // If not proceed, the user cancelled, so we don't close any more tabs
                }
            );
        } else {
            // All buffers processed or user cancelled, close all remaining tabs
            while notebook.n_pages() > 0 {
                notebook.remove_page(Some(0));
            }
        }
    }
    
    // Start processing the buffers
    process_next_buffer(window, notebook, buffer_paths, buffers_to_check);
}

/// Gets the currently selected text or word under cursor
fn get_selected_text_or_word(buffer: &TextBuffer) -> String {
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
fn find_next_advanced(
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
        if let Some(match_pos) = search_text_in_buffer(buffer, search_text, &cursor_iter, match_case, false) {
            return Some(match_pos);
        }
        
        // If not found, wrap around to the beginning
        let start_iter = buffer.start_iter();
        search_text_in_buffer(buffer, search_text, &start_iter, match_case, false)
    }
}

/// Finds the previous occurrence of the search text (advanced version with regex support)
fn find_previous_advanced(
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
        if let Some(match_pos) = search_text_in_buffer_backward(buffer, search_text, &cursor_iter, match_case, false) {
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
    if let Some(match_pos) = search_text_in_buffer_whole_word(buffer, search_text, &cursor_iter, match_case) {
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
    if let Some(match_pos) = search_text_in_buffer_whole_word_backward(buffer, search_text, &cursor_iter, match_case) {
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
    match compile_regex(pattern, match_case) {
        Ok(regex) => {
            // Get current cursor position
            let insert_mark = buffer.get_insert();
            let mut cursor_iter = buffer.iter_at_mark(&insert_mark);
            
            // Move one character forward to avoid matching the same text again
            cursor_iter.forward_char();
            
            // Get the text from cursor to end
            let text = buffer.text(&cursor_iter, &buffer.end_iter(), false).to_string();
            
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
            let text = buffer.text(&start_iter, &buffer.end_iter(), false).to_string();
            
            if let Some(mat) = regex.find(&text) {
                let start_offset = start_iter.offset() + mat.start() as i32;
                let end_offset = start_iter.offset() + mat.end() as i32;
                let start_iter = buffer.iter_at_offset(start_offset);
                let end_iter = buffer.iter_at_offset(end_offset);
                return Some((start_iter, end_iter));
            }
            
            None
        },
        Err(_) => None
    }
}

/// Finds the previous occurrence using regex
fn find_previous_regex(
    buffer: &TextBuffer,
    pattern: &str,
    match_case: bool,
) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    match compile_regex(pattern, match_case) {
        Ok(regex) => {
            // Get current cursor position
            let insert_mark = buffer.get_insert();
            let cursor_iter = buffer.iter_at_mark(&insert_mark);
            
            // Get the text from start to cursor
            let text = buffer.text(&buffer.start_iter(), &cursor_iter, false).to_string();
            
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
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            
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
        },
        Err(_) => None
    }
}

/// Compiles a regex pattern with optional case insensitivity
fn compile_regex(pattern: &str, match_case: bool) -> Result<Regex, regex::Error> {
    if match_case {
        Regex::new(pattern)
    } else {
        Regex::new(&format!("(?i){}", pattern))
    }
}

/// Replaces the current selection with replacement text (advanced version with regex support)
fn replace_selection_advanced(buffer: &TextBuffer, search_text: &str, replacement_text: &str, use_regex: bool) {
    if let Some((start, end)) = buffer.selection_bounds() {
        let mut start_mut = start;
        let mut end_mut = end;
        
        if use_regex {
            // For regex replacement, we need to get the matched text and apply the replacement
            let matched_text = buffer.text(&start, &end, false).to_string();
            match compile_regex(search_text, true) { // We don't handle case insensitivity here as it's in the pattern
                Ok(regex) => {
                    if let Some(_mat) = regex.find(&matched_text) {
                        let actual_replacement = regex.replace(&matched_text, replacement_text).to_string();
                        buffer.begin_user_action();
                        buffer.delete(&mut start_mut, &mut end_mut);
                        buffer.insert(&mut start_mut, &actual_replacement);
                        buffer.end_user_action();
                    }
                },
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
fn replace_all_advanced(
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
    match compile_regex(pattern, match_case) {
        Ok(regex) => {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            let result = regex.replace_all(&text, replacement_text).to_string();
            
            // Replace the entire buffer content
            buffer.begin_user_action();
            buffer.set_text(&result);
            buffer.end_user_action();
            
            // Count matches for return value
            regex.find_iter(&text).count() as u32
        },
        Err(_) => 0
    }
}

pub fn setup_actions(
    app: &Application,
    window: &ApplicationWindow,
    notebook: &Notebook,
    highlight_closure: &Rc<dyn Fn(TextBuffer) + 'static>,
    current_theme: &Rc<RefCell<syntect::highlighting::Theme>>,
    ts: &Rc<ThemeSet>,
    current_font_desc: &Rc<RefCell<pango::FontDescription>>,
    update_font: &Rc<dyn Fn(&pango::FontDescription) + 'static>,
    app_settings: &Rc<RefCell<AppSettings>>,
    tree_store: &TreeStore,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    initial_font_size: &Rc<RefCell<f64>>,
    setup_buffer_connections: &Rc<dyn Fn(&TextBuffer, &TextView)>,
) {

fn get_current_text_view(notebook: &Notebook) -> Option<TextView> {
    if let Some(current_page) = notebook.current_page() {
        if let Some(page) = notebook.nth_page(Some(current_page)) {
            // The actual structure is: Box (line_numbers_area + ScrolledWindow (TextView))
            if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                    if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                        return Some(text_view.clone());
                    }
                }
            }
        }
    }
    None
}

    let new_action = SimpleAction::new("new", None);
    let notebook_clone_new = notebook.clone();
    let highlight_closure_clone_new = highlight_closure.clone();
    let buffer_paths_clone_new = buffer_paths.clone();
    let app_clone_new = app.clone();
    let current_font_desc_clone_new = current_font_desc.clone();
    let update_font_clone_new = update_font.clone();
    let initial_font_size_clone_new = initial_font_size.clone();
    let setup_buffer_connections_clone_new = setup_buffer_connections.clone();
    let window_clone_new = window.clone();
    new_action.connect_activate(move |_, _| {
        // Check if current file needs saving
        if let Some(current_page) = notebook_clone_new.current_page() {
            if let Some(page) = notebook_clone_new.nth_page(Some(current_page)) {
                if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                    if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                        if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_new.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer).cloned();
                            
                            if is_buffer_modified(&buffer, file_path.as_ref()) {
                                // Drop the borrow before showing dialog
                                drop(buffer_paths_borrowed);
                                
                                let notebook_clone = notebook_clone_new.clone();
                                let highlight_closure_clone = highlight_closure_clone_new.clone();
                                let buffer_paths_clone = buffer_paths_clone_new.clone();
                                let app_clone = app_clone_new.clone();
                                let current_font_desc_clone = current_font_desc_clone_new.clone();
                                let update_font_clone = update_font_clone_new.clone();
                                let initial_font_size_clone = initial_font_size_clone_new.clone();
                                let setup_buffer_connections_clone = setup_buffer_connections_clone_new.clone();
                                
                                // Show save prompt asynchronously
                                prompt_save_changes_async(
                                    &window_clone_new,
                                    buffer,
                                    file_path,
                                    buffer_paths_clone_new.clone(),
                                    notebook_clone_new.clone(),
                                    current_page,
                                    move |proceed| {
                                        if proceed {
                                            // Create new file tab
                                            create_new_file_tab(
                                                &notebook_clone,
                                                &highlight_closure_clone,
                                                &buffer_paths_clone,
                                                &app_clone,
                                                &current_font_desc_clone,
                                                &update_font_clone,
                                                &initial_font_size_clone,
                                                &setup_buffer_connections_clone,
                                            );
                                        }
                                    }
                                );
                                
                                // Return early since we're handling this asynchronously
                                return;
                            }
                        }
                    }
                }
            }
        }
        
        create_new_file_tab(
            &notebook_clone_new,
            &highlight_closure_clone_new,
            &buffer_paths_clone_new,
            &app_clone_new,
            &current_font_desc_clone_new,
            &update_font_clone_new,
            &initial_font_size_clone_new,
            &setup_buffer_connections_clone_new,
        );
    });
    app.add_action(&new_action);

    let open_action = SimpleAction::new("open", None);
    let window_clone_open = window.clone();
    let notebook_clone_open = notebook.clone();
    let highlight_closure_clone_open = highlight_closure.clone();
    let buffer_paths_clone_open = buffer_paths.clone();
    let app_clone_open = app.clone();
    let current_font_desc_clone_open = current_font_desc.clone();
    let update_font_clone_open = update_font.clone();
    let initial_font_size_clone_open = initial_font_size.clone();
    let setup_buffer_connections_clone_open = setup_buffer_connections.clone();
    open_action.connect_activate(move |_, _| {
        // Check if current file needs saving
        if let Some(current_page) = notebook_clone_open.current_page() {
            if let Some(page) = notebook_clone_open.nth_page(Some(current_page)) {
                if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                    if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                        if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_open.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer).cloned();
                            
                            if is_buffer_modified(&buffer, file_path.as_ref()) {
                                // Drop the borrow before showing dialog
                                drop(buffer_paths_borrowed);
                                
                                let window_clone = window_clone_open.clone();
                                let notebook_clone = notebook_clone_open.clone();
                                let highlight_closure_clone = highlight_closure_clone_open.clone();
                                let buffer_paths_clone = buffer_paths_clone_open.clone();
                                let app_clone = app_clone_open.clone();
                                let current_font_desc_clone = current_font_desc_clone_open.clone();
                                let update_font_clone = update_font_clone_open.clone();
                                let initial_font_size_clone = initial_font_size_clone_open.clone();
                                let setup_buffer_connections_clone = setup_buffer_connections_clone_open.clone();
                                
                                // Show save prompt asynchronously
                                prompt_save_changes_async(
                                    &window_clone_open,
                                    buffer,
                                    file_path,
                                    buffer_paths_clone_open.clone(),
                                    notebook_clone_open.clone(),
                                    current_page,
                                    move |proceed| {
                                        if proceed {
                                            // Open file dialog
                                            open_file_dialog(
                                                &window_clone,
                                                notebook_clone,
                                                highlight_closure_clone,
                                                buffer_paths_clone,
                                                app_clone,
                                                current_font_desc_clone,
                                                update_font_clone,
                                                initial_font_size_clone,
                                                setup_buffer_connections_clone,
                                            );
                                        }
                                    }
                                );
                                
                                // Return early since we're handling this asynchronously
                                return;
                            }
                        }
                    }
                }
            }
        }
        
        open_file_dialog(
            &window_clone_open,
            notebook_clone_open.clone(),
            highlight_closure_clone_open.clone(),
            buffer_paths_clone_open.clone(),
            app_clone_open.clone(),
            current_font_desc_clone_open.clone(),
            update_font_clone_open.clone(),
            initial_font_size_clone_open.clone(),
            setup_buffer_connections_clone_open.clone(),
        );
    });
    app.add_action(&open_action);

    let open_directory_action = SimpleAction::new("open_directory", None);
    let window_clone_open_dir = window.clone();
    let tree_store_clone_open_dir = tree_store.clone();
    let app_settings_clone_open_dir = app_settings.clone();
    open_directory_action.connect_activate(move |_, _| {
        open_directory_dialog(
            &window_clone_open_dir,
            tree_store_clone_open_dir.clone(),
            app_settings_clone_open_dir.clone(),
        );
    });
    app.add_action(&open_directory_action);

    let close_current_file_action = SimpleAction::new("close_current_file", None);
    let window_clone_close = window.clone();
    let notebook_clone_close = notebook.clone();
    let buffer_paths_clone_close = buffer_paths.clone();
    close_current_file_action.connect_activate(move |_, _| {
        close_current_tab(&window_clone_close, &notebook_clone_close, &buffer_paths_clone_close);
    });
    app.add_action(&close_current_file_action);

    let close_all_files_action = SimpleAction::new("close_all_files", None);
    let window_clone_close_all = window.clone();
    let notebook_clone_close_all = notebook.clone();
    let buffer_paths_clone_close_all = buffer_paths.clone();
    close_all_files_action.connect_activate(move |_, _| {
        let window_clone = window_clone_close_all.clone();
        let notebook_clone = notebook_clone_close_all.clone();
        let buffer_paths_clone = buffer_paths_clone_close_all.clone();
        close_all_tabs_with_prompts(window_clone, notebook_clone, buffer_paths_clone);
    });
    app.add_action(&close_all_files_action);

    let save_action = SimpleAction::new("save", None);
    let window_clone_save = window.clone();
    let notebook_clone_save = notebook.clone();
    let buffer_paths_clone_save = buffer_paths.clone();
    save_action.connect_activate(move |_, _| {
        if let Some(current_page) = notebook_clone_save.current_page() {
            if let Some(page) = notebook_clone_save.nth_page(Some(current_page)) {
                if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                    if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                        if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_save.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer);
                            
                            if let Some(path) = file_path {
                                // Save to existing file
                                if let Err(e) = save_buffer_to_file(&window_clone_save, &buffer, path) {
                                    eprintln!("Error saving file: {}", e);
                                    // Show error dialog
                                    let dialog = gtk4::MessageDialog::builder()
                                        .transient_for(&window_clone_save)
                                        .modal(true)
                                        .buttons(gtk4::ButtonsType::Ok)
                                        .text("Error saving file")
                                        .secondary_text(&format!("Could not save file: {}", e))
                                        .build();
                                    dialog.run_async(|dialog, _| {
                                        dialog.close();
                                    });
                                }
                            } else {
                                // Need to save as - open save dialog
                                drop(buffer_paths_borrowed); // Drop borrow before calling save_file_dialog
                                crate::file_operations::save_file_dialog(
                                    &window_clone_save,
                                    buffer,
                                    buffer_paths_clone_save.clone(),
                                    Some(notebook_clone_save.clone()),
                                );
                            }
                        }
                    }
                }
            }
        }
    });
    app.add_action(&save_action);

    let save_as_action = SimpleAction::new("save_as", None);
    let window_clone_save_as = window.clone();
    let notebook_clone_save_as = notebook.clone();
    let buffer_paths_clone_save_as = buffer_paths.clone();
    save_as_action.connect_activate(move |_, _| {
        if let Some(current_page) = notebook_clone_save_as.current_page() {
            if let Some(page) = notebook_clone_save_as.nth_page(Some(current_page)) {
                if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                    if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                        if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                            let buffer = text_view.buffer();
                            crate::file_operations::save_file_dialog(
                                &window_clone_save_as,
                                buffer,
                                buffer_paths_clone_save_as.clone(),
                                Some(notebook_clone_save_as.clone()),
                            );
                        }
                    }
                }
            }
        }
    });
    app.add_action(&save_as_action);

    let search_and_replace_action = SimpleAction::new("search_and_replace", None);
    let window_clone_search_replace = window.clone();
    let notebook_clone_search_replace = notebook.clone();
    search_and_replace_action.connect_activate(move |_, _| {
        // Get the current buffer
        if let Some(current_page) = notebook_clone_search_replace.current_page() {
            if let Some(page) = notebook_clone_search_replace.nth_page(Some(current_page)) {
                if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                    if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                        if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                            let buffer = text_view.buffer();
                            
                            // Get selected text or word under cursor as initial search text
                            let initial_text = get_selected_text_or_word(&buffer);
                            
                            // Create search and replace dialog
                            let (dialog, search_entry, replace_entry, match_case_cb, whole_word_cb, regex_cb, status_label) = 
                                search_dialog::create_search_replace_dialog(&window_clone_search_replace, &initial_text);
                            
                            // Clone references for use in closures
                            let buffer_clone = buffer.clone();
                            let text_view_clone = text_view.clone();
                            let status_label_clone = status_label.clone();
                            
                            // Connect dialog buttons
                            dialog.connect_response(move |d, response| {
                                let search_text = search_entry.text().to_string();
                                let replace_text = replace_entry.text().to_string();
                                let match_case = match_case_cb.is_active();
                                let whole_word = whole_word_cb.is_active();
                                let use_regex = regex_cb.is_active();
                                
                                // If using regex, validate the pattern first
                                if use_regex && !search_text.is_empty() {
                                    match Regex::new(&search_text) {
                                        Ok(_) => {},
                                        Err(e) => {
                                            status_label_clone.set_text(&format!("Invalid regex: {}", e));
                                            return;
                                        }
                                    }
                                }
                                
                                if response == ResponseType::Ok {
                                    // Find next occurrence
                                    if !search_text.is_empty() {
                                        match find_next_advanced(&buffer_clone, &search_text, match_case, whole_word, use_regex) {
                                            Some((start_iter, end_iter)) => {
                                                // Select the found text
                                                buffer_clone.select_range(&start_iter, &end_iter);
                                                // Scroll to the found text
                                                let mut start_iter_mut = start_iter.clone();
                                                text_view_clone.scroll_to_iter(&mut start_iter_mut, 0.0, false, 0.0, 0.0);
                                                status_label_clone.set_text("");
                                            },
                                            None => {
                                                status_label_clone.set_text("Text not found");
                                            }
                                        }
                                    }
                                } else if response == ResponseType::Other(0) {
                                    // Find previous occurrence
                                    if !search_text.is_empty() {
                                        match find_previous_advanced(&buffer_clone, &search_text, match_case, whole_word, use_regex) {
                                            Some((start_iter, end_iter)) => {
                                                // Select the found text
                                                buffer_clone.select_range(&start_iter, &end_iter);
                                                // Scroll to the found text
                                                let mut start_iter_mut = start_iter.clone();
                                                text_view_clone.scroll_to_iter(&mut start_iter_mut, 0.0, false, 0.0, 0.0);
                                                status_label_clone.set_text("");
                                            },
                                            None => {
                                                status_label_clone.set_text("Text not found");
                                            }
                                        }
                                    }
                                } else if response == ResponseType::Apply {
                                    // Replace current selection
                                    if !search_text.is_empty() {
                                        replace_selection_advanced(&buffer_clone, &search_text, &replace_text, use_regex);
                                        // Find next occurrence
                                        match find_next_advanced(&buffer_clone, &search_text, match_case, whole_word, use_regex) {
                                            Some((start_iter, end_iter)) => {
                                                // Select the found text
                                                buffer_clone.select_range(&start_iter, &end_iter);
                                                // Scroll to the found text
                                                let mut start_iter_mut = start_iter.clone();
                                                text_view_clone.scroll_to_iter(&mut start_iter_mut, 0.0, false, 0.0, 0.0);
                                                status_label_clone.set_text("");
                                            },
                                            None => {
                                                status_label_clone.set_text("Text not found");
                                            }
                                        }
                                    }
                                } else if response == ResponseType::Other(1) {
                                    // Replace all occurrences
                                    if !search_text.is_empty() {
                                        let count = replace_all_advanced(&buffer_clone, &search_text, &replace_text, match_case, whole_word, use_regex);
                                        status_label_clone.set_text(&format!("Replaced {} occurrences", count));
                                    }
                                } else if response == ResponseType::Cancel {
                                    d.response(ResponseType::None);
                                    d.close();
                                }
                            });
                            
                            dialog.present();
                        }
                    }
                }
            }
        }
    });
    app.add_action(&search_and_replace_action);

    let cut_action = SimpleAction::new("cut", None);
    let notebook_clone_cut = notebook.clone();
    cut_action.connect_activate(move |_, _| {
        if let Some(text_view) = get_current_text_view(&notebook_clone_cut) {
            let buffer = text_view.buffer();
            if let Some((start, end)) = buffer.selection_bounds() {
                let selected_text = buffer.text(&start, &end, false).to_string();
                if let Some(display) = gtk4::gdk::Display::default() { // Changed here
                    let clipboard = display.clipboard();
                    clipboard.set_text(&selected_text);
                }
                // Delete the selected text
                buffer.delete(&mut start.clone(), &mut end.clone());
            }
        }
    });
    app.add_action(&cut_action);

    let copy_action = SimpleAction::new("copy", None);
    let notebook_clone_copy = notebook.clone();
    copy_action.connect_activate(move |_, _| {
        if let Some(text_view) = get_current_text_view(&notebook_clone_copy) {
            let buffer = text_view.buffer();
            if let Some((start, end)) = buffer.selection_bounds() {
                let selected_text = buffer.text(&start, &end, false).to_string();
                if let Some(display) = gtk4::gdk::Display::default() { // Changed here
                    let clipboard = display.clipboard();
                    clipboard.set_text(&selected_text);
                }
            }
        }
    });
    app.add_action(&copy_action);

    let paste_action = SimpleAction::new("paste", None);
    let notebook_clone_paste = notebook.clone();
    paste_action.connect_activate(move |_, _| {
        if let Some(text_view) = get_current_text_view(&notebook_clone_paste) {
            let buffer = text_view.buffer();
            if let Some(display) = gtk4::gdk::Display::default() { // Changed here
                let clipboard = display.clipboard();
                clipboard.read_text_async(None::<&gio::Cancellable>, move |res| {
                    if let Ok(Some(text)) = res {
                        let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                        buffer.insert(&mut iter, &text);
                    }
                });
            }
        }
    });
    app.add_action(&paste_action);

    let about_action = SimpleAction::new("about", None);
    let window_clone_about = window.clone();
    about_action.connect_activate(move |_, _| {
        let dialog = crate::ui::windows::create_about_dialog(&window_clone_about);
        dialog.present();
    });
    app.add_action(&about_action);

    // Settings Action
    let settings_gtk = Settings::default().unwrap();
    let settings_action = SimpleAction::new("settings", None);
    let window_clone = window.clone();
    let highlight_closure_clone = highlight_closure.clone();
    let notebook_clone = notebook.clone();
    let current_theme_clone = current_theme.clone();
    let ts_clone = ts.clone();
    let current_font_desc_clone = current_font_desc.clone();
    let update_font_clone = update_font.clone();
    let app_settings_clone = app_settings.clone();

    settings_action.connect_activate(move |_, _| {
        // Get current settings to pass to the dialog
        let current_theme = app_settings_clone.borrow().theme.clone();
        let current_font = app_settings_clone.borrow().font.clone();
        
        let dialog = crate::ui::windows::create_settings_dialog(&window_clone, &current_theme, &current_font);

        let settings_clone_response = settings_gtk.clone();
        let highlight_closure_response = highlight_closure_clone.clone();
        let notebook_response = notebook_clone.clone();
        let current_theme_response = current_theme_clone.clone();
        let ts_response = ts_clone.clone();
        let current_font_desc_response = current_font_desc_clone.clone();
        let update_font_response = update_font_clone.clone();
        let app_settings_response = app_settings_clone.clone();

        dialog.connect_response(move |d, r| {
            if r == gtk4::ResponseType::Apply {
                // Get the combo box and font button from the dialog
                let content_area = d.content_area();
                if let Some(widget) = content_area.first_child() {
                    if let Ok(vbox) = widget.downcast::<gtk4::Box>() {
                        // Get theme combo (first hbox)
                        if let Some(widget) = vbox.first_child() {
                            if let Ok(theme_hbox) = widget.downcast::<gtk4::Box>() {
                                if let Some(widget) = theme_hbox.last_child() {
                                    if let Ok(combo) = widget.downcast::<gtk4::ComboBoxText>() {
                                        let mut new_settings = app_settings_response.borrow_mut();

                                        if let Some(active_id) = combo.active_id() {
                                            let is_dark = active_id == "dark";
                                            new_settings.theme = active_id.to_string();
                                            settings_clone_response.set_gtk_application_prefer_dark_theme(is_dark);
                                            if is_dark {
                                                *current_theme_response.borrow_mut() = ts_response.themes["base16-ocean.dark"].clone();
                                            } else {
                                                *current_theme_response.borrow_mut() = ts_response.themes["InspiredGitHub"].clone();
                                            }
                                            
                                            for i in 0..notebook_response.n_pages() {
                                                if let Some(page) = notebook_response.nth_page(Some(i)) {
                                                    // The actual structure is: Box (line_numbers_area + ScrolledWindow (TextView))
                                                    if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                                                        // Get the second child which should be the ScrolledWindow
                                                        if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                                                            if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                                                                let buffer = text_view.buffer();
                                                                highlight_closure_response(buffer.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Get font button (second hbox)
                        if let Some(widget) = vbox.last_child() {
                            if let Ok(font_hbox) = widget.downcast::<gtk4::Box>() {
                                if let Some(widget) = font_hbox.last_child() {
                                    if let Ok(font_button) = widget.downcast::<gtk4::FontButton>() {
                                        if let Some(new_font_desc) = font_button.font_desc() {
                                            let mut new_settings = app_settings_response.borrow_mut();
                                            new_settings.font = new_font_desc.to_string();
                                            *current_font_desc_response.borrow_mut() = new_font_desc.clone();
                                            update_font_response(&new_font_desc);
                                        }
                                    }
                                }
                            }
                        }
                        
                        save_settings(&app_settings_response.borrow());
                    }
                }
            }
            d.close();
        });
        
        dialog.present();
    });
    app.add_action(&settings_action);

    let quit_action = SimpleAction::new("quit", None);
    let app_clone = app.clone();
    let window_clone_quit = window.clone();
    let notebook_clone_quit = notebook.clone();
    let buffer_paths_clone_quit = buffer_paths.clone();
    quit_action.connect_activate(move |_, _| {
        // Check if any files have unsaved changes
        let mut has_unsaved_changes = false;
        let mut first_unsaved_buffer = None;
        let mut first_unsaved_file_path = None;
        let mut first_unsaved_page_index = 0;
        
        for i in 0..notebook_clone_quit.n_pages() {
            if let Some(page) = notebook_clone_quit.nth_page(Some(i)) {
                if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                    if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                        if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_quit.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer).cloned();
                            
                            if is_buffer_modified(&buffer, file_path.as_ref()) {
                                has_unsaved_changes = true;
                                first_unsaved_buffer = Some(buffer);
                                first_unsaved_file_path = file_path;
                                first_unsaved_page_index = i;
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        if has_unsaved_changes {
            if let Some(buffer) = first_unsaved_buffer {
                let app_clone2 = app_clone.clone();
                let buffer_paths_clone2 = buffer_paths_clone_quit.clone();
                let notebook_clone2 = notebook_clone_quit.clone();
                
                prompt_save_changes_async(
                    &window_clone_quit,
                    buffer,
                    first_unsaved_file_path,
                    buffer_paths_clone2,
                    notebook_clone2,
                    first_unsaved_page_index as u32,
                    move |proceed| {
                        if proceed {
                            // User wants to proceed with exit
                            app_clone2.quit();
                        }
                        // If not proceed, the user cancelled, so we don't exit
                    }
                );
            }
        } else {
            // No unsaved changes, exit immediately
            app_clone.quit();
        }
    });
    app.add_action(&quit_action);

    // Set accelerators for actions
    app.set_accels_for_action("app.new", &["<Control>n"]);
    app.set_accels_for_action("app.open", &["<Control>o"]);
    app.set_accels_for_action("app.close_current_file", &["<Control>w"]);
    app.set_accels_for_action("app.close_all_files", &["<Control><Shift>w"]);
    app.set_accels_for_action("app.save", &["<Control>s"]);
    app.set_accels_for_action("app.save_as", &["<Control><Shift>s"]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);
    app.set_accels_for_action("app.search_and_replace", &["<Control>f"]);
    app.set_accels_for_action("app.cut", &["<Control>x"]);
    app.set_accels_for_action("app.copy", &["<Control>c"]);
    app.set_accels_for_action("app.paste", &["<Control>v"]);
}

