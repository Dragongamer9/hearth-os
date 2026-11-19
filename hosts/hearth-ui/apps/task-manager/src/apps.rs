//! Which app each process belongs to: hearth_tasks_procs.py's rules in Rust (the same names, the same protected
//! parts, the same grouping), so Task Manager shows the same groups whichever one draws it.

use std::collections::{HashMap, HashSet};

use crate::sample::{Group, P};

/// What keeps the session running: never ended from here (hearth_tasks_base.py's PROTECTED).
pub fn protected_name(n: &str) -> Option<&'static str> {
    Some(match n {
        "systemd" | "(sd-pam)" => "System services (systemd)",
        "gamescope" | "gamescope-wl" | "gamescopereaper" => "Game Mode screen (gamescope)",
        "labwc" => "Desktop windows (labwc)",
        "quickshell" | "qs" => "Mantel taskbar and menus",
        "nimbus" | "hearth" => "Hearth",
        "nimbus-session" | "nimbus-mode" | "nimbus-watchdog" | "autostart" => "Hearth session",
        "Xwayland" => "Older-app support (Xwayland)",
        "dbus-daemon" | "dbus-broker" | "dbus-broker-launch" => "App messaging (D-Bus)",
        "pipewire" | "pipewire-pulse" => "Sound (PipeWire)",
        "wireplumber" => "Sound (WirePlumber)",
        "gnome-keyring-daemon" | "kwalletd6" | "kwalletd5" => "Password store",
        "mangoapp" => "Performance overlay",
        "xdg-desktop-portal" | "xdg-desktop-portal-wlr" | "xdg-desktop-portal-gtk" | "xdg-desktop-portal-kde" | "xdg-document-portal"
        | "xdg-permission-store" => "Desktop portal",
        "polkit-gnome-authentication-agent-1" | "polkit-kde-authentication-agent-1" | "lxqt-policykit-agent" => "Password prompts",
        "kwin_wayland" | "kwin_wayland_wrapper" | "plasmashell" | "ksmserver" | "gnome-shell" | "kded6" | "startplasma-wayland"
        | "plasma_session" => "Desktop",
        "sddm-helper" | "gdm-wayland-session" | "login" => "Sign-in",
        _ => return None,
    })
}

const INTERPRETERS: [&str; 8] = ["sh", "bash", "dash", "zsh", "python", "python3", "perl", "env"];
const SHARE_MARK: &str = "/nimbus/desktop/";

fn is_interp(n: &str) -> bool {
    INTERPRETERS.contains(&n) || n.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.') == "python"
}

pub fn basename(a: &str) -> String {
    a.rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

fn is_interpreter(p: &P) -> bool {
    !p.cmd.is_empty() && (is_interp(&basename(&p.cmd[0])) || is_interp(&p.comm) || is_interp(&p.exe))
}

/// The script an interpreter (sh, python…) runs, or "".
pub fn script_name(p: &P) -> String {
    if !is_interpreter(p) {
        return String::new();
    }
    let args = &p.cmd[1..];
    if let Some(i) = args.iter().position(|a| a == "-c") {
        if let Some(c) = args.get(i + 1) {
            return c.split_whitespace().find(|w| *w != "exec" && *w != "env" && !w.contains('=')).map(basename).unwrap_or_default();
        }
    }
    args.iter().find(|a| !a.starts_with('-')).map(|a| basename(a)).unwrap_or_default()
}

pub fn names_of(p: &P) -> Vec<String> {
    let mut out = vec![p.exe.clone(), p.comm.clone()];
    if let Some(c) = p.cmd.first() {
        out.push(basename(c));
        out.push(script_name(p));
    }
    out.into_iter().filter(|n| !n.is_empty()).collect()
}

pub fn protected_reason(p: &P, guarded: &HashSet<i32>) -> Option<&'static str> {
    if p.pid == 1 {
        return Some("System services (systemd)");
    }
    if guarded.contains(&p.pid) {
        return Some("Hearth Task Manager");
    }
    let joined = p.cmd.join(" ");
    for n in names_of(p) {
        if let Some(r) = protected_name(&n) {
            if (n == "nimbus" || n == "hearth") && p.cmd.iter().any(|a| a == "--browser") {
                return None; // Hearth Browser is a normal app
            }
            return Some(r);
        }
        if n.starts_with("hearth-") && joined.contains(SHARE_MARK) {
            return Some("Hearth controller and buttons");
        }
    }
    if joined.contains("nimbus-session") || joined.contains("labwc/autostart") {
        return Some("Hearth session");
    }
    None
}

