//! Glue object for GtkBuilder template loading of our custom `TempoWindow`.

use gtk::glib;
use gtk::subclass::prelude::*;
use adw::subclass::prelude::*;

/// Object template for the main window defined in `data/resources/window.ui`.
#[derive(Default)]
pub struct TempoWindow;

#[glib::object_subclass]
impl ObjectSubclass for TempoWindow {
    const NAME: &'static str = "TempoWindow";
    type Type = super::TempoWindow;
    type ParentType = adw::ApplicationWindow;
}

impl ObjectImpl for TempoWindow {}
impl WidgetImpl for TempoWindow {}
impl WindowImpl for TempoWindow {}
impl ApplicationWindowImpl for TempoWindow {}
impl AdwApplicationWindowImpl for TempoWindow {}
