//! Settings › Import games: bring games and personal files over from this PC's other drives (Windows, SteamOS,
//! Bazzite and other Linux, second drives, SD cards), and afterwards remove the old system. Port of
//! hosts/linux/src/importer.js. The work on the drives is done by hearth-import (as root through hearth-admin);
//! this side keeps the state for the page and, once copied, registers the games: GOG in Hearth's GOG list,
//! Lutris games as your own games, Epic games with legendary (now, or as soon as you sign in to Epic).
//! Changes to library.json go through the host ({type:'internal', what:'importDone'}), which owns the library.

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub struct Importer { pub state: Arc<Mutex<Value>>, pid: Arc<Mutex<Option<u32>>>, cancelled: Arc<std::sync::atomic::AtomicBool>, epic_busy: Arc<std::sync::atomic::AtomicBool> }

impl Default for Importer { fn default() -> Self { Self::new() } }

fn push(state: &Value, pending: usize) {
    let mut s = state.clone();
    s["supported"] = json!(crate::admin::can_admin());
    s["pendingEpic"] = json!(pending);
    crate::stores::notify(json!({"type": "event", "name": "import", "data": s}));
}
/// Stop for hearth-import: /run/user/<uid>/hearth-import.cancel (it checks for it while copying and checking).
pub fn cancel_flag() -> std::path::PathBuf {
    let uid = std::fs::read_to_string("/proc/self/status").ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("Uid:").and_then(|v| v.split_whitespace().next().map(str::to_string))))
        .unwrap_or_else(|| "0".into());
    std::path::PathBuf::from(format!("/run/user/{uid}/hearth-import.cancel"))
}
fn pending_count() -> usize {
    std::fs::read(crate::paths::data_dir().join("library.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|d| d["pendingEpicImports"].as_array().map(|a| a.len())).unwrap_or(0)
}

impl Importer {
    pub fn new() -> Importer {
        Importer { state: Arc::new(Mutex::new(json!({"state": "idle", "scan": null, "scannedAt": 0, "progress": null, "speeds": [], "result": null, "error": null}))),
                   pid: Arc::new(Mutex::new(None)), cancelled: Arc::default(), epic_busy: Arc::default() }
    }
    pub fn status(&self, pending: usize) -> Value {
        let mut s = self.state.lock().unwrap().clone();
        s["supported"] = json!(crate::admin::can_admin());
        s["pendingEpic"] = json!(pending);
        s
    }
    fn set(&self, patch: Value) {
        let mut s = self.state.lock().unwrap();
        for (k, v) in patch.as_object().cloned().unwrap_or_default() { s[k] = v; }
        push(&s, pending_count());
    }
    fn need(&self) -> Result<(), String> { if crate::admin::can_admin() { Ok(()) } else { Err("Importing needs Hearth, the OS.".into()) } }
    fn idle(&self) -> bool { self.state.lock().unwrap()["state"] == "idle" }

    /// Looks at the other drives (blocking: up to a few minutes on slow drives, so it runs in the background).
    pub fn scan(&self, pending: usize) -> Result<Value, String> {
        self.need()?;
        if !self.idle() { return Ok(self.status(pending)); }
        self.set(json!({"state": "scanning", "error": null, "result": null}));
        let state = self.state.clone();
        std::thread::spawn(move || {
            let r = crate::admin::run(&["import-scan"], None, &mut |_| {});
            let mut s = state.lock().unwrap();
            s["state"] = json!("idle");
            match r { Ok(scan) => { s["scan"] = scan; s["scannedAt"] = json!(crate::library::now_ms()); } Err(e) => s["error"] = json!(e) }
            push(&s, pending_count());
        });
        Ok(self.status(pending))
    }

    /// keys: items from the scan. mode: 'copy' or 'move' (originals deleted after a good copy).
    pub fn start(&self, keys: &Value, mode: &str, root: &std::path::Path, pending: usize) -> Result<Value, String> {
        self.need()?;
        if !self.idle() { return Err("Already busy".into()); }
        let n = keys.as_array().map(|a| a.len()).unwrap_or(0);
        if n == 0 { return Err("Pick something to import first".into()); }
        if mode != "copy" && mode != "move" { return Err("Copy or move".into()); }
        self.set(json!({"state": "importing", "error": null, "result": null, "speeds": [], "started": crate::library::now_ms(), "mode": mode,
                        "progress": {"overall": 0, "total": 0, "i": 0, "n": n, "name": "", "done": 0, "size": 0, "bps": 0}}));
        let input = format!("{}\n", json!({"mode": mode, "items": keys, "root": root.to_string_lossy()}));
        let _ = std::fs::remove_file(cancel_flag());
        let (state, pid, cancelled) = (self.state.clone(), self.pid.clone(), self.cancelled.clone());
        std::thread::spawn(move || {
            let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
            let r = crate::admin::run_with(&["import-run"], Some(&input), &mut |line: &str| {
                let Some(st) = line.strip_prefix("stage: ").and_then(|j| serde_json::from_str::<Value>(j).ok()) else { return };
                let mut s = state.lock().unwrap();
                match st["phase"].as_str() {
                    Some("start") => { s["progress"]["total"] = st["total"].clone(); s["progress"]["n"] = st["n"].clone(); }
                    Some("done") => { s["progress"]["overall"] = st["total"].clone(); }
                    _ => {
                        for (k, v) in st.as_object().cloned().unwrap_or_default() { s["progress"][k] = v; }
                        if let Some(bps) = st["bps"].as_u64().filter(|b| *b > 0) {
                            let now = crate::library::now_ms();
                            let mut sp: Vec<Value> = s["speeds"].as_array().cloned().unwrap_or_default().into_iter().filter(|p| now.saturating_sub(p[0].as_u64().unwrap_or(0)) < 300_000).collect();
                            sp.push(json!([now, bps]));
                            let skip = sp.len().saturating_sub(400);
                            s["speeds"] = json!(sp[skip..]);
                        }
                        if last.elapsed().as_millis() < 500 { return; }
                    }
                }
                last = std::time::Instant::now();
                push(&s, pending_count());
            }, &mut |p| { *pid.lock().unwrap() = Some(p); });
            *pid.lock().unwrap() = None;
            let stopped = cancelled.swap(false, std::sync::atomic::Ordering::SeqCst);
            match r {
                // The host registers what was copied, then this finishes (finish()).
                Ok(r) => crate::stores::notify(json!({"type": "internal", "what": "importDone", "result": r})),
                Err(e) => { let mut s = state.lock().unwrap(); s["state"] = json!("idle"); s["error"] = json!(if stopped { "Stopped".to_string() } else { e }); push(&s, pending_count()); }
            }
        });
        Ok(self.status(pending))
    }

    /// First start after installing (often next to Windows): looks once for games on other drives and offers
    /// to bring them over.
    pub fn offer(&self) {
        if !crate::admin::can_admin() || !self.idle() { return; }
        let state = self.state.clone();
        std::thread::spawn(move || {
            let Ok(scan) = crate::admin::run(&["import-scan"], None, &mut |_| {}) else { return };
            let len = |d: &Value, k: &str| d[k].as_array().map(|a| a.len()).unwrap_or(0);
            let drives = scan["drives"].as_array().cloned().unwrap_or_default();
            let count = |d: &Value| len(d, "steam") + len(d, "epic") + len(d, "gog") + len(d, "lutris");
            let games: usize = drives.iter().map(count).sum();
            let mut s = state.lock().unwrap();
            if s["state"] != "idle" { return; }
            s["scan"] = scan.clone(); s["scannedAt"] = json!(crate::library::now_ms());
            if games > 0 {
                let from: Vec<Value> = drives.iter().filter(|d| count(d) > 0).map(|d| d["name"].clone()).collect();
                crate::bridge::log(&format!("import: offering {games} games from {}", json!(from)));
                s["offer"] = json!({"games": games, "from": from});
            }
            push(&s, pending_count());
        });
    }

    pub fn cancel(&self, pending: usize) -> Value {
        if self.state.lock().unwrap()["state"] == "importing" {
            // hearth-import runs as root (pkexec), which this app may not be allowed to signal: it also watches
            // for this file. Either way it finishes or removes the item in progress and reports what's done.
            let _ = std::fs::write(cancel_flag(), b"");
            if let Some(p) = *self.pid.lock().unwrap() { self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst); let _ = std::process::Command::new("kill").args(["-TERM", &p.to_string()]).status(); }
        }
        self.status(pending)
    }

    /// After a good import: notes for the page, and the import state back to idle (with the result).
    pub fn finish(&self, r: &Value, notes: Vec<String>) {
        let mut res = r.clone();
        res["notes"] = json!(notes);
        self.set(json!({"state": "idle", "result": res}));
        self.state.lock().unwrap()["scan"] = Value::Null;   // the drives changed: scan again rather than show a stale list
    }

    /// Epic games copied over get registered with legendary (needs an Epic sign-in). In the background; the host
    /// hears {what:'epicImported', left:[...]} with the ones still waiting.
    pub fn finish_pending_epic(&self, pending: Vec<Value>) {
        if pending.is_empty() || !crate::stores::epic::signed_in() || self.epic_busy.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }
        let busy = self.epic_busy.clone();
        std::thread::spawn(move || {
            let mut left = Vec::new();
            let mut done = 0;
            for g in pending {
                let (app, path) = (g["app"].as_str().unwrap_or(""), g["path"].as_str().unwrap_or(""));
                match crate::stores::epic::import_game(app, path) {
                    Ok(()) => { done += 1; crate::bridge::log(&format!("import: Epic {app} registered")); }
                    Err(e) => { crate::bridge::log(&format!("import: Epic {app}: {e}")); if !e.contains("didn't accept") { left.push(g.clone()); } }
                }
            }
            busy.store(false, std::sync::atomic::Ordering::SeqCst);
            crate::stores::notify(json!({"type": "internal", "what": "epicImported", "left": left, "done": done}));
        });
    }

    // ------------------------------------------------------------------ remove the old system
    // Erases the other systems' partitions on Hearth's drive and gives Hearth the space (hearth-takeover).
    // Only partitions it offers can be chosen, never Hearth's own.

    pub fn takeover_plan(&self, pending: usize) -> Result<Value, String> {
        if !crate::admin::can_admin() { return Err("This needs Hearth, the OS.".into()); }
        if !self.idle() { return Ok(self.status(pending)); }
        match crate::admin::run(&["takeover-plan"], None, &mut |_| {}) {
            Ok(p) => self.set(json!({"takeover": p, "takeoverError": null})),
            Err(e) => self.set(json!({"takeover": null, "takeoverError": e})),
        }
        Ok(self.status(pending))
    }
    pub fn takeover_run(&self, remove: &Value, windows_boot: bool, pending: usize) -> Result<Value, String> {
        if !self.idle() { return Err("Already busy".into()); }
        if remove.as_array().map(|a| a.is_empty()).unwrap_or(true) { return Err("Choose what to remove first".into()); }
        self.set(json!({"state": "takeover", "takeoverStage": {"phase": "erase", "step": 0, "steps": 3, "desc": "Starting"}, "takeoverResult": null, "takeoverError": null}));
        let input = format!("{}\n", json!({"remove": remove, "windowsBoot": windows_boot}));
        let state = self.state.clone();
        std::thread::spawn(move || {
            let r = crate::admin::run(&["takeover-run"], Some(&input), &mut |line: &str| {
                if let Some(st) = line.strip_prefix("stage: ").and_then(|j| serde_json::from_str::<Value>(j).ok()) {
                    let mut s = state.lock().unwrap(); s["takeoverStage"] = st; push(&s, pending_count());
                }
            });
            let mut s = state.lock().unwrap();
            s["state"] = json!("idle");
            match r {
                Ok(v) => { crate::bridge::log(&format!("takeover: done {v}")); s["takeoverResult"] = if v.is_null() { json!({}) } else { v }; s["takeover"] = Value::Null; s["scan"] = Value::Null; }
                Err(e) => { crate::bridge::log(&format!("takeover: {e}")); s["takeoverError"] = json!(e); }
            }
            push(&s, pending_count());
        });
        Ok(self.status(pending))
    }
}
