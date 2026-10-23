//! Ludusavi's manifest: the open, community-kept list of where games keep their saves
//! (https://github.com/mtkennerly/ludusavi-manifest). Downloaded from GitHub when it's missing or a week old (no
//! account, nothing sent), and cached as compact JSON with only what Saves uses, so reading it again is quick.
//! $HEARTH_SAVES_MANIFEST points at a local copy instead (YAML or JSON), for tests.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub const URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";
/// Looked for again after this long.
const MAX_AGE_MS: u64 = 7 * 24 * 3600 * 1000;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct When {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct FileRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<When>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct IdOnly { #[serde(default)] id: Option<u64> }

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct MoreIds {
    #[serde(default, rename = "steamExtra", skip_serializing_if = "Vec::is_empty")]
    steam_extra: Vec<u64>,
    #[serde(default, rename = "gogExtra", skip_serializing_if = "Vec::is_empty")]
    gog_extra: Vec<u64>,
}

/// One game as the manifest has it (only the parts Saves uses; the rest is skipped while reading).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Entry {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, Option<FileRule>>,
    #[serde(default, rename = "installDir", skip_serializing_if = "BTreeMap::is_empty")]
    install_dir: BTreeMap<String, Nothing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    steam: Option<IdOnly>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gog: Option<IdOnly>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<MoreIds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// A value that's skipped (installDir's are always `{}`); written back as null.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Nothing;
impl<'de> Deserialize<'de> for Nothing {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> { serde::de::IgnoredAny::deserialize(d)?; Ok(Nothing) }
}

impl Entry {
    pub fn install_dirs(&self) -> impl Iterator<Item = &String> { self.install_dir.keys() }
    pub fn steam_ids(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self.steam.as_ref().and_then(|s| s.id).into_iter().collect();
        if let Some(m) = &self.id { v.extend(&m.steam_extra); }
        v
    }
    pub fn gog_ids(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self.gog.as_ref().and_then(|s| s.id).into_iter().collect();
        if let Some(m) = &self.id { v.extend(&m.gog_extra); }
        v
    }
}

/// Lower-case letters and digits only: how titles and folder names are compared ("Baldur's Gate 3" = "baldursgate3").
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

#[derive(Default)]
pub struct Manifest {
    pub games: BTreeMap<String, Entry>,
    by_steam: HashMap<u64, String>,
    by_gog: HashMap<u64, String>,
    by_title: HashMap<String, String>,
    by_dir: HashMap<String, String>,
}

impl Manifest {
    pub fn from_games(games: BTreeMap<String, Entry>) -> Manifest {
        let mut m = Manifest { games, ..Default::default() };
        for (name, e) in &m.games {
            // An alias points at the game that has the data; the alias itself is still findable by its title.
            let target = e.alias.clone().filter(|a| a != name).unwrap_or_else(|| name.clone());
            m.by_title.entry(norm(name)).or_insert_with(|| target.clone());
            if e.alias.is_some() { continue; }
            for id in e.steam_ids() { m.by_steam.entry(id).or_insert_with(|| name.clone()); }
            for id in e.gog_ids() { m.by_gog.entry(id).or_insert_with(|| name.clone()); }
            for d in e.install_dirs() { if !d.contains('<') { m.by_dir.entry(norm(d)).or_insert_with(|| name.clone()); } }
        }
        m
    }

    pub fn len(&self) -> usize { self.games.len() }
    pub fn is_empty(&self) -> bool { self.games.is_empty() }

    /// The manifest's game for one of yours: by Steam or GOG id first, then by title, then by its folder's name.
    pub fn find(&self, steam: Option<u64>, gog: Option<u64>, title: &str, folder: Option<&str>) -> Option<(&str, &Entry)> {
        let name = steam.and_then(|id| self.by_steam.get(&id))
            .or_else(|| gog.and_then(|id| self.by_gog.get(&id)))
            .or_else(|| self.by_title.get(&norm(title)))
            .or_else(|| folder.and_then(|f| self.by_dir.get(&norm(f))))?;
        let (k, e) = self.games.get_key_value(name)?;
        // An alias of an alias isn't followed further.
        if let Some(a) = &e.alias { return self.games.get_key_value(a).map(|(k, e)| (k.as_str(), e)); }
        Some((k.as_str(), e))
    }
}

/// Reads the manifest's YAML, keeping only games that have save files or are another game's alias.
pub fn parse_yaml(text: &str) -> Result<BTreeMap<String, Entry>, String> {
    let mut all: BTreeMap<String, Entry> = serde_yaml::from_str(text).map_err(|e| format!("The save-location list couldn't be read ({e})"))?;
    all.retain(|_, e| !e.files.is_empty() || e.alias.is_some());
    Ok(all)
}

pub fn cache_dir() -> PathBuf { crate::util::xdg_cache().join("hearth").join("saves") }
fn cache_file() -> PathBuf { cache_dir().join("manifest.json") }

/// What `load` tells the page about the list.
#[derive(Debug, Clone, Serialize)]
pub struct Info { pub games: usize, pub fetched: u64, pub source: String, pub note: Option<String> }

/// The manifest: the cached copy while it's fresh, otherwise downloaded again (the old copy is used if that fails).
/// `refresh`: download now even if the copy is fresh.
pub fn load(refresh: bool) -> Result<(Manifest, Info), String> {
    if let Some(p) = std::env::var_os("HEARTH_SAVES_MANIFEST").map(PathBuf::from) {
        let text = std::fs::read_to_string(&p).map_err(|e| format!("Couldn't read {} ({e})", p.display()))?;
        let games = if text.trim_start().starts_with('{') { serde_json::from_str(&text).map_err(|e| e.to_string())? } else { parse_yaml(&text)? };
        let m = Manifest::from_games(games);
        let n = m.len();
        return Ok((m, Info { games: n, fetched: crate::util::now_ms(), source: p.display().to_string(), note: None }));
    }
    let file = cache_file();
    let age = std::fs::metadata(&file).ok().map(|m| crate::util::now_ms().saturating_sub(crate::util::mtime_ms(&m)));
    let mut note = None;
    if refresh || age.map(|a| a > MAX_AGE_MS).unwrap_or(true) {
        match download(&file) {
            Ok(()) => {}
            Err(e) if file.exists() => note = Some(format!("{e} Using the copy from before.")),
            Err(e) => return Err(e),
        }
    }
    let bytes = std::fs::read(&file).map_err(|e| format!("Couldn't read the save-location list ({e})"))?;
    let games: BTreeMap<String, Entry> = match serde_json::from_slice(&bytes) {
        Ok(g) => g,
        Err(_) => { let _ = std::fs::remove_file(&file); return Err("The saved copy of the save-location list was damaged; try again to download it fresh.".into()); }
    };
    let fetched = std::fs::metadata(&file).map(|m| crate::util::mtime_ms(&m)).unwrap_or(0);
    let m = Manifest::from_games(games);
    let n = m.len();
    Ok((m, Info { games: n, fetched, source: URL.into(), note }))
}

/// Downloads the YAML with curl (on every Hearth OS image; nothing to sign in to) and caches it as JSON.
fn download(dest: &Path) -> Result<(), String> {
    let dir = dest.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let yaml = dir.join("manifest.yaml.download");
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "--proto", "=https", "--retry", "2", "--connect-timeout", "20", "--max-time", "300", "-o"]).arg(&yaml).arg(URL)
        .output().map_err(|e| format!("Couldn't download the save-location list (curl: {e}).", ))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&yaml);
        return Err(format!("Couldn't download the save-location list. Is this PC online? ({})", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = std::fs::read_to_string(&yaml).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&yaml);
    let games = parse_yaml(&text?)?;
    if games.len() < 1000 { return Err("The downloaded save-location list looks too short; not using it.".into()); }
    crate::util::write_atomic(dest, &serde_json::to_vec(&games).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    pub const SAMPLE: &str = r#"
Hollow Knight:
  files:
    "<base>/hollow_knight_Data/Config.ini":
      tags: [config]
      when:
        - os: windows
        - os: linux
    "<home>/AppData/LocalLow/Team Cherry/Hollow Knight/*.dat":
      tags: [save]
      when:
        - os: windows
    "<xdgConfig>/unity3d/Team Cherry/Hollow Knight/*.dat":
      tags: [save]
      when:
        - os: linux
  installDir:
    Hollow Knight: {}
  launch:
    "<base>/hollow_knight.exe":
      - when:
          - os: windows
  steam:
    id: 367520
  gog:
    id: 1308320804
HK Alias:
  alias: Hollow Knight
Celeste:
  files:
    "<xdgData>/Celeste/Saves/*.celeste":
      tags: [save]
      when:
        - os: linux
    "<base>/Saves":
      when:
        - os: windows
          store: epic
  installDir:
    Celeste: {}
No Saves Game:
  steam:
    id: 1
Steam Cloud Game:
  files:
    "<root>/userdata/<storeUserId>/999/remote":
      tags: [save]
      when:
        - store: steam
    "<winDocuments>/My Games/Thing": {}
  steam:
    id: 999
"#;

    #[test]
    fn reads_the_manifest() {
        let g = parse_yaml(SAMPLE).unwrap();
        assert!(!g.contains_key("No Saves Game"), "games without saves are left out");
        let m = Manifest::from_games(g);
        assert_eq!(m.find(Some(367520), None, "x", None).unwrap().0, "Hollow Knight");
        assert_eq!(m.find(None, Some(1308320804), "x", None).unwrap().0, "Hollow Knight");
        assert_eq!(m.find(None, None, "HOLLOW KNIGHT", None).unwrap().0, "Hollow Knight");
        assert_eq!(m.find(None, None, "HK Alias", None).unwrap().0, "Hollow Knight");
        assert_eq!(m.find(None, None, "Something else", Some("Celeste")).unwrap().0, "Celeste");
        assert!(m.find(Some(5), None, "nope", None).is_none());
        let hk = &m.games["Hollow Knight"];
        assert_eq!(hk.files.len(), 3);
        let rule = hk.files["<home>/AppData/LocalLow/Team Cherry/Hollow Knight/*.dat"].clone().unwrap();
        assert_eq!(rule.when, vec![When { os: Some("windows".into()), store: None }]);
        assert!(m.games["Steam Cloud Game"].files["<winDocuments>/My Games/Thing"].is_some());
    }

    #[test]
    fn the_cache_reads_back() {
        let g = parse_yaml(SAMPLE).unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let back: BTreeMap<String, Entry> = serde_json::from_str(&json).unwrap();
        let m = Manifest::from_games(back);
        assert_eq!(m.find(Some(367520), None, "", None).unwrap().1.files.len(), 3);
        assert_eq!(m.find(None, None, "", Some("hollow knight")).unwrap().0, "Hollow Knight");
    }
}
