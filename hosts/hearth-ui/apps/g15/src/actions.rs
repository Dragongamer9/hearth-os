//! What a G-key does (config.rs ACTION), playing it, and recording a macro with MR.
//!
//! Recording, as the approved mockups show it: press MR (its light comes on), type on the G15's keys, then press the
//! G-key to keep it on (MR again, or an M-key, throws it away). The ordinary keys are read only while
//! recording, and only the G15's own (046d:c221), never another keyboard. The keys still reach the app in front.

use crate::input::Output;
use crate::keys;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub code: u16,
    pub down: bool,
    pub ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Keys(Vec<u16>),
    Text(String),
    Macro(Vec<Step>),
    App { desktop: String, command: String },
    Hearth(String),
    Media(String),
}

/// The longest pause a macro keeps between two keys, and the most steps one holds.
pub const MAX_PAUSE_MS: u32 = 5000;
pub const MAX_STEPS: usize = 2000;

pub fn parse(v: &Value) -> Option<Action> {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    Some(match v.get("kind")?.as_str()? {
        "keys" => Action::Keys(keys::combo(&s("keys"))?),
        "text" => Action::Text(s("text")).filter_text()?,
        "macro" => {
            let steps: Vec<Step> = v
                .get("steps")?
                .as_array()?
                .iter()
                .filter_map(|st| {
                    let code = keys::code_of(st.get("key")?.as_str()?)?;
                    Some(Step { code, down: st.get("down").and_then(|d| d.as_bool()).unwrap_or(true), ms: st.get("ms").and_then(|m| m.as_u64()).unwrap_or(0).min(MAX_PAUSE_MS as u64) as u32 })
                })
                .take(MAX_STEPS)
                .collect();
            if steps.is_empty() {
                return None;
            }
            Action::Macro(steps)
        }
        "app" => {
            let (desktop, command) = (s("desktop"), s("command"));
            if desktop.is_empty() && command.is_empty() {
                return None;
            }
            Action::App { desktop, command }
        }
        "hearth" => Action::Hearth(s("action")).filter_known()?,
        "media" => Action::Media(s("action")),
        _ => return None,
    })
}

trait Filters: Sized {
    fn filter_text(self) -> Option<Self>;
    fn filter_known(self) -> Option<Self>;
}
impl Filters for Action {
    fn filter_text(self) -> Option<Self> {
        match &self {
            Action::Text(t) if t.is_empty() => None,
            _ => Some(self),
        }
    }
    fn filter_known(self) -> Option<Self> {
        match &self {
            Action::Hearth(a) if !HEARTH_ACTIONS.iter().any(|(k, _)| k == a) => None,
            _ => Some(self),
        }
    }
}

/// A macro's options (the G-keys page: "Wait between keys", "Repeat", "While held"):
/// "wait": "recorded" (default) | "none" | a number of ms; "repeat": how many times (default 1);
/// "whileHeld": "once" (default) | "repeat" (again and again while the G-key is held).
#[derive(Debug, Clone, PartialEq)]
pub struct MacroOpts {
    pub wait: Option<u32>,
    pub repeat: u32,
    pub while_held: bool,
}

pub fn macro_opts(v: &Value) -> MacroOpts {
    let wait = match v.get("wait") {
        Some(Value::String(s)) if s == "none" => Some(0),
        Some(Value::Number(n)) => n.as_u64().map(|n| n.min(MAX_PAUSE_MS as u64) as u32),
        _ => None,
    };
    MacroOpts {
        wait,
        repeat: v.get("repeat").and_then(|r| r.as_u64()).unwrap_or(1).clamp(1, 100) as u32,
        while_held: v.get("whileHeld").and_then(|w| w.as_str()) == Some("repeat"),
    }
}

/// The steps with the chosen waits (the first key never waits).
pub fn with_waits(steps: &[Step], wait: Option<u32>) -> Vec<Step> {
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| Step { code: s.code, down: s.down, ms: if i == 0 { 0 } else { wait.unwrap_or(s.ms) } })
        .collect()
}

/// Steps as saved in g15.json.
pub fn steps_json(steps: &[Step]) -> Value {
    Value::Array(steps.iter().map(|s| json!({"key": keys::name_of(s.code), "down": s.down, "ms": s.ms})).collect())
}