pub fn steam_app_id(p: &P) -> Option<String> {
    for a in &p.cmd {
        if let Some(v) = a.strip_prefix("SteamAppId=").or_else(|| a.strip_prefix("AppId=")) {
            if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) && v != "0" {
                return Some(v.to_string());
            }
        }
    }
    let env = std::fs::read(format!("/proc/{}/environ", p.pid)).unwrap_or_default();
    for kv in env.split(|c| *c == 0) {
        if let Some(v) = kv.strip_prefix(b"SteamAppId=") {
            let v = String::from_utf8_lossy(v).to_string();
            if !v.is_empty() && v != "0" && v.bytes().all(|b| b.is_ascii_digit()) {
                return Some(v);
            }
        }
    }
    None
}

/// A Windows program (.exe) this process runs, as its name, or None.
pub fn windows_exe(p: &P) -> Option<String> {
    for a in p.cmd.iter().take(4) {
        let b = basename(a);
        if b.to_lowercase().ends_with(".exe") && !["wine64-preloader", "wineserver", "explorer.exe", "services.exe", "winedevice.exe", "plugplay.exe", "rpcss.exe", "svchost.exe", "steam.exe", "conhost.exe"].contains(&b.to_lowercase().as_str()) {
            return Some(b[..b.len() - 4].to_string());
        }
    }
    None
}

// ------------------------------------------------------------------ the apps' menu entries
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub hidden: bool,
}
pub type AppIndex = HashMap<String, Entry>;

/// An icon's file from its name (a .desktop file's Icon=): a path as it is, else the icon themes (hicolor, Breeze,
/// Adwaita: the bigger PNGs first, then SVG) and pixmaps, in your folders, the system's and Flatpak's. Looked for
/// once per name (the sampler keeps what it found).
pub fn icon_file(name: &str) -> String {
    if name.starts_with('/') {
        return if std::path::Path::new(name).is_file() { name.to_string() } else { String::new() };
    }
    let mut roots: Vec<String> = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        roots.push(format!("{home}/.icons"));
    }
    for d in data_dirs() {
        roots.push(format!("{d}/icons"));
    }
    for r in &roots {
        for theme in ["hicolor", "breeze", "Adwaita"] {
            for size in ["48x48", "64x64", "32x32", "128x128", "256x256", "24x24"] {
                let p = format!("{r}/{theme}/{size}/apps/{name}.png");
                if std::path::Path::new(&p).is_file() {
                    return p;
                }
            }
            for p in [format!("{r}/{theme}/scalable/apps/{name}.svg"), format!("{r}/{theme}/apps/48/{name}.svg"), format!("{r}/{theme}/apps/32/{name}.svg")] {
                if std::path::Path::new(&p).is_file() {
                    return p;
                }
            }
        }
    }
    for d in data_dirs() {
        for ext in ["png", "svg"] {
            let p = format!("{d}/pixmaps/{name}.{ext}");
            if std::path::Path::new(&p).is_file() {
                return p;
            }
        }
    }
    String::new()
}

fn data_dirs() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let user = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let mut out: Vec<String> = Vec::new();
    for d in std::iter::once(user.clone()).chain(sys.split(':').map(String::from)).chain([format!("{user}/flatpak/exports/share"), "/var/lib/flatpak/exports/share".into()]) {
        if !d.is_empty() && !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

fn parse_desktop(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut in_entry = false;
    for l in text.lines() {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if l.starts_with('[') {
            in_entry = l == "[Desktop Entry]";
            continue;
        }
        if in_entry {
            if let Some((k, v)) = l.split_once('=') {
                let k = k.trim();
                if !k.contains('[') {
                    out.entry(k.to_string()).or_insert_with(|| v.trim().to_string());
                }
            }
        }
    }
    out
}

fn exec_names(exec: &str) -> (Vec<String>, Option<String>) {
    let mut args: Vec<&str> = exec.split_whitespace().filter(|a| !a.starts_with('%')).collect();
    while let Some(a) = args.first() {
        let is_assign = a.split_once('=').map(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).unwrap_or(false);
        if *a == "env" || is_assign {
            args.remove(0);
        } else {
            break;
        }
    }
    let Some(first) = args.first().map(|a| basename(a.trim_matches('"'))) else { return (vec![], None) };
    if first == "flatpak" {
        if let Some(i) = args.iter().position(|a| *a == "run") {
            return (vec![], args[i + 1..].iter().find(|a| !a.starts_with('-')).map(|s| s.to_string()));
        }
    }
    let mut names = vec![first.clone()];
    if is_interp(&first) {
        if let Some(a) = args.iter().skip(1).find(|a| !a.starts_with('-')) {
            names.push(basename(a));
        }
    }
    (names, None)
}

fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<(String, std::path::PathBuf)>, depth: u32) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && depth < 3 {
            walk(&p, base, out, depth + 1);
        } else if p.extension().map(|x| x == "desktop").unwrap_or(false) {
            let rel = p.strip_prefix(base).map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
            out.push((rel.trim_end_matches(".desktop").replace('/', "-"), p));
        }
    }
}

