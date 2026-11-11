//! Small native GTK panels; each row owns its session-only schedule.
use crate::{
    alarm::Alarm, cycle::Cycle, notifications::Notifier, storage, timer::Timer,
};
use gtk::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class(class);
    label
}
fn button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .build()
}
fn panel(
    title: &str,
    subtitle: &str,
    empty: &str,
    creation_label: Option<&str>,
    quick: Option<gtk::Widget>,
) -> (gtk::Widget, gtk::Box, gtk::ListBox) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 6);
    heading.append(&label(title, "title-1"));
    let sub = label(subtitle, "dim-label");
    sub.set_wrap(true);
    heading.append(&sub);
    content.append(&heading);
    if let Some(quick) = quick {
        content.append(&quick);
    }
    let form = gtk::Box::new(gtk::Orientation::Vertical, 12);
    form.add_css_class("card");
    form.add_css_class("composer");
    let expander = creation_label.map(|title| {
        let expander = gtk::Expander::builder()
            .label(title)
            .expanded(true)
            .child(&form)
            .build();
        content.append(&expander);
        expander
    });
    if expander.is_none() {
        content.append(&form);
    }
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    let placeholder = label(empty, "dim-label");
    placeholder.set_margin_top(24);
    placeholder.set_margin_bottom(24);
    placeholder.set_xalign(0.5);
    placeholder.set_wrap(true);
    list.set_placeholder(Some(&placeholder));
    content.append(&list);
    if let Some(expander) = expander {
        let weak_expander = expander.downgrade();
        let children = list.observe_children();
        let handler = children.connect_items_changed(move |children, _, _, added| {
            if let Some(expander) = weak_expander.upgrade() {
                let has_rows = children
                    .iter::<glib::Object>()
                    .any(|child| child.is_ok_and(|widget| widget.is::<gtk::ListBoxRow>()));
                if !has_rows {
                    expander.set_expanded(true);
                } else if added > 0 {
                    expander.set_expanded(false);
                }
            }
        });
        // GTK only weakly caches this model. Retain it until the expander is
        // destroyed, then disconnect the observer as part of widget cleanup.
        let handler = RefCell::new(Some(handler));
        expander.connect_destroy(move |_| {
            if let Some(handler) = handler.borrow_mut().take() {
                children.disconnect(handler);
            }
        });
    }
    let clamp = adw::Clamp::builder()
        .maximum_size(620)
        // Keep the clamp independent of each page's minimum content width.
        .tightening_threshold(620)
        .child(&content)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build();
    (scroll.upcast(), form, list)
}
fn name_entry(placeholder: &str) -> gtk::Entry {
    gtk::Entry::builder()
        .placeholder_text(placeholder)
        .max_length(80)
        .build()
}
fn name(entry: &gtk::Entry, fallback: &str) -> String {
    let text = entry.text();
    if text.trim().is_empty() {
        fallback.into()
    } else {
        text.trim().into()
    }
}
fn spin(max: f64, value: f64) -> gtk::SpinButton {
    let spin = gtk::SpinButton::with_range(0.0, max, 1.0);
    spin.set_numeric(true);
    spin.set_value(value);
    spin.set_width_chars(2);
    spin
}
#[derive(Clone)]
struct DurationInput {
    hours: gtk::SpinButton,
    minutes: gtk::SpinButton,
    seconds: gtk::SpinButton,
}
impl DurationInput {
    fn new(form: &gtk::Box, title: &str, minutes: f64) -> Self {
        form.append(&label(title, "heading"));
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let input = Self {
            hours: spin(99.0, 0.0),
            minutes: spin(59.0, minutes),
            seconds: spin(59.0, 0.0),
        };
        for (widget, unit) in [
            (&input.hours, "h"),
            (&input.minutes, "m"),
            (&input.seconds, "s"),
        ] {
            row.append(widget);
            row.append(&label(unit, "dim-label"));
        }
        form.append(&row);
        input
    }
    fn duration(&self) -> Duration {
        // Commit text that has not yet lost focus.
        self.hours.update();
        self.minutes.update();
        self.seconds.update();
        Duration::from_secs(
            (self.hours.value_as_int() * 3600
                + self.minutes.value_as_int() * 60
                + self.seconds.value_as_int()) as u64,
        )
    }
}
fn create_button(form: &gtk::Box, text: &str) -> gtk::Button {
    let button = gtk::Button::with_label(text);
    button.add_css_class("suggested-action");
    button.set_halign(gtk::Align::End);
    form.append(&button);
    button
}
fn row(
    list: &gtk::ListBox,
    title: &str,
    on_delete: Option<Box<dyn Fn() + 'static>>,
) -> (gtk::ListBoxRow, gtk::Box) {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.add_css_class("schedule");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = label(title, "heading");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_hexpand(true);
    header.append(&title);
    let delete = button("user-trash-symbolic", "Delete");
    delete.add_css_class("flat");
    header.append(&delete);
    content.append(&header);
    row.set_child(Some(&content));
    list.append(&row);
    let weak_list = list.downgrade();
    let weak_row = row.downgrade();
    delete.connect_clicked(move |_| {
        if let (Some(list), Some(row)) = (weak_list.upgrade(), weak_row.upgrade()) {
            list.remove(&row);
        }
        if let Some(on_delete) = on_delete.as_ref() {
            on_delete();
        }
    });
    (row, content)
}

