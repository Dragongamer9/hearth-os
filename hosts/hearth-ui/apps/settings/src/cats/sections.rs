//! Each section's front page, as the approved Settings redesign draws them (docs/mockups/settings-cleanup, approved
//! Nov 23, 2026; checklist docs/mockups/checklists/settings-cleanup.md): "Settings › <section>" with its line, a
//! banner or a row of four tiles, then the section's pages grouped in cards, every row with its value read from this
//! PC (hearth-settings-values: a value it can't read is left out, never made up), switches right on the page, and the
//! KDE-only pages in one "More KDE settings" row at the bottom (only where KDE is installed: Hearth OS on Debian has
//! none). Clicking the section on the left opens this page; each row opens its page as before.

use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{t, tb, Kind as BtnKind};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{Map, Value};

/// Not a Settings page: the Hearth Store app (hearth-store-app), Hearth's own apps and parts with their updates.
pub const STORE: &str = "@store";

pub const PAGES: &[&str] = &["sec-system", "sec-net", "sec-bt", "sec-link", "sec-pers", "sec-apps", "sec-gaming", "sec-accounts", "sec-hs", "sec-privacy", "sec-a11y", "sec-updates"];

/// The section page for a category (its id in registry::CATEGORIES), if it has one here.
pub fn page_for(cat: &str) -> Option<&'static str> {
    PAGES.iter().chain(crate::cats::ai::PAGES.iter()).copied().find(|p| p.strip_prefix("sec-") == Some(cat))
}

#[derive(Clone, Copy)]
pub enum Act {
    Go(&'static str),
    Run(&'static [&'static str]),
    Later,
}

