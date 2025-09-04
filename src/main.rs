mod actions;
mod settings;
mod ui;
mod file_operations;
mod syntax_highlighting;
mod utils;

use std::collections::HashMap;
use gtk4::{Application, Box, HeaderBar, MenuButton, Orientation, PopoverMenu, Notebook, Label, TextView, TextBuffer, Settings, TextMark, TextIter, Paned, ScrolledWindow, TreeView, TreeStore, ApplicationWindow, DrawingArea};
use gtk4::prelude::*;

use gtk4::pango;
use syntect::parsing::{SyntaxSet, SyntaxReference};
use syntect::highlighting::{ThemeSet, Theme};
use gtk4::gdk;

use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::env;

use settings::{load_settings, save_settings, AppSettings};
use actions::{setup_actions, open_file_in_new_tab, open_directory_in_tree};
use file_operations::populate_tree_view;
use utils::add_zoom_controllers_to_text_view;
use ui::components::{find_matching_bracket, LINE_NUMBER_WIDTH, LINE_NUMBER_PADDING};
use gio::{self};






// Struct to contain application state
struct AppState {
    app_settings: Rc<RefCell<AppSettings>>,
    buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    ps: Rc<SyntaxSet>,
    ts: Rc<ThemeSet>,
    syntax: Rc<SyntaxReference>,
    current_theme: Rc<RefCell<Theme>>,
    current_font_desc: Rc<RefCell<pango::FontDescription>>,
    update_font: Rc<dyn Fn(&pango::FontDescription)>,
    initial_font_size: Rc<RefCell<f64>>,
    status_bar: Rc<RefCell<Label>>,
    last_line: Rc<RefCell<u32>>,
    last_col: Rc<RefCell<u32>>,
    setup_buffer_connections: Rc<dyn Fn(&TextBuffer, &TextView)>,
    tree_store: TreeStore,
    notebook: Notebook,
    window: ApplicationWindow,
    highlight_closure: Rc<dyn Fn(TextBuffer)>,
    line_numbers_area: DrawingArea,
}

impl AppState {
    fn new(app: &Application) -> Rc<RefCell<Self>> {
        // --- Initial Setup ---
        let app_settings = Rc::new(RefCell::new(load_settings()));

        let buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>> = Rc::new(RefCell::new(HashMap::new()));
        
        let initial_directory: PathBuf = app_settings.borrow().last_opened_directory.clone()
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Save the initial directory if it is valid and not already saved
        if initial_directory.is_dir() {
            app_settings.borrow_mut().last_opened_directory = Some(initial_directory.clone());
            save_settings(&app_settings.borrow());
        }

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("E4Code - GTK Version")
            .default_width(800)
            .default_height(600)
            .build();

        let provider = gtk4::CssProvider::new();
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let settings = Settings::default().unwrap();
        settings.set_gtk_application_prefer_dark_theme(app_settings.borrow().theme == "dark");

        // --- Main Layout ---
        let main_paned = Paned::new(Orientation::Horizontal);
        main_paned.set_hexpand(true);
        main_paned.set_vexpand(true);

        let vbox = Box::new(Orientation::Vertical, 0);
        let header_bar = HeaderBar::new();
        window.set_titlebar(Some(&header_bar));

        // --- Directory Tree View ---
        let tree_store = TreeStore::new(&[String::static_type(), String::static_type()]); // Column 0: Name, Column 1: Path
        let tree_view = TreeView::builder()
            .model(&tree_store)
            .hexpand(true)
            .vexpand(true)
            .build();

        let column = gtk4::TreeViewColumn::new();
        let cell = gtk4::CellRendererText::new();
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", 0);
        tree_view.append_column(&column);

        let tree_scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&tree_view)
            .build();
        main_paned.set_start_child(Some(&tree_scrolled_window));

        // --- Data and State Initialization ---
        let ps = Rc::new(SyntaxSet::load_defaults_newlines());
        let ts = Rc::new(ThemeSet::load_defaults());
        let syntax: Rc<SyntaxReference> = Rc::new(ps.find_syntax_by_extension("rs").unwrap_or_else(|| ps.find_syntax_plain_text()).clone());
        
