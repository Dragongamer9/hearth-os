//! Apps › Default apps (Mantel's SetDefaults.qml): which installed app opens web links, folders, text, pictures,
//! videos, music, PDFs, office files, calendars, contacts, maps and archives, and which terminal starts. hearth-system
//! defaults / default-set / default-reset, the same as on Debian: your choice is saved in ~/.config/mimeapps.list,
//! which KDE Plasma's own Default Applications and Open With read too.

use super::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub kinds: Vec<Value>,
    pub read: bool,
    pub open: String,
    pub confirm: bool,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Toggle(String),
    Choose(String, String, String, String),
    ResetOne(String, String, String),
    Confirm(bool),
    ResetAll,
    Saved(String, Res),
}

fn load() -> Task<M> {
    read(py("hearth-system", &["defaults"], 60), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.open.clear();
    st.confirm = false;
    st.say.clear();
    load()
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            st.read = true;
            if r.ok {
                st.kinds = ga(&r.v, "kinds").to_vec();
            } else {
                st.say.bad(r.err);
            }
        }
        M::Toggle(k) => st.open = if st.open == k { String::new() } else { k },
        M::Choose(kind, id, name, label) => {
            let cur = st.kinds.iter().find(|k| gs(k, "kind") == kind).map(|k| gs(k, "current")).unwrap_or_default();
            if cur == id {
                st.open.clear();
                return Task::none();
            }
            sh.reread_history = true;
            let said = format!("{name} opens {} now.", label.to_lowercase());
            return read(py("hearth-system", &["default-set", &kind, &id], 30), move |r| M::Saved(said.clone(), r));
        }
        M::ResetOne(kind, hearth_name, label) => {
            let said = format!("{hearth_name} opens {} again.", label.to_lowercase());
            return read(py("hearth-system", &["default-reset", &kind], 30), move |r| M::Saved(said.clone(), r));
        }
        M::Confirm(c) => st.confirm = c,
        M::ResetAll => return read(py("hearth-system", &["default-reset"], 30), |r| M::Saved("Hearth's default apps are back.".into(), r)),
        M::Saved(said, r) => {
            if r.ok {
                st.say.ok(said);
                st.open.clear();
                st.confirm = false;
                return load();
            }
            st.say.bad(r.err);
        }
    }
    Task::none()
}

fn icon(kind: &str) -> G {
    match kind {
        "browser" | "maps" => G::Link,
        "files" | "archives" => G::Folder,
        "text" | "ebooks" | "documents" => G::List,
        "image" => G::Palette,
        "video" | "slides" => G::Display,
        "music" => G::Sound,
        "sheets" => G::Grid,
        "terminal" => G::Tasks,
        "email" => G::Moon,
        "calendar" => G::Clock,
        "contacts" => G::User,
        "store" => G::Store,
        _ => G::Grid,
    }
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut g: Vec<E<M>> = vec![words(&p, "Which app opens links and files, in Desktop Mode and Game Mode. Apps you install that can open them show up here.", false)];
    for k in &st.kinds {
        let opts = ga(k, "options");
        if opts.is_empty() && !gb(k, "current") && !gb(k, "hearth") {
            continue;
        }
        let kind = gs(k, "kind");
        let label = gs(k, "label");
        let cur = gs(k, "current");
        let sub = if cur.is_empty() {
            if opts.is_empty() { "No installed app can do this yet".to_string() } else { "None chosen".into() }
        } else {
            format!("{}{}", gs(k, "currentName"), if gb(k, "isHearth") { " · Hearth's own" } else if gb(k, "yours") { " · your choice" } else { "" })
        };
        let open = st.open == kind;
        g.push(r(icon(&kind), label.clone(), sub).value(if open { "Pick one below".to_string() } else { plural(opts.len(), "app", "apps") }).on(M::Toggle(kind.clone())).view(&p));
        if open {
            for o in opts {
                let id = gs(o, "id");
                let in_use = id == cur;
                g.push(r(G::Store, gs(o, "name"), "").value(if in_use { "In use" } else if gb(o, "hearth") { "Hearth's default" } else { "" }).indent().on(M::Choose(kind.clone(), id, gs(o, "name"), label.clone())).view(&p));
            }
            if gb(k, "yours") && gb(k, "hearth") {
                g.push(r(G::Update, "Reset to Hearth's default", gs(k, "hearthName")).indent().on(M::ResetOne(kind.clone(), gs(k, "hearthName"), label.clone())).view(&p));
            }
        }
    }
    let mut d: Vec<E<M>> = Vec::new();
    if !st.kinds.is_empty() {
        let any = st.kinds.iter().any(|k| gb(k, "yours"));
        if !st.confirm {
            d.push(r(G::Update, "Reset to Hearth's defaults…", if any { "Hearth's own apps open everything again; your choices above are forgotten" } else { "Everything already opens in the app Hearth chose" }).on(M::Confirm(true)).view(&p));
        } else {
            d.push(buttons(vec![button(&p, "Yes, reset them", true, Some(M::ResetAll)), button(&p, "Cancel", false, Some(M::Confirm(false)))]));
        }
    }
    opt(&mut d, say_view(&p, &st.say));
    if st.kinds.is_empty() {
        d.push(words(&p, if st.read { "Couldn't read the installed apps." } else { "Looking at your apps…" }, false));
    }
    page(vec![group(&p, "Open with", g), group(&p, "Hearth's defaults", d)])
}