#[derive(Clone, Copy)]
enum V {
    /// The value under this key, then ›.
    K(&'static str),
    /// Just ›.
    N,
    /// A switch: on when the value under the key is "on"; what turns it on or off (hearth-settings-values set).
    S(&'static str, &'static str),
    /// A button in the row (Use it, Send).
    B(&'static str, Act),
    /// A chip from the value under the key (signed in, connected).
    C(&'static str),
}

struct R {
    go: &'static str,
    label: &'static str,
    line: &'static str,
    v: V,
    /// Only where KDE is installed.
    kde: bool,
}

const fn r(go: &'static str, label: &'static str, line: &'static str, v: V) -> R {
    R { go, label, line, v, kde: false }
}
const fn k(go: &'static str, label: &'static str, line: &'static str, v: V) -> R {
    R { go, label, line, v, kde: true }
}

struct Sec {
    id: &'static str,
    title: &'static str,
    line: &'static str,
    cols: usize,
    cards: &'static [(&'static str, &'static [R])],
    kde: Option<(&'static str, &'static str)>,
}

const SECS: &[Sec] = &[
    Sec {
        id: "system",
        title: "System",
        line: "How this PC looks, sounds, starts and runs",
        cols: 3,
        cards: &[
            ("Screen and sound", &[
                r("display", "Display", "Brightness, night light, scale, resolution", V::K("display")),
                r("sound", "Sound", "Speakers, headphones, microphone", V::K("sound")),
                r("notifications", "Notifications", "Do not disturb, each app, history", V::K("notifications")),
                r("couch-desk-away", "Couch, Desk and Away", "Each screen's mode", V::K("couch")),
            ]),
            ("Power and storage", &[
                r("power", "Power & battery", "Screen off, sleep, power button", V::K("power")),
                r("memory", "Memory", "zram, swap, what closes first", V::K("memory")),
                r("storage", "Storage", "Space on your drives, clean up", V::K("storage_free")),
                r("drives", "Drives & folders", "Hearth (C:), drive letters", V::K("drives")),
            ]),
            ("Starting and the desktop", &[
                r("startup", "Startup", "Game Mode, Desktop Mode or Server mode", V::K("startup")),
                r("your-desktop", "Your desktop", "Mantel or Mantel in Rust", V::K("your_desktop")),
                r("boot", "Boot splash", "While Hearth OS starts", V::K("boot_splash")),
                // Only on the live USB (the owner, Nov 24: "remove the annouying install hearth os in the settings").
                r("install", "Install Hearth", "Next to Windows or on its own", V::N),
            ]),
            ("Language and time", &[
                r("datetime", "Date & time", "Time zone, 24-hour clock", V::K("datetime")),
                r("region", "Region & language", "Language, dates, money", V::K("region")),
                r("clipboard", "Clipboard", "History (Super+V)", V::K("clipboard")),
                r("search", "Search", "File index, inside documents", V::K("search")),
            ]),
            ("Reaching this PC", &[
                r("remote-desktop", "Remote desktop", "VNC through Tailscale", V::K("vnc")),
                k("kde-krdp", "Remote desktop (RDP)", "For Windows' Remote Desktop app", V::K("rdp")),
            ]),
            ("About", &[
                r("about", "About", "Version, parts and name", V::K("hearth_os")),
                k("kde-info", "System information", "Processor, graphics, sensors, USB, PCI", V::N),
            ]),
        ],
        kde: Some(("kde-session", "Desktop session, Plasma renderer, KRunner, trash, KDE background services, your folders, day and night times, colour profiles")),
    },
    Sec {
        id: "net",
        title: "Network & internet",
        line: "How this PC gets online, and how your devices reach it",
        cols: 2,
        cards: &[
            ("Wi-Fi", &[]),
            ("Private and away", &[
                r("hs-tailscale", "Tailscale", "Your devices from anywhere, privately", V::K("tailscale")),
                r("vpn", "VPN", "WireGuard, OpenVPN, IPsec and more", V::K("vpn")),
                r("hotspot", "Mobile hotspot", "Share this PC's internet", V::K("hotspot")),
                r("proxy", "Proxy", "For apps and browsers", V::K("proxy")),
            ]),
            ("Details", &[
                r("net-details", "Connection details", "DNS, IP, MAC, 802.1X, duplicates", V::N),
                k("kde-webshortcuts", "Web search keywords", "gg:cats in KRunner and Files", V::K("webkeys")),
                k("kde-push", "Push services", "UnifiedPush for apps", V::K("push")),
                k("kde-netpref", "Connection preferences", "How long KDE apps wait", V::K("netpref")),
            ]),
            ("This network", &[
                r("net-details", "Metered", "Hold back big downloads", V::S("metered", "metered")),
                r("net-details", "Private network", "Other devices can find this PC", V::S("private_net", "private_net")),
                r("net-details", "Data used this month", "Wi-Fi", V::K("data_used")),
            ]),
        ],
        kde: None,
    },
    Sec {
        id: "bt",
        title: "Bluetooth & devices",
        line: "Everything plugged in or paired",
        cols: 3,
        cards: &[
            ("Wireless", &[
                r("bluetooth", "Bluetooth", "Headphones, controllers and more", V::K("bt_connected")),
                r("phone", "Phone", "Notifications, files, calls", V::K("phone")),
                r("kwin-tablet", "Drawing tablet & touchscreen", "Pen pressure, buttons", V::K("tablet")),
            ]),
            ("Typing and pointing", &[
                r("keyboard", "Keyboard", "Layouts, on-screen keyboard, shortcuts", V::K("kbd_layout")),
                r("mouse", "Mouse & touchpad", "Speed, scrolling, tapping", V::K("mouse")),
                r("g15", "Logitech G15", "G-keys, its screen, lights", V::K("g15")),
                k("kde-shortcuts", "Shortcuts", "Every shortcut, Hearth's and KDE's", V::N),
            ]),
            ("Plugged in", &[
                r("printers", "Printers & scanners", "Queues, ink, scan to PDF", V::K("printers")),
                r("usb-devices", "USB devices & drives", "What happens when one's plugged in", V::K("usb")),
                r("thunderbolt", "Thunderbolt", "Docks, drives and screens", V::K("thunderbolt")),
                k("kde-device-actions", "Device actions", "Plug in a phone, camera or disc", V::N),
            ]),
        ],
        kde: Some(("kde-plasmakeyboard", "Plasma Keyboard (on-screen keyboard sounds and keys)")),
    },
    Sec {
        id: "link",
        title: "Hearth Link",
        line: "Your Hearth devices together: files, AI, games and help, encrypted between them",
        cols: 2,
        cards: &[
            ("Share between them", &[
                r("link", "Files", "Drop files on any device", V::K("link_files")),
                r("ai", "AI", "Share this PC's AI · use another's", V::K("ai_share")),
                r("link", "Clipboard", "Copy here, paste there", V::S("link_clipboard", "link_clipboard")),
                r("link", "Game library", "See what's installed where", V::K("link_library")),
            ]),
            ("Your devices", &[
                r("devices", "Battery, temperatures, drives", "Each device at a glance", V::N),
                r("devices", "Find a device", "Ring it, show it on the map", V::N),
                r("devices", "Forget a device", "Unpair and stop sharing", V::N),
            ]),
        ],
        kde: None,
    },
    Sec {
        id: "pers",
        title: "Personalization",
        line: "How Hearth looks, from the colours to where every bar goes",
        cols: 3,
        cards: &[
            ("Your desktop", &[
                r("desktop-layout", "Desktop layout", "Ready-made layouts, or your own", V::K("layout")),
                r("bar-dock", "Bar and dock", "Edge, auto-hide, app menu", V::K("bar")),
                r("wallpaper-more", "Wallpaper", "Per screen, slideshow, picture of the day", V::K("wallpaper")),
                r("my-looks", "My looks", "Save it all and put it back", V::K("my_looks")),
            ]),
            ("Windows", &[
                r("kwin-effects", "Window effects", "Animations, blur, overview", V::K("effects")),
                r("kwin-desktops", "Virtual desktops", "How many, names, switching", V::K("desktops")),
                r("kwin-tiling", "Tiling", "Tile layouts, snapping", V::K("tiling")),
                r("kwin-edges", "Screen edges", "Hot corners, swipes", V::K("edges")),
                r("kwin-behaviour", "Window behaviour", "Focus, title bar clicks", V::K("focus")),
                k("kde-tabbox", "Task switcher", "What Alt+Tab lists", V::K("tabbox")),
            ]),
            ("Text and frames", &[
                r("fonts", "Fonts, cursor & icons", "Font size, smoothing, cursor", V::K("font")),
                r("look-more", "Window frames", "Accent in frames, apps follow Hearth's colours", V::K("frames")),
                r("look-more", "Sign-in splash", "After you sign in", V::K("splash")),
            ]),
        ],
        kde: Some(("kde-style", "Application style, Plasma style, colours, global theme, title bars, KWin scripts, activities, font management")),
    },
    Sec {
        id: "apps",
        title: "Apps",
        line: "What's installed, what opens what, and what starts by itself",
        cols: 3,
        cards: &[
            ("Your apps", &[
                r("apps", "Installed apps", "Uninstall, move, repair", V::K("apps_n")),
                r(STORE, "Hearth Store", "Hearth's apps and parts, updates", V::K("parts_ready")),
                r("store", "App store (Flathub)", "Find and install apps", V::N),
                r("winapps", "Windows apps", "Programs with Proton", V::K("winapps")),
            ]),
            ("What opens what", &[
                r("defaults", "Default apps", "Web, pictures, music, mail", V::K("browser")),
                r("file-types", "File types", "Each kind of file and its app", V::N),
                r("menu-editor", "App menu editor", "Rename, hide, add your own", V::N),
                r("browser", "Web browser", "Hearth's browser settings", V::N),
            ]),
            ("Running and allowed", &[
                r("autostart", "Autostart", "Starts when you sign in", V::K("autostart")),
                r("services", "Background services", "Run in the background for you", V::K("services")),
                r("app-permissions", "App permissions", "What Flathub apps may reach", V::N),
                r("kwin-rules", "Window rules", "Where and how windows open", V::K("rules")),
                r("claude", "Claude", "Claude on this PC", V::N),
            ]),
        ],
        kde: Some(("kde-cddb", "Music CDs (album names, ripping quality)")),
    },
    Sec {
        id: "gaming",
        title: "Gaming",
        line: "Game Mode, your stores, controllers and streaming",
        cols: 3,
        cards: &[
            ("Game Mode", &[
                r("steam", "Steam Game Mode and Decky", "Big-screen mode, plugins, Hearth Sync", V::K("decky")),
                r("gameperf", "Performance", "Resolution, refresh, frame limit, HDR", V::K("perf")),
                r("startup-animation", "Startup animation", "While Game Mode starts", V::K("anim")),
                r("kwin-games", "Full-screen games & X11 apps", "Tearing, X11 keys, scaling", V::K("tearing")),
            ]),
            ("Your games", &[
                r("games", "Games & stores", "Epic, GOG, Xbox, mods", V::K("stores")),
                r("savedgames", "Saved games", "Backed up and put back (Ludusavi)", V::K("saves")),
                r("discord", "Discord", "Rich presence, friends joining", V::K("discord")),
            ]),
            ("Playing", &[
                r("controller", "Controller", "Mouse, layout, remapping", V::K("controller")),
                r("remote", "Remote play", "Stream to phone or TV, play another PC", V::K("remote")),
                r("handheld", "Handheld", "Power limit, fans, lights", V::K("handheld")),
                k("kde-gamecontroller", "Controllers in KDE", "As a mouse and keyboard", V::N),
            ]),
        ],
        kde: None,
    },
    Sec {
        id: "accounts",
        title: "Accounts",
        line: "You, the people on this PC, and how you sign in",
        cols: 3,
        cards: &[
            ("You and others", &[
                r("users", "Your account", "Picture, name, password", V::N),
                r("users", "Other people", "Accounts on this PC", V::K("others")),
                r("hs-people", "Family and friends", "People you share with (Home server)", V::K("family")),
            ]),
            ("Signing in", &[
                r("signin", "Sign-in options", "Automatic, password, lock after sleep", V::K("signin")),
                r("signin", "Screen locking", "After how long, after waking", V::K("lock_after")),
                r("login-screen", "Login screen", "The picture behind it", V::K("login_pic")),
                r("signin", "Fingerprint", "On this PC's reader", V::K("fingerprint")),
            ]),
            ("Online accounts", &[]),
        ],
        kde: None,
    },
    Sec {
        id: "hs",
        title: "Home server",
        line: "This PC serving your home: files, media, AI and backups",
        cols: 3,
        cards: &[
            ("Services", &[
                r("hs-services", "Services", "Media, files on the web, AI chat, pictures", V::K("server_count")),
                r("hs-apps", "Server apps", "Game servers, your own apps", V::K("server_apps")),
                r("ai", "AI devices", "Every device's AI (Settings › AI)", V::N),
                r("hs-gaming", "Gaming", "Games folder, Steam downloads at home", V::K("hs_cache")),
            ]),
            ("Files and people", &[
                r("hs-shares", "Shared folders", "What your devices can open", V::K("shares")),
                r("hs-files", "Files & links", "Search, share links, drop-off", V::K("hs_files")),
                r("hs-people", "People", "Family and friends, own passwords", V::K("family")),
                r("hs-family", "Family & home", "Family folder, calendar, cameras", V::K("hs_family")),
            ]),
            ("Keeping it safe", &[
                r("hs-drives", "Drives & undo", "Health, snapshots", V::K("drive_health")),
                r("hs-backups", "Backups & sync", "To a drive, a device or the cloud", V::K("backup_last")),
                r("hs-safety", "Safety", "Who reaches what, ransomware guard", V::K("hs_safety")),
                r("hs-alerts", "Alerts & power", "Failing drives, sleep, UPS", V::K("hs_alerts")),
                r("hs-phone", "Phone notifications", "Through ntfy", V::K("ntfy")),
            ]),
        ],
        kde: None,
    },
    Sec {
        id: "privacy",
        title: "Privacy & security",
        line: "What's protected, and what leaves this PC (almost nothing)",
        cols: 3,
        cards: &[
            ("Protection", &[
                r("firewall", "Firewall", "On or off, open ports", V::K("firewall")),
                r("firmware", "Firmware & device security", "Secure Boot, TPM, firmware updates", V::K("firmware")),
                r("vaults", "Vaults", "Encrypted folders", V::K("vaults")),
                r("passwords", "Passwords", "Keyrings, what apps keep", V::K("keyring")),
            ]),
            ("What Hearth remembers", &[
                r("recent-files", "Recent files", "Remember, for how long, forget", V::K("recent")),
                r("clipboard", "Clipboard history", "Super+V", V::K("clipboard")),
                r("search", "Search index", "Files Hearth can find", V::K("search")),
            ]),
            ("What leaves this PC", &[
                r("player-count", "Player count", "Hearth OS's anonymous count", V::S("player_count", "player_count")),
                r("updates", "Send logs", "Only when you press it", V::N),
                k("kde-feedback", "KDE user feedback", "Anonymous facts to KDE", V::S("kde_feedback", "kde_feedback")),
            ]),
        ],
        kde: None,
    },
    Sec {
        id: "a11y",
        title: "Accessibility",
        line: "Make Hearth easier to see, hear and use",
        cols: 3,
        cards: &[
            ("Seeing", &[]),
            ("Hearing", &[
                r("accessibility", "Mono audio", "", V::S("a.monoAudio", "access.monoAudio")),
                r("accessibility", "Flash the screen for alerts", "", V::S("a.visualBell", "access.visualBell")),
                r("accessibility", "Live captions", "Speech in videos and calls, on this PC", V::K("captions")),
            ]),
            ("Using", &[
                r("accessibility", "Sticky keys", "", V::S("a.stickyKeys", "access.stickyKeys")),
                r("accessibility", "Slow keys", "", V::S("a.slowKeys", "access.slowKeys")),
                r("accessibility", "Mouse keys", "Move the pointer with the number pad", V::S("a.mouseKeys", "access.mouseKeys")),
                r("accessibility", "On-screen keyboard", "Always, or with a controller", V::K("osk")),
                r("accessibility", "Screen reader (Orca)", "Super+Alt+S", V::S("a.screenReader", "access.screenReader")),
            ]),
        ],
        kde: None,
    },
    Sec {
        id: "updates",
        title: "Updates & recovery",
        line: "Hearth OS, Debian underneath, Hearth's parts and your apps — and getting back if something goes wrong",
        cols: 3,
        cards: &[
            ("Updates", &[
                r("updates", "Hearth OS", "@os_line", V::K("os_state")),
                r("updates", "Debian updates", "With Hearth OS · security, drivers", V::K("debian_state")),
                r(STORE, "Hearth parts", "Mantel, apps, services · no restart", V::K("parts_ready")),
                r("store", "Apps (Flathub)", "Through the Hearth Store", V::K("flathub_state")),
            ]),
            ("How they come", &[
                r("updates", "Hearth OS updates by themselves", "In at your next restart", V::S("os_auto", "os_auto")),
                r(STORE, "Hearth parts by themselves", "Off: only when you press Update", V::S("parts_auto", "parts_auto")),
                r(STORE, "Get test versions", "Try new parts before everyone", V::S("test_versions", "test_versions")),
                k("kde-updates", "App updates (Discover)", "KDE's own reminders", V::K("discover")),
            ]),
            ("Recovery and help", &[
                r("updates", "Previous version", "@os_prev_line", V::B("Use it", Act::Go("updates"))),
                r("backup", "Backup and restore Hearth", "Your settings and looks", V::N),
                r("backup", "Send logs", "To Dragongamer9 in one press", V::B("Send", Act::Run(&["nimbus", "--send-logs", "Sent from Settings › Updates & recovery"]))),
                r("backup", "Recovery USB", "Make one for this PC", V::N),
            ]),
        ],
        kde: None,
    },
];

/// A page's name and its value as the section pages show it (Home's "You use these most"): the first row that opens
/// it, with the value under its key in `v` (empty when the PC can't tell).
pub fn row_of(page: &str, v: &Map<String, Value>) -> Option<(&'static str, String)> {
    for sec in SECS {
        for (_, rows) in sec.cards.iter() {
            if let Some(x) = rows.iter().find(|x| x.go == page) {
                let val = match x.v {
                    V::K(k) | V::C(k) => s(v, k),
                    V::S(k, _) => if on(v, k) { "on".into() } else { "off".into() },
                    _ => String::new(),
                };
                return Some((x.label, val));
            }
        }
    }
    None
}

#[derive(Debug, Default)]
pub struct State {
    pub vals: Map<String, Value>,
    pub of: String,
    pub later: bool,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Arrange this page's parts (secs.rs parts_dyn; main.rs gives it to Describe it)
    Arrange(hearth_ui::describe::DMsg),
    Values(String, run::Output),
    Go(&'static str),
    Run(&'static [&'static str]),
    Banner(usize),
    Switch(&'static str, &'static str, bool),
    Done(run::Output),
    Accent(&'static str),
    Saved(run::Output),
    TextSize(f32),
    TextSizeDone,
}

impl From<hearth_ui::describe::DMsg> for Msg {
    fn from(d: hearth_ui::describe::DMsg) -> Msg {
        Msg::Arrange(d)
    }
}

fn helper() -> String {
    run::shell_helper("hearth-settings-values")
}

fn load(sec: &str) -> Task<Msg> {
    let s = sec.to_string();
    Task::perform(run::spawn(vec!["python3".into(), helper(), s.clone()]), move |o| Msg::Values(s.clone(), o))
}

pub fn open(_sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    let sec = page.trim_start_matches("sec-");
    if st.of != sec {
        st.vals.clear();
    }
    st.of = sec.to_string();
    st.msg.clear();
    load(sec)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Arrange(_) => return Task::none(),
        Msg::Values(sec, o) => {
            if sec == st.of {
                if let Some(Value::Object(v)) = o.json() {
                    st.vals = v;
                }
            }
        }
        Msg::Go(id) if id == STORE => run::detached(&["hearth-store-app"]),
        Msg::Go(id) => sh.go(id),
        Msg::Run(cmd) => return act(sh, st, Act::Run(cmd)),
        Msg::Banner(i) => return banner_act(sh, st, i),
        Msg::Switch(val, key, on) => {
            st.vals.insert(val.to_string(), Value::String(if on { "on" } else { "off" }.into()));
            return Task::perform(run::spawn(vec!["python3".into(), helper(), "set".into(), key.into(), if on { "on" } else { "off" }.into()]), Msg::Done);
        }
        Msg::Done(o) => {
            match o.json() {
                Some(v) if v.get("ok").and_then(Value::as_bool) == Some(true) => st.msg.clear(),
                Some(v) => st.msg = v.get("error").and_then(Value::as_str).unwrap_or("That didn't change.").to_string(),
                None => st.msg = o.why(),
            }
            return load(&st.of.clone());
        }
        // An accent swatch: Hearth's own setting in desktop.json, through hearth_settings_io (atomic, .bak, history).
        Msg::Accent(c) => {
            let cmd = sh.desktop_cmd(serde_json::json!({ "accent": c }));
            return Task::perform(run::spawn(cmd), Msg::Saved);
        }
        Msg::TextSize(x) => {
            if let Some(Value::Object(a)) = st.vals.get_mut("access") {
                a.insert("textScale".into(), serde_json::json!(x));
            }
        }
        Msg::TextSizeDone => {
            let x = st.vals.get("access").and_then(|a| a.get("textScale")).cloned().unwrap_or(serde_json::json!(1.0));
            let j = serde_json::json!({ "textScale": x }).to_string();
            return Task::perform(run::spawn(vec!["python3".into(), run::shell_helper("hearth-access"), "set".into(), j]), Msg::Saved);
        }
        Msg::Saved(o) => {
            st.msg = if o.ok { String::new() } else { o.why() };
        }
    }
    Task::none()
}

fn act(sh: &mut Shared, st: &mut State, a: Act) -> Task<Msg> {
    match a {
        Act::Go(id) if id == STORE => run::detached(&["hearth-store-app"]),
        Act::Go(id) => sh.go(id),
        Act::Run(cmd) => {
            let v: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
            return Task::perform(run::spawn(v), Msg::Done);
        }
        Act::Later => st.later = true,
    }
    Task::none()
}

/// What each banner's buttons do, in order (the section's own list: banner_buttons).
fn banner_act(sh: &mut Shared, st: &mut State, i: usize) -> Task<Msg> {
    let acts = banner_acts(&st.of);
    match acts.get(i) {
        Some(a) => act(sh, st, *a),
        None => Task::none(),
    }
}

fn banner_acts(sec: &str) -> Vec<Act> {
    match sec {
        "net" => vec![Act::Go("net-details"), Act::Go("wifi")],
        "link" => vec![Act::Go("link")],
        "pers" => vec![Act::Go("my-looks"), Act::Go("appearance")],
        "gaming" => vec![Act::Run(&["nimbus-mode", "game"])],
        "accounts" => vec![Act::Go("users")],
        "hs" => vec![Act::Go("hs-overview"), Act::Go("hs-server")],
        "updates" => vec![Act::Later, Act::Go("updates")],
        "home" => vec![Act::Later, Act::Go(STORE)],
        _ => vec![],
    }
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}

pub fn s<'a>(v: &'a Map<String, Value>, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => if *b { "on".into() } else { "off".into() },
        _ => String::new(),
    }
}

/// A switch's state: "on" / true, or an accessibility setting under "a.<name>" (hearth-access's settings).
fn on(v: &Map<String, Value>, k: &str) -> bool {
    if let Some(name) = k.strip_prefix("a.") {
        return v.get("access").and_then(|a| a.get(name)).and_then(Value::as_bool).unwrap_or(false);
    }
    matches!(v.get(k), Some(Value::Bool(true))) || s(v, k) == "on"
}

fn sec_of(page: &str) -> Option<&'static Sec> {
    let id = page.strip_prefix("sec-")?;
    SECS.iter().find(|s| s.id == id)
}

