//! hearth-remap-app: Hearth Remap (board docs/mockups/open-source-apps/10-remap.png; checklist
//! docs/mockups/checklists/open-source-10-remap.md). Any key, mouse or controller button, your way, built on Input
//! Remapper (GPL-3): every change is an Input Remapper preset (ir.rs) on its device, its service does the remapping,
//! and Hearth's runner (run.rs, `--run`) turns each profile on when its game or app is in front.
//!
//! Input Remapper not on this PC: Install Input Remapper gets Debian's own package (debs.rs, hearth-admin deb-install); its
//! service not running: the app turns it on (one OK as administrator). Neither is part of starting the PC.
//!
//! $HEARTH_REMAP_FIXTURE (a JSON file: devices, state): CI's pictures; nothing is started or stopped then.

#[path = "../../oss-kit/kit.rs"]
mod kit;
#[path = "../../oss-kit/debs.rs"]
mod debs;
mod ir;
mod pad;
mod run;

use hearth_ui::app::{self, win};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, t, tb, ts, TitleMsgs, G};
use hearth_ui::Pal;
use ir::{Change, Ctl, Macro, Out, Step};
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{canvas, column, container, row, scrollable, slider, text_input, Column, Row, Space};
use iced::{Color, Element, Length, Subscription, Task};
use kit::{card, caps, chip, click, dot, pad, seg, switch, tag, Tag, E};
use serde_json::{json, Value};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Device(String),
    Profile(String),
    PickCtl(String),
    Pressed(u16),
    Edit(String),
    EditText(String),
    EditTo(Out),
    EditDone,
    EditCancel,
    Pop(Pop),
    CopyFrom(String),
    When(String),
    AddCtl(String),
    NewProfile(String),
    NewName(String),
    NewMake,
    Test,
    Pause,
    Dead(u8),
    Speed(u8),
    Released,
    Show,
    HoldOff,
    MacroPick(String),
    MacroMode(&'static str),
    Record,
    Key(KEvent),
    StepPick(usize),
    StepKind(&'static str),
    StepText(String),
    StepDelete,
    StepAdd,
    NewMacro,
    TryIt,
    StartService,
    /// Debian's own Input Remapper, installed from here (debs.rs)
    InstallIt,
    Done(Result<String, String>),
    Refresh,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pop {
    None,
    CopyFrom,
    When,
    AddCtl,
    NewProfile,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    settings: Value,
    devs: Vec<ir::Dev>,
    dev: String,
    presets: Vec<String>,
    preset: String,
    changes: Vec<Change>,
    kept: Vec<Value>,
    everywhere: Vec<Change>,
    macros: Vec<Macro>,
    macro_sel: String,
    picked: Option<Ctl>,
    editing: Option<Ctl>,
    edit_text: String,
    pop: Pop,
    new_name: String,
    step: Option<usize>,
    step_text: String,
    recording: bool,
    rec_last: Option<Instant>,
    on: String,
    installed: bool,
    running: bool,
    said: String,
    games: Vec<(String, String)>,
    ks: u32,
    /// where each macro is used ("X · BFME II"), worked out when presets change (never per frame)
    used: std::collections::HashMap<String, String>,
}

const MACRO_COLOURS: [&str; 6] = ["#ff8a3d", "#5ec8ff", "#9be36b", "#e36bd3", "#ffd36b", "#b04fc8"];

fn rc(s: &str) -> Color {
    kit::hex(s).unwrap_or(Color::from_rgb8(0xff, 0x8a, 0x3d))
}
fn fixture() -> bool {
    std::env::var("HEARTH_REMAP_FIXTURE").is_ok()
}

/// The games Steam has installed (their app manifests): id, name.
fn games() -> Vec<(String, String)> {
    let home = hearth_ui::paths::home();
    let mut out = Vec::new();
    for root in [home.join(".local/share/Steam/steamapps"), home.join(".steam/steam/steamapps")] {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !n.starts_with("appmanifest_") {
                continue;
            }
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            let field = |k: &str| text.lines().find(|l| l.trim_start().starts_with(&format!("\"{k}\""))).and_then(|l| l.split('"').nth(3)).unwrap_or("").to_string();
            let (id, name) = (field("appid"), field("name"));
            if !id.is_empty() && !name.is_empty() && !out.iter().any(|x: &(String, String)| x.0 == id) {
                out.push((id, name));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// Hearth's apps that can have a profile of their own (their window's app id).
const APPS: [(&str, &str); 6] = [("hearth-files", "Hearth Files"), ("hearth-notes", "Hearth Notes"), ("hearth-cad", "Hearth CAD"), ("firefox", "Firefox"), ("org.kde.konsole", "Konsole"), ("hearth-settings", "Settings")];

impl App {
    fn new() -> (App, Task<Msg>) {
        let settings = kit::read_settings(ir::APP);
        let macros = ir::macros_of(&settings);
        let devs = ir::devices();
        let dev = app::arg("--device").filter(|d| devs.iter().any(|x| &x.name == d)).or_else(|| devs.iter().find(|d| d.kind == "controller").map(|d| d.name.clone())).or_else(|| devs.first().map(|d| d.name.clone())).unwrap_or_default();
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("remap", "Hearth Remap").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            settings,
            devs,
            dev,
            presets: Vec::new(),
            preset: String::new(),
            changes: Vec::new(),
            kept: Vec::new(),
            everywhere: Vec::new(),
            macro_sel: macros.first().map(|m| m.id.clone()).unwrap_or_default(),
            macros,
            picked: None,
            editing: None,
            edit_text: String::new(),
            pop: Pop::None,
            new_name: String::new(),
            step: None,
            step_text: String::new(),
            recording: false,
            rec_last: None,
            on: String::new(),
            installed: ir::installed(),
            running: ir::running(),
            said: String::new(),
            games: games(),
            ks: ir::keystroke_ms(),
            used: Default::default(),
        };
        a.refresh_used();
        a.load_device(app::arg("--profile"));
        (a, Task::none())
    }

    fn key(&self, preset: &str) -> String {
        format!("{}/{}", self.dev, preset)
    }
    fn meta(&self, preset: &str) -> Value {
        self.settings.get("profiles").and_then(|p| p.get(self.key(preset))).cloned().unwrap_or_else(|| json!({}))
    }
    fn meta_num(&self, k: &str, d: u64) -> u8 {
        self.meta(&self.preset).get(k).and_then(Value::as_u64).unwrap_or(d).min(100) as u8
    }
    fn meta_bool(&self, k: &str) -> bool {
        self.meta(&self.preset).get(k).and_then(Value::as_bool).unwrap_or(true)
    }
    fn macro_code(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |id: &str| self.macros.iter().find(|m| m.id == id).map(Macro::compile)
    }
    fn macro_name(&self, id: &str) -> String {
        self.macros.iter().find(|m| m.id == id).map(|m| m.name.clone()).unwrap_or_else(|| "a macro".into())
    }

    fn load_device(&mut self, want: Option<String>) {
        self.presets = ir::presets(&self.dev);
        let order = |p: &String| if p == "Everywhere" { 0 } else { 1 };
        self.presets.sort_by(|a, b| order(a).cmp(&order(b)).then(a.cmp(b)));
        self.on = run::now_read().pointer(&format!("/on/{}", run::pointer_key(&self.dev))).and_then(Value::as_str).map(str::to_string).unwrap_or_default();
        if self.on.is_empty() && fixture() {
            if ir::state(&self.dev) == "RUNNING" {
                self.on = std::env::var("HEARTH_REMAP_ON").unwrap_or_default();
            }
        }
        let pick = want.filter(|w| self.presets.contains(w)).or_else(|| (!self.on.is_empty() && self.presets.contains(&self.on)).then(|| self.on.clone())).or_else(|| self.presets.first().cloned()).unwrap_or_default();
        self.load_preset(&pick);
    }

    fn load_preset(&mut self, name: &str) {
        self.preset = name.to_string();
        let ms = self.macros.clone();
        let of = |sym: &str| ms.iter().find(|m| m.compile() == sym).map(|m| m.id.clone());
        let (c, k) = if name.is_empty() { (Vec::new(), Vec::new()) } else { ir::read_preset(&self.dev, name, of) };
        self.changes = c;
        self.kept = k;
        self.everywhere = if name != "Everywhere" && self.presets.iter().any(|p| p == "Everywhere") { ir::read_preset(&self.dev, "Everywhere", of).0 } else { Vec::new() };
        self.picked = None;
        self.editing = None;
        // the macro card: this profile's first macro
        if let Some(id) = self.changes.iter().find_map(|c| if let Out::Macro(id) = &c.out { Some(id.clone()) } else { None }) {
            self.macro_sel = id;
        }
    }

    /// The preset written (and the running one told to load it again).
    fn write(&mut self) -> Task<Msg> {
        if self.preset.is_empty() {
            return Task::none();
        }
        let dz = self.meta_num("deadzone", 8) as f32 / 100.0;
        let sp = self.meta_num("speed", 60) as f32 / 100.0;
        let maps = ir::to_mappings(&self.changes, self.macro_code(), dz, sp);
        if let Err(e) = ir::write_preset(&self.dev, &self.preset, maps, &self.kept) {
            self.said = format!("Couldn't save the profile: {e}");
            return Task::none();
        }
        self.said.clear();
        self.refresh_used();
        // the runner turns profiles on by themselves from now on (only once the person has made a change)
        let runner: Task<Msg> = if fixture() {
            Task::none()
        } else {
            Task::perform(
                kit::bg(|| {
                    kit::user_unit("enable", "hearth-remap.service");
                    if !kit::user_unit_active("hearth-remap.service") {
                        kit::user_unit("start", "hearth-remap.service");
                    }
                    Ok::<String, String>(String::new())
                }),
                |r| Msg::Done(r.unwrap_or(Err("stopped".into()))),
            )
        };
        if self.on == self.preset {
            let (d, p) = (self.dev.clone(), self.preset.clone());
            return Task::batch([runner, Task::perform(kit::bg(move || ir::start(&d, &p).map(|_| String::new())), |r| Msg::Done(r.unwrap_or(Err("stopped".into()))))]);
        }
        runner
    }

    fn keep_meta(&mut self, k: &str, v: Value) {
        let key = self.key(&self.preset);
        let mut profiles = self.settings.get("profiles").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
        if !profiles.get(&key).map(Value::is_object).unwrap_or(false) {
            profiles[&key] = json!({});
        }
        profiles[&key][k] = v;
        self.settings["profiles"] = profiles.clone();
        if let Err(e) = kit::save(ir::APP, vec![("profiles", profiles)]) {
            self.said = format!("Couldn't keep that: {e}");
        }
    }
    /// The run slot for step `i` of the macro being changed: the one it has, else the first free one.
    fn slot_for(&self, i: usize) -> u8 {
        match self.sel_macro().and_then(|m| m.steps.get(i)) {
            Some(Step::Run(k, _)) => *k,
            _ => ir::free_slot(&self.macros).unwrap_or(ir::RUN_SLOTS),
        }
    }
    fn keep_macros(&mut self) {
        self.refresh_used();
        let v = ir::macros_json(&self.macros);
        self.settings["macros"] = v.clone();
        if let Err(e) = kit::save(ir::APP, vec![("macros", v)]) {
            self.said = format!("Couldn't keep that: {e}");
        }
    }
    fn sel_macro(&self) -> Option<&Macro> {
        self.macros.iter().find(|m| m.id == self.macro_sel)
    }
    fn change_of(&self, c: Ctl) -> Option<&Change> {
        self.changes.iter().find(|x| x.ctl == c)
    }
    /// Changed in this profile from Everywhere (the board's lit ones).
    fn lit(&self) -> Vec<Ctl> {
        self.changes.iter().filter(|c| self.preset == "Everywhere" || !self.everywhere.contains(c)).map(|c| c.ctl).collect()
    }
    fn when_words(&self, preset: &str) -> String {
        let m = self.meta(preset);
        let w = m.get("when").and_then(Value::as_str).unwrap_or(if preset == "Everywhere" { "everywhere" } else { "" });
        match w.split_once(':') {
            Some(("game", _)) => "when the game is in front".into(),
            Some(("app", c)) => format!("when {} is in front", APPS.iter().find(|a| a.0 == c).map(|a| a.1.trim_start_matches("Hearth ")).unwrap_or(c)),
            _ if w == "everywhere" => "when nothing else fits".into(),
            _ if w == "desktop" => m.get("about").and_then(Value::as_str).unwrap_or("in Desktop Mode").to_string(),
            _ => "only when you test it".into(),
        }
    }
    fn when_label(&self) -> String {
        let m = self.meta(&self.preset);
        let w = m.get("when").and_then(Value::as_str).unwrap_or(if self.preset == "Everywhere" { "everywhere" } else { "" }).to_string();
        let label = m.get("label").and_then(Value::as_str).map(str::to_string);
        match w.split_once(':') {
            Some(("game", id)) => format!("{} is in front", label.or_else(|| self.games.iter().find(|g| g.0 == id).map(|g| g.1.clone())).unwrap_or_else(|| "The game".into())),
            Some(("app", c)) => format!("{} is in front", APPS.iter().find(|a| a.0 == c).map(|a| a.1).unwrap_or(c)),
            _ if w == "everywhere" => "Nothing else fits".into(),
            _ if w == "desktop" => "In Desktop Mode".into(),
            _ => "Only when tested".into(),
        }
    }
}

fn key_symbol(k: &Key) -> Option<String> {
    Some(match k {
        Key::Character(c) => {
            let c = c.as_str();
            match c {
                " " => "space".into(),
                "." => "period".into(),
                "," => "comma".into(),
                "-" => "minus".into(),
                "/" => "slash".into(),
                _ if c.chars().count() == 1 => c.to_lowercase(),
                _ => return None,
            }
        }
        Key::Named(n) => match n {
            Named::Escape => "Escape".into(),
            Named::Enter => "Return".into(),
            Named::Space => "space".into(),
            Named::Tab => "Tab".into(),
            Named::Backspace => "BackSpace".into(),
            Named::Shift => "Shift_L".into(),
            Named::Control => "Control_L".into(),
            Named::Alt => "Alt_L".into(),
            Named::ArrowLeft => "Left".into(),
            Named::ArrowRight => "Right".into(),
            Named::ArrowUp => "Up".into(),
            Named::ArrowDown => "Down".into(),
            Named::Delete => "Delete".into(),
            other => {
                let s = format!("{other:?}");
                if s.starts_with('F') && s[1..].chars().all(|c| c.is_ascii_digit()) {
                    s
                } else {
                    return None;
                }
            }
        },
        _ => return None,
    })
}

/// What a typed key or combination means ("ctrl + 1" → Control_L + 1, "shift" → Shift_L).
fn sym_of(words: &str) -> String {
    words
        .split('+')
        .map(|p| {
            let p = p.trim();
            match p.to_lowercase().as_str() {
                "shift" => "Shift_L".to_string(),
                "ctrl" | "control" => "Control_L".to_string(),
                "alt" => "Alt_L".to_string(),
                "super" | "windows" => "Super_L".to_string(),
                "esc" | "escape" => "Escape".to_string(),
                "enter" | "return" => "Return".to_string(),
                "space" => "space".to_string(),
                "tab" => "Tab".to_string(),
                x if x.len() == 1 => x.to_string(),
                _ if p.len() <= 3 && p.to_uppercase().starts_with('F') => p.to_uppercase(),
                _ => p.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Device(d) => {
            a.dev = d;
            a.load_device(None);
        }
        Msg::Profile(p) => a.load_preset(&p),
        Msg::PickCtl(id) => {
            if let Some(c) = ir::ctl_of(&id) {
                a.picked = Some(c);
                if let Some(Out::Macro(mid)) = a.change_of(c).map(|x| x.out.clone()) {
                    a.macro_sel = mid;
                }
            }
        }
        Msg::Pressed(code) => {
            let c = match code {
                1000 | 1001 => Ctl::LeftStick,
                1003 | 1004 => Ctl::RightStick,
                1016 | 1017 => Ctl::Dpad,
                1002 => Ctl::Lt,
                1005 => Ctl::Rt,
                k if k < 1000 => Ctl::Btn(k),
                _ => return Task::none(),
            };
            return update(a, Msg::PickCtl(ir::ctl_id(c)));
        }
        Msg::Edit(id) => {
            if let Some(c) = ir::ctl_of(&id) {
                a.editing = Some(c);
                a.picked = Some(c);
                a.edit_text = match a.change_of(c).map(|x| x.out.clone()) {
                    Some(Out::Key(k)) => ir::words(&k),
                    _ => String::new(),
                };
                a.pop = Pop::None;
            }
        }
        Msg::EditText(s) => a.edit_text = s,
        Msg::EditTo(o) => {
            if let Some(c) = a.editing {
                a.changes.retain(|x| x.ctl != c);
                a.changes.push(Change { ctl: c, out: o });
                a.editing = None;
                return a.write();
            }
        }
        Msg::EditDone => {
            if let Some(c) = a.editing {
                let t = a.edit_text.trim().to_string();
                a.changes.retain(|x| x.ctl != c);
                if !t.is_empty() {
                    let out = if c == Ctl::Dpad {
                        let s = sym_of(&t);
                        Out::Four([s.clone(), s.clone(), s.clone(), s])
                    } else {
                        Out::Key(sym_of(&t))
                    };
                    a.changes.push(Change { ctl: c, out });
                }
                a.editing = None;
                return a.write();
            }
        }
        Msg::EditCancel => a.editing = None,
        Msg::Pop(p) => {
            a.pop = if a.pop == p { Pop::None } else { p };
            a.new_name.clear();
        }
        Msg::CopyFrom(p) => {
            let ms = a.macros.clone();
            let (c, _) = ir::read_preset(&a.dev, &p, |sym| ms.iter().find(|m| m.compile() == sym).map(|m| m.id.clone()));
            for ch in c {
                a.changes.retain(|x| x.ctl != ch.ctl);
                a.changes.push(ch);
            }
            a.pop = Pop::None;
            a.said = format!("Copied the changes from {p}.");
            return a.write();
        }
        Msg::When(w) => {
            let label = match w.split_once(':') {
                Some(("game", id)) => a.games.iter().find(|g| g.0 == id).map(|g| g.1.clone()),
                _ => None,
            };
            a.keep_meta("when", json!(w));
            if let Some(l) = label {
                a.keep_meta("label", json!(l));
                a.keep_meta("from", json!("your Steam library"));
            }
            a.pop = Pop::None;
        }
        Msg::AddCtl(id) => {
            a.pop = Pop::None;
            return update(a, Msg::Edit(id));
        }
        Msg::NewProfile(w) => {
            a.pop = Pop::NewProfile;
            a.new_name = match w.split_once(':') {
                Some(("game", id)) => a.games.iter().find(|g| g.0 == id).map(|g| g.1.clone()).unwrap_or_default(),
                Some(("app", c)) => APPS.iter().find(|x| x.0 == c).map(|x| x.1.to_string()).unwrap_or_default(),
                _ => String::new(),
            };
            a.settings["newWhen"] = json!(w);
        }
        Msg::NewName(s) => a.new_name = s.chars().filter(|c| *c != '/').take(60).collect(),
        Msg::NewMake => {
            let name = a.new_name.trim().to_string();
            if name.is_empty() || a.presets.contains(&name) {
                return Task::none();
            }
            let w = a.settings.get("newWhen").and_then(Value::as_str).unwrap_or("").to_string();
            if let Some(o) = a.settings.as_object_mut() {
                o.remove("newWhen");
            }
            if let Err(e) = ir::write_preset(&a.dev, &name, Vec::new(), &[]) {
                a.said = format!("Couldn't make it: {e}");
                return Task::none();
            }
            a.presets.push(name.clone());
            a.load_preset(&name);
            if !w.is_empty() {
                a.keep_meta("when", json!(w));
                if let Some(("game", _)) = w.split_once(':') {
                    a.keep_meta("label", json!(name));
                    a.keep_meta("from", json!("your Steam library"));
                }
            }
            a.pop = Pop::None;
        }
        Msg::Test => {
            let mut now = run::now_read();
            let testing = now.pointer("/test/preset").and_then(Value::as_str) == Some(a.preset.as_str());
            now["test"] = if testing { Value::Null } else { json!({"device": a.dev, "preset": a.preset}) };
            run::now_write(&now);
            a.said = if testing { "Test over: the profile turns on by itself again.".into() } else { format!("Testing {}: it's on now, whatever's in front. Press Test it again to stop.", a.preset) };
            if !testing {
                let (d, p) = (a.dev.clone(), a.preset.clone());
                a.on = p.clone();
                return Task::perform(kit::bg(move || ir::start(&d, &p).map(|_| String::new())), |r| Msg::Done(r.unwrap_or(Err("stopped".into()))));
            }
        }
        Msg::Pause => {
            let paused = !a.settings.get("paused").and_then(Value::as_bool).unwrap_or(false);
            a.settings["paused"] = json!(paused);
            if let Err(e) = kit::save(ir::APP, vec![("paused", json!(paused))]) {
                a.said = format!("Couldn't keep that: {e}");
            }
            if paused {
                let mut now = run::now_read();
                now["on"] = json!({});
                run::now_write(&now);
                a.on.clear();
                return Task::perform(kit::bg(|| ir::stop_all().map(|_| "Every remap is paused.".to_string())), |r| Msg::Done(r.unwrap_or(Err("stopped".into()))));
            }
            a.said = "Remaps are back: each profile turns on when its game or app is in front.".into();
        }
        Msg::Dead(v) => {
            a.keep_meta_soft("deadzone", v);
        }
        Msg::Speed(v) => {
            a.keep_meta_soft("speed", v);
        }
        Msg::Released => {
            let (dz, sp) = (a.meta_num("deadzone", 8), a.meta_num("speed", 60));
            a.keep_meta("deadzone", json!(dz));
            a.keep_meta("speed", json!(sp));
            return a.write();
        }
        Msg::Show => {
            let v = !a.meta_bool("show");
            a.keep_meta("show", json!(v));
        }
        Msg::HoldOff => {
            let v = !a.meta_bool("holdOff");
            a.keep_meta("holdOff", json!(v));
        }
        Msg::MacroPick(id) => {
            a.macro_sel = id;
            a.step = None;
        }
        Msg::MacroMode(mo) => {
            let sel = a.macro_sel.clone();
            if let Some(m) = a.macros.iter_mut().find(|m| m.id == sel) {
                m.mode = mo.to_string();
            }
            a.keep_macros();
            return a.write_all_using();
        }
        Msg::Record => {
            a.recording = !a.recording;
            a.rec_last = None;
            if a.recording {
                let sel = a.macro_sel.clone();
                if let Some(m) = a.macros.iter_mut().find(|m| m.id == sel) {
                    m.steps.clear();
                }
                a.said = "Recording: press the keys on your keyboard (the waits between them are kept). Press ● Record again to stop.".into();
            } else {
                a.said.clear();
                a.keep_macros();
                return a.write_all_using();
            }
        }
        Msg::Key(KEvent::KeyPressed { key, .. }) if a.recording => {
            let Some(sym) = key_symbol(&key) else { return Task::none() };
            let now = Instant::now();
            let wait = a.rec_last.map(|t| now.duration_since(t).as_millis() as u32).unwrap_or(0);
            a.rec_last = Some(now);
            let sel = a.macro_sel.clone();
            if let Some(m) = a.macros.iter_mut().find(|m| m.id == sel) {
                if wait > 30 {
                    m.steps.push(Step::Wait(((wait + 5) / 10 * 10).min(5000)));
                }
                m.steps.push(Step::Press(sym));
            }
        }
        Msg::Key(_) => {}
        Msg::StepPick(i) => {
            a.step = if a.step == Some(i) { None } else { Some(i) };
            a.step_text = a.sel_macro().and_then(|m| m.steps.get(i)).map(step_value).unwrap_or_default();
        }
        Msg::StepKind(k) => {
            let (sel, i) = (a.macro_sel.clone(), a.step.unwrap_or(usize::MAX));
            let txt = a.step_text.clone();
            let slot = a.slot_for(i);
            if let Some(s) = a.macros.iter_mut().find(|m| m.id == sel).and_then(|m| m.steps.get_mut(i)) {
                *s = make_step(k, &txt, slot);
            }
            a.keep_macros();
            return a.write_all_using();
        }
        Msg::StepText(v) => {
            a.step_text = v;
            let (sel, i) = (a.macro_sel.clone(), a.step.unwrap_or(usize::MAX));
            let txt = a.step_text.clone();
            let slot = a.slot_for(i);
            if let Some(s) = a.macros.iter_mut().find(|m| m.id == sel).and_then(|m| m.steps.get_mut(i)) {
                *s = make_step(s.kind(), &txt, slot);
            }
        }
        Msg::StepDelete => {
            let (sel, i) = (a.macro_sel.clone(), a.step.unwrap_or(usize::MAX));
            if let Some(m) = a.macros.iter_mut().find(|m| m.id == sel) {
                if i < m.steps.len() {
                    m.steps.remove(i);
                }
            }
            a.step = None;
            a.keep_macros();
            return a.write_all_using();
        }
        Msg::StepAdd => {
            let sel = a.macro_sel.clone();
            if let Some(m) = a.macros.iter_mut().find(|m| m.id == sel) {
                m.steps.push(Step::Wait(50));
                a.step = Some(m.steps.len() - 1);
                a.step_text = "50".into();
            }
            a.keep_macros();
            return a.write_all_using();
        }
        Msg::NewMacro => {
            let n = a.macros.len() + 1;
            let mut id = format!("macro-{n}");
            let mut k = n;
            while a.macros.iter().any(|m| m.id == id) {
                k += 1;
                id = format!("macro-{k}");
            }
            a.macros.push(Macro { id: id.clone(), name: format!("Macro {n}"), colour: MACRO_COLOURS[(n - 1) % 6].into(), mode: "once".into(), steps: vec![Step::Press("a".into())] });
            a.macro_sel = id;
            a.keep_macros();
        }
        Msg::TryIt => {
            // Input Remapper runs macros only from a button: this profile goes on now so its button tries it
            let on = a.changes.iter().find(|c| c.out == Out::Macro(a.macro_sel.clone())).map(|c| ir::ctl_name(c.ctl));
            a.said = match &on {
                Some(b) => format!("Press {b} on the controller to try {}.", a.sel_macro().map(|m| m.name.clone()).unwrap_or_default()),
                None => "Give the macro a button first (Edit on the right), then press it to try it.".into(),
            };
            if on.is_some() {
                let (d, p) = (a.dev.clone(), a.preset.clone());
                a.on = p.clone();
                return Task::perform(kit::bg(move || ir::start(&d, &p).map(|_| String::new())), |r| Msg::Done(r.unwrap_or(Err("stopped".into()))));
            }
        }
        Msg::InstallIt => {
            a.said = "Getting Debian's Input Remapper…".into();
            return Task::perform(kit::bg(|| debs::input_remapper().map(|_| "Input Remapper is on this PC now.".to_string()).map_err(|e| format!("It couldn't be installed: {e}"))), |r| Msg::Done(r.unwrap_or(Err("stopped".into()))));
        }
        Msg::StartService => {
            a.said = "Turning Input Remapper on…".into();
            return Task::perform(
                kit::bg(|| {
                    let o = hearth_ui::run::blocking(&hearth_ui::run::args(&["pkexec", "systemctl", "enable", "--now", "input-remapper.service"]));
                    if o.ok { Ok("Input Remapper is on.".to_string()) } else { Err(format!("It couldn't be turned on: {}", o.why())) }
                }),
                |r| Msg::Done(r.unwrap_or(Err("stopped".into()))),
            )
            .chain(Task::done(Msg::Refresh));
        }
        Msg::Done(r) => match r {
            Ok(s) => {
                if !s.is_empty() {
                    a.said = s;
                }
            }
            Err(e) => a.said = e,
        },
        Msg::Refresh => {
            a.running = ir::running();
            a.installed = ir::installed();
            a.devs = ir::devices();
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

impl App {
    /// A slider moving: shown as it moves (kept when it's let go).
    fn keep_meta_soft(&mut self, k: &str, v: u8) {
        let key = self.key(&self.preset);
        if !self.settings.get("profiles").map(Value::is_object).unwrap_or(false) {
            self.settings["profiles"] = json!({});
        }
        if !self.settings["profiles"].get(&key).map(Value::is_object).unwrap_or(false) {
            self.settings["profiles"][&key] = json!({});
        }
        self.settings["profiles"][&key][k] = json!(v);
    }
    /// A macro changed: every profile of this device that uses it is written again.
    fn write_all_using(&mut self) -> Task<Msg> {
        let cur = self.preset.clone();
        let sel = self.macro_sel.clone();
        let mut t = Vec::new();
        for p in self.presets.clone() {
            self.load_preset_keep_sel(&p);
            if self.changes.iter().any(|c| c.out == Out::Macro(sel.clone())) {
                t.push(self.write());
            }
        }
        self.load_preset_keep_sel(&cur);
        self.macro_sel = sel;
        Task::batch(t)
    }
    fn load_preset_keep_sel(&mut self, p: &str) {
        let sel = self.macro_sel.clone();
        let step = self.step;
        self.load_preset(p);
        self.macro_sel = sel;
        self.step = step;
    }
}

fn step_value(s: &Step) -> String {
    match s {
        Step::Press(k) | Step::Hold(k) | Step::LetGo(k) => ir::words(k),
        Step::Type(x) => x.clone(),
        Step::Click(b) => b.to_string(),
        Step::Move(d, n) => format!("{d} {n}"),
        Step::Scroll(d) => d.to_string(),
        Step::Wait(ms) => ms.to_string(),
        Step::Run(_, c) => c.clone(),
    }
}
fn make_step(kind: &str, v: &str, slot: u8) -> Step {
    let v = v.trim();
    match kind {
        "run a command" => Step::Run(slot, v.to_string()),
        "press" => Step::Press(sym_of(if v.is_empty() { "a" } else { v })),
        "hold" => Step::Hold(sym_of(if v.is_empty() { "shift" } else { v })),
        "let go" => Step::LetGo(sym_of(if v.is_empty() { "shift" } else { v })),
        "type words" => Step::Type(v.to_string()),
        "click" => Step::Click(if v.contains("right") { "right" } else { "left" }),
        "move the pointer" => {
            let d = ["up", "down", "left", "right"].into_iter().find(|d| v.contains(d)).unwrap_or("right");
            Step::Move(d, v.split_whitespace().find_map(|w| w.parse().ok()).unwrap_or(10))
        }
        "scroll" => Step::Scroll(if v.contains("up") { "up" } else { "down" }),
        _ => Step::Wait(v.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(50)),
    }
}

// ------------------------------------------------------------------ the window

fn side_row<'a>(p: &Pal, on: bool, lead: E<'a, Msg>, name: String, sub: String, m: Msg) -> E<'a, Msg> {
    let r = row![lead, column![t(name, 13.0, p.text), t(sub, 11.0, p.muted)].spacing(2.0).width(Length::Fill)].spacing(10.0).align_y(iced::Center);
    let body = container(r).padding(pad(9.0, 10.0, 9.0, 10.0)).width(Length::Fill);
    let el: E<'a, Msg> = if on {
        container(row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(48.0)).style(ui::boxed(p.accent, 0.0, None)), body].height(Length::Shrink)).clip(true).style(ui::boxed(Color::from_rgb8(0x3b, 0x2a, 0x1d), 9.0, None)).into()
    } else {
        body.into()
    };
    click(el, Some(m))
}

fn devices_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(2.0).push(container(caps(&p, "Devices")).padding(pad(2.0, 0.0, 8.0, 0.0)));
    for d in &a.devs {
        let on = d.name == a.dev;
        let lead = match d.kind.as_str() {
            "mouse" => dot(if on { p.accent } else { Color::from_rgb8(0xe0, 0xd8, 0xd0) }, 16.0),
            _ => kit::square(if on { p.accent } else { Color::from_rgb8(0xe0, 0xd8, 0xd0) }, 16.0, 4.0, None),
        };
        c = c.push(side_row(&p, on, lead, d.name.clone(), d.sub.clone(), Msg::Device(d.name.clone())));
    }
    if a.devs.is_empty() {
        c = c.push(container(t("No keyboard, mouse or controller found.", 12.0, p.muted)).padding(10.0));
    }
    c.into()
}

fn profiles_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let kind = a.devs.iter().find(|d| d.name == a.dev).map(|d| d.kind.as_str()).unwrap_or("controller");
    let mut c = Column::new().spacing(2.0).push(container(caps(&p, format!("Profiles for this {kind}"))).padding(pad(14.0, 0.0, 8.0, 0.0)));
    for (i, pr) in a.presets.iter().enumerate() {
        let on = *pr == a.preset;
        let colour = if pr == "Everywhere" { p.muted } else { rc(MACRO_COLOURS[(i + 5) % 6]) };
        let colour = if on { p.accent } else { colour };
        c = c.push(side_row(&p, on, dot(colour, 16.0), pr.clone(), a.when_words(pr), Msg::Profile(pr.clone())));
    }
    c = c.push(container(chip(&p, "＋ Profile for a game or app", Some(Msg::Pop(Pop::NewProfile)))).padding(pad(8.0, 0.0, 0.0, 0.0)));
    if a.pop == Pop::NewProfile {
        c = c.push(new_profile(a));
    }
    c = c.push(container(click(row![ui::glyph(G::Pencil, p.muted, 12.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(6.0).align_y(iced::Center), Some(Msg::Describe(hearth_ui::arrange::edit_layout())))).padding(pad(14.0, 10.0, 0.0, 10.0)));
    scrollable(c).height(Length::Fill).into()
}

fn new_profile(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0).push(t("For which game or app?", 12.0, p.muted));
    for (id, name) in a.games.iter().take(8) {
        c = c.push(click(t(name.clone(), 12.0, p.text), Some(Msg::NewProfile(format!("game:{id}")))));
    }
    for (id, name) in APPS.iter().take(3) {
        c = c.push(click(t(*name, 12.0, p.text), Some(Msg::NewProfile(format!("app:{id}")))));
    }
    c = c.push(click(t("Desktop Mode", 12.0, p.text), Some(Msg::NewProfile("desktop".into()))));
    c = c.push(text_input("Its name", &a.new_name).on_input(Msg::NewName).on_submit(Msg::NewMake).size(12.0).padding(pad(6.0, 8.0, 6.0, 8.0)).style(kit::field_style(&p)));
    c = c.push(row![kit::chip_acc(&p, "Make it", Some(Msg::NewMake)), chip(&p, "Cancel", Some(Msg::Pop(Pop::NewProfile)))].spacing(6.0));
    container(c).padding(10.0).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0)))).into()
}

