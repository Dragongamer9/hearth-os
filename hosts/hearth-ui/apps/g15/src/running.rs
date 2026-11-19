//! Which games are running, for "Switch key sets for each game" and a screen's "Also show it while … runs", told
//! by the PC itself instead of looking through every program again and again. The service keeps one of these only
//! while there's a rule to follow (service.rs check_games), and it sleeps in the service's poll() with the rest:
//!
//! * **The desktop's windows**: on KWin, hearth-kwin-bridge (Mantel's window list) keeps
//!   $XDG_RUNTIME_DIR/hearth/windows.json up to date on every change (appId, title, pid): watched with inotify. On
//!   labwc (Hearth on Debian), the window list Mantel's taskbar reads itself (Quickshell's ToplevelManager): the
//!   wlr-foreign-toplevel protocol, which tells us when a window opens, is renamed or closes.
//! * **Steam**: Steam keeps the game it's running in ~/.steam/registry.vdf (RunningAppID), and its name in the game's
//!   appmanifest: watched with inotify. That covers Steam's Game Mode, where there's no desktop window list.
//! * **Hearth's own Game Mode**: $XDG_RUNTIME_DIR/hearth-playing while a game runs (hearth-core link.rs), with its
//!   title in Hearth Link's now-playing.json: watched with inotify.
//! * Only when none of the window lists can be had (an X11 desktop, a non-Steam game in Steam's Game Mode): a look
//!   through the running programs every 30 seconds. A kernel process feed (the proc connector) would need root, and
//!   pidfds only tell about programs already known, so this is the last resort, six times slower than before.
//!
//! A rule is what you typed ("Trains 2010"): it matches when each of its words is in a window's title, its app id,
//! the program's name or command line, or the Steam game's name (z and s count as the same, so "Trains" finds
//! "Trainz Simulator 2010"). Nothing here writes a setting: the set a game picks is live, and back when it closes.

use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often the last resort looks through the running programs.
pub const SCAN_EVERY: Duration = Duration::from_secs(30);

// ------------------------------------------------------------------ matching
/// A name's words: lowercase letters and digits, z read as s.
pub fn words(s: &str) -> Vec<String> {
    s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(|w| w.replace('z', "s")).collect()
}

/// Does a rule ("Trains 2010") match one name (a title, an app id, a command line)?
pub fn matches(rule: &str, name: &str) -> bool {
    let r = words(rule);
    if r.is_empty() {
        return false;
    }
    let n = words(name);
    r.iter().all(|w| n.contains(w))
}

pub fn any_match(rule: &str, names: &[String]) -> bool {
    !words(rule).is_empty() && names.iter().any(|n| matches(rule, n))
}

// ------------------------------------------------------------------ the sources, read from files
/// A program's name and its first few command-line arguments, whole (a Wine game's Windows path has the game's
/// folder name in it), from /proc/PID.
pub fn proc_names(proc_: &Path, pid: u64) -> Vec<String> {
    let d = proc_.join(pid.to_string());
    let mut out = Vec::new();
    if let Ok(c) = std::fs::read_to_string(d.join("comm")) {
        out.push(c.trim().to_string());
    }
    if let Ok(cl) = std::fs::read(d.join("cmdline")) {
        for a in cl.split(|&b| b == 0).filter(|a| !a.is_empty()).take(3) {
            out.push(String::from_utf8_lossy(a).to_string());
        }
    }
    out
}

/// The windows in hearth-kwin-bridge's windows.json: their titles and app ids, and their programs' names. A window
/// whose program has ended is left out (the file can outlive KWin). None when there's no such file.
pub fn kwin_names(text: &str, proc_: &Path) -> Option<Vec<String>> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let mut out = Vec::new();
    for w in v.get("windows")?.as_array()? {
        let pid = w.get("pid").and_then(|p| p.as_u64()).unwrap_or(0);
        if pid > 0 && !proc_.join(pid.to_string()).exists() {
            continue;
        }
        for k in ["title", "appId"] {
            if let Some(s) = w.get(k).and_then(|s| s.as_str()).filter(|s| !s.is_empty()) {
                out.push(s.to_string());
            }
        }
        if pid > 0 {
            out.extend(proc_names(proc_, pid));
        }
    }
    Some(out)
}

