//! KWin's own settings, as Mantel's KwinSettings.qml / KwinGate.qml read and change them: through
//! hearth-kwin-settings (status, get, set FILE with {group: {key: value or null}} on its stdin), which writes only the
//! keys changed here (kwinrc, kdeglobals, kaccessrc) and asks KWin to read them again. Nothing is written when a page
//! opens. Changes show at once and are saved together half a second later (a slider moves in small steps).
//!
//! Fedora edition: KDE Plasma runs on KWin, so the helper is told HEARTH_WM=kwin (kit::helper_env) and these pages
//! are live there; Mantel on KWin (Debian) sets it itself. Anywhere else they show greyed, with why.
//!
//! And the five pages of Personalization that are KWin's (SetKwinEffects/Desktops/Tiling/Edges/Windows.qml).

use super::kit::{self, dropdown, iact, irow_if, pick_dialog, slide, small, Ic};
use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, t, t1, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Element, Length, Padding, Task};
use serde_json::{json, Map, Value};

type E<'a> = Element<'a, KMsg>;

#[derive(Default)]
pub struct Kw {
    pub st: Option<Value>,
    pub files: Map<String, Value>,
    pub bad: Map<String, Value>,
    pending: Map<String, Value>,
    flushing: bool,
    pub saving: u32,
    pub msg: String,
    pub msg_bad: bool,
    pub asking: bool,
    pub pick: Option<String>,
    // Virtual desktops
    pub ds: Value,
    pub names: Vec<String>,
    // Tiling
    pub outs: Vec<Value>,
    pub layouts: Map<String, Value>,
    pub padding: f32,
    pub screen: usize,
    pub skip: usize,
    tiles_due: bool,
    gap_due: bool,
}

#[derive(Debug, Clone)]
pub enum KMsg {
    Status(run::Output),
    Got(run::Output),
    Set(&'static str, Value),
    Flush,
    Saved(String, run::Output),
    Shortcut(&'static str),
    Did(run::Output),
    Open(String),
    Close,
    // Virtual desktops
    DesktopsGot(run::Output),
    Desktops(Value),
    Name(usize, String),
    NameSave(usize),
    // Tiling
    Outputs(run::Output),
    Screen(usize),
    Preset(usize),
    ColWidth(usize, f32),
    ColRows(usize, usize),
    ColRemove(usize),
    ColAdd,
    Gap(f32),
    TilesFlush,
    TilesSaved(run::Output),
    Go(&'static str),
}

fn helper(args: &[&str]) -> Vec<String> {
    kit::job("hearth-kwin-settings", args)
}

impl Kw {
    /// Reads KWin's status (and, when it's live, its settings). `page` asks for that page's own reads too.
    pub fn open(&mut self, page: &str) -> Task<KMsg> {
        self.msg.clear();
        let mut t = vec![];
        if self.st.is_none() && !self.asking {
            self.asking = true;
            t.push(Task::perform(kit::run_in(helper(&["status"]), String::new()), KMsg::Status));
        } else if self.live() {
            t.push(Task::perform(kit::run_in(helper(&["get"]), String::new()), KMsg::Got));
        }
        match page {
            "kwin-desktops" => t.push(Task::perform(kit::run_in(helper(&["desktops"]), String::new()), KMsg::DesktopsGot)),
            "kwin-tiling" => t.push(Task::perform(kit::run_in(helper(&["outputs"]), String::new()), KMsg::Outputs)),
            _ => {}
        }
        Task::batch(t)
    }

    pub fn ready(&self) -> bool {
        self.st.is_some()
    }
    pub fn live(&self) -> bool {
        self.st.as_ref().map(|s| kit::b_of(s, "live")).unwrap_or(false)
    }
    pub fn running(&self) -> bool {
        self.st.as_ref().map(|s| kit::b_of(s, "running")).unwrap_or(false)
    }
    pub fn preview(&self) -> bool {
        self.st.as_ref().map(|s| kit::b_of(s, "preview")).unwrap_or(false)
    }
    pub fn version(&self) -> String {
        self.st.as_ref().map(|s| kit::s_of(s, "version")).unwrap_or_default()
    }
    /// KWin's version as a number (6.3 → 603), 0 when not known.
    pub fn ver(&self) -> u32 {
        let v = self.version();
        let mut it = v.split('.');
        match (it.next().and_then(|x| x.parse::<u32>().ok()), it.next().and_then(|x| x.parse::<u32>().ok())) {
            (Some(a), Some(b)) => a * 100 + b,
            _ => 0,
        }
    }

    // ---- reading (values are KDE's text: "true", "4", "0.5")
    pub fn v(&self, file: &str, group: &str, key: &str) -> Option<String> {
        self.files.get(file).and_then(|f| f.get(group)).and_then(|g| g.get(key)).map(|x| match x {
            Value::String(s) => s.clone(),
            o => o.to_string(),
        })
    }
    pub fn s(&self, file: &str, group: &str, key: &str, d: &str) -> String {
        self.v(file, group, key).unwrap_or_else(|| d.to_string())
    }
    pub fn b(&self, file: &str, group: &str, key: &str, d: bool) -> bool {
        match self.v(file, group, key) {
            Some(x) if !x.is_empty() => x.eq_ignore_ascii_case("true") || x == "1",
            _ => d,
        }
    }
    pub fn n(&self, file: &str, group: &str, key: &str, d: f64) -> f64 {
        self.v(file, group, key).and_then(|x| x.trim().parse::<f64>().ok()).unwrap_or(d)
    }
    pub fn list(&self, file: &str, group: &str, key: &str, d: &[i64]) -> Vec<i64> {
        match self.v(file, group, key) {
            Some(x) => x.split(',').filter_map(|s| s.trim().parse::<i64>().ok()).collect(),
            None => d.to_vec(),
        }
    }
    /// An effect is on: what you chose, else KWin's own default (status.defaults).
    pub fn effect_on(&self, id: &str) -> bool {
        let d = self.st.as_ref().and_then(|s| s.get("defaults")).and_then(|d| d.get(id)).and_then(Value::as_bool).unwrap_or(false);
        self.b("kwinrc", "Plugins", &format!("{id}Enabled"), d)
    }
    /// KWin has this effect (when KWin can be asked; otherwise every KWin 6 effect is offered).
    pub fn has_effect(&self, id: &str) -> bool {
        match self.st.as_ref().and_then(|s| s.get("effects")).and_then(Value::as_array) {
            Some(l) => l.iter().any(|x| x.as_str() == Some(id)),
            None => true,
        }
    }
    pub fn has_script(&self, id: &str) -> bool {
        match self.st.as_ref().and_then(|s| s.get("scripts")).and_then(Value::as_array) {
            Some(l) => l.iter().any(|x| x.as_str() == Some(id)),
            None => true,
        }
    }

    fn take(&mut self, g: &Value) {
        self.files = g.get("files").and_then(Value::as_object).cloned().unwrap_or_default();
        self.bad = g.get("bad").and_then(Value::as_object).cloned().unwrap_or_default();
    }

    /// Shown at once; saved with the others half a second later.
    fn set_many(&mut self, file: &'static str, groups: &Value) -> Task<KMsg> {
        let Some(gs) = groups.as_object() else { return Task::none() };
        let f = self.files.entry(file.to_string()).or_insert_with(|| json!({}));
        let pend = self.pending.entry(file.to_string()).or_insert_with(|| json!({}));
        for (g, kv) in gs {
            let Some(kv) = kv.as_object() else { continue };
            if !f.get(g).map(|x| x.is_object()).unwrap_or(false) {
                f[g] = json!({});
            }
            if !pend.get(g).map(|x| x.is_object()).unwrap_or(false) {
                pend[g] = json!({});
            }
            for (k, val) in kv {
                if let Some(fo) = f[g].as_object_mut() {
                    match val {
                        Value::Null => {
                            fo.remove(k);
                        }
                        Value::Bool(b) => {
                            fo.insert(k.clone(), Value::String(if *b { "true".into() } else { "false".into() }));
                        }
                        Value::String(s) => {
                            fo.insert(k.clone(), Value::String(s.clone()));
                        }
                        o => {
                            fo.insert(k.clone(), Value::String(o.to_string()));
                        }
                    }
                }
                pend[g][k] = val.clone();
            }
        }
        if self.flushing {
            return Task::none();
        }
        self.flushing = true;
        Task::perform(run::after(500), |_| KMsg::Flush)
    }

    pub fn update(&mut self, m: KMsg) -> Task<KMsg> {
        match m {
            KMsg::Status(o) => {
                self.asking = false;
                match kit::answer(&o) {
                    Ok(v) if v.is_object() => {
                        self.st = Some(v);
                        if self.live() {
                            return Task::perform(kit::run_in(helper(&["get"]), String::new()), KMsg::Got);
                        }
                    }
                    Ok(_) => self.st = Some(json!({"live": false})),
                    Err(e) => {
                        self.st = Some(json!({"live": false, "error": e}));
                        self.say(&e, true);
                    }
                }
            }
            KMsg::Got(o) => match kit::answer(&o) {
                Ok(v) => self.take(&v),
                Err(e) => self.say(&e, true),
            },
            KMsg::Set(file, groups) => {
                if !self.live() {
                    return Task::none();
                }
                return self.set_many(file, &groups);
            }
            KMsg::Flush => {
                self.flushing = false;
                let p = std::mem::take(&mut self.pending);
                let mut t = vec![];
                for (file, groups) in p {
                    self.saving += 1;
                    let f2 = file.clone();
                    t.push(Task::perform(kit::run_in(helper(&["set", &file]), groups.to_string()), move |o| KMsg::Saved(f2.clone(), o)));
                }
                return Task::batch(t);
            }
            KMsg::Saved(_file, o) => {
                self.saving = self.saving.saturating_sub(1);
                match kit::answer(&o) {
                    Ok(r) => {
                        if r.get("files").is_some() {
                            self.take(&r);
                        }
                        let reloaded = kit::b_of(&r, "reloaded");
                        if reloaded || !self.running() {
                            self.msg.clear();
                        } else {
                            self.say("Saved. KWin uses it from the next time you sign in.", false);
                        }
                    }
                    Err(e) => {
                        self.say(&e, true);
                        return Task::perform(kit::run_in(helper(&["get"]), String::new()), KMsg::Got);
                    }
                }
            }
            KMsg::Shortcut(n) => return Task::perform(kit::run_in(helper(&["shortcut", n]), String::new()), KMsg::Did),
            KMsg::Did(o) => {
                if let Err(e) = kit::answer(&o) {
                    self.say(&e, true);
                }
            }
            KMsg::Open(id) => self.pick = Some(id),
            KMsg::Close => self.pick = None,
            KMsg::DesktopsGot(o) => match kit::answer(&o) {
                Ok(v) => {
                    self.names = kit::arr(&v, "desktops").iter().map(|d| kit::s_of(d, "name")).collect();
                    self.ds = v;
                }
                Err(e) => self.say(&e, true),
            },
            KMsg::Desktops(change) => {
                if !self.live() {
                    return Task::none();
                }
                self.saving += 1;
                return Task::perform(kit::run_in(helper(&["desktops-set"]), change.to_string()), |o| KMsg::Saved("desktops".into(), o)).chain(Task::perform(kit::run_in(helper(&["desktops"]), String::new()), KMsg::DesktopsGot));
            }
            KMsg::Name(i, s) => {
                if i < self.names.len() {
                    self.names[i] = s;
                }
            }
            KMsg::NameSave(i) => {
                let ds = kit::arr(&self.ds, "desktops");
                if let (Some(d), Some(n)) = (ds.get(i), self.names.get(i)) {
                    if *n != kit::s_of(d, "name") {
                        let id = { let x = kit::s_of(d, "id"); if x.is_empty() { (i + 1).to_string() } else { x } };
                        let mut names = Map::new();
                        names.insert(id, Value::String(n.clone()));
                        return self.update(KMsg::Desktops(json!({ "names": names })));
                    }
                }
            }
            KMsg::Outputs(o) => match kit::answer(&o) {
                Ok(v) => {
                    self.outs = kit::arr(&v, "outputs").into_iter().cloned().collect();
                    self.padding = v.get("padding").and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok()))).unwrap_or(4.0) as f32;
                    self.layouts.clear();
                    for o in &self.outs {
                        if let Some(l) = o.get("layout").filter(|l| l.is_object()) {
                            self.layouts.insert(kit::s_of(o, "uuid"), l.clone());
                        }
                    }
                    if self.screen >= self.outs.len() {
                        self.screen = 0;
                    }
                }
                Err(e) => self.say(&e, true),
            },
            KMsg::Screen(i) => {
                let mut t = vec![];
                if self.tiles_due || self.gap_due {
                    t.push(self.tiles_save());
                }
                self.screen = i;
                return Task::batch(t);
            }
            KMsg::Preset(i) => return self.edit(|l| l["columns"] = presets()[i].1.clone()),
            KMsg::ColWidth(i, v) => {
                return self.edit(move |l| {
                    let cols = l["columns"].as_array_mut().unwrap();
                    let n = cols.len();
                    let old = cols[i]["width"].as_f64().unwrap_or(0.0);
                    let nv = (v as f64).clamp(0.1, 0.9);
                    let rest = 1.0 - old;
                    let nrest = 1.0 - nv;
                    for (j, c) in cols.iter_mut().enumerate() {
                        let w = c["width"].as_f64().unwrap_or(0.0);
                        c["width"] = json!(if j == i { nv } else if rest > 0.0 { w / rest * nrest } else { nrest / (n as f64 - 1.0).max(1.0) });
                    }
                })
            }
            KMsg::ColRows(i, n) => return self.edit(move |l| l["columns"][i]["rows"] = if n > 1 { json!(vec![1.0; n]) } else { json!([]) }),
            KMsg::ColRemove(i) => {
                return self.edit(move |l| {
                    if let Some(a) = l["columns"].as_array_mut() {
                        if a.len() > 1 {
                            a.remove(i);
                        }
                    }
                })
            }
            KMsg::ColAdd => {
                return self.edit(|l| {
                    if let Some(a) = l["columns"].as_array_mut() {
                        let n = a.len().max(1) as f64;
                        a.push(json!({"width": 1.0 / n, "rows": []}));
                    }
                })
            }
            KMsg::Gap(v) => {
                if !self.live() {
                    return Task::none();
                }
                self.padding = (v * 64.0).round();
                let first = !self.tiles_due && !self.gap_due;
                self.gap_due = true;
                if first {
                    return Task::perform(run::after(600), |_| KMsg::TilesFlush);
                }
            }
            KMsg::TilesFlush => return self.tiles_save(),
            KMsg::TilesSaved(o) => {
                self.saving = self.saving.saturating_sub(1);
                match kit::answer(&o) {
                    Ok(r) => {
                        if kit::b_of(&r, "live") || !self.running() {
                            self.msg.clear();
                        } else {
                            self.say("Saved. KWin uses it from the next time you sign in.", false);
                        }
                    }
                    Err(e) => self.say(&e, true),
                }
            }
            KMsg::Go(_) => {}
        }
        Task::none()
    }

