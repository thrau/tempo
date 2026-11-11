//! `GtkApplication`/`AdwApplication` lifecycle handling.

use adw::subclass::prelude::*;
use gtk::prelude::*;

use crate::config;

/// The `AdwApplication` subclass that wires up GTK/Adwaita, actions and windows.
#[derive(Default)]
pub struct TempoApplication;

#[glib::object_subclass]
impl ObjectSubclass for TempoApplication {
    const NAME: &'static str = "TempoApplication";
    type Type = super::TempoApplication;
    type ParentType = adw::Application;
}

impl ObjectImpl for TempoApplication {
    fn constructed(&self) {
        self.parent_constructed();
        let app = self.obj();
        app.set_application_id(Some(config::APP_ID));
        app.set_resource_base_path(Some(config::RESOURCE_BASE_PATH));
        let show = gio::SimpleAction::new("show", None);
        let weak = app.downgrade();
        show.connect_activate(move |_, _| {
            if let Some(app) = weak.upgrade() {
                app.show_window();
            }
        });
        app.add_action(&show);
    }
}

impl ApplicationImpl for TempoApplication {
    fn activate(&self) {
        self.obj().show_window();
    }
}

impl GtkApplicationImpl for TempoApplication {}
impl AdwApplicationImpl for TempoApplication {}
