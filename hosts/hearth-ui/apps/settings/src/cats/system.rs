//! System: every page of Mantel's System category (settings-pages.js, cat "system"), ported from its QML page
//! (hosts/linux/desktop/shell/Set*.qml) with the same helpers and files, so Debian's Mantel and the Fedora edition's
//! Rust Settings share their settings. The Display page is the approved mockup's (settings-A2-display-mouse.png).
//!
//! Where the Fedora edition differs (KDE Plasma on KWin, no Mantel running), each page also sets KDE's own setting,
//! the one Plasma really uses, with KDE's own tool (kwriteconfig6, which writes through KConfig: atomic, KDE's lock),
//! and says so in a comment: Do not disturb and quiet in full screen (plasmanotifyrc), the screen-off and sleep times,
//! the power button and the lid (PowerDevil's powerdevilrc), over-amplification (plasmaparc), the sound theme
//! (kdeglobals [Sounds]), Klipper (klipperrc), the file index (Baloo), KRunner's plugins (krunnerrc), the clock
//! (Plasma's digital clock) and the session (ksmserverrc). Hearth's own copy of each is kept too (desktop.json and
//! the other nimbus files, through hearth_settings_io: atomic, .bak, merged, the change history).
//!
//! The pages live in sibling files (system_*.rs): this file has the machinery they share: reading (one program per
//! read, off the window's thread, each with a time limit), running a change and reading again, and the few pieces
//! of UI the pages share (a chooser list, the Plasma check). Opening a page only reads; nothing is written until a
//! control is used (red flag 9).
//!
//! CI's pictures (.github/scripts/hearth-apps-rust-shots.d/settings-system.sh) set HEARTH_SYSTEM_FIXTURE to a folder
//! of answers (one file per read, named like the read): every read then comes from there, and nothing is run.

#[path = "system_about.rs"]
mod about;
#[path = "system_display.rs"]
mod display;
#[path = "system_lang.rs"]
mod lang;
#[path = "system_more.rs"]
mod more;
#[path = "system_power.rs"]
mod power;
#[path = "system_sound.rs"]
mod sound;

use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{t, t1, G};
use hearth_ui::Pal;
use iced::widget::{column, row, Column};
use iced::{Element, Length, Subscription, Task};
use serde_json::Value;
use std::collections::HashMap;

pub const PAGES: &[&str] = &[
    "install", "display", "sound", "sound-speakers", "sound-apps", "push-to-talk", "notifications", "sounds", "power", "power-profiles", "power-awake",
    "battery", "memory", "storage", "drives", "your-desktop", "startup", "boot", "remote-desktop", "datetime", "region", "languages", "spellcheck",
    "language-packs", "input-methods", "clipboard", "search", "about", "info-cpu", "info-gpu", "info-memory", "info-graphics", "info-pci", "info-usb",
    "info-sensors", "info-modules", "info-interrupts", "info-energy",
];

// ------------------------------------------------------------------ a change: programs run one after another

/// A change: its programs (run in turn, stopping at the first that fails), text for the first one's stdin (and
/// HEARTH_INPUT, as Mantel's SetJob gives it), what to say when it worked, which reads to do again, and where to keep
/// its answer (`store`: a read's key, for helpers that answer with the new state).
#[derive(Debug, Clone, Default)]
pub struct Ex {
    pub cmds: Vec<Vec<String>>,
    pub input: String,
    pub ok: String,
    pub fail: String,
    pub re: Vec<&'static str>,
    pub store: Option<&'static str>,
    pub secs: u64,
}

impl Ex {
    pub fn new(cmd: Vec<String>) -> Ex {
        Ex { cmds: vec![cmd], secs: 120, ..Ex::default() }
    }
    pub fn none() -> Ex {
        Ex { secs: 120, ..Ex::default() }
    }
    pub fn and(mut self, cmd: Vec<String>) -> Ex {
        self.cmds.push(cmd);
        self
    }
    pub fn and_all(mut self, cmds: Vec<Vec<String>>) -> Ex {
        self.cmds.extend(cmds);
        self
    }
    pub fn ok(mut self, s: impl Into<String>) -> Ex {
        self.ok = s.into();
        self
    }
    pub fn fail(mut self, s: impl Into<String>) -> Ex {
        self.fail = s.into();
        self
    }
    pub fn re(mut self, k: &'static str) -> Ex {
        self.re.push(k);
        self
    }
    pub fn store(mut self, k: &'static str) -> Ex {
        self.store = Some(k);
        self
    }
    pub fn input(mut self, s: impl Into<String>) -> Ex {
        self.input = s.into();
        self
    }
    pub fn secs(mut self, s: u64) -> Ex {
        self.secs = s;
        self
    }
}

/// A display change waiting to be kept (the mockup's "X Keep these settings"): what to go back to, the screens it's
/// for, and the seconds left.
pub struct Keep {
    pub prev: Value,
    pub names: Vec<String>,
    pub left: u32,
}