/// What the app and the screen say an action does.
pub fn describe(a: &Action) -> String {
    match a {
        Action::Keys(k) => keys::combo_text(k),
        Action::Text(t) => format!("Type: {}", t.chars().take(30).collect::<String>()),
        Action::Macro(s) => format!("Macro: {} keys", s.iter().filter(|x| x.down).count()),
        Action::App { desktop, command } => format!("Open {}", (if desktop.is_empty() { command } else { desktop }).trim_end_matches(".desktop")),
        Action::Hearth(h) => HEARTH_ACTIONS.iter().find(|(k, _)| k == h).map(|(_, l)| l.to_string()).unwrap_or_default(),
        Action::Media(m) => m.clone(),
    }
}

/// Hearth's own actions a G-key can do (the app lists them in this order).
pub const HEARTH_ACTIONS: &[(&str, &str)] = &[
    ("apps", "Open the app menu"),
    ("quick", "Open Quick Access"),
    ("settings", "Open Settings"),
    ("files", "Open Hearth Files"),
    ("tasks", "Open Task Manager"),
    ("screenshot", "Take a screenshot"),
    ("screenshot-area", "Screenshot of an area"),
    ("lock", "Lock the screen"),
    ("power", "Power menu"),
    ("volume-up", "Volume up"),
    ("volume-down", "Volume down"),
    ("mute", "Mute"),
    ("next-page", "Next page on the keyboard's screen"),
];

const SHELL: &str = "/usr/share/nimbus/desktop/shell";

fn mantel() -> bool {
    crate::which("quickshell") && std::path::Path::new(SHELL).join("shell.qml").exists() && std::env::var("XDG_CURRENT_DESKTOP").map(|d| !d.to_uppercase().contains("KDE")).unwrap_or(true)
}

/// The command for a Hearth action: Mantel's own (quickshell ipc) when Mantel is the desktop, otherwise a program
/// that works anywhere (KDE Plasma on the Fedora edition). Empty: done with keys instead (see `hearth_keys`).
pub fn hearth_command(a: &str) -> Vec<String> {
    let ipc = |t: &str, f: &str| -> Vec<String> { ["timeout", "5", "quickshell", "ipc", "-p", SHELL, "call", t, f].iter().map(|s| s.to_string()).collect() };
    let v = |x: &[&str]| -> Vec<String> { x.iter().map(|s| s.to_string()).collect() };
    let m = mantel();
    match a {
        "apps" if m => ipc("launcher", "toggle"),
        "quick" if m => ipc("quick", "toggle"),
        "tasks" if m => ipc("tasks", "toggle"),
        "power" if m => ipc("power", "menu"),
        "volume-up" if m => ipc("buttons", "up"),
        "volume-down" if m => ipc("buttons", "down"),
        "mute" if m => ipc("buttons", "mute"),
        "settings" => v(&["hearth-settings"]),
        "files" => v(&["hearth-filemanager"]),
        "tasks" => v(&["hearth-task-manager"]),
        "screenshot" if crate::which("hearth-capture") => v(&["hearth-capture", "screen"]),
        "screenshot-area" if crate::which("hearth-capture") => v(&["hearth-capture", "area"]),
        "lock" => v(&["loginctl", "lock-session"]),
        "volume-up" => v(&["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", "5%+"]),
        "volume-down" => v(&["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"]),
        "mute" => v(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]),
        _ => Vec::new(),
    }
}

/// Hearth actions done with keys where no program does them (KDE Plasma's own shortcuts).
pub fn hearth_keys(a: &str) -> Vec<u16> {
    match a {
        "apps" => vec![keys::KEY_LEFTMETA],
        "quick" => vec![keys::KEY_LEFTMETA, keys::KEY_LEFTALT, 30],
        "power" => vec![keys::KEY_LEFTCTRL, keys::KEY_LEFTALT, 111],
        // Without Hearth Capture: the Print key (Mantel's and KDE's screenshot), with Shift for an area.
        "screenshot" => vec![99],
        "screenshot-area" => vec![keys::KEY_LEFTSHIFT, 99],
        _ => Vec::new(),
    }
}

