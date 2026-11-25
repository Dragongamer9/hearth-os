//! Apps › Autostart (Mantel's SetAutostart.qml, KDE's Autostart page in Hearth's look): the apps and commands that
//! start when you sign in, each with a switch; add an installed app (the menu's apps, from hearth-menu) or a command
//! of your own; remove one you added. The XDG autostart files KDE Plasma reads too (hearth-autostart).

use super::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub items: Option<Vec<Value>>,
    pub apps: Vec<Value>,
    pub find: String,
    pub cmd_name: String,
    pub cmd_line: String,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Got(Res),
    Apps(Res),
    Set(String, bool),
    Remove(String, String),
    Find(String),
    AddApp(String, String),
    CmdName(String),
    CmdLine(String),
    AddCmd,
    Done(String, bool, Res),
}

fn load() -> Task<M> {
    read(py("hearth-autostart", &["list"], 30), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    Task::batch([load(), read(py("hearth-menu", &["list"], 60), M::Apps)])
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::Got(r) => {
            if r.ok {
                st.items = Some(ga(&r.v, "items").to_vec());
            } else {
                st.say.bad(r.err);
            }
        }
        M::Apps(r) => st.apps = ga(&r.v, "apps").to_vec(),
        M::Set(file, on) => return read(py("hearth-autostart", &["set", &file, if on { "on" } else { "off" }], 30), |r| M::Done(String::new(), true, r)),
        M::Remove(file, name) => return read(py("hearth-autostart", &["remove", &file], 30), move |r| M::Done(format!("{name} removed."), false, r)),
        M::Find(f) => st.find = f,
        M::AddApp(id, name) => {
            let id = if id.ends_with(".desktop") { id } else { format!("{id}.desktop") };
            return read(py("hearth-autostart", &["add-app", &id], 30), move |r| M::Done(format!("{name} starts when you sign in."), false, r));
        }
        M::CmdName(t) => st.cmd_name = t,
        M::CmdLine(t) => st.cmd_line = t,
        M::AddCmd => {
            let input = serde_json::json!({"name": st.cmd_name, "command": st.cmd_line}).to_string();
            return read_in(py("hearth-autostart", &["add-command"], 30), Some(input), |r| M::Done("Added.".into(), false, r));
        }
        M::Done(said, reload_on_fail, r) => {
            if r.ok {
                if said == "Added." {
                    st.cmd_name.clear();
                    st.cmd_line.clear();
                }
                if !said.is_empty() {
                    st.say.ok(said);
                }
                return load();
            }
            st.say.bad(r.err);
            if reload_on_fail {
                return load();
            }
        }
    }
    Task::none()
}

fn unquote(c: &str) -> String {
    let c = c.strip_prefix("sh -c \"").and_then(|x| x.strip_suffix('"')).unwrap_or(c);
    c.replace("\\\"", "\"").replace("\\\\", "\\").replace("%%", "%")
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut g: Vec<E<M>> = Vec::new();
    if let Some(items) = &st.items {
        if items.is_empty() {
            g.push(r(G::Info, "Nothing starts by itself yet", "Add an app or a command below").view(&p));
        }
        for it in items {
            let file = gs(it, "file");
            let cmd = gs(it, "command");
            let on = gb(it, "on");
            let can = gb(it, "canChange");
            g.push(sw(&p, if gb(it, "added") && !cmd.is_empty() { G::List } else { G::Store }, gs(it, "name"), if cmd.is_empty() { gs(it, "sub") } else { unquote(&cmd) }, on, if can { Some(M::Set(file.clone(), !on)) } else { None }));
            if gb(it, "yours") {
                g.push(r(G::Close, format!("Remove {}", gs(it, "name")), "").danger().on(M::Remove(file, gs(it, "name"))).view(&p));
            }
        }
    }
    let have: Vec<String> = st.items.iter().flatten().map(|i| gs(i, "file")).collect();
    let q = st.find.trim().to_lowercase();
    let mut apps: Vec<&Value> = st
        .apps
        .iter()
        .filter(|a| !gb(a, "hidden"))
        .filter(|a| {
            let id = gs(a, "id");
            let full = if id.ends_with(".desktop") { id.clone() } else { format!("{id}.desktop") };
            !have.contains(&full) && !have.contains(&id)
        })
        .filter(|a| q.is_empty() || gs(a, "name").to_lowercase().contains(&q))
        .collect();
    apps.sort_by_key(|a| gs(a, "name").to_lowercase());
    let mut add: Vec<E<M>> = vec![field(&p, "Find an app", &st.find, M::Find).into()];
    for a in apps.into_iter().take(if q.is_empty() { 8 } else { 20 }) {
        add.push(r(G::Store, gs(a, "name"), gs(a, "comment")).chip("Add", Chip::New).on(M::AddApp(gs(a, "id"), gs(a, "name"))).view(&p));
    }
    let cmd = vec![
        field(&p, "A name for it, like Mount my NAS", &st.cmd_name, M::CmdName).into(),
        field_on(&p, "The command, like rclone mount nas: ~/NAS", &st.cmd_line, M::CmdLine, M::AddCmd),
        r(G::Plus, "Add this command", "Runs once each time you sign in").on(M::AddCmd).view(&p),
    ];
    let mut v = crate::secs::SecVec::new();
    v.push(group(&p, "Start when you sign in", g));
    v.push(group(&p, "Add an app", add));
    v.push(group(&p, "Add a command", cmd));
    opt(&mut v, say_view(&p, &st.say));
    v.push(r(G::Gear, "Background services", "Programs that run in the background and Hearth's own parts").chev().on(M::Go("services")).view(&p));
    page(v)
}
