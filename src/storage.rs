//! Simple JSON persistence under `~/.config/tempo/`.
//!
//! Stores only the *base configuration* of alarms, timers and cycles — names,
//! durations and schedules. Transient state (whether something is running, how
//! much time remains, a fired alarm's due moment) is intentionally **not**
//! persisted, so a restart always begins with clean, idle items whose
//! countdowns are re-derived from the stored configuration.
//!
//! The file is plain UTF-8 JSON at `$XDG_CONFIG_HOME/tempo/config.json`
//! (defaulting to `~/.config/tempo/config.json`). Each item has a stable,
//! monotonically increasing `id` so `Store` operations can target the right
//! entry even when names/durations collide.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

#[cfg(test)]
static CONFIG_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialize tests that mutate the process-global `TEMPO_CONFIG_FILE` override.
#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlarmConfig {
    pub id: u64,
    pub name: String,
    pub hour: i32,
    pub minute: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerConfig {
    pub id: u64,
    pub name: String,
    pub seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CycleConfig {
    pub id: u64,
    pub name: String,
    pub focus_seconds: u64,
    pub break_seconds: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// Monotonic id counter, persisted so ids are never reused across restarts.
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub alarms: Vec<AlarmConfig>,
    #[serde(default)]
    pub timers: Vec<TimerConfig>,
    #[serde(default)]
    pub cycles: Vec<CycleConfig>,
}

/// Handles loading, mutating and saving the configuration.
#[derive(Clone)]
pub struct Store {
    config: Rc<RefCell<Config>>,
    path: PathBuf,
}

/// Base directory holding the configuration file.
pub fn config_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("tempo")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("tempo")
    } else {
        PathBuf::from(".")
    }
}

/// Full path of the config file. Overridable for tests via `TEMPO_CONFIG_FILE`.
fn config_path() -> PathBuf {
    if let Some(file) = std::env::var_os("TEMPO_CONFIG_FILE") {
        return PathBuf::from(file);
    }
    config_dir().join("config.json")
}

impl Store {
    /// Load the config from disk, starting empty (and inert) if it is missing
    /// or unreadable.
    pub fn load() -> Self {
        let path = config_path();
        let config = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Config>(&raw).ok())
            .unwrap_or_default();
        Store {
            config: Rc::new(RefCell::new(config)),
            path,
        }
    }

    /// Shared access to the underlying configuration for read-only rendering.
    pub fn config(&self) -> Rc<RefCell<Config>> {
        self.config.clone()
    }

    /// Reserves a fresh, unique id and records it in the persisted counter.
    fn next_id(&self) -> u64 {
        let mut config = self.config.borrow_mut();
        config.next_id += 1;
        config.next_id
    }

    /// Write the current configuration to disk, creating the directory if needed.
    pub fn save(&self) {
        let config = self.config.borrow();
        let Ok(raw) = serde_json::to_string_pretty(&*config) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&self.path, raw);
    }

    pub fn add_alarm(&self, mut alarm: AlarmConfig) -> AlarmConfig {
        alarm.id = self.next_id();
        self.config.borrow_mut().alarms.push(alarm.clone());
        self.save();
        alarm
    }

    pub fn remove_alarm(&self, id: u64) {
        self.config.borrow_mut().alarms.retain(|a| a.id != id);
        self.save();
    }

    pub fn set_alarm_enabled(&self, id: u64, enabled: bool) {
        let mut config = self.config.borrow_mut();
        if let Some(alarm) = config.alarms.iter_mut().find(|a| a.id == id) {
            alarm.enabled = enabled;
        }
        drop(config);
        self.save();
    }

    pub fn add_timer(&self, mut timer: TimerConfig) -> TimerConfig {
        timer.id = self.next_id();
        self.config.borrow_mut().timers.push(timer.clone());
        self.save();
        timer
    }

    pub fn remove_timer(&self, id: u64) {
        self.config.borrow_mut().timers.retain(|t| t.id != id);
        self.save();
    }

    pub fn add_cycle(&self, mut cycle: CycleConfig) -> CycleConfig {
        cycle.id = self.next_id();
        self.config.borrow_mut().cycles.push(cycle.clone());
        self.save();
        cycle
    }

    pub fn remove_cycle(&self, id: u64) {
        self.config.borrow_mut().cycles.retain(|c| c.id != id);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_disk() {
        let _guard = test_lock();
        let dir = std::env::temp_dir().join(format!("tempo-test-{}", std::process::id()));
        let file = dir.join("config.json");
        std::env::set_var("TEMPO_CONFIG_FILE", &file);

        let store = Store::load();
        assert!(
            store.config().borrow().alarms.is_empty(),
            "starts empty in a clean location"
        );
        let alarm = store.add_alarm(AlarmConfig {
            id: 0,
            name: "Wake up".into(),
            hour: 7,
            minute: 0,
            enabled: true,
        });
        let timer = store.add_timer(TimerConfig {
            id: 0,
            name: "Coffee".into(),
            seconds: 300,
        });
        let cycle = store.add_cycle(CycleConfig {
            id: 0,
            name: "Pomodoro".into(),
            focus_seconds: 1500,
            break_seconds: 300,
        });

        let reloaded = Store::load();
        let config = reloaded.config();
        assert_eq!(config.borrow().alarms, vec![alarm.clone()]);
        assert_eq!(config.borrow().timers, vec![timer.clone()]);
        assert_eq!(config.borrow().cycles, vec![cycle.clone()]);

        reloaded.set_alarm_enabled(alarm.id, false);
        let after = Store::load();
        assert!(!after.config().borrow().alarms[0].enabled);

        after.remove_timer(timer.id);
        assert!(Store::load().config().borrow().timers.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_loads_default() {
        let _guard = test_lock();
        let dir = std::env::temp_dir().join(format!("tempo-nofile-{}", std::process::id()));
        let file = dir.join("config.json");
        std::env::set_var("TEMPO_CONFIG_FILE", &file);
        let store = Store::load();
        assert!(store.config().borrow().alarms.is_empty());
        // A mutation triggers a save that creates the directory and file.
        store.add_alarm(AlarmConfig {
            id: 0,
            name: "x".into(),
            hour: 1,
            minute: 2,
            enabled: true,
        });
        assert!(file.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
