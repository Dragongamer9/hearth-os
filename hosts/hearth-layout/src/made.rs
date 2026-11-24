//! Blocks your AI makes (build-anything board 5): when Hearth hasn't got a block for what you asked ("my 3D printer's
//! camera and how far the print is, in the corner"), your AI writes a new one on the spot. It isn't code: it's a recipe
//! of what to read (one address on your home network, one web address, or one file of yours) and how to show it (a
//! title, lines with values from what was read, a progress bar, a picture), checked here before anything runs.
//!
//! It runs on its own, so nothing else can break: `hearth-layout run-block ID` is its own small program, started by the
//! Mantel that shows it, stopped by itself after 2 s of processor time or 256 MB of memory (setrlimit) and after 10 s
//! whatever it's doing. It can't touch starting up, Hearth's system or your settings (it only reads, and only what it
//! was allowed). It asks before it reaches anything: Allow (remembered), Just this once, or No. Turn any of them off in
//! Settings › Your desktop › Your blocks.
//!
//! ~/.config/nimbus/blocks/<id>.json is the block (written with settings_file: atomic, a .bak), allowed.json what you
//! said to each one's ask.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Reach {
    /// "address" (one address on your home network), "internet" (one https address), "file" (one file of yours), "" none
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub value: String,
    /// what it's for, in a few words ("your printer")
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub what: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Shows {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// lines with {a.b.c} for a value from what was read (JSON), {text} for all of it (plain text), {time}
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    /// a value 0-100 (or 0-1) from what was read: a progress bar
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub progress: String,
    /// a picture from the same address (a camera's snapshot): its path on it ("/webcam/?action=snapshot")
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub picture: String,
    /// "Live · Anycubic Chiron": the small chip on top
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Made {
    pub format: u32,
    pub id: String,
    pub name: String,
    /// what you asked, kept with it
    #[serde(default)]
    pub asked: String,
    /// who made it ("Claude (your AI team)")
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub by: String,
    #[serde(default)]
    pub reach: Reach,
    /// the path read at that address ("/api/job"); for a file or an internet address, empty
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default)]
    pub shows: Shows,
    /// seconds between looks (5-3600)
    #[serde(default = "every")]
    pub every: u32,
    /// off in Settings › Your desktop › Your blocks
    #[serde(default = "yes")]
    pub on: bool,
}
fn every() -> u32 {
    30
}
fn yes() -> bool {
    true
}

pub fn dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".config")).join("nimbus/blocks")
}
fn file(id: &str) -> PathBuf {
    dir().join(format!("{id}.json"))
}
fn allowed_file() -> PathBuf {
    dir().join("allowed.json")
}

/// An address on your home network: 192.168.x.x, 10.x.x.x, 172.16-31.x.x, or a name ending .local / .lan; a port is fine.
pub fn home_address(a: &str) -> bool {
    let host = a.split(':').next().unwrap_or("");
    if host.ends_with(".local") || host.ends_with(".lan") {
        return host.len() < 80 && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    }
    let p: Vec<u8> = host.split('.').filter_map(|x| x.parse::<u8>().ok()).collect();
    let port_ok = a.split(':').nth(1).map(|p| p.parse::<u16>().is_ok()).unwrap_or(true);
    p.len() == 4 && host.split('.').count() == 4 && port_ok && (p[0] == 192 && p[1] == 168 || p[0] == 10 || p[0] == 172 && (16..=31).contains(&p[1]))
}

