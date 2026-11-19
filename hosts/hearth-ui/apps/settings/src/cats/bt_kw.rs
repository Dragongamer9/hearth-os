//! KWin's own settings, as Mantel's KwinSettings / KwinGate read and change them: through hearth-kwin-settings, which
//! writes only the keys changed (kwinrc, kcminputrc…) and asks KWin to read them again. Nothing is written when a page
//! opens. On KDE Plasma KWin is always there; elsewhere the pages show their KWin controls greyed.

use crate::cats::net::kit::{self, *};
use hearth_ui::ui::G;
use hearth_ui::Pal;
use iced::widget::Column;
use serde_json::Value;
use std::future::Future;

pub const H: &str = "hearth-kwin-settings";

#[derive(Default)]
pub struct Kw {
    pub st: Option<Value>,
    pub got: Option<Value>,
    pub say: Say,
}

impl Kw {
    pub fn ready(&self) -> bool {
        self.st.is_some()
    }
    pub fn live(&self) -> bool {
        self.st.as_ref().map(|v| kit::bo(v, "live")).unwrap_or(false)
    }
    pub fn running(&self) -> bool {
        self.st.as_ref().map(|v| kit::bo(v, "running")).unwrap_or(false)
    }
    pub fn preview(&self) -> bool {
        self.st.as_ref().map(|v| kit::bo(v, "preview")).unwrap_or(false)
    }
    pub fn version(&self) -> String {
        self.st.as_ref().map(|v| kit::st(v, "version")).unwrap_or_default()
    }
    /// One group of a file as KWin sees it ({key: "value"}); group names can hold "][" (ButtonRebinds][Mouse).
    pub fn group(&self, file: &str, group: &str) -> Option<&serde_json::Map<String, Value>> {
        self.got.as_ref()?.get("files")?.get(file)?.get(group)?.as_object()
    }
    pub fn v(&self, file: &str, group: &str, key: &str) -> Option<String> {
        self.group(file, group)?.get(key).map(|x| match x {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
    }
    pub fn b(&self, file: &str, group: &str, key: &str, dflt: bool) -> bool {
        match self.v(file, group, key) {
            Some(x) if !x.is_empty() => x.eq_ignore_ascii_case("true") || x == "1",
            _ => dflt,
        }
    }
    pub fn bad(&self) -> Vec<String> {
        self.got.as_ref().and_then(|g| g.get("bad")).and_then(|b| b.as_object()).map(|m| m.values().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
    }
    /// Shown at once (KWin's text form), saved by `set`.
    pub fn put(&mut self, file: &str, group: &str, key: &str, val: Option<&str>) {
        let g = self.got.get_or_insert_with(|| serde_json::json!({ "files": {} }));
        if !g.get("files").map(|f| f.is_object()).unwrap_or(false) {
            g["files"] = serde_json::json!({});
        }
        let files = g["files"].as_object_mut().unwrap();
        let f = files.entry(file.to_string()).or_insert_with(|| serde_json::json!({}));
        if !f.is_object() {
            *f = serde_json::json!({});
        }
        let gr = f.as_object_mut().unwrap().entry(group.to_string()).or_insert_with(|| serde_json::json!({}));
        if !gr.is_object() {
            *gr = serde_json::json!({});
        }
        let m = gr.as_object_mut().unwrap();
        match val {
            Some(v) => {
                m.insert(key.to_string(), Value::String(v.to_string()));
            }
            None => {
                m.remove(key);
            }
        }
    }
    /// What hearth-kwin-settings answered to a get or a set.
    pub fn take(&mut self, r: &Res) {
        match r {
            Res::Ok(v) => self.got = Some(v.clone()),
            Res::Err(e) => self.say.bad(e.clone()),
        }
    }
}

pub fn status() -> impl Future<Output = Res> + Send + 'static {
    kit::job(H, kit::a(&["status"]), String::new())
}
pub fn get() -> impl Future<Output = Res> + Send + 'static {
    kit::job(H, kit::a(&["get"]), String::new())
}
/// `changes`: {group: {key: value or null}} for one file.
pub fn set(file: &str, changes: Value) -> impl Future<Output = Res> + Send + 'static {
    kit::job(H, vec!["set".into(), file.into()], changes.to_string())
}

/// Mantel's KwinGate: what's said above KWin's controls (not running here, a preview, a file that couldn't be read).
pub fn gate<'a, M: Clone + 'a>(p: &Pal, kw: &Kw, what: &str, go: M) -> Column<'a, M> {
    let mut c = Column::new().spacing(8.0);
    if !kw.ready() {
        return c.push(words(p, "Asking KWin…"));
    }
    if !kw.live() {
        c = c.push(banner(
            p,
            G::Display,
            "Needs KWin",
            format!("{what} are KWin's, KDE Plasma's window manager. This desktop isn't running on KWin now: they work in KDE Plasma or Mantel on KWin. Nothing here is changed until then."),
            false,
            Some(("Your desktop".to_string(), go)),
        ));
    }
    if kw.preview() {
        c = c.push(banner(p, G::Info, "Preview", "KWin isn't running: these show what's saved, and KWin reads them when it starts.", false, None));
    }
    let bad = kw.bad();
    if !bad.is_empty() {
        c = c.push(banner(p, G::Info, "A KWin settings file couldn't be read", format!("{}. Hearth won't write over it; nothing in it was lost.", bad.join(" · ")), true, None));
    }
    c
}
