//! Apps › Installed apps (Mantel's SetApps.qml): Hearth's Debian box for .deb files (hearth-debbox: on the Fedora
//! edition the system is an image, so a .deb goes into the box), Flathub through Bazaar (hearth-flatpak), your
//! Flathub apps with Remove (the Fedora edition's apps mostly come from Flathub), and AppImages.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub boxst: Value,
    pub packages: Vec<Value>,
    pub flat: Value,
    pub flatpaks: Option<Vec<Value>>,
    pub confirm: String,
    pub say: Say,
    pub busy: Busy,
}

#[derive(Debug, Clone)]
pub enum M {
    Box(Res),
    List(Res),
    Flat(Res),
    Flatpaks(Res),
    Confirm(String),
    Work(Vec<String>, String),
    Ev(String, Ev),
    Bazaar,
    Opened(Res),
    Downloads,
    Repair,
    Repaired(Res),
    RemoveFlatpak(String, String),
}

fn load() -> Task<M> {
    Task::batch([
        read(py("hearth-debbox", &["status"], 60), M::Box),
        read(py("hearth-debbox", &["list"], 60), M::List),
        read(py("hearth-flatpak", &["status"], 30), M::Flat),
        read(py("hearth-flatpak", &["installed"], 30), M::Flatpaks),
    ])
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.confirm.clear();
    st.say.clear();
    load()
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Box(r) => {
            if r.ok {
                st.boxst = r.v
            } else {
                st.boxst = serde_json::json!({"problem": r.err});
                st.say.bad(r.err)
            }
        }
        M::List(r) => st.packages = ga(&r.v, "packages").to_vec(),
        M::Flat(r) => st.flat = if r.ok { r.v } else { serde_json::json!({"bazaarMissing": r.err}) },
        M::Flatpaks(r) => st.flatpaks = Some(ga(&r.v, "apps").to_vec()),
        M::Confirm(c) => st.confirm = if st.confirm == c { String::new() } else { c },
        M::Work(args, ok) => {
            if st.busy.on {
                return Task::none();
            }
            st.confirm.clear();
            st.say.clear();
            st.busy.start();
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            let cmd = if a.first() == Some(&"flatpak-remove") { py("hearth-flatpak", &["remove", a[1]], 900) } else { py("hearth-debbox", &a, 1800) };
            return job(cmd, None, move |e| M::Ev(ok.clone(), e));
        }
        M::Ev(ok, e) => match e {
            Ev::Line(f, l) => st.busy.step(f, l),
            Ev::Done(r) => {
                st.busy.stop();
                if r.ok {
                    st.say.ok(ok)
                } else {
                    st.say.bad(r.err)
                }
                return load();
            }
        },
        M::Bazaar => return read(py("hearth-flatpak", &["open-bazaar"], 30), M::Opened),
        M::Opened(r) => {
            if r.ok {
                st.say.ok("Opening Bazaar…")
            } else {
                st.say.bad(r.err)
            }
        }
        M::Downloads => kit::open(&["hearth-filemanager", &format!("{}/Downloads", home())]),
        M::Repair => return read(admin(&["home-repair"], 120), M::Repaired),
        M::Repaired(r) => {
            if r.ok {
                st.say.ok("Your Flatpak folders are yours again.");
                return load();
            }
            st.say.bad(format!("Couldn't fix it: {}", r.err))
        }
        M::RemoveFlatpak(id, name) => {
            if st.confirm != format!("fp:{id}") {
                st.confirm = format!("fp:{id}");
            } else {
                return update(_sh, st, M::Work(vec!["flatpak-remove".into(), id], format!("{name} was removed.")));
            }
        }
    }
    Task::none()
}