/// Everything wrong with a block, in plain words (empty: it can run).
pub fn check(m: &Made) -> Vec<String> {
    let mut e = Vec::new();
    if m.format != 1 {
        e.push("A made block is format 1.".into());
    }
    if !crate::is_slug(&m.id) {
        e.push("A block's id is short, lowercase letters, digits and dashes.".into());
    }
    if m.name.trim().is_empty() || m.name.chars().count() > 60 {
        e.push("A block has a short name.".into());
    }
    if !(5..=3600).contains(&m.every) {
        e.push("It looks again every 5 seconds to an hour.".into());
    }
    match m.reach.kind.as_str() {
        "" => {
            if !m.reach.value.is_empty() {
                e.push("Nothing to reach, so no address.".into());
            }
        }
        "address" => {
            if !home_address(&m.reach.value) {
                e.push("A home network address is like 192.168.1.40 (or printer.local), nothing on the internet.".into());
            }
        }
        "internet" => {
            let v = &m.reach.value;
            if !v.starts_with("https://") || v.len() > 300 || v.contains(char::is_whitespace) || v.contains('@') {
                e.push("An internet address is one https address.".into());
            }
        }
        "file" => {
            let v = &m.reach.value;
            if !v.starts_with("~/") || v.contains("..") || v.len() > 300 || v.contains("/.ssh") || v.contains("/.config/nimbus") || v.contains("/.gnupg") {
                e.push("A file is one of yours in your home folder (not a hidden settings or key folder).".into());
            }
        }
        k => e.push(format!("\"{k}\" isn't something a block can reach (an address at home, the internet, or a file).")),
    }
    if !m.path.is_empty() && (!m.path.starts_with('/') || m.path.len() > 200 || m.path.contains(char::is_whitespace) || m.path.contains("..")) {
        e.push("The path at the address starts with / and has no spaces.".into());
    }
    if !m.shows.picture.is_empty() && (m.reach.kind != "address" || !m.shows.picture.starts_with('/') || m.shows.picture.contains(char::is_whitespace)) {
        e.push("A picture comes from the same home address, by its path.".into());
    }
    if m.shows.lines.len() > 8 || m.shows.lines.iter().any(|l| l.chars().count() > 160) || m.shows.title.chars().count() > 80 {
        e.push("Up to 8 short lines.".into());
    }
    e
}

/// What the AI is told about made blocks (Describe it's rules for a block that isn't here).
pub fn rules() -> String {
    "A new block (when Hearth hasn't got one): {\"kind\": \"block\", \"block\": {\"format\": 1, \"id\": \"<lowercase-dashes>\", \"name\": \"...\", \"asked\": \"<their words>\", \
\"reach\": {\"kind\": \"address\"|\"internet\"|\"file\"|\"\", \"value\": \"192.168.1.40\" | \"https://...\" | \"~/file\", \"what\": \"your printer\"}, \"path\": \"/api/...\", \
\"shows\": {\"title\": \"{job.file} · {progress.completion}%\", \"lines\": [\"{progress.printTimeLeft} s left\"], \"progress\": \"progress.completion\", \"picture\": \"/webcam/?action=snapshot\", \"chip\": \"Live · ...\"}, \
\"every\": 5-3600}, \"changes\": [...]}. It only reads; values from what it read go in {dotted.path}. Never code, commands or passwords."
        .into()
}

pub fn parse(v: &Value) -> Result<Made, Vec<String>> {
    let m: Made = serde_json::from_value(v.clone()).map_err(|e| vec![format!("The block couldn't be read ({e}).")])?;
    let e = check(&m);
    if e.is_empty() { Ok(m) } else { Err(e) }
}

