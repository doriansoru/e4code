//! Main module for the E4Code application
//!
//! This module sets up the GTK application, initializes the main window,
//! and manages the core application context.

mod actions;
mod buffer_tags;
mod change_tracker;
mod clipboard;
mod dialogs;
mod file_operations;
mod indentation;
mod incremental_highlighting;
pub mod search;
mod settings;
mod syntax_highlighting;
pub mod tab_manager;
mod ui;
mod utils;

use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, HeaderBar, Label, MenuButton, Notebook, Orientation,
    Paned, PopoverMenu, ScrolledWindow, Settings, TextBuffer, TextIter, TextMark, TextView,
    TreeStore, TreeView,
};
use std::collections::HashMap;

use gtk4::pango;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};

use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

use actions::{open_directory_in_tree, setup_actions};
use settings::{AppSettings, load_settings, save_settings};

use file_operations::populate_tree_view;

use gio::{self};
use syntax_highlighting::{SyntaxHighlightingContext};
use change_tracker::ChangeTracker;

/// Application context containing all shared state and components
///
/// This struct holds references to all the major components of the application
/// including the GTK application, settings, buffers, syntax highlighting components,
/// and UI elements.
pub struct AppContext {
    /// The GTK application instance
    pub app: Application,
    /// Application settings
    pub app_settings: Rc<RefCell<AppSettings>>,
    /// Map of text buffers to their file paths
    pub buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>>,
    /// Syntax highlighting context
    pub syntax_context: Rc<RefCell<SyntaxHighlightingContext>>,
    /// Current font description
    pub current_font_desc: Rc<RefCell<pango::FontDescription>>,
    /// Function to update the font
    pub update_font: Rc<dyn Fn(&pango::FontDescription)>,
    /// Initial font size
    pub initial_font_size: Rc<RefCell<f64>>,
    /// Status bar label
    pub status_bar: Rc<RefCell<Label>>,
    /// Last line number tracked
    pub last_line: Rc<RefCell<u32>>,
    /// Last column number tracked
    pub last_col: Rc<RefCell<u32>>,
    /// Function to set up buffer connections
    pub setup_buffer_connections: Rc<dyn Fn(&TextBuffer, &TextView)>,
    /// Tree store for the file tree view
    pub tree_store: TreeStore,
    /// Notebook for tab management
    pub notebook: Notebook,
    /// Main application window
    pub window: ApplicationWindow,
    /// Timer for syntax highlighting
    pub syntax_highlight_timer: Rc<RefCell<Option<glib::SourceId>>>,
    /// Track last changed line for incremental highlighting
    pub last_changed_line: Rc<RefCell<HashMap<TextBuffer, i32>>>,
    /// Change trackers for each buffer
    pub change_trackers: Rc<RefCell<HashMap<TextBuffer, ChangeTracker>>>,
}

impl AppContext {
    /// Creates a new application context and initializes the main window
    ///
    /// This function sets up the entire application, including:
    /// - Loading settings
    /// - Creating the main window and UI elements
    /// - Setting up the file tree view
    /// - Initializing syntax highlighting
    /// - Setting up menus and actions
    ///
    /// # Arguments
    ///
    /// * `app` - The GTK application instance
    ///
    /// # Returns
    ///
    /// A reference-counted pointer to the new application context
    fn new(app: &Application) -> Rc<RefCell<Self>> {
        // --- Initial Setup ---
        let app_settings = Rc::new(RefCell::new(load_settings()));

        let buffer_paths: Rc<RefCell<HashMap<gtk4::TextBuffer, PathBuf>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let initial_directory: PathBuf = app_settings
            .borrow()
            .last_opened_directory
            .clone()
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Save the initial directory if it is valid and not already saved
        if initial_directory.is_dir() {
            app_settings.borrow_mut().last_opened_directory = Some(initial_directory.clone());
            save_settings(&app_settings.borrow());
        }

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("E4Code")
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
        let syntax: Rc<SyntaxReference> = Rc::new(
            ps.find_syntax_by_extension("rs")
                .unwrap_or_else(|| ps.find_syntax_plain_text())
                .clone(),
        );

        let initial_syntax_theme_name = if app_settings.borrow().theme == "dark" {
            "base16-ocean.dark"
        } else {
            "InspiredGitHub"
        };
        let current_theme = Rc::new(RefCell::new(ts.themes[initial_syntax_theme_name].clone()));

        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);

