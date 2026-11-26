//! Shield working fully from its Hearth Store copy, on any Hearth OS that runs AdGuard Home (the owner, Nov 26: "All
//! this has to be updated thru the store"):
//!   * the counters (each day's totals for 7 days and 30 days, each device's look-ups and blocked ones since midnight)
//!     are Shield's own: `hearth-shield-app --count`, run every minute by the user service hearth-shield-count (a
//!     timer in your own systemd folder that Shield makes and turns on only while you have blocking on in Shield;
//!     never part of starting the PC or Game Mode: the service has no [Install], and it ends at once when AdGuard isn't
//!     there), counted from AdGuard Home's own API (its statistics and query log), kept in ~/.local/state/hearth/shield;
//!   * the block lists, always block and family rules on a Hearth OS whose hearth-server-dns doesn't have them yet:
//!     set through AdGuard Home's own API, and kept here (choices.json, written safely) so each minute's count puts
//!     them back if AdGuard started again from its own file without them. Nothing changes by itself: only what you
//!     chose in Shield is put back.
//! The image's own counter (hearth-server-dns, root) is used instead wherever it's there.

use crate::data;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

pub const UNIT: &str = "hearth-shield-count";

/// Shield's block lists (the board's), as AdGuard filters (the same as hearth-server-dns's LISTS).
pub const LISTS: [(&str, &str, &str); 6] = [
    ("main", "Ads and trackers (main list)", "https://adguardteam.github.io/HostlistsRegistry/assets/filter_1.txt"),
    ("tv", "Smart TV spying", "https://raw.githubusercontent.com/Perflyst/PiHoleBlocklist/master/SmartTV-AGH.txt"),
    ("phone", "Phone app trackers", "https://raw.githubusercontent.com/Perflyst/PiHoleBlocklist/master/android-tracking.txt"),
    ("scam", "Scam and malware sites", "https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-agh.txt"),
    ("games", "Game launcher ads", "https://raw.githubusercontent.com/DandelionSprout/adfilt/master/GameConsoleAdblockList.txt"),
    ("adult", "Adult sites", "https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn-only/hosts"),
];

pub fn state() -> PathBuf {
    let h = hearth_ui::paths::home();
    std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/state")).join("hearth/shield")
}

