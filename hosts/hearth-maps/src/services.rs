// The map services Hearth Maps uses, and the rules it keeps for each (docs/HEARTH-MAPS.md has the details):
//   map pieces   OpenFreeMap (tiles.openfreemap.org): free, no key; attribution shown; offline copies allowed up to
//                50 MB / 3,000 tiles with an app-identifying User-Agent and at most 4 requests at once.
//   search       Nominatim (nominatim.openstreetmap.org): at most 1 request a second for the whole app, an
//                identifying User-Agent, results cached, no search-as-you-type, attribution shown.
//   directions   OSRM on FOSSGIS's routing server (routing.openstreetmap.de): at most 1 request a second, an
//                identifying User-Agent, attribution with a "fix the map" link, no heavy use.
// Nominatim's policy asks that an app can be switched off without a software update: Hearth Maps reads a small
// file from Hearth's repository (config/hearth-maps-services.json, at most once a day) that can turn any of the
// three off or point it elsewhere. ~/.config/hearth-maps/services.json can do the same on one computer (for
// example to use a self-hosted Nominatim or OSRM).
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub const USER_AGENT: &str = concat!("HearthMaps/", env!("CARGO_PKG_VERSION"), " (Hearth OS; +https://github.com/Dragongamer9/hearth-os)");
pub const REMOTE: &str = "https://raw.githubusercontent.com/Dragongamer9/hearth-os/main/config/hearth-maps-services.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Services {
    pub tiles: String,
    pub tiles_enabled: bool,
    pub styles: Vec<Style>,
    pub nominatim: String,
    pub search_enabled: bool,
    pub routing: String,
    pub routing_enabled: bool,
    /// A message to show (for example why something is switched off).
    pub notice: String,
    /// How much room the copies of map pieces may take, in MB.
    pub cache_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Style {
    pub id: String,
    pub name: String,
    pub dark: bool,
}

impl Default for Style {
    fn default() -> Self { Style { id: String::new(), name: String::new(), dark: false } }
}

impl Default for Services {
    fn default() -> Self {
        let st = |id: &str, name: &str, dark: bool| Style { id: id.into(), name: name.into(), dark };
        Services {
            tiles: "https://tiles.openfreemap.org".into(),
            tiles_enabled: true,
            styles: vec![st("liberty", "Liberty", false), st("bright", "Bright", false), st("positron", "Positron", false), st("dark", "Dark", true), st("fiord", "Fiord", true)],
            nominatim: "https://nominatim.openstreetmap.org".into(),
            search_enabled: true,
            routing: "https://routing.openstreetmap.de".into(),
            routing_enabled: true,
            notice: String::new(),
            cache_mb: 50,
        }
    }
}

fn https(s: &str) -> bool { s.starts_with("https://") && url::Url::parse(s).is_ok_and(|u| u.host_str().is_some()) }

impl Services {
    /// Lays an override (the remote switch file, or the local one) over these. Addresses must be https://.
    pub fn apply(&mut self, o: &serde_json::Value) {
        let s = |k: &str| o.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let b = |k: &str| o.get(k).and_then(|v| v.as_bool());
        if let Some(v) = s("tiles").filter(|v| https(v)) { self.tiles = v.trim_end_matches('/').into(); }
        if let Some(v) = s("nominatim").filter(|v| https(v)) { self.nominatim = v.trim_end_matches('/').into(); }
        if let Some(v) = s("routing").filter(|v| https(v)) { self.routing = v.trim_end_matches('/').into(); }
        if let Some(v) = b("tilesEnabled") { self.tiles_enabled = v; }
        if let Some(v) = b("searchEnabled") { self.search_enabled = v; }
        if let Some(v) = b("routingEnabled") { self.routing_enabled = v; }
        if let Some(v) = s("notice") { self.notice = v; }
        if let Some(v) = o.get("cacheMb").and_then(|v| v.as_u64()) { self.cache_mb = v.clamp(0, 50); }
        if let Some(v) = o.get("styles").and_then(|v| serde_json::from_value::<Vec<Style>>(v.clone()).ok()).filter(|v| !v.is_empty()) {
            self.styles = v.into_iter().filter(|s| !s.id.is_empty() && s.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).collect();
        }
    }

    pub fn tiles_host(&self) -> String { url::Url::parse(&self.tiles).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default() }
}

pub fn cache_dir() -> PathBuf {
    std::env::var_os("HEARTH_MAPS_CACHE").map(PathBuf::from).unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("hearth-maps"))
}
pub fn data_dir() -> PathBuf {
    std::env::var_os("HEARTH_MAPS_DATA").map(PathBuf::from).unwrap_or_else(|| dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("hearth-maps"))
}
fn config_file() -> PathBuf { dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("hearth-maps").join("services.json") }

pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().user_agent(USER_AGENT).timeout_connect(Duration::from_secs(10)).timeout(Duration::from_secs(30)).build()
}

/// The services as they are now: the built-in ones, then Hearth's switch file (fetched again when a day old; the
/// last copy is used when offline), then this computer's own file.
pub fn load() -> Services {
    let mut s = Services::default();
    let cached = cache_dir().join("services.json");
    let fresh = std::fs::metadata(&cached).and_then(|m| m.modified()).ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok()).is_some_and(|age| age < Duration::from_secs(86400));
    if !fresh && std::env::var_os("HEARTH_MAPS_NO_REMOTE").is_none() {
        if let Ok(r) = agent().get(REMOTE).timeout(Duration::from_secs(5)).call() {
            if let Ok(text) = r.into_string() {
                if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                    let _ = std::fs::create_dir_all(cache_dir());
                    let _ = std::fs::write(&cached, &text);
                }
            }
        }
    }
    for f in [cached, config_file()] {
        if let Some(v) = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) { s.apply(&v); }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_switch_turns_things_off_and_only_takes_https() {
        let mut s = Services::default();
        s.apply(&json!({"searchEnabled": false, "notice": "Search is off for now", "routing": "http://evil.example", "nominatim": "https://nominatim.example.org/", "cacheMb": 999}));
        assert!(!s.search_enabled);
        assert!(s.tiles_enabled && s.routing_enabled);
        assert_eq!(s.routing, "https://routing.openstreetmap.de");
        assert_eq!(s.nominatim, "https://nominatim.example.org");
        assert_eq!(s.cache_mb, 50, "never more than OpenFreeMap's 50 MB for offline copies");
        assert_eq!(s.notice, "Search is off for now");
        assert_eq!(s.tiles_host(), "tiles.openfreemap.org");
    }

    #[test]
    fn the_repository_switch_file_is_valid() {
        let t = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/hearth-maps-services.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&t).unwrap();
        let mut s = Services::default();
        s.apply(&v);
        assert!(s.tiles_enabled && s.search_enabled && s.routing_enabled, "the switch file in the repository turns something off");
    }
}
