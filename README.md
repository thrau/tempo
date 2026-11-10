# Tempo

A simple timer / alarm / stopwatch app for GNOME, written in Rust with GTK4 and
libadwaita (the same toolkit family GNOME Clocks uses).

> **Status:** project skeleton only — no functionality implemented yet.

## Project layout

```
├── Cargo.toml            # Rust dependencies & build config
├── build.rs              # Compiles UI/CSS into an embedded GResource
├── meson.build           # Meson build for packaging (desktop file, icons, i18n)
├── data/
│   ├── resources.gresource.xml
│   ├── resources/
│   │   ├── window.ui     # Main window (GtkBuilder template, AdwApplicationWindow)
│   │   └── style.css     # App stylesheet
│   ├── org.example.Tempo.desktop
│   ├── org.example.Tempo.gschema.xml   # GSettings preferences schema
│   ├── icons/org.example.Tempo.svg     # Placeholder icon
│   └── meson.build
├── po/                   # Translations (gettext)
│   ├── meson.build
│   └── tempo.pot
└── src/
    ├── main.rs           # Entry point, registers resources & i18n
    ├── config.rs         # App constants (id, name, resource paths)
    ├── application/      # AdwApplication subclass + lifecycle
    ├── window/           # Main window subclass (from window.ui template)
    ├── timer.rs          # Feature skeleton: countdown timer
    ├── alarm.rs          # Feature skeleton: alarms
    └── stopwatch.rs      # Feature skeleton: stopwatch
```

## Prerequisites

Install the GTK4 and libadwaita development libraries (the Rust `gtk4`/`adw`
crates bind to these system libraries at build time):

**Debian / Ubuntu / Mint**
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev \
     libglib2.0-dev-bin      # provides glib-compile-resources
```

**Fedora**
```bash
sudo dnf install gtk4-devel libadwaita-devel glib2-devel
```

**Arch**
```bash
sudo pacman -S gtk4 libadwaita
```

## Build & run

Build and run directly with Cargo:

```bash
cargo run
```

Cargo needs `glib-compile-resources` on the `PATH` (provided by the packages
above) because `build.rs` compiles the GResource.

### Packaging (optional)

For a proper install / Flatpak you'd use Meson and Ninja:

```bash
meson setup _build
meson compile -C _build
meson install -C _build
```

## Translations

The app is wired for gettext (domain `tempo`). To add a language:

```bash
cd po
msginit --locale=fr --input=tempo.pot
```

## Next steps (implementation)

- Add real models (e.g. `glib::Object` subclasses) for timer/alarm/stopwatch.
- Replace the placeholder labels in `src/{timer,alarm,stopwatch}.rs` with real
  panels and wire them into `data/resources/window.ui`.
- Add a GSettings schema with real preference keys.
- Register app actions and keyboard shortcuts.
