//! A monotonic stopwatch and its GTK panel.
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

/// Session-only stopwatch state.  `Instant` is deliberately used instead of
/// wall-clock time, so changing the system clock cannot affect a measurement.
#[derive(Debug)]
pub struct Stopwatch {
    elapsed: Duration,
    started_at: Option<Instant>,
    laps: Vec<Duration>,
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            started_at: None,
            laps: Vec::new(),
        }
    }

    pub fn running(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        self.started_at.map_or(self.elapsed, |start| {
            self.elapsed + now.saturating_duration_since(start)
        })
    }

    pub fn start(&mut self, now: Instant) {
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }

    pub fn pause(&mut self, now: Instant) {
        if self.started_at.is_some() {
            self.elapsed = self.elapsed(now);
            self.started_at = None;
        }
    }

    /// Clear the elapsed time and all recorded laps.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.started_at = None;
        self.laps.clear();
    }

    /// Record a lap as the elapsed time at the moment it was taken.
    pub fn lap(&mut self, now: Instant) -> Duration {
        let elapsed = self.elapsed(now);
        self.laps.push(elapsed);
        elapsed
    }

    pub fn laps(&self) -> &[Duration] {
        &self.laps
    }
}

fn format_elapsed(duration: Duration) -> String {
    let minutes = duration.as_secs() / 60;
    let seconds = duration.as_secs() % 60;
    let tenths = duration.subsec_millis() / 100;
    if minutes >= 60 {
        format!(
            "{:02}:{:02}:{:02}.{}",
            minutes / 60,
            minutes % 60,
            seconds,
            tenths
        )
    } else {
        format!("{:02}:{:02}.{}", minutes, seconds, tenths)
    }
}

// Use relative Pango sizing so the fractional part follows the display font
// while remaining smaller and aligned with the main digits' baseline.
fn display_markup(duration: Duration) -> String {
    let formatted = format_elapsed(duration);
    let (whole, fraction) = formatted.split_once('.').unwrap();
    format!("{whole}<span size=\"smaller\">.{fraction}</span>")
}

/// Build the stopwatch page.  The model is owned by the page, so it continues
/// to run while another page is selected (and, like timers, while the window
/// is hidden).
pub fn build_panel() -> gtk::Widget {
    use adw::prelude::*;
    let model = Rc::new(RefCell::new(Stopwatch::new()));
    let root = gtk::Box::new(gtk::Orientation::Vertical, 20);
    root.set_margin_top(24);
    root.set_margin_bottom(24);
    root.set_margin_start(24);
    root.set_margin_end(24);
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let title = gtk::Label::new(Some("Stopwatch"));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    let subtitle = gtk::Label::new(Some("Measure elapsed time."));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    heading.append(&title);
    heading.append(&subtitle);
    root.append(&heading);

    // Keep the controls in the same boxed surface and clamp the page to the
    // same 620px content width as alarms, timers, and cycles.
    let card = gtk::ListBox::new();
    card.set_selection_mode(gtk::SelectionMode::None);
    card.add_css_class("boxed-list");
    let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
    body.add_css_class("schedule");
    let display = gtk::Label::new(None);
    display.set_markup(&display_markup(Duration::ZERO));
    display.add_css_class("stopwatch-display");
    display.set_xalign(0.5);
    body.append(&display);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    controls.set_halign(gtk::Align::Center);
    let start = gtk::Button::with_label("Start");
    let pause = gtk::Button::with_label("Pause");
    let lap = gtk::Button::with_label("Lap");
    let reset = gtk::Button::with_label("Reset");
    pause.set_sensitive(false);
    lap.set_sensitive(false);
    reset.set_sensitive(false);
    controls.append(&start);
    controls.append(&pause);
    controls.append(&lap);
    controls.append(&reset);
    body.append(&controls);
    let laps = gtk::ListBox::new();
    laps.set_selection_mode(gtk::SelectionMode::None);
    laps.add_css_class("boxed-list");
    // A nested boxed-list casts a second shadow inside the stopwatch card.
    // Keep laps as a sibling surface, hidden until there is a recorded lap.
    laps.set_visible(false);
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_child(Some(&body));
    card.append(&row);
    root.append(&card);
    root.append(&laps);

    let update = {
        let model = model.clone();
        let display = display.clone();
        let start = start.clone();
        let pause = pause.clone();
        let lap = lap.clone();
        let reset = reset.clone();
        move || {
            let model = model.borrow();
            let elapsed = model.elapsed(Instant::now());
            display.set_markup(&display_markup(elapsed));
            start.set_sensitive(!model.running());
            pause.set_sensitive(model.running());
            lap.set_sensitive(model.running());
            reset.set_sensitive(elapsed > Duration::ZERO || !model.laps().is_empty());
        }
    };
    let update_ref = Rc::new(update);
    {
        let model = model.clone();
        let update = update_ref.clone();
        start.connect_clicked(move |_| {
            model.borrow_mut().start(Instant::now());
            update();
        });
    }
    {
        let model = model.clone();
        let update = update_ref.clone();
        pause.connect_clicked(move |_| {
            model.borrow_mut().pause(Instant::now());
            update();
        });
    }
    {
        let model = model.clone();
        let laps = laps.clone();
        let update = update_ref.clone();
        lap.connect_clicked(move |_| {
            let value = model.borrow_mut().lap(Instant::now());
            let row = gtk::Label::new(Some(&format!(
                "Lap {}  ·  {}",
                model.borrow().laps().len(),
                format_elapsed(value)
            )));
            row.set_xalign(0.0);
            row.set_margin_top(8);
            row.set_margin_bottom(8);
            row.set_margin_start(12);
            row.set_margin_end(12);
            laps.append(&row);
            laps.set_visible(true);
            update();
        });
    }
    {
        let model = model.clone();
        let laps = laps.clone();
        let update = update_ref.clone();
        reset.connect_clicked(move |_| {
            model.borrow_mut().reset();
            while let Some(child) = laps.first_child() {
                laps.remove(&child);
            }
            laps.set_visible(false);
            update();
        });
    }
    gtk::glib::timeout_add_local(Duration::from_millis(50), move || {
        update_ref();
        gtk::glib::ControlFlow::Continue
    });
    let clamp = adw::Clamp::builder()
        .maximum_size(620)
        .tightening_threshold(620)
        .child(&root)
        .build();
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build()
        .upcast()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn elapsed_display_uses_one_fractional_digit() {
        for (millis, expected) in [
            (0, "00:00.0"),
            (99, "00:00.0"),
            (123, "00:00.1"),
            (59_999, "00:59.9"),
            (60_000, "01:00.0"),
            (3_600_987, "01:00:00.9"),
        ] {
            assert_eq!(format_elapsed(Duration::from_millis(millis)), expected);
        }
        assert_eq!(
            display_markup(Duration::from_millis(123)),
            "00:00<span size=\"smaller\">.1</span>"
        );
    }

    #[test]
    fn monotonic_pause_resume_and_laps() {
        let base = Instant::now();
        let mut watch = Stopwatch::new();
        watch.start(base);
        assert_eq!(
            watch.elapsed(base + Duration::from_secs(2)),
            Duration::from_secs(2)
        );
        watch.lap(base + Duration::from_secs(2));
        watch.pause(base + Duration::from_secs(3));
        watch.start(base + Duration::from_secs(10));
        assert_eq!(
            watch.elapsed(base + Duration::from_secs(11)),
            Duration::from_secs(4)
        );
        assert_eq!(watch.laps().len(), 1);
    }
}
