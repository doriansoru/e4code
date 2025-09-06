//! UI components module
//!
//! This module provides reusable UI components used throughout the application,
//! such as line number areas and text view containers.

use gtk4::pango;
use gtk4::prelude::*;
use gtk4::{DrawingArea, Orientation, ScrolledWindow, TextView};
use std::cell::RefCell;
use std::rc::Rc;

// Constants for line numbers
/// Width of the line numbers area in pixels
pub const LINE_NUMBER_WIDTH: i32 = 50;
/// Padding around line numbers in pixels
pub const LINE_NUMBER_PADDING: f64 = 5.0;

/// Creates a line numbers area widget for a text view
///
/// This function creates a drawing area that displays line numbers alongside
/// a text view, automatically updating as the text content changes.
///
/// # Arguments
///
/// * `text_view` - The text view to display line numbers for
/// * `scrolled_window` - The scrolled window containing the text view
/// * `current_font_desc` - Reference to the current font description
///
/// # Returns
///
/// A drawing area widget that displays line numbers
pub fn create_line_numbers_area(
    text_view: &TextView,
    scrolled_window: &ScrolledWindow,
    current_font_desc: Rc<RefCell<pango::FontDescription>>,
) -> DrawingArea {
    let line_numbers_area = DrawingArea::new();
    line_numbers_area.set_width_request(LINE_NUMBER_WIDTH);
    line_numbers_area.set_hexpand(false);
    line_numbers_area.set_vexpand(true);

    line_numbers_area.clone().set_draw_func({
        let text_view_clone = text_view.clone();
        let scrolled_window_clone = scrolled_window.clone();
        let current_font_desc_clone = current_font_desc.clone();
        let line_numbers_area_clone_for_closure = line_numbers_area.clone();

        move |_, cr, width, height| {
            let text_view = text_view_clone.clone();
            let vadjustment = scrolled_window_clone.vadjustment();
            let font_desc = current_font_desc_clone.borrow();
            let font_size_pts = font_desc.size() as f64 / pango::SCALE as f64;

            cr.set_source_rgb(0.95, 0.95, 0.95); // Light gray background
            cr.paint().expect("Failed to paint background");

            cr.set_source_rgb(0.2, 0.2, 0.2); // Dark gray for text
            let buffer = text_view.buffer();

            cr.set_font_size(font_size_pts);

            // Calculate dynamic width for line numbers area
            let max_line_number = buffer.line_count().max(1);
            let max_digits = max_line_number.to_string().len();
            let test_string = "8".repeat(max_digits);
            let extents = cr
                .text_extents(&test_string)
                .expect("Failed to get text extents");
            let required_width = extents.width() + LINE_NUMBER_PADDING * 2.0;

            // Update the width_request of the DrawingArea
            if (line_numbers_area_clone_for_closure.width_request() as f64 - required_width).abs()
                > 1.0
            {
                line_numbers_area_clone_for_closure.set_width_request(required_width as i32);
            }

            let scroll_y = vadjustment.value();
            let allocation_height = text_view.allocation().height() as f64;

            // More accurate line height calculation using Pango
            let pango_context = text_view.pango_context();
            let font_metrics = pango_context.metrics(Some(&font_desc), None);
            let line_height =
                (font_metrics.ascent() + font_metrics.descent()) as f64 / pango::SCALE as f64;

            // Calculate visible lines range
            let start_line = (scroll_y / line_height).floor() as i32;
            let end_line = ((scroll_y + allocation_height) / line_height).ceil() as i32 + 1;

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

    line_numbers_area
}

/// Creates a text view with line numbers in a horizontal box
///
/// This function creates a container that holds both a line numbers area
/// and a text view, arranging them horizontally.
///
/// # Arguments
///
/// * `_text_view` - The text view (unused in current implementation)
/// * `scrolled_window` - The scrolled window containing the text view
/// * `line_numbers_area` - The line numbers area to display
///
/// # Returns
///
/// A horizontal box containing the line numbers area and text view
pub fn create_text_view_with_line_numbers(
    _text_view: &TextView,
    scrolled_window: &ScrolledWindow,
    line_numbers_area: &DrawingArea,
) -> gtk4::Box {
    let text_view_with_line_numbers_box = gtk4::Box::new(Orientation::Horizontal, 0);
    text_view_with_line_numbers_box.append(line_numbers_area);
    text_view_with_line_numbers_box.append(scrolled_window);
    text_view_with_line_numbers_box
}
