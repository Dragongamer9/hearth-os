//! hearth-pad-host: Hearth's controller in KDE Plasma (the Fedora edition), as Mantel's Pad.qml gives it on Debian.
//!
//! hearth-pad --plasma (desktop/shell/hearth-pad, the one controller helper; link.rs) reads the controller and
//! works the pointer, clicks, scrolling, keys and KWin's shortcuts. KWin's hearthpad script tells it which window is
//! in front (loaded here for the session when the image didn't turn it on: kwin.rs), so it pauses over a fullscreen
//! game and stands aside for controller apps (docs/CONTROLLER-FOCUS.md). This program draws what it asks for, each
//! as a layer-shell overlay that KWin keeps on top and that never takes the keyboard from the window in front:
//!   Guide          the Hearth menu (RadialMenu.qml)
//!   Y, Guide + X   the on-screen keyboard (Keyboard.qml / OskBoard.qml, Hearth's shared engine: osk.rs)
//!   View           the controls card (Help.qml, osk-plasma.qml's list) and short messages
//!   after use      the button hints along the bottom for 12 s (HintStrip.qml)
//! Nothing is drawn and nothing runs while none of them is open: the overlays are made when they open and removed
//! when they close, and every wait is one sleeping thread (no polling).
//!
//! Started by /usr/libexec/hearth/hearth-pad-plasma (Plasma's autostart). Exit 4: this desktop has no layer-shell,
//! the launcher then falls back to hearth-pad --standalone (the controller as a plain mouse and keyboard).
//!
//! For CI's pictures: --show menu | keyboard | accents | wheel | controls | hints | toast (no helper started).

mod link;
mod looks;
mod osk;
mod view;

use hearth_ui::app;
use hearth_ui::ui::G;
use hearth_ui::Pal;
use iced::widget::Space;
use iced::{window, Element, Event, Size, Subscription, Task};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Item {
    pub action: &'static str,
    pub label: &'static str,
    pub icon: G,
}

/// The Hearth menu, clockwise from the top (hearth-pad assumes 8), as RadialMenu.qml has it.
pub const ITEMS: [Item; 8] = [
    Item { action: "apps", label: "Apps", icon: G::Grid },
    Item { action: "keyboard", label: "Keyboard", icon: G::Keyboard },
    Item { action: "quick", label: "Quick Access", icon: G::Menu },
    Item { action: "close", label: "Close window", icon: G::Close },
    Item { action: "game", label: "Game Mode", icon: G::Pad },
    Item { action: "screenshot", label: "Screenshot", icon: G::Camera },
    Item { action: "mouse", label: "Controller mouse", icon: G::Mouse },
    Item { action: "help", label: "Controls", icon: G::Info },
];

const HINTS_FOR: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Radial,
    Help,
    Toast,
    Hints,
    Keyboard,
}

#[derive(Debug, Clone, Copy)]
pub enum Wait {
    Help = 0,
    Toast = 1,
    Hints = 2,
    Save = 3,
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Msg {
    Pad(link::Ev),
    Theme,
    Resized(window::Id, Size),
    Closed(window::Id),
    RadialHover(i32),
    RadialPick(i32),
    RadialDismiss,
    HelpClose,
    OpenWelcome,
    KeyTap(usize, usize),
    SuggTap(usize),
    AltTap(usize),
    Tick,
    Waited(Wait, u64),
    Clip(Vec<String>),
    Nothing,
}

pub struct App {
    pub p: Pal,
    ids: HashMap<Kind, window::Id>,
    sizes: HashMap<window::Id, Size>,
    /// the screen, from the full-screen overlays (a guess until one has opened)
    scr: Size,
    pub sel: i32,
    radial: bool,
    pub mouse_on: bool,
    owner: String,
    help: bool,
    pub toast: String,
    hints_until: Option<Instant>,
    /// the newest wait of each kind (an older one that ends does nothing)
    gens: [u64; 4],
    // the keyboard
    kb: bool,
    osk: Option<osk::Osk>,
    pub osk_err: String,
    pub vm: Option<osk::Vm>,
    pub look: looks::Look,
    desk_theme: String,
    busy: bool,
    save: Option<Value>,
    problem: String,
}

fn pad_send(v: Value) {
    link::send(&v);
}

/// A program in the background, waited for on a thread of its own (no zombies), with nothing on its stdin.
fn bg(cmd: Vec<String>) {
    let _ = std::thread::Builder::new().name("hearth-pad-run".into()).stack_size(64 * 1024).spawn(move || {
        if let Some((p, a)) = cmd.split_first() {
            let _ = std::process::Command::new(p).args(a).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).status();
        }
    });
}
fn sh(script: &str) {
    bg(vec!["sh".into(), "-c".into(), script.into()]);
}

