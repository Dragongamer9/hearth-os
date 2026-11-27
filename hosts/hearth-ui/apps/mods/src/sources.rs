//! Where mods come from (board 3): one search across Nexus Mods (your own API key, as Hearth Mods always did),
//! Thunderstore (no account), mod.io (its public API, with a key you type, kept in the keyring) and CurseForge (its
//! API needs a key Hearth applies for; one typed in is kept in the keyring). Updates checked against each mod's page
//! (board 4), Describe it's Hearth AI (board 6), the home server's shared folder (board 8).

use crate::App;
use hearth_gaming::nexus;
use serde_json::{json, Value};

pub const SOURCES: &[(&str, &str)] = &[("nexus", "Nexus Mods"), ("thunderstore", "Thunderstore"), ("curseforge", "CurseForge"), ("modio", "mod.io")];

/// One result.
#[derive(Debug, Clone, Default)]
pub struct Hit {
    pub source: &'static str,
    pub game: String,
    pub domain: String,
    pub id: String,
    pub name: String,
    pub author: String,
    pub summary: String,
    pub endorsed: u64,
    pub downloads: u64,
    pub rating: f64,
    pub version: String,
    pub updated: String,
    pub picture: String,
    pub page: String,
    pub adult: bool,
    /// (file name, file id, download address)
    pub files: Vec<(String, u64, String)>,
    pub needs: Vec<String>,
    /// "works" or "! …" (board 3's line under each result)
    pub check: String,
    pub have: bool,
}

impl Hit {
    pub fn check(&mut self, mine: &[String]) {
        let n = self.name.to_lowercase();
        self.have = mine.iter().any(|m| *m == n);
        let missing: Vec<&String> = self.needs.iter().filter(|x| !mine.iter().any(|m| m.contains(&x.to_lowercase()))).collect();
        self.check = if self.have {
            "you have it".into()
        } else if missing.is_empty() {
            if self.needs.is_empty() { "works with your mods · needs nothing new".into() } else { format!("works · needs {} (you have it)", self.needs.join(", ")) }
        } else {
            format!("! needs {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
        };
    }
    pub fn label(&self) -> &'static str {
        SOURCES.iter().find(|s| s.0 == self.source).map(|s| s.1).unwrap_or("")
    }
}

