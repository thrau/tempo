project('tempo', 'rust',
  version: '0.1.0',
  meson_version: '>= 0.60.0',
  license: 'GPL-3.0-or-later'
)

# Standard GNOME app build: compile the Rust binary with Cargo, then use Meson
# to install the data files and set up translations. See:
#   https://gitlab.gnome.org/GNOME/gnome-rust-examples

i18n = import('i18n')

cargo = find_program('cargo', required: true)

# ---- Rust binary -----------------------------------------------------------
cargo_release = custom_target(
  'tempo',
  output: 'tempo',
  console: true,
  build_by_default: true,
  command: [
    cargo, 'build', '--release',
    '--manifest-path', meson.project_source_root() / 'Cargo.toml',
    '--target-dir', meson.project_build_root() / 'target',
  ],
  install: true,
  install_dir: get_option('bindir'),
)

subdir('data')

# ---- Translations ----------------------------------------------------------
subdir('po')

# Compile the GSettings schemas at install time.
gnome = import('gnome')
gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
)