    fn say(&mut self, s: &str, bad: bool) {
        self.msg = s.to_string();
        self.msg_bad = bad;
    }

    fn out(&self) -> Option<&Value> {
        self.outs.get(self.screen)
    }
    pub fn lay(&self) -> Value {
        self.out().and_then(|o| self.layouts.get(&kit::s_of(o, "uuid"))).cloned().unwrap_or_else(default_layout)
    }
    fn edit(&mut self, f: impl FnOnce(&mut Value)) -> Task<KMsg> {
        if !self.live() {
            return Task::none();
        }
        let Some(o) = self.out() else { return Task::none() };
        let uuid = kit::s_of(o, "uuid");
        let mut l = self.lay();
        if !l["columns"].is_array() {
            l = default_layout();
        }
        f(&mut l);
        let ws: Vec<f64> = l["columns"].as_array().unwrap().iter().map(|c| c["width"].as_f64().unwrap_or(0.0).max(0.05)).collect();
        let w = norm(&ws);
        for (i, c) in l["columns"].as_array_mut().unwrap().iter_mut().enumerate() {
            c["width"] = json!(w[i]);
            let rows: Vec<f64> = c["rows"].as_array().map(|r| r.iter().map(|h| h.as_f64().unwrap_or(0.0).max(0.05)).collect()).unwrap_or_default();
            c["rows"] = if rows.len() > 1 { json!(norm(&rows)) } else { json!([]) };
        }
        self.layouts.insert(uuid, l);
        let first = !self.tiles_due && !self.gap_due;
        self.tiles_due = true;
        if first {
            Task::perform(run::after(600), |_| KMsg::TilesFlush)
        } else {
            Task::none()
        }
    }
    fn tiles_save(&mut self) -> Task<KMsg> {
        let Some(o) = self.out().cloned() else { return Task::none() };
        let uuid = kit::s_of(&o, "uuid");
        let name = { let n = kit::s_of(&o, "name"); if n.is_empty() { "-".to_string() } else { n } };
        let mut t = vec![];
        if std::mem::take(&mut self.tiles_due) {
            self.saving += 1;
            let l = self.lay();
            t.push(Task::perform(kit::run_in(helper(&["tiles-set", &uuid, &name]), l.to_string()), KMsg::TilesSaved));
        }
        if std::mem::take(&mut self.gap_due) {
            self.saving += 1;
            t.push(Task::perform(kit::run_in(helper(&["tiles-set", &uuid, &name]), json!({"padding": self.padding}).to_string()), KMsg::TilesSaved));
        }
        Task::batch(t)
    }
}

fn norm(l: &[f64]) -> Vec<f64> {
    let t: f64 = l.iter().sum::<f64>();
    let t = if t > 0.0 { t } else { 1.0 };
    l.iter().map(|x| (x / t * 1000.0).round() / 1000.0).collect()
}
fn default_layout() -> Value {
    json!({"columns": [{"width": 0.25, "rows": []}, {"width": 0.5, "rows": []}, {"width": 0.25, "rows": []}]})
}
fn presets() -> Vec<(&'static str, Value)> {
    vec![
        ("Halves", json!([{"width": 0.5, "rows": []}, {"width": 0.5, "rows": []}])),
        ("Thirds", json!([{"width": 1, "rows": []}, {"width": 1, "rows": []}, {"width": 1, "rows": []}])),
        ("Wide middle", json!([{"width": 0.25, "rows": []}, {"width": 0.5, "rows": []}, {"width": 0.25, "rows": []}])),
        ("Big left", json!([{"width": 0.66, "rows": []}, {"width": 0.34, "rows": [0.5, 0.5]}])),
        ("Quarters", json!([{"width": 0.5, "rows": [0.5, 0.5]}, {"width": 0.5, "rows": [0.5, 0.5]}])),
    ]
}

// ------------------------------------------------------------------ the pieces of a KWin page
pub fn set(file: &'static str, group: &str, key: &str, v: Value) -> KMsg {
    KMsg::Set(file, json!({ group: { key: v } }))
}
fn effect(id: &str, on: bool) -> KMsg {
    set("kwinrc", "Plugins", &format!("{id}Enabled"), json!(on))
}

