//! Small native GTK panels; each row owns its session-only schedule.
use crate::{alarm::Alarm, cycle::Cycle, notifications::Notifier, timer::Timer};
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
fn panel(title: &str, subtitle: &str, empty: &str) -> (gtk::Widget, gtk::Box, gtk::ListBox) {
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
    let form = gtk::Box::new(gtk::Orientation::Vertical, 12);
    form.add_css_class("card");
    form.add_css_class("composer");
    content.append(&form);
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
    let clamp = adw::Clamp::builder()
        .maximum_size(620)
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
fn row(list: &gtk::ListBox, title: &str) -> (gtk::ListBoxRow, gtk::Box) {
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
    });
    (row, content)
}

pub fn alarms(notifier: Notifier) -> gtk::Widget {
    let (panel, form, list) = panel(
        "Alarms",
        "A gentle reminder, right on time.",
        "No alarms yet. Make time for something.",
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
    add.connect_clicked(move |_| {
        hour.update();
        minute.update();
        let result = glib::DateTime::now_local()
            .and_then(|now| Alarm::new(hour.value_as_int(), minute.value_as_int(), &now));
        let alarm = match result {
            Ok(alarm) => alarm,
            Err(_) => {
                notifier.message("Unable to schedule that local time.");
                return;
            }
        };
        let title = name(&entry, "Alarm");
        let (row, content) = row(&list, &title);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let clock = label(
            &format!("{:02}:{:02}", alarm.hour, alarm.minute),
            "countdown",
        );
        clock.set_hexpand(true);
        line.append(&clock);
        let enabled = gtk::Switch::builder()
            .active(true)
            .valign(gtk::Align::Center)
            .tooltip_text("Enable alarm")
            .build();
        line.append(&enabled);
        content.append(&line);
        let status = label(&alarm_status(&alarm), "dim-label");
        content.append(&status);
        let model = Rc::new(RefCell::new(alarm));
        let m = model.clone();
        let s = status.clone();
        let n = notifier.clone();
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
                }
                if let Some(row) = weak_row.upgrade() {
                    row.remove_css_class("triggered");
                }
                s.set_text(&alarm_status(&model));
            } else {
                model.enabled = false;
                s.set_text("Off · enable to schedule again");
            }
        });
        let weak_row = row.downgrade();
        let n = notifier.clone();
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
                n.alert(&title, "It's time.");
            }
            glib::ControlFlow::Continue
        });
        entry.set_text("");
    });
    panel
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
fn countdown_row(list: &gtk::ListBox, title: String, mut model: Countdown, notifier: Notifier) {
    model.start(Instant::now());
    let model = Rc::new(RefCell::new(model));
    let (row, content) = row(list, &title);
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

pub fn timers(notifier: Notifier, cycles: bool) -> gtk::Widget {
    let (panel, form, list) = if cycles {
        panel(
            "Cycles",
            "Find your rhythm. Focus, rest, repeat.",
            "Your next focus session starts here.",
        )
    } else {
        panel(
            "Timers",
            "A little space for the things that take time.",
            "No timers running. Start one above.",
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
    let add = create_button(&form, if cycles { "Start cycle" } else { "Start timer" });
    add.connect_clicked(move |_| {
        let duration = duration.duration();
        let rest = break_time.as_ref().map(DurationInput::duration);
        if duration.is_zero() || rest.is_some_and(|d| d.is_zero()) {
            notifier.message("Choose a duration greater than zero for each phase.");
            return;
        }
        let title = name(&entry, if cycles { "One Pomodoro" } else { "Timer" });
        let model = match rest {
            Some(rest) => Countdown::Cycle(Cycle::new(duration, rest)),
            None => Countdown::Timer(Timer::new(duration)),
        };
        countdown_row(&list, title, model, notifier.clone());
        entry.set_text("");
    });
    panel
}
