//! Copying a game from another Hearth device, as fast as the link goes.
//!
//! The receiving side drives it (a copy someone sends is an offer it accepts first):
//! 1. It asks for the game's manifest (manifest.rs) and keeps it with the copy (transfers/<id>/), so the copy can
//!    resume later, even after a restart.
//! 2. It checks there's room, makes the folders and links, and for each file decides what's needed: nothing when
//!    the same file is already there (same size and time), every chunk for a new file (allocated in one go with
//!    fallocate, so the drive doesn't fragment it), or only the chunks that differ for a file that's there but
//!    different (an older version of the game: both sides hash their copy, BLAKE3 per 1 MiB chunk, and only the
//!    chunks whose hashes differ are sent). A copy that was interrupted picks up from its journal.
//! 3. The chunks come over several QUIC streams at once (8 by default), each asking for a run of chunks; the
//!    sending side reads, hashes and (only where it helps) compresses them in their own threads, so the link stays
//!    full. Small files go many to a request.
//! 4. Each chunk is decompressed and checked against its hash before it's written (with pwrite, by a few writer
//!    threads): verified as it's written, no second pass. The journal records written chunks; every few seconds
//!    the file system is flushed first and then the journal, so the journal never claims a chunk that isn't on the
//!    drive.
//! 5. When every file is complete the store is told (games.rs finalize) and the copy's own folder is removed.

use crate::manifest::{resolve_inside, safe_rel, Entry};
use crate::wire::{self, Header, CHUNK, HEADER, T_CHUNK, T_DONE, T_ERROR};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Journal entry chunk number meaning "this copy made the file" (so its contents are only what the journal says).
const MADE: u32 = u32::MAX;
/// Files at least this big that are already here but different are compared chunk by chunk; smaller ones are sent whole.
const DELTA_MIN: u64 = 4 << 20;
/// A request: up to this much data, or this many files.
const UNIT_BYTES: u64 = 32 << 20;
const UNIT_ITEMS: usize = 256;

// ------------------------------------------------------------------ the sending side

/// A game's manifest: its roots, every file, and what the other side needs to put it in place.
pub fn manifest_for(game: &crate::games::LocalGame, with_prefix: bool) -> Result<Value, String> {
    let mut roots = Vec::new();
    let mut files: Vec<Entry> = Vec::new();
    for root in &game.roots {
        if root.kind == "prefix" && !with_prefix { continue; }
        if !root.path.exists() { if root.kind == "install" { return Err("The game's folder isn't there anymore".into()); } continue; }
        let r = roots.len() as u32;
        crate::manifest::walk(&root.path, r, &root.skip, &mut files).map_err(|e| format!("Reading the game's files: {e}"))?;
        roots.push(json!({"kind": root.kind}));
    }
    let size: u64 = files.iter().filter(|e| e.k == "f").map(|e| e.s).sum();
    let mut meta = game.meta.clone();
    // Hearth's controller layout for the game comes along.
    if let Ok(f) = hearth_core::input_map::file_name(&game.id) {
        if let Some(v) = std::fs::read(hearth_core::input_map::dir().join(f)).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) { meta["input"] = v; }
    }
    Ok(json!({"game": {"id": game.id, "title": game.title, "store": game.store, "meta": meta}, "roots": roots, "files": files, "size": size}))
}

/// The paths of a game's roots that a request may read: the same ones the manifest listed.
pub fn root_paths(game: &crate::games::LocalGame, with_prefix: bool) -> Vec<PathBuf> {
    game.roots.iter().filter(|r| (r.kind != "prefix" || with_prefix) && r.path.exists()).map(|r| r.path.clone()).collect()
}

#[derive(Debug, Clone)]
pub struct Item { pub r: usize, pub p: String, pub c0: u32, pub c1: u32 }

pub fn parse_items(req: &Value) -> Result<Vec<Item>, String> {
    let list = req["items"].as_array().ok_or("No items")?;
    if list.len() > 4096 { return Err("Too many items".into()); }
    list.iter().map(|i| {
        let p = i["p"].as_str().filter(|p| safe_rel(p).is_some()).ok_or("Bad path")?.to_string();
        let c0 = i["c0"].as_u64().unwrap_or(0) as u32;
        let c1 = i["c1"].as_u64().unwrap_or(0) as u32;
        if c1 < c0 || c1 - c0 > 1 << 20 { return Err("Bad range".to_string()); }
        Ok(Item { r: i["r"].as_u64().unwrap_or(0) as usize, p, c0, c1 })
    }).collect()
}

fn advise_sequential(f: &std::fs::File) {
    use std::os::unix::io::AsRawFd;
    unsafe { libc::posix_fadvise(f.as_raw_fd(), 0, 0, libc::POSIX_FADV_SEQUENTIAL); }
}

