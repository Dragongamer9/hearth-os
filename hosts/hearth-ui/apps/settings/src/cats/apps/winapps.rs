//! Apps › Windows apps (Mantel's SetWinApps.qml, docs/WINDOWS-APPS.md): the Windows programs opened with Hearth, each
//! in its own space, started with the GE-Proton built into Hearth OS unless the Proton detector or you pick another.
//! Open, the last start and its log, a detailed Proton log once, Proton, DXVK / VKD3D, the Windows version, files
//! from Windows (DLL overrides), the common fixes, In Steam, Remove. All through hearth-winapps.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub open: String,
    pub confirm_remove: String,
    pub pending: Option<(String, String, String, bool)>, // (app id, proton, app name, run after)
    pub dll_open: String,
    pub dll_text: String,
    pub say: Say,
    pub busy: Busy,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Reload,
    Toggle(String),
    Launch(String, String),
    Change(String, String, String, String),
    Changed(String, Res),
    Proton(String, String),
    TryNext(String),
    Download,
    NotNow,
    Fetched(Ev),
    Log(String),
    Detailed(String, bool),
    DllOpen(String),
    DllText(String),
    DllSave,
    Remove(String),
    RemoveYes(String, String),
    CancelRemove,
    OpenFiles,
}

fn hw(args: &[&str]) -> Vec<String> {
    py("hearth-winapps", args, 60)
}
fn reload() -> Task<M> {
    read(hw(&["list"]), M::Got)
}
fn app<'a>(st: &'a St, id: &str) -> Option<&'a Value> {
    ga(&st.st, "apps").iter().find(|a| gs(a, "id") == id)
}

pub fn open(sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    // `winapps:<app id>` opens that app's settings.
    let part = std::mem::take(&mut sh.part);
    if !part.is_empty() {
        st.open = part;
    }
    st.confirm_remove.clear();
    st.pending = None;
    reload()
}

