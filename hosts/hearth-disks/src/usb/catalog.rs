//! The list of systems Make a bootable USB offers (usb/catalog.json, built into Disks) and how the newest version
//! of each is found from the project's own release index. Finding is done with a `get` function (curl in Disks,
//! canned pages in the tests), so every way of finding a release is tested without the network.
//!
//! Each entry's `resolve.kind`:
//!  - fedora:   Fedora's releases.json; the newest stable version of the subvariant (x86_64 ISO). Its checksum file
//!              is found in the image's folder (`checksums.listing`), clearsigned by Fedora's key.
//!  - arch:     Arch's releng JSON; the newest available release on `mirror`; its signer must be the one Arch names.
//!  - pop:      System76's build service: the address and SHA-256 of the newest build.
//!  - dir-index: a folder listing of versions (`dir`, a regex on the links; `lts` keeps even years, for Ubuntu),
//!              the newest one, then `sub`; the image is picked from the checksum file (`checksums.pick`) or from
//!              the folder's listing (`image`).
//!  - dir:      a fixed folder; the image is picked from its checksum file.
//!  - single:   a fixed address. `followName`: the address is a moving "Current" link; the real file name is read
//!              from its checksum file so the download and its checksum always match.
//!  - page:     the project's download page lists the image's address (`pick`, the newest version wins).
//!  - microsoft-page: Windows. Nothing is downloaded by Disks: Microsoft's own page opens in the browser.
//! Templates in `checksums.file` and `signature.file`: {url} the image's address, {dir} its folder (ends in /).

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const CATALOG_JSON: &str = include_str!("../../usb/catalog.json");

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema: u32,
    #[serde(default)]
    pub about: String,
    pub groups: Vec<Group>,
    pub keys: BTreeMap<String, Key>,
    pub entries: Vec<Entry>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Group { pub id: String, pub name: String }

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    pub file: String,
    pub owner: String,
    pub fingerprints: Vec<String>,
    pub source: String,
    #[serde(default)]
    pub documented: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub group: String,
    pub family: String,
    pub name: String,
    #[serde(default)]
    pub flavour: String,
    #[serde(default = "x86_64")]
    pub arch: String,
    pub homepage: String,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub size_hint: String,
    pub resolve: Resolve,
    #[serde(default)]
    pub checksums: Option<Checksums>,
    #[serde(default)]
    pub signature: Option<Signature>,
    /// raw (the default: written as it is) or windows (FAT32 and Windows' files copied).
    #[serde(default)]
    pub write: String,
    /// When the version isn't found from an index: what's pinned and how to update it.
    #[serde(default)]
    pub pinned: String,
    #[serde(default)]
    pub note: String,
}

