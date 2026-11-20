//! Hearth Files' extras, as Mantel's FilesExtras.qml and FilesMarks.qml have them (the work is hearth-files',
//! hearth_files_extras.py; this asks it and shows what it said):
//!   tags (Tags ›, the Tags list in the left menu, finding what carries one; user.xdg.tags, the same as Dolphin's),
//!   git status badges (on items in a git repository, and the branch in the status bar),
//!   Act as administrator (changes outside your home through pkexec hearth-files-admin, its own polkit action: polkit
//!     asks for an administrator's password; never the parts that start the PC; this window only, never saved, a red
//!     bar while on),
//!   Share on your network… (Properties › Sharing),
//! and your devices' shared folders (FileManager.qml's netOpen / netSignIn: one click opens one, the first time it
//! asks for the user name and sharing password; Remember keeps it in the system keyring, never in Hearth's files).

use crate::app::*;
use crate::backend::{self, Reply};
use crate::icons::Ic;
use crate::model::{parent_of, Entry};
use iced::{Color, Task};
use serde_json::{json, Map, Value};
use std::sync::atomic::Ordering;

fn sv(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

#[derive(Debug, Clone, Default)]
pub struct Fx {
    /// Act as administrator is on (this window only).
    pub admin: bool,
    /// git status of the folder shown: its repository, branch, how many to push, and each item's state.
    pub git_repo: bool,
    pub git_for: String,
    pub git_branch: String,
    pub git_ahead: u64,
    pub git: Map<String, Value>,
    /// Tags in your home folder: (name, how many carry it).
    pub tags: Vec<(String, u64)>,
    pub pending_tag: String,
}

/// What a text box of extras is for.
#[derive(Debug, Clone)]
pub enum AskFx {
    NewTag(Vec<String>),
    NetUser(Value),
    NetPass(Value, String),
    NetPick(Value, String),
}

#[derive(Debug, Clone)]
pub enum XMsg {
    Git(String, Reply),
    TagList(Reply),
    TagDone(String, Reply),
    AdminOn,
    AdminOff,
    Net(Value, String, String, bool),
    NetAnswer(Value, String, bool, Reply),
    NetWake(String, String),
    Auto(Value, bool),
    Sharing(String, Reply),
    Share(String),
}

// ------------------------------------------------------------------ Act as administrator
pub fn can_admin(a: &App) -> bool {
    a.listing.is_some() && !a.on_home() && !a.on_special() && !a.in_trash() && backend::system_path(&a.path)
}
pub fn admin_here(a: &App) -> bool {
    a.fx.admin && can_admin(a)
}

pub fn set_admin(a: &mut App, on: bool) -> Task<Msg> {
    if !on {
        a.fx.admin = false;
        backend::ADMIN.store(false, Ordering::SeqCst);
        let t = a.say("Not acting as administrator any more", false);
        return Task::batch(vec![t, a.reload()]);
    }
    a.confirm(
        "Act as administrator?",
        "You can change system files here, and system files can break apps or Hearth itself. Every change asks for an administrator's password (remembered for a few minutes). Hearth never lets Files change what starts the PC.",
        "Act as administrator",
        Yes::Then(Msg::Fx(XMsg::AdminOn)),
    );
    if let Some(Overlay::Confirm(c)) = &mut a.overlay {
        c.danger = true;
    }
    Task::none()
}

// ------------------------------------------------------------------ git
pub fn git_on(a: &App) -> bool {
    a.prefs.get("gitBadges") != Some(&Value::Bool(false))
}

/// An item's git state ("modified", "added", "untracked", "deleted", "conflict", "ignored", "clean") or "".
pub fn git_of<'a>(a: &'a App, e: &Entry) -> &'a str {
    if !a.fx.git_repo || !git_on(a) || e.path.is_empty() || parent_of(&e.path) != a.fx.git_for {
        return "";
    }
    a.fx.git.get(&e.name).and_then(Value::as_str).unwrap_or("")
}

