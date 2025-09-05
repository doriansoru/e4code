use gtk4::prelude::*;
use gtk4::{
    FileChooserAction, FileChooserDialog, ResponseType, ScrolledWindow, TextView, TreeStore, Box, Label
};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use crate::AppContext;

/// Opens a file chooser dialog for opening files
pub fn open_file_dialog(
    parent: &impl IsA<gtk4::Window>,
    notebook: gtk4::Notebook,
    highlight_closure: Rc<dyn Fn(gtk4::TextBuffer)>,
    buffer_paths: Rc<RefCell<std::collections::HashMap<gtk4::TextBuffer, PathBuf>>>,
    app: gtk4::Application,
    current_font_desc: Rc<RefCell<gtk4::pango::FontDescription>>,
    update_font: Rc<dyn Fn(&gtk4::pango::FontDescription)>,
    initial_font_size: Rc<RefCell<f64>>,
    setup_buffer_connections: Rc<dyn Fn(&gtk4::TextBuffer, &gtk4::TextView)>,
) {
    let file_chooser = FileChooserDialog::builder()
        .title("Open File")
        .transient_for(parent)
        .modal(true)
        .action(FileChooserAction::Open)
        .build();

    file_chooser.add_button("Cancel", ResponseType::Cancel);
    file_chooser.add_button("Open", ResponseType::Accept);

    file_chooser.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    crate::tab_manager::open_file_in_new_tab(
                        &path,
                        &notebook,
                        &highlight_closure,
                        &buffer_paths,
                        &app,
                        &current_font_desc,
                        &update_font,
                        &initial_font_size,
                        &setup_buffer_connections,
                    );
                }
            }
        }
        dialog.close();
    });
    file_chooser.present();
}

/// Opens a folder chooser dialog for opening directories
pub fn open_directory_dialog(
    parent: &impl IsA<gtk4::Window>,
    app_context: Rc<RefCell<AppContext>>,
) {
    let folder_chooser = FileChooserDialog::builder()
        .title("Open Directory")
        .transient_for(parent)
        .modal(true)
        .action(FileChooserAction::SelectFolder)
        .build();

    folder_chooser.add_button("Cancel", ResponseType::Cancel);
    folder_chooser.add_button("Open", ResponseType::Accept);

    folder_chooser.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(folder) = dialog.file() {
                if let Some(path) = folder.path() {
                    // Pass app_context directly
                    crate::actions::open_directory_in_tree(&path, &app_context);
                }
            }
        }
        dialog.close();
    });
    folder_chooser.present();
}

/// Updates the tab label for a buffer
pub fn update_tab_label(
    notebook: &gtk4::Notebook,
    buffer: &gtk4::TextBuffer,
    path: &std::path::Path,
) {
    // Find the page containing this buffer
    for i in 0..notebook.n_pages() {
        if let Some(page) = notebook.nth_page(Some(i)) {
            if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                if let Some(scrolled_window) = text_view_with_line_numbers_box
                    .last_child()
                    .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                {
                    if let Some(text_view) = scrolled_window
                        .child()
                        .and_then(|w| w.downcast::<TextView>().ok())
                    {
                        if text_view.buffer() == *buffer {
                            // Update the tab label
                            let filename = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Untitled");
                            if let Some(tab_label_box) = notebook.tab_label(&page).and_then(|w| w.downcast::<Box>().ok()) {
                                if let Some(label) = tab_label_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                                    label.set_text(filename);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Opens a file chooser dialog for saving files
pub fn save_file_dialog(
    parent: &impl IsA<gtk4::Window>,
    buffer: gtk4::TextBuffer,
    buffer_paths: Rc<RefCell<std::collections::HashMap<gtk4::TextBuffer, PathBuf>>>,
    notebook: Option<gtk4::Notebook>, // Optional notebook to update tab label
) {
    let file_chooser = FileChooserDialog::builder()
        .title("Save File")
        .transient_for(parent)
        .modal(true)
        .action(FileChooserAction::Save)
        .build();

    file_chooser.add_button("Cancel", ResponseType::Cancel);
    file_chooser.add_button("Save", ResponseType::Accept);

    // Clone values for the closure
    let buffer_clone = buffer.clone();
    let buffer_paths_clone = buffer_paths.clone();
    let notebook_clone = notebook.clone();

    file_chooser.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    // Save the buffer content to the file
                    let start = buffer_clone.start_iter();
                    let end = buffer_clone.end_iter();
                    let content = buffer_clone.text(&start, &end, false).to_string();

                    match std::fs::write(&path, content) {
                        Ok(_) => {
                            // Update the buffer_paths map with the new path
                            buffer_paths_clone
                                .borrow_mut()
                                .insert(buffer_clone.clone(), path.clone());

                            // Update tab label with filename if notebook is provided
                            if let Some(notebook) = &notebook_clone {
                                update_tab_label(notebook, &buffer_clone, &path);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error saving file: {}", e);
                            // TODO: Show error dialog
                        }
                    }
                }
            }
        }
        dialog.close();
    });
    file_chooser.present();
}

/// Populates the tree view with directory contents
pub fn populate_tree_view(tree_store: &TreeStore, path: &std::path::Path) {
    tree_store.clear();

    // Add ".." entry if not at the root
    if path.parent().is_some() {
        let parent_path = path.parent().unwrap().to_path_buf();
        tree_store.insert_with_values(
            None,
            None,
            &[(0, &".."), (1, &parent_path.to_str().unwrap_or(""))],
        );
    }

    if let Ok(entries) = fs::read_dir(path) {
        // Separate directories and files for sorting
        let mut directories = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                directories.push(entry_path);
            } else {
                files.push(entry_path);
            }
        }

        // Sort directories and files alphabetically
        directories.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        // Add sorted directories
        for entry_path in directories {
            let file_name = entry_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let full_path = entry_path.to_str().unwrap_or("").to_string();
            tree_store.insert_with_values(None, None, &[(0, &file_name), (1, &full_path)]);
        }

        // Add sorted files
        for entry_path in files {
            let file_name = entry_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let full_path = entry_path.to_str().unwrap_or("").to_string();
            tree_store.insert_with_values(None, None, &[(0, &file_name), (1, &full_path)]);
        }
    } else {
        eprintln!("Error reading directory: {:?}", path);
    }
}