/// The top of a KWin page (KwinGate.qml): why it's greyed, a preview note, a file that couldn't be read.
pub fn gate<'a>(p: &Pal, kw: &Kw, what: &str) -> kit::Col<'a, KMsg> {
    let mut c = kit::Col::new(kw.skip);
    if !kw.ready() {
        return c.push(small(p, "Asking KWin…"));
    }
    if !kw.live() {
        c = c.push(kit::banner(p, Ic::Window, "Needs KWin", format!("{what} are KWin's, the window manager KDE Plasma runs on. This desktop isn't on KWin now, so they're shown greyed: nothing here is changed until it is."), false, Some(("Your desktop", KMsg::Go("your-desktop")))));
    }
    if kw.preview() {
        c = c.push(kit::banner(p, Ic::G(G::Info), "Preview", "KWin isn't running: these show what's saved, and KWin reads them when it starts.", false, None));
    }
    if !kw.bad.is_empty() {
        let why: Vec<String> = kw.bad.values().filter_map(|v| v.as_str().map(String::from)).collect();
        c = c.push(kit::banner(p, Ic::Warn, "A KWin settings file couldn't be read", format!("{}. Hearth won't write over it; nothing in it was lost.", why.join(" · ")), true, None));
    }
    c
}
fn heading<'a>(p: &Pal, kw: &Kw, s: &str) -> E<'a> {
    head(p, if kw.saving > 0 { format!("{s} · saving…") } else { s.to_string() })
}
fn msg_line<'a>(p: &Pal, kw: &Kw) -> Option<E<'a>> {
    if kw.msg.is_empty() { None } else { Some(kit::said(p, &kw.msg, kw.msg_bad)) }
}

/// A switch row for a key.
pub fn sw<'a>(p: &Pal, kw: &Kw, i: Ic, title: impl Into<String>, sub: impl Into<String>, on: bool, msg: KMsg, usable: bool) -> E<'a> {
    let ok = usable && kw.live();
    irow_if(p, ok, i, title, sub, switch(p, on, if ok { Some(msg) } else { None }))
}
fn eff_sub(kw: &Kw, id: &str, text: &str) -> String {
    if kw.has_effect(id) { text.to_string() } else { format!("{text}. Not in this KWin") }
}
fn eff_sw<'a>(p: &Pal, kw: &Kw, i: Ic, title: &str, id: &str, text: &str) -> E<'a> {
    let on = kw.effect_on(id);
    sw(p, kw, i, title, eff_sub(kw, id, text), on, effect(id, !on), kw.has_effect(id))
}
/// Choices side by side for a key.
pub fn ch<'a, V: PartialEq + Clone>(p: &Pal, kw: &Kw, i: Ic, title: impl Into<String>, sub: impl Into<String>, opts: &[(&str, V)], cur: &V, to: impl Fn(V) -> KMsg, usable: bool) -> E<'a> {
    let ok = usable && kw.live();
    let items = opts.iter().map(|(l, v)| (l.to_string(), v == cur, if ok { Some(to(v.clone())) } else { None })).collect();
    irow_if(p, ok, i, title, sub, seg(p, items))
}
/// A slider for a number.
pub fn sl<'a>(p: &Pal, kw: &Kw, i: Ic, title: impl Into<String>, sub: impl Into<String>, v: f32, step: f32, value: String, to: impl Fn(f32) -> KMsg + 'a, usable: bool) -> E<'a> {
    let ok = usable && kw.live();
    let ctl: E<'a> = if ok { slide(p, v.clamp(0.0, 1.0), 0.0..=1.0, step, value, to) } else { t1(value, sz(13.0), p.muted).into() };
    irow_if(p, ok, i, title, sub, ctl)
}
fn dd<'a>(p: &Pal, kw: &Kw, i: Ic, title: impl Into<String>, sub: impl Into<String>, label: String, id: &str) -> E<'a> {
    let ok = kw.live();
    irow_if(p, ok, i, title, sub, dropdown(p, label, if ok { Some(KMsg::Open(id.to_string())) } else { None }))
}
fn link<'a>(p: &Pal, title: &str, sub: &str, to: &'static str) -> E<'a> {
    let i = match to { "kwin-desktops" => Ic::Desktops, "kwin-edges" => Ic::G(G::Display), _ => Ic::Window };
    kit::ilink(p, i, title, sub, "", KMsg::Go(to))
}

// ------------------------------------------------------------------ Window effects
const SPEEDS: &[(&str, f64)] = &[("Very slow", 4.0), ("Slow", 2.0), ("Normal", 1.0), ("Fast", 0.5), ("Very fast", 0.25), ("Instant", 0.0)];

fn on_of(kw: &Kw, ids: &[&str]) -> String {
    ids.iter().find(|id| kw.effect_on(id)).map(|s| s.to_string()).unwrap_or_else(|| "none".into())
}
/// One of several effects that do the same job: turning one on turns the others off.
fn pick_one(kw: &Kw, ids: &[&str], chosen: &str) -> KMsg {
    let mut g = Map::new();
    for id in ids {
        if kw.effect_on(id) != (*id == chosen) {
            g.insert(format!("{id}Enabled"), json!(*id == chosen));
        }
    }
    KMsg::Set("kwinrc", json!({ "Plugins": g }))
}
fn one_of<'a>(p: &Pal, kw: &Kw, i: Ic, title: &str, sub: &str, opts: &[(&str, &'static str)]) -> E<'a> {
    let ids: Vec<&str> = opts.iter().map(|o| o.1).filter(|x| *x != "none").collect();
    let cur = on_of(kw, &ids);
    let ok = kw.live();
    let items = opts
        .iter()
        .map(|(l, v)| {
            let usable = ok && (*v == "none" || kw.has_effect(v));
            (l.to_string(), cur == *v, if usable { Some(pick_one(kw, &ids, v)) } else { None })
        })
        .collect();
    irow_if(p, ok, i, title, sub, seg(p, items))
}
fn switcher_looks(kw: &Kw) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = match kw.st.as_ref().and_then(|s| s.get("switchers")).and_then(Value::as_array) {
        Some(l) => l.iter().map(|s| { let lb = kit::s_of(s, "label"); (kit::s_of(s, "id"), if kit::b_of(s, "hearth") { format!("{lb} (Hearth)") } else { lb }) }).collect(),
        None => vec![("thumbnail_grid".into(), "Thumbnail grid".into())],
    };
    v.push(("__none".into(), "No switcher, just switch".into()));
    v
}
fn look_now(kw: &Kw, group: &str) -> String {
    if kw.b("kwinrc", group, "ShowTabBox", true) { kw.s("kwinrc", group, "LayoutName", "thumbnail_grid") } else { "__none".into() }
}
fn set_look(group: &str, v: &str) -> KMsg {
    if v == "__none" { set("kwinrc", group, "ShowTabBox", json!(false)) } else { KMsg::Set("kwinrc", json!({ group: { "ShowTabBox": true, "LayoutName": v } })) }
}
fn label_of(list: &[(String, String)], v: &str) -> String {
    list.iter().find(|x| x.0 == v).map(|x| x.1.clone()).unwrap_or_else(|| v.to_string())
}