/// The app id Steam says it's running in registry.vdf (RunningAppID; 0: none). Only that one line is read.
pub fn steam_running(vdf: &str) -> Option<u64> {
    for l in vdf.lines() {
        let l = l.trim();
        if l.len() > 14 && l[..14].eq_ignore_ascii_case("\"RunningAppID\"") {
            let id: u64 = l[14..].trim().trim_matches('"').parse().ok()?;
            return (id > 0).then_some(id);
        }
    }
    None
}

/// A value in a Steam .vdf / .acf line: `"name"   "Trainz"`.
fn vdf_value(l: &str, key: &str) -> Option<String> {
    let l = l.trim();
    let q = format!("\"{key}\"");
    if l.len() <= q.len() || !l[..q.len()].eq_ignore_ascii_case(&q) {
        return None;
    }
    Some(l[q.len()..].trim().trim_matches('"').replace("\\\\", "\\"))
}

/// The game's name from its appmanifest, in Steam's library folders.
pub fn steam_name(home: &Path, id: u64) -> Option<String> {
    let roots = [home.join(".steam/steam"), home.join(".local/share/Steam"), home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")];
    let mut libs: Vec<PathBuf> = Vec::new();
    for r in roots.iter().filter(|r| r.is_dir()) {
        libs.push(r.clone());
        if let Ok(t) = std::fs::read_to_string(r.join("steamapps/libraryfolders.vdf")) {
            libs.extend(t.lines().filter_map(|l| vdf_value(l, "path")).map(PathBuf::from));
        }
    }
    for l in libs {
        if let Ok(t) = std::fs::read_to_string(l.join(format!("steamapps/appmanifest_{id}.acf"))) {
            if let Some(n) = t.lines().find_map(|l| vdf_value(l, "name")) {
                return Some(n);
            }
        }
    }
    None
}

/// Every running program's names (the last resort).
pub fn scan_all(proc_: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(proc_) else { return out };
    for e in rd.filter_map(|e| e.ok()) {
        if let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()) {
            out.extend(proc_names(proc_, pid));
        }
    }
    out
}

// ------------------------------------------------------------------ inotify (no folders are made)
struct Watch {
    fd: RawFd,
}

impl Watch {
    fn new() -> Option<Watch> {
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        (fd >= 0).then_some(Watch { fd })
    }
    fn add(&self, dir: &Path) {
        if let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()) {
            unsafe {
                libc::inotify_add_watch(self.fd, c.as_ptr(), libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_CREATE | libc::IN_DELETE | libc::IN_MOVED_FROM);
            }
        }
    }
    fn names(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            let hdr = std::mem::size_of::<libc::inotify_event>();
            let mut i = 0usize;
            while i + hdr <= n as usize {
                let ev: libc::inotify_event = unsafe { std::ptr::read_unaligned(buf[i..].as_ptr() as *const libc::inotify_event) };
                let name = &buf[i + hdr..(i + hdr + ev.len as usize).min(n as usize)];
                out.push(String::from_utf8_lossy(name).trim_end_matches('\0').to_string());
                i += hdr + ev.len as usize;
            }
        }
        out
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