/// Reads, hashes and compresses the chunks a request asks for, in this (blocking) thread, handing whole frames to
/// `tx`. Ends with a done frame, or an error frame.
pub fn produce(roots: Vec<PathBuf>, items: Vec<Item>, compress: bool, tx: tokio::sync::mpsc::Sender<bytes::Bytes>) {
    let fail = |tx: &tokio::sync::mpsc::Sender<bytes::Bytes>, m: &str| { let _ = tx.blocking_send(bytes::Bytes::from(wire::control_frame(T_ERROR, m))); };
    for (i, it) in items.iter().enumerate() {
        let Some(path) = roots.get(it.r).and_then(|root| resolve_inside(root, &it.p)) else { return fail(&tx, "A file of the game isn't there anymore") };
        let f = match std::fs::File::open(&path) { Ok(f) => f, Err(e) => return fail(&tx, &format!("Couldn't read {}: {e}", it.p)) };
        advise_sequential(&f);
        let size = f.metadata().map(|m| m.len()).unwrap_or(0);
        for c in it.c0..it.c1 {
            let off = c as u64 * CHUNK;
            if off >= size && size > 0 || (size == 0 && c > 0) { return fail(&tx, "The game changed on the other device while it was being copied: try again"); }
            let len = CHUNK.min(size - off.min(size)) as usize;
            let mut buf = vec![0u8; len];
            if let Err(e) = f.read_exact_at(&mut buf, off) { return fail(&tx, &format!("Couldn't read {}: {e}", it.p)); }
            if tx.blocking_send(bytes::Bytes::from(wire::chunk_frame(i as u32, c, &buf, compress))).is_err() { return; }
        }
    }
    let _ = tx.blocking_send(bytes::Bytes::from(wire::control_frame(T_DONE, "")));
}

/// Hashes of every chunk of each requested file (for comparing with an older copy on the other side).
pub fn hash_files(roots: &[PathBuf], req: &Value) -> Result<Value, String> {
    let list = req["items"].as_array().ok_or("No items")?;
    let mut out = Vec::new();
    for i in list.iter().take(64) {
        let p = i["p"].as_str().ok_or("Bad path")?;
        let path = roots.get(i["r"].as_u64().unwrap_or(0) as usize).and_then(|root| resolve_inside(root, p)).ok_or("A file of the game isn't there anymore")?;
        out.push(json!(chunk_hashes(&path, None)?));
    }
    Ok(json!({"hashes": out}))
}

/// BLAKE3 of each chunk of a file, hex. With `size`: as if the file were that long (chunks past its end are "").
pub fn chunk_hashes(path: &Path, size: Option<u64>) -> Result<Vec<String>, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    advise_sequential(&f);
    let have = f.metadata().map(|m| m.len()).unwrap_or(0);
    let size = size.unwrap_or(have);
    let n = size.div_ceil(CHUNK);
    let mut out = Vec::with_capacity(n as usize);
    let mut buf = vec![0u8; CHUNK as usize];
    for c in 0..n {
        let off = c * CHUNK;
        let len = CHUNK.min(size - off) as usize;
        if off + len as u64 > have { out.push(String::new()); continue; }
        f.read_exact_at(&mut buf[..len], off).map_err(|e| e.to_string())?;
        out.push(blake3::hash(&buf[..len]).to_hex().to_string());
    }
    Ok(out)
}

// ------------------------------------------------------------------ the receiving side

/// A copy coming to this device (also what the UI shows).
pub struct Job {
    pub id: String,
    pub peer: String,
    pub game_id: String,
    pub title: String,
    pub store: String,
    pub place: PathBuf,
    pub with_prefix: bool,
    pub dir: PathBuf,
    pub state: Mutex<String>,
    pub error: Mutex<Option<String>>,
    pub note: Mutex<Option<String>>,
    pub total: AtomicU64,
    pub done: AtomicU64,
    pub skipped: AtomicU64,
    pub wire: AtomicU64,
    pub relay: AtomicBool,
    /// Pause or cancel asked: the workers stop at the next chunk.
    pub stop: AtomicBool,
    pub cancel: AtomicBool,
    pub running: AtomicBool,
    pub created: u64,
    pub data_ms: AtomicU64,
    samples: Mutex<VecDeque<(Instant, u64)>>,
}