fn wait(ms: u64, w: Wait, gen: u64) -> Task<Msg> {
    Task::perform(hearth_ui::run::after(ms), move |_| Msg::Waited(w, gen))
}

impl App {
    fn open(&mut self, k: Kind) -> Task<Msg> {
        if self.ids.contains_key(&k) {
            return Task::none();
        }
        let id = window::Id::unique();
        self.ids.insert(k, id);
        let all = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
        let base = NewLayerShellSettings {
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::None,
            output_option: OutputOption::Active,
            ..NewLayerShellSettings::default()
        };
        let settings = match k {
            Kind::Radial => NewLayerShellSettings { anchor: all, size: Some((0, 0)), exclusive_zone: Some(-1), namespace: Some("hearth-radial".into()), ..base },
            Kind::Help => NewLayerShellSettings { anchor: all, size: Some((0, 0)), exclusive_zone: Some(-1), namespace: Some("hearth-help".into()), ..base },
            Kind::Toast => NewLayerShellSettings {
                anchor: Anchor::Top | Anchor::Left | Anchor::Right,
                size: Some((0, 92)),
                exclusive_zone: Some(-1),
                events_transparent: true,
                namespace: Some("hearth-toast".into()),
                ..base
            },
            Kind::Hints => NewLayerShellSettings {
                anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
                size: Some((0, 48)),
                exclusive_zone: Some(0),
                events_transparent: true,
                namespace: Some("hearth-hints".into()),
                ..base
            },
            Kind::Keyboard => {
                let h = self.kb_height(self.scr.width);
                let top = self.vm.as_ref().map(|v| v.position == "top").unwrap_or(false);
                NewLayerShellSettings {
                    anchor: (if top { Anchor::Top } else { Anchor::Bottom }) | Anchor::Left | Anchor::Right,
                    size: Some((0, h)),
                    exclusive_zone: Some(h as i32),
                    namespace: Some("hearth-keyboard".into()),
                    ..base
                }
            }
        };
        Task::done(Msg::NewLayerShell { settings, id })
    }

    fn close(&mut self, k: Kind) -> Task<Msg> {
        match self.ids.remove(&k) {
            Some(id) => {
                self.sizes.remove(&id);
                Task::done(Msg::RemoveWindow(id))
            }
            None => Task::none(),
        }
    }

    fn kind(&self, id: window::Id) -> Option<Kind> {
        self.ids.iter().find(|(_, v)| **v == id).map(|(k, _)| *k)
    }

    fn next(&mut self, w: Wait) -> u64 {
        self.gens[w as usize] += 1;
        self.gens[w as usize]
    }

    /// Keyboard.qml's height: a share of the screen by the size setting, never too small for the keys.
    fn kb_height(&self, width: f32) -> u32 {
        let size = self.vm.as_ref().map(|v| v.size.clone()).unwrap_or_default();
        let w = width.max(320.0);
        // Only the full-screen overlays say how tall the screen is: until one has, a 16:9 screen this wide.
        let h = if (self.scr.width - w).abs() < 1.0 { self.scr.height } else { w * 9.0 / 16.0 };
        let share: f32 = if size == "l" { 0.5 } else { 0.44 };
        let least: f32 = if size == "s" { 250.0 } else { 290.0 };
        let wide = w * if size == "s" { 0.2 } else if size == "l" { 0.29 } else { 0.24 };
        (h * share).min(least.max(wide)).round().max(160.0) as u32
    }

