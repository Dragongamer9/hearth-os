//! Hearth Settings as its own app, in pure Rust (hearth-ui, iced): not part of Mantel's shell and not the Game Mode
//! app, so it opens the same on KDE Plasma, Firehold or any desktop. The layout is the approved new Settings' (layout
//! A, docs/mockups/next-update/settings-A1-home-mouse.png and settings-A2-display-mouse.png): you, Search settings and
//! the 13 categories on the left; the page on the right with its path as the title, sections of rows (icon, name,
//! the line under it, its control) and the button hints along the bottom. What's built is listed in
//! docs/mockups/checklists/settings-rust.md.
//!
//! Hearth's own settings are written only through hearth_settings_io.py (atomic, .bak, merged, the change history
//! with Undo: red flag 9); the system's through its own tools (NetworkManager, BlueZ, PipeWire, KScreen,
//! power-profiles-daemon, timedated, Hearth's hearth-admin through pkexec). Opening it writes nothing.
//!
//! `hearth-settings-app [PAGE]` or `--page PAGE` opens at a page (Mantel's page ids: wifi, bluetooth, display, sound,
//! power, storage, appearance, updates, about...).

mod data;
mod pages;

use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key, Modifiers};
use iced::{Element, Subscription, Task};
use serde_json::Value;

/// The 13 categories, in the mockup's order: (id, label, its first page).
pub const CATS: [(&str, &str, &str); 13] = [
    ("home", "Home", "home"),
    ("system", "System", "system"),
    ("net", "Network & internet", "wifi"),
    ("bt", "Bluetooth & devices", "bt"),
    ("link", "Hearth Link", "link"),
    ("pers", "Personalization", "appearance"),
    ("apps", "Apps", "apps-home"),
    ("gaming", "Gaming", "gaming"),
    ("accounts", "Accounts", "accounts"),
    ("hs", "Home server", "hs-overview"),
    ("privacy", "Privacy & security", "privacy"),
    ("a11y", "Accessibility", "accessibility"),
    ("updates", "Updates & recovery", "updates"),
];

/// A page: (id, category, label, the line under it on its category's page, words search finds, built here yet).
pub struct PageDef {
    pub id: &'static str,
    pub cat: &'static str,
    pub label: &'static str,
    pub desc: &'static str,
    pub words: &'static str,
    pub built: bool,
}

const fn pg(id: &'static str, cat: &'static str, label: &'static str, desc: &'static str, words: &'static str, built: bool) -> PageDef {
    PageDef { id, cat, label, desc, words, built }
}