/// {program name, desktop id or window class (lower case): entry}, as hearth_tasks_procs.py's desktop_index.
pub fn desktop_index() -> AppIndex {
    let mut by: AppIndex = HashMap::new();
    let mut put = |key: &str, e: &Entry, weak: bool| {
        let k = key.to_lowercase();
        if k.is_empty() {
            return;
        }
        match by.get(&k) {
            None => {
                by.insert(k, e.clone());
            }
            Some(old) if old.hidden && !e.hidden && !weak => {
                by.insert(k, e.clone());
            }
            _ => {}
        }
    };
    for d in data_dirs() {
        let base = std::path::PathBuf::from(format!("{d}/applications"));
        let mut files = Vec::new();
        walk(&base, &base, &mut files, 0);
        files.sort();
        for (id, path) in files {
            let de = parse_desktop(&std::fs::read_to_string(&path).unwrap_or_default());
            if de.get("Type").map(|t| t != "Application").unwrap_or(false) {
                continue;
            }
            let Some(name) = de.get("Name").cloned() else { continue };
            let e = Entry {
                id: id.clone(),
                name,
                icon: de.get("Icon").cloned().unwrap_or_default(),
                hidden: de.get("NoDisplay").map(|v| v == "true").unwrap_or(false) || de.get("Hidden").map(|v| v == "true").unwrap_or(false),
            };
            put(&id, &e, false);
            let (names, flatpak) = exec_names(de.get("Exec").map(String::as_str).unwrap_or(""));
            if let Some(f) = flatpak {
                put(&f, &e, false);
            }
            for n in names {
                if !is_interp(&n) {
                    put(&n, &e, true);
                }
            }
            if let Some(c) = de.get("StartupWMClass") {
                put(c, &e, false);
            }
            if let Some(last) = id.rsplit('.').next() {
                if last != id {
                    put(last, &e, true);
                }
            }
        }
    }
    by
}

fn cgroup_app(pid: i32) -> (Option<&'static str>, Option<String>) {
    let text = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap_or_default();
    for line in text.lines() {
        let last = line.rsplit('/').next().unwrap_or("");
        if let Some(rest) = last.strip_prefix("app-flatpak-") {
            if let Some(stem) = rest.strip_suffix(".scope") {
                if let Some((id, num)) = stem.rsplit_once('-') {
                    if num.bytes().all(|b| b.is_ascii_digit()) {
                        return (Some("flatpak"), Some(id.to_string()));
                    }
                }
            }
        }
        if let Some(rest) = last.strip_prefix("app-") {
            let stem = rest.strip_suffix(".scope").or_else(|| rest.strip_suffix(".service"));
            if let Some(stem) = stem {
                // app-[launcher-]ID[@…][-N]
                let mut s = stem.split('@').next().unwrap_or("").to_string();
                if let Some((a, b)) = s.rsplit_once('-') {
                    if b.bytes().all(|x| x.is_ascii_digit()) && !b.is_empty() {
                        s = a.to_string();
                    }
                }
                for pre in ["gnome-", "kde-", "plasma-", "dbus-", "cosmic-", "hearth-launch-"] {
                    if let Some(x) = s.strip_prefix(pre) {
                        if x.contains('.') || x.contains('-') {
                            s = x.to_string();
                            break;
                        }
                    }
                }
                return (Some("unit"), Some(s.replace("\\x2d", "-")));
            }
        }
    }
    (None, None)
}