    // ------------------------------------------------------------------ hearth-pad
    fn on_pad(&mut self, m: &Value) -> Task<Msg> {
        let t = m.get("t").and_then(Value::as_str).unwrap_or("");
        let mut tasks = Vec::new();
        // The controller was just used for Hearth's menus: the button hints show for a while (Pad.qml's padSeen).
        if matches!(t, "radial" | "osk" | "oskpad" | "launcher" | "help") {
            self.hints_until = Some(Instant::now() + HINTS_FOR);
            let g = self.next(Wait::Hints);
            tasks.push(wait(HINTS_FOR.as_millis() as u64 + 50, Wait::Hints, g));
        }
        match t {
            "ready" => {
                self.problem.clear();
                pad_send(json!({ "cmd": "mouse", "on": self.mouse_on }));
                if self.kb {
                    let split = self.vm.as_ref().map(|v| v.fast == "split").unwrap_or(false);
                    pad_send(json!({ "cmd": "osk", "open": true, "split": split }));
                }
            }
            "error" => {
                self.problem = m.get("text").and_then(Value::as_str).unwrap_or("").to_string();
                eprintln!("hearth-pad-host: hearth-pad: {}", self.problem);
            }
            "radial" => {
                if m.get("open").and_then(Value::as_bool).unwrap_or(false) {
                    self.radial = true;
                    self.sel = m.get("sel").and_then(Value::as_i64).unwrap_or(-1) as i32;
                    tasks.push(self.open(Kind::Radial));
                } else {
                    self.radial = false;
                    tasks.push(self.close(Kind::Radial));
                    let pick = m.get("pick").and_then(Value::as_i64).unwrap_or(-1);
                    if pick >= 0 && (pick as usize) < ITEMS.len() {
                        tasks.push(self.run(ITEMS[pick as usize].action));
                    }
                }
            }
            "osk" => tasks.push(if m.get("open").and_then(Value::as_bool).unwrap_or(false) { self.open_kb() } else { self.close_kb() }),
            "oskpad" => {
                if self.kb {
                    let b = m.get("b").and_then(Value::as_str).unwrap_or("");
                    let down = m.get("down").and_then(Value::as_bool).unwrap_or(false);
                    tasks.push(self.engine(&format!("__e.setHints(true); __e.handle({}, {}, Date.now());", json!(b), down)));
                }
            }
            "oskaxes" => {
                if self.kb {
                    let f = |k: &str| m.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                    tasks.push(self.engine(&format!("__e.axes({}, {}, {}, {}, Date.now());", f("lx"), f("ly"), f("rx"), f("ry"))));
                }
            }
            "launcher" => open_launcher(),
            "help" => tasks.push(if self.help { self.hide_help() } else { self.show_help() }),
            "toast" => tasks.push(self.show_toast(m.get("text").and_then(Value::as_str).unwrap_or(""))),
            "mouse" => self.mouse_on = m.get("on").and_then(Value::as_bool).unwrap_or(true),
            "owner" => self.owner = m.get("app").and_then(Value::as_str).unwrap_or("").to_string(),
            _ => {}
        }
        tasks.push(self.sync_hints());
        Task::batch(tasks)
    }