fn missing(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    if !a.installed {
        return Some(kit::missing(&p, "Input Remapper isn't on this PC yet", "Hearth Remap is built on Input Remapper (GPL-3), which does the remapping. Install Debian's own Input Remapper from here (it asks for the administrator's password once; Hearth OS keeps it through its updates).", Some(("Install Input Remapper".into(), Msg::InstallIt)), false));
    }
    if !a.running {
        return Some(kit::missing(&p, "Input Remapper is off", "Turn it on and your profiles work (it asks once, as administrator). It never holds up starting the PC: if it fails, nothing else waits for it.", Some(("Turn Input Remapper on".into(), Msg::StartService)), false));
    }
    None
}

fn head_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let m = a.meta(&a.preset);
    let w = m.get("when").and_then(Value::as_str).unwrap_or(if a.preset == "Everywhere" { "everywhere" } else { "" }).to_string();
    let mut line = match w.split_once(':') {
        Some(("game", _)) => "Turns on by itself when the game is in front, off when you leave it".to_string(),
        Some(("app", _)) => "Turns on by itself when the app is in front, off when you leave it".to_string(),
        _ if w == "everywhere" => "On whenever nothing else fits".to_string(),
        _ if w == "desktop" => "Turns on by itself in Desktop Mode".to_string(),
        _ => "Only on while you test it".to_string(),
    };
    if let Some(f) = m.get("from").and_then(Value::as_str) {
        line.push_str(&format!(" · from {f}"));
    }
    let on = !a.preset.is_empty() && a.on == a.preset;
    let paused = a.settings.get("paused").and_then(Value::as_bool).unwrap_or(false);
    let status = if paused { tag(&p, "Paused", Tag::Warn) } else if on { tag(&p, "On", Tag::Ok) } else { tag(&p, "Off", Tag::Dark) };
    let mut r = row![column![tb(if a.preset.is_empty() { "No profile yet".to_string() } else { a.preset.clone() }, 26.0, p.text), t(line, 12.0, p.muted)].spacing(4.0).width(Length::Fill), status, chip(&p, "Copy from…", Some(Msg::Pop(Pop::CopyFrom))), chip(&p, "Test it", Some(Msg::Test))].spacing(8.0).align_y(iced::Center);
    if a.preset.is_empty() {
        r = row![tb("No profile yet", 26.0, p.text), Space::new().width(Length::Fill), chip(&p, "＋ Profile for a game or app", Some(Msg::Pop(Pop::NewProfile)))].align_y(iced::Center);
    }
    let mut c = Column::new().spacing(8.0).push(r);
    if a.pop == Pop::CopyFrom {
        let mut l = Row::new().spacing(6.0).push(t("Copy the changes from:", 12.0, p.muted));
        for pr in a.presets.iter().filter(|x| **x != a.preset) {
            l = l.push(chip(&p, pr.clone(), Some(Msg::CopyFrom(pr.clone()))));
        }
        c = c.push(l);
    }
    if !a.said.is_empty() {
        c = c.push(t(a.said.clone(), 12.0, kit::YELLOW));
    }
    c.into()
}