/// Mantel's page registry (settings-pages.js), the same ids so `hearth-settings PAGE` links keep working. `built`:
/// the page is in this app; the others are listed on their category with a plain "not in this app yet".
pub const PAGES: &[PageDef] = &[
    pg("home", "home", "Home", "", "device name rename status recommended start", true),
    pg("system", "system", "System", "", "", true),
    pg("display", "system", "Display", "Brightness, night light, scale, resolution and refresh rate", "screen resolution refresh scale monitor brightness night light", true),
    pg("sound", "system", "Sound", "Volume, speakers and headphones, microphone", "volume audio speakers output microphone", true),
    pg("notifications", "system", "Notifications", "Do not disturb, quiet in full screen and on a schedule, each app", "do not disturb mute alerts", false),
    pg("power", "system", "Power & battery", "Power mode and battery", "battery sleep power mode profile", true),
    pg("memory", "system", "Memory", "Compressed memory (zram), the swap file, what closes when memory runs out", "ram swap zram", false),
    pg("storage", "system", "Storage", "Space on your drives", "disk space drives", true),
    pg("your-desktop", "system", "Your desktop", "KDE Plasma or Firehold: what Desktop Mode opens", "desktop plasma firehold kde window manager", true),
    pg("startup", "system", "Startup", "Start in Game Mode or Desktop Mode", "start in boot mode game mode desktop mode", true),
    pg("datetime", "system", "Date & time", "Time zone, setting the time by itself", "clock time zone ntp", true),
    pg("region", "system", "Region & language", "Language, and how dates, numbers and money are written", "language locale formats", false),
    pg("clipboard", "system", "Clipboard", "Clipboard history", "copy paste history", false),
    pg("about", "system", "About", "Version, this PC's parts and name", "system version hardware name rename", true),
    pg("wifi", "net", "Wi-Fi", "Connect to wireless networks", "wireless network internet", true),
    pg("hs-tailscale", "net", "Tailscale", "Reach your devices from anywhere, privately", "vpn remote", false),
    pg("vpn", "net", "VPN", "WireGuard, OpenVPN and more", "vpn wireguard openvpn", false),
    pg("hotspot", "net", "Mobile hotspot", "Share this PC's internet over Wi-Fi", "hotspot tethering", false),
    pg("proxy", "net", "Proxy", "A proxy for apps and browsers", "proxy", false),
    pg("bt", "bt", "Bluetooth & devices", "", "", true),
    pg("bluetooth", "bt", "Bluetooth", "Headphones, controllers and other wireless devices", "headphones headset speaker pair devices controller", true),
    pg("keyboard", "bt", "Keyboard", "Layouts, typing, the on-screen keyboard, shortcuts", "layout typing shortcuts", false),
    pg("mouse", "bt", "Mouse & touchpad", "Pointer speed, scrolling, tapping", "touchpad pointer", false),
    pg("phone", "bt", "Phone", "Your phone linked with KDE Connect", "kde connect phone", false),
    pg("printers", "bt", "Printers", "Printers and scanners", "print scanner", false),
    pg("link", "link", "Hearth Link", "Your devices together: storage, backups, saves", "phone devices sync pair", false),
    pg("appearance", "pers", "Personalization", "", "look theme accent colour color dark light mode global theme", true),
    pg("bar-dock", "pers", "Bar and dock", "Where the bar goes, the dock, the app menu", "taskbar panel dock", false),
    pg("fonts", "pers", "Fonts, cursor & icons", "The font and its size, the cursor, the icons", "font cursor icons", false),
    pg("apps-home", "apps", "Apps", "Installed apps, default apps, startup apps", "apps installed default", false),
    pg("gaming", "gaming", "Gaming", "Steam, controllers, Game Mode's resolution and frame limit", "steam controller game mode performance", false),
    pg("accounts", "accounts", "Accounts", "Your account, its picture and password, other people", "account user password", false),
    pg("hs-overview", "hs", "Home server", "Game servers, media, backups and files for your other devices", "server", false),
    pg("privacy", "privacy", "Privacy & security", "Lock screen, firewall, what apps may use", "privacy security lock firewall", false),
    pg("accessibility", "a11y", "Accessibility", "Text size, contrast, the on-screen keyboard, reduce motion", "accessibility contrast", false),
    pg("updates", "updates", "Updates & recovery", "", "update upgrade version restart go back restore", true),
];

pub fn page(id: &str) -> Option<&'static PageDef> {
    PAGES.iter().find(|p| p.id == id)
}

pub fn cat_of(id: &str) -> usize {
    let c = page(id).map(|p| p.cat).unwrap_or("home");
    CATS.iter().position(|x| x.0 == c).unwrap_or(0)
}

