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
use std::os::unix::fs::OpenOptionsExt;
use std::process::{Command, Stdio};

use crate::{blocks, feature, idea, layout};

/// Who answers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Engine {
    pub base: String,
    pub model: String,
    pub label: String,
    /// an online AI's key, read from the system's password store when it's used (never kept in a file or a log)
    pub key: String,
    /// where it is: "here" (an engine on this device), "link" (your other device over Hearth Link), "online", or ""
    /// (your own address, or a test's)
    pub from: String,
}

fn share() -> std::path::PathBuf {
    std::env::var_os("NIMBUS_SHARE").map(std::path::PathBuf::from).unwrap_or_else(|| "/usr/share/nimbus".into())
}

/// When no AI answers: what to do, in plain words (Settings › AI's own words once it's built).
pub const NO_AI: &str = "No AI is answering. Turn on Ollama in Settings › Home server, or use your PC's AI: Settings › Hearth Link › Local AI › Use its AI.";

/// Describe it's AI: engine_for("describe").
pub fn engine() -> Option<Engine> {
    engine_for("describe")
}

/// Settings › AI's own picks (~/.config/nimbus/ai-uses.json, written only by Settings through hearth_settings_io):
/// {"uses": {USE: {"url", "model", "label"} | {"online": provider}}, "order": ["here", "link", "online"]}.
fn uses_file() -> (Value, Vec<String>) {
    let v = crate::settings_file::read_json(&crate::paths::home().join(".config/nimbus/ai-uses.json"));
    let mut order: Vec<String> = v.get("order").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).filter(|x| ORDER.contains(x)).map(str::to_string).collect()).unwrap_or_default();
    for k in ORDER {
        if !order.iter().any(|o| o == k) {
            order.push(k.to_string());
        }
    }
    (v.get("uses").cloned().unwrap_or(Value::Null), order)
}

/// Where Hearth's AI comes from, first that answers (Settings › AI › Where your AI comes from): this device's engines,
/// your devices over Hearth Link, an online AI with your key, unless you dragged them into another order.
pub const ORDER: [&str; 3] = ["here", "link", "online"];

/// The AI one use goes to (describe: Describe it and Build anything; sortdl: Sort my Downloads; photos; ithil): its own
/// pick in Settings › AI when that answers (Describe it's older describe.json too), else Hearth's AI. A model that can
/// only search (nomic-embed-text and the like) is never picked for writing. None answering: None (NO_AI says why).
pub fn engine_for(use_: &str) -> Option<Engine> {
    if std::env::var_os("HEARTH_DESCRIBE_FAKE").is_some() {
        // (the waiting screen's pictures name an engine and where it is: HEARTH_DESCRIBE_FAKE_LABEL, _FROM)
        let label = std::env::var("HEARTH_DESCRIBE_FAKE_LABEL").unwrap_or_else(|_| "the answer given for this check".into());
        return Some(Engine { label, from: std::env::var("HEARTH_DESCRIBE_FAKE_FROM").unwrap_or_default(), ..Default::default() });
    }
    if let Ok(u) = std::env::var("HEARTH_DESCRIBE_URL") {
        return Some(Engine { base: u, model: std::env::var("HEARTH_DESCRIBE_MODEL").unwrap_or_default(), label: "the AI set for this test".into(), ..Default::default() });
    }
    let (uses, order) = uses_file();
    let mut pick = uses.get(use_).cloned().filter(Value::is_object);
    if pick.is_none() && use_ == "describe" {
        pick = Some(crate::settings_file::read_json(&crate::paths::home().join(".config/nimbus/describe.json"))).filter(|v| v.get("url").is_some());
    }
    if let Some(p) = pick {
        // Describe it uses… (Settings › AI): where it asks first, "Fastest available" when it's not there
        if let Some(from) = p.get("from").and_then(Value::as_str) {
            return fastest(&order, ORDER.iter().find(|k| **k == from).copied());
        }
        if let Some(provider) = p.get("online").and_then(Value::as_str) {
            if let Some(e) = online().filter(|_| online_provider().as_deref() == Some(provider)) {
                return Some(e);
            }
        } else if let Some(u) = p.get("url").and_then(Value::as_str).filter(|u| u.starts_with("http://") || u.starts_with("https://")) {
            let base = u.trim_end_matches('/').to_string();
            if let Some(have) = models(&base) {
                let want = p.get("model").and_then(Value::as_str).unwrap_or("").to_string();
                let model = if !want.is_empty() && have.iter().any(|m| *m == want) { want } else { pick_model(&have) };
                if !model.is_empty() {
                    return Some(Engine { base, model, label: p.get("label").and_then(Value::as_str).unwrap_or("your AI").to_string(), ..Default::default() });
                }
            }
        }
        // its own pick isn't answering: Hearth's AI answers instead
    }
    if use_ == "describe" {
        return fastest(&order, None);
    }
    hearths_ai(&order)
}