impl Job {
    pub fn new(dir: PathBuf, id: String, peer: String, game_id: String, title: String, store: String, place: PathBuf, with_prefix: bool) -> Job {
        Job { id, peer, game_id, title, store, place, with_prefix, dir, state: Mutex::new("waiting".into()), error: Mutex::new(None), note: Mutex::new(None),
              total: AtomicU64::new(0), done: AtomicU64::new(0), skipped: AtomicU64::new(0), wire: AtomicU64::new(0), relay: AtomicBool::new(false),
              stop: AtomicBool::new(false), cancel: AtomicBool::new(false), running: AtomicBool::new(false), created: crate::now_ms(), data_ms: AtomicU64::new(0),
              samples: Mutex::new(VecDeque::new()) }
    }
    pub fn set_state(&self, s: &str) { *crate::lock(&self.state) = s.to_string(); let _ = self.save(); }
    pub fn state(&self) -> String { crate::lock(&self.state).clone() }
    pub fn save(&self) -> Result<(), String> {
        let st = self.state();
        // A finished or cancelled copy has no folder of its own anymore.
        if st == "done" || st == "cancelled" { return Ok(()); }
        let want = if st == "paused" || st == "cancelled" || st == "done" || st == "error" { st.as_str() } else { "run" };
        let v = json!({"id": self.id, "peer": self.peer, "game": self.game_id, "title": self.title, "store": self.store, "place": self.place,
                       "prefix": self.with_prefix, "created": self.created, "want": want, "error": crate::lock(&self.error).clone()});
        let _ = std::fs::create_dir_all(&self.dir);
        crate::write_atomic(&self.dir.join("job.json"), v.to_string().as_bytes())
    }
    pub fn load(dir: &Path) -> Option<Job> {
        let v: Value = serde_json::from_slice(&std::fs::read(dir.join("job.json")).ok()?).ok()?;
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        let j = Job::new(dir.to_path_buf(), s("id"), s("peer"), s("game"), s("title"), s("store"), PathBuf::from(s("place")), v["prefix"] == true);
        let want = s("want");
        *crate::lock(&j.state) = match want.as_str() { "run" => "interrupted".into(), w => w.to_string() };
        *crate::lock(&j.error) = v["error"].as_str().map(str::to_string);
        Some(j)
    }
    /// Bytes per second over the last few seconds.
    pub fn speed(&self) -> f64 {
        let now = Instant::now();
        let done = self.done.load(Ordering::Relaxed);
        let mut s = crate::lock(&self.samples);
        s.push_back((now, done));
        while s.len() > 2 && now.duration_since(s[0].0).as_secs_f64() > 5.0 { s.pop_front(); }
        let (t0, d0) = s[0];
        let dt = now.duration_since(t0).as_secs_f64();
        if dt < 0.2 { return 0.0; }
        (done.saturating_sub(d0)) as f64 / dt
    }
    pub fn to_json(&self) -> Value {
        let (total, done) = (self.total.load(Ordering::Relaxed), self.done.load(Ordering::Relaxed));
        let speed = if self.state() == "copying" { self.speed() } else { 0.0 };
        let eta = if speed > 1.0 { Some(((total.saturating_sub(done)) as f64 / speed).round() as u64) } else { None };
        let ms = self.data_ms.load(Ordering::Relaxed);
        json!({"id": self.id, "peer": self.peer, "game": self.game_id, "title": self.title, "store": self.store, "state": self.state(),
               "total": total, "done": done, "skipped": self.skipped.load(Ordering::Relaxed), "wire": self.wire.load(Ordering::Relaxed),
               "speed": speed.round(), "eta": eta, "relay": self.relay.load(Ordering::Relaxed), "error": crate::lock(&self.error).clone(),
               "note": crate::lock(&self.note).clone(), "place": self.place, "prefix": self.with_prefix, "created": self.created,
               "dataMs": ms, "avgSpeed": if ms > 0 { (done as f64 * 1000.0 / ms as f64).round() } else { 0.0 }})
    }
}

/// Written chunks: (file, chunk) as 8 bytes each, appended; MADE marks a file this copy created.
struct Journal { file: std::fs::File, pending: Vec<u8>, roots: Vec<PathBuf> }

fn load_journal(path: &Path) -> HashSet<(u32, u32)> {
    let b = std::fs::read(path).unwrap_or_default();
    b.chunks_exact(8).map(|c| (u32::from_le_bytes(c[0..4].try_into().unwrap()), u32::from_le_bytes(c[4..8].try_into().unwrap()))).collect()
}

fn syncfs(dir: &Path) {
    use std::os::unix::io::AsRawFd;
    if let Ok(d) = std::fs::File::open(dir) { unsafe { libc::syncfs(d.as_raw_fd()); } }
}

impl Journal {
    /// Flushes the file systems the copy writes to, then the journal: what it says is on the drive.
    fn flush(j: &Mutex<Journal>) {
        let (pending, roots) = { let mut g = crate::lock(j); (std::mem::take(&mut g.pending), g.roots.clone()) };
        if pending.is_empty() { return; }
        for r in &roots { syncfs(r); }
        let g = crate::lock(j);
        use std::io::Write;
        let mut f = &g.file;
        let _ = f.write_all(&pending);
        let _ = g.file.sync_data();
    }
}

struct Ctx {
    files: Vec<Entry>,
    paths: Vec<Option<PathBuf>>,
    handles: Mutex<HashMap<u32, Arc<std::fs::File>>>,
    remaining: Vec<AtomicU32>,
    journal: Mutex<Journal>,
    failed: Mutex<Option<String>>,
}

struct WJob { fidx: u32, chunk: u32, h: Header, payload: Vec<u8> }

fn set_times_and_mode(path: &Path, e: &Entry) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(f) = std::fs::OpenOptions::new().write(true).open(path) {
        let t = std::time::UNIX_EPOCH + std::time::Duration::new(e.m.max(0) as u64, e.n.min(999_999_999));
        let _ = f.set_times(std::fs::FileTimes::new().set_modified(t).set_accessed(std::time::SystemTime::now()));
    }
    if e.x != 0 { let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(e.x & 0o7777)); }
}

fn same_time(md: &std::fs::Metadata, e: &Entry) -> bool {
    use std::os::unix::fs::MetadataExt;
    md.mtime() == e.m && md.mtime_nsec() as u32 == e.n
}

fn writer(ctx: Arc<Ctx>, job: Arc<Job>, rx: Arc<Mutex<tokio::sync::mpsc::Receiver<WJob>>>) {
    loop {
        let next = { let mut r = crate::lock(&rx); r.blocking_recv() };
        let Some(w) = next else { return };
        if crate::lock(&ctx.failed).is_some() { continue; }
        let r = (|| -> Result<(), String> {
            let e = ctx.files.get(w.fidx as usize).ok_or("Unknown file")?;
            let off = w.chunk as u64 * CHUNK;
            let want = CHUNK.min(e.s.saturating_sub(off));
            if off >= e.s.max(1) || w.h.raw as u64 != want { return Err("The game changed on the other device while it was being copied: try again".into()); }
            let data = wire::open_chunk(&w.h, w.payload)?;
            let f = {
                let mut hs = crate::lock(&ctx.handles);
                match hs.get(&w.fidx) {
                    Some(f) => f.clone(),
                    None => {
                        let p = ctx.paths[w.fidx as usize].as_ref().ok_or("No place for a file")?;
                        let f = Arc::new(std::fs::OpenOptions::new().write(true).open(p).map_err(|er| format!("{}: {er}", p.display()))?);
                        hs.insert(w.fidx, f.clone());
                        f
                    }
                }
            };
            f.write_all_at(&data, off).map_err(|er| format!("Writing {}: {er}", e.p))?;
            { let mut j = crate::lock(&ctx.journal); j.pending.extend_from_slice(&w.fidx.to_le_bytes()); j.pending.extend_from_slice(&w.chunk.to_le_bytes()); }
            job.done.fetch_add(data.len() as u64, Ordering::Relaxed);
            if ctx.remaining[w.fidx as usize].fetch_sub(1, Ordering::AcqRel) == 1 {
                crate::lock(&ctx.handles).remove(&w.fidx);
                if let Some(p) = &ctx.paths[w.fidx as usize] { set_times_and_mode(p, e); }
            }
            Ok(())
        })();
        if let Err(e) = r { let mut f = crate::lock(&ctx.failed); if f.is_none() { *f = Some(e); } job.stop.store(true, Ordering::SeqCst); }
    }
}

