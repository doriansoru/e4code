use gtk4::gdk;
use gtk4::pango;
use gtk4::prelude::*;
use gtk4::{Application, EventControllerKey, EventControllerScroll, TextView};
use std::cell::RefCell;
use std::rc::Rc;

/// Adds zoom controllers to a text view
pub fn add_zoom_controllers_to_text_view(
    text_view: &TextView,
    current_font_desc: Rc<RefCell<pango::FontDescription>>,
    update_font: Rc<dyn Fn(&pango::FontDescription)>,
    app: Application,
    initial_font_size: Rc<RefCell<f64>>,
) {
    let scroll_controller = EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    let key_controller = EventControllerKey::new();

    // Zoom functionality
    let current_font_desc_clone_scroll = current_font_desc.clone();
    let update_font_clone_scroll = update_font.clone();
    let app_clone_scroll = app.clone();
    let scroll_controller_clone = scroll_controller.clone();
    scroll_controller.connect_scroll(move |_, _, dy| {
        if let Some(_) = app_clone_scroll.active_window() {
            if scroll_controller_clone
                .current_event_state()
                .contains(gdk::ModifierType::CONTROL_MASK)
            {
                let mut font_desc = current_font_desc_clone_scroll.borrow_mut();
                let mut current_size = font_desc.size() as f64 / pango::SCALE as f64;
                if dy < 0.0 {
                    // Scroll up
                    current_size *= 1.1; // Zoom in
                } else {
                    // Scroll down
                    current_size /= 1.1; // Zoom out
                }
                font_desc.set_size((current_size * pango::SCALE as f64) as i32);
                update_font_clone_scroll(&font_desc);
                return glib::Propagation::Stop; // Inhibits default scroll behavior
            }
        }
        glib::Propagation::Proceed
    });

    let current_font_desc_clone_key = current_font_desc.clone();
    let update_font_clone_key = update_font.clone();
    let initial_font_size_clone_key = initial_font_size.clone();
    let app_clone_key = app.clone();
    key_controller.connect_key_pressed(move |_, keyval, _, state| {
        if let Some(_) = app_clone_key.active_window() {
            if state.contains(gdk::ModifierType::CONTROL_MASK) {
                let mut font_desc = current_font_desc_clone_key.borrow_mut();
                let mut current_size = font_desc.size() as f64 / pango::SCALE as f64;
                let mut changed = false;

                match keyval {
                    gdk::Key::plus | gdk::Key::equal => {
                        // Ctrl + '+' o Ctrl + '='
                        current_size *= 1.1;
                        changed = true;
                    }
                    gdk::Key::minus => {
                        // Ctrl + '-'
                        current_size /= 1.1;
                        changed = true;
                    }
                    gdk::Key::_0 => {
                        // Ctrl + '0'
                        current_size = *initial_font_size_clone_key.borrow();
                        changed = true;
                    }
                    _ => {}
                }

                if changed {
                    font_desc.set_size((current_size * pango::SCALE as f64) as i32);
                    update_font_clone_key(&font_desc);
                    return glib::Propagation::Stop; // Inhibits default key behavior
                }
            }
        }
        glib::Propagation::Proceed
    });

    text_view.add_controller(scroll_controller);
    text_view.add_controller(key_controller);
}
