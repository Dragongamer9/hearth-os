//! Paths in drive letters, the Windows way, and Hearth (C:) with Windows folder names: Mantel's drive-paths.js
//! (hosts/linux/desktop/shell, checked there by test_hearth_this_pc.py and test_hearth_cdrive.py), ported line for
//! line so both editions name every place the same. Pure functions; nothing on disk moves.
//!
//! C: is always the system: C:\ is /, C:\Users\<you> is your home folder. Other letters are their drive's mount place.
//! With the view on (files.json "cdrive", on unless turned off) Hearth (C:) has Windows' folders: Program Files (every
//! installed app, one folder each), Program Files (x86) (Windows apps and Steam), ProgramData (/var/lib, look only),
//! Users, Windows (/usr, look only) and AppData (Roaming = ~/.config, Local = ~/.local/share, Local\Temp = ~/.cache).

use serde_json::Value;

pub const C_ROOT: &str = "hearth:c";

#[derive(Debug, Clone, Default)]
pub struct Drive {
    pub letter: String,
    pub path: String,
    pub label: String,
}

#[derive(Debug, Clone, Default)]
pub struct Folder {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct CApp {
    pub name: String,
    pub path: String,
    pub x86: bool,
    pub no_map: bool,
    /// The Installed apps entry it came from (Flatpak, Windows app, Recomp); none for Hearth, Hearth Games, Steam.
    pub app: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub home: String,
    pub user: String,
    pub drives: Vec<Drive>,
    pub folders: Vec<Folder>,
    pub c_on: bool,
    pub users: String,
    pub apps: Vec<CApp>,
}

/// What a typed or C:\ path stands for: a real path (or one of the view's pages), or why it can't be.
#[derive(Debug, Clone, PartialEq)]
pub enum Res {
    Path(String),
    Error(String),
}

impl Res {
    pub fn path(&self) -> Option<&str> {
        match self {
            Res::Path(p) => Some(p),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Crumb {
    pub label: String,
    pub path: String,
}

fn trim_end_slash(s: &str) -> &str {
    let t = s.trim_end_matches('/');
    t
}

pub fn users_dir(ctx: &Ctx) -> String {
    if !ctx.users.is_empty() {
        return trim_end_slash(&ctx.users).to_string();
    }
    let h = if ctx.home.is_empty() { "/home/you" } else { &ctx.home };
    let h = trim_end_slash(h);
    match h.rfind('/') {
        Some(0) | None => "/".into(),
        Some(i) => h[..i].to_string(),
    }
}

pub fn join_path(root: &str, segs: &[String]) -> String {
    let r = trim_end_slash(root);
    let rest: Vec<&str> = segs.iter().map(|s| s.as_str()).filter(|s| !s.is_empty()).collect();
    let rest = rest.join("/");
    if rest.is_empty() {
        if r.is_empty() { "/".into() } else { r.to_string() }
    } else if r.is_empty() {
        format!("/{rest}")
    } else {
        format!("{r}/{rest}")
    }
}

pub fn is_c_path(p: &str) -> bool {
    p.starts_with(C_ROOT)
}

fn c_segs(p: &str) -> Vec<String> {
    p[C_ROOT.len()..].split('/').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
}

/// The view's pages: "root" (C:\), "pf" (Program Files), "x86" (Program Files (x86)), "appdata" (Users\<you>\AppData);
/// "" for anything else.
pub fn c_page(p: &str, ctx: &Ctx) -> &'static str {
    if !is_c_path(p) {
        return "";
    }
    let segs: Vec<String> = c_segs(p).iter().map(|s| s.to_lowercase()).collect();
    if segs.is_empty() {
        return "root";
    }
    if segs.len() == 1 && segs[0] == "program files" {
        return "pf";
    }
    if segs.len() == 1 && segs[0] == "program files (x86)" {
        return "x86";
    }
    if segs.len() == 3 && segs[0] == "users" && segs[2] == "appdata" && segs[1] == ctx.user.to_lowercase() {
        return "appdata";
    }
    ""
}

/// What a C:\ form path (as segments after "C:") stands for with the view on.
pub fn c_segs_to_path(segs: &[String], ctx: &Ctx) -> Res {
    if segs.is_empty() {
        return Res::Path(C_ROOT.into());
    }
    let low: Vec<String> = segs.iter().map(|s| s.to_lowercase()).collect();
    let home = trim_end_slash(&ctx.home).to_string();
    if low[0] == "program files" || low[0] == "program files (x86)" {
        let x86 = low[0] == "program files (x86)";
        let page = format!("{C_ROOT}/{}", if x86 { "Program Files (x86)" } else { "Program Files" });
        if segs.len() == 1 {
            return Res::Path(page);
        }
        match ctx.apps.iter().find(|a| !a.path.is_empty() && a.x86 == x86 && a.name.to_lowercase() == low[1]) {
            Some(a) => return Res::Path(join_path(&a.path, &segs[2..])),
            None => {
                return Res::Error(format!("There's no “{}” in {}", segs[1], if x86 { "Program Files (x86)" } else { "Program Files" }));
            }
        }
    }
    if low[0] == "programdata" {
        return Res::Path(join_path("/var/lib", &segs[1..]));
    }
    if low[0] == "windows" {
        return Res::Path(join_path("/usr", &segs[1..]));
    }
    if low[0] == "users" {
        if segs.len() == 1 {
            return Res::Path(users_dir(ctx));
        }
        if !home.is_empty() && low[1] == ctx.user.to_lowercase() {
            let r = &segs[2..];
            let rl = &low[2..];
            if rl.first().map(|s| s.as_str()) == Some("appdata") {
                if r.len() == 1 {
                    return Res::Path(format!("{C_ROOT}/Users/{}/AppData", ctx.user));
                }
                let r1 = rl.get(1).map(|s| s.as_str()).unwrap_or("");
                if r1 == "roaming" {
                    return Res::Path(join_path(&format!("{home}/.config"), &r[2..]));
                }
                if r1 == "local" && rl.get(2).map(|s| s.as_str()) == Some("temp") {
                    return Res::Path(join_path(&format!("{home}/.cache"), &r[3..]));
                }
                if r1 == "local" {
                    return Res::Path(join_path(&format!("{home}/.local/share"), &r[2..]));
                }
                return Res::Error("AppData has Roaming and Local (Local\\Temp too)".into());
            }
            return Res::Path(join_path(&home, r));
        }
        return Res::Path(join_path(&users_dir(ctx), &segs[1..]));
    }
    Res::Path(format!("/{}", segs.join("/")))
}

/// The real place a hearth:c path stands for (a view page answers with itself), or why not.
pub fn c_resolve(p: &str, ctx: &Ctx) -> Res {
    if !is_c_path(p) {
        return Res::Path(p.to_string());
    }
    c_segs_to_path(&c_segs(p), ctx)
}

/// The real places the view maps, longest first: (path, C:\ name).
pub fn c_map(ctx: &Ctx) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for a in &ctx.apps {
        if a.path.starts_with('/') && !a.name.is_empty() && !a.no_map {
            out.push((trim_end_slash(&a.path).to_string(), format!("C:\\{}\\{}", if a.x86 { "Program Files (x86)" } else { "Program Files" }, a.name)));
        }
    }
    let home = trim_end_slash(&ctx.home).to_string();
    if !home.is_empty() {
        let u = if ctx.user.is_empty() { "you" } else { &ctx.user };
        out.push((format!("{home}/.config"), format!("C:\\Users\\{u}\\AppData\\Roaming")));
        out.push((format!("{home}/.local/share"), format!("C:\\Users\\{u}\\AppData\\Local")));
        out.push((format!("{home}/.cache"), format!("C:\\Users\\{u}\\AppData\\Local\\Temp")));
        out.push((home.clone(), format!("C:\\Users\\{u}")));
    }
    out.push((users_dir(ctx), "C:\\Users".into()));
    out.push(("/var/lib".into(), "C:\\ProgramData".into()));
    out.push(("/usr".into(), "C:\\Windows".into()));
    out.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    out
}

fn bs(s: &str) -> String {
    s.split('/').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("\\")
}

fn c_windows(p: &str, ctx: &Ctx) -> String {
    for (path, win) in c_map(ctx) {
        if p == path || p.starts_with(&format!("{path}/")) {
            let r = bs(&p[path.len()..]);
            return if r.is_empty() { win } else { format!("{win}\\{r}") };
        }
    }
    format!("C:\\{}", bs(p))
}

pub fn under(p: &str, root: &str) -> bool {
    if p.is_empty() || root.is_empty() {
        return false;
    }
    let r = trim_end_slash(root);
    p == root || p == r || p.starts_with(&format!("{r}/")) || r.is_empty()
}

fn rest(p: &str, root: &str) -> String {
    let r = trim_end_slash(root);
    if p.len() > r.len() { p[(r.len() + 1).min(p.len())..].to_string() } else { String::new() }
}

/// The lettered drive a path is on (not C:), longest mount place first.
pub fn drive_for<'a>(p: &str, ctx: &'a Ctx) -> Option<&'a Drive> {
    let mut best: Option<&Drive> = None;
    for d in &ctx.drives {
        if d.letter.is_empty() || d.path.is_empty() || d.letter == "C" || d.path == "/" {
            continue;
        }
        if under(p, &d.path) && best.map(|b| d.path.len() > b.path.len()).unwrap_or(true) {
            best = Some(d);
        }
    }
    best
}

