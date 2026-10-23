//! Nearby Share running: the HTTPS server, discovery and the bookkeeping the window shows (devices seen, the request
//! waiting for a yes or no, what's being received, settings). One per window (or per test instance).

use crate::proto::{FileMeta, Info};
use crate::tls::Identity;
use crate::{firewall, tls, Emit};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, watch, Notify};
use tokio_util::sync::CancellationToken;

/// A lock that keeps working after a panic elsewhere.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> { m.lock().unwrap_or_else(|p| p.into_inner()) }

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub fingerprint: String,
    pub alias: String,
    pub device_model: Option<String>,
    pub device_type: Option<String>,
    pub ip: IpAddr,
    pub port: u16,
    pub https: bool,
    pub seen: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Trusted {
    pub fingerprint: String,
    pub alias: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub alias: String,
    /// Senders must give this PIN (LocalSend asks for it). None: no PIN.
    #[serde(default)]
    pub pin: Option<String>,
    /// Devices whose files are saved without asking. Only devices that prove who they are (their certificate).
    #[serde(default)]
    pub trusted: Vec<Trusted>,
}

pub enum Decision {
    Accept { ids: Option<Vec<String>>, trust: bool },
    Decline,
}

/// The one transfer to this PC at a time: asking, then receiving.
pub struct Receive {
    pub session: String,
    pub ip: IpAddr,
    pub info: Info,
    pub cert_fp: Option<String>,
    pub files: HashMap<String, FileMeta>,
    pub decide: Option<oneshot::Sender<Decision>>,
    pub tokens: HashMap<String, String>,
    pub done: HashSet<String>,
    pub uploading: HashSet<String>,
    pub saved: Vec<String>,
    pub received: u64,
    pub total: u64,
    pub last: Instant,
    pub cancelled: Arc<AtomicBool>,
}

impl Receive {
    /// Did the sender prove the fingerprint it claims (its certificate)?
    pub fn verified(&self) -> bool { self.cert_fp.as_deref().map(|c| c.eq_ignore_ascii_case(&self.info.fingerprint)).unwrap_or(false) }
}

pub struct Options {
    pub port: u16,
    pub alias: Option<String>,
    pub dest: PathBuf,
    pub home: PathBuf,
    /// Tests only (HEARTH_NEARBY_TEST): answer every request without a window.
    pub answer: Option<bool>,
    pub firewall: bool,
    pub discovery: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { port: crate::DEFAULT_PORT, alias: None, dest: crate::receive_dir(), home: crate::config_dir(), answer: None, firewall: false, discovery: true }
    }
}

pub struct Service {
    pub id: Identity,
    pub port: u16,
    pub opts: Options,
    pub settings: Mutex<Settings>,
    pub devices: Mutex<HashMap<String, Device>>,
    pub receive: Mutex<Option<Receive>>,
    pub emit: Emit,
    pub refresh: Notify,
    pub stop_tx: watch::Sender<bool>,
    pub pin_fail: Mutex<HashMap<IpAddr, (u32, Instant)>>,
    pub firewall: Mutex<Value>,
    pub sending: Mutex<Option<CancellationToken>>,
    pub answered: Mutex<HashMap<String, Instant>>,
    pub stopped: AtomicBool,
}

fn settings_path(o: &Options) -> PathBuf { o.home.join("settings.json") }

/// Listens on `port`, or the next free one up to port + 10 (another LocalSend may have 53317).
async fn bind(port: u16) -> Result<(tokio::net::TcpListener, u16), String> {
    let mut last = String::new();
    for p in port..port.saturating_add(11) {
        match tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, p))).await {
            Ok(l) => return Ok((l, p)),
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("No free port from {port}: {last}"))
}

