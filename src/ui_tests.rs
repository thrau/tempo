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
    // Serialize against storage tests that also set the shared env override.
    let _config_lock = crate::storage::test_lock();
    // Point persistence at a throwaway file so the test never touches ~/.config.
    std::env::set_var(
        "TEMPO_CONFIG_FILE",
        std::env::temp_dir().join(format!("tempo-ui-{}.json", std::process::id())),
    );
    let app = crate::application::TempoApplication::new();
    app.set_flags(gio::ApplicationFlags::NON_UNIQUE);
    app.register(gio::Cancellable::NONE).unwrap();
    let window = crate::window::TempoWindow::new(&app);
    let stack = descendants::<gtk::Stack>(&window)
        .into_iter()
        .find(|s| s.child_by_name("alarms").is_some())
        .unwrap();
    // All schedule pages use the same width and composer/list alignment, even
    // though their controls have different minimum widths.
    stack.set_transition_type(gtk::StackTransitionType::None);
    window.present();
    for width in [480, 640, 900] {
        window.set_default_size(width, 660);
        let mut expected = None;
        for page in ["alarms", "timers", "cycles"] {
            stack.set_visible_child_name(page);
            let context = glib::MainContext::default();
            let until = std::time::Instant::now() + std::time::Duration::from_millis(150);
            while std::time::Instant::now() < until {
                while context.pending() {
                    context.iteration(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let panel = stack.child_by_name(page).unwrap();
            let clamp = descendants::<adw::Clamp>(&panel).remove(0);
            let entry = descendants::<gtk::Entry>(&panel).remove(0);
            let form = entry.parent().unwrap();
            let list = descendants::<gtk::ListBox>(&panel).remove(0);
            let expander = descendants::<gtk::Expander>(&panel).remove(0);
            assert!(expander.is_expanded());
            assert_eq!(expander.child().unwrap(), form);
            let sizes = (clamp.child().unwrap().width(), form.width(), list.width());
            assert!(sizes.0 > 0 && sizes.1 > 0 && sizes.2 > 0);
            if let Some(expected) = expected {
                assert_eq!(
                    sizes, expected,
                    "{page}: aligned widths at window width {width}"
                );
            } else {
                expected = Some(sizes);
            }
        }
    }
    stack.set_visible_child_name("alarms");
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
    let expander = descendants::<gtk::Expander>(&alarms).remove(0);
    assert!(list.row_at_index(0).is_some());
    assert!(
        !expander.is_expanded(),
        "creating an alarm collapses the form"
    );
    let enabled = descendants::<gtk::Switch>(&list).remove(0);
    enabled.set_active(false);
    enabled.set_active(true);
    assert!(enabled.is_active());
    icon_click(&list, "Delete");
    assert!(list.row_at_index(0).is_none());
    assert!(
        expander.is_expanded(),
        "deleting the last alarm reopens the form"
    );

    for (page, action) in [("timers", "Start timer"), ("cycles", "Start cycle")] {
        let panel = stack.child_by_name(page).unwrap();
        let list = descendants::<gtk::ListBox>(&panel).remove(0);
        let expander = descendants::<gtk::Expander>(&panel).remove(0);
        assert!(
            expander.is_expanded(),
            "empty panels show the creation form"
        );
        let inputs = descendants::<gtk::SpinButton>(&panel);
        for input in &inputs {
            input.set_value(0.0);
        }
        click(&panel, action);
        assert!(list.row_at_index(0).is_none(), "reject zero durations");
        assert!(expander.is_expanded(), "invalid input leaves the form open");
        for input in &inputs {
            input.set_value(1.0);
        }
        click(&panel, action);
        assert!(list.row_at_index(0).is_some());
        assert!(
            !expander.is_expanded(),
            "creating an item collapses the form"
        );
        expander.set_expanded(true);
        click(&panel, action);
        assert!(
            list.row_at_index(1).is_some(),
            "the form can add another item"
        );
        assert!(!expander.is_expanded());
        let second = list.row_at_index(1).unwrap();
        expander.set_expanded(true);
        icon_click(&second, "Delete");
        assert!(
            expander.is_expanded(),
            "deletion preserves a manually opened form"
        );
        expander.set_expanded(false);
        icon_click(&list, "Pause");
        icon_click(&list, "Start / resume");
        icon_click(&list, "Reset");
        assert!(descendants::<gtk::Button>(&list)
            .iter()
            .any(|b| b.tooltip_text().as_deref() == Some("Start / resume")));
        icon_click(&list, "Delete");
        assert!(list.row_at_index(0).is_none());
        assert!(
            expander.is_expanded(),
            "deleting the last item reopens the form"
        );
    }
    // Closing hides the existing window without destroying schedules or
    // stopping the main-loop countdown updates. Reopening must reuse it.
    let timers = stack.child_by_name("timers").unwrap();
    let inputs = descendants::<gtk::SpinButton>(&timers);
    inputs[0].set_value(0.0);
    inputs[1].set_value(0.0);
    inputs[2].set_value(5.0);
    click(&timers, "Start timer");
    let list = descendants::<gtk::ListBox>(&timers).remove(0);
    let original_row = list.row_at_index(0).unwrap();
    window.present();
    assert!(window.property::<bool>("hide-on-close"));
    window.close();
    assert!(!window.is_visible());
    assert_eq!(app.windows().len(), 1);
    let context = glib::MainContext::default();
    let until = std::time::Instant::now() + std::time::Duration::from_millis(1200);
    while std::time::Instant::now() < until {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        descendants::<gtk::Label>(&original_row)
            .iter()
            .any(|label| { label.has_css_class("countdown") && label.text() != "00:05" }),
        "countdowns continue while hidden"
    );
    app.activate_action("show", None);
    assert!(window.is_visible());
    assert_eq!(app.windows().len(), 1);
    assert_eq!(list.row_at_index(0).unwrap(), original_row);

    let file = descendants::<gtk::MenuButton>(&window)
        .into_iter()
        .find(|button| button.label().as_deref() == Some("File"))
        .unwrap();
    assert!(file.menu_model().is_some());
    assert!(app.has_action("quit"));
    app.activate_action("quit", None);
    assert!(app.windows().is_empty());
}