#[derive(Default)]
pub struct State {
    pub page: &'static str,
    /// What each read answered (its JSON; Null when it failed).
    pub v: HashMap<&'static str, Value>,
    /// Each read's text, and whether it worked.
    pub raw: HashMap<&'static str, String>,
    pub read_ok: HashMap<&'static str, bool>,
    /// Hearth's own small settings files, read when a page opens (buttons.json, sounds.json, notify.json…).
    pub files: HashMap<&'static str, Value>,
    pub text: HashMap<&'static str, String>,
    pub flag: HashMap<&'static str, bool>,
    pub num: HashMap<&'static str, f32>,
    /// Sliders of things that come and go (each app's volume, each screen's brightness), by name.
    pub numof: HashMap<String, f32>,
    /// The chooser list that's open ("" none).
    pub open: String,
    /// A change running now (its words).
    pub busy: String,
    pub keep: Option<Keep>,
    /// Display: the screen these settings are for ("" the first that's on).
    pub picked: String,
    pub rename: Option<String>,
    pub desk_saved: bool,
    pub desk: data::Desk,
    pub start: String,
    pub updates: data::Updates,
    /// A slider that waits a moment before it's saved (its key, and the ticks left).
    pub settle: Option<(&'static str, u32)>,
    /// Boot splash: the splash whose preview is open, and its pictures (bootpics.rs).
    pub boot_pv: Option<Value>,
    pub flip: crate::bootpics::Flip,
    /// Startup: the schedule as it's being changed ({weekday, weekend}), saved a moment after the last change.
    pub sched: Option<Value>,
}

impl State {
    pub fn j(&self, k: &str) -> &Value {
        self.v.get(k).unwrap_or(&NULL)
    }
    pub fn has(&self, k: &str) -> bool {
        self.v.get(k).map(|v| !v.is_null()).unwrap_or(false)
    }
    pub fn read(&self, k: &str) -> bool {
        self.read_ok.contains_key(k)
    }
    pub fn raw(&self, k: &str) -> &str {
        self.raw.get(k).map(|s| s.as_str()).unwrap_or("")
    }
    pub fn file(&self, k: &str) -> &Value {
        self.files.get(k).unwrap_or(&NULL)
    }
    pub fn txt(&self, k: &str) -> &str {
        self.text.get(k).map(|s| s.as_str()).unwrap_or("")
    }
    pub fn on(&self, k: &str) -> bool {
        self.flag.get(k).copied().unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Arrange this page's parts (Settings' sections as blocks, widgets.rs Secs; main.rs gives it to Describe it)
    Arrange(hearth_ui::describe::DMsg),
    Got(&'static str, run::Output),
    Exec(Ex),
    Done(Ex, run::Output),
    /// Save these keys in desktop.json (hearth_settings_io merge, with the change history), then run the change.
    Desk(Value, Ex),
    /// Merge these keys into ~/.config/nimbus/<file> (buttons.json, sounds.json…), then run the change.
    Json(&'static str, Value, Ex),
    Text(&'static str, String),
    Flag(&'static str, bool),
    Num(&'static str, f32),
    /// A slider of one of several (an app, a screen): its name and value (the last one moved is remembered).
    NumOf(String, f32),
    /// A slider let go: saved a moment later (the page's own save for that key).
    NumDone(&'static str),
    Open(String),
    Go(&'static str),
    App(Vec<String>),
    Reread(&'static str),
    /// Change a page's own unsaved copy (Memory's changes before Save, what's picked to clean up): the read's key,
    /// a path in it ("zram.gb"; "" with Null forgets it all) and the value.
    Draft(&'static str, String, Value),
    /// Several messages, one after another.
    Seq(Vec<Msg>),
    // Display
    Pick(String),
    Try(Value),
    SetNow(Value, String),
    Keep,
    Revert,
    Tick,
    Settle,
    Live,
    // About
    Rename,
    RenameText(String),
    RenameSave,
    RenameCancel,
    Desktop(&'static str),
    SwitchNow,
    StartIn(&'static str),
    // Boot splash
    BootPreview(Option<Value>),
    BootPics,
    BootTick,
    BootHover(String),
    BootUnhover(String),
    // Startup: a change of the schedule (its day, which change, what to do), and saving it.
    Sched(&'static str, usize, SchedOp),
    SchedSave,
}

impl From<hearth_ui::describe::DMsg> for Msg {
    fn from(d: hearth_ui::describe::DMsg) -> Msg {
        Msg::Arrange(d)
    }
}

/// What a change of Server mode's schedule does (SetStartup.qml: ← → move it 30 minutes, A changes its mode, X removes
/// it; Add a change).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SchedOp {
    Move(i32),
    Mode,
    Remove,
    Add,
}

// ------------------------------------------------------------------ reading JSON answers

pub static NULL: Value = Value::Null;

/// The value at "a.b.0.c" (Null when it isn't there).
pub fn at<'v>(v: &'v Value, path: &str) -> &'v Value {
    let mut cur = v;
    for k in path.split('.').filter(|k| !k.is_empty()) {
        cur = match cur {
            Value::Object(m) => m.get(k).unwrap_or(&NULL),
            Value::Array(a) => k.parse::<usize>().ok().and_then(|i| a.get(i)).unwrap_or(&NULL),
            _ => &NULL,
        };
    }
    cur
}
pub fn vs(v: &Value, path: &str) -> String {
    match at(v, path) {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}
pub fn vb(v: &Value, path: &str) -> bool {
    match at(v, path) {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => s == "true" || s == "on" || s == "yes",
        _ => false,
    }
}
pub fn vf(v: &Value, path: &str, d: f64) -> f64 {
    match at(v, path) {
        Value::Number(n) => n.as_f64().unwrap_or(d),
        Value::String(s) => s.trim().parse().unwrap_or(d),
        _ => d,
    }
}
pub fn va<'v>(v: &'v Value, path: &str) -> &'v [Value] {
    at(v, path).as_array().map(|a| a.as_slice()).unwrap_or(&[])
}
/// One key as it is (keys with dots in them: PipeWire's "application.name", the KDE reads' "app.<id>.ShowPopups").
pub fn kg(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}
pub fn present(v: &Value, path: &str) -> bool {
    !at(v, path).is_null()
}

// ------------------------------------------------------------------ the programs

/// Is this KDE Plasma (the Fedora edition's desktop)? Its own settings are then the ones that act.
pub fn plasma() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_uppercase().contains("KDE")).unwrap_or(false)
}

pub fn sv(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}
/// One of Mantel's helpers next to its shell (hearth-screens, hearth-sound…), as you.
pub fn py(helper: &str, args: &[&str]) -> Vec<String> {
    let mut c = vec!["python3".to_string(), run::shell_helper(helper)];
    c.extend(args.iter().map(|a| a.to_string()));
    c
}
pub fn py_s(helper: &str, args: Vec<String>) -> Vec<String> {
    let mut c = vec!["python3".to_string(), run::shell_helper(helper)];
    c.extend(args);
    c
}
pub const LIBEXEC: &str = "/usr/libexec/nimbus/";
/// One of Hearth's helpers in /usr/libexec/nimbus, as you (Mantel's SetJob runs these with python3 too).
pub fn lx(name: &str, args: &[&str]) -> Vec<String> {
    let mut c = vec!["python3".to_string(), format!("{LIBEXEC}{name}")];
    c.extend(args.iter().map(|a| a.to_string()));
    c
}
/// The same helper with the system's permission (pkexec; Hearth's polkit rule lets the person at the PC do it).
pub fn adm(name: &str, args: Vec<String>) -> Vec<String> {
    let mut c = vec!["pkexec".to_string(), format!("{LIBEXEC}{name}")];
    c.extend(args);
    c
}
pub fn shc(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}
/// A KDE setting, written by KDE's own tool (KConfig: atomic, its lock), with KDE's "config changed" signal so the
/// running Plasma takes it at once. `groups` nest ([AC][Display]).
pub fn kw(file: &str, groups: &[&str], key: &str, value: &str) -> Vec<String> {
    let mut c = sv(&["kwriteconfig6", "--notify", "--file", file]);
    for g in groups {
        c.push("--group".into());
        c.push(g.to_string());
    }
    c.extend(sv(&["--key", key, value]));
    c
}
pub fn kdel(file: &str, groups: &[&str], key: &str) -> Vec<String> {
    let mut c = sv(&["kwriteconfig6", "--notify", "--file", file]);
    for g in groups {
        c.push("--group".into());
        c.push(g.to_string());
    }
    c.extend(sv(&["--key", key, "--delete"]));
    c
}
/// A line of a read: NAME=value from kreadconfig6 (for reads that gather KDE's settings).
pub fn kr(name: &str, file: &str, groups: &[&str], key: &str) -> String {
    let g: String = groups.iter().map(|g| format!(" --group {}", data::q(g))).collect();
    format!("printf '%s=%s\\n' {} \"$(kreadconfig6 --file {}{g} --key {} 2>/dev/null)\"; ", data::q(name), data::q(file), data::q(key))
}
pub fn dbus(dest: &str, path: &str, method: &str, args: &[&str]) -> Vec<String> {
    let mut c = sv(&["dbus-send", "--session", "--type=method_call", "--print-reply", &format!("--dest={dest}"), path, method]);
    c.extend(args.iter().map(|a| a.to_string()));
    c
}
pub fn nimbus_file(name: &str) -> std::path::PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config"));
    cfg.join("nimbus").join(name)
}

/// What each read runs (its key names it in State.v and in the fixture folder).
fn reader(key: &'static str) -> Vec<String> {
    match key {
        // Display (hearth-screens: kscreen-doctor and KWin's D-Bus on Plasma, the same as Mantel on KWin)
        "screens" => py("hearth-screens", &["info"]),
        "ddc" => py("hearth-screens", &["ddc"]),
        "gm" => py("hearth-gamemode-display", &["get"]),
        "psr" => lx("hearth-screen-psr", &["status"]),
        "bright" => shc("brightnessctl -m -c backlight 2>/dev/null | head -1"),
        "nvidia" => shc("test -d /sys/module/nvidia && { cat \"${XDG_CONFIG_HOME:-$HOME/.config}/nimbus/nvidia-drawing\" 2>/dev/null || echo auto; }"),
        // Sound
        "snd" => data::sh(data::SOUND),
        "cards" => py("hearth-sound", &["cards"]),
        "outputs" => py("hearth-sound", &["outputs"]),
        "streams" => shc("echo '#@play'; pactl -f json list sink-inputs 2>/dev/null; echo; echo '#@rec'; pactl -f json list source-outputs 2>/dev/null; echo; echo '#@sinks'; pactl -f json list sinks 2>/dev/null; echo; echo '#@sources'; pactl -f json list sources 2>/dev/null; echo"),
        "k-sound" => shc(&format!("{}{}{}", kr("overamp", "plasmaparc", &["General"], "RaiseMaximumVolume"), kr("theme", "kdeglobals", &["Sounds"], "Theme"), kr("enable", "kdeglobals", &["Sounds"], "Enable"))),
        "fdo" => shc("test -d /usr/share/sounds/freedesktop/stereo && echo yes"),
        // Notifications
        "notify" => py("hearth-notify", &["get"]),
        "desk" => py("hearth-desk", &["state"]),
        "k-notify" => shc(&format!(
            "{}{}{}{}{}printf 'now=%s\\n' \"$(date +%Y,%m,%d,%H,%M,%S)\"; f=\"${{XDG_CONFIG_HOME:-$HOME/.config}}/plasmanotifyrc\"; grep -o '^\\[Applications\\]\\[[^]]*\\]' \"$f\" 2>/dev/null | sed 's/^\\[Applications\\]\\[//; s/\\]$//' | sort -u | while read -r g; do for k in ShowPopups ShowInHistory ShowPopupsInDndMode; do printf 'app.%s.%s=%s\\n' \"$g\" \"$k\" \"$(kreadconfig6 --file plasmanotifyrc --group Applications --group \"$g\" --key $k 2>/dev/null)\"; done; done",
            kr("until", "plasmanotifyrc", &["DoNotDisturb"], "Until"),
            kr("fullscreen", "plasmanotifyrc", &["DoNotDisturb"], "WhenFullscreen"),
            kr("sharing", "plasmanotifyrc", &["DoNotDisturb"], "WhenScreenSharing"),
            kr("mirrored", "plasmanotifyrc", &["DoNotDisturb"], "WhenScreensMirrored"),
            kr("sounds", "plasmanotifyrc", &["DoNotDisturb"], "NotificationSoundsMuted")
        )),
        // Power
        "power" => py("hearth-power", &["status"]),
        "hw" => lx("hearth-hwset", &["status"]),
        "pset" => py("hearth-powerset", &["status"]),
        "canoff" => shc(&format!("sh {} check >/dev/null 2>&1 && echo yes || echo no", data::q(&run::shell_helper("hearth-screen-power")))),
        "k-power" => shc(&power::kde_read()),
        "upower" => shc("upower --dump 2>/dev/null"),
        "hist-charge" | "hist-rate" => Vec::new(),
        // Memory, storage, drives
        "memory" => lx("hearth-memory", &["status"]),
        "disk" => sv(&["nimbus", "--settings-call", "diskspace.status", "{}"]),
        "df" => data::sh(data::STORAGE),
        "prefs" => py("hearth-files", &["prefs"]),
        "thispc" => py("hearth-files", &["this-pc"]),
        // Your desktop, Startup, Boot splash, Remote desktop
        "deskchoice" => data::sh(data::DESK),
        "server" => py("hearth-server-mode", &["status"]),
        "k-session" => shc(&format!("{}{}", kr("login", "ksmserverrc", &["General"], "loginMode"), kr("confirm", "ksmserverrc", &["General"], "confirmLogout"))),
        "boot" => lx("hearth-boot-look", &["status"]),
        "vnc" => py("hearth-vnc", &["status"]),
        // Date & time, Region & language
        "time" => py("hearth-system", &["time"]),
        "zones" => py("hearth-system", &["timezones"]),
        "k-clock" => shc("grep -h -A40 'plugin=org.kde.plasma.digitalclock' \"${XDG_CONFIG_HOME:-$HOME/.config}/plasma-org.kde.plasma.desktop-appletsrc\" 2>/dev/null | grep -m1 '^use24hFormat=' | sed 's/^use24hFormat=/clock=/'"),
        "region" => py("hearth-region", &["status"]),
        "languages" => py("hearth-region", &["languages"]),
        "spell" => py("hearth-region", &["spell"]),
        "langpacks" => py("hearth-langpacks", &["status"]),
        "im" => py("hearth-region", &["im"]),
        // Clipboard, Search
        "clip" => py("hearth-clipboard", &["status"]),
        "k-clip" => shc(&format!(
            "{}{}{}{}",
            kr("max", "klipperrc", &["General"], "MaxClipItems"),
            kr("keep", "klipperrc", &["General"], "KeepClipboardContents"),
            kr("ignoresel", "klipperrc", &["General"], "IgnoreSelection"),
            kr("sync", "klipperrc", &["General"], "SyncClipboards")
        )),
        "baloo" => shc("balooctl6 status 2>/dev/null || balooctl status 2>/dev/null"),
        "k-search" => shc(&lang::kde_search_read()),
        "hsearch" => py("hearth-search", &["status"]),
        // About, Info Center
        "about" => shc(about::ABOUT),
        "updates" => vec!["pkexec".into(), data::ADMIN.into(), "update-status".into()],
        "info-cpu" => py("hearth-info", &["cpu"]),
        "info-gpu" => py("hearth-info", &["gpu"]),
        "info-memory" => py("hearth-info", &["memory"]),
        "info-graphics" => py("hearth-info", &["graphics"]),
        "info-pci" => py("hearth-info", &["pci"]),
        "info-usb" => py("hearth-info", &["usb"]),
        "info-sensors" => py("hearth-info", &["sensors"]),
        "info-modules" => py("hearth-info", &["modules"]),
        "info-interrupts" => py("hearth-info", &["interrupts"]),
        "live" => shc("test -e /run/hearth-live || grep -qw 'rd.live.image\\|boot=live' /proc/cmdline && echo live"),
        _ => Vec::new(),
    }
}

/// The reads a page does as it opens (only reads).
fn reads_of(page: &str) -> &'static [&'static str] {
    match page {
        "display" => &["screens", "gm", "psr", "bright", "nvidia", "ddc"],
        "sound" => &["snd", "cards", "streams", "k-sound"],
        "sound-speakers" => &["outputs"],
        "sound-apps" => &["streams"],
        "sounds" => &["k-sound", "fdo"],
        "notifications" => &["notify", "desk", "k-notify"],
        "power" => &["power", "hw", "pset", "canoff", "k-power"],
        "power-profiles" => &["pset", "hw", "k-power", "power"],
        "power-awake" => &["pset"],
        "battery" | "info-energy" => &["pset", "upower"],
        "memory" => &["memory"],
        "storage" => &["disk", "df"],
        "drives" => &["prefs", "thispc"],
        "your-desktop" => &["deskchoice"],
        "startup" => &["server", "k-session"],
        "boot" => &["boot"],
        "remote-desktop" => &["vnc"],
        "datetime" => &["time", "zones", "k-clock"],
        "region" => &["region"],
        "languages" => &["languages", "region"],
        "spellcheck" => &["spell"],
        "language-packs" => &["langpacks"],
        "input-methods" => &["im"],
        "clipboard" => &["clip", "k-clip"],
        "search" => &["baloo", "k-search", "hsearch"],
        "about" => &["about", "updates"],
        "info-cpu" => &["info-cpu"],
        "info-gpu" => &["info-gpu"],
        "info-memory" => &["info-memory"],
        "info-graphics" => &["info-graphics"],
        "info-pci" => &["info-pci"],
        "info-usb" => &["info-usb"],
        "info-sensors" => &["info-sensors"],
        "info-modules" => &["info-modules"],
        "info-interrupts" => &["info-interrupts"],
        "install" => &["live"],
        _ => &[],
    }
}

/// Runs the programs of a change in turn, each with its own time limit, on its own thread: text for the first one on
/// its stdin and in HEARTH_INPUT (never on a command line). Stops at the first that fails.
pub fn job(cmds: Vec<Vec<String>>, input: String, secs: u64) -> impl std::future::Future<Output = run::Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("settings-system".into()).spawn(move || {
        let mut all = run::Output { ok: true, code: 0, out: String::new(), err: String::new() };
        for (i, cmd) in cmds.iter().enumerate() {
            let o = one(cmd, if i == 0 { &input } else { "" }, secs.max(1));
            all.out.push_str(&o.out);
            all.err.push_str(&o.err);
            all.code = o.code;
            if !o.ok {
                all.ok = false;
                break;
            }
        }
        let _ = tx.send(all);
    });
    async move { rx.await.unwrap_or(run::Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}

fn one(cmd: &[String], input: &str, secs: u64) -> run::Output {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let fail = |e: String| run::Output { ok: false, code: -1, out: String::new(), err: e };
    let Some((prog, args)) = cmd.split_first() else { return fail("nothing to run".into()) };
    let child = Command::new(prog)
        .args(args)
        .env("HEARTH_INPUT", input)
        .stdin(if input.is_empty() { Stdio::null() } else { Stdio::piped() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return fail(format!("{prog} isn't on this system ({e})")),
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(input.as_bytes());
    }
    let pid = child.id() as i32;
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new().name("settings-system-wait".into()).spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(Ok(o)) => run::Output {
            ok: o.status.success(),
            code: o.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&o.stdout).to_string(),
            err: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Ok(Err(e)) => fail(e.to_string()),
        Err(_) => {
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
            fail(format!("{} took too long, so it was stopped", prog.rsplit('/').next().unwrap_or(prog)))
        }
    }
}

fn fixture(key: &str) -> Option<String> {
    let dir = std::env::var_os("HEARTH_SYSTEM_FIXTURE")?;
    std::fs::read_to_string(std::path::Path::new(&dir).join(key)).ok()
}

pub fn read(key: &'static str) -> Task<Msg> {
    if let Some(text) = fixture(key) {
        return Task::done(Msg::Got(key, run::Output { ok: true, code: 0, out: text, err: String::new() }));
    }
    let cmd = reader(key);
    if cmd.is_empty() {
        return Task::none();
    }
    let secs = if key == "ddc" { 40 } else { 25 };
    Task::perform(job(vec![cmd], String::new(), secs), move |o| Msg::Got(key, o))
}

/// A read whose program depends on the page (a battery's history): kept like any read, from the fixture in CI.
pub fn read_cmd(key: &'static str, cmd: Vec<String>) -> Task<Msg> {
    if let Some(text) = fixture(key) {
        return Task::done(Msg::Got(key, run::Output { ok: true, code: 0, out: text, err: String::new() }));
    }
    Task::perform(job(vec![cmd], String::new(), 25), move |o| Msg::Got(key, o))
}

pub fn exec(ex: Ex) -> Task<Msg> {
    if ex.cmds.is_empty() {
        return Task::done(Msg::Done(ex, run::Output { ok: true, code: 0, out: String::new(), err: String::new() }));
    }
    let (cmds, input, secs) = (ex.cmds.clone(), ex.input.clone(), ex.secs.max(5));
    Task::perform(job(cmds, input, secs), move |o| Msg::Done(ex.clone(), o))
}

/// Hearth's small settings files a page shows (read here, written only through hearth_settings_io).
fn files_of(page: &str) -> &'static [&'static str] {
    match page {
        "sound" | "power" => &["buttons.json"],
        "sounds" => &["sounds.json"],
        "notifications" => &["notify.json"],
        "datetime" => &["bar.json"],
        "clipboard" => &["clipboard.json"],
        _ => &[],
    }
}

/// "NAME=value" lines (the KDE reads) as a JSON object of strings.
fn kv(text: &str) -> Value {
    let mut m = serde_json::Map::new();
    for l in text.lines() {
        if let Some((k, v)) = l.split_once('=') {
            m.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
        }
    }
    Value::Object(m)
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.page = page;
    st.open.clear();
    st.desk_saved = false;
    st.rename = None;
    st.settle = None;
    st.boot_pv = None;
    st.flip.halt();
    st.sched = None;
    // `hearth-settings boot:preview-<id>` opens that splash's preview once the list is read.
    if page == "boot" {
        if let Some(id) = sh.part.strip_prefix("preview-") {
            st.text.insert("bootwant", id.to_string());
        }
    }
    st.start = data::start_mode();
    sh.desktop = crate::shared::read_desktop();
    for f in files_of(page) {
        st.files.insert(f, hearth_ui::settings_file::read_json(&nimbus_file(f)));
    }
    if page == "battery" || page == "info-energy" {
        st.num.entry("hours").or_insert(24.0);
    }
    // CI's pictures of a page's other states: HEARTH_SYSTEM_SHOW=keep (the display change waiting), open:KEY (a
    // chooser's list), flag:KEY, text:KEY=VALUE.
    if let Ok(show) = std::env::var("HEARTH_SYSTEM_SHOW") {
        let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };
        for part in show.split(',').filter(|x| !x.is_empty()) {
            if part == "keep" {
                st.keep = Some(Keep { prev: serde_json::json!({"outputs": {}}), names: vec![], left: 15 });
            } else if let Some(k) = part.strip_prefix("open:") {
                st.open = k.to_string();
            } else if let Some(k) = part.strip_prefix("flag:") {
                st.flag.insert(leak(k), true);
            } else if let Some((k, v)) = part.strip_prefix("text:").and_then(|x| x.split_once('=')) {
                st.text.insert(leak(k), v.to_string());
            }
        }
    }
    Task::batch(reads_of(page).iter().map(|k| read(k)))
}

/// After a read: the pages that read more once they know something (a battery's history once its path is known).
fn after_read(st: &mut State, key: &'static str) -> Task<Msg> {
    match key {
        "pset" if st.page == "battery" || st.page == "info-energy" => power::history(st),
        "boot" => {
            if let Some(want) = st.text.remove("bootwant") {
                st.boot_pv = va(st.j("boot"), "splash.presets").iter().find(|x| vs(x, "id") == want).cloned();
            }
            more::boot_posters(st)
        }
        _ => Task::none(),
    }
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Arrange(_) => return Task::none(),
        Msg::Got(key, o) => {
            let val = if key.starts_with("k-") { kv(&o.out) } else if o.ok { o.json().unwrap_or(Value::Null) } else { Value::Null };
            match key {
                "deskchoice" => st.desk = data::desk(&o.out),
                "updates" => {
                    if let Some(v) = o.json().filter(|_| o.ok) {
                        st.updates = data::updates(&v);
                    }
                }
                _ => {}
            }
            st.read_ok.insert(key, o.ok);
            st.raw.insert(key, if o.ok { o.out.clone() } else { o.why() });
            st.v.insert(key, val);
            if key == "screens" {
                display::took(st);
            }
            if key == "server" && !matches!(st.settle, Some(("sched", _))) && st.busy.is_empty() {
                let v = st.j("server");
                st.sched = Some(serde_json::json!({"weekday": at(v, "settings.schedule.weekday"), "weekend": at(v, "settings.schedule.weekend")}));
            }
            return after_read(st, key);
        }
        Msg::Exec(ex) => {
            if !ex.input.is_empty() {
                // A password typed for a change goes to the helper's stdin only; the field is emptied.
                st.text.remove("vncpw");
                st.flag.remove("pwopen");
            }
            sh.error.clear();
            sh.note.clear();
            st.busy = "Working on it…".into();
            return exec(ex);
        }
        Msg::Done(ex, o) => {
            st.busy.clear();
            if !o.ok {
                let why = o.why();
                let what = if ex.fail.is_empty() { "That didn't work" } else { ex.fail.as_str() };
                sh.error = if why.is_empty() { what.to_string() } else { format!("{what}: {why}") };
            } else if !ex.ok.is_empty() {
                sh.note = ex.ok.clone();
            }
            if let Some(k) = ex.store {
                if let Some(v) = o.json() {
                    st.v.insert(k, v);
                    if k == "screens" {
                        display::took(st);
                    }
                }
            }
            return Task::batch(ex.re.iter().map(|k| read(k)));
        }
        Msg::Desk(v, ex) => {
            let mut ex = ex;
            ex.cmds.insert(0, sh.desktop_cmd(v));
            return update(sh, st, Msg::Exec(ex));
        }
        Msg::Json(file, v, ex) => {
            let mut ex = ex;
            ex.cmds.insert(0, sh.json_cmd(&nimbus_file(file), v.clone()));
            // The page shows the change at once (the file is merged the same way).
            let cur = st.files.entry(file).or_insert(Value::Object(Default::default()));
            if let (Value::Object(c), Value::Object(n)) = (cur, &v) {
                for (k, x) in n {
                    c.insert(k.clone(), x.clone());
                }
            }
            return update(sh, st, Msg::Exec(ex));
        }
        Msg::Text(k, s) => {
            st.text.insert(k, s);
        }
        Msg::Flag(k, b) => {
            st.flag.insert(k, b);
        }
        Msg::Num(k, v) => {
            st.num.insert(k, v);
            if k == "hours" {
                return power::history(st);
            }
        }
        Msg::NumOf(k, v) => {
            st.numof.insert(k.clone(), v);
            st.text.insert("numof", k);
        }
        Msg::NumDone(k) => return slider_done(sh, st, k),
        Msg::Open(k) => st.open = if st.open == k { String::new() } else { k },
        Msg::Go(id) => sh.go(id),
        Msg::App(cmd) => {
            let c: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
            run::detached(&c);
        }
        Msg::Reread(k) => return read(k),
        Msg::Draft(target, path, v) => {
            if path.is_empty() {
                st.v.remove(target);
            } else {
                let d = st.v.entry(target).or_insert_with(|| Value::Object(Default::default()));
                if !d.is_object() {
                    *d = Value::Object(Default::default());
                }
                // Memory's changes are "group.key"; other drafts (what's picked to clean up) keep the key whole.
                let parts: Vec<&str> = if target == "memdraft" { path.splitn(2, '.').collect() } else { vec![path.as_str()] };
                if parts.len() == 2 {
                    if !d[parts[0]].is_object() {
                        d[parts[0]] = Value::Object(Default::default());
                    }
                    d[parts[0]][parts[1]] = v;
                } else {
                    d[parts[0]] = v;
                }
            }
        }
        Msg::Seq(list) => {
            let mut t = Vec::new();
            for m in list {
                t.push(update(sh, st, m));
            }
            return Task::batch(t);
        }
        Msg::Pick(n) => {
            // A change waiting for Keep stays waiting (it still goes back by itself unless it's kept).
            st.picked = n;
            display::took(st);
        }
        Msg::Try(changes) => return display::try_change(sh, st, changes),
        Msg::SetNow(changes, label) => return display::set_now(st, changes, label),
        Msg::Keep => return display::keep(st),
        Msg::Revert => return display::revert(sh, st, false),
        Msg::Tick => {
            let done = match st.keep.as_mut() {
                Some(k) => {
                    k.left = k.left.saturating_sub(1);
                    k.left == 0
                }
                None => false,
            };
            if done {
                return display::revert(sh, st, true);
            }
        }
        Msg::Settle => {
            if let Some((k, n)) = st.settle {
                if n <= 1 {
                    st.settle = None;
                    return slider_save(sh, st, k);
                }
                st.settle = Some((k, n - 1));
            }
        }
        Msg::Live => return Task::batch(live_reads(st.page).iter().map(|k| read(k))),
        Msg::Rename => st.rename = Some(sh.info.host.clone()),
        Msg::RenameText(t) => st.rename = Some(data::host_chars(&t)),
        Msg::RenameSave => {
            if let Some(n) = st.rename.take().filter(|n| !n.is_empty()) {
                sh.info.host = n.clone();
                return exec(Ex::new(vec!["pkexec".into(), data::ADMIN.into(), "hostname".into(), n]).fail("Couldn't rename this PC").ok("Renamed").re("about"));
            }
        }
        Msg::RenameCancel => st.rename = None,
        Msg::BootPreview(v) => {
            if v.is_none() {
                st.flip.halt();
            }
            st.boot_pv = v;
        }
        Msg::BootPics => {}
        Msg::BootTick => {
            let a = more::boot_active(st);
            let g = crate::bootpics::geo(st.j("boot"));
            return match st.flip.tick(a.as_ref(), g) {
                Some(f) => Task::perform(f, |_| Msg::BootPics),
                None => Task::none(),
            };
        }
        Msg::BootHover(path) => st.flip.hover(path),
        Msg::BootUnhover(path) => {
            st.flip.unhover(&path);
            if more::boot_active(st).is_none() {
                st.flip.halt();
            }
        }
        Msg::Sched(day, i, op) => sched_edit(st, day, i, op),
        Msg::SchedSave => return sched_save(sh, st),
        Msg::Desktop(pick) => {
            st.desk_saved = true;
            st.desk.chosen = pick.to_string();
            return exec(Ex::new(data::desk_cmd(&st.desk, pick)).fail("Couldn't save the desktop").re("deskchoice"));
        }
        Msg::SwitchNow => run::detached(&["nimbus-mode", "desktop"]),
        Msg::StartIn(mode) => {
            st.start = mode.to_string();
            let cmd = sh.conf_cmd(&data::session_conf(), "NIMBUS_START_MODE", mode);
            return exec(Ex::new(cmd).fail("Couldn't save Start in").re("server"));
        }
    }
    Task::none()
}

/// A slider was let go: what it saves waits a moment (Mantel's pages wait too), so a quick second move is one save.
fn slider_done(sh: &mut Shared, st: &mut State, k: &'static str) -> Task<Msg> {
    match k {
        // Saved at once: one program each.
        "vol" | "mic" | "bright" | "balance" => slider_save(sh, st, k),
        _ => {
            st.settle = Some((k, 1));
            Task::none()
        }
    }
}

fn slider_save(sh: &mut Shared, st: &mut State, k: &'static str) -> Task<Msg> {
    match k {
        "vol" | "mic" | "balance" | "appvol" | "recvol" => sound::slider_save(sh, st, k),
        "bright" | "autooff" | "sdr" | "gamut" | "overscan" | "barrier" | "nighttemp" | "ext" => display::slider_save(sh, st, k),
        "tdp" => power::slider_save(sh, st, k),
        "sched" => sched_save(sh, st),
        _ => Task::none(),
    }
}

/// Pages that change by themselves while they're open (Mantel's refresh their reads too): what they read again.
fn live_reads(page: &str) -> &'static [&'static str] {
    match page {
        "power-awake" => &["pset"],
        "datetime" => &["time"],
        "info-sensors" => &["info-sensors"],
        "info-interrupts" => &["info-interrupts"],
        "remote-desktop" => &["vnc"],
        "memory" => &["memory"],
        _ => &[],
    }
}
fn live_every(st: &State) -> u64 {
    match st.page {
        "power-awake" => 6,
        "datetime" => 20,
        "info-sensors" | "info-interrupts" => 3,
        "remote-desktop" if vb(st.j("vnc"), "on") => 5,
        "memory" if st.busy.is_empty() => 5,
        _ => 0,
    }
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if !PAGES.contains(&page) {
        return None;
    }
    Some(match page {
        "display" => display::view(a),
        "sound" | "sound-speakers" | "sound-apps" | "push-to-talk" | "sounds" => sound::view(a, page),
        "info-energy" => column![about::tabs(&a.sh.p, page), power::view(a, page)].spacing(10.0).into(),
        "power" | "power-profiles" | "power-awake" | "battery" => power::view(a, page),
        "notifications" | "memory" | "storage" | "drives" | "your-desktop" | "boot" | "remote-desktop" => more::view(a, page),
        "startup" => startup(a),
        "datetime" | "region" | "languages" | "spellcheck" | "language-packs" | "input-methods" | "clipboard" | "search" => lang::view(a, page),
        _ => about::view(a, page),
    })
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    let mut s = Vec::new();
    if st.keep.is_some() {
        s.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Tick));
    }
    if st.settle.is_some() {
        s.push(iced::time::every(std::time::Duration::from_millis(600)).map(|_| Msg::Settle));
    }
    // Boot splash: a picture plays (its preview, or the card under the mouse or the focus), only while the page shows.
    if st.page == "boot" && BOOT_SHOWN.load(std::sync::atomic::Ordering::Relaxed) && more::boot_active(st).is_some() {
        s.push(iced::time::every(crate::bootpics::every(crate::bootpics::geo(st.j("boot")))).map(|_| Msg::BootTick));
    }
    let n = live_every(st);
    if n > 0 && PAGES.contains(&st.page) {
        s.push(iced::time::every(std::time::Duration::from_secs(n)).map(|_| Msg::Live));
    }
    Subscription::batch(s)
}

