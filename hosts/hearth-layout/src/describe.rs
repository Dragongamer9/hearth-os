//! Describe it (docs/mockups/desktop-layouts boards 7-9): what you type (and a sketch or picture, when you add one)
//! turned into a layout or a new feature by your AI, then checked here before anything is shown. Every Hearth app
//! uses this same part: Mantel's bars and panels, and each app's own parts (the "part" asked about).
//!
//! The AI is yours: Describe it's own choice in Settings (~/.config/nimbus/describe.json: {"url", "model", "label"},
//! any OpenAI-style address: Hearth AI on your Nvidia PC through Hearth Link, a local engine, another AI), else the
//! default engine Hearth already has (Settings › AI engines: hearth_ai_engines.py default). Nothing is sent anywhere
//! else, and nothing is put in place here: the answer is only a suggestion, shown as NEW next to NOW, until Keep this.
//!
//! The AI may only answer with Hearth's own parts: the layout's settings and blocks (layout.rs, blocks.rs), or a
//! feature's sources and actions (feature.rs). Its answer is checked like any layout file; a wrong one is asked for
//! once more with the reasons, and anything Hearth can't do yet comes back as "needs something new" with the
//! closest it can make and the idea to send (idea.rs), never as code.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::{blocks, feature, idea, layout};

/// Who answers.
#[derive(Debug, Clone, PartialEq)]
pub struct Engine {
    pub base: String,
    pub model: String,
    pub label: String,
}

fn share() -> std::path::PathBuf {
    std::env::var_os("NIMBUS_SHARE").map(std::path::PathBuf::from).unwrap_or_else(|| "/usr/share/nimbus".into())
}

/// Describe it's AI: your choice, else Hearth's default engine, else none (the page says how to pick one).
pub fn engine() -> Option<Engine> {
    if std::env::var_os("HEARTH_DESCRIBE_FAKE").is_some() {
        return Some(Engine { base: String::new(), model: String::new(), label: "the answer given for this check".into() });
    }
    if let Ok(u) = std::env::var("HEARTH_DESCRIBE_URL") {
        return Some(Engine { base: u, model: std::env::var("HEARTH_DESCRIBE_MODEL").unwrap_or_default(), label: "the AI set for this test".into() });
    }
    let own = crate::settings_file::read_json(&crate::paths::home().join(".config/nimbus/describe.json"));
    if let Some(u) = own.get("url").and_then(Value::as_str).filter(|u| u.starts_with("http://") || u.starts_with("https://")) {
        return Some(Engine {
            base: u.trim_end_matches('/').to_string(),
            model: own.get("model").and_then(Value::as_str).unwrap_or("").to_string(),
            label: own.get("label").and_then(Value::as_str).unwrap_or("your AI").to_string(),
        });
    }
    let out = Command::new("python3").arg(share().join("desktop/shell/hearth_ai_engines.py")).arg("default").stderr(Stdio::null()).output().ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    let base = v.get("openai").or_else(|| v.get("base")).and_then(Value::as_str)?.trim_end_matches('/').to_string();
    Some(Engine { base, model: v.get("model").and_then(Value::as_str).unwrap_or("").to_string(), label: v.get("label").and_then(Value::as_str).unwrap_or("Hearth's AI engine").to_string() })
}

/// What the AI is told: the rules and every part it may use.
pub fn rules(part: &str) -> String {
    let keys: Vec<String> = layout::DESKTOP_KEYS.iter().map(|(k, v)| format!("{k}: {}", v.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(" | "))).collect();
    let blocks: Vec<String> = blocks::BLOCKS.iter().map(|b| format!("{} ({}; {})", b.id, b.name, match b.place {
        blocks::Where::Bar => "main bar",
        blocks::Where::Panel => "panels",
        blocks::Where::Both => "main bar or panels",
    })).collect();
    let sources: Vec<String> = feature::SOURCES.iter().map(|(id, n)| format!("{id} ({n})")).collect();
    let actions: Vec<String> = feature::ACTIONS.iter().map(|(id, n)| format!("{id} ({n})")).collect();
    format!(
        "You change Hearth OS's desktop for its owner, using only Hearth's own parts. The part being changed: {part}.\n\
Answer with ONE JSON object and nothing else, one of:\n\
1. A layout: {{\"kind\": \"layout\", \"layout\": <layout>, \"changes\": [\"short plain sentences of what changed\"]}}\n\
   <layout> = {{\"format\": 1, \"id\": \"<lowercase-dashes>\", \"name\": \"...\", \"desktop\": {{...}}, \"bar\": {{\"start\": [...], \"centre\": [...], \"end\": [...]}}, \"panels\": [{{\"edge\", \"items\", \"size\" 28-96, \"autoHide\", \"style\" \"\"|\"floating\"|\"centred\"}}], \"windowButtons\": {{\"side\": \"left\"|\"right\", \"buttons\": [\"iconify\", \"max\", \"close\"]}}}}\n\
   desktop keys: {}\n\
   blocks: {}\n\
   Rules: the app menu (menu) and Game Mode always stay; up to four panels, none on the main bar's edge or on the dock's (bottom), one per edge; windows keep a close button.\n\
2. A new feature: {{\"kind\": \"feature\", \"feature\": {{\"format\": 1, \"id\", \"name\", \"part\", \"shows\": [{{\"source\", \"label\"}}], \"every\": seconds, \"click\": {{\"action\", \"target\"}}}}, \"changes\": [...]}}\n\
   sources: {}\n\
   actions: {}\n\
3. When it needs something Hearth doesn't have: {{\"kind\": \"needs\", \"needs\": [\"what's missing, in a few words\"], \"why\": \"one or two plain sentences why Hearth can't do it yet\", \"closest\": <feature or null>, \"changes\": []}}\n\
Never write code, commands or file paths. Use plain words for the owner in \"changes\".",
        keys.join("; "),
        blocks.join(", "),
        sources.join(", "),
        actions.join(", ")
    )
}