pub fn effects<'a>(p: &Pal, kw: &'a Kw) -> E<'a> {
    let mut c = gate(p, kw, "Window effects");
    let f = kw.n("kdeglobals", "KDE", "AnimationDurationFactor", 1.0);
    let speed = SPEEDS.iter().min_by(|a, b| (a.1 - f).abs().partial_cmp(&(b.1 - f).abs()).unwrap()).map(|s| s.1).unwrap_or(1.0);
    c = c.push(heading(p, kw, "Animations"));
    c = c.push(ch(p, kw, Ic::Motion, "Animation speed", "How fast windows open, close, move and switch. Reduce motion in Accessibility also stills Hearth's own", SPEEDS, &speed, |v| set("kdeglobals", "KDE", "AnimationDurationFactor", json!(v)), true));
    c = c.push(one_of(p, kw, Ic::Window, "When a window opens or closes", "", &[("Grow", "scale"), ("Glide", "glide"), ("Fade", "fade"), ("Nothing", "none")]));
    c = c.push(one_of(p, kw, Ic::Window, "When a window is minimized", "", &[("Squash", "squash"), ("Magic lamp", "magiclamp"), ("Nothing", "none")]));
    c = c.push(one_of(p, kw, Ic::G(G::Home), "When the desktop is shown", "Windows move out of the way, or shrink to the middle of the screen", &[("Move aside", "windowaperture"), ("Eye on screen", "eyeonscreen"), ("Nothing", "none")]));
    c = c.push(link(p, "Switching between desktops", "Slide, fade or nothing: in Virtual desktops", "kwin-desktops"));
    c = c.push(eff_sw(p, kw, Ic::Window, "Maximize and full screen animate", "maximize", "Windows grow smoothly when maximized"));
    let pop = kw.effect_on("fadingpopups") || kw.effect_on("slidingpopups");
    c = c.push(sw(p, kw, Ic::Menu, "Menus and pop-ups fade and slide", "", pop, KMsg::Set("kwinrc", json!({"Plugins": {"fadingpopupsEnabled": !pop, "slidingpopupsEnabled": !pop}})), true));
    c = c.push(eff_sw(p, kw, Ic::Window, "Dialogs slide out of their window", "sheet", "A message from an app drops out of the top of its window"));
    c = c.push(eff_sw(p, kw, Ic::Trash, "Windows fall apart when closed", "fallapart", "Closing a window breaks it into pieces"));
    let login = kw.effect_on("login");
    c = c.push(sw(p, kw, Ic::G(G::Power), "Fade in when you sign in, fade out when you sign out", "", login, KMsg::Set("kwinrc", json!({"Plugins": {"loginEnabled": !login, "logoutEnabled": !login}})), true));
    let sf = kw.effect_on("startupfeedback");
    c = c.push(sw(p, kw, Ic::G(G::Rocket), "Bounce the pointer while an app starts", "", sf, effect("startupfeedback", !sf), true));

    c = c.push(heading(p, kw, "Seeing your windows"));
    c = c.push(eff_sw(p, kw, Ic::G(G::Grid), "Overview", "overview", "All your windows and desktops at once, with search. Super+W, four fingers up on a touchpad, or the hot corner"));
    if kw.effect_on("overview") {
        let im = kw.b("kwinrc", "Effect-overview", "IgnoreMinimized", false);
        c = c.push(sw(p, kw, Ic::G(G::Grid), "Overview: leave out minimized windows", "", im, set("kwinrc", "Effect-overview", "IgnoreMinimized", json!(!im)), true));
        let ts_ = !kw.b("kwinrc", "Effect-overview", "FilterWindows", true);
        c = c.push(sw(p, kw, Ic::G(G::Search), "Overview: typing searches", "Off: typing only filters the windows shown", ts_, set("kwinrc", "Effect-overview", "FilterWindows", json!(ts_)), true));
        c = c.push(iact(p, Ic::Desktops, "Desktop grid", "Every desktop side by side, to move windows between them: Super+G, or four fingers down. Part of the overview in KWin 6", false, if kw.live() { Some(KMsg::Shortcut("grid")) } else { None }));
    }
    c = c.push(eff_sw(p, kw, Ic::Window, "Present windows", "windowview", "Every window side by side to pick one (Ctrl+F9, or Ctrl+F10 for this app's)"));
    c = c.push(link(p, "Hot corners", "Which corner opens the overview, the grid or every window", "kwin-edges"));

    let looks = switcher_looks(kw);
    c = c.push(heading(p, kw, "Window switcher (Alt+Tab)"));
    c = c.push(dd(p, kw, Ic::Window, "Look", "What Alt+Tab shows while you hold it. Alt+` shows only the windows of the app you're on, in the same look", label_of(&looks, &look_now(kw, "TabBox")), "tabbox"));
    let hl = kw.b("kwinrc", "TabBox", "HighlightWindows", true);
    c = c.push(sw(p, kw, Ic::Eye, "Show the window you're on", "The others fade while Alt+Tab is held", hl, set("kwinrc", "TabBox", "HighlightWindows", json!(!hl)), true));
    c = c.push(ch(p, kw, Ic::Desktops, "Which windows", "", &[("This desktop", 1i64), ("Every desktop", 0)], &(kw.n("kwinrc", "TabBox", "DesktopMode", 1.0) as i64), |v| set("kwinrc", "TabBox", "DesktopMode", json!(v)), true));
    c = c.push(ch(p, kw, Ic::Window, "Minimized windows", "", &[("Included", 0i64), ("Left out", 1), ("Only those", 2)], &(kw.n("kwinrc", "TabBox", "MinimizedMode", 0.0) as i64), |v| set("kwinrc", "TabBox", "MinimizedMode", json!(v)), true));
    c = c.push(ch(p, kw, Ic::G(G::Grid), "Order", "", &[("Most recent first", 0i64), ("As they are stacked", 1)], &(kw.n("kwinrc", "TabBox", "SwitchingMode", 0.0) as i64), |v| set("kwinrc", "TabBox", "SwitchingMode", json!(v)), true));

    c = c.push(heading(p, kw, "Second switcher (Super+Tab)"));
    c = c.push(dd(p, kw, Ic::Window, "Second switcher's look", "A switcher of its own, set up apart from Alt+Tab", label_of(&looks, &look_now(kw, "TabBoxAlternative")), "tabbox2"));
    c = c.push(ch(p, kw, Ic::G(G::Grid), "Which apps", "Only the app you're on: Super+Tab goes between that app's windows", &[("Every window", 0i64), ("One per app", 1), ("Only the app you're on", 2)], &(kw.n("kwinrc", "TabBoxAlternative", "ApplicationsMode", 0.0) as i64), |v| set("kwinrc", "TabBoxAlternative", "ApplicationsMode", json!(v)), true));
    c = c.push(ch(p, kw, Ic::Desktops, "Which windows", "", &[("This desktop", 1i64), ("Every desktop", 0)], &(kw.n("kwinrc", "TabBoxAlternative", "DesktopMode", 1.0) as i64), |v| set("kwinrc", "TabBoxAlternative", "DesktopMode", json!(v)), true));
    c = c.push(ch(p, kw, Ic::Window, "Minimized windows", "", &[("Included", 0i64), ("Left out", 1), ("Only those", 2)], &(kw.n("kwinrc", "TabBoxAlternative", "MinimizedMode", 0.0) as i64), |v| set("kwinrc", "TabBoxAlternative", "MinimizedMode", json!(v)), true));

    c = c.push(heading(p, kw, "Blur and see-through windows"));
    c = c.push(eff_sw(p, kw, Ic::Sparkle, "Blur behind see-through windows", "blur", "Panels, menus and windows that let the background show through blur it"));
    if kw.effect_on("blur") {
        let bs = kw.n("kwinrc", "Effect-blur", "BlurStrength", 15.0);
        c = c.push(sl(p, kw, Ic::Sparkle, "How much blur", "", ((bs - 1.0) / 14.0) as f32, 1.0 / 14.0, format!("{}", bs.round()), |v| set("kwinrc", "Effect-blur", "BlurStrength", json!((1.0 + v * 14.0).round() as i64)), true));
        let ns = kw.n("kwinrc", "Effect-blur", "NoiseStrength", 5.0);
        c = c.push(sl(p, kw, Ic::Sparkle, "Grain in the blur", "A little grain hides banding", (ns / 14.0) as f32, 1.0 / 14.0, format!("{}", ns.round()), |v| set("kwinrc", "Effect-blur", "NoiseStrength", json!((v * 14.0).round() as i64)), true));
    }
    c = c.push(eff_sw(p, kw, Ic::Eye, "Keep text readable on glass", "backgroundcontrast", "More contrast behind see-through panels"));
    c = c.push(eff_sw(p, kw, Ic::Window, "See-through windows", "translucency", "Windows become see-through while moved, when not in front, and menus"));
    if kw.effect_on("translucency") {
        for (key, label, d) in [("MoveResize", "While moved or resized", 80.0), ("Inactive", "Windows not in front", 100.0), ("Dialogs", "Dialogs", 100.0), ("Menus", "Menus", 100.0)] {
            let v = kw.n("kwinrc", "Effect-translucency", key, d) / 100.0;
            c = c.push(sl(p, kw, Ic::Window, label, "", v as f32, 0.01, format!("{}% solid", (v * 100.0).round()), move |x| set("kwinrc", "Effect-translucency", key, json!(((x * 100.0).round() as i64).max(10))), true));
        }
    }

    c = c.push(heading(p, kw, "Moving windows"));
    c = c.push(eff_sw(p, kw, Ic::Wave, "Wobbly windows", "wobblywindows", "Windows wobble like jelly while you move them"));
    if kw.effect_on("wobblywindows") {
        c = c.push(ch(p, kw, Ic::Wave, "How wobbly", "", &[("A little", 0i64), ("Some", 1), ("Medium", 2), ("Lots", 3), ("Most", 4)], &(kw.n("kwinrc", "Effect-wobblywindows", "WobblynessLevel", 0.0) as i64), |v| set("kwinrc", "Effect-wobblywindows", "WobblynessLevel", json!(v)), true));
    }
    c = c.push(eff_sw(p, kw, Ic::G(G::Grid), "Show where a window snaps", "snaphelper", "Lines to the middle of the screen while you move a window"));
    c = c.push(eff_sw(p, kw, Ic::Window, "Window you click slides to the front", "slideback", "Windows above it slide away and back"));
    c = c.push(eff_sw(p, kw, Ic::Image, "A picture of the window you're on, at the side", "thumbnailaside", "Super+Ctrl+T puts one there"));

    c = c.push(heading(p, kw, "Dimming"));
    c = c.push(eff_sw(p, kw, Ic::G(G::Moon), "Dim windows you're not using", "diminactive", "Only the window in front stays bright"));
    if kw.effect_on("diminactive") {
        let s = kw.n("kwinrc", "Effect-diminactive", "Strength", 25.0) / 100.0;
        c = c.push(sl(p, kw, Ic::G(G::Moon), "How much", "", s as f32, 0.01, format!("{}%", (s * 100.0).round()), |v| set("kwinrc", "Effect-diminactive", "Strength", json!(((v * 100.0).round() as i64).max(5))), true));
        let dp = kw.b("kwinrc", "Effect-diminactive", "DimPanels", false);
        c = c.push(sw(p, kw, Ic::Window, "Dim the bar and dock too", "", dp, set("kwinrc", "Effect-diminactive", "DimPanels", json!(!dp)), true));
    }
    c = c.push(eff_sw(p, kw, Ic::G(G::Shield), "Dim the screen when asked for your password", "dimscreen", "When an app asks for admin rights, everything else dims"));
    let fz = kw.effect_on("frozenapp");
    c = c.push(sw(p, kw, Ic::Warn, "Grey out apps that stopped answering", "", fz, effect("frozenapp", !fz), true));
    let dpar = kw.effect_on("dialogparent");
    c = c.push(sw(p, kw, Ic::Window, "Darken a window while its dialog is open", "", dpar, effect("dialogparent", !dpar), true));

    c = c.push(heading(p, kw, "Pointer"));
    c = c.push(eff_sw(p, kw, Ic::Mouse, "Shake the pointer to find it", "shakecursor", "Wiggle the mouse and the pointer grows for a moment"));
    if kw.effect_on("shakecursor") {
        let m = kw.n("kwinrc", "Effect-shakecursor", "Magnification", 3.0);
        c = c.push(sl(p, kw, Ic::Mouse, "How big it grows", "", ((m - 1.5) / 4.5) as f32, 0.01, format!("{:.1}×", m), |v| set("kwinrc", "Effect-shakecursor", "Magnification", json!(((1.5 + v as f64 * 4.5) * 2.0).round() / 2.0)), true));
    }
    c = c.push(eff_sw(p, kw, Ic::Mouse, "Show mouse clicks", "mouseclick", "Rings where you click (Super+* turns them on and off): handy for recording"));
    c = c.push(eff_sw(p, kw, Ic::G(G::Search), "Find the pointer with Super+Ctrl", "trackmouse", "Hold Super and Ctrl: a ring shows where the pointer is"));
    c = c.push(eff_sw(p, kw, Ic::Edit, "Draw on the screen", "mousemark", "Hold Super+Shift and drag to draw; Super+Shift+F11 clears it"));
    let typing = kw.effect_on("hidecursor") && kw.b("kwinrc", "Effect-hidecursor", "HideOnTyping", true);
    let still = if kw.effect_on("hidecursor") { kw.n("kwinrc", "Effect-hidecursor", "InactivityDuration", 0.0) as i64 } else { 0 };
    c = c.push(sw(p, kw, Ic::Mouse, "Hide the pointer while you type", eff_sub(kw, "hidecursor", "It comes back when you move the mouse"), typing, KMsg::Set("kwinrc", json!({"Plugins": {"hidecursorEnabled": !typing || still > 0}, "Effect-hidecursor": {"HideOnTyping": !typing}})), kw.has_effect("hidecursor")));
    let default_typing = kw.b("kwinrc", "Effect-hidecursor", "HideOnTyping", true);
    c = c.push(ch(p, kw, Ic::Mouse, "Hide the pointer when the mouse is still", eff_sub(kw, "hidecursor", "It comes back when you move the mouse"), &[("Never", 0i64), ("5 s", 5), ("10 s", 10), ("30 s", 30), ("1 min", 60)], &still, move |sec| {
        let mut fx = json!({"InactivityDuration": sec});
        // Turning the effect on for this alone must not also start hiding it while typing (that's on by default).
        if !typing && default_typing {
            fx["HideOnTyping"] = json!(false);
        }
        KMsg::Set("kwinrc", json!({"Plugins": {"hidecursorEnabled": sec > 0 || typing}, "Effect-hidecursor": fx}))
    }, kw.has_effect("hidecursor")));
    c = c.push(eff_sw(p, kw, Ic::G(G::Pad), "Show touches on a touchscreen", "touchpoints", "Circles where fingers touch"));

    c = c.push(heading(p, kw, "Window helpers"));
    for (id, ic, title, text) in [
        ("minimizeall", Ic::Window, "Minimize every window with Super+Shift+D", "Press it again to bring them all back"),
        ("videowall", Ic::G(G::Display), "Video wall", "A video played full screen stretches over every screen"),
        ("synchronizeskipswitcher", Ic::G(G::Grid), "Leave out of Alt+Tab what's left off the taskbar", "Windows an app keeps off the taskbar aren't in the switchers either"),
    ] {
        let on = kw.effect_on(id);
        let sub = if kw.has_script(id) { text.to_string() } else { format!("{text}. Not in this KWin") };
        c = c.push(sw(p, kw, ic, title, sub, on, effect(id, !on), kw.has_script(id)));
    }
    c = c.push(heading(p, kw, "For developers"));
    c = c.push(eff_sw(p, kw, Ic::Cpu, "Show FPS", "showfps", "How many frames a second KWin draws, in a corner of the screen"));
    c = c.push(eff_sw(p, kw, Ic::Edit, "Show paint", "showpaint", "Flashes a colour over every part of the screen KWin draws again"));
    if let Some(m) = msg_line(p, kw) {
        c = c.push(m);
    }
    c.into()
}