fn get(url: &str, headers: &[(&str, &str)]) -> Result<Value, String> {
    let (code, _, body) = nexus::curl_get(url, headers, 25)?;
    if code == 401 || code == 403 {
        return Err("needs a key".into());
    }
    if code != 200 {
        return Err(format!("HTTP {code}"));
    }
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

fn enc(s: &str) -> String {
    nexus::urlencode(s)
}

/// Thunderstore's community for a game ("Valheim" → valheim).
fn community(title: &str) -> String {
    title.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

/// The keys typed in for mod.io and CurseForge (the keyring), and whether Nexus is signed in.
pub fn have_keys() -> impl std::future::Future<Output = Vec<(&'static str, bool)>> + Send + 'static {
    async move {
        let m = hearth_oss::secret::lookup("hearth-mods", "modio").await;
        let c = hearth_oss::secret::lookup("hearth-mods", "curseforge").await;
        vec![("nexus", nexus::account().is_some()), ("modio", !m.is_empty()), ("curseforge", !c.is_empty()), ("thunderstore", true)]
    }
}

pub fn keep_key(src: &'static str, key: String) -> impl std::future::Future<Output = Vec<(&'static str, bool)>> + Send + 'static {
    async move {
        let label = format!("Mod Manager · {src} API key");
        let _ = hearth_oss::secret::store("hearth-mods", src, &label, key).await;
        have_keys().await
    }
}

/// One source's results for every game of yours (`games`: (title, Steam app)).
pub fn search(src: &'static str, q: String, games: Vec<(String, String)>) -> impl std::future::Future<Output = Result<Vec<Hit>, String>> + Send + 'static {
    async move {
        let key = match src {
            "modio" | "curseforge" => hearth_oss::secret::lookup("hearth-mods", src).await,
            _ => String::new(),
        };
        crate::back::work(move || search_now(src, &q, &games, &key)).await
    }
}

fn search_now(src: &'static str, q: &str, games: &[(String, String)], key: &str) -> Result<Vec<Hit>, String> {
    let mut out = Vec::new();
    let mut errs: Vec<String> = Vec::new();
    match src {
        "nexus" => {
            let acct = nexus::account().ok_or("Sign in to Nexus Mods (Accounts) with your own API key.")?;
            for (title, app) in games {
                let Some(domain) = nexus::STEAM_DOMAINS.iter().find(|d| d.0.to_string() == *app).map(|d| d.1) else { continue };
                let v = nexus::search(&acct.key, domain, q).unwrap_or(Value::Null);
                for m in v["mods"].as_array().or(v.as_array()).cloned().unwrap_or_default().into_iter().take(10) {
                    let id = m["modId"].as_u64().or(m["mod_id"].as_u64()).unwrap_or(0);
                    out.push(Hit {
                        source: "nexus",
                        game: title.clone(),
                        domain: domain.to_string(),
                        id: id.to_string(),
                        name: m["name"].as_str().unwrap_or("").into(),
                        author: m["author"].as_str().unwrap_or("").into(),
                        summary: m["summary"].as_str().unwrap_or("").into(),
                        endorsed: m["endorsements"].as_u64().or(m["endorsement_count"].as_u64()).unwrap_or(0),
                        downloads: m["downloads"].as_u64().unwrap_or(0),
                        version: m["version"].as_str().unwrap_or("").into(),
                        updated: m["updatedAt"].as_str().or(m["updated_time"].as_str()).unwrap_or("").chars().take(10).collect(),
                        picture: m["pictureUrl"].as_str().or(m["picture_url"].as_str()).unwrap_or("").into(),
                        page: format!("{}/{domain}/mods/{id}", nexus::SITE),
                        adult: m["adultContent"].as_bool().or(m["contains_adult_content"].as_bool()).unwrap_or(false),
                        ..Default::default()
                    });
                }
            }
        }
        "thunderstore" => {
            for (title, _) in games {
                let c = community(title);
                let v = match get(&format!("https://thunderstore.io/api/cyberstorm/listing/{c}/?q={}&ordering=most-downloaded", enc(q)), &[("Accept", "application/json")]) {
                    Ok(v) => v,
                    Err(e) => {
                        errs.push(format!("{title}: {e}"));
                        continue;
                    }
                };
                for m in v["results"].as_array().cloned().unwrap_or_default().into_iter().take(10) {
                    let (ns, name) = (m["namespace"].as_str().unwrap_or(""), m["name"].as_str().unwrap_or(""));
                    out.push(Hit {
                        source: "thunderstore",
                        game: title.clone(),
                        domain: c.clone(),
                        id: format!("{ns}-{name}"),
                        name: name.replace('_', " "),
                        author: ns.into(),
                        summary: m["description"].as_str().unwrap_or("").into(),
                        downloads: m["download_count"].as_u64().unwrap_or(0),
                        endorsed: m["rating_count"].as_u64().unwrap_or(0),
                        updated: m["last_updated"].as_str().unwrap_or("").chars().take(10).collect(),
                        picture: m["icon_url"].as_str().unwrap_or("").into(),
                        page: format!("https://thunderstore.io/c/{c}/p/{ns}/{name}/"),
                        files: vec![("Latest".into(), 0, format!("https://thunderstore.io/package/download/{ns}/{name}/{}/", m["latest_version"].as_str().unwrap_or("")))],
                        ..Default::default()
                    });
                }
            }
        }
        "modio" => {
            if key.is_empty() {
                return Err("Type your mod.io API key in Accounts (mod.io › API access); it's kept in the keyring.".into());
            }
            let gs = get(&format!("https://api.mod.io/v1/games?api_key={}&_limit=100", enc(key)), &[])?;
            let all = gs["data"].as_array().cloned().unwrap_or_default();
            for (title, _) in games {
                let Some(g) = all.iter().find(|g| g["name"].as_str().map(|n| n.eq_ignore_ascii_case(title)).unwrap_or(false)) else { continue };
                let gid = g["id"].as_u64().unwrap_or(0);
                let v = get(&format!("https://api.mod.io/v1/games/{gid}/mods?api_key={}&_q={}&_limit=10", enc(key), enc(q)), &[])?;
                for m in v["data"].as_array().cloned().unwrap_or_default() {
                    out.push(Hit {
                        source: "modio",
                        game: title.clone(),
                        domain: gid.to_string(),
                        id: m["id"].as_u64().unwrap_or(0).to_string(),
                        name: m["name"].as_str().unwrap_or("").into(),
                        author: m["submitted_by"]["username"].as_str().unwrap_or("").into(),
                        summary: m["summary"].as_str().unwrap_or("").into(),
                        downloads: m["stats"]["downloads_total"].as_u64().unwrap_or(0),
                        picture: m["logo"]["thumb_320x180"].as_str().unwrap_or("").into(),
                        page: m["profile_url"].as_str().unwrap_or("").into(),
                        files: vec![(m["modfile"]["filename"].as_str().unwrap_or("").into(), m["modfile"]["id"].as_u64().unwrap_or(0), m["modfile"]["download"]["binary_url"].as_str().unwrap_or("").into())],
                        version: m["modfile"]["version"].as_str().unwrap_or("").into(),
                        ..Default::default()
                    });
                }
            }
        }
        "curseforge" => {
            if key.is_empty() {
                return Err("CurseForge needs Hearth's own API key (applied for at console.curseforge.com); until then one you type in Accounts is used.".into());
            }
            let h = [("x-api-key", key), ("Accept", "application/json")];
            let gs = get("https://api.curseforge.com/v1/games?pageSize=50", &h)?;
            let all = gs["data"].as_array().cloned().unwrap_or_default();
            for (title, _) in games {
                let Some(g) = all.iter().find(|g| g["name"].as_str().map(|n| n.eq_ignore_ascii_case(title)).unwrap_or(false)) else { continue };
                let gid = g["id"].as_u64().unwrap_or(0);
                let v = get(&format!("https://api.curseforge.com/v1/mods/search?gameId={gid}&searchFilter={}&pageSize=10&sortField=6&sortOrder=desc", enc(q)), &h)?;
                for m in v["data"].as_array().cloned().unwrap_or_default() {
                    out.push(Hit {
                        source: "curseforge",
                        game: title.clone(),
                        domain: gid.to_string(),
                        id: m["id"].as_u64().unwrap_or(0).to_string(),
                        name: m["name"].as_str().unwrap_or("").into(),
                        summary: m["summary"].as_str().unwrap_or("").into(),
                        downloads: m["downloadCount"].as_u64().unwrap_or(0),
                        picture: m["logo"]["thumbnailUrl"].as_str().unwrap_or("").into(),
                        page: m["links"]["websiteUrl"].as_str().unwrap_or("").into(),
                        files: m["latestFiles"].as_array().cloned().unwrap_or_default().iter().take(4).map(|f| (f["displayName"].as_str().unwrap_or("").to_string(), f["id"].as_u64().unwrap_or(0), f["downloadUrl"].as_str().unwrap_or("").to_string())).collect(),
                        ..Default::default()
                    });
                }
            }
        }
        _ => {}
    }
    // every game's search failed (none of them is on that site, or it didn't answer): say why
    if out.is_empty() && !errs.is_empty() && src == "thunderstore" {
        errs.sort_by_key(|e| e.contains("HTTP 404"));
        return Err(format!("none of your games' communities answered ({})", errs.into_iter().take(2).collect::<Vec<_>>().join(" · ")));
    }
    Ok(out)
}

/// A Thunderstore, mod.io or CurseForge file, downloaded into Hearth's downloads (on the home server's shared folder
/// when there is one, so it's downloaded once): its path, for Install.
pub fn fetch(h: Hit, file: usize) -> impl std::future::Future<Output = Option<String>> + Send + 'static {
    crate::back::work(move || {
        let (name, _, url) = h.files.get(file).cloned().or(h.files.first().cloned())?;
        if url.is_empty() {
            return None;
        }
        let dir = downloads_dir().join(h.source);
        let _ = std::fs::create_dir_all(&dir);
        let fname = if name.contains('.') { name } else { format!("{}.zip", h.id) };
        let dest = dir.join(hearth_gaming::util::safe_name(&fname));
        let ok = std::process::Command::new("curl").args(["-fsSL", "-m", "600", "-o"]).arg(&dest).arg(&url).status().map(|s| s.success()).unwrap_or(false);
        ok.then(|| dest.to_string_lossy().to_string())
    })
}

// ------------------------------------------------------------------ the home server

/// The home server's shared folder for mod files, when Hearth's Home server helper has one mounted.
pub fn home_server() -> Option<std::path::PathBuf> {
    let s = hearth_ui::settings_file::read_json(&hearth_gaming::util::config_dir().join("home-server.json"));
    for k in ["modsFolder", "sharedFolder", "share"] {
        if let Some(p) = s[k].as_str().map(std::path::PathBuf::from).filter(|p| p.is_dir()) {
            return Some(p.join("Mods"));
        }
    }
    let m = hearth_oss_kit_a::home().join("HomeServer");
    m.is_dir().then(|| m.join("Mods"))
}

/// Where downloads go: the home server's Mods folder when there is one (every device then takes the same file),
/// else Hearth Mods' own downloads folder.
pub fn downloads_dir() -> std::path::PathBuf {
    home_server().unwrap_or_else(|| hearth_gaming::util::data_dir().join("mods").join("downloads"))
}

pub fn sync_words() -> String {
    let d = crate::back::read_own("devices.json");
    match d["devices"].as_array().map(|a| a.len()).unwrap_or(0) {
        0 => "This PC".into(),
        _ => format!("{} · synced with {}", d["here"].as_str().unwrap_or("This PC"), d["devices"][0]["name"].as_str().unwrap_or("your devices")),
    }
}

pub fn ticks() -> impl futures::Stream<Item = ()> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("mods-tick".into()).spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if tx.unbounded_send(()).is_err() {
            break;
        }
    });
    rx
}

