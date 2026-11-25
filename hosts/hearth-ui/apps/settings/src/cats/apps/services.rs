//! Apps › Background services (Mantel's SetServices.qml): the programs that run in the background for you
//! (systemd --user): which are running, and a switch for whether each starts when you sign in (on starts it now too,
//! off stops it). Hearth's own and the ones the desktop stands on are shown locked, with why (hearth-services).
//! Mantel reads the list again every 15 s; here it's read when the page opens and after each change, and "Check
//! again" reads it at once (nothing runs while the page just sits there: red flag 2).

use super::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub list: Option<Vec<Value>>,
    pub locked_open: bool,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Load,
    Set(String, String, bool),
    Done(String, Res),
    Locked,
}

fn load() -> Task<M> {
    read(py("hearth-services", &["list"], 30), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    load()
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.list = Some(ga(&r.v, "services").to_vec());
            } else {
                st.say.bad(r.err);
            }
        }
        M::Load => return load(),
        M::Set(id, name, on) => {
            let said = if on { format!("{name} starts when you sign in.") } else { format!("{name} is off.") };
            return read(py("hearth-services", &["set", &id, if on { "on" } else { "off" }], 60), move |r| M::Done(said.clone(), r));
        }
        M::Done(said, r) => {
            if r.ok {
                st.say.ok(said);
            } else {
                st.say.bad(r.err);
            }
            return load();
        }
        M::Locked => st.locked_open = !st.locked_open,
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let all = st.list.as_deref().unwrap_or(&[]);
    let mine: Vec<&Value> = all.iter().filter(|s| !gb(s, "locked")).collect();
    let locked: Vec<&Value> = all.iter().filter(|s| gb(s, "locked")).collect();
    let mut g: Vec<E<M>> = Vec::new();
    for s in &mine {
        let id = gs(s, "id");
        let on = gb(s, "enabled");
        g.push(sw(&p, G::Gear, gs(s, "name"), format!("{} · {}", if gb(s, "running") { "Running" } else { "Not running" }, id.trim_end_matches(".service")), on, Some(M::Set(id.clone(), gs(s, "name"), !on))));
    }
    if st.list.is_some() && mine.is_empty() {
        g.push(words(&p, "Nothing of yours runs in the background.", false));
    }
    if st.list.is_none() {
        g.push(words(&p, "Looking…", false));
    }
    opt(&mut g, say_view(&p, &st.say));
    g.push(r(G::Update, "Check again", "Read which services are running now").on(M::Load).view(&p));
    let mut v = crate::secs::SecVec::new();
    v.push(group(&p, "Your background services", g));
    if !locked.is_empty() {
        let mut h: Vec<E<M>> = vec![expander(&p, G::Shield, format!("{} services Hearth needs", locked.len()), "Locked: switching them off would stop sound, apps or Hearth itself", None, String::new(), st.locked_open, M::Locked)];
        if st.locked_open {
            h.push(inside(locked.iter().map(|s| r(G::Shield, gs(s, "name"), format!("{} · {}", gs(s, "why"), if gb(s, "running") { "running" } else { "not running" })).view(&p)).collect()));
        }
        v.push(group(&p, "Hearth and the desktop", h));
    }
    page(v)
}
