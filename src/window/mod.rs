//! The main application window.

mod imp;

use glib::subclass::prelude::ObjectSubclassIsExt;
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
        let obj: Self = glib::Object::builder().property("application", app).build();
        let imp = obj.imp();
        let notifier = crate::notifications::Notifier::new(app.as_ref(), &imp.overlay);
        imp.pages.add_titled(
            &crate::panels::alarms(notifier.clone()),
            Some("alarms"),
            "Alarms",
        );
        imp.pages.add_titled(
            &crate::panels::timers(notifier.clone(), false),
            Some("timers"),
            "Timers",
        );
        imp.pages.add_titled(
            &crate::panels::timers(notifier, true),
            Some("cycles"),
            "Cycles",
        );
        obj
    }
}