fn launch(st: &mut St, id: &str, proton: &str) -> Task<M> {
    let name = app(st, id).map(|a| gs(a, "name")).unwrap_or_default();
    let mut cmd = vec!["python3".to_string(), shell("hearth-winapps"), "run".into(), id.to_string()];
    if !proton.is_empty() {
        cmd.push("--proton".into());
        cmd.push(proton.to_string());
    }
    let c: Vec<&str> = cmd.iter().map(String::as_str).collect();
    kit::open(&c);
    st.say.ok(format!("Starting {name}{}…", if proton.is_empty() { String::new() } else { format!(" with {proton}") }));
    Task::perform(hearth_ui::run::after(20000), |_| M::Reload)
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.st = r.v
            } else {
                st.st = serde_json::json!({"apps": []});
                st.say.bad(r.err)
            }
        }
        M::Reload => return reload(),
        M::Toggle(id) => st.open = if st.open == id { String::new() } else { id },
        M::Launch(id, proton) => return launch(st, &id, &proton),
        M::Change(id, key, value, said) => return read(hw(&["set", &id, &key, &value]), move |r| M::Changed(said.clone(), r)),
        M::Changed(said, r) => {
            if r.ok {
                if !said.is_empty() {
                    st.say.ok(said);
                }
                return reload();
            }
            st.say.bad(r.err);
        }
        M::Proton(id, name) => {
            let a = app(st, &id).cloned().unwrap_or(Value::Null);
            let needs = ga(&a, "candidates").iter().any(|c| gs(c, "name") == name && gb(c, "download"));
            if !name.is_empty() && needs {
                st.pending = Some((id, name, gs(&a, "name"), false));
                return Task::none();
            }
            let said = if name.is_empty() { format!("Hearth picks the Proton for {} again.", gs(&a, "name")) } else { format!("{} starts with {name} now.", gs(&a, "name")) };
            return update(_sh, st, M::Change(id, "proton".into(), name, said));
        }
        M::TryNext(id) => {
            let a = app(st, &id).cloned().unwrap_or(Value::Null);
            let next = go(go(&a, "last"), "next");
            if gb(next, "download") {
                st.pending = Some((id, gs(next, "name"), gs(&a, "name"), true));
            } else {
                return launch(st, &id, &gs(next, "name"));
            }
        }
        M::NotNow => st.pending = None,
        M::Download => {
            if let Some((_, name, _, _)) = &st.pending {
                st.busy.start();
                st.busy.line = format!("Downloading {name}");
                return job(py("hearth-winapps", &["fetch", name], 3600), None, M::Fetched);
            }
        }
        M::Fetched(e) => match e {
            Ev::Line(f, l) => st.busy.step(f, l),
            Ev::Done(r) => {
                st.busy.stop();
                let Some((id, name, _, run_after)) = st.pending.take() else { return Task::none() };
                if !r.ok {
                    st.say.bad(r.err);
                    return Task::none();
                }
                st.say.ok(format!("{name} is downloaded."));
                if app(st, &id).is_none() {
                    return reload();
                }
                if run_after {
                    return launch(st, &id, &name);
                }
                return update(_sh, st, M::Change(id, "proton".into(), name, String::new()));
            }
        },
        M::Log(path) => {
            if path.is_empty() {
                st.say.ok("There's no log yet: it's written the first time the app starts.");
            } else {
                kit::open(&["xdg-open", &path]);
            }
        }
        M::Detailed(id, on) => return read(hw(&["detailed", &id, if on { "on" } else { "off" }]), |r| M::Changed(String::new(), r)),
        M::DllOpen(id) => {
            st.dll_open = if st.dll_open == id { String::new() } else { id };
            st.dll_text.clear();
        }
        M::DllText(t) => st.dll_text = t,
        M::DllSave => {
            let id = std::mem::take(&mut st.dll_open);
            let t = std::mem::take(&mut st.dll_text);
            return update(_sh, st, M::Change(id, "dlls".into(), t, String::new()));
        }
        M::Remove(id) => st.confirm_remove = id,
        M::CancelRemove => st.confirm_remove.clear(),
        M::RemoveYes(id, name) => {
            st.confirm_remove.clear();
            return read(hw(&["remove", &id]), move |r| M::Changed(if r.ok { format!("{name} is removed.") } else { String::new() }, r));
        }
        M::OpenFiles => kit::open(&["hearth-filemanager", &format!("{}/Downloads", home())]),
    }
    Task::none()
}

fn summary(a: &Value) -> String {
    let l = go(a, "last");
    let pick = gs(a, "proton");
    if gb(l, "crashed") {
        let next = go(l, "next");
        return format!("Closed right away with {}{}", gs(l, "proton"), if gb(next, "name") { format!(". Try {} below.", gs(next, "name")) } else { ".".into() });
    }
    if gb(l, "proton") {
        return format!("Last started with {}{}", gs(l, "proton"), if pick.is_empty() { " (picked by Hearth)" } else { " (your pick)" });
    }
    if pick.is_empty() { "Hearth picks the Proton when it starts".into() } else { format!("Starts with {pick} (your pick)") }
}

fn took(sec: f64) -> String {
    if sec < 120.0 { format!("{sec:.1} s") } else { format!("{} min", (sec / 60.0).round()) }
}

