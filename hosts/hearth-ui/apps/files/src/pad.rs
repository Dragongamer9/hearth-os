//! The controller and Hearth's on-screen keyboard inside the Open / Save window in Game Mode (Mantel's picker.qml with
//! HEARTH_PICKER_PAD=1, which hearth-portal sets when the window opens over a game): the window starts its own
//! hearth-pad (desktop/shell/hearth-pad, the one controller helper), which works the window with the controller (A
//! Enter, B Esc, the D-pad and stick, LB / RB…) and types through its uinput keyboard; Y opens Hearth's keyboard,
//! drawn here at the bottom of the window by the shared engine (ui/osk/hearth-osk.js, the same as everywhere: same
//! layouts, words, accents, Ember Wheel, osk.json, learned words in osk-words.json written the safe way).
//!
//! Light: hearth-pad is one program read on a thread of its own (blocked on its pipe); the keyboard's engine only
//! gets frames while a button is held or a stick is tilted. In Desktop Mode the desktop's own hearth-pad drives the
//! window, as for every app, and none of this starts.

use crate::app::{App, Msg};
use crate::looks;
use crate::osk;
use futures::channel::mpsc;
use iced::Task;
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock};

static STDIN: OnceLock<Mutex<Option<ChildStdin>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<ChildStdin>> {
    STDIN.get_or_init(|| Mutex::new(None))
}

pub fn helper() -> String {
    std::env::var("HEARTH_PAD_HELPER").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| hearth_ui::run::shell_helper("hearth-pad"))
}

/// Game Mode's Open / Save window: hearth-portal asked for the controller here.
pub fn wanted() -> bool {
    std::env::var("HEARTH_PICKER_PAD").ok().as_deref() == Some("1")
}

/// A command to hearth-pad (dropped when it isn't running, as picker.qml's pad.send does).
pub fn send(v: Value) {
    let Ok(mut g) = slot().lock() else { return };
    if let Some(w) = g.as_mut() {
        let mut s = v.to_string();
        s.push('\n');
        if w.write_all(s.as_bytes()).and_then(|_| w.flush()).is_err() {
            *g = None;
        }
    }
}

/// hearth-pad's events for as long as the window is open (it's stopped when the window closes: its stdin closes).
pub fn stream() -> mpsc::UnboundedReceiver<Value> {
    let (tx, rx) = mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-files-pad".into()).stack_size(256 * 1024).spawn(move || {
        let child = Command::new("python3").arg(helper()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
        let Ok(mut child) = child else { return };
        if let Ok(mut g) = slot().lock() {
            *g = child.stdin.take();
        }
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { continue };
                if tx.unbounded_send(v).is_err() {
                    let _ = child.kill();
                    break;
                }
            }
        }
        if let Ok(mut g) = slot().lock() {
            *g = None;
        }
        let _ = child.wait();
    });
    rx
}

#[derive(Default)]
pub struct Pad {
    pub osk: Option<osk::Osk>,
    pub vm: Option<osk::Vm>,
    pub look: Option<looks::Look>,
    pub err: String,
    pub kb: bool,
    pub busy: bool,
    desk_theme: String,
    save: Option<Value>,
    save_gen: u64,
}

impl std::fmt::Debug for Pad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pad {{ kb: {} }}", self.kb)
    }
}

#[derive(Debug, Clone)]
pub enum PMsg {
    Ev(Value),
    Key(usize, usize),
    Sugg(usize),
    Alt(usize),
    Tick,
    Save(u64),
}

fn engine(a: &mut App, js: &str) -> Task<Msg> {
    let Some(o) = a.pad.osk.as_ref() else { return Task::none() };
    let step = o.call(js);
    apply(a, step)
}

fn apply(a: &mut App, step: osk::Step) -> Task<Msg> {
    if !step.err.is_empty() {
        eprintln!("hearth-file-chooser: keyboard: {}", step.err.lines().next().unwrap_or(""));
    }
    a.pad.busy = step.busy;
    let mut tasks = vec![];
    for o in step.out {
        match o.get("k").and_then(Value::as_str).unwrap_or("") {
            "emit" => send(json!({ "cmd": "emit", "text": o.get("text").cloned().unwrap_or(json!("")), "mods": o.get("mods").cloned().unwrap_or(json!([])) })),
            "key" => send(json!({ "cmd": "key", "key": o.get("key").cloned().unwrap_or(json!("")), "n": o.get("n").cloned().unwrap_or(json!(1)), "mods": o.get("mods").cloned().unwrap_or(json!([])) })),
            "rumble" => send(json!({ "cmd": "rumble", "strength": o.get("strength").cloned().unwrap_or(json!(0.3)), "ms": o.get("ms").cloned().unwrap_or(json!(20)) })),
            "hide" => close_kb(a),
            "save" => {
                a.pad.save = o.get("d").cloned();
                a.pad.save_gen += 1;
                let g = a.pad.save_gen;
                tasks.push(Task::perform(hearth_ui::run::after(1500), move |_| Msg::Pad(PMsg::Save(g))));
            }
            _ => {}
        }
    }
    if let Some(v) = step.vm {
        a.pad.vm = Some(osk::Vm::from(&v));
        restyle(a);
    }
    Task::batch(tasks)
}

