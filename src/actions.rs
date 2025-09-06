//! Module for handling application actions and menu commands
//!
//! This module defines all the actions that can be triggered through the application's
//! menus, keyboard shortcuts, or other UI elements. It includes file operations,
//! editing functions, search functionality, and application settings.

use std::path::PathBuf;

use gtk4::prelude::TextViewExt;
use gtk4::prelude::*;
use gtk4::{ResponseType, Settings};

use gio::SimpleAction;
use std::cell::RefCell;
use std::rc::Rc;

use crate::AppContext;

use crate::settings::save_settings;

use crate::file_operations::{open_directory_dialog, open_file_dialog};
use crate::ui::search_dialog;

use crate::tab_manager;
use crate::indentation;

use crate::search;

/// Opens a directory in the tree view
///
/// This function populates the tree view with the contents of the specified directory
/// and updates the application settings to remember this directory for future sessions.
///
/// # Arguments
///
/// * `path` - The path to the directory to open
/// * `app_context` - Reference to the application context
pub fn open_directory_in_tree(
    path: &PathBuf,
    app_context: &Rc<RefCell<AppContext>>,
) {
    crate::file_operations::populate_tree_view(&app_context.borrow().tree_store, path);
    app_context.borrow_mut().app_settings.borrow_mut().last_opened_directory = Some(path.clone());
    save_settings(&app_context.borrow().app_settings.borrow());
}


