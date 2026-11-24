//! The Hearth Store's workings, apart from its window (main.rs, view.rs), so they're tested on their own:
//!
//! * [`paths`]: where everything is (your home folder only; nothing on the system is ever written);
//! * [`catalog`]: the list of apps: built into Hearth OS, refreshed from hearthos.fun only when signed with the key
//!   built into Hearth OS, plus the private list on the owner's own devices;
//! * [`release`]: an app's newest release on GitHub (its AppImage, SHA256SUMS.txt, what's new);
//! * [`files`]: installing, updating (one swap), checking and repairing, uninstalling (settings kept, or to the Trash);
//! * [`net`]: downloads through curl, with time limits, that can be cancelled;
//! * [`parts`]: quick updates for Hearth's own apps and Mantel (hearth-part does the work);
//! * [`prefs`]: the Store's own settings (settings_file.rs: atomic, .bak, merged, never written by opening it).

pub mod catalog;
pub mod check;
pub mod files;
pub mod net;
pub mod parts;
pub mod paths;
pub mod prefs;
pub mod release;
pub mod system;

/// This PC's Hearth OS version ("1.17") and edition ("Fedora edition"); the Store's pictures read the fixture's.
pub fn this_pc() -> (String, String) {
    if let Some(f) = paths::fixture() {
        if let Ok(v) = std::fs::read(f.join("this-pc.json")).map(|b| serde_json::from_slice::<serde_json::Value>(&b).unwrap_or_default()) {
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            return (s("hearth"), s("edition"));
        }
    }
    let info: serde_json::Value = std::fs::read("/usr/share/nimbus/image-info.json").ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let ver = std::fs::read_to_string("/usr/share/nimbus/version").map(|s| s.trim().to_string()).unwrap_or_default();
    let ver = if ver.is_empty() { info.get("version").and_then(|x| x.as_str()).unwrap_or("").to_string() } else { ver };
    let base = info.get("base").and_then(|x| x.as_str()).unwrap_or("");
    let edition = if base == "anatase" || std::path::Path::new("/usr/bin/rpm-ostree").exists() || std::path::Path::new("/usr/bin/dnf5").exists() {
        "Fedora edition"
    } else if base == "peppermint" || std::path::Path::new("/etc/debian_version").exists() {
        "Debian edition"
    } else {
        ""
    };
    (short_version(&ver), edition.to_string())
}

/// A version as numbers ("0.6.0" < "0.10.1"; "3.13.0" == "v3.13.0").
pub fn newer(a: &str, b: &str) -> bool {
    cmp_versions(a, b) == std::cmp::Ordering::Greater
}

pub fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |s: &str| -> Vec<u64> {
        s.trim().trim_start_matches(['v', 'V']).split(['.', '-', '+']).map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect()
    };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p.cmp(&q);
        }
    }
    std::cmp::Ordering::Equal
}

/// "0.6.0" → "0.6" (the home page's big title), "3.13.0" → "3.13".
pub fn short_version(v: &str) -> String {
    let p: Vec<&str> = v.split('.').collect();
    if p.len() >= 3 && p[2..].iter().all(|x| *x == "0") {
        p[..2].join(".")
    } else {
        v.to_string()
    }
}

/// A size in the mockups' words: "129 MB", "about 120 MB" (`about`), "2.3 GB".
pub fn mb(bytes: u64, about: bool) -> String {
    if bytes == 0 {
        return String::new();
    }
    let s = if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1e9)
    } else {
        format!("{} MB", ((bytes as f64) / 1e6).round().max(1.0) as u64)
    };
    if about { format!("about {s}") } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers() {
        assert!(newer("0.6.0", "0.5.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(!newer("v3.13.0", "3.13.0"));
        assert!(newer("3.13.1", "3.13"));
        assert!(!newer("1.0.0-beta", "1.0.0"));
        assert_eq!(short_version("0.6.0"), "0.6");
        assert_eq!(short_version("3.13.1"), "3.13.1");
        assert_eq!(mb(128_936_986, false), "129 MB");
        assert_eq!(mb(120_000_000, true), "about 120 MB");
        assert_eq!(mb(2_300_000_000, false), "2.3 GB");
    }
}