// ------------------------------------------------------------------ the window list on labwc (wlr-foreign-toplevel)
mod wl {
    use std::collections::HashMap;
    use std::os::unix::io::{AsRawFd, RawFd};
    use wayland_client::backend::WaylandError;
    use wayland_client::protocol::{wl_callback, wl_registry};
    use wayland_client::{event_created_child, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
    use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1};
    use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1};

    #[derive(Default)]
    pub struct State {
        pub windows: HashMap<u32, (String, String)>,
        bound: bool,
        /// The compositor answered the first round: whether it has the window list is known.
        pub answered: bool,
        pub finished: bool,
        pub changed: bool,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(st: &mut Self, reg: &wl_registry::WlRegistry, ev: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
            if let wl_registry::Event::Global { name, interface, version } = ev {
                if interface == ZwlrForeignToplevelManagerV1::interface().name && !st.bound {
                    reg.bind::<ZwlrForeignToplevelManagerV1, _, _>(name, version.min(3), qh, ());
                    st.bound = true;
                }
            }
        }
    }

    impl Dispatch<wl_callback::WlCallback, ()> for State {
        fn event(st: &mut Self, _: &wl_callback::WlCallback, _: wl_callback::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
            st.answered = true;
            st.changed = true;
        }
    }

    impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
        fn event(st: &mut Self, _: &ZwlrForeignToplevelManagerV1, ev: zwlr_foreign_toplevel_manager_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
            match ev {
                zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                    st.windows.insert(toplevel.id().protocol_id(), Default::default());
                }
                zwlr_foreign_toplevel_manager_v1::Event::Finished => st.finished = true,
                _ => {}
            }
        }
        event_created_child!(State, ZwlrForeignToplevelManagerV1, [
            zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
        ]);
    }

    impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
        fn event(st: &mut Self, h: &ZwlrForeignToplevelHandleV1, ev: zwlr_foreign_toplevel_handle_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
            let id = h.id().protocol_id();
            match ev {
                zwlr_foreign_toplevel_handle_v1::Event::Title { title } => st.windows.entry(id).or_default().1 = title,
                zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => st.windows.entry(id).or_default().0 = app_id,
                zwlr_foreign_toplevel_handle_v1::Event::Done => st.changed = true,
                zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                    st.windows.remove(&id);
                    h.destroy();
                    st.changed = true;
                }
                _ => {}
            }
        }
    }

    pub struct Toplevels {
        conn: Connection,
        queue: EventQueue<State>,
        pub st: State,
    }

    impl Toplevels {
        /// Asks the compositor at this socket for its window list (nothing waits for the answer).
        pub fn connect(socket: &std::path::Path) -> Option<Toplevels> {
            let s = std::os::unix::net::UnixStream::connect(socket).ok()?;
            let conn = Connection::from_socket(s).ok()?;
            let queue = conn.new_event_queue::<State>();
            let qh = queue.handle();
            let d = conn.display();
            d.get_registry(&qh, ());
            d.sync(&qh, ());
            conn.flush().ok()?;
            Some(Toplevels { conn, queue, st: State::default() })
        }
        pub fn fd(&self) -> RawFd {
            self.conn.backend().poll_fd().as_raw_fd()
        }
        /// Reads what's arrived. Err: the compositor went away, or it has no window list.
        pub fn pump(&mut self) -> Result<(), ()> {
            if let Some(g) = self.queue.prepare_read() {
                match g.read() {
                    Ok(_) => {}
                    Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => return Err(()),
                }
            }
            self.queue.dispatch_pending(&mut self.st).map_err(|_| ())?;
            let _ = self.conn.flush();
            if self.st.finished || (self.st.answered && !self.st.bound) {
                return Err(());
            }
            Ok(())
        }
        pub fn names(&self) -> Vec<String> {
            self.st.windows.values().flat_map(|(a, t)| [a.clone(), t.clone()]).filter(|s| !s.is_empty()).collect()
        }
    }
}

// ------------------------------------------------------------------ all of it together
pub struct Paths {
    /// $XDG_RUNTIME_DIR
    pub run: PathBuf,
    pub home: PathBuf,
    pub proc_: PathBuf,
    /// Hearth Link's folder (~/.local/share/nimbus/link): now-playing.json.
    pub link: PathBuf,
    /// Ask the compositor for its window list (not for the pretend keyboard).
    pub wayland: bool,
}

impl Paths {
    pub fn real(proc_: &Path, wayland: bool) -> Paths {
        let home = crate::root::home();
        let run = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
        Paths { run, link: data.join("nimbus/link"), home, proc_: proc_.to_path_buf(), wayland }
    }
}

pub struct Running {
    p: Paths,
    watch: Option<Watch>,
    wl: Option<wl::Toplevels>,
    kwin: Option<Vec<String>>,
    steam: Vec<String>,
    hearth: Vec<String>,
    scanned: Vec<String>,
    next_scan: Option<Instant>,
    watching_steam: bool,
}

impl Running {
    pub fn start(p: Paths) -> Running {
        let watch = Watch::new();
        if let Some(w) = &watch {
            w.add(&p.run);
            w.add(&p.run.join("hearth"));
            w.add(&p.link);
        }
        let mut r = Running { p, watch, wl: None, kwin: None, steam: Vec::new(), hearth: Vec::new(), scanned: Vec::new(), next_scan: None, watching_steam: false };
        r.watch_steam();
        r.read_kwin();
        r.read_steam();
        r.read_hearth();
        r.connect_wayland();
        r.plan_scan();
        r
    }

