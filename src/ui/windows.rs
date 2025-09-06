//! Windows UI module
//!
//! This module provides dialog windows for the application, such as
//! settings and about dialogs.

use gtk4::prelude::*;
use gtk4::{AboutDialog, Box, ComboBoxText, Dialog, FontButton, Label, Orientation, ResponseType};

/// Creates a settings dialog
///
/// This function creates a dialog window for configuring application settings
/// such as theme and font preferences.
///
/// # Arguments
///
/// * `parent` - Parent window for the dialog
/// * `current_theme` - Current theme setting ("light" or "dark")
/// * `current_font` - Current font setting in Pango format
///
/// # Returns
///
/// A dialog window for settings configuration
pub fn create_settings_dialog(
    parent: &impl IsA<gtk4::Window>,
    current_theme: &str,
    current_font: &str,
) -> Dialog {
    let dialog = Dialog::builder()
        .title("Settings")
        .transient_for(parent)
        .modal(true)
        .build();

    dialog.add_button("Apply", ResponseType::Apply);
    dialog.add_button("Cancel", ResponseType::Cancel);

    let content_area = dialog.content_area();
    let vbox = Box::new(Orientation::Vertical, 10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);

    let theme_hbox = Box::new(Orientation::Horizontal, 10);
    let theme_label = Label::new(Some("Theme:"));
    let theme_combo = ComboBoxText::new();
    theme_combo.append(Some("light"), "Light");
    theme_combo.append(Some("dark"), "Dark");
    theme_combo.set_active_id(Some(current_theme));
    theme_hbox.append(&theme_label);
    theme_hbox.append(&theme_combo);
    vbox.append(&theme_hbox);

    let font_hbox = Box::new(Orientation::Horizontal, 10);
    let font_label = Label::new(Some("Font:"));
    let font_button = FontButton::builder().font(current_font).build();
    font_hbox.append(&font_label);
    font_hbox.append(&font_button);
    vbox.append(&font_hbox);

    content_area.append(&vbox);

    dialog
}

/// Creates an about dialog
///
/// This function creates a dialog window displaying information about
/// the application, including version, authors, and website.
///
/// # Arguments
///
/// * `parent` - Parent window for the dialog
///
/// # Returns
///
/// An about dialog window
pub fn create_about_dialog(parent: &impl IsA<gtk4::Window>) -> AboutDialog {
    let dialog = AboutDialog::builder()
        .transient_for(parent)
        .modal(true)
        .program_name("E4Code")
        .version("0.1.0")
        .comments("A lightweight code editor built with Rust and GTK4.")
        .website("https://github.com/doriansoru/e4code") // Changed to a more specific placeholder
        .authors(
            env!("CARGO_PKG_AUTHORS")
                .split(':')
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .build();
    dialog
}