fn restyle(a: &mut App) {
    let engine_theme = a.pad.vm.as_ref().map(|v| v.theme.clone()).unwrap_or_default();
    a.pad.look = Some(looks::look(looks::resolve(&a.pad.desk_theme, &engine_theme), &a.p));
}

/// Y on the controller (hearth-pad says "osk"), or CI's picture: the keyboard opens at the bottom of the window.
pub fn open_kb(a: &mut App) -> Task<Msg> {
    if a.pad.osk.is_none() && a.pad.err.is_empty() {
        match osk::Osk::new() {
            Ok(o) => a.pad.osk = Some(o),
            Err(e) => {
                eprintln!("hearth-file-chooser: the keyboard engine didn't load: {e}");
                a.pad.err = e;
            }
        }
    }
    let cfg = osk::read_obj(&osk::config_file()).unwrap_or_default();
    a.pad.desk_theme = cfg.get("deskTheme").and_then(Value::as_str).unwrap_or("").to_string();
    let mut t = Task::none();
    if let Some(o) = a.pad.osk.as_mut() {
        o.prepare(&cfg);
        let step = o.call("__e.reset('text', 'Done'); __e.setHints(true);");
        t = apply(a, step);
    }
    restyle(a);
    if !a.pad.kb {
        a.pad.kb = true;
        send(json!({ "cmd": "osk", "open": true }));
    }
    t
}

pub fn close_kb(a: &mut App) {
    if a.pad.kb {
        a.pad.kb = false;
        a.pad.busy = false;
        send(json!({ "cmd": "osk", "open": false }));
    }
}

fn save_learned(a: &mut App) {
    let Some(Value::Object(d)) = a.pad.save.take() else { return };
    let path = osk::learned_file();
    let mut m = Map::new();
    for k in ["version", "learned", "next", "recentEmoji"] {
        if let Some(v) = d.get(k) {
            m.insert(k.to_string(), v.clone());
        }
    }
    let _ = std::thread::Builder::new().name("hearth-files-osk-save".into()).spawn(move || {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = hearth_ui::settings_file::merge_json(&path, &m) {
            eprintln!("hearth-file-chooser: couldn't keep the learned words: {e}");
        }
    });
}

pub fn update(a: &mut App, m: PMsg) -> Task<Msg> {
    match m {
        PMsg::Ev(v) => match v.get("t").and_then(Value::as_str).unwrap_or("") {
            // picker.qml: once hearth-pad is ready, it types keys into this window and moves its pointer.
            "ready" => {
                send(json!({ "cmd": "keys", "on": true }));
                send(json!({ "cmd": "mouse", "on": true }));
            }
            "osk" => {
                if v.get("open").and_then(Value::as_bool).unwrap_or(false) {
                    return open_kb(a);
                }
                close_kb(a);
            }
            "oskpad" => {
                if a.pad.kb {
                    let b = v.get("b").cloned().unwrap_or(json!(""));
                    let down = v.get("down").and_then(Value::as_bool).unwrap_or(false);
                    return engine(a, &format!("__e.setHints(true); __e.handle({b}, {down}, Date.now());"));
                }
            }
            "oskaxes" => {
                if a.pad.kb {
                    let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                    return engine(a, &format!("__e.axes({}, {}, {}, {}, Date.now());", f("lx"), f("ly"), f("rx"), f("ry")));
                }
            }
            _ => {}
        },
        PMsg::Key(r, c) => return engine(a, &format!("__e.setHints(false); __e.tap({r}, {c});")),
        PMsg::Sugg(i) => return engine(a, &format!("__e.setHints(false); __e.tap(-1, {i});")),
        PMsg::Alt(i) => return engine(a, &format!("__e.pickAlt({i});")),
        PMsg::Tick => return engine(a, "__e.tick(Date.now());"),
        PMsg::Save(g) => {
            if g == a.pad.save_gen {
                save_learned(a);
            }
        }
    }
    Task::none()
}

/// The window is closing: hearth-pad stops typing into it (picker.qml's quit()).
pub fn quit() {
    send(json!({ "cmd": "keys", "on": false }));
}