/// One OpenAI-style chat call through curl (no password or key: your own AI on your own network), or the CI's
/// canned answer (HEARTH_DESCRIBE_FAKE: a file with what the AI would say).
pub fn chat(e: &Engine, system: &str, user: &str, sketch: Option<&std::path::Path>) -> Result<String, String> {
    if let Ok(f) = std::env::var("HEARTH_DESCRIBE_FAKE") {
        return std::fs::read_to_string(f).map_err(|e| e.to_string());
    }
    let mut content: Value = json!(user);
    if let Some(s) = sketch {
        let b = std::fs::read(s).map_err(|e| format!("The sketch couldn't be read ({e})."))?;
        if b.len() > 6 * 1024 * 1024 {
            return Err("The sketch is too big (6 MB at most).".into());
        }
        let mime = if s.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("png")).unwrap_or(false) { "image/png" } else { "image/jpeg" };
        content = json!([{ "type": "text", "text": user }, { "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{}", b64(&b)) } }]);
    }
    let body = json!({ "model": e.model, "temperature": 0.2, "messages": [{ "role": "system", "content": system }, { "role": "user", "content": content }] });
    let mut child = Command::new("curl")
        .args(["-sS", "--max-time", "180", "-H", "Content-Type: application/json", "--data-binary", "@-", &format!("{}/chat/completions", e.base)])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't reach your AI ({e})."))?;
    child.stdin.take().map(|mut i| i.write_all(body.to_string().as_bytes()));
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Your AI didn't answer ({}).", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|_| "Your AI's answer couldn't be read.".to_string())?;
    v.pointer("/choices/0/message/content").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "Your AI answered with nothing.".into())
}

fn b64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len() * 4 / 3 + 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// The first JSON object in the AI's words (it may wrap it in ``` or a sentence).
pub fn first_object(t: &str) -> Option<Value> {
    let start = t.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, ch) in t[start..].char_indices() {
        if in_str {
            match (esc, ch) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return serde_json::from_str(&t[start..start + i + 1]).ok();
                }
            }
            _ => {}
        }
    }
    None
}