fn picture_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    if let Some(m) = missing(a) {
        return card(&p, container(m).center_y(Length::Fill)).height(Length::Fixed(450.0)).into();
    }
    let lit = a.lit();
    let calls: Vec<pad::Callout> = a
        .changes
        .iter()
        .filter(|c| !matches!(c.ctl, Ctl::Btn(314) | Ctl::Btn(315) | Ctl::Btn(316) | Ctl::Btn(317) | Ctl::Btn(318)))
        .map(|c| {
            let notes = a.meta(&a.preset).pointer(&format!("/notes/{}", ir::ctl_id(c.ctl))).cloned().unwrap_or(Value::Null);
            let base = ir::out_words(&c.out, |id| a.macro_name(id));
            let what = match (notes.get("says").and_then(Value::as_str), notes.get("note").and_then(Value::as_str)) {
                (Some(s), _) => s.to_string(),
                (None, Some(n)) => format!("{base} ({n})"),
                _ if c.ctl == Ctl::Dpad => format!("{base}{}", if base.contains(" · ") { " (groups)" } else { "" }),
                _ => base,
            };
            let name = if c.ctl == Ctl::Dpad { "D-pad".to_string() } else { ir::ctl_name(c.ctl) };
            pad::Callout { ctl: c.ctl, name, what, lit: lit.contains(&c.ctl) }
        })
        .collect();
    let kind = a.devs.iter().find(|d| d.name == a.dev).map(|d| d.kind.clone()).unwrap_or_default();
    let hint = if kind == "controller" { "Press a button on the controller to pick it · lit ones are changed in this profile" } else { "Click a button in the picture to pick it · lit ones are changed in this profile" };
    let body = column![canvas(pad::Pad { calls, picked: a.picked, lit }).width(Length::Fill).height(Length::Fill), container(t(hint, 12.0, p.muted)).padding(pad(0.0, 20.0, 14.0, 20.0))];
    container(body).height(Length::Fixed(450.0)).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into()
}

