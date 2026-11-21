//! Apps › Window rules (Mantel's SetKwinRules.qml, docs/KWIN-SETTINGS.md): Hearth's editor for KWin's window rules
//! (kwinrulesrc) through hearth-kwin-settings. Each rule matches an app (its window class) and, if you like, a window
//! title, and sets where the window goes, its size, desktop and screen, keep above or below, no frame, see-through,
//! full screen, tearing for games and more, once when it opens, always, or remembered. Each change writes only that
//! rule's changed keys; KWin reads the rules again at once. On the Fedora edition KDE Plasma runs on KWin, so the
//! page is live whenever KWin is. `kwin-rules:<window class>` opens that app's rule (or offers to make one).

use super::kit::*;
use super::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

#[derive(Default)]
pub struct St {
    pub status: Value,
    pub rules: Vec<Value>,
    pub desktops: Vec<Value>,
    pub open: String,
    pub for_class: String,
    pub edit: HashMap<String, String>,
    pub slide: HashMap<String, u32>,
    pub saving: bool,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Status(Res),
    Rules(Res),
    Desktops(Res),
    Toggle(String),
    Change(String, Value, bool),
    Changed(bool, Res),
    Delete(String),
    Edit(String, String, String),
    Submit(String, String),
    Pick(String),
    Picked(String, Res),
    Slide(String, String, u32),
    SlideDone(String, String),
    New,
    MakeFor,
}

fn kw(args: &[&str], secs: u32) -> Vec<String> {
    py("hearth-kwin-settings", args, secs)
}

pub fn open(sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    st.edit.clear();
    st.for_class = std::mem::take(&mut sh.part);
    Task::batch([read(kw(&["status"], 30), M::Status), read(kw(&["rules"], 30), M::Rules), read(kw(&["desktops"], 30), M::Desktops)])
}

fn live(st: &St) -> bool {
    gb(&st.status, "live") || gb(&st.status, "running")
}

fn v(rule: &Value, k: &str) -> String {
    gs(go(rule, "values"), k)
}
fn on(rule: &Value, k: &str) -> bool {
    v(rule, &format!("{k}rule")).parse::<i64>().unwrap_or(0) > 1
}
fn int(rule: &Value, k: &str, d: i64) -> i64 {
    v(rule, k).parse::<i64>().unwrap_or(d)
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::Status(r) => st.status = if r.ok { r.v } else { json!({"live": false, "error": r.err}) },
        M::Rules(r) => {
            if r.ok {
                st.rules = ga(&r.v, "rules").to_vec();
                if !st.for_class.is_empty() {
                    let c = st.for_class.to_lowercase();
                    if let Some(x) = st.rules.iter().find(|x| v(x, "wmclass").to_lowercase() == c) {
                        st.open = gs(x, "id");
                    }
                }
            } else {
                st.say.bad(r.err);
            }
        }
        M::Desktops(r) => st.desktops = ga(&r.v, "desktops").to_vec(),
        M::Toggle(id) => st.open = if st.open == id { String::new() } else { id },
        M::Change(id, set, open_after) => {
            st.saving = true;
            sh.reread_history = true;
            return read_in(kw(&["rule-set"], 30), Some(json!({"id": id, "set": set}).to_string()), move |r| M::Changed(open_after, r));
        }
        M::Changed(open_after, r) => {
            st.saving = false;
            if r.ok {
                st.rules = ga(&r.v, "rules").to_vec();
                if open_after {
                    st.open = gs(&r.v, "id");
                }
                st.say.clear();
            } else {
                st.say.bad(r.err);
            }
        }
        M::Delete(id) => return read(kw(&["rule-delete", &id], 30), |r| M::Changed(false, r)),
        M::Edit(id, k, t) => {
            st.edit.insert(format!("{id}/{k}"), t);
        }
        M::Submit(id, k) => {
            let t = st.edit.remove(&format!("{id}/{k}")).unwrap_or_default();
            let val = match k.as_str() {
                "Description" => Value::String(if t.is_empty() { "Window rule".into() } else { t }),
                "position" | "size" => Value::String(t.split_whitespace().collect::<String>()),
                _ => Value::String(t),
            };
            let mut s = Map::new();
            s.insert(k, val);
            return update(sh, st, M::Change(id, Value::Object(s), false));
        }
        M::Pick(id) => return read(kw(&["pick-window"], 120), move |r| M::Picked(id.clone(), r)),
        M::Picked(id, r) => {
            if r.ok {
                return update(sh, st, M::Change(id, json!({"wmclass": gs(&r.v, "wmclass"), "wmclassmatch": 1, "wmclasscomplete": false}), false));
            }
            st.say.bad(r.err);
        }
        M::Slide(id, k, n) => {
            st.slide.insert(format!("{id}/{k}"), n);
        }
        M::SlideDone(id, k) => {
            if let Some(n) = st.slide.remove(&format!("{id}/{k}")) {
                let mut s = Map::new();
                s.insert(k, json!(n.max(10)));
                return update(sh, st, M::Change(id, Value::Object(s), false));
            }
        }
        M::New => return update(sh, st, M::Change(String::new(), json!({"Description": "New rule"}), true)),
        M::MakeFor => {
            let c = st.for_class.clone();
            return update(sh, st, M::Change(String::new(), json!({"Description": format!("{c} windows"), "wmclass": c, "wmclassmatch": 1, "wmclasscomplete": false}), true));
        }
    }
    Task::none()
}

