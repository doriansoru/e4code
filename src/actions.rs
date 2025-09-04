use std::collections::HashMap;
use std::path::PathBuf;

use gtk4::prelude::TextViewExt;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Notebook, ResponseType, ScrolledWindow, Settings, TextBuffer,
    TextView, TreeStore,
};

use gio::SimpleAction;
use gtk4::pango;
use std::cell::RefCell;
use std::rc::Rc;
use syntect::highlighting::ThemeSet;

use crate::settings::{AppSettings, save_settings};

use crate::file_operations::{open_directory_dialog, open_file_dialog};
use crate::ui::search_dialog;

use crate::tab_manager;

pub fn open_directory_in_tree(
    path: &PathBuf,
    tree_store: &TreeStore,
    app_settings: &Rc<RefCell<AppSettings>>,
) {
    crate::file_operations::populate_tree_view(tree_store, path);
    app_settings.borrow_mut().last_opened_directory = Some(path.clone());
    save_settings(&app_settings.borrow());
}

use crate::search;

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
                    if let Some(scrolled_window) = text_view_with_line_numbers_box
                        .last_child()
                        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                    {
                        if let Some(text_view) = scrolled_window
                            .child()
                            .and_then(|w| w.downcast::<TextView>().ok())
                        {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_new.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer).cloned();

                            if tab_manager::is_buffer_modified(&buffer, file_path.as_ref()) {
                                // Drop the borrow before showing dialog
                                drop(buffer_paths_borrowed);

                                let notebook_clone = notebook_clone_new.clone();
                                let highlight_closure_clone = highlight_closure_clone_new.clone();
                                let buffer_paths_clone = buffer_paths_clone_new.clone();
                                let app_clone = app_clone_new.clone();
                                let current_font_desc_clone = current_font_desc_clone_new.clone();
                                let update_font_clone = update_font_clone_new.clone();
                                let initial_font_size_clone = initial_font_size_clone_new.clone();
                                let setup_buffer_connections_clone =
                                    setup_buffer_connections_clone_new.clone();

                                // Show save prompt asynchronously
                                tab_manager::prompt_save_changes_async(
                                    &window_clone_new,
                                    buffer,
                                    file_path,
                                    buffer_paths_clone_new.clone(),
                                    notebook_clone_new.clone(),
                                    current_page,
                                    move |proceed| {
                                        if proceed {
                                            // Create new file tab
                                            tab_manager::create_new_file_tab(
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
                                    },
                                );

                                // Return early since we're handling this asynchronously
                                return;
                            }
                        }
                    }
                }
            }
        }

        tab_manager::create_new_file_tab(
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
                    if let Some(scrolled_window) = text_view_with_line_numbers_box
                        .last_child()
                        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                    {
                        if let Some(text_view) = scrolled_window
                            .child()
                            .and_then(|w| w.downcast::<TextView>().ok())
                        {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_open.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer).cloned();

                            if tab_manager::is_buffer_modified(&buffer, file_path.as_ref()) {
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
                                let setup_buffer_connections_clone =
                                    setup_buffer_connections_clone_open.clone();

                                // Show save prompt asynchronously
                                tab_manager::prompt_save_changes_async(
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
                                    },
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
        tab_manager::close_current_tab(
            &window_clone_close,
            &notebook_clone_close,
            &buffer_paths_clone_close,
        );
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
        tab_manager::close_all_tabs_with_prompts(window_clone, notebook_clone, buffer_paths_clone);
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
                    if let Some(scrolled_window) = text_view_with_line_numbers_box
                        .last_child()
                        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                    {
                        if let Some(text_view) = scrolled_window
                            .child()
                            .and_then(|w| w.downcast::<TextView>().ok())
                        {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_save.borrow();
                            let file_path = buffer_paths_borrowed.get(&buffer);

                            if let Some(path) = file_path {
                                // Save to existing file
                                if let Err(e) = tab_manager::save_buffer_to_file(
                                    &window_clone_save,
                                    &buffer,
                                    path,
                                ) {
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
                    if let Some(scrolled_window) = text_view_with_line_numbers_box
                        .last_child()
                        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                    {
                        if let Some(text_view) = scrolled_window
                            .child()
                            .and_then(|w| w.downcast::<TextView>().ok())
                        {
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
                    if let Some(scrolled_window) = text_view_with_line_numbers_box
                        .last_child()
                        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                    {
                        if let Some(text_view) = scrolled_window
                            .child()
                            .and_then(|w| w.downcast::<TextView>().ok())
                        {
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
                                &window_clone_search_replace,
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
                if let Some(display) = gtk4::gdk::Display::default() {
                    // Changed here
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
                if let Some(display) = gtk4::gdk::Display::default() {
                    // Changed here
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
            if let Some(display) = gtk4::gdk::Display::default() {
                // Changed here
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

        let dialog = crate::ui::windows::create_settings_dialog(
            &window_clone,
            &current_theme,
            &current_font,
        );

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
                                            settings_clone_response
                                                .set_gtk_application_prefer_dark_theme(is_dark);
                                            if is_dark {
                                                *current_theme_response.borrow_mut() =
                                                    ts_response.themes["base16-ocean.dark"].clone();
                                            } else {
                                                *current_theme_response.borrow_mut() =
                                                    ts_response.themes["InspiredGitHub"].clone();
                                            }

                                            for i in 0..notebook_response.n_pages() {
                                                if let Some(page) =
                                                    notebook_response.nth_page(Some(i))
                                                {
                                                    // The actual structure is: Box (line_numbers_area + ScrolledWindow (TextView))
                                                    if let Some(text_view_with_line_numbers_box) =
                                                        page.downcast_ref::<gtk4::Box>()
                                                    {
                                                        // Get the second child which should be the ScrolledWindow
                                                        if let Some(scrolled_window) =
                                                            text_view_with_line_numbers_box
                                                                .last_child()
                                                                .and_then(|w| {
                                                                    w.downcast::<ScrolledWindow>()
                                                                        .ok()
                                                                })
                                                        {
                                                            if let Some(text_view) = scrolled_window
                                                                .child()
                                                                .and_then(|w| {
                                                                    w.downcast::<TextView>().ok()
                                                                })
                                                            {
                                                                let buffer = text_view.buffer();
                                                                highlight_closure_response(
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
                            }
                        }

                        // Get font button (second hbox)
                        if let Some(widget) = vbox.last_child() {
                            if let Ok(font_hbox) = widget.downcast::<gtk4::Box>() {
                                if let Some(widget) = font_hbox.last_child() {
                                    if let Ok(font_button) = widget.downcast::<gtk4::FontButton>() {
                                        if let Some(new_font_desc) = font_button.font_desc() {
                                            let mut new_settings =
                                                app_settings_response.borrow_mut();
                                            new_settings.font = new_font_desc.to_string();
                                            *current_font_desc_response.borrow_mut() =
                                                new_font_desc.clone();
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
                    if let Some(scrolled_window) = text_view_with_line_numbers_box
                        .last_child()
                        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
                    {
                        if let Some(text_view) = scrolled_window
                            .child()
                            .and_then(|w| w.downcast::<TextView>().ok())
                        {
                            let buffer = text_view.buffer();
                            let buffer_paths_borrowed = buffer_paths_clone_quit.borrow();
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
            }
        }

        if has_unsaved_changes {
            if let Some(buffer) = first_unsaved_buffer {
                let app_clone2 = app_clone.clone();
                let buffer_paths_clone2 = buffer_paths_clone_quit.clone();
                let notebook_clone2 = notebook_clone_quit.clone();

                tab_manager::prompt_save_changes_async(
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
                    },
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