fn macro_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let Some(m) = a.sel_macro() else {
        return card(&p, column![ts("Macros", 18.0, p.text), t("A macro is a few steps one button does: press, hold, let go, type words, click, move the pointer, scroll, wait.", 12.0, p.muted), chip(&p, "＋ New macro", Some(Msg::NewMacro))].spacing(10.0)).height(Length::Fill).into();
    };
    let on_btn = a.changes.iter().find(|c| c.out == Out::Macro(m.id.clone())).map(|c| ir::ctl_name(c.ctl));
    let how = match m.mode.as_str() {
        "held" => "repeats while held",
        "toggle" => "first press on, next press off",
        _ => "runs once per press",
    };
    let sub = format!("{}{how} · {:.1} s", on_btn.map(|b| format!("on {b} · ")).unwrap_or_default(), m.seconds(a.ks));
    let modes = seg(&p, vec![("Once".into(), m.mode == "once", Some(Msg::MacroMode("once"))), ("While held".into(), m.mode == "held", Some(Msg::MacroMode("held"))), ("Turn on/off".into(), m.mode == "toggle", Some(Msg::MacroMode("toggle")))]);
    let rec = chip(&p, if a.recording { "■ Stop recording" } else { "● Record" }, Some(Msg::Record));
    let head = row![ts(format!("Macro · {}", m.name), 18.0, p.text), t(sub, 12.0, p.muted), Space::new().width(Length::Fill), modes, rec].spacing(10.0).align_y(iced::Center);
    // the steps, one after another
    let mut steps = Row::new().spacing(8.0).align_y(iced::Center);
    for (i, s) in m.steps.iter().enumerate() {
        let (w, key) = s.words();
        let n = container(t((i + 1).to_string(), 10.0, p.muted)).center_x(Length::Fixed(20.0)).center_y(Length::Fixed(20.0)).style(ui::boxed(p.bg, 10.0, None));
        let mut r = row![n, t(w, 13.0, p.text)].spacing(8.0).align_y(iced::Center);
        if let Some(k) = key {
            r = r.push(keycap(&p, k, false));
        }
        let on = a.step == Some(i);
        steps = steps.push(click(container(r).padding(pad(9.0, 12.0, 9.0, 12.0)).style(ui::boxed(p.panel2, 10.0, Some((if on { p.accent } else { p.line }, 1.0)))), Some(Msg::StepPick(i))));
        if i + 1 < m.steps.len() {
            steps = steps.push(t("→", 13.0, p.muted));
        }
    }
    let add = click(container(t("＋ Step", 13.0, p.muted)).padding(pad(9.0, 12.0, 9.0, 12.0)).style(ui::boxed(Color::TRANSPARENT, 10.0, Some((p.line, 1.0)))), Some(Msg::StepAdd));
    let mut c = Column::new().spacing(14.0).push(head).push(scrollable(row![steps, add].spacing(8.0).align_y(iced::Center)).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(4.0).scroller_width(4.0))));
    if let Some(i) = a.step {
        if let Some(s) = m.steps.get(i) {
            let kinds = seg(&p, ir::STEP_KINDS.iter().map(|k| (k.to_string(), s.kind() == *k, Some(Msg::StepKind(*k)))).collect());
            c = c.push(row![kinds, text_input("what", &a.step_text).on_input(Msg::StepText).on_submit(Msg::StepPick(i)).size(12.0).width(Length::Fixed(160.0)).padding(pad(6.0, 8.0, 6.0, 8.0)).style(kit::field_style(&p)), chip(&p, "Delete step", Some(Msg::StepDelete))].spacing(8.0).align_y(iced::Center));
        }
    }
    // your macros
    c = c.push(container(caps(&p, format!("Your macros · {}", a.macros.len()))).padding(pad(8.0, 0.0, 0.0, 0.0)));
    let mut grid = Column::new().spacing(10.0);
    let mut cells: Vec<E<'_, Msg>> = Vec::new();
    for mc in &a.macros {
        let used = a.used.get(&mc.id).cloned().unwrap_or_else(|| "no button yet".into());
        let on = mc.id == a.macro_sel;
        let r = row![kit::square(rc(&mc.colour), 16.0, 4.0, None), t(mc.name.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(used, 12.0, p.muted)].spacing(8.0).align_y(iced::Center);
        cells.push(click(container(r).padding(pad(9.0, 12.0, 9.0, 12.0)).width(Length::Fill).style(ui::boxed(p.panel2, 10.0, Some((if on { p.accent } else { p.line }, 1.0)))), Some(Msg::MacroPick(mc.id.clone()))));
    }
    cells.push(click(container(t("＋ New macro", 13.0, p.muted)).padding(pad(9.0, 12.0, 9.0, 12.0)).width(Length::Fill).style(ui::boxed(Color::TRANSPARENT, 10.0, Some((p.line, 1.0)))), Some(Msg::NewMacro)));
    let mut it = cells.into_iter();
    loop {
        let mut r = Row::new().spacing(10.0);
        let mut any = false;
        for _ in 0..3 {
            match it.next() {
                Some(e) => {
                    any = true;
                    r = r.push(container(e).width(Length::FillPortion(1)));
                }
                None => r = r.push(Space::new().width(Length::FillPortion(1))),
            }
        }
        if !any {
            break;
        }
        grid = grid.push(r);
    }
    c = c.push(grid);
    c = c.push(row![t("Steps can be: press, hold, let go, type words, click, move the pointer, scroll, wait, run a command", 12.0, p.muted), Space::new().width(Length::Fill), chip(&p, "Try it", Some(Msg::TryIt))].spacing(16.0).align_y(iced::Center));
    card(&p, scrollable(c).height(Length::Fill)).height(Length::Fill).into()
}