const HOWS: [(i64, &str); 3] = [(3, "When it opens"), (2, "Always"), (4, "Remember")];
const MATCHES: [(i64, &str); 4] = [(0, "Any"), (1, "Is"), (2, "Has"), (3, "Pattern")];
const TOGGLES: [(&str, &str, Option<&str>); 12] = [
    ("above", "Keep above other windows", None),
    ("below", "Keep below other windows", None),
    ("noborder", "No title bar and frame", None),
    ("fullscreen", "Full screen", None),
    ("maximizehoriz", "Maximized", Some("maximizevert")),
    ("minimize", "Minimized", None),
    ("skiptaskbar", "Leave out of the bar", None),
    ("skipswitcher", "Leave out of Alt+Tab", None),
    ("tearing", "Allow tearing (less lag in games)", None),
    ("adaptivesync", "Variable refresh rate", None),
    ("acceptfocus", "Can take keyboard focus", None),
    ("closeable", "Can be closed", None),
];

fn set(pairs: &[(&str, Value)]) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.clone());
    }
    Value::Object(m)
}

fn summary(x: &Value) -> String {
    let vals = go(x, "values");
    let app = if v(x, "wmclass").is_empty() { "any app".to_string() } else { v(x, "wmclass") };
    let n = vals.as_object().map(|o| o.iter().filter(|(k, val)| k.ends_with("rule") && val.as_str().and_then(|s| s.trim().parse::<i64>().ok()).or(val.as_i64()).unwrap_or(0) > 1).count()).unwrap_or(0);
    let title = v(x, "title");
    let t = if title.is_empty() { String::new() } else { format!(" · title {} “{title}”", if int(x, "titlematch", 0) == 1 { "is" } else { "has" }) };
    format!("{app}{t} · {}", plural(n, "setting", "settings"))
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut out: Vec<E<M>> = Vec::new();
    if st.status.is_null() {
        out.push(words(&p, "Asking KWin…", false));
    } else if !live(st) {
        out.push(card(&p, iced::widget::column![hearth_ui::ui::tm("KWin isn't running", sz(15.0), p.text), hearth_ui::ui::t("Window rules are KWin's. They're shown as they're saved, and KWin uses them when it runs (KDE Plasma's desktop).", sz(13.0), p.muted)].spacing(4.0)));
    }
    if let Some(bad) = go(&st.status, "bad").as_object() {
        if !bad.is_empty() {
            out.push(words(&p, format!("A KWin settings file couldn't be read: {}. Hearth won't write over it; nothing in it was lost.", bad.values().map(|x| x.as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(" · ")), true));
        }
    }
    if !st.for_class.is_empty() {
        let c = st.for_class.to_lowercase();
        let n = st.rules.iter().filter(|x| v(x, "wmclass").to_lowercase() == c).count();
        let sub = if n > 0 { if n == 1 { "Its app's rule is open below.".to_string() } else { format!("Its app has {n} rules: the first is open below.") } } else { format!("No rule for {} yet. A rule sets where its windows open, their size, keep above, no frame and more.", st.for_class) };
        let row = r(G::Grid, format!("Special settings for “{}”", st.for_class), sub);
        out.push(if n == 0 { row.value("Make a rule for it").on(M::MakeFor).view(&p) } else { row.view(&p) });
    }
    out.push(head(&p, if st.saving { "Your rules · saving…" } else { "Your rules" }));
    if st.rules.is_empty() && !st.status.is_null() {
        out.push(words(&p, "No rules yet. A rule makes an app's windows open where, how big and how you want, every time.", false));
    }
    let can = live(st) || !st.status.is_null();
    for x in &st.rules {
        let id = gs(x, "id");
        let open = st.open == id;
        let enabled = v(x, "Enabled") != "false";
        let name = if v(x, "Description").is_empty() { "Window rule".to_string() } else { v(x, "Description") };
        out.push(expander(&p, G::Grid, name, summary(x), if enabled { None } else { Some(("Off".into(), Chip::Off)) }, String::new(), open, M::Toggle(id.clone())));
        if !open {
            continue;
        }
        let ed = |k: &str, d: String| -> String { st.edit.get(&format!("{id}/{k}")).cloned().unwrap_or(d) };
        let fld = |k: &'static str, ph: &str, d: String| -> E<'a, M> {
            let idc = id.clone();
            let ids = id.clone();
            field_on(&p, ph, &ed(k, d), move |t| M::Edit(idc.clone(), k.into(), t), M::Submit(ids, k.into()))
        };
        let mut c: Vec<E<M>> = Vec::new();
        c.push(fld("Description", "Name of this rule", v(x, "Description")));
        c.push(sw(&p, G::Check, "Use this rule", "", enabled, if can { Some(M::Change(id.clone(), set(&[("Enabled", if enabled { json!(false) } else { Value::Null })]), false)) } else { None }));
        let running = gb(&st.status, "running");
        c.push(r(G::Search, "Pick the app's window", if running { "Then click the window: its app and title are filled in" } else { "Needs KWin running" }).on_maybe(if running { Some(M::Pick(id.clone())) } else { None }).view(&p));
        let wm = int(x, "wmclassmatch", 0);
        c.push(choice(&p, G::Store, "App (window class)", if v(x, "wmclass").is_empty() { "Every app".to_string() } else { v(x, "wmclass") }, MATCHES.iter().map(|(n, l)| (l.to_string(), *n == wm, Some(M::Change(id.clone(), set(&[("wmclassmatch", json!(n))]), false)))).collect()));
        if wm > 0 {
            c.push(fld("wmclass", "firefox, steam, org.kde.dolphin…", v(x, "wmclass")));
        }
        let tm = int(x, "titlematch", 0);
        c.push(choice(&p, G::List, "Window title", if v(x, "title").is_empty() { "Any title".to_string() } else { v(x, "title") }, MATCHES.iter().map(|(n, l)| (l.to_string(), *n == tm, Some(M::Change(id.clone(), set(&[("titlematch", json!(n))]), false)))).collect()));
        if tm > 0 {
            c.push(fld("title", "Words in the title", v(x, "title")));
        }
        for (k, label, also) in TOGGLES {
            let is = on(x, k);
            let yes = v(x, k) == "true";
            let how = int(x, &format!("{k}rule"), 2);
            let how_label = HOWS.iter().find(|h| h.0 == how).map(|h| h.1).unwrap_or("Always");
            let keys: Vec<&str> = std::iter::once(k).chain(also).collect();
            let mut s = Map::new();
            for kk in &keys {
                s.insert(kk.to_string(), if is { Value::Null } else { json!(true) });
                s.insert(format!("{kk}rule"), if is { Value::Null } else { json!(2) });
            }
            c.push(sw(&p, G::Grid, label, if is { format!("{}, {}", if yes { "Yes" } else { "No" }, how_label.to_lowercase()) } else { "Not set by this rule".into() }, is, Some(M::Change(id.clone(), Value::Object(s), false))));
            if is {
                let mk = |val: bool| {
                    let mut s = Map::new();
                    for kk in &keys {
                        s.insert(kk.to_string(), json!(val));
                    }
                    Value::Object(s)
                };
                let mkh = |h: i64| {
                    let mut s = Map::new();
                    for kk in &keys {
                        s.insert(format!("{kk}rule"), json!(h));
                    }
                    Value::Object(s)
                };
                c.push(choice(&p, G::Grid, format!("{label}: set to"), "", vec![("Yes".into(), yes, Some(M::Change(id.clone(), mk(true), false))), ("No".into(), !yes, Some(M::Change(id.clone(), mk(false), false)))]));
                c.push(choice(&p, G::Clock, format!("{label}: when"), "", HOWS.iter().map(|(h, l)| (l.to_string(), *h == how, Some(M::Change(id.clone(), mkh(*h), false)))).collect()));
            }
        }
        for (k, label, hint, dflt) in [("position", "Position", "x,y in pixels, e.g. 0,0", "0,0"), ("size", "Size", "width,height, e.g. 1280,720", "1280,720")] {
            let is = on(x, k);
            let rk = format!("{k}rule");
            c.push(sw(&p, G::Display, label, if is { v(x, k) } else { "Not set by this rule".into() }, is, Some(M::Change(id.clone(), set(&[(k, if is { Value::Null } else { json!(dflt) }), (rk.as_str(), if is { Value::Null } else { json!(3) })]), false))));
            if is {
                c.push(fld(k, hint, v(x, k)));
                let how = int(x, &rk, 3);
                c.push(choice(&p, G::Clock, format!("{label}: how"), "", HOWS.iter().map(|(h, l)| (l.to_string(), *h == how, Some(M::Change(id.clone(), set(&[(rk.as_str(), json!(h))]), false)))).collect()));
            }
        }
        let dk = on(x, "desktops");
        let dname = st.desktops.iter().find(|d| gs(d, "id") == v(x, "desktops")).map(|d| gs(d, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| "All desktops".into());
        let first = st.desktops.first().map(|d| gs(d, "id")).unwrap_or_default();
        c.push(sw(&p, G::Grid, "On a desktop", if dk { dname } else { "Not set by this rule".into() }, dk, Some(M::Change(id.clone(), set(&[("desktops", if dk { Value::Null } else { json!(first) }), ("desktopsrule", if dk { Value::Null } else { json!(3) })]), false))));
        if dk {
            let cur = v(x, "desktops");
            let mut items: Vec<(String, bool, Option<M>)> = st.desktops.iter().filter(|d| gb(d, "id")).map(|d| (gs(d, "name"), gs(d, "id") == cur, Some(M::Change(id.clone(), set(&[("desktops", json!(gs(d, "id")))]), false)))).collect();
            items.push(("All".into(), cur.is_empty(), Some(M::Change(id.clone(), set(&[("desktops", json!(""))]), false))));
            c.push(choice_wide(&p, G::Grid, "Which desktop", "", items));
        }
        let sk = on(x, "screen");
        let scr = int(x, "screen", 0);
        c.push(sw(&p, G::Display, "On a screen", if sk { format!("Screen {}", scr + 1) } else { "Not set by this rule".into() }, sk, Some(M::Change(id.clone(), set(&[("screen", if sk { Value::Null } else { json!(0) }), ("screenrule", if sk { Value::Null } else { json!(3) })]), false))));
        if sk {
            c.push(choice(&p, G::Display, "Which screen", "", (0..4).map(|i| ((i + 1).to_string(), i == scr, Some(M::Change(id.clone(), set(&[("screen", json!(i))]), false)))).collect()));
        }
        for (k, label) in [("opacityactive", "How solid it is in front"), ("opacityinactive", "How solid it is behind others")] {
            let is = on(x, k);
            let rk = format!("{k}rule");
            let val = st.slide.get(&format!("{id}/{k}")).copied().unwrap_or(int(x, k, 100).clamp(10, 100) as u32);
            c.push(sw(&p, G::Display, label, if is { format!("{}%", int(x, k, 100)) } else { "Not set by this rule".into() }, is, Some(M::Change(id.clone(), set(&[(k, if is { Value::Null } else { json!(90) }), (rk.as_str(), if is { Value::Null } else { json!(2) })]), false))));
            if is {
                let (a1, a2, b1, b2) = (id.clone(), k.to_string(), id.clone(), k.to_string());
                c.push(srow(&p, G::Display, label, format!("{val}%"), slider_u32(val, 10..=100, move |n| M::Slide(a1.clone(), a2.clone(), n), M::SlideDone(b1, b2))));
            }
        }
        c.push(buttons(vec![button(&p, "Delete this rule", false, Some(M::Delete(id.clone())))]));
        let other: Vec<String> = ga(x, "other").iter().filter_map(|o| o.as_str().map(String::from)).collect();
        if !other.is_empty() {
            c.push(small(&p, format!("Also has settings made in KDE's own editor ({}): kept as they are.", other.join(", "))));
        }
        out.push(inside(c));
    }
    out.push(buttons(vec![button(&p, "New rule", true, Some(M::New))]));
    out.push(small(&p, "An app's window class is usually its name (firefox, steam). “Pick the app's window” finds it for you."));
    opt(&mut out, say_view(&p, &st.say));
    page(out)
}
