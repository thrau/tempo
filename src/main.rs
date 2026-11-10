//! Application entry point.

mod application;
mod config;
mod window;

// Feature skeletons (Clocks-like): implementations come later.
pub mod alarm;
pub mod stopwatch;
pub mod timer;

use gtk::prelude::*;

use crate::application::TempoApplication;
use gettextrs::*;

fn main() -> glib::ExitCode {
    // Initialize translations (best-effort: only present in packaged installs).
    // The TextDomain builder searches the standard system/XDG data dirs for the
    // compiled `tempo.mo` files under po/.
    let _ = TextDomain::new("tempo").init();

    // Register the GResource that embeds our UI/CSS so GtkBuilder can find them.
    gio::resources_register_include!("tempo.gresource").expect("Failed to register resources.");

    // Run the application.
    let app = TempoApplication::new();
    app.run()
}
