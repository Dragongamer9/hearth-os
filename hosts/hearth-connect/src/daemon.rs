//! The service's state and what each packet does: devices (reachable now, and paired ones), pairing, and the
//! features ("plugins" in KDE Connect): ping, battery, share (files, text, links), notifications, clipboard,
//! remote input, media control both ways, find my phone. The window reaches all of it through `command`.

use crate::desktop;
use crate::identity::Identity;
use crate::input::{self, Input};
use crate::lan::{self, Io, Peer};
use crate::lock;
use crate::packet::{self, Packet};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::sync::{mpsc, watch, Notify};

const PAIR_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq)]
pub enum Pair { None, Requested { ts: i64, at: Instant }, RequestedByPeer { ts: i64, at: Instant }, Paired }

pub struct Link { tx: mpsc::UnboundedSender<String>, pub ip: IpAddr, pub cert: Vec<u8>, gen: u64 }

pub struct Dev {
    pub peer: Peer,
    pub link: Option<Link>,
    pub pair: Pair,
    pub battery: Option<(i64, bool)>,
    pub notifications: Vec<Value>,
    notif_ids: HashMap<String, u32>,
    pub players: Vec<String>,
    pub playing: HashMap<String, Value>,
    pub keyboard: bool,
}

fn yes() -> bool { true }

/// A paired phone, remembered across restarts (paired.json): its certificate, and the person's choices for it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDev {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub cert: String,
    /// Save its files without asking.
    #[serde(default)]
    pub auto_accept: bool,
    #[serde(default = "yes")]
    pub clipboard: bool,
    #[serde(default = "yes")]
    pub input: bool,
    #[serde(default = "yes")]
    pub notifications: bool,
    #[serde(default)]
    pub last_ip: Option<String>,
}

/// A file a phone sent: fetched into a hidden folder at once (phones only wait a few seconds), and moved into the
/// receive folder when the person accepts it (or straight away for a phone set to "accept without asking").
pub struct IncomingFile {
    pub token: String,
    pub device: String,
    pub from: String,
    pub name: String,
    pub size: i64,
    pub received: u64,
    pub tmp: PathBuf,
    pub done: Option<Result<(), String>>,
    pub decided: Option<bool>,
    pub saved: Option<PathBuf>,
    pub at: Instant,
}

pub struct Daemon {
    pub me: Identity,
    pub name: Mutex<String>,
    pub kind: &'static str,
    pub home: PathBuf,
    pub dest: PathBuf,
    pub devices: Mutex<HashMap<String, Dev>>,
    pub paired: Mutex<HashMap<String, PairedDev>>,
    pub clients: Mutex<Vec<mpsc::UnboundedSender<String>>>,
    pub input: Mutex<Input>,
    pub files: Mutex<Vec<IncomingFile>>,
    pub texts: Mutex<VecDeque<Value>>,
    pub log: Mutex<VecDeque<Value>>,
    pub tcp_port: AtomicU16,
    pub announce: Notify,
    gen: AtomicU64,
    recent: Mutex<HashMap<String, Instant>>,
    clip_last: Mutex<String>,
    pub manual: Mutex<Vec<IpAddr>>,
    pub old_protocol: Mutex<HashMap<String, String>>,
    pub firewall: Mutex<Value>,
    pub stop_tx: watch::Sender<bool>,
    my_battery: Mutex<Option<(i64, bool)>>,
}

fn now_secs() -> i64 { crate::now_ms() / 1000 }

