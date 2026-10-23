// Directions with OSRM on FOSSGIS's public routing server (routing.openstreetmap.de, the one openstreetmap.org
// uses): car, bike and foot, worldwide. Its terms: at most one request a second, an identifying User-Agent, the
// attribution shown with a link to fix the map, no heavy use. Only asked when the person presses Go. When it's
// switched off (services.rs) or unreachable, the page offers the same directions on openstreetmap.org instead.
use crate::search::get_json;
use crate::services::Services;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub text: String,
    pub distance: f64,
    pub duration: f64,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub distance: f64,
    pub duration: f64,
    /// [lon, lat] pairs (GeoJSON order).
    pub line: Vec<[f64; 2]>,
    pub steps: Vec<Step>,
}

pub fn profile(mode: &str) -> Option<&'static str> {
    match mode { "car" => Some("routed-car"), "bike" => Some("routed-bike"), "foot" => Some("routed-foot"), _ => None }
}

/// The same trip on openstreetmap.org (for "Open in browser").
pub fn browser_url(mode: &str, from: (f64, f64), to: (f64, f64)) -> String {
    let engine = match mode { "bike" => "fossgis_osrm_bike", "foot" => "fossgis_osrm_foot", _ => "fossgis_osrm_car" };
    format!("https://www.openstreetmap.org/directions?engine={engine}&route={:.5}%2C{:.5}%3B{:.5}%2C{:.5}", from.0, from.1, to.0, to.1)
}

pub fn route(s: &Services, mode: &str, from: (f64, f64), to: (f64, f64)) -> Result<Route, String> {
    if !s.routing_enabled { return Err(if s.notice.is_empty() { "Directions are switched off for now: open them in the browser instead.".into() } else { s.notice.clone() }); }
    let p = profile(mode).ok_or("Choose car, bike or on foot.")?;
    let ok = |lat: f64, lon: f64| lat.is_finite() && lon.is_finite() && lat.abs() <= 90.0 && lon.abs() <= 180.0;
    if !ok(from.0, from.1) || !ok(to.0, to.1) { return Err("Those aren't places on Earth.".into()); }
    let url = format!("{}/{p}/route/v1/driving/{:.6},{:.6};{:.6},{:.6}?overview=full&geometries=geojson&steps=true&alternatives=false",
        s.routing, from.1, from.0, to.1, to.0);
    let v = get_json(&s.routing, &url)?;
    parse(&v)
}

pub fn parse(v: &Value) -> Result<Route, String> {
    match v.get("code").and_then(|c| c.as_str()) {
        Some("Ok") => {}
        Some("NoRoute") => return Err("There's no way there by this means.".into()),
        Some("NoSegment") => return Err("One of the places is too far from any road or path.".into()),
        Some(c) => return Err(format!("The routing service couldn't do that ({c}).")),
        None => return Err("The routing service's answer couldn't be read.".into()),
    }
    let r = v.get("routes").and_then(|r| r.get(0)).ok_or("No route came back.")?;
    let f = |x: &Value, k: &str| x.get(k).and_then(|n| n.as_f64()).unwrap_or(0.0);
    let line: Vec<[f64; 2]> = r.pointer("/geometry/coordinates").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|p| {
        let p = p.as_array()?; Some([p.first()?.as_f64()?, p.get(1)?.as_f64()?])
    }).collect()).unwrap_or_default();
    let mut steps = Vec::new();
    for leg in r.get("legs").and_then(|l| l.as_array()).into_iter().flatten() {
        for st in leg.get("steps").and_then(|s| s.as_array()).into_iter().flatten() {
            let m = st.get("maneuver").cloned().unwrap_or(Value::Null);
            let loc = m.get("location").and_then(|l| l.as_array()).cloned().unwrap_or_default();
            steps.push(Step {
                text: instruction(st),
                distance: f(st, "distance"),
                duration: f(st, "duration"),
                lon: loc.first().and_then(|x| x.as_f64()).unwrap_or(0.0),
                lat: loc.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0),
            });
        }
    }
    Ok(Route { distance: f(r, "distance"), duration: f(r, "duration"), line, steps })
}