fn x86_64() -> String { "x86_64".into() }

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Resolve {
    Fedora { index: String, subvariant: String },
    Arch { index: String, mirror: String },
    Pop { index: String },
    #[serde(rename_all = "camelCase")]
    DirIndex { index: String, dir: String, #[serde(default)] lts: bool, #[serde(default)] sub: String, #[serde(default)] image: String },
    Dir { dir: String },
    #[serde(rename_all = "camelCase")]
    Single { url: String, #[serde(default)] follow_name: bool },
    Page { page: String, pick: String },
    MicrosoftPage { page: String, product: String },
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Checksums {
    /// The checksum file's address (a template), or empty with `listing`.
    #[serde(default)]
    pub file: String,
    /// A regex for the checksum file's name in the image's folder listing (Fedora's *-CHECKSUM).
    #[serde(default)]
    pub listing: String,
    /// A regex for the image's file name in the checksum file (its first group is the version).
    #[serde(default)]
    pub pick: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    /// detached (a .sig/.gpg/.asc/.sign file) or clearsigned (the checksum file itself is signed)
    pub kind: String,
    /// What the signature is over: checksums, or the image itself.
    pub of: String,
    #[serde(default)]
    pub file: String,
    pub key: String,
}

/// A release found for an entry: everything needed to download and check it.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub id: String,
    pub name: String,
    pub version: String,
    pub url: String,
    pub file_name: String,
    pub size: Option<u64>,
    /// A SHA-256 the release index itself gives (Fedora, Arch, Pop!_OS); must agree with the checksum file.
    pub index_sha256: Option<String>,
    pub checksums_url: Option<String>,
    pub signature_url: Option<String>,
    /// checksums or image
    pub signature_of: String,
    /// detached or clearsigned
    pub signature_kind: String,
    /// The key's name in the catalog.
    pub key: String,
    /// Arch: the fingerprint its release index names as the signer (must be one of the key's).
    pub index_signer: Option<String>,
    /// The checksum file as fetched (it's checked against its signature before it's believed).
    #[serde(skip)]
    pub checksums_body: Option<Vec<u8>>,
}

pub type Get<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

pub fn load() -> Catalog { serde_json::from_str(CATALOG_JSON).expect("usb/catalog.json is valid (catalog tests)") }

impl Catalog {
    pub fn entry(&self, id: &str) -> Option<&Entry> { self.entries.iter().find(|e| e.id == id) }
}

/// Checks the catalog: unique ids, known groups and keys, https everywhere, regexes that compile, a way to check
/// every download. Returns every problem found.
pub fn validate(c: &Catalog) -> Vec<String> {
    let mut bad = vec![];
    let mut ids = std::collections::HashSet::new();
    for (name, k) in &c.keys {
        if k.fingerprints.is_empty() { bad.push(format!("key {name}: no fingerprints")); }
        for f in &k.fingerprints {
            if f.len() != 40 || !f.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()) { bad.push(format!("key {name}: fingerprint {f} isn't 40 upper-case hex digits")); }
        }
        if super::keys::text(&k.file).is_none() { bad.push(format!("key {name}: usb/keys/{} isn't built in (src/usb/keys.rs)", k.file)); }
        if k.source.is_empty() { bad.push(format!("key {name}: no source")); }
    }
    for e in &c.entries {
        if !ids.insert(e.id.clone()) { bad.push(format!("{}: id used twice", e.id)); }
        if !e.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') { bad.push(format!("{}: ids are lower-case letters, digits and -", e.id)); }
        if !c.groups.iter().any(|g| g.id == e.group) { bad.push(format!("{}: unknown group {}", e.id, e.group)); }
        if e.name.is_empty() || e.family.is_empty() { bad.push(format!("{}: needs a name and a family", e.id)); }
        if !e.homepage.starts_with("https://") { bad.push(format!("{}: homepage must be https", e.id)); }
        if !matches!(e.write.as_str(), "" | "raw" | "windows") { bad.push(format!("{}: write is raw or windows", e.id)); }
        let mut urls: Vec<&str> = vec![];
        let mut res: Vec<&str> = vec![];
        let windows = matches!(e.resolve, Resolve::MicrosoftPage { .. });
        match &e.resolve {
            Resolve::Fedora { index, subvariant } => { urls.push(index); if subvariant.is_empty() { bad.push(format!("{}: subvariant", e.id)); } }
            Resolve::Arch { index, mirror } => { urls.push(index); urls.push(mirror); }
            Resolve::Pop { index } => urls.push(index),
            Resolve::DirIndex { index, dir, image, .. } => { urls.push(index); res.push(dir); if !image.is_empty() { res.push(image); } if !index.ends_with('/') { bad.push(format!("{}: index must end in /", e.id)); } }
            Resolve::Dir { dir } => { urls.push(dir); if !dir.ends_with('/') { bad.push(format!("{}: dir must end in /", e.id)); } }
            Resolve::Single { url, .. } => urls.push(url),
            Resolve::Page { page, pick } => { urls.push(page); res.push(pick); }
            Resolve::MicrosoftPage { page, product } => {
                urls.push(page);
                if !page.starts_with("https://www.microsoft.com/") { bad.push(format!("{}: Windows comes only from www.microsoft.com", e.id)); }
                if !matches!(product.as_str(), "windows11" | "windows10") { bad.push(format!("{}: unknown Windows product", e.id)); }
                if e.write != "windows" { bad.push(format!("{}: Windows is written in windows mode", e.id)); }
            }
        }
        for u in &urls { if !u.starts_with("https://") { bad.push(format!("{}: {u} must be https", e.id)); } }
        if let Some(cs) = &e.checksums {
            if cs.file.is_empty() == cs.listing.is_empty() { bad.push(format!("{}: checksums need a file or a listing (one)", e.id)); }
            if !cs.file.is_empty() && !(cs.file.starts_with("{url}") || cs.file.starts_with("{dir}") || cs.file.starts_with("https://")) { bad.push(format!("{}: checksums.file must start with {{url}}, {{dir}} or https://", e.id)); }
            if !cs.listing.is_empty() { res.push(&cs.listing); }
            if !cs.pick.is_empty() { res.push(&cs.pick); }
        }
        let needs_pick = matches!(&e.resolve, Resolve::Dir { .. }) || matches!(&e.resolve, Resolve::DirIndex { image, .. } if image.is_empty());
        if needs_pick && e.checksums.as_ref().map(|c| c.pick.is_empty()).unwrap_or(true) { bad.push(format!("{}: needs checksums.pick to find the image", e.id)); }
        for r in res { if let Err(err) = Regex::new(r) { bad.push(format!("{}: regex {r}: {err}", e.id)); } }
        if let Some(s) = &e.signature {
            if !matches!(s.kind.as_str(), "detached" | "clearsigned") { bad.push(format!("{}: signature.kind is detached or clearsigned", e.id)); }
            if !matches!(s.of.as_str(), "checksums" | "image") { bad.push(format!("{}: signature.of is checksums or image", e.id)); }
            if s.kind == "detached" && s.file.is_empty() { bad.push(format!("{}: a detached signature needs its file", e.id)); }
            if s.kind == "clearsigned" && s.of != "checksums" { bad.push(format!("{}: only a checksum file can be clearsigned", e.id)); }
            if s.of == "checksums" && e.checksums.is_none() { bad.push(format!("{}: signed checksums, but no checksum file", e.id)); }
            if !s.file.is_empty() && !(s.file.starts_with("{url}") || s.file.starts_with("{dir}") || s.file.starts_with("https://")) { bad.push(format!("{}: signature.file must start with {{url}}, {{dir}} or https://", e.id)); }
            if !c.keys.contains_key(&s.key) { bad.push(format!("{}: unknown key {}", e.id, s.key)); }
        }
        // Every download is checked against something from the project: a checksum file, or the index's own SHA-256.
        let has_index_sum = matches!(e.resolve, Resolve::Pop { .. } | Resolve::Arch { .. } | Resolve::Fedora { .. });
        if !windows && e.checksums.is_none() && !has_index_sum { bad.push(format!("{}: no way to check the download", e.id)); }
        if !windows && e.signature.is_none() && e.note.is_empty() { bad.push(format!("{}: unsigned: say so in note", e.id)); }
        if matches!(e.resolve, Resolve::Single { follow_name: false, .. } | Resolve::Pop { .. }) && e.pinned.is_empty() { bad.push(format!("{}: a fixed address: say what's pinned in pinned", e.id)); }
    }
    bad
}

/// Fills a template: {url} and {dir}.
pub fn fill(t: &str, url: &str) -> String {
    let dir = match url.rfind('/') { Some(i) => &url[..=i], None => url };
    t.replace("{url}", url).replace("{dir}", dir)
}

/// The links in an HTML folder listing (href="..."), without query strings.
pub fn hrefs(html: &str) -> Vec<String> {
    let re = Regex::new(r#"(?i)href\s*=\s*["']([^"'?#]+)"#).unwrap();
    re.captures_iter(html).map(|c| c[1].to_string()).collect()
}

/// Version order: numbers compared as numbers ("26.04.1" > "26.04" > "24.04.5", "22.10" > "22.9").
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |s: &str| -> Vec<u64> { s.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).map(|p| p.parse().unwrap_or(0)).collect() };
    parts(a).cmp(&parts(b))
}