/// Sets up all application actions and connects them to their respective handlers
///
/// This function creates all the menu actions for the application and connects them
/// to appropriate callback functions. It also sets up keyboard accelerators for
/// common operations.
///
/// # Arguments
///
/// * `app_context` - Reference to the application context
pub fn setup_actions(
    app_context: Rc<RefCell<AppContext>>,
) {
    let app_context_for_app = app_context.clone(); // Clone for app
    let app_context_for_closures = app_context.clone(); // Clone once for all closures

    let app = &app_context_for_app.borrow().app; // Borrow app directly from app_context_for_app
    

    let new_action = SimpleAction::new("new", None);
    let app_context_clone_new = app_context_for_closures.clone(); // Use the clone for closures
    new_action.connect_activate(move |_, _| {
        let context = app_context_clone_new.borrow(); // Borrow context inside the closure
        // Remove: let app = &context.app; // Borrow app inside the closure
        // Check if current file needs saving
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            let buffer = text_view.buffer();
            let buffer_paths_borrowed = context.buffer_paths.borrow();
            let file_path = buffer_paths_borrowed.get(&buffer).cloned();

            if tab_manager::is_buffer_modified(&buffer, file_path.as_ref()) {
                // Drop the borrow before showing dialog
                drop(buffer_paths_borrowed);

                let app_context_clone_for_prompt = app_context_clone_new.clone();
                // Show save prompt asynchronously
                tab_manager::prompt_save_changes_async(
                    &context.window,
                    buffer,
                    file_path,
                    context.buffer_paths.clone(),
                    context.notebook.clone(),
                    context.notebook.current_page().unwrap_or(0), // Get current page index
                    move |proceed| {
                        if proceed {
                            // Create new file tab
                            tab_manager::create_new_file_tab(
                                &app_context_clone_for_prompt,
                            );
                        }
                    },
                );

                // Return early since we're handling this asynchronously
                return;
            }
        }

        tab_manager::create_new_file_tab(
            &app_context,
        );
    });
    app.add_action(&new_action);

    let open_action = SimpleAction::new("open", None);
    let app_context_clone_open = app_context_for_closures.clone();
    open_action.connect_activate(move |_, _| {
        let context = app_context_clone_open.borrow();
        // Check if current file needs saving
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            let buffer = text_view.buffer();
            let buffer_paths_borrowed = context.buffer_paths.borrow();
            let file_path = buffer_paths_borrowed.get(&buffer).cloned();

            if tab_manager::is_buffer_modified(&buffer, file_path.as_ref()) {
                // Drop the borrow before showing dialog
                drop(buffer_paths_borrowed);

                let app_context_clone_for_prompt = app_context_clone_open.clone();
                // Show save prompt asynchronously
                tab_manager::prompt_save_changes_async(
                    &context.window,
                    buffer,
                    file_path,
                    context.buffer_paths.clone(),
                    context.notebook.clone(),
                    context.notebook.current_page().unwrap_or(0), // Get current page index
                    move |proceed| {
                        if proceed {
                            let context_for_dialog = app_context_clone_for_prompt.borrow();
                            // Open file dialog
                            open_file_dialog(
                                &context_for_dialog.window,
                                app_context_clone_for_prompt.clone(),
                            );
                        }
                    },
                );

                // Return early since we're handling this asynchronously
                return;
            }
        }

        open_file_dialog(
            &context.window,
            app_context_clone_open.clone(),
        );
    });
    app.add_action(&open_action);

    let open_directory_action = SimpleAction::new("open_directory", None);
    let app_context_clone_open_dir = app_context_for_closures.clone();
    open_directory_action.connect_activate(move |_, _| {
        let context = app_context_clone_open_dir.borrow();
        open_directory_dialog(
            &context.window,
            app_context_clone_open_dir.clone(),
        );
    });
    app.add_action(&open_directory_action);

    let close_current_file_action = SimpleAction::new("close_current_file", None);
    let app_context_clone_close = app_context_for_closures.clone();
    close_current_file_action.connect_activate(move |_, _| {
        let context = app_context_clone_close.borrow();
        tab_manager::close_current_tab(
            &context.window,
            &context.notebook,
            &context.buffer_paths,
        );
    });
    app.add_action(&close_current_file_action);

    let close_all_files_action = SimpleAction::new("close_all_files", None);
    let app_context_clone_close_all = app_context_for_closures.clone();
    close_all_files_action.connect_activate(move |_, _| {
        let context = app_context_clone_close_all.borrow();
        tab_manager::close_all_tabs_with_prompts(
            context.window.clone(),
            context.notebook.clone(),
            context.buffer_paths.clone(),
        );
    });
    app.add_action(&close_all_files_action);

    let save_action = SimpleAction::new("save", None);
    let app_context_clone_save = app_context_for_closures.clone();
    save_action.connect_activate(move |_, _| {
        let context = app_context_clone_save.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            let buffer = text_view.buffer();
            let buffer_paths_borrowed = context.buffer_paths.borrow();
            let file_path = buffer_paths_borrowed.get(&buffer);

            if let Some(path) = file_path {
                // Save to existing file
                if let Err(e) = tab_manager::save_buffer_to_file(
                    &context.window,
                    &buffer,
                    path,
                ) {
                    eprintln!("Error saving file: {}", e);
                    // Show error dialog
                            crate::dialogs::show_error_dialog(
                                &context.window,
                                "Error saving file",
                                &format!("Could not save file: {}", e)
                            );
                }
            } else {
                // Need to save as - open save dialog
                drop(buffer_paths_borrowed); // Drop borrow before calling save_file_dialog
                crate::file_operations::save_file_dialog(
                    &context.window,
                    buffer,
                    context.buffer_paths.clone(),
                    Some(context.notebook.clone()),
                );
            }
        }
    });
    app.add_action(&save_action);

    let save_as_action = SimpleAction::new("save_as", None);
    let app_context_clone_save_as = app_context_for_closures.clone();
    save_as_action.connect_activate(move |_, _| {
        let context = app_context_clone_save_as.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            let buffer = text_view.buffer();
            crate::file_operations::save_file_dialog(
                &context.window,
                buffer,
                context.buffer_paths.clone(),
                Some(context.notebook.clone()),
            );
        }
    });
    app.add_action(&save_as_action);

    let search_and_replace_action = SimpleAction::new("search_and_replace", None);
    let app_context_clone_search_replace = app_context_for_closures.clone();
    search_and_replace_action.connect_activate(move |_, _| {
        let context = app_context_clone_search_replace.borrow();
        // Get the current buffer
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            let buffer = text_view.buffer();

            // Get selected text or word under cursor as initial search text
            let initial_text = search::get_selected_text_or_word(&buffer);

            // Create search and replace dialog
            let (
                dialog,
                search_entry,
                replace_entry,
                match_case_cb,
                whole_word_cb,
                regex_cb,
                status_label,
            ) = search_dialog::create_search_replace_dialog(
                &context.window,
                &initial_text,
            );

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
                    match search::compile_regex(&search_text, match_case) {
                        Ok(_) => {}
                        Err(e) => {
                            status_label_clone
                                .set_text(&format!("Invalid regex: {}", e));
                            return;
                        }
                    }
                }

                if response == ResponseType::Ok {
                    // Find next occurrence
                    if !search_text.is_empty() {
                        match search::find_next_advanced(
                            &buffer_clone,
                            &search_text,
                            match_case,
                            whole_word,
                            use_regex,
                        ) {
                            Some((start_iter, end_iter)) => {
                                // Select the found text
                                buffer_clone.select_range(&start_iter, &end_iter);
                                // Scroll to the found text
                                let mut start_iter_mut = start_iter.clone();
                                text_view_clone.scroll_to_iter(
                                    &mut start_iter_mut,
                                    0.0,
                                    false,
                                    0.0,
                                    0.0,
                                );
                                status_label_clone.set_text("");
                            }
                            None => {
                                status_label_clone.set_text("Text not found");
                            }
                        }
                    }
                } else if response == ResponseType::Other(0) {
                    // Find previous occurrence
                    if !search_text.is_empty() {
                        match search::find_previous_advanced(
                            &buffer_clone,
                            &search_text,
                            match_case,
                            whole_word,
                            use_regex,
                        ) {
                            Some((start_iter, end_iter)) => {
                                // Select the found text
                                buffer_clone.select_range(&start_iter, &end_iter);
                                // Scroll to the found text
                                let mut start_iter_mut = start_iter.clone();
                                text_view_clone.scroll_to_iter(
                                    &mut start_iter_mut,
                                    0.0,
                                    false,
                                    0.0,
                                    0.0,
                                );
                                status_label_clone.set_text("");
                            }
                            None => {
                                status_label_clone.set_text("Text not found");
                            }
                        }
                    }
                } else if response == ResponseType::Apply {
                    // Replace current selection
                    if !search_text.is_empty() {
                        search::replace_selection_advanced(
                            &buffer_clone,
                            &search_text,
                            &replace_text,
                            use_regex,
                        );
                        // Find next occurrence
                        match search::find_next_advanced(
                            &buffer_clone,
                            &search_text,
                            match_case,
                            whole_word,
                            use_regex,
                        ) {
                            Some((start_iter, end_iter)) => {
                                // Select the found text
                                buffer_clone.select_range(&start_iter, &end_iter);
                                // Scroll to the found text
                                let mut start_iter_mut = start_iter.clone();
                                text_view_clone.scroll_to_iter(
                                    &mut start_iter_mut,
                                    0.0,
                                    false,
                                    0.0,
                                    0.0,
                                );
                                status_label_clone.set_text("");
                            }
                            None => {
                                status_label_clone.set_text("Text not found");
                            }
                        }
                    }
                } else if response == ResponseType::Other(1) {
                    // Replace all occurrences
                    if !search_text.is_empty() {
                        let count = search::replace_all_advanced(
                            &buffer_clone,
                            &search_text,
                            &replace_text,
                            match_case,
                            whole_word,
                            use_regex,
                        );
                        status_label_clone
                            .set_text(&format!("Replaced {} occurrences", count));
                    }
                } else if response == ResponseType::Cancel {
                    d.response(ResponseType::None);
                    d.close();
                }
            });

            dialog.present();
        }
    });
    app.add_action(&search_and_replace_action);

    let cut_action = SimpleAction::new("cut", None);
    let app_context_clone_cut = app_context_for_closures.clone();
    cut_action.connect_activate(move |_, _| {
        let context = app_context_clone_cut.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            crate::clipboard::cut_selected_text(&text_view.buffer());
        }
    });
    app.add_action(&cut_action);

    let copy_action = SimpleAction::new("copy", None);
    let app_context_clone_copy = app_context_for_closures.clone();
    copy_action.connect_activate(move |_, _| {
        let context = app_context_clone_copy.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            crate::clipboard::copy_selected_text(&text_view.buffer());
        }
    });
    app.add_action(&copy_action);

    let paste_action = SimpleAction::new("paste", None);
    let app_context_clone_paste = app_context_for_closures.clone();
    paste_action.connect_activate(move |_, _| {
        let context = app_context_clone_paste.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            crate::clipboard::paste_text_async(&text_view);
        }
    });
    app.add_action(&paste_action);

    let indent_action = SimpleAction::new("indent", None);
    let app_context_clone_indent = app_context_for_closures.clone();
    indent_action.connect_activate(move |_, _| {
        let context = app_context_clone_indent.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            // Indent logic will go here
            indentation::indent_selection(&text_view.buffer());
        }
    });
    app.add_action(&indent_action);

    let outdent_action = SimpleAction::new("outdent", None);
    let app_context_clone_outdent = app_context_for_closures.clone();
    outdent_action.connect_activate(move |_, _| {
        let context = app_context_clone_outdent.borrow();
        if let Some(text_view) = crate::ui::helpers::get_current_text_view(&context.notebook) {
            // Outdent logic will go here
            indentation::outdent_selection(&text_view.buffer());
        }
    });
    app.add_action(&outdent_action);

    let about_action = SimpleAction::new("about", None);
    let app_context_clone_about = app_context_for_closures.clone();
    about_action.connect_activate(move |_, _| {
        let context = app_context_clone_about.borrow();
        let dialog = crate::ui::windows::create_about_dialog(&context.window);
        dialog.present();
    });
    app.add_action(&about_action);

    // Settings Action
    let settings_gtk = Settings::default().unwrap();
    let settings_action = SimpleAction::new("settings", None);
    let app_context_clone_settings = app_context_for_closures.clone();

    settings_action.connect_activate(move |_, _| {
        let context = app_context_clone_settings.borrow();
        // Get current settings to pass to the dialog
        let current_theme = context.app_settings.borrow().theme.clone();
        let current_font = context.app_settings.borrow().font.clone();

        let dialog = crate::ui::windows::create_settings_dialog(
            &context.window,
            &current_theme,
            &current_font,
        );

        let app_context_clone_response = app_context_clone_settings.clone();
        let settings_clone_response = settings_gtk.clone();

        dialog.connect_response(move |d, r| {
            let context_response = app_context_clone_response.borrow();
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
                                        let mut new_settings = context_response.app_settings.borrow_mut();

                                        if let Some(active_id) = combo.active_id() {
                                            let is_dark = active_id == "dark";
                                            new_settings.theme = active_id.to_string();
                                            settings_clone_response
                                                .set_gtk_application_prefer_dark_theme(is_dark);
                                            if is_dark {
                                                let ts_clone = context_response.syntax_context.borrow().ts.clone();
                                                *context_response.syntax_context.borrow_mut().current_theme.borrow_mut() =
                                                    ts_clone.themes["base16-ocean.dark"].clone();
                                            } else {
                                                let ts_clone = context_response.syntax_context.borrow().ts.clone();
                                                *context_response.syntax_context.borrow_mut().current_theme.borrow_mut() =
                                                    ts_clone.themes["InspiredGitHub"].clone();
                                            }

                                            for i in 0..context_response.notebook.n_pages() {
                                                if let Some(page) =
                                                    context_response.notebook.nth_page(Some(i))
                                                {
                                                    if let Some(text_view) = crate::ui::helpers::get_text_view_from_page(&page) {
                                                        let buffer = text_view.buffer();
                                                        (context_response.syntax_context.borrow().highlight_closure)( // Call the closure directly
                                                            buffer.clone(),
                                                        );
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
                                            let mut new_settings =
                                                context_response.app_settings.borrow_mut();
                                            new_settings.font = new_font_desc.to_string();
                                            *context_response.current_font_desc.borrow_mut() =
                                                new_font_desc.clone();
                                            (context_response.update_font)(&new_font_desc); // Call the closure directly
                                        }
                                    }
                                }
                            }
                        }

                        save_settings(&context_response.app_settings.borrow());
                    }
                }
            }
            d.close();
        });

        dialog.present();
    });
    app.add_action(&settings_action);

    let quit_action = SimpleAction::new("quit", None);
    let app_context_clone_quit = app_context_for_closures.clone();
    quit_action.connect_activate(move |_, _| {
        let context = app_context_clone_quit.borrow();
        // Check if any files have unsaved changes
        let (has_unsaved_changes, first_unsaved_buffer, first_unsaved_file_path, first_unsaved_page_index) = {
            let mut has_unsaved_changes = false;
            let mut first_unsaved_buffer = None;
            let mut first_unsaved_file_path = None;
            let mut first_unsaved_page_index = 0;

            for i in 0..context.notebook.n_pages() {
                if let Some(page) = context.notebook.nth_page(Some(i)) {
                    if let Some(text_view) = crate::ui::helpers::get_text_view_from_page(&page) {
                        let buffer = text_view.buffer();
                        let buffer_paths_borrowed = context.buffer_paths.borrow();
                        let file_path = buffer_paths_borrowed.get(&buffer).cloned();

                        if tab_manager::is_buffer_modified(&buffer, file_path.as_ref()) {
                            has_unsaved_changes = true;
                            first_unsaved_buffer = Some(buffer);
                            first_unsaved_file_path = file_path;
                            first_unsaved_page_index = i;
                            break;
                        }
                    }
                }
            }
            (has_unsaved_changes, first_unsaved_buffer, first_unsaved_file_path, first_unsaved_page_index)
        }; // End of the block that defines the variables

        if has_unsaved_changes {
            if let Some(buffer) = first_unsaved_buffer {
                let app_context_clone_for_prompt = app_context_clone_quit.clone();

                tab_manager::prompt_save_changes_async(
                    &context.window,
                    buffer,
                    first_unsaved_file_path,
                    context.buffer_paths.clone(),
                    context.notebook.clone(),
                    first_unsaved_page_index as u32,
                    move |proceed| {
                        if proceed {
                            // User wants to proceed with exit
                            app_context_clone_for_prompt.borrow().app.quit();
                        }
                        // If not proceed, the user cancelled, so we don't exit
                    },
                );
            }
        } else {
            // No unsaved changes, exit immediately
            context.app.quit();
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
    app.set_accels_for_action("app.indent", &["Tab"]);
    app.set_accels_for_action("app.outdent", &["<Control><Shift>Tab"]);
}