/// Hearth's AI: the first that answers, in your order.
fn hearths_ai(order: &[String]) -> Option<Engine> {
    order.iter().find_map(|k| from_source(k))
}

/// Describe it's AI (the owner, Nov 24: his laptop's Ollama on its CPU took 20-48 s an answer while his PC's was there):
/// FIRST when the person picked where it asks first and that answers; else your order, except that a slow engine on
/// this device (is_slow) is asked only when nothing faster answers.
pub fn fastest(order: &[String], first: Option<&str>) -> Option<Engine> {
    if let Some(e) = first.and_then(from_source) {
        return Some(e);
    }
    let mut slow: Option<Engine> = None;
    for k in order {
        if Some(k.as_str()) == first {
            continue;
        }
        let Some(e) = from_source(k) else { continue };
        if e.from == "here" && is_slow(&e.model) {
            slow.get_or_insert(e);
            continue;
        }
        return Some(e);
    }
    slow
}

/// Under 8 words a second (measured by hearth-ai-check speed, or as Describe it found it), or, not measured yet, no
/// graphics card it can run on.
pub const SLOW_WORDS: f64 = 8.0;

pub fn is_slow(model: &str) -> bool {
    match word_speed(model) {
        Some(w) => w < SLOW_WORDS,
        None => !has_gpu(),
    }
}

/// The speeds file, read plainly (it's a cache hearth-ai-check and Describe it write whole, not a setting): {} when
/// it's missing or can't be read.
fn read_speeds() -> Value {
    std::fs::read(speed_file()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}))
}

fn speed_file() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".cache")).join("hearth/ai-speed.json")
}

/// How many words a second this model writes here: measured (hearth-ai-check speed "words"), else as Describe it
/// found it ("felt": its whole answers, reading the question included).
pub fn word_speed(model: &str) -> Option<f64> {
    if let Some(w) = std::env::var("HEARTH_DESCRIBE_FAKE_WORDS").ok().and_then(|v| v.parse::<f64>().ok()) {
        return Some(w);
    }
    let v = read_speeds();
    let m = v.get(model)?;
    m.get("words").and_then(Value::as_f64).filter(|w| *w > 0.0).or_else(|| m.get("felt").and_then(Value::as_f64).filter(|w| *w > 0.0))
}

/// Describe it's own measure of a model here (its "felt" speed; hearth-ai-check's measured one stays as it is).
fn note_speed(model: &str, words: f64) {
    if model.is_empty() || !(words > 0.0) {
        return;
    }
    let f = speed_file();
    let mut v = read_speeds();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let e = v.as_object_mut().unwrap().entry(model.to_string()).or_insert(json!({}));
    if !e.is_object() {
        *e = json!({});
    }
    e["felt"] = json!((words * 10.0).round() / 10.0);
    e["feltAt"] = json!(now);
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = f.with_extension("json.tmp");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &f);
    }
}

