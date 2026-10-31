//! Hearth Mods' downloads from Nexus Mods: a queue (kept in ~/.local/share/nimbus/mods/downloads.json) that
//! downloads two files at a time with curl, can be paused and picks up where it stopped (curl -C -), and survives
//! closing the app. Files go to mods/downloads/<nexus game>/ unless something asked for another folder (a
//! Wabbajack modlist's downloads folder, with the exact file name the modlist wants).
//!
//! A download address from Nexus runs out after a while. A paused or failed download asks again: Premium accounts
//! get a new address from the API; free accounts need the site's "Mod Manager Download" button once more (the
//! item waits as "needs-link", and the next nxm link for the same file brings it back).

use crate::nexus::{self, Account, Nxm};
use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const AT_ONCE: usize = 2;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub domain: String,
    pub mod_id: u64,
    pub file_id: u64,
    /// What it's called: the mod's name and the file's ("SkyUI · Main file 5.2").
    pub name: String,
    pub file_name: String,
    pub size: u64,
    pub done: u64,
    /// queued, downloading, paused, done, failed, needs-link
    pub state: String,
    #[serde(default)]
    pub error: String,
    pub dir: PathBuf,
    /// The address Nexus gave (it runs out); not shown to the page.
    #[serde(default)]
    pub url: String,
    /// The game in Hearth's library it was downloaded for, if one.
    #[serde(default)]
    pub game: String,
    /// The Wabbajack install that wants it, if one.
    #[serde(default)]
    pub job: String,
    pub added: u64,
    #[serde(default)]
    pub finished: u64,
    #[serde(default, skip)]
    pub speed: u64,
}
impl Item {
    pub fn path(&self) -> PathBuf { self.dir.join(&self.file_name) }
    fn part(&self) -> PathBuf { self.dir.join(format!("{}.hearth-part", self.file_name)) }
    pub fn public(&self) -> Value {
        json!({"id": self.id, "domain": self.domain, "modId": self.mod_id, "fileId": self.file_id, "name": self.name, "fileName": self.file_name,
               "size": self.size, "done": self.done, "state": self.state, "error": self.error, "path": self.path(), "game": self.game, "job": self.job,
               "added": self.added, "finished": self.finished, "speed": self.speed, "page": nexus::files_page(&self.domain, self.mod_id, Some(self.file_id))})
    }
}

#[derive(Default)]
struct Queue { items: Vec<Item>, loaded: bool, running: BTreeMap<String, Arc<Mutex<Option<String>>>> }

static Q: Mutex<Queue> = Mutex::new(Queue { items: Vec::new(), loaded: false, running: BTreeMap::new() });

fn queue_file() -> PathBuf { util::data_dir().join("mods").join("downloads.json") }
pub fn default_dir(domain: &str) -> PathBuf { util::data_dir().join("mods").join("downloads").join(util::safe_name(domain)) }

fn with<R>(f: impl FnOnce(&mut Queue) -> R) -> R {
    let mut q = Q.lock().unwrap_or_else(|e| e.into_inner());
    if !q.loaded {
        q.loaded = true;
        q.items = serde_json::from_value(util::read_json(&queue_file())).unwrap_or_default();
        // Downloads stopped by closing the app go back in the queue.
        for i in q.items.iter_mut() { if i.state == "downloading" { i.state = "queued".into(); } }
    }
    f(&mut q)
}
fn save(q: &Queue) { let _ = util::write_json(&queue_file(), &serde_json::to_value(&q.items).unwrap_or(Value::Null)); }

pub fn list() -> Value {
    with(|q| {
        let active = q.items.iter().filter(|i| matches!(i.state.as_str(), "queued" | "downloading")).count();
        json!({"items": q.items.iter().rev().map(Item::public).collect::<Vec<_>>(), "active": active, "quota": nexus::quota()})
    })
}
pub fn get(id: &str) -> Option<Item> { with(|q| q.items.iter().find(|i| i.id == id).cloned()) }
pub fn for_job(job: &str) -> Vec<Item> { with(|q| q.items.iter().filter(|i| i.job == job).cloned().collect()) }

/// Where a file should go and what it should be called, for something that wants it (a Wabbajack install).
#[derive(Debug, Clone, Default)]
pub struct Want { pub dir: Option<PathBuf>, pub file_name: Option<String>, pub game: String, pub job: String }