pub fn alarms(notifier: Notifier, store: storage::Store) -> gtk::Widget {
    let (panel, form, list) = panel(
        "Alarms",
        "A gentle reminder, right on time.",
        "No alarms yet. Make time for something.",
        Some("New alarm"),
        None,
    );
    let entry = name_entry("Name (optional)");
    form.append(&label("Time", "heading"));
    let time = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let now = glib::DateTime::now_local().ok();
    let hour = spin(23.0, now.as_ref().map_or(9.0, |n| n.hour() as f64));
    let minute = spin(59.0, 0.0);
    time.append(&hour);
    time.append(&label(":", "heading"));
    time.append(&minute);
    time.append(&label("24-hour", "dim-label"));
    form.append(&time);
    form.append(&entry);
    let add = create_button(&form, "Add alarm");
    let n = notifier.clone();
    let s = store.clone();
    let populate_list = list.clone();
    add.connect_clicked(move |_| {
        hour.update();
        minute.update();
        if glib::DateTime::now_local()
            .and_then(|now| Alarm::new(hour.value_as_int(), minute.value_as_int(), &now))
            .is_err()
        {
            n.message("Unable to schedule that local time.");
            return;
        }
        let config = s.add_alarm(storage::AlarmConfig {
            id: 0,
            name: name(&entry, "Alarm"),
            hour: hour.value_as_int(),
            minute: minute.value_as_int(),
            enabled: true,
        });
        add_alarm_row(&list, &config, n.clone(), s.clone());
        entry.set_text("");
    });
    // Restore persisted alarms.
    for config in store.config().borrow().alarms.clone() {
        add_alarm_row(&populate_list, &config, notifier.clone(), store.clone());
    }
    panel
}