/// A graphics card an engine here can use: Nvidia's driver, or a card with 4 GB of its own memory or more.
fn has_gpu() -> bool {
    if std::fs::read_dir("/proc/driver/nvidia/gpus").map(|mut d| d.next().is_some()).unwrap_or(false) {
        return true;
    }
    std::fs::read_dir("/sys/class/drm")
        .map(|d| {
            d.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.starts_with("card") && !n.contains('-') && std::fs::read_to_string(e.path().join("device/mem_info_vram_total")).ok().and_then(|t| t.trim().parse::<u64>().ok()).map(|b| b >= 4 << 30).unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// The first that answers in one place: "here", "link" or "online".
fn from_source(k: &str) -> Option<Engine> {
    let mut e = {
        match k {
            "here" => {
                // Hearth's engines on this PC, the default first (hearth_ai_engines.py list)
                let mut tries: Vec<(String, String, String)> = Vec::new();
                if let Some(v) = Command::new("python3").arg(share().join("desktop/shell/hearth_ai_engines.py")).arg("list").stderr(Stdio::null()).output().ok().and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok()) {
                    let mut list: Vec<&Value> = v.get("engines").and_then(Value::as_array).map(|a| a.iter().collect()).unwrap_or_default();
                    list.sort_by_key(|e| e.get("default") != Some(&json!(true)));
                    for e in list {
                        if let Some(b) = e.get("openai").or_else(|| e.get("base")).and_then(Value::as_str) {
                            tries.push((b.trim_end_matches('/').to_string(), e.get("model").and_then(Value::as_str).unwrap_or("").to_string(), e.get("label").and_then(Value::as_str).unwrap_or("Hearth's AI engine").to_string()));
                        }
                    }
                }
                first_answering(tries)
            }
            "link" => first_answering(vec![("http://127.0.0.1:11435/v1".into(), String::new(), "your other device's AI (Hearth Link)".into())]),
            "online" => online(),
            _ => None,
        }
    }?;
    e.from = k.to_string();
    Some(e)
}

fn first_answering(tries: Vec<(String, String, String)>) -> Option<Engine> {
    for (base, model, label) in tries {
        let Some(have) = models(&base) else { continue };
        let model = if !model.is_empty() && have.iter().any(|m| *m == model) { model } else { pick_model(&have) };
        if model.is_empty() {
            continue;
        }
        return Some(Engine { base, model, label, ..Default::default() });
    }
    None
}

fn online_provider() -> Option<String> {
    crate::settings_file::read_json(&crate::paths::home().join(".config/nimbus/ai-online.json")).get("provider").and_then(Value::as_str).map(str::to_string)
}

/// An online AI with your own key (Settings › AI: ~/.config/nimbus/ai-online.json {"provider", "model"}; the key in the
/// system's password store: secret-tool, service hearth-ai, account <provider>). Last, after every AI of your own.
pub const ONLINE: &[(&str, &str, &str)] = &[
    ("claude", "https://api.anthropic.com/v1", "Claude"),
    ("chatgpt", "https://api.openai.com/v1", "ChatGPT"),
    ("gemini", "https://generativelanguage.googleapis.com/v1beta/openai", "Gemini"),
];

fn online() -> Option<Engine> {
    let c = crate::settings_file::read_json(&crate::paths::home().join(".config/nimbus/ai-online.json"));
    let provider = c.get("provider").and_then(Value::as_str)?;
    let (_, base, label) = ONLINE.iter().find(|(p, _, _)| *p == provider)?;
    let model = c.get("model").and_then(Value::as_str).unwrap_or("").to_string();
    if model.is_empty() {
        return None;
    }
    let out = Command::new("secret-tool").args(["lookup", "service", "hearth-ai", "account", provider]).stderr(Stdio::null()).output().ok()?;
    let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if key.is_empty() {
        return None;
    }
    Some(Engine { base: base.to_string(), model, label: format!("{label} (online, your key)"), key, from: "online".into() })
}

/// The models an OpenAI-style address has (GET /models, 3 s at most); None when nothing answers there.
fn models(base: &str) -> Option<Vec<String>> {
    let out = Command::new("curl").args(["-sS", "-m", "3", &format!("{base}/models")]).stderr(Stdio::null()).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(v.get("data").and_then(Value::as_array)?.iter().filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string)).collect())
}

