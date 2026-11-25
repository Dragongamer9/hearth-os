//! Apps › App menu editor (Mantel's SetMenuEditor.qml, KDE's Menu Editor in Hearth's look): find an app, change its
//! name, description, icon, command and menu category, run it in a terminal, take it off the menu, or put back how
//! it was; and add your own command to the menu. Your copy of the app's entry (hearth-menu); the system's is never
//! changed. `hearth-settings menu-editor:<app id>` opens at that app.

use super::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, tb, G};
use iced::widget::{column, row};
use iced::{Length, Task};
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub apps: Option<Vec<Value>>,
    pub cats: Vec<Value>,
    pub find: String,
    pub pick: String,
    pub f: [String; 4],
    pub cat_open: bool,
    pub new_name: String,
    pub new_exec: String,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Find(String),
    Pick(String),
    F(usize, String),
    Save(Value, String),
    SaveFields,
    SaveOne(usize),
    CatOpen,
    Saved(String, Res),
    Reset,
    NewName(String),
    NewExec(String),
    New,
    Made(Res),
}

const KEYS: [&str; 4] = ["name", "comment", "icon", "exec"];

fn load() -> Task<M> {
    read(py("hearth-menu", &["list"], 60), M::Got)
}

fn cur<'a>(st: &'a St) -> Option<&'a Value> {
    if st.pick.is_empty() {
        return None;
    }
    st.apps.as_ref()?.iter().find(|a| gs(a, "id") == st.pick || gs(a, "id") == format!("{}.desktop", st.pick))
}

fn fill(st: &mut St) {
    if let Some(a) = cur(st).cloned() {
        for (i, k) in KEYS.iter().enumerate() {
            st.f[i] = gs(&a, k);
        }
    }
}

