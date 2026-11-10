# Meson build for installing app data (icons, desktop file, GSettings schema).
# The Rust binary itself is built by Cargo; Meson handles distribution/packaging.

install_data(
  'org.example.Tempo.desktop',
  install_dir: join_paths(get_option('datadir'), 'applications'),
)

install_data(
  'org.example.Tempo.gschema.xml',
  install_dir: join_paths(get_option('datadir'), 'glib-2.0', 'schemas'),
)

install_data(
  'resources/style.css',
  install_dir: join_paths(get_option('datadir'), 'tempo'),
)

# Icon (scalable SVG). Provide an actual SVG/PNG here before packaging.
install_data(
  'icons/org.example.Tempo.svg',
  install_dir: join_paths(get_option('datadir'), 'icons', 'hicolor', 'scalable', 'apps'),
)
