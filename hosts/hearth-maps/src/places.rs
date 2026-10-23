// Saved places and the last view of the map, in ~/.local/share/hearth-maps/ (only on this computer).
use crate::services::data_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Saved {
    pub name: String,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct View {
    pub lat: f64,
    pub lon: f64,
    pub zoom: f64,
    pub style: String,
}

impl Default for View {
    // The whole world, until the person looks somewhere (their location is never looked up for this).
    fn default() -> Self { View { lat: 30.0, lon: 10.0, zoom: 1.6, style: "liberty".into() } }
}

fn read<T: for<'de> Deserialize<'de> + Default>(name: &str) -> T {
    std::fs::read_to_string(data_dir().join(name)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}
fn write<T: Serialize>(name: &str, v: &T) -> Result<(), String> {
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let p = data_dir().join(name);
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, p).map_err(|e| e.to_string())
}

pub fn saved() -> Vec<Saved> { read("places.json") }
pub fn save(p: Saved) -> Result<Vec<Saved>, String> {
    let mut all = saved();
    all.retain(|x| !((x.lat - p.lat).abs() < 1e-6 && (x.lon - p.lon).abs() < 1e-6));
    all.insert(0, p);
    write("places.json", &all)?;
    Ok(all)
}
pub fn forget(lat: f64, lon: f64) -> Result<Vec<Saved>, String> {
    let mut all = saved();
    all.retain(|x| !((x.lat - lat).abs() < 1e-6 && (x.lon - lon).abs() < 1e-6));
    write("places.json", &all)?;
    Ok(all)
}
pub fn view() -> View { read("view.json") }
pub fn set_view(v: &View) -> Result<(), String> { write("view.json", v) }