    fn watch_steam(&mut self) {
        let d = self.p.home.join(".steam");
        if !self.watching_steam && d.is_dir() {
            if let Some(w) = &self.watch {
                w.add(&d);
                self.watching_steam = true;
            }
        }
    }

    fn read_kwin(&mut self) {
        self.kwin = std::fs::read_to_string(self.p.run.join("hearth/windows.json")).ok().and_then(|t| kwin_names(&t, &self.p.proc_)).filter(|v| !v.is_empty());
    }

    fn read_steam(&mut self) {
        let vdf = std::fs::read_to_string(self.p.home.join(".steam/registry.vdf")).unwrap_or_default();
        self.steam = match steam_running(&vdf) {
            Some(id) => steam_name(&self.p.home, id).into_iter().collect(),
            None => Vec::new(),
        };
    }

    fn read_hearth(&mut self) {
        self.hearth.clear();
        if !self.p.run.join("hearth-playing").exists() {
            return;
        }
        if let Ok(v) = std::fs::read_to_string(self.p.link.join("now-playing.json")).map(|t| serde_json::from_str::<serde_json::Value>(&t).unwrap_or_default()) {
            if v.get("quitAt").map(|q| q.is_null()).unwrap_or(true) {
                for k in ["title", "game"] {
                    if let Some(s) = v.get(k).and_then(|s| s.as_str()).filter(|s| !s.is_empty()) {
                        self.hearth.push(s.to_string());
                    }
                }
            }
        }
    }

