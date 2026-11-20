//! Dragging files out of Hearth Files into another app's window (FileManager.qml's Drag.Automatic): iced (winit)
//! can't start a drag that leaves its window, so this does it on Wayland itself, on the window's own connection:
//!   * the window's wl_display and wl_surface come from its raw window handle; the display is wrapped with
//!     wayland-backend's client_system (Backend::from_foreign_display), so this talks on the same connection as the
//!     window, with an event queue of its own;
//!   * it binds its own wl_seat, wl_pointer (every wl_pointer of a client gets the window's button events, so it knows
//!     the press's serial) and wl_data_device_manager;
//!   * while Files' own drag (dnd.rs) is on, it's "armed" with what's dragged; when the pointer, still pressed, leaves
//!     the window, it makes a wl_data_source offering text/uri-list (and GNOME's and plain text's forms), starts the
//!     drag with a small icon (wl_data_device.start_drag with the press's serial), and answers the other app's
//!     requests for the data (send), copy or move as the other app chooses.
//! On X11 (XWayland) it isn't started: there, drags stay inside the window (Fedora's Plasma is Wayland).
//!
//! Light: one thread asleep in poll() on the connection (and on an eventfd for Files' messages): at rest it never
//! wakes; it wakes only when the window gets events (it reads them for the window too, as libwayland shares a
//! connection's reading between its readers).

use futures::channel::mpsc as fmpsc;
use iced::window::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_data_device, wl_data_device_manager, wl_data_offer, wl_data_source, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle, WEnum};

/// What Files tells the drag.
#[derive(Debug, Clone)]
pub enum Cmd {
    /// Files' own drag is on: these paths, the window's size (to know when the pointer leaves it), the accent.
    Arm { paths: Vec<String>, w: f64, h: f64, accent: (u8, u8, u8) },
    Disarm,
}

/// What the drag tells Files.
#[derive(Debug, Clone)]
pub enum Out {
    /// The drag left the window: it's the system's now (Files' own drag ends).
    Started(usize),
    /// It ended: "copy", "move", "ask", "none" (dropped) or "cancelled".
    Ended(String),
}

struct Link {
    tx: mpsc::Sender<Cmd>,
    wake: OwnedFd,
}

static LINK: OnceLock<Mutex<Option<Link>>> = OnceLock::new();
static OUT: OnceLock<Mutex<Option<fmpsc::UnboundedSender<Out>>>> = OnceLock::new();

fn link() -> &'static Mutex<Option<Link>> {
    LINK.get_or_init(|| Mutex::new(None))
}
fn out() -> &'static Mutex<Option<fmpsc::UnboundedSender<Out>>> {
    OUT.get_or_init(|| Mutex::new(None))
}
fn emit(o: Out) {
    if let Ok(g) = out().lock() {
        if let Some(t) = g.as_ref() {
            let _ = t.unbounded_send(o);
        }
    }
}

/// The drag's news, for Files (a subscription for as long as the window is open).
pub fn events() -> fmpsc::UnboundedReceiver<Out> {
    let (tx, rx) = fmpsc::unbounded();
    if let Ok(mut g) = out().lock() {
        *g = Some(tx);
    }
    rx
}

/// The window's Wayland display and surface (as numbers, to cross threads), or None (X11, or no handle).
pub fn handles_of<W: HasWindowHandle + HasDisplayHandle + ?Sized>(w: &W) -> Option<(usize, usize)> {
    let wh = w.window_handle().ok()?.as_raw();
    let dh = w.display_handle().ok()?.as_raw();
    match (wh, dh) {
        (RawWindowHandle::Wayland(s), RawDisplayHandle::Wayland(d)) => Some((d.display.as_ptr() as usize, s.surface.as_ptr() as usize)),
        _ => None,
    }
}

static ASKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// True the first time only: the window's handles are asked for once.
pub fn ask_once() -> bool {
    !ASKED.swap(true, std::sync::atomic::Ordering::SeqCst)
}

pub fn running() -> bool {
    link().lock().map(|g| g.is_some()).unwrap_or(false)
}

pub fn send(c: Cmd) {
    if let Ok(g) = link().lock() {
        if let Some(l) = g.as_ref() {
            if l.tx.send(c).is_ok() {
                let one: u64 = 1;
                unsafe {
                    libc::write(l.wake.as_raw_fd(), &one as *const u64 as *const libc::c_void, 8);
                }
            }
        }
    }
}