pub fn git_mark(s: &str) -> (&'static str, &'static str) {
    match s {
        "modified" => ("M", "Changed"),
        "added" => ("+", "Added"),
        "untracked" => ("?", "Not in git yet"),
        "deleted" => ("−", "Deleted"),
        "conflict" => ("!", "In conflict"),
        "ignored" => ("·", "Ignored by git"),
        "clean" => ("✓", "Up to date"),
        _ => ("", ""),
    }
}

pub fn git_color(a: &App, s: &str) -> Color {
    let p = &a.p;
    match s {
        "conflict" => p.danger,
        "modified" | "deleted" => Color::from_rgb8(0xe0, 0xa2, 0x3a),
        "added" => Color::from_rgb8(0x4f, 0xbf, 0x7f),
        "untracked" => Color::from_rgb8(0x5a, 0xa7, 0xe6),
        "clean" => Color { r: 0.4, g: 0.75, b: 0.5, a: 0.85 },
        _ => p.muted,
    }
}

/// "⎇ main · 2 changes here · 1 to push", for the status bar.
pub fn git_line(a: &App) -> String {
    if !a.fx.git_repo || a.fx.git_for != a.path || !git_on(a) {
        return String::new();
    }
    let changed = a.fx.git.values().filter(|v| matches!(v.as_str().unwrap_or(""), "modified" | "added" | "deleted" | "conflict" | "untracked")).count();
    let b = if a.fx.git_branch.is_empty() { "git".to_string() } else { a.fx.git_branch.clone() };
    format!(
        "⎇ {b}{}{}",
        if changed > 0 { format!(" · {changed} {}", if changed == 1 { "change here" } else { "changes here" }) } else { " · up to date".into() },
        if a.fx.git_ahead > 0 { format!(" · {} to push", a.fx.git_ahead) } else { String::new() }
    )
}

// ------------------------------------------------------------------ tags
const TAG_COLORS: [(u8, u8, u8); 8] = [(0xe5, 0x48, 0x4d), (0xf5, 0xa5, 0x24), (0x46, 0xa7, 0x58), (0x3e, 0x9b, 0xdb), (0x8e, 0x4e, 0xc6), (0xd6, 0x40, 0x9f), (0x12, 0xa5, 0x94), (0x97, 0x83, 0x65)];

/// A tag's colour: always the same one for the same name (FilesExtras.qml's tagColor, so Mantel and here agree).
pub fn tag_color(t: &str) -> Color {
    let mut h: u32 = 0;
    for c in t.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(c as u32);
    }
    let c = TAG_COLORS[(h % TAG_COLORS.len() as u32) as usize];
    Color::from_rgb8(c.0, c.1, c.2)
}

pub fn refresh_tags() -> Task<Msg> {
    Task::perform(backend::call(backend::a(&["tag-list"]), 30), |r| Msg::Fx(XMsg::TagList(r)))
}

/// Opening a tag (the left menu): what carries it, found in your home folder (as a search).
pub fn open_tag(a: &mut App, t: &str) -> Task<Msg> {
    if a.listing.is_none() || a.on_special() || a.on_home() || !a.path.starts_with('/') {
        a.fx.pending_tag = t.to_string();
        let h = a.home.clone();
        return a.navigate(&h, "");
    }
    run_tag_search(a, t)
}

pub fn run_tag_search(a: &mut App, t: &str) -> Task<Msg> {
    a.found = Some(vec![]);
    a.found_query = format!("tag: {t}");
    a.query.clear();
    a.search_text.clear();
    a.marked.clear();
    a.cur = 0;
    a.zone = Zone::Main;
    a.seq += 1;
    a.search = Some(Search { seq: a.seq, cmd: backend::argv(&backend::a(&["tag-find", t])) });
    a.rebuild();
    Task::none()
}