/// Your AI makes a block for what you asked (Describe it in the block editor: "Describe a block that isn't here").
pub fn make(asked: &str) -> Value {
    let asked: String = asked.trim().chars().take(1000).collect();
    if asked.is_empty() {
        return json!({ "ok": false, "error": "Say what the block should show." });
    }
    let Some(e) = crate::describe::engine() else {
        return json!({ "ok": false, "error": crate::describe::NO_AI });
    };
    let system = format!("You make one new block for Hearth OS's desktop. Answer with ONE JSON object and nothing else.\n{}", rules());
    let mut user = format!("The owner asks: {asked}");
    for attempt in 0..2 {
        let reply = match crate::describe::chat(&e, &system, &user, None) {
            Ok(r) => r,
            Err(m) => return json!({ "ok": false, "error": m }),
        };
        let Some(v) = crate::describe::first_object(&reply) else {
            user = format!("{user}\nAnswer with only the JSON object.");
            continue;
        };
        let b = v.get("block").cloned().unwrap_or(v.clone());
        match parse(&b) {
            Ok(mut m) => {
                m.asked = asked.clone();
                if m.by.is_empty() {
                    m.by = e.label.clone();
                }
                return json!({ "ok": true, "kind": "block", "block": m, "asks": asks(&m), "engine": e.label });
            }
            Err(why) if attempt == 0 => user = format!("{user}\nYour last answer can't be used: {}. Answer again, fixed.", why.join(" ")),
            Err(why) => return json!({ "ok": false, "error": format!("Your AI's block couldn't be used: {}", why.join(" ")) }),
        }
    }
    json!({ "ok": false, "error": "Your AI didn't make a block Hearth can use. Try again, or say it another way." })
}

/// What it asks to reach, as the board says it: "It asks to reach your printer at 192.168.1.40".
pub fn asks(m: &Made) -> Value {
    match m.reach.kind.as_str() {
        "" => Value::Null,
        k => {
            let what = if m.reach.what.is_empty() { "something".to_string() } else { m.reach.what.clone() };
            let (title, sub) = match k {
                "address" => (format!("It asks to reach {what} at {}", m.reach.value), "Only that address on your home network: nothing on the internet, none of your files.".to_string()),
                "internet" => (format!("It asks to reach {}", m.reach.value), "Only that address on the internet: nothing at home, none of your files.".to_string()),
                _ => (format!("It asks to read {}", m.reach.value), "Only that file, to read: nothing else of yours, nothing on the network.".to_string()),
            };
            json!({ "kind": k, "value": m.reach.value, "title": title, "sub": sub })
        }
    }
}

/// Keep a block (Keep this): written as yours, on.
pub fn keep(m: &Made) -> Result<(), String> {
    let e = check(m);
    if !e.is_empty() {
        return Err(e.join(" "));
    }
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    crate::settings_file::write_json(&file(&m.id), &serde_json::to_value(m).unwrap_or(Value::Null)).map(|_| ()).map_err(|e| e.to_string())
}

pub fn get(id: &str) -> Option<Made> {
    if !crate::is_slug(id) {
        return None;
    }
    serde_json::from_value(crate::settings_file::read_json(&file(id))).ok().filter(|m: &Made| check(m).is_empty())
}

/// Every block you kept, with what you said to its ask.
pub fn mine() -> Value {
    let allowed = crate::settings_file::read_json(&allowed_file());
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir()) {
        let mut names: Vec<String> = rd.flatten().filter_map(|x| x.file_name().to_str().map(str::to_string)).filter(|n| n.ends_with(".json") && n != "allowed.json").collect();
        names.sort();
        for n in names {
            if let Some(m) = get(n.trim_end_matches(".json")) {
                out.push(json!({ "id": m.id, "name": m.name, "asked": m.asked, "by": m.by, "on": m.on, "reach": m.reach, "allowed": allowed.get(&m.id).and_then(Value::as_str).unwrap_or("") }));
            }
        }
    }
    json!(out)
}

/// A block on or off (Settings › Your desktop › Your blocks).
pub fn set_on(id: &str, on: bool) -> Result<(), String> {
    let mut m = get(id).ok_or("That block isn't one of yours.")?;
    m.on = on;
    keep(&m)
}