    /// The compositor's socket: $WAYLAND_DISPLAY, else the first wayland-N in the runtime folder (a service started
    /// by udev may not have been given it). Gamescope's own sockets have no window list and aren't tried.
    fn socket(&self) -> Option<PathBuf> {
        if let Some(w) = std::env::var_os("WAYLAND_DISPLAY").filter(|w| !w.is_empty()) {
            let p = PathBuf::from(&w);
            return Some(if p.is_absolute() { p } else { self.p.run.join(p) });
        }
        let mut v: Vec<String> = std::fs::read_dir(&self.p.run).ok()?.filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.starts_with("wayland-") && !n.ends_with(".lock")).collect();
        v.sort();
        v.first().map(|n| self.p.run.join(n))
    }

    fn connect_wayland(&mut self) {
        if !self.p.wayland || self.wl.is_some() || self.kwin.is_some() {
            return;
        }
        if let Some(s) = self.socket() {
            self.wl = wl::Toplevels::connect(&s);
        }
    }

    /// The last resort runs only while no window list can be had.
    fn plan_scan(&mut self) {
        let have_windows = self.kwin.is_some() || self.wl.is_some();
        if have_windows {
            self.next_scan = None;
            self.scanned.clear();
        } else if self.next_scan.is_none() {
            self.scanned = scan_all(&self.p.proc_);
            self.next_scan = Some(Instant::now() + SCAN_EVERY);
        }
    }

    /// The files to poll() for POLLIN, in this order (give the same order back to [`Running::handle`]).
    pub fn fds(&self) -> Vec<RawFd> {
        let mut v = Vec::new();
        if let Some(w) = &self.watch {
            v.push(w.fd);
        }
        if let Some(t) = &self.wl {
            v.push(t.fd());
        }
        v
    }

    /// When the last resort next looks (None: it isn't needed).
    pub fn next_scan(&self) -> Option<Instant> {
        self.next_scan
    }

    /// After poll(): `rev` is the revents for [`Running::fds`], in order. True when what's running may have changed.
    pub fn handle(&mut self, rev: &[i16]) -> bool {
        let mut changed = false;
        let mut i = 0;
        if self.watch.is_some() {
            if rev.get(i).map(|r| r & libc::POLLIN != 0).unwrap_or(false) {
                let names = self.watch.as_ref().map(|w| w.names()).unwrap_or_default();
                if names.iter().any(|n| n == "windows.json") {
                    let had = self.kwin.is_some();
                    self.read_kwin();
                    if self.kwin.is_some() && !had {
                        self.wl = None;
                    }
                    changed = true;
                }
                if names.iter().any(|n| n == "registry.vdf") {
                    self.read_steam();
                    changed = true;
                }
                if names.iter().any(|n| n == ".steam") {
                    self.watch_steam();
                }
                if names.iter().any(|n| n == "hearth-playing" || n == "now-playing.json") {
                    self.read_hearth();
                    changed = true;
                }
                if names.iter().any(|n| n.starts_with("wayland-")) && self.wl.is_none() {
                    self.connect_wayland();
                    changed = true;
                }
            }
            i += 1;
        }
        if self.wl.is_some() {
            let r = rev.get(i).copied().unwrap_or(0);
            if r & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
                let t = self.wl.as_mut().unwrap();
                match t.pump() {
                    Ok(()) => {
                        if std::mem::take(&mut t.st.changed) {
                            changed = true;
                        }
                    }
                    Err(()) => {
                        // The compositor ended (or has no window list): asked again when a new one starts.
                        self.wl = None;
                        changed = true;
                    }
                }
            }
        }
        if let Some(t) = self.next_scan {
            if Instant::now() >= t {
                self.next_scan = None;
                changed = true;
            }
        }
        self.plan_scan();
        changed
    }

    /// The names of what's running now: windows, the Steam game, Hearth's game (or every program, as a last resort).
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        if let Some(k) = &self.kwin {
            v.extend(k.iter().cloned());
        }
        if let Some(t) = &self.wl {
            v.extend(t.names());
        }
        v.extend(self.steam.iter().cloned());
        v.extend(self.hearth.iter().cloned());
        v.extend(self.scanned.iter().cloned());
        v
    }

    /// Which sources are in use, for the journal.
    pub fn sources(&self) -> String {
        let mut s = Vec::new();
        if self.kwin.is_some() {
            s.push("KWin's window list");
        }
        if self.wl.is_some() {
            s.push("the compositor's window list");
        }
        if self.watching_steam {
            s.push("Steam");
        }
        s.push("Hearth's Game Mode");
        if self.next_scan.is_some() {
            s.push("a look at the programs every 30 s");
        }
        s.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_match_what_dad_types() {
        assert!(matches("Trains 2010", "Trainz Simulator 2010: Engineers Edition"));
        assert!(matches("Trains 2010", r"C:\Program Files (x86)\Auran\Trainz Simulator 2010\bin\TRS2010.exe"));
        assert!(matches("Trainz", "trainz.exe"));
        assert!(matches("trainz", "Trainz Simulator 2010"));
        assert!(!matches("Trains 2010", "Trainz Simulator 2009"));
        assert!(!matches("Trains 2010", "Firefox"));
        assert!(!matches("", "anything"), "an empty rule never matches");
        assert!(!matches("  ", "anything"));
        assert!(any_match("Minecraft", &["firefox".into(), "minecraft-launcher".into()]));
        assert!(!any_match("Minecraft", &[]));
    }

    fn fake_proc(d: &Path, pid: u64, comm: &str, args: &[&str]) {
        let p = d.join(pid.to_string());
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("comm"), format!("{comm}\n")).unwrap();
        std::fs::write(p.join("cmdline"), args.join("\0")).unwrap();
    }

    #[test]
    fn kwin_window_list() {
        let d = tempfile::tempdir().unwrap();
        let proc_ = d.path().join("proc");
        fake_proc(&proc_, 4242, "TRS2010.exe", &[r"C:\Games\Trainz Simulator 2010\TRS2010.exe"]);
        let j = r#"{"windows": [{"appId": "trs2010.exe", "title": "", "pid": 4242}, {"appId": "firefox", "title": "News", "pid": 999999}], "active": "1"}"#;
        let names = kwin_names(j, &proc_).unwrap();
        assert!(any_match("Trains 2010", &names));
        assert!(!names.iter().any(|n| n == "firefox"), "a window whose program ended is left out");
        assert!(kwin_names("not json", &proc_).is_none());
    }

    #[test]
    fn steam_says_what_runs() {
        let vdf = "\"Registry\"\n{\n\t\"HKCU\"\n\t{\n\t\t\"Software\"\n\t\t{\n\t\t\t\"Valve\"\n\t\t\t{\n\t\t\t\t\"Steam\"\n\t\t\t\t{\n\t\t\t\t\t\"RunningAppID\"\t\t\"24010\"\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
        assert_eq!(steam_running(vdf), Some(24010));
        assert_eq!(steam_running(&vdf.replace("24010", "0")), None);
        assert_eq!(steam_running(""), None);
        let d = tempfile::tempdir().unwrap();
        let home = d.path();
        let lib2 = home.join("games");
        std::fs::create_dir_all(home.join(".steam/steam/steamapps")).unwrap();
        std::fs::create_dir_all(lib2.join("steamapps")).unwrap();
        std::fs::write(home.join(".steam/steam/steamapps/libraryfolders.vdf"), format!("\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n", lib2.display())).unwrap();
        std::fs::write(lib2.join("steamapps/appmanifest_24010.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"24010\"\n\t\"name\"\t\t\"Train Simulator Classic\"\n}\n").unwrap();
        assert_eq!(steam_name(home, 24010).as_deref(), Some("Train Simulator Classic"));
        assert_eq!(steam_name(home, 1), None);
    }

    #[test]
    fn follows_the_files_without_looking_again() {
        let d = tempfile::tempdir().unwrap();
        let run = d.path().join("run");
        let proc_ = d.path().join("proc");
        std::fs::create_dir_all(run.join("hearth")).unwrap();
        std::fs::create_dir_all(&proc_).unwrap();
        fake_proc(&proc_, 77, "TRS2010.exe", &["TRS2010.exe"]);
        // KWin's list is there: no look through the programs is needed.
        std::fs::write(run.join("hearth/windows.json"), r#"{"windows": [{"appId": "org.kde.dolphin", "title": "Home", "pid": 0}]}"#).unwrap();
        let p = Paths { run: run.clone(), home: d.path().join("home"), proc_: proc_.clone(), link: d.path().join("link"), wayland: false };
        let mut r = Running::start(p);
        assert!(r.next_scan().is_none(), "with a window list nothing is scanned");
        assert!(!any_match("Trains 2010", &r.names()));
        let fds = r.fds();
        assert_eq!(fds.len(), 1);
        // Trainz opens: the bridge writes the list again (atomically).
        let tmp = run.join("hearth/.w.tmp");
        std::fs::write(&tmp, r#"{"windows": [{"appId": "trs2010.exe", "title": "Trainz Simulator 2010", "pid": 77}]}"#).unwrap();
        std::fs::rename(&tmp, run.join("hearth/windows.json")).unwrap();
        let rev = crate::input::poll(&[(fds[0], libc::POLLIN)], 1000);
        assert!(r.handle(&rev));
        assert!(any_match("Trains 2010", &r.names()));
        // Hearth's Game Mode says a game runs (and its title).
        std::fs::create_dir_all(d.path().join("link")).unwrap();
        std::fs::write(d.path().join("link/now-playing.json"), r#"{"game": "steam:24010", "title": "Train Simulator Classic", "at": 1}"#).unwrap();
        std::fs::write(run.join("hearth-playing"), "1\n").unwrap();
        let rev = crate::input::poll(&[(fds[0], libc::POLLIN)], 1000);
        assert!(r.handle(&rev));
        assert!(any_match("Train Simulator", &r.names()));
        std::fs::remove_file(run.join("hearth-playing")).unwrap();
        let rev = crate::input::poll(&[(fds[0], libc::POLLIN)], 1000);
        r.handle(&rev);
        assert!(!any_match("Train Simulator", &r.names()));
    }

    #[test]
    fn without_a_window_list_it_looks_every_30_seconds() {
        let d = tempfile::tempdir().unwrap();
        let proc_ = d.path().join("proc");
        fake_proc(&proc_, 5, "TRS2010.exe", &[r"Z:\Trainz Simulator 2010\TRS2010.exe"]);
        let p = Paths { run: d.path().join("run"), home: d.path().join("home"), proc_, link: d.path().join("link"), wayland: false };
        let r = Running::start(p);
        let next = r.next_scan().expect("the last resort is planned");
        assert!(next > Instant::now() + Duration::from_secs(25));
        assert!(any_match("Trains 2010", &r.names()));
    }
}
