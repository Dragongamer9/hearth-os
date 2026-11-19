//! The list of apps (docs/HEARTH-STORE.md, "The list of apps").
//!
//! The owner (Nov 19): "Build it to where it's built into Hearth OS and not available elsewhere." So the list is
//! Hearth OS's own: /usr/share/hearth/store/catalog.json, in the image, updated with each Hearth OS release. Between
//! releases it may be refreshed from Hearth's own place, hearthos.fun/store/catalog.json, but only when that file is
//! signed (catalog.json.sig, Ed25519) by the owner's key, whose public half is built into the image
//! (/usr/share/hearth/store/catalog-key.pub), and only when it's newer (its "serial") than the one in use. Anything
//! else is ignored and the built-in list stays. Without a key in the image no refresh is ever asked for.
//!
//! The owner's private apps (Hearth OS CAD, the Trains 2010 remake: "Only on my own devices") are never in either
//! list: they come from ~/.config/nimbus/store-private.json, a file only the owner adds on his own devices.

use serde_json::{Map, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Installed by the Store (an AppImage from the app's GitHub releases).
    Install,
    /// Part of Hearth OS: shown with "Built in", updates with Hearth.
    BuiltIn,
    /// Not out yet.
    Coming,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    /// The name on small tiles ("AI Command Center").
    pub short: String,
    /// One line under the name on its page.
    pub tagline: String,
    /// One line on its tile.
    pub card: String,
    pub about: String,
    pub author: String,
    /// Its icon: letters and two colours (top left to bottom right).
    pub icon: String,
    pub from: String,
    pub to: String,
    pub kind: Kind,
    /// owner/repo on GitHub, the AppImage's name ends with `asset`, the release's checksums are in `sums`.
    pub github: String,
    pub asset: String,
    pub sums: String,
    /// What the AppImage is called in its folder (no version in it: an app's own updater replaces it in place).
    pub file: String,
    /// The app updates itself (Command Center): the Store installs it once and never replaces its files.
    pub self_updates: bool,
    /// The app's own folders (kept by default when it's uninstalled; to the Trash otherwise).
    pub data: Vec<String>,
    /// Uninstall's switch, the line under it ("Her memories, settings and chats stay in your home folder").
    pub keep_note: String,
    /// What its own folders hold, in words ("Your settings, chats and sign-ins").
    pub data_note: String,
    /// Update's last step ("Update the menu entry and open her again").
    pub reopen_title: String,
    /// Update's steps: closing it ("She saves her memories first"), opening it again ("Hearth AI opens where you left
    /// her"), and the line under its update ("Her memories and settings stay as they are…").
    pub close_note: String,
    pub reopen_note: String,
    pub update_note: String,
    /// The home page's featured text for a new version (else what's new's first line).
    pub hero: String,
    /// The controller look's line for it ("Colibri is her brain now").
    pub hero_short: String,
    /// Built-in apps: the program that opens it, and how it's said ("Comes with Hearth OS").
    pub open: String,
    /// Pictures of it (https links; kept in the cache once seen).
    pub pictures: Vec<String>,
    /// On the owner's devices only (store-private.json).
    pub private: bool,
    /// The release the Store offers may be pinned here (the owner's okay): version, link, SHA-256, size. When it's
    /// set, the Store uses it instead of asking GitHub for the newest.
    pub pin: Option<Pin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}
fn list(v: &Value, k: &str) -> Vec<String> {
    v.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A file name the Store may write in an app's folder (no slashes, no dots in front).
fn safe_file(f: &str) -> bool {
    !f.is_empty() && !f.starts_with('.') && !f.contains('/') && !f.contains('\0') && f.len() <= 128
}

impl Entry {
    pub fn from_json(v: &Value, private: bool) -> Option<Entry> {
        let id = s(v, "id");
        if !safe_id(&id) {
            return None;
        }
        let name = s(v, "name");
        if name.is_empty() {
            return None;
        }
        let kind = match s(v, "kind").as_str() {
            "builtin" | "built-in" => Kind::BuiltIn,
            "coming" => Kind::Coming,
            _ => Kind::Install,
        };
        let mut file = s(v, "file");
        if !safe_file(&file) {
            file = format!("{}.AppImage", name.replace(' ', "-"));
        }
        let pin = v.get("pin").and_then(|p| {
            let sha = s(p, "sha256").to_lowercase();
            let url = s(p, "url");
            (sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()) && url.starts_with("https://")).then(|| Pin { version: s(p, "version"), url, sha256: sha, size: p.get("size").and_then(Value::as_u64).unwrap_or(0) })
        });
        let short = s(v, "short");
        Some(Entry {
            short: if short.is_empty() { name.clone() } else { short },
            tagline: s(v, "tagline"),
            card: s(v, "card"),
            about: s(v, "about"),
            author: { let a = s(v, "author"); if a.is_empty() { "Dragongamer9".into() } else { a } },
            icon: s(v, "icon"),
            from: s(v, "from"),
            to: s(v, "to"),
            kind,
            github: s(v, "github"),
            asset: { let a = s(v, "asset"); if a.is_empty() { "x86_64.AppImage".into() } else { a } },
            sums: { let a = s(v, "sums"); if a.is_empty() { "SHA256SUMS.txt".into() } else { a } },
            file,
            self_updates: v.get("selfUpdates").and_then(Value::as_bool).unwrap_or(false),
            data: list(v, "data"),
            keep_note: s(v, "keepNote"),
            data_note: s(v, "dataNote"),
            reopen_title: s(v, "reopenTitle"),
            close_note: s(v, "closeNote"),
            reopen_note: s(v, "reopenNote"),
            update_note: s(v, "updateNote"),
            hero: s(v, "hero"),
            hero_short: s(v, "heroShort"),
            open: s(v, "open"),
            pictures: list(v, "pictures").into_iter().filter(|u| u.starts_with("https://") || u.starts_with("file://")).collect(),
            private,
            pin,
            id,
            name,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub apps: Vec<Entry>,
    /// The list's own number (a refresh is used only when it's higher).
    pub serial: u64,
    /// Where it came from: "built-in", or "hearthos.fun" (signed), with when it was checked (unix time).
    pub source: String,
    pub signed_at: i64,
    /// Private apps were added from the owner's list.
    pub private: usize,
}

pub fn parse(text: &[u8]) -> Option<(u64, Vec<Entry>)> {
    let v: Value = serde_json::from_slice(text).ok()?;
    let serial = v.get("serial").and_then(Value::as_u64).unwrap_or(0);
    let apps: Vec<Entry> = v.get("apps")?.as_array()?.iter().filter_map(|a| Entry::from_json(a, false)).collect();
    Some((serial, apps))
}

/// The private list (only on the owner's devices). Its apps never replace a public one with the same id.
pub fn private_apps(path: &Path) -> Vec<Entry> {
    let Ok(b) = std::fs::read(path) else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<Value>(&b) else { return Vec::new() };
    v.get("apps").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| Entry::from_json(x, true)).collect()).unwrap_or_default()
}

// ------------------------------------------------------------------ the signature
/// Hex (the key file, the .sig file) to bytes.
pub fn unhex(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The key built into Hearth OS, or None (then no list from the web is used).
pub fn built_in_key(path: &Path) -> Option<[u8; 32]> {
    let t = std::fs::read_to_string(path).ok()?;
    let line = t.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))?;
    unhex(line)?.try_into().ok()
}

/// Whether `sig` (64 bytes, as hex text) is the owner's signature of exactly these bytes.
pub fn verify(key: &[u8; 32], text: &[u8], sig_hex: &str) -> bool {
    use ed25519_dalek::{Signature, VerifyingKey};
    let Some(sig) = unhex(sig_hex.trim()) else { return false };
    let Ok(sig): Result<[u8; 64], _> = sig.try_into() else { return false };
    let Ok(vk) = VerifyingKey::from_bytes(key) else { return false };
    vk.verify_strict(text, &Signature::from_bytes(&sig)).is_ok()
}

/// The refreshed list kept in the cache (what hearthos.fun gave, checked again each time it's read).
pub fn cached_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let c = crate::paths::cache();
    (c.join("catalog.json"), c.join("catalog.json.sig"))
}