fn folder_for<'a>(p: &str, ctx: &'a Ctx) -> Option<&'a Folder> {
    let mut best: Option<&Folder> = None;
    for f in &ctx.folders {
        if !f.path.is_empty() && under(p, &f.path) && best.map(|b| f.path.len() > b.path.len()).unwrap_or(true) {
            best = Some(f);
        }
    }
    best
}

fn c_map_for(p: &str, ctx: &Ctx) -> String {
    for (path, _) in c_map(ctx) {
        if p == path || p.starts_with(&format!("{path}/")) {
            return path;
        }
    }
    String::new()
}

fn c_label(ctx: &Ctx) -> String {
    ctx.drives.iter().find(|d| d.letter == "C").map(|d| d.label.clone()).unwrap_or_else(|| "Hearth (C:)".into())
}

/// The steps of a C:\ form: each step is where typing it would go.
pub fn c_crumbs(win: &str, ctx: &Ctx, this_pc: &str) -> Vec<Crumb> {
    let mut out = vec![Crumb { label: "This PC".into(), path: this_pc.into() }, Crumb { label: c_label(ctx), path: C_ROOT.into() }];
    let w = win.strip_prefix("C:").or_else(|| win.strip_prefix("c:")).unwrap_or(win);
    let segs: Vec<String> = w.split('\\').filter(|x| !x.is_empty()).map(|s| s.to_string()).collect();
    for i in 0..segs.len() {
        let r = c_segs_to_path(&segs[..=i], ctx);
        out.push(Crumb { label: segs[i].clone(), path: r.path().map(|s| s.to_string()).unwrap_or_else(|| C_ROOT.into()) });
    }
    out
}

