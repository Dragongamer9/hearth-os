//! Hearth's on-screen keyboard engine (ui/osk/hearth-osk.js, copied to desktop/shell/osk by tools/osk-sync.py) run
//! as it is in QuickJS, as Mantel's Keyboard.qml runs it in QML: the same layouts, word lists, suggestions, accents,
//! Ember Wheel and split sticks, the same settings (~/.config/nimbus/osk.json, read only) and learned words
//! (~/.local/share/nimbus/osk-words.json, shared with Game Mode's keyboard and written the safe way). The engine says
//! what to type (emit / key, sent on to hearth-pad, which types through its uinput keyboard: KWin has no wtype) and
//! what to draw (engine.view(), parsed into [`Vm`]).

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// What one call into the engine gave back.
#[derive(Debug, Default)]
pub struct Step {
    /// emit {text, mods}, key {key, n, mods}, hide, rumble {strength, ms}, save {d}
    pub out: Vec<Value>,
    /// engine.view() when it changed
    pub vm: Option<Value>,
    /// a button is held or the stick is tilted: the engine wants tick() (repeats, accents, stick moves)
    pub busy: bool,
    pub err: String,
}

// The glue: the adapter queues what to type, onChange marks the view dirty, and every call returns both as JSON.
const GLUE: &str = r#"
var __o = [], __d = true, __e = null;
function __m(m) { var a = []; if (m) { if (m.ctrl) a.push('ctrl'); if (m.alt) a.push('alt'); if (m.shift) a.push('shift'); } return a; }
HearthOSK.configure({ save: function (d) { __o.push({ k: 'save', d: d }); } });
__e = HearthOSK.createEngine({
  host: 'mantel',
  adapter: {
    insert: function (t, m) { __o.push({ k: 'emit', text: t, mods: __m(m) }); },
    deleteBack: function (n) { __o.push({ k: 'key', key: 'bksp', n: n || 1, mods: [] }); },
    key: function (n, m) { __o.push({ k: 'key', key: n, n: 1, mods: __m(m) }); }
  },
  onChange: function () { __d = true; },
  onHide: function () { __o.push({ k: 'hide' }); },
  onHaptic: function (s, ms) { __o.push({ k: 'rumble', strength: s, ms: ms }); }
});
function __step(f) {
  var err = '';
  try { if (f) f(); } catch (e) { err = String((e && e.stack) || e); }
  var r = { out: __o, err: err, busy: false };
  try { var E = __e.state; r.busy = Object.keys(E.held).length > 0 || !!E.stickNav; } catch (e) { }
  if (__d) { try { r.vm = __e.view(); } catch (e) { r.err += String(e); } __d = false; }
  __o = [];
  return JSON.stringify(r);
}
function __langs() {
  var out = [], ids = HearthOSK.activeLayouts();
  for (var i = 0; i < ids.length; i++) for (var j = 0; j < HearthOSK.LAYOUTS.length; j++)
    if (HearthOSK.LAYOUTS[j].id === ids[i] && out.indexOf(HearthOSK.LAYOUTS[j].lang) < 0) out.push(HearthOSK.LAYOUTS[j].lang);
  return JSON.stringify(out);
}
"#;

pub struct Osk {
    _rt: rquickjs::Runtime,
    cx: rquickjs::Context,
    dir: PathBuf,
    words: Vec<String>,
    learned_loaded: bool,
}

/// Where the engine and its word lists are: $HEARTH_SHELL_DIR/osk (CI), else the image's desktop/shell/osk.
pub fn osk_dir() -> PathBuf {
    hearth_ui::run::shell_dir().join("osk")
}

fn home() -> PathBuf {
    hearth_ui::paths::home()
}
pub fn config_file() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("nimbus/osk.json")
}
pub fn learned_file() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("nimbus/osk-words.json")
}

/// A JSON file's object, read only (nothing is ever written back from here; a broken file reads as nothing).
pub fn read_obj(p: &Path) -> Option<Map<String, Value>> {
    let t = std::fs::read_to_string(p).ok()?;
    match serde_json::from_str::<Value>(&t) {
        Ok(Value::Object(m)) => Some(m),
        _ => None,
    }
}