/// The list in use: the built-in one, or the cached refresh when it's signed by the built-in key and newer; then the
/// private apps after the public ones. Reads only.
pub fn load() -> Catalog {
    let mut cat = Catalog { source: "built-in".into(), ..Default::default() };
    if let Ok(b) = std::fs::read(crate::paths::system_catalog()) {
        if let Some((serial, apps)) = parse(&b) {
            cat.serial = serial;
            cat.apps = apps;
        }
    }
    if let Some(key) = built_in_key(&crate::paths::signing_key()) {
        let (cj, cs) = cached_paths();
        if let (Ok(b), Ok(sig)) = (std::fs::read(&cj), std::fs::read_to_string(&cs)) {
            if verify(&key, &b, &sig) {
                if let Some((serial, apps)) = parse(&b) {
                    if serial > cat.serial && !apps.is_empty() {
                        cat.serial = serial;
                        cat.apps = apps;
                        cat.source = "hearthos.fun".into();
                        cat.signed_at = std::fs::metadata(&cj).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
                    }
                }
            }
        }
    }
    for e in private_apps(&crate::paths::private_list()) {
        if !cat.apps.iter().any(|a| a.id == e.id) {
            cat.private += 1;
            cat.apps.push(e);
        }
    }
    cat
}

/// A list from hearthos.fun: kept in the cache only if it's signed and newer than `serial`. True: kept.
pub fn accept_refresh(key: &[u8; 32], text: &[u8], sig: &str, serial: u64) -> std::io::Result<bool> {
    if !verify(key, text, sig) {
        return Ok(false);
    }
    match parse(text) {
        Some((n, apps)) if n > serial && !apps.is_empty() => {}
        _ => return Ok(false),
    }
    let (cj, cs) = cached_paths();
    std::fs::create_dir_all(crate::paths::cache())?;
    crate::files::write_atomic(&cs, sig.trim().as_bytes())?;
    crate::files::write_atomic(&cj, text)?;
    Ok(true)
}

