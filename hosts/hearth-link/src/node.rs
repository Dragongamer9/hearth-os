//! The service: one iroh endpoint (this device's key, ALPNs for pairing and for everything else), the paired devices,
//! the copies coming here, and the answers to other devices and to the UI.
//!
//! Requests from a paired device (one stream each, wire.rs):
//!   hello {name, signedIn}            names and which stores each side is signed in to (never any sign-in itself)
//!   games                             the games this device can send
//!   manifest {game, prefix}           a game's file list (xfer.rs)
//!   hashes {game, items}              chunk hashes of files, to send only what differs
//!   chunks {game, items, z}           the chunks themselves
//!   sync {fields, play, rev, got}     Hearth data (sync.rs); the answer is this device's
//!   offer {game}                      "I'd like to send you this game" (accepted here, then pulled)
//!   unpair                            the other device unpaired: forget it here too
//! A connection from a device that isn't paired is closed at once (except pairing, while a code is showing).

use crate::state::{Config, Peers, Relay};
use crate::sync::{Sources, SyncState};
use crate::wire;
use crate::xfer::Job;
use iroh::endpoint::{presets, Connection, QuicTransportConfig, VarInt};
use iroh::{Endpoint, EndpointAddr, RelayMode, TransportAddr};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Found { pub id: String, pub name: String, pub addr: SocketAddr, pub pairing: bool, pub at: u64 }

#[derive(Debug, Clone)]
pub struct Offer { pub id: String, pub peer: String, pub game: Value, pub at: u64 }

/// What this device is sending right now (per peer and game): bytes and when the last chunk went.
#[derive(Debug, Clone, Default)]
pub struct Sending { pub title: String, pub bytes: u64, pub at: u64, pub streams: u32 }

pub struct Node {
    pub ep: Endpoint,
    pub me: String,
    pub dir: PathBuf,
    pub home: PathBuf,
    pub port: u16,
    pub cfg: Mutex<Config>,
    pub peers: Mutex<Peers>,
    pub found: Mutex<HashMap<String, Found>>,
    /// The code on screen: (code, until, wrong tries).
    pub pairing: Mutex<Option<(String, u64, u32)>>,
    pub jobs: Mutex<Vec<Arc<Job>>>,
    pub offers: Mutex<Vec<Offer>>,
    pub conns: tokio::sync::Mutex<HashMap<String, Connection>>,
    pub remote_games: Mutex<HashMap<String, (u64, Value)>>,
    pub game_cache: Mutex<HashMap<String, (Instant, Arc<crate::games::LocalGame>)>>,
    pub sync: Mutex<SyncState>,
    pub sync_info: Mutex<HashMap<String, Value>>,
    pub src: Sources,
    pub sending: Mutex<HashMap<String, Sending>>,
    pub sent_bytes: AtomicU64,
    pub announce_now: tokio::sync::Notify,
}

fn transport() -> QuicTransportConfig {
    // Big windows so one connection fills a fast link even with some latency (2.5 Gbit/s at 50 ms needs ~16 MB in
    // flight); many streams so a copy's requests run side by side.
    QuicTransportConfig::builder()
        .stream_receive_window(VarInt::from_u32(32 << 20))
        .receive_window(VarInt::from_u32(512 << 20))
        .send_window(512 << 20)
        .max_concurrent_bidi_streams(VarInt::from_u32(256))
        .max_concurrent_uni_streams(VarInt::from_u32(16))
        .build()
}

/// The relay setting as iroh's: $HEARTH_LINK_RELAY (a URL, or "off") overrides it (tests).
fn relay_mode(cfg: &Config) -> (RelayMode, bool) {
    if let Ok(r) = std::env::var("HEARTH_LINK_RELAY") {
        if r == "off" { return (RelayMode::Disabled, false); }
        if let Ok(u) = r.parse::<iroh::RelayUrl>() { return (RelayMode::custom([u]), false); }
    }
    match cfg.relay {
        Relay::Default => (RelayMode::Default, true),
        Relay::Custom => match cfg.relay_url.parse::<iroh::RelayUrl>() { Ok(u) => (RelayMode::custom([u]), true), Err(_) => (RelayMode::Default, true) },
        Relay::Off => (RelayMode::Disabled, false),
    }
}

async fn bind(key: iroh::SecretKey, cfg: &Config, port: u16) -> Result<Endpoint, String> {
    let (mode, lookup) = relay_mode(cfg);
    let b = if lookup && std::env::var_os("HEARTH_LINK_NO_DNS").is_none() { Endpoint::builder(presets::N0) } else { Endpoint::builder(presets::Minimal) };
    let b = b.secret_key(key).alpns(vec![crate::ALPN_PAIR.to_vec(), crate::ALPN_LINK.to_vec()]).transport_config(transport()).relay_mode(mode);
    let b = b.bind_addr(SocketAddr::from(([0, 0, 0, 0], port))).map_err(|e| e.to_string())?;
    b.bind().await.map_err(|e| e.to_string())
}

fn signed_in_here(home: &std::path::Path) -> Value {
    // Only whether: these are file checks, nothing of a sign-in is read or sent.
    let steam = hearth_core::steam::steam_path(home).map(|s| s.join("config/loginusers.vdf").exists()).unwrap_or(false);
    json!({"steam": steam, "epic": hearth_core::stores::signed_in("epic"), "gog": hearth_core::stores::signed_in("gog"), "xbox": hearth_core::stores::signed_in("xbox")})
}

