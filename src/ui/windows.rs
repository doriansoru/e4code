use gtk4::{AboutDialog, Dialog, ResponseType, Orientation, Box, Label, ComboBoxText, FontButton};
use gtk4::prelude::*;

/// Creates a settings dialog
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
    let font_button = FontButton::builder()
        .font(current_font)
        .build();
    font_hbox.append(&font_label);
    font_hbox.append(&font_button);
    vbox.append(&font_hbox);
    
    content_area.append(&vbox);
    
    dialog
}

/// Creates an about dialog
pub fn create_about_dialog(parent: &impl IsA<gtk4::Window>) -> AboutDialog {
    let dialog = AboutDialog::builder()
        .transient_for(parent)
        .modal(true)
        .program_name("E4Code")
        .version("0.1.0")
        .comments("A lightweight code editor built with Rust and GTK4.")
        .website("https://github.com/doriansoru/e4code") // Changed to a more specific placeholder
        .authors(vec!["Dorian Soru".to_string(), "Enzo Battero Productions".to_string()]) // Changed to English
        .build();
    dialog
}