    /// What the Hearth menu's items do in Plasma (osk-plasma.qml's run()).
    fn run(&mut self, action: &str) -> Task<Msg> {
        match action {
            "apps" => open_launcher(),
            "keyboard" => return self.open_kb(),
            // Hearth's Quick Access widget (Meta+Alt+A) where Plasma has it; the Fedora edition's Plasma doesn't
            // carry it yet, so there the Hearth menu opens Hearth Settings (its home has the quick tiles).
            "quick" => {
                if std::path::Path::new("/usr/share/plasma/plasmoids/org.hearth.quicksettings").exists() {
                    pad_send(json!({ "cmd": "plasma", "do": "quick" }));
                } else {
                    bg(vec!["hearth-settings-app".into()]);
                }
            }
            "close" => pad_send(json!({ "cmd": "plasma", "do": "close" })),
            "game" => bg(vec!["nimbus-mode".into(), "game".into()]),
            "screenshot" => {
                let share = std::env::var("NIMBUS_SHARE").unwrap_or_else(|_| "/usr/share/nimbus".into());
                bg(vec![
                    "sh".into(),
                    "-c".into(),
                    "sleep 0.5; spectacle -f -b -n 2>/dev/null || exec sh \"$0\" screenshot".into(),
                    format!("{share}/desktop/open.sh"),
                ]);
            }
            "mouse" => {
                // Over a controller app: the pointer in that app, on / off (like Guide + R3).
                if !self.owner.is_empty() {
                    pad_send(json!({ "cmd": "padmouse" }));
                } else {
                    self.mouse_on = !self.mouse_on;
                    pad_send(json!({ "cmd": "mouse", "on": self.mouse_on }));
                    let t = format!("Controller mouse {}", if self.mouse_on { "on" } else { "off" });
                    return self.show_toast(&t);
                }
            }
            "help" => return self.show_help(),
            _ => {}
        }
        Task::none()
    }

    fn show_help(&mut self) -> Task<Msg> {
        self.help = true;
        let g = self.next(Wait::Help);
        Task::batch([self.open(Kind::Help), wait(12_000, Wait::Help, g), self.sync_hints()])
    }
    fn hide_help(&mut self) -> Task<Msg> {
        self.help = false;
        Task::batch([self.close(Kind::Help), self.sync_hints()])
    }
    fn show_toast(&mut self, text: &str) -> Task<Msg> {
        if text.is_empty() {
            return Task::none();
        }
        self.toast = text.to_string();
        let g = self.next(Wait::Toast);
        Task::batch([self.open(Kind::Toast), wait(1800, Wait::Toast, g)])
    }
    /// The hints show while the controller was used lately, not over the menu or the controls card.
    fn sync_hints(&mut self) -> Task<Msg> {
        let recent = self.hints_until.map(|t| Instant::now() < t).unwrap_or(false);
        if recent && !self.radial && !self.help {
            self.open(Kind::Hints)
        } else {
            self.close(Kind::Hints)
        }
    }

    // ------------------------------------------------------------------ the keyboard
    fn engine(&mut self, js: &str) -> Task<Msg> {
        let Some(o) = self.osk.as_ref() else { return Task::none() };
        let step = o.call(js);
        self.apply(step)
    }

    fn apply(&mut self, step: osk::Step) -> Task<Msg> {
        if !step.err.is_empty() {
            eprintln!("hearth-pad-host: keyboard: {}", step.err.lines().next().unwrap_or(""));
        }
        self.busy = step.busy;
        let mut tasks = Vec::new();
        for o in step.out {
            match o.get("k").and_then(Value::as_str).unwrap_or("") {
                "emit" => pad_send(json!({ "cmd": "emit", "text": o.get("text").cloned().unwrap_or(json!("")), "mods": o.get("mods").cloned().unwrap_or(json!([])) })),
                "key" => pad_send(json!({ "cmd": "key", "key": o.get("key").cloned().unwrap_or(json!("")), "n": o.get("n").cloned().unwrap_or(json!(1)), "mods": o.get("mods").cloned().unwrap_or(json!([])) })),
                "rumble" => pad_send(json!({ "cmd": "rumble", "strength": o.get("strength").cloned().unwrap_or(json!(0.3)), "ms": o.get("ms").cloned().unwrap_or(json!(20)) })),
                "hide" => tasks.push(self.close_kb()),
                "save" => {
                    self.save = o.get("d").cloned();
                    let g = self.next(Wait::Save);
                    tasks.push(wait(1500, Wait::Save, g));
                }
                _ => {}
            }
        }
        if let Some(v) = step.vm {
            let before = self.vm.as_ref().map(|v| (v.size.clone(), v.position.clone()));
            self.vm = Some(osk::Vm::from(&v));
            let after = self.vm.as_ref().map(|v| (v.size.clone(), v.position.clone()));
            if before.is_some() && before != after {
                tasks.push(self.fit_kb());
            }
        }
        Task::batch(tasks)
    }

