//! Native widget smoke test. Skipped when no display server is available.
use gtk::prelude::*;

fn descendants<T: IsA<gtk::Widget> + Clone + 'static>(widget: &impl IsA<gtk::Widget>) -> Vec<T> {
    let widget = widget.as_ref();
    let mut found = Vec::new();
    if let Ok(object) = widget.clone().downcast::<T>() {
        found.push(object);
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        found.extend(descendants::<T>(&widget));
        child = widget.next_sibling();
    }
    found
}
fn click(widget: &impl IsA<gtk::Widget>, label: &str) {
    descendants::<gtk::Button>(widget)
        .into_iter()
        .find(|button| button.label().as_deref() == Some(label))
        .unwrap()
        .emit_clicked();
}
fn icon_click(widget: &impl IsA<gtk::Widget>, tooltip: &str) {
    descendants::<gtk::Button>(widget)
        .into_iter()
        .find(|button| button.tooltip_text().as_deref() == Some(tooltip))
        .unwrap()
        .emit_clicked();
}

#[test]
fn native_window_create_pause_reset_delete() {
    if gtk::init().is_err() {
        eprintln!("Skipping GTK smoke test: no display available");
        return;
    }
    adw::init().unwrap();
    gio::resources_register_include!("tempo.gresource").unwrap();
    let app = crate::application::TempoApplication::new();
    app.set_flags(gio::ApplicationFlags::NON_UNIQUE);
    app.register(gio::Cancellable::NONE).unwrap();
    let window = crate::window::TempoWindow::new(&app);
    let stack = descendants::<gtk::Stack>(&window)
        .into_iter()
        .find(|s| s.child_by_name("alarms").is_some())
        .unwrap();
    // Names are secondary: all time/duration inputs must precede the entry.
    for page in ["alarms", "timers", "cycles"] {
        let panel = stack.child_by_name(page).unwrap();
        let entry = descendants::<gtk::Entry>(&panel).remove(0);
        let form = entry.parent().unwrap();
        assert!(form.has_css_class("composer"));
        let mut child = form.first_child();
        let mut reached_name = false;
        let mut time_inputs = 0;
        while let Some(widget) = child {
            if widget == entry {
                reached_name = true;
            }
            let count = descendants::<gtk::SpinButton>(&widget).len();
            assert!(
                !reached_name || count == 0,
                "{page}: time controls belong above the name"
            );
            time_inputs += count;
            child = widget.next_sibling();
        }
        assert!(time_inputs > 0);
    }
    let alarms = stack.child_by_name("alarms").unwrap();
    click(&alarms, "Add alarm");
    let list = descendants::<gtk::ListBox>(&alarms).remove(0);
    assert!(list.row_at_index(0).is_some());
    let enabled = descendants::<gtk::Switch>(&list).remove(0);
    enabled.set_active(false);
    enabled.set_active(true);
    assert!(enabled.is_active());
    icon_click(&list, "Delete");
    assert!(list.row_at_index(0).is_none());

    for (page, action) in [("timers", "Start timer"), ("cycles", "Start cycle")] {
        let panel = stack.child_by_name(page).unwrap();
        let list = descendants::<gtk::ListBox>(&panel).remove(0);
        let inputs = descendants::<gtk::SpinButton>(&panel);
        for input in &inputs {
            input.set_value(0.0);
        }
        click(&panel, action);
        assert!(list.row_at_index(0).is_none(), "reject zero durations");
        for input in &inputs {
            input.set_value(1.0);
        }
        click(&panel, action);
        assert!(list.row_at_index(0).is_some());
        icon_click(&list, "Pause");
        icon_click(&list, "Start / resume");
        icon_click(&list, "Reset");
        assert!(descendants::<gtk::Button>(&list)
            .iter()
            .any(|b| b.tooltip_text().as_deref() == Some("Start / resume")));
        icon_click(&list, "Delete");
        assert!(list.row_at_index(0).is_none());
    }
    window.destroy();
}