/// An nxm:// link from the site: the file is looked up and goes in the queue. If the same file is already waiting
/// for a new link (or failed), that one is brought back instead.
pub fn add_nxm(link: &str, want: Want) -> Result<Item, String> {
    let n = nexus::parse_nxm(link)?;
    let acct = nexus::account().ok_or("Sign in to Nexus Mods in Hearth Mods first (Accounts).")?;
    if let (Some(u), true) = (n.user_id, acct.user_id != 0) { if u != acct.user_id { return Err("That link was made by another Nexus account than the one signed in to Hearth Mods.".into()); } }
    if n.key.is_none() && !acct.premium { return Err("That link has no download key. Press \"Mod Manager Download\" on the mod's page.".into()); }
    let url = nexus::download_link(&acct, &n.domain, n.mod_id, n.file_id, n.key.as_deref().zip(n.expires))?;
    add(&acct, &n, url, want)
}

/// Premium: a file straight from the API.
pub fn add_file(domain: &str, mod_id: u64, file_id: u64, want: Want) -> Result<Item, String> {
    let acct = nexus::account().ok_or("Sign in to Nexus Mods first.")?;
    let n = Nxm { domain: domain.to_ascii_lowercase(), mod_id, file_id, ..Default::default() };
    let url = nexus::download_link(&acct, &n.domain, mod_id, file_id, None)?;
    add(&acct, &n, url, want)
}

fn add(acct: &Account, n: &Nxm, url: String, want: Want) -> Result<Item, String> {
    // Waiting items for the same file (a Wabbajack list asks for one, then the site's button answers) get the link.
    let waiting = with(|q| q.items.iter().position(|i| i.domain == n.domain && i.file_id == n.file_id && matches!(i.state.as_str(), "needs-link" | "failed" | "paused")));
    if let Some(ix) = waiting {
        let it = with(|q| { let i = &mut q.items[ix]; i.url = url.clone(); i.state = "queued".into(); i.error.clear(); let c = i.clone(); save(q); c });
        kick();
        return Ok(it);
    }
    let f = nexus::file_info(&acct.key, &n.domain, n.mod_id, n.file_id).unwrap_or(Value::Null);
    let m = nexus::mod_info(&acct.key, &n.domain, n.mod_id).unwrap_or(Value::Null);
    let file_name = want.file_name.clone().or_else(|| f["file_name"].as_str().map(str::to_string)).or_else(|| url_file_name(&url)).unwrap_or_else(|| format!("{}-{}.7z", n.mod_id, n.file_id));
    let file_name = util::safe_name(&file_name);
    let mod_name = m["name"].as_str().unwrap_or("");
    let file_title = f["name"].as_str().unwrap_or("");
    let name = match (mod_name.is_empty(), file_title.is_empty()) { (false, false) if mod_name != file_title => format!("{mod_name} · {file_title}"), (false, _) => mod_name.into(), (true, false) => file_title.into(), _ => file_name.clone() };
    let size = f["size_in_bytes"].as_u64().or_else(|| f["size_kb"].as_u64().map(|k| k * 1024)).unwrap_or(0);
    let it = Item { id: format!("d{}", util::now_ms()), domain: n.domain.clone(), mod_id: n.mod_id, file_id: n.file_id, name, file_name, size, done: 0, state: "queued".into(),
        error: String::new(), dir: want.dir.unwrap_or_else(|| default_dir(&n.domain)), url, game: want.game, job: want.job, added: util::now_ms(), finished: 0, speed: 0 };
    with(|q| { q.items.push(it.clone()); save(q); });
    kick();
    Ok(it)
}

/// Waiting for a link: an entry for a file a Wabbajack install needs that a free account must click on the site.
pub fn add_needs_link(domain: &str, mod_id: u64, file_id: u64, name: &str, file_name: &str, size: u64, dir: &Path, job: &str) -> Item {
    let exists = with(|q| q.items.iter().find(|i| i.job == job && i.domain == domain && i.file_id == file_id && i.state != "done").cloned());
    if let Some(i) = exists { return i; }
    let it = Item { id: format!("d{}{}", util::now_ms(), file_id % 1000), domain: domain.into(), mod_id, file_id, name: name.into(), file_name: util::safe_name(file_name), size, done: 0,
        state: "needs-link".into(), error: String::new(), dir: dir.to_path_buf(), url: String::new(), game: String::new(), job: job.into(), added: util::now_ms(), finished: 0, speed: 0 };
    with(|q| { q.items.push(it.clone()); save(q); });
    it
}

