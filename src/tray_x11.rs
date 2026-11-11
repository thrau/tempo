//! Legacy freedesktop/XEmbed system tray support (MATE, Xfce, older panels).
//! GTK 4 intentionally has no GtkStatusIcon. This isolated X11 connection owns
//! only the tray icon and its popup menu; GTK actions are delivered by channel.
use crate::tray::Action;
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{xproto::*, Event},
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
const ICON_SIZE: u16 = 24;
const MENU_WIDTH: u16 = 160;
const ITEM_HEIGHT: u16 = 32;

pub struct LegacyTray {
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl LegacyTray {
    pub fn new(sender: async_channel::Sender<Action>) -> Self {
        let enabled = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let active = enabled.clone();
        let stopping = stop.clone();
        let worker = thread::spawn(move || {
            // No DISPLAY (e.g. native Wayland): there can be no legacy tray.
            if std::env::var_os("DISPLAY").is_none() {
                return;
            }
            if let Err(error) = run(sender, active, stopping) {
                eprintln!("Tempo: legacy tray unavailable: {error}");
            }
        });
        Self {
            enabled,
            stop,
            worker: Some(worker),
        }
    }
    pub fn switch(&self) -> Arc<AtomicBool> {
        self.enabled.clone()
    }
}
impl Drop for LegacyTray {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct TrayWindow {
    window: Window,
    owner: Window,
    width: u16,
    height: u16,
}
struct XTray {
    connection: RustConnection,
    screen: usize,
    selection: Atom,
    opcode: Atom,
    xembed: Atom,
    icon: Option<TrayWindow>,
    popup: Option<Window>,
    gc: Gcontext,
    font: Font,
    blue: u32,
    white: u32,
    ink: u32,
    hover: u32,
    highlighted: Option<usize>,
    sender: async_channel::Sender<Action>,
}
impl XTray {
    fn new(sender: async_channel::Sender<Action>) -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let atom = |name: &str| -> Result<Atom> {
            Ok(connection
                .intern_atom(false, name.as_bytes())?
                .reply()?
                .atom)
        };
        let selection = atom(&format!("_NET_SYSTEM_TRAY_S{screen}"))?;
        let opcode = atom("_NET_SYSTEM_TRAY_OPCODE")?;
        let xembed = atom("_XEMBED_INFO")?;
        let root = &connection.setup().roots[screen];
        let color = |r, g, b| -> Result<u32> {
            Ok(connection
                .alloc_color(root.default_colormap, r, g, b)?
                .reply()?
                .pixel)
        };
        let blue = color(0x3535, 0x8484, 0xe4e4)?;
        let white = color(0xffff, 0xffff, 0xffff)?;
        let ink = color(0x2424, 0x2424, 0x2424)?;
        let hover = color(0xe5e5, 0xeeee, 0xfafa)?;
        let font = connection.generate_id()?;
        connection.open_font(font, b"fixed")?.check()?;
        let gc = connection.generate_id()?;
        connection
            .create_gc(
                gc,
                root.root,
                &CreateGCAux::new().font(font).graphics_exposures(0),
            )?
            .check()?;
        Ok(Self {
            connection,
            screen,
            selection,
            opcode,
            xembed,
            icon: None,
            popup: None,
            gc,
            font,
            blue,
            white,
            ink,
            hover,
            highlighted: None,
            sender,
        })
    }
    fn reconcile(&mut self, enabled: bool) -> Result<()> {
        let owner = if enabled {
            self.connection
                .get_selection_owner(self.selection)?
                .reply()?
                .owner
        } else {
            0
        };
        if self.icon.as_ref().is_some_and(|icon| icon.owner == owner) {
            return Ok(());
        }
        self.close_menu()?;
        if let Some(icon) = self.icon.take() {
            self.connection.destroy_window(icon.window)?;
        }
        if owner == 0 {
            self.connection.flush()?;
            return Ok(());
        }
        let window = self.connection.generate_id()?;
        let root = &self.connection.setup().roots[self.screen];
        self.connection
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                window,
                root.root,
                0,
                0,
                ICON_SIZE,
                ICON_SIZE,
                0,
                WindowClass::INPUT_OUTPUT,
                x11rb::COPY_FROM_PARENT,
                &CreateWindowAux::new()
                    .background_pixmap(BackPixmap::PARENT_RELATIVE)
                    .event_mask(
                        EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::BUTTON_PRESS,
                    ),
            )?
            .check()?;
        self.connection.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"Tempo tray",
        )?;
        self.connection.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            b"tempo\0Tempo\0",
        )?;
        // GtkSocket-based panels otherwise allocate only a one-pixel width.
        let hints = x11rb::properties::WmSizeHints {
            min_size: Some((ICON_SIZE.into(), ICON_SIZE.into())),
            base_size: Some((ICON_SIZE.into(), ICON_SIZE.into())),
            ..Default::default()
        };
        hints.set_normal_hints(&self.connection, window)?;
        self.connection.change_property32(
            PropMode::REPLACE,
            window,
            self.xembed,
            self.xembed,
            &[0, 1],
        )?;
        // SYSTEM_TRAY_REQUEST_DOCK. The panel reparents/maps the XEmbed window.
        self.connection.send_event(
            false,
            owner,
            EventMask::NO_EVENT,
            ClientMessageEvent::new(
                32,
                owner,
                self.opcode,
                [x11rb::CURRENT_TIME, 0, window, 0, 0],
            ),
        )?;
        self.icon = Some(TrayWindow {
            window,
            owner,
            width: ICON_SIZE,
            height: ICON_SIZE,
        });
        self.connection.flush()?;
        Ok(())
    }
    fn draw_icon(&self) -> Result<()> {
        let Some(icon) = &self.icon else {
            return Ok(());
        };
        let c = &self.connection;
        c.clear_area(false, icon.window, 0, 0, 0, 0)?;
        let size = icon.width.min(icon.height).saturating_sub(4).max(4);
        let x = (icon.width.saturating_sub(size) / 2) as i16;
        let y = (icon.height.saturating_sub(size) / 2) as i16;
        c.change_gc(self.gc, &ChangeGCAux::new().foreground(self.blue))?;
        c.poly_fill_arc(
            icon.window,
            self.gc,
            &[x11rb::protocol::xproto::Arc {
                x,
                y,
                width: size,
                height: size,
                angle1: 0,
                angle2: 360 * 64,
            }],
        )?;
        let inset = (size / 8).max(1);
        c.change_gc(self.gc, &ChangeGCAux::new().foreground(self.white))?;
        c.poly_fill_arc(
            icon.window,
            self.gc,
            &[x11rb::protocol::xproto::Arc {
                x: x + inset as i16,
                y: y + inset as i16,
                width: size - 2 * inset,
                height: size - 2 * inset,
                angle1: 0,
                angle2: 360 * 64,
            }],
        )?;
        let cx = x + (size / 2) as i16;
        let cy = y + (size / 2) as i16;
        c.change_gc(
            self.gc,
            &ChangeGCAux::new()
                .foreground(self.blue)
                .line_width(2)
                .cap_style(CapStyle::ROUND),
        )?;
        c.poly_line(
            CoordMode::ORIGIN,
            icon.window,
            self.gc,
            &[
                Point {
                    x: cx,
                    y: cy - (size / 4) as i16,
                },
                Point { x: cx, y: cy },
                Point {
                    x: cx + (size / 5) as i16,
                    y: cy + (size / 7) as i16,
                },
            ],
        )?;
        Ok(())
    }
    fn show_menu(&mut self, x: i16, y: i16, time: Timestamp) -> Result<()> {
        self.close_menu()?;
        let c = &self.connection;
        let root = &c.setup().roots[self.screen];
        let window = c.generate_id()?;
        let height = ITEM_HEIGHT * 2;
        let (x, y) = menu_position(x, y, root.width_in_pixels, root.height_in_pixels);
        c.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root.root,
            x,
            y,
            MENU_WIDTH,
            height,
            1,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new()
                .override_redirect(1)
                .background_pixel(self.white)
                .border_pixel(self.ink)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::BUTTON_PRESS
                        | EventMask::POINTER_MOTION
                        | EventMask::KEY_PRESS,
                ),
        )?
        .check()?;
        c.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"Tempo tray menu",
        )?;
        c.map_window(window)?;
        self.popup = Some(window);
        // Grabs implement normal popup dismissal on outside clicks/Escape.
        let pointer = c
            .grab_pointer(
                false,
                window,
                EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
                x11rb::NONE,
                x11rb::NONE,
                time,
            )?
            .reply()?;
        let keyboard = c
            .grab_keyboard(false, window, time, GrabMode::ASYNC, GrabMode::ASYNC)?
            .reply()?;
        if pointer.status != GrabStatus::SUCCESS || keyboard.status != GrabStatus::SUCCESS {
            self.close_menu()?;
        } else {
            self.draw_menu()?;
        }
        self.connection.flush()?;
        Ok(())
    }
    fn draw_menu(&self) -> Result<()> {
        let Some(window) = self.popup else {
            return Ok(());
        };
        let c = &self.connection;
        c.clear_area(false, window, 0, 0, 0, 0)?;
        if let Some(index) = self.highlighted {
            c.change_gc(self.gc, &ChangeGCAux::new().foreground(self.hover))?;
            c.poly_fill_rectangle(
                window,
                self.gc,
                &[Rectangle {
                    x: 0,
                    y: index as i16 * ITEM_HEIGHT as i16,
                    width: MENU_WIDTH,
                    height: ITEM_HEIGHT,
                }],
            )?;
        }
        for (index, text) in [b"Show Tempo".as_slice(), b"Quit".as_slice()]
            .into_iter()
            .enumerate()
        {
            let background = if self.highlighted == Some(index) {
                self.hover
            } else {
                self.white
            };
            c.change_gc(
                self.gc,
                &ChangeGCAux::new()
                    .foreground(self.ink)
                    .background(background),
            )?;
            c.image_text8(
                window,
                self.gc,
                12,
                21 + index as i16 * ITEM_HEIGHT as i16,
                text,
            )?;
        }
        Ok(())
    }
    fn close_menu(&mut self) -> Result<()> {
        if let Some(window) = self.popup.take() {
            self.connection.ungrab_pointer(x11rb::CURRENT_TIME)?;
            self.connection.ungrab_keyboard(x11rb::CURRENT_TIME)?;
            self.connection.destroy_window(window)?;
        }
        self.highlighted = None;
        Ok(())
    }
    fn event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Expose(event) if self.popup == Some(event.window) => self.draw_menu()?,
            Event::Expose(event)
                if self
                    .icon
                    .as_ref()
                    .is_some_and(|icon| icon.window == event.window) =>
            {
                self.draw_icon()?
            }
            Event::ConfigureNotify(event) => {
                if let Some(icon) = &mut self.icon {
                    if icon.window == event.window {
                        icon.width = event.width;
                        icon.height = event.height;
                        self.draw_icon()?;
                    }
                }
            }
            Event::DestroyNotify(event) => {
                if self
                    .icon
                    .as_ref()
                    .is_some_and(|icon| icon.window == event.window)
                {
                    self.icon = None;
                    self.close_menu()?;
                }
            }
            Event::ButtonPress(event) if self.popup == Some(event.event) => {
                let action = if event.detail == 1 {
                    menu_action(event.event_x, event.event_y)
                } else {
                    None
                };
                self.close_menu()?;
                if let Some(action) = action {
                    let _ = self.sender.try_send(action);
                }
            }
            Event::ButtonPress(event)
                if self
                    .icon
                    .as_ref()
                    .is_some_and(|icon| icon.window == event.event) =>
            {
                match event.detail {
                    1 | 2 => {
                        let _ = self.sender.try_send(Action::Show);
                    }
                    3 => self.show_menu(event.root_x, event.root_y, event.time)?,
                    _ => {}
                }
            }
            Event::MotionNotify(event) if self.popup == Some(event.event) => {
                self.highlighted = menu_action(event.event_x, event.event_y).map(|a| {
                    if a == Action::Show {
                        0
                    } else {
                        1
                    }
                });
                self.draw_menu()?;
            }
            Event::KeyPress(event) if self.popup == Some(event.event) => self.close_menu()?,
            Event::Error(error) => eprintln!("Tempo: X11 tray protocol error: {error:?}"),
            _ => {}
        }
        self.connection.flush()?;
        Ok(())
    }
}
fn menu_action(x: i16, y: i16) -> Option<Action> {
    if x < 0 || x >= MENU_WIDTH as i16 || y < 0 || y >= (ITEM_HEIGHT * 2) as i16 {
        return None;
    }
    Some(if y < ITEM_HEIGHT as i16 {
        Action::Show
    } else {
        Action::Quit
    })
}
fn menu_position(x: i16, y: i16, width: u16, height: u16) -> (i16, i16) {
    (
        x.max(0).min(width.saturating_sub(MENU_WIDTH + 2) as i16),
        y.max(0)
            .min(height.saturating_sub(ITEM_HEIGHT * 2 + 2) as i16),
    )
}
fn run(
    sender: async_channel::Sender<Action>,
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let mut tray = XTray::new(sender)?;
    let mut last_check = Instant::now() - Duration::from_secs(1);
    let mut was_enabled = false;
    while !stop.load(Ordering::Relaxed) {
        let active = enabled.load(Ordering::Relaxed);
        if active != was_enabled || last_check.elapsed() >= Duration::from_millis(500) {
            tray.reconcile(active)?;
            was_enabled = active;
            last_check = Instant::now();
        }
        while let Some(event) = tray.connection.poll_for_event()? {
            tray.event(event)?;
        }
        thread::sleep(Duration::from_millis(30));
    }
    tray.close_menu()?;
    if let Some(icon) = tray.icon.take() {
        tray.connection.destroy_window(icon.window)?;
    }
    tray.connection.free_gc(tray.gc)?;
    tray.connection.close_font(tray.font)?;
    tray.connection.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xembed_docking_and_cleanup() {
        if std::env::var_os("DISPLAY").is_none() {
            return;
        }
        let (sender, _) = async_channel::unbounded();
        let Ok(mut tray) = XTray::new(sender) else {
            return;
        };
        // A private selection simulates a panel without interfering with the
        // desktop's real tray or displaying any test windows.
        tray.selection = tray
            .connection
            .intern_atom(
                false,
                format!("_TEMPO_TEST_TRAY_{}", std::process::id()).as_bytes(),
            )
            .unwrap()
            .reply()
            .unwrap()
            .atom;
        let owner = tray.connection.generate_id().unwrap();
        let root = tray.connection.setup().roots[tray.screen].root;
        tray.connection
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                owner,
                root,
                0,
                0,
                100,
                30,
                0,
                WindowClass::INPUT_OUTPUT,
                x11rb::COPY_FROM_PARENT,
                &CreateWindowAux::new(),
            )
            .unwrap()
            .check()
            .unwrap();
        tray.connection
            .set_selection_owner(owner, tray.selection, x11rb::CURRENT_TIME)
            .unwrap()
            .check()
            .unwrap();
        tray.reconcile(true).unwrap();
        let icon = tray.icon.as_ref().unwrap().window;
        tray.connection.get_input_focus().unwrap().reply().unwrap();
        let mut docked = false;
        while let Some(event) = tray.connection.poll_for_event().unwrap() {
            if let Event::ClientMessage(event) = event {
                assert_eq!(event.type_, tray.opcode);
                assert_eq!(event.data.as_data32()[1], 0);
                assert_eq!(event.data.as_data32()[2], icon);
                docked = true;
            }
        }
        assert!(docked, "sends SYSTEM_TRAY_REQUEST_DOCK to the panel");
        let property = tray
            .connection
            .get_property(false, icon, tray.xembed, tray.xembed, 0, 2)
            .unwrap()
            .reply()
            .unwrap();
        assert_eq!(property.value32().unwrap().collect::<Vec<_>>(), vec![0, 1]);
        let hints = x11rb::properties::WmSizeHints::get_normal_hints(&tray.connection, icon)
            .unwrap()
            .reply()
            .unwrap()
            .unwrap();
        assert_eq!(hints.min_size, Some((24, 24)));
        tray.connection
            .reparent_window(icon, owner, 0, 0)
            .unwrap()
            .check()
            .unwrap();
        tray.draw_icon().unwrap();
        tray.reconcile(false).unwrap();
        assert!(tray.icon.is_none());
        assert!(tray
            .connection
            .get_window_attributes(icon)
            .unwrap()
            .reply()
            .is_err());
        // Re-enabling after an SNI host disappears docks a fresh icon.
        tray.reconcile(true).unwrap();
        assert_ne!(tray.icon.as_ref().unwrap().window, icon);
        tray.reconcile(false).unwrap();
        tray.connection.destroy_window(owner).unwrap();
        tray.connection.flush().unwrap();
    }

    #[test]
    fn menu_hit_testing_and_screen_edges() {
        assert_eq!(menu_action(10, 10), Some(Action::Show));
        assert_eq!(menu_action(10, 32), Some(Action::Quit));
        assert_eq!(menu_action(-1, 10), None);
        assert_eq!(menu_action(10, 64), None);
        assert_eq!(menu_action(160, 10), None);
        assert_eq!(menu_position(1919, 1079, 1920, 1080), (1758, 1014));
        assert_eq!(menu_position(-5, -5, 1920, 1080), (0, 0));
    }
}