    fn open_kb(&mut self) -> Task<Msg> {
        if self.osk.is_none() && self.osk_err.is_empty() {
            match osk::Osk::new() {
                Ok(o) => self.osk = Some(o),
                Err(e) => {
                    eprintln!("hearth-pad-host: the keyboard engine didn't load: {e}");
                    self.osk_err = e;
                }
            }
        }
        let cfg = osk::read_obj(&osk::config_file()).unwrap_or_default();
        self.desk_theme = cfg.get("deskTheme").and_then(Value::as_str).unwrap_or("").to_string();
        let mut tasks = Vec::new();
        if let Some(o) = self.osk.as_mut() {
            o.prepare(&cfg);
            let step = o.call("__e.reset('text', ''); __e.setHints(true);");
            tasks.push(self.apply(step));
        }
        self.restyle();
        let was = self.kb;
        self.kb = true;
        if !was {
            let split = self.vm.as_ref().map(|v| v.fast == "split").unwrap_or(false);
            pad_send(json!({ "cmd": "osk", "open": true, "split": split }));
            // Recent copies for the keyboard's clipboard panel: Plasma keeps them in Klipper.
            tasks.push(Task::perform(hearth_ui::run::spawn(klipper_cmd()), |o| {
                Msg::Clip(o.json().and_then(|v| v.as_array().cloned()).unwrap_or_default().iter().filter_map(|x| x.as_str().map(String::from)).collect())
            }));
        }
        tasks.push(self.open(Kind::Keyboard));
        Task::batch(tasks)
    }

    fn close_kb(&mut self) -> Task<Msg> {
        if !self.kb {
            return Task::none();
        }
        self.kb = false;
        self.busy = false;
        pad_send(json!({ "cmd": "osk", "open": false }));
        self.close(Kind::Keyboard)
    }

    fn restyle(&mut self) {
        let engine_theme = self.vm.as_ref().map(|v| v.theme.clone()).unwrap_or_default();
        self.look = looks::look(looks::resolve(&self.desk_theme, &engine_theme), &self.p);
    }

    /// The keyboard's overlay at the height its size setting asks for, and its space kept free for windows.
    fn fit_kb(&mut self) -> Task<Msg> {
        let Some(id) = self.ids.get(&Kind::Keyboard).copied() else { return Task::none() };
        let w = self.sizes.get(&id).map(|s| s.width).unwrap_or(self.scr.width);
        let h = self.kb_height(w);
        let top = self.vm.as_ref().map(|v| v.position == "top").unwrap_or(false);
        Task::batch([
            Task::done(Msg::AnchorSizeChange { id, anchor: (if top { Anchor::Top } else { Anchor::Bottom }) | Anchor::Left | Anchor::Right, size: (0, h) }),
            Task::done(Msg::ExclusiveZoneChange { id, zone_size: h as i32 }),
        ])
    }

    fn save_learned(&mut self) {
        let Some(Value::Object(d)) = self.save.take() else { return };
        let path = osk::learned_file();
        let mut m = Map::new();
        for k in ["version", "learned", "next", "recentEmoji"] {
            if let Some(v) = d.get(k) {
                m.insert(k.to_string(), v.clone());
            }
        }
        let _ = std::thread::Builder::new().name("hearth-pad-save".into()).spawn(move || {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = hearth_ui::settings_file::merge_json(&path, &m) {
                eprintln!("hearth-pad-host: couldn't keep the learned words: {e}");
            }
        });
    }
}