pub async fn start(opts: Options, emit: Emit) -> Result<Arc<Service>, String> {
    std::fs::create_dir_all(&opts.home).map_err(|e| format!("{}: {e}", opts.home.display()))?;
    let id = tls::load_or_create(&opts.home)?;
    let mut settings: Settings = std::fs::read_to_string(settings_path(&opts)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    if let Some(a) = &opts.alias { settings.alias = a.clone(); }
    if settings.alias.trim().is_empty() { settings.alias = crate::host_name(); }
    let (listener, port) = bind(opts.port).await?;
    let acceptor = tokio_rustls::TlsAcceptor::from(tls::server_config(&id)?);
    let (stop_tx, stop_rx) = watch::channel(false);
    let firewall_on = opts.firewall;
    let discovery_on = opts.discovery;
    let svc = Arc::new(Service {
        id, port, opts, settings: Mutex::new(settings), devices: Mutex::new(HashMap::new()), receive: Mutex::new(None), emit,
        refresh: Notify::new(), stop_tx, pin_fail: Mutex::new(HashMap::new()),
        firewall: Mutex::new(json!({"ok": true, "firewall": "unmanaged", "note": "The firewall isn't changed in this mode"})),
        sending: Mutex::new(None), answered: Mutex::new(HashMap::new()), stopped: AtomicBool::new(false),
    });
    tokio::spawn(crate::server::run(svc.clone(), listener, acceptor, stop_rx.clone()));
    if discovery_on { tokio::spawn(crate::discovery::run(svc.clone(), stop_rx.clone())); }
    tokio::spawn(housekeeping(svc.clone(), stop_rx));
    if firewall_on {
        *lock(&svc.firewall) = json!({"ok": true, "firewall": "opening", "note": "Opening the firewall for local devices…"});
        let s = svc.clone();
        tokio::task::spawn_blocking(move || {
            let v = firewall::helper("open", "nearby", Some(s.port));
            *lock(&s.firewall) = v;
            s.changed();
        });
    }
    Ok(svc)
}

/// The receive that was given up on: nothing came for two minutes.
async fn housekeeping(svc: Arc<Service>, mut stop: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            _ = stop.changed() => return,
        }
        let stale = {
            let g = lock(&svc.receive);
            g.as_ref().map(|r| r.decide.is_none() && r.uploading.is_empty() && r.last.elapsed() > Duration::from_secs(120)).unwrap_or(false)
        };
        if stale { svc.cancel_receive("The sender stopped sending"); }
        lock(&svc.pin_fail).retain(|_, (_, t)| t.elapsed() < Duration::from_secs(600));
        lock(&svc.answered).retain(|_, t| t.elapsed() < Duration::from_secs(60));
    }
}

impl Service {
    pub fn alias(&self) -> String { lock(&self.settings).alias.clone() }

    /// Who this PC is, as LocalSend wants it (register, info, announcements, prepare-upload).
    pub fn info(&self, announce: Option<bool>) -> Info {
        Info {
            alias: self.alias(), version: crate::PROTOCOL_VERSION.into(), device_model: Some("Hearth OS".into()), device_type: Some("desktop".into()),
            fingerprint: self.id.fingerprint.clone(), port: Some(self.port), protocol: Some("https".into()), download: false, announce, announcement: announce,
        }
    }

    /// The window should look again.
    pub fn changed(&self) { (self.emit)("state", json!({})); }

