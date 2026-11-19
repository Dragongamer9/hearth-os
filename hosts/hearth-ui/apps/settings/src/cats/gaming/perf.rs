//! Gaming › Performance (Mantel's SetGamePerf.qml; the Display page's "Related" row leads here, mockup
//! settings-A2-display-mouse.png): Game Mode's resolution, refresh rate, frame limit and HDR, set from the desktop.
//! Everything goes through hearth-gamemode-display (session.conf's NIMBUS_GAMESCOPE_ARGS line only: atomic, .bak,
//! under the lock); nothing is written until you pick something, and it's used the next time Game Mode starts. A
//! mode the screen can't show doesn't leave the PC stuck: the session starts Game Mode at the screen's own mode.

use crate::cats::apps::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::{json, Value};

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub msg: String,
    pub said: String,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Set(Value, String),
    Saved(String, Res),
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.msg.clear();
    st.said.clear();
    read(py("hearth-gamemode-display", &["get"], 30), M::Got)
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.st = r.v
            } else {
                st.msg = r.err
            }
        }
        M::Set(ch, said) => {
            sh.reread_history = true;
            return read(py("hearth-gamemode-display", &["set", &ch.to_string()], 30), move |r| M::Saved(said.clone(), r));
        }
        M::Saved(said, r) => {
            if r.ok {
                st.st = r.v;
                st.msg.clear();
                st.said = format!("{said} · from the next time Game Mode starts");
            } else {
                st.msg = r.err;
            }
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let scr = go(s, "screen");
    let modes = ga(scr, "modes");
    let (w, h) = (gn(s, "w") as i64, gn(s, "h") as i64);
    let size = if w > 0 { format!("{w}x{h}") } else { "auto".into() };
    let mut v: Vec<E<M>> = Vec::new();
    if !st.msg.is_empty() {
        v.push(card(&p, iced::widget::column![hearth_ui::ui::tm("That didn't work", sz(15.0), p.danger), hearth_ui::ui::t(st.msg.clone(), sz(13.0), p.muted)].spacing(4.0)));
    }
    if !st.said.is_empty() {
        v.push(words(&p, st.said.clone(), false));
    }
    v.push(head(&p, "Game Mode's screen"));
    let model = gs(scr, "model");
    let sub = if !model.is_empty() {
        format!("On {model}{}", modes.first().map(|m| format!(" · its own is {} × {}", gs(m, "w"), gs(m, "h"))).unwrap_or_default())
    } else {
        "The screen's own unless you pick one".into()
    };
    let mut opts: Vec<(String, bool, Option<M>)> = vec![("The screen's own".into(), size == "auto", Some(M::Set(json!({"w": 0, "h": 0, "r": 0}), "The screen's own resolution".into())))];
    for m in modes.iter().take(3) {
        let (mw, mh) = (gn(m, "w") as i64, gn(m, "h") as i64);
        opts.push((format!("{mw} × {mh}"), size == format!("{mw}x{mh}"), Some(M::Set(json!({"w": mw, "h": mh}), format!("{mw} × {mh}")))));
    }
    if size != "auto" && !modes.iter().take(3).any(|m| format!("{}x{}", gs(m, "w"), gs(m, "h")) == size) {
        opts.push((format!("{w} × {h}"), true, None));
    }
    v.push(choice_wide(&p, G::Display, "Resolution", sub, opts));
    let cur = if w > 0 { modes.iter().find(|m| gn(m, "w") as i64 == w && gn(m, "h") as i64 == h) } else { modes.first() };
    let rates: Vec<i64> = cur.map(|m| ga(m, "rates").iter().filter_map(|x| x.as_f64().map(|f| f.round() as i64)).collect()).unwrap_or_default();
    if rates.len() > 1 {
        let rr = gn(s, "r") as i64;
        let mut o = vec![("Highest".to_string(), rr == 0, Some(M::Set(json!({"r": 0}), "The highest refresh rate".into())))];
        for x in rates.iter().take(3) {
            o.push((format!("{x} Hz"), rr == *x, Some(M::Set(json!({"r": x}), format!("{x} Hz")))));
        }
        v.push(choice(&p, G::Power, "Refresh rate", "Higher is smoother; lower saves battery", o));
    }
    let has = go(s, "has");
    let can_limit = s.is_null() || gb(has, "limit");
    let limit = gn(s, "limit") as i64;
    let lo: Vec<(String, bool, Option<M>)> = [0, 30, 40, 60, 90, 120]
        .iter()
        .map(|&n| {
            let label = if n == 0 { "Off".to_string() } else { n.to_string() };
            let said = if n == 0 { "No frame limit".to_string() } else { format!("Limited to {n} frames a second") };
            (label, limit == n, if can_limit || n == 0 { Some(M::Set(json!({"limit": n}), said)) } else { None })
        })
        .collect();
    v.push(choice(&p, G::Rocket, "Frame limit", if can_limit { "Steadier and cooler; games can still set their own in Steam's Quick Access" } else { "This PC's gamescope can't limit the frame rate" }, lo));
    let hdr_screen = gb(scr, "hdr");
    let hdr_ok = hdr_screen && gb(has, "hdr");
    let hdr = gb(s, "hdr");
    let hsub = if scr.is_null() { "" } else if !hdr_screen { "This screen doesn't have HDR" } else if !gb(has, "hdr") { "This PC's gamescope can't do HDR" } else { "Brighter, more vivid picture in games and videos that have it" };
    v.push(sw(&p, G::Palette, "HDR", hsub, hdr, if hdr_ok { Some(M::Set(json!({"hdr": !hdr}), if hdr { "HDR off".into() } else { "HDR on".into() })) } else { None }));
    v.push(r(G::Info, "Used the next time Game Mode starts", "If the screen can't show what you picked, Game Mode starts at the screen's own mode and tells you.").view(&p));
    page(v)
}