/// A model that can write: never one only for search or speech; Qwen, Llama, Gemma or Mistral first; else the first.
pub fn pick_model(have: &[String]) -> String {
    let writes = |m: &&String| {
        let l = m.to_lowercase();
        !["embed", "nomic", "bge", "minilm", "rerank", "whisper", "clip", "tts"].iter().any(|w| l.contains(w))
    };
    let ok: Vec<&String> = have.iter().filter(writes).collect();
    for want in ["qwen3", "qwen2.5", "llama3", "gemma", "mistral", "qwen", "llama"] {
        if let Some(m) = ok.iter().find(|m| m.to_lowercase().contains(want)) {
            return (*m).clone();
        }
    }
    ok.first().map(|m| (*m).clone()).unwrap_or_default()
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

/// How long Describe it waits for its AI: one answer at most PER_ASK seconds, both tries together at most TOTAL
/// (HEARTH_DESCRIBE_LIMIT changes TOTAL), then it says so instead of waiting on in silence.
pub const PER_ASK: u64 = 150;
pub const TOTAL: u64 = 240;
static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Who's answering, for the app that waits (HEARTH_DESCRIBE_STATUS: a file it reads while it waits): the AI's name,
/// its model, where it is and how fast it writes here when that's known.
fn note_status(e: &Engine, attempt: u32) {
    let Some(f) = std::env::var_os("HEARTH_DESCRIBE_STATUS").map(std::path::PathBuf::from) else { return };
    let words = if e.from == "here" || std::env::var_os("HEARTH_DESCRIBE_FAKE").is_some() { word_speed(&e.model) } else { None };
    let v = json!({ "engine": e.label, "model": e.model, "from": e.from, "words": words, "attempt": attempt, "slow": e.from == "here" && is_slow(&e.model) });
    let tmp = f.with_extension("tmp");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &f);
    }
}

/// "Your AI took too long…" in plain words, with how fast it writes here when that's known.
pub fn too_slow(e: &Engine, secs: u64) -> String {
    let speed = if e.from == "here" { word_speed(&e.model).map(|w| format!(" It writes about {} words a second on this device.", w.round().max(1.0) as i64)).unwrap_or_default() } else { String::new() };
    format!("{} took too long, so Hearth stopped waiting after {secs} s.{speed} Pick a faster AI in Settings › AI › Describe it uses.", cap(&e.label))
}

/// Words with the key taken out (a provider that says it back in an error).
fn hide(t: &str, key: &str) -> String {
    if key.is_empty() { t.to_string() } else { t.replace(key, "•••") }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "Your AI".into(),
    }
}

