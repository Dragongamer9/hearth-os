//! Where one game's saves are on this PC: the manifest's paths (`<base>/Saves/*.sav`, `<winAppData>/Game`...)
//! filled in for this game, natively and inside its Proton/Wine prefix, then looked for on disk.
//!
//! Every file found is also given a portable name: which of the game's places it's under ("game", "home",
//! "prefix-user"...) and where inside it. Backups keep files by those names, so a backup made on one Hearth PC
//! restores on another, where the game, the home folder and the prefix may be somewhere else.

use crate::manifest::{Entry, FileRule};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One game's places on this PC.
#[derive(Debug, Clone, Default)]
pub struct Place {
    pub home: PathBuf,
    pub xdg_data: PathBuf,
    pub xdg_config: PathBuf,
    /// steam, epic, gog, or "" (anything else: then rules for any store count).
    pub store: String,
    /// The game's folder.
    pub base: Option<PathBuf>,
    /// What `<root>` can be: Steam's library folder the game is in, and Steam's own folder.
    pub roots: Vec<PathBuf>,
    /// The folder holding drive_c (a Proton prefix's pfx, or a Wine prefix).
    pub prefix: Option<PathBuf>,
    /// Steam's own folder (its userdata has Steam Cloud's local copies).
    pub steam_dir: Option<PathBuf>,
    pub steam_app: Option<u64>,
    /// The account name for `<osUserName>` on Linux.
    pub user: String,
}

impl Place {
    /// The Windows user folder inside the prefix: steamuser (Proton), or your own name (plain Wine).
    pub fn prefix_user(&self) -> Option<PathBuf> {
        let users = self.prefix.as_ref()?.join("drive_c/users");
        for n in ["steamuser", self.user.as_str()] {
            if !n.is_empty() && users.join(n).is_dir() { return Some(users.join(n)); }
        }
        Some(users.join("steamuser"))
    }

