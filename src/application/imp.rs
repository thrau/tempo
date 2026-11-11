//! `GtkApplication`/`AdwApplication` lifecycle handling.

use adw::subclass::prelude::*;
use gtk::prelude::*;

use crate::config;
use std::cell::RefCell;

/// The `AdwApplication` subclass that wires up GTK/Adwaita, actions and windows.
#[derive(Default)]
pub struct TempoApplication {
    pub tray: RefCell<Option<ksni::Handle<crate::tray::TempoTray>>>,
    pub legacy_tray: RefCell<Option<crate::tray_x11::LegacyTray>>,
    tray_task: RefCell<Option<glib::JoinHandle<()>>>,
}

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
        let quit = gio::SimpleAction::new("quit", None);
        let weak = app.downgrade();
        quit.connect_activate(move |_, _| {
            if let Some(app) = weak.upgrade() {
                app.stop_tray();
                // Destroy rather than close: close intentionally only hides windows.
                for window in app.windows() {
                    window.destroy();
                }
                app.quit();
            }
        });
        app.add_action(&quit);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    }
}

impl TempoApplication {
    pub fn stop_tray(&self) {
        self.legacy_tray.borrow_mut().take();
        if let Some(task) = self.tray_task.borrow_mut().take() {
            task.abort();
        }
        if let Some(tray) = self.tray.borrow_mut().take() {
            tray.shutdown();
        }
    }
}

impl ApplicationImpl for TempoApplication {
    fn startup(&self) {
        self.parent_startup();
        self.tray_task
            .replace(Some(crate::tray::start(&self.obj())));
    }
    fn shutdown(&self) {
        self.stop_tray();
        self.parent_shutdown();
    }
    fn activate(&self) {
        self.obj().show_window();
    }
}

impl GtkApplicationImpl for TempoApplication {}
impl AdwApplicationImpl for TempoApplication {}