// ------------------------------------------------------------------ Virtual desktops
pub fn desktops<'a>(p: &Pal, kw: &'a Kw) -> E<'a> {
    let mut c = gate(p, kw, "Virtual desktops");
    let count = kit::arr(&kw.ds, "desktops").len();
    let ok = kw.live();
    c = c.push(heading(p, kw, "Desktops"));
    let ctl: E<'a> = if ok {
        kit::slide(p, (count.max(1) - 1) as f32 / 19.0, 0.0..=1.0, 1.0 / 19.0, count.to_string(), move |v| {
            let n = ((1.0 + v * 19.0).round() as usize).clamp(1, 20);
            if n != count { KMsg::Desktops(json!({"count": n})) } else { KMsg::Close }
        })
    } else {
        t1(count.to_string(), sz(13.0), p.muted).into()
    };
    c = c.push(irow_if(p, ok, Ic::Desktops, "How many", "Super+Ctrl+Left / Right switch between them; three or four fingers sideways on a touchpad", ctl));
    for (i, d) in kit::arr(&kw.ds, "desktops").iter().enumerate() {
        let name = kw.names.get(i).cloned().unwrap_or_else(|| kit::s_of(d, "name"));
        let ph = format!("Desktop {}", i + 1);
        let fl = if ok { field(p, &ph, &name, move |s| KMsg::Name(i, s)).on_submit(KMsg::NameSave(i)) } else { field(p, &ph, &name, move |s| KMsg::Name(i, s)) };
        c = c.push(column![small(p, format!("Desktop {} · Enter saves its name", i + 1)), fl].spacing(2.0));
    }
    let rows = (kit::n_of(&kw.ds, "rows", 1.0) as usize).clamp(1, count.max(1));
    let items = (1..=4usize).map(|n| (n.to_string(), n == rows, if ok && n <= count { Some(KMsg::Desktops(json!({"rows": n}))) } else { None })).collect();
    c = c.push(irow_if(p, ok, Ic::G(G::Grid), "Rows", "Desktops laid out in a grid: Super+Ctrl+Up / Down move between rows", seg(p, items)));
    let ro = kw.b("kwinrc", "Windows", "RollOverDesktops", true);
    c = c.push(sw(p, kw, Ic::Sync, "Go round", "From the last desktop, the next one is the first", ro, set("kwinrc", "Windows", "RollOverDesktops", json!(!ro)), true));
    c = c.push(small(p, "Every screen switches together: KWin 6 has one set of desktops for all screens."));

    c = c.push(heading(p, kw, "Switching"));
    let cur = if kw.effect_on("slide") { "slide" } else if kw.effect_on("fadedesktop") { "fadedesktop" } else { "none" };
    let anim = |v: &str| {
        let mut g = Map::new();
        if kw.effect_on("slide") != (v == "slide") {
            g.insert("slideEnabled".into(), json!(v == "slide"));
        }
        if kw.effect_on("fadedesktop") != (v == "fadedesktop") {
            g.insert("fadedesktopEnabled".into(), json!(v == "fadedesktop"));
        }
        KMsg::Set("kwinrc", json!({"Plugins": g}))
    };
    let items = vec![
        ("Slide".to_string(), cur == "slide", if ok && kw.has_effect("slide") { Some(anim("slide")) } else { None }),
        ("Fade".to_string(), cur == "fadedesktop", if ok && kw.has_effect("fadedesktop") { Some(anim("fadedesktop")) } else { None }),
        ("None".to_string(), cur == "none", if ok { Some(anim("none")) } else { None }),
    ];
    c = c.push(irow_if(p, ok, Ic::Motion, "Animation", "", seg(p, items)));
    if kw.effect_on("slide") {
        let sb = kw.b("kwinrc", "Effect-slide", "SlideBackground", true);
        c = c.push(sw(p, kw, Ic::Image, "The wallpaper slides too", "", sb, set("kwinrc", "Effect-slide", "SlideBackground", json!(!sb)), true));
        let gap = kw.n("kwinrc", "Effect-slide", "HorizontalGap", 45.0) / 200.0;
        c = c.push(sl(p, kw, Ic::Desktops, "Gap between desktops while sliding", "", gap as f32, 0.01, format!("{} px", (gap * 200.0).round()), |v| {
            let h = (v * 200.0).round() as i64;
            KMsg::Set("kwinrc", json!({"Effect-slide": {"HorizontalGap": h, "VerticalGap": ((v * 200.0 * 20.0 / 45.0).round() as i64)}}))
        }, true));
    }
    let osd = kw.effect_on("desktopchangeosd");
    c = c.push(sw(p, kw, Ic::G(G::Info), "Show the desktop's name when switching", "", osd, effect("desktopchangeosd", !osd), true));
    c = c.push(link(p, "Switch by pushing against the screen's edge", "In Screen edges", "kwin-edges"));

    c = c.push(heading(p, kw, "Desktop cube"));
    let cube = kw.effect_on("cube");
    let sub = if kw.has_effect("cube") { "Super+C puts your desktops on the sides of a cube: drag to spin it, click a side to go there" } else { "Not in this KWin: the cube comes with the kwin-addons package, which isn't installed" };
    c = c.push(sw(p, kw, Ic::Box, "Desktop cube", sub, cube, effect("cube", !cube), kw.has_effect("cube")));
    if cube && kw.has_effect("cube") {
        c = c.push(iact(p, Ic::Box, "Open the cube now", if count < 3 { "It needs three desktops or more to make a cube" } else { "Esc or Super+C closes it" }, false, if ok && count >= 3 { Some(KMsg::Shortcut("cube")) } else { None }));
    }
    if let Some(m) = msg_line(p, kw) {
        c = c.push(m);
    }
    c.into()
}