fn fallocate(f: &std::fs::File, len: u64) {
    use std::os::unix::io::AsRawFd;
    if len == 0 { return; }
    // Allocates the whole file at once (contiguous where the drive can); a file system without it just grows the file.
    let r = unsafe { libc::fallocate(f.as_raw_fd(), 0, 0, len as libc::off_t) };
    if r != 0 { let _ = f.set_len(len); }
}

/// What a file needs: the chunks still to come.
struct Plan { need: Vec<(u32, Vec<u32>)>, delta: Vec<u32>, needed_bytes: u64 }

fn chunk_len(e: &Entry, c: u32) -> u64 { CHUNK.min(e.s.saturating_sub(c as u64 * CHUNK)) }

/// Makes folders and links, allocates new files, and works out what each file needs. Blocking.
fn prepare(files: &[Entry], paths: &[Option<PathBuf>], dests: &[Option<PathBuf>], journal: &HashSet<(u32, u32)>, jfile: &mut std::fs::File, job: &Job) -> Result<Plan, String> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let mut plan = Plan { need: Vec::new(), delta: Vec::new(), needed_bytes: 0 };
    let mut made: Vec<u8> = Vec::new();
    let in_root = |i: usize| -> Result<PathBuf, String> {
        let e = &files[i];
        let root = dests.get(e.r as usize).cloned().flatten().ok_or("No place for a file")?;
        resolve_inside(&root, &e.p).ok_or_else(|| format!("{} can't be put there safely", e.p))
    };
    for (i, e) in files.iter().enumerate() {
        if job.stop.load(Ordering::Relaxed) { return Err("stopped".into()); }
        let Some(path) = &paths[i] else { continue };
        match e.k.as_str() {
            "d" => { std::fs::create_dir_all(path).map_err(|er| format!("{}: {er}", path.display()))?; in_root(i)?; }
            "l" => {
                let t = e.t.clone().unwrap_or_default();
                if !crate::manifest::link_ok(&e.p, &t) { continue; }
                let real = in_root(i)?;
                if std::fs::read_link(&real).map(|x| x.to_string_lossy() == t).unwrap_or(false) { continue; }
                if let Ok(md) = std::fs::symlink_metadata(&real) { if !md.is_dir() { let _ = std::fs::remove_file(&real); } }
                std::os::unix::fs::symlink(&t, &real).map_err(|er| format!("{}: {er}", e.p))?;
            }
            "f" => {
                let real = in_root(i)?;
                let md = std::fs::symlink_metadata(&real).ok();
                if md.as_ref().map(|m| m.file_type().is_symlink() || m.is_dir()).unwrap_or(false) {
                    // Something else is in the way (a link or a folder where the file goes): replaced.
                    let _ = std::fs::remove_file(&real);
                }
                let md = std::fs::metadata(&real).ok().filter(|m| m.is_file());
                let n = e.chunks() as u32;
                let fidx = i as u32;
                let started = journal.contains(&(fidx, MADE)) || (0..n).any(|c| journal.contains(&(fidx, c)));
                if e.s == 0 {
                    if md.as_ref().map(|m| m.len() != 0).unwrap_or(true) { std::fs::File::create(&real).map_err(|er| format!("{}: {er}", e.p))?; }
                    set_times_and_mode(&real, e);
                    continue;
                }
                if let (Some(m), true) = (&md, started) {
                    // Resuming: what the journal doesn't have.
                    if m.len() != e.s { let f = std::fs::OpenOptions::new().write(true).open(&real).map_err(|er| er.to_string())?; f.set_len(e.s).map_err(|er| er.to_string())?; }
                    let need: Vec<u32> = (0..n).filter(|c| !journal.contains(&(fidx, *c))).collect();
                    let have: u64 = (0..n).filter(|c| journal.contains(&(fidx, *c))).map(|c| chunk_len(e, c)).sum();
                    job.skipped.fetch_add(have, Ordering::Relaxed);
                    if need.is_empty() { set_times_and_mode(&real, e); } else { plan.needed_bytes += need.iter().map(|c| chunk_len(e, *c)).sum::<u64>(); plan.need.push((fidx, need)); }
                    continue;
                }
                if let Some(m) = &md {
                    if m.len() == e.s && same_time(m, e) { job.skipped.fetch_add(e.s, Ordering::Relaxed); continue; }   // already here
                    if m.permissions().mode() & 0o200 == 0 { let _ = std::fs::set_permissions(&real, std::fs::Permissions::from_mode(m.permissions().mode() | 0o200)); }
                    if e.s >= DELTA_MIN && m.len() > 0 { plan.delta.push(fidx); continue; }
                }
                // New (or small and different): made here, allocated at its full size, every chunk sent.
                let f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&real).map_err(|er| format!("{}: {er}", real.display()))?;
                fallocate(&f, e.s);
                if f.metadata().map(|m| m.len()).unwrap_or(0) != e.s { f.set_len(e.s).map_err(|er| er.to_string())?; }
                made.extend_from_slice(&fidx.to_le_bytes()); made.extend_from_slice(&MADE.to_le_bytes());
                plan.needed_bytes += e.s;
                plan.need.push((fidx, (0..n).collect()));
            }
            _ => {}
        }
    }
    if !made.is_empty() { jfile.write_all(&made).map_err(|e| e.to_string())?; let _ = jfile.sync_data(); }
    Ok(plan)
}

