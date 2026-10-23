// Place search and "what's here" with Nominatim, kept to its usage policy: one request a second at most for the
// whole app (a shared clock; the page also only searches when Enter is pressed, never while typing), Hearth Maps'
// own User-Agent, and answers cached on disk for 30 days so the same search isn't sent twice. Directions (OSRM on
// FOSSGIS's server) share the same one-a-second clock per service.
use crate::services::{agent, cache_dir, Services};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
    /// [south, north, west, east], as Nominatim gives it.
    pub bbox: Option<[f64; 4]>,
    pub kind: String,
    pub osm: String,
}

/// One request a second, per service (keyed by its address).
static LAST: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

pub fn wait_turn(service: &str) {
    let gap = Duration::from_millis(1100);
    loop {
        let wait = {
            let mut g = LAST.lock().unwrap_or_else(|e| e.into_inner());
            let map = g.get_or_insert_with(HashMap::new);
            let now = Instant::now();
            let since = map.get(service).map(|t| now.duration_since(*t));
            match since {
                Some(d) if d < gap => Some(gap - d),
                _ => { map.insert(service.to_string(), now); None }
            }
        };
        match wait { Some(w) => std::thread::sleep(w), None => return }
    }
}

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

#[derive(Default, Serialize, Deserialize)]
struct Cache { entries: HashMap<String, (u64, Value)> }

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
const KEEP: u64 = 30 * 86400;

fn cache_file() -> std::path::PathBuf { cache_dir().join("search.json") }

