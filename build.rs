use std::path::PathBuf;

fn main() {
    // Compiles data/resources.gresource.xml into the binary so the UI files,
    // CSS and icons are embedded at build time (no external file lookups at runtime).
    glib_build_tools::compile_resources(
        &["data/resources"],
        "data/resources.gresource.xml",
        "tempo.gresource",
    );

    // Re-run the build script if any of the resource sources (GtkBuilder UI files,
    // CSS, etc.) change.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!(
        "cargo:rerun-if-changed={}",
        out_dir.join("tempo.gresource").display()
    );

    // Global "-Wl,--as-needed" is not needed here, but keep a hook for future C deps.
}