/// Tags › for what's picked: each tag (✓ when they all have it), New tag…, Remove all tags.
pub fn tag_menu(a: &mut App, t: Vec<String>) {
    let ents: Vec<Entry> = t.iter().filter_map(|p| a.entries.iter().find(|e| e.path == *p).cloned()).collect();
    let all = |n: &str| !ents.is_empty() && ents.iter().all(|e| e.tags.iter().any(|x| x == n));
    let mut names: Vec<String> = a.fx.tags.iter().map(|g| g.0.clone()).collect();
    for e in &ents {
        for n in &e.tags {
            if !names.contains(n) {
                names.push(n.clone());
            }
        }
    }
    let mut items: Vec<MenuItem> = names.iter().take(16).map(|n| MenuItem::new(&format!("{}{n}", if all(n) { "x-untag:" } else { "x-tag:" }), n).check(all(n)).dot(tag_color(n))).collect();
    items.push(MenuItem::sep());
    items.push(MenuItem::new("x-newtag", "New tag…").icon(Ic::Plus));
    if ents.iter().any(|e| !e.tags.is_empty()) {
        items.push(MenuItem::new("x-cleartags", "Remove all tags").icon(Ic::Close));
    }
    a.menu_targets = t;
    a.show_menu("Tags", items, false);
}

pub fn set_tags(t: Vec<String>, add: Vec<String>, remove: Vec<String>) -> Task<Msg> {
    let mut args = vec!["tag-set".to_string()];
    for x in &add {
        args.push("--add".into());
        args.push(x.clone());
    }
    for x in &remove {
        args.push("--remove".into());
        args.push(x.clone());
    }
    args.extend(t);
    let msg = if add.is_empty() { "Tags removed".to_string() } else { format!("Tagged “{}”", add.join(", ")) };
    Task::perform(backend::call(args, 30), move |r| Msg::Fx(XMsg::TagDone(msg.clone(), r)))
}

/// The left menu's tags (after your folders): (name, count).
pub fn tag_places(a: &App) -> Vec<NavItem> {
    a.fx.tags
        .iter()
        .take(12)
        .map(|(n, c)| NavItem { id: format!("tag:{n}"), label: n.clone(), kind: "tag".into(), section: "folders".into(), sub: c.to_string(), raw: json!({ "tag": n }), ..Default::default() })
        .collect()
}

// ------------------------------------------------------------------ the menus
pub fn menu_items(a: &App, e: Option<&Entry>, n: usize) -> Vec<MenuItem> {
    let mut out = vec![];
    if a.picking() || a.in_trash() {
        return out;
    }
    if let Some(e) = e {
        if n > 0 && a.writable() && e.path.starts_with('/') {
            out.push(MenuItem::new("x-tags", "Tags  ›").icon(Ic::Pin));
        }
        if n == 1 && e.dir && e.path.starts_with('/') {
            out.push(MenuItem::new("x-share", "Share on your network…").icon(Ic::Share));
        }
    }
    out
}

pub fn folder_menu_items(a: &App) -> Vec<MenuItem> {
    let mut out = vec![];
    if can_admin(a) && !a.picking() {
        let mut it = MenuItem::new("x-admin", if a.fx.admin { "Stop acting as administrator" } else { "Act as administrator" }).icon(Ic::Shield);
        if !a.fx.admin {
            it = it.danger();
        }
        out.push(it);
    }
    if a.fx.git_repo && a.fx.git_for == a.path {
        out.push(MenuItem::new("x-gitbadges", "Show git status").check(git_on(a)));
    }
    out
}