/// One OpenAI-style chat call through curl, or the CI's canned answer (HEARTH_DESCRIBE_FAKE: a file with what the AI
/// would say; HEARTH_DESCRIBE_FAKE_WAIT: seconds it takes, for the waiting screen's pictures).
pub fn chat(e: &Engine, system: &str, user: &str, sketch: Option<&std::path::Path>) -> Result<String, String> {
    let started = *STARTED.get_or_init(std::time::Instant::now);
    let total = std::env::var("HEARTH_DESCRIBE_LIMIT").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(TOTAL);
    let used = started.elapsed().as_secs();
    if used + 15 > total {
        return Err(too_slow(e, used));
    }
    let limit = PER_ASK.min(total - used);
    note_status(e, if used == 0 { 1 } else { 2 });
    if let Ok(f) = std::env::var("HEARTH_DESCRIBE_FAKE") {
        if let Some(w) = std::env::var("HEARTH_DESCRIBE_FAKE_WAIT").ok().and_then(|v| v.parse::<u64>().ok()) {
            if w >= limit {
                std::thread::sleep(std::time::Duration::from_secs(limit));
                return Err(too_slow(e, limit));
            }
            std::thread::sleep(std::time::Duration::from_secs(w));
        }
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
    // "/no_think": Qwen3 answers at once instead of thinking out loud first (other models pass over it)
    let mut system = format!("{system}\n/no_think");
    let mut body = json!({ "model": e.model, "temperature": 0.2, "messages": [{ "role": "system", "content": "" }, { "role": "user", "content": content }] });
    if e.from != "online" {
        // a short answer is a quick one on a slow engine: compact JSON, only what's needed, and a cap on its length so
        // a model that runs on can't keep a slow PC busy (an online AI is asked as before)
        system.push_str("\nKeep the answer short: compact JSON on one line, no spaces between its parts, only what's needed, at most three short \"changes\".");
        body["max_tokens"] = json!(3000);
    }
    body["messages"][0]["content"] = json!(system);
    // an online AI's key goes in a header file only this user can read, gone right after (never in the command line);
    // its name has this process's number, so Cancel (which ends this process) takes it away too
    let hdr = (!e.key.is_empty()).then(|| {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let f = dir.join(format!("hearth-describe-{}.h", std::process::id()));
        let _ = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&f).and_then(|mut h| writeln!(h, "Authorization: Bearer {}", e.key));
        f
    });
    let lim = limit.to_string();
    let mut args: Vec<String> = ["-sS", "--max-time", lim.as_str(), "-H", "Content-Type: application/json"].iter().map(|s| s.to_string()).collect();
    if let Some(f) = &hdr {
        args.push("-H".into());
        args.push(format!("@{}", f.display()));
    }
    args.extend(["--data-binary".to_string(), "@-".to_string(), format!("{}/chat/completions", e.base)]);
    let t0 = std::time::Instant::now();
    let child = Command::new("curl").args(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(x) => {
            if let Some(f) = &hdr {
                let _ = std::fs::remove_file(f);
            }
            return Err(format!("Couldn't reach your AI ({x})."));
        }
    };
    child.stdin.take().map(|mut i| i.write_all(body.to_string().as_bytes()));
    let out = child.wait_with_output().map_err(|e| e.to_string());
    if let Some(f) = &hdr {
        let _ = std::fs::remove_file(f);
    }
    let out = out?;
    if !out.status.success() {
        let why = hide(&String::from_utf8_lossy(&out.stderr), &e.key).trim().chars().take(160).collect::<String>();
        return Err(match out.status.code() {
            Some(28) => too_slow(e, t0.elapsed().as_secs()),
            Some(6) | Some(7) => format!("Couldn't reach {}. Is it on?", e.label),
            _ => format!("{} didn't answer ({why}).", cap(&e.label)),
        });
    }
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|_| format!("{}'s answer couldn't be read.", cap(&e.label)))?;
    if let Some(m) = v.pointer("/error/message").and_then(Value::as_str) {
        return Err(format!("{} said: {}", cap(&e.label), hide(m, &e.key).chars().take(200).collect::<String>()));
    }
    let said = v.pointer("/choices/0/message/content").and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("{} answered with nothing.", cap(&e.label)))?;
    // how fast it was here, for the next time (Settings › AI and the waiting screen say it)
    if e.from == "here" {
        let secs = t0.elapsed().as_secs_f64();
        let tokens = v.pointer("/usage/completion_tokens").and_then(Value::as_f64).unwrap_or(said.len() as f64 / 4.0);
        if secs > 1.0 {
            note_speed(&e.model, tokens * 0.75 / secs);
        }
    }
    Ok(said)
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

/// The first JSON object in the AI's words (it may wrap it in ``` or a sentence, or think out loud first: Qwen3's and
/// DeepSeek's <think>…</think> is skipped, and a `{` in its words that isn't a whole object is passed over).
pub fn first_object(t: &str) -> Option<Value> {
    let t = match t.rfind("</think>") { Some(i) => &t[i + 8..], None => t };
    let mut from = 0;
    let mut first = None;
    while let Some(i) = t[from..].find('{') {
        if let Some(v) = object_at(&t[from + i..]) {
            // an example or a piece of the screen it quotes can come first: the answer is the one with a kind
            if ["kind", "type", "layout", "feature", "needs", "screen"].iter().any(|k| v.get(k).is_some()) {
                return Some(v);
            }
            first.get_or_insert(v);
        }
        from += i + 1;
    }
    first
}

fn object_at(t: &str) -> Option<Value> {
    let start = 0;
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
    let v = &normalize(v);
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
        _ => Err(vec![format!("The answer wasn't a layout, a feature or what's needed (it said: {}).", glimpse(v))]),
    }
}