// ------------------------------------------------------------------ Tiling
pub fn tiling<'a>(p: &Pal, kw: &'a Kw) -> E<'a> {
    let mut c = gate(p, kw, "Tile layouts");
    let ok = kw.live();
    c = c.push(heading(p, kw, "Your screens' tiles"));
    if kw.outs.len() > 1 {
        let items = kw.outs.iter().enumerate().map(|(i, o)| { let n = kit::s_of(o, "name"); (if n.is_empty() { kit::s_of(o, "model") } else { n }, i == kw.screen, if ok { Some(KMsg::Screen(i)) } else { None }) }).collect();
        c = c.push(irow_if(p, ok, Ic::G(G::Display), "Screen", "", seg(p, items)));
    }
    let lay = kw.lay();
    if kw.outs.is_empty() {
        c = c.push(small(p, if kw.ready() { "No screen found to lay out." } else { "" }));
    } else {
        let accent = p.accent;
        let mut cols = Row::new().spacing(6.0).height(Length::Fixed(sz(230.0)));
        for col in lay["columns"].as_array().cloned().unwrap_or_default() {
            let w = (col["width"].as_f64().unwrap_or(0.3) * 1000.0).round().max(1.0) as u16;
            let rows: Vec<f64> = col["rows"].as_array().map(|r| r.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
            let mut cc = Column::new().spacing(6.0).width(Length::FillPortion(w)).height(Length::Fill);
            for h in if rows.len() > 1 { rows } else { vec![1.0] } {
                cc = cc.push(container(Space::new()).width(Length::Fill).height(Length::FillPortion((h * 1000.0).round().max(1.0) as u16)).style(ui::boxed(Pal::a(accent, 0.18), p.r(8.0), Some((Pal::a(accent, 0.6), 1.0)))));
            }
            cols = cols.push(cc);
        }
        let mut drawn = Column::new().push(cols);
        if let Some(o) = kw.out() {
            if kit::b_of(o, "custom") && !kw.layouts.contains_key(&kit::s_of(o, "uuid")) {
                drawn = drawn.push(small(p, "Made in KWin's own editor (Super+T): changing it here starts from KWin's three columns"));
            }
        }
        c = c.push(container(drawn.spacing(6.0)).padding(8.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.5), p.r(10.0), Some((p.line, 1.0)))));
        let items = presets().iter().enumerate().map(|(i, (l, _))| (l.to_string(), false, if ok { Some(KMsg::Preset(i)) } else { None })).collect();
        c = c.push(irow_if(p, ok, Ic::G(G::Grid), "Start from", "", seg(p, items)));
        let cols_v = lay["columns"].as_array().cloned().unwrap_or_default();
        let n = cols_v.len();
        for (i, col) in cols_v.iter().enumerate() {
            let w = col["width"].as_f64().unwrap_or(0.3) as f32;
            c = c.push(sl(p, kw, Ic::Window, format!("Column {} width", i + 1), "", w, 0.05, format!("{}%", (w * 100.0).round()), move |v| KMsg::ColWidth(i, v), true));
            let nr = col["rows"].as_array().map(|r| r.len()).unwrap_or(0).max(1);
            let items = (1..=3usize).map(|k| (k.to_string(), k == nr, if ok { Some(KMsg::ColRows(i, k)) } else { None })).collect();
            c = c.push(irow_if(p, ok, Ic::G(G::Grid), format!("Column {} rows", i + 1), "", seg(p, items)));
            if n > 1 {
                c = c.push(kit::indent(btn(p, format!("Remove column {}", i + 1), ui::Kind::Normal, false, if ok { Some(KMsg::ColRemove(i)) } else { None })));
            }
        }
        let mut r = Row::new().spacing(10.0).push(btn(p, "Add a column", ui::Kind::Normal, false, if ok && n < 8 { Some(KMsg::ColAdd) } else { None }));
        if kw.running() {
            r = r.push(btn(p, "Open KWin's tile editor", ui::Kind::Normal, false, Some(KMsg::Shortcut("tiles"))));
        }
        c = c.push(container(r).padding(Padding { top: 4.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    }
    c = c.push(sl(p, kw, Ic::G(G::Grid), "Gap between tiles", "", kw.padding / 64.0, 1.0 / 64.0, format!("{} px", kw.padding.round()), KMsg::Gap, !kw.outs.is_empty()));
    c = c.push(small(p, "Hold Shift while you drag a window to drop it into a tile. Super+T opens KWin's own editor too."));

    c = c.push(heading(p, kw, "Quick tiling"));
    let ebt = kw.b("kwinrc", "Windows", "ElectricBorderTiling", true);
    c = c.push(sw(p, kw, Ic::Window, "Drag a window to a side or corner to tile it", "Half the screen at a side, a quarter in a corner", ebt, set("kwinrc", "Windows", "ElectricBorderTiling", json!(!ebt)), true));
    let ebm = kw.b("kwinrc", "Windows", "ElectricBorderMaximize", true);
    c = c.push(sw(p, kw, Ic::Window, "Drag a window to the top to maximize it", "", ebm, set("kwinrc", "Windows", "ElectricBorderMaximize", json!(!ebm)), true));
    let cr = kw.n("kwinrc", "Windows", "ElectricBorderCornerRatio", 0.25) / 0.5;
    c = c.push(sl(p, kw, Ic::G(G::Grid), "Size of the corners", "How much of each edge counts as its corner", cr as f32, 0.01, format!("{}%", (cr * 50.0).round()), |v| set("kwinrc", "Windows", "ElectricBorderCornerRatio", json!(((v as f64 * 50.0).round() / 100.0).max(0.05))), true));
    let bl = kw.b("kwinrc", "Windows", "BorderlessMaximizedWindows", false);
    c = c.push(sw(p, kw, Ic::Window, "Maximized windows have no title bar", "", bl, set("kwinrc", "Windows", "BorderlessMaximizedWindows", json!(!bl)), true));

    c = c.push(heading(p, kw, "Snapping"));
    for (key, title, d, ic) in [("BorderSnapZone", "Snap to the screen's edges", 10.0, Ic::G(G::Display)), ("WindowSnapZone", "Snap to other windows", 10.0, Ic::Window), ("CenterSnapZone", "Snap to the middle of the screen", 0.0, Ic::G(G::Grid))] {
        let v = kw.n("kwinrc", "Windows", key, d) / 50.0;
        let shown = if v == 0.0 { "Off".to_string() } else { format!("{} px", (v * 50.0).round()) };
        c = c.push(sl(p, kw, ic, title, "", v as f32, 0.02, shown, move |x| set("kwinrc", "Windows", key, json!((x * 50.0).round() as i64)), true));
    }
    let so = kw.b("kwinrc", "Windows", "SnapOnlyWhenOverlapping", false);
    c = c.push(sw(p, kw, Ic::Window, "Only snap when windows would overlap", "", so, set("kwinrc", "Windows", "SnapOnlyWhenOverlapping", json!(!so)), true));
    if let Some(m) = msg_line(p, kw) {
        c = c.push(m);
    }
    c.into()
}

// ------------------------------------------------------------------ Screen edges
/// KWin's ElectricBorder numbers and its [ElectricBorders] key names.
const EDGES: &[(i64, &str, &str)] = &[(7, "TopLeft", "Top-left corner"), (0, "Top", "Top edge"), (1, "TopRight", "Top-right corner"), (6, "Left", "Left edge"), (2, "Right", "Right edge"), (5, "BottomLeft", "Bottom-left corner"), (4, "Bottom", "Bottom edge"), (3, "BottomRight", "Bottom-right corner")];
struct Act {
    value: &'static str,
    label: &'static str,
    group: &'static str,
    list: &'static str,
    touch: &'static str,
    effect: &'static str,
    dflt: &'static [i64],
}
const ACTS: &[Act] = &[
    Act { value: "none", label: "Nothing", group: "", list: "", touch: "", effect: "", dflt: &[] },
    Act { value: "overview", label: "Overview", group: "Effect-overview", list: "BorderActivate", touch: "TouchBorderActivate", effect: "overview", dflt: &[7] },
    Act { value: "grid", label: "Desktop grid", group: "Effect-overview", list: "GridBorderActivate", touch: "GridTouchBorderActivate", effect: "overview", dflt: &[] },
    Act { value: "windows", label: "All windows", group: "Effect-windowview", list: "BorderActivateAll", touch: "TouchBorderActivateAll", effect: "windowview", dflt: &[] },
    Act { value: "appwindows", label: "This app's windows", group: "Effect-windowview", list: "BorderActivateClass", touch: "TouchBorderActivateClass", effect: "windowview", dflt: &[] },
    Act { value: "ShowDesktop", label: "Show the desktop", group: "", list: "", touch: "", effect: "", dflt: &[] },
    Act { value: "LockScreen", label: "Lock the screen", group: "", list: "", touch: "", effect: "", dflt: &[] },
];
fn list_of(kw: &Kw, a: &Act, touch: bool) -> Vec<i64> {
    kw.list("kwinrc", a.group, if touch { a.touch } else { a.list }, if touch { &[] } else { a.dflt })
}
fn action_at(kw: &Kw, n: i64, key: &str, touch: bool) -> &'static str {
    for a in ACTS.iter().filter(|a| !a.group.is_empty()) {
        if list_of(kw, a, touch).contains(&n) {
            return a.value;
        }
    }
    let plain = kw.s("kwinrc", if touch { "TouchEdges" } else { "ElectricBorders" }, key, "None");
    match plain.as_str() {
        "ShowDesktop" => "ShowDesktop",
        "LockScreen" => "LockScreen",
        _ => "none",
    }
}
fn choose_edge(kw: &Kw, n: i64, key: &str, value: &str, touch: bool) -> KMsg {
    let mut ch = Map::new();
    for a in ACTS.iter().filter(|a| !a.group.is_empty()) {
        let l = list_of(kw, a, touch);
        let has = l.contains(&n);
        let want = a.value == value;
        if has == want {
            continue;
        }
        // 9 is KWin's "no edge" (what KDE's own settings write for an empty list).
        let nl: Vec<i64> = if want { l.iter().cloned().chain([n]).collect() } else { l.iter().cloned().filter(|x| *x != n).collect() };
        let nl: Vec<String> = nl.into_iter().filter(|x| *x != 9).map(|x| x.to_string()).collect();
        let g = ch.entry(a.group.to_string()).or_insert_with(|| json!({}));
        g[if touch { a.touch } else { a.list }] = json!(if nl.is_empty() { "9".to_string() } else { nl.join(",") });
    }
    let g = if touch { "TouchEdges" } else { "ElectricBorders" };
    let now = kw.s("kwinrc", g, key, "None");
    let want = if value == "ShowDesktop" || value == "LockScreen" { value } else { "None" };
    if now != want {
        ch.insert(g.into(), json!({ key: want }));
    }
    KMsg::Set("kwinrc", Value::Object(ch))
}
fn act_label(kw: &Kw, v: &str) -> String {
    let a = ACTS.iter().find(|a| a.value == v).unwrap_or(&ACTS[0]);
    if !a.effect.is_empty() && !kw.effect_on(a.effect) { format!("{} (its effect is off)", a.label) } else { a.label.to_string() }
}

