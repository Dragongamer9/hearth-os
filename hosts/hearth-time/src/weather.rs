//! Hearth Weather: forecasts from Open-Meteo (open-meteo.com: free, no account or key), for a place you type in.
//! Your location is never looked up or guessed: no place typed, no weather.
//!   ~/.config/hearth-weather/weather.json   {"place": {name, admin1, country, latitude, longitude, timezone}, "units": "metric"|"imperial"}
//!   ~/.cache/hearth-weather/forecast.json   the last forecast as Open-Meteo sent it, with "fetchedAt"
//!   ~/.cache/hearth-weather/summary.json    a few words for the taskbar and Quick Access (`hearth-time --weather-summary`)

use serde_json::{json, Value};

/// After this long the forecast is downloaded again (when something asks for it).
pub const FRESH_MS: i64 = 30 * 60 * 1000;

pub fn settings() -> Value { crate::store::load(&crate::paths::weather_file()) }

pub fn imperial(s: &Value) -> bool { s["units"] == "imperial" }

/// Places matching what you typed (Open-Meteo's place search).
pub fn search(q: &str) -> Result<Vec<Value>, String> {
    let q = q.trim();
    if q.chars().count() < 2 { return Ok(vec![]); }
    let url = format!("https://geocoding-api.open-meteo.com/v1/search?name={}&count=10&language=en&format=json", crate::net::urlencode(q));
    let v: Value = serde_json::from_str(&crate::net::get(&url, 20)?).map_err(|_| "The place search answered strangely.".to_string())?;
    Ok(v["results"].as_array().cloned().unwrap_or_default().into_iter().map(|r| json!({
        "name": r["name"], "admin1": r["admin1"], "country": r["country"], "countryCode": r["country_code"],
        "latitude": r["latitude"], "longitude": r["longitude"], "timezone": r["timezone"],
    })).collect())
}

pub fn forecast_url(place: &Value, imperial: bool) -> Option<String> {
    let (lat, lon) = (place["latitude"].as_f64()?, place["longitude"].as_f64()?);
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) { return None; }
    let mut u = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,is_day,wind_speed_10m,wind_direction_10m,precipitation,pressure_msl\
         &hourly=temperature_2m,weather_code,precipitation_probability,is_day\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,sunrise,sunset,uv_index_max\
         &timezone=auto&forecast_days=7&forecast_hours=48"
    );
    if imperial { u.push_str("&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch"); }
    Some(u)
}

/// What a WMO weather code means, in a few words.
pub fn describe(code: i64) -> &'static str {
    match code {
        0 => "Clear", 1 => "Mostly clear", 2 => "Partly cloudy", 3 => "Cloudy",
        45 | 48 => "Fog", 51 | 53 | 55 => "Drizzle", 56 | 57 => "Freezing drizzle",
        61 => "Light rain", 63 => "Rain", 65 => "Heavy rain", 66 | 67 => "Freezing rain",
        71 => "Light snow", 73 => "Snow", 75 => "Heavy snow", 77 => "Snow grains",
        80 | 81 => "Showers", 82 => "Heavy showers", 85 | 86 => "Snow showers",
        95 => "Thunderstorm", 96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

/// The freedesktop icon name for a code (the taskbar draws it).
pub fn icon(code: i64, day: bool) -> &'static str {
    match code {
        0 | 1 => if day { "weather-clear" } else { "weather-clear-night" },
        2 => if day { "weather-few-clouds" } else { "weather-few-clouds-night" },
        3 => "weather-overcast", 45 | 48 => "weather-fog",
        51..=67 | 80..=82 => "weather-showers", 71..=77 | 85 | 86 => "weather-snow",
        95..=99 => "weather-storm", _ => "weather-severe-alert",
    }
}

/// The short summary from a forecast.
pub fn summary_of(place: &Value, f: &Value, imperial: bool) -> Value {
    let c = &f["current"];
    let code = c["weather_code"].as_i64().unwrap_or(-1);
    let day = c["is_day"].as_i64().unwrap_or(1) == 1;
    let t = c["temperature_2m"].as_f64();
    let unit = if imperial { "°F" } else { "°C" };
    let hi = f["daily"]["temperature_2m_max"][0].as_f64();
    let lo = f["daily"]["temperature_2m_min"][0].as_f64();
    let temp = t.map(|t| format!("{}°", t.round() as i64)).unwrap_or_default();
    let name = place["name"].as_str().unwrap_or("").to_string();
    let text = describe(code);
    let line = if temp.is_empty() { String::new() } else { format!("{temp} {text}") };
    json!({
        "place": name, "temp": temp, "unit": unit, "text": text, "code": code, "icon": icon(code, day), "day": day,
        "high": hi.map(|x| x.round() as i64), "low": lo.map(|x| x.round() as i64),
        "line": line,
        "updated": f["fetchedAt"],
    })
}

