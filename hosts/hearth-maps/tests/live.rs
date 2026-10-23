// Against the real services, lightly (a handful of requests, one search, one route): runs only with
// HEARTH_MAPS_LIVE=1 (the maps workflow sets it).
use hearth_maps::{route, search, services::Services, tiles};

#[test]
fn the_real_services() {
    if std::env::var("HEARTH_MAPS_LIVE").ok().as_deref() != Some("1") { eprintln!("HEARTH_MAPS_LIVE isn't 1: skipped"); return; }
    let dir = std::env::temp_dir().join("hearth-maps-live");
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("HEARTH_MAPS_CACHE", &dir);
    let s = Services::default();

    // Every style loads, and points back through hearth://.
    for st in &s.styles {
        let f = tiles::fetch(&s, &format!("hearth://tiles.openfreemap.org/styles/{}", st.id)).unwrap_or_else(|e| panic!("style {}: {e}", st.id));
        let text = String::from_utf8(f.data).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(v.get("layers").is_some(), "style {} has no layers", st.id);
        assert!(!text.contains("https://tiles.openfreemap.org"), "style {} still has direct addresses", st.id);
        eprintln!("style {}: {} layers", st.id, v["layers"].as_array().map(|a| a.len()).unwrap_or(0));
    }
    // The TileJSON and one tile of Berlin (zoom 10).
    let style: serde_json::Value = serde_json::from_slice(&tiles::fetch(&s, "hearth://tiles.openfreemap.org/styles/liberty").unwrap().data).unwrap();
    let src = style["sources"]["openmaptiles"]["url"].as_str().expect("the style's source").to_string();
    let tj: serde_json::Value = serde_json::from_slice(&tiles::fetch(&s, &src).unwrap().data).unwrap();
    let tmpl = tj["tiles"][0].as_str().expect("TileJSON tiles").to_string();
    assert!(tmpl.starts_with("hearth://"), "{tmpl}");
    let tile = tmpl.replace("{z}", "10").replace("{x}", "550").replace("{y}", "335");
    let t1 = tiles::fetch(&s, &tile).unwrap();
    assert!(t1.data.len() > 1000, "tile is {} bytes", t1.data.len());
    eprintln!("tile {tile}: {} bytes, {}", t1.data.len(), t1.ctype);
    // Kept for offline use.
    assert!(tiles::usage() > 0);
    // A glyph range for the labels.
    let font = style["layers"].as_array().unwrap().iter().find_map(|l| l["layout"]["text-font"][0].as_str()).expect("a label font").to_string();
    let glyphs = style["glyphs"].as_str().unwrap().replace("{fontstack}", &font).replace("{range}", "0-255").replace(' ', "%20");
    assert!(tiles::fetch(&s, &glyphs).unwrap().data.len() > 100);
    // Not a way to fetch anything else.
    assert!(tiles::fetch(&s, "hearth://example.com/").is_err());

    // One search, one "what's here", one route (with a second between them, as the services ask).
    let t = std::time::Instant::now();
    let found = search::search(&s, "Brandenburger Tor, Berlin", None, "en").unwrap();
    assert!(!found.is_empty());
    let p = &found[0];
    eprintln!("found {} at {}, {}", p.name, p.lat, p.lon);
    assert!((p.lat - 52.5163).abs() < 0.01 && (p.lon - 13.3777).abs() < 0.01);
    let here = search::reverse(&s, 52.5163, 13.3777, 18, "en").unwrap().expect("something there");
    eprintln!("what's here: {} ({})", here.name, here.address);
    assert!(t.elapsed() >= std::time::Duration::from_millis(1000), "two Nominatim requests within a second");
    let again = search::search(&s, "Brandenburger Tor, Berlin", None, "en").unwrap();
    assert_eq!(again, found, "a repeated search comes from the cache");

    let r = route::route(&s, "foot", (52.5163, 13.3777), (52.5186, 13.4081)).unwrap();
    eprintln!("on foot: {:.0} m, {:.0} s, {} steps; first: {}", r.distance, r.duration, r.steps.len(), r.steps[0].text);
    assert!(r.distance > 1500.0 && r.distance < 5000.0);
    assert!(r.line.len() > 5);
    let _ = std::fs::remove_dir_all(&dir);
}