/// Boot splash is the page showing (set while the window is drawn): its pictures play only then.
static BOOT_SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    BOOT_SHOWN.store(a.page == "boot", std::sync::atomic::Ordering::Relaxed);
    if a.page == "boot" {
        return more::boot_preview(a);
    }
    None
}

pub fn back(st: &mut State) -> bool {
    if st.boot_pv.take().is_some() {
        st.flip.halt();
        return true;
    }
    if !st.open.is_empty() {
        st.open.clear();
        return true;
    }
    st.rename.take().is_some()
}

// ------------------------------------------------------------------ pieces the pages share

pub type El<'a> = Element<'a, Msg>;

/// Choices side by side (Mantel's SetChoice), each with what picking it sends.
pub fn choice<'a>(p: &Pal, items: Vec<(String, bool, Msg)>) -> El<'a> {
    seg(p, items.into_iter().map(|(l, on, m)| (l, on, Some(m))).collect())
}

/// A switch that sends `m` when pressed.
pub fn sw<'a>(p: &Pal, on: bool, m: Msg) -> El<'a> {
    switch(p, on, Some(m))
}

/// A row that only says something (no control).
pub fn info_row<'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>) -> El<'a> {
    srow(p, g, title, sub, iced::widget::Space::new().width(Length::Fixed(0.0)).into())
}

