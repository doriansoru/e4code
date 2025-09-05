use std::collections::HashMap;
use std::path::PathBuf;

use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, Label, Notebook, ScrolledWindow, TextBuffer,
    TextView,
};

use gtk4::pango;
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::components::{create_line_numbers_area, create_text_view_with_line_numbers};
use crate::utils::add_zoom_controllers_to_text_view;

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
                        if let Some(text_view) = crate::ui::helpers::get_text_view_from_page(&page) {
                            if text_view.buffer() == *buffer {
                                notebook.set_current_page(Some(i));
                                return; // Exit the function as we've switched to the existing tab
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
            // Setup standard buffer tags
            crate::buffer_tags::setup_buffer_tags(&new_buffer);

            // Now it's safe to mutably borrow buffer_paths
            buffer_paths
                .borrow_mut()
                .insert(new_buffer.clone(), path.clone());
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
            scrolled_window
                .vadjustment()
                .connect_value_changed(move |_| {
                    line_numbers_area_clone_for_scroll.queue_draw();
                });

            // Connect new_buffer's changed signal to redraw line_numbers_area
            let line_numbers_area_clone_for_changed = line_numbers_area.clone();
            new_buffer.connect_changed(move |_| {
                line_numbers_area_clone_for_changed.queue_draw();
            });

            // Connect signals to the new buffer (this will also connect bracket highlighting)
            setup_buffer_connections(&new_buffer, &new_text_view);

            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled");
            let tab_label_box = Box::new(gtk4::Orientation::Horizontal, 5);
            let tab_label = Label::new(Some(filename));
            let close_button = Button::from_icon_name("window-close-symbolic");
            close_button.add_css_class("flat");

            tab_label_box.append(&tab_label);
            tab_label_box.append(&close_button);

            let page_num =
                notebook.append_page(&text_view_with_line_numbers_box, Some(&tab_label_box));
            notebook.set_current_page(Some(page_num));

            let notebook_clone = notebook.clone();
            let buffer_paths_clone = buffer_paths.clone();
            if let Some(window) = app.active_window() {
                if let Some(app_window) = window.downcast_ref::<ApplicationWindow>() {
                    let window_clone = app_window.clone();
                    close_button.connect_clicked(move |_| {
                        close_tab(&window_clone, &notebook_clone, &buffer_paths_clone, page_num);
                    });
                }
            }

            highlight_closure(new_buffer.clone());
        }
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            // Show error dialog
            // We need to get a reference to the window for the dialog
            // For now, we'll just print the error to console
            // In a full implementation, we'd pass the window reference to this function
        }
    }
}