fn lookup_app(p: &P, index: &AppIndex) -> Option<Entry> {
    let (kind, app) = cgroup_app(p.pid);
    if kind == Some("flatpak") {
        if let Some(e) = app.as_ref().and_then(|a| index.get(&a.to_lowercase())) {
            return Some(e.clone());
        }
    }
    for n in names_of(p) {
        let n = n.to_lowercase();
        if is_interp(&n) {
            continue;
        }
        if let Some(e) = index.get(&n) {
            return Some(e.clone());
        }
    }
    app.and_then(|a| index.get(&a.to_lowercase()).cloned())
}

pub fn steam_name(app_id: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let roots = [format!("{home}/.local/share/Steam"), format!("{home}/.steam/steam"), format!("{home}/.var/app/com.valvesoftware.Steam/.local/share/Steam")];
    let mut libs = Vec::new();
    for r in &roots {
        libs.push(format!("{r}/steamapps"));
        let lf = std::fs::read_to_string(format!("{r}/steamapps/libraryfolders.vdf")).unwrap_or_default();
        for l in lf.lines() {
            let l = l.trim();
            if let Some(rest) = l.strip_prefix("\"path\"") {
                let v = rest.trim().trim_matches('"').replace("\\\\", "\\");
                libs.push(format!("{v}/steamapps"));
            }
        }
    }
    for lib in libs {
        let t = std::fs::read_to_string(format!("{lib}/appmanifest_{app_id}.acf")).unwrap_or_default();
        for l in t.lines() {
            let l = l.trim();
            if let Some(rest) = l.strip_prefix("\"name\"") {
                return Some(rest.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Your processes grouped by the app that started them (hearth_tasks_procs.py's build_groups).
pub fn build_groups(procs: &HashMap<i32, P>, uid: u32, me: i32, index: &AppIndex, game_name: &mut dyn FnMut(&str) -> Option<String>) -> Vec<Group> {
    let own: HashMap<i32, &P> = procs.iter().filter(|(_, p)| p.uid == uid && !p.kthread).map(|(k, v)| (*k, v)).collect();
    let mut guarded = HashSet::new();
    let mut cur = me;
    while let Some(p) = procs.get(&cur) {
        if !guarded.insert(cur) {
            break;
        }
        cur = p.ppid;
    }
    // Task Manager itself and its own parents aren't "protected" the way Mantel is: only itself.
    let guarded: HashSet<i32> = [me].into_iter().collect::<HashSet<i32>>().intersection(&guarded).copied().collect();
    let reasons: HashMap<i32, Option<&'static str>> = own.iter().map(|(pid, p)| (*pid, protected_reason(p, &guarded))).collect();
    let infra: HashSet<i32> = reasons.iter().filter(|(_, r)| r.is_some()).map(|(p, _)| *p).collect();
    let mut roots: HashMap<i32, i32> = HashMap::new();
    let mut appids: HashMap<i32, Option<String>> = HashMap::new();
    let mut appid_of = |pid: i32| -> Option<String> { appids.entry(pid).or_insert_with(|| own.get(&pid).and_then(|p| steam_app_id(p))).clone() };
    let mut members: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut pids: Vec<i32> = own.keys().copied().collect();
    pids.sort();
    for pid in pids.iter().copied() {
        if infra.contains(&pid) {
            members.insert(pid, vec![pid]);
            continue;
        }
        let mut path = Vec::new();
        let mut c = pid;
        let root;
        loop {
            if let Some(r) = roots.get(&c) {
                root = *r;
                break;
            }
            path.push(c);
            let par = own[&c].ppid;
            if appid_of(c).is_some() || !own.contains_key(&par) || infra.contains(&par) || path.contains(&par) {
                root = c;
                break;
            }
            c = par;
        }
        for x in path {
            roots.insert(x, root);
        }
        members.entry(root).or_default().push(pid);
    }
    let mut groups = Vec::new();
    for (root, mut ps) in members {
        let rp = own[&root];
        ps.sort();
        let mut g = Group {
            id: format!("{}:{}", rp.pid, rp.start),
            pid: root,
            mem: ps.iter().map(|x| own[x].mem).sum(),
            kind: "background".into(),
            ..Group::default()
        };
        g.pids = ps.clone();
        if let Some(Some(why)) = reasons.get(&root) {
            g.kind = "system".into();
            g.protected = true;
            g.why = why.to_string();
            let sn = script_name(rp);
            g.name = if !sn.is_empty() { sn } else if !rp.exe.is_empty() { rp.exe.clone() } else { rp.comm.clone() };
            g.tag = "Part of Hearth".into();
            groups.push(g);
            continue;
        }
        let order: Vec<i32> = std::iter::once(root).chain(ps.iter().copied().filter(|x| *x != root)).collect();
        let entry = order.iter().take(40).find_map(|x| lookup_app(own[x], index));
        let exe = order.iter().find_map(|x| windows_exe(own[x]));
        if let Some(id) = appid_of(root) {
            g.kind = "game".into();
            g.name = game_name(&id).or_else(|| exe.clone()).unwrap_or_else(|| format!("Steam game {id}"));
            g.icon = format!("steam_icon_{id}");
            g.tag = if exe.is_some() { "Game · Proton".into() } else { "Steam".into() };
        } else if let Some(e) = &entry {
            g.kind = "app".into();
            g.name = e.name.clone();
            g.icon = e.icon.clone();
            g.desktop = e.id.clone();
            if exe.is_some() {
                g.tag = "Windows app".into();
            }
        } else if let Some(x) = &exe {
            g.kind = "game".into();
            g.name = x.clone();
            g.icon = "wine".into();
            g.tag = if rp.cmd.iter().take(3).any(|a| a.to_lowercase().contains("proton")) { "Game · Proton".into() } else { "Windows app".into() };
        } else {
            g.name = if !rp.exe.is_empty() { rp.exe.clone() } else if !rp.comm.is_empty() { rp.comm.clone() } else { root.to_string() };
        }
        groups.push(g);
    }
    groups
}

// ------------------------------------------------------------------ the graphics card, per program (DRM fdinfo)
/// Each program's share of the graphics card from its DRM files' engine time (drm-engine-*: ns), like Windows'
/// GPU column. A program's DRM files are looked for once (and again every 30 s), then only their fdinfo is read.
#[derive(Default)]
pub struct Drm {
    fds: HashMap<(i32, u64), (std::time::Instant, Vec<String>)>,
    prev: HashMap<i32, (std::time::Instant, u64, [u64; 3])>,
    pub last: HashMap<i32, f32>,
    /// Each program's video memory (bytes) and its share of 3D, video decode and video encode (percent), as
    /// hearth-tasks' drm_engines (Performance › GPU › Tuning: Programs on the graphics card).
    pub extra: HashMap<i32, (u64, [f32; 3])>,
}

/// A DRM engine's name: 0 = 3D, 1 = video decode, 2 = video encode (hearth-tasks' engine_kind), else none.
pub fn engine_kind(name: &str) -> Option<usize> {
    let n = name.to_lowercase();
    let n = n.as_str();
    if ["render", "gfx", "rcs", "fragment", "vertex-tiler", "3d"].contains(&n) {
        Some(0)
    } else if ["video", "dec", "vcs", "jpeg", "vcn", "uvd"].contains(&n) || n.starts_with("dec") || n.starts_with("vcn_dec") || n.starts_with("jpeg") {
        Some(1)
    } else if ["enc", "video-enhance", "vecs", "vce"].contains(&n) || n.starts_with("enc") || n.starts_with("vcn_enc") {
        Some(2)
    } else {
        None
    }
}

/// drm-memory-vram / drm-total-vram / drm-total-local: bytes (KiB, MiB or GiB).
pub fn fdinfo_vram(text: &str) -> u64 {
    for l in text.lines() {
        let Some(rest) = l.strip_prefix("drm-total-").or_else(|| l.strip_prefix("drm-memory-")) else { continue };
        let Some((kind, v)) = rest.split_once(':') else { continue };
        let k = kind.trim_end_matches(|c: char| c.is_ascii_digit());
        if k != "vram" && k != "local" {
            continue;
        }
        let mut it = v.split_whitespace();
        let n: u64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let mul = match it.next() {
            Some("KiB") => 1024,
            Some("MiB") => 1048576,
            Some("GiB") => 1073741824,
            _ => 1,
        };
        return n * mul;
    }
    0
}

impl Drm {
    pub fn busy(&mut self, procs: &HashMap<i32, P>, uid: u32) -> HashMap<i32, f32> {
        let now = std::time::Instant::now();
        let mut out = HashMap::new();
        let mut seen = HashSet::new();
        for p in procs.values().filter(|p| p.uid == uid && !p.kthread) {
            let key = p.key();
            seen.insert(key);
            let stale = self.fds.get(&key).map(|(t, _)| now.duration_since(*t).as_secs() > 30).unwrap_or(true);
            if stale {
                let mut list = Vec::new();
                if let Ok(rd) = std::fs::read_dir(format!("/proc/{}/fd", p.pid)) {
                    for e in rd.flatten() {
                        if let Ok(t) = std::fs::read_link(e.path()) {
                            if t.to_string_lossy().starts_with("/dev/dri/") {
                                list.push(e.file_name().to_string_lossy().to_string());
                            }
                        }
                    }
                }
                self.fds.insert(key, (now, list));
            }
            let fds = &self.fds[&key].1;
            if fds.is_empty() {
                continue;
            }
            let mut clients = HashSet::new();
            let mut ns = 0u64;
            let mut kinds = [0u64; 3];
            let mut vram = 0u64;
            for fd in fds {
                let t = std::fs::read_to_string(format!("/proc/{}/fdinfo/{fd}", p.pid)).unwrap_or_default();
                let client = t.lines().find_map(|l| l.strip_prefix("drm-client-id:")).map(|s| s.trim().to_string()).unwrap_or_default();
                if !client.is_empty() && !clients.insert(client) {
                    continue;
                }
                vram += fdinfo_vram(&t);
                for l in t.lines() {
                    if let Some(rest) = l.strip_prefix("drm-engine-") {
                        if let Some((name, v)) = rest.split_once(':') {
                            let x = v.trim().trim_end_matches(" ns").trim().parse::<u64>().unwrap_or(0);
                            ns += x;
                            if let Some(k) = engine_kind(name.trim()) {
                                kinds[k] += x;
                            }
                        }
                    }
                }
            }
            let mut share = [0.0f32; 3];
            if let Some((t0, n0, k0)) = self.prev.get(&p.pid) {
                let dt = now.duration_since(*t0).as_nanos() as f64;
                if dt > 0.0 {
                    out.insert(p.pid, ((ns.saturating_sub(*n0)) as f64 / dt * 100.0).clamp(0.0, 100.0) as f32);
                    for i in 0..3 {
                        share[i] = ((kinds[i].saturating_sub(k0[i])) as f64 / dt * 100.0).clamp(0.0, 100.0) as f32;
                    }
                }
            }
            self.prev.insert(p.pid, (now, ns, kinds));
            self.extra.insert(p.pid, (vram, share));
        }
        self.fds.retain(|k, _| seen.contains(k));
        self.prev.retain(|pid, _| procs.contains_key(pid));
        self.extra.retain(|pid, _| procs.contains_key(pid));
        self.last = out.clone();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: i32, ppid: i32, exe: &str, cmd: &[&str]) -> P {
        P { pid, ppid, comm: exe.into(), exe: exe.into(), cmd: cmd.iter().map(|s| s.to_string()).collect(), uid: 1000, start: 1, ..P::default() }
    }

    #[test]
    fn groups_follow_hearth_tasks_rules() {
        let mut procs = HashMap::new();
        for x in [
            p(1, 0, "systemd", &["/usr/lib/systemd/systemd"]),
            p(10, 1, "plasmashell", &["plasmashell"]),
            p(20, 10, "firefox", &["/usr/lib/firefox/firefox"]),
            p(21, 20, "firefox", &["/usr/lib/firefox/firefox", "-contentproc"]),
            p(30, 1, "python3", &["python3", "/usr/bin/my-script"]),
        ] {
            procs.insert(x.pid, x);
        }
        let mut index = AppIndex::new();
        index.insert("firefox".into(), Entry { id: "firefox".into(), name: "Firefox".into(), icon: "firefox".into(), hidden: false });
        let mut none = |_: &str| -> Option<String> { None };
        let g = build_groups(&procs, 1000, 9999, &index, &mut none);
        let ff = g.iter().find(|x| x.name == "Firefox").expect("Firefox is an app");
        assert_eq!(ff.pids, vec![20, 21]);
        assert_eq!(ff.kind, "app");
        let shell = g.iter().find(|x| x.pid == 10).unwrap();
        assert!(shell.protected, "Plasma's shell keeps the desktop running");
        assert_eq!(g.iter().find(|x| x.pid == 30).unwrap().name, "python3");
        assert_eq!(script_name(&procs[&30]), "my-script");
    }
}