        let initial_syntax_theme_name = if app_settings.borrow().theme == "dark" { "base16-ocean.dark" } else { "InspiredGitHub" };
        let current_theme = Rc::new(RefCell::new(ts.themes[initial_syntax_theme_name].clone()));

        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);

        // Font Description Management
        let initial_font_desc = pango::FontDescription::from_string(&app_settings.borrow().font);
        let current_font_desc = Rc::new(RefCell::new(initial_font_desc));

        let update_font: Rc<dyn Fn(&pango::FontDescription)> = Rc::new({
            let provider = provider.clone();
            let notebook_clone_for_font_update = notebook.clone(); // Clone notebook
            move |font_desc: &pango::FontDescription| {
                let family = font_desc.family().unwrap_or("Monospace".into());
                let size_pts = font_desc.size() as f64 / pango::SCALE as f64;
                let css = format!(
                    r#"textview {{ font-family: "{}"; font-size: {}pt; }}"#,
                    family,
                    size_pts
                );
                provider.load_from_data(&css);

                // Trigger redraw of line numbers area for the current tab
                if let Some(page_num) = notebook_clone_for_font_update.current_page() {
                    if let Some(page) = notebook_clone_for_font_update.nth_page(Some(page_num)) {
                        if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                            if let Some(line_numbers_area) = text_view_with_line_numbers_box.first_child().and_then(|w| w.downcast_ref::<gtk4::DrawingArea>().map(|w| w.clone())) {
                                line_numbers_area.queue_draw();
                            }
                        }
                    }
                }
            }
        });
        update_font(&current_font_desc.borrow());

        // --- Controllers and Signals ---
        let initial_font_size_from_settings = {
            let font_str = &app_settings.borrow().font;
            // Parse the font size from the font string (e.g., "Monospace 14" -> 14.0)
            let parts: Vec<&str> = font_str.split_whitespace().collect();
            if parts.len() >= 2 {
                parts.last().unwrap_or(&"14.0").parse::<f64>().unwrap_or(14.0)
            } else {
                14.0
            }
        };
        let initial_font_size = Rc::new(RefCell::new(initial_font_size_from_settings));
        
        let status_bar = Rc::new(RefCell::new(Label::new(Some("Line 1, Column 1"))));
        status_bar.borrow_mut().set_halign(gtk4::Align::Start);
        status_bar.borrow_mut().set_margin_start(5);
        status_bar.borrow_mut().set_margin_end(5);
        status_bar.borrow_mut().set_margin_top(2);
        status_bar.borrow_mut().set_margin_bottom(2);

        let last_line = Rc::new(RefCell::new(1u32));
        let last_col = Rc::new(RefCell::new(1u32));

        // --- Main Closures ---

        let highlight_closure: Rc<dyn Fn(TextBuffer)> = Rc::new({
            let syntax = syntax.clone();
            let ps = ps.clone();
            let current_theme = current_theme.clone();

            move |buffer: TextBuffer| {
                syntax_highlighting::apply_syntax_highlighting(&buffer, &*syntax, &ps, &current_theme.borrow());
            }
        });

        // --- Helper Function for Buffer Connections ---
        let setup_buffer_connections: Rc<dyn Fn(&TextBuffer, &TextView)> = {
            let highlight_closure = highlight_closure.clone();
            let status_bar = status_bar.clone();
            let last_line = last_line.clone();
            let last_col = last_col.clone();

            Rc::new(move |buffer: &TextBuffer, text_view: &TextView| {
                // connect_changed
                let highlight_closure_clone = highlight_closure.clone();
                buffer.connect_changed(move |buf| {
                    highlight_closure_clone(buf.clone());
                });
        
                // connect_mark_set
                let status_bar_clone_for_mark_set_closure = status_bar.clone();
                let text_view_clone_for_mark_set = text_view.clone(); // Clone text_view for this closure
                let last_line_clone_for_mark_set = last_line.clone();
                let last_col_clone_for_mark_set = last_col.clone();
                buffer.connect_mark_set(move |buffer: &TextBuffer, _iter: &TextIter, mark: &TextMark| {
                    // Ensure we are only reacting to the insert mark (cursor)
                    if mark.name() == Some("insert".into()) {
                        let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());
                        let line = cursor_iter.line() + 1;
                        let col = cursor_iter.line_offset() + 1;

                        if *last_line_clone_for_mark_set.borrow() != (line as u32) || *last_col_clone_for_mark_set.borrow() != (col as u32) {
                            status_bar_clone_for_mark_set_closure.borrow_mut().set_text(&format!("Linea {}, Colonna {}", line, col));
                            *last_line_clone_for_mark_set.borrow_mut() = line as u32;
                            *last_col_clone_for_mark_set.borrow_mut() = col as u32;
                        }
                    }

                    let text_view_for_idle = text_view_clone_for_mark_set.clone();
                glib::idle_add_local_once(move || {
                    syntax_highlighting::update_bracket_highlighting(&text_view_for_idle, find_matching_bracket);
                });

                    // Clear existing highlights
                    buffer.remove_tag_by_name("document_highlight", &buffer.start_iter(), &buffer.end_iter());

                });

                // Connect to ScrolledWindow's adjustment value-changed for scrolling
                if let Some(parent_widget) = text_view.parent() {
                    if let Some(parent_box) = parent_widget.downcast_ref::<gtk4::Box>() {
                        if let Some(line_numbers_area) = parent_box.first_child().and_then(|w| w.downcast_ref::<gtk4::DrawingArea>().map(|w| w.clone())) {
                            if let Some(scrolled_window) = line_numbers_area.next_sibling().and_then(|w| w.downcast_ref::<gtk4::ScrolledWindow>().map(|w| w.clone())) {
                                let text_view_clone_for_scroll = text_view.clone();
                                let line_numbers_area_clone_for_scroll = line_numbers_area.clone();
                                scrolled_window.vadjustment().connect_value_changed(move |_| {
                                    syntax_highlighting::update_bracket_highlighting(&text_view_clone_for_scroll, find_matching_bracket);
                                    line_numbers_area_clone_for_scroll.queue_draw();
                                });

                                // Connect to text_buffer's changed signal to update line numbers if text changes
                                let line_numbers_area_clone_for_changed = line_numbers_area.clone();
                                buffer.connect_changed(move |_| {
                                    line_numbers_area_clone_for_changed.queue_draw();
                                });
                            }
                        }
                    }
                }
            })
        };

        // --- Initial Buffer ---
        let text_buffer = TextBuffer::builder().text("").build();
        // Add highlight tag to the initial buffer's tag table
        let highlight_tag = gtk4::TextTag::new(Some("document_highlight"));
        highlight_tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(0.0, 0.0, 1.0, 0.3))); // Blue with some transparency
        text_buffer.tag_table().add(&highlight_tag);
        // Add bracket_match tag to initial buffer
        let bracket_match_tag = gtk4::TextTag::new(Some("bracket_match"));
        let yellow = gdk::RGBA::new(1.0f32, 1.0f32, 0.0f32, 0.5f32);
        bracket_match_tag.set_property("background-rgba", &yellow);
        bracket_match_tag.set_scale(1.3);
        bracket_match_tag.set_weight(700);
        
        text_buffer.tag_table().add(&bracket_match_tag);
        
        let text_view = TextView::builder()
            .buffer(&text_buffer)
            .hexpand(true)
            .vexpand(true)
            .build();

        let scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&text_view)
            .build();
        
        // Line numbers area
        let line_numbers_area = gtk4::DrawingArea::new();
        line_numbers_area.set_width_request(LINE_NUMBER_WIDTH);
        line_numbers_area.set_hexpand(false);
        line_numbers_area.set_vexpand(true);
        line_numbers_area.clone().set_draw_func({ // Added .clone()
            let text_view_clone = text_view.clone();
            let scrolled_window_clone = scrolled_window.clone();
            let current_font_desc_clone = current_font_desc.clone(); // Clone font description
            let line_numbers_area_clone_for_closure = line_numbers_area.clone(); // Clone for use inside closure
            move |_, cr, width, height| {
                let text_view = text_view_clone.clone();
                let vadjustment = scrolled_window_clone.vadjustment();
                let font_desc = current_font_desc_clone.borrow(); // Borrow font description
                let font_size_pts = font_desc.size() as f64 / pango::SCALE as f64; // Get font size in points

                cr.set_source_rgb(0.95, 0.95, 0.95); // Light gray background
                cr.paint().expect("Failed to paint background");

                cr.set_source_rgb(0.2, 0.2, 0.2); // Dark gray for text
                let buffer = text_view.buffer(); // Get buffer from text_view (moved to top)

                cr.set_font_size(font_size_pts); // Use dynamic font size

                // Calculate dynamic width for line numbers area
                let max_line_number = buffer.line_count().max(1); // Get total lines, at least 1
                let max_digits = max_line_number.to_string().len();
                let test_string = "8".repeat(max_digits); // Use '8' as it's typically wide
                let extents = cr.text_extents(&test_string).expect("Failed to get text extents");
                let required_width = extents.width() + LINE_NUMBER_PADDING * 2.0; // Add padding

                // Update the width_request of the DrawingArea
                // Only update if significantly different to avoid excessive redraws
                if (line_numbers_area_clone_for_closure.width_request() as f64 - required_width).abs() > 1.0 {
                    line_numbers_area_clone_for_closure.set_width_request(required_width as i32);
                }

                let scroll_y = vadjustment.value();
                let allocation_height = text_view.allocation().height() as f64;

                // More accurate line height calculation using Pango
                let pango_context = text_view.pango_context();
                let font_metrics = pango_context.metrics(Some(&font_desc), None);
                let line_height = (font_metrics.ascent() + font_metrics.descent()) as f64 / pango::SCALE as f64;

                // Calculate visible lines range
                let start_line = (scroll_y / line_height).floor() as i32;
                let end_line = ((scroll_y + allocation_height) / line_height).ceil() as i32 + 1; // +1 for safety

                // Ensure we don't go out of bounds
                let start_line = start_line.max(0);
                let end_line = end_line.min(buffer.line_count().max(1));

                // Draw line numbers for visible lines
                for line_num in start_line..end_line {
                    if let Some(iter) = buffer.iter_at_line(line_num) {
                        let (line_y_start, _) = text_view.line_yrange(&iter);
                        let display_y = line_y_start as f64 - scroll_y;

                        // Only draw if the line is visible
                        if display_y + line_height >= 0.0 && display_y <= height as f64 {
                            let line_number = line_num + 1;
                            let text = format!("{}", line_number);
                            let extents = cr.text_extents(&text).expect("Failed to get text extents");
                            let x = width as f64 - extents.width() - LINE_NUMBER_PADDING;
                            let y = display_y + (line_height / 2.0) + (extents.height() / 2.0);

                            cr.move_to(x, y);
                            cr.show_text(&text).expect("Failed to draw text");
                        }
                    }
                }
            }
        });

        let text_view_with_line_numbers_box = gtk4::Box::new(Orientation::Horizontal, 0);
        text_view_with_line_numbers_box.append(&line_numbers_area);
        text_view_with_line_numbers_box.append(&scrolled_window);
        
        notebook.append_page(&text_view_with_line_numbers_box, Some(&Label::new(Some("Untitled-1"))));

        add_zoom_controllers_to_text_view(
            &text_view,
            current_font_desc.clone(),
            update_font.clone(),
            app.clone(),
            initial_font_size.clone(),
        );

        setup_buffer_connections(&text_buffer, &text_view);
        highlight_closure(text_buffer.clone());

        // --- Menu and Action Setup ---
        let file_menu_button = MenuButton::builder().label("File").build();
        let file_menu_model = gio::Menu::new();
        file_menu_model.append(Some("New"), Some("app.new"));
        file_menu_model.append(Some("Open"), Some("app.open"));
        file_menu_model.append(Some("Open directory"), Some("app.open_directory"));
        file_menu_model.append(Some("Close this file"), Some("app.close_current_file"));
        file_menu_model.append(Some("Close all files"), Some("app.close_all_files"));
        file_menu_model.append(Some("Save"), Some("app.save"));
        file_menu_model.append(Some("Save as"), Some("app.save_as"));
        file_menu_model.append(Some("Exit"), Some("app.quit"));
        let file_popover = PopoverMenu::from_model(Some(&file_menu_model));
        file_menu_button.set_popover(Some(&file_popover));
        header_bar.pack_start(&file_menu_button);

        let edit_menu_button = MenuButton::builder().label("Edit").build();
        let edit_menu_model = gio::Menu::new();
        edit_menu_model.append(Some("Search and replace"), Some("app.search_and_replace"));
        edit_menu_model.append(Some("Cut"), Some("app.cut"));
        edit_menu_model.append(Some("Copy"), Some("app.copy"));
        edit_menu_model.append(Some("Paste"), Some("app.paste"));
        edit_menu_model.append(Some("Settings"), Some("app.settings"));
        let edit_popover = PopoverMenu::from_model(Some(&edit_menu_model));
        edit_menu_button.set_popover(Some(&edit_popover));
        header_bar.pack_start(&edit_menu_button);

        let help_menu_button = MenuButton::builder().label("?").build();
        let help_menu_model = gio::Menu::new();
        help_menu_model.append(Some("About"), Some("app.about"));
        let help_popover = PopoverMenu::from_model(Some(&help_menu_model));
        help_menu_button.set_popover(Some(&help_popover));
        header_bar.pack_start(&help_menu_button);

        // --- Action Definitions ---
        setup_actions(
            app,
            &window,
            &notebook,
            &highlight_closure,
            &current_theme,
            &ts,
            &current_font_desc,
            &update_font,
            &app_settings,
            &tree_store,
            &buffer_paths,
            &initial_font_size,
            &setup_buffer_connections,
        );

        // Populate the tree view with the initial directory
        populate_tree_view(&tree_store, &initial_directory);

        // --- Tree View Row Activation ---
        let notebook_clone_tree_view = notebook.clone();
        let highlight_closure_clone_tree_view = highlight_closure.clone();
        let tree_store_clone_tree_view = tree_store.clone();
        let buffer_paths_for_tree_view_closure = buffer_paths.clone();
        let app_clone_tree_view = app.clone();
        let current_font_desc_clone_tree_view = current_font_desc.clone();
        let update_font_clone_tree_view = update_font.clone();
        let initial_font_size_clone_tree_view = initial_font_size.clone();
        let setup_buffer_connections_clone_tree_view = setup_buffer_connections.clone();
        let app_settings_clone_tree_view = app_settings.clone();
        
        tree_view.connect_row_activated(move |_, tree_path, _| {
            if let Some(iter) = tree_store_clone_tree_view.iter(tree_path) {
                if let Ok(path_value) = tree_store_clone_tree_view.get_value(&iter, 1).get::<String>() {
                    let path = PathBuf::from(path_value);
                    if path.is_file() {
                        open_file_in_new_tab(
                            &path,
                            &notebook_clone_tree_view,
                            &highlight_closure_clone_tree_view,
                            &buffer_paths_for_tree_view_closure,
                            &app_clone_tree_view,
                            &current_font_desc_clone_tree_view,
                            &update_font_clone_tree_view,
                            &initial_font_size_clone_tree_view,
                            &setup_buffer_connections_clone_tree_view,
                        );
                    } else if path.is_dir() {
                        open_directory_in_tree(
                            &path,
                            &tree_store_clone_tree_view,
                            &app_settings_clone_tree_view,
                        );
                    }
                }
            }
        });
        
        vbox.append(&notebook);
        vbox.append(&*status_bar.borrow());
        main_paned.set_end_child(Some(&vbox));
        window.set_child(Some(&main_paned));

        let app_clone_for_close = app.clone();
        let notebook_clone_for_close = notebook.clone();
        let buffer_paths_clone_for_close = buffer_paths.clone();
        window.connect_close_request(move |win| {
            // Check if any files have unsaved changes
            let mut has_unsaved_changes = false;
            let mut first_unsaved_buffer = None;
            let mut first_unsaved_file_path = None;
            let mut first_unsaved_page_index = 0;
            
            for i in 0..notebook_clone_for_close.n_pages() {
                if let Some(page) = notebook_clone_for_close.nth_page(Some(i)) {
                    if let Some(text_view_with_line_numbers_box) = page.downcast_ref::<gtk4::Box>() {
                        if let Some(scrolled_window) = text_view_with_line_numbers_box.last_child().and_then(|w| w.downcast::<ScrolledWindow>().ok()) {
                            if let Some(text_view) = scrolled_window.child().and_then(|w| w.downcast::<TextView>().ok()) {
                                let buffer = text_view.buffer();
                                let buffer_paths_borrowed = buffer_paths_clone_for_close.borrow();
                                let file_path = buffer_paths_borrowed.get(&buffer).cloned();
                                
                                if crate::actions::is_buffer_modified(&buffer, file_path.as_ref()) {
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
                    let app_clone2 = app_clone_for_close.clone();
                    let buffer_paths_clone2 = buffer_paths_clone_for_close.clone();
                    let notebook_clone2 = notebook_clone_for_close.clone();
                    let window_clone2 = win.clone();
                    
                    crate::actions::prompt_save_changes_async(
                        &window_clone2,
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
                    // Return glib::Propagation::Stop to prevent the window from closing immediately
                    return glib::Propagation::Stop;
                }
            }
            
            // No unsaved changes, exit immediately
            win.application().expect("No application associated with window").quit();
            glib::Propagation::Proceed
        });

        Rc::new(RefCell::new(AppState {
            app_settings,
            buffer_paths,
            ps,
            ts,
            syntax,
            current_theme,
            current_font_desc,
            update_font,
            initial_font_size,
            status_bar,
            last_line,
            last_col,
            setup_buffer_connections,
            tree_store,
            notebook,
            window,
            highlight_closure,
            line_numbers_area,
        }))
    }
}
        
fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.e4code.editor")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    // Use a RefCell to allow mutable access to AppState from different closures
    let app_state: Rc<RefCell<Option<Rc<RefCell<AppState>>>>> = Rc::new(RefCell::new(None));

    app.connect_activate({
        let app_state = app_state.clone();
        move |app: &Application| {
            // Create AppState only if it hasn't been created by connect_open
            if app_state.borrow().is_none() {
                *app_state.borrow_mut() = Some(AppState::new(app));
            }
            // Present the window
            if let Some(state) = app_state.borrow().as_ref() {
                state.borrow().window.present();
            }
        }
    });

    app.connect_open({
        let app_state = app_state.clone();
        move |app, files, _| {
            // Create AppState only if it hasn't been created by connect_activate
            if app_state.borrow().is_none() {
                *app_state.borrow_mut() = Some(AppState::new(app));
            }

            if let Some(state) = app_state.borrow().as_ref() {
                let state_borrowed = state.borrow();
                for file in files {
                    if let Some(path) = file.path() {
                        if path.is_file() {
                            open_file_in_new_tab(
                                &path,
                                &state_borrowed.notebook,
                                &state_borrowed.highlight_closure,
                                &state_borrowed.buffer_paths,
                                app,
                                &state_borrowed.current_font_desc,
                                &state_borrowed.update_font,
                                &state_borrowed.initial_font_size,
                                &state_borrowed.setup_buffer_connections,
                            );
                        } else if path.is_dir() {
                            open_directory_in_tree(
                                &path,
                                &state_borrowed.tree_store,
                                &state_borrowed.app_settings,
                            );
                        }
                    }
                }
                state_borrowed.window.present();
            }
        }
    });

    app.run_with_args(&env::args().collect::<Vec<_>>())
}



