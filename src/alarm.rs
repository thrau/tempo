//! Alarm feature.
//!
//! # TODO (not implemented yet)
//! - Alarm model: time-of-day, weekdays, label, enabled flag.
//! - `ListBox` UI listing alarms with add/remove controls.
//! - GSettings persistence.
//! - Firing logic when the alarm time is reached (notification/ring).

/// Placeholder data type for a single alarm.
///
/// Replace with a real model implementing `glib::Object` once implemented.
#[derive(Default, Debug, Clone)]
pub struct Alarm {
    /// Time of day in "HH:MM" 24h format.
    pub time: String,
    /// Optional label.
    pub label: String,
    /// Whether the alarm is enabled.
    pub enabled: bool,
}

use gtk::prelude::*;

/// Build the alarm panel widget (skeleton — returns a simple label for now).
pub fn build_panel() -> gtk::Widget {
    let label = gtk::Label::new(Some("Alarm — not implemented yet"));
    label.upcast::<gtk::Widget>()
}