fn store_name(s: &str) -> &'static str { match s { "steam" => "Steam", "epic" => "Epic Games", "gog" => "GOG", "xbox" => "Xbox", _ => "a store" } }

impl Node {
    pub async fn start() -> Result<Arc<Node>, String> {
        let dir = crate::home_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let key = crate::state::load_or_make_key(&dir)?;
        let cfg = Config::load(&dir);
        let want = std::env::var("HEARTH_LINK_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(crate::QUIC_PORT);
        let ep = match bind(key.clone(), &cfg, want).await { Ok(e) => e, Err(_) => bind(key.clone(), &cfg, 0).await? };
        let port = ep.bound_sockets().iter().find(|a| a.is_ipv4()).map(|a| a.port()).unwrap_or(want);
        let me = ep.id().to_string();
        let mut jobs = Vec::new();
        for e in std::fs::read_dir(dir.join("transfers")).into_iter().flatten().flatten() {
            if let Some(j) = Job::load(&e.path()) { jobs.push(Arc::new(j)); }
        }
        let node = Arc::new(Node {
            ep, me, port, home: hearth_core::paths::home(), cfg: Mutex::new(cfg), peers: Mutex::new(Peers::load(&dir)),
            found: Mutex::new(HashMap::new()), pairing: Mutex::new(None), jobs: Mutex::new(jobs), offers: Mutex::new(Vec::new()),
            conns: tokio::sync::Mutex::new(HashMap::new()), remote_games: Mutex::new(HashMap::new()), game_cache: Mutex::new(HashMap::new()),
            sync: Mutex::new(SyncState::load(&dir)), sync_info: Mutex::new(HashMap::new()), src: Sources::here(),
            sending: Mutex::new(HashMap::new()), sent_bytes: AtomicU64::new(0), announce_now: tokio::sync::Notify::new(), dir,
        });
        eprintln!("hearth-link: {} ({}) on UDP {}", node.name(), node.me, node.port);
        Ok(node)
    }

    pub fn name(&self) -> String { crate::lock(&self.cfg).display_name() }
    fn key_bytes(&self) -> [u8; 32] { *self.ep.id().as_bytes() }

    /// Runs everything in the background: incoming connections, the network announcements, sync, resuming copies.
    pub fn spawn_all(self: &Arc<Self>) {
        let n = self.clone(); tokio::spawn(async move { n.accept_loop().await });
        let n = self.clone(); tokio::spawn(async move { n.discovery_loop().await });
        let n = self.clone(); tokio::spawn(async move { n.sync_loop().await });
        // The firewall (Anatase has firewalld): our two UDP ports, to the local network only (hearth-link-firewall;
        // Hearth's polkit rule lets it run without a prompt). Over the internet nothing needs opening: hole
        // punching starts from both sides.
        const FW: &str = "/usr/libexec/nimbus/hearth-link-firewall";
        if std::path::Path::new(FW).exists() && std::env::var_os("HEARTH_LINK_NO_FIREWALL").is_none() {
            std::thread::spawn(|| {
                let r = std::process::Command::new("pkexec").args([FW, "open", "hearth"]).stdin(std::process::Stdio::null()).output();
                if let Ok(o) = r { eprintln!("hearth-link: firewall: {}", String::from_utf8_lossy(&o.stdout).trim()); }
            });
        }
        // Copies that were running when the service stopped carry on.
        let n = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let list: Vec<Arc<Job>> = crate::lock(&n.jobs).iter().filter(|j| j.state() == "interrupted").cloned().collect();
            for j in list { n.clone().spawn_job(j); }
        });
    }

    // ------------------------------------------------------------------ connections

    fn addr_of(&self, id: &str, relay: Option<&str>) -> Result<EndpointAddr, String> {
        let pk: iroh::PublicKey = id.parse().map_err(|_| "That isn't a Hearth device's id".to_string())?;
        let mut addrs: Vec<TransportAddr> = Vec::new();
        if let Some(f) = crate::lock(&self.found).get(id) { addrs.push(TransportAddr::Ip(f.addr)); }
        if let Some(p) = crate::lock(&self.peers).get(id) {
            for a in &p.addrs { if let Ok(sa) = a.parse::<SocketAddr>() { let t = TransportAddr::Ip(sa); if !addrs.contains(&t) { addrs.push(t); } } }
            if let Some(r) = p.signed_in.get("relay").and_then(Value::as_str).and_then(|r| r.parse::<iroh::RelayUrl>().ok()) { addrs.push(TransportAddr::Relay(r)); }
        }
        if let Some(r) = relay.and_then(|r| r.parse::<iroh::RelayUrl>().ok()) { addrs.push(TransportAddr::Relay(r)); }
        Ok(EndpointAddr::from_parts(pk, addrs))
    }

    /// A connection to a paired device (reused while it's open).
    pub async fn connect(self: &Arc<Self>, id: &str) -> Result<Connection, String> {
        let mut conns = self.conns.lock().await;
        if let Some(c) = conns.get(id) { if c.close_reason().is_none() { return Ok(c.clone()); } }
        let addr = self.addr_of(id, None)?;
        let name = crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| "the other device".into());
        let c = match tokio::time::timeout(Duration::from_secs(20), self.ep.connect(addr, crate::ALPN_LINK)).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(format!("Can't reach {name}: {e}")),
            Err(_) => return Err(format!("Can't reach {name} (it's off, asleep or offline)")),
        };
        conns.insert(id.to_string(), c.clone());
        drop(conns);
        // Who it is, and what it's signed in to.
        let n = self.clone(); let c2 = c.clone(); let id2 = id.to_string();
        tokio::spawn(async move { let _ = n.hello(&id2, &c2).await; });
        Ok(c)
    }

    async fn hello(self: &Arc<Self>, id: &str, c: &Connection) -> Result<(), String> {
        let relay = self.ep.addr().relay_urls().next().map(|u| u.to_string());
        let r = wire::request(c, &json!({"op": "hello", "name": self.name(), "signedIn": signed_in_here(&self.home), "relay": relay})).await;
        match r {
            Ok(v) => { self.take_hello(id, &v); Ok(()) }
            Err(e) => { self.check_unpaired(id, &e); Err(e) }
        }
    }

    fn take_hello(&self, id: &str, v: &Value) {
        let mut ps = crate::lock(&self.peers);
        if let Some(p) = ps.get_mut(id) {
            if let Some(n) = v["name"].as_str() { p.name = n.chars().filter(|c| !c.is_control()).take(40).collect(); }
            let mut si = v["signedIn"].clone();
            if !si.is_object() { si = json!({}); }
            if let Some(r) = v["relay"].as_str().filter(|r| crate::state::valid_relay_url(r)) { si["relay"] = json!(r); }
            p.signed_in = si;
            p.last_seen = crate::now_ms();
        }
        let _ = ps.save();
    }

    fn add_peer(&self, id: &str, name: &str, relay: Option<String>) {
        let mut ps = crate::lock(&self.peers);
        ps.add(id, name);
        if let (Some(r), Some(p)) = (relay, ps.get_mut(id)) { if !p.signed_in.is_object() { p.signed_in = json!({}); } p.signed_in["relay"] = json!(r); }
        let _ = ps.save();
    }

    /// The other device said it isn't paired with this one anymore (it unpaired while this one was away).
    fn check_unpaired(&self, id: &str, err: &str) {
        if err.contains("not paired") {
            eprintln!("hearth-link: {id} unpaired this device");
            crate::lock(&self.peers).remove(id);
        }
    }

    async fn accept_loop(self: Arc<Self>) {
        while let Some(inc) = self.ep.accept().await {
            let n = self.clone();
            tokio::spawn(async move {
                let Ok(conn) = inc.await else { return };
                n.on_conn(conn).await;
            });
        }
    }

    async fn on_conn(self: Arc<Self>, conn: Connection) {
        let remote = conn.remote_id().to_string();
        let alpn = conn.alpn().to_vec();
        if alpn == crate::ALPN_PAIR { return self.host_pair(conn).await; }
        if alpn != crate::ALPN_LINK || !crate::lock(&self.peers).is_paired(&remote) {
            conn.close(VarInt::from_u32(403), b"not paired");
            return;
        }
        let direct = conn.paths().iter().find(|p| p.is_selected()).and_then(|p| match p.remote_addr() { TransportAddr::Ip(a) => Some(*a), _ => None });
        crate::lock(&self.peers).seen(&remote, direct);
        loop {
            let Ok((s, r)) = conn.accept_bi().await else { break };
            let n = self.clone(); let id = remote.clone();
            tokio::spawn(async move { n.on_stream(id, s, r).await });
        }
    }

    async fn host_pair(self: Arc<Self>, conn: Connection) {
        let code = {
            let p = crate::lock(&self.pairing);
            match &*p { Some((c, until, tries)) if *until > crate::now_ms() && *tries < crate::pair::MAX_TRIES => c.clone(), _ => { conn.close(VarInt::from_u32(403), b"pairing isn't open"); return; } }
        };
        let remote = conn.remote_id().to_string();
        let relay = self.ep.addr().relay_urls().next().map(|u| u.to_string());
        match tokio::time::timeout(Duration::from_secs(30), crate::pair::host(&conn, &self.key_bytes(), &self.name(), relay, &code)).await {
            Ok(Ok((name, their_relay))) => {
                self.add_peer(&remote, &name, their_relay);
                *crate::lock(&self.pairing) = None;
                self.announce_now.notify_one();
                eprintln!("hearth-link: paired with {name} ({remote})");
                let n = self.clone();
                tokio::spawn(async move { tokio::time::sleep(Duration::from_secs(1)).await; let _ = n.sync_with(&remote).await; });
            }
            Ok(Err(true)) => {
                let mut p = crate::lock(&self.pairing);
                if let Some(x) = p.as_mut() { x.2 += 1; if x.2 >= crate::pair::MAX_TRIES { *p = None; } }
            }
            _ => {}
        }
    }

    async fn on_stream(self: Arc<Self>, peer: String, mut s: iroh::endpoint::SendStream, mut r: iroh::endpoint::RecvStream) {
        let req = match wire::read_msg(&mut r).await { Ok(v) => v, Err(_) => return };
        let op = req["op"].as_str().unwrap_or("").to_string();
        if op == "chunks" {
            if let Err(e) = self.serve_chunks(&peer, &req, &mut s).await {
                let _ = s.write_all(&wire::control_frame(wire::T_ERROR, &e)).await;
            }
            let _ = s.finish();
            let _ = tokio::time::timeout(Duration::from_secs(30), s.stopped()).await;
            return;
        }
        let reply = match self.clone().answer(&peer, &op, &req).await { Ok(v) => v, Err(e) => json!({"error": e}) };
        let _ = wire::write_msg(&mut s, &reply).await;
        let _ = s.finish();
        let _ = tokio::time::timeout(Duration::from_secs(30), s.stopped()).await;
    }

    /// A game of this device by id, kept for a minute (a copy asks for its files many times).
    async fn local_game(self: &Arc<Self>, id: &str) -> Result<Arc<crate::games::LocalGame>, String> {
        if let Some((t, g)) = crate::lock(&self.game_cache).get(id) { if t.elapsed() < Duration::from_secs(60) { return Ok(g.clone()); } }
        let (home, id2) = (self.home.clone(), id.to_string());
        let g = tokio::task::spawn_blocking(move || crate::games::find(&home, &id2)).await.map_err(|e| e.to_string())?
            .ok_or("That game isn't on this device anymore")?;
        let g = Arc::new(g);
        crate::lock(&self.game_cache).insert(id.to_string(), (Instant::now(), g.clone()));
        Ok(g)
    }

    async fn serve_chunks(self: &Arc<Self>, peer: &str, req: &Value, s: &mut iroh::endpoint::SendStream) -> Result<(), String> {
        let gid = req["game"].as_str().unwrap_or("");
        let game = self.local_game(gid).await?;
        let roots = crate::xfer::root_paths(&game, req["prefix"] == true);
        let items = crate::xfer::parse_items(req)?;
        let z = req["z"] == true;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(6);
        tokio::task::spawn_blocking(move || crate::xfer::produce(roots, items, z, tx));
        let key = format!("{peer}|{gid}");
        { let mut m = crate::lock(&self.sending); let e = m.entry(key.clone()).or_default(); e.title = game.title.clone(); e.streams += 1; }
        let mut result = Ok(());
        while let Some(b) = rx.recv().await {
            let n = b.len() as u64;
            if let Err(e) = s.write_chunk(b).await { result = Err(e.to_string()); break; }
            self.sent_bytes.fetch_add(n, Ordering::Relaxed);
            let mut m = crate::lock(&self.sending);
            if let Some(e) = m.get_mut(&key) { e.bytes += n; e.at = crate::now_ms(); }
        }
        { let mut m = crate::lock(&self.sending); if let Some(e) = m.get_mut(&key) { e.streams = e.streams.saturating_sub(1); } }
        result
    }

    async fn answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "hello" => {
                self.take_hello(peer, req);
                let relay = self.ep.addr().relay_urls().next().map(|u| u.to_string());
                Ok(json!({"name": self.name(), "signedIn": signed_in_here(&self.home), "relay": relay}))
            }
            "games" => {
                let home = self.home.clone();
                let list = tokio::task::spawn_blocking(move || {
                    let mut l = crate::games::list(&home);
                    l.extend(crate::games::list_beacon(&home));
                    l.iter().map(|g| g.to_json()).collect::<Vec<_>>()
                }).await.map_err(|e| e.to_string())?;
                Ok(json!({"games": list}))
            }
            "manifest" => {
                let game = self.local_game(req["game"].as_str().unwrap_or("")).await?;
                let prefix = req["prefix"] == true;
                tokio::task::spawn_blocking(move || crate::xfer::manifest_for(&game, prefix)).await.map_err(|e| e.to_string())?
            }
            "hashes" => {
                let game = self.local_game(req["game"].as_str().unwrap_or("")).await?;
                let roots = crate::xfer::root_paths(&game, req["prefix"] == true);
                let req = req.clone();
                tokio::task::spawn_blocking(move || crate::xfer::hash_files(&roots, &req)).await.map_err(|e| e.to_string())?
            }
            "sync" => {
                if crate::game_running() { return Err("playing".into()); }
                if !crate::lock(&self.cfg).sync { return Err("Sync is off on the other device".into()); }
                let n = self.clone(); let peer2 = peer.to_string(); let req = req.clone();
                tokio::task::spawn_blocking(move || n.sync_answer(&peer2, &req)).await.map_err(|e| e.to_string())?
            }
            "offer" => {
                let game = req["game"].clone();
                if game["id"].as_str().is_none() { return Err("No game".into()); }
                let always = crate::lock(&self.cfg).always_accept || crate::lock(&self.peers).get(peer).map(|p| p.always_accept).unwrap_or(false);
                let id = crate::hex(&crate::random_bytes::<8>());
                if always {
                    let store = game["store"].as_str().unwrap_or("");
                    let place = crate::games::check_place(&self.home, store, None)?;
                    self.start_job(peer, &game, place, game["hasPrefix"] == true)?;
                    return Ok(json!({"accepted": true}));
                }
                let mut o = crate::lock(&self.offers);
                o.retain(|x| !(x.peer == peer && x.game["id"] == game["id"]));
                o.push(Offer { id, peer: peer.to_string(), game, at: crate::now_ms() });
                Ok(json!({"accepted": false, "waiting": true}))
            }
            "unpair" => { crate::lock(&self.peers).remove(peer); self.conns.lock().await.remove(peer); Ok(json!({"ok": true})) }
            "ping" => Ok(json!({"ok": true})),
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    // ------------------------------------------------------------------ sync

    fn sync_answer(&self, peer: &str, req: &Value) -> Result<Value, String> {
        let mut s = crate::lock(&self.sync);
        s.scan(&self.me, &self.src);
        let (c, g) = s.merge(&self.me, req);
        s.apply(&self.me, &self.src, &c, &g)?;
        if let Some(r) = req["rev"].as_u64() { s.got.insert(peer.to_string(), r); }
        if let Some(a) = req["got"].as_u64() { s.ack.insert(peer.to_string(), a); }
        let mut out = s.outgoing(peer);
        // Not back what it just sent.
        if let (Some(mine), Some(theirs)) = (out["fields"].as_object_mut(), req["fields"].as_object()) {
            mine.retain(|k, f| theirs.get(k).map(|t| t["t"] != f["t"] || t["d"] != f["d"]).unwrap_or(true));
        }
        s.save(&self.dir)?;
        crate::lock(&self.sync_info).insert(peer.to_string(), json!({"at": crate::now_ms(), "got": c.len(), "playGrew": g.len()}));
        Ok(out)
    }

    pub async fn sync_with(self: &Arc<Self>, peer: &str) -> Result<Value, String> {
        if !crate::lock(&self.cfg).sync { return Err("Sync is off".into()); }
        let conn = self.connect(peer).await?;
        let n = self.clone(); let p = peer.to_string();
        let mut req = tokio::task::spawn_blocking(move || {
            let mut s = crate::lock(&n.sync);
            s.scan(&n.me, &n.src);
            let out = s.outgoing(&p);
            let _ = s.save(&n.dir);
            out
        }).await.map_err(|e| e.to_string())?;
        req["op"] = json!("sync");
        let resp = match wire::request(&conn, &req).await { Ok(v) => v, Err(e) => { self.check_unpaired(peer, &e); return Err(e); } };
        let n = self.clone(); let p = peer.to_string();
        tokio::task::spawn_blocking(move || {
            let mut s = crate::lock(&n.sync);
            let (c, g) = s.merge(&n.me, &resp);
            s.apply(&n.me, &n.src, &c, &g)?;
            if let Some(r) = resp["rev"].as_u64() { s.got.insert(p.clone(), r); }
            if let Some(a) = resp["got"].as_u64() { s.ack.insert(p.clone(), a); }
            s.save(&n.dir)?;
            let info = json!({"at": crate::now_ms(), "got": c.len(), "playGrew": g.len()});
            crate::lock(&n.sync_info).insert(p, info.clone());
            Ok::<Value, String>(info)
        }).await.map_err(|e| e.to_string())?
    }

    async fn sync_loop(self: Arc<Self>) {
        let mut last_full: Option<Instant> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if !crate::lock(&self.cfg).sync || crate::game_running() { continue; }
            let changed = { let n = self.clone(); tokio::task::spawn_blocking(move || { let mut s = crate::lock(&n.sync); let c = s.scan(&n.me, &n.src); let _ = s.save(&n.dir); c }).await.unwrap_or(false) };
            if !changed && last_full.map(|t| t.elapsed() < Duration::from_secs(300)).unwrap_or(false) { continue; }
            last_full = Some(Instant::now());
            let ids: Vec<String> = crate::lock(&self.peers).list.iter().map(|p| p.id.clone()).collect();
            for id in ids { if let Err(e) = self.sync_with(&id).await { eprintln!("hearth-link: sync with {id}: {e}"); } }
        }
    }

    // ------------------------------------------------------------------ the network announcements

    async fn discovery_loop(self: Arc<Self>) {
        let port = std::env::var("HEARTH_LINK_DISCOVERY_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(crate::DISCOVERY_PORT);
        let sock = match crate::discovery::socket(port) { Ok(s) => s, Err(e) => { eprintln!("hearth-link: no network announcements: {e}"); return; } };
        let mut next = Instant::now();
        loop {
            let pairing = crate::pair::status_json(&crate::lock(&self.pairing)).is_object();
            if Instant::now() >= next {
                let a = crate::discovery::Announce { id: self.me.clone(), name: self.name(), port: self.port, pairing };
                let bytes = crate::discovery::encode(&a);
                for t in crate::discovery::targets() { let _ = sock.send_to(&bytes, SocketAddr::from((t, port))).await; }
                next = Instant::now() + if pairing { Duration::from_secs(2) } else { Duration::from_secs(15) };
            }
            let wait = next.saturating_duration_since(Instant::now());
            tokio::select! {
                r = async { let mut b = vec![0u8; 2048]; sock.recv_from(&mut b).await.map(|(n, f)| { b.truncate(n); (b, f) }) } => {
                    let Ok((data, from)) = r else { continue };
                    if !crate::discovery::is_local(from.ip()) { continue; }
                    let Some(a) = crate::discovery::decode(&data) else { continue };
                    if a.id == self.me { continue; }
                    let addr = SocketAddr::new(from.ip(), a.port);
                    let was = crate::lock(&self.found).insert(a.id.clone(), Found { id: a.id.clone(), name: a.name.clone(), addr, pairing: a.pairing, at: crate::now_ms() });
                    let paired = crate::lock(&self.peers).is_paired(&a.id);
                    if paired {
                        crate::lock(&self.peers).seen(&a.id, Some(addr));
                        // Back on the network after a while: catch up.
                        if was.map(|w| crate::now_ms() - w.at > 120_000).unwrap_or(true) {
                            let n = self.clone(); let id = a.id.clone();
                            tokio::spawn(async move { if !crate::game_running() { let _ = n.sync_with(&id).await; } });
                        }
                    }
                }
                _ = tokio::time::sleep(wait) => {}
                _ = self.announce_now.notified() => { next = Instant::now(); }
            }
        }
    }

    // ------------------------------------------------------------------ copies

    pub fn start_job(self: &Arc<Self>, peer: &str, game: &Value, place: PathBuf, prefix: bool) -> Result<Arc<Job>, String> {
        let gid = game["id"].as_str().ok_or("No game")?.to_string();
        if crate::lock(&self.jobs).iter().any(|j| j.game_id == gid && !matches!(j.state().as_str(), "done" | "cancelled" | "error")) {
            return Err("That game is already being copied".into());
        }
        let id = crate::hex(&crate::random_bytes::<8>());
        let job = Arc::new(Job::new(self.dir.join("transfers").join(&id), id, peer.to_string(), gid, game["title"].as_str().unwrap_or("Game").to_string(),
                                    game["store"].as_str().unwrap_or("").to_string(), place, prefix));
        job.save()?;
        crate::lock(&self.jobs).push(job.clone());
        self.clone().spawn_job(job.clone());
        Ok(job)
    }

    /// Runs a copy; while the other device can't be reached it waits and tries again.
    fn spawn_job(self: Arc<Self>, job: Arc<Job>) {
        tokio::spawn(async move {
            loop {
                if job.stop.load(Ordering::SeqCst) || job.cancel.load(Ordering::SeqCst) { break; }
                match self.connect(&job.peer).await {
                    Ok(conn) => {
                        let (streams, compress) = { let c = crate::lock(&self.cfg); (c.streams, c.compress != "off") };
                        let r = crate::xfer::run(job.clone(), conn, self.home.clone(), streams, compress).await;
                        if let Err(e) = &r { eprintln!("hearth-link: copy of {}: {e}", job.title); }
                        // The connection dropped in the middle: wait for it and carry on.
                        let lost = matches!(&r, Err(e) if e.starts_with("The connection broke"));
                        if !lost { break; }
                        *crate::lock(&job.error) = None;
                        job.set_state("waiting");
                    }
                    Err(e) => { *crate::lock(&job.note) = Some(e); if job.state() != "waiting" { job.set_state("waiting"); } }
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
    }

    fn job(&self, id: &str) -> Result<Arc<Job>, String> { crate::lock(&self.jobs).iter().find(|j| j.id == id).cloned().ok_or_else(|| "No such copy".into()) }

    // ------------------------------------------------------------------ the UI's requests

    pub fn status(&self) -> Value {
        let now = crate::now_ms();
        let found = crate::lock(&self.found).clone();
        let peers: Vec<Value> = crate::lock(&self.peers).list.iter().map(|p| {
            let online = found.get(&p.id).map(|f| now - f.at < 60_000).unwrap_or(false) || now.saturating_sub(p.last_seen) < 60_000;
            json!({"id": p.id, "name": p.name, "pairedAt": p.paired_at, "lastSeen": p.last_seen, "online": online, "alwaysAccept": p.always_accept,
                   "signedIn": p.signed_in, "sync": crate::lock(&self.sync_info).get(&p.id)})
        }).collect();
        let paired: Vec<String> = crate::lock(&self.peers).list.iter().map(|p| p.id.clone()).collect();
        let nearby: Vec<Value> = found.values().filter(|f| !paired.contains(&f.id) && now - f.at < 60_000)
            .map(|f| json!({"id": f.id, "name": f.name, "pairing": f.pairing, "addr": f.addr.to_string()})).collect();
        let here = signed_in_here(&self.home);
        let mut hints = Vec::new();
        for st in ["steam", "epic", "gog", "xbox"] {
            if here[st] == true { continue; }
            if let Some(p) = crate::lock(&self.peers).list.iter().find(|p| p.signed_in[st] == true) {
                hints.push(json!({"store": st, "name": store_name(st), "on": p.name, "text": format!("You're signed in to {} on {}: sign in once on this device too", store_name(st), p.name)}));
            }
        }
        let sending: Vec<Value> = crate::lock(&self.sending).iter().filter(|(_, s)| now - s.at < 10_000).map(|(k, s)| {
            let peer = k.split('|').next().unwrap_or("");
            let name = crate::lock(&self.peers).get(peer).map(|p| p.name.clone()).unwrap_or_default();
            json!({"to": name, "title": s.title, "bytes": s.bytes})
        }).collect();
        let relay = self.ep.addr().relay_urls().next().map(|u| u.to_string());
        let offers: Vec<Value> = crate::lock(&self.offers).iter().map(|o| {
            let name = crate::lock(&self.peers).get(&o.peer).map(|p| p.name.clone()).unwrap_or_default();
            json!({"id": o.id, "peer": o.peer, "from": name, "game": o.game, "at": o.at})
        }).collect();
        let jobs: Vec<Value> = crate::lock(&self.jobs).iter().map(|j| {
            let mut v = j.to_json();
            v["from"] = json!(crate::lock(&self.peers).get(&j.peer).map(|p| p.name.clone()).unwrap_or_default());
            v
        }).collect();
        json!({
            "me": {"id": self.me, "name": self.name(), "port": self.port, "relay": relay, "ticket": self.ticket()},
            "settings": crate::lock(&self.cfg).to_json(),
            "pairing": crate::pair::status_json(&crate::lock(&self.pairing)),
            "peers": peers, "nearby": nearby, "transfers": jobs, "offers": offers, "sending": sending,
            "signInHints": hints, "sync": crate::lock(&self.sync).summary(), "playing": crate::game_running(),
        })
    }

    /// What another device needs to pair with this one from somewhere else: its id, and its relay.
    pub fn ticket(&self) -> String {
        match self.ep.addr().relay_urls().next() { Some(u) => format!("{}@{}", self.me, u), None => self.me.clone() }
    }

    pub async fn command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let s = |k: &str| a[k].as_str().filter(|x| !x.is_empty()).map(str::to_string).ok_or(format!("Missing '{k}'"));
        match cmd {
            "status" => Ok(self.status()),
            "pair.open" => {
                let code = crate::pair::new_code();
                let until = crate::now_ms() + crate::pair::CODE_LIFETIME_MS;
                *crate::lock(&self.pairing) = Some((code.clone(), until, 0));
                self.announce_now.notify_one();
                Ok(json!({"code": code, "until": until}))
            }
            "pair.close" => { *crate::lock(&self.pairing) = None; self.announce_now.notify_one(); Ok(Value::Null) }
            "pair.join" => {
                let code = crate::pair::clean_code(&s("code")?).ok_or("The code is 6 digits")?;
                let target = s("id")?;
                let (id, relay) = match target.split_once('@') { Some((i, r)) => (i.trim().to_string(), Some(r.trim().to_string())), None => (target.trim().to_string(), None) };
                if id == self.me { return Err("That's this device".into()); }
                let addr = self.addr_of(&id, relay.as_deref())?;
                let conn = match tokio::time::timeout(Duration::from_secs(25), self.ep.connect(addr, crate::ALPN_PAIR)).await {
                    Ok(Ok(c)) => c,
                    Ok(Err(e)) => return Err(format!("Couldn't reach that device: {e}")),
                    Err(_) => return Err("Couldn't reach that device".into()),
                };
                let mine = self.ep.addr().relay_urls().next().map(|u| u.to_string());
                let r = tokio::time::timeout(Duration::from_secs(30), crate::pair::join(&conn, &self.key_bytes(), &self.name(), mine, &code)).await
                    .map_err(|_| "The other device didn't answer".to_string())?;
                conn.close(VarInt::from_u32(0), b"done");
                let (name, their_relay) = r.map_err(|e| if e.contains("closed") || e.contains("pairing isn't open") { "Pairing isn't open on that device (or the code expired)".to_string() } else { e })?;
                self.add_peer(&id, &name, their_relay.or(relay.filter(|r| crate::state::valid_relay_url(r))));
                let n = self.clone(); let id2 = id.clone();
                tokio::spawn(async move { let _ = n.sync_with(&id2).await; });
                Ok(json!({"id": id, "name": name}))
            }
            "unpair" => {
                let id = s("id")?;
                let notify = self.connect(&id).await.ok();
                if let Some(c) = notify { let _ = tokio::time::timeout(Duration::from_secs(5), wire::request(&c, &json!({"op": "unpair"}))).await; c.close(VarInt::from_u32(0), b"unpaired"); }
                self.conns.lock().await.remove(&id);
                crate::lock(&self.peers).remove(&id);
                Ok(Value::Null)
            }
            "peer.set" => {
                let id = s("id")?;
                let mut ps = crate::lock(&self.peers);
                let p = ps.get_mut(&id).ok_or("Not a paired device")?;
                if let Some(b) = a["alwaysAccept"].as_bool() { p.always_accept = b; }
                ps.save()?;
                Ok(Value::Null)
            }
            "settings.set" => {
                let before = { let c = crate::lock(&self.cfg); (c.relay.clone(), c.relay_url.clone()) };
                let mut c = crate::lock(&self.cfg).clone();
                c.apply(a)?;
                c.save(&self.dir)?;
                let restart = (c.relay.clone(), c.relay_url.clone()) != before;
                *crate::lock(&self.cfg) = c;
                self.announce_now.notify_one();
                if restart {
                    // A new relay setting needs a new endpoint: the service restarts (systemd starts it again).
                    tokio::spawn(async { tokio::time::sleep(Duration::from_millis(500)).await; std::process::exit(75); });
                }
                Ok(json!({"restarting": restart}))
            }
            "games" => {
                let peer = s("peer")?;
                if !crate::lock(&self.peers).is_paired(&peer) { return Err("Not a paired device".into()); }
                if a["refresh"] != true {
                    if let Some((t, v)) = crate::lock(&self.remote_games).get(&peer) { if crate::now_ms() - t < 30_000 { return Ok(v.clone()); } }
                }
                let conn = self.connect(&peer).await?;
                let v = match tokio::time::timeout(Duration::from_secs(40), wire::request(&conn, &json!({"op": "games"}))).await {
                    Ok(Ok(v)) => v, Ok(Err(e)) => { self.check_unpaired(&peer, &e); return Err(e); } Err(_) => return Err("The other device took too long to answer".into()),
                };
                crate::lock(&self.remote_games).insert(peer, (crate::now_ms(), v.clone()));
                Ok(v)
            }
            "local.games" => {
                let home = self.home.clone();
                let v = tokio::task::spawn_blocking(move || { let mut l = crate::games::list(&home); l.extend(crate::games::list_beacon(&home)); l.iter().map(|g| g.to_json()).collect::<Vec<_>>() })
                    .await.map_err(|e| e.to_string())?;
                Ok(json!({"games": v}))
            }
            "places" => Ok(json!({"places": crate::games::places(&self.home, a["store"].as_str().unwrap_or(""))})),
            "get" => {
                let peer = s("peer")?;
                if !crate::lock(&self.peers).is_paired(&peer) { return Err("Not a paired device".into()); }
                let gid = s("game")?;
                // The game as the other device listed it.
                let listed = crate::lock(&self.remote_games).get(&peer).and_then(|(_, v)| v["games"].as_array().and_then(|l| l.iter().find(|g| g["id"] == gid.as_str()).cloned()));
                let game = match listed { Some(g) => g, None => json!({"id": gid, "title": a["title"].as_str().unwrap_or(&gid), "store": gid.split(':').next().unwrap_or("")}) };
                let store = game["store"].as_str().unwrap_or("").to_string();
                let place = crate::games::check_place(&self.home, &store, a["place"].as_str())?;
                let size = game["size"].as_u64().unwrap_or(0);
                if size > 0 { if let Some(free) = crate::free_space(&place) { if size > free { return Err(format!("Not enough space there: it needs {} and {} is free", crate::human_bytes(size), crate::human_bytes(free))); } } }
                let job = self.start_job(&peer, &game, place, a["prefix"] != false)?;
                Ok(job.to_json())
            }
            "send" => {
                let peer = s("peer")?;
                if !crate::lock(&self.peers).is_paired(&peer) { return Err("Not a paired device".into()); }
                let game = self.local_game(&s("game")?).await?;
                let conn = self.connect(&peer).await?;
                wire::request(&conn, &json!({"op": "offer", "game": game.to_json()})).await
            }
            "offer.accept" => {
                let id = s("id")?;
                let o = { let mut l = crate::lock(&self.offers); let i = l.iter().position(|x| x.id == id).ok_or("That offer isn't there anymore")?; l.remove(i) };
                let store = o.game["store"].as_str().unwrap_or("").to_string();
                let place = crate::games::check_place(&self.home, &store, a["place"].as_str())?;
                let job = self.start_job(&o.peer, &o.game, place, a["prefix"] != false && o.game["hasPrefix"] == true)?;
                Ok(job.to_json())
            }
            "offer.decline" => { let id = s("id")?; crate::lock(&self.offers).retain(|x| x.id != id); Ok(Value::Null) }
            "transfer.pause" => { let j = self.job(&s("id")?)?; j.stop.store(true, Ordering::SeqCst); if !j.running.load(Ordering::SeqCst) { j.set_state("paused"); } Ok(Value::Null) }
            "transfer.resume" => {
                let j = self.job(&s("id")?)?;
                if j.running.load(Ordering::SeqCst) { return Ok(j.to_json()); }
                j.stop.store(false, Ordering::SeqCst); j.cancel.store(false, Ordering::SeqCst);
                j.set_state("waiting");
                self.clone().spawn_job(j.clone());
                Ok(j.to_json())
            }
            "transfer.cancel" => {
                let j = self.job(&s("id")?)?;
                j.cancel.store(true, Ordering::SeqCst); j.stop.store(true, Ordering::SeqCst);
                if !j.running.load(Ordering::SeqCst) {
                    let j2 = j.clone();
                    tokio::task::spawn_blocking(move || { crate::xfer::Job::cancel_now(&j2) }).await.map_err(|e| e.to_string())?;
                }
                Ok(Value::Null)
            }
            "transfer.clear" => { crate::lock(&self.jobs).retain(|j| !matches!(j.state().as_str(), "done" | "cancelled")); Ok(Value::Null) }
            "sync.now" => {
                let ids: Vec<String> = match a["peer"].as_str() { Some(p) => vec![p.to_string()], None => crate::lock(&self.peers).list.iter().map(|p| p.id.clone()).collect() };
                let mut out = serde_json::Map::new();
                for id in ids {
                    let r = self.sync_with(&id).await;
                    out.insert(id, match r { Ok(v) => v, Err(e) => json!({"error": e}) });
                }
                Ok(Value::Object(out))
            }
            "ticket" => Ok(json!({"ticket": self.ticket()})),
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }

    /// The service is stopping: copies pause (their journals flushed) so they resume next time.
    pub async fn shutdown(&self) {
        let jobs: Vec<Arc<Job>> = crate::lock(&self.jobs).iter().filter(|j| j.running.load(Ordering::SeqCst)).cloned().collect();
        for j in &jobs { j.stop.store(true, Ordering::SeqCst); }
        for _ in 0..50 { if !jobs.iter().any(|j| j.running.load(Ordering::SeqCst)) { break; } tokio::time::sleep(Duration::from_millis(100)).await; }
        // Paused by the service stopping, not by the person: they carry on next time.
        for j in &jobs { if j.state() == "paused" && !j.cancel.load(Ordering::SeqCst) { *crate::lock(&j.state) = "copying".into(); let _ = j.save(); } }
        self.ep.close().await;
    }
}
