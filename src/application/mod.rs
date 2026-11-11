//! `TempoApplication` — owns the main window and the app's global actions.

mod imp;

use glib::subclass::prelude::ObjectSubclassIsExt;
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

    pub(crate) fn stop_tray(&self) {
        self.imp().stop_tray();
    }

    /// Create (or focus) the main window and present it.
    fn show_window(&self) {
        // Reuse an existing window if one is already open.
        // Hidden windows may no longer be the active window. Reuse the same
        // window so reopening from the tray never discards its schedules.
        if let Some(window) = self.windows().into_iter().next() {
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
