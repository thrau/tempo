//! Application entry point.

mod application;
mod config;
mod tray;
mod tray_x11;
#[cfg(test)]
mod ui_tests;
mod window;

pub mod alarm;
mod cycle;
mod notifications;
mod panels;
pub mod storage;
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

    let app = TempoApplication::new();
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/example/Tempo/style.css");
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
    // Run the application.
    app.run()
}