/// Starts the drag's helper on the window's connection (once).
pub fn start(display: usize, surface: usize) {
    if running() || display == 0 || surface == 0 {
        return;
    }
    let efd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
    if efd < 0 {
        return;
    }
    let wake = unsafe { OwnedFd::from_raw_fd(efd) };
    let Ok(wake2) = wake.try_clone() else { return };
    let (tx, rx) = mpsc::channel();
    if let Ok(mut g) = link().lock() {
        *g = Some(Link { tx, wake });
    }
    let _ = std::thread::Builder::new().name("hearth-files-dragout".into()).stack_size(256 * 1024).spawn(move || {
        if let Err(e) = run(display, surface, rx, wake2) {
            eprintln!("hearth-files: dragging out to other apps is off: {e}");
        }
        if let Ok(mut g) = link().lock() {
            *g = None;
        }
    });
}

// ------------------------------------------------------------------ the helper
struct St {
    surface: wl_surface::WlSurface,
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    mgr: Option<wl_data_device_manager::WlDataDeviceManager>,
    dev: Option<wl_data_device::WlDataDevice>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    /// The press on the window that's still held (its serial).
    pressed: Option<u32>,
    on_window: bool,
    armed: Option<(Vec<String>, f64, f64, (u8, u8, u8))>,
    source: Option<(wl_data_source::WlDataSource, Vec<String>, String)>,
    icon: Option<(wl_surface::WlSurface, wl_buffer::WlBuffer, wl_shm_pool::WlShmPool)>,
    offer: Option<wl_data_offer::WlDataOffer>,
}

fn run(display: usize, surface: usize, rx: mpsc::Receiver<Cmd>, wake: OwnedFd) -> Result<(), String> {
    let backend = unsafe { Backend::from_foreign_display(display as *mut _) };
    let conn = Connection::from_backend(backend);
    let sid = unsafe { ObjectId::from_ptr(wl_surface::WlSurface::interface(), surface as *mut _) }.map_err(|e| format!("{e:?}"))?;
    let surface = wl_surface::WlSurface::from_id(&conn, sid).map_err(|e| format!("{e:?}"))?;
    let mut q = conn.new_event_queue::<St>();
    let qh = q.handle();
    let _reg = conn.display().get_registry(&qh, ());
    let mut st = St { surface, seat: None, pointer: None, mgr: None, dev: None, compositor: None, shm: None, pressed: None, on_window: false, armed: None, source: None, icon: None, offer: None };
    q.roundtrip(&mut st).map_err(|e| e.to_string())?;
    if st.mgr.is_none() || st.seat.is_none() {
        return Err("no data device manager or seat".into());
    }
    loop {
        q.dispatch_pending(&mut st).map_err(|e| e.to_string())?;
        let _ = conn.flush();
        if let Some(guard) = q.prepare_read() {
            let mut fds = [
                libc::pollfd { fd: guard.connection_fd().as_raw_fd(), events: libc::POLLIN, revents: 0 },
                libc::pollfd { fd: wake.as_raw_fd(), events: libc::POLLIN, revents: 0 },
            ];
            let n = unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) };
            if n > 0 && fds[0].revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP) != 0 {
                if let Err(e) = guard.read() {
                    if !matches!(&e, wayland_client::backend::WaylandError::Io(io) if io.kind() == std::io::ErrorKind::WouldBlock) {
                        return Err(e.to_string());
                    }
                }
            } else {
                drop(guard);
            }
            if fds[1].revents & libc::POLLIN != 0 {
                let mut buf = [0u8; 8];
                unsafe {
                    libc::read(wake.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, 8);
                }
            }
        }
        while let Ok(c) = rx.try_recv() {
            match c {
                Cmd::Arm { paths, w, h, accent } => st.armed = Some((paths, w, h, accent)),
                Cmd::Disarm => st.armed = None,
            }
        }
    }
}