fn write(name: &str, v: &Value) {
    let d = state();
    let _ = std::fs::create_dir_all(&d);
    let f = d.join(name);
    let tmp = f.with_extension("part");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &f);
    }
}
fn read(name: &str) -> Value {
    std::fs::read(state().join(name)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

fn today() -> (String, usize) {
    let t = hearth_ui::time::local(hearth_ui::time::now());
    (format!("{:04}-{:02}-{:02}", t.year, t.mon + 1, t.day), (t.hour.clamp(0, 23) + 1) as usize)
}

/// Since midnight, from AdGuard's last 24 hours by the hour: asked, blocked.
pub fn today_of(s: &Value, hours: usize) -> (u64, u64) {
    let sum = |k: &str| s.get(k).and_then(Value::as_array).map(|a| a.iter().rev().take(hours).filter_map(Value::as_u64).sum()).unwrap_or(0);
    (sum("dns_queries"), sum("blocked_filtering"))
}

/// One minute's count (days.json, clients-today.json), then Shield's own choices put back if AdGuard lost them.
pub fn tick() -> Result<(), String> {
    let s = data::ag_get("/control/stats")?;
    let (day, hours) = today();
    let (asked, blocked) = today_of(&s, hours);
    let mut days = read("days.json").as_object().cloned().unwrap_or_default();
    days.insert(day.clone(), json!({"asked": asked, "blocked": blocked}));
    let mut keys: Vec<String> = days.keys().cloned().collect();
    keys.sort();
    for k in keys.iter().take(keys.len().saturating_sub(31)) {
        days.remove(k);
    }
    write("days.json", &Value::Object(days));
    // each device, counted from the query log as it grows (only entries newer than the last counted)
    let mut cur = read("clients-today.json");
    if cur.get("day").and_then(Value::as_str) != Some(day.as_str()) {
        cur = json!({"day": day, "last": "", "clients": {}});
    }
    let last = cur["last"].as_str().unwrap_or("").to_string();
    let mut newest = last.clone();
    let log = data::ag_get("/control/querylog?limit=500").unwrap_or(Value::Null);
    for e in log.get("data").and_then(Value::as_array).cloned().unwrap_or_default() {
        let t = e.get("time").and_then(Value::as_str).unwrap_or("").to_string();
        if t <= last || !t.starts_with(&day) {
            break;
        }
        if t > newest {
            newest = t.clone();
        }
        let who = e.get("client").and_then(Value::as_str).unwrap_or("?").to_string();
        let r = e.get("reason").and_then(Value::as_str).unwrap_or("");
        let c = cur["clients"].as_object_mut().map(|m| m.entry(who).or_insert_with(|| json!({"asked": 0, "blocked": 0})).clone()).unwrap_or(Value::Null);
        let mut c = c;
        c["asked"] = json!(c["asked"].as_u64().unwrap_or(0) + 1);
        if ["FilteredBlackList", "FilteredBlockedService", "FilteredParental", "FilteredSafeBrowsing"].iter().any(|x| r.starts_with(x)) {
            c["blocked"] = json!(c["blocked"].as_u64().unwrap_or(0) + 1);
        }
        let who = e.get("client").and_then(Value::as_str).unwrap_or("?").to_string();
        cur["clients"][who] = c;
    }
    cur["last"] = json!(newest);
    write("clients-today.json", &cur);
    put_back()
}

fn choices() -> Map<String, Value> {
    hearth_ui::settings_file::read_json(&state().join("choices.json")).as_object().cloned().unwrap_or_default()
}
fn keep(f: impl FnOnce(&mut Map<String, Value>)) -> Result<(), String> {
    let _ = std::fs::create_dir_all(state());
    hearth_ui::settings_file::update_json(&state().join("choices.json"), |m| f(m)).map(|_| ()).map_err(|e| e.to_string())
}

fn set_list(id: &str, on: bool) -> Result<(), String> {
    let Some((_, label, url)) = LISTS.iter().find(|l| l.0 == id) else { return Err(format!("no list {id}")) };
    let st = data::ag_get("/control/filtering/status")?;
    let have = st.get("filters").and_then(Value::as_array).map(|a| a.iter().any(|f| f.get("url").and_then(Value::as_str) == Some(*url))).unwrap_or(false);
    if !have && on {
        data::ag_send("POST", "/control/filtering/add_url", &json!({"name": label, "url": url, "whitelist": false}))?;
    }
    if have || on {
        data::ag_send("POST", "/control/filtering/set_url", &json!({"url": url, "whitelist": false, "data": {"enabled": on, "name": label, "url": url}}))?;
    }
    Ok(())
}
fn rules() -> Result<Vec<String>, String> {
    let st = data::ag_get("/control/filtering/status")?;
    Ok(st.get("user_rules").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default())
}
fn set_rules(r: Vec<String>) -> Result<(), String> {
    data::ag_send("POST", "/control/filtering/set_rules", &json!({"rules": r})).map(|_| ())
}
fn set_safe(on: bool) -> Result<(), String> {
    data::ag_send("PUT", "/control/safesearch/settings", &json!({"enabled": on, "bing": true, "duckduckgo": true, "google": true, "pixabay": true, "yandex": true, "youtube": true})).map(|_| ())
}

/// A change Shield makes itself (the image's helper doesn't have it): AdGuard's API, and kept.
pub fn change(args: &[&str]) -> Result<(), String> {
    match args {
        ["list", id, on] => {
            let on = *on == "on";
            set_list(id, on)?;
            let id = id.to_string();
            keep(|m| {
                let mut l = m.get("lists").cloned().filter(Value::is_object).unwrap_or(json!({}));
                l[id] = json!(on);
                m.insert("lists".into(), l);
            })
        }
        ["block", d] | ["unblock", d] => {
            let add = args[0] == "block";
            let rule = format!("||{d}^");
            let mut r = rules()?;
            r.retain(|x| x != &rule);
            if add {
                r.push(rule);
            }
            set_rules(r)?;
            let d = d.to_string();
            keep(|m| {
                let mut b: Vec<Value> = m.get("block").and_then(Value::as_array).cloned().unwrap_or_default();
                b.retain(|x| x.as_str() != Some(d.as_str()));
                if add {
                    b.push(json!(d));
                }
                m.insert("block".into(), Value::Array(b));
            })
        }
        ["unallow", d] => {
            let rule = format!("@@||{d}^");
            let mut r = rules()?;
            r.retain(|x| x != &rule);
            set_rules(r)
        }
        ["safe-search", on] => {
            let on = *on == "on";
            set_safe(on)?;
            keep(|m| {
                m.insert("safeSearch".into(), json!(on));
            })
        }
        _ => Err("Shield can't do that on this Hearth OS".into()),
    }
}

/// What you chose in Shield, put back where AdGuard lost it (it started again from its own file).
fn put_back() -> Result<(), String> {
    let c = choices();
    if c.is_empty() {
        return Ok(());
    }
    if let Some(l) = c.get("lists").and_then(Value::as_object) {
        let st = data::ag_get("/control/filtering/status")?;
        for (id, on) in l {
            let on = on.as_bool().unwrap_or(false);
            let url = LISTS.iter().find(|x| x.0 == id).map(|x| x.2).unwrap_or("");
            let now = st.get("filters").and_then(Value::as_array).and_then(|a| a.iter().find(|f| f.get("url").and_then(Value::as_str) == Some(url))).and_then(|f| f.get("enabled")).and_then(Value::as_bool).unwrap_or(false);
            if now != on {
                set_list(id, on)?;
            }
        }
    }
    if let Some(b) = c.get("block").and_then(Value::as_array) {
        let mut r = rules()?;
        let mut changed = false;
        for d in b.iter().filter_map(Value::as_str) {
            let rule = format!("||{d}^");
            if !r.contains(&rule) {
                r.push(rule);
                changed = true;
            }
        }
        if changed {
            set_rules(r)?;
        }
    }
    if let Some(on) = c.get("safeSearch").and_then(Value::as_bool) {
        let now = data::ag_get("/control/safesearch/status").ok().and_then(|v| v.get("enabled").and_then(Value::as_bool));
        if now != Some(on) {
            set_safe(on)?;
        }
    }
    Ok(())
}

/// The counter's user service and timer (made by Shield, in your own systemd folder; on only while blocking is on).
pub fn units(me: &str) -> (String, String) {
    let start = if std::path::Path::new("/usr/libexec/hearth/hearth-part").exists() { "/usr/libexec/hearth/hearth-part run hearth-shield-app /usr/bin/hearth-shield-app --count".to_string() } else { format!("{me} --count") };
    (
        format!("# Hearth Shield's counter (made by Hearth Shield; its timer is on only while blocking is on)\n[Unit]\nDescription=Hearth Shield: count today's look-ups\nConditionPathExists=%h/.config/systemd/user/{UNIT}.timer\n\n[Service]\nType=oneshot\nExecStart={start}\nNice=15\nTimeoutStartSec=60\n"),
        format!("# Hearth Shield's counter, each minute (made by Hearth Shield)\n[Unit]\nDescription=Hearth Shield: count today's look-ups each minute\n\n[Timer]\nOnActiveSec=30\nOnUnitActiveSec=60\nAccuracySec=15\n\n[Install]\nWantedBy=timers.target\n"),
    )
}

/// The counter on (made and its timer started) or off (stopped, its files taken out). The image's own counter, where
/// it's there, makes this unneeded: then it's left off.
pub fn set_on(on: bool) -> Result<(), String> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config")).join("systemd/user");
    let sc = |a: &[&str]| hearth_ui::run::blocking(&std::iter::once("systemctl".to_string()).chain(std::iter::once("--user".to_string())).chain(a.iter().map(|s| s.to_string())).collect::<Vec<_>>());
    if on && !data::helper_has_lists() {
        let me = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "hearth-shield-app".into());
        let (svc, tmr) = units(&me);
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        for (f, body) in [(format!("{UNIT}.service"), svc), (format!("{UNIT}.timer"), tmr)] {
            let p = base.join(f);
            let tmp = p.with_extension("part");
            std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, &p)).map_err(|e| e.to_string())?;
        }
        let _ = sc(&["daemon-reload"]);
        let o = sc(&["enable", "--now", &format!("{UNIT}.timer")]);
        if !o.ok {
            return Err(o.why());
        }
    } else if !on {
        let _ = sc(&["disable", "--now", &format!("{UNIT}.timer")]);
        let _ = std::fs::remove_file(base.join(format!("{UNIT}.timer")));
        let _ = std::fs::remove_file(base.join(format!("{UNIT}.service")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn todays_numbers_and_the_counter() {
        let s = json!({"dns_queries": [5, 5, 1, 2, 3], "blocked_filtering": [1, 1, 0, 1, 1]});
        assert_eq!(today_of(&s, 3), (6, 2));
        let (svc, tmr) = units("/x/hearth-shield-app");
        assert!(svc.contains("--count") && !svc.contains("[Install]"), "the service is started only by its timer");
        assert!(tmr.contains("OnUnitActiveSec=60"));
        assert_eq!(LISTS.len(), 6);
    }
}