/// What you said to a block's ask: "always" (Allow), "no", or "" (ask again). Just this once isn't written.
pub fn allow(id: &str, answer: &str) -> Result<(), String> {
    if !crate::is_slug(id) || !["always", "no", ""].contains(&answer) {
        return Err("Allow, No, or ask again.".into());
    }
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    crate::settings_file::update_json(&allowed_file(), |m: &mut Map<String, Value>| {
        if answer.is_empty() {
            m.remove(id);
        } else {
            m.insert(id.to_string(), json!(answer));
        }
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// A value from what was read, by a dotted path ("progress.completion", "temps.0.nozzle").
fn pick<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = v;
    for k in path.split('.').filter(|k| !k.is_empty()) {
        cur = match cur {
            Value::Object(m) => m.get(k)?,
            Value::Array(a) => a.get(k.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}
fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.chars().take(200).collect(),
        Value::Number(n) => n.as_f64().map(|f| if f.fract() == 0.0 { format!("{}", f as i64) } else { format!("{f:.1}") }).unwrap_or_default(),
        Value::Bool(b) => if *b { "yes".into() } else { "no".into() },
        Value::Null => "—".into(),
        _ => "…".into(),
    }
}
/// "{job.file} · {progress.completion}%" with the values put in.
pub fn fill(t: &str, data: &Value, text: &str) -> String {
    let mut out = String::new();
    let mut rest = t;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else { break };
        let key = &rest[i + 1..i + j];
        out.push_str(&match key {
            "text" => text.chars().take(300).collect(),
            "time" => chrono::Local::now().format("%H:%M").to_string(),
            k => pick(data, k).map(show).unwrap_or_else(|| "—".into()),
        });
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    out
}

/// The limits it runs under: 2 s of processor, 256 MB of memory (it stops itself past them).
fn box_in() {
    unsafe {
        let cpu = libc::rlimit { rlim_cur: 2, rlim_max: 3 };
        libc::setrlimit(libc::RLIMIT_CPU, &cpu);
        let mem = libc::rlimit { rlim_cur: 256 << 20, rlim_max: 256 << 20 };
        libc::setrlimit(libc::RLIMIT_AS, &mem);
    }
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    // CI's pictures: what the address would answer, from a folder of files (HEARTH_BLOCK_FAKE: "<dir>", the URL's path)
    if let Ok(d) = std::env::var("HEARTH_BLOCK_FAKE") {
        let name = url.split("://").nth(1).and_then(|r| r.split_once('/')).map(|(_, p)| p.replace(['/', '?', '&', '='], "_")).unwrap_or_default();
        return std::fs::read(std::path::Path::new(&d).join(name)).map_err(|e| e.to_string());
    }
    let out = Command::new("curl")
        .args(["-sS", "--max-time", "6", "--max-filesize", "2000000", "--proto", "=http,https", "--max-redirs", "0", url])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("couldn't reach it ({})", String::from_utf8_lossy(&out.stderr).trim().chars().take(120).collect::<String>()));
    }
    Ok(out.stdout)
}

/// One look (`hearth-layout run-block ID`, boxed in): {ok, title, lines, progress, picture, chip} or {asks} or {error}.
/// `once`: you said Just this once (for this look only).
pub fn run(id: &str, once: bool) -> Value {
    box_in();
    let Some(m) = get(id) else { return json!({ "ok": false, "error": "That block isn't here any more." }) };
    if !m.on {
        return json!({ "ok": false, "off": true, "error": "Off in Settings › Your desktop › Your blocks." });
    }
    let said = crate::settings_file::read_json(&allowed_file()).get(id).and_then(Value::as_str).unwrap_or("").to_string();
    if !m.reach.kind.is_empty() && said != "always" && !once {
        return json!({ "ok": false, "no": said == "no", "asks": asks(&m) });
    }
    let (mut data, mut text) = (Value::Null, String::new());
    let base = match m.reach.kind.as_str() {
        "address" => format!("http://{}", m.reach.value),
        _ => String::new(),
    };
    let got: Result<Vec<u8>, String> = match m.reach.kind.as_str() {
        "address" => fetch(&format!("{base}{}", m.path)),
        "internet" => fetch(&m.reach.value),
        "file" => {
            let p = crate::paths::home().join(m.reach.value.trim_start_matches("~/"));
            match std::fs::metadata(&p) {
                Ok(md) if md.len() <= 1_000_000 => std::fs::read(&p).map_err(|e| e.to_string()),
                Ok(_) => Err("the file is over 1 MB".into()),
                Err(e) => Err(e.to_string()),
            }
        }
        _ => Ok(Vec::new()),
    };
    match got {
        Ok(b) => {
            text = String::from_utf8_lossy(&b).chars().take(4000).collect();
            data = serde_json::from_slice(&b).unwrap_or(Value::Null);
        }
        Err(e) => return json!({ "ok": false, "error": format!("{}: {e}", m.name) }),
    }
    let progress = if m.shows.progress.is_empty() { Value::Null } else {
        pick(&data, &m.shows.progress).and_then(Value::as_f64).map(|f| json!(if f <= 1.0 { f * 100.0 } else { f.min(100.0) })).unwrap_or(Value::Null)
    };
    json!({
        "ok": true, "name": m.name,
        "title": fill(&m.shows.title, &data, &text),
        "lines": m.shows.lines.iter().map(|l| fill(l, &data, &text)).collect::<Vec<_>>(),
        "progress": progress,
        "picture": if m.shows.picture.is_empty() || base.is_empty() { Value::Null } else { json!(format!("{base}{}", m.shows.picture)) },
        "chip": m.shows.chip, "every": m.every,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn printer() -> Value {
        json!({"format": 1, "id": "printer-camera", "name": "Printer camera", "asked": "Add my 3D printer's camera and how far the print is, in the corner",
               "reach": {"kind": "address", "value": "192.168.1.40", "what": "your printer"}, "path": "/api/job",
               "shows": {"title": "{job.file.display} · {progress.completion}%", "lines": ["{progress.printTimeLeft} s left"], "progress": "progress.completion",
                         "picture": "/webcam/?action=snapshot", "chip": "Live · Anycubic Chiron"}, "every": 10})
    }

    #[test]
    fn a_printer_block_is_good_and_asks_first() {
        let m = parse(&printer()).unwrap();
        assert_eq!(asks(&m)["title"], "It asks to reach your printer at 192.168.1.40");
        let d = json!({"job": {"file": {"display": "Quad Mech base plate"}}, "progress": {"completion": 62, "printTimeLeft": 6480}});
        assert_eq!(fill(&m.shows.title, &d, ""), "Quad Mech base plate · 62%");
    }

    #[test]
    fn it_only_reaches_what_it_says() {
        let mut v = printer();
        v["reach"]["value"] = json!("8.8.8.8");
        assert!(parse(&v).is_err(), "not a home address");
        let mut v = printer();
        v["reach"] = json!({"kind": "file", "value": "~/.ssh/id_ed25519"});
        assert!(parse(&v).is_err(), "never a key");
        let mut v = printer();
        v["reach"] = json!({"kind": "internet", "value": "http://example.com"});
        assert!(parse(&v).is_err(), "https only");
        let mut v = printer();
        v["reach"] = json!({"kind": "shell", "value": "rm -rf ~"});
        assert!(parse(&v).is_err());
        assert!(home_address("printer.local:8080") && home_address("10.0.0.5") && !home_address("172.40.1.1"));
    }

    #[test]
    fn it_asks_before_it_reaches_anything_and_remembers_allow() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path());
        let m = parse(&printer()).unwrap();
        keep(&m).unwrap();
        let r = run("printer-camera", false);
        assert!(r.get("asks").is_some(), "{r}");
        allow("printer-camera", "no").unwrap();
        assert_eq!(run("printer-camera", false)["no"], true);
        set_on("printer-camera", false).unwrap();
        assert_eq!(run("printer-camera", false)["off"], true);
        assert_eq!(mine()[0]["allowed"], "no");
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