/// An OSRM step as a sentence ("Turn left onto Main Street").
pub fn instruction(st: &Value) -> String {
    let m = st.get("maneuver").cloned().unwrap_or(Value::Null);
    let ty = m.get("type").and_then(|x| x.as_str()).unwrap_or("");
    let modi = m.get("modifier").and_then(|x| x.as_str()).unwrap_or("");
    let name = st.get("name").and_then(|x| x.as_str()).unwrap_or("").trim();
    let rf = st.get("ref").and_then(|x| x.as_str()).unwrap_or("").trim();
    let road = match (name.is_empty(), rf.is_empty()) { (false, false) => format!("{name} ({rf})"), (false, true) => name.to_string(), (true, false) => rf.to_string(), _ => String::new() };
    let onto = |verb: &str| if road.is_empty() { verb.to_string() } else { format!("{verb} onto {road}") };
    let dir = match modi { "uturn" => "Make a U-turn", "sharp right" => "Turn sharp right", "right" => "Turn right", "slight right" => "Bear right",
        "straight" => "Go straight on", "slight left" => "Bear left", "left" => "Turn left", "sharp left" => "Turn sharp left", _ => "Continue" };
    let side = if modi.contains("left") { "left" } else if modi.contains("right") { "right" } else { "" };
    match ty {
        "depart" => if road.is_empty() { "Head off".into() } else { format!("Head off on {road}") },
        "arrive" => match side { "" => "You've arrived".into(), s => format!("You've arrived; it's on the {s}") },
        "roundabout" | "rotary" => {
            let exit = m.get("exit").and_then(|x| x.as_u64());
            let base = match exit { Some(n) => format!("At the roundabout, take exit {n}"), None => "Go round the roundabout".into() };
            if road.is_empty() { base } else { format!("{base} onto {road}") }
        }
        "exit roundabout" | "exit rotary" => onto("Leave the roundabout"),
        "merge" => onto(&format!("Merge {}", if side.is_empty() { "ahead" } else { side })),
        "on ramp" => onto(&format!("Take the ramp{}", if side.is_empty() { String::new() } else { format!(" on the {side}") })),
        "off ramp" => onto(&format!("Take the exit{}", if side.is_empty() { String::new() } else { format!(" on the {side}") })),
        "fork" => onto(&format!("Keep {}", if side.is_empty() { "ahead" } else { side })),
        "end of road" => onto(&format!("At the end of the road, turn {}", if side.is_empty() { "ahead" } else { side })),
        "new name" => if road.is_empty() { "Continue".into() } else { format!("Continue on {road}") },
        "continue" if modi == "straight" || modi.is_empty() => if road.is_empty() { "Continue".into() } else { format!("Continue on {road}") },
        _ => onto(dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_osrm_route() {
        let v: Value = serde_json::from_str(r#"{"code":"Ok","routes":[{"distance":1234.5,"duration":321.0,
          "geometry":{"type":"LineString","coordinates":[[13.37,52.51],[13.38,52.52]]},
          "legs":[{"steps":[
            {"distance":100,"duration":20,"name":"Unter den Linden","maneuver":{"type":"depart","location":[13.37,52.51]}},
            {"distance":500,"duration":60,"name":"Friedrichstraße","ref":"B 96","maneuver":{"type":"turn","modifier":"left","location":[13.375,52.515]}},
            {"distance":300,"duration":40,"name":"","maneuver":{"type":"roundabout","exit":2,"location":[13.378,52.517]}},
            {"distance":0,"duration":0,"name":"","maneuver":{"type":"arrive","modifier":"right","location":[13.38,52.52]}}]}]}]}"#).unwrap();
        let r = parse(&v).unwrap();
        assert_eq!(r.line.len(), 2);
        assert_eq!(r.distance, 1234.5);
        let t: Vec<&str> = r.steps.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(t, vec!["Head off on Unter den Linden", "Turn left onto Friedrichstraße (B 96)", "At the roundabout, take exit 2", "You've arrived; it's on the right"]);
        assert_eq!(r.steps[1].lat, 52.515);
        assert!(parse(&serde_json::json!({"code":"NoRoute"})).unwrap_err().contains("no way"));
    }

    #[test]
    fn browser_link() {
        assert_eq!(browser_url("bike", (52.5, 13.4), (52.6, 13.5)), "https://www.openstreetmap.org/directions?engine=fossgis_osrm_bike&route=52.50000%2C13.40000%3B52.60000%2C13.50000");
    }
}