// ------------------------------------------------------------------ updates

fn newer(a: &str, b: &str) -> bool {
    let n = |s: &str| s.trim_start_matches('v').split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    n(b) > n(a)
}

/// Checks every Nexus mod of this game against its page (version and changelog); nothing is installed.
pub fn check_updates(key: &str, game: &str, domain: &str, mods: &[Value]) -> Result<Value, String> {
    let mut ups = Vec::new();
    let mut said = String::new();
    match nexus::account() {
        None => said = "Sign in to Nexus Mods (Accounts) to check its mods.".into(),
        Some(acct) => {
            for m in mods.iter().filter(|m| m["nexusId"].as_u64().is_some()) {
                let id = m["nexusId"].as_u64().unwrap_or(0);
                let Ok(info) = nexus::mod_info(&acct.key, domain, id) else { continue };
                let (have, now) = (m["version"].as_str().unwrap_or(""), info["version"].as_str().unwrap_or(""));
                if info["status"].as_str().map(|s| s != "published").unwrap_or(false) {
                    ups.push(json!({"id": format!("{key}:{id}:gone"), "gone": true, "game": game, "mod": m["name"], "date": info["updated_time"]}));
                    continue;
                }
                if !now.is_empty() && newer(have, now) {
                    let log = nexus::call(&acct.key, &format!("/games/{domain}/mods/{id}/changelogs.json")).unwrap_or(Value::Null);
                    let lines: Vec<Value> = log[now].as_array().cloned().unwrap_or_default();
                    let files = nexus::files(&acct.key, domain, id).unwrap_or(Value::Null);
                    let main = files["files"].as_array().and_then(|f| f.iter().filter(|x| x["category_name"] == "MAIN").max_by_key(|x| x["uploaded_timestamp"].as_u64().unwrap_or(0)).cloned()).unwrap_or(Value::Null);
                    ups.push(json!({"id": format!("{key}:{id}"), "game": game, "mod": m["name"], "from": have, "to": now, "source": "nexus", "domain": domain, "modId": id,
                        "fileId": main["file_id"], "size": main["size_kb"].as_u64().map(|k| k * 1024), "date": info["updated_time"], "changelog": lines, "tag": "safe"}));
                }
            }
        }
    }
    let gone: Vec<Value> = ups.iter().filter(|u| u["gone"] == true).cloned().collect();
    ups.retain(|u| u["gone"] != true);
    Ok(json!({"game": game, "updates": ups, "gone": gone, "said": said}))
}