impl App {
    fn refresh_used(&mut self) {
        let ids: Vec<String> = self.macros.iter().map(|m| m.id.clone()).collect();
        let mut m = std::collections::HashMap::new();
        for id in ids {
            let w = used_where(self, &id);
            m.insert(id, w);
        }
        self.used = m;
    }
}

/// "X · BFME II", "Mouse 5 · Files", "View · everywhere": where a macro is used.
fn used_where(a: &App, id: &str) -> String {
    let mut out = Vec::new();
    for d in &a.devs {
        for pr in ir::presets(&d.name) {
            let ms = a.macros.clone();
            let (c, _) = ir::read_preset(&d.name, &pr, |sym| ms.iter().find(|m| m.compile() == sym).map(|m| m.id.clone()));
            if let Some(ch) = c.iter().find(|c| c.out == Out::Macro(id.to_string())) {
                let short = a.settings.pointer(&format!("/profiles/{}/short", run::pointer_key(&format!("{}/{pr}", d.name)))).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| if pr == "Everywhere" { "everywhere".into() } else { pr.clone() });
                let b = if d.kind == "keyboard" { format!("Keyboard {}", ir::ctl_name(ch.ctl)) } else { ir::ctl_name(ch.ctl) };
                out.push(if d.kind == "keyboard" { b } else { format!("{b} · {short}") });
            }
        }
    }
    out.first().cloned().unwrap_or_else(|| "no button yet".into())
}