fn url_file_name(url: &str) -> Option<String> {
    let p = url.split('?').next()?.rsplit('/').next()?;
    let n = nexus::urldecode(p);
    if n.is_empty() { None } else { Some(n) }
}

pub fn pause(id: &str) {
    with(|q| {
        if let Some(stop) = q.running.get(id) { *stop.lock().unwrap_or_else(|e| e.into_inner()) = Some("paused".into()); }
        else if let Some(i) = q.items.iter_mut().find(|i| i.id == id && i.state == "queued") { i.state = "paused".into(); save(q); }
    });
}
pub fn resume(id: &str) -> Result<(), String> {
    let it = get(id).ok_or("That download isn't in the list anymore.")?;
    if it.state == "needs-link" { return Err("Press \"Mod Manager Download\" for it on Nexus Mods (Open on Nexus) to give Hearth a new link.".into()); }
    with(|q| { if let Some(i) = q.items.iter_mut().find(|i| i.id == id) { if i.state != "done" && i.state != "downloading" { i.state = "queued".into(); i.error.clear(); } } save(q); });
    kick();
    Ok(())
}
/// Takes it out of the list. A finished file stays on disk unless `delete_file`.
pub fn remove(id: &str, delete_file: bool) {
    let it = with(|q| {
        if let Some(stop) = q.running.get(id) { *stop.lock().unwrap_or_else(|e| e.into_inner()) = Some("removed".into()); }
        let pos = q.items.iter().position(|i| i.id == id);
        let it = pos.map(|p| q.items.remove(p));
        save(q);
        it
    });
    if let Some(i) = it { let _ = std::fs::remove_file(i.part()); if delete_file && i.state == "done" { let _ = std::fs::remove_file(i.path()); } }
}
pub fn clear_finished() { with(|q| { q.items.retain(|i| i.state != "done"); save(q); }); }

/// Starts queued downloads until AT_ONCE are running.
pub fn kick() {
    let start: Vec<(Item, Arc<Mutex<Option<String>>>)> = with(|q| {
        let mut out = Vec::new();
        let mut running = q.running.len();
        for i in q.items.iter_mut() {
            if running >= AT_ONCE { break; }
            if i.state != "queued" || q.running.contains_key(&i.id) { continue; }
            i.state = "downloading".into();
            let stop = Arc::new(Mutex::new(None));
            q.running.insert(i.id.clone(), stop.clone());
            out.push((i.clone(), stop));
            running += 1;
        }
        if !out.is_empty() { save(q); }
        out
    });
    for (it, stop) in start { std::thread::spawn(move || run(it, stop)); }
}

fn set(id: &str, f: impl FnOnce(&mut Item)) { with(|q| { if let Some(i) = q.items.iter_mut().find(|i| i.id == id) { f(i); } save(q); }); }

fn run(mut it: Item, stop: Arc<Mutex<Option<String>>>) {
    let outcome = (|| -> Result<(), String> {
        std::fs::create_dir_all(&it.dir).map_err(|e| format!("Couldn't make the folder {} ({e})", it.dir.display()))?;
        if it.url.is_empty() { it.url = fresh_url(&it)?; set(&it.id, |i| i.url = it.url.clone()); }
        let mut tried_fresh = false;
        loop {
            match fetch(&it, &stop) {
                Ok(()) => return Ok(()),
                Err(Fetch::Stopped) => return Err(String::new()),
                Err(Fetch::Expired) if !tried_fresh => { tried_fresh = true; it.url = fresh_url(&it)?; set(&it.id, |i| i.url = it.url.clone()); }
                Err(Fetch::Expired) => return Err("needs-link".into()),
                Err(Fetch::Failed(e)) => return Err(e),
            }
        }
    })();
    let why = stop.lock().unwrap_or_else(|e| e.into_inner()).clone();
    with(|q| { q.running.remove(&it.id); });
    match (outcome, why.as_deref()) {
        (_, Some("removed")) => { let _ = std::fs::remove_file(it.part()); }
        (_, Some("paused")) => set(&it.id, |i| { i.state = "paused".into(); i.speed = 0; }),
        (Ok(()), _) => set(&it.id, |i| { i.state = "done".into(); i.done = i.size.max(i.done); i.speed = 0; i.finished = util::now_ms(); i.url.clear(); }),
        (Err(e), _) if e == "needs-link" => set(&it.id, |i| { i.state = "needs-link".into(); i.speed = 0; i.url.clear(); i.error = "The download link ran out. Press \"Mod Manager Download\" on the site again.".into(); }),
        (Err(e), _) => set(&it.id, |i| { i.state = "failed".into(); i.speed = 0; i.error = e; }),
    }
    kick();
}