/// (file name, sha256) pairs from a checksum file: "HEX  name", "HEX *name", "SHA256 (name) = HEX", or a lone HEX.
/// PGP armour (a clearsigned file's header and signature) is skipped.
pub fn parse_sums(text: &str) -> Vec<(String, String)> {
    let bsd = Regex::new(r"^SHA256 \((.+)\) = ([0-9A-Fa-f]{64})$").unwrap();
    let gnu = Regex::new(r"^([0-9A-Fa-f]{64})\s+\*?(.+)$").unwrap();
    let lone = Regex::new(r"^([0-9A-Fa-f]{64})$").unwrap();
    let mut out = vec![];
    let mut in_sig = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("-----BEGIN PGP SIGNATURE") { in_sig = true; continue; }
        if l.starts_with("-----END PGP SIGNATURE") { in_sig = false; continue; }
        if in_sig || l.is_empty() || l.starts_with('#') || l.starts_with("-----") || l.starts_with("Hash:") { continue; }
        if let Some(c) = bsd.captures(l) { out.push((c[1].to_string(), c[2].to_lowercase())); }
        else if let Some(c) = gnu.captures(l) { out.push((c[2].trim().trim_start_matches("./").to_string(), c[1].to_lowercase())); }
        else if let Some(c) = lone.captures(l) { out.push((String::new(), c[1].to_lowercase())); }
    }
    out
}

/// The SHA-256 for `name` in a checksum file (a lone checksum counts for any name).
pub fn sum_for(sums: &[(String, String)], name: &str) -> Option<String> {
    sums.iter().find(|(n, _)| n == name).or_else(|| if sums.len() == 1 && sums[0].0.is_empty() { sums.first() } else { None }).map(|(_, h)| h.clone())
}

/// The newest file in a checksum file matching `pick` (its first group is the version): (name, version).
pub fn pick_in_sums(sums: &[(String, String)], pick: &str) -> Option<(String, String)> {
    let re = Regex::new(pick).ok()?;
    sums.iter().filter_map(|(n, _)| re.captures(n).map(|c| (n.clone(), c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())))
        .max_by(|a, b| version_cmp(&a.1, &b.1))
}

fn text(b: Vec<u8>) -> String { String::from_utf8_lossy(&b).into_owned() }

fn base(r: &mut Release, e: &Entry) {
    r.id = e.id.clone();
    r.name = if e.flavour.is_empty() { e.name.clone() } else { format!("{} ({})", e.name, e.flavour) };
    if let Some(s) = &e.signature {
        r.signature_kind = s.kind.clone();
        r.signature_of = s.of.clone();
        r.key = s.key.clone();
    }
}

fn file_name(url: &str) -> String { url.rsplit('/').next().unwrap_or("").to_string() }