/// A row with a value on the right (In use, 82%…).
pub fn value_row<'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, value: impl Into<String>, accent: bool) -> El<'a> {
    srow(p, g, title, sub, t1(value.into(), sz(13.0), if accent { p.accent } else { p.muted }).into())
}

/// A row you press (no chevron): the button's words on the right.
pub fn act_row<'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, label: impl Into<String>, m: Option<Msg>) -> El<'a> {
    srow(p, g, title, sub, pill(p, label.into(), true, m))
}

/// Mantel's SetDropdown, in Settings' look: the row shows what's picked and Change; pressing it lists the choices
/// under it (with a field to narrow a long list), and picking one sends `on(id)`.
pub fn chooser<'a>(a: &'a App, key: &str, g: G, title: impl Into<String>, sub: impl Into<String>, options: Vec<(String, String)>, current: &str, on: impl Fn(String) -> Msg + 'a) -> El<'a> {
    let p = a.sh.p;
    let st = &a.system;
    let cur = options.iter().find(|o| o.0 == current).map(|o| o.1.clone()).unwrap_or_else(|| if current.is_empty() { "Pick one".into() } else { current.to_string() });
    let open = st.open == key;
    let head = srow(&p, g, title, sub, row![t1(cur, sz(13.0), p.text), pill(&p, if open { "Close" } else { "Change" }, true, Some(Msg::Open(key.to_string())))].spacing(12.0).align_y(iced::Center).into());
    if !open {
        return head;
    }
    let mut c = Column::new().spacing(6.0).push(head);
    let q = st.txt("chooser").to_lowercase();
    if options.len() > 12 {
        c = c.push(field(&p, "Find one", st.txt("chooser"), |s| Msg::Text("chooser", s)).width(Length::Fixed(sz(320.0))));
    }
    let mut n = 0;
    for (id, label) in options.iter() {
        if !q.is_empty() && !label.to_lowercase().contains(&q) && !id.to_lowercase().contains(&q) {
            continue;
        }
        let mark = if id == current { "  ✓" } else { "" };
        c = c.push(pick_row(&p, format!("{label}{mark}"), on(id.clone())));
        n += 1;
        if n >= 40 {
            break;
        }
    }
    c.into()
}

