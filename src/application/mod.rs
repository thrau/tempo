//! `TempoApplication` — owns the main window and the app's global actions.

mod imp;

use gtk::prelude::*;

use crate::window::TempoWindow;

glib::wrapper! {
    pub struct TempoApplication(ObjectSubclass<imp::TempoApplication>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl TempoApplication {
    /// Create the application instance.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    /// Create (or focus) the main window and present it.
    fn show_window(&self) {
        // Reuse an existing window if one is already open.
        if let Some(window) = self.active_window() {
            window.present();
            return;
        }
        let window = TempoWindow::new(self);
        window.present();
    }
}

impl Default for TempoApplication {
    fn default() -> Self {
        Self::new()
    }
}