    /// The places files are kept by in a backup, most specific first.
    pub fn named_roots(&self) -> Vec<(&'static str, PathBuf)> {
        let mut v: Vec<(&'static str, PathBuf)> = Vec::new();
        if let Some(b) = &self.base { v.push(("game", b.clone())); }
        if let Some(u) = self.prefix_user() { v.push(("prefix-user", u)); }
        if let Some(p) = &self.prefix {
            v.push(("prefix-public", p.join("drive_c/users/Public")));
            v.push(("prefix-drive", p.join("drive_c")));
        }
        if let Some(s) = &self.steam_dir { v.push(("steam-userdata", s.join("userdata"))); }
        v.push(("xdg-data", self.xdg_data.clone()));
        v.push(("xdg-config", self.xdg_config.clone()));
        v.push(("home", self.home.clone()));
        v
    }

    /// A file's portable name: (place, path inside it), from the most specific place that holds it.
    pub fn portable(&self, file: &Path) -> Option<(&'static str, PathBuf)> {
        let mut best: Option<(&'static str, PathBuf, usize)> = None;
        for (name, root) in self.named_roots() {
            if let Ok(rel) = file.strip_prefix(&root) {
                let depth = root.components().count();
                if !rel.as_os_str().is_empty() && best.as_ref().map(|b| depth > b.2).unwrap_or(true) { best = Some((name, rel.to_path_buf(), depth)); }
            }
        }
        best.map(|(n, r, _)| (n, r))
    }

    /// Where a portable name is on this PC (None: this PC doesn't have that place for the game).
    pub fn resolve(&self, root: &str, rel: &Path) -> Option<PathBuf> {
        self.named_roots().into_iter().find(|(n, _)| *n == root).map(|(_, p)| p.join(rel))
    }
}

/// Whether a manifest rule is for this system (linux, or windows inside a prefix) and store.
pub fn applies(rule: &Option<FileRule>, os: &str, store: &str) -> bool {
    let when = rule.as_ref().map(|r| r.when.as_slice()).unwrap_or(&[]);
    when.is_empty() || when.iter().any(|w| w.os.as_deref().map_or(true, |o| o == os) && w.store.as_deref().map_or(true, |s| store.is_empty() || s == store))
}

/// A manifest path with this game's places filled in. Several results when `<root>` can be more than one folder;
/// none when a placeholder has nothing here (a Windows folder without a prefix, a Linux folder inside one).
pub fn expand(pattern: &str, p: &Place, os: &str) -> Vec<String> {
    let win = os == "windows";
    let pu = if win { p.prefix_user() } else { None };
    let s = |x: &Path| x.to_string_lossy().trim_end_matches('/').to_string();
    let one = |v: Option<PathBuf>| -> Option<Vec<String>> { v.map(|x| vec![s(x.as_path())]) };
    let mut out = vec![pattern.replace('\\', "/")];
    let tokens = ["<base>", "<root>", "<game>", "<home>", "<storeUserId>", "<osUserName>", "<winAppData>", "<winLocalAppData>", "<winLocalAppDataLow>",
                  "<winDocuments>", "<winPublic>", "<winProgramData>", "<winDir>", "<xdgData>", "<xdgConfig>", "<regHkcu>", "<regHklm>"];
    for t in tokens {
        if !out.iter().any(|o| o.contains(t)) { continue; }
        let vals: Option<Vec<String>> = match t {
            "<base>" => one(p.base.clone()),
            "<root>" => { let r: Vec<String> = p.roots.iter().map(|r| s(r.as_path())).collect(); if r.is_empty() { None } else { Some(r) } }
            "<game>" => one(p.base.as_ref().and_then(|b| b.file_name()).map(PathBuf::from)),
            "<home>" => if win { one(pu.clone()) } else { one(Some(p.home.clone())) },
            "<storeUserId>" => Some(vec!["*".into()]),
            "<osUserName>" => if win { one(pu.as_ref().and_then(|u| u.file_name()).map(PathBuf::from)) } else { Some(vec![p.user.clone()]).filter(|v| !v[0].is_empty()) },
            "<winAppData>" if win => one(pu.as_ref().map(|u| u.join("AppData/Roaming"))),
            "<winLocalAppData>" if win => one(pu.as_ref().map(|u| u.join("AppData/Local"))),
            "<winLocalAppDataLow>" if win => one(pu.as_ref().map(|u| u.join("AppData/LocalLow"))),
            "<winDocuments>" if win => one(pu.as_ref().map(|u| u.join("Documents"))),
            "<winPublic>" if win => one(p.prefix.as_ref().map(|x| x.join("drive_c/users/Public"))),
            "<winProgramData>" if win => one(p.prefix.as_ref().map(|x| x.join("drive_c/ProgramData"))),
            "<winDir>" if win => one(p.prefix.as_ref().map(|x| x.join("drive_c/windows"))),
            "<xdgData>" if !win => one(Some(p.xdg_data.clone())),
            "<xdgConfig>" if !win => one(Some(p.xdg_config.clone())),
            _ => None,
        };
        let Some(vals) = vals else { return vec![] };
        out = out.iter().flat_map(|o| vals.iter().map(move |v| o.replace(t, v))).collect();
    }
    out.retain(|o| o.starts_with('/') && !o.contains('<'));
    out.sort();
    out.dedup();
    out
}

fn has_magic(s: &str) -> bool { s.contains(['*', '?', '[']) }

/// Glob matching for one path part: * (anything), ? (one character), [abc] / [a-z] / [!x] (one of, or not).
pub fn wild_match(pat: &str, name: &str, nocase: bool) -> bool {
    let fold = |s: &str| -> Vec<char> { if nocase { s.chars().flat_map(|c| c.to_lowercase()).collect() } else { s.chars().collect() } };
    let p = fold(pat);
    let n = fold(name);
    fn class(p: &[char], i: usize, c: char) -> Option<(bool, usize)> {
        // p[i] == '['; returns (matched, index after ']'), or None when there's no closing ']' (then '[' is literal).
        let mut j = i + 1;
        let neg = j < p.len() && (p[j] == '!' || p[j] == '^');
        if neg { j += 1; }
        let start = j;
        let mut hit = false;
        while j < p.len() && (p[j] != ']' || j == start) {
            if j + 2 < p.len() && p[j + 1] == '-' && p[j + 2] != ']' {
                if p[j] <= c && c <= p[j + 2] { hit = true; }
                j += 3;
            } else {
                if p[j] == c { hit = true; }
                j += 1;
            }
        }
        if j >= p.len() { return None; }
        Some((hit != neg, j + 1))
    }
    fn go(p: &[char], n: &[char], pi: usize, ni: usize) -> bool {
        if pi == p.len() { return ni == n.len(); }
        match p[pi] {
            '*' => (ni..=n.len()).any(|k| go(p, n, pi + 1, k)),
            '?' => ni < n.len() && go(p, n, pi + 1, ni + 1),
            '[' => {
                if ni >= n.len() { return false; }
                match class(p, pi, n[ni]) {
                    Some((true, next)) => go(p, n, next, ni + 1),
                    Some((false, _)) => false,
                    None => n[ni] == '[' && go(p, n, pi + 1, ni + 1),
                }
            }
            c => ni < n.len() && n[ni] == c && go(p, n, pi + 1, ni + 1),
        }
    }
    go(&p, &n, 0, 0)
}

/// Files and folders matching an absolute glob pattern. `nocase`: Windows-style names (inside a prefix a game may
/// have made "Saved Games" or "saved games").
pub fn glob(pattern: &str, nocase: bool) -> Vec<PathBuf> {
    let mut cur = vec![PathBuf::from("/")];
    for part in pattern.split('/').filter(|s| !s.is_empty() && *s != ".") {
        let mut next = Vec::new();
        if part == ".." { return vec![]; }
        if part == "**" {
            for c in &cur { next.push(c.clone()); all_dirs(c, &mut next, 0); }
        } else {
            for c in &cur {
                let exact = c.join(part);
                if !has_magic(part) && std::fs::symlink_metadata(&exact).is_ok() { next.push(exact); continue; }
                if !has_magic(part) && !nocase { continue; }
                let Ok(rd) = std::fs::read_dir(c) else { continue };
                for e in rd.flatten() {
                    if wild_match(part, &e.file_name().to_string_lossy(), nocase) { next.push(e.path()); }
                }
            }
        }
        next.sort();
        next.dedup();
        cur = next;
        if cur.is_empty() || cur.len() > 20000 { break; }
    }
    cur
}

fn all_dirs(p: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 10 || out.len() > 20000 { return; }
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) { out.push(e.path()); all_dirs(&e.path(), out, depth + 1); }
        }
    }
}

