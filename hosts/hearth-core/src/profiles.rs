//! Profiles: several people on one Hearth, each with their own sign-ins, favorites, play time; installed games
//! shared. A person's own files live in Hearth's data folder while their profile is in use, and are put away in
//! profiles/<id>/ otherwise. Parental controls: a household PIN (salted scrypt, the same as the Electron host's,
//! so either host reads it), minutes of play a day, allowed games. Port of hosts/linux/src/profiles.js.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const OWN: [&str; 5] = ["library.json", "profile", "accounts", "legendary/user.json", "gog/auth.json"];
const CACHES: [&str; 4] = ["cache/epic-owned.json", "cache/gog-owned.json", "cache/xbox-owned.json", "cache/xbox-titles.json"];
const COLORS: [&str; 7] = ["#ff8a3d", "#3fd0b8", "#5aa9ff", "#b57bff", "#ffd166", "#e5483b", "#7ad15a"];

pub struct Profiles { pub dir: PathBuf, pub data: Value }

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
fn unhex(s: &str) -> Vec<u8> { (0..s.len() / 2).filter_map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()).collect() }
fn scrypt32(pin: &str, salt: &[u8]) -> Vec<u8> {
    // Node's crypto.scryptSync defaults: N=16384, r=8, p=1.
    let params = scrypt::Params::new(14, 8, 1, 32).expect("scrypt params");
    let mut out = vec![0u8; 32];
    scrypt::scrypt(pin.as_bytes(), salt, &params, &mut out).expect("scrypt");
    out
}
pub fn day_key(t: chrono::DateTime<chrono::Local>) -> String { t.format("%Y-%m-%d").to_string() }
fn is_color(c: &str) -> bool { c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()) }

fn move_path(src: &Path, dst: &Path) {
    if !src.exists() { return; }
    if let Some(p) = dst.parent() { let _ = std::fs::create_dir_all(p); }
    let _ = std::fs::remove_dir_all(dst); let _ = std::fs::remove_file(dst);
    if std::fs::rename(src, dst).is_err() {
        // Different file system: copy, then remove.
        let _ = crate::system::run("cp", &["-a", &src.to_string_lossy(), &dst.to_string_lossy()]);
        let _ = std::fs::remove_dir_all(src); let _ = std::fs::remove_file(src);
    }
}

impl Profiles {
    pub fn open(dir: &Path) -> Profiles {
        let file = dir.join("profiles.json");
        let mut data = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(|d| d["list"].as_array().map(|l| !l.is_empty()).unwrap_or(false))
            .unwrap_or_else(|| json!({"active": "main", "askAtStart": false, "list": [{"id": "main", "name": null, "color": COLORS[0], "steam": null}]}));
        let active = data["active"].clone();
        if !data["list"].as_array().unwrap().iter().any(|p| p["id"] == active) { data["active"] = data["list"][0]["id"].clone(); }
        Profiles { dir: dir.to_path_buf(), data }
    }
    pub fn save(&self) -> Result<(), String> {
        crate::paths::write_atomic(&self.dir.join("profiles.json"), serde_json::to_string_pretty(&self.data).unwrap().as_bytes()).map_err(|e| e.to_string())
    }
    pub fn active(&self) -> String { self.data["active"].as_str().unwrap_or("main").to_string() }
    pub fn list(&self) -> Vec<Value> { self.data["list"].as_array().cloned().unwrap_or_default() }
    fn find_mut(&mut self, id: &str) -> Option<&mut Value> { self.data["list"].as_array_mut()?.iter_mut().find(|p| p["id"] == id) }
    pub fn find(&self, id: &str) -> Option<Value> { self.list().into_iter().find(|p| p["id"] == id) }
    fn stash(&self, id: &str) -> PathBuf { self.dir.join("profiles").join(id) }

