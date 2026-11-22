//! A new feature made with Describe it (desktop-layouts board 8, hearth-ideas board 1): a recipe of Hearth's own
//! parts, never code. It names where it goes, what it shows (from a fixed list of things Hearth can read), and what a
//! click does (from a fixed list of things Hearth can do). Anything the recipe asks for that isn't on those lists
//! can't be made yet: it's listed in `needs`, and that's what goes to Hearth's makers as an idea (idea.rs).
//!
//! ```json
//! { "format": 1, "id": "gpu-temp-pill", "name": "GPU temperature", "part": "bar",
//!   "shows": [{ "source": "gpu-temp", "label": "GPU" }], "every": 5,
//!   "click": { "action": "open-app", "target": "hearth-monitor" } }
//! ```

use serde::{Deserialize, Serialize};

/// Where a feature can go: Mantel's parts, then Hearth's apps that have Describe it.
pub const PARTS: &[&str] = &[
    "bar", "panel", "dock", "menu", "quickaccess", "calendar", "notifications", "desktop", "lockscreen",
    "files", "settings", "store", "music", "monitor",
];

/// What a feature can show: things Hearth already reads.
pub const SOURCES: &[(&str, &str)] = &[
    ("time", "the time"),
    ("date", "the date"),
    ("battery", "the battery"),
    ("volume", "the volume"),
    ("network", "the network"),
    ("bluetooth", "Bluetooth"),
    ("cpu", "how busy the processor is"),
    ("memory", "memory in use"),
    ("gpu-temp", "the graphics card's temperature"),
    ("cpu-temp", "the processor's temperature"),
    ("disk-free", "free space"),
    ("weather", "the weather"),
    ("now-playing", "what's playing"),
    ("downloads", "copies and downloads"),
    ("updates", "updates waiting"),
    ("game-running", "the game running"),
    ("link-device", "a device on Hearth Link (its battery, what it's doing)"),
    ("text", "your own words"),
];