        // Font Description Management
        let initial_font_desc = pango::FontDescription::from_string(&app_settings.borrow().font);
        let current_font_desc = Rc::new(RefCell::new(initial_font_desc));

        let update_font: Rc<dyn Fn(&pango::FontDescription)> = Rc::new({
            let provider = provider.clone();
            let window_clone_for_font_update = window.clone();
            let notebook_clone_for_font_update = notebook.clone();
            move |font_desc: &pango::FontDescription| {
                let family = font_desc.family().unwrap_or("Monospace".into());
                let size_pts = font_desc.size() as f64 / pango::SCALE as f64;
                let css = format!(
                    r#"textview {{ font-family: \"{}\"; font-size: {}pt; }}"#,
                    family,
                    size_pts
                );
                provider.load_from_data(&css);

                // Redraw the current tab\'s line number area and queue a resize for the window
                if let Some(page_num) = notebook_clone_for_font_update.current_page() {
                    if let Some(page) = notebook_clone_for_font_update.nth_page(Some(page_num)) {
                        if let Some(text_view_with_line_numbers_box) =
                            page.downcast_ref::<gtk4::Box>()
                        {
                            if let Some(line_numbers_area) =
                                text_view_with_line_numbers_box.first_child().and_then(|w| {
                                    w.downcast_ref::<gtk4::DrawingArea>().map(|w| w.clone())
                                })
                            {
                                line_numbers_area.queue_draw();
                            }
                        }
                    }
                }
                window_clone_for_font_update.queue_resize();
            }
        });
        update_font(&current_font_desc.borrow());

