//! Gaming › Saved games (Mantel's SetSavedGames.qml): every game's saves, found by Ludusavi (hearth-saves), backed up
//! into your Saved Games folder and put back on this PC or another. Opening it only reads; Back up and Restore are
//! buttons, and a game's row backs up (or restores) just that game.

use crate::cats::apps::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub busy: Busy,
    pub what: String,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Run(Vec<String>, String, String),
    Ev(String, Ev),
}

fn refresh() -> Task<M> {
    read(py("hearth-saves", &["status"], 120), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    refresh()
}

fn size(b: f64) -> String {
    if b >= 1073741824.0 {
        format!("{:.1} GB", b / 1073741824.0)
    } else if b >= 1048576.0 {
        format!("{} MB", (b / 1048576.0).round())
    } else {
        format!("{} KB", (b / 1024.0).round().max(1.0))
    }
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.st = r.v
            } else {
                st.say.bad(r.err)
            }
        }
        M::Run(args, what, kind) => {
            if st.busy.on {
                return Task::none();
            }
            st.busy.start();
            st.what = what;
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return job(py("hearth-saves", &a, 3600), None, move |e| M::Ev(kind.clone(), e));
        }
        M::Ev(kind, e) => match e {
            Ev::Line(f, l) => st.busy.step(f, l),
            Ev::Done(r) => {
                st.busy.stop();
                if !r.ok {
                    st.say.bad(r.err);
                    return Task::none();
                }
                let said = match kind.as_str() {
                    "backup" => format!("Backed up {} games ({}) ✓", gs(&r.v, "games"), size(gn(&r.v, "bytes"))),
                    "restore" => format!("Restored {} games ✓", gs(&r.v, "games")),
                    k => format!("{k} ✓"),
                };
                st.say.ok(said);
                return refresh();
            }
        },
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let mut v: Vec<E<M>> = Vec::new();
    if !s.is_null() && !gb(s, "ludusavi") {
        v.push(card(&p, iced::widget::column![hearth_ui::ui::tm("Ludusavi isn't on this Hearth", sz(15.0), p.text), hearth_ui::ui::t("Saves can't be found here without it.", sz(13.0), p.muted)].spacing(4.0)));
    }
    let games = ga(s, "games");
    let here = games.iter().filter(|g| g.get("here").and_then(Value::as_bool) != Some(false)).count();
    let sub = if s.is_null() { "Looking for saves…".to_string() } else { format!("{here} games with saves on this PC · into {}{}", gs(s, "folder"), if gb(s, "last") { format!(" · last {}", gs(s, "last")) } else { String::new() }) };
    let idle = !st.busy.on;
    v.push(r(G::Update, if idle { "Back up all saves".to_string() } else { st.what.clone() }, sub).on_maybe(idle.then(|| M::Run(vec!["backup".into()], "Backing up…".into(), "backup".into()))).view(&p));
    v.push(r(G::Update, "Restore all saves", "Puts every backed-up save back where its game looks for it (after a reinstall, or on another PC). What it replaces is kept by Ludusavi.").on_maybe(idle.then(|| M::Run(vec!["restore".into()], "Restoring…".into(), "restore".into()))).view(&p));
    opt(&mut v, busy_view(&p, &st.busy, &st.what));
    if !st.say.text.is_empty() {
        v.push(words(&p, st.say.text.clone(), st.say.bad));
    }
    v.push(head(&p, "Games"));
    for g in games {
        let name = gs(g, "name");
        let only_backup = g.get("here").and_then(Value::as_bool) == Some(false);
        let (args, kind) = if only_backup { (vec!["restore".to_string(), name.clone()], format!("Restored {name}")) } else { (vec!["backup".to_string(), name.clone()], format!("Backed up {name}")) };
        v.push(
            r(G::Pad, name.clone(), format!("{} · {} files{}", size(gn(g, "bytes")), gs(g, "files"), if only_backup { " · only in the backup" } else { "" }))
                .chip(if gb(g, "backedUp") { "backed up" } else { "not backed up" }, if gb(g, "backedUp") { Chip::Ok } else { Chip::Warn })
                .on_maybe(idle.then(|| M::Run(args, format!("{name}…"), kind)))
                .view(&p),
        );
    }
    page(v)
}