// ------------------------------------------------------------------ Describe it (Hearth AI)

/// What Hearth AI is told: the game, the profile and its mods, the device.
pub fn ai_context(a: &App) -> Value {
    let mods: Vec<String> = a.mods().iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true).filter_map(|m| m["name"].as_str().map(str::to_string)).collect();
    json!({"game": a.game().map(|g| g.title.clone()).unwrap_or_default(), "profile": a.profile(), "mods": mods, "device": a.settings["describeFor"].as_str().unwrap_or("the ROG Ally")})
}

/// Hearth AI's engines (Hearth OS's ~/.config/nimbus/ai-engines.json: Colibri on the Nvidia PC over Hearth Link,
/// Ollama the fallback), as Describe it elsewhere uses them.
pub fn engines() -> Vec<(String, String)> {
    let f = hearth_ui::settings_file::read_json(&hearth_gaming::util::config_dir().join("ai-engines.json"));
    let mut v = Vec::new();
    let colibri = f["colibri"]["url"].as_str().or(f["colibri"].as_str()).or(f["url"].as_str()).unwrap_or("http://127.0.0.1:8000/v1").trim_end_matches('/').to_string();
    v.push((colibri, f["colibri"]["model"].as_str().or(f["model"].as_str()).unwrap_or("").to_string()));
    let ollama = f["ollama"]["url"].as_str().or(f["ollamaUrl"].as_str()).unwrap_or("http://127.0.0.1:11434").trim_end_matches('/').to_string();
    v.push((format!("{ollama}/v1"), f["ollama"]["model"].as_str().unwrap_or("").to_string()));
    v
}