fn uri_of(p: &str) -> String {
    let mut s = String::from("file://");
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// The drag's icon: a page (two for several), the second with the accent, below and right of the pointer.
fn icon_pixels(many: bool, accent: (u8, u8, u8)) -> (i32, i32, Vec<u8>) {
    let (w, h) = (64i32, 64i32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: i32, y: i32, c: (u8, u8, u8, u8)| {
        if x >= 0 && y >= 0 && x < w && y < h {
            let i = ((y * w + x) * 4) as usize;
            // ARGB8888, premultiplied, little-endian: B G R A
            let a = c.3 as u32;
            px[i] = ((c.2 as u32 * a) / 255) as u8;
            px[i + 1] = ((c.1 as u32 * a) / 255) as u8;
            px[i + 2] = ((c.0 as u32 * a) / 255) as u8;
            px[i + 3] = c.3;
        }
    };
    let page = |x0: i32, y0: i32, fill: (u8, u8, u8), put: &mut dyn FnMut(i32, i32, (u8, u8, u8, u8))| {
        let (pw, ph, fold) = (30, 38, 9);
        for y in 0..ph {
            for x in 0..pw {
                if x >= pw - fold && y < fold && (x - (pw - fold)) > y {
                    continue;
                }
                let edge = x == 0 || y == ph - 1 || x == pw - 1 || y == 0 || (x >= pw - fold && y < fold && (x - (pw - fold)) == y);
                put(x0 + x, y0 + y, if edge { (90, 96, 110, 255) } else { (fill.0, fill.1, fill.2, 245) });
            }
        }
    };
    if many {
        page(26, 14, (accent.0, accent.1, accent.2), &mut put);
    }
    page(18, 20, (236, 238, 242), &mut put);
    (w, h, px)
}

impl St {
    fn start_drag(&mut self, qh: &QueueHandle<St>, serial: u32) {
        let (Some(mgr), Some(dev)) = (self.mgr.clone(), self.dev.clone()) else { return };
        let Some((paths, _, _, accent)) = self.armed.take() else { return };
        let src = mgr.create_data_source(qh, ());
        for m in ["text/uri-list", "x-special/gnome-copied-files", "text/plain;charset=utf-8", "text/plain"] {
            src.offer(m.to_string());
        }
        if mgr.version() >= 3 {
            src.set_actions(wl_data_device_manager::DndAction::Copy | wl_data_device_manager::DndAction::Move);
        }
        let icon = self.make_icon(qh, paths.len() > 1, accent);
        dev.start_drag(Some(&src), &self.surface, icon.as_ref(), serial);
        if let Some(i) = &icon {
            i.commit();
        }
        let n = paths.len();
        self.source = Some((src, paths, "none".into()));
        emit(Out::Started(n));
    }

    fn make_icon(&mut self, qh: &QueueHandle<St>, many: bool, accent: (u8, u8, u8)) -> Option<wl_surface::WlSurface> {
        let (comp, shm) = (self.compositor.clone()?, self.shm.clone()?);
        if let Some((s, b, p)) = self.icon.take() {
            b.destroy();
            p.destroy();
            s.destroy();
        }
        let (w, h, px) = icon_pixels(many, accent);
        let name = std::ffi::CString::new("hearth-files-drag").ok()?;
        let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
        if fd < 0 {
            return None;
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut f = std::fs::File::from(fd.try_clone().ok()?);
        f.write_all(&px).ok()?;
        let pool = shm.create_pool(fd.as_fd(), px.len() as i32, qh, ());
        let buf = pool.create_buffer(0, w, h, w * 4, wl_shm::Format::Argb8888, qh, ());
        let s = comp.create_surface(qh, ());
        s.attach(Some(&buf), 0, 0);
        s.damage(0, 0, w, h);
        self.icon = Some((s.clone(), buf, pool));
        Some(s)
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for St {
    fn event(st: &mut St, reg: &wl_registry::WlRegistry, ev: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<St>) {
        if let wl_registry::Event::Global { name, interface, version } = ev {
            match interface.as_str() {
                "wl_seat" if st.seat.is_none() => st.seat = Some(reg.bind(name, version.min(5), qh, ())),
                "wl_data_device_manager" => st.mgr = Some(reg.bind(name, version.min(3), qh, ())),
                "wl_compositor" => st.compositor = Some(reg.bind(name, version.min(4), qh, ())),
                "wl_shm" => st.shm = Some(reg.bind(name, 1, qh, ())),
                _ => {}
            }
            if st.dev.is_none() {
                if let (Some(m), Some(s)) = (&st.mgr, &st.seat) {
                    st.dev = Some(m.get_data_device(s, qh, ()));
                }
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for St {
    fn event(st: &mut St, seat: &wl_seat::WlSeat, ev: wl_seat::Event, _: &(), _: &Connection, qh: &QueueHandle<St>) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(c) } = ev {
            if c.contains(wl_seat::Capability::Pointer) && st.pointer.is_none() {
                st.pointer = Some(seat.get_pointer(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for St {
    fn event(st: &mut St, _: &wl_pointer::WlPointer, ev: wl_pointer::Event, _: &(), _: &Connection, qh: &QueueHandle<St>) {
        match ev {
            wl_pointer::Event::Enter { surface, .. } => st.on_window = surface.id() == st.surface.id(),
            wl_pointer::Event::Leave { .. } => {
                st.on_window = false;
                st.pressed = None;
            }
            wl_pointer::Event::Button { serial, button, state, .. } => {
                if button == 0x110 {
                    match state {
                        WEnum::Value(wl_pointer::ButtonState::Pressed) if st.on_window => st.pressed = Some(serial),
                        _ => st.pressed = None,
                    }
                }
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                if let (Some(serial), Some((_, w, h, _))) = (st.pressed, st.armed.as_ref()) {
                    let (w, h) = (*w, *h);
                    if surface_x < 0.0 || surface_y < 0.0 || surface_x > w || surface_y > h {
                        st.pressed = None;
                        st.start_drag(qh, serial);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_data_source::WlDataSource, ()> for St {
    fn event(st: &mut St, src: &wl_data_source::WlDataSource, ev: wl_data_source::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {
        let mine = st.source.as_ref().map(|s| s.0.id() == src.id()).unwrap_or(false);
        match ev {
            wl_data_source::Event::Send { mime_type, fd } => {
                let Some((_, paths, _)) = st.source.as_ref().filter(|_| mine) else { return };
                let uris: Vec<String> = paths.iter().map(|p| uri_of(p)).collect();
                let data = match mime_type.as_str() {
                    "text/uri-list" => format!("{}\r\n", uris.join("\r\n")),
                    "x-special/gnome-copied-files" => format!("copy\n{}", uris.join("\n")),
                    _ => paths.join("\n"),
                };
                // Written on a thread of its own: a slow reader never holds up the window's events.
                let _ = std::thread::Builder::new().stack_size(64 * 1024).spawn(move || {
                    let mut f = std::fs::File::from(fd);
                    let _ = f.write_all(data.as_bytes());
                });
            }
            wl_data_source::Event::Action { dnd_action } => {
                if let (true, Some(s), WEnum::Value(a)) = (mine, st.source.as_mut(), dnd_action) {
                    s.2 = if a.contains(wl_data_device_manager::DndAction::Move) {
                        "move".into()
                    } else if a.contains(wl_data_device_manager::DndAction::Copy) {
                        "copy".into()
                    } else if a.contains(wl_data_device_manager::DndAction::Ask) {
                        "ask".into()
                    } else {
                        "none".into()
                    };
                }
            }
            wl_data_source::Event::DndFinished => {
                if mine {
                    if let Some((s, _, act)) = st.source.take() {
                        s.destroy();
                        emit(Out::Ended(act));
                    }
                }
            }
            wl_data_source::Event::Cancelled => {
                if mine {
                    if let Some((s, _, _)) = st.source.take() {
                        s.destroy();
                    }
                    emit(Out::Ended("cancelled".into()));
                } else {
                    src.destroy();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for St {
    fn event(st: &mut St, _: &wl_data_device::WlDataDevice, ev: wl_data_device::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {
        // This copy of the window's data device only starts drags: what others offer is let go of at once.
        match ev {
            wl_data_device::Event::Selection { id: Some(o) } => o.destroy(),
            wl_data_device::Event::Enter { id: Some(o), .. } => {
                if let Some(old) = st.offer.replace(o) {
                    old.destroy();
                }
            }
            wl_data_device::Event::Leave | wl_data_device::Event::Drop => {
                if let Some(o) = st.offer.take() {
                    o.destroy();
                }
            }
            _ => {}
        }
    }
    event_created_child!(St, wl_data_device::WlDataDevice, [wl_data_device::EVT_DATA_OFFER_OPCODE => (wl_data_offer::WlDataOffer, ())]);
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for St {
    fn event(_: &mut St, _: &wl_data_offer::WlDataOffer, _: wl_data_offer::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}
impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for St {
    fn event(_: &mut St, _: &wl_data_device_manager::WlDataDeviceManager, _: wl_data_device_manager::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}
impl Dispatch<wl_compositor::WlCompositor, ()> for St {
    fn event(_: &mut St, _: &wl_compositor::WlCompositor, _: wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}
impl Dispatch<wl_surface::WlSurface, ()> for St {
    fn event(_: &mut St, _: &wl_surface::WlSurface, _: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}
impl Dispatch<wl_shm::WlShm, ()> for St {
    fn event(_: &mut St, _: &wl_shm::WlShm, _: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}
impl Dispatch<wl_shm_pool::WlShmPool, ()> for St {
    fn event(_: &mut St, _: &wl_shm_pool::WlShmPool, _: wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}
impl Dispatch<wl_buffer::WlBuffer, ()> for St {
    fn event(_: &mut St, _: &wl_buffer::WlBuffer, _: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<St>) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn uris() {
        assert_eq!(super::uri_of("/home/ci/My File #1.txt"), "file:///home/ci/My%20File%20%231.txt");
        let (w, h, px) = super::icon_pixels(true, (10, 20, 30));
        assert_eq!(px.len() as i32, w * h * 4);
    }
}
