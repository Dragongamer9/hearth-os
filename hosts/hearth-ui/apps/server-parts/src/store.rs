//! Each part's state in the Hearth Store, read the way the Store reads it, nothing written: the newest version in the
//! Store's last signed list (~/.cache/hearth/hearth-store/catalog.json, its "apps" and its test versions in "tests")
//! against the version on this PC (a system part's /var/lib/hearth/system-parts/<id>/part.json, an app part's
//! ~/.local/share/hearth/parts/<id>/part.json, else the one that came with Hearth OS). Installing is the Store's own
//! work: a system part through `hearth-admin system-part-install` (the admin password), an app part in the Store.
//!
//! CI's pictures: $HEARTH_SERVER_FIXTURES/store.json ({"<part id>": {"state": "ready", "version", "notes": [..],
//! "test": true}}).

use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Up {
    /// the Store's part id
    pub id: String,
    /// a newer version is in the Store
    pub ready: bool,
    pub version: String,
    pub have: String,
    /// what's new in it, one line each
    pub notes: Vec<String>,
    /// a test version is in the Store
    pub test: bool,
    /// a system part (installed root-owned, with the admin password)
    pub system: bool,
}

fn read(p: &std::path::Path) -> Value {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null)
}

fn home() -> PathBuf {
    hearth_ui::paths::home()
}
fn cache() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join(".cache")).join("hearth/hearth-store/catalog.json")
}

fn nums(v: &str) -> Vec<u64> {
    v.trim_start_matches(['v', 'V']).split(['.', '-']).map(|x| x.chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)).collect()
}
/// a is newer than b.
pub fn newer(a: &str, b: &str) -> bool {
    let (mut x, mut y) = (nums(a), nums(b));
    let n = x.len().max(y.len());
    x.resize(n, 0);
    y.resize(n, 0);
    x > y
}

fn installed(id: &str, system: bool) -> String {
    let p = if system { PathBuf::from("/var/lib/hearth/system-parts").join(id).join("part.json") } else { home().join(".local/share/hearth/parts").join(id).join("part.json") };
    let v = read(&p);
    let have = v.get("version").and_then(Value::as_str).unwrap_or("").to_string();
    if !have.is_empty() {
        return have;
    }
    std::fs::read_to_string("/usr/share/nimbus/version").map(|s| s.trim().to_string()).unwrap_or_default()
}

/// The state of one Store part.
pub fn state(id: &str) -> Up {
    if let Some(d) = crate::engine::fixtures() {
        let v = read(&d.join("store.json"));
        let x = v.get(id).cloned().unwrap_or(Value::Null);
        return Up {
            id: id.into(),
            ready: x.get("state").and_then(Value::as_str) == Some("ready"),
            version: crate::s(&x, "version"),
            have: crate::s(&x, "have"),
            notes: crate::strs(&x, "notes"),
            test: crate::b(&x, "test"),
            system: id == crate::parts::HELPER_PART,
        };
    }
    let list = read(&cache());
    let find = |k: &str| -> Option<Value> { list.get(k).and_then(Value::as_array).and_then(|a| a.iter().find(|e| e.get("part").and_then(|p| p.get("id")).and_then(Value::as_str) == Some(id)).cloned()) };
    let main = find("apps");
    let test = find("tests");
    let system = main.as_ref().or(test.as_ref()).map(|e| crate::b(e, "part.system")).unwrap_or(id == crate::parts::HELPER_PART);
    let have = installed(id, system);
    let mut u = Up { id: id.into(), have: have.clone(), system, test: test.is_some(), ..Default::default() };
    if let Some(e) = main {
        u.version = crate::s(&e, "part.version");
        u.notes = crate::strs(&e, "part.notes");
        u.ready = !u.version.is_empty() && (have.is_empty() || newer(&u.version, &have));
    }
    u
}

/// The command that installs it (the Store's own way): a system part with the admin password; an app part opens the
/// Store, which installs it.
pub fn install_cmd(u: &Up, test: bool) -> Vec<String> {
    if u.system {
        let mut a = vec!["system-part-install", u.id.as_str()];
        if test {
            a.push("test");
        }
        crate::engine::admin(&a)
    } else {
        vec!["hearth-store-app".into(), "--page".into(), "parts".into()]
    }
}