        // --- Controllers and Signals ---
        let initial_font_size_from_settings = {
            let font_str = &app_settings.borrow().font;
            // Use regex to parse the font size from the font string (e.g., "Monospace 14" -> 14.0)
            let re = regex::Regex::new(r"(\d+(\.\d+)?)$").unwrap();
            if let Some(captures) = re.captures(font_str) {
                if let Some(size_str) = captures.get(1) {
                    size_str.as_str().parse::<f64>().unwrap_or(14.0)
                } else {
                    14.0
                }
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

        // Initialize these Rc<RefCell>s here
        let syntax_highlight_timer = Rc::new(RefCell::new(None::<glib::SourceId>));
        let last_changed_line = Rc::new(RefCell::new(HashMap::new()));
        let change_trackers = Rc::new(RefCell::new(HashMap::<TextBuffer, ChangeTracker>::new()));

        // Create syntax_context with highlight_closure
        let highlight_closure: Rc<dyn Fn(TextBuffer)> = Rc::new({
            let syntax = syntax.clone();
            let ps = ps.clone();
            let current_theme = current_theme.clone();
            let change_trackers_highlight = change_trackers.clone();
            let syntax_context_ref = Rc::new(RefCell::new(None::<SyntaxHighlightingContext>));

            move |buffer: TextBuffer| {
                // Check if this is the first time highlighting (initial load)
                let is_initial_highlight = {
                    let trackers = change_trackers_highlight.borrow();
                    if let Some(tracker) = trackers.get(&buffer) {
                        !tracker.has_changes()
                    } else {
                        true // No tracker found, treat as initial
                    }
                };
                
                if is_initial_highlight {
                    // For initial highlighting, use the full document approach
                    syntax_highlighting::apply_syntax_highlighting(
                        &buffer,
                        &*syntax,
                        &ps,
                        &current_theme.borrow(),
                    );
                } else {
                    // For incremental highlighting, use the changed lines
                    let trackers = change_trackers_highlight.borrow();
                    if let Some(tracker) = trackers.get(&buffer) {
                        if tracker.has_changes() {
                            let changed_lines = tracker.changed_lines.clone();
                            // Get the syntax context from the RefCell
                            if let Some(ref context) = *syntax_context_ref.borrow() {
                                incremental_highlighting::apply_incremental_highlighting(
                                    &buffer,
                                    context,
                                    &changed_lines,
                                );
                            }
                        }
                    }
                }
            }
        });

        let syntax_context = Rc::new(RefCell::new(SyntaxHighlightingContext::new(
            ps,
            ts,
            syntax,
            current_theme,
            highlight_closure,
        )));

        // --- Helper Function for Buffer Connections ---
        let setup_buffer_connections: Rc<dyn Fn(&TextBuffer, &TextView)> = {
            let syntax_context_clone = syntax_context.clone();
            let status_bar = status_bar.clone();
            let last_line = last_line.clone();
            let last_col = last_col.clone();
            let syntax_highlight_timer = syntax_highlight_timer.clone();
            let change_trackers = change_trackers.clone();

            Rc::new(move |buffer: &TextBuffer, text_view: &TextView| {
                // Create the brackets state
                let prev_bracket_pos1 = Rc::new(RefCell::new(None));
                let prev_bracket_pos2 = Rc::new(RefCell::new(None));
                
                // Initialize change tracker for this buffer
                change_trackers.borrow_mut().insert(buffer.clone(), ChangeTracker::new());
                
                // connect_changed
                let syntax_context_clone_for_highlight = syntax_context_clone.clone();
                let syntax_highlight_timer_clone = syntax_highlight_timer.clone();
                let change_trackers_clone = change_trackers.clone();
                buffer.connect_changed(move |buf| {
                    // Track the changes for incremental highlighting
                    let mut trackers = change_trackers_clone.borrow_mut();
                    if let Some(tracker) = trackers.get_mut(buf) {
                        // For now, we'll mark all lines as changed to maintain compatibility
                        // In a more advanced implementation, we would track specific insertions/deletions
                        for i in 0..buf.line_count() {
                            tracker.changed_lines.insert(i);
                        }
                    }
                    drop(trackers); // Release the borrow
                    
                    // Cancel any existing timer
                    if let Some(source_id) = syntax_highlight_timer_clone.borrow_mut().take() {
                        source_id.remove();
                    }

                    let buf_clone = buf.clone();
                    let syntax_context_clone_inner = syntax_context_clone_for_highlight.clone();
                    let timer_ref = syntax_highlight_timer_clone.clone();
                    let change_trackers_timer_clone = change_trackers_clone.clone();

                    // Set a new timer with a shorter delay for more responsive highlighting
                    let source_id = glib::timeout_add_local_once(
                        std::time::Duration::from_millis(30), // Further reduced delay for responsiveness
                        move || {
                            // Apply incremental highlighting
                            let trackers = change_trackers_timer_clone.borrow();
                            if let Some(tracker) = trackers.get(&buf_clone) {
                                if tracker.has_changes() {
                                    let changed_lines = tracker.changed_lines.clone();
                                    incremental_highlighting::apply_incremental_highlighting(
                                        &buf_clone,
                                        &syntax_context_clone_inner.borrow(),
                                        &changed_lines,
                                    );
                                }
                            }
                            drop(trackers); // Release the borrow
                            
                            // Clear the changed lines and timer ID
                            let mut trackers = change_trackers_timer_clone.borrow_mut();
                            if let Some(tracker) = trackers.get_mut(&buf_clone) {
                                tracker.changed_lines.clear();
                            }
                            drop(trackers); // Release the borrow
                            
                            *timer_ref.borrow_mut() = None; // Clear the timer ID once it fires
                        },
                    );
                    *syntax_highlight_timer_clone.borrow_mut() = Some(source_id);
                });

                // connect_mark_set
                let status_bar_clone_for_mark_set_closure = status_bar.clone();
                let text_view_clone_for_mark_set = text_view.clone(); // Clone text_view for this closure
                let last_line_clone_for_mark_set = last_line.clone();
                let last_col_clone_for_mark_set = last_col.clone();
                let prev_bracket_pos1_for_mark_set = prev_bracket_pos1.clone(); // Clone for mark_set closure
                let prev_bracket_pos2_for_mark_set = prev_bracket_pos2.clone(); // Clone for mark_set closure
                buffer.connect_mark_set(
                    move |buffer: &TextBuffer, _iter: &TextIter, mark: &TextMark| {
                        // Ensure we are only reacting to the insert mark (cursor)
                        if mark.name() == Some("insert".into()) {
                            let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());
                            let line = cursor_iter.line() + 1;
                            let col = cursor_iter.line_offset() + 1;

                            if *last_line_clone_for_mark_set.borrow() != (line as u32)
                                || *last_col_clone_for_mark_set.borrow() != (col as u32)
                            {
                                status_bar_clone_for_mark_set_closure
                                    .borrow_mut()
                                    .set_text(&format!("Line {}, Column {}", line, col));
                                *last_line_clone_for_mark_set.borrow_mut() = line as u32;
                                *last_col_clone_for_mark_set.borrow_mut() = col as u32;
                            }
                        }

                        let text_view_for_idle = text_view_clone_for_mark_set.clone();
                        let prev_bracket_pos1_clone_for_idle =
                            prev_bracket_pos1_for_mark_set.clone();
                        let prev_bracket_pos2_clone_for_idle =
                            prev_bracket_pos2_for_mark_set.clone();
                        glib::idle_add_local_once(move || {
                            syntax_highlighting::update_bracket_highlighting(
                                &text_view_for_idle,
                                syntax_highlighting::find_matching_bracket,
                                &prev_bracket_pos1_clone_for_idle,
                                &prev_bracket_pos2_clone_for_idle,
                            );
                        });

                        // Clear existing highlights
                        buffer.remove_tag_by_name(
                            "document_highlight",
                            &buffer.start_iter(),
                            &buffer.end_iter(),
                        );
                    },
                );

                // Note: ScrolledWindow adjustment and buffer changed connections are now handled in tab_manager.rs
                // when creating the text_view_with_line_numbers_box to avoid duplicate widget connections
            })
        };

        let status_bar_clone = status_bar.clone();
        let window_clone = window.clone();
        let notebook_clone = notebook.clone();
        let tree_store_clone = tree_store.clone();

        let new_context_rc = Rc::new(RefCell::new(AppContext {
            app: app.clone(),
            app_settings,
            buffer_paths,
            syntax_context,
            current_font_desc,
            update_font,
            initial_font_size,
            status_bar: status_bar_clone,
            last_line,
            last_col,
            setup_buffer_connections,
            tree_store: tree_store_clone,
            notebook: notebook_clone,
            window: window_clone,
            syntax_highlight_timer,
            last_changed_line,
            change_trackers,
        }));


        // --- Menu and Action Setup ---
        let file_menu_button = MenuButton::builder().label("File").build();
        let file_menu_model = gio::Menu::new();
        file_menu_model.append(Some("New"), Some("app.new"));
        file_menu_model.append(Some("Open"), Some("app.open"));
        file_menu_model.append(Some("Open directory"), Some("app.open_directory"));
        file_menu_model.append(Some("Save"), Some("app.save"));
        file_menu_model.append(Some("Save as"), Some("app.save_as"));
        file_menu_model.append(Some("Close this file"), Some("app.close_current_file"));
        file_menu_model.append(Some("Close all files"), Some("app.close_all_files"));
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
        edit_menu_model.append(Some("Indent"), Some("app.indent"));
        edit_menu_model.append(Some("Outdent"), Some("app.outdent"));
        let edit_popover = PopoverMenu::from_model(Some(&edit_menu_model));
        edit_menu_button.set_popover(Some(&edit_popover));
        header_bar.pack_start(&edit_menu_button);

        let settings_menu_button = MenuButton::builder().label("Settings").build();
        let settings_menu_model = gio::Menu::new();
        settings_menu_model.append(Some("Preferences"), Some("app.settings"));
        let settings_popover = PopoverMenu::from_model(Some(&settings_menu_model));
        settings_menu_button.set_popover(Some(&settings_popover));
        header_bar.pack_start(&settings_menu_button);

        let help_menu_button = MenuButton::builder().label("?").build();
        let help_menu_model = gio::Menu::new();
        help_menu_model.append(Some("About"), Some("app.about"));
        let help_popover = PopoverMenu::from_model(Some(&help_menu_model));
        help_menu_button.set_popover(Some(&help_popover));
        header_bar.pack_start(&help_menu_button);

        // --- Action Definitions ---
        setup_actions(new_context_rc.clone());

        // Populate the tree view with the initial directory
        populate_tree_view(&tree_store, &initial_directory);

        // --- Tree View Row Activation ---
        let app_context_clone_tree_view = new_context_rc.clone();
        tree_view.connect_row_activated(move |_, tree_path, _column| {
            let context = app_context_clone_tree_view.borrow();
            if let Some(iter) = context.tree_store.iter(tree_path) {
                if let Ok(path_value) = context.tree_store
                    .get_value(&iter, 1)
                    .get::<String>()
                {
                    let path = PathBuf::from(path_value);
                    if path.is_file() {
                        tab_manager::open_file_in_new_tab(
                            &path,
                            &app_context_clone_tree_view,
                        );
                    } else if path.is_dir() {
                        open_directory_in_tree(
                            &path,
                            &app_context_clone_tree_view,
                        );
                    }
                }
            }
        });

        vbox.append(&notebook);
        vbox.append(&*status_bar.borrow());
        main_paned.set_end_child(Some(&vbox));
        window.set_child(Some(&main_paned));

        // Connect to the window's close-request signal to handle unsaved changes
        let app_context_clone_for_window_close = new_context_rc.clone();
        window.connect_close_request(move |_window| {
            let context = app_context_clone_for_window_close.borrow();
            
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
                    let app_context_clone_for_prompt = app_context_clone_for_window_close.clone();

                    tab_manager::prompt_save_changes_async(
                        &context.window,
                        buffer,
                        first_unsaved_file_path,
                        context.buffer_paths.clone(),
                        context.notebook.clone(),
                        first_unsaved_page_index as u32,
                        move |proceed| {
                            if proceed {
                                // User wants to proceed with closing the window
                                app_context_clone_for_prompt.borrow().app.quit();
                            }
                            // If not proceed, the user cancelled, so we don't close the window
                        },
                    );
                }
                // Return Inhibit(true) to prevent the window from closing immediately
                glib::Propagation::Stop
            } else {
                // No unsaved changes, allow the window to close
                glib::Propagation::Proceed
            }
        });

        new_context_rc
    }
}

