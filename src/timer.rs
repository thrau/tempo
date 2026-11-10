//! Timer feature (countdown timer).
//!
//! # TODO (not implemented yet)
//! - Timer model: duration, remaining time, state (idle/running/paused/finished).
//! - A `Timer` panel widget shown in the main window.
//! - Persistence of presets via GSettings.
//! - Notification / audible alert when a timer finishes.

/// Placeholder data type for a single timer.
///
/// Replace with a real model implementing `glib::Object` once implemented.
#[derive(Default, Debug, Clone)]
pub struct Timer {
    /// Duration in seconds.
    pub duration_seconds: u32,
    /// Whether the timer is currently running.
    pub running: bool,
}

use gtk::prelude::*;

/// Build the timer panel widget (skeleton — returns a simple label for now).
pub fn build_panel() -> gtk::Widget {
    let label = gtk::Label::new(Some("Timer — not implemented yet"));
    label.upcast::<gtk::Widget>()
}