/// Creates a new untitled file tab
pub fn create_new_file_tab(
    notebook: &Notebook,
    highlight_closure: &Rc<dyn Fn(TextBuffer) + 'static>,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    app: &Application,
    current_font_desc: &Rc<RefCell<pango::FontDescription>>,
    update_font: &Rc<dyn Fn(&pango::FontDescription) + 'static>,
    initial_font_size: &Rc<RefCell<f64>>,
    setup_buffer_connections: &Rc<dyn Fn(&TextBuffer, &TextView)>,
) {
    // Create a new empty buffer
    let new_buffer = gtk4::TextBuffer::builder().text("").build();
    // Setup standard buffer tags
    crate::buffer_tags::setup_buffer_tags(&new_buffer);

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
    let line_numbers_area =
        create_line_numbers_area(&new_text_view, &scrolled_window, current_font_desc.clone());

    let text_view_with_line_numbers_box =
        create_text_view_with_line_numbers(&new_text_view, &scrolled_window, &line_numbers_area);

    // Connect scrolled_window's vadjustment to redraw line_numbers_area
    let line_numbers_area_clone_for_scroll = line_numbers_area.clone();
    scrolled_window
        .vadjustment()
        .connect_value_changed(move |_| {
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
                    if let Some(tab_box) = label_widget.downcast_ref::<Box>() {
                        if let Some(child_widget) = tab_box.first_child() {
                            if let Some(label) = child_widget.downcast_ref::<Label>() {
                                if label.text().as_str() == tab_name {
                                    name_exists = true;
                                    break;
                                }
                            }
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

    let tab_label_box = Box::new(gtk4::Orientation::Horizontal, 5);
    let tab_label = Label::new(Some(&tab_name));
    let close_button = Button::from_icon_name("window-close-symbolic");
    close_button.add_css_class("flat");

    tab_label_box.append(&tab_label);
    tab_label_box.append(&close_button);

    let page_num =
        notebook.append_page(&text_view_with_line_numbers_box, Some(&tab_label_box));
    notebook.set_current_page(Some(page_num));

    let notebook_clone = notebook.clone();
    let buffer_paths_clone = buffer_paths.clone();
    if let Some(window) = app.active_window() {
        if let Some(app_window) = window.downcast_ref::<ApplicationWindow>() {
            let window_clone = app_window.clone();
            close_button.connect_clicked(move |_| {
                close_tab(&window_clone, &notebook_clone, &buffer_paths_clone, page_num);
            });
        }
    }

    highlight_closure(new_buffer.clone());
}

pub fn is_buffer_modified(buffer: &TextBuffer, file_path: Option<&PathBuf>) -> bool {
    crate::file_operations::is_buffer_modified(buffer, file_path)
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
                        crate::dialogs::show_error_dialog(
                            &parent_clone,
                            "Error saving file",
                            &format!("Could not save file: {}", e)
                        );
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
            }
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
            }
            gtk4::ResponseType::Cancel | gtk4::ResponseType::DeleteEvent => {
                // User cancelled
                dialog.close();
                if let Some(callback) = callback {
                    callback(false); // Don't proceed
                }
            }
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
pub fn save_buffer_to_file(
    _parent: &impl IsA<gtk4::Window>,
    buffer: &TextBuffer,
    file_path: &PathBuf,
) -> Result<(), std::io::Error> {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    let content = buffer.text(&start, &end, false).to_string();
    std::fs::write(file_path, content)
}

pub fn close_tab(
    window: &ApplicationWindow,
    notebook: &Notebook,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    page_num: u32,
) {
    if let Some(page) = notebook.nth_page(Some(page_num)) {
        if let Some(text_view) = crate::ui::helpers::get_text_view_from_page(&page) {
            let buffer = text_view.buffer();
            let buffer_paths_borrowed = buffer_paths.borrow();
            let file_path = buffer_paths_borrowed.get(&buffer).cloned();

            if is_buffer_modified(&buffer, file_path.as_ref()) {
                drop(buffer_paths_borrowed);
                prompt_save_changes_async(
                    window,
                    buffer,
                    file_path,
                    buffer_paths.clone(),
                    notebook.clone(),
                    page_num,
                    |_proceed| {},
                );
            } else {
                drop(buffer_paths_borrowed);
                buffer_paths.borrow_mut().remove(&buffer);
                notebook.remove_page(Some(page_num));
            }
        }
    }
}

/// Closes the current tab with save prompt if needed
pub fn close_current_tab(
    window: &ApplicationWindow,
    notebook: &Notebook,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
) {
    if let Some(current_page) = notebook.current_page() {
        close_tab(window, notebook, buffer_paths, current_page);
    }
}

/// Closes all tabs with save prompts if needed, handling each tab individually
pub fn close_all_tabs_with_prompts(
    window: ApplicationWindow,
    notebook: Notebook,
    buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
) {
    // Create a list of all buffers that are actually modified
    let mut buffers_to_check = Vec::new();

    // Collect all buffers and their paths, but only if they are modified
    for i in 0..notebook.n_pages() {
        if let Some(page) = notebook.nth_page(Some(i)) {
            if let Some(text_view) = crate::ui::helpers::get_text_view_from_page(&page) {
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
        buffer_paths: Rc<RefCell<HashMap<TextBuffer, PathBuf>>>,
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
                },
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

pub fn get_open_file_paths(
    notebook: &Notebook,
    buffer_paths: &Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
) -> Vec<PathBuf> {
    let mut open_paths = Vec::new();
    let buffer_paths_borrowed = buffer_paths.borrow();

    for i in 0..notebook.n_pages() {
        if let Some(page) = notebook.nth_page(Some(i)) {
            if let Some(text_view) = crate::ui::helpers::get_text_view_from_page(&page) {
                let buffer = text_view.buffer();
                if let Some(path) = buffer_paths_borrowed.get(&buffer) {
                    open_paths.push(path.clone());
                }
            }
        }
    }
    open_paths
}