/// What a read is for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Src {
    Info,
    Wifi,
    Bt,
    Sound,
    Display,
    Power,
    Storage,
    Time,
    Zones,
    Desk,
    Updates,
    History,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Theme,
    Got(Src, run::Output),
    /// A change ran; read this again (and say why if it failed).
    Did(Src, &'static str, run::Output),
    Cat(usize),
    Open(&'static str),
    Back,
    Query(String),
    SearchPick(&'static str),
    // Wi-Fi
    WifiOn(bool),
    WifiScan,
    WifiPick(String),
    WifiPw(String),
    WifiJoin,
    WifiCancel,
    WifiDisconnect(String),
    WifiForget(String),
    // Bluetooth
    BtOn(bool),
    BtScan,
    BtScanned(run::Output),
    BtConnect(String, bool),
    BtPair(String),
    BtForget(String),
    // Sound
    Vol(f32),
    VolSet,
    Mute,
    Mic(f32),
    MicSet,
    MicMute,
    Out(String),
    In(String),
    // Display
    Bright(u32),
    BrightSet,
    Night(bool),
    Res(String, u32, u32),
    Rate(String, String),
    Scale(String, f32),
    Keep,
    Revert,
    Tick,
    // Power, time, desktop, start
    Profile(String),
    Ntp(bool),
    ZoneQuery(String),
    Zone(String),
    Desk(&'static str),
    SwitchNow,
    StartIn(&'static str),
    // Personalization
    Look(String),
    Mode(&'static str),
    Accent(String),
    // Updates
    Check,
    Update,
    Restart,
    GoBack,
    OpenApp(&'static str),
    // Home / About
    Rename,
    RenameText(String),
    RenameSave,
    RenameCancel,
    Undo(i64),
    Key(KEvent),
    Drag,
    Maximize,
    Minimize,
    Close,
    Resize(iced::window::Direction),
}

/// A display change waiting to be kept: the command that puts the old one back, and seconds left.
pub struct Keep {
    pub undo: Vec<String>,
    pub left: u32,
}

pub struct App {
    pub p: Pal,
    pub cat: usize,
    pub page: &'static str,
    pub query: String,
    pub info: data::Info,
    pub wifi: data::Wifi,
    pub wifi_read: bool,
    pub bt: data::Bt,
    pub bt_read: bool,
    pub bt_scanning: bool,
    pub sound: data::Sound,
    pub display: data::Display,
    pub power: data::Power,
    pub storage: Vec<data::Drive>,
    pub time: data::Time,
    pub zone_query: String,
    pub desk: data::Desk,
    pub desk_saved: bool,
    pub start: String,
    pub updates: data::Updates,
    pub history: Vec<data::Change>,
    pub desktop: serde_json::Map<String, Value>,
    pub join: Option<(String, String)>,
    pub keep: Option<Keep>,
    pub rename: Option<String>,
    pub busy: Option<&'static str>,
    pub note: String,
    pub error: String,
}

pub fn io_helper() -> String {
    run::shell_helper("hearth_settings_io.py")
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let first = app::arg("--page").or_else(|| std::env::args().skip(1).find(|a| !a.starts_with('-')));
        let first: &'static str = first.as_deref().and_then(|id| {
            let id = id.split(':').next().unwrap_or(id);
            page(id).map(|p| p.id).or_else(|| PAGES.iter().find(|p| p.words.split(' ').any(|w| w == id)).map(|p| p.id))
        }).unwrap_or("home");
        let mut a = App {
            p: hearth_ui::theme::load(),
            cat: cat_of(first),
            page: first,
            query: String::new(),
            info: data::Info::default(),
            wifi: data::Wifi::default(),
            wifi_read: false,
            bt: data::Bt::default(),
            bt_read: false,
            bt_scanning: false,
            sound: data::Sound::default(),
            display: data::Display::default(),
            power: data::Power::default(),
            storage: Vec::new(),
            time: data::Time::default(),
            zone_query: String::new(),
            desk: data::Desk::default(),
            desk_saved: false,
            start: data::start_mode(),
            updates: data::Updates::default(),
            history: Vec::new(),
            desktop: read_desktop(),
            join: None,
            keep: None,
            rename: None,
            busy: None,
            note: String::new(),
            error: String::new(),
        };
        let t = Task::batch([a.read(Src::Info), a.read(Src::History), a.reads_for(first)]);
        a.error.clear();
        (a, t)
    }

    pub fn read(&self, s: Src) -> Task<Msg> {
        let cmd = match s {
            Src::Info => data::sh(data::INFO),
            Src::Wifi => data::sh(data::WIFI),
            Src::Bt => data::sh(data::BT),
            Src::Sound => data::sh(data::SOUND),
            Src::Display => data::sh(data::DISPLAY),
            Src::Power => data::sh(data::POWER),
            Src::Storage => data::sh(data::STORAGE),
            Src::Time => data::sh(data::TIME),
            Src::Zones => data::sh("timedatectl list-timezones 2>/dev/null"),
            Src::Desk => data::sh(data::DESK),
            Src::Updates => vec!["pkexec".into(), data::ADMIN.into(), "update-status".into()],
            Src::History => vec!["python3".into(), io_helper(), "history".into(), "20".into()],
        };
        Task::perform(run::spawn(cmd), move |o| Msg::Got(s, o))
    }

    /// What a page needs read when it opens.
    fn reads_for(&self, id: &str) -> Task<Msg> {
        let srcs: &[Src] = match id {
            "home" => &[Src::Wifi, Src::Bt, Src::Storage, Src::Display, Src::Updates],
            "display" => &[Src::Display],
            "sound" => &[Src::Sound],
            "power" => &[Src::Power],
            "storage" => &[Src::Storage],
            "datetime" => &[Src::Time, Src::Zones],
            "your-desktop" => &[Src::Desk],
            "wifi" | "net" => &[Src::Wifi],
            "bluetooth" | "bt" => &[Src::Bt],
            "updates" => &[Src::Updates],
            "system" | "about" => &[Src::Display, Src::Sound, Src::Power, Src::Storage],
            _ => &[],
        };
        Task::batch(srcs.iter().map(|s| self.read(*s)))
    }

    fn open(&mut self, id: &'static str) -> Task<Msg> {
        self.page = id;
        self.cat = cat_of(id);
        self.query.clear();
        self.join = None;
        self.rename = None;
        self.error.clear();
        self.note.clear();
        self.desk_saved = false;
        self.reads_for(id)
    }

    /// Runs a change, then reads `then` again.
    fn act(&mut self, cmd: Vec<String>, then: Src, what: &'static str) -> Task<Msg> {
        self.error.clear();
        Task::perform(run::spawn(cmd), move |o| Msg::Did(then, what, o))
    }

    /// A change to Hearth's own settings (desktop.json), through hearth_settings_io.py with the change history.
    fn set_desktop(&mut self, changes: Value) -> Task<Msg> {
        if let Value::Object(m) = &changes {
            for (k, v) in m {
                self.desktop.insert(k.clone(), v.clone());
            }
        }
        let path = hearth_ui::theme::desktop_json().to_string_lossy().to_string();
        self.act(vec!["python3".into(), io_helper(), "merge".into(), path, changes.to_string(), "--history".into()], Src::History, "That setting couldn't be saved")
    }
}

pub fn read_desktop() -> serde_json::Map<String, Value> {
    match hearth_ui::settings_file::read_json(&hearth_ui::theme::desktop_json()) {
        Value::Object(m) => m,
        _ => serde_json::Map::new(),
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => {
            a.p = hearth_ui::theme::load();
            a.desktop = read_desktop();
        }
        Msg::Got(s, o) => {
            match s {
                Src::Info => a.info = data::info(&o.out),
                Src::Wifi => {
                    a.wifi = data::wifi(&o.out);
                    a.wifi_read = true;
                }
                Src::Bt => {
                    a.bt = data::bt(&o.out);
                    a.bt_read = true;
                }
                Src::Sound => a.sound = data::sound(&o.out),
                Src::Display => a.display = data::display(&o.out),
                Src::Power => a.power = data::power(&o.out),
                Src::Storage => a.storage = data::storage(&o.out),
                Src::Time => {
                    let zones = std::mem::take(&mut a.time.zones);
                    a.time = data::time(&o.out);
                    a.time.zones = zones;
                }
                Src::Zones => a.time.zones = o.out.lines().map(String::from).collect(),
                Src::Desk => a.desk = data::desk(&o.out),
                Src::Updates => match o.json() {
                    Some(v) if o.ok => a.updates = data::updates(&v),
                    _ => {
                        a.updates.read = true;
                        a.updates.error = if o.code == 126 || o.code == 127 { "Hearth couldn't ask for permission to read the updates".into() } else { o.why() };
                    }
                },
                Src::History => a.history = o.json().map(|v| data::changes(&v)).unwrap_or_default(),
            }
        }
        Msg::Did(s, what, o) => {
            a.busy = None;
            if !o.ok {
                let why = o.why();
                a.error = if why.is_empty() { what.to_string() } else { format!("{what}: {why}") };
            }
            if s == Src::Updates && o.ok {
                a.note = match what {
                    "The update didn't finish" => "The update is ready. Restart to start it.".into(),
                    "Hearth couldn't go back" => "The version before starts after a restart.".into(),
                    _ => String::new(),
                };
            }
            return Task::batch([a.read(s), if s == Src::Wifi { a.read(Src::Info) } else { Task::none() }]);
        }
        Msg::Cat(i) => {
            if i < CATS.len() {
                return a.open(CATS[i].2);
            }
        }
        Msg::Open(id) => return a.open(id),
        Msg::Back => {
            if a.join.is_some() {
                a.join = None;
            } else if !a.query.is_empty() {
                a.query.clear();
            } else if let Some(p) = page(a.page) {
                let landing = CATS[cat_of(p.id)].2;
                if a.page != landing {
                    return a.open(landing);
                }
            }
        }
        Msg::Query(q) => a.query = q,
        Msg::SearchPick(id) => return a.open(id),
        // ---- Wi-Fi
        Msg::WifiOn(on) => return a.act(data::sh(&format!("nmcli radio wifi {}", if on { "on" } else { "off" })), Src::Wifi, "Wi-Fi couldn't be turned on or off"),
        Msg::WifiScan => return a.act(data::sh("nmcli dev wifi rescan 2>/dev/null; sleep 3"), Src::Wifi, "Couldn't look for networks"),
        Msg::WifiPick(ssid) => {
            let n = a.wifi.nets.iter().find(|n| n.ssid == ssid).cloned();
            if let Some(n) = n {
                if n.active {
                    return Task::none();
                }
                if n.known || !n.secure {
                    a.busy = Some("wifi");
                    let cmd = if n.known { format!("nmcli con up id {}", data::q(&n.ssid)) } else { format!("nmcli dev wifi connect {}", data::q(&n.ssid)) };
                    return a.act(data::sh(&cmd), Src::Wifi, "Couldn't connect");
                }
                a.join = Some((ssid, String::new()));
            }
        }
        Msg::WifiPw(pw) => {
            if let Some(j) = a.join.as_mut() {
                j.1 = pw;
            }
        }
        Msg::WifiJoin => {
            if let Some((ssid, pw)) = a.join.take() {
                a.busy = Some("wifi");
                return a.act(vec!["nmcli".into(), "dev".into(), "wifi".into(), "connect".into(), ssid, "password".into(), pw], Src::Wifi, "Couldn't connect (is the password right?)");
            }
        }
        Msg::WifiCancel => a.join = None,
        Msg::WifiDisconnect(ssid) => return a.act(vec!["nmcli".into(), "con".into(), "down".into(), "id".into(), ssid], Src::Wifi, "Couldn't disconnect"),
        Msg::WifiForget(ssid) => return a.act(vec!["nmcli".into(), "con".into(), "delete".into(), "id".into(), ssid], Src::Wifi, "Couldn't forget that network"),
        // ---- Bluetooth
        Msg::BtOn(on) => return a.act(data::sh(&format!("bluetoothctl power {}", if on { "on" } else { "off" })), Src::Bt, "Bluetooth couldn't be turned on or off"),
        Msg::BtScan => {
            a.bt_scanning = true;
            return Task::perform(run::spawn(data::sh("timeout 12 bluetoothctl --timeout 10 scan on >/dev/null 2>&1; true")), Msg::BtScanned);
        }
        Msg::BtScanned(_) => {
            a.bt_scanning = false;
            return a.read(Src::Bt);
        }
        Msg::BtConnect(mac, on) => {
            a.busy = Some("bt");
            return a.act(vec!["timeout".into(), "20".into(), "bluetoothctl".into(), if on { "connect".into() } else { "disconnect".into() }, mac], Src::Bt, "Couldn't do that");
        }
        Msg::BtPair(mac) => {
            a.busy = Some("bt");
            let m = data::q(&mac);
            return a.act(data::sh(&format!("timeout 25 bluetoothctl pair {m} && bluetoothctl trust {m} >/dev/null && timeout 20 bluetoothctl connect {m}")), Src::Bt, "Couldn't pair (is the device in pairing mode?)");
        }
        Msg::BtForget(mac) => return a.act(vec!["bluetoothctl".into(), "remove".into(), mac], Src::Bt, "Couldn't forget that device"),
        // ---- Sound
        Msg::Vol(v) => a.sound.vol = v,
        Msg::VolSet => return a.act(vec!["wpctl".into(), "set-volume".into(), "@DEFAULT_AUDIO_SINK@".into(), format!("{:.2}", a.sound.vol)], Src::Sound, "Couldn't change the volume"),
        Msg::Mute => return a.act(data::sh("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), Src::Sound, "Couldn't mute"),
        Msg::Mic(v) => a.sound.mic = v,
        Msg::MicSet => return a.act(vec!["wpctl".into(), "set-volume".into(), "@DEFAULT_AUDIO_SOURCE@".into(), format!("{:.2}", a.sound.mic)], Src::Sound, "Couldn't change the microphone's volume"),
        Msg::MicMute => return a.act(data::sh("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), Src::Sound, "Couldn't mute the microphone"),
        Msg::Out(n) => return a.act(vec!["pactl".into(), "set-default-sink".into(), n], Src::Sound, "Couldn't switch the speakers"),
        Msg::In(n) => return a.act(vec!["pactl".into(), "set-default-source".into(), n], Src::Sound, "Couldn't switch the microphone"),
        // ---- Display
        Msg::Bright(v) => a.display.bright = Some(v),
        Msg::BrightSet => {
            if let Some(b) = a.display.bright {
                return a.act(vec!["brightnessctl".into(), "-q".into(), "-c".into(), "backlight".into(), "set".into(), format!("{}%", b.max(1))], Src::Display, "Couldn't change the brightness");
            }
        }
        Msg::Night(on) => return a.act(data::night_cmd(on), Src::Display, "Couldn't change Night light"),
        Msg::Res(out, w, h) => {
            let o = a.display.outputs.iter().find(|o| o.name == out).cloned();
            if let Some(o) = o {
                if let Some(id) = o.mode_for(w, h) {
                    return display_change(a, &o.name, format!("mode.{id}"), format!("mode.{}", o.current));
                }
            }
        }
        Msg::Rate(out, id) => {
            let o = a.display.outputs.iter().find(|o| o.name == out).cloned();
            if let Some(o) = o {
                return display_change(a, &o.name, format!("mode.{id}"), format!("mode.{}", o.current));
            }
        }
        Msg::Scale(out, s) => {
            let o = a.display.outputs.iter().find(|o| o.name == out).cloned();
            if let Some(o) = o {
                return display_change(a, &o.name, format!("scale.{s}"), format!("scale.{}", o.scale));
            }
        }
        Msg::Keep => a.keep = None,
        Msg::Revert => {
            if let Some(k) = a.keep.take() {
                return a.act(k.undo, Src::Display, "Couldn't put the display back");
            }
        }
        Msg::Tick => {
            if let Some(k) = a.keep.as_mut() {
                k.left = k.left.saturating_sub(1);
                if k.left == 0 {
                    return update(a, Msg::Revert);
                }
            }
        }
        // ---- Power, time, desktop, start
        Msg::Profile(pr) => return a.act(vec!["powerprofilesctl".into(), "set".into(), pr], Src::Power, "Couldn't change the power mode"),
        Msg::Ntp(on) => return a.act(vec!["timedatectl".into(), "set-ntp".into(), if on { "true".into() } else { "false".into() }], Src::Time, "Couldn't change that"),
        Msg::ZoneQuery(q) => a.zone_query = q,
        Msg::Zone(z) => {
            a.zone_query.clear();
            return a.act(vec!["timedatectl".into(), "set-timezone".into(), z], Src::Time, "Couldn't change the time zone");
        }
        Msg::Desk(pick) => {
            a.desk_saved = true;
            a.desk.chosen = pick.to_string();
            return a.act(data::desk_cmd(&a.desk, pick), Src::Desk, "Couldn't save the desktop");
        }
        Msg::SwitchNow => run::detached(&["nimbus-mode", "desktop"]),
        Msg::StartIn(mode) => {
            a.start = mode.to_string();
            let conf = data::session_conf().to_string_lossy().to_string();
            return a.act(vec!["python3".into(), io_helper(), "conf-set".into(), conf, "NIMBUS_START_MODE".into(), mode.into()], Src::History, "Couldn't save Start in");
        }
        // ---- Personalization (hearth-plasma-looks puts the look's Global Theme on when desktop.json changes)
        Msg::Look(l) => return a.set_desktop(serde_json::json!({ "look": l })),
        Msg::Mode(md) => return a.set_desktop(serde_json::json!({ "mode": md })),
        Msg::Accent(c) => return a.set_desktop(serde_json::json!({ "accent": c })),
        // ---- Updates & recovery
        Msg::Check => {
            a.busy = Some("check");
            return a.act(vec!["pkexec".into(), data::ADMIN.into(), "update-check".into()], Src::Updates, "Couldn't look for updates");
        }
        Msg::Update => {
            a.busy = Some("update");
            let to = data::shown_version(&a.updates.available);
            let rp = run::shell_helper("hearth_restore.py");
            let script = format!("python3 {} snapshot --kind update --to {} >/dev/null 2>&1; exec pkexec {} update", data::q(&rp), data::q(&to), data::ADMIN);
            return a.act(data::sh(&script), Src::Updates, "The update didn't finish");
        }
        Msg::Restart => run::detached(&["systemctl", "reboot"]),
        Msg::GoBack => {
            a.busy = Some("back");
            return a.act(vec!["pkexec".into(), data::ADMIN.into(), "rollback".into()], Src::Updates, "Hearth couldn't go back");
        }
        Msg::OpenApp(bin) => run::detached(&[bin]),
        // ---- Home / About
        Msg::Rename => a.rename = Some(a.info.host.clone()),
        Msg::RenameText(t) => a.rename = Some(t.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(63).collect()),
        Msg::RenameSave => {
            if let Some(n) = a.rename.take() {
                if !n.is_empty() {
                    return a.act(vec!["pkexec".into(), data::ADMIN.into(), "hostname".into(), n], Src::Info, "Couldn't rename this PC");
                }
            }
        }
        Msg::RenameCancel => a.rename = None,
        Msg::Undo(id) => return a.act(vec!["python3".into(), io_helper(), "undo".into(), id.to_string()], Src::History, "That change couldn't be undone (it was changed again since)"),
        Msg::Key(KEvent::KeyPressed { key, modifiers, .. }) => return key_press(a, key, modifiers),
        Msg::Key(_) => {}
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

/// A resolution, refresh rate or scale change: put on now, and back by itself in 15 s unless it's kept (the
/// mockup's "X Keep these settings").
fn display_change(a: &mut App, out: &str, set: String, undo: String) -> Task<Msg> {
    a.keep = Some(Keep { undo: vec!["kscreen-doctor".into(), format!("output.{out}.{undo}")], left: 15 });
    a.act(vec!["kscreen-doctor".into(), format!("output.{out}.{set}")], Src::Display, "That display setting didn't take")
}

fn key_press(a: &mut App, key: Key, mods: Modifiers) -> Task<Msg> {
    match key {
        Key::Named(Named::Escape) | Key::Named(Named::BrowserBack) => update(a, Msg::Back),
        Key::Named(Named::PageDown) => update(a, Msg::Cat((a.cat + 1) % CATS.len())),
        Key::Named(Named::PageUp) => update(a, Msg::Cat((a.cat + CATS.len() - 1) % CATS.len())),
        Key::Character(c) if mods.control() && c.as_str() == "w" => win::close(),
        Key::Character(c) if a.keep.is_some() && (c.as_str() == "x" || c.as_str() == "X") => update(a, Msg::Keep),
        _ => Task::none(),
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key)];
    if a.keep.is_some() {
        s.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn view(a: &App) -> Element<'_, Msg> {
    pages::frame(a)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-settings-app", "Hearth Settings: Wi-Fi, Bluetooth, sound, display, power, your desktop, Hearth's look and updates.\n  [PAGE] | --page PAGE   open at a page (wifi, bluetooth, display, sound, power, storage, appearance, updates, about...)") {
        return Ok(());
    }
    iced::application(App::new, update, view)
        .title("Hearth Settings")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-settings", (1180.0, 760.0), (820.0, 540.0)))
        .default_font(app::INTER)
        .run()
}