/// Plasma's launcher (Hearth's, or whichever the panel has), as Super does (osk-plasma.qml's openLauncher).
fn open_launcher() {
    sh("timeout 3 dbus-send --session --type=method_call --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu 2>/dev/null \
        || timeout 3 qdbus6 org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu");
}

fn klipper_cmd() -> Vec<String> {
    let py = "import json\ntry:\n    import gi\n    gi.require_version('Gio', '2.0')\n    from gi.repository import Gio, GLib\n    b = Gio.bus_get_sync(Gio.BusType.SESSION, None)\n    r = b.call_sync('org.kde.klipper', '/klipper', 'org.kde.klipper.klipper', 'getClipboardHistoryMenu', None, GLib.VariantType('(as)'), Gio.DBusCallFlags.NONE, 2000, None)\n    print(json.dumps([x for x in r.unpack()[0] if x][:12]))\nexcept Exception:\n    print('[]')\n";
    vec!["timeout".into(), "4".into(), "python3".into(), "-c".into(), py.into()]
}

/// KWin's hearthpad script (os/system_files/usr/share/kwin/scripts/hearthpad) says which window is in front, which
/// hearth-pad --plasma needs to pause over fullscreen games and stand aside for controller apps. Debian's Hearth
/// Plasma turns it on in its kwinrc; the Fedora edition's Plasma doesn't, so it's loaded here for this session only
/// (KWin's scripting over D-Bus: no setting of yours is changed), unless KWin has it already.
fn load_kwin_script() {
    sh(r#"p="${HEARTH_KWIN_PAD_JS:-/usr/share/kwin/scripts/hearthpad/contents/code/main.js}"
[ -f "$p" ] || exit 0
q() { timeout 3 gdbus call --session --dest org.kde.KWin "$@" 2>/dev/null; }
case "$(q --object-path /Scripting --method org.kde.kwin.Scripting.isScriptLoaded hearthpad)" in *true*) exit 0 ;; esac
id=$(q --object-path /Scripting --method org.kde.kwin.Scripting.loadScript "$p" hearthpad | tr -dc '0-9-')
case "$id" in ''|-*) echo "hearth-pad-host: KWin didn't load its hearthpad script" >&2; exit 0 ;; esac
q --object-path "/Scripting/Script$id" --method org.kde.kwin.Script.run >/dev/null || q --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null"#);
}