/// What came back, checked: ready to show as NEW, or what's needed and the idea to send.
pub fn check_answer(v: &Value, part: &str, asked: &str, sketch: bool) -> Result<Value, Vec<String>> {
    let changes: Vec<String> = v.get("changes").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.chars().take(200).collect())).take(12).collect()).unwrap_or_default();
    // the app asking and its version ("Mantel in Rust 1.17.1"), given by the app that runs Describe it
    let versions = (hearth_version(), std::env::var("HEARTH_DESCRIBE_APP").unwrap_or_default().chars().take(40).collect::<String>());
    match v.get("kind").and_then(Value::as_str) {
        Some("layout") => {
            let l: layout::Layout = serde_json::from_value(v.get("layout").cloned().unwrap_or(Value::Null)).map_err(|e| vec![format!("The layout couldn't be read ({e}).")])?;
            let errs = layout::check(&l);
            if !errs.is_empty() {
                return Err(errs);
            }
            Ok(json!({ "ok": true, "kind": "layout", "layout": l, "changes": changes, "coming": l.not_built() }))
        }
        Some("feature") => {
            let f: feature::Feature = serde_json::from_value(v.get("feature").cloned().unwrap_or(Value::Null)).map_err(|e| vec![format!("The feature couldn't be read ({e}).")])?;
            let c = feature::check(&f);
            if !c.problems.is_empty() {
                return Err(c.problems);
            }
            if !c.needs.is_empty() {
                let i = idea::Idea::new(part, asked, c.needs.clone(), Some(f.clone()), sketch, &versions.0, &versions.1);
                return Ok(json!({ "ok": true, "kind": "needs", "needs": c.needs, "closest": f, "idea": i, "preview": i.preview(), "changes": changes }));
            }
            Ok(json!({ "ok": true, "kind": "feature", "feature": f, "described": feature::describe(&f), "changes": changes }))
        }
        Some("needs") => {
            let needs: Vec<String> = v.get("needs").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.chars().take(120).collect())).take(8).collect()).unwrap_or_default();
            let closest: Option<feature::Feature> = v.get("closest").cloned().and_then(|c| serde_json::from_value(c).ok()).filter(|f| feature::check(f).usable());
            let why: String = v.get("why").and_then(Value::as_str).unwrap_or("").chars().take(400).collect();
            let i = idea::Idea::new(part, asked, needs.clone(), closest.clone(), sketch, &versions.0, &versions.1);
            Ok(json!({ "ok": true, "kind": "needs", "needs": needs, "why": why, "closest": closest, "closestDescribed": closest.as_ref().map(feature::describe), "idea": i, "preview": i.preview(), "changes": changes }))
        }
        _ => Err(vec!["The answer wasn't a layout, a feature or what's needed.".into()]),
    }
}

/// Describe it for a whole screen (build-anything boards 4-6): your words (and a sketch) and the screen as it is now, to
/// a new screen by your AI, with any blocks it had to make (made.rs), all checked. Nothing is put in place here.
pub fn describe_screen(text: &str, sketch: Option<&std::path::Path>, current: &Value) -> Value {
    let text: String = text.trim().chars().take(2000).collect();
    if text.is_empty() && sketch.is_none() {
        return json!({ "ok": false, "error": "Say what you'd like, or add a sketch." });
    }
    let Some(e) = engine() else {
        return json!({ "ok": false, "error": "No AI is set for Describe it: pick one in Settings › Personalization › Desktop layout, or turn on an AI engine in Settings › Home server." });
    };
    let system = format!(
        "You build a whole screen for Hearth OS's desktop (or Game Mode) from Hearth's own blocks, for its owner. It can look like anything they ask (\"like a console\", \"a cabin\", \"my first PC\"), \
always made of Hearth's own icons, sounds and fonts: never another company's logos or names. Answer with ONE JSON object and nothing else:\n\
{{\"kind\": \"screen\", \"screen\": <screen>, \"make\": [<new block>, ...], \"changes\": [\"short plain sentences of what changed\"]}}\n{}\n{}\n\
Start from the screen they have now (given) and change what they asked; keep the rest. Use \"make\" only for blocks Hearth hasn't got (their id is the own:<id> in the screen).",
        crate::screen::rules(),
        crate::made::rules()
    );
    let mut user = format!("The screen now: {current}\nThe owner asks: {text}");
    for attempt in 0..2 {
        let reply = match chat(&e, &system, &user, sketch) {
            Ok(r) => r,
            Err(m) => return json!({ "ok": false, "error": m, "engine": e.label }),
        };
        let Some(v) = first_object(&reply) else {
            user = format!("{user}\nYour last answer wasn't one JSON object. Answer again with only the JSON object.");
            continue;
        };
        let mut errs = Vec::new();
        let screen = crate::screen::parse(&v.get("screen").cloned().unwrap_or(Value::Null));
        let mut made = Vec::new();
        for b in v.get("make").and_then(Value::as_array).cloned().unwrap_or_default() {
            match crate::made::parse(&b) {
                Ok(mut m) => {
                    m.asked = text.clone();
                    if m.by.is_empty() {
                        m.by = e.label.clone();
                    }
                    made.push(m);
                }
                Err(x) => errs.extend(x),
            }
        }
        match screen {
            Ok(mut s) => {
                for own in crate::screen::own_blocks(&s) {
                    if !made.iter().any(|m| m.id == own) && crate::made::get(&own).is_none() {
                        errs.push(format!("own:{own} is used but not made."));
                    }
                }
                if errs.is_empty() {
                    if s.said.is_empty() {
                        s.said = text.clone();
                    }
                    let changes: Vec<String> = v.get("changes").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.chars().take(200).collect())).take(12).collect()).unwrap_or_default();
                    let asks: Vec<Value> = made.iter().map(crate::made::asks).filter(|a| !a.is_null()).collect();
                    return json!({ "ok": true, "kind": "screen", "screen": s, "made": made, "asks": asks, "changes": changes, "engine": e.label });
                }
            }
            Err(x) => errs.extend(x),
        }
        if attempt == 0 {
            user = format!("{user}\nYour last answer can't be used: {}. Answer again, fixed.", errs.join(" "));
        } else {
            return json!({ "ok": false, "error": format!("Your AI's screen couldn't be used: {}", errs.join(" ")), "engine": e.label });
        }
    }
    json!({ "ok": false, "error": "Your AI didn't give a screen Hearth can use. Try again, or say it another way.", "engine": e.label })
}

