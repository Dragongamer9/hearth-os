//! An app's newest release: read from its public GitHub release page (no account, no sign-in, no token), or pinned in
//! the list of apps. What the Store needs from it: the version, the AppImage's link and size, its SHA-256 from the
//! release's SHA256SUMS.txt, what's new (the release's notes), the day it came out and its page.
//!
//! Stable is the newest published release (never drafts; never pre-releases unless "Get test versions" is on, as
//! Command Center's own updater does). What was seen last is kept in ~/.cache/hearth/hearth-store/releases.json, so the
//! Store shows it offline and says when it looked.

use crate::catalog::Entry;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rel {
    pub version: String,
    pub asset: String,
    pub url: String,
    pub size: u64,
    /// From the release's SHA256SUMS.txt (or the list's pin). Empty: unknown, and nothing is installed without it.
    pub sha256: String,
    pub sums_url: String,
    pub notes: Vec<String>,
    /// "2026-11-15"
    pub date: String,
    pub page: String,
    /// A test version (pre-release).
    pub test: bool,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// What's new as lines: the notes' bullet points (markdown "- " / "* "), else their first paragraph; never the
/// SHA-256 block the release workflows add.
pub fn notes(body: &str) -> Vec<String> {
    let clean = |l: &str| -> String {
        l.replace("**", "").replace('`', "").replace("__", "").trim().to_string()
    };
    let mut bullets = Vec::new();
    let mut para = Vec::new();
    for l in body.lines() {
        let t = l.trim();
        if t.to_lowercase().starts_with("sha-256") || t.to_lowercase().starts_with("sha256") {
            break;
        }
        if let Some(b) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            bullets.push(clean(b));
        } else if bullets.is_empty() && !t.is_empty() && !t.starts_with('#') {
            para.push(clean(t));
        }
    }
    if !bullets.is_empty() {
        bullets.into_iter().filter(|b| !b.is_empty()).take(8).collect()
    } else {
        para.into_iter().take(3).collect()
    }
}

/// A GitHub release (the REST API's JSON) for this app, without the SHA-256 yet (that's in its sums file).
pub fn from_github(e: &Entry, v: &Value) -> Option<Rel> {
    if v.get("draft").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let tag = s(v, "tag_name");
    let version = tag.trim_start_matches(['v', 'V']).to_string();
    if version.is_empty() {
        return None;
    }
    let assets = v.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
    let app = assets.iter().find(|a| s(a, "name").ends_with(&e.asset))?;
    let sums = assets.iter().find(|a| s(a, "name") == e.sums);
    let url = s(app, "browser_download_url");
    if !url.starts_with("https://") && !url.starts_with("file://") {
        return None;
    }
    Some(Rel {
        version,
        asset: s(app, "name"),
        url,
        size: app.get("size").and_then(Value::as_u64).unwrap_or(0),
        sha256: String::new(),
        sums_url: sums.map(|x| s(x, "browser_download_url")).unwrap_or_default(),
        notes: notes(&s(v, "body")),
        date: s(v, "published_at").chars().take(10).collect(),
        page: s(v, "html_url"),
        test: v.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// SHA256SUMS.txt's line for `name` ("<64 hex>  <name>", or with "*" before a binary file's name).
pub fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let h = it.next()?.to_lowercase();
        let n = it.next()?.trim_start_matches('*');
        (n == name && h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit())).then_some(h)
    })
}

/// The list's pin (the owner's okay) as a release.
pub fn from_pin(e: &Entry) -> Option<Rel> {
    let p = e.pin.as_ref()?;
    Some(Rel { version: p.version.clone(), asset: p.url.rsplit('/').next().unwrap_or("").to_string(), url: p.url.clone(), size: p.size, sha256: p.sha256.clone(), ..Default::default() })
}

/// The GitHub API's address for this app's newest release (stable), or its list (with test versions).
pub fn api_url(e: &Entry, test: bool) -> String {
    if test {
        format!("https://api.github.com/repos/{}/releases?per_page=10", e.github)
    } else {
        format!("https://api.github.com/repos/{}/releases/latest", e.github)
    }
}

/// From the API's answer: the newest release this PC may have (a list: the first that isn't a draft).
pub fn pick(e: &Entry, v: &Value, test: bool) -> Option<Rel> {
    match v {
        Value::Array(a) => a.iter().filter_map(|r| from_github(e, r)).find(|r| test || !r.test),
        _ => from_github(e, v).filter(|r| test || !r.test),
    }
}

pub fn to_json(r: &Rel) -> Value {
    json!({"version": r.version, "asset": r.asset, "url": r.url, "size": r.size, "sha256": r.sha256, "sumsUrl": r.sums_url,
           "notes": r.notes, "date": r.date, "page": r.page, "test": r.test})
}