/// Build one alarm row and keep it in sync with the store.
fn add_alarm_row(
    list: &gtk::ListBox,
    config: &storage::AlarmConfig,
    notifier: Notifier,
    store: storage::Store,
) {
    let Some(mut alarm) = glib::DateTime::now_local()
        .and_then(|now| Alarm::new(config.hour, config.minute, &now))
        .ok()
    else {
        return;
    };
    alarm.enabled = config.enabled;
    let title = config.name.clone();
    let id = config.id;
    let delete_store = store.clone();
    let (row, content) = row(
        list,
        &title,
        Some(Box::new(move || delete_store.remove_alarm(id))),
    );
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let clock = label(
        &format!("{:02}:{:02}", alarm.hour, alarm.minute),
        "countdown",
    );
    clock.set_hexpand(true);
    line.append(&clock);
    let enabled = gtk::Switch::builder()
        .active(config.enabled)
        .valign(gtk::Align::Center)
        .tooltip_text("Enable alarm")
        .build();
    line.append(&enabled);
    content.append(&line);
    let status = label(&alarm_status(&alarm), "dim-label");
    content.append(&status);
    let model = Rc::new(RefCell::new(alarm));
    let s = status.clone();
    let m = model.clone();
    let n = notifier.clone();
    let toggle_store = store.clone();
    let weak_row = row.downgrade();
    enabled.connect_active_notify(move |switch| {
        let mut model = m.borrow_mut();
        if switch.is_active() {
            if glib::DateTime::now_local()
                .and_then(|now| model.enable(&now))
                .is_err()
            {
                // Defer the switch update to avoid re-entering this borrow.
                let weak = switch.downgrade();
                glib::idle_add_local_once(move || {
                    if let Some(s) = weak.upgrade() {
                        s.set_active(false);
                    }
                });
                n.message("Unable to schedule that local time.");
            } else {
                if let Some(row) = weak_row.upgrade() {
                    row.remove_css_class("triggered");
                }
                s.set_text(&alarm_status(&model));
                toggle_store.set_alarm_enabled(id, true);
            }
        } else {
            model.enabled = false;
            s.set_text("Off · enable to schedule again");
            toggle_store.set_alarm_enabled(id, false);
        }
    });
    let weak_row = row.downgrade();
    let n = notifier.clone();
    let fire_store = store.clone();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        let Some(row) = weak_row.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let fired =
            glib::DateTime::now_local().is_ok_and(|now| model.borrow_mut().tick(now.to_unix()));
        if fired {
            enabled.set_active(false);
            status.set_text("Alarm rang · enable to schedule again");
            row.add_css_class("triggered");
            fire_store.set_alarm_enabled(id, false);
            n.alert(&title, "It's time.");
        }
        glib::ControlFlow::Continue
    });
}
fn alarm_status(alarm: &Alarm) -> String {
    glib::DateTime::from_unix_local(alarm.due)
        .and_then(|date| date.format("Scheduled · %a, %b %e at %H:%M"))
        .map_or_else(|_| "Scheduled".into(), |s| s.to_string())
}

