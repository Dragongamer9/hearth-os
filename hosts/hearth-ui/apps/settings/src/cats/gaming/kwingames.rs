//! Gaming › Full-screen games & X11 apps (Mantel's SetKwinGames.qml, docs/KWIN-SETTINGS.md): tearing in full-screen
//! games (kwinrc [Compositing] AllowTearing; per game with a window rule), screen edges over full-screen games, which
//! keys older X11 apps (Discord's push-to-talk, OBS) may see while another app is in front ([Xwayland]
//! XwaylandEavesdrops / ...Mouse), and whether X11 apps scale themselves (kdeglobals [KScreen] XwaylandClientsScale).
//! Through hearth-kwin-settings, which writes only the keys changed and asks KWin to read them again: on the Fedora
//! edition that's KDE Plasma's own KWin.

use crate::cats::apps::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::{json, Value};

#[derive(Default)]
pub struct St {
    pub status: Value,
    pub files: Value,
    pub saving: bool,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Status(Res),
    Files(Res),
    Set(&'static str, &'static str, &'static str, Value),
    Saved(Res),
}

fn kw(args: &[&str]) -> Vec<String> {
    py("hearth-kwin-settings", args, 30)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    Task::batch([read(kw(&["status"]), M::Status), read(kw(&["get"]), M::Files)])
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::Status(r) => st.status = if r.ok { r.v } else { json!({"error": r.err}) },
        M::Files(r) => {
            if r.ok {
                st.files = r.v
            } else {
                st.say.bad(r.err)
            }
        }
        M::Set(file, group, key, val) => {
            // Shown at once, saved now (Mantel waits half a second to gather a slider's steps; these are switches).
            let text = match &val {
                Value::String(s) => s.clone(),
                x => x.to_string(),
            };
            if st.files.get("files").is_none() {
                st.files = json!({"files": {}});
            }
            if let Some(files) = st.files.get_mut("files").and_then(Value::as_object_mut) {
                let f = files.entry(file.to_string()).or_insert_with(|| json!({}));
                if let Some(f) = f.as_object_mut() {
                    let g = f.entry(group.to_string()).or_insert_with(|| json!({}));
                    if let Some(g) = g.as_object_mut() {
                        g.insert(key.to_string(), Value::String(text));
                    }
                }
            }
            st.saving = true;
            sh.reread_history = true;
            let mut inner = serde_json::Map::new();
            inner.insert(key.to_string(), val);
            let mut outer = serde_json::Map::new();
            outer.insert(group.to_string(), Value::Object(inner));
            let input = Value::Object(outer).to_string();
            return read_in(kw(&["set", file]), Some(input), M::Saved);
        }
        M::Saved(r) => {
            st.saving = false;
            if r.ok {
                if r.v.get("files").is_some() {
                    st.files = r.v.clone();
                }
                if !gb(&r.v, "reloaded") && gb(&st.status, "running") {
                    st.say.ok("Saved. KWin uses it from the next time you sign in.");
                } else {
                    st.say.clear();
                }
            } else {
                st.say.bad(r.err);
                return read(kw(&["get"]), M::Files);
            }
        }
    }
    Task::none()
}

fn val(st: &St, file: &str, group: &str, key: &str) -> Option<String> {
    st.files.pointer(&format!("/files/{file}/{group}/{key}")).map(|v| match v {
        Value::String(s) => s.clone(),
        x => x.to_string(),
    })
}
fn b(st: &St, file: &str, group: &str, key: &str, d: bool) -> bool {
    match val(st, file, group, key) {
        Some(x) if !x.is_empty() => x.eq_ignore_ascii_case("true") || x == "1",
        _ => d,
    }
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut v = crate::secs::SecVec::new();
    if st.status.is_null() {
        v.push(words(&p, "Asking KWin…", false));
    } else if !gb(&st.status, "running") && !gb(&st.status, "live") {
        v.push(words(&p, "KWin isn't running: these show what's saved, and KWin reads them when it starts.", false));
    }
    if let Some(bad) = st.files.get("bad").and_then(Value::as_object) {
        if !bad.is_empty() {
            v.push(words(&p, format!("A KWin settings file couldn't be read: {}. Hearth won't write over it; nothing in it was lost.", bad.values().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" · ")), true));
        }
    }
    let ready = !st.files.is_null();
    let en = |m: M| if ready { Some(m) } else { None };
    v.push(head(&p, if st.saving { "Full-screen games · saving…" } else { "Full-screen games" }));
    let tear = b(st, "kwinrc", "Compositing", "AllowTearing", true);
    v.push(sw(&p, G::Pad, "Allow tearing in full-screen games", "Games that ask for it show each frame the moment it's ready: less lag, with a tear line now and then. Others stay smooth", tear, en(M::Set("kwinrc", "Compositing", "AllowTearing", json!(!tear)))));
    v.push(link_row(&p, G::Grid, "Tearing for one game", "A window rule can turn tearing on or off for one app", M::Go("kwin-rules")));
    let edges = b(st, "kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", false);
    v.push(sw(&p, G::Display, "Screen edges over full-screen games", "Off: hot corners never go off while a game is full screen", edges, en(M::Set("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", json!(!edges)))));
    v.push(head(&p, "Older X11 apps"));
    let eav = val(st, "kwinrc", "Xwayland", "XwaylandEavesdrops").filter(|x| !x.is_empty()).unwrap_or_else(|| "Combinations".into());
    let opts = [("None", "None"), ("Modifiers", "Shift, Ctrl, Alt, Super"), ("Combinations", "Those with another key"), ("All", "Every key")];
    v.push(choice_wide(&p, G::Grid, "Keys X11 apps can see while you're in another app", "For push-to-talk and hotkeys in Discord, OBS and other X11 apps. Wayland apps never see them", opts.iter().map(|(k, l)| (l.to_string(), eav == *k, en(M::Set("kwinrc", "Xwayland", "XwaylandEavesdrops", json!(k))))).collect()));
    let mouse = b(st, "kwinrc", "Xwayland", "XwaylandEavesdropsMouse", false);
    v.push(sw(&p, G::Pad, "Mouse buttons too", "X11 apps also see mouse buttons pressed in other apps", mouse, en(M::Set("kwinrc", "Xwayland", "XwaylandEavesdropsMouse", json!(!mouse)))));
    let selfscale = b(st, "kdeglobals", "KScreen", "XwaylandClientsScale", true);
    v.push(choice(
        &p,
        G::Display,
        "Scaling X11 apps",
        if selfscale { "Apps draw themselves at your screen's scale: sharp, though a few older ones look small" } else { "KWin enlarges them: the right size everywhere, a little blurry on a scaled screen" },
        vec![("Apps scale themselves".into(), selfscale, en(M::Set("kdeglobals", "KScreen", "XwaylandClientsScale", json!(true)))), ("KWin scales them".into(), !selfscale, en(M::Set("kdeglobals", "KScreen", "XwaylandClientsScale", json!(false))))],
    ));
    v.push(small(&p, "Most games, Steam and Proton use X11 through Xwayland."));
    opt(&mut v, say_view(&p, &st.say));
    page(v)
}