/// What the extras' menu items do. T: what's picked; E: the item the menu is about.
pub fn run(a: &mut App, id: &str, t: Vec<String>, e: Option<Entry>) -> Task<Msg> {
    if id == "x-admin" {
        let on = !a.fx.admin;
        return set_admin(a, on);
    }
    if id == "x-tags" {
        tag_menu(a, t);
        return Task::none();
    }
    let t = if t.is_empty() { std::mem::take(&mut a.menu_targets) } else { t };
    if let Some(n) = id.strip_prefix("x-tag:") {
        return set_tags(t, vec![n.to_string()], vec![]);
    }
    if let Some(n) = id.strip_prefix("x-untag:") {
        return set_tags(t, vec![], vec![n.to_string()]);
    }
    match id {
        "x-cleartags" => {
            let mut names: Vec<String> = vec![];
            for p in &t {
                if let Some(en) = a.entries.iter().find(|q| q.path == *p) {
                    for n in &en.tags {
                        if !names.contains(n) {
                            names.push(n.clone());
                        }
                    }
                }
            }
            set_tags(t, vec![], names)
        }
        "x-newtag" => a.ask("New tag", "", "Tag", 0, Ask::Fx(AskFx::NewTag(t))),
        "x-share" => {
            if let Some(e) = e {
                let p = e.path.clone();
                let task = a.properties(vec![p.clone()]);
                if let Some(Overlay::Props(pr)) = &mut a.overlay {
                    pr.tab = 99;
                }
                return Task::batch(vec![task, load_sharing(p)]);
            }
            Task::none()
        }
        "x-gitbadges" => {
            let on = !git_on(a);
            a.prefs.insert("gitBadges".into(), json!(on));
            let t1 = Task::perform(backend::call(vec!["prefs-set".into(), json!({ "gitBadges": on }).to_string()], 15), |r| Msg::Did(Then::Nothing, r));
            let t2 = a.say(if on { "Showing git status on items" } else { "Git status is hidden" }, false);
            Task::batch(vec![t1, t2, a.reload()])
        }
        _ => Task::none(),
    }
}

pub fn ask_done(a: &mut App, ask: AskFx, text: String) -> Task<Msg> {
    match ask {
        AskFx::NewTag(t) => {
            let n = text.trim().to_string();
            if n.is_empty() {
                return Task::none();
            }
            set_tags(t, vec![n], vec![])
        }
        AskFx::NetUser(net) => {
            let u = text.trim().to_string();
            let who = net_name(&net);
            let t = a.ask(&format!("Sharing password for {u} on {who}"), "", "Sign in", 0, Ask::Fx(AskFx::NetPass(net, u)));
            if let Some(Overlay::Input(i)) = &mut a.overlay {
                i.secret = true;
            }
            t
        }
        AskFx::NetPass(net, u) => {
            a.confirm(
                "Remember this password?",
                "Kept in this device's keyring (the system's password store), never in Hearth's files. Then it's one click next time.",
                "Remember",
                Yes::Then(Msg::Fx(XMsg::Net(net.clone(), u.clone(), text.clone(), true))),
            );
            if let Some(Overlay::Confirm(c)) = &mut a.overlay {
                c.danger = false;
                c.alt = Some(("Just this time".into(), Yes::Then(Msg::Fx(XMsg::Net(net, u, text, false)))));
            }
            Task::none()
        }
        AskFx::NetPick(net, u) => {
            let mut n = net.clone();
            n["share"] = json!(text.trim());
            net_open(a, n, u, String::new(), false)
        }
    }
}

// ------------------------------------------------------------------ your devices' shared folders
fn net_name(net: &Value) -> String {
    let n = sv(net, "name");
    if n.is_empty() { "the device".into() } else { n }
}

/// One click: mounted with gvfs and opened (the home name at home, the Tailscale name away).
pub fn net_open(a: &mut App, net: Value, user: String, password: String, remember: bool) -> Task<Msg> {
    let what = { let s = sv(&net, "share"); if s.is_empty() { sv(&net, "name") } else { s } };
    let t = a.say(&format!("Opening “{what}”…"), false);
    let mut args = vec!["net-open".to_string(), net.to_string()];
    if !user.is_empty() {
        args.push("--user".into());
        args.push(user.clone());
    }
    if remember {
        args.push("--remember".into());
    }
    let had = !password.is_empty();
    let input = if had { json!({ "password": password }).to_string() } else { String::new() };
    Task::batch(vec![t, Task::perform(backend::call_in(args, 90, input), move |r| Msg::Fx(XMsg::NetAnswer(net.clone(), user.clone(), had, r)))])
}