fn keycap<'a>(p: &Pal, k: String, accent: bool) -> E<'a, Msg> {
    let c = if accent { p.accent } else { p.line };
    container(tb(k, 12.0, if accent { p.accent } else { p.text }).wrapping(iced::widget::text::Wrapping::None))
        .padding(pad(5.0, 9.0, 5.0, 9.0))
        .style(ui::boxed(p.bg, 7.0, Some((c, 1.0))))
        .into()
}

fn changes_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().push(row![ts("All changes", 18.0, p.text), Space::new().width(Length::Fill), t(format!("{} in this profile", a.changes.len()), 12.0, p.muted)].align_y(iced::Center));
    let lit = a.lit();
    let mut list = Column::new();
    let mut shown: Vec<&Change> = a.changes.iter().collect();
    let order = |c: &Ctl| ir::CONTROLS.iter().position(|x| x.0 == *c).unwrap_or(99);
    shown.sort_by_key(|c| order(&c.ctl));
    for ch in shown {
        let notes = a.meta(&a.preset).pointer(&format!("/notes/{}", ir::ctl_id(ch.ctl))).cloned().unwrap_or(Value::Null);
        let mut what = match &ch.out {
            Out::Four(k) => ir::words(&k[0]),
            o => ir::out_words(o, |id| a.macro_name(id)),
        };
        if let (Some(n), true) = (notes.get("note").and_then(Value::as_str), notes.get("says").is_some()) {
            what = format!("{what} ({n})");
        }
        let on = a.picked == Some(ch.ctl) || (lit.contains(&ch.ctl) && matches!(ch.out, Out::Macro(_)));
        let id = ir::ctl_id(ch.ctl);
        let row_el = row![
            container(keycap(&p, ir::ctl_name(ch.ctl), on)).width(Length::Fixed(150.0)),
            container(t("→", 13.0, p.muted)).width(Length::Fixed(22.0)),
            container(t(what, 13.0, p.text)).width(Length::Fill),
            chip(&p, "Edit", Some(Msg::Edit(id.clone())))
        ]
        .spacing(10.0)
        .align_y(iced::Center);
        list = list.push(click(container(row_el).padding(pad(8.0, 0.0, 8.0, 0.0)), Some(Msg::PickCtl(id))));
        list = list.push(ui::hline(Color::from_rgb8(0x2c, 0x26, 0x22)));
        if a.editing == Some(ch.ctl) {
            list = list.push(editor(a, ch.ctl));
        }
    }
    if let Some(c2) = a.editing.filter(|c| a.change_of(*c).is_none()) {
        list = list.push(editor(a, c2));
    }
    c = c.push(container(list).padding(pad(10.0, 0.0, 0.0, 0.0)));
    c = c.push(container(chip(&p, "＋ Change another button", Some(Msg::Pop(Pop::AddCtl)))).padding(pad(12.0, 0.0, 0.0, 0.0)));
    if a.pop == Pop::AddCtl {
        let mut r = Column::new().spacing(6.0);
        let mut line = Row::new().spacing(6.0);
        let mut n = 0;
        for (ctl, name) in ir::CONTROLS.iter().filter(|x| a.change_of(x.0).is_none()) {
            line = line.push(chip(&p, *name, Some(Msg::AddCtl(ir::ctl_id(*ctl)))));
            n += 1;
            if n % 4 == 0 {
                r = r.push(line);
                line = Row::new().spacing(6.0);
            }
        }
        r = r.push(line);
        c = c.push(container(r).padding(pad(8.0, 0.0, 0.0, 0.0)));
    }
    c.into()
}