fn new() -> (App, Task<Msg>) {
    let p = hearth_ui::theme::load();
    let mut a = App {
        p,
        ids: HashMap::new(),
        sizes: HashMap::new(),
        scr: Size::new(1280.0, 800.0),
        sel: -1,
        radial: false,
        mouse_on: true,
        owner: String::new(),
        help: false,
        toast: String::new(),
        hints_until: None,
        gens: [0; 4],
        kb: false,
        osk: None,
        osk_err: String::new(),
        vm: None,
        look: looks::look("steam", &p),
        desk_theme: String::new(),
        busy: false,
        save: None,
        problem: String::new(),
    };
    let task = match app::arg("--show").as_deref() {
        Some("menu") => a.on_pad(&json!({ "t": "radial", "open": true, "sel": 1 })),
        Some("keyboard") => {
            let t = a.open_kb();
            // A word on its way, so the suggestions show (typing goes nowhere: no helper runs for pictures).
            let t2 = a.engine(
                "var R = __e.view().rows; ['h','e','l','l'].forEach(function (ch) { for (var r = 0; r < R.length; r++) for (var c = 0; c < R[r].length; c++) if (R[r][c].t === 'ch' && R[r][c].v === ch) { __e.tap(r, c); return; } }); __e.setHints(true); __e.state.sel = { r: 2, c: 4 };",
            );
            Task::batch([t, t2])
        }
        Some("accents") => {
            let t = a.open_kb();
            let t2 = a.engine("__e.state.sel = { r: 1, c: 2 }; var t0 = Date.now(); __e.handle('a', true, t0); __e.tick(t0 + 600);");
            Task::batch([t, t2])
        }
        Some("wheel") => {
            let t = a.open_kb();
            let t2 = a.engine("__e.axes(0.62, -0.62, 0, 0, Date.now());");
            Task::batch([t, t2])
        }
        Some("controls") => a.show_help(),
        Some("hints") => {
            a.hints_until = Some(Instant::now() + Duration::from_secs(3600));
            a.sync_hints()
        }
        Some("toast") => a.show_toast("Controller mouse off"),
        _ => {
            load_kwin_script();
            Task::none()
        }
    };
    (a, task)
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Pad(link::Ev::Line(v)) => return a.on_pad(&v),
        Msg::Pad(link::Ev::Ended(code)) => {
            eprintln!("hearth-pad-host: hearth-pad ended ({code}){}", if code == 2 || code == 3 { ": not started again" } else { "" });
            // Its menus can't be answered any more.
            a.radial = false;
            return Task::batch([a.close(Kind::Radial), a.close_kb(), a.sync_hints()]);
        }
        Msg::Theme => {
            a.p = hearth_ui::theme::load();
            a.restyle();
        }
        Msg::Resized(id, s) => {
            a.sizes.insert(id, s);
            match a.kind(id) {
                Some(Kind::Radial) | Some(Kind::Help) => a.scr = s,
                Some(Kind::Keyboard) => {
                    let h = a.kb_height(s.width);
                    if (h as f32 - s.height).abs() > 1.0 {
                        return a.fit_kb();
                    }
                }
                _ => {}
            }
        }
        Msg::Closed(id) => {
            a.sizes.remove(&id);
            if let Some(k) = a.kind(id) {
                a.ids.remove(&k);
                match k {
                    Kind::Radial if a.radial => {
                        a.radial = false;
                        pad_send(json!({ "cmd": "radial", "open": false }));
                    }
                    Kind::Keyboard if a.kb => {
                        a.kb = false;
                        pad_send(json!({ "cmd": "osk", "open": false }));
                    }
                    Kind::Help => a.help = false,
                    _ => {}
                }
            }
        }
        Msg::RadialHover(i) => a.sel = i,
        // Picked or closed with the mouse or touch: hearth-pad still thinks the menu is open.
        Msg::RadialPick(i) => {
            a.radial = false;
            pad_send(json!({ "cmd": "radial", "open": false }));
            let t = a.close(Kind::Radial);
            let r = if i >= 0 && (i as usize) < ITEMS.len() { a.run(ITEMS[i as usize].action) } else { Task::none() };
            return Task::batch([t, r, a.sync_hints()]);
        }
        Msg::RadialDismiss => {
            a.radial = false;
            pad_send(json!({ "cmd": "radial", "open": false }));
            return Task::batch([a.close(Kind::Radial), a.sync_hints()]);
        }
        Msg::HelpClose => return a.hide_help(),
        Msg::OpenWelcome => {
            bg(vec!["hearth-tools".into(), "--app".into(), "welcome".into(), "--page".into(), "help".into()]);
            return a.hide_help();
        }
        Msg::KeyTap(r, c) => return a.engine(&format!("__e.setHints(false); __e.tap({r}, {c});")),
        Msg::SuggTap(i) => return a.engine(&format!("__e.setHints(false); __e.tap(-1, {i});")),
        Msg::AltTap(i) => return a.engine(&format!("__e.pickAlt({i});")),
        Msg::Tick => return a.engine("__e.tick(Date.now());"),
        Msg::Waited(w, g) => {
            if g != a.gens[w as usize] {
                return Task::none();
            }
            match w {
                Wait::Help => return a.hide_help(),
                Wait::Toast => {
                    a.toast.clear();
                    return a.close(Kind::Toast);
                }
                Wait::Hints => return a.sync_hints(),
                Wait::Save => a.save_learned(),
            }
        }
        Msg::Clip(items) => {
            if let Some(o) = a.osk.as_ref() {
                let step = o.clipboard(&items);
                return a.apply(step);
            }
        }
        Msg::Nothing => {}
        _ => {}
    }
    Task::none()
}