/// A short line under the rows (Mantel's small muted Text).
pub fn line<'a>(p: &Pal, s: impl Into<String>) -> El<'a> {
    note(p, s, false)
}

/// "Reading…" or why a read failed, for a page whose read hasn't come back with anything to show.
pub fn waiting<'a>(a: &'a App, key: &str, what: &str) -> Option<El<'a>> {
    let st = &a.system;
    let p = a.sh.p;
    if st.has(key) {
        return None;
    }
    Some(if !st.read(key) {
        note(&p, format!("Reading {what}…"), false)
    } else {
        note(&p, format!("{} couldn't be read{}", cap(what), if st.raw(key).is_empty() { ".".to_string() } else { format!(": {}", st.raw(key)) }), true)
    })
}

pub fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Options for a choice of minutes ([0, 2, 5]) as words.
pub fn minutes_word(m: i64) -> String {
    match m {
        -1 => "As plugged in".into(),
        0 => "Never".into(),
        60 => "1 hour".into(),
        120 => "2 hours".into(),
        m => format!("{m} min"),
    }
}

// ------------------------------------------------------------------ Startup (SetStartup.qml)

/// Startup: Start in (Game Mode, Desktop Mode, Server mode once it's on), Server mode's schedule, reopening your apps
/// and leaving. Start in and the schedule through hearth-server-mode (session.conf's NIMBUS_START_MODE, only that
/// line); on Plasma the session's own (ksmserverrc: restore the last session, confirm logging out) too.
fn startup(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let srv = st.j("server");
    let server_on = vb(srv, "settings.on");
    let scheduled = server_on && vb(srv, "settings.schedule.on");
    let start_in = if st.has("server") { vs(srv, "startIn") } else { st.start.clone() };
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    c = c.push(t(format!("What this PC opens when it turns on{}", if server_on { ", and when it changes mode by itself." } else { "." }), sz(14.0), p.muted));
    // Game Mode on this PC (the owner, Nov 19: "need to add a option that turns off the game modes"): desktop.json's
    // gameModeOff through hearth_settings_io (merge, .bak, the change history); missing is On. nimbus-session reads it
    // at every start (Off: Desktop Mode), nimbus-mode refuses Game Mode while it's off, and `nimbus-mode gamemodes
    // apply` hides or shows Plasma's Back to Game Mode (menu, taskbar, Super+G) at once.
    let gm_off = game_modes_off(&a.sh);
    c = c.push(head(&p, "Game Mode on this PC"));
    c = c.push(srow(&p, G::Pad, "Game Mode on this PC", if gm_off { "Off: this PC always starts in Desktop Mode. Back to Game Mode, Super+G and the Guide menu's Game Mode are hidden" } else { "On: Start in, Back to Game Mode, Super+G and the Guide menu's Game Mode can open Game Mode" }, choice(&p, vec![
        ("On".into(), !gm_off, Msg::Desk(serde_json::json!({"gameModeOff": false}), Ex::none().and(gm_apply_cmd()).ok("Game Mode is on ✓"))),
        ("Off".into(), gm_off, Msg::Desk(serde_json::json!({"gameModeOff": true}), Ex::none().and(gm_apply_cmd()).ok("Game Mode is off on this PC"))),
    ])));
    c = c.push(line(&p, "Game Mode stays available for recovery: if Desktop Mode can't start, Hearth still opens Game Mode so you always get a working screen. First-time setup runs as usual."));
    if server_on {
        c = c.push(srow(&p, G::Power, "When this PC turns on", "Start in one mode every time, or follow a schedule", choice(&p, vec![
            ("Always the same mode".into(), !scheduled, Msg::Exec(Ex::new(py("hearth-server-mode", &["set", "scheduleOn", "off"])).ok("Starting in the same mode every time").re("server"))),
            ("Follow a schedule".into(), scheduled, Msg::Exec(Ex::new(py("hearth-server-mode", &["set", "scheduleOn", "on"])).ok("Following the schedule ✓").re("server"))),
        ])));
    }
    c = c.push(head(&p, if scheduled { "Modes in the schedule" } else { "Start in" }));
    let modes = [
        ("game", G::Pad, "Game Mode", "Steam Game Mode or Hearth Game Mode (Gaming), for the TV and controllers"),
        ("desktop", G::Grid, "Desktop Mode", if plasma() { "KDE Plasma with windows, files and apps" } else { "The Mantel desktop with windows, files and apps" }),
        ("server", G::Server, "Server mode", "A dashboard and full power for your services; one press to play"),
    ];
    let mut cards = iced::widget::Row::new().spacing(12.0);
    for (id, g, name, sub) in modes {
        let usable = id != "server" || server_on;
        let sub = if id == "game" && gm_off { "Off on this PC (above): it starts in Desktop Mode until you turn Game Mode on" } else { sub };
        let chosen = !scheduled && start_in == id;
        let body = column![
            row![hearth_ui::ui::glyph(g, if chosen { p.accent } else { p.text }, sz(20.0), 1.8), hearth_ui::ui::ts(name, sz(16.0), p.text), t1(if chosen { "· Start in" } else { "" }, sz(14.0), p.accent)].spacing(8.0).align_y(iced::Center),
            t(if usable { sub.to_string() } else { "Off. Press to turn it on in Home server › Server mode".to_string() }, sz(13.0), p.muted)
        ]
        .spacing(8.0);
        let msg = if !usable { Msg::Go("hs-server") } else { Msg::Exec(Ex::new(py("hearth-server-mode", &["start-in", id])).ok(format!("Starts in {name} ✓")).re("server")) };
        cards = cards.push(
            iced::widget::mouse_area(iced::widget::container(body).padding(16.0).width(Length::Fill).style(hearth_ui::ui::boxed(p.panel, p.r(12.0), Some((if chosen { p.accent } else { p.line }, if chosen { 2.0 } else { 1.0 })))))
                .on_press(msg)
                .interaction(iced::mouse::Interaction::Pointer),
        );
    }
    c = c.push(cards);
    if scheduled {
        c = c.push(head(&p, "Schedule"));
        c = c.push(schedule(a));
    }
    c = c.push(head(&p, "When you sign in"));
    let k = st.j("k-session");
    // Plasma: the session's own restore (ksmserverrc loginMode: restorePreviousLogout / emptySession); Hearth's
    // restoreApps too (Mantel's SessionRestore.qml).
    let restore = if plasma() && st.has("k-session") { vs(k, "login") == "restorePreviousLogout" || (vs(k, "login").is_empty() && a.sh.d_bool("restoreApps", false)) } else { a.sh.d_bool("restoreApps", false) };
    let restore_kde = |on: bool| if plasma() { vec![kw("ksmserverrc", &["General"], "loginMode", if on { "restorePreviousLogout" } else { "emptySession" })] } else { vec![] };
    c = c.push(srow(&p, G::Update, "Reopen my apps from last time", "The apps open when you signed out or left Desktop Mode open again (not games or Steam)", sw(&p, restore, Msg::Desk(serde_json::json!({"restoreApps": !restore}), Ex::none().and_all(restore_kde(!restore)).re("k-session")))));
    c = c.push(head(&p, "When you leave"));
    let confirm = if plasma() && st.has("k-session") && !vs(k, "confirm").is_empty() { vs(k, "confirm") == "true" } else { a.sh.d_bool("confirmLogout", false) };
    let confirm_kde = |on: bool| if plasma() { vec![kw("ksmserverrc", &["General"], "confirmLogout", if on { "true" } else { "false" })] } else { vec![] };
    c = c.push(srow(&p, G::User, "Confirm before logging out", "Log out (in the app menu, Super+X and the desktop's menu) asks first, so a stray click can't close your apps", sw(&p, confirm, Msg::Desk(serde_json::json!({"confirmLogout": !confirm}), Ex::none().and_all(confirm_kde(!confirm)).re("k-session")))));
    let leave = a.sh.d_str("leaveDefault");
    let opts = [("", "Lock"), ("Sleep", "Sleep"), ("Restart", "Restart"), ("Shut down", "Shut down"), ("Log out", "Log out"), ("Game Mode", "Game Mode")];
    c = c.push(chooser(a, "leave", G::Power, "Default leave option", "What the power menu has picked when it opens: press the power button twice (or A) to do it", opts.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect(), &leave, |v| Msg::Desk(serde_json::json!({"leaveDefault": v}), Ex::none())));
    if scheduled {
        c = c.push(head(&p, "When it's time to change"));
        c = c.push(value_row(&p, G::Pad, "If a game is running", "Server mode never closes your game: the change waits until it ends", "Wait until the game ends", false));
        c = c.push(value_row(&p, G::Grid, "If you're in Desktop Mode", "Unsaved work stays open: the schedule never switches away from Desktop Mode", "Stay", false));
        c = c.push(value_row(&p, G::Server, "In Server mode", "The dashboard counts down 10 seconds first; B cancels", "10-second countdown", false));
    }
    if !st.has("server") && st.read("server") {
        c = c.push(line(&p, format!("Server mode's settings couldn't be read ({}); Start in still works.", st.raw("server"))));
    }
    c.into()
}