/// A new address for a download: Premium asks the API; free accounts can't.
fn fresh_url(it: &Item) -> Result<String, String> {
    let acct = nexus::account().ok_or("Sign in to Nexus Mods first.")?;
    if !acct.premium { return Err("needs-link".into()); }
    nexus::download_link(&acct, &it.domain, it.mod_id, it.file_id, None)
}

enum Fetch { Stopped, Expired, Failed(String) }

/// One go with curl: into the .hearth-part file (continuing it), then renamed. Progress is read off the file's size.
fn fetch(it: &Item, stop: &Arc<Mutex<Option<String>>>) -> Result<(), Fetch> {
    use std::io::Write;
    let part = it.part();
    let cfg = format!("url = {}\noutput = {}\n", nexus::curl_quote(&it.url), nexus::curl_quote(&part.to_string_lossy()));
    let mut child = std::process::Command::new("curl")
        .args(["-sS", "-L", "-f", "--proto", "=https", "--proto-redir", "=https", "-C", "-", "--retry", "3", "--retry-delay", "2", "--connect-timeout", "30",
               "--speed-limit", "1", "--speed-time", "120", "-A", concat!("Hearth/", env!("CARGO_PKG_VERSION")), "-w", "%{http_code}", "-K", "-"])
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| Fetch::Failed(format!("Couldn't start curl ({e})")))?;
    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(cfg.as_bytes()); }
    let (mut last, mut last_t) = (std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0), std::time::Instant::now());
    let status = loop {
        if stop.lock().unwrap_or_else(|e| e.into_inner()).is_some() { let _ = child.kill(); let _ = child.wait(); return Err(Fetch::Stopped); }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {}
            Err(e) => return Err(Fetch::Failed(e.to_string())),
        }
        std::thread::sleep(Duration::from_millis(500));
        let now = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let dt = last_t.elapsed().as_secs_f64();
        if dt >= 1.0 { let sp = ((now.saturating_sub(last)) as f64 / dt) as u64; last = now; last_t = std::time::Instant::now(); with(|q| { if let Some(i) = q.items.iter_mut().find(|i| i.id == it.id) { i.done = now; i.speed = sp; } }); }
    };
    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut o) = child.stdout.take() { use std::io::Read; let _ = o.read_to_string(&mut out); }
    if let Some(mut e) = child.stderr.take() { use std::io::Read; let _ = e.read_to_string(&mut err); }
    let code: u16 = out.trim().parse().unwrap_or(0);
    if status.success() {
        let len = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if it.size > 0 && len + 4096 < it.size { return Err(Fetch::Failed(format!("The download stopped early ({} of {}).", len, it.size))); }
        let dest = it.path();
        std::fs::rename(&part, &dest).map_err(|e| Fetch::Failed(format!("Couldn't keep the file ({e})")))?;
        set(&it.id, |i| { i.done = len; if i.size == 0 { i.size = len; } });
        return Ok(());
    }
    // 416: the part file is already all of it.
    if code == 416 { if std::fs::rename(&part, it.path()).is_ok() { return Ok(()); } }
    if matches!(code, 401 | 403 | 404 | 410) { return Err(Fetch::Expired); }
    Err(Fetch::Failed(if code == 0 { format!("No connection ({})", err.trim()) } else { format!("The download failed ({code}).") }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_names_from_addresses() {
        assert_eq!(url_file_name("https://cf-files.nexusmods.com/cdn/1704/12604/SkyUI%205.2-12604-5-2.7z?md5=x&expires=1").as_deref(), Some("SkyUI 5.2-12604-5-2.7z"));
        assert_eq!(url_file_name("https://x/"), None);
    }
    #[test]
    fn items_know_their_files() {
        let i = Item { dir: "/d".into(), file_name: "a.7z".into(), ..Default::default() };
        assert_eq!(i.path(), PathBuf::from("/d/a.7z"));
        assert_eq!(i.part(), PathBuf::from("/d/a.7z.hearth-part"));
        assert!(i.public()["url"].is_null(), "the address isn't given to the page");
    }
}
