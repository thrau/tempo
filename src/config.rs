//! Compile-time application-wide constants (resource paths, application id, ...).
//! These are reserved for use during implementation.
#![allow(dead_code)]

/// Reverse-DNS application id used by GApplication and for the desktop file.
/// Change the domain to your own (e.g. `dev.yourname.Tempo`).
pub const APP_ID: &str = "org.example.Tempo";

/// Human-readable application name shown in the UI.
pub const APP_NAME: &str = "Tempo";

/// Base path of the embedded GResource inside the binary.
pub const RESOURCE_BASE_PATH: &str = "/org/example/Tempo";