fn last_result(a: &Value) -> String {
    let l = go(a, "last");
    let at = gn(l, "at") as i64;
    if at == 0 {
        return "Not started yet".into();
    }
    let what = gs(l, "what");
    let what = if what.is_empty() || what == "started" {
        "Started".to_string()
    } else {
        let mut c = what.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    };
    let code = gn(l, "code") as i64;
    format!("{what} {} with {}, closed after {}{}", hearth_ui::time::when(at), gs(l, "proton"), took(gn(l, "seconds")), if code == 0 { " (no error)".to_string() } else { format!(" with code {code}") })
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let mut v = crate::secs::SecVec::new();
    let builtin = gs(s, "builtin");
    let sub = if s.is_null() {
        "Checking…".to_string()
    } else if !builtin.is_empty() {
        format!("{builtin}: Windows programs start without the internet.{}", if gb(s, "valve") { format!(" Valve's {} from Steam is used when it works better.", gs(s, "valve")) } else { String::new() })
    } else {
        "This Hearth OS has none built in. Older GE-Protons can be downloaded when an app needs one.".into()
    };
    v.push(r(G::Display, "Proton built into Hearth OS", sub).chip(if s.is_null() { "" } else if builtin.is_empty() { "Missing" } else { "Built in" }, if builtin.is_empty() { Chip::Warn } else { Chip::Ok }).view(&p));
    v.push(r(G::Folder, "Open a Windows program", "Open a .exe or .msi in Files (or right-click it › Open with Hearth). An installer gets its own space and its app goes in your app menu.").chev().on(M::OpenFiles).view(&p));
    let apps = ga(s, "apps");
    if !apps.is_empty() {
        v.push(head(&p, "Your Windows apps"));
    } else if !s.is_null() {
        v.push(words(&p, "No Windows apps yet. They're kept in “Windows apps” in your Games folder.", false));
    }
    for a in apps {
        let id = gs(a, "id");
        let name = gs(a, "name");
        let last = go(a, "last");
        let crashed = gb(last, "crashed");
        let chip = if crashed { Some(("Closed right away".to_string(), Chip::Warn)) } else if gb(a, "steam") { Some(("In Steam".to_string(), Chip::Ok)) } else { None };
        let open = st.open == id;
        v.push(expander(&p, G::Display, name.clone(), summary(a), chip, String::new(), open, M::Toggle(id.clone())));
        if !open {
            continue;
        }
        let mut c: Vec<E<M>> = Vec::new();
        let args: Vec<String> = ga(a, "args").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        c.push(r(G::Play, "Open", if args.is_empty() { gs(a, "exe") } else { format!("{} {}", gs(a, "exe"), args.join(" ")) }).on(M::Launch(id.clone(), String::new())).view(&p));
        let log = gs(a, "log");
        c.push(r(G::Info, "Show log", format!("{}. {}", last_result(a), if log.is_empty() { "The log is written at its first start.".to_string() } else { format!("Everything Proton and the program said: {log}") })).chev().on(M::Log(log)).view(&p));
        let dn = gb(a, "detailedNext");
        let dl = gs(a, "detailedLog");
        let dsub = if dn {
            format!("On for the next start only (PROTON_LOG=1), then off again. It's written to {}/logs/steam-hearth-winapp-{id}.log and Send logs includes it.", gs(a, "dir"))
        } else if !dl.is_empty() {
            format!("Off. The last detailed log: {dl}")
        } else {
            "For one start, Proton writes everything it does (big, slower) to the app's logs folder, and Send logs includes it.".into()
        };
        c.push(sw(&p, G::Info, "Detailed Proton log for the next start", dsub, dn, Some(M::Detailed(id.clone(), !dn))));
        if !dl.is_empty() {
            c.push(r(G::Info, "Show the detailed Proton log", dl.clone()).chev().on(M::Log(dl)).view(&p));
        }
        let next = go(last, "next");
        if crashed && !next.is_null() {
            c.push(r(G::Update, format!("Try with {}", gs(next, "name")), if gb(next, "download") { "Downloads it first (about 500 MB), after you say yes" } else { "Hearth remembers it if it works" }).on(M::TryNext(id.clone())).view(&p));
        }
        c.push(head(&p, "Compatibility"));
        let cur = gs(a, "proton");
        let mut opts: Vec<(String, String)> = vec![(String::new(), "Automatic".into())];
        for cand in ga(a, "candidates").iter().take(4) {
            let n = gs(cand, "name");
            opts.push((n.clone(), format!("{n}{}", if gb(cand, "download") { " ↓" } else { "" })));
        }
        if !cur.is_empty() && !opts.iter().any(|o| o.0 == cur) {
            opts.push((cur.clone(), cur.clone()));
        }
        c.push(choice_wide(&p, G::Rocket, "Proton", "", opts.into_iter().map(|(v, l)| (l, v == cur, Some(M::Proton(id.clone(), v)))).collect()));
        let dx = gb(a, "dxvk");
        c.push(sw(&p, G::Display, "DXVK", "DirectX 9, 10 and 11 through Vulkan. Off: Wine's own, slower, for programs that draw wrongly.", dx, Some(M::Change(id.clone(), "dxvk".into(), if dx { "off" } else { "on" }.into(), String::new()))));
        let vk = gb(a, "vkd3d");
        c.push(sw(&p, G::Display, "VKD3D", "DirectX 12 through Vulkan. Off: the program falls back to DirectX 11 if it can.", vk, Some(M::Change(id.clone(), "vkd3d".into(), if vk { "off" } else { "on" }.into(), String::new()))));
        let wv = gs(a, "winver");
        let vers = [("", "Automatic"), ("win11", "11"), ("win10", "10"), ("win7", "7"), ("winxp", "XP")];
        c.push(choice(&p, G::Display, "Windows version", "", vers.iter().map(|(v, l)| (l.to_string(), *v == wv, Some(M::Change(id.clone(), "winver".into(), v.to_string(), String::new())))).collect()));
        let dlls: Vec<String> = ga(a, "dlls").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        c.push(r(G::List, "Use these files from Windows", if dlls.is_empty() { "None: Proton's own. For a mod's d3d9.dll, dinput8.dll, dxgi.dll…".to_string() } else { format!("{} · the app's own copy first", dlls.join(", ")) }).chev().on(M::DllOpen(id.clone())).view(&p));
        if st.dll_open == id {
            c.push(field_on(&p, "File names, like d3d9, dinput8 (empty: none), then Enter", &st.dll_text, M::DllText, M::DllSave));
        }
        let fixes: Vec<String> = ga(a, "fixes").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        if let Some(fx) = go(a, "fixNames").as_object() {
            for (k, label) in fx {
                let on = fixes.contains(k);
                c.push(sw(&p, G::Gear, label.as_str().unwrap_or(k).to_string(), "", on, Some(M::Change(id.clone(), format!("fix.{k}"), if on { "off" } else { "on" }.into(), String::new()))));
            }
        }
        let steam = gb(a, "steam");
        let said = if steam { format!("{name} comes out of Steam at Hearth Sync's next sync.") } else { format!("{name} goes into Steam at Hearth Sync's next sync.") };
        c.push(sw(&p, G::Pad, "In Steam", "Hearth Sync adds it to Steam's library (Game Mode too) the next time it syncs", steam, Some(M::Change(id.clone(), "steam".into(), if steam { "off" } else { "on" }.into(), said))));
        if st.confirm_remove != id {
            c.push(r(G::Close, "Remove…", "Deletes the app, its menu entry and everything installed in its space").danger().on(M::Remove(id.clone())).view(&p));
        } else {
            c.push(r(G::Close, format!("Yes, remove {name}"), gs(a, "dir")).danger().on(M::RemoveYes(id.clone(), name.clone())).view(&p));
            c.push(r(G::Close, "Cancel", "").on(M::CancelRemove).view(&p));
        }
        v.push(inside(c));
    }
    if let Some((_, name, appname, _)) = &st.pending {
        v.push(r(G::Update, format!("Download {name}?"), format!("About 500 MB from GE-Proton's releases on GitHub, checked against Hearth's checksum, for {appname}. It's kept for other apps too.")).on(M::Download).view(&p));
        v.push(r(G::Close, "Not now", "").on(M::NotNow).view(&p));
    }
    opt(&mut v, busy_view(&p, &st.busy, "Downloading…"));
    opt(&mut v, say_view(&p, &st.say));
    page(v)
}