    pub fn create(&mut self, name: &str, color: Option<&str>) -> Result<Value, String> {
        let clean: String = name.trim().chars().take(32).collect();
        if clean.is_empty() { return Err("Give the profile a name".into()); }
        let n = self.list().len();
        if n >= 8 { return Err("Hearth has room for 8 profiles".into()); }
        let id = format!("p{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
        let color = color.filter(|c| is_color(c)).unwrap_or(COLORS[n % COLORS.len()]).to_string();
        let p = json!({"id": id, "name": clean, "color": color, "steam": null});
        self.data["list"].as_array_mut().unwrap().push(p.clone());
        // Its own library.json from the start, with the non-library games everyone shares.
        let dst = self.stash(&id);
        std::fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
        let shared = std::fs::read(self.dir.join("library.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).map(|d| d["manual"].clone()).filter(|m| m.is_array()).unwrap_or(json!([]));
        std::fs::write(dst.join("library.json"), json!({"manual": shared, "profile": {"name": clean, "accent": color}}).to_string()).map_err(|e| e.to_string())?;
        self.save()?;
        Ok(p)
    }
    pub fn update(&mut self, id: &str, patch: &Value) -> Result<Value, String> {
        let p = self.find_mut(id).ok_or("No such profile")?;
        if let Some(n) = patch["name"].as_str() { let t: String = n.trim().chars().take(32).collect(); if !t.is_empty() { p["name"] = json!(t); } }
        if let Some(c) = patch["color"].as_str().filter(|c| is_color(c)) { p["color"] = json!(c); }
        if let Some(s) = patch.get("steam") { p["steam"] = match s.as_str() { Some(v) if !v.is_empty() => json!(v.chars().take(64).collect::<String>()), _ => Value::Null }; }
        let out = p.clone();
        self.save()?;
        Ok(out)
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if id == self.active() { return Err("Switch to another profile before removing this one".into()); }
        if self.list().len() <= 1 { return Err("There has to be one profile".into()); }
        self.data["list"].as_array_mut().unwrap().retain(|p| p["id"] != id);
        let _ = std::fs::remove_dir_all(self.stash(id));
        self.save()
    }
    pub fn set_ask_at_start(&mut self, on: bool) -> Result<(), String> { self.data["askAtStart"] = json!(on); self.save() }

    /// Puts the current person's files away and brings out the other's. Hearth must restart afterwards.
    pub fn switch_to(&mut self, id: &str) -> Result<Value, String> {
        let to = self.find(id).ok_or("No such profile")?;
        let from = self.active();
        if id == from { return Ok(to); }
        let lib = self.dir.join("library.json");
        let read_lib = |p: &Path| std::fs::read(p).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).unwrap_or(json!({}));
        let manual = read_lib(&lib)["manual"].as_array().cloned().unwrap_or_default();
        let away = self.stash(&from);
        let _ = std::fs::remove_dir_all(&away);
        std::fs::create_dir_all(&away).map_err(|e| e.to_string())?;
        for rel in OWN { move_path(&self.dir.join(rel), &away.join(rel)); }
        let back = self.stash(id);
        for rel in OWN { move_path(&back.join(rel), &self.dir.join(rel)); }
        let _ = std::fs::remove_dir_all(&back);
        for rel in CACHES { let _ = std::fs::remove_file(self.dir.join(rel)); }
        let mut d = read_lib(&lib);
        let mut mine = d["manual"].as_array().cloned().unwrap_or_default();
        for g in manual { if !mine.iter().any(|m| m["id"] == g["id"]) { mine.push(g); } }
        d["manual"] = Value::Array(mine);
        if d["profile"].is_null() { d["profile"] = json!({"name": to["name"], "accent": to["color"]}); }
        std::fs::write(&lib, serde_json::to_string_pretty(&d).unwrap()).map_err(|e| e.to_string())?;
        self.data["active"] = json!(id);
        self.save()?;
        Ok(to)
    }

    // ---------------------------------------------------------------- parental controls
    pub fn has_pin(&self) -> bool { self.data["pin"]["hash"].is_string() }
    pub fn check_pin(&self, pin: &str) -> bool {
        if !self.has_pin() { return true; }
        let salt = unhex(self.data["pin"]["salt"].as_str().unwrap_or(""));
        hex(&scrypt32(pin, &salt)) == self.data["pin"]["hash"].as_str().unwrap_or("")
    }
    /// Sets or changes the PIN ("" removes it, and every limit with it).
    pub fn set_pin(&mut self, pin: &str) -> Result<(), String> {
        if pin.is_empty() {
            self.data.as_object_mut().unwrap().remove("pin");
            for p in self.data["list"].as_array_mut().unwrap() { p.as_object_mut().unwrap().remove("limits"); }
            return self.save();
        }
        if !(4..=8).contains(&pin.len()) || !pin.chars().all(|c| c.is_ascii_digit()) { return Err("The PIN is 4 to 8 numbers".into()); }
        let mut salt = [0u8; 16];
        getrandom::getrandom(&mut salt).map_err(|e| e.to_string())?;
        self.data["pin"] = json!({"salt": hex(&salt), "hash": hex(&scrypt32(pin, &salt))});
        self.save()
    }
    pub fn limits(&self, id: &str) -> Value {
        let l = self.find(id).map(|p| p["limits"].clone()).unwrap_or(Value::Null);
        json!({"minutesPerDay": l["minutesPerDay"].as_u64().unwrap_or(0), "allowed": if l["allowed"].is_array() { l["allowed"].clone() } else { Value::Null }})
    }
    pub fn restricted(&self, id: &str) -> bool { let l = self.limits(id); self.has_pin() && (l["minutesPerDay"].as_u64().unwrap_or(0) > 0 || l["allowed"].is_array()) }
    pub fn set_limits(&mut self, id: &str, patch: &Value) -> Result<Value, String> {
        if self.find(id).is_none() { return Err("No such profile".into()); }
        if !self.has_pin() { return Err("Set a PIN first, so limits can only be changed by you".into()); }
        let mut l = self.limits(id);
        if let Some(m) = patch.get("minutesPerDay") { l["minutesPerDay"] = json!(m.as_f64().unwrap_or(0.0).round().clamp(0.0, 1440.0) as u64); }
        if let Some(a) = patch.get("allowed") {
            l["allowed"] = match a.as_array() {
                Some(list) => { let mut seen: Vec<String> = Vec::new(); for v in list { let s = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()); if !seen.contains(&s) { seen.push(s); } } json!(seen.into_iter().take(2000).collect::<Vec<_>>()) }
                None => Value::Null,
            };
        }
        self.find_mut(id).unwrap()["limits"] = l.clone();
        self.save()?;
        Ok(l)
    }
    pub fn played_today(&self, id: &str, now: chrono::DateTime<chrono::Local>) -> f64 {
        let t = &self.data["today"][id];
        if t["date"] == day_key(now).as_str() { t["minutes"].as_f64().unwrap_or(0.0) } else { 0.0 }
    }
    pub fn add_played(&mut self, id: &str, minutes: f64, now: chrono::DateTime<chrono::Local>) -> Result<(), String> {
        if minutes <= 0.0 { return Ok(()); }
        let k = day_key(now);
        let prev = self.played_today(id, now);
        if !self.data["today"].is_object() { self.data["today"] = json!({}); }
        self.data["today"][id] = json!({"date": k, "minutes": ((prev + minutes) * 10.0).round() / 10.0});
        self.save()
    }
    /// Can the active profile start this game now? {ok, reason, minutesLeft}
    pub fn can_play(&self, game: Option<&str>, now: chrono::DateTime<chrono::Local>, session_minutes: f64) -> Value {
        let id = self.active();
        if !self.restricted(&id) { return json!({"ok": true, "minutesLeft": null}); }
        let l = self.limits(&id);
        if let (Some(list), Some(g)) = (l["allowed"].as_array(), game) {
            if !list.iter().any(|x| x == g) { return json!({"ok": false, "reason": "This game needs a grown-up to allow it (Settings › Profiles › Limits).", "minutesLeft": null}); }
        }
        let per_day = l["minutesPerDay"].as_f64().unwrap_or(0.0);
        if per_day == 0.0 { return json!({"ok": true, "minutesLeft": null}); }
        let left = per_day - self.played_today(&id, now) - session_minutes;
        if left > 0.0 { json!({"ok": true, "minutesLeft": left.floor() as u64}) } else { json!({"ok": false, "reason": "That's all the play time for today. More tomorrow!", "minutesLeft": 0}) }
    }
}

// ------------------------------------------------------------------ Steam's account

/// Makes Steam sign in as this account next time (it must have signed in here once, with "Remember me").
pub fn steam_registry_text(text: &str, account: &str) -> String {
    let v = account.replace('"', "");
    let auto = regex::Regex::new(r#"("AutoLoginUser"\s+)"[^"]*""#).unwrap();
    let mut t = if auto.is_match(text) { auto.replace(text, format!("${{1}}\"{v}\"")).into_owned() }
        else { regex::Regex::new(r#"("Steam"\s*\{)"#).unwrap().replace(text, format!("${{1}}\n\t\t\t\t\t\"AutoLoginUser\"\t\t\"{v}\"")).into_owned() };
    let rem = regex::Regex::new(r#"("RememberPassword"\s+)"[^"]*""#).unwrap();
    if rem.is_match(&t) { t = rem.replace(&t, "${1}\"1\"").into_owned(); }
    t
}
/// loginusers.vdf with `account` marked as the one to sign in next.
pub fn steam_login_users_text(text: &str, account: &str) -> String {
    let want = account.to_lowercase();
    let block = regex::Regex::new(r#"("\d{17}"\s*\{)([^{}]*)(\})"#).unwrap();
    block.replace_all(text, |c: &regex::Captures| {
        let body = &c[2];
        let name = regex::Regex::new(r#""AccountName"\s+"([^"]*)""#).unwrap().captures(body).map(|m| m[1].to_lowercase()).unwrap_or_default();
        let mine = !want.is_empty() && name == want;
        let set = |b: &str, key: &str, val: &str| {
            let re = regex::Regex::new(&format!(r#"("{key}"\s+)"[^"]*""#)).unwrap();
            if re.is_match(b) { re.replace(b, format!("${{1}}\"{val}\"")).into_owned() } else { format!("{}\n\t\t\"{key}\"\t\t\"{val}\"\n\t", b.trim_end()) }
        };
        let mut b = set(body, "MostRecent", if mine { "1" } else { "0" });
        if mine { b = set(&b, "AllowAutoLogin", "1"); }
        format!("{}{}{}", &c[1], b, &c[3])
    }).into_owned()
}

/// Steam's two files that say who signs in: (registry.vdf, loginusers.vdf)
pub fn steam_files() -> Option<(Option<std::path::PathBuf>, std::path::PathBuf)> {
    let dir = crate::steam::steam_path(&crate::paths::home())?;
    let reg = [dir.parent().map(|p| p.join("registry.vdf")), Some(crate::paths::home().join(".steam/registry.vdf"))].into_iter().flatten().find(|f| f.exists());
    Some((reg, dir.join("config/loginusers.vdf")))
}
pub fn known_steam_accounts() -> Vec<Value> { steam_files().and_then(|(_, u)| std::fs::read_to_string(u).ok()).map(|t| steam_accounts(&t)).unwrap_or_default() }

/// Closes Steam (it writes its files as it exits), waiting up to about 10 seconds.
pub fn stop_steam() {
    let up = || std::process::Command::new("pgrep").args(["-x", "steam"]).output().map(|o| o.status.success()).unwrap_or(false);
    if !up() { return; }
    let _ = crate::steam::spawn(&crate::paths::home(), "-shutdown");
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 8 && up() { std::thread::sleep(std::time::Duration::from_millis(300)); }
    if up() { let _ = std::process::Command::new("pkill").args(["-x", "steam"]).status(); std::thread::sleep(std::time::Duration::from_millis(1500)); }
}
/// Steam signs in as `account` the next time it starts ('' shows Steam's own sign-in). Steam must be closed.
pub fn set_steam_account(account: &str) {
    let Some((reg, users)) = steam_files() else { return };
    if let Some(reg) = reg { if let Ok(t) = std::fs::read_to_string(&reg) { let _ = std::fs::write(&reg, steam_registry_text(&t, account)); } }
    // Empty: no account marked either, so Hearth doesn't show the last person's Steam games (steam::signed_in_id64).
    if let Ok(t) = std::fs::read_to_string(&users) { let _ = std::fs::write(&users, steam_login_users_text(&t, account)); }
}

/// The Steam account signed in now (the one Steam signs in as by itself): registry.vdf's AutoLoginUser, else the
/// most recent in loginusers.vdf.
pub fn signed_in_account(registry: &str, login_users: &str) -> Option<String> {
    let auto = regex::Regex::new(r#""AutoLoginUser"\s+"([^"]*)""#).unwrap().captures(registry).map(|c| c[1].trim().to_string()).filter(|a| !a.is_empty());
    auto.or_else(|| {
        let block = regex::Regex::new(r#""\d{17}"\s*\{([^{}]*)\}"#).unwrap();
        block.captures_iter(login_users).find(|c| regex::Regex::new(r#""MostRecent"\s+"1""#).unwrap().is_match(&c[1]))
            .and_then(|c| regex::Regex::new(r#""AccountName"\s+"([^"]*)""#).unwrap().captures(&c[1]).map(|m| m[1].to_string()))
    })
}
pub fn current_steam_account() -> Option<String> {
    let (reg, users) = steam_files()?;
    let r = reg.and_then(|f| std::fs::read_to_string(f).ok()).unwrap_or_default();
    signed_in_account(&r, &std::fs::read_to_string(users).unwrap_or_default())
}

/// Leaving a profile: the Steam account it used stays its own. A profile without a linked account keeps the one
/// signed in now, unless another profile already has it (then that one's sign-in is left alone). Returns the account
/// linked, if it changed.
pub fn keep_steam_account(list: &[Value], leaving: &str, signed_in: Option<&str>) -> Option<String> {
    let acc = signed_in?.trim();
    if acc.is_empty() { return None; }
    let me = list.iter().find(|p| p["id"] == leaving)?;
    if me["steam"].as_str().map(|s| !s.is_empty()).unwrap_or(false) { return None; }
    let taken = list.iter().any(|p| p["id"] != leaving && p["steam"].as_str().map(|s| s.eq_ignore_ascii_case(acc)).unwrap_or(false));
    if taken { None } else { Some(acc.to_string()) }
}

pub fn steam_accounts(login_users: &str) -> Vec<Value> {
    let block = regex::Regex::new(r#""\d{17}"\s*\{([^{}]*)\}"#).unwrap();
    let field = |b: &str, k: &str| regex::Regex::new(&format!(r#""{k}"\s+"([^"]*)""#)).unwrap().captures(b).map(|c| c[1].to_string());
    block.captures_iter(login_users).filter_map(|c| { let b = &c[1]; let acc = field(b, "AccountName")?; Some(json!({"account": acc, "name": field(b, "PersonaName").unwrap_or_else(|| acc.clone())})) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn marks_the_steam_account_to_sign_in() {
        let t = "\"users\"\n{\n\t\"76561190000000001\"\n\t{\n\t\t\"AccountName\"\t\t\"sam\"\n\t\t\"MostRecent\"\t\t\"1\"\n\t}\n\t\"76561190000000002\"\n\t{\n\t\t\"AccountName\"\t\t\"Kim\"\n\t}\n}\n";
        let out = steam_login_users_text(t, "kim");
        let a = steam_accounts(&out);
        assert_eq!(a.len(), 2);
        assert!(out.contains("\"AccountName\"\t\t\"sam\"\n\t\t\"MostRecent\"\t\t\"0\""));
        assert!(out.contains("\"MostRecent\"\t\t\"1\"\n\t\t\"AllowAutoLogin\"\t\t\"1\""));
    }
    #[test]
    fn each_profile_keeps_its_steam_account() {
        let users = "\"users\"\n{\n\t\"76561190000000001\"\n\t{\n\t\t\"AccountName\"\t\t\"sam\"\n\t\t\"MostRecent\"\t\t\"1\"\n\t}\n}\n";
        assert_eq!(signed_in_account("", users).as_deref(), Some("sam"));
        assert_eq!(signed_in_account("\"AutoLoginUser\"\t\t\"kim\"", users).as_deref(), Some("kim"));
        assert_eq!(signed_in_account("\"AutoLoginUser\"\t\t\"\"", "").as_deref(), None);
        let list = vec![json!({"id": "main", "steam": null}), json!({"id": "kid", "steam": "kim"}), json!({"id": "new", "steam": null})];
        assert_eq!(keep_steam_account(&list, "main", Some("sam")).as_deref(), Some("sam"), "main keeps what it signed in with");
        assert_eq!(keep_steam_account(&list, "main", Some("Kim")), None, "kim is the kid's");
        assert_eq!(keep_steam_account(&list, "kid", Some("sam")), None, "the kid already has one");
        assert_eq!(keep_steam_account(&list, "main", None), None);
    }
    #[test]
    fn parental_controls() {
        let dir = tempfile::tempdir().unwrap();
        let mut ps = Profiles::open(dir.path());
        let kid = ps.create("Sam", None).unwrap();
        let kid_id = kid["id"].as_str().unwrap().to_string();
        assert!(ps.set_limits(&kid_id, &json!({"minutesPerDay": 60})).unwrap_err().contains("PIN first"));
        assert!(ps.set_pin("12").unwrap_err().contains("4 to 8"));
        ps.set_pin("2468").unwrap();
        assert!(ps.check_pin("2468") && !ps.check_pin("1111"));
        ps.set_limits(&kid_id, &json!({"minutesPerDay": 60, "allowed": ["steam:1", "steam:1", "gog:2"]})).unwrap();
        assert_eq!(ps.limits(&kid_id), json!({"minutesPerDay": 60, "allowed": ["steam:1", "gog:2"]}));
        ps.switch_to(&kid_id).unwrap();
        let day = chrono::Local.with_ymd_and_hms(2026, 10, 18, 15, 0, 0).unwrap();
        assert_eq!(ps.can_play(Some("xbox:9"), day, 0.0)["ok"], false);
        assert_eq!(ps.can_play(Some("steam:1"), day, 0.0), json!({"ok": true, "minutesLeft": 60}));
        ps.add_played(&kid_id, 45.0, day).unwrap();
        assert_eq!(ps.can_play(Some("steam:1"), day, 10.0)["minutesLeft"], 5);
        let again = Profiles::open(dir.path());
        assert!(again.check_pin("2468"));
        assert_eq!(again.played_today(&kid_id, day), 45.0);
    }
    #[test]
    fn reads_a_pin_made_by_the_electron_host() {
        // Node: crypto.scryptSync('2468', Buffer.from('00112233445566778899aabbccddeeff','hex'), 32).toString('hex')
        let salt = unhex("00112233445566778899aabbccddeeff");
        assert_eq!(hex(&scrypt32("2468", &salt)), "f9e3f4915303b2269ab1d8c2017b630f2e7b8d557ec3264dd028464f3cdca09b");
    }
    #[test]
    fn steam_files() {
        let reg = "\"Registry\" { \"HKCU\" { \"Software\" { \"Valve\" { \"Steam\" { \"RememberPassword\" \"0\" } } } } }";
        let t = steam_registry_text(reg, "wife");
        assert!(t.contains("\"AutoLoginUser\"\t\t\"wife\"") && t.contains("\"RememberPassword\" \"1\""));
        let users = "\"users\" { \"76561197960265729\" { \"AccountName\" \"sam\" \"PersonaName\" \"Sam\" } \"76561197960265730\" { \"AccountName\" \"kid\" } }";
        assert_eq!(steam_accounts(users), vec![json!({"account": "sam", "name": "Sam"}), json!({"account": "kid", "name": "kid"})]);
    }
}