pub fn obj(v: &Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn entries_read_and_bad_ones_are_left_out() {
        let t = br#"{"serial": 3, "apps": [
            {"id": "command-center", "name": "Hearth AI Command Center", "short": "AI Command Center", "github": "Dragongamer9/hearth-ai-command-center", "selfUpdates": true, "file": "Hearth-AI-Command-Center.AppImage"},
            {"id": "../etc", "name": "Bad"},
            {"id": "usb", "name": "Hearth USB maker", "kind": "builtin"},
            {"id": "x", "name": "X", "file": "../../.bashrc", "pin": {"version": "1", "url": "http://plain", "sha256": "00"}}
        ]}"#;
        let (serial, apps) = parse(t).unwrap();
        assert_eq!(serial, 3);
        assert_eq!(apps.len(), 3);
        assert!(apps[0].self_updates);
        assert_eq!(apps[0].asset, "x86_64.AppImage");
        assert_eq!(apps[1].kind, Kind::BuiltIn);
        assert_eq!(apps[2].file, "X.AppImage");
        assert!(apps[2].pin.is_none(), "a pin over plain http or without a full SHA-256 is ignored");
    }

    #[test]
    fn only_the_owners_signature_counts() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let key = sk.verifying_key().to_bytes();
        let text = br#"{"serial": 9, "apps": [{"id": "a", "name": "A"}]}"#;
        let sig = hex(&sk.sign(text).to_bytes());
        assert!(verify(&key, text, &sig));
        assert!(!verify(&key, br#"{"serial": 9, "apps": [{"id": "b", "name": "B"}]}"#, &sig), "a changed list is refused");
        let other = SigningKey::from_bytes(&[8u8; 32]);
        assert!(!verify(&key, text, &hex(&other.sign(text).to_bytes())), "someone else's signature is refused");
        assert!(!verify(&key, text, "zz"));
        assert!(!verify(&key, text, ""));
    }

    #[test]
    fn a_refresh_is_kept_only_when_signed_and_newer() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CACHE_HOME", d.path());
        let sk = SigningKey::from_bytes(&[5u8; 32]);
        let key = sk.verifying_key().to_bytes();
        let text = br#"{"serial": 4, "apps": [{"id": "a", "name": "A"}]}"#;
        let sig = hex(&sk.sign(text).to_bytes());
        assert!(!accept_refresh(&key, text, &sig, 4).unwrap(), "not newer");
        assert!(!accept_refresh(&key, b"{}", &sig, 0).unwrap(), "not signed");
        assert!(accept_refresh(&key, text, &sig, 3).unwrap());
        let (cj, cs) = cached_paths();
        assert_eq!(std::fs::read(cj).unwrap(), text);
        assert!(verify(&key, text, &std::fs::read_to_string(cs).unwrap()));
        std::env::remove_var("XDG_CACHE_HOME");
    }

    #[test]
    fn the_key_file_is_hex_with_comments() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("k.pub");
        std::fs::write(&p, format!("# Hearth Store signing key\n{}\n", "ab".repeat(32))).unwrap();
        assert_eq!(built_in_key(&p), Some([0xab; 32]));
        std::fs::write(&p, "abcd\n").unwrap();
        assert_eq!(built_in_key(&p), None);
        assert_eq!(built_in_key(&d.path().join("none")), None);
    }

    #[test]
    fn private_apps_are_marked_and_never_replace_public_ones() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("store-private.json");
        std::fs::write(&p, r#"{"apps": [{"id": "hearth-cad", "name": "Hearth OS CAD", "kind": "coming"}]}"#).unwrap();
        let a = private_apps(&p);
        assert_eq!(a.len(), 1);
        assert!(a[0].private);
        assert_eq!(a[0].kind, Kind::Coming);
        std::fs::write(&p, "not json").unwrap();
        assert!(private_apps(&p).is_empty());
    }
}