/// Requests of about UNIT_BYTES: big files in runs of chunks, small ones many to a request.
fn units(files: &[Entry], need: &[(u32, Vec<u32>)]) -> VecDeque<Vec<(u32, u32, u32)>> {
    let mut out = VecDeque::new();
    let mut cur: Vec<(u32, u32, u32)> = Vec::new();
    let mut bytes = 0u64;
    let per_unit = (UNIT_BYTES / CHUNK) as u32;
    for (f, chunks) in need {
        let e = &files[*f as usize];
        // Runs of consecutive chunks.
        let mut i = 0;
        while i < chunks.len() {
            let start = chunks[i];
            let mut end = start + 1;
            while i + 1 < chunks.len() && chunks[i + 1] == end && end - start < per_unit { i += 1; end += 1; }
            i += 1;
            let b: u64 = (start..end).map(|c| chunk_len(e, c)).sum();
            if !cur.is_empty() && (bytes + b > UNIT_BYTES || cur.len() >= UNIT_ITEMS) { out.push_back(std::mem::take(&mut cur)); bytes = 0; }
            cur.push((*f, start, end));
            bytes += b;
        }
    }
    if !cur.is_empty() { out.push_back(cur); }
    out
}

async fn run_unit(conn: &iroh::endpoint::Connection, job: &Job, ctx: &Ctx, unit: &[(u32, u32, u32)], compress: bool, tx: &tokio::sync::mpsc::Sender<WJob>) -> Result<(), String> {
    let (mut s, mut r) = conn.open_bi().await.map_err(|e| e.to_string())?;
    let items: Vec<Value> = unit.iter().map(|(f, c0, c1)| { let e = &ctx.files[*f as usize]; json!({"r": e.r, "p": e.p, "c0": c0, "c1": c1}) }).collect();
    wire::write_msg(&mut s, &json!({"op": "chunks", "game": job.game_id, "prefix": job.with_prefix, "items": items, "z": compress})).await?;
    let _ = s.finish();
    let mut hb = [0u8; HEADER];
    loop {
        if job.stop.load(Ordering::Relaxed) { let _ = r.stop(iroh::endpoint::VarInt::from_u32(0)); return Err("stopped".into()); }
        r.read_exact(&mut hb).await.map_err(|e| format!("The connection broke: {e}"))?;
        let h = Header::decode(&hb)?;
        if h.kind == T_DONE { return Ok(()); }
        let mut payload = vec![0u8; h.wire as usize];
        r.read_exact(&mut payload).await.map_err(|e| format!("The connection broke: {e}"))?;
        if h.kind == T_ERROR { return Err(String::from_utf8_lossy(&payload).to_string()); }
        if h.kind != T_CHUNK { return Err("Unexpected frame".into()); }
        job.wire.fetch_add(h.wire as u64 + HEADER as u64, Ordering::Relaxed);
        let (f, c0, c1) = *unit.get(h.item as usize).ok_or("A chunk nobody asked for")?;
        if h.chunk < c0 || h.chunk >= c1 { return Err("A chunk nobody asked for".into()); }
        tx.send(WJob { fidx: f, chunk: h.chunk, h, payload }).await.map_err(|_| "stopped".to_string())?;
    }
}

/// Runs (or resumes) a copy over `conn`. `home`: for the stores (finalize).
pub async fn run(job: Arc<Job>, conn: iroh::endpoint::Connection, home: PathBuf, streams: usize, compress: bool) -> Result<(), String> {
    if job.running.swap(true, Ordering::SeqCst) { return Ok(()); }
    if job.stop.load(Ordering::SeqCst) { job.running.store(false, Ordering::SeqCst); return Err("stopped".into()); }
    *crate::lock(&job.error) = None;
    let r = run_inner(&job, &conn, &home, streams, compress).await;
    job.running.store(false, Ordering::SeqCst);
    match &r {
        Ok(()) => {}
        Err(_) if job.cancel.load(Ordering::SeqCst) => { cleanup(&job); job.set_state("cancelled"); }
        // "stopped": paused by the person (or the service stopping); anything else went wrong.
        Err(e) if e == "stopped" => job.set_state("paused"),
        Err(e) => { *crate::lock(&job.error) = Some(e.clone()); job.set_state("error"); }
    }
    r
}

impl Job {
    /// Cancels a copy that isn't running (paused, waiting, failed): what it made is removed.
    pub fn cancel_now(job: &Job) { cleanup(job); job.set_state("cancelled"); }
}