enum Countdown {
    Timer(Timer),
    Cycle(Cycle),
}
impl Countdown {
    fn running(&self) -> bool {
        match self {
            Self::Timer(t) => t.running(),
            Self::Cycle(c) => c.running(),
        }
    }
    fn remaining(&self, now: Instant) -> Duration {
        match self {
            Self::Timer(t) => t.remaining(now),
            Self::Cycle(c) => c.remaining(now),
        }
    }
    fn start(&mut self, now: Instant) {
        match self {
            Self::Timer(t) => t.start(now),
            Self::Cycle(c) => c.start(now),
        }
    }
    fn pause(&mut self, now: Instant) {
        match self {
            Self::Timer(t) => t.pause(now),
            Self::Cycle(c) => c.pause(now),
        }
    }
    fn reset(&mut self) {
        match self {
            Self::Timer(t) => t.reset(),
            Self::Cycle(c) => c.reset(),
        }
    }
    fn tick(&mut self, now: Instant) -> Option<String> {
        match self {
            Self::Timer(t) => t.tick(now).then(|| "Timer complete.".into()),
            Self::Cycle(c) => c
                .tick(now)
                .map(|phase| format!("{} time. The cycle continues.", phase.label())),
        }
    }
    fn status(&self) -> String {
        let state = if self.running() { "Running" } else { "Paused" };
        match self {
            Self::Timer(t) if t.finished => "Complete · ready to restart".into(),
            Self::Timer(t) => format!("{state} · {} total", format_duration(t.duration)),
            Self::Cycle(c) => format!(
                "{} · {state} · {} focus / {} break",
                c.phase.label(),
                format_duration(c.focus),
                format_duration(c.break_time)
            ),
        }
    }
}
fn format_duration(duration: Duration) -> String {
    // Round up: a newly started timer should not immediately lose a second.
    let seconds = duration.as_secs() + u64::from(duration.subsec_nanos() > 0);
    if seconds >= 3600 {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}
fn refresh(model: &Countdown, clock: &gtk::Label, status: &gtk::Label, toggle: &gtk::Button) {
    clock.set_text(&format_duration(model.remaining(Instant::now())));
    status.set_text(&model.status());
    toggle.set_icon_name(if model.running() {
        "media-playback-pause-symbolic"
    } else {
        "media-playback-start-symbolic"
    });
    toggle.set_tooltip_text(Some(if model.running() {
        "Pause"
    } else {
        "Start / resume"
    }));
}
fn countdown_row(
    list: &gtk::ListBox,
    title: String,
    mut model: Countdown,
    notifier: Notifier,
    on_delete: Option<Box<dyn Fn() + 'static>>,
) {
    model.start(Instant::now());
    let model = Rc::new(RefCell::new(model));
    let (row, content) = row(list, &title, on_delete);
    let clock = label("", "countdown");
    content.append(&clock);
    let status = label("", "dim-label");
    status.set_wrap(true);
    content.append(&status);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let toggle = button("media-playback-pause-symbolic", "Pause");
    let reset = button("view-refresh-symbolic", "Reset");
    controls.append(&toggle);
    controls.append(&reset);
    content.append(&controls);
    refresh(&model.borrow(), &clock, &status, &toggle);
    let m = model.clone();
    let c = clock.clone();
    let s = status.clone();
    let weak = row.downgrade();
    let n = notifier.clone();
    let name = title.clone();
    toggle.connect_clicked(move |button| {
        let mut model = m.borrow_mut();
        // Process an overdue deadline before pausing so a boundary cannot be lost.
        if let Some(message) = model.tick(Instant::now()) {
            n.alert(&name, &message);
        }
        if model.running() {
            model.pause(Instant::now());
        } else {
            model.start(Instant::now());
        }
        if let Some(row) = weak.upgrade() {
            row.remove_css_class("triggered");
        }
        refresh(&model, &c, &s, button);
    });
    let m = model.clone();
    let c = clock.clone();
    let s = status.clone();
    let t = toggle.clone();
    let weak = row.downgrade();
    reset.connect_clicked(move |_| {
        m.borrow_mut().reset();
        if let Some(row) = weak.upgrade() {
            row.remove_css_class("triggered");
        }
        refresh(&m.borrow(), &c, &s, &t);
    });
    let weak = row.downgrade();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(row) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let mut model = model.borrow_mut();
        if let Some(message) = model.tick(Instant::now()) {
            row.add_css_class("triggered");
            notifier.alert(&title, &message);
        }
        refresh(&model, &clock, &status, &toggle);
        glib::ControlFlow::Continue
    });
}

