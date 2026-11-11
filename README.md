# Tempo

A minimalist desktop clock app for GNOME, written in Rust with GTK4 and
libadwaita. Native controls and automatic light/dark styling keep it at home on
your desktop.

<img width="2188" height="897" alt="tempo" src="https://github.com/user-attachments/assets/74b4502c-a493-416f-abdc-a525e45a31b8" />


## Features

- **Alarms:** choose a local time in 24-hour format and an optional name. Alarms
  fire once at the next occurrence of that time. Toggle an alarm back on to
  schedule its next occurrence; the scheduled date is shown below it.
- **Timers:** create named countdowns with hours, minutes, and seconds. Pause,
  resume, reset, restart completed timers, or delete them independently.
- **Cycles:** name a repeating focus/break routine (for example, “One Pomodoro”
  with 25 minutes of focus and 5 minutes of rest). Each phase transition alerts
  you, and the cycle keeps repeating until paused or deleted. Reset returns it
  to the beginning of the focus phase, paused.
- **Alerts:** a bundled audio chime, a desktop notification, an in-app toast,
  and a highlighted schedule row. Clicking a desktop notification opens Tempo.

Timers and cycles start immediately when created. Durations must be greater
than zero. Countdown deadlines use a monotonic clock rather than subtracting a
second on each UI update. Delayed cycle updates preserve the schedule and
coalesce missed transitions into one alert for the current phase.

**Closing the window hides Tempo to the system tray.** Alarms, timers, and
cycles keep running in the background. Click the tray icon or its **Show Tempo**
menu item to reopen the same window. Launching Tempo again also reopens it.
Use **File → Quit**, **Ctrl+Q**, or the tray's **Quit** menu item to actually exit.
Schedules are session-only: quitting stops and discards them.

Tempo automatically supports both common Linux tray protocols:

- **StatusNotifierItem/AppIndicator** on modern desktops (X11 or Wayland).
- **Legacy X11/XEmbed** when no SNI host is available, including MATE's standard
  notification area. No extra panel applet is needed on those desktops.

Left-click the legacy icon to show Tempo; right-click it for **Show Tempo** and
**Quit**. The legacy icon follows panel resizing and reconnects if the panel
restarts. If an SNI host becomes available, Tempo removes the legacy icon to
avoid duplicates; it falls back again if that host disappears.

GNOME Shell does not provide either tray protocol by default, so it still needs
a tray-support extension. If neither type of tray host exists, Tempo continues
running in the background; launch it again to reopen the window and use
File → Quit to exit.

Tempo does not wake a suspended computer. Desktop notification delivery and
audio depend on your desktop's notification, sound, and Do Not Disturb settings.

## Prerequisites

Install Rust/Cargo, GTK4, libadwaita **1.5 or newer**, and GLib development tools.

**Debian / Ubuntu / Mint** (a release providing libadwaita 1.5+):
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev libglib2.0-dev-bin \
     libgtk-4-media-gstreamer
```

**Fedora:**
```bash
sudo dnf install gtk4-devel libadwaita-devel glib2-devel
```

**Arch:**
```bash
sudo pacman -S gtk4 libadwaita
```

The chime normally plays through GTK's media backend. On systems without that
backend, Tempo falls back to `paplay` (the `pulseaudio-utils` package on Debian /
Ubuntu, also compatible with PipeWire). If neither works, it requests the
system bell. No downloaded sound files are needed.

## Build and run

```bash
cargo run
```

Cargo embeds the window template, stylesheet, and chime into the executable.
`glib-compile-resources` must be on `PATH`.

## Tests

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Tests cover alarm scheduling and one-shot firing, countdown completion and
pause/resume/reset behavior, cycle boundaries and delayed updates, and native
widget creation/control/deletion, hide/reopen/background timing, and explicit
quit behavior. Tray tests cover icon data, Show/Quit action routing, backend
switching, XEmbed docking/cleanup, and popup hit testing. The GTK smoke test
skips itself if no display server is available; the XEmbed protocol test also
requires an X11 display.

## Project layout

- `src/alarm.rs`, `src/timer.rs`, `src/cycle.rs`: scheduling models and unit tests.
- `src/panels.rs`: alarm, timer, and cycle forms and schedule rows.
- `src/notifications.rs`: desktop notifications, toasts, and audio playback.
- `src/tray.rs`: StatusNotifierItem icon, tray menu, and main-thread action routing.
- `src/tray_x11.rs`: legacy XEmbed icon and popup menu fallback.
- `src/window/`, `src/application/`: GTK/Adwaita window and app lifecycle.
- `data/resources/`: window template, styling, and original synthesized chime.
- `src/stopwatch.rs`: reserved skeleton, not currently exposed in the UI.

### Packaging (optional)

```bash
meson setup _build
meson compile -C _build
meson install -C _build
```

### Translations

The app initializes gettext with the `tempo` domain. Translation catalogs live
under `po/`; newly added programmatic UI strings are not yet extracted.
