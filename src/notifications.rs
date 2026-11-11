//! Desktop + persistent row feedback + in-app toast, with a bundled audible chime.
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
pub struct Notifier {
    app: glib::WeakRef<adw::Application>,
    overlay: glib::WeakRef<adw::ToastOverlay>,
    sound: gtk::MediaFile,
    serial: Rc<Cell<u64>>,
}
impl Notifier {
    pub fn new(app: &adw::Application, overlay: &adw::ToastOverlay) -> Self {
        let sound = gtk::MediaFile::for_resource("/org/example/Tempo/chime.wav");
        let weak = overlay.downgrade();
        sound.connect_error_notify(move |_| {
            if let Some(overlay) = weak.upgrade() {
                overlay.error_bell();
            }
        });
        Self {
            app: app.downgrade(),
            overlay: overlay.downgrade(),
            sound,
            serial: Rc::new(Cell::new(0)),
        }
    }
    pub fn message(&self, text: &str) {
        if let Some(overlay) = self.overlay.upgrade() {
            let toast = adw::Toast::new(text);
            toast.set_timeout(8);
            overlay.add_toast(toast);
        }
    }
    fn play_fallback(&self) {
        let args = [
            "paplay",
            "--raw",
            "--format=s16le",
            "--rate=22050",
            "--channels=1",
            "--client-name=Tempo",
        ];
        let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        let weak = self.overlay.clone();
        match gio::Subprocess::newv(
            &args,
            gio::SubprocessFlags::STDIN_PIPE | gio::SubprocessFlags::STDERR_SILENCE,
        ) {
            Ok(process) => {
                let pcm =
                    glib::Bytes::from_static(&include_bytes!("../data/resources/chime.wav")[44..]);
                process.clone().communicate_async(
                    Some(&pcm),
                    gio::Cancellable::NONE,
                    move |result| {
                        if result.is_err() || !process.is_successful() {
                            if let Some(overlay) = weak.upgrade() {
                                overlay.error_bell();
                            }
                        }
                    },
                );
            }
            Err(_) => {
                if let Some(overlay) = weak.upgrade() {
                    overlay.error_bell();
                }
            }
        }
    }
    pub fn alert(&self, title: &str, body: &str) {
        self.message(&format!("{title} — {body}"));
        if let Some(app) = self.app.upgrade() {
            let notification = gio::Notification::new(title);
            notification.set_body(Some(body));
            notification.set_priority(gio::NotificationPriority::High);
            notification.set_default_action("app.show");
            let id = self.serial.get() + 1;
            self.serial.set(id);
            app.send_notification(Some(&format!("tempo-{id}")), &notification);
        }
        if self.sound.error().is_some() {
            // Some distributions package GTK's media backend separately. PulseAudio's
            // client also works with PipeWire and can play our bundled PCM directly.
            self.play_fallback();
        } else {
            self.sound.pause();
            self.sound.seek(0);
            self.sound.play();
        }
    }
}
