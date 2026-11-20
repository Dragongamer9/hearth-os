//! The vault: any folder of Markdown notes (by default the clone of hearth-workspace). Read once when it opens and
//! again only when something changed (a save, a sync); nothing watches it in the background.
//!
//! The workspace's layout (its CLAUDE.md) is understood: `<project>/<kind>/<item>/`, the daily logs in `log/`, and the
//! top files (INDEX.md is Home). Links are `[[links]]` and, since the workspace's own notes name items by their path
//! (`hearth/mockups/hearth-store`), a note that mentions an item's path links to it too ("log mentions").

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PROJECTS: [&str; 8] = ["hearth", "beacon", "quad-mech", "gravesleep", "tributary", "hearth-ai", "command-center", "other"];
pub const KINDS: [&str; 5] = ["videos", "mockups", "research", "notes", "assets"];

/// Most notes read; a bigger folder is cut there (and said so).
pub const MAX_NOTES: usize = 20_000;
const MAX_NOTE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Its path in the vault, with '/'.
    pub rel: String,
    pub text: String,
    pub title: String,
    /// The graph's and tree's short name: an item's folder for its README, "11-19" for a log, else the file's name.
    pub label: String,
    /// One of PROJECTS, "log", "home" (the top files), "tools", or "" (anything else).
    pub project: String,
    pub kind: String,
    pub item: String,
    pub links: Vec<String>,
    pub embeds: Vec<String>,
    pub tags: Vec<String>,
    pub words: usize,
    pub chars: usize,
    pub mtime: i64,
    pub log: Option<crate::day::Ymd>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    /// `[[link]]`
    Wiki,
    /// The item's path written out (the logs').
    Mention,
    /// `![[picture]]` or another note embedded.
    Embed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Default)]
pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    /// Every other file (pictures, scripts), by path.
    pub files: Vec<String>,
    /// Every folder, by path.
    pub dirs: Vec<String>,
    pub edges: Vec<Edge>,
    pub cut: bool,
    /// Folders the vault's .gitignore leaves out (build/, __pycache__/): read (a note may show a picture from one) but
    /// not shown in the folder tree, as Git doesn't keep them.
    pub ignored: Vec<String>,
}

pub fn classify(rel: &str) -> (String, String, String) {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() == 1 {
        return ("home".into(), String::new(), String::new());
    }
    match parts[0] {
        "log" => ("log".into(), String::new(), String::new()),
        "tools" => ("tools".into(), String::new(), String::new()),
        p if PROJECTS.contains(&p) => {
            let kind = parts.get(1).filter(|_| parts.len() > 2).map(|s| s.to_string()).unwrap_or_default();
            let item = parts.get(2).filter(|_| parts.len() > 3).map(|s| s.to_string()).unwrap_or_default();
            (p.to_string(), kind, item)
        }
        _ => (String::new(), String::new(), String::new()),
    }
}

fn label_of(rel: &str, item: &str) -> String {
    if let Some(d) = crate::day::log_of(rel) {
        return format!("{:02}-{:02}", d.1, d.2);
    }
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let stem = name.strip_suffix(".md").unwrap_or(name);
    if stem.eq_ignore_ascii_case("readme") {
        if !item.is_empty() {
            return item.to_string();
        }
        if let Some(parent) = rel.rsplit('/').nth(1) {
            return parent.to_string();
        }
    }
    stem.to_string()
}

pub fn note_from(rel: &str, text: String, mtime: i64) -> Note {
    let (project, kind, item) = classify(rel);
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let stem = name.strip_suffix(".md").unwrap_or(name).to_string();
    let mut links = Vec::new();
    let mut embeds = Vec::new();
    for (t, e) in crate::md::links(&text) {
        if e { embeds.push(t) } else { links.push(t) }
    }
    Note {
        rel: rel.to_string(),
        title: crate::md::title(&text, &stem),
        label: label_of(rel, &item),
        tags: crate::md::tags(&text),
        words: crate::md::words(&text),
        chars: text.chars().count(),
        log: crate::day::log_of(rel),
        project,
        kind,
        item,
        links,
        embeds,
        mtime,
        text,
    }
}