impl Daemon {
    pub fn new(me: Identity, home: PathBuf, dest: PathBuf) -> Arc<Daemon> {
        let paired: HashMap<String, PairedDev> = std::fs::read_to_string(home.join("paired.json")).ok()
            .and_then(|t| serde_json::from_str::<Vec<PairedDev>>(&t).ok()).unwrap_or_default()
            .into_iter().map(|p| (p.id.clone(), p)).collect();
        let settings: Value = std::fs::read_to_string(home.join("settings.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
        let name = settings["name"].as_str().map(crate::identity::filter_name).filter(|n| !n.is_empty()).unwrap_or_else(|| crate::identity::filter_name(&crate::host_name()));
        let manual = settings["manual"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()?.parse().ok()).collect()).unwrap_or_default();
        let (stop_tx, _) = watch::channel(false);
        Arc::new(Daemon {
            me, name: Mutex::new(if name.is_empty() { "Hearth".into() } else { name }), kind: packet::device_type(), home, dest,
            devices: Mutex::new(HashMap::new()), paired: Mutex::new(paired), clients: Mutex::new(vec![]), input: Mutex::new(Input::new()),
            files: Mutex::new(vec![]), texts: Mutex::new(VecDeque::new()), log: Mutex::new(VecDeque::new()), tcp_port: AtomicU16::new(0),
            announce: Notify::new(), gen: AtomicU64::new(0), recent: Mutex::new(HashMap::new()), clip_last: Mutex::new(String::new()),
            manual: Mutex::new(manual), old_protocol: Mutex::new(HashMap::new()),
            firewall: Mutex::new(json!({"ok": true, "firewall": "unmanaged"})), stop_tx, my_battery: Mutex::new(None),
        })
    }

    // ------------------------------------------------------------------ plumbing

    /// Tells every open window something happened (and keeps a short history of what people should see).
    pub fn emit(&self, ev: &str, data: Value) {
        let line = json!({"event": ev, "data": data}).to_string() + "\n";
        if matches!(ev, "ping" | "text" | "url" | "file-saved" | "file-failed" | "paired" | "unpaired" | "pair-failed" | "ring" | "warning" | "sent" | "send-failed") {
            let mut l = lock(&self.log);
            l.push_front(json!({"event": ev, "data": data, "at": crate::now_ms()}));
            l.truncate(60);
        }
        lock(&self.clients).retain(|c| c.send(line.clone()).is_ok());
    }

    fn changed(&self) { self.emit("state", json!({})); }

    pub fn identity_packet(&self, tcp_port: Option<u16>, target: Option<&str>) -> Packet {
        let name = lock(&self.name).clone();
        packet::identity(&packet::Me { device_id: &self.me.device_id, name: &name, device_type: self.kind }, tcp_port, target)
    }

    pub fn set_tcp_port(&self, p: u16) { self.tcp_port.store(p, Ordering::SeqCst); }

    fn save_paired(&self) {
        let list: Vec<PairedDev> = lock(&self.paired).values().cloned().collect();
        let _ = std::fs::create_dir_all(&self.home);
        let _ = crate::identity::write_private(&self.home.join("paired.json"), serde_json::to_string_pretty(&list).unwrap_or_default().as_bytes());
    }

    fn save_settings(&self) {
        let v = json!({"name": *lock(&self.name), "manual": lock(&self.manual).iter().map(|i| i.to_string()).collect::<Vec<_>>()});
        let _ = crate::identity::write_private(&self.home.join("settings.json"), v.to_string().as_bytes());
    }

    pub fn is_paired(&self, id: &str) -> bool { lock(&self.paired).contains_key(id) }

    fn paired_dev(&self, id: &str) -> Option<PairedDev> { lock(&self.paired).get(id).cloned() }

    fn device_name(&self, id: &str) -> String {
        lock(&self.devices).get(id).map(|d| d.peer.name.clone()).or_else(|| self.paired_dev(id).map(|p| p.name)).unwrap_or_else(|| "Your phone".into())
    }

    /// Paired phones we aren't linked to, and addresses added by hand: told about us directly (where broadcasts
    /// don't get through).
    pub fn addresses_to_try(&self) -> Vec<IpAddr> {
        let linked: Vec<String> = lock(&self.devices).iter().filter(|(_, d)| d.link.is_some()).map(|(k, _)| k.clone()).collect();
        let mut out: Vec<IpAddr> = lock(&self.paired).values().filter(|p| !linked.contains(&p.id)).filter_map(|p| p.last_ip.as_deref()?.parse().ok()).collect();
        out.extend(lock(&self.manual).iter().copied());
        out.retain(|ip| crate::is_local_ip(*ip));
        out
    }

    /// Should we connect to a device that announced itself? Not if already linked, nor more than every few seconds.
    pub fn may_connect(&self, id: &str) -> bool {
        if lock(&self.devices).get(id).map(|d| d.link.is_some()).unwrap_or(false) { return false; }
        self.may_accept(id)
    }

    /// A connection from a device: at most one every two seconds per device (a new one replaces an old link).
    pub fn may_accept(&self, id: &str) -> bool {
        let mut r = lock(&self.recent);
        if r.get(id).map(|t| t.elapsed() < Duration::from_secs(2)).unwrap_or(false) { return false; }
        r.insert(id.to_string(), Instant::now());
        if r.len() > 200 { r.retain(|_, t| t.elapsed() < Duration::from_secs(60)); }
        true
    }

    pub fn note_old_protocol(&self, id: &str, name: &str) {
        let new = lock(&self.old_protocol).insert(id.to_string(), crate::identity::filter_name(name)).is_none();
        if new { self.changed(); }
    }

    /// The link to a paired, connected device: its address and certificate (for file transfers).
    pub fn link_of(&self, id: &str) -> Option<(IpAddr, Vec<u8>)> {
        if !self.is_paired(id) { return None; }
        lock(&self.devices).get(id).and_then(|d| d.link.as_ref().map(|l| (l.ip, l.cert.clone())))
    }

    pub fn send(&self, id: &str, p: &Packet) -> Result<(), String> {
        let g = lock(&self.devices);
        let l = g.get(id).and_then(|d| d.link.as_ref()).ok_or("The phone isn't connected right now")?;
        l.tx.send(p.line()).map_err(|_| "The phone isn't connected right now".to_string())
    }

    fn send_body(&self, id: &str, kind: &str, body: Value) -> Result<(), String> { self.send(id, &Packet::new(kind, body)) }

    // ------------------------------------------------------------------ links

    /// A device is linked (TLS up, identities exchanged). Runs until the link drops.
    pub async fn attach(self: &Arc<Self>, peer: Peer, mut rd: BufReader<ReadHalf<Box<dyn Io>>>, mut wr: WriteHalf<Box<dyn Io>>) {
        let paired = self.paired_dev(&peer.id);
        if let Some(p) = &paired {
            if crate::unhex(&p.cert).as_deref() != Some(peer.cert.as_slice()) {
                self.emit("warning", json!({"device": p.name, "text": format!("A device calling itself {} showed a different certificate than when it was paired, so it was refused. If you reinstalled KDE Connect on it, forget it here and pair again.", p.name)}));
                return;
            }
        }
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        let id = peer.id.clone();
        {
            let mut g = lock(&self.devices);
            let link = Link { tx, ip: peer.ip, cert: peer.cert.clone(), gen };
            match g.get_mut(&id) {
                Some(d) => {
                    d.peer = peer.clone();
                    d.link = Some(link);
                    if paired.is_some() { d.pair = Pair::Paired; } else if d.pair == Pair::Paired { d.pair = Pair::None; }
                }
                None => {
                    g.insert(id.clone(), Dev { peer: peer.clone(), link: Some(link), pair: if paired.is_some() { Pair::Paired } else { Pair::None },
                        battery: None, notifications: vec![], notif_ids: HashMap::new(), players: vec![], playing: HashMap::new(), keyboard: false });
                }
            }
        }
        if paired.is_some() {
            if let Some(p) = lock(&self.paired).get_mut(&id) { p.last_ip = Some(peer.ip.to_string()); p.name = peer.name.clone(); p.kind = peer.kind.clone(); }
            self.save_paired();
        }
        lock(&self.old_protocol).remove(&id);
        self.changed();
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                if wr.write_all(line.as_bytes()).await.is_err() { break; }
                let _ = wr.flush().await;
            }
            let _ = wr.shutdown().await;
        });
        if paired.is_some() { self.on_paired_link(&id); }
        let mut line = String::new();
        loop {
            match lan::read_limited(&mut rd, &mut line, lan::MAX_PACKET).await { Ok(0) | Err(_) => break, Ok(_) => {} }
            if let Some(p) = Packet::parse(&line) { self.handle(&id, p).await; }
        }
        {
            let mut g = lock(&self.devices);
            if let Some(d) = g.get_mut(&id) {
                if d.link.as_ref().map(|l| l.gen) == Some(gen) {
                    d.link = None;
                    if d.pair != Pair::Paired { d.pair = Pair::None; }
                    d.keyboard = false;
                }
            }
        }
        self.changed();
    }

    /// A paired phone just linked: what it should know right away, and what we want to know.
    fn on_paired_link(&self, id: &str) {
        if let Some((c, charging)) = desktop::battery() { let _ = self.send_body(id, "kdeconnect.battery", json!({"currentCharge": c, "isCharging": charging, "thresholdEvent": 0})); }
        let _ = self.send_body(id, "kdeconnect.mousepad.keyboardstate", json!({"state": true}));
        if self.paired_dev(id).map(|p| p.notifications).unwrap_or(true) { let _ = self.send_body(id, "kdeconnect.notification.request", json!({"request": true})); }
        let _ = self.send_body(id, "kdeconnect.mpris.request", json!({"requestPlayerList": true}));
    }

    // ------------------------------------------------------------------ packets

    pub async fn handle(self: &Arc<Self>, id: &str, p: Packet) {
        if p.kind == "kdeconnect.pair" { self.on_pair(id, &p); return; }
        // Nothing else from a device that isn't paired.
        let Some(pd) = self.paired_dev(id) else { return };
        let from = pd.name.clone();
        match p.kind.as_str() {
            "kdeconnect.ping" => {
                let msg = p.str("message").unwrap_or("Ping!").to_string();
                self.notify_desktop(&from, &msg, "smartphone", false);
                self.emit("ping", json!({"device": id, "from": from, "message": msg}));
            }
            "kdeconnect.battery" => {
                let charge = p.int("currentCharge").unwrap_or(-1);
                let charging = p.bool("isCharging");
                if let Some(d) = lock(&self.devices).get_mut(id) { d.battery = Some((charge, charging)); }
                if p.int("thresholdEvent") == Some(1) { self.notify_desktop(&format!("{from}: battery low"), &format!("{charge}% left"), "battery-low", true); }
                self.changed();
            }
            "kdeconnect.share.request" => self.share_in(id, &from, &p, &pd),
            "kdeconnect.notification" => if pd.notifications { self.notification_in(id, &from, &p) },
            "kdeconnect.clipboard" | "kdeconnect.clipboard.connect" => if pd.clipboard {
                let content = p.str("content").unwrap_or("").to_string();
                if !content.is_empty() && *lock(&self.clip_last) != content {
                    *lock(&self.clip_last) = content.clone();
                    tokio::task::spawn_blocking(move || desktop::clipboard_set(&content));
                    self.emit("clipboard", json!({"device": id, "from": from}));
                }
            },
            "kdeconnect.mousepad.request" => if pd.input {
                let a = input::action(&p.body);
                let r = lock(&self.input).apply(&a);
                match r {
                    Ok(rest) if !rest.is_empty() => { tokio::task::spawn_blocking(move || desktop::type_text(&rest)); }
                    Ok(_) => {}
                    Err(e) => self.emit("input-error", json!({"error": e})),
                }
                if p.bool("sendAck") {
                    let mut b = p.body.clone();
                    b["isAck"] = json!(true);
                    let _ = self.send_body(id, "kdeconnect.mousepad.echo", b);
                }
            },
            "kdeconnect.mousepad.keyboardstate" => { if let Some(d) = lock(&self.devices).get_mut(id) { d.keyboard = p.bool("state"); } }
            "kdeconnect.mpris.request" => self.mpris_request(id, p.clone()),
            "kdeconnect.mpris" => self.phone_media(id, &p),
            "kdeconnect.findmyphone.request" => {
                desktop::ring();
                self.notify_desktop(&format!("{from} is looking for this PC"), "Here I am!", "find-location", true);
                self.emit("ring", json!({"device": id, "from": from}));
            }
            _ => {}
        }
    }

    fn notify_desktop(&self, summary: &str, body: &str, icon: &str, urgent: bool) {
        let (s, b, i) = (summary.to_string(), body.to_string(), icon.to_string());
        tokio::task::spawn_blocking(move || desktop::notify(&s, &b, &i, 0, urgent));
    }

    // ------------------------------------------------------------------ pairing

    fn on_pair(self: &Arc<Self>, id: &str, p: &Packet) {
        let wants = p.bool("pair");
        let ts = p.int("timestamp");
        let mut event: Option<(&str, Value)> = None;
        let mut store = false;
        let mut forget = false;
        {
            let mut g = lock(&self.devices);
            let Some(d) = g.get_mut(id) else { return };
            let name = d.peer.name.clone();
            if wants {
                let mut state = d.pair.clone();
                if state == Pair::Paired && ts.is_some() { state = Pair::None; forget = true; }
                match state {
                    Pair::Requested { .. } => { d.pair = Pair::Paired; store = true; }
                    Pair::RequestedByPeer { .. } => {}
                    Pair::Paired => {}
                    Pair::None => {
                        let Some(ts) = ts else { return };
                        if (now_secs() - ts).abs() > 1800 {
                            d.pair = Pair::None;
                            event = Some(("pair-failed", json!({"device": id, "from": name, "error": "The phone's clock and this PC's are too far apart. Set both to the right time and try again."})));
                        } else {
                            d.pair = Pair::RequestedByPeer { ts, at: Instant::now() };
                            let key = d.link.as_ref().and_then(|l| crate::identity::verification_key(&self.me.cert_der, &l.cert, ts)).unwrap_or_default();
                            event = Some(("pair-request", json!({"device": id, "from": name, "key": key})));
                        }
                    }
                }
            } else {
                match d.pair {
                    Pair::Requested { .. } | Pair::RequestedByPeer { .. } => { d.pair = Pair::None; event = Some(("pair-failed", json!({"device": id, "from": name, "error": format!("{name} said no, or cancelled")}))); }
                    Pair::Paired => { d.pair = Pair::None; forget = true; event = Some(("unpaired", json!({"device": id, "from": name}))); }
                    Pair::None => {}
                }
            }
        }
        if forget { lock(&self.paired).remove(id); self.save_paired(); }
        if store { self.store_pair(id); }
        if let Some((ev, data)) = event {
            if ev == "pair-request" {
                let who = data["from"].as_str().unwrap_or("A phone").to_string();
                let key = data["key"].as_str().unwrap_or("").to_string();
                self.notify_desktop(&format!("{who} wants to pair"), &format!("Open Hearth Connect to accept. Check the phone shows {key}."), "smartphone", true);
            }
            self.emit(ev, data);
        }
        self.changed();
    }

    fn store_pair(self: &Arc<Self>, id: &str) {
        let info = lock(&self.devices).get(id).and_then(|d| d.link.as_ref().map(|l| (d.peer.clone(), l.cert.clone(), l.ip)));
        let Some((peer, cert, ip)) = info else { return };
        lock(&self.paired).insert(id.to_string(), PairedDev { id: id.into(), name: peer.name.clone(), kind: peer.kind.clone(), cert: crate::hex(&cert),
            auto_accept: false, clipboard: true, input: true, notifications: true, last_ip: Some(ip.to_string()) });
        self.save_paired();
        self.emit("paired", json!({"device": id, "from": peer.name}));
        self.on_paired_link(id);
    }

    pub fn request_pair(self: &Arc<Self>, id: &str) -> Result<Value, String> {
        let state = lock(&self.devices).get(id).map(|d| (d.pair.clone(), d.link.is_some())).ok_or("That device isn't around")?;
        match state {
            (Pair::RequestedByPeer { .. }, _) => self.accept_pair(id),
            (Pair::Paired, _) => Ok(json!({"paired": true})),
            (_, false) => Err("That device isn't connected right now".into()),
            _ => {
                let ts = now_secs();
                self.send_body(id, "kdeconnect.pair", json!({"pair": true, "timestamp": ts}))?;
                let key = {
                    let mut g = lock(&self.devices);
                    let d = g.get_mut(id).ok_or("That device isn't around")?;
                    d.pair = Pair::Requested { ts, at: Instant::now() };
                    d.link.as_ref().and_then(|l| crate::identity::verification_key(&self.me.cert_der, &l.cert, ts)).unwrap_or_default()
                };
                self.changed();
                Ok(json!({"key": key}))
            }
        }
    }

    pub fn accept_pair(self: &Arc<Self>, id: &str) -> Result<Value, String> {
        {
            let mut g = lock(&self.devices);
            let d = g.get_mut(id).ok_or("That device isn't around")?;
            if !matches!(d.pair, Pair::RequestedByPeer { .. }) { return Err("That phone isn't asking to pair any more".into()); }
            d.pair = Pair::Paired;
        }
        self.send_body(id, "kdeconnect.pair", json!({"pair": true}))?;
        self.store_pair(id);
        self.changed();
        Ok(json!({"paired": true}))
    }

    pub fn reject_pair(&self, id: &str) -> Result<Value, String> {
        if let Some(d) = lock(&self.devices).get_mut(id) { d.pair = Pair::None; }
        let _ = self.send_body(id, "kdeconnect.pair", json!({"pair": false}));
        self.changed();
        Ok(json!({}))
    }

    /// Unpair (tell the phone if it's there) and forget it.
    pub fn unpair(&self, id: &str) -> Result<Value, String> {
        let _ = self.send_body(id, "kdeconnect.pair", json!({"pair": false}));
        if let Some(d) = lock(&self.devices).get_mut(id) { d.pair = Pair::None; d.notifications.clear(); d.battery = None; }
        lock(&self.paired).remove(id);
        self.save_paired();
        self.emit("unpaired", json!({"device": id}));
        self.changed();
        Ok(json!({}))
    }

    // ------------------------------------------------------------------ share

    fn share_in(self: &Arc<Self>, id: &str, from: &str, p: &Packet, pd: &PairedDev) {
        if let (Some(name), Some(port)) = (p.str("filename"), p.payload_port()) {
            let safe = safe_name(name);
            let token = crate::random_hex(8);
            let hidden = self.dest.join(".incoming");
            let tmp = hidden.join(format!("{token}.part"));
            let size = p.payload_size.unwrap_or(-1);
            lock(&self.files).push(IncomingFile { token: token.clone(), device: id.into(), from: from.into(), name: safe.clone(), size, received: 0, tmp: tmp.clone(),
                done: None, decided: if pd.auto_accept { Some(true) } else { None }, saved: None, at: Instant::now() });
            self.emit("file-offer", json!({"token": token, "device": id, "from": from, "name": safe, "size": size, "auto": pd.auto_accept}));
            if !pd.auto_accept { self.notify_desktop(&format!("{from} sent {safe}"), "Open Hearth Connect to keep it.", "document-save", false); }
            let (d, id) = (self.clone(), id.to_string());
            tokio::spawn(async move {
                let _ = tokio::fs::create_dir_all(&hidden).await;
                let d2 = d.clone();
                let t2 = token.clone();
                let last = Mutex::new(Instant::now());
                let r = lan::download(&d, &id, port, size, &tmp, move |got| {
                    if let Some(f) = lock(&d2.files).iter_mut().find(|f| f.token == t2) { f.received = got; }
                    let mut l = lock(&last);
                    if l.elapsed() > Duration::from_millis(300) { *l = Instant::now(); d2.emit("file-progress", json!({"token": t2, "received": got, "size": size})); }
                }).await.map(|_| ());
                if r.is_err() { let _ = tokio::fs::remove_file(&tmp).await; }
                if let Some(f) = lock(&d.files).iter_mut().find(|f| f.token == token) { f.done = Some(r.clone()); }
                if let Err(e) = &r { d.emit("file-failed", json!({"token": token, "error": e})); }
                d.settle(&token);
            });
            return;
        }
        if let Some(text) = p.str("text") {
            let v = json!({"device": id, "from": from, "text": text, "at": crate::now_ms()});
            { let mut t = lock(&self.texts); t.push_front(v.clone()); t.truncate(30); }
            self.notify_desktop(&format!("{from} shared text"), &text.chars().take(200).collect::<String>(), "edit-paste", false);
            self.emit("text", v);
            return;
        }
        if let Some(url) = p.str("url") {
            let v = json!({"device": id, "from": from, "url": url, "at": crate::now_ms()});
            { let mut t = lock(&self.texts); t.push_front(v.clone()); t.truncate(30); }
            self.notify_desktop(&format!("{from} shared a link"), url, "applications-internet", false);
            self.emit("url", v);
        }
    }

    /// A received file whose download finished and/or whose answer came: saved, thrown away, or still waiting.
    fn settle(&self, token: &str) {
        let mut ev = None;
        {
            let mut g = lock(&self.files);
            let Some(f) = g.iter_mut().find(|f| f.token == token) else { return };
            match (&f.done, f.decided) {
                (Some(Ok(())), Some(true)) if f.saved.is_none() => {
                    let _ = std::fs::create_dir_all(&self.dest);
                    let target = unique(&self.dest.join(&f.name));
                    match std::fs::rename(&f.tmp, &target) {
                        Ok(()) => { f.saved = Some(target.clone()); ev = Some(("file-saved", json!({"token": token, "from": f.from, "name": f.name, "path": target.to_string_lossy()}))); }
                        Err(e) => ev = Some(("file-failed", json!({"token": token, "error": format!("Couldn't save it: {e}")}))),
                    }
                }
                (Some(_), Some(false)) | (Some(Err(_)), _) => { let _ = std::fs::remove_file(&f.tmp); }
                _ => {}
            }
            // Kept: still downloading or waiting for an answer, or saved a little while ago (the window shows it).
            g.retain(|f| match (&f.done, f.decided) {
                (Some(Err(_)), _) => false,
                (done, Some(false)) => done.is_none(),
                (Some(Ok(())), Some(true)) => f.saved.is_none() || f.at.elapsed() < Duration::from_secs(600),
                _ => true,
            });
        }
        if let Some((e, v)) = ev { self.emit(e, v); }
        self.changed();
    }

    pub fn decide_file(&self, token: &str, accept: bool) -> Result<Value, String> {
        {
            let mut g = lock(&self.files);
            let f = g.iter_mut().find(|f| f.token == token).ok_or("That file isn't waiting any more")?;
            f.decided = Some(accept);
        }
        self.settle(token);
        Ok(json!({}))
    }

    /// Sends files to a phone, one after another (KDE Connect's share plugin).
    pub fn share_files(self: &Arc<Self>, id: &str, paths: Vec<PathBuf>) -> Result<Value, String> {
        if self.link_of(id).is_none() { return Err("The phone isn't connected right now".into()); }
        let files: Vec<(PathBuf, u64)> = paths.into_iter().filter_map(|p| std::fs::metadata(&p).ok().filter(|m| m.is_file()).map(|m| (p, m.len()))).collect();
        if files.is_empty() { return Err("Choose files to send (folders can't be sent)".into()); }
        let total: u64 = files.iter().map(|f| f.1).sum();
        let (d, id) = (self.clone(), id.to_string());
        let count = files.len();
        tokio::spawn(async move {
            let name = d.device_name(&id);
            let mut done = 0u64;
            for (i, (path, size)) in files.into_iter().enumerate() {
                let fname = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
                let modified = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
                let p = Packet::new("kdeconnect.share.request", json!({"filename": fname, "open": false, "numberOfFiles": count, "totalPayloadSize": total, "lastModified": modified}));
                let d2 = d.clone();
                let idc = id.clone();
                let last = Mutex::new(Instant::now());
                let base = done;
                let r = lan::upload(&d, &id, p, Some(path.clone()), vec![], move |sent| {
                    let mut l = lock(&last);
                    if l.elapsed() > Duration::from_millis(300) { *l = Instant::now(); d2.emit("send-progress", json!({"device": idc, "sent": base + sent, "total": total, "file": fname, "index": i + 1, "count": count})); }
                }).await;
                if let Err(e) = r { d.emit("send-failed", json!({"device": id, "to": name, "error": e})); d.changed(); return; }
                done += size;
            }
            d.emit("sent", json!({"device": id, "to": name, "count": count, "total": total}));
        });
        Ok(json!({"count": count, "total": total}))
    }

    // ------------------------------------------------------------------ notifications

    fn notification_in(&self, id: &str, from: &str, p: &Packet) {
        let nid = p.str("id").unwrap_or("").to_string();
        if nid.is_empty() { return; }
        if p.bool("isCancel") {
            let desk = lock(&self.devices).get_mut(id).and_then(|d| { d.notifications.retain(|n| n["id"] != nid.as_str()); d.notif_ids.remove(&nid) });
            if let Some(n) = desk { tokio::task::spawn_blocking(move || desktop::close_notification(n)); }
            self.emit("notifications", json!({"device": id}));
            return;
        }
        let entry = json!({
            "id": nid, "appName": p.str("appName").unwrap_or(""), "title": p.str("title").unwrap_or(""), "text": p.str("text").unwrap_or(p.str("ticker").unwrap_or("")),
            "time": p.int("time").unwrap_or(crate::now_ms()), "replyId": p.str("requestReplyId"), "actions": p.body.get("actions").cloned().unwrap_or(json!([])),
            "dismissable": p.body.get("isClearable").and_then(|v| v.as_bool()).unwrap_or(true),
        });
        let silent = p.bool("silent");
        let replaces = {
            let mut g = lock(&self.devices);
            let Some(d) = g.get_mut(id) else { return };
            d.notifications.retain(|n| n["id"] != nid.as_str());
            d.notifications.insert(0, entry.clone());
            d.notifications.truncate(100);
            d.notif_ids.get(&nid).copied().unwrap_or(0)
        };
        if !silent {
            let app = entry["appName"].as_str().unwrap_or("").to_string();
            let title = entry["title"].as_str().unwrap_or("").to_string();
            let summary = match (app.is_empty(), title.is_empty()) { (false, false) if app != title => format!("{app}: {title}"), (false, _) => app, _ => title };
            let body = entry["text"].as_str().unwrap_or("").to_string();
            let summary = if summary.is_empty() { from.to_string() } else { summary };
            let r = desktop::notify(&summary, &body, "smartphone", replaces, false);
            if let Some(n) = r { if let Some(d) = lock(&self.devices).get_mut(id) { d.notif_ids.insert(nid.clone(), n); } }
        }
        self.emit("notifications", json!({"device": id}));
    }

    // ------------------------------------------------------------------ media

    /// The phone controls this PC's players (or asks what's playing).
    fn mpris_request(self: &Arc<Self>, id: &str, p: Packet) {
        let (d, id) = (self.clone(), id.to_string());
        tokio::task::spawn_blocking(move || {
            let players = desktop::players();
            if p.bool("requestPlayerList") {
                let _ = d.send_body(&id, "kdeconnect.mpris", json!({"playerList": players.iter().map(|x| x.1.clone()).collect::<Vec<_>>(), "supportAlbumArtPayload": false}));
            }
            let Some(shown) = p.str("player") else { return };
            let wants_control = p.has("action") || p.has("setVolume") || p.has("Seek") || p.has("SetPosition");
            match players.iter().find(|x| x.1 == shown) {
                Some((name, shown)) => {
                    if wants_control { desktop::player_control(name, &p.body); std::thread::sleep(Duration::from_millis(150)); }
                    if wants_control || p.bool("requestNowPlaying") || p.bool("requestVolume") { let _ = d.send_body(&id, "kdeconnect.mpris", desktop::player_status(name, shown)); }
                }
                None => if let Some(a) = p.str("action") {
                    // No such player: the media keys still reach whatever is playing (games, Steam...).
                    let key = match a { "PlayPause" | "Play" | "Pause" => Some(input::KEY_PLAYPAUSE), "Next" => Some(input::KEY_NEXTSONG), "Previous" => Some(input::KEY_PREVIOUSSONG), "Stop" => Some(input::KEY_STOPCD), _ => None };
                    if let Some(k) = key { let _ = lock(&d.input).key(k); }
                },
            }
        });
    }

    /// The phone's players and what they play (for controlling the phone's music from here).
    fn phone_media(&self, id: &str, p: &Packet) {
        let mut ask = None;
        {
            let mut g = lock(&self.devices);
            let Some(d) = g.get_mut(id) else { return };
            if let Some(list) = p.body.get("playerList").and_then(|v| v.as_array()) {
                d.players = list.iter().filter_map(|x| x.as_str().map(String::from)).collect();
                d.playing.retain(|k, _| d.players.contains(k));
                ask = d.players.first().cloned();
            }
            if let Some(pl) = p.str("player") {
                if p.has("title") || p.has("isPlaying") || p.has("nowPlaying") {
                    let e = d.playing.entry(pl.to_string()).or_insert_with(|| json!({}));
                    if let (Some(o), Some(b)) = (e.as_object_mut(), p.body.as_object()) { for (k, v) in b { o.insert(k.clone(), v.clone()); } }
                }
            }
        }
        if let Some(pl) = ask { let _ = self.send_body(id, "kdeconnect.mpris.request", json!({"player": pl, "requestNowPlaying": true, "requestVolume": true})); }
        self.emit("media", json!({"device": id}));
    }

    // ------------------------------------------------------------------ background work

    /// Pairing that wasn't answered in time; the PC's clipboard and battery going to the phones.
    pub async fn housekeeping(self: Arc<Self>, mut stop: watch::Receiver<bool>) {
        let mut tick = 0u64;
        loop {
            tokio::select! { _ = tokio::time::sleep(Duration::from_secs(2)) => {}, _ = stop.changed() => return }
            tick += 1;
            // Pairing timeouts.
            let expired: Vec<String> = lock(&self.devices).iter().filter(|(_, d)| match d.pair {
                Pair::Requested { at, .. } | Pair::RequestedByPeer { at, .. } => at.elapsed() > PAIR_TIMEOUT, _ => false
            }).map(|(k, _)| k.clone()).collect();
            for id in expired {
                let _ = self.send_body(&id, "kdeconnect.pair", json!({"pair": false}));
                if let Some(d) = lock(&self.devices).get_mut(&id) { d.pair = Pair::None; }
                self.emit("pair-failed", json!({"device": id, "from": self.device_name(&id), "error": "Pairing timed out"}));
                self.changed();
            }
            // Received files nobody answered for half an hour: thrown away.
            let stale: Vec<(String, PathBuf)> = lock(&self.files).iter().filter(|f| f.decided.is_none() && f.at.elapsed() > Duration::from_secs(1800)).map(|f| (f.token.clone(), f.tmp.clone())).collect();
            for (t, p) in stale { let _ = std::fs::remove_file(p); lock(&self.files).retain(|f| f.token != t); self.changed(); }
            // Clipboard: this PC's, to phones that sync it.
            let linked: Vec<String> = lock(&self.devices).iter().filter(|(_, d)| d.link.is_some()).map(|(k, _)| k.clone()).collect();
            let targets: Vec<String> = linked.into_iter().filter(|k| self.paired_dev(k).map(|p| p.clipboard).unwrap_or(false)).collect();
            if !targets.is_empty() {
                if let Ok(Some(text)) = tokio::task::spawn_blocking(desktop::clipboard_get).await {
                    let changed = { let mut l = lock(&self.clip_last); if !text.is_empty() && *l != text { *l = text.clone(); true } else { false } };
                    if changed { for id in &targets { let _ = self.send_body(id, "kdeconnect.clipboard", json!({"content": text})); } }
                }
            }
            // Battery, every minute when it changed.
            if tick % 30 == 1 {
                let b = desktop::battery();
                let changed = { let mut m = lock(&self.my_battery); let c = *m != b; *m = b; c };
                if changed {
                    if let Some((c, ch)) = b {
                        let linked: Vec<String> = lock(&self.devices).iter().filter(|(_, d)| d.link.is_some()).map(|(k, _)| k.clone()).collect();
                        for id in linked.into_iter().filter(|k| self.is_paired(k)) { let _ = self.send_body(&id, "kdeconnect.battery", json!({"currentCharge": c, "isCharging": ch, "thresholdEvent": if c <= 15 && !ch { 1 } else { 0 }})); }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ the window's requests

    pub fn snapshot(&self) -> Value {
        let paired = lock(&self.paired).clone();
        let g = lock(&self.devices);
        let mut ids: Vec<String> = g.keys().cloned().collect();
        for k in paired.keys() { if !ids.contains(k) { ids.push(k.clone()); } }
        let mut devices: Vec<Value> = ids.iter().map(|id| {
            let d = g.get(id);
            let pd = paired.get(id);
            let pair = match d.map(|d| &d.pair) {
                Some(Pair::Requested { ts, .. }) => json!({"state": "requested", "key": d.and_then(|d| d.link.as_ref()).and_then(|l| crate::identity::verification_key(&self.me.cert_der, &l.cert, *ts))}),
                Some(Pair::RequestedByPeer { ts, .. }) => json!({"state": "asking", "key": d.and_then(|d| d.link.as_ref()).and_then(|l| crate::identity::verification_key(&self.me.cert_der, &l.cert, *ts))}),
                _ => json!({"state": if pd.is_some() { "paired" } else { "none" }}),
            };
            json!({
                "id": id, "name": d.map(|d| d.peer.name.clone()).or_else(|| pd.map(|p| p.name.clone())).unwrap_or_default(),
                "kind": d.map(|d| d.peer.kind.clone()).or_else(|| pd.map(|p| p.kind.clone())).unwrap_or_default(),
                "reachable": d.map(|d| d.link.is_some()).unwrap_or(false), "ip": d.and_then(|d| d.link.as_ref().map(|l| l.ip.to_string())),
                "paired": pd.is_some(), "pair": pair,
                "battery": d.and_then(|d| d.battery).map(|(c, ch)| json!({"charge": c, "charging": ch})),
                "keyboard": d.map(|d| d.keyboard).unwrap_or(false),
                "players": d.map(|d| d.players.clone()).unwrap_or_default(), "playing": d.map(|d| json!(d.playing)).unwrap_or(json!({})),
                "notifications": d.map(|d| json!(d.notifications)).unwrap_or(json!([])),
                "options": pd.map(|p| json!({"autoAccept": p.auto_accept, "clipboard": p.clipboard, "input": p.input, "notifications": p.notifications})),
            })
        }).collect();
        drop(g);
        devices.sort_by_key(|d| (!d["paired"].as_bool().unwrap_or(false), !d["reachable"].as_bool().unwrap_or(false), d["name"].as_str().unwrap_or("").to_lowercase()));
        let files: Vec<Value> = lock(&self.files).iter().map(|f| json!({"token": f.token, "device": f.device, "from": f.from, "name": f.name, "size": f.size, "received": f.received,
            "done": f.done.as_ref().map(|r| r.is_ok()), "error": f.done.as_ref().and_then(|r| r.clone().err()), "decided": f.decided, "saved": f.saved.as_ref().map(|p| p.to_string_lossy().to_string())})).collect();
        let input = lock(&self.input);
        json!({
            "name": *lock(&self.name), "deviceId": self.me.device_id, "tcpPort": self.tcp_port.load(Ordering::SeqCst), "kind": self.kind,
            "devices": devices, "files": files, "texts": json!(*lock(&self.texts)), "log": json!(*lock(&self.log)),
            "folder": self.dest.to_string_lossy(), "firewall": *lock(&self.firewall), "inputReady": input.ready(), "inputError": input.error,
            "oldProtocol": json!(*lock(&self.old_protocol)), "manual": lock(&self.manual).iter().map(|i| i.to_string()).collect::<Vec<_>>(),
        })
    }

    pub async fn command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let s = |k: &str| a.get(k).and_then(|v| v.as_str()).map(String::from).ok_or_else(|| format!("missing {k}"));
        match cmd {
            "state" => Ok(self.snapshot()),
            "pair" => self.request_pair(&s("id")?),
            "accept" => self.accept_pair(&s("id")?),
            "reject" => self.reject_pair(&s("id")?),
            "unpair" | "forget" => self.unpair(&s("id")?),
            "ping" => self.send_body(&s("id")?, "kdeconnect.ping", json!({"message": "Ping from Hearth"})).map(|_| json!({})),
            "ring" => self.send_body(&s("id")?, "kdeconnect.findmyphone.request", json!({})).map(|_| json!({})),
            "share" => {
                let paths: Vec<PathBuf> = a.get("paths").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_str().map(PathBuf::from)).collect()).unwrap_or_default();
                self.share_files(&s("id")?, paths)
            }
            "text" => self.send_body(&s("id")?, "kdeconnect.share.request", json!({"text": s("text")?})).map(|_| json!({})),
            "url" => self.send_body(&s("id")?, "kdeconnect.share.request", json!({"url": s("url")?})).map(|_| json!({})),
            "clipboard" => {
                let id = s("id")?;
                let text = tokio::task::spawn_blocking(desktop::clipboard_get).await.ok().flatten().filter(|t| !t.is_empty()).ok_or("The clipboard is empty (or can't be read here)")?;
                *lock(&self.clip_last) = text.clone();
                self.send_body(&id, "kdeconnect.clipboard", json!({"content": text})).map(|_| json!({}))
            }
            "media" => {
                let (id, player) = (s("id")?, s("player")?);
                let mut b = json!({"player": player});
                if let Some(act) = a.get("action").and_then(|v| v.as_str()) { b["action"] = json!(act); }
                if let Some(v) = a.get("volume").and_then(|v| v.as_i64()) { b["setVolume"] = json!(v.clamp(0, 100)); }
                self.send_body(&id, "kdeconnect.mpris.request", b)?;
                let _ = self.send_body(&id, "kdeconnect.mpris.request", json!({"player": player, "requestNowPlaying": true, "requestVolume": true}));
                Ok(json!({}))
            }
            "dismiss" => {
                let (id, n) = (s("id")?, s("notification")?);
                let _ = self.send_body(&id, "kdeconnect.notification.request", json!({"cancel": n}));
                let desk = lock(&self.devices).get_mut(&id).and_then(|d| { d.notifications.retain(|x| x["id"] != n.as_str()); d.notif_ids.remove(&n) });
                if let Some(x) = desk { tokio::task::spawn_blocking(move || desktop::close_notification(x)); }
                self.changed();
                Ok(json!({}))
            }
            "reply" => self.send_body(&s("id")?, "kdeconnect.notification.reply", json!({"requestReplyId": s("replyId")?, "message": s("message")?})).map(|_| json!({})),
            "action" => self.send_body(&s("id")?, "kdeconnect.notification.action", json!({"key": s("notification")?, "action": s("action")?})).map(|_| json!({})),
            "file" => self.decide_file(&s("token")?, a.get("accept").and_then(|v| v.as_bool()).unwrap_or(false)),
            "option" => {
                let (id, key) = (s("id")?, s("key")?);
                let on = a.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
                {
                    let mut g = lock(&self.paired);
                    let p = g.get_mut(&id).ok_or("Pair with it first")?;
                    match key.as_str() { "autoAccept" => p.auto_accept = on, "clipboard" => p.clipboard = on, "input" => p.input = on, "notifications" => p.notifications = on, _ => return Err("unknown option".into()) }
                }
                self.save_paired();
                if key == "notifications" && on { let _ = self.send_body(&id, "kdeconnect.notification.request", json!({"request": true})); }
                self.changed();
                Ok(json!({}))
            }
            "rename" => {
                let n = crate::identity::filter_name(&s("name")?);
                if n.is_empty() { return Err("The name can't be empty".into()); }
                *lock(&self.name) = n;
                self.save_settings();
                self.announce.notify_one();
                self.changed();
                Ok(json!({}))
            }
            "refresh" => { self.announce.notify_one(); Ok(json!({})) }
            "add_ip" => {
                let ip: IpAddr = s("ip")?.trim().parse().map_err(|_| "That isn't an IP address".to_string())?;
                if !crate::is_local_ip(ip) { return Err("Only addresses on your local network".into()); }
                { let mut m = lock(&self.manual); if !m.contains(&ip) { m.push(ip); } }
                self.save_settings();
                self.announce.notify_one();
                self.changed();
                Ok(json!({}))
            }
            "remove_ip" => {
                let ip = s("ip")?;
                lock(&self.manual).retain(|i| i.to_string() != ip);
                self.save_settings();
                self.changed();
                Ok(json!({}))
            }
            "quit" => { let _ = self.stop_tx.send(true); Ok(json!({})) }
            _ => Err(format!("unknown command {cmd}")),
        }
    }
}

/// The phone's file name made safe: no folders, no hidden files, no control characters.
pub fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = base.chars().filter(|c| !c.is_control() && !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
    let clean = clean.trim().trim_start_matches('.').trim().to_string();
    let clean: String = clean.chars().take(200).collect();
    if clean.is_empty() || clean == ".." { "file".into() } else { clean }
}

pub fn unique(path: &Path) -> PathBuf {
    if !path.exists() { return path.to_path_buf(); }
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let dir = path.parent().unwrap_or(Path::new("."));
    for n in 1..10000 { let p = dir.join(format!("{stem} ({n}){ext}")); if !p.exists() { return p; } }
    dir.join(format!("{stem} ({}){ext}", crate::random_hex(4)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_from_phones() {
        assert_eq!(safe_name("IMG_2041.jpg"), "IMG_2041.jpg");
        assert_eq!(safe_name("../../.bashrc"), "bashrc");
        assert_eq!(safe_name("/storage/emulated/0/DCIM/a b.mp4"), "a b.mp4");
        assert_eq!(safe_name("C:\\x\\evil?.exe"), "evil.exe");
        assert_eq!(safe_name(".."), "file");
        assert_eq!(safe_name(""), "file");
    }

    #[test]
    fn pairing_needs_the_person() {
        let dir = std::env::temp_dir().join(format!("connect-d-{}", crate::random_hex(4)));
        let me = crate::identity::load_or_create(&dir).unwrap();
        let other = crate::identity::generate().unwrap();
        let d = Daemon::new(me, dir.clone(), dir.join("in"));
        let (tx, _rx) = mpsc::unbounded_channel();
        let peer = Peer { id: other.device_id.clone(), name: "Pixel".into(), kind: "phone".into(), protocol: 8, incoming: vec![], outgoing: vec![], cert: other.cert_der.clone(), ip: "192.168.1.5".parse().unwrap() };
        lock(&d.devices).insert(peer.id.clone(), Dev { peer: peer.clone(), link: Some(Link { tx, ip: peer.ip, cert: peer.cert.clone(), gen: 1 }), pair: Pair::None,
            battery: None, notifications: vec![], notif_ids: HashMap::new(), players: vec![], playing: HashMap::new(), keyboard: false });
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            // Unpaired: a ping (or anything) does nothing.
            d.handle(&peer.id, Packet::new("kdeconnect.ping", json!({}))).await;
            assert!(lock(&d.log).is_empty());
            // A pairing request waits for the person.
            d.handle(&peer.id, Packet::new("kdeconnect.pair", json!({"pair": true, "timestamp": now_secs()}))).await;
            assert!(matches!(lock(&d.devices)[&peer.id].pair, Pair::RequestedByPeer { .. }));
            assert!(!d.is_paired(&peer.id));
            d.accept_pair(&peer.id).unwrap();
            assert!(d.is_paired(&peer.id));
            assert_eq!(lock(&d.paired)[&peer.id].cert, crate::hex(&other.cert_der));
            // Now a ping counts.
            d.handle(&peer.id, Packet::new("kdeconnect.ping", json!({"message": "hi"}))).await;
            assert!(lock(&d.log).iter().any(|e| e["event"] == "ping"));
            // A request with an old timestamp is refused (clocks out of sync).
            d.unpair(&peer.id).unwrap();
            d.handle(&peer.id, Packet::new("kdeconnect.pair", json!({"pair": true, "timestamp": now_secs() - 7200}))).await;
            assert_eq!(lock(&d.devices)[&peer.id].pair, Pair::None);
        });
        let _ = std::fs::remove_dir_all(dir);
    }
}