pub fn edges<'a>(p: &Pal, kw: &'a Kw) -> E<'a> {
    let mut c = gate(p, kw, "Screen edges and hot corners");
    c = c.push(heading(p, kw, "Hot corners and edges"));
    // The screen, with what each corner and edge does.
    let cell = |n: i64, key: &str, al: iced::alignment::Horizontal| -> E<'a> {
        let a = action_at(kw, n, key, false);
        let lb = ACTS.iter().find(|x| x.value == a).map(|x| x.label).unwrap_or("Nothing");
        let tx = if a == "none" { t(lb, sz(13.0), p.muted) } else { kit_ts(lb, p.accent) };
        container(tx).width(Length::Fill).align_x(al).into()
    };
    use iced::alignment::Horizontal as H;
    let scr = column![
        row![cell(7, "TopLeft", H::Left), cell(0, "Top", H::Center), cell(1, "TopRight", H::Right)],
        Space::new().height(Length::Fill),
        row![cell(6, "Left", H::Left), Space::new().width(Length::Fill), cell(2, "Right", H::Right)],
        Space::new().height(Length::Fill),
        row![cell(5, "BottomLeft", H::Left), cell(4, "Bottom", H::Center), cell(3, "BottomRight", H::Right)],
    ];
    c = c.push(container(scr).padding(Padding { top: 12.0, right: 14.0, bottom: 12.0, left: 14.0 }).width(Length::Fill).height(Length::Fixed(sz(220.0))).style(ui::boxed(Pal::mix(p.bg, p.panel, 0.5), p.r(10.0), Some((p.line, 1.0)))));
    for (n, key, label) in EDGES {
        c = c.push(dd(p, kw, Ic::G(G::Display), *label, "", act_label(kw, action_at(kw, *n, key, false)), &format!("edge:{n}")));
    }
    let d = kw.n("kwinrc", "Windows", "ElectricBorderDelay", 75.0) / 1000.0;
    c = c.push(sl(p, kw, Ic::G(G::Clock), "How long to push before it happens", "", d as f32, 0.025, format!("{} ms", (d * 1000.0).round()), |v| set("kwinrc", "Windows", "ElectricBorderDelay", json!((v * 1000.0).round() as i64)), true));
    let cd = kw.n("kwinrc", "Windows", "ElectricBorderCooldown", 350.0) / 2000.0;
    c = c.push(sl(p, kw, Ic::G(G::Clock), "Wait before it can happen again", "", cd as f32, 0.025, format!("{} ms", (cd * 2000.0).round()), |v| set("kwinrc", "Windows", "ElectricBorderCooldown", json!((v * 2000.0).round() as i64)), true));
    let fs = kw.b("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", false);
    c = c.push(sw(p, kw, Ic::G(G::Display), "Also over full-screen windows", "Off: a game or video in full screen never sets off a corner", fs, set("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", json!(!fs)), true));
    c = c.push(ch(p, kw, Ic::Desktops, "Switch desktops at the left and right edges", "", &[("Never", 0i64), ("When moving a window", 1), ("Always", 2)], &(kw.n("kwinrc", "Windows", "ElectricBorders", 0.0) as i64), |v| set("kwinrc", "Windows", "ElectricBorders", json!(v)), true));

    c = c.push(heading(p, kw, "Between screens"));
    let eb = kw.n("kwinrc", "EdgeBarrier", "EdgeBarrier", 100.0) / 500.0;
    c = c.push(sl(p, kw, Ic::G(G::Display), "Sticky edges between screens", "How far you push the pointer before it crosses to the next screen; 0 is off", eb as f32, 0.02, format!("{} px", (eb * 500.0).round()), |v| set("kwinrc", "EdgeBarrier", "EdgeBarrier", json!((v * 500.0).round() as i64)), true));
    let cb = kw.b("kwinrc", "EdgeBarrier", "CornerBarrier", true);
    c = c.push(sw(p, kw, Ic::G(G::Display), "Corners between screens are sticky too", "", cb, set("kwinrc", "EdgeBarrier", "CornerBarrier", json!(!cb)), true));

    c = c.push(heading(p, kw, "Touchscreen: swipe in from an edge"));
    for (n, key, label) in EDGES.iter().filter(|e| [0, 2, 4, 6].contains(&e.0)) {
        c = c.push(dd(p, kw, Ic::G(G::Pad), format!("Swipe in from the {}", label.replace(" edge", "").to_lowercase()), "", act_label(kw, action_at(kw, *n, key, true)), &format!("touch:{n}")));
    }
    c = c.push(small(p, "KWin's own touch gestures stay: three fingers up for the overview, down for the grid, sideways to switch desktops."));
    if let Some(m) = msg_line(p, kw) {
        c = c.push(m);
    }
    c.into()
}
fn kit_ts<'a>(s: &'a str, c: iced::Color) -> iced::widget::Text<'a> {
    hearth_ui::ui::ts(s, sz(13.0), c)
}

// ------------------------------------------------------------------ Window behaviour
const POLICIES: &[(&str, &str, &str)] = &[
    ("ClickToFocus", "Click a window", "The window you click gets the keyboard"),
    ("FocusFollowsMouse", "Follows the pointer", "The window the pointer moves onto gets it, and keeps it over the desktop"),
    ("FocusUnderMouse", "Under the pointer", "Whichever window is under the pointer has it"),
    ("FocusStrictlyUnderMouse", "Strictly under the pointer", "The same, and no window has it over the desktop"),
];
const STEALING: &[(&str, &str, &str)] = &[
    ("0", "Off", "Any window can take the keyboard when it opens"),
    ("1", "Low", "KWin's usual: a window you didn't ask for waits if you're typing"),
    ("2", "Medium", "Stricter about windows that open by themselves"),
    ("3", "High", "Only windows you just opened get the keyboard"),
    ("4", "Extreme", "Only a click gives a window the keyboard"),
];
const DOUBLE: &[(&str, &str, &str)] = &[
    ("Maximize", "Maximize and restore", ""), ("Maximize (vertical only)", "Maximize up and down", ""), ("Maximize (horizontal only)", "Maximize side to side", ""),
    ("Minimize", "Minimize", ""), ("Shade", "Roll up into the title bar", ""), ("Lower", "Send it behind the others", ""),
    ("OnAllDesktops", "Show on every desktop", ""), ("Close", "Close", ""), ("Nothing", "Nothing", ""),
];
const WHEEL: &[(&str, &str, &str)] = &[
    ("Nothing", "Nothing", ""), ("Raise/Lower", "Bring to front / send behind", ""), ("Shade/Unshade", "Roll up / roll down", ""), ("Maximize/Restore", "Maximize / restore", ""),
    ("Above/Below", "Keep above / keep below", ""), ("Previous/Next Desktop", "Previous / next desktop", ""), ("Change Opacity", "See-through more / less", ""),
];
const MIDDLE: &[(&str, &str, &str)] = &[
    ("Nothing", "Nothing", ""), ("Lower", "Send it behind the others", ""), ("Toggle raise and lower", "Bring to front or send behind", ""),
    ("Minimize", "Minimize", ""), ("Close", "Close", ""), ("Operations menu", "Open the window menu", ""),
];
const DRAG: &[(&str, &str, &str)] = &[
    ("Move", "Move the window", ""), ("Resize", "Resize the window", ""), ("Operations menu", "Open the window menu", ""),
    ("Toggle raise and lower", "Bring to front or send behind", ""), ("Minimize", "Minimize", ""), ("Nothing", "Nothing", ""),
];
const PLACEMENTS: &[(&str, &str, &str)] = &[
    ("Centered", "In the middle of the screen", "KWin's usual"), ("Smart", "Where it covers the least", "Over as little of your other windows as it can"),
    ("UnderMouse", "Under the pointer", ""), ("Maximizing", "Maximized", "Windows that can be maximized open full size"),
    ("ZeroCornered", "Top-left corner", ""), ("Random", "Anywhere", ""),
];
fn lab(list: &[(&str, &str, &str)], v: &str) -> String {
    list.iter().find(|x| x.0 == v).map(|x| x.1.to_string()).unwrap_or_else(|| v.to_string())
}
fn desc(list: &[(&str, &str, &str)], v: &str) -> String {
    list.iter().find(|x| x.0 == v).map(|x| x.2.to_string()).unwrap_or_default()
}
/// A choice back on its default is taken out of your kwinrc, so the system value counts again.
fn put(group: &str, key: &str, v: Value, dflt: Value) -> KMsg {
    set("kwinrc", group, key, if v == dflt { Value::Null } else { v })
}
fn put_str(group: &str, key: &str, v: &str, dflt: &str) -> KMsg {
    put(group, key, json!(v), json!(dflt))
}

