//! Places (1.14): the small map on a photo's page and the Places map, from OpenStreetMap's map pictures, and a
//! place's name from OpenStreetMap's Nominatim. Only the spot being looked at is asked for (never a list of your
//! photos), with Hearth Photos' name, one name a second at most, and both are kept here so a place seen once shows
//! again offline. Your photos themselves never leave the PC.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const UA: &str = "HearthPhotos/1.14 (Hearth OS; https://hearthos.fun)";
const TILES: &str = "https://tile.openstreetmap.org";
const NOMINATIM: &str = "https://nominatim.openstreetmap.org";

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().user_agent(UA).timeout_connect(Duration::from_secs(8)).timeout(Duration::from_secs(20)).build()
}

fn tile_file(z: u32, x: u32, y: u32) -> PathBuf { crate::imaging::cache_dir("tiles").join(format!("{z}-{x}-{y}.png")) }

/// One map picture (256 px PNG): the copy here when it's under 30 days old, else fetched (the old copy when offline).
pub fn tile(z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
    if z > 19 || x >= (1 << z) || y >= (1 << z) { return Err("No such map piece".into()); }
    let f = tile_file(z, x, y);
    let fresh = std::fs::metadata(&f).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|a| a < Duration::from_secs(30 * 86400)).unwrap_or(false);
    if fresh { if let Ok(b) = std::fs::read(&f) { return Ok(b); } }
    let got = agent().get(&format!("{TILES}/{z}/{x}/{y}.png")).call().map_err(|e| e.to_string()).and_then(|r| {
        let mut b = Vec::new();
        std::io::Read::read_to_end(&mut r.into_reader().take(2_000_000), &mut b).map_err(|e| e.to_string())?;
        Ok(b)
    });
    match got {
        Ok(b) if b.starts_with(b"\x89PNG") => {
            let _ = std::fs::create_dir_all(f.parent().unwrap_or(&f));
            let _ = crate::write_atomic(&f, &b);
            Ok(b)
        }
        _ => std::fs::read(&f).map_err(|_| "The map isn't reachable right now".into()),
    }
}

use std::io::Read as _;

static LAST: Mutex<Option<Instant>> = Mutex::new(None);

/// A place's name ("Near the lake", "Lake Travis, Texas"): kept here per spot (about 100 m).
pub fn name(lat: f64, lon: f64) -> Result<String, String> {
    let key = format!("{:.3},{:.3}", lat, lon);
    let file = crate::imaging::cache_dir("places").join(format!("{key}.txt"));
    if let Ok(s) = std::fs::read_to_string(&file) { return Ok(s); }
    {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = *last { let gap = Duration::from_millis(1100); if t.elapsed() < gap { std::thread::sleep(gap - t.elapsed()); } }
        *last = Some(Instant::now());
    }
    let url = format!("{NOMINATIM}/reverse?format=jsonv2&lat={lat:.4}&lon={lon:.4}&zoom=14");
    let v: serde_json::Value = agent().get(&url).call().map_err(|_| "The place's name isn't reachable right now".to_string())?
        .into_json().map_err(|e| e.to_string())?;
    let n = describe(&v);
    if !n.is_empty() {
        let _ = std::fs::create_dir_all(file.parent().unwrap_or(&file));
        let _ = crate::write_atomic(&file, n.as_bytes());
    }
    Ok(n)
}

/// "Lake Travis · Austin, Texas": the nearest named thing, then the town and state.
pub fn describe(v: &serde_json::Value) -> String {
    let a = &v["address"];
    let get = |k: &str| a[k].as_str().map(str::to_string);
    let near = v["name"].as_str().filter(|s| !s.is_empty()).map(str::to_string)
        .or_else(|| get("water")).or_else(|| get("park")).or_else(|| get("neighbourhood")).or_else(|| get("suburb"));
    let town = get("city").or_else(|| get("town")).or_else(|| get("village")).or_else(|| get("hamlet")).or_else(|| get("county"));
    let state = get("state").or_else(|| get("country"));
    let tail: Vec<String> = [town, state].into_iter().flatten().collect();
    match (near, tail.is_empty()) {
        (Some(n), false) if !tail.contains(&n) => format!("{n} · {}", tail.join(", ")),
        (_, false) => tail.join(", "),
        (Some(n), true) => n,
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_places() {
        let v = serde_json::json!({"name": "Lake Travis", "address": {"city": "Austin", "state": "Texas", "country": "United States"}});
        assert_eq!(super::describe(&v), "Lake Travis · Austin, Texas");
        let v = serde_json::json!({"name": "", "address": {"village": "Hope", "state": "Arkansas"}});
        assert_eq!(super::describe(&v), "Hope, Arkansas");
        assert!(super::tile(30, 0, 0).is_err());
    }
}
