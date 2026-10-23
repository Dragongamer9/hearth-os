// geo: links (RFC 5870, plus Android's ?q= and ?z=): what Hearth Maps opens when a link or another app asks for a
// place. "geo:52.5163,13.3777?z=16", "geo:0,0?q=Brandenburger+Tor", "geo:0,0?q=52.5163,13.3777(The+gate)".
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoLink {
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub zoom: Option<f64>,
    /// Something to search for.
    pub query: String,
    pub label: String,
}

fn decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn latlon(s: &str) -> Option<(f64, f64)> {
    let mut it = s.split(',');
    let lat: f64 = it.next()?.trim().parse().ok()?;
    let lon: f64 = it.next()?.trim().parse().ok()?;
    (lat.abs() <= 90.0 && lon.abs() <= 180.0).then_some((lat, lon))
}

pub fn parse(uri: &str) -> Option<GeoLink> {
    let rest = uri.trim();
    let rest = rest.get(..4).filter(|p| p.eq_ignore_ascii_case("geo:")).map(|_| &rest[4..])?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let coords = path.split(';').next().unwrap_or("");
    let mut g = GeoLink::default();
    if let Some((lat, lon)) = latlon(coords) {
        // geo:0,0 with a query means "search for this", not a point at sea off Africa.
        if !(lat == 0.0 && lon == 0.0 && query.contains("q=")) { g.lat = Some(lat); g.lon = Some(lon); }
    }
    for kv in query.split('&') {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        match k {
            "z" => g.zoom = v.parse::<f64>().ok().filter(|z| (0.0..=22.0).contains(z)),
            "q" => {
                let v = decode(v);
                // "lat,lon(Label)"
                let (head, label) = match v.find('(') { Some(i) if v.ends_with(')') => (v[..i].to_string(), v[i + 1..v.len() - 1].to_string()), _ => (v.clone(), String::new()) };
                if let Some((lat, lon)) = latlon(&head) { g.lat = Some(lat); g.lon = Some(lon); g.label = label; }
                else { g.query = v.trim().to_string(); }
            }
            _ => {}
        }
    }
    (g.lat.is_some() || !g.query.is_empty()).then_some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_links() {
        assert_eq!(parse("geo:52.5163,13.3777?z=16"), Some(GeoLink { lat: Some(52.5163), lon: Some(13.3777), zoom: Some(16.0), ..Default::default() }));
        assert_eq!(parse("geo:48.2,16.37;u=35;crs=wgs84").unwrap().lon, Some(16.37));
        let q = parse("geo:0,0?q=Brandenburger+Tor%2C+Berlin").unwrap();
        assert_eq!((q.lat, q.query.as_str()), (None, "Brandenburger Tor, Berlin"));
        let l = parse("GEO:0,0?q=52.5163,13.3777(The%20gate)").unwrap();
        assert_eq!((l.lat, l.label.as_str()), (Some(52.5163), "The gate"));
        assert_eq!(parse("geo:"), None);
        assert_eq!(parse("geo:91,0"), None);
        assert_eq!(parse("https://example.com"), None);
    }
}