    pub fn save_settings(&self) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&*lock(&self.settings)).map_err(|e| e.to_string())?;
        tls::write_private(&settings_path(&self.opts), text.as_bytes())
    }

    /// A device announced itself, registered, or answered us.
    pub fn saw(&self, info: &Info, ip: IpAddr) {
        if info.fingerprint.is_empty() || info.fingerprint.eq_ignore_ascii_case(&self.id.fingerprint) || !crate::is_local_ip(ip) { return; }
        let d = Device {
            fingerprint: info.fingerprint.clone(), alias: info.alias.chars().filter(|c| !c.is_control()).take(64).collect(),
            device_model: info.device_model.clone(), device_type: info.device_type.clone(), ip: crate::canonical(ip),
            port: info.port.unwrap_or(crate::DEFAULT_PORT), https: info.https(), seen: crate::now_ms(),
        };
        let new = {
            let mut g = lock(&self.devices);
            let new = g.get(&d.fingerprint).map(|o| o.ip != d.ip || o.alias != d.alias || o.port != d.port).unwrap_or(true);
            g.insert(d.fingerprint.clone(), d);
            new
        };
        if new { self.changed(); }
    }

    pub fn is_trusted(&self, cert_fp: Option<&str>) -> bool {
        let Some(fp) = cert_fp else { return false };
        lock(&self.settings).trusted.iter().any(|t| t.fingerprint.eq_ignore_ascii_case(fp))
    }

    /// The person answered the request in the window.
    pub fn decide(&self, accept: bool, ids: Option<Vec<String>>, trust: bool) -> Result<(), String> {
        let tx = lock(&self.receive).as_mut().and_then(|r| r.decide.take()).ok_or("That request isn't waiting any more")?;
        let _ = tx.send(if accept { Decision::Accept { ids, trust } } else { Decision::Decline });
        Ok(())
    }

    /// Stops what's being received (the person cancelled, or the sender went away). Partial files are removed by
    /// the uploads themselves when they notice.
    pub fn cancel_receive(&self, why: &str) {
        let r = lock(&self.receive).take();
        if let Some(r) = r {
            r.cancelled.store(true, Ordering::SeqCst);
            (self.emit)("cancelled", json!({"sessionId": r.session, "from": r.info.alias, "reason": why, "saved": r.saved}));
            self.changed();
        }
    }

    pub fn trust(&self, fingerprint: &str, alias: &str, on: bool) -> Result<(), String> {
        {
            let mut s = lock(&self.settings);
            s.trusted.retain(|t| !t.fingerprint.eq_ignore_ascii_case(fingerprint));
            if on { s.trusted.push(Trusted { fingerprint: fingerprint.to_uppercase(), alias: alias.to_string() }); }
        }
        self.save_settings()?;
        self.changed();
        Ok(())
    }

    pub fn set_alias(&self, alias: &str) -> Result<(), String> {
        let a: String = alias.chars().filter(|c| !c.is_control()).take(48).collect::<String>().trim().to_string();
        if a.is_empty() { return Err("The name can't be empty".into()); }
        lock(&self.settings).alias = a;
        self.save_settings()?;
        self.refresh.notify_one();
        self.changed();
        Ok(())
    }

    /// A PIN senders must give (4 to 12 digits or letters), or None for no PIN.
    pub fn set_pin(&self, pin: Option<String>) -> Result<(), String> {
        let pin = pin.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
        if let Some(p) = &pin {
            if p.len() < 4 || p.len() > 12 || !p.chars().all(|c| c.is_ascii_alphanumeric()) { return Err("The PIN must be 4 to 12 letters or digits".into()); }
        }
        lock(&self.settings).pin = pin;
        self.save_settings()?;
        self.changed();
        Ok(())
    }

    /// Everything the window shows.
    pub fn snapshot(&self) -> Value {
        let s = lock(&self.settings).clone();
        let mut devices: Vec<Device> = lock(&self.devices).values().cloned().collect();
        devices.sort_by_key(|d| d.alias.to_lowercase());
        let devices: Vec<Value> = devices.iter().map(|d| {
            let mut v = serde_json::to_value(d).unwrap_or(Value::Null);
            v["typeLabel"] = json!(crate::proto::type_label(d.device_type.as_deref()));
            v["trusted"] = json!(s.trusted.iter().any(|t| t.fingerprint.eq_ignore_ascii_case(&d.fingerprint)));
            v
        }).collect();
        let receive = lock(&self.receive).as_ref().map(|r| self.describe(r));
        let fw = lock(&self.firewall).clone();
        json!({
            "alias": s.alias, "fingerprint": self.id.fingerprint, "port": self.port, "folder": self.opts.dest.to_string_lossy(),
            "pin": s.pin, "trusted": s.trusted, "devices": devices, "receive": receive,
            "firewall": firewall::describe(&fw), "firewallOk": fw["ok"].as_bool().unwrap_or(false),
            "sending": lock(&self.sending).is_some(), "addresses": local_addresses(),
        })
    }

    /// A transfer to this PC as the window shows it (the request, or the progress).
    pub fn describe(&self, r: &Receive) -> Value {
        let mut files: Vec<&FileMeta> = r.files.values().collect();
        files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        json!({
            "sessionId": r.session, "from": r.info.alias, "deviceModel": r.info.device_model, "deviceType": crate::proto::type_label(r.info.device_type.as_deref()),
            "ip": r.ip.to_string(), "verified": r.verified(), "certFingerprint": r.cert_fp, "claimedFingerprint": r.info.fingerprint,
            "trusted": self.is_trusted(r.cert_fp.as_deref()), "asking": r.decide.is_some(),
            "files": files.iter().map(|f| json!({"id": f.id, "name": f.file_name, "size": f.size, "type": f.file_type,
                "accepted": r.tokens.contains_key(&f.id), "done": r.done.contains(&f.id)})).collect::<Vec<_>>(),
            "count": r.files.len(), "total": r.total, "received": r.received, "saved": r.saved,
        })
    }

    /// Stop listening and close the firewall again (the window is closing).
    pub fn shutdown(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) { return; }
        let _ = self.stop_tx.send(true);
        if let Some(c) = lock(&self.sending).take() { c.cancel(); }
        self.cancel_receive("Nearby Share closed");
        if self.opts.firewall { let _ = firewall::helper("close", "nearby", Some(self.port)); }
    }
}

/// This PC's local IPv4 addresses (shown in the window, for "add by address" on the other device).
pub fn local_addresses() -> Vec<String> {
    if_addrs::get_if_addrs().unwrap_or_default().into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() { IpAddr::V4(a) if crate::is_local_ip(IpAddr::V4(a)) => Some(a.to_string()), _ => None })
        .collect()
}