fn mtime_of(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn skip(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules" || name == "target" || name == "__pycache__"
}

impl Vault {
    /// Reads the folder: every .md note, the other files' names, the folders.
    pub fn scan(root: &Path) -> Vault {
        let mut v = Vault { root: root.to_path_buf(), ..Default::default() };
        let ignore: Vec<String> = std::fs::read_to_string(root.join(".gitignore"))
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().trim_start_matches('/').trim_end_matches('/').to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.contains('*') && !l.contains('/'))
            .collect();
        let mut stack = vec![(root.to_path_buf(), String::new())];
        let mut seen = 0usize;
        while let Some((dir, rel)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            let mut entries: Vec<_> = rd.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let name = e.file_name().to_string_lossy().to_string();
                if skip(&name) {
                    continue;
                }
                seen += 1;
                if seen > MAX_NOTES * 4 || v.notes.len() >= MAX_NOTES {
                    v.cut = true;
                    break;
                }
                let r = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_dir() {
                    if ignore.iter().any(|x| *x == name) || v.ignored.iter().any(|x| r.starts_with(&format!("{x}/"))) {
                        v.ignored.push(r.clone());
                    }
                    v.dirs.push(r.clone());
                    stack.push((e.path(), r));
                } else if name.to_lowercase().ends_with(".md") {
                    let Ok(m) = e.metadata() else { continue };
                    if m.len() > MAX_NOTE_BYTES {
                        v.files.push(r);
                        continue;
                    }
                    let text = std::fs::read(e.path()).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
                    v.notes.push(note_from(&r, text, mtime_of(&m)));
                } else {
                    v.files.push(r);
                }
            }
        }
        v.notes.sort_by(|a, b| a.rel.cmp(&b.rel));
        v.files.sort();
        v.dirs.sort();
        v.link();
        v
    }

    /// The links between notes again (after a note changed).
    pub fn link(&mut self) {
        let mut edges: Vec<Edge> = Vec::new();
        let items = self.items();
        let item_note: Vec<(String, Option<usize>)> = items.iter().map(|it| (it.clone(), self.item_note(it))).collect();
        for (i, n) in self.notes.iter().enumerate() {
            for t in &n.links {
                if let Some(j) = self.resolve(t) {
                    if j != i {
                        edges.push(Edge { from: i, to: j, kind: EdgeKind::Wiki });
                    }
                }
            }
            for t in &n.embeds {
                if let Some(j) = self.resolve(t) {
                    if j != i {
                        edges.push(Edge { from: i, to: j, kind: EdgeKind::Embed });
                    }
                }
            }
            for (it, target) in &item_note {
                let Some(j) = *target else { continue };
                if j == i || n.rel.starts_with(&format!("{it}/")) {
                    continue;
                }
                if mentions(&n.text, it) {
                    edges.push(Edge { from: i, to: j, kind: EdgeKind::Mention });
                }
            }
        }
        edges.sort_by_key(|e| (e.from, e.to, e.kind));
        edges.dedup_by_key(|e| (e.from, e.to));
        self.edges = edges;
    }

    pub fn find(&self, rel: &str) -> Option<usize> {
        self.notes.binary_search_by(|n| n.rel.as_str().cmp(rel)).ok()
    }

    /// Every `<project>/<kind>/<item>` folder.
    pub fn items(&self) -> Vec<String> {
        self.dirs
            .iter()
            .filter(|d| {
                let p: Vec<&str> = d.split('/').collect();
                p.len() == 3 && PROJECTS.contains(&p[0]) && KINDS.contains(&p[1])
            })
            .cloned()
            .collect()
    }

    /// An item's note: its README.md, else its first note.
    pub fn item_note(&self, item: &str) -> Option<usize> {
        self.find(&format!("{item}/README.md")).or_else(|| self.notes.iter().position(|n| n.rel.starts_with(&format!("{item}/"))))
    }

    /// What a `[[link]]` points at: a note's path (with or without .md), a folder's README, a log's day, a name.
    pub fn resolve(&self, target: &str) -> Option<usize> {
        let t = target.split(['#', '|']).next().unwrap_or("").trim().trim_matches('/');
        if t.is_empty() {
            return None;
        }
        let t = t.strip_prefix("./").unwrap_or(t);
        if let Some(i) = self.find(t) {
            return Some(i);
        }
        if let Some(i) = self.find(&format!("{t}.md")) {
            return Some(i);
        }
        if let Some(i) = self.find(&format!("{t}/README.md")) {
            return Some(i);
        }
        if crate::day::parse_ymd(t).is_some() && t.len() == 10 {
            if let Some(i) = self.find(&format!("log/{t}.md")) {
                return Some(i);
            }
        }
        let low = t.to_lowercase();
        let base = low.strip_suffix(".md").unwrap_or(&low);
        // A note's name, then an item's name, then a path ending.
        if let Some(i) = self.notes.iter().position(|n| n.rel.rsplit('/').next().map(|f| f.to_lowercase().strip_suffix(".md").map(|s| s == base).unwrap_or(false)).unwrap_or(false) && !n.rel.to_lowercase().ends_with("readme.md")) {
            return Some(i);
        }
        if let Some(it) = self.items().into_iter().find(|it| it.rsplit('/').next().map(|s| s.eq_ignore_ascii_case(base)).unwrap_or(false)) {
            return self.item_note(&it);
        }
        self.notes.iter().position(|n| n.rel.to_lowercase().strip_suffix(".md").map(|r| r.ends_with(&format!("/{base}"))).unwrap_or(false))
    }

    /// A file (a picture) a note embeds: by path, or by its name anywhere in the vault.
    pub fn resolve_file(&self, target: &str, from: &str) -> Option<PathBuf> {
        let t = target.trim().trim_start_matches("./");
        if t.starts_with("http://") || t.starts_with("https://") {
            return None;
        }
        let dir = from.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for cand in [t.to_string(), if dir.is_empty() { t.to_string() } else { format!("{dir}/{t}") }] {
            if self.files.binary_search(&cand).is_ok() {
                return Some(self.root.join(&cand));
            }
        }
        let name = t.rsplit('/').next().unwrap_or(t);
        self.files.iter().find(|f| f.ends_with(&format!("/{t}")) || f.rsplit('/').next() == Some(name)).map(|f| self.root.join(f))
    }

    /// Notes linking to note `i`, with the line that does it.
    pub fn backlinks(&self, i: usize) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        let Some(n) = self.notes.get(i) else { return out };
        let item = if !n.item.is_empty() { format!("{}/{}/{}", n.project, n.kind, n.item) } else { String::new() };
        for e in self.edges.iter().filter(|e| e.to == i) {
            let src = &self.notes[e.from];
            let line = src
                .text
                .lines()
                .find(|l| (!item.is_empty() && l.contains(&item)) || l.contains("[[") && src.links.iter().any(|t| self.resolve(t) == Some(i) && l.contains(t.as_str())) || l.contains(&n.rel))
                .unwrap_or("")
                .trim()
                .trim_start_matches("- ")
                .to_string();
            out.push((e.from, line));
        }
        out
    }

    /// Mentions of the note's name that aren't links (Obsidian's "unlinked mentions").
    pub fn unlinked(&self, i: usize) -> usize {
        let Some(n) = self.notes.get(i) else { return 0 };
        let name = n.label.to_lowercase();
        if name.len() < 4 {
            return 0;
        }
        let linked: Vec<usize> = self.edges.iter().filter(|e| e.to == i).map(|e| e.from).collect();
        self.notes.iter().enumerate().filter(|(j, m)| *j != i && !linked.contains(j) && m.text.to_lowercase().contains(&name)).count()
    }

    /// Notes per project (the graph's filters and "no notes yet").
    pub fn project_counts(&self) -> BTreeMap<String, usize> {
        let mut m = BTreeMap::new();
        for p in PROJECTS {
            m.insert(p.to_string(), 0);
        }
        for n in &self.notes {
            *m.entry(n.project.clone()).or_insert(0) += 1;
        }
        m
    }

    /// Every tag with how many notes have it.
    pub fn tag_counts(&self) -> Vec<(String, usize)> {
        let mut m: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for n in &self.notes {
            for t in &n.tags {
                let e = m.entry(t.to_lowercase()).or_insert((t.clone(), 0));
                e.1 += 1;
            }
        }
        let mut v: Vec<(String, usize)> = m.into_values().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }

    /// Replaces one note's text (after it was saved) and links again.
    pub fn update(&mut self, rel: &str, text: String, mtime: i64) {
        let n = note_from(rel, text, mtime);
        match self.notes.binary_search_by(|x| x.rel.as_str().cmp(rel)) {
            Ok(i) => self.notes[i] = n,
            Err(i) => self.notes.insert(i, n),
        }
        self.link();
    }
}