/// The title over a section page: "Settings › System" (the section in the accent) and its line.
pub fn title<'a, M: 'a>(p: &Pal, page: &str) -> Option<Element<'a, M>> {
    let sec = sec_of(page).map(|s| (s.title, s.line)).or_else(|| if page == "sec-ai" { Some(("AI", "Everything AI on this PC in one place")) } else { None })?;
    // Windows 11's look (settings-windows-11 system.png): the section's own name, nothing before or under it.
    let _ = sec.1;
    Some(tb(sec.0, 34.0, p.text).into())
}

fn rows_of<'a>(p: &Pal, v: &'a Map<String, Value>, list: &'static [R], kde: bool) -> Vec<Element<'a, Msg>> {
    let live = v.contains_key("live");
    let shown: Vec<&R> = list.iter().filter(|x| (!x.kde || kde) && (x.go != "install" || live)).collect();
    let n = shown.len();
    shown
        .into_iter()
        .enumerate()
        .map(|(j, x)| {
            let last = j + 1 == n;
            // "@key": the line is a value read from the PC (Hearth OS's version, the version before).
            let mut line_v = x.line.strip_prefix('@').map(|k| s(v, k));
            // Sticky, slow and mouse keys: labwc (Hearth's desktop) has none of them; said plainly, the switch saves it.
            if matches!(x.v, V::S("a.stickyKeys" | "a.slowKeys" | "a.mouseKeys", _)) && s(v, "access_keys") == "no" {
                line_v = Some("Not on this desktop yet (saved)".into());
            }
            let line = line_v.as_deref().unwrap_or(x.line);
            match x.v {
                V::K(key) => vrow(p, x.label, line, value_chev(p, &s(v, key)), Some(Msg::Go(x.go)), last),
                V::N => vrow(p, x.label, line, value_chev(p, ""), Some(Msg::Go(x.go)), last),
                V::C(key) => {
                    let c = s(v, key);
                    vrow(p, x.label, line, if c.is_empty() { value_chev(p, "") } else { sec_chip(p, &c, true) }, Some(Msg::Go(x.go)), last)
                }
                V::S(val, key) => {
                    let o = on(v, val);
                    let sw = switch(p, o, Some(Msg::Switch(val, key, !o)));
                    vrow(p, x.label, line, sw, None, last)
                }
                V::B(label, a) => {
                    let m = match a {
                        Act::Go(id) => Msg::Go(id),
                        Act::Run(cmd) => Msg::Run(cmd),
                        Act::Later => Msg::Banner(99),
                    };
                    let b = btn(p, label, BtnKind::Normal, false, Some(m));
                    vrow(p, x.label, line, b, None, last)
                }
            }
        })
        .collect()
}