/// The address bar's steps for a path: This PC › Games (D:) › SteamLibrary, This PC › Documents › …, or
/// This PC › Hearth (C:) › usr › … for the rest of the system. None where Files' own steps fit better.
pub fn crumbs_for(p: &str, ctx: &Ctx, this_pc: &str) -> Option<Vec<Crumb>> {
    if is_c_path(p) {
        return Some(c_crumbs(&to_windows(p, ctx), ctx, this_pc));
    }
    if !p.starts_with('/') {
        return None;
    }
    let parts = |root: &str, label: &str| -> Vec<Crumb> {
        let mut out = vec![Crumb { label: "This PC".into(), path: this_pc.into() }, Crumb { label: label.into(), path: root.into() }];
        let mut at = trim_end_slash(root).to_string();
        for s in rest(p, root).split('/') {
            if s.is_empty() {
                continue;
            }
            at = format!("{at}/{s}");
            out.push(Crumb { label: s.into(), path: at.clone() });
        }
        out
    };
    if let Some(d) = drive_for(p, ctx) {
        let l = if d.label.is_empty() { format!("{}:", d.letter) } else { d.label.clone() };
        return Some(parts(&d.path, &l));
    }
    if let Some(f) = folder_for(p, ctx) {
        return Some(parts(&f.path, &f.label));
    }
    if ctx.c_on {
        if !ctx.home.is_empty() && under(p, &ctx.home) {
            let m = c_map_for(p, ctx);
            if m.is_empty() || m.len() <= trim_end_slash(&ctx.home).len() {
                return None;
            }
        }
        return Some(c_crumbs(&c_windows(p, ctx), ctx, this_pc));
    }
    if !ctx.home.is_empty() && under(p, &ctx.home) {
        return None;
    }
    Some(parts("/", &c_label(ctx)))
}

/// D:\SteamLibrary\steamapps for a path on a lettered drive; C:\Users\<you>\… in the home folder; C:\usr\… elsewhere.
pub fn to_windows(p: &str, ctx: &Ctx) -> String {
    if is_c_path(p) {
        return format!("C:\\{}", c_segs(p).join("\\"));
    }
    if !p.starts_with('/') {
        return String::new();
    }
    if let Some(d) = drive_for(p, ctx) {
        return format!("{}:\\{}", d.letter, bs(&rest(p, &d.path)));
    }
    if ctx.c_on {
        return c_windows(p, ctx);
    }
    if !ctx.home.is_empty() && under(p, &ctx.home) {
        let r = bs(&rest(p, &ctx.home));
        let u = if ctx.user.is_empty() { "you" } else { &ctx.user };
        return if r.is_empty() { format!("C:\\Users\\{u}") } else { format!("C:\\Users\\{u}\\{r}") };
    }
    format!("C:\\{}", bs(p))
}