/// The system's keyboard layouts, in order. Plasma keeps them in kxkbrc ([Layout] LayoutList, Fedora's KDE); Mantel
/// reads labwc's XKB_DEFAULT_LAYOUT, which is the fallback here.
pub fn system_layouts() -> Vec<String> {
    let kx = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config")).join("kxkbrc");
    if let Ok(t) = std::fs::read_to_string(kx) {
        let mut in_layout = false;
        for l in t.lines() {
            let l = l.trim();
            if l.starts_with('[') {
                in_layout = l == "[Layout]";
            } else if in_layout {
                if let Some(v) = l.strip_prefix("LayoutList=") {
                    let v: Vec<String> = v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                    if !v.is_empty() {
                        return v;
                    }
                }
            }
        }
    }
    std::env::var("XKB_DEFAULT_LAYOUT").unwrap_or_default().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

impl Osk {
    /// The engine, or why it couldn't be loaded.
    pub fn new() -> Result<Osk, String> {
        let dir = osk_dir();
        let src = std::fs::read_to_string(dir.join("hearth-osk.js")).map_err(|e| format!("{}: {e}", dir.join("hearth-osk.js").display()))?;
        let rt = rquickjs::Runtime::new().map_err(|e| e.to_string())?;
        rt.set_memory_limit(64 << 20);
        let cx = rquickjs::Context::full(&rt).map_err(|e| e.to_string())?;
        let ok: Result<(), String> = cx.with(|ctx| {
            ctx.eval::<(), _>(src).map_err(|e| format!("hearth-osk.js: {e}"))?;
            ctx.eval::<(), _>(GLUE).map_err(|e| format!("glue: {e}"))?;
            Ok(())
        });
        ok?;
        Ok(Osk { _rt: rt, cx, dir, words: Vec::new(), learned_loaded: false })
    }

    fn eval_str(&self, code: &str) -> Result<String, String> {
        self.cx.with(|ctx| ctx.eval::<String, _>(code.to_string()).map_err(|e| e.to_string()))
    }

    /// Runs `body` (JavaScript, with the engine as __e) and returns what came of it.
    pub fn call(&self, body: &str) -> Step {
        let code = format!("__step(function () {{ {body} }})");
        match self.eval_str(&code) {
            Ok(s) => {
                let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
                Step {
                    out: v.get("out").and_then(Value::as_array).cloned().unwrap_or_default(),
                    vm: v.get("vm").filter(|x| x.is_object()).cloned(),
                    busy: v.get("busy").and_then(Value::as_bool).unwrap_or(false),
                    err: v.get("err").and_then(Value::as_str).unwrap_or("").to_string(),
                }
            }
            Err(e) => Step { err: e, ..Step::default() },
        }
    }

    /// Before each opening, as Keyboard.qml does: the settings from osk.json, the system's layouts, the learned words
    /// (once; after that the engine's copy is the newest) and the word lists of the layouts in use.
    pub fn prepare(&mut self, settings: &Map<String, Value>) {
        let mut c = json!({ "settings": settings, "systemLayouts": system_layouts() });
        if !self.learned_loaded {
            self.learned_loaded = true;
            if let Some(d) = read_obj(&learned_file()) {
                c["learned"] = json!({
                    "learned": d.get("learned").cloned().unwrap_or(json!({})),
                    "next": d.get("next").cloned().unwrap_or(json!({})),
                    "recentEmoji": d.get("recentEmoji").cloned().unwrap_or(json!([])),
                });
            }
        }
        let _ = self.call(&format!("HearthOSK.configure({c});"));
        let langs: Vec<String> = self.eval_str("__langs()").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        for l in langs {
            if self.words.contains(&l) || l.len() != 2 || !l.chars().all(|c| c.is_ascii_lowercase()) {
                continue;
            }
            self.words.push(l.clone());
            if let Ok(t) = std::fs::read_to_string(self.dir.join("words").join(format!("{l}.js"))) {
                // The file is one call: HearthOSK.words("<lang>", "<words>").
                let _ = self.cx.with(|ctx| ctx.eval::<(), _>(t));
            }
        }
    }

    pub fn clipboard(&self, items: &[String]) -> Step {
        self.call(&format!("HearthOSK.configure({{ clipboard: {} }});", json!(items)))
    }
}

// ---------------------------------------------------------------------------------------------- what to draw

#[derive(Debug, Clone, Default)]
pub struct Key {
    pub t: String,
    pub v: String,
    pub label: String,
    pub w: f32,
    pub on: bool,
    pub lock: bool,
    pub go: bool,
    pub off: bool,
    pub emoji: bool,
    pub clip: bool,
    pub half: String,
    pub row: i64,
    pub col: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Vm {
    pub panel: String,
    pub rows: Vec<Vec<Key>>,
    pub sel: (i64, i64),
    pub active: bool,
    pub sugg: Vec<String>,
    pub shift: bool,
    pub caps: bool,
    pub popup: Option<(Vec<String>, i64)>,
    /// (active, petal, petals)
    pub wheel: Option<(bool, i64, Vec<String>)>,
    pub split: Option<(Option<(i64, i64)>, Option<(i64, i64)>)>,
    pub layout_short: String,
    pub hints: bool,
    pub glyphs: Map<String, Value>,
    pub size: String,
    pub shape: String,
    pub fast: String,
    pub position: String,
    pub theme: String,
    /// the ember glides with its sparks (osk.json trail, on unless turned off)
    pub trail: bool,
}

fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
fn rc(v: Option<&Value>) -> Option<(i64, i64)> {
    let v = v?;
    Some((v.get("r")?.as_i64()?, v.get("c")?.as_i64()?))
}
fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default()
}

impl Vm {
    pub fn from(v: &Value) -> Vm {
        let rows = v
            .get("rows")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|r| {
                        r.as_array()
                            .map(|ks| {
                                ks.iter()
                                    .map(|k| Key {
                                        t: s(k, "t"),
                                        v: s(k, "v"),
                                        label: s(k, "label"),
                                        w: k.get("w").and_then(Value::as_f64).unwrap_or(1.0).max(0.05) as f32,
                                        on: b(k, "on"),
                                        lock: b(k, "lock"),
                                        go: b(k, "go"),
                                        off: b(k, "off"),
                                        emoji: b(k, "emoji"),
                                        clip: b(k, "clip"),
                                        half: s(k, "half"),
                                        row: k.get("row").and_then(Value::as_i64).unwrap_or(0),
                                        col: k.get("col").and_then(Value::as_i64).unwrap_or(0),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let st = v.get("settings").cloned().unwrap_or(Value::Null);
        let popup = v.get("popup").filter(|p| p.is_object()).map(|p| (strs(p.get("alts")), p.get("i").and_then(Value::as_i64).unwrap_or(-1)));
        let wheel = v.get("wheel").filter(|p| p.is_object()).map(|w| (b(w, "active"), w.get("petal").and_then(Value::as_i64).unwrap_or(-1), strs(w.get("petals"))));
        let split = v.get("split").filter(|p| p.is_object()).map(|sp| (rc(sp.get("l")), rc(sp.get("r"))));
        Vm {
            panel: s(v, "panel"),
            rows,
            sel: rc(v.get("sel")).unwrap_or((1, 0)),
            active: b(v, "active"),
            sugg: strs(v.get("suggestions")),
            shift: b(v, "shift"),
            caps: b(v, "caps"),
            popup,
            wheel,
            split,
            layout_short: s(v, "layoutShort"),
            hints: b(v, "hints"),
            glyphs: v.get("glyphs").and_then(Value::as_object).cloned().unwrap_or_default(),
            size: s(&st, "size"),
            shape: s(&st, "shape"),
            fast: s(&st, "fast"),
            position: s(&st, "position"),
            theme: s(&st, "theme"),
            trail: st.get("trail").and_then(Value::as_bool) != Some(false),
        }
    }

    /// The key's text (OskBoard.qml's label()).
    pub fn label(&self, k: &Key) -> String {
        if k.t == "ch" {
            return if (self.shift || self.caps) && k.v != "ß" { k.v.to_uppercase() } else { k.v.clone() };
        }
        if k.t == "shift" && self.caps {
            return "⇪".into();
        }
        k.label.clone()
    }
    /// The controller button shown on the key (OskBoard.qml's glyph()).
    pub fn glyph(&self, k: &Key) -> String {
        if !self.hints || self.glyphs.is_empty() || (k.t == "shift" && k.v == "r") {
            return String::new();
        }
        if self.fast == "split" {
            if k.t == "shift" {
                return "LB".into();
            }
            if k.t == "sym" || k.t == "abc" {
                return "RB".into();
            }
            if k.t == "left" || k.t == "right" {
                return String::new();
            }
        }
        self.glyphs.get(&k.t).and_then(Value::as_str).unwrap_or("").to_string()
    }
    pub fn split_shape(&self) -> bool {
        self.shape == "split" && self.panel != "num"
    }
}

/// The keys that are drawn as modifier keys (OskBoard.qml's modKeys).
pub const MOD_KEYS: [&str; 25] = [
    "bksp", "enter", "shift", "sym", "abc", "sym2", "emoji", "clip", "layout", "left", "right", "up", "down", "hide", "tab", "esc", "ctrl", "alt",
    "home", "end", "paste", "prev", "next", "page", "cat",
];