/// The saved forecast (maybe old; "fetchedAt" says when).
pub fn cached() -> Value { crate::store::load(&crate::paths::weather_cache().join("forecast.json")) }

/// Downloads the forecast for the saved place and keeps it (and its summary). Err when there's no place yet.
pub fn refresh() -> Result<Value, String> {
    let s = settings();
    let place = &s["place"];
    let imp = imperial(&s);
    let url = forecast_url(place, imp).ok_or("Choose a place first.")?;
    let mut f: Value = serde_json::from_str(&crate::net::get(&url, 20)?).map_err(|_| "The weather service answered strangely.".to_string())?;
    if f["current"].is_null() { return Err(f["reason"].as_str().map(|r| format!("The weather service said: {r}")).unwrap_or_else(|| "No forecast came back.".into())); }
    f["fetchedAt"] = json!(crate::now_ms());
    f["place"] = place.clone();
    let units = if imp { "imperial" } else { "metric" };
    f["units"] = json!(units);
    let dir = crate::paths::weather_cache();
    crate::store::save(&dir.join("forecast.json"), &f)?;
    crate::store::save(&dir.join("summary.json"), &summary_of(place, &f, imp))?;
    Ok(f)
}

/// The forecast, downloaded again when it's old or for another place or other units.
pub fn current(force: bool) -> Result<Value, String> {
    let s = settings();
    if s["place"].is_null() { return Ok(Value::Null); }
    let c = cached();
    let units = if imperial(&s) { "imperial" } else { "metric" };
    let same = c["place"]["latitude"] == s["place"]["latitude"] && c["place"]["longitude"] == s["place"]["longitude"] && c["units"] == units;
    let fresh = c["fetchedAt"].as_i64().map(|t| crate::now_ms() - t < FRESH_MS).unwrap_or(false);
    if same && fresh && !force { return Ok(c); }
    match refresh() {
        Ok(f) => Ok(f),
        // Offline: the old forecast (the page says how old) beats nothing.
        Err(e) if same && c["current"].is_object() => { let mut c = c; c["stale"] = json!(e); Ok(c) }
        Err(e) => Err(e),
    }
}

/// Saves the place and units (only what the page sends: a place it got from `search`).
pub fn set(place: Option<Value>, units: Option<String>) -> Result<Value, String> {
    let mut s = settings();
    if let Some(p) = place {
        if p.is_null() { s["place"] = Value::Null; let _ = std::fs::remove_file(crate::paths::weather_cache().join("summary.json")); }
        else {
            if forecast_url(&p, false).is_none() { return Err("That place has no position.".into()); }
            s["place"] = json!({"name": p["name"], "admin1": p["admin1"], "country": p["country"], "latitude": p["latitude"], "longitude": p["longitude"], "timezone": p["timezone"]});
        }
    }
    if let Some(u) = units { let u = if u == "imperial" { "imperial" } else { "metric" }; s["units"] = json!(u); }
    crate::store::save(&crate::paths::weather_file(), &s)?;
    Ok(s)
}

/// `hearth-time --weather-summary`: the saved summary (empty object when there's no place).
pub fn summary_json() -> Value {
    if settings()["place"].is_null() { return json!({}); }
    crate::store::load(&crate::paths::weather_cache().join("summary.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_words() {
        let p = json!({"name": "Lisbon", "latitude": 38.71667, "longitude": -9.13333});
        let u = forecast_url(&p, true).unwrap();
        assert!(u.contains("latitude=38.7167&longitude=-9.1333"));
        assert!(u.contains("temperature_unit=fahrenheit"));
        assert!(forecast_url(&json!({"name": "x"}), false).is_none());
        assert!(forecast_url(&json!({"latitude": 99.0, "longitude": 0.0}), false).is_none());
        assert_eq!(describe(63), "Rain");
        assert_eq!(icon(0, false), "weather-clear-night");
    }

    #[test]
    fn summary() {
        let f = json!({"current": {"temperature_2m": 17.6, "weather_code": 2, "is_day": 1}, "daily": {"temperature_2m_max": [21.2], "temperature_2m_min": [12.5]}, "fetchedAt": 5});
        let s = summary_of(&json!({"name": "Lisbon"}), &f, false);
        assert_eq!(s["line"], "18° Partly cloudy");
        assert_eq!(s["high"], 21);
        assert_eq!(s["low"], 13);
        assert_eq!(s["unit"], "°C");
    }
}