pub fn from_json(v: &Value) -> Option<Rel> {
    let version = s(v, "version");
    if version.is_empty() {
        return None;
    }
    Some(Rel {
        version,
        asset: s(v, "asset"),
        url: s(v, "url"),
        size: v.get("size").and_then(Value::as_u64).unwrap_or(0),
        sha256: s(v, "sha256"),
        sums_url: s(v, "sumsUrl"),
        notes: v.get("notes").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        date: s(v, "date"),
        page: s(v, "page"),
        test: v.get("test").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// What the Store saw last (releases.json): when it looked, whether it could, each app's release.
#[derive(Debug, Clone, Default)]
pub struct Seen {
    pub checked: i64,
    pub offline: bool,
    pub apps: Map<String, Value>,
    /// Versions a notice was already given for (one notice per version).
    pub told: Vec<String>,
}

pub fn seen_path() -> std::path::PathBuf {
    crate::paths::cache().join("releases.json")
}

pub fn read_seen() -> Seen {
    let v: Value = std::fs::read(seen_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    Seen {
        checked: v.get("checked").and_then(Value::as_i64).unwrap_or(0),
        offline: v.get("offline").and_then(Value::as_bool).unwrap_or(false),
        apps: v.get("apps").and_then(Value::as_object).cloned().unwrap_or_default(),
        told: v.get("told").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
    }
}

pub fn write_seen(s: &Seen) -> std::io::Result<()> {
    std::fs::create_dir_all(crate::paths::cache())?;
    let v = json!({"checked": s.checked, "offline": s.offline, "apps": s.apps, "told": s.told});
    crate::files::write_atomic(&seen_path(), serde_json::to_string_pretty(&v).unwrap_or_default().as_bytes())
}

impl Seen {
    pub fn rel(&self, id: &str) -> Option<Rel> {
        self.apps.get(id).and_then(from_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cc() -> Entry {
        crate::catalog::Entry::from_json(&json!({"id": "command-center", "name": "Hearth AI Command Center", "github": "Dragongamer9/hearth-ai-command-center"}), false).unwrap()
    }

    fn release(tag: &str, pre: bool, draft: bool) -> Value {
        json!({"tag_name": tag, "prerelease": pre, "draft": draft, "published_at": "2026-11-15T06:26:37Z", "html_url": "https://github.com/x/y/releases/tag/v3.13.0",
               "body": "Settings that fit:\n\n- **Settings** that fit on the screen\n- A side list\n\nSHA-256:\n    abc  file",
               "assets": [{"name": format!("Hearth-AI-Command-Center-{}-amd64.deb", &tag[1..]), "size": 1, "browser_download_url": "https://github.com/a.deb"},
                          {"name": format!("Hearth-AI-Command-Center-{}-x86_64.AppImage", &tag[1..]), "size": 128936986, "browser_download_url": "https://github.com/a.AppImage"},
                          {"name": "SHA256SUMS.txt", "size": 222, "browser_download_url": "https://github.com/SHA256SUMS.txt"}]})
    }

    #[test]
    fn a_github_release_is_read() {
        let r = from_github(&cc(), &release("v3.13.0", false, false)).unwrap();
        assert_eq!(r.version, "3.13.0");
        assert_eq!(r.asset, "Hearth-AI-Command-Center-3.13.0-x86_64.AppImage");
        assert_eq!(r.size, 128936986);
        assert_eq!(r.date, "2026-11-15");
        assert_eq!(r.sums_url, "https://github.com/SHA256SUMS.txt");
        assert_eq!(r.notes, vec!["Settings that fit on the screen", "A side list"]);
        assert!(from_github(&cc(), &release("v3.14.0", false, true)).is_none(), "drafts are never offered");
    }

    #[test]
    fn test_versions_only_when_asked() {
        let list = json!([release("v3.14.0", true, false), release("v3.13.0", false, false)]);
        assert_eq!(pick(&cc(), &list, false).unwrap().version, "3.13.0");
        assert_eq!(pick(&cc(), &list, true).unwrap().version, "3.14.0");
        assert!(pick(&cc(), &release("v3.14.0", true, false), false).is_none());
    }

    #[test]
    fn sums_are_matched_by_name() {
        let h = "a".repeat(64);
        let t = format!("{h}  Hearth-AI-0.6.0-x86_64.AppImage\n{}  Hearth-AI-0.6.0-amd64.deb\n", "b".repeat(64));
        assert_eq!(sum_for(&t, "Hearth-AI-0.6.0-x86_64.AppImage"), Some(h.clone()));
        assert_eq!(sum_for(&format!("{h} *x.AppImage"), "x.AppImage"), Some(h));
        assert_eq!(sum_for(&t, "other"), None);
        assert_eq!(sum_for("zz  x", "x"), None);
    }

    #[test]
    fn notes_without_bullets_are_the_first_lines() {
        assert_eq!(notes("Hearth AI 0.6.0\n\nSHA-256:\n  x"), vec!["Hearth AI 0.6.0"]);
        assert!(notes("").is_empty());
    }
}