/// `text` names `item` ("hearth/mockups/hearth-store") as a whole path, not a longer one.
pub fn mentions(text: &str, item: &str) -> bool {
    let mut from = 0;
    while let Some(at) = text[from..].find(item) {
        let s = from + at;
        let e = s + item.len();
        let before = text[..s].chars().next_back().unwrap_or(' ');
        let after = text[e..].chars().next().unwrap_or(' ');
        let ok_b = !(before.is_alphanumeric() || before == '-' || before == '_' || before == '/');
        let ok_a = !(after.is_alphanumeric() || after == '-' || after == '_');
        if ok_b && ok_a {
            return true;
        }
        from = e;
    }
    false
}

#[cfg(test)]
pub fn sample(dir: &Path) {
    let w = |p: &str, t: &str| {
        let f = dir.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, t).unwrap();
    };
    w("INDEX.md", "# Index\n\n## Hearth OS\n- `hearth/mockups/hearth-store`: the Store. Status: approved.\n");
    w("README.md", "# hearth-workspace\n");
    w("hearth/mockups/hearth-store/README.md", "# Hearth Store (mockups and plan)\n\nStatus: **approved** · Belongs to: **1.16**\n#mockup #1.16\n- [x] Home\n- [ ] Tried on the ROG Ally with a controller\nThe built Store's pictures: [[hearth/assets/1.16-store-shots]]\n![[store-1-home.png]]\n");
    w("hearth/mockups/hearth-store/build/store-1-home.png", "png");
    w("hearth/assets/1.16-store-shots/README.md", "# Store shots\nchecked against its mockups #1.16\n");
    w("hearth/mockups/hearth-notes/README.md", "# Hearth Notes\n#draft\nWaiting: the owner's look.\n");
    w("other/research/ts2010-freeware/README.md", "# TS2010 freeware\n#dad\n- [ ] Run the TS2010 freeware scripts on Dad's PC\n");
    w("log/2026-11-19.md", "# 2026-11-19\n\n- Store mockups approved: hearth/mockups/hearth-store (#1.16)\n- ts2010: other/research/ts2010-freeware\n");
    w("log/2026-11-20.md", "# 2026-11-20\n\n- 03:04 Made: hearth/mockups/hearth-notes. Waiting: the owner's look.\n- [ ] Look at labwc's busy minute #1.16\n- [x] Push the Hearth Notes mockups #draft\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_workspace_and_its_links() {
        let d = tempfile::tempdir().unwrap();
        sample(d.path());
        std::fs::create_dir_all(d.path().join(".git")).unwrap();
        std::fs::write(d.path().join(".git/HEAD.md"), "not a note").unwrap();
        let v = Vault::scan(d.path());
        assert_eq!(v.notes.len(), 8, "{:?}", v.notes.iter().map(|n| &n.rel).collect::<Vec<_>>());
        assert!(v.find(".git/HEAD.md").is_none(), "hidden folders are skipped");
        let store = v.find("hearth/mockups/hearth-store/README.md").unwrap();
        assert_eq!(v.notes[store].label, "hearth-store");
        assert_eq!(v.notes[store].project, "hearth");
        assert_eq!(v.notes[store].kind, "mockups");
        assert_eq!(v.notes[store].title, "Hearth Store (mockups and plan)");
        assert_eq!(v.notes[v.find("log/2026-11-19.md").unwrap()].label, "11-19");
        // [[hearth/assets/1.16-store-shots]] is the folder's README.
        let shots = v.find("hearth/assets/1.16-store-shots/README.md").unwrap();
        assert_eq!(v.resolve("hearth/assets/1.16-store-shots"), Some(shots));
        assert_eq!(v.resolve("1.16-store-shots"), Some(shots));
        assert_eq!(v.resolve("2026-11-20"), v.find("log/2026-11-20.md"));
        assert!(v.edges.iter().any(|e| e.from == store && e.to == shots && e.kind == EdgeKind::Wiki));
        // The logs' written-out paths are links too, and backlinks show the line.
        let log19 = v.find("log/2026-11-19.md").unwrap();
        assert!(v.edges.iter().any(|e| e.from == log19 && e.to == store && e.kind == EdgeKind::Mention));
        let bl = v.backlinks(store);
        assert!(bl.iter().any(|(i, l)| *i == log19 && l.contains("hearth/mockups/hearth-store")), "{bl:?}");
        assert!(v.resolve_file("store-1-home.png", "hearth/mockups/hearth-store/README.md").is_some());
        let c = v.project_counts();
        assert_eq!(c["beacon"], 0, "a project with no notes says so");
        assert_eq!(c["hearth"], 3);
        assert!(v.tag_counts().iter().any(|(t, n)| t == "1.16" && *n == 4));
        assert!(!mentions("hearth/mockups/hearth-store-shots", "hearth/mockups/hearth-store"));
    }
}