pub fn open(sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    st.pick = std::mem::take(&mut sh.part);
    load()
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.apps = Some(ga(&r.v, "apps").to_vec());
                st.cats = ga(&r.v, "categories").to_vec();
                fill(st);
            } else {
                st.apps = Some(Vec::new());
                st.say.bad(r.err);
            }
        }
        M::Find(f) => st.find = f,
        M::Pick(id) => {
            st.pick = id;
            st.cat_open = false;
            fill(st);
        }
        M::F(i, t) => st.f[i] = t,
        M::SaveOne(i) => {
            let mut ch = serde_json::Map::new();
            ch.insert(KEYS[i].to_string(), Value::String(st.f[i].clone()));
            return update(_sh, st, M::Save(Value::Object(ch), String::new()));
        }
        M::SaveFields => {
            let ch = serde_json::json!({"name": st.f[0], "comment": st.f[1], "icon": st.f[2], "exec": st.f[3]});
            return update(_sh, st, M::Save(ch, String::new()));
        }
        M::Save(ch, ok) => {
            let Some(id) = cur(st).map(|a| gs(a, "id")) else { return Task::none() };
            st.say.clear();
            st.cat_open = false;
            let ok = if ok.is_empty() { "Saved. The menu shows it in a moment.".to_string() } else { ok };
            return read_in(py("hearth-menu", &["save", &id], 30), Some(ch.to_string()), move |r| M::Saved(ok.clone(), r));
        }
        M::CatOpen => st.cat_open = !st.cat_open,
        M::Saved(ok, r) => {
            if r.ok {
                st.say.ok(ok);
                return load();
            }
            st.say.bad(r.err);
        }
        M::Reset => {
            let Some(id) = cur(st).map(|a| gs(a, "id")) else { return Task::none() };
            st.pick.clear();
            return read(py("hearth-menu", &["reset", &id], 30), |r| M::Saved("Done.".into(), r));
        }
        M::NewName(t) => st.new_name = t,
        M::NewExec(t) => st.new_exec = t,
        M::New => {
            let input = serde_json::json!({"name": st.new_name, "exec": st.new_exec}).to_string();
            return read_in(py("hearth-menu", &["new"], 30), Some(input), M::Made);
        }
        M::Made(r) => {
            if r.ok {
                st.say.ok("Added to the menu.");
                st.new_name.clear();
                st.new_exec.clear();
                st.pick = gs(&r.v, "id");
                return load();
            }
            st.say.bad(r.err);
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut v = crate::secs::SecVec::new();
    if let Some(a) = cur(st) {
        let yours = gb(a, "yours");
        let mut top = row![icon_box(&p, G::Store), column![tb(gs(a, "name"), sz(19.0), p.text), t(if gs(a, "comment").is_empty() { gs(a, "id") } else { gs(a, "comment") }, sz(13.0), p.muted)].spacing(2.0).width(Length::Fill)].spacing(18.0).align_y(iced::Center);
        if yours {
            top = top.push(chip_of(&p, "Changed by you", Chip::New));
        }
        top = top.push(pill(&p, "Other app", false, Some(M::Pick(String::new()))));
        v.push(card(&p, top));
        let labels = ["Name", "Description", "Icon (a name from the icon theme, or a picture's place)", "Command"];
        let mut c: Vec<E<M>> = Vec::new();
        for (i, l) in labels.iter().enumerate() {
            c.push(labelled(&p, l, "", &st.f[i], move |t| M::F(i, t), Some(M::SaveOne(i))));
        }
        c.push(r(G::Check, "Save the name, description, icon and command", "").on(M::SaveFields).view(&p));
        let cat = gs(a, "category");
        let cat_label = st.cats.iter().find(|c| gs(c, "id") == cat).map(|c| gs(c, "label")).unwrap_or_else(|| "No category".into());
        c.push(r(G::Grid, "Menu category", "").value(cat_label).chev().on(M::CatOpen).view(&p));
        if st.cat_open {
            let mut opts = vec![(String::new(), "No category".to_string())];
            opts.extend(st.cats.iter().map(|c| (gs(c, "id"), gs(c, "label"))));
            for (id, label) in opts {
                let on = id == cat;
                c.push(r(G::Grid, label, "").value(if on { "In use" } else { "" }).indent().on(M::Save(serde_json::json!({"category": id}), String::new())).view(&p));
            }
        }
        let term = gb(a, "terminal");
        c.push(sw(&p, G::Tasks, "Run in a terminal", "For programs that type in a terminal window", term, Some(M::Save(serde_json::json!({"terminal": !term}), String::new()))));
        let shown = !gb(a, "hidden");
        c.push(sw(&p, G::Display, "Show in the app menu", "Off takes it off the menu and search; it stays installed", shown, Some(M::Save(serde_json::json!({"hidden": shown}), if shown { "Taken off the menu.".into() } else { "It's back on the menu.".into() }))));
        if yours {
            let own = gb(a, "own");
            c.push(r(G::Update, if own { "Remove this entry" } else { "Put back how it was" }, if own { "Your own command, taken off the menu" } else { "Your changes to it taken out" }).on(M::Reset).view(&p));
        }
        v.push(group(&p, "Change it", c));
    } else {
        let mut c: Vec<E<M>> = vec![field(&p, "Find an app", &st.find, M::Find).into()];
        let q = st.find.trim().to_lowercase();
        for a in st.apps.iter().flatten().filter(|a| q.is_empty() || gs(a, "name").to_lowercase().contains(&q) || gs(a, "id").to_lowercase().contains(&q)).take(if q.is_empty() { 12 } else { 30 }) {
            c.push(r(G::Store, gs(a, "name"), if gb(a, "hidden") { "Not on the menu".into() } else { gs(a, "comment") }).chip(if gb(a, "yours") { "Changed" } else { "" }, Chip::New).chev().on(M::Pick(gs(a, "id"))).view(&p));
        }
        if st.apps.is_none() {
            c.push(words(&p, "Looking at your apps…", false));
        }
        v.push(group(&p, "Pick an app to change", c));
        v.push(group(
            &p,
            "Add your own",
            vec![
                field(&p, "Name, like Backup script", &st.new_name, M::NewName).into(),
                field(&p, "Command, like sh ~/bin/backup.sh", &st.new_exec, M::NewExec).into(),
                r(G::Plus, "Add to the menu", "").on(M::New).view(&p),
            ],
        ));
    }
    opt(&mut v, say_view(&p, &st.say));
    page(v)
}