fn editor(a: &App, c: Ctl) -> E<'_, Msg> {
    let p = a.p;
    let mut opts = Row::new().spacing(6.0);
    let stick = matches!(c, Ctl::LeftStick | Ctl::RightStick);
    if stick {
        opts = opts.push(chip(&p, "Mouse pointer", Some(Msg::EditTo(Out::Pointer)))).push(chip(&p, "Arrow keys", Some(Msg::EditTo(Out::Arrows))));
    } else {
        opts = opts.push(chip(&p, "Left click", Some(Msg::EditTo(Out::Click("left"))))).push(chip(&p, "Right click", Some(Msg::EditTo(Out::Click("right")))));
    }
    let mut macros = Row::new().spacing(6.0);
    if !stick && c != Ctl::Dpad {
        for m in &a.macros {
            macros = macros.push(chip(&p, format!("Macro: {}", m.name), Some(Msg::EditTo(Out::Macro(m.id.clone())))));
        }
    }
    let field = text_input(if c == Ctl::Dpad { "a key, like Ctrl + 1" } else { "a key or combination, like Shift or Ctrl + 1" }, &a.edit_text).on_input(Msg::EditText).on_submit(Msg::EditDone).size(12.0).padding(pad(6.0, 8.0, 6.0, 8.0)).style(kit::field_style(&p));
    let col = column![
        t(format!("{} does:", ir::ctl_name(c)), 12.0, p.muted),
        row![field, kit::chip_acc(&p, "Save", Some(Msg::EditDone)), chip(&p, "Cancel", Some(Msg::EditCancel))].spacing(6.0).align_y(iced::Center),
        opts,
        scrollable(macros).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(3.0).scroller_width(3.0))),
        t("More: any key, a macro, a click. Leave it empty and Save to put the button back as it was.", 11.0, p.muted),
    ]
    .spacing(8.0);
    container(col).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, 8.0, Some((p.accent, 1.0)))).into()
}

fn profile_block(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let sl = |v: u8, m: fn(u8) -> Msg| -> E<'static, Msg> { slider(0.0..=100.0, v as f32, move |x| m(x.round() as u8)).on_release(Msg::Released).width(Length::Fixed(120.0)).style(kit::slider_style(&p)).into() };
    let num = |v: u8| -> E<'static, Msg> { container(t(format!("{v} %"), 12.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right).into() };
    let (dz, sp) = (a.meta_num("deadzone", 8), a.meta_num("speed", 60));
    let field = click(container(t(format!("{} ▾", a.when_label()), 12.0, p.text)).padding(pad(7.0, 10.0, 7.0, 10.0)).width(Length::Fixed(190.0)).style(ui::boxed(p.bg, 8.0, Some((p.line, 1.0)))), Some(Msg::Pop(Pop::When)));
    let mut c = column![
        caps(&p, "This profile"),
        kit::line(&p, "Turns on when", field),
    ]
    .spacing(10.0);
    if a.pop == Pop::When {
        let mut l = Column::new().spacing(5.0);
        l = l.push(click(t("Nothing else fits (Everywhere)", 12.0, p.text), Some(Msg::When("everywhere".into()))));
        l = l.push(click(t("In Desktop Mode", 12.0, p.text), Some(Msg::When("desktop".into()))));
        for (id, name) in a.games.iter().take(12) {
            l = l.push(click(t(format!("{name} is in front"), 12.0, p.text), Some(Msg::When(format!("game:{id}")))));
        }
        for (id, name) in APPS {
            l = l.push(click(t(format!("{name} is in front"), 12.0, p.text), Some(Msg::When(format!("app:{id}")))));
        }
        l = l.push(click(t("Only when I test it", 12.0, p.text), Some(Msg::When(String::new()))));
        c = c.push(container(scrollable(l).height(Length::Fixed(180.0))).padding(10.0).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0)))));
    }
    c = c.push(row![t("Stick dead zone", 13.0, p.text), Space::new().width(Length::Fill), sl(dz, Msg::Dead), num(dz)].spacing(10.0).align_y(iced::Center));
    c = c.push(row![t("Pointer speed", 13.0, p.text), Space::new().width(Length::Fill), sl(sp, Msg::Speed), num(sp)].spacing(10.0).align_y(iced::Center));
    c = c.push(kit::line(&p, "Show what changed on screen", switch(&p, a.meta_bool("show"), Some(Msg::Show))));
    c = c.push(kit::line(&p, "Hold ≡ for 3 s to turn remaps off", switch(&p, a.meta_bool("holdOff"), Some(Msg::HoldOff))));
    c.into()
}