/// Finds the newest release of an entry. `get` fetches a page (https; text or JSON).
pub fn resolve(e: &Entry, get: Get) -> Result<Release, String> {
    let mut r = Release::default();
    base(&mut r, e);
    match &e.resolve {
        Resolve::MicrosoftPage { .. } => return Err("Windows comes from Microsoft's own page, opened in your browser.".into()),
        Resolve::Fedora { index, subvariant } => {
            let v: Value = serde_json::from_slice(&get(index)?).map_err(|e| format!("Fedora's release list didn't make sense: {e}"))?;
            let best = v.as_array().into_iter().flatten()
                .filter(|x| x["subvariant"] == subvariant.as_str() && x["arch"] == "x86_64")
                .filter(|x| x["link"].as_str().map(|l| l.ends_with(".iso") && l.starts_with("https://")).unwrap_or(false))
                .filter(|x| x["version"].as_str().map(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit())).unwrap_or(false))
                .max_by(|a, b| version_cmp(a["version"].as_str().unwrap_or(""), b["version"].as_str().unwrap_or("")))
                .ok_or(format!("Fedora's release list has no {subvariant} ISO"))?;
            r.url = best["link"].as_str().unwrap_or_default().to_string();
            r.version = best["version"].as_str().unwrap_or_default().to_string();
            r.size = best["size"].as_str().and_then(|s| s.parse().ok()).or(best["size"].as_u64());
            r.index_sha256 = best["sha256"].as_str().map(|s| s.to_lowercase());
        }
        Resolve::Arch { index, mirror } => {
            let v: Value = serde_json::from_slice(&get(index)?).map_err(|e| format!("Arch's release list didn't make sense: {e}"))?;
            let rel = v["releases"].as_array().into_iter().flatten().find(|x| x["available"] == true).ok_or("Arch's release list has no available release")?;
            let path = rel["iso_url"].as_str().ok_or("Arch's release has no ISO")?;
            if !path.starts_with("/iso/") || path.contains("..") { return Err("Arch's release list gave an odd address".into()); }
            r.url = format!("{}{}", mirror.trim_end_matches('/'), path);
            r.version = rel["version"].as_str().unwrap_or_default().to_string();
            r.size = rel["torrent"]["file_length"].as_u64();
            r.index_sha256 = rel["sha256_sum"].as_str().map(|s| s.to_lowercase());
            r.index_signer = rel["pgp_fingerprint"].as_str().map(|s| s.to_uppercase());
        }
        Resolve::Pop { index } => {
            let v: Value = serde_json::from_slice(&get(index)?).map_err(|e| format!("System76's build service didn't make sense: {e}"))?;
            let url = v["url"].as_str().ok_or("System76's build service gave no address")?;
            if !url.starts_with("https://iso.pop-os.org/") { return Err("System76's build service gave an address that isn't iso.pop-os.org".into()); }
            r.url = url.to_string();
            r.version = format!("{} (build {})", v["version"].as_str().unwrap_or(""), v["build"].as_str().map(str::to_string).unwrap_or_else(|| v["build"].to_string()));
            r.size = v["size"].as_u64().or_else(|| v["size"].as_str().and_then(|s| s.parse().ok()));
            r.index_sha256 = v["sha_sum"].as_str().map(|s| s.to_lowercase());
        }
        Resolve::DirIndex { index, dir, lts, sub, image } => {
            let html = text(get(index)?);
            let re = Regex::new(dir).map_err(|e| e.to_string())?;
            let mut dirs: Vec<(String, String)> = hrefs(&html).into_iter().filter_map(|h| {
                let h = h.trim_start_matches(index.as_str()).to_string();
                re.captures(&h).map(|c| (h.clone(), c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()))
            }).collect();
            if *lts { dirs.retain(|(_, y)| y.parse::<u32>().map(|y| y % 2 == 0).unwrap_or(false)); }
            let (best, _) = dirs.into_iter().max_by(|a, b| version_cmp(&a.0, &b.0)).ok_or("The release folder lists no version")?;
            let folder = format!("{index}{best}{sub}");
            if !image.is_empty() {
                let listing = text(get(&folder)?);
                let ire = Regex::new(image).map_err(|e| e.to_string())?;
                let (name, ver) = hrefs(&listing).into_iter().filter_map(|h| { let n = file_name(&h); ire.captures(&n).map(|c| (n.clone(), c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())) })
                    .max_by(|a, b| version_cmp(&a.1, &b.1)).ok_or("The release folder has no image")?;
                r.url = format!("{folder}{name}");
                r.version = ver;
            } else {
                r.url = folder.clone();   // the file name comes from the checksum file below
            }
        }
        Resolve::Dir { dir } => { r.url = dir.clone(); }
        Resolve::Single { url, follow_name } => {
            r.url = url.clone();
            if *follow_name {
                let cs = e.checksums.as_ref().ok_or("followName needs a checksum file")?;
                let body = get(&fill(&cs.file, url))?;
                let sums = parse_sums(&text(body));
                let real = sums.iter().map(|(n, _)| n.clone()).find(|n| !n.is_empty() && !n.contains('/')).ok_or("The checksum file names no file")?;
                r.url = fill("{dir}", url) + &real;
            }
        }
        Resolve::Page { page, pick } => {
            let html = text(get(page)?);
            let re = Regex::new(pick).map_err(|e| e.to_string())?;
            let (url, ver) = re.captures_iter(&html).map(|c| (c[0].to_string(), c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()))
                .max_by(|a, b| version_cmp(&a.1, &b.1)).ok_or("The download page lists no image")?;
            r.url = url;
            r.version = ver;
        }
    }
    // The checksum file: found, fetched, and (for a folder) the image picked from it.
    if let Some(cs) = &e.checksums {
        let cs_url = if !cs.listing.is_empty() {
            let folder = fill("{dir}", &r.url);
            let listing = text(get(&folder)?);
            let lre = Regex::new(&cs.listing).map_err(|e| e.to_string())?;
            let name = hrefs(&listing).into_iter().map(|h| file_name(&h)).find(|n| lre.is_match(n)).ok_or("The image's folder has no checksum file")?;
            format!("{folder}{name}")
        } else { fill(&cs.file, &r.url) };
        let body = get(&cs_url)?;
        if r.url.ends_with('/') {
            let sums = parse_sums(&text(body.clone()));
            let (name, ver) = pick_in_sums(&sums, &cs.pick).ok_or("The checksum file doesn't list the image")?;
            r.url = format!("{}{name}", r.url);
            if r.version.is_empty() { r.version = ver; }
        }
        r.checksums_url = Some(cs_url);
        r.checksums_body = Some(body);
    }
    if r.url.ends_with('/') || r.url.is_empty() { return Err("No image was found".into()); }
    r.file_name = file_name(&r.url);
    if r.version.is_empty() {
        r.version = match &e.resolve { Resolve::Single { .. } => "newest".into(), _ => String::new() };
    }
    if let Some(s) = &e.signature { if !s.file.is_empty() { r.signature_url = Some(fill(&s.file, &r.url)); } }
    if r.file_name.is_empty() || r.file_name.contains("..") || r.file_name.starts_with('.') { return Err("The image's file name is odd; it isn't used".into()); }
    Ok(r)
}