fn view(a: &App, id: window::Id) -> Element<'_, Msg> {
    let size = a.sizes.get(&id).copied().unwrap_or(a.scr);
    match a.kind(id) {
        Some(Kind::Radial) => view::radial(a, size),
        Some(Kind::Help) => view::help(a, size),
        Some(Kind::Toast) => view::toast(a),
        Some(Kind::Hints) => view::hints(a),
        Some(Kind::Keyboard) => view::keyboard(a, size),
        None => Space::new().into(),
    }
}

fn window_events(e: Event, _s: iced::event::Status, id: window::Id) -> Option<Msg> {
    match e {
        Event::Window(window::Event::Opened { size, .. }) => Some(Msg::Resized(id, size)),
        Event::Window(window::Event::Resized(s)) => Some(Msg::Resized(id, s)),
        Event::Window(window::Event::Closed) => Some(Msg::Closed(id)),
        _ => None,
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut subs = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::event::listen_with(window_events)];
    if app::arg("--show").is_none() && !app::flag("--no-helper") {
        subs.push(Subscription::run(link::stream).map(Msg::Pad));
    }
    // Held buttons repeat and a tilted stick moves the selection: only then does the engine need its frames.
    if a.kb && a.busy {
        subs.push(iced::time::every(Duration::from_millis(16)).map(|_| Msg::Tick));
    }
    Subscription::batch(subs)
}

/// Does this desktop have layer-shell (KWin, sway, labwc...)? Asked before anything is drawn.
fn has_layer_shell() -> bool {
    use wayland_client::protocol::wl_registry;
    use wayland_client::{Connection, Dispatch, QueueHandle};
    struct S(bool);
    impl Dispatch<wl_registry::WlRegistry, ()> for S {
        fn event(st: &mut S, _: &wl_registry::WlRegistry, ev: wl_registry::Event, _: &(), _: &Connection, _: &QueueHandle<S>) {
            if let wl_registry::Event::Global { interface, .. } = ev {
                if interface == "zwlr_layer_shell_v1" {
                    st.0 = true;
                }
            }
        }
    }
    let Ok(conn) = Connection::connect_to_env() else { return false };
    let mut q = conn.new_event_queue();
    let qh = q.handle();
    let _reg = conn.display().get_registry(&qh, ());
    let mut s = S(false);
    q.roundtrip(&mut s).is_ok() && s.0
}

fn main() {
    if app::handle_cli(
        "hearth-pad-host",
        "The controller in KDE Plasma: runs hearth-pad --plasma and draws the Hearth menu, the on-screen keyboard and the controls.\n\
         --show menu|keyboard|accents|wheel|controls|hints|toast  one of them, for pictures (no controller helper)\n\
         --no-helper  don't start hearth-pad",
    ) {
        return;
    }
    if !has_layer_shell() {
        eprintln!("hearth-pad-host: this desktop has no layer-shell (or no Wayland): nothing to draw on");
        std::process::exit(4);
    }
    iced_layershell::disable_clipboard();
    // The overlays are see-through (the dimmed screen behind the Hearth menu, the hints over your window): buffers
    // with alpha (hearth-ui's softbuffer, vendor/README.md). Set before the first surface; nothing else here reads it.
    std::env::set_var("HEARTH_SOFTBUFFER_ALPHA", "1");
    let r = daemon(new, || "hearth-pad".to_string(), update, view)
        .subscription(subscription)
        .theme(|a: &App, _id: window::Id| app::iced_theme(&a.p))
        .style(|a: &App, _t: &iced::Theme| iced::theme::Style { background_color: iced::Color::TRANSPARENT, text_color: a.p.text })
        .settings(Settings {
            layer_settings: LayerShellSettings { start_mode: StartMode::Background, ..LayerShellSettings::default() },
            default_font: app::INTER,
            ..Settings::default()
        })
        .run();
    if let Err(e) = r {
        eprintln!("hearth-pad-host: {e}");
        std::process::exit(1);
    }
}