fn net_answer(a: &mut App, net: Value, user: String, had: bool, r: Reply) -> Task<Msg> {
    if !r.ok() {
        return a.say(&r.err, true);
    }
    let v = &r.value;
    let what = { let s = sv(&net, "share"); if s.is_empty() { sv(&net, "name") } else { s } };
    if v.get("ok").and_then(Value::as_bool) == Some(true) && !sv(v, "path").is_empty() {
        let t = if v.get("remembered") == Some(&Value::Bool(false)) {
            a.say("Opened. The password couldn't be kept in the keyring, so it's asked again next time", true)
        } else {
            a.say(&format!("“{what}” is open{}", if sv(v, "where") == "away" { " (through Tailscale)" } else { "" }), false)
        };
        let p = sv(v, "path");
        return Task::batch(vec![t, a.refresh_places(), a.navigate(&p, "")]);
    }
    let need = sv(v, "need");
    if need == "password" {
        let u = { let x = sv(v, "user"); if x.is_empty() { user } else { x } };
        let why = if had { let e = sv(v, "error"); format!("{}. ", if e.is_empty() { "Wrong user name or password".to_string() } else { e }) } else { String::new() };
        let n = u.chars().count();
        return a.ask(&format!("{why}Sign in to {}: user name", net_name(&net)), &u, "Next", n, Ask::Fx(AskFx::NetUser(net)));
    }
    if need == "pick" {
        let shares: Vec<String> = v.get("shares").and_then(Value::as_array).map(|l| l.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        if let Some(first) = shares.first().cloned() {
            let u = { let x = sv(v, "user"); if x.is_empty() { user } else { x } };
            let n = first.chars().count();
            return a.ask(&format!("Which shared folder on {}? {}", if sv(&net, "name").is_empty() { "it".to_string() } else { sv(&net, "name") }, shares.join(", ")), &first, "Open", n, Ask::Fx(AskFx::NetPick(net, u)));
        }
    }
    let device = sv(&net, "device");
    if need == "asleep" && (v.get("wake").and_then(Value::as_bool) == Some(true) || net.get("wake").and_then(Value::as_bool) == Some(true)) && !device.is_empty() {
        let name = sv(&net, "name");
        let e = { let x = sv(v, "error"); if x.is_empty() { "It may be off or asleep".to_string() } else { x } };
        a.confirm(&format!("{} isn't answering", if name.is_empty() { "It".to_string() } else { name.clone() }), &format!("{e}. Hearth can wake it over the network (Wake-on-LAN), then try again."), "Wake it", Yes::Then(Msg::Fx(XMsg::NetWake(device, name))));
        if let Some(Overlay::Confirm(c)) = &mut a.overlay {
            c.danger = false;
        }
        return Task::none();
    }
    let err = { let x = sv(v, "error"); if x.is_empty() { "Couldn't open it".to_string() } else { x } };
    let detail = sv(v, "detail");
    let text = if !detail.is_empty() && detail != err { format!("{err}\n\n({detail})") } else { err };
    a.confirm(&format!("Couldn't open “{}”", if what.is_empty() { "it".to_string() } else { what }), &text, "OK", Yes::Note);
    if let Some(Overlay::Confirm(c)) = &mut a.overlay {
        c.danger = false;
    }
    Task::none()
}

// ------------------------------------------------------------------ Properties › Sharing
pub fn load_sharing(p: String) -> Task<Msg> {
    Task::perform(backend::call(backend::a(&["sharing", &p]), 60), move |r| Msg::Fx(XMsg::Sharing(p.clone(), r)))
}

// ------------------------------------------------------------------ answers
pub fn update(a: &mut App, m: XMsg) -> Task<Msg> {
    match m {
        XMsg::Git(at, r) => {
            if a.path == at && r.ok() && !r.get("repo").is_null() {
                a.fx.git_repo = true;
                a.fx.git_for = at;
                a.fx.git_branch = sv(&r.value, "branch");
                a.fx.git_ahead = r.get("ahead").as_u64().unwrap_or(0);
                a.fx.git = r.get("status").as_object().cloned().unwrap_or_default();
            } else if a.path == at {
                a.fx.git_repo = false;
                a.fx.git.clear();
            }
        }
        XMsg::TagList(r) => {
            if let Some(l) = r.get("tags").as_array() {
                a.fx.tags = l.iter().map(|g| (sv(g, "name"), g.get("count").and_then(Value::as_u64).unwrap_or(0))).filter(|g| !g.0.is_empty()).collect();
                a.places = a.build_nav();
            }
        }
        XMsg::TagDone(msg, r) => {
            if !r.ok() {
                return a.say(&r.err, true);
            }
            let t = a.say(&msg, false);
            return Task::batch(vec![t, a.reload(), refresh_tags()]);
        }
        XMsg::AdminOn => {
            a.fx.admin = true;
            backend::ADMIN.store(true, Ordering::SeqCst);
            let t = a.say("Acting as administrator in system folders", false);
            return Task::batch(vec![t, a.reload()]);
        }
        XMsg::AdminOff => return set_admin(a, false),
        XMsg::Net(net, u, pw, remember) => return net_open(a, net, u, pw, remember),
        XMsg::NetAnswer(net, u, had, r) => return net_answer(a, net, u, had, r),
        XMsg::NetWake(device, name) => {
            let n = if name.is_empty() { "it".to_string() } else { name };
            return Task::perform(backend::call(backend::a(&["net-wake", &device]), 30), move |r| {
                Msg::Did(if r.ok() { Then::Say(format!("Waking {n}… try again in a minute")) } else { Then::Nothing }, r)
            });
        }
        XMsg::Auto(net, on) => {
            let t = Task::perform(backend::call(vec!["net-auto".into(), net.to_string(), if on { "on" } else { "off" }.into()], 20), move |r| {
                Msg::Did(Then::PlacesToo(if on { "It connects by itself from now on".into() } else { "It no longer connects by itself".into() }), r)
            });
            return t;
        }
        XMsg::Sharing(p, r) => {
            if let Some(Overlay::Props(pr)) = &mut a.overlay {
                if pr.paths.first() == Some(&p) {
                    pr.share_busy = false;
                    pr.share = if r.ok() { r.value.clone() } else { json!({ "error": r.err }) };
                }
            }
        }
        XMsg::Share(what) => {
            if let Some(Overlay::Props(pr)) = &mut a.overlay {
                if pr.share_busy || pr.paths.len() != 1 {
                    return Task::none();
                }
                pr.share_busy = true;
                let p = pr.paths[0].clone();
                return Task::perform(backend::call(backend::a(&["share", &p, &what]), 120), move |r| Msg::Fx(XMsg::Sharing(p.clone(), r)));
            }
        }
    }
    Task::none()
}

/// The folder shown changed: its git status, and a tag search waiting for it.
pub fn folder_loaded(a: &mut App) -> Task<Msg> {
    let mut tasks = vec![];
    let p = a.path.clone();
    if a.fx.git_for != p {
        a.fx.git_repo = false;
        a.fx.git.clear();
    }
    if git_on(a) && p.starts_with('/') && !a.in_trash() && a.listing.as_ref().map(|l| l.error.is_empty()).unwrap_or(false) {
        let at = p.clone();
        tasks.push(Task::perform(backend::call(backend::a(&["git-status", &p]), 15), move |r| Msg::Fx(XMsg::Git(at.clone(), r))));
    }
    if !a.fx.pending_tag.is_empty() {
        let t = std::mem::take(&mut a.fx.pending_tag);
        tasks.push(run_tag_search(a, &t));
    }
    Task::batch(tasks)
}