/// Entry point for the E4Code application
///
/// This function initializes the GTK application and sets up the main event loop.
/// It handles both activation (when the app is launched without arguments) and
/// opening files (when files are passed as command line arguments).
fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.e4code.editor")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    // Use a RefCell to allow mutable access to AppState from different closures
    let app_context: Rc<RefCell<Option<Rc<RefCell<AppContext>>>>> = Rc::new(RefCell::new(None));

    app.connect_activate({
        let app_context_clone = app_context.clone();
        move |app: &Application| {
            // Create AppContext only if it hasn't been created by connect_open
            if app_context_clone.borrow().is_none() {
                let new_context = AppContext::new(app);
                let mut opened_any_file = false;
                // If no files were opened via command line, open last opened files
                if new_context
                    .borrow()
                    .app_settings
                    .borrow()
                    .last_opened_files
                    .is_some()
                {
                    if let Some(files_to_open) = new_context
                        .borrow()
                        .app_settings
                        .borrow()
                        .last_opened_files
                        .clone()
                    {
                        for path in files_to_open {
                            if path.is_file() {
                                tab_manager::open_file_in_new_tab(
                                    &path,
                                    &new_context,
                                );
                                opened_any_file = true;
                            }
                        }
                    }
                }
                // If no files were opened (neither from command line nor from settings), create a new untitled tab
                if !opened_any_file {
                    tab_manager::create_new_file_tab(
                        &new_context,
                    );
                }
                *app_context_clone.borrow_mut() = Some(new_context);
            }
            // Present the window
            if let Some(context_ref) = app_context_clone.borrow().as_ref() {
                context_ref.borrow().window.present();
            }
        }
    });

    app.connect_open({
        let app_context_clone = app_context.clone();
        move |app, files, _| {
            // Create AppContext only if it hasn't been created by connect_activate
            if app_context_clone.borrow().is_none() {
                *app_context_clone.borrow_mut() = Some(AppContext::new(app));
            }

            if let Some(context_ref) = app_context_clone.borrow().as_ref() {
                let context = context_ref.borrow();
                for file in files {
                    if let Some(path) = file.path() {
                        if path.is_file() {
                            tab_manager::open_file_in_new_tab(
                                &path,
                                context_ref,
                            );
                        } else if path.is_dir() {
                            open_directory_in_tree(
                                &path,
                                context_ref,
                            );
                        }
                    }
                }
                context.window.present();
            }
        }
    });

    app.run_with_args(&env::args().collect::<Vec<_>>())
}
