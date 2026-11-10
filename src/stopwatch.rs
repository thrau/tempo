//! Stopwatch feature.
//!
//! # TODO (not implemented yet)
//! - Stopwatch model: elapsed time, state (stopped/running/paused), lap list.
//! - A `Stopwatch` panel widget with start/pause/stop/lap controls.
//! - Measurement of elapsed time against a monotonic clock (e.g. `std::time::Instant`).

/// Placeholder data type for a stopwatch.
///
/// Replace with a real model implementing `glib::Object` once implemented.
#[derive(Default, Debug, Clone)]
pub struct Stopwatch {
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// Whether the stopwatch is running.
    pub running: bool,
    /// Recorded lap times in milliseconds.
    pub laps_ms: Vec<u64>,
}

use gtk::prelude::*;

/// Build the stopwatch panel widget (skeleton — returns a simple label for now).
pub fn build_panel() -> gtk::Widget {
    let label = gtk::Label::new(Some("Stopwatch — not implemented yet"));
    label.upcast::<gtk::Widget>()
}