fn cached(key: &str) -> Option<Value> {
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let c = g.get_or_insert_with(|| std::fs::read_to_string(cache_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default());
    c.entries.get(key).filter(|(t, _)| now().saturating_sub(*t) < KEEP).map(|(_, v)| v.clone())
}

fn remember(key: &str, v: &Value) {
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let c = g.get_or_insert_with(Cache::default);
    c.entries.insert(key.to_string(), (now(), v.clone()));
    // At most 500 answers; the oldest go.
    if c.entries.len() > 500 {
        let mut ks: Vec<(u64, String)> = c.entries.iter().map(|(k, (t, _))| (*t, k.clone())).collect();
        ks.sort();
        for (_, k) in ks.into_iter().take(c.entries.len() - 500) { c.entries.remove(&k); }
    }
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(cache_file(), serde_json::to_string(c).unwrap_or_default());
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// GET with the shared clock and the cache.
pub fn get_json(service: &str, url: &str) -> Result<Value, String> {
    if let Some(v) = cached(url) { return Ok(v); }
    wait_turn(service);
    let r = agent().get(url).call().map_err(|e| match e {
        ureq::Error::Status(429, _) => "The service is busy (too many requests). Try again in a minute.".to_string(),
        ureq::Error::Status(403, _) => "The service turned the request down.".to_string(),
        ureq::Error::Status(c, _) => format!("The service answered {c}."),
        ureq::Error::Transport(t) => format!("Couldn't reach the service ({}). Are you online?", t.kind()),
    })?;
    let text = r.into_string().map_err(|e| format!("The service's answer couldn't be read ({e})."))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("The service's answer couldn't be read ({e})."))?;
    remember(url, &v);
    Ok(v)
}

pub fn parse_places(v: &Value) -> Vec<Place> {
    let one = |p: &Value| -> Option<Place> {
        let f = |k: &str| p.get(k).and_then(|x| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64()));
        let lat = f("lat")?; let lon = f("lon")?;
        let display = p.get("display_name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let name = p.get("name").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(str::to_string)
            .unwrap_or_else(|| display.split(',').next().unwrap_or("").trim().to_string());
        let address = display.strip_prefix(&name).map(|s| s.trim_start_matches([',', ' ']).to_string()).unwrap_or(display.clone());
        let bbox = p.get("boundingbox").and_then(|b| b.as_array()).and_then(|a| {
            let n: Vec<f64> = a.iter().filter_map(|x| x.as_str().and_then(|s| s.parse().ok()).or_else(|| x.as_f64())).collect();
            (n.len() == 4).then(|| [n[0], n[1], n[2], n[3]])
        });
        let kind = [p.get("type"), p.get("category").or(p.get("class"))].iter().flatten().filter_map(|x| x.as_str()).next().unwrap_or("").replace('_', " ");
        let osm = match (p.get("osm_type").and_then(|x| x.as_str()), p.get("osm_id").and_then(|x| x.as_u64())) {
            (Some(t), Some(id)) => format!("{t}/{id}"), _ => String::new(),
        };
        Some(Place { name, address, lat, lon, bbox, kind, osm })
    };
    match v { Value::Array(a) => a.iter().filter_map(one).collect(), o if o.get("lat").is_some() => one(o).into_iter().collect(), _ => vec![] }
}

/// Places matching `q`, those near `near` (the map's view: west, south, east, north) first.
pub fn search(s: &Services, q: &str, near: Option<[f64; 4]>, lang: &str) -> Result<Vec<Place>, String> {
    if !s.search_enabled { return Err(if s.notice.is_empty() { "Search is switched off for now.".into() } else { s.notice.clone() }); }
    let q = q.trim();
    if q.is_empty() { return Ok(vec![]); }
    if q.len() > 300 { return Err("That search is too long.".into()); }
    let mut url = format!("{}/search?format=jsonv2&addressdetails=0&limit=10&q={}&accept-language={}", s.nominatim, enc(q), enc(lang));
    if let Some([w, so, e, n]) = near { url.push_str(&format!("&viewbox={w:.3},{n:.3},{e:.3},{so:.3}")); }
    Ok(parse_places(&get_json(&s.nominatim, &url)?))
}

/// What's at a point (the address or place there).
pub fn reverse(s: &Services, lat: f64, lon: f64, zoom: u8, lang: &str) -> Result<Option<Place>, String> {
    if !s.search_enabled { return Err(if s.notice.is_empty() { "Search is switched off for now.".into() } else { s.notice.clone() }); }
    let url = format!("{}/reverse?format=jsonv2&lat={lat:.5}&lon={lon:.5}&zoom={}&accept-language={}", s.nominatim, zoom.min(18), enc(lang));
    let v = get_json(&s.nominatim, &url)?;
    if v.get("error").is_some() { return Ok(None); }
    Ok(parse_places(&v).into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nominatim_answers() {
        let v: Value = serde_json::from_str(r#"[{"place_id":1,"osm_type":"way","osm_id":518071791,"lat":"52.5162746","lon":"13.3777041","category":"tourism","type":"attraction","name":"Brandenburger Tor","display_name":"Brandenburger Tor, Pariser Platz, Mitte, Berlin, 10117, Deutschland","boundingbox":["52.5161","52.5164","13.3775","13.3778"]}]"#).unwrap();
        let p = &parse_places(&v)[0];
        assert_eq!(p.name, "Brandenburger Tor");
        assert_eq!(p.address, "Pariser Platz, Mitte, Berlin, 10117, Deutschland");
        assert!((p.lat - 52.5162746).abs() < 1e-9);
        assert_eq!(p.bbox.unwrap()[3], 13.3778);
        assert_eq!(p.kind, "attraction");
        assert_eq!(p.osm, "way/518071791");
        let r: Value = serde_json::from_str(r#"{"lat":"1.5","lon":"2.5","display_name":"Somewhere, Land","name":""}"#).unwrap();
        assert_eq!(parse_places(&r)[0].name, "Somewhere");
    }

    #[test]
    fn one_request_a_second() {
        let t = Instant::now();
        wait_turn("test-service"); wait_turn("test-service"); wait_turn("test-service");
        assert!(t.elapsed() >= Duration::from_millis(2200), "{:?}", t.elapsed());
        let t = Instant::now();
        wait_turn("another-service");
        assert!(t.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn query_encoding() {
        assert_eq!(enc("Café Zürich & co"), "Caf%C3%A9%20Z%C3%BCrich%20%26%20co");
    }
}