/// Smaller AIs answer the right thing in a slightly different shape: "type" for "kind", "Layout", the layout's fields
/// at the top instead of under "layout", or no kind at all. Those are put in the shape asked for; nothing is guessed
/// beyond which of the three it is.
pub fn normalize(v: &Value) -> Value {
    let Some(o) = v.as_object() else { return v.clone() };
    let mut o = o.clone();
    let named = o.get("kind").or_else(|| o.get("type")).and_then(Value::as_str).map(|k| k.trim().to_lowercase());
    let kind = match named.as_deref() {
        Some(k @ ("layout" | "feature" | "needs")) => Some(k.to_string()),
        Some("need" | "missing" | "idea") => Some("needs".into()),
        _ if o.get("layout").is_some_and(Value::is_object) => Some("layout".into()),
        _ if o.get("feature").is_some_and(Value::is_object) => Some("feature".into()),
        _ if o.get("needs").is_some_and(Value::is_array) => Some("needs".into()),
        _ => None,
    };
    let Some(kind) = kind else { return Value::Object(o) };
    // its fields at the top: put them under their name, leaving what goes beside them
    if (kind == "layout" || kind == "feature") && !o.get(&kind).is_some_and(Value::is_object) {
        let inner: serde_json::Map<String, Value> =
            o.iter().filter(|(k, _)| !matches!(k.as_str(), "kind" | "type" | "changes" | "layout" | "feature")).map(|(k, v)| (k.clone(), v.clone())).collect();
        if !inner.is_empty() {
            o.insert(kind.clone(), Value::Object(inner));
        }
    }
    o.insert("kind".into(), json!(kind));
    Value::Object(o)
}

/// A few words of what the AI sent, so a wrong answer can be told apart from a broken one.
fn glimpse(v: &Value) -> String {
    let t = v.to_string();
    let mut g: String = t.chars().take(120).collect();
    if t.chars().count() > 120 {
        g.push('…');
    }
    g
}