/// Asks Hearth AI for a set of mods (nothing is installed: the answer is shown, and installs wait for you).
pub fn ask_ai(ctx: &Value, talk: &Value) -> Result<Value, String> {
    let system = "You are Hearth AI inside Hearth OS's Mod Manager. The person describes how they want a game modded. Answer ONLY with JSON: {\"reply\": \"plain words, a few sentences, say nothing is installed until they say\", \"groups\": [{\"name\": \"Looks\", \"mods\": [{\"name\": \"real mod name\", \"source\": \"Nexus Mods\", \"fps\": -1, \"size\": \"3.8 GB\"}]}], \"why\": [\"one line per reason for the order\"], \"profile\": \"a new profile's name\"}. Use real mods from Nexus Mods, Thunderstore, mod.io or CurseForge. Keep their own mods the person asks to keep.";
    let mut msgs = vec![json!({"role": "system", "content": system}), json!({"role": "user", "content": format!("Game: {}. Profile: {}. Device: {}. Mods on now: {}.", ctx["game"], ctx["profile"], ctx["device"], ctx["mods"])})];
    for t in talk["talk"].as_array().cloned().unwrap_or_default() {
        if let Some(y) = t["you"].as_str() {
            msgs.push(json!({"role": "user", "content": y}));
        } else if let Some(r) = t["ai"].as_str() {
            msgs.push(json!({"role": "assistant", "content": r}));
        }
    }
    let mut last = String::from("Hearth AI isn't answering: is the Nvidia PC on? (Describe it waits for it.)");
    for (url, model) in engines() {
        let body = json!({"model": if model.is_empty() { "default" } else { model.as_str() }, "messages": msgs, "temperature": 0.3}).to_string();
        let out = std::process::Command::new("curl")
            .args(["-fsS", "-m", "120", "-H", "Content-Type: application/json", "-d", "@-", &format!("{url}/chat/completions")])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut c| {
                use std::io::Write;
                if let Some(mut i) = c.stdin.take() {
                    let _ = i.write_all(body.as_bytes());
                }
                c.wait_with_output()
            });
        let Ok(o) = out else { continue };
        if !o.status.success() {
            last = format!("Hearth AI didn't answer ({}).", String::from_utf8_lossy(&o.stderr).trim());
            continue;
        }
        let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
        let text = v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        let j = text.find('{').and_then(|s| text.rfind('}').map(|e| text[s..=e].to_string())).and_then(|x| serde_json::from_str::<Value>(&x).ok());
        return Ok(j.unwrap_or_else(|| json!({"reply": text})));
    }
    Err(last)
}
