use gtk4::prelude::*;
use gtk4::{ButtonsType, MessageDialog};

/// Creates and shows an error dialog
pub fn show_error_dialog(
    parent: &impl IsA<gtk4::Window>,
    title: &str,
    message: &str,
) -> MessageDialog {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .buttons(ButtonsType::Ok)
        .text(title)
        .secondary_text(message)
        .build();
    
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.present();
    dialog
}