/// What the checksum file (already checked against its signature when it has one) says the image's SHA-256 is,
/// agreeing with the index's own when it gives one.
pub fn expected_sha256(r: &Release, verified_checksums: Option<&str>) -> Result<String, String> {
    let from_file = match verified_checksums {
        Some(t) => Some(sum_for(&parse_sums(t), &r.file_name).ok_or_else(|| format!("The checksum file doesn't list {}", r.file_name))?),
        None => None,
    };
    match (from_file, &r.index_sha256) {
        (Some(a), Some(b)) if a != *b => Err("The release index and the checksum file disagree about the SHA-256. Nothing is written.".into()),
        (Some(a), _) => Ok(a),
        (None, Some(b)) => Ok(b.clone()),
        (None, None) => Err("There's nothing to check this download against.".into()),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_valid() {
        let c = load();
        let bad = validate(&c);
        assert!(bad.is_empty(), "usb/catalog.json:\n{}", bad.join("\n"));
        // What the owner asked for is there.
        for id in ["fedora-workstation", "fedora-kde", "ubuntu-lts", "kubuntu-lts", "debian-netinst", "debian-live-kde", "mint-cinnamon", "popos-intel",
                   "arch", "cachyos", "bazzite", "opensuse-tumbleweed", "manjaro-kde", "windows-11", "hearth-live"] {
            assert!(c.entry(id).is_some(), "{id} is missing");
        }
    }

    /// The Recovery USB is the way back to a Hearth device that won't start, so it gets a test of its own: the
    /// entry must stay in the catalog, right after Hearth Peppermint, at its fixed address, and check out offline.
    /// A merge once dropped it silently (the ISO and the docs stayed; only this entry went), which is what this pins.
    #[test]
    fn the_recovery_usb_stays_in_the_catalog() {
        let c = load();
        let ids: Vec<&str> = c.entries.iter().map(|e| e.id.as_str()).collect();
        let i = ids.iter().position(|id| *id == "hearth-recovery").expect("the Hearth Recovery USB entry is gone from the catalog");
        let pepper = ids.iter().position(|id| *id == "hearth-peppermint").expect("hearth-peppermint is gone from the catalog");
        assert_eq!(i, pepper + 1, "the Recovery USB comes right after Hearth Peppermint (another Hearth image between them means this line needs updating)");
        let e = &c.entries[i];
        assert_eq!((e.group.as_str(), e.homepage.as_str()), ("hearth", "https://hearthos.fun/docs/troubleshooting.html#recovery-usb"));
        match &e.resolve {
            Resolve::Single { url, follow_name } => {
                assert_eq!(url, "https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/hearth-recovery.iso", "a recovery image has to keep coming from the same address");
                assert!(!*follow_name, "the address is the image itself, not a moving link to follow");
            }
            other => panic!("the Recovery USB is a fixed address, not {other:?}"),
        }
        assert_eq!(e.checksums.as_ref().map(|cs| cs.file.as_str()), Some("{url}.sha256"), "its SHA-256 is published next to it");
        assert!(e.signature.is_none() && !e.note.is_empty(), "it's unsigned, and the note says so (validate requires one)");
        assert!(!e.pinned.is_empty(), "a fixed address says what's pinned (validate requires it)");
        // It resolves with nothing fetched but its own checksum file, to the file the checksum names.
        let get = canned();
        let r = resolve(e, &get).unwrap_or_else(|err| panic!("hearth-recovery: {err}"));
        assert_eq!((r.url.as_str(), r.file_name.as_str()), ("https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/hearth-recovery.iso", "hearth-recovery.iso"));
        let body = String::from_utf8(r.checksums_body.clone().unwrap()).unwrap();
        assert_eq!(expected_sha256(&r, Some(&body)).unwrap(), "c6fdf8b766fd7ce3d13644e3c4a11338fa4017dc0e340173632b3d1e471b5ed7");
    }

    #[test]
    fn validation_catches_mistakes() {
        let mut c = load();
        c.entries[0].homepage = "http://example.com".into();
        c.entries[1].id = c.entries[2].id.clone();
        if let Some(s) = c.entries[3].signature.as_mut() { s.key = "nobody".into(); }
        c.entries.push(Entry { id: "x".into(), group: "linux".into(), family: "X".into(), name: "X".into(), flavour: "".into(), arch: "x86_64".into(),
            homepage: "https://x".into(), about: "".into(), size_hint: "".into(), resolve: Resolve::Single { url: "http://x/a.iso".into(), follow_name: false },
            checksums: None, signature: None, write: "".into(), pinned: "".into(), note: "".into() });
        let bad = validate(&c).join("\n");
        for want in ["homepage must be https", "id used twice", "unknown key nobody", "x: http://x/a.iso must be https", "x: no way to check", "x: unsigned", "x: a fixed address"] {
            assert!(bad.contains(want), "{want} not caught in:\n{bad}");
        }
    }

    #[test]
    fn checksum_files() {
        let gnu = "601e30fbf5d97759367c632e2c33630665039b7e2158fd068403da3ccf1bda1f *ubuntu-26.04.1-desktop-amd64.iso\n487f87faaf547ea30e0aba4d5b53346292571256b25333a978db1692bcee9dd2 *ubuntu-26.04-desktop-amd64.iso\n";
        let s = parse_sums(gnu);
        assert_eq!(s.len(), 2);
        assert_eq!(pick_in_sums(&s, r"^ubuntu-(\d\d\.04(?:\.\d+)?)-desktop-amd64\.iso$").unwrap(), ("ubuntu-26.04.1-desktop-amd64.iso".into(), "26.04.1".into()));
        let fedora = "-----BEGIN PGP SIGNED MESSAGE-----\nHash: SHA256\n\n# Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso: 3368683520 bytes\nSHA256 (Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso) = C8295961D4C4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n-----BEGIN PGP SIGNATURE-----\n\niQIzBAEBCAAd\n=qpqn\n-----END PGP SIGNATURE-----\n";
        assert_eq!(sum_for(&parse_sums(fedora), "Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso").unwrap(), "c8295961d4c4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let lone = "f710fb32edb6b3ca319a07cd910d2fcaa9c5a088a3093a0d2508daf9dad660c2\n";
        assert!(sum_for(&parse_sums(lone), "anything.iso").is_some());
        let two_spaces = "959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c  cachyos-desktop-linux-260809.iso\n";
        assert_eq!(sum_for(&parse_sums(two_spaces), "cachyos-desktop-linux-260809.iso").unwrap().len(), 64);
        assert!(sum_for(&parse_sums(two_spaces), "other.iso").is_none());
    }

    #[test]
    fn versions() {
        use std::cmp::Ordering::*;
        assert_eq!(version_cmp("26.04.1", "26.04"), Greater);
        assert_eq!(version_cmp("22.10", "22.9"), Greater);
        assert_eq!(version_cmp("260809", "260628"), Greater);
        assert_eq!(version_cmp("44", "45"), Less);
    }

    /// Canned pages, as the projects' servers answered when the catalog was written (October 2026).
    pub fn pages() -> BTreeMap<&'static str, String> {
        let mut m = BTreeMap::new();
        m.insert("https://fedoraproject.org/releases.json", r#"[
          {"version":"45 Beta","arch":"x86_64","link":"https://download.fedoraproject.org/pub/fedora/linux/releases/test/45_Beta/KDE/x86_64/iso/Fedora-KDE-Desktop-Live-45_Beta-1.3.x86_64.iso","variant":"KDE","subvariant":"KDE","sha256":"cd2321a8537667e0017efc3371bac191d7747f834da8fc8c30c88f00a480811e","size":"3584274432"},
          {"version":"44","arch":"x86_64","link":"https://download.fedoraproject.org/pub/fedora/linux/releases/44/KDE/x86_64/iso/Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso","variant":"KDE","subvariant":"KDE","sha256":"C8295961d4c4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":"3368683520"},
          {"version":"43","arch":"x86_64","link":"https://download.fedoraproject.org/pub/fedora/linux/releases/43/KDE/x86_64/iso/Fedora-KDE-Desktop-Live-43-1.6.x86_64.iso","variant":"KDE","subvariant":"KDE","sha256":"181fe3e265fb","size":"3265843200"},
          {"version":"44","arch":"aarch64","link":"https://download.fedoraproject.org/pub/fedora/linux/releases/44/KDE/aarch64/iso/x.iso","variant":"KDE","subvariant":"KDE","sha256":"00","size":"1"}
        ]"#.into());
        m.insert("https://download.fedoraproject.org/pub/fedora/linux/releases/44/KDE/x86_64/iso/",
            r#"<a href="/fedora-buffet/fedora/linux/releases/44/KDE/x86_64/">Parent</a> <a href="Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso">x</a> <a href="Fedora-KDE-iso-44-1.7-x86_64-CHECKSUM">c</a>"#.into());
        m.insert("https://download.fedoraproject.org/pub/fedora/linux/releases/44/KDE/x86_64/iso/Fedora-KDE-iso-44-1.7-x86_64-CHECKSUM",
            "-----BEGIN PGP SIGNED MESSAGE-----\nHash: SHA256\n\nSHA256 (Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso) = c8295961d4c4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n-----BEGIN PGP SIGNATURE-----\nx\n-----END PGP SIGNATURE-----\n".into());
        m.insert("https://releases.ubuntu.com/", r#"<a href="22.04/">22.04/</a><a href="24.04.5/">24.04.5/</a><a href="25.10/">25.10/</a><a href="26.04/">26.04/</a><a href="26.04.1/">26.04.1/</a><a href="26.10/">26.10/</a><a href="?C=M">sort</a>"#.into());
        m.insert("https://releases.ubuntu.com/26.04.1/SHA256SUMS", "601e30fbf5d97759367c632e2c33630665039b7e2158fd068403da3ccf1bda1f *ubuntu-26.04.1-desktop-amd64.iso\n487f87faaf547ea30e0aba4d5b53346292571256b25333a978db1692bcee9dd2 *ubuntu-26.04-desktop-amd64.iso\ncc8a95cde20f6ced61a322420de00f10cc3c90ced545daa46cb9c1a117f1d927 *ubuntu-26.04.1-live-server-amd64.iso\n".into());
        m.insert("https://releases.ubuntu.com/26.10/SHA256SUMS", "733569320adee426accac397f451c26dd0a8521cc09ea89d021072c40a2990fd *ubuntu-26.10-beta-desktop-amd64.iso\n19efc38dde60055b192526bfbba075ac1da23992f1b006ddd41cbd30db0d9b0c *ubuntu-26.10-desktop-amd64.iso\n".into());
        m.insert("https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/SHA256SUMS", "a7ef94ac2fb9a7fec454552abd629b7cc9d5155c886165a45649f5ce6167e355  debian-13.7.0-amd64-netinst.iso\nd33ed0c64bde5ff43445c0c530f9da599954d8585771c55345cb840d7997d4a7  debian-edu-13.7.0-amd64-netinst.iso\n".into());
        m.insert("https://archlinux.org/releng/releases/json/", r#"{"version":1,"releases":[{"version":"2026.11.01","available":false,"iso_url":"/iso/2026.11.01/x.iso"},{"version":"2026.10.01","available":true,"sha256_sum":"684ded26c63240ff4a41e8c25ee84ea6da233f557364821f13d12c2b0a9059a5","pgp_fingerprint":"3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C","iso_url":"/iso/2026.10.01/archlinux-2026.10.01-x86_64.iso","torrent":{"file_length":1640497152}}]}"#.into());
        m.insert("https://geo.mirror.pkgbuild.com/iso/2026.10.01/sha256sums.txt", "684ded26c63240ff4a41e8c25ee84ea6da233f557364821f13d12c2b0a9059a5  archlinux-2026.10.01-x86_64.iso\n684ded26c63240ff4a41e8c25ee84ea6da233f557364821f13d12c2b0a9059a5  archlinux-x86_64.iso\n".into());
        m.insert("https://api.pop-os.org/builds/24.04/intel", r#"{"version":"24.04","url":"https://iso.pop-os.org/24.04/amd64/intel/20/pop-os_24.04_amd64_intel_20.iso","urgent":"true","size":2955067392,"sha_sum":"a0ef3842ab710db4f4407cf3499560b59dddbbcd59bee17beab7b0e99dc22b4c","channel":"intel","build":"20"}"#.into());
        m.insert("https://mirror.cachyos.org/ISO/desktop/", r#"<a href="../">../</a><a href="260426/">260426/</a><a href="260809/">260809/</a><a href="260628/">260628/</a>"#.into());
        m.insert("https://mirror.cachyos.org/ISO/desktop/260809/", r#"<a href="cachyos-desktop-linux-260809.iso">i</a><a href="cachyos-desktop-linux-260809.iso.sha256">s</a><a href="cachyos-desktop-linux-260809.iso.sig">g</a>"#.into());
        m.insert("https://mirror.cachyos.org/ISO/desktop/260809/cachyos-desktop-linux-260809.iso.sha256", "959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c  cachyos-desktop-linux-260809.iso\n".into());
        m.insert("https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Current.iso.sha256", "826a5378103c16a283881a27e8ce249f9034bc733ecadd60a4d6f2ad296299f1  openSUSE-Tumbleweed-DVD-x86_64-Snapshot20261023-Media.iso\n".into());
        m.insert("https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Snapshot20261023-Media.iso.sha256", "826a5378103c16a283881a27e8ce249f9034bc733ecadd60a4d6f2ad296299f1  openSUSE-Tumbleweed-DVD-x86_64-Snapshot20261023-Media.iso\n".into());
        m.insert("https://manjaro.org/products/download/x86", r#"<a href="https://download.manjaro.org/kde/26.0.4/manjaro-kde-26.0.4-260401-linux618.iso">old</a> <a href="https://download.manjaro.org/kde/26.1.0/manjaro-kde-26.1.0-260812-linux71.iso">KDE</a> <a href="https://download.manjaro.org/gnome/26.1.0/manjaro-gnome-26.1.0-260812-linux71.iso">GNOME</a>"#.into());
        m.insert("https://download.manjaro.org/kde/26.1.0/manjaro-kde-26.1.0-260812-linux71.iso.sha256", "711b64362abe0b45348dc33bd4a2c654641e069b50ae9ae37c01771da880aa99  manjaro-kde-26.1.0-260812-linux71.iso\n".into());
        m.insert("https://download.bazzite.gg/bazzite-stable-amd64.iso-CHECKSUM", "9c8d06cd8e57f2274678edeb14b4b13a79b8117c70571a65199919a66305b5c7  bazzite-stable-amd64.iso\n".into());
        // Hearth's own ISOs: a fixed address with the SHA-256 published beside it (as the download server answered).
        m.insert("https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/hearth-recovery.iso.sha256", "c6fdf8b766fd7ce3d13644e3c4a11338fa4017dc0e340173632b3d1e471b5ed7  hearth-recovery.iso\n".into());
        m
    }

    fn canned() -> impl Fn(&str) -> Result<Vec<u8>, String> {
        let p = pages();
        move |u: &str| p.get(u).map(|s| s.as_bytes().to_vec()).ok_or_else(|| format!("no canned page for {u}"))
    }

    fn res(id: &str) -> Release {
        let c = load();
        let get = canned();
        resolve(c.entry(id).unwrap(), &get).unwrap_or_else(|e| panic!("{id}: {e}"))
    }

    #[test]
    fn finds_fedora() {
        let r = res("fedora-kde");
        assert_eq!(r.version, "44", "the beta isn't offered");
        assert_eq!(r.file_name, "Fedora-KDE-Desktop-Live-44-1.7.x86_64.iso");
        assert_eq!(r.checksums_url.as_deref(), Some("https://download.fedoraproject.org/pub/fedora/linux/releases/44/KDE/x86_64/iso/Fedora-KDE-iso-44-1.7-x86_64-CHECKSUM"));
        assert_eq!(r.signature_kind, "clearsigned");
        let body = String::from_utf8(r.checksums_body.clone().unwrap()).unwrap();
        assert_eq!(expected_sha256(&r, Some(&body)).unwrap(), "c8295961d4c4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        // The index and the checksum file must agree.
        let mut wrong = r.clone();
        wrong.index_sha256 = Some("00".repeat(32));
        assert!(expected_sha256(&wrong, Some(&body)).is_err());
    }

    #[test]
    fn finds_ubuntu_lts_and_newest() {
        let r = res("ubuntu-lts");
        assert_eq!(r.url, "https://releases.ubuntu.com/26.04.1/ubuntu-26.04.1-desktop-amd64.iso");
        assert_eq!(r.version, "26.04.1");
        assert_eq!(r.signature_url.as_deref(), Some("https://releases.ubuntu.com/26.04.1/SHA256SUMS.gpg"));
        let r = res("ubuntu-latest");
        assert_eq!(r.url, "https://releases.ubuntu.com/26.10/ubuntu-26.10-desktop-amd64.iso", "not the beta");
    }

    #[test]
    fn finds_debian_arch_pop_cachy_suse_manjaro_bazzite() {
        let r = res("debian-netinst");
        assert_eq!((r.url.as_str(), r.version.as_str()), ("https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-13.7.0-amd64-netinst.iso", "13.7.0"));
        assert_eq!(r.signature_url.as_deref(), Some("https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/SHA256SUMS.sign"));
        let r = res("arch");
        assert_eq!(r.url, "https://geo.mirror.pkgbuild.com/iso/2026.10.01/archlinux-2026.10.01-x86_64.iso");
        assert_eq!(r.signature_url.as_deref(), Some("https://geo.mirror.pkgbuild.com/iso/2026.10.01/archlinux-2026.10.01-x86_64.iso.sig"));
        assert_eq!(r.signature_of, "image");
        assert_eq!(r.index_signer.as_deref(), Some("3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C"));
        let body = String::from_utf8(r.checksums_body.clone().unwrap()).unwrap();
        assert_eq!(expected_sha256(&r, Some(&body)).unwrap(), "684ded26c63240ff4a41e8c25ee84ea6da233f557364821f13d12c2b0a9059a5");
        let r = res("popos-intel");
        assert_eq!(r.file_name, "pop-os_24.04_amd64_intel_20.iso");
        assert_eq!(r.version, "24.04 (build 20)");
        assert_eq!(expected_sha256(&r, None).unwrap(), "a0ef3842ab710db4f4407cf3499560b59dddbbcd59bee17beab7b0e99dc22b4c");
        let r = res("cachyos");
        assert_eq!(r.url, "https://mirror.cachyos.org/ISO/desktop/260809/cachyos-desktop-linux-260809.iso");
        assert_eq!(r.signature_url.as_deref(), Some("https://mirror.cachyos.org/ISO/desktop/260809/cachyos-desktop-linux-260809.iso.sig"));
        let r = res("opensuse-tumbleweed");
        assert_eq!(r.url, "https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Snapshot20261023-Media.iso", "the snapshot, not the moving link");
        assert_eq!(r.signature_url.as_deref(), Some("https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Snapshot20261023-Media.iso.sha256.asc"));
        let r = res("manjaro-kde");
        assert_eq!(r.version, "26.1.0");
        assert_eq!(r.file_name, "manjaro-kde-26.1.0-260812-linux71.iso");
        let r = res("bazzite");
        assert_eq!(r.version, "newest");
        let body = String::from_utf8(r.checksums_body.clone().unwrap()).unwrap();
        assert!(expected_sha256(&r, Some(&body)).is_ok());
    }

    #[test]
    fn odd_answers_are_refused() {
        let c = load();
        let pop = |body: &'static str| move |_: &str| -> Result<Vec<u8>, String> { Ok(body.as_bytes().to_vec()) };
        let g = pop(r#"{"url":"https://evil.example/pop.iso","sha_sum":"00"}"#);
        assert!(resolve(c.entry("popos-intel").unwrap(), &g).is_err());
        let g = pop(r#"{"releases":[{"available":true,"iso_url":"/iso/../../x.iso"}]}"#);
        assert!(resolve(c.entry("arch").unwrap(), &g).is_err());
        assert!(resolve(c.entry("windows-11").unwrap(), &g).is_err());
    }
}
