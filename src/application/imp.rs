//! `GtkApplication`/`AdwApplication` lifecycle handling.

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use adw::subclass::prelude::*;

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
    }
}

impl ApplicationImpl for TempoApplication {
    fn activate(&self) {
        self.obj().show_window();
    }
}

impl GtkApplicationImpl for TempoApplication {}
impl AdwApplicationImpl for TempoApplication {}