/// This Hearth OS's version (os-release), for the idea.
pub fn hearth_version() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("HEARTH_VERSION=").or_else(|| l.strip_prefix("VERSION_ID=")).map(|v| v.trim_matches('"').to_string())))
        .unwrap_or_default()
}

/// The whole of Describe it: ask, check, ask once more with the reasons when the answer was wrong.
pub fn describe(part: &str, text: &str, sketch: Option<&std::path::Path>, current: &Value) -> Value {
    let text: String = text.trim().chars().take(2000).collect();
    if text.is_empty() && sketch.is_none() {
        return json!({ "ok": false, "error": "Type what you'd like, or add a sketch." });
    }
    let Some(e) = engine() else {
        return json!({ "ok": false, "error": "No AI is set for Describe it: pick one in Settings › Personalization › Desktop layout, or turn on an AI engine in Settings › Home server." });
    };
    let system = rules(part);
    let mut user = format!("Now: {}\nThe owner asks: {}", current, text);
    for attempt in 0..2 {
        let reply = match chat(&e, &system, &user, sketch) {
            Ok(r) => r,
            Err(m) => return json!({ "ok": false, "error": m, "engine": e.label }),
        };
        let Some(v) = first_object(&reply) else {
            user = format!("{user}\nYour last answer wasn't one JSON object. Answer again with only the JSON object.");
            continue;
        };
        match check_answer(&v, part, &text, sketch.is_some()) {
            Ok(mut ok) => {
                ok["engine"] = json!(e.label);
                return ok;
            }
            Err(why) if attempt == 0 => user = format!("{user}\nYour last answer can't be used: {}. Answer again, fixed.", why.join(" ")),
            Err(why) => return json!({ "ok": false, "error": format!("Your AI's answer couldn't be used: {}", why.join(" ")), "engine": e.label }),
        }
    }
    json!({ "ok": false, "error": "Your AI didn't give an answer Hearth can use. Try again, or say it another way.", "engine": e.label })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_json_is_found_in_the_ais_words() {
        let v = first_object("Sure! ```json\n{\"kind\": \"needs\", \"needs\": [\"moon {phase}\"]}\n``` hope that helps").unwrap();
        assert_eq!(v["needs"][0], "moon {phase}");
        assert!(first_object("no json here").is_none());
    }

    #[test]
    fn a_layout_answer_is_checked_like_a_file() {
        let good = json!({"kind": "layout", "changes": ["Bar at the bottom"], "layout": {"format": 1, "id": "mine", "name": "Mine", "desktop": {"barLayout": "one", "barEdge": "bottom", "dockFloat": "off"}}});
        let r = check_answer(&good, "bar", "bar at the bottom", false).unwrap();
        assert_eq!(r["kind"], "layout");
        let bad = json!({"kind": "layout", "layout": {"format": 1, "id": "mine", "name": "Mine", "bar": {"hidden": ["menu"]}}});
        assert!(check_answer(&bad, "bar", "hide the flame", false).is_err());
    }

    #[test]
    fn what_hearth_cant_do_becomes_an_idea() {
        let v = json!({"kind": "feature", "feature": {"format": 1, "id": "moon", "name": "Moon", "part": "calendar", "shows": [{"source": "moon-phase"}]}});
        let r = check_answer(&v, "calendar", "show moon phases, mail me@example.com", true).unwrap();
        assert_eq!(r["kind"], "needs");
        assert!(r["preview"].as_str().unwrap().contains("Sketch: yes"));
        assert!(!r["preview"].as_str().unwrap().contains("me@example.com"));
    }

    #[test]
    fn base64_as_everyone_writes_it() {
        assert_eq!(b64(b"Man"), "TWFu");
        assert_eq!(b64(b"Ma"), "TWE=");
        assert_eq!(b64(b"M"), "TQ==");
    }

    #[test]
    fn the_rules_name_every_part() {
        let r = rules("bar");
        for b in blocks::BLOCKS {
            assert!(r.contains(b.id));
        }
        assert!(r.contains("gpu-temp") && r.contains("open-app"));
    }
}