// ------------------------------------------------------------------ Startup's schedule (SetStartup.qml)

/// "18:30" → minutes after midnight.
pub fn sched_mins(at: &str) -> i32 {
    let (h, m) = at.split_once(':').unwrap_or(("0", "0"));
    h.trim().parse::<i32>().unwrap_or(0).clamp(0, 23) * 60 + m.trim().parse::<i32>().unwrap_or(0).clamp(0, 59)
}
/// Minutes → "18:30" (round the clock).
pub fn sched_at(m: i32) -> String {
    let m = m.rem_euclid(1440);
    format!("{:02}:{:02}", m / 60, m % 60)
}
/// "18:30" → "6:30 PM", "00:00" → "12 AM" (SetStartup.qml's label).
pub fn sched_label(at: &str) -> String {
    let m = sched_mins(at);
    let (h, mm) = (m / 60, m % 60);
    format!("{}{} {}", (h + 11) % 12 + 1, if mm > 0 { format!(":{mm:02}") } else { String::new() }, if h < 12 { "AM" } else { "PM" })
}
const SCHED_DAYS: [(&str, &str); 2] = [("weekday", "Monday – Friday"), ("weekend", "Saturday – Sunday")];

/// One change of the schedule, as SetStartup.qml makes it: the list sorted by time again, saved a moment after the
/// last change (several quick moves are one save). Never fewer than one change a day or more than eight.
pub fn sched_edit(st: &mut State, day: &'static str, i: usize, op: SchedOp) {
    let mut d = st.sched.clone().unwrap_or_else(|| serde_json::json!({"weekday": [], "weekend": []}));
    if !d[day].is_array() {
        d[day] = Value::Array(Vec::new());
    }
    let Some(list) = d[day].as_array_mut() else { return };
    let taken = |list: &[Value], m: i32, skip: usize| list.iter().enumerate().any(|(j, x)| j != skip && sched_mins(&vs(x, "at")) == m.rem_euclid(1440));
    match op {
        SchedOp::Move(step) => {
            if i >= list.len() {
                return;
            }
            let mut m = sched_mins(&vs(&list[i], "at"));
            for _ in 0..48 {
                m = (m + step * 30).rem_euclid(1440);
                if !taken(&list[..], m, i) {
                    break;
                }
            }
            list[i]["at"] = Value::String(sched_at(m));
        }
        SchedOp::Mode => {
            if i >= list.len() {
                return;
            }
            let order = ["server", "game", "desktop"];
            let cur = vs(&list[i], "mode");
            let n = order.iter().position(|x| *x == cur).map(|k| (k + 1) % 3).unwrap_or(1);
            list[i]["mode"] = Value::String(order[n].into());
        }
        SchedOp::Remove => {
            if i < list.len() && list.len() > 1 {
                list.remove(i);
            }
        }
        SchedOp::Add => {
            if list.len() >= 8 {
                return;
            }
            let last = list.last().map(|x| sched_mins(&vs(x, "at"))).unwrap_or(0);
            let mut m = (last + 60).min(1410);
            for _ in 0..48 {
                if !taken(&list[..], m, usize::MAX) {
                    break;
                }
                m = (m + 30).rem_euclid(1440);
            }
            list.push(serde_json::json!({"at": sched_at(m), "mode": "game"}));
        }
    }
    list.sort_by_key(|x| sched_mins(&vs(x, "at")));
    st.sched = Some(d);
    st.settle = Some(("sched", 2));
}

