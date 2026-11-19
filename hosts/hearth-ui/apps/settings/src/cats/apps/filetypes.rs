//! Apps › File types (Mantel's SetFileTypes.qml, KDE's File Associations in Hearth's look): every kind of file your
//! apps open, found by name or ending, with the app it opens in; open one to pick another app, put back the system's,
//! or add and remove endings of your own. Through hearth-mime; written only when you pick.

use super::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::widget::Row;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub all: Option<Vec<Value>>,
    pub find: String,
    pub group: String,
    pub open: String,
    pub picking: bool,
    pub glob: String,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Got(Res),
    Find(String),
    Group(String),
    Toggle(String),
    Picking,
    Set(String, String, String),
    Reset(String),
    RemoveGlob(String, String),
    Glob(String),
    AddGlob(String),
    Done(String, Res),
}

fn load() -> Task<M> {
    read(py("hearth-mime", &["types"], 60), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    load()
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    let run = |args: Vec<String>, said: String| -> Task<M> {
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        read(py("hearth-mime", &a, 30), move |r| M::Done(said.clone(), r))
    };
    match m {
        M::Go(id) => sh.go(id),
        M::Got(r) => {
            if r.ok {
                st.all = Some(ga(&r.v, "types").to_vec());
            } else {
                st.all = Some(Vec::new());
                st.say.bad(r.err);
            }
        }
        M::Find(f) => st.find = f,
        M::Group(g) => st.group = g,
        M::Toggle(mime) => {
            st.open = if st.open == mime { String::new() } else { mime };
            st.picking = false;
            st.glob.clear();
        }
        M::Picking => st.picking = !st.picking,
        M::Set(mime, id, said) => {
            st.picking = false;
            sh.reread_history = true;
            return run(vec!["set".into(), mime, id], said);
        }
        M::Reset(mime) => return run(vec!["reset".into(), mime], "Back to the system's choice.".into()),
        M::RemoveGlob(mime, g) => return run(vec!["remove-glob".into(), mime, g], "Removed.".into()),
        M::Glob(g) => st.glob = g,
        M::AddGlob(mime) => {
            let t = st.glob.trim().to_string();
            if t.is_empty() {
                return Task::none();
            }
            let g = if t.starts_with("*.") { t } else { format!("*.{}", t.trim_start_matches('.')) };
            st.glob.clear();
            return run(vec!["add-glob".into(), mime, g], "Added.".into());
        }
        M::Done(said, r) => {
            if r.ok {
                st.say.ok(said);
                return load();
            }
            st.say.bad(r.err);
        }
    }
    Task::none()
}

fn strs_of(v: &Value, k: &str) -> Vec<String> {
    ga(v, k).iter().filter_map(|x| x.as_str().map(String::from)).collect()
}

fn shown<'a>(st: &'a St) -> Vec<&'a Value> {
    let Some(all) = &st.all else { return Vec::new() };
    let q = st.find.trim().to_lowercase();
    let q = q.trim_start_matches('*').trim_start_matches('.').to_string();
    all.iter()
        .filter(|t| st.group.is_empty() || gs(t, "group") == st.group)
        .filter(|t| {
            if q.is_empty() {
                return true;
            }
            let globs: Vec<String> = strs_of(t, "globs").into_iter().chain(strs_of(t, "ownGlobs")).collect();
            gs(t, "name").to_lowercase().contains(&q) || gs(t, "mime").contains(&q) || globs.iter().any(|g| g.to_lowercase().trim_start_matches("*.") == q || g.to_lowercase().contains(&q))
        })
        .take(40)
        .collect()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut v: Vec<E<M>> = vec![field(&p, "Find a kind of file, or an ending like .png", &st.find, M::Find).into()];
    let mut groups: Vec<String> = st.all.iter().flatten().map(|t| gs(t, "group")).collect();
    groups.sort();
    groups.dedup();
    let mut chips = Row::new().spacing(6.0);
    chips = chips.push(pill(&p, "All", st.group.is_empty(), Some(M::Group(String::new()))));
    for g in groups {
        chips = chips.push(pill(&p, g.clone(), st.group == g, Some(M::Group(g))));
    }
    v.push(chips.wrap().vertical_spacing(6.0).into());
    let list = shown(st);
    let title = match &st.all {
        None => "Looking at your apps…".to_string(),
        Some(all) if !list.is_empty() => format!("{} kinds of files", all.len()),
        Some(_) => "Nothing matches".into(),
    };
    let mut g: Vec<E<M>> = Vec::new();
    for ty in list {
        let mime = gs(ty, "mime");
        let group = gs(ty, "group");
        let gl = match group.as_str() {
            "Pictures" => G::Palette,
            "Videos" => G::Play,
            "Music and sound" => G::Sound,
            _ => G::List,
        };
        let globs: Vec<String> = strs_of(ty, "globs").into_iter().chain(strs_of(ty, "ownGlobs")).take(5).collect();
        let open = st.open == mime;
        g.push(expander(&p, gl, gs(ty, "name"), if globs.is_empty() { mime.clone() } else { globs.join("  ") }, if gb(ty, "yours") { Some(("Yours".into(), Chip::New)) } else { None }, gs(ty, "currentName"), open, M::Toggle(mime.clone())));
        if !open {
            continue;
        }
        let mut c: Vec<E<M>> = Vec::new();
        let apps = ga(ty, "apps");
        let cur = gs(ty, "current");
        c.push(r(G::Store, "Opens in", "").value(if gs(ty, "currentName").is_empty() { "Pick an app".to_string() } else { gs(ty, "currentName") }).chev().on(M::Picking).view(&p));
        if st.picking {
            for a in apps {
                let id = gs(a, "id");
                let name = gs(a, "name");
                c.push(r(G::Store, name.clone(), "").value(if id == cur { "In use" } else { "" }).indent().on(M::Set(mime.clone(), id, format!("{} now opens in {name}.", gs(ty, "name")))).view(&p));
            }
        }
        if gb(ty, "yours") {
            c.push(r(G::Update, "Use the system's choice", "").on(M::Reset(mime.clone())).view(&p));
        }
        for own in strs_of(ty, "ownGlobs") {
            c.push(r(G::Close, format!("Remove the ending {own}"), "").on(M::RemoveGlob(mime.clone(), own)).view(&p));
        }
        c.push(field_on(&p, "Add an ending, like *.myext", &st.glob, M::Glob, M::AddGlob(mime.clone())));
        c.push(small(&p, mime.clone()));
        g.push(inside(c));
    }
    v.push(group(&p, title, g));
    opt(&mut v, say_view(&p, &st.say));
    v.push(r(G::Grid, "Default apps", "The browser, music, pictures and other big kinds in one place").chev().on(M::Go("defaults")).view(&p));
    page(v)
}