pub fn media_command(a: &str) -> Vec<String> {
    let verb = match a {
        "play-pause" | "previous" | "next" | "stop" => a,
        _ => return Vec::new(),
    };
    ["timeout", "5", "playerctl", verb].iter().map(|s| s.to_string()).collect()
}

/// A command line split into words, with "…" and '…' kept together (no shell: nothing else is interpreted).
pub fn split_command(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q: Option<char> = None;
    let mut any = false;
    for ch in s.chars() {
        match (q, ch) {
            (Some(c), x) if x == c => q = None,
            (Some(_), x) => cur.push(x),
            (None, '"') | (None, '\'') => {
                q = Some(ch);
                any = true;
            }
            (None, x) if x.is_whitespace() => {
                if !cur.is_empty() || any {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            (None, x) => cur.push(x),
        }
    }
    if !cur.is_empty() || any {
        out.push(cur);
    }
    out
}

/// Where an app's menu entry is (yours, the system's, Flatpak's).
pub fn find_desktop(id: &str) -> Option<std::path::PathBuf> {
    let id = if id.ends_with(".desktop") { id.to_string() } else { format!("{id}.desktop") };
    if id.contains('/') {
        return None;
    }
    let home = crate::root::home();
    let data = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
    let mut dirs = vec![data.join("applications"), data.join("flatpak/exports/share/applications")];
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(sys.split(':').filter(|d| !d.is_empty()).map(|d| std::path::Path::new(d).join("applications")));
    dirs.push("/var/lib/flatpak/exports/share/applications".into());
    dirs.into_iter().map(|d| d.join(&id)).find(|p| p.is_file())
}

pub fn app_command(desktop: &str, command: &str) -> Vec<String> {
    if !desktop.is_empty() {
        if let Some(p) = find_desktop(desktop) {
            let p = p.to_string_lossy().to_string();
            if crate::which("gio") {
                return vec!["gio".into(), "launch".into(), p];
            }
            if crate::which("kioclient") {
                return vec!["kioclient".into(), "exec".into(), p];
            }
        }
        let id = desktop.trim_end_matches(".desktop").to_string();
        if crate::which("gtk-launch") {
            return vec!["gtk-launch".into(), id];
        }
        return Vec::new();
    }
    split_command(command)
}

fn tap(out: &mut dyn Output, combo: &[u16]) {
    for &k in combo {
        out.key(k, true);
        out.sync();
    }
    for &k in combo.iter().rev() {
        out.key(k, false);
        out.sync();
    }
}

/// Plays an action's keys (combos, text, macros) on `out`. Keys still held at the end are let go, so a macro cut
/// short never leaves a key stuck down. Other kinds are run as programs by `run`.
pub fn play(a: &Action, out: &mut dyn Output, pace: &mut dyn FnMut(u32)) {
    match a {
        Action::Keys(k) => tap(out, k),
        Action::Text(t) => {
            for ch in t.chars() {
                if let Some((code, shift)) = keys::char_key(ch) {
                    if shift {
                        tap(out, &[keys::KEY_LEFTSHIFT, code]);
                    } else {
                        tap(out, &[code]);
                    }
                    pace(4);
                }
            }
        }
        Action::Macro(steps) => {
            let mut held: Vec<u16> = Vec::new();
            for s in steps {
                if s.ms > 0 {
                    pace(s.ms.min(MAX_PAUSE_MS));
                }
                out.key(s.code, s.down);
                out.sync();
                if s.down {
                    if !held.contains(&s.code) {
                        held.push(s.code);
                    }
                } else {
                    held.retain(|&c| c != s.code);
                }
            }
            for c in held.into_iter().rev() {
                out.key(c, false);
                out.sync();
            }
        }
        Action::Hearth(h) => {
            let k = hearth_keys(h);
            if hearth_command(h).is_empty() && !k.is_empty() {
                tap(out, &k);
            }
        }
        _ => {}
    }
}

/// Runs what isn't typing (apps, Hearth's actions, music). Returns false when there's nothing that can do it here.
pub fn run(a: &Action) -> bool {
    let cmd = match a {
        Action::App { desktop, command } => app_command(desktop, command),
        Action::Hearth(h) => hearth_command(h),
        Action::Media(m) => media_command(m),
        _ => return true,
    };
    if cmd.is_empty() {
        return matches!(a, Action::Hearth(h) if !hearth_keys(h).is_empty());
    }
    crate::spawn_detached(&cmd)
}

pub fn sleep_ms(ms: u32) {
    std::thread::sleep(Duration::from_millis(ms as u64));
}

// ------------------------------------------------------------------ recording (MR)
#[derive(Debug, Clone, PartialEq)]
pub enum Rec {
    Idle,
    /// MR pressed: what you type on the G15 is being kept.
    Recording,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecOut {
    /// The MR light and the screen's message change.
    Show,
    /// Keep these steps on this G-key.
    Save(u8, Vec<Step>),
    /// Thrown away (MR again, an M-key, or a G-key before anything was typed).
    Cancelled,
}

/// The approved mockups' way (g15-1 and g15-5): press MR, type, then press the G-key to keep it on.
pub struct Recorder {
    pub state: Rec,
    pub steps: Vec<Step>,
    pub delays: bool,
    last_ms: Option<u64>,
    pub started_ms: u64,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder { state: Rec::Idle, steps: Vec::new(), delays: true, last_ms: None, started_ms: 0 }
    }
    pub fn mr(&mut self, now_ms: u64) -> RecOut {
        match std::mem::replace(&mut self.state, Rec::Idle) {
            Rec::Idle => {
                self.state = Rec::Recording;
                self.steps.clear();
                self.last_ms = None;
                self.started_ms = now_ms;
                RecOut::Show
            }
            Rec::Recording => {
                self.steps.clear();
                RecOut::Cancelled
            }
        }
    }
    /// A G-key while recording keeps the macro on it (None: not recording, the key does what it always does).
    pub fn g(&mut self, g: u8) -> Option<RecOut> {
        if self.state != Rec::Recording {
            return None;
        }
        self.state = Rec::Idle;
        let steps = finish(std::mem::take(&mut self.steps));
        Some(if steps.is_empty() { RecOut::Cancelled } else { RecOut::Save(g, steps) })
    }
    pub fn cancel(&mut self) -> bool {
        let was = self.state != Rec::Idle;
        self.state = Rec::Idle;
        self.steps.clear();
        was
    }
    /// A key typed on the G15's ordinary keys while recording (repeats are skipped).
    pub fn key(&mut self, code: u16, value: i32, now_ms: u64) {
        if self.state != Rec::Recording || value == 2 || self.steps.len() >= MAX_STEPS {
            return;
        }
        let ms = match (self.delays, self.last_ms) {
            (true, Some(l)) => (now_ms.saturating_sub(l)).min(MAX_PAUSE_MS as u64) as u32,
            _ => 0,
        };
        self.last_ms = Some(now_ms);
        self.steps.push(Step { code, down: value == 1, ms });
    }
    pub fn recording(&self) -> bool {
        self.state == Rec::Recording
    }
    /// For the screen: how many keys and seconds so far.
    pub fn summary(&self, now_ms: u64) -> (usize, f32) {
        let keys = self.steps.iter().filter(|s| s.down).count();
        (keys, now_ms.saturating_sub(self.started_ms) as f32 / 1000.0)
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Recorder::new()
    }
}

/// A recording made whole: a key let go that was never pressed in it (held down when MR was pressed) is dropped,
/// and a key still down at the end is let go.
pub fn finish(steps: Vec<Step>) -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    let mut down: Vec<u16> = Vec::new();
    for s in steps {
        if s.down {
            if !down.contains(&s.code) {
                down.push(s.code);
            }
            out.push(s);
        } else if down.contains(&s.code) {
            down.retain(|&c| c != s.code);
            out.push(s);
        }
    }
    for c in down.into_iter().rev() {
        out.push(Step { code: c, down: false, ms: 0 });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Recorded;

    #[test]
    fn reads_actions() {
        assert_eq!(parse(&json!({"kind": "keys", "keys": "Ctrl+C"})), Some(Action::Keys(vec![29, 46])));
        assert_eq!(parse(&json!({"kind": "text", "text": ""})), None);
        assert_eq!(parse(&json!({"kind": "hearth", "action": "screenshot"})), Some(Action::Hearth("screenshot".into())));
        assert_eq!(parse(&json!({"kind": "hearth", "action": "rm -rf"})), None);
        assert_eq!(parse(&json!({"kind": "macro", "steps": [{"key": "A", "down": true, "ms": 99999}]})), Some(Action::Macro(vec![Step { code: 30, down: true, ms: MAX_PAUSE_MS }])));
        assert_eq!(parse(&json!({"kind": "nope"})), None);
        assert_eq!(split_command("firefox --new-window 'https://hearthos.fun/a b'"), vec!["firefox", "--new-window", "https://hearthos.fun/a b"]);
    }

    #[test]
    fn plays_combos_and_text() {
        let mut o = Recorded::default();
        play(&Action::Keys(vec![29, 46]), &mut o, &mut |_| {});
        assert_eq!(o.keys, vec![(29, true), (46, true), (46, false), (29, false)]);
        let mut o = Recorded::default();
        play(&Action::Text("Hi!".into()), &mut o, &mut |_| {});
        assert_eq!(o.keys, vec![(42, true), (35, true), (35, false), (42, false), (23, true), (23, false), (42, true), (2, true), (2, false), (42, false)]);
    }

    #[test]
    fn records_and_plays_back_a_macro() {
        let mut r = Recorder::new();
        assert_eq!(r.g(5), None, "an ordinary press when not recording");
        assert_eq!(r.mr(1000), RecOut::Show);
        assert!(r.recording());
        r.key(42, 1, 1200); // Shift held from here
        r.key(35, 1, 1300);
        r.key(35, 2, 1350); // a repeat: skipped
        r.key(35, 0, 1400);
        r.key(99, 0, 1450); // let go but never pressed here: dropped
        assert_eq!(r.summary(4200), (2, 3.2));
        match r.g(5) {
            Some(RecOut::Save(g, steps)) => {
                assert_eq!(g, 5);
                assert_eq!(steps.iter().map(|s| (s.code, s.down)).collect::<Vec<_>>(), vec![(42, true), (35, true), (35, false), (42, false)]);
                assert_eq!(steps[1].ms, 100);
                // Saved and read back, then played: the same keys, the pauses kept.
                let a = parse(&json!({"kind": "macro", "steps": steps_json(&steps)})).unwrap();
                let mut o = Recorded::default();
                let mut waited = 0;
                play(&a, &mut o, &mut |ms| waited += ms);
                assert_eq!(o.keys, vec![(42, true), (35, true), (35, false), (42, false)]);
                assert_eq!(waited, 200);
            }
            x => panic!("{x:?}"),
        }
        assert_eq!(r.state, Rec::Idle);
        // MR, then MR again: nothing kept. MR, then a G-key before typing: nothing kept either.
        r.mr(0);
        assert_eq!(r.mr(10), RecOut::Cancelled);
        r.mr(20);
        assert_eq!(r.g(3), Some(RecOut::Cancelled));
    }

    #[test]
    fn macro_options() {
        let o = macro_opts(&json!({"wait": "none", "repeat": 3, "whileHeld": "repeat"}));
        assert_eq!(o, MacroOpts { wait: Some(0), repeat: 3, while_held: true });
        assert_eq!(macro_opts(&json!({})), MacroOpts { wait: None, repeat: 1, while_held: false });
        let st = vec![Step { code: 30, down: true, ms: 50 }, Step { code: 30, down: false, ms: 70 }];
        assert_eq!(with_waits(&st, Some(10))[1].ms, 10);
        assert_eq!(with_waits(&st, None)[0].ms, 0);
        assert_eq!(with_waits(&st, None)[1].ms, 70);
    }

    #[test]
    fn a_macro_never_leaves_a_key_down() {
        let mut o = Recorded::default();
        play(&Action::Macro(vec![Step { code: 29, down: true, ms: 0 }, Step { code: 46, down: true, ms: 0 }]), &mut o, &mut |_| {});
        assert_eq!(o.keys.last(), Some(&(29, false)));
        assert!(o.keys.contains(&(46, false)));
    }
}