/// What a click can do: things Hearth already does.
pub const ACTIONS: &[(&str, &str)] = &[
    ("open-app", "open an app"),
    ("open-settings", "open a page of Settings"),
    ("open-folder", "open a folder in Files"),
    ("open-url", "open a web page (https only)"),
    ("game-mode", "switch to Game Mode"),
    ("show-desktop", "show the desktop"),
    ("overview", "show all windows"),
    ("menu", "open the app menu"),
    ("quickaccess", "open Quick Access"),
    ("volume-mute", "mute or unmute"),
    ("screenshot", "take a screenshot"),
    ("lock", "lock the screen"),
    ("dnd", "turn Do not disturb on or off"),
    ("none", "nothing"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Show {
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Click {
    pub action: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub format: u32,
    pub id: String,
    pub name: String,
    pub part: String,
    #[serde(default)]
    pub shows: Vec<Show>,
    /// how often it looks again, in seconds (2 to 3600; the time and date follow the clock instead)
    #[serde(default = "default_every")]
    pub every: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub click: Option<Click>,
    /// what was asked for that Hearth can't do yet (Describe it fills this in; the editor shows it, and it can be
    /// sent as an idea)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<String>,
}
fn default_every() -> u32 {
    30
}

/// What a recipe found: plain-word problems (it can't be used) and what it needs that Hearth doesn't have yet.
#[derive(Debug, Default, PartialEq)]
pub struct Checked {
    pub problems: Vec<String>,
    pub needs: Vec<String>,
}
impl Checked {
    /// It can be put in place now.
    pub fn usable(&self) -> bool {
        self.problems.is_empty() && self.needs.is_empty()
    }
}

pub fn parse(text: &str) -> Result<Feature, String> {
    serde_json::from_str(text).map_err(|e| format!("It isn't a feature Hearth can read ({e})."))
}

pub fn check(f: &Feature) -> Checked {
    let mut c = Checked::default();
    if f.format != 1 {
        c.problems.push(format!("It's made for a newer Hearth (feature format {}).", f.format));
    }
    if !crate::is_slug(&f.id) {
        c.problems.push("Its id must be short, with lowercase letters, digits and dashes.".into());
    }
    if f.name.trim().is_empty() || f.name.chars().count() > 60 {
        c.problems.push("It needs a name (up to 60 letters).".into());
    }
    if !PARTS.contains(&f.part.as_str()) {
        c.needs.push(format!("a place called \"{}\"", f.part));
    }
    if f.shows.is_empty() && f.click.is_none() {
        c.problems.push("It shows nothing and does nothing.".into());
    }
    if f.shows.len() > 6 {
        c.problems.push("Up to six things in one feature.".into());
    }
    if !(2..=3600).contains(&f.every) {
        c.problems.push("It looks again every 2 seconds at most often, and every hour at least.".into());
    }
    for s in &f.shows {
        if !SOURCES.iter().any(|(id, _)| *id == s.source) {
            c.needs.push(format!("showing \"{}\"", s.source));
        }
        if s.label.chars().count() > 40 {
            c.problems.push("A label is up to 40 letters.".into());
        }
    }
    if let Some(k) = &f.click {
        match k.action.as_str() {
            a if !ACTIONS.iter().any(|(id, _)| *id == a) => c.needs.push(format!("a click that does \"{a}\"")),
            "open-url" if !k.target.starts_with("https://") || k.target.chars().any(char::is_whitespace) => {
                c.problems.push("A web page must start with https:// .".into())
            }
            "open-app" | "open-settings" | "open-folder" if k.target.trim().is_empty() => c.problems.push("Say which one to open.".into()),
            "open-app" | "open-settings" if !k.target.chars().all(|ch| ch.is_ascii_alphanumeric() || "-._".contains(ch)) => {
                c.problems.push(format!("\"{}\" isn't an app or a page of Settings.", k.target))
            }
            "open-folder" if k.target.contains("..") => c.problems.push("A folder can't climb out with \"..\".".into()),
            _ => {}
        }
    }
    for n in &f.needs {
        if !c.needs.contains(n) {
            c.needs.push(n.clone());
        }
    }
    c
}

/// The feature in a sentence, as the editor shows it before it's put in place.
pub fn describe(f: &Feature) -> String {
    let name = |list: &[(&str, &'static str)], id: &str| list.iter().find(|(i, _)| *i == id).map(|(_, n)| n.to_string()).unwrap_or(format!("\"{id}\""));
    let shows: Vec<String> = f.shows.iter().map(|s| name(SOURCES, &s.source)).collect();
    let mut t = format!("{} ({}): ", f.name, f.part);
    t += &if shows.is_empty() { "shows nothing".into() } else { format!("shows {}", shows.join(", ")) };
    if let Some(k) = &f.click {
        t += &format!("; a click will {}", name(ACTIONS, &k.action));
        if !k.target.is_empty() {
            t += &format!(" ({})", k.target);
        }
    }
    t + "."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recipe_of_hearths_parts_is_usable() {
        let f = parse(r#"{"format":1,"id":"gpu","name":"GPU temperature","part":"bar","shows":[{"source":"gpu-temp","label":"GPU"}],
            "every":5,"click":{"action":"open-app","target":"hearth-monitor"}}"#).unwrap();
        assert!(check(&f).usable(), "{:?}", check(&f));
        assert_eq!(describe(&f), "GPU temperature (bar): shows the graphics card's temperature; a click will open an app (hearth-monitor).");
    }

    #[test]
    fn what_hearth_cant_do_yet_becomes_a_need() {
        let f = parse(r#"{"format":1,"id":"x","name":"X","part":"bar","shows":[{"source":"stock-prices"}],
            "click":{"action":"run-command","target":"rm -rf ~"}}"#).unwrap();
        let c = check(&f);
        assert!(c.problems.is_empty());
        assert_eq!(c.needs, ["showing \"stock-prices\"", "a click that does \"run-command\""]);
        assert!(!c.usable());
    }

    #[test]
    fn unsafe_targets_are_refused() {
        let base = |click: &str| parse(&format!(r#"{{"format":1,"id":"x","name":"X","part":"bar","click":{click}}}"#)).unwrap();
        assert!(!check(&base(r#"{"action":"open-url","target":"http://example.com"}"#)).problems.is_empty());
        assert!(!check(&base(r#"{"action":"open-url","target":"javascript:alert(1)"}"#)).problems.is_empty());
        assert!(!check(&base(r#"{"action":"open-app","target":"firefox; rm -rf ~"}"#)).problems.is_empty());
        assert!(!check(&base(r#"{"action":"open-folder","target":"../../etc"}"#)).problems.is_empty());
        assert!(check(&base(r#"{"action":"open-url","target":"https://hearthos.fun"}"#)).usable());
    }
}