fn grid<'a>(cards: Vec<Element<'a, Msg>>, cols: usize) -> Element<'a, Msg> {
    let mut c = Column::new().spacing(18.0);
    let mut it = cards.into_iter().peekable();
    while it.peek().is_some() {
        let mut r = Row::new().spacing(18.0);
        for _ in 0..cols {
            match it.next() {
                Some(x) => r = r.push(container(x).width(Length::FillPortion(1))),
                None => r = r.push(Space::new().width(Length::FillPortion(1))),
            }
        }
        c = c.push(r);
    }
    c.into()
}

/// Pads a row of cards' row lists to the same length (empty rows), so the cards in a row line up as the mockups draw.
fn even<'a>(mut lists: Vec<Vec<Element<'a, Msg>>>, cols: usize) -> Vec<Vec<Element<'a, Msg>>> {
    for chunk in lists.chunks_mut(cols) {
        let most = chunk.iter().map(|l| l.len()).max().unwrap_or(0);
        for l in chunk.iter_mut() {
            while l.len() < most {
                l.push(Space::new().height(Length::Fixed(64.0)).into());
            }
        }
    }
    lists
}

fn top<'a>(p: &Pal, st: &'a State, sec: &str) -> Option<Element<'a, Msg>> {
    let v = &st.vals;
    let g = |x: &str| s(v, x);
    let b = |i: usize, label: &'static str, primary: bool| btn(p, label, if primary { BtnKind::Primary } else { BtnKind::Normal }, false, Some(Msg::Banner(i)));
    let tiles = |list: Vec<(&'static str, String, String, &'static str)>| -> Element<'a, Msg> {
        let mut r = Row::new().spacing(18.0);
        for (label, big, small, go) in list {
            r = r.push(container(stile(p, label, big, small, Some(Msg::Go(go)))).width(Length::FillPortion(1)));
        }
        r.into()
    };
    Some(match sec {
        "net" => {
            let name = g("net_name");
            let ok = !name.is_empty();
            let line = [g("net_kind"), g("net_ip"), if g("tailscale_ip").is_empty() { String::new() } else { format!("Tailscale on ({})", g("tailscale_ip")) }]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            sec_banner(p, sec_dot(if ok { p.good } else { p.warn }), if ok { format!("Connected · {name}") } else { "Not connected".into() }, line, vec![b(0, "Details", false), b(1, "Share Wi-Fi (QR code)", false)])
        }
        "bt" => tiles(vec![
            ("Bluetooth", g("bt_on"), g("bt_line"), "bluetooth"),
            ("Controller", or(g("controller"), "none"), g("controller_line"), "controller"),
            ("Phone", or(g("phone"), "none"), g("phone_line"), "phone"),
            ("Printer", or(g("printer"), "none"), g("printers"), "printers"),
        ]),
        "link" => sec_banner(p, sec_dot(p.good), format!("{} paired", or(g("link_count"), "No devices")), "Encrypted over your home network and Tailscale · nothing opened to the internet".into(), vec![b(0, "Pair a device", true)]),
        "pers" => {
            let sw = container(Space::new()).width(Length::Fixed(42.0)).height(Length::Fixed(42.0)).style(hearth_ui::ui::boxed(p.accent, 10.0, None));
            sec_banner(p, sw.into(), or(g("look"), "Hearth"), format!("Your desktop: {}{}", or(g("layout"), "Mantel"), if g("wallpaper").is_empty() { String::new() } else { format!(" · wallpaper: {}", g("wallpaper")) }), vec![b(0, "My looks", false), b(1, "Change the look", true)])
        }
        "apps" => tiles(vec![
            ("Installed", g("apps_count"), g("apps_line"), "apps"),
            ("Windows apps", or(g("winapps"), "0"), ".exe with Proton".into(), "winapps"),
            ("Starting with you", g("autostart_line"), g("autostart_names"), "autostart"),
            ("Updates", or(g("parts_ready"), "up to date"), "in the Hearth Store".into(), STORE),
        ]),
        "gaming" => sec_banner(p, sec_dot(p.good), "Steam Game Mode with Hearth Sync".into(), [g("decky_line"), g("games_line")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · "), vec![b(0, "Switch to Game Mode", false)]),
        "accounts" => {
            let initial = g("real").chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
            let pic = container(tb(initial, 24.0, Color::WHITE)).center_x(Length::Fixed(64.0)).center_y(Length::Fixed(64.0)).style(hearth_ui::ui::boxed(p.accent, 32.0, None));
            sec_banner(p, pic.into(), or(g("real"), "You"), g("account_line"), vec![b(0, "Change picture", false)])
        }
        "hs" => sec_banner(p, sec_dot(p.good), format!("{} services running", g("server_count").trim_end_matches(" running")), format!("{} — all reached privately through Tailscale", g("server_line")), vec![b(0, "Dashboard", false), b(1, "Server mode", false)]),
        "privacy" => tiles(vec![
            ("Firewall", or(g("firewall"), "off"), g("firewall_line"), "firewall"),
            ("Secure Boot", or(g("secure_boot"), "off"), g("tpm"), "firmware"),
            ("Vaults", or(g("vaults_locked"), "none"), g("vault_names"), "vaults"),
            ("Player count", or(g("player_count"), "on"), "anonymous, one number".into(), "player-count"),
        ]),
        "updates" => {
            if st.later || g("ready_title").is_empty() {
                return None;
            }
            sec_banner(p, sec_dot(p.warn), g("ready_title"), g("ready_line"), vec![b(0, "Later", false), b(1, "Get it now", true)])
        }
        _ => return None,
    })
}

/// A paired device's card (settings-05): its letter, name, what it is with battery, temperature and what it's doing,
/// online, a line of what it uses; Send files, Help them, More.
fn link_card<'a>(p: &Pal, d: &Value) -> Element<'a, Msg> {
    let g = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let online = d.get("online").and_then(Value::as_bool) == Some(true);
    let letter = container(tb(or(g("letter"), "·"), 18.0, p.text)).center_x(Length::Fixed(46.0)).center_y(Length::Fixed(46.0)).style(hearth_ui::ui::boxed(Pal::mix(p.panel, p.text, 0.06), 10.0, None));
    let head = row![
        letter,
        column![tb(g("name"), 17.0, p.text), t(g("line"), 13.0, p.muted)].spacing(2.0).width(Length::Fill),
        sec_chip(p, if online { "online" } else { "offline" }, online)
    ]
    .spacing(14.0)
    .align_y(iced::Center);
    let b = |label: &'static str, go: &'static str| btn(p, label, BtnKind::Normal, false, Some(Msg::Go(go)));
    let body = column![head, t(g("uses"), 14.0, p.muted), row![b("Send files", "link"), b("Help them", "remote"), b("More", "devices")].spacing(10.0)].spacing(14.0);
    container(body).padding(Padding { top: 22.0, right: 20.0, bottom: 18.0, left: 20.0 }).width(Length::Fill).style(hearth_ui::ui::boxed(p.panel, p.r(14.0), Some((Pal::a(p.text, 0.06), 1.0)))).into()
}

fn or(s: String, d: &str) -> String {
    if s.is_empty() { d.to_string() } else { s }
}

/// Accessibility › Seeing's switches: the name, its line, where its state is, what changes it (hearth-access).
const SEEING: &[(&str, &str, &str, &str)] = &[
    ("Bigger cursor", "", "a.largePointer", "access.largePointer"),
    ("High contrast", "", "a.highContrast", "access.highContrast"),
    ("Reduce motion", "Fewer animations, no wobbly windows", "reduce_motion", "reduce_motion"),
    ("Zoom", "Super + plus", "zoom", "zoom"),
];

/// The accent swatches (settings-06): Hearth's ember first; the picked one ringed.
const ACCENTS: &[(&str, (f32, f32, f32))] = &[
    ("#ff7a33", (1.0, 0.478, 0.2)),
    ("#e24a4a", (0.886, 0.29, 0.29)),
    ("#40b5fc", (0.25, 0.71, 0.99)),
    ("#5cd18c", (0.36, 0.82, 0.55)),
    ("#b08cfa", (0.69, 0.55, 0.98)),
    ("#fcd44a", (0.99, 0.83, 0.29)),
    ("#fa8fc7", (0.98, 0.56, 0.78)),
    ("#9ea8ba", (0.62, 0.66, 0.73)),
];

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    let sec = sec_of(page)?;
    let p = a.sh.p;
    let st = &a.sections;
    let v = &st.vals;
    let kde = v.contains_key("kde");
    // its parts (secs.rs parts_dyn): what's on top, each card, KDE's more; as it always was when nothing is customised
    let mut c = Column::new().spacing(18.0);
    let mut has_top = false;
    if !st.msg.is_empty() {
        c = c.push(note(&p, st.msg.clone(), true));
        has_top = true;
    }
    if let Some(t) = top(&p, st, sec.id) {
        c = c.push(t);
        has_top = true;
    }
    if sec.id == "link" {
        let devs = v.get("link_devices").and_then(Value::as_array).cloned().unwrap_or_default();
        if !devs.is_empty() {
            let cards: Vec<Element<'a, Msg>> = devs.iter().map(|d| link_card(&p, d)).collect();
            c = c.push(grid(cards, 3));
            has_top = true;
        }
    }
    if sec.id == "pers" {
        let mut r = Row::new().spacing(14.0).align_y(iced::Center);
        let cur = a.sh.d_str("accent").to_lowercase();
        for &(name, (cr, cg, cb)) in ACCENTS {
            let picked = cur == name || (cur.is_empty() && name == "#ff7a33");
            let sw = container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(40.0)).style(hearth_ui::ui::boxed(Color::from_rgb(cr, cg, cb), 10.0, if picked { Some((Color::WHITE, 3.0)) } else { None }));
            r = r.push(mouse_area(sw).on_press(Msg::Accent(name)).interaction(mouse::Interaction::Pointer));
        }
        r = r.push(Space::new().width(Length::Fill)).push(btn(&p, if s(v, "mode") == "light" { "Light ▾" } else { "Dark ▾" }, BtnKind::Normal, false, Some(Msg::Go("appearance"))));
        c = c.push(r);
        has_top = true;
    }
    // The cards: their rows, the special ones (Wi-Fi's networks, Online accounts, Seeing's slider), evened out.
    let mut lists: Vec<Vec<Element<'a, Msg>>> = Vec::new();
    let mut heads: Vec<&'static str> = Vec::new();
    for (head, rows) in sec.cards.iter() {
        let mut l = rows_of(&p, v, rows, kde);
        match (sec.id, *head) {
            ("net", "Wi-Fi") => {
                let nets = v.get("wifi_list").and_then(Value::as_array).cloned().unwrap_or_default();
                for n in nets {
                    let name = n.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                    let line = n.get("line").and_then(Value::as_str).unwrap_or("").to_string();
                    let right = if n.get("connected").and_then(Value::as_bool) == Some(true) { sec_chip(&p, "connected", true) } else { value_chev(&p, "") };
                    l.push(vrow(&p, &name, &line, right, Some(Msg::Go("wifi")), false));
                }
                let o = on(v, "wifi_on");
                l.push(vrow(&p, "Wi-Fi", "", switch(&p, o, Some(Msg::Switch("wifi_on", "wifi", !o))), None, true));
            }
            ("accounts", "Online accounts") => {
                for acc in v.get("online").and_then(Value::as_array).cloned().unwrap_or_default() {
                    let name = acc.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                    let line = acc.get("line").and_then(Value::as_str).unwrap_or("").to_string();
                    l.push(vrow(&p, &name, &line, sec_chip(&p, "signed in", true), Some(Msg::Go("online-accounts")), false));
                }
                l.push(vrow(&p, "Add an account", "Microsoft, CalDAV, IMAP...", value_chev(&p, ""), Some(Msg::Go("online-accounts")), true));
            }
            ("a11y", "Seeing") => {
                let text = v.get("access").and_then(|x| x.get("textScale")).and_then(Value::as_f64).unwrap_or(1.0) as f32;
                let slider = container(slider_f32(text, 1.0..=2.0, 0.25, Msg::TextSize, Msg::TextSizeDone)).width(Length::Fixed(190.0));
                l.push(vrow(&p, "Text size", "", slider.into(), None, false));
                for &(label, line, val, set) in SEEING {
                    let o = on(v, val);
                    l.push(vrow(&p, label, line, switch(&p, o, Some(Msg::Switch(val, set, !o))), None, false));
                }
                l.push(vrow(&p, "Colour filters", "Deuteranopia, protanopia, greyscale", value_chev(&p, &or(s(v, "filter"), "none")), Some(Msg::Go("accessibility")), true));
            }
            _ => {}
        }
        lists.push(l);
        heads.push(head);
    }
    let lists = even(lists, sec.cols);
    let mut items: Vec<(String, Element<'a, Msg>)> = Vec::new();
    if has_top {
        items.push((String::new(), c.into()));
    }
    let n_cards = heads.len();
    for (l, h) in lists.into_iter().zip(heads) {
        items.push((h.to_string(), scard(&p, h, l)));
    }
    let mut has_kde = false;
    if let Some((go, list)) = sec.kde {
        if kde {
            items.push(("More in KDE's settings".to_string(), kde_more(&p, list, Msg::Go(go))));
            has_kde = true;
        }
    }
    let cols = sec.cols;
    Some(crate::secs::parts_dyn(a, items, move |els| {
        let mut it = els.into_iter();
        let mut c = Column::new().spacing(18.0);
        if has_top {
            if let Some(t) = it.next() {
                // the top's own column, its parts 18 apart as before
                c = c.push(t);
            }
        }
        let cards: Vec<Element<'a, Msg>> = it.by_ref().take(n_cards).collect();
        c = c.push(grid(cards, cols));
        if has_kde {
            if let Some(k) = it.next() {
                c = c.push(k);
            }
        }
        c.into()
    }))
}