pub fn timers(notifier: Notifier, store: storage::Store, cycles: bool) -> gtk::Widget {
    // Quick-create buttons (timers only), wired to the list once it exists.
    let mut quick_buttons: Vec<(gtk::Button, u64)> = Vec::new();
    let quick: Option<gtk::Widget> = if cycles {
        None
    } else {
        let row = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .column_spacing(8)
            .row_spacing(8)
            .homogeneous(true)
            .max_children_per_line(6)
            .build();
        for minutes in [3u64, 5, 10, 15, 30, 50] {
            let button = gtk::Button::with_label(&format!("{minutes} min"));
            button.set_tooltip_text(Some(&format!("Start a {minutes} minute timer")));
            row.insert(&button, -1);
            quick_buttons.push((button, minutes));
        }
        Some(row.upcast())
    };
    let (panel, form, list) = if cycles {
        panel(
            "Cycles",
            "Find your rhythm. Focus, rest, repeat.",
            "Your next focus session starts here.",
            Some("New cycle"),
            None,
        )
    } else {
        panel(
            "Timers",
            "A little space for the things that take time.",
            "No timers running. Start one above.",
            Some("New timer"),
            quick,
        )
    };
    let entry = name_entry(if cycles {
        "Cycle name (e.g. One Pomodoro)"
    } else {
        "Name (optional)"
    });
    let duration = DurationInput::new(
        &form,
        if cycles { "Focus" } else { "Duration" },
        if cycles { 25.0 } else { 5.0 },
    );
    let break_time = cycles.then(|| DurationInput::new(&form, "Break", 5.0));
    form.append(&entry);
    // Wire quick-create buttons, each starting (and storing) a timer.
    for (button, minutes) in quick_buttons {
        let list = list.clone();
        let entry = entry.clone();
        let n = notifier.clone();
        let store = store.clone();
        button.connect_clicked(move |_| {
            let config = store.add_timer(storage::TimerConfig {
                id: 0,
                name: "Timer".into(),
                seconds: minutes * 60,
            });
            let duration = Duration::from_secs(config.seconds);
            let id = config.id;
            let delete_store = store.clone();
            countdown_row(
                &list,
                config.name,
                Countdown::Timer(Timer::new(duration)),
                n.clone(),
                Some(Box::new(move || delete_store.remove_timer(id))),
            );
            entry.set_text("");
        });
    }
    let add = create_button(&form, if cycles { "Start cycle" } else { "Start timer" });
    let populate_notifier = notifier.clone();
    let populate_store = store.clone();
    let populate_list = list.clone();
    add.connect_clicked(move |_| {
        let duration = duration.duration();
        let rest = break_time.as_ref().map(DurationInput::duration);
        if duration.is_zero() || rest.is_some_and(|d| d.is_zero()) {
            notifier.message("Choose a duration greater than zero for each phase.");
            return;
        }
        let title = name(&entry, if cycles { "One Pomodoro" } else { "Timer" });
        if let Some(rest) = rest {
            let config = store.add_cycle(storage::CycleConfig {
                id: 0,
                name: title,
                focus_seconds: duration.as_secs(),
                break_seconds: rest.as_secs(),
            });
            let id = config.id;
            let delete_store = store.clone();
            countdown_row(
                &list,
                config.name,
                Countdown::Cycle(Cycle::new(
                    Duration::from_secs(config.focus_seconds),
                    Duration::from_secs(config.break_seconds),
                )),
                notifier.clone(),
                Some(Box::new(move || delete_store.remove_cycle(id))),
            );
        } else {
            let config = store.add_timer(storage::TimerConfig {
                id: 0,
                name: title,
                seconds: duration.as_secs(),
            });
            let id = config.id;
            let delete_store = store.clone();
            countdown_row(
                &list,
                config.name,
                Countdown::Timer(Timer::new(duration)),
                notifier.clone(),
                Some(Box::new(move || delete_store.remove_timer(id))),
            );
        }
        entry.set_text("");
    });
    // Restore persisted items (timers or cycles depending on the panel).
    if cycles {
        for cycle in populate_store.config().borrow().cycles.clone() {
            let id = cycle.id;
            let delete_store = populate_store.clone();
            countdown_row(
                &populate_list,
                cycle.name,
                Countdown::Cycle(Cycle::new(
                    Duration::from_secs(cycle.focus_seconds),
                    Duration::from_secs(cycle.break_seconds),
                )),
                populate_notifier.clone(),
                Some(Box::new(move || delete_store.remove_cycle(id))),
            );
        }
    } else {
        for timer in populate_store.config().borrow().timers.clone() {
            let id = timer.id;
            let delete_store = populate_store.clone();
            countdown_row(
                &populate_list,
                timer.name,
                Countdown::Timer(Timer::new(Duration::from_secs(timer.seconds))),
                populate_notifier.clone(),
                Some(Box::new(move || delete_store.remove_timer(id))),
            );
        }
    }
    panel
}