fn box_line(b: &Value) -> String {
    if b.is_null() {
        return "Looking…".into();
    }
    if gb(b, "problem") {
        return gs(b, "problem");
    }
    if !gb(b, "box") {
        return "Not set up yet: it's made the first time you install a .deb file (1–2 minutes and a download).".into();
    }
    let n = gn(b, "count") as usize;
    format!(
        "Ready{} · {} installed through Hearth{}",
        if gb(b, "size") { format!(" · {}", gs(b, "size")) } else { String::new() },
        plural(n, "package", "packages"),
        if gb(b, "nvidia") { " · Nvidia graphics shared" } else { "" }
    )
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let b = &st.boxst;
    let mut deb: Vec<E<M>> = Vec::new();
    let limits = gs(b, "limits");
    deb.push(r(G::Store, "Hearth's Debian box", if limits.is_empty() { box_line(b) } else { format!("{}\n{limits}", box_line(b)) }).view(&p));
    opt(&mut deb, busy_view(&p, &st.busy, "Working…"));
    opt(&mut deb, say_view(&p, &st.say));
    for pk in &st.packages {
        let name = gs(pk, "package");
        let apps: Vec<String> = ga(pk, "apps").iter().map(|a| gs(a, "name")).collect();
        let sub = [gs(pk, "summary"), if apps.is_empty() { "No app in the menu".into() } else { format!("In your menu: {}", apps.join(", ")) }].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
        let open = st.confirm == name;
        let ver = gs(pk, "version");
        deb.push(r(G::Grid, if ver.is_empty() { name.clone() } else { format!("{name}  {ver}") }, sub).value(if open { "" } else { "Remove…" }).on(M::Confirm(name.clone())).view(&p));
        if open {
            deb.push(r(G::Close, format!("Remove {name}"), "Its apps leave your menu; your files stay").danger().indent().on(M::Work(vec!["remove".into(), name.clone()], format!("{name} was removed."))).view(&p));
        }
    }
    deb.push(r(G::Folder, "Install a .deb file", "Open it in Files (double-click, or right-click › Install). Downloads opens now.").chev().on(M::Downloads).view(&p));
    let made = gb(b, "box");
    if made && st.confirm != "reset" {
        deb.push(r(G::Update, "Reset the Debian box…", "A fresh, empty box: everything installed in it is removed").on(M::Confirm("reset".into())).view(&p));
    }
    if st.confirm == "reset" {
        deb.push(r(G::Update, "Reset it now", "Removes every package in the box and makes a new one (1–2 minutes)").danger().on(M::Work(vec!["reset".into()], "The Debian box is fresh and empty.".into())).view(&p));
    }
    if made && st.confirm != "remove-box" {
        deb.push(r(G::Close, "Remove the Debian box…", "Frees its space; it's made again the next time you install a .deb").danger().on(M::Confirm("remove-box".into())).view(&p));
    }
    if st.confirm == "remove-box" {
        deb.push(r(G::Close, "Remove it now", "The box, everything in it and its apps in your menu").danger().on(M::Work(vec!["remove-box".into()], "The Debian box is removed.".into())).view(&p));
    }
    if st.confirm == "reset" || st.confirm == "remove-box" {
        deb.push(r(G::Close, "Cancel", "").on(M::Confirm(String::new())).view(&p));
    }

    let f = &st.flat;
    let mut fl: Vec<E<M>> = Vec::new();
    let bazaar = gb(f, "bazaar");
    let sub = if f.is_null() { "Looking…".to_string() } else if bazaar { "Hearth OS's app store: find, install and update apps from Flathub".into() } else { gs(f, "bazaarMissing") };
    fl.push(r(G::Store, "Bazaar", sub).value(if bazaar { "Open" } else { "" }).on(M::Bazaar).view(&p));
    let setup = go(f, "setup");
    if !f.is_null() && !bazaar && !setup.is_null() {
        let when = hearth_ui::time::when(gn(setup, "time") as i64);
        fl.push(r(G::Info, if gb(setup, "ok") { "Bazaar's install" } else { "Bazaar's install: not done yet" }, format!("{} ({when})", gs(setup, "message"))).view(&p));
    }
    if gb(f, "flatpak") && f.get("userWritable").is_some() && !gb(f, "userWritable") {
        fl.push(r(G::Info, "Fix your Flatpak folder", gs(f, "userProblem")).danger().on(M::Repair).view(&p));
    }
    // The Fedora edition: apps come from Flathub, so they're listed here with Remove (hearth-flatpak).
    match &st.flatpaks {
        _ if !gb(f, "flatpak") => {}
        None => {}
        Some(l) if l.is_empty() => fl.push(words(&p, "No apps from Flathub yet.", false)),
        Some(l) => {
            for a in l {
                let id = gs(a, "id");
                let name = if gs(a, "name").is_empty() { id.clone() } else { gs(a, "name") };
                let key = format!("fp:{id}");
                let sub = [gs(a, "version"), gs(a, "size"), if gs(a, "installation") == "system" { "for everyone".into() } else { "yours".into() }, gs(a, "origin")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
                fl.push(r(G::Store, name.clone(), sub).value(if st.confirm == key { "Remove it" } else { "Remove…" }).danger_if(st.confirm == key).on(M::RemoveFlatpak(id, name)).view(&p));
            }
        }
    }

    let ai = vec![r(
        G::Info,
        "AppImages run from Files",
        "Double-click one to run it (Hearth asks once, then makes it runnable). Right-click › Add to app menu moves it to the Applications folder in your home folder and puts it in your menu; Remove from app menu takes it out again.",
    )
    .view(&p)];
    page(vec![group(&p, "Debian packages (.deb)", deb), group(&p, "Flathub", fl), group(&p, "AppImages", ai)])
}
