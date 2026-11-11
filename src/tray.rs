//! Freedesktop StatusNotifierItem + DBusMenu, usable on X11 and Wayland.
//! Only plain messages cross from the tray service into GTK's main context.
use glib::subclass::prelude::ObjectSubclassIsExt;
use gtk::prelude::*;
use ksni::TrayMethods;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{application::TempoApplication, config};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Show,
    Quit,
}

pub struct TempoTray {
    sender: async_channel::Sender<Action>,
    icons: Vec<ksni::Icon>,
    legacy_enabled: Arc<AtomicBool>,
}
impl TempoTray {
    fn send(&self, action: Action) {
        let _ = self.sender.try_send(action);
    }
}
impl ksni::Tray for TempoTray {
    fn id(&self) -> String {
        config::APP_ID.into()
    }
    fn title(&self) -> String {
        "Tempo".into()
    }
    fn icon_name(&self) -> String {
        if self.icons.is_empty() {
            "alarm-symbolic".into()
        } else {
            String::new()
        }
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Tempo".into(),
            description: "Alarms, timers and cycles are running in the background".into(),
            ..Default::default()
        }
    }
    fn activate(&mut self, _: i32, _: i32) {
        self.send(Action::Show);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{MenuItem, StandardItem};
        vec![
            StandardItem {
                label: "Show Tempo".into(),
                activate: Box::new(|tray: &mut Self| tray.send(Action::Show)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| tray.send(Action::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
    fn watcher_online(&self) {
        self.legacy_enabled.store(false, Ordering::Relaxed);
    }
    fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
        self.legacy_enabled.store(true, Ordering::Relaxed);
        true // Keep watching so we can switch back when an SNI host appears.
    }
}

fn icons() -> Vec<ksni::Icon> {
    let source = std::io::Cursor::new(include_bytes!("../data/icons/org.example.Tempo.svg"));
    let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_read(source) else {
        return vec![];
    };
    let Some(pixbuf) = pixbuf.scale_simple(32, 32, gtk::gdk_pixbuf::InterpType::Bilinear) else {
        return vec![];
    };
    let bytes = pixbuf.read_pixel_bytes();
    let channels = pixbuf.n_channels() as usize;
    let stride = pixbuf.rowstride() as usize;
    let mut argb = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let pixel = &bytes[y * stride + x * channels..];
            argb.extend_from_slice(&[
                if channels == 4 { pixel[3] } else { 255 },
                pixel[0],
                pixel[1],
                pixel[2],
            ]);
        }
    }
    vec![ksni::Icon {
        width: 32,
        height: 32,
        data: argb,
    }]
}

pub fn start(app: &TempoApplication) -> glib::JoinHandle<()> {
    let (sender, receiver) = async_channel::unbounded();
    let legacy = crate::tray_x11::LegacyTray::new(sender.clone());
    let legacy_enabled = legacy.switch();
    app.imp().legacy_tray.replace(Some(legacy));
    let tray = TempoTray {
        sender,
        icons: icons(),
        legacy_enabled: legacy_enabled.clone(),
    };
    let weak = app.downgrade();
    glib::spawn_future_local(async move {
        match tray.assume_sni_available(true).spawn().await {
            Ok(handle) => {
                if let Some(app) = weak.upgrade() {
                    app.imp().tray.replace(Some(handle));
                } else {
                    handle.shutdown();
                    return;
                }
            }
            Err(error) => {
                eprintln!(
                    "Tempo: SNI tray unavailable ({error}); using the X11 tray if available."
                );
                legacy_enabled.store(true, Ordering::Relaxed);
            }
        }
        while let Ok(action) = receiver.recv().await {
            let Some(app) = weak.upgrade() else {
                break;
            };
            match action {
                Action::Show => app.activate_action("show", None),
                Action::Quit => {
                    // Let this receive task finish before Quit tears down its source.
                    let weak = app.downgrade();
                    glib::idle_add_local_once(move || {
                        if let Some(app) = weak.upgrade() {
                            app.activate_action("quit", None);
                        }
                    });
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ksni::Tray;
    #[test]
    fn activation_and_menu_route_actions() {
        let (sender, receiver) = async_channel::unbounded();
        let mut tray = TempoTray {
            sender,
            icons: vec![],
            legacy_enabled: Arc::new(AtomicBool::new(false)),
        };
        tray.watcher_offline(ksni::OfflineReason::No);
        assert!(tray.legacy_enabled.load(Ordering::Relaxed));
        tray.watcher_online();
        assert!(!tray.legacy_enabled.load(Ordering::Relaxed));
        tray.activate(0, 0);
        assert_eq!(receiver.try_recv().unwrap(), Action::Show);
        let menu = tray.menu();
        if let ksni::MenuItem::Standard(item) = &menu[0] {
            (item.activate)(&mut tray);
        } else {
            panic!("missing Show action");
        }
        assert_eq!(receiver.try_recv().unwrap(), Action::Show);
        if let ksni::MenuItem::Standard(item) = &menu[2] {
            (item.activate)(&mut tray);
        } else {
            panic!("missing Quit action");
        }
        assert_eq!(receiver.try_recv().unwrap(), Action::Quit);
    }
    #[test]
    fn bundled_icon_is_valid_argb() {
        let icons = icons();
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].data.len(), 32 * 32 * 4);
    }
}
