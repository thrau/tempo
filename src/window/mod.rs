//! The main application window.

mod imp;

use gtk::prelude::*;

glib::wrapper! {
    /// The main top-level window wrapping the `TempoWindow` template.
    pub struct TempoWindow(ObjectSubclass<imp::TempoWindow>)
        @extends gtk::Widget, gtk::Window, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl TempoWindow {
    /// Build a new window bound to the given application.
    pub fn new(app: &impl IsA<adw::Application>) -> Self {
        // Loads the "TempoWindow" template from the embedded GResource.
        let obj: Self = glib::Object::builder()
            .property("application", app)
            .build();
        obj
    }
}