/// A cancelled copy: the folders it made are removed (folders that were there before it are left).
fn cleanup(job: &Job) {
    let made: Value = std::fs::read(job.dir.join("made.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    for p in made.as_array().into_iter().flatten().filter_map(Value::as_str) {
        let p = PathBuf::from(p);
        if p.starts_with(&job.place) || p.starts_with(hearth_core::paths::data_dir().join("prefixes")) { let _ = std::fs::remove_dir_all(&p); }
    }
    let _ = std::fs::remove_dir_all(&job.dir);
}

fn relayed(conn: &iroh::endpoint::Connection) -> bool {
    conn.paths().iter().find(|p| p.is_selected()).map(|p| p.is_relay()).unwrap_or(false)
}

async fn run_inner(job: &Arc<Job>, conn: &iroh::endpoint::Connection, home: &Path, streams: usize, compress: bool) -> Result<(), String> {
    job.set_state("preparing");
    job.relay.store(relayed(conn), Ordering::Relaxed);
    let mf = job.dir.join("manifest.json");
    let manifest: Value = match std::fs::read(&mf).ok().and_then(|b| serde_json::from_slice(&b).ok()) {
        Some(m) => m,
        None => {
            let m = wire::request(conn, &json!({"op": "manifest", "game": job.game_id, "prefix": job.with_prefix})).await?;
            std::fs::create_dir_all(&job.dir).map_err(|e| e.to_string())?;
            crate::write_atomic(&mf, serde_json::to_vec(&m).unwrap().as_slice())?;
            m
        }
    };
    let files: Vec<Entry> = serde_json::from_value(manifest["files"].clone()).map_err(|e| format!("The game's file list isn't readable: {e}"))?;
    let roots = manifest["roots"].as_array().cloned().unwrap_or_default();
    let game = manifest["game"].clone();
    let dests = crate::games::destinations(&game, &roots, &job.place)?;
    // Each file's place, checked: plain relative paths only.
    let mut paths = Vec::with_capacity(files.len());
    for e in &files {
        let root = dests.get(e.r as usize).cloned().flatten();
        let rel = safe_rel(&e.p).ok_or_else(|| format!("{} isn't a path that can be used here", e.p))?;
        paths.push(root.map(|r| r.join(rel)));
    }
    // Room for it: what's new or bigger, per drive.
    if !job.dir.join("made.json").exists() {
        let mut need_by_dev: HashMap<u64, (u64, PathBuf)> = HashMap::new();
        for (i, e) in files.iter().enumerate() {
            let (Some(p), true) = (&paths[i], e.k == "f") else { continue };
            let have = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let root = dests[e.r as usize].clone().unwrap_or_default();
            let mut probe = root.clone();
            while !probe.exists() { if !probe.pop() { break; } }
            use std::os::unix::fs::MetadataExt;
            let dev = std::fs::metadata(&probe).map(|m| m.dev()).unwrap_or(0);
            let slot = need_by_dev.entry(dev).or_insert((0, probe));
            slot.0 += e.s.saturating_sub(have);
        }
        for (_, (need, probe)) in need_by_dev {
            let free = crate::free_space(&probe).unwrap_or(u64::MAX);
            if need > free { return Err(format!("Not enough space: it needs {} more and {} is free", crate::human_bytes(need), crate::human_bytes(free))); }
        }
        let made: Vec<String> = dests.iter().flatten().filter(|d| !d.exists()).map(|d| d.to_string_lossy().to_string()).collect();
        crate::write_atomic(&job.dir.join("made.json"), json!(made).to_string().as_bytes())?;
    }
    for d in dests.iter().flatten() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }

    let jpath = job.dir.join("journal");
    let done_set = load_journal(&jpath);
    let mut jfile = std::fs::OpenOptions::new().create(true).append(true).open(&jpath).map_err(|e| e.to_string())?;
    job.skipped.store(0, Ordering::Relaxed);
    job.done.store(0, Ordering::Relaxed);
    let (plan, jfile) = {
        let (f2, p2, d2, j2) = (files.clone(), paths.clone(), dests.clone(), job.clone());
        tokio::task::spawn_blocking(move || { let p = prepare(&f2, &p2, &d2, &done_set, &mut jfile, &j2); (p, jfile) }).await.map_err(|e| e.to_string())?
    };
    let mut plan = plan?;

    // Files that are here but different: compare chunk hashes with the other side's, a few files at a time.
    if !plan.delta.is_empty() {
        job.set_state("comparing");
        let mut tasks = tokio::task::JoinSet::new();
        let mut queue: VecDeque<u32> = plan.delta.drain(..).collect();
        let mut extra_journal: Vec<u8> = Vec::new();
        loop {
            while tasks.len() < streams.clamp(1, 8) {
                let Some(f) = queue.pop_front() else { break };
                let e = files[f as usize].clone();
                let p = paths[f as usize].clone().unwrap_or_default();
                let conn = conn.clone();
                let gid = job.game_id.clone();
                let prefix = job.with_prefix;
                tasks.spawn(async move {
                    let theirs = wire::request(&conn, &json!({"op": "hashes", "game": gid, "prefix": prefix, "items": [{"r": e.r, "p": e.p}]})).await?;
                    let theirs: Vec<String> = theirs["hashes"][0].as_array().map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();
                    let size = e.s;
                    let p2 = p.clone();
                    let mine = tokio::task::spawn_blocking(move || chunk_hashes(&p2, Some(size))).await.map_err(|er| er.to_string())??;
                    Ok::<_, String>((f, theirs, mine, p))
                });
            }
            let Some(res) = tasks.join_next().await else { break };
            let (f, theirs, mine, p) = res.map_err(|e| e.to_string())??;
            let e = &files[f as usize];
            let n = e.chunks() as u32;
            if theirs.len() != n as usize { return Err("The game changed on the other device while it was being copied: try again".into()); }
            let mut need = Vec::new();
            for c in 0..n {
                if mine.get(c as usize).map(|h| !h.is_empty() && *h == theirs[c as usize]).unwrap_or(false) {
                    job.skipped.fetch_add(chunk_len(e, c), Ordering::Relaxed);
                    extra_journal.extend_from_slice(&f.to_le_bytes()); extra_journal.extend_from_slice(&c.to_le_bytes());
                } else { need.push(c); }
            }
            let fh = std::fs::OpenOptions::new().write(true).open(&p).map_err(|er| er.to_string())?;
            fh.set_len(e.s).map_err(|er| er.to_string())?;
            extra_journal.extend_from_slice(&f.to_le_bytes()); extra_journal.extend_from_slice(&MADE.to_le_bytes());
            if need.is_empty() { set_times_and_mode(&p, e); } else { plan.needed_bytes += need.iter().map(|c| chunk_len(e, *c)).sum::<u64>(); plan.need.push((f, need)); }
            if job.stop.load(Ordering::Relaxed) { return Err("stopped".into()); }
        }
        { use std::io::Write; let mut jf = &jfile; jf.write_all(&extra_journal).map_err(|e| e.to_string())?; let _ = jfile.sync_data(); }
    }

    job.total.store(plan.needed_bytes, Ordering::Relaxed);
    let mut remaining = Vec::with_capacity(files.len());
    for _ in 0..files.len() { remaining.push(AtomicU32::new(0)); }
    for (f, need) in &plan.need { remaining[*f as usize].store(need.len() as u32, Ordering::Relaxed); }
    let ctx = Arc::new(Ctx {
        files: files.clone(), paths: paths.clone(), handles: Mutex::new(HashMap::new()), remaining,
        journal: Mutex::new(Journal { file: jfile, pending: Vec::new(), roots: dests.iter().flatten().cloned().collect() }),
        failed: Mutex::new(None),
    });
    let queue = Arc::new(Mutex::new(units(&files, &plan.need)));
    job.set_state("copying");
    let started = Instant::now();

    // Writers (blocking threads), the journal's flusher, and the streams.
    let (tx, rx) = tokio::sync::mpsc::channel::<WJob>(64);
    let rx = Arc::new(Mutex::new(rx));
    let nwriters = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 6);
    let writers: Vec<_> = (0..nwriters).map(|_| { let (c, j, r) = (ctx.clone(), job.clone(), rx.clone()); std::thread::spawn(move || writer(c, j, r)) }).collect();
    let flusher = {
        let (c, j) = (ctx.clone(), job.clone());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                let c2 = c.clone();
                let _ = tokio::task::spawn_blocking(move || Journal::flush(&c2.journal)).await;
                let _ = &j;
            }
        })
    };
    let mut workers = tokio::task::JoinSet::new();
    for _ in 0..streams.max(1) {
        let (conn, job, ctx, queue, tx) = (conn.clone(), job.clone(), ctx.clone(), queue.clone(), tx.clone());
        workers.spawn(async move {
            loop {
                let Some(unit) = crate::lock(&queue).pop_front() else { return Ok::<(), String>(()) };
                if let Err(e) = run_unit(&conn, &job, &ctx, &unit, compress, &tx).await {
                    job.stop.store(true, Ordering::SeqCst);
                    return Err(e);
                }
            }
        });
    }
    drop(tx);
    let mut first_err: Option<String> = None;
    while let Some(r) = workers.join_next().await {
        let r = r.map_err(|e| e.to_string()).and_then(|x| x);
        if let Err(e) = r { if first_err.is_none() && e != "stopped" { first_err = Some(e); } }
        job.relay.store(relayed(conn), Ordering::Relaxed);
    }
    // Every chunk handed over is written before the copy stops.
    let _ = tokio::task::spawn_blocking(move || { for w in writers { let _ = w.join(); } }).await;
    flusher.abort();
    { let c2 = ctx.clone(); let _ = tokio::task::spawn_blocking(move || Journal::flush(&c2.journal)).await; }
    job.data_ms.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
    if let Some(e) = crate::lock(&ctx.failed).clone() { return Err(e); }
    if let Some(e) = first_err { return Err(e); }
    if job.stop.load(Ordering::SeqCst) { return Err("stopped".into()); }
    if ctx.remaining.iter().any(|r| r.load(Ordering::Relaxed) != 0) { return Err("Some of the game didn't arrive: resume to finish it".into()); }

    job.set_state("finishing");
    let (g2, d2, p2, h2) = (game.clone(), dests.clone(), job.place.clone(), home.to_path_buf());
    let note = tokio::task::spawn_blocking(move || crate::games::finalize(&h2, &g2, &d2, &p2)).await.map_err(|e| e.to_string())??;
    *crate::lock(&job.note) = Some(note);
    let _ = std::fs::remove_dir_all(&job.dir);
    *crate::lock(&job.state) = "done".into();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(p: &str, s: u64) -> Entry { Entry { r: 0, p: p.into(), k: "f".into(), s, m: 0, n: 0, x: 0o644, t: None } }

    #[test]
    fn requests_group_small_files_and_split_big_ones() {
        let files = vec![entry("big", 100 * CHUNK), entry("a", 10), entry("b", 10), entry("c", 3 * CHUNK)];
        let need = vec![(0, (0..100).collect()), (1, vec![0]), (2, vec![0]), (3, vec![0, 2])];
        let u = units(&files, &need);
        // 100 chunks of 1 MiB in runs of 32, then the small files together with the rest.
        let all: Vec<(u32, u32, u32)> = u.iter().flatten().cloned().collect();
        assert_eq!(all.iter().filter(|x| x.0 == 0).map(|x| x.2 - x.1).sum::<u32>(), 100);
        assert!(u.iter().all(|x| x.iter().map(|(f, a, b)| (*a..*b).map(|c| chunk_len(&files[*f as usize], c)).sum::<u64>()).sum::<u64>() <= UNIT_BYTES));
        assert!(all.contains(&(3, 0, 1)) && all.contains(&(3, 2, 3)), "gaps split runs: {all:?}");
        let small = u.iter().find(|x| x.iter().any(|(f, _, _)| *f == 1)).unwrap();
        assert!(small.iter().any(|(f, _, _)| *f == 2), "small files share a request");
    }

    #[test]
    fn chunk_hashes_and_sizes() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("f");
        std::fs::write(&p, vec![7u8; (CHUNK + 10) as usize]).unwrap();
        let h = chunk_hashes(&p, None).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0], blake3::hash(&vec![7u8; CHUNK as usize]).to_hex().to_string());
        // Compared as if it were longer: the missing chunk hashes to "".
        let h = chunk_hashes(&p, Some(3 * CHUNK)).unwrap();
        assert_eq!(h.len(), 3);
        assert!(h[1].is_empty() && h[2].is_empty());
    }

    #[test]
    fn produce_reads_what_was_asked_and_stays_in_the_game() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("game");
        std::fs::create_dir_all(&root).unwrap();
        let data: Vec<u8> = (0..(2 * CHUNK + 5)).map(|i| (i % 251) as u8).collect();
        std::fs::write(root.join("x.pak"), &data).unwrap();
        std::fs::write(d.path().join("secret"), "no").unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let collect = |items: Vec<Item>| {
            let (tx, mut rx) = tokio::sync::mpsc::channel(16);
            let roots = vec![root.clone()];
            std::thread::spawn(move || produce(roots, items, true, tx));
            let mut frames = Vec::new();
            rt.block_on(async { while let Some(b) = rx.recv().await { frames.push(b); } });
            frames
        };
        let frames = collect(vec![Item { r: 0, p: "x.pak".into(), c0: 1, c1: 3 }]);
        assert_eq!(frames.len(), 3);
        let h = Header::decode(frames[1][..HEADER].try_into().unwrap()).unwrap();
        assert_eq!((h.chunk, h.raw), (2, 5));
        assert_eq!(wire::open_chunk(&h, frames[1][HEADER..].to_vec()).unwrap(), data[(2 * CHUNK) as usize..]);
        assert_eq!(frames[2][0], T_DONE);
        let frames = collect(vec![Item { r: 0, p: "../secret".into(), c0: 0, c1: 1 }]);
        assert_eq!(frames[0][0], T_ERROR);
    }

    #[test]
    fn prepare_skips_whats_here_resumes_and_allocates() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("dest");
        std::fs::create_dir_all(&root).unwrap();
        let mut same = entry("same.bin", 10); same.m = 1_700_000_000;
        std::fs::write(root.join("same.bin"), vec![1u8; 10]).unwrap();
        set_times_and_mode(&root.join("same.bin"), &same);
        let files = vec![Entry { r: 0, p: "sub".into(), k: "d".into(), s: 0, m: 0, n: 0, x: 0o755, t: None }, same.clone(), entry("sub/new.bin", 3 * CHUNK), entry("empty", 0),
                         Entry { r: 0, p: "sub/link".into(), k: "l".into(), s: 0, m: 0, n: 0, x: 0, t: Some("new.bin".into()) },
                         Entry { r: 0, p: "bad".into(), k: "l".into(), s: 0, m: 0, n: 0, x: 0, t: Some("../../etc".into()) }];
        let dests = vec![Some(root.clone())];
        let paths: Vec<Option<PathBuf>> = files.iter().map(|e| Some(root.join(&e.p))).collect();
        let job = Job::new(d.path().join("job"), "t".into(), "p".into(), "g".into(), "G".into(), "steam".into(), root.clone(), false);
        let jp = d.path().join("journal");
        let mut jf = std::fs::OpenOptions::new().create(true).append(true).open(&jp).unwrap();
        let plan = prepare(&files, &paths, &dests, &HashSet::new(), &mut jf, &job).unwrap();
        assert_eq!(plan.need, vec![(2, vec![0, 1, 2])]);
        assert_eq!(std::fs::metadata(root.join("sub/new.bin")).unwrap().len(), 3 * CHUNK);
        assert_eq!(job.skipped.load(Ordering::Relaxed), 10);
        assert!(root.join("empty").exists());
        assert_eq!(std::fs::read_link(root.join("sub/link")).unwrap(), PathBuf::from("new.bin"));
        assert!(std::fs::symlink_metadata(root.join("bad")).is_err(), "a link out of the game isn't made");
        // Resuming with chunk 1 in the journal: only 0 and 2 are asked for.
        let mut j = load_journal(&jp);
        assert!(j.contains(&(2, MADE)));
        j.insert((2, 1));
        let plan = prepare(&files, &paths, &dests, &j, &mut jf, &job).unwrap();
        assert_eq!(plan.need, vec![(2, vec![0, 2])]);
    }
}