/// Typed in the address bar: "D:\Games", "d:/Games", "D:" → the Linux path, an error for a letter no drive has, or
/// None when it isn't a drive-letter path at all.
pub fn from_windows(text: &str, ctx: &Ctx) -> Option<Res> {
    let t = text.trim();
    let mut ch = t.chars();
    let l = ch.next()?;
    if !l.is_ascii_alphabetic() || ch.next() != Some(':') {
        return None;
    }
    let tail = &t[2..];
    if !tail.is_empty() && !tail.starts_with('\\') && !tail.starts_with('/') {
        return None;
    }
    let letter = l.to_ascii_uppercase().to_string();
    let mut segs: Vec<String> = Vec::new();
    for s in tail.split(|c| c == '\\' || c == '/') {
        if s.is_empty() || s == "." {
            continue;
        }
        if s == ".." {
            segs.pop();
            continue;
        }
        segs.push(s.to_string());
    }
    if letter == "C" && ctx.c_on {
        return Some(c_segs_to_path(&segs, ctx));
    }
    if letter == "C" {
        if segs.len() >= 2 && segs[0].to_lowercase() == "users" && !ctx.home.is_empty() && segs[1].to_lowercase() == ctx.user.to_lowercase() {
            let mut v = vec![trim_end_slash(&ctx.home).to_string()];
            v.extend(segs[2..].iter().cloned());
            return Some(Res::Path(v.join("/")));
        }
        return Some(Res::Path(format!("/{}", segs.join("/"))));
    }
    match ctx.drives.iter().find(|x| x.letter == letter && !x.path.is_empty()) {
        Some(d) => {
            let mut v = vec![trim_end_slash(&d.path).to_string()];
            v.extend(segs);
            Some(Res::Path(v.join("/")))
        }
        None => Some(Res::Error(format!("There's no drive {letter}: on this PC"))),
    }
}

/// A typed letter no drive has (from_windows answers None for it above): the message, for the address bar.
pub fn no_such_drive(text: &str, ctx: &Ctx) -> Option<String> {
    let t = text.trim();
    let mut ch = t.chars();
    let l = ch.next()?;
    if !l.is_ascii_alphabetic() || ch.next() != Some(':') {
        return None;
    }
    let tail = &t[2..];
    if !tail.is_empty() && !tail.starts_with('\\') && !tail.starts_with('/') {
        return None;
    }
    let letter = l.to_ascii_uppercase().to_string();
    if letter == "C" || ctx.drives.iter().any(|x| x.letter == letter && !x.path.is_empty()) {
        return None;
    }
    Some(format!("There's no drive {letter}: on this PC"))
}

// ---- The view's pages (FilesCDrive.qml draws them)
fn tilde(p: &str, ctx: &Ctx) -> String {
    let h = trim_end_slash(&ctx.home);
    if !h.is_empty() && (p == h || p.starts_with(&format!("{h}/"))) {
        format!("~{}", &p[h.len()..])
    } else {
        p.to_string()
    }
}