pub fn behaviour<'a>(p: &Pal, kw: &'a Kw) -> E<'a> {
    let mut c = gate(p, kw, "Window behaviour settings");
    let policy = kw.s("kwinrc", "Windows", "FocusPolicy", "ClickToFocus");
    let by_pointer = policy != "ClickToFocus";
    let md = if kw.s("kwinrc", "MouseBindings", "CommandAllKey", "Meta") == "Alt" { "Alt" } else { "Super" };
    c = c.push(heading(p, kw, "Focus"));
    c = c.push(dd(p, kw, Ic::Kbd, "Which window gets the keyboard", desc(POLICIES, &policy), lab(POLICIES, &policy), "focus"));
    let dfi = kw.n("kwinrc", "Windows", "DelayFocusInterval", 300.0) / 2000.0;
    c = c.push(sl(p, kw, Ic::G(G::Clock), "Wait before the focus follows", if by_pointer { "How long the pointer rests on a window before it gets the keyboard" } else { "When the focus follows the pointer" }, dfi as f32, 0.025, format!("{} ms", (dfi * 2000.0).round()), |v| put("Windows", "DelayFocusInterval", json!(((v * 40.0).round() as i64) * 50), json!(300)), by_pointer));
    let ar = kw.b("kwinrc", "Windows", "AutoRaise", false);
    c = c.push(sw(p, kw, Ic::Window, "Raise the window under the pointer", if by_pointer { "It comes to the front when the pointer rests on it" } else { "When the focus follows the pointer" }, ar, put("Windows", "AutoRaise", json!(!ar), json!(false)), by_pointer));
    if by_pointer && ar {
        let ari = kw.n("kwinrc", "Windows", "AutoRaiseInterval", 750.0) / 3000.0;
        c = c.push(sl(p, kw, Ic::G(G::Clock), "Raise after", "", ari as f32, 1.0 / 60.0, format!("{} ms", (ari * 3000.0).round()), |v| put("Windows", "AutoRaiseInterval", json!(((v * 60.0).round() as i64) * 50), json!(750)), true));
    }
    let cr = kw.b("kwinrc", "Windows", "ClickRaise", true);
    c = c.push(sw(p, kw, Ic::Mouse, "Clicking a window brings it to the front", "Off: a window you click in stays where it is in the stack", cr, put("Windows", "ClickRaise", json!(!cr), json!(true)), true));
    let nf = kw.b("kwinrc", "Windows", "NextFocusPrefersMouse", false);
    c = c.push(sw(p, kw, Ic::Mouse, "The pointer comes first", "When a window closes or you switch desktops, the window under the pointer gets the keyboard, not the last one you used", nf, put("Windows", "NextFocusPrefersMouse", json!(!nf), json!(false)), true));
    let fsp = (kw.n("kwinrc", "Windows", "FocusStealingPreventionLevel", 1.0) as i64).to_string();
    c = c.push(dd(p, kw, Ic::Lock, "Stop windows taking the keyboard", desc(STEALING, &fsp), lab(STEALING, &fsp), "steal"));

    c = c.push(heading(p, kw, "Title bar"));
    c = c.push(dd(p, kw, Ic::Window, "Double-click", "", lab(DOUBLE, &kw.s("kwinrc", "Windows", "TitlebarDoubleClickCommand", "Maximize")), "dbl"));
    c = c.push(dd(p, kw, Ic::Mouse, "Middle-click", "", lab(MIDDLE, &kw.s("kwinrc", "MouseBindings", "CommandActiveTitlebar2", "Nothing")), "middle"));
    c = c.push(dd(p, kw, Ic::Mouse, "Turning the wheel", "", lab(WHEEL, &kw.s("kwinrc", "MouseBindings", "CommandTitlebarWheel", "Nothing")), "wheel"));
    c = c.push(small(p, "A right-click on the title bar opens the window menu (with Snap and More actions)."));

    c = c.push(heading(p, kw, &format!("{md} + drag anywhere in a window")));
    let key = if md == "Alt" { "Alt".to_string() } else { "Meta".to_string() };
    c = c.push(ch(p, kw, Ic::Kbd, "Hold", "Pressed on its own, Super still opens the app menu", &[("Super", "Meta".to_string()), ("Alt", "Alt".to_string())], &key, |v| put_str("MouseBindings", "CommandAllKey", &v, "Meta"), true));
    c = c.push(dd(p, kw, Ic::Mouse, format!("{md} + left button"), "", lab(DRAG, &kw.s("kwinrc", "MouseBindings", "CommandAll1", "Move")), "all1"));
    c = c.push(dd(p, kw, Ic::Mouse, format!("{md} + right button"), "", lab(DRAG, &kw.s("kwinrc", "MouseBindings", "CommandAll3", "Resize")), "all3"));
    c = c.push(dd(p, kw, Ic::Mouse, format!("{md} + middle button"), "", lab(DRAG, &kw.s("kwinrc", "MouseBindings", "CommandAll2", "Toggle raise and lower")), "all2"));
    c = c.push(dd(p, kw, Ic::Mouse, format!("{md} + wheel"), "", lab(WHEEL, &kw.s("kwinrc", "MouseBindings", "CommandAllWheel", "Nothing")), "allwheel"));

    c = c.push(heading(p, kw, "New windows"));
    let pl = kw.s("kwinrc", "Windows", "Placement", "Centered");
    c = c.push(dd(p, kw, Ic::Window, "Where new windows open", desc(PLACEMENTS, &pl), lab(PLACEMENTS, &pl), "placement"));
    c = c.push(link(p, "Just one app's windows", "A window rule opens them where, how big and how you want", "kwin-rules"));
    if let Some(m) = msg_line(p, kw) {
        c = c.push(m);
    }
    c.into()
}

// ------------------------------------------------------------------ the drop-downs' lists
pub fn dialog<'a>(p: &Pal, kw: &'a Kw) -> Option<(E<'a>, Option<KMsg>)> {
    let id = kw.pick.as_deref()?;
    let close = KMsg::Close;
    let opts3 = |title: &str, list: &[(&str, &str, &str)], cur: String, to: &dyn Fn(&str) -> KMsg| -> E<'a> {
        let items = list.iter().map(|(v, l, d)| (l.to_string(), d.to_string(), *v == cur, to(v))).collect();
        pick_dialog(p, title, items, KMsg::Close)
    };
    let d: E<'a> = match id {
        "tabbox" | "tabbox2" => {
            let g = if id == "tabbox" { "TabBox" } else { "TabBoxAlternative" };
            let cur = look_now(kw, g);
            let items = switcher_looks(kw).into_iter().map(|(v, l)| { let on = v == cur; (l, String::new(), on, set_look(g, &v)) }).collect();
            pick_dialog(p, if id == "tabbox" { "Window switcher's look" } else { "Second switcher's look" }, items, close.clone())
        }
        "focus" => opts3("Which window gets the keyboard", POLICIES, kw.s("kwinrc", "Windows", "FocusPolicy", "ClickToFocus"), &|v| put_str("Windows", "FocusPolicy", v, "ClickToFocus")),
        "steal" => opts3("Stop windows taking the keyboard", STEALING, (kw.n("kwinrc", "Windows", "FocusStealingPreventionLevel", 1.0) as i64).to_string(), &|v| put("Windows", "FocusStealingPreventionLevel", json!(v.parse::<i64>().unwrap_or(1)), json!(1))),
        "dbl" => opts3("Double-click on the title bar", DOUBLE, kw.s("kwinrc", "Windows", "TitlebarDoubleClickCommand", "Maximize"), &|v| put_str("Windows", "TitlebarDoubleClickCommand", v, "Maximize")),
        "middle" => opts3("Middle-click on the title bar", MIDDLE, kw.s("kwinrc", "MouseBindings", "CommandActiveTitlebar2", "Nothing"), &|v| {
            let val = if v == "Nothing" { Value::Null } else { json!(v) };
            KMsg::Set("kwinrc", json!({"MouseBindings": {"CommandActiveTitlebar2": val.clone(), "CommandInactiveTitlebar2": val}}))
        }),
        "wheel" => opts3("Turning the wheel on the title bar", WHEEL, kw.s("kwinrc", "MouseBindings", "CommandTitlebarWheel", "Nothing"), &|v| put_str("MouseBindings", "CommandTitlebarWheel", v, "Nothing")),
        "all1" => opts3("Left button", DRAG, kw.s("kwinrc", "MouseBindings", "CommandAll1", "Move"), &|v| put_str("MouseBindings", "CommandAll1", v, "Move")),
        "all3" => opts3("Right button", DRAG, kw.s("kwinrc", "MouseBindings", "CommandAll3", "Resize"), &|v| put_str("MouseBindings", "CommandAll3", v, "Resize")),
        "all2" => opts3("Middle button", DRAG, kw.s("kwinrc", "MouseBindings", "CommandAll2", "Toggle raise and lower"), &|v| put_str("MouseBindings", "CommandAll2", v, "Toggle raise and lower")),
        "allwheel" => opts3("The wheel", WHEEL, kw.s("kwinrc", "MouseBindings", "CommandAllWheel", "Nothing"), &|v| put_str("MouseBindings", "CommandAllWheel", v, "Nothing")),
        "placement" => opts3("Where new windows open", PLACEMENTS, kw.s("kwinrc", "Windows", "Placement", "Centered"), &|v| put_str("Windows", "Placement", v, "Centered")),
        x if x.starts_with("edge:") || x.starts_with("touch:") => {
            let touch = x.starts_with("touch:");
            let n: i64 = x.split(':').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            let (_, key, label) = EDGES.iter().find(|e| e.0 == n).copied().unwrap_or(EDGES[0]);
            let cur = action_at(kw, n, key, touch);
            let items = ACTS.iter().map(|a| (act_label(kw, a.value), String::new(), a.value == cur, choose_edge(kw, n, key, a.value, touch))).collect();
            pick_dialog(p, if touch { "Swipe in from this edge" } else { label }, items, close.clone())
        }
        _ => return None,
    };
    Some((d, Some(close)))
}

/// After a choice in a list: close it. (The page's module calls this for every KMsg::Set while a list is open.)
pub fn closes(m: &KMsg) -> bool {
    matches!(m, KMsg::Set(..))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_kde_values_and_edges() {
        let mut kw = Kw::default();
        kw.st = Some(json!({"live": true, "defaults": {"blur": true}}));
        kw.files = json!({"kwinrc": {"Plugins": {"wobblywindowsEnabled": "true"}, "Effect-overview": {"BorderActivate": "7,1"}}}).as_object().unwrap().clone();
        assert!(kw.effect_on("blur"));
        assert!(kw.effect_on("wobblywindows"));
        assert!(!kw.effect_on("cube"));
        assert_eq!(action_at(&kw, 1, "TopRight", false), "overview");
        assert_eq!(action_at(&kw, 3, "BottomRight", false), "none");
        // Picking "Lock the screen" for the top-right corner takes it out of the overview's list.
        match choose_edge(&kw, 1, "TopRight", "LockScreen", false) {
            KMsg::Set(_, v) => {
                assert_eq!(v["Effect-overview"]["BorderActivate"], "7");
                assert_eq!(v["ElectricBorders"]["TopRight"], "LockScreen");
            }
            _ => panic!(),
        }
        let _ = kw.set_many("kwinrc", &json!({"Windows": {"AutoRaise": true}}));
        assert!(kw.b("kwinrc", "Windows", "AutoRaise", false));
        assert_eq!(norm(&[1.0, 1.0]), vec![0.5, 0.5]);
    }
}