/// Saves the schedule as it is on the page (hearth-server-mode schedule-set, the schedule on its stdin: it checks it
/// and saves only what changed, through hearth_settings_io), then reads it again.
pub fn sched_save(sh: &mut Shared, st: &mut State) -> Task<Msg> {
    let Some(d) = st.sched.clone() else { return Task::none() };
    let body = serde_json::json!({"weekday": d.get("weekday").cloned().unwrap_or(Value::Null), "weekend": d.get("weekend").cloned().unwrap_or(Value::Null)}).to_string();
    update(sh, st, Msg::Exec(Ex::new(py("hearth-server-mode", &["schedule-set"])).input(body).ok("Schedule saved ✓").fail("Couldn't save the schedule").re("server").secs(30)))
}

/// The change the controller's focus is on (its row's name and line), for X: remove it.
pub fn sched_focused(st: &State, title: &str, sub: &str) -> Option<(&'static str, usize)> {
    let d = st.sched.as_ref()?;
    for (day, name) in SCHED_DAYS {
        if !sub.ends_with(name) {
            continue;
        }
        if let Some(i) = va(d, day).iter().position(|x| format!("From {}", sched_label(&vs(x, "at"))) == title) {
            return Some((day, i));
        }
    }
    None
}

fn mode_colour(p: &Pal, m: &str) -> iced::Color {
    match m {
        "desktop" => iced::Color::from_rgb(0.357, 0.82, 0.545),
        "server" => iced::Color::from_rgb(0.545, 0.612, 0.969),
        _ => p.accent,
    }
}

/// A small button on a change's row (the mouse's way to move or remove it).
fn sched_btn<'a>(p: &Pal, label: &'static str, m: Msg) -> El<'a> {
    iced::widget::mouse_area(
        iced::widget::container(t1(label, sz(13.0), p.muted))
            .padding(iced::Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 })
            .style(hearth_ui::ui::boxed(iced::Color::TRANSPARENT, 12.0, Some((p.line, 1.0)))),
    )
    .on_press(m)
    .interaction(iced::mouse::Interaction::Pointer)
    .into()
}