// ------------------------------------------------------------------ its blocks
const BLOCKS: &[PageDef] = &[PageDef {
    id: "main",
    title: "Hearth Remap",
    blocks: &[
        bl("devices", "Devices", 0, 0, 2, 5),
        bl("profiles", "Profiles", 0, 5, 2, 7),
        bl("head", "The profile's name and buttons", 2, 0, 7, 1),
        bl("picture", "The controller", 2, 1, 7, 5),
        bl("macro", "Macros", 2, 6, 7, 6),
        bl("changes", "All changes", 9, 0, 3, 7),
        bl("profile", "This profile", 9, 7, 3, 5),
    ],
}];

const KIT: Kit = Kit {
    name: "Remap",
    buttons: &[
        tile("pause", "Pause all remaps", G::Stop),
        tile("test", "Test it", G::Play),
        tile("copy", "Copy from…", G::Doc),
        tile("change", "Change another button", G::Plus),
        tile("new-macro", "New macro", G::Plus),
        tile("record", "Record a macro", G::Camera),
        tile("try", "Try the macro", G::Play),
        tile("new-profile", "Profile for a game or app", G::Pad),
    ],
    panes: &[tile("macros", "Your macros", G::List), tile("changes", "The changes", G::Columns)],
    info: &[tile("profile", "The profile", G::User), tile("changes", "Changes", G::Info), tile("macros", "Macros", G::List), tile("device", "The device", G::Pad)],
    pane_page: "main",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "pause" => Msg::Pause,
        "test" => Msg::Test,
        "copy" => Msg::Pop(Pop::CopyFrom),
        "change" => Msg::Pop(Pop::AddCtl),
        "new-macro" => Msg::NewMacro,
        "record" => Msg::Record,
        "try" => Msg::TryIt,
        "new-profile" => Msg::Pop(Pop::NewProfile),
        _ => return None,
    })
}

fn pane(a: &App, which: &str) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    if which == "macros" {
        for m in &a.macros {
            c = c.push(click(row![kit::square(rc(&m.colour), 12.0, 3.0, None), t(m.name.clone(), 13.0, p.text)].spacing(8.0).align_y(iced::Center), Some(Msg::MacroPick(m.id.clone()))));
        }
    } else {
        for ch in &a.changes {
            c = c.push(click(row![t(ir::ctl_name(ch.ctl), 13.0, p.text), Space::new().width(Length::Fill), t(ir::out_words(&ch.out, |id| a.macro_name(id)), 12.0, p.muted)].spacing(8.0), Some(Msg::PickCtl(ir::ctl_id(ch.ctl)))));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    a.describe.values(vec![("profile", a.preset.clone()), ("changes", a.changes.len().to_string()), ("macros", a.macros.len().to_string()), ("device", a.dev.clone())]);
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![
        ("devices", hearth_ui::studio::made(&a.describe, "main", "devices", || devices_block(a))),
        ("profiles", profiles_block(a)),
        ("head", head_block(a)),
        ("picture", picture_block(a)),
        ("macro", macro_block(a)),
        ("changes", changes_block(a)),
        ("profile", profile_block(a)),
    ];
    for (id, which) in [("pane:macros", "macros"), ("pane:changes", "changes")] {
        if a.describe.wants_pane(which) {
            items.push((id, pane(a, which)));
        }
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "main",
        items,
        move |ps| {
            let side = container(column![ps.take("devices"), container(ps.take("profiles")).height(Length::Fill)]).width(Length::Fixed(270.0)).height(Length::Fill).padding(pad(14.0, 12.0, 14.0, 12.0)).style(ui::boxed(Color::from_rgba8(20, 17, 15, 0.5), 0.0, None));
            let main = column![ps.take("head"), ps.take("picture"), container(ps.take("macro")).height(Length::Fill)].spacing(16.0).padding(pad(20.0, 24.0, 20.0, 24.0)).width(Length::Fill);
            let right = container(scrollable(column![ps.take("changes"), ps.take("profile")].spacing(22.0)).height(Length::Fill)).width(Length::Fixed(420.0)).height(Length::Fill).padding(pad(18.0, 20.0, 18.0, 20.0)).style(ui::boxed(Color::from_rgba8(20, 17, 15, 0.5), 0.0, None));
            row![side, ui::vline(p.line), main, ui::vline(p.line), right].height(Length::Fill).into()
        },
        Msg::Describe,
    );
    let paused = a.settings.get("paused").and_then(Value::as_bool).unwrap_or(false);
    let bar = kit::title_bar(
        &p,
        "RM",
        Color::from_rgb8(0xff, 0xe2, 0x8a),
        Color::from_rgb8(0xf0, 0xa0, 0x20),
        "Hearth Remap",
        "any key, mouse or controller button, your way",
        vec![chip(&p, if paused { "Resume remaps" } else { "Pause all remaps" }, Some(Msg::Pause))],
        TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close },
        Msg::Describe(hearth_ui::describe::DMsg::Open),
    );
    let all = container(column![bar, container(body).height(Length::Fill), kit::credit(&p, "Hearth Remap", "Input Remapper (GPL-3)")]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

/// The picked controller's own button presses (its event device; a joystick, which the person at the PC may read).
fn pad_events(path: &String) -> futures::channel::mpsc::UnboundedReceiver<u16> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let p = path.clone();
    let _ = std::thread::Builder::new().name("hearth-remap-pick".into()).stack_size(64 * 1024).spawn(move || {
        use std::io::Read;
        let Ok(mut f) = std::fs::File::open(format!("/dev/input/{p}")) else { return };
        let mut ev = [0u8; 24];
        while f.read_exact(&mut ev).is_ok() {
            let ty = u16::from_le_bytes([ev[16], ev[17]]);
            let code = u16::from_le_bytes([ev[18], ev[19]]);
            let val = i32::from_le_bytes([ev[20], ev[21], ev[22], ev[23]]);
            let send = match ty {
                1 if val == 1 => Some(code),
                3 if matches!(code, 16 | 17) && val != 0 => Some(1000 + code),
                3 if matches!(code, 0 | 1 | 3 | 4) && val.abs() > 24000 => Some(1000 + code),
                3 if matches!(code, 2 | 5) && val > 200 => Some(1000 + code),
                _ => None,
            };
            if let Some(c) = send {
                if tx.unbounded_send(c).is_err() {
                    break;
                }
            }
        }
    });
    rx
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    if a.recording {
        s.push(iced::keyboard::listen().map(Msg::Key));
    }
    if let Some(d) = a.devs.iter().find(|d| d.name == a.dev && d.kind == "controller" && !d.event.is_empty()) {
        s.push(Subscription::run_with(d.event.clone(), pad_events).map(Msg::Pressed));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if let Some(n) = app::arg("--run-step") {
        std::process::exit(run::run_step(n.parse().unwrap_or(0)));
    }
    if app::flag("--run") {
        std::process::exit(run::run_forever());
    }
    if app::handle_cli("hearth-remap-app", "Hearth Remap: any key, mouse or controller button, your way (built on Input Remapper).\n  --run-step N       run the command of a macro's step N (labwc's keybind for Super+Ctrl+Alt+F12+N)\n  --run              turn profiles on when their game or app is in front (hearth-remap.service)\n  --device NAME      open with that device\n  --profile NAME     and that profile") {
        return Ok(());
    }
    let w = app::window("hearth-remap", (1840.0, 970.0), (1100.0, 700.0));
    iced::application(App::new, update, shown)
        .title(|_: &App| "Hearth Remap".to_string())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(w)
        .default_font(app::INTER)
        .run()
}
