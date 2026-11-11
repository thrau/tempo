use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/example/Tempo/window.ui")]
pub struct TempoWindow {
    #[template_child]
    pub overlay: TemplateChild<adw::ToastOverlay>,
    #[template_child]
    pub pages: TemplateChild<gtk::Stack>,
}

#[glib::object_subclass]
impl ObjectSubclass for TempoWindow {
    const NAME: &'static str = "TempoWindow";
    type Type = super::TempoWindow;
    type ParentType = adw::ApplicationWindow;
    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for TempoWindow {}
impl WidgetImpl for TempoWindow {}
impl WindowImpl for TempoWindow {}
impl ApplicationWindowImpl for TempoWindow {}
impl AdwApplicationWindowImpl for TempoWindow {}