/// The schedule (SetStartup.qml): each day as a 24-hour track, a block per mode; each change on its own row (← → move
/// it 30 minutes, A changes its mode, X removes it; and with the mouse, its buttons); Add a change.
fn schedule(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let empty = serde_json::json!({});
    let d = st.sched.as_ref().unwrap_or(&empty);
    let mut c = Column::new().spacing(8.0);
    for (day, name) in SCHED_DAYS {
        let items = va(d, day);
        // The track: from midnight to the first change in the last change's mode (it carries over), then each.
        let mut track = iced::widget::Row::new().spacing(2.0).height(Length::Fixed(sz(30.0)));
        for k in 0..=items.len() {
            let from = if k == 0 { 0 } else { sched_mins(&vs(&items[k - 1], "at")) };
            let to = if k < items.len() { sched_mins(&vs(&items[k], "at")) } else { 1440 };
            if to <= from {
                continue;
            }
            let mode = if k == 0 { items.last().map(|x| vs(x, "mode")).unwrap_or_default() } else { vs(&items[k - 1], "mode") };
            let col = mode_colour(&p, &mode);
            let words = if to - from >= 240 { format!("{} · {} – {}", mode_name(&mode), sched_label(&sched_at(from)), sched_label(&sched_at(to))) } else { String::new() };
            track = track.push(
                iced::widget::container(t1(words, sz(12.0), p.text))
                    .padding(iced::Padding { top: 0.0, right: 4.0, bottom: 0.0, left: 8.0 })
                    .center_y(Length::Fill)
                    .width(Length::FillPortion((to - from) as u16))
                    .clip(true)
                    .style(hearth_ui::ui::boxed(iced::Color { a: 0.35, ..col }, 6.0, None)),
            );
        }
        c = c.push(card(&p, column![hearth_ui::ui::ts(name, sz(15.0), p.text), track].spacing(10.0)));
        for (i, x) in items.iter().enumerate() {
            let at = vs(x, "at");
            let mode = vs(x, "mode");
            let title = format!("From {}", sched_label(&at));
            let sub = format!("{} · {name}", mode_name(&mode));
            let act = |op: SchedOp| -> crate::nav::Act { Box::new(Msg::Sched(day, i, op)) };
            crate::nav::control(crate::nav::Kind::Choice, vec![act(SchedOp::Mode)], vec![act(SchedOp::Move(-1))], vec![act(SchedOp::Move(1))], String::new(), String::new());
            let nr = crate::nav::item(&title, &sub, false);
            let dot = iced::widget::container(iced::widget::Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(hearth_ui::ui::boxed(mode_colour(&p, &mode), 6.0, None));
            let mut line = iced::widget::Row::new()
                .spacing(12.0)
                .align_y(iced::Center)
                .push(dot)
                .push(hearth_ui::ui::ts(title, sz(15.0), p.text))
                .push(iced::widget::mouse_area(t1(mode_name(&mode), sz(15.0), p.muted)).on_press(Msg::Sched(day, i, SchedOp::Mode)).interaction(iced::mouse::Interaction::Pointer))
                .push(iced::widget::Space::new().width(Length::Fill))
                .push(t1("← → move · A mode · X remove", sz(12.0), p.muted))
                .push(sched_btn(&p, "‹", Msg::Sched(day, i, SchedOp::Move(-1))))
                .push(sched_btn(&p, "›", Msg::Sched(day, i, SchedOp::Move(1))));
            if items.len() > 1 {
                line = line.push(sched_btn(&p, "×", Msg::Sched(day, i, SchedOp::Remove)));
            }
            let row_el = iced::widget::container(line)
                .id(nr.key)
                .padding(iced::Padding { top: sz(14.0), right: 18.0, bottom: sz(14.0), left: 18.0 })
                .width(Length::Fill)
                .style(crate::widgets::row_style(p, p.panel, p.r(10.0), nr.lit));
            c = c.push(crate::widgets::ring(&p, nr.lit, p.r(10.0), row_el));
        }
        if items.len() < 8 {
            c = c.push(link_row(&p, G::Plus, "Add a change", "A new change an hour after the last one", Msg::Sched(day, items.len(), SchedOp::Add)));
        }
    }
    c.into()
}

/// Game Mode on this PC is off (desktop.json's gameModeOff is true; missing or anything else: On).
pub fn game_modes_off(sh: &Shared) -> bool {
    sh.d_bool("gameModeOff", false)
}
/// Plasma's Back to Game Mode entries shown or hidden to match (nimbus-mode gamemodes apply; 15 s at most).
fn gm_apply_cmd() -> Vec<String> {
    sv(&["timeout", "15", "nimbus-mode", "gamemodes", "apply"])
}

pub fn mode_name(m: &str) -> &'static str {
    match m {
        "server" => "Server mode",
        "desktop" => "Desktop Mode",
        _ => "Game Mode",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schedule_is_changed_as_setstartup_does() {
        assert_eq!(sched_label("18:30"), "6:30 PM");
        assert_eq!(sched_label("00:00"), "12 AM");
        assert_eq!(sched_label("12:00"), "12 PM");
        assert_eq!((sched_at(-30), sched_at(1470)), ("23:30".to_string(), "00:30".to_string()));
        let mut st = State::default();
        st.sched = Some(serde_json::json!({"weekday": [{"at": "00:00", "mode": "server"}, {"at": "18:00", "mode": "game"}], "weekend": [{"at": "09:00", "mode": "game"}]}));
        sched_edit(&mut st, "weekday", 1, SchedOp::Move(1));
        assert_eq!(vs(st.sched.as_ref().unwrap(), "weekday.1.at"), "18:30");
        sched_edit(&mut st, "weekday", 1, SchedOp::Mode);
        assert_eq!(vs(st.sched.as_ref().unwrap(), "weekday.1.mode"), "desktop");
        // Moving onto another change's time skips past it; the list stays in order.
        sched_edit(&mut st, "weekday", 0, SchedOp::Move(-1));
        assert_eq!(vs(st.sched.as_ref().unwrap(), "weekday.1.at"), "23:30");
        assert_eq!(vs(st.sched.as_ref().unwrap(), "weekday.0.at"), "18:30");
        sched_edit(&mut st, "weekend", 1, SchedOp::Add);
        assert_eq!(vs(st.sched.as_ref().unwrap(), "weekend.1.at"), "10:00");
        sched_edit(&mut st, "weekend", 0, SchedOp::Remove);
        sched_edit(&mut st, "weekend", 0, SchedOp::Remove);
        assert_eq!(va(st.sched.as_ref().unwrap(), "weekend").len(), 1, "never fewer than one a day");
        assert_eq!(st.settle, Some(("sched", 2)));
        assert_eq!(sched_focused(&st, "From 6:30 PM", "Desktop Mode · Monday – Friday"), Some(("weekday", 0)));
        assert_eq!(sched_focused(&st, "From 6:30 PM", "Desktop Mode · Saturday – Sunday"), None);
    }
    #[test]
    fn values_and_reads() {
        let v: Value = serde_json::json!({"a": {"b": [1, {"c": "x"}]}, "n": "12", "t": true});
        assert_eq!(vs(&v, "a.b.1.c"), "x");
        assert_eq!(vf(&v, "n", 0.0), 12.0);
        assert!(vb(&v, "t"));
        assert!(at(&v, "a.z").is_null());
        assert_eq!(kv("until=2099,1,1\nfullscreen=true\n")["fullscreen"], "true");
        for p in PAGES {
            assert!(crate::registry::page(p).is_some(), "{p} isn't a Settings page");
            for k in reads_of(p) {
                assert!(!reader(k).is_empty(), "{p}'s read {k} runs nothing");
            }
        }
        assert_eq!(kw("kdeglobals", &["Sounds"], "Enable", "false").join(" "), "kwriteconfig6 --notify --file kdeglobals --group Sounds --key Enable false");
    }
    #[test]
    fn every_system_page_is_built() {
        for p in crate::registry::PAGES.iter().filter(|p| p.cat == "system" && !p.landing) {
            assert!(PAGES.contains(&p.id) || crate::cats::kde::PAGES.contains(&p.id) || crate::cats::couch::PAGES.contains(&p.id), "System's page {} isn't built", p.id);
        }
    }
    #[test]
    fn a_job_times_out() {
        let o = one(&sv(&["sleep", "5"]), "", 1);
        assert!(!o.ok && o.err.contains("too long"));
        let o = one(&sv(&["sh", "-c", "cat; printf %s \"$HEARTH_INPUT\""]), "hi", 5);
        assert_eq!(o.out, "hihi");
    }
}