fn c_name(n: &str) -> String {
    let s = n.replace(['\\', '/'], "-");
    let s = s.trim();
    if s.is_empty() { "?".into() } else { s.to_string() }
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn parent_dir(p: &str) -> String {
    let s = trim_end_slash(p);
    match s.rfind('/') {
        Some(0) | None => "/".into(),
        Some(i) => s[..i].to_string(),
    }
}

fn common_dir(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let mut parts: Vec<&str> = paths[0].split('/').collect();
    for p in &paths[1..] {
        let q: Vec<&str> = p.split('/').collect();
        let mut i = 0;
        while i < parts.len() && i < q.len() && parts[i] == q[i] {
            i += 1;
        }
        parts.truncate(i);
    }
    let j = parts.join("/");
    if j.is_empty() { "/".into() } else { j }
}

/// Hearth Games' stores (GOG, Epic…): one folder, at the folder their games are in.
pub struct HearthGames {
    pub path: String,
    pub dirs: Vec<String>,
    pub stores: Vec<String>,
    pub size: u64,
}

pub fn hearth_games(apps: &[Value]) -> Option<HearthGames> {
    let gs: Vec<&Value> = apps.iter().filter(|a| s(a, "kind") == "games" && s(a, "path").starts_with('/')).collect();
    if gs.is_empty() {
        return None;
    }
    let mut dirs: Vec<String> = Vec::new();
    for a in &gs {
        let d = parent_dir(&s(a, "path"));
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    let common = if dirs.len() == 1 { dirs[0].clone() } else { common_dir(&dirs) };
    let mut stores: Vec<String> = Vec::new();
    for a in &gs {
        let p = s(a, "publisher");
        if !p.is_empty() && !stores.contains(&p) {
            stores.push(p);
        }
    }
    let size = gs.iter().map(|a| a.get("size").and_then(Value::as_u64).unwrap_or(0)).sum();
    Some(HearthGames { path: if common == "/" { dirs[0].clone() } else { common }, dirs, stores, size })
}

/// What the view's pages need: Installed apps' list and hearth-files cdrive.
#[derive(Debug, Clone, Default)]
pub struct CData {
    pub apps: Vec<Value>,
    pub hearth: String,
    pub steam: String,
    pub linux: Vec<String>,
    pub flatpak: Vec<String>,
    pub show_linux: bool,
}

/// Names Windows-style: each app once; a second app with the same name gets its id after it.
pub fn c_app_list(data: &CData) -> Vec<CApp> {
    let mut out: Vec<CApp> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut add = |name: &str, a: Option<&Value>, x86: bool, path: String, out: &mut Vec<CApp>, no_map: bool| {
        let mut n = c_name(name);
        let key = format!("{}{}", if x86 { "x:" } else { "p:" }, n.to_lowercase());
        if seen.contains(&key) {
            let id = a.map(|a| {
                let x = s(a, "appId");
                if x.is_empty() { s(a, "id") } else { x }
            }).unwrap_or_default();
            n = format!("{n} ({})", c_name(if id.is_empty() { "2" } else { &id }));
        }
        seen.insert(format!("{}{}", if x86 { "x:" } else { "p:" }, n.to_lowercase()));
        out.push(CApp { name: n, path, x86, no_map, app: a.cloned() });
    };
    let mut sorted: Vec<&Value> = data.apps.iter().collect();
    sorted.sort_by(|a, b| s(a, "name").to_lowercase().cmp(&s(b, "name").to_lowercase()));
    for a in sorted {
        let k = s(a, "kind");
        let path = s(a, "path");
        let dir = s(a, "dir");
        if (k == "flatpak" || k == "recomp") && !path.is_empty() {
            add(&s(a, "name"), Some(a), false, path, &mut out, false);
        } else if k == "winapp" && (!dir.is_empty() || !path.is_empty()) {
            add(&s(a, "name"), Some(a), true, if dir.is_empty() { path } else { dir }, &mut out, false);
        }
    }
    if let Some(g) = hearth_games(&data.apps) {
        let nm = g.dirs.len() > 1;
        add("Hearth Games", None, false, g.path, &mut out, nm);
    }
    if !data.hearth.is_empty() {
        add("Hearth", None, false, data.hearth.clone(), &mut out, false);
    }
    if !data.steam.is_empty() {
        add("Steam", None, true, data.steam.clone(), &mut out, false);
    }
    out
}

/// One of the view's folders: what its row and its Properties show.
#[derive(Debug, Clone, Default)]
pub struct CView {
    pub kind_text: String,
    pub size: u64,
    pub linux: String,
    pub win: String,
    pub real: Vec<String>,
    pub note: String,
    pub lock: bool,
    pub look: bool,
    pub dim: bool,
    pub group: String,
    pub app: Option<Value>,
    pub kind: String,
    pub view: bool,
}

pub struct CEntry {
    pub name: String,
    pub path: String,
    pub cv: CView,
}

fn and_list(xs: &[String]) -> String {
    match xs.len() {
        0 => String::new(),
        1 => xs[0].clone(),
        n => format!("{} and {}", xs[..n - 1].join(", "), xs[n - 1]),
    }
}

pub fn c_entries(page: &str, ctx: &Ctx, data: &CData) -> Vec<CEntry> {
    let home = trim_end_slash(&ctx.home).to_string();
    let user = if ctx.user.is_empty() { "you".to_string() } else { ctx.user.clone() };
    let users = users_dir(ctx);
    let t = |p: &str| tilde(p, ctx);
    let mut out: Vec<CEntry> = Vec::new();
    let mut add = |name: &str, path: String, cv: CView| out.push(CEntry { name: name.into(), path, cv });
    let apps = &data.apps;
    let flat_dirs: Vec<String> = if data.flatpak.is_empty() { vec!["/var/lib/flatpak/app".into()] } else { data.flatpak.clone() };
    let mut win_dirs: Vec<String> = Vec::new();
    for a in apps {
        if s(a, "kind") == "winapp" {
            let d0 = if s(a, "dir").is_empty() { s(a, "path") } else { s(a, "dir") };
            if !d0.is_empty() {
                let d = parent_dir(&d0);
                if !win_dirs.contains(&d) {
                    win_dirs.push(d);
                }
            }
        }
    }
    if win_dirs.is_empty() && !home.is_empty() {
        win_dirs.push(format!("{home}/Games/Windows apps"));
    }
    let mut recomp_dirs: Vec<String> = Vec::new();
    for a in apps.iter().filter(|a| s(a, "kind") == "recomp" && !s(a, "path").is_empty()) {
        let d = parent_dir(&s(a, "path"));
        if !recomp_dirs.contains(&d) {
            recomp_dirs.push(d);
        }
    }
    let hg = hearth_games(apps);
    let steam_size: u64 = apps
        .iter()
        .filter(|a| s(a, "kind") == "steam" && (s(a, "drive").is_empty() || s(a, "drive") == "/"))
        .map(|a| a.get("size").and_then(Value::as_u64).unwrap_or(0))
        .sum();
    if page == "root" {
        let mut game_dirs: Vec<String> = hg.as_ref().map(|g| g.dirs.clone()).unwrap_or_default();
        game_dirs.extend(recomp_dirs.iter().cloned());
        let mut real = Vec::new();
        if !data.hearth.is_empty() {
            real.push(format!("Hearth apps: {}", t(&data.hearth)));
        }
        real.push(format!("Flatpaks: {}", flat_dirs.iter().map(|d| t(d)).collect::<Vec<_>>().join(", ")));
        if !game_dirs.is_empty() {
            real.push(format!("Games: {}", game_dirs.iter().map(|d| t(d)).collect::<Vec<_>>().join(", ")));
        }
        add("Program Files", format!("{C_ROOT}/Program Files"), CView {
            kind_text: "Folder · your apps".into(), linux: "several places (see Details)".into(), view: true,
            win: "C:\\Program Files".into(), note: "Every app you have installed, one folder each, wherever it really lives. Opening one goes to its real folder.".into(),
            real, ..Default::default()
        });
        let mut real86: Vec<String> = win_dirs.iter().map(|d| t(d)).collect();
        if !data.steam.is_empty() {
            real86.push(t(&data.steam));
        }
        add("Program Files (x86)", format!("{C_ROOT}/Program Files (x86)"), CView {
            kind_text: "Folder · Windows apps".into(), view: true,
            linux: format!("{}{}", win_dirs.iter().map(|d| t(d)).collect::<Vec<_>>().join(", "), if data.steam.is_empty() { "" } else { " + Steam" }),
            win: "C:\\Program Files (x86)".into(), note: "Your Windows apps and games, and Steam, like on a Windows PC.".into(), real: real86, ..Default::default()
        });
        add("ProgramData", "/var/lib".into(), CView {
            kind_text: "Folder · look only".into(), linux: "/var/lib".into(), win: "C:\\ProgramData".into(), real: vec!["/var/lib".into()], lock: true, look: true,
            note: "What apps keep for everyone on this PC. You can look, but Hearth keeps changes out.".into(), ..Default::default()
        });
        let tu = t(&users);
        add("Users", users.clone(), CView {
            kind_text: "Folder".into(), linux: tu, win: "C:\\Users".into(), real: vec![users.clone()],
            note: format!("Everyone’s own folder. Yours is C:\\Users\\{user}."), ..Default::default()
        });
        add("Windows", "/usr".into(), CView {
            kind_text: "Hearth itself · look only".into(), linux: "/usr".into(), win: "C:\\Windows".into(), real: vec!["/usr".into()], lock: true, look: true,
            note: "Hearth’s own system files. Each update replaces them all at once, so this folder is look only, exactly like now.".into(), ..Default::default()
        });
        if data.show_linux {
            let mut names = data.linux.clone();
            names.sort();
            let mut first = true;
            for n in names {
                let kt = match n.as_str() {
                    "etc" => "Linux folder · settings",
                    "usr" => "Linux folder · Hearth itself",
                    _ => "Linux folder",
                };
                add(&n, format!("/{n}"), CView {
                    kind_text: kt.into(), linux: format!("/{n}"), win: format!("C:\\{n}"), real: vec![format!("/{n}")], dim: true,
                    group: if first { "THE LINUX FOLDERS".into() } else { String::new() },
                    note: "A real Linux folder, shown as it is. Turn off “Show the Linux folders too” to hide these.".into(), ..Default::default()
                });
                first = false;
            }
        }
    } else if page == "pf" || page == "x86" {
        let x86 = page == "x86";
        let base = if x86 { "C:\\Program Files (x86)\\" } else { "C:\\Program Files\\" };
        for e in c_app_list(data).into_iter().filter(|e| e.x86 == x86) {
            let mut cv = match (&e.app, e.name.as_str()) {
                (None, "Hearth") => CView {
                    kind_text: "Part of Hearth · look only".into(), linux: t(&e.path), real: vec![t(&e.path)], lock: true, look: true, kind: "hearth".into(),
                    note: "Hearth’s built-in apps: Files, Settings, Task Manager and the rest. Look only.".into(), ..Default::default()
                },
                (None, "Hearth Games") => {
                    let g = hg.as_ref();
                    CView {
                        kind_text: format!("Folder · {}", g.map(|g| and_list(&g.stores)).filter(|x| !x.is_empty()).unwrap_or_else(|| "Hearth Games".into())),
                        size: g.map(|g| g.size).unwrap_or(0),
                        linux: g.map(|g| g.dirs.iter().map(|d| t(d)).collect::<Vec<_>>().join(", ")).unwrap_or_default(),
                        real: g.map(|g| g.dirs.iter().map(|d| t(d)).collect()).unwrap_or_default(), kind: "games".into(),
                        note: "Games installed from Hearth Games’ stores.".into(), ..Default::default()
                    }
                }
                (None, _) => CView {
                    kind_text: "Folder · Steam".into(), size: steam_size, linux: t(&e.path), real: vec![t(&e.path)], kind: "steam".into(),
                    note: "Steam and its library: steamapps\\common holds your games, like on Windows.".into(), ..Default::default()
                },
                (Some(a), _) => {
                    let k = s(a, "kind");
                    let sz = a.get("size").and_then(Value::as_u64).unwrap_or(0);
                    if k == "flatpak" {
                        CView { kind_text: "App · Flatpak".into(), size: sz, linux: e.path.clone(), real: vec![e.path.clone()], app: Some(a.clone()), kind: "flatpak".into(),
                                note: "A Flatpak app. Uninstall and Modify work like in Installed apps.".into(), ..Default::default() }
                    } else if k == "recomp" {
                        CView { kind_text: "Game · Recomp".into(), size: sz, linux: t(&e.path), real: vec![t(&e.path)], app: Some(a.clone()), kind: "recomp".into(),
                                note: "A Recomp. Play, Update and Uninstall work like in Hearth Games.".into(), ..Default::default() }
                    } else {
                        CView { kind_text: "Windows app".into(), size: sz, linux: t(&e.path), real: vec![t(&e.path)], app: Some(a.clone()), kind: "winapp".into(),
                                note: "A Windows app. Inside the app itself, it still sees its own C: drive.".into(), ..Default::default() }
                    }
                }
            };
            cv.win = format!("{base}{}", e.name);
            add(&e.name, e.path.clone(), cv);
        }
        out.sort_by(|a, b| {
            let ha = a.name == "Hearth";
            let hb = b.name == "Hearth";
            hb.cmp(&ha).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    } else if page == "appdata" && !home.is_empty() {
        let w = format!("C:\\Users\\{user}\\AppData\\");
        add("Local", format!("{home}/.local/share"), CView {
            kind_text: "Folder · app data".into(), linux: "~/.local/share".into(), win: format!("{w}Local"), real: vec![format!("{home}/.local/share")],
            note: "Where apps keep their data on this PC. Local\\Temp inside it is ~/.cache.".into(), ..Default::default()
        });
        add("Roaming", format!("{home}/.config"), CView {
            kind_text: "Folder · app settings".into(), linux: "~/.config".into(), win: format!("{w}Roaming"), real: vec![format!("{home}/.config")],
            note: "Where apps keep their settings.".into(), ..Default::default()
        });
    }
    out
}

/// The extra entries a real folder gets with the view on: AppData in your home folder, Temp in AppData\Local.
pub fn c_extras(p: &str, ctx: &Ctx, have: &[String]) -> Vec<CEntry> {
    if !ctx.c_on || p.is_empty() || ctx.home.is_empty() {
        return vec![];
    }
    let home = trim_end_slash(&ctx.home).to_string();
    let names: Vec<String> = have.iter().map(|n| n.to_lowercase()).collect();
    let u = if ctx.user.is_empty() { "you" } else { &ctx.user };
    if p == home && !names.iter().any(|n| n == "appdata") {
        return vec![CEntry {
            name: "AppData".into(),
            path: format!("{C_ROOT}/Users/{u}/AppData"),
            cv: CView {
                kind_text: "Folder · app settings".into(), linux: "~/.config, ~/.local/share".into(), win: format!("C:\\Users\\{u}\\AppData"), view: true,
                real: vec!["Roaming: ~/.config".into(), "Local: ~/.local/share".into(), "Local\\Temp: ~/.cache".into()],
                note: "Where apps keep their settings and data. Roaming and Local inside it are your Linux settings folders.".into(), ..Default::default()
            },
        }];
    }
    if p == format!("{home}/.local/share") && !names.iter().any(|n| n == "temp") {
        return vec![CEntry {
            name: "Temp".into(),
            path: format!("{C_ROOT}/Users/{u}/AppData/Local/Temp"),
            cv: CView {
                kind_text: "Folder · temporary files".into(), linux: "~/.cache".into(), win: format!("C:\\Users\\{u}\\AppData\\Local\\Temp"), real: vec![format!("{home}/.cache")],
                note: "Temporary files and caches (~/.cache).".into(), ..Default::default()
            },
        }];
    }
    vec![]
}

/// Why a drop or paste into this place is refused ("" when it isn't).
pub fn c_refusal(p: &str, ctx: &Ctx) -> String {
    if is_c_path(p) {
        return match c_page(p, ctx) {
            "pf" => "You can’t put files straight into Program Files. To add an app, install it, and it shows up here by itself.".into(),
            "x86" => "You can’t put files straight into Program Files (x86). To add an app, install it, and it shows up here by itself.".into(),
            "appdata" => "You can’t put files straight into AppData: open Roaming or Local first.".into(),
            "root" => "You can’t put files straight into Hearth (C:). Open a folder in it first (your own folder is in Users).".into(),
            _ => "That’s a name for another folder: open it first.".into(),
        };
    }
    if p == "/usr" || p.starts_with("/usr/") {
        return "Windows (C:\\Windows) is Hearth itself: look only. Each update replaces it all at once.".into();
    }
    if p == "/var/lib" || p.starts_with("/var/lib/") {
        return "ProgramData (C:\\ProgramData) is look only: what apps keep there is changed by the apps themselves.".into();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(c_on: bool) -> Ctx {
        Ctx {
            home: "/var/home/chris".into(),
            user: "chris".into(),
            drives: vec![
                Drive { letter: "C".into(), path: "/".into(), label: "Hearth (C:)".into() },
                Drive { letter: "G".into(), path: "/run/media/chris/Games".into(), label: "Games (G:)".into() },
            ],
            folders: vec![Folder { label: "Documents".into(), path: "/var/home/chris/Documents".into() }],
            c_on,
            users: "/var/home".into(),
            apps: vec![CApp { name: "Firefox".into(), path: "/var/lib/flatpak/app/org.mozilla.firefox".into(), x86: false, no_map: false, app: None }],
        }
    }

    #[test]
    fn windows_names() {
        let c = ctx(true);
        assert_eq!(to_windows("/var/home/chris/Downloads", &c), "C:\\Users\\chris\\Downloads");
        assert_eq!(to_windows("/run/media/chris/Games/Steam", &c), "G:\\Steam");
        assert_eq!(to_windows("/usr/bin", &c), "C:\\Windows\\bin");
        assert_eq!(to_windows("/var/lib/flatpak/app/org.mozilla.firefox/x", &c), "C:\\Program Files\\Firefox\\x");
        assert_eq!(to_windows("/var/home/chris/.config/foo", &c), "C:\\Users\\chris\\AppData\\Roaming\\foo");
        let off = ctx(false);
        assert_eq!(to_windows("/usr/bin", &off), "C:\\usr\\bin");
        assert_eq!(to_windows("/var/home/chris", &off), "C:\\Users\\chris");
    }

    #[test]
    fn typed_letters() {
        let c = ctx(true);
        assert_eq!(from_windows("C:\\Users\\chris\\Music", &c), Some(Res::Path("/var/home/chris/Music".into())));
        assert_eq!(from_windows("g:/Steam", &c), Some(Res::Path("/run/media/chris/Games/Steam".into())));
        assert_eq!(from_windows("C:\\Program Files", &c), Some(Res::Path("hearth:c/Program Files".into())));
        assert_eq!(from_windows("C:\\Windows\\fonts", &c), Some(Res::Path("/usr/fonts".into())));
        assert_eq!(from_windows("/home", &c), None);
        assert!(no_such_drive("Q:\\x", &c).is_some());
    }

    #[test]
    fn crumbs_and_pages() {
        let c = ctx(true);
        let cr = crumbs_for("/var/home/chris/Documents/x", &c, "hearth:thispc").unwrap();
        assert_eq!(cr.iter().map(|x| x.label.as_str()).collect::<Vec<_>>(), vec!["This PC", "Documents", "x"]);
        let cr = crumbs_for("hearth:c/Program Files", &c, "hearth:thispc").unwrap();
        assert_eq!(cr.iter().map(|x| x.label.as_str()).collect::<Vec<_>>(), vec!["This PC", "Hearth (C:)", "Program Files"]);
        assert_eq!(c_page("hearth:c", &c), "root");
        assert_eq!(c_page("hearth:c/Users/chris/AppData", &c), "appdata");
        assert_eq!(c_resolve("hearth:c/Users/chris/AppData/Local/Temp", &c), Res::Path("/var/home/chris/.cache".into()));
        let root = c_entries("root", &c, &CData::default());
        assert_eq!(root.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Program Files", "Program Files (x86)", "ProgramData", "Users", "Windows"]);
        assert!(!c_refusal("/usr/share", &c).is_empty());
        assert_eq!(c_extras("/var/home/chris", &c, &[]).len(), 1);
    }
}