/// Every save file of one game: the manifest's paths (as Linux, and as Windows inside its prefix), Steam Cloud's
/// local copy, and folders you added yourself. Sorted, each once.
pub fn find_files(p: &Place, entry: Option<&Entry>, extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut found: BTreeSet<PathBuf> = BTreeSet::new();
    let mut take = |path: &Path| {
        let mut v = Vec::new();
        crate::util::files_under(path, &mut v);
        for f in v { found.insert(f); }
    };
    if let Some(e) = entry {
        let mut systems = vec!["linux"];
        if p.prefix.as_ref().map(|x| x.join("drive_c").is_dir()).unwrap_or(false) { systems.push("windows"); }
        for (pattern, rule) in &e.files {
            for os in &systems {
                if !applies(rule, os, &p.store) { continue; }
                for full in expand(pattern, p, os) {
                    // A pattern matching the whole game folder (or a whole prefix) is never a save location.
                    if p.base.as_ref().map(|b| Path::new(&full) == b.as_path()).unwrap_or(false) { continue; }
                    for hit in glob(&full, *os == "windows") { take(&hit); }
                }
            }
        }
    }
    if let (Some(s), Some(app)) = (&p.steam_dir, p.steam_app) {
        for hit in glob(&format!("{}/userdata/*/{app}/remote", s.to_string_lossy().trim_end_matches('/')), false) { take(&hit); }
    }
    for x in extra { take(x); }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{parse_yaml, Manifest};

    fn put(p: &Path) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, b"x").unwrap(); }

    #[test]
    fn glob_parts() {
        assert!(wild_match("*.dat", "user1.dat", false));
        assert!(!wild_match("*.dat", "user1.dat.bak", false));
        assert!(wild_match("save?.sav", "save1.sav", false));
        assert!(wild_match("profile[0-3].ojs", "profile2.ojs", false));
        assert!(!wild_match("profile[!0-3].ojs", "profile2.ojs", false));
        assert!(wild_match("[store name]", "s", false));
        assert!(wild_match("Saved Games", "saved games", true));
        assert!(!wild_match("Saved Games", "saved games", false));
        assert!(wild_match("a[b", "a[b", false));
    }

    fn sample_place(t: &Path) -> Place {
        let steam = t.join("home/.local/share/Steam");
        Place { home: t.join("home"), xdg_data: t.join("home/.local/share"), xdg_config: t.join("home/.config"), store: "steam".into(),
                base: Some(steam.join("steamapps/common/Hollow Knight")), roots: vec![steam.clone()],
                prefix: Some(steam.join("steamapps/compatdata/367520/pfx")), steam_dir: Some(steam), steam_app: Some(367520), user: "you".into() }
    }

    #[test]
    fn fills_in_places() {
        let t = tempfile::tempdir().unwrap();
        let p = sample_place(t.path());
        let h = t.path().to_string_lossy().to_string();
        assert_eq!(expand("<xdgConfig>/unity3d/x", &p, "linux"), vec![format!("{h}/home/.config/unity3d/x")]);
        assert!(expand("<xdgConfig>/unity3d/x", &p, "windows").is_empty());
        assert_eq!(expand("<winAppData>/G", &p, "windows"), vec![format!("{h}/home/.local/share/Steam/steamapps/compatdata/367520/pfx/drive_c/users/steamuser/AppData/Roaming/G")]);
        assert!(expand("<winAppData>/G", &p, "linux").is_empty());
        assert_eq!(expand("<home>/AppData/LocalLow/T", &p, "windows"), vec![format!("{h}/home/.local/share/Steam/steamapps/compatdata/367520/pfx/drive_c/users/steamuser/AppData/LocalLow/T")]);
        assert_eq!(expand("<root>/userdata/<storeUserId>/1/remote", &p, "linux"), vec![format!("{h}/home/.local/share/Steam/userdata/*/1/remote")]);
        assert_eq!(expand("<base>/../x", &p, "linux").len(), 1);
    }

    #[test]
    fn finds_saves_natively_and_in_the_prefix() {
        let t = tempfile::tempdir().unwrap();
        let p = sample_place(t.path());
        let pfx_user = p.prefix.clone().unwrap().join("drive_c/users/steamuser");
        put(&pfx_user.join("AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat"));
        put(&pfx_user.join("AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat.bak"));
        put(&p.xdg_config.join("unity3d/Team Cherry/Hollow Knight/user2.dat"));
        put(&p.base.clone().unwrap().join("hollow_knight_Data/Config.ini"));
        put(&p.steam_dir.clone().unwrap().join("userdata/1234/367520/remote/cloud.sav"));
        put(&t.path().join("home/Custom/mine.sav"));
        let m = Manifest::from_games(parse_yaml(crate::manifest::tests::SAMPLE).unwrap());
        let (_, e) = m.find(Some(367520), None, "", None).unwrap();
        let files = find_files(&p, Some(e), &[t.path().join("home/Custom")]);
        let names: Vec<String> = files.iter().map(|f| f.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names.len(), 5, "{names:?}");
        for n in ["user1.dat", "user2.dat", "Config.ini", "cloud.sav", "mine.sav"] { assert!(names.contains(&n.to_string()), "{n} in {names:?}"); }
        // Portable names, and back on "another PC" with the game somewhere else.
        let f = files.iter().find(|f| f.ends_with("user1.dat")).unwrap();
        let (root, rel) = p.portable(f).unwrap();
        assert_eq!((root, rel.to_string_lossy().to_string()), ("prefix-user", "AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat".to_string()));
        let mut other = p.clone();
        other.prefix = Some(t.path().join("elsewhere/pfx"));
        assert_eq!(other.resolve(root, &rel).unwrap(), t.path().join("elsewhere/pfx/drive_c/users/steamuser/AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat"));
        let g = files.iter().find(|f| f.ends_with("Config.ini")).unwrap();
        assert_eq!(p.portable(g).unwrap().0, "game");
        let c = files.iter().find(|f| f.ends_with("user2.dat")).unwrap();
        assert_eq!(p.portable(c).unwrap().0, "xdg-config");
    }

    #[test]
    fn windows_names_ignore_case_in_the_prefix() {
        let t = tempfile::tempdir().unwrap();
        let p = sample_place(t.path());
        put(&p.prefix.clone().unwrap().join("drive_c/users/steamuser/Documents/my games/thing/a.sav"));
        let m = Manifest::from_games(parse_yaml(crate::manifest::tests::SAMPLE).unwrap());
        let e = &m.games["Steam Cloud Game"];
        let files = find_files(&p, Some(e), &[]);
        assert_eq!(files.len(), 1, "{files:?}");
    }
}