/// Describe it for a whole screen (build-anything boards 4-6): your words (and a sketch) and the screen as it is now, to
/// a new screen by your AI, with any blocks it had to make (made.rs), all checked. Nothing is put in place here.
pub fn describe_screen(text: &str, sketch: Option<&std::path::Path>, current: &Value) -> Value {
    let text: String = text.trim().chars().take(2000).collect();
    if text.is_empty() && sketch.is_none() {
        return json!({ "ok": false, "error": "Say what you'd like, or add a sketch." });
    }
    // Hearth's own first: the common things, offline and at once (offline.rs), only when it understood all of it
    // (Settings › AI › Test it asks the AI itself: HEARTH_DESCRIBE_ASK_AI)
    if sketch.is_none() && std::env::var_os("HEARTH_DESCRIBE_ASK_AI").is_none() {
        if let Some(v) = crate::offline::screen(&text, current) {
            if let Ok(s) = crate::screen::parse(&v["screen"]) {
                return json!({ "ok": true, "kind": "screen", "screen": s, "made": [], "asks": [], "changes": v["changes"], "engine": "Hearth (no AI needed)" });
            }
        }
    }
    let Some(e) = engine() else {
        return json!({ "ok": false, "error": format!("{NO_AI} {}", crate::offline::KNOWS) });
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

/// Where Sort my Downloads may put what its own rules don't know (Hearth Files sortdl.rs; never Hearth's own folders
/// or _to_delete: those only by the rules).
pub const SORT_PLACES: &[&str] = &["Installers", "Installers/Archives", "Media/Images", "Media/Videos", "Media/Sound", "Documents", "Documents/Scripts", "Documents/Unsorted"];

/// Sort my Downloads' AI (Settings › AI: its own pick, else Hearth's AI): for names its rules leave in
/// Documents/Unsorted, which of SORT_PLACES each goes to. Only names are sent, never what's in the files; an answer
/// outside SORT_PLACES is dropped (the rules' place stays).
pub fn sort_ask(names: &[String]) -> Value {
    let names: Vec<String> = names.iter().take(40).map(|n| n.chars().take(200).collect()).collect();
    if names.is_empty() {
        return json!({ "ok": true, "places": {} });
    }
    let Some(e) = engine_for("sortdl") else { return json!({ "ok": false, "error": NO_AI }) };
    let system = format!(
        "You sort files in someone's Downloads folder by their names only. Answer with ONE JSON object and nothing else: {{\"places\": {{\"<file name>\": \"<folder>\"}}}}. The folder is one of: {}. Use Documents/Unsorted when you can't tell.",
        SORT_PLACES.join(", ")
    );
    let user = format!("The files: {}", serde_json::to_string(&names).unwrap_or_default());
    let reply = match chat(&e, &system, &user, None) {
        Ok(r) => r,
        Err(m) => return json!({ "ok": false, "error": m, "engine": e.label }),
    };
    let v = first_object(&reply).unwrap_or(Value::Null);
    let mut places = serde_json::Map::new();
    if let Some(m) = v.get("places").and_then(Value::as_object) {
        for (k, p) in m {
            if let Some(p) = p.as_str().filter(|p| SORT_PLACES.contains(p)) {
                if names.iter().any(|n| n == k) {
                    places.insert(k.clone(), json!(p));
                }
            }
        }
    }
    json!({ "ok": true, "places": places, "engine": e.label })
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
        return json!({ "ok": false, "error": NO_AI });
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
    fn a_model_that_writes_is_picked() {
        let have: Vec<String> = ["nomic-embed-text:latest", "huihui_ai/qwen3-abliterated:8b-v2", "qwen3:30b-a3b"].iter().map(|s| s.to_string()).collect();
        assert_eq!(pick_model(&have), "huihui_ai/qwen3-abliterated:8b-v2");
        assert_eq!(pick_model(&["nomic-embed-text:latest".to_string()]), "");
        assert_eq!(pick_model(&["phi4".to_string()]), "phi4");
    }

    #[test]
    fn answers_in_a_slightly_different_shape_are_read() {
        let n = normalize(&json!({"type": "Layout", "layout": {"a": 1}}));
        assert_eq!(n["kind"], "layout");
        let n = normalize(&json!({"layout": {"a": 1}, "changes": ["x"]}));
        assert_eq!(n["kind"], "layout");
        let n = normalize(&json!({"kind": "feature", "name": "Moon", "shows": "moon"}));
        assert_eq!(n["feature"]["name"], "Moon");
        assert!(n["feature"].get("kind").is_none());
        let n = normalize(&json!({"needs": ["a moon"], "why": "no moon"}));
        assert_eq!(n["kind"], "needs");
        let n = normalize(&json!({"hello": 1}));
        assert!(n.get("kind").is_none());
        let e = check_answer(&json!({"hello": 1}), "bar", "x", false).unwrap_err();
        assert!(e[0].contains("hello"), "{e:?}");
        let v = first_object("Like {\"a\": 1}, so: {\"kind\": \"needs\", \"needs\": [\"x\"]}").unwrap();
        assert_eq!(v["kind"], "needs");
    }

    #[test]
    fn the_json_is_found_in_the_ais_words() {
        let v = first_object("Sure! ```json\n{\"kind\": \"needs\", \"needs\": [\"moon {phase}\"]}\n``` hope that helps").unwrap();
        assert_eq!(v["needs"][0], "moon {phase}");
        assert!(first_object("no json here").is_none());
        // a thinking model's words first, with a stray brace in them
        let v = first_object("<think>The user wants {a cabin}. I'll make {\"kind\"...</think>\n{\"kind\": \"screen\"}").unwrap();
        assert_eq!(v["kind"], "screen");
        let v = first_object("I'd use {this} or {\"kind\": \"layout\"}").unwrap();
        assert_eq!(v["kind"], "layout");
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
    fn a_slow_answer_is_said_plainly_and_a_key_never_shows() {
        let e = Engine { label: "Ollama".into(), model: "dolphin3:8b".into(), from: "online".into(), ..Default::default() };
        let m = too_slow(&e, 150);
        assert!(m.starts_with("Ollama took too long") && m.contains("150 s") && m.contains("Settings › AI › Describe it uses"));
        assert_eq!(hide("bad key sk-123 here", "sk-123"), "bad key ••• here");
        assert_eq!(hide("nothing to hide", ""), "nothing to hide");
        assert_eq!(cap("your other device's AI"), "Your other device's AI");
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
