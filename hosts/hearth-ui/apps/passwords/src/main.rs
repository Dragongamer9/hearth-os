//! hearth-passwords-app: Hearth Passwords (docs/mockups/checklists/open-source-16-passwords.md; the approved board
//! docs/mockups/open-source-apps/16-passwords.png). Your vault, kept on your own server: Vaultwarden (AGPL-3) on the
//! Nvidia PC (hearth-oss-server, reached over Tailscale), spoken to with the Bitwarden protocol (bw.rs: RustCrypto's
//! PBKDF2 / Argon2id, HKDF, AES-256-CBC + HMAC-SHA256, RSA-OAEP), and KeePass files through KeePassXC's own command
//! (keepassxc-cli, GPL-3; kp.rs). The password maker, 2-step codes, the security check.
//!
//! The master password is typed here, used to make your keys and wiped: never logged, never written, never sent (only
//! its hash signs you in). The copy on this PC (~/.local/share/hearth/passwords/vault.json, 0600) is the server's, still
//! encrypted, so it opens with no internet. Your choices (not secrets): ~/.config/nimbus/passwords.json.
//!
//! $HEARTH_PASSWORDS_FIXTURE: a made-up, already opened vault (CI's pictures; nothing is sent or written then).

mod bw;
mod kp;
mod vault;

use bw::Keys;
use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, mouse_area, row, scrollable, slider, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};
use std::path::PathBuf;
use vault::{Item, Kind, Maker};
use zeroize::Zeroize;

#[derive(Clone)]
struct Opened {
    keys: Option<Keys>,
    token: String,
    items: Vec<Item>,
    folders: Vec<vault::Folder>,
    synced: i64,
    offline: bool,
    devices: Vec<Value>,
}
impl std::fmt::Debug for Opened {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Opened({} items)", self.items.len())
    }
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Email(String),
    Password(String),
    Code(String),
    Server(String),
    Unlock,
    Unlocked(Result<Opened, String>),
    Lock,
    Nav(String),
    Search(String),
    Sort,
    Pick(String),
    Copy(&'static str),
    ClearClip(Option<String>, String),
    ShowPw,
    OpenSite,
    Favourite,
    Edit,
    EditField(&'static str, String),
    SaveEdit,
    CancelEdit,
    New,
    More,
    Delete,
    Wrote(Result<String, String>),
    Maker,
    MakerHow(&'static str),
    MakerLen(f32),
    MakerSet(&'static str),
    Again,
    UseIt,
    CloseMaker,
    Fill(&'static str),
    Backup,
    Fix,
    Leaks,
    LeaksDone(Result<usize, String>),
    Server2(Value),
    Install,
    TurnOn,
    Ran(run::Output),
    OpenKeepass,
    KpPicked(String),
    KpOpen,
    KpOpened(Result<(String, Vec<Item>), String>),
    OpenServerPage,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

#[derive(Debug, Clone, Default)]
struct EditBuf {
    id: String,
    name: String,
    user: String,
    password: String,
    uri: String,
    notes: String,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    prefs: Prefs,
    email: String,
    pw: String,
    code: String,
    need_code: bool,
    busy: bool,
    keys: Option<Keys>,
    token: String,
    items: Vec<Item>,
    folders: Vec<vault::Folder>,
    open_ok: bool,
    offline: bool,
    synced: i64,
    devices: Vec<Value>,
    nav: String,
    q: String,
    sort_name: bool,
    open: String,
    show_pw: bool,
    maker: Option<(Maker, String)>,
    edit: Option<EditBuf>,
    menu: bool,
    said: String,
    now: i64,
    srv: Value,
    fixing: usize,
    leaks: Option<usize>,
    kp_file: String,
    kp_pw: String,
    device: String,
    used: Value,
    events: Value,
    kp_pending: String,
    fixture: bool,
}

fn data() -> PathBuf {
    hearth_oss::data_dir("passwords")
}
fn read(p: &std::path::Path) -> Value {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}
/// This device's id for your server (not a secret: it tells your devices apart).
fn device_id() -> String {
    let f = data().join("device.json");
    if let Some(id) = read(&f).get("id").and_then(Value::as_str) {
        return id.to_string();
    }
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let id = format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]);
    let _ = std::fs::create_dir_all(data());
    let _ = hearth_ui::settings_file::write_json(&f, &json!({"id": id}));
    id
}
fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-passwords".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

/// Unlocking: the keys from your master password, signed in to your server and the vault synced (or, with no
/// internet, the encrypted copy on this PC opened with the same keys).
fn unlock(base: String, email: String, mut pw: String, code: String) -> Result<Opened, String> {
    let cache_f = data().join("vault.json");
    let cache = read(&cache_f);
    let online = bw::prelogin(&base, &email);
    let kdf = match &online {
        Ok(k) => k.clone(),
        Err(e) => {
            if cache.get("email").and_then(Value::as_str) != Some(email.trim()) {
                pw.zeroize();
                return Err(e.clone());
            }
            bw::kdf_of(cache.get("kdf").unwrap_or(&Value::Null))
        }
    };
    let mut mk = bw::master_key(&pw, &email, &kdf)?;
    let hash = bw::master_hash(&mk, &pw);
    pw.zeroize();
    let stretched = bw::stretch(&mk);
    mk.zeroize();
    let (protected, private, token, sync, offline) = if online.is_ok() {
        let tok = bw::token(&base, &email, &hash, &device_id(), &code)?;
        let at = hearth_oss::ci_str(&tok, &["access_token"]);
        let sync = bw::get(&base, "/api/sync?excludeDomains=true", &at)?;
        let key = hearth_oss::ci_str(&tok, &["Key", "key"]);
        let private = hearth_oss::ci(&sync, &["profile", "Profile"]).map(|p| hearth_oss::ci_str(p, &["privateKey", "PrivateKey"])).unwrap_or_default();
        // the copy on this PC: the server's own, still encrypted (opens only with your master password)
        let _ = std::fs::create_dir_all(data());
        let v = json!({"email": email.trim(), "kdf": {"kdf": kdf.kind, "kdfIterations": kdf.iterations, "kdfMemory": kdf.memory, "kdfParallelism": kdf.parallelism}, "key": key, "sync": sync, "synced": hearth_oss::now()});
        if hearth_ui::settings_file::write_json(&cache_f, &v).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&cache_f, std::fs::Permissions::from_mode(0o600));
                let _ = std::fs::set_permissions(hearth_ui::settings_file::bak_path(&cache_f), std::fs::Permissions::from_mode(0o600));
            }
        }
        (key, private, at, sync, false)
    } else {
        let sync = cache.get("sync").cloned().unwrap_or(Value::Null);
        let private = hearth_oss::ci(&sync, &["profile", "Profile"]).map(|p| hearth_oss::ci_str(p, &["privateKey", "PrivateKey"])).unwrap_or_default();
        (cache.get("key").and_then(Value::as_str).unwrap_or("").to_string(), private, String::new(), sync, true)
    };
    let mut raw = bw::decrypt(&protected, &stretched)?;
    let user = Keys::from64(&raw).ok_or("Your vault's key isn't the right size")?;
    raw.zeroize();
    let der = if private.is_empty() { None } else { bw::decrypt(&private, &user).ok() };
    let (items, folders) = vault::open(&sync, &user, der.as_deref());
    let devices = if offline { Vec::new() } else { bw::get(&base, "/api/devices", &token).ok().and_then(|d| hearth_oss::ci(&d, &["data", "Data"]).and_then(Value::as_array).cloned()).unwrap_or_default() };
    let synced = if offline { cache.get("synced").and_then(Value::as_i64).unwrap_or(0) } else { hearth_oss::now() };
    Ok(Opened { keys: Some(user), token, items, folders, synced, offline, devices })
}

fn fixture() -> Option<Opened> {
    let f = std::env::var("HEARTH_PASSWORDS_FIXTURE").ok()?;
    let j: Value = serde_json::from_str(&std::fs::read_to_string(f).ok()?).ok()?;
    let kind = |s: &str| match s {
        "note" => Kind::Note,
        "card" => Kind::Card,
        "wifi" => Kind::Wifi,
        _ => Kind::Login,
    };
    let g = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let items = j.get("items").and_then(Value::as_array).cloned().unwrap_or_default().iter().map(|i| Item {
        id: g(i, "id"),
        kind: Some(kind(&g(i, "kind"))),
        name: g(i, "name"),
        user: g(i, "user"),
        password: g(i, "password"),
        totp: g(i, "totp"),
        uri: g(i, "uri"),
        notes: g(i, "notes"),
        folder: g(i, "folder"),
        favourite: i.get("favourite").and_then(Value::as_bool).unwrap_or(false),
        card_last4: g(i, "last4"),
        changed: g(i, "changed"),
        made: g(i, "made"),
        history: i.get("history").and_then(Value::as_array).map(|a| a.iter().map(|h| (g(h, "when"), g(h, "what"))).collect()).unwrap_or_default(),
        org: String::new(),
        from: "server".into(),
        raw: Value::Null,
    }).collect();
    let folders = j.get("folders").and_then(Value::as_array).map(|a| a.iter().filter_map(|f| f.as_str()).map(|n| vault::Folder { id: n.into(), name: n.into() }).collect()).unwrap_or_default();
    Some(Opened { keys: None, token: String::new(), items, folders, synced: hearth_oss::now() - j.get("syncedAgo").and_then(Value::as_i64).unwrap_or(120), offline: false, devices: j.get("devices").and_then(Value::as_array).cloned().unwrap_or_default() })
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = Prefs::load("passwords");
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("passwords", "Hearth Passwords").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            email: prefs.s("email", ""),
            prefs,
            pw: String::new(),
            code: String::new(),
            need_code: false,
            busy: false,
            keys: None,
            token: String::new(),
            items: Vec::new(),
            folders: Vec::new(),
            open_ok: false,
            offline: false,
            synced: 0,
            devices: Vec::new(),
            nav: "all".into(),
            q: String::new(),
            sort_name: true,
            open: String::new(),
            show_pw: false,
            maker: None,
            edit: None,
            menu: false,
            said: String::new(),
            now: hearth_oss::now(),
            srv: Value::Null,
            fixing: 0,
            leaks: read(&data().join("leaks.json")).get("found").and_then(Value::as_u64).map(|x| x as usize),
            kp_file: String::new(),
            kp_pw: String::new(),
            device: read(&data().join("device.json")).get("id").and_then(Value::as_str).unwrap_or("").to_string(),
            used: read(&data().join("used.json")),
            events: read(&data().join("events.json")),
            kp_pending: String::new(),
            fixture: false,
        };
        if let Some(o) = fixture() {
            a.fixture = true;
            a.leaks = Some(0);
            a.opened(o);
            a.open = app::arg("--open").unwrap_or_else(|| a.items.first().map(|i| i.id.clone()).unwrap_or_default());
            if app::arg("--show").as_deref() == Some("maker") {
                a.maker = Some((a.maker_prefs(), "r7#Vq4mLp!8eZw2Tn%kHs9Yb".into()));
            }
            return (a, Task::none());
        }
        (a, Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "status"])), |o| Msg::Server2(o.json().unwrap_or(Value::Null))))
    }
    fn base(&self) -> String {
        self.prefs.s("server", "http://127.0.0.1:8222").trim_end_matches('/').to_string()
    }
    fn opened(&mut self, o: Opened) {
        self.keys = o.keys;
        self.token = o.token;
        self.items = o.items;
        self.folders = o.folders;
        self.synced = o.synced;
        self.offline = o.offline;
        self.devices = o.devices;
        self.open_ok = true;
        if self.open.is_empty() || !self.items.iter().any(|i| i.id == self.open) {
            self.open = self.items.first().map(|i| i.id.clone()).unwrap_or_default();
        }
    }
    fn lock(&mut self) {
        for i in self.items.iter_mut() {
            i.wipe();
        }
        self.items.clear();
        self.keys = None;
        self.token.zeroize();
        self.pw.zeroize();
        self.kp_pw.zeroize();
        self.kp_file.clear();
        self.code.clear();
        self.open_ok = false;
        self.maker = None;
        self.edit = None;
        self.show_pw = false;
    }
    fn item(&self) -> Option<&Item> {
        self.items.iter().find(|i| i.id == self.open)
    }
    fn shown(&self) -> Vec<&Item> {
        let q = self.q.trim().to_lowercase();
        let mut v: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| match self.nav.as_str() {
                "fav" => i.favourite,
                "logins" => i.kind() == Kind::Login,
                "codes" => !i.totp.is_empty(),
                "cards" => i.kind() == Kind::Card,
                "wifi" => i.kind() == Kind::Wifi,
                "notes" => i.kind() == Kind::Note,
                n if n.starts_with("folder:") => i.folder == n[7..],
                _ => true,
            })
            .filter(|i| q.is_empty() || i.name.to_lowercase().contains(&q) || i.user.to_lowercase().contains(&q) || i.uri.to_lowercase().contains(&q))
            .collect();
        if !self.sort_name {
            v.sort_by(|a, b| b.changed.cmp(&a.changed));
        }
        v
    }
    fn maker_prefs(&self) -> Maker {
        Maker {
            how: self.prefs.s("makerHow", "password"),
            length: self.prefs.f("makerLength", 24.0) as u32,
            upper: self.prefs.b("makerUpper", true),
            lower: self.prefs.b("makerLower", true),
            digits: self.prefs.b("makerDigits", true),
            symbols: self.prefs.b("makerSymbols", true),
            no_lookalikes: self.prefs.b("makerNoLookalikes", false),
        }
    }
    fn set(&mut self, k: &str, v: Value) {
        if let Err(e) = self.prefs.set(k, v) {
            self.said = e;
        }
    }
    fn writable(&self) -> bool {
        !self.fixture && self.keys.is_some() && !self.token.is_empty()
    }
}

/// A changed item sent back to your server: its fields encrypted with the vault's key (the old password into its
/// history), then everything synced again.
fn put_item(a: &App, e: &EditBuf, new: bool) -> Task<Msg> {
    let Some(k) = a.keys.clone() else { return Task::none() };
    let (base, token) = (a.base(), a.token.clone());
    let old = a.items.iter().find(|i| i.id == e.id).cloned();
    let e = e.clone();
    Task::perform(
        bg(move || -> Result<String, String> {
            let enc = |s: &str| if s.is_empty() { Ok(Value::Null) } else { bw::encrypt(s.as_bytes(), &k).map(Value::String) };
            let mut body = old.as_ref().map(|o| bw::camel(&o.raw)).unwrap_or(json!({"type": 1, "login": {}}));
            body["name"] = enc(&e.name)?;
            body["notes"] = enc(&e.notes)?;
            if body.get("login").map(Value::is_object) != Some(true) {
                body["login"] = json!({});
            }
            body["login"]["username"] = enc(&e.user)?;
            body["login"]["password"] = enc(&e.password)?;
            body["login"]["uris"] = if e.uri.is_empty() { json!([]) } else { json!([{"uri": enc(&e.uri)?, "match": null}]) };
            if let Some(o) = &old {
                if o.password != e.password && !o.password.is_empty() {
                    let mut h = body.get("passwordHistory").and_then(Value::as_array).cloned().unwrap_or_default();
                    h.insert(0, json!({"password": enc(&o.password)?, "lastUsedDate": now_iso()}));
                    h.truncate(5);
                    body["passwordHistory"] = Value::Array(h);
                    body["login"]["passwordRevisionDate"] = json!(now_iso());
                }
                body["lastKnownRevisionDate"] = json!(o.changed);
            }
            if new {
                bw::send(&base, "POST", "/api/ciphers", &token, &body)?;
                Ok(format!("\"{}\" is in your vault.", e.name))
            } else {
                bw::send(&base, "PUT", &format!("/api/ciphers/{}", e.id), &token, &body)?;
                Ok(format!("\"{}\" is changed.", e.name))
            }
        }),
        |r| Msg::Wrote(r.unwrap_or_else(|| Err("it stopped".into()))),
    )
}

/// Now as the server writes dates (UTC, ISO 8601).
fn now_iso() -> String {
    let t = hearth_oss::now();
    let (days, secs) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    // days since 1970 to a civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", secs / 3600, secs / 60 % 60, secs % 60)
}

/// "Nov 15" from the server's dates (2026-11-15T…).
fn day_of(iso: &str) -> String {
    hearth_ui::time::parse_date(iso.get(..10).unwrap_or("")).map(|(_, m, d)| format!("{} {d}", hearth_ui::time::month(m))).unwrap_or_default()
}

fn resync(a: &mut App) -> Task<Msg> {
    let (base, token, keys) = (a.base(), a.token.clone(), a.keys.clone());
    let devices = a.devices.clone();
    Task::perform(
        bg(move || -> Result<Opened, String> {
            let k = keys.ok_or("locked")?;
            let sync = bw::get(&base, "/api/sync?excludeDomains=true", &token)?;
            let private = hearth_oss::ci(&sync, &["profile", "Profile"]).map(|p| hearth_oss::ci_str(p, &["privateKey", "PrivateKey"])).unwrap_or_default();
            let der = bw::decrypt(&private, &k).ok();
            let (items, folders) = vault::open(&sync, &k, der.as_deref());
            Ok(Opened { keys: Some(k), token, items, folders, synced: hearth_oss::now(), offline: false, devices })
        }),
        |r| Msg::Unlocked(r.unwrap_or_else(|| Err("it stopped".into()))),
    )
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    a.now = hearth_oss::now();
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {}
        Msg::Email(s) => a.email = s,
        Msg::Password(s) => {
            a.pw.zeroize();
            a.pw = s;
        }
        Msg::Code(s) => a.code = s.chars().filter(char::is_ascii_digit).take(8).collect(),
        Msg::Server(s) => {
            let s = s.trim().to_string();
            a.prefs.m.insert("server".into(), json!(s));
        }
        Msg::Unlock => {
            if a.busy || a.email.trim().is_empty() || a.pw.is_empty() {
                return Task::none();
            }
            a.busy = true;
            a.said = "Opening your vault…".into();
            if a.prefs.s("email", "") != a.email.trim() {
                let e = a.email.trim().to_string();
                a.set("email", json!(e));
            }
            let srv = a.base();
            if Prefs::load("passwords").s("server", "") != srv {
                a.set("server", json!(srv));
            }
            let pw = std::mem::take(&mut a.pw);
            let (b, e, c) = (a.base(), a.email.clone(), a.code.clone());
            return Task::perform(bg(move || unlock(b, e, pw, c)), |r| Msg::Unlocked(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::Unlocked(r) => {
            a.busy = false;
            match r {
                Ok(o) => {
                    a.said = if o.offline { "Opened the copy on this PC (your server isn't reachable now).".into() } else { String::new() };
                    a.need_code = false;
                    a.code.clear();
                    a.opened(o);
                    a.device = read(&data().join("device.json")).get("id").and_then(Value::as_str).unwrap_or("").to_string();
                }
                Err(e) if e == "two-step" => {
                    a.need_code = true;
                    a.said = "Your server asks for your 2-step code too.".into();
                }
                Err(e) => a.said = e,
            }
        }
        Msg::Lock => {
            a.lock();
            a.said = "Locked.".into();
        }
        Msg::Nav(n) => {
            a.nav = n;
            if let Some(f) = a.shown().first().map(|i| i.id.clone()) {
                a.open = f;
            }
        }
        Msg::Search(q) => a.q = q,
        Msg::Sort => a.sort_name = !a.sort_name,
        Msg::Pick(id) => {
            a.open = id;
            a.show_pw = false;
            a.edit = None;
            a.menu = false;
        }
        Msg::Copy(what) => {
            let Some(i) = a.item() else { return Task::none() };
            let s = match what {
                "user" => i.user.clone(),
                "password" => i.password.clone(),
                "code" => vault::totp_now(&i.totp, a.now as u64).map(|c| c.0).unwrap_or_default(),
                "maker" => a.maker.as_ref().map(|m| m.1.clone()).unwrap_or_default(),
                _ => i.uri.clone(),
            };
            if s.is_empty() {
                return Task::none();
            }
            if what == "password" && !a.fixture {
                let id = a.open.clone();
                let now = a.now;
                let _ = std::fs::create_dir_all(data());
                if let Ok((m, _)) = hearth_ui::settings_file::update_json(&data().join("used.json"), move |m| {
                    m.insert(id, json!(now));
                }) {
                    a.used = Value::Object(m);
                }
            }
            a.said = if matches!(what, "password" | "code" | "maker") { "Copied. It's taken off the clipboard in 30 s.".into() } else { "Copied.".into() };
            let (s2, s3) = (s.clone(), s);
            // a password stays on the clipboard only 30 seconds (unless something else was copied since)
            return Task::batch([iced::clipboard::write(s2), Task::perform(run::after(30_000), |_| ()).then(move |_| { let s = s3.clone(); iced::clipboard::read().map(move |c| Msg::ClearClip(c, s.clone())) })]);
        }
        Msg::ClearClip(now, mut was) => {
            let same = now.as_deref() == Some(was.as_str());
            was.zeroize();
            if same {
                return iced::clipboard::write(String::new());
            }
        }
        Msg::ShowPw => a.show_pw = !a.show_pw,
        Msg::OpenSite => {
            if let Some(i) = a.item() {
                let u = if i.uri.contains("://") { i.uri.clone() } else { format!("https://{}", i.uri) };
                run::detached(&["xdg-open", &u]);
            }
        }
        Msg::Favourite => {
            let Some(i) = a.item().cloned() else { return Task::none() };
            if !a.writable() {
                a.said = "Unlock with your server reachable to change it.".into();
                return Task::none();
            }
            let (base, token) = (a.base(), a.token.clone());
            let folder = hearth_oss::ci_str(&i.raw, &["folderId", "FolderId"]);
            let fav = !i.favourite;
            return Task::perform(
                bg(move || bw::send(&base, "PUT", &format!("/api/ciphers/{}/partial", i.id), &token, &json!({"favorite": fav, "folderId": if folder.is_empty() { Value::Null } else { json!(folder) }})).map(|_| if fav { "A favourite now.".to_string() } else { "Not a favourite now.".to_string() })),
                |r| Msg::Wrote(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Edit => {
            if let Some(i) = a.item() {
                a.edit = Some(EditBuf { id: i.id.clone(), name: i.name.clone(), user: i.user.clone(), password: i.password.clone(), uri: i.uri.clone(), notes: i.notes.clone() });
            }
        }
        Msg::New => a.edit = Some(EditBuf::default()),
        Msg::EditField(k, v) => {
            if let Some(e) = a.edit.as_mut() {
                match k {
                    "name" => e.name = v,
                    "user" => e.user = v,
                    "password" => e.password = v,
                    "uri" => e.uri = v,
                    _ => e.notes = v,
                }
            }
        }
        Msg::CancelEdit => {
            if let Some(mut e) = a.edit.take() {
                e.password.zeroize();
            }
        }
        Msg::SaveEdit => {
            let Some(e) = a.edit.clone() else { return Task::none() };
            if !a.writable() {
                a.said = "Unlock with your server reachable to change it.".into();
                return Task::none();
            }
            let new = e.id.is_empty();
            a.edit = None;
            return put_item(a, &e, new);
        }
        Msg::More => a.menu = !a.menu,
        Msg::Delete => {
            a.menu = false;
            let Some(i) = a.item().cloned() else { return Task::none() };
            if !a.writable() {
                return Task::none();
            }
            let (base, token) = (a.base(), a.token.clone());
            return Task::perform(bg(move || bw::send(&base, "PUT", &format!("/api/ciphers/{}/delete", i.id), &token, &json!({})).map(|_| format!("\"{}\" is in your server's bin (30 days).", i.name))), |r| Msg::Wrote(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::Wrote(r) => {
            a.said = match r {
                Ok(s) | Err(s) => s,
            };
            if a.writable() {
                return resync(a);
            }
        }
        Msg::Maker => {
            let m = a.maker_prefs();
            let s = vault::make(&m);
            a.maker = Some((m, s));
        }
        Msg::MakerHow(h) => {
            a.set("makerHow", json!(h));
            return update(a, Msg::Maker);
        }
        Msg::MakerLen(l) => {
            if let Some(m) = a.maker.as_mut() {
                m.0.length = l.round() as u32;
                m.1 = vault::make(&m.0);
            }
        }
        Msg::MakerSet(k) => {
            let key = match k {
                "upper" => "makerUpper",
                "lower" => "makerLower",
                "digits" => "makerDigits",
                "symbols" => "makerSymbols",
                "len" => "makerLength",
                _ => "makerNoLookalikes",
            };
            if k == "len" {
                let l = a.maker.as_ref().map(|m| m.0.length).unwrap_or(24);
                a.set(key, json!(l));
            } else {
                let d = key != "makerNoLookalikes";
                let v = !a.prefs.b(key, d);
                a.set(key, json!(v));
                return update(a, Msg::Maker);
            }
        }
        Msg::Again => {
            if let Some(m) = a.maker.as_mut() {
                m.1 = vault::make(&m.0);
            }
        }
        Msg::CloseMaker => {
            if let Some((_, mut s)) = a.maker.take() {
                s.zeroize();
            }
        }
        Msg::UseIt => {
            let Some((_, s)) = a.maker.take() else { return Task::none() };
            let Some(i) = a.item().cloned() else { return Task::none() };
            if i.from != "server" {
                let (file, kpw) = (a.kp_file.clone(), a.kp_pw.clone());
                return Task::perform(bg(move || kp::set_password(&file, &kpw, &i, &s)), |r| Msg::Wrote(r.unwrap_or_else(|| Err("it stopped".into()))));
            }
            if !a.writable() {
                a.said = "Unlock with your server reachable to change it.".into();
                return Task::none();
            }
            let e = EditBuf { id: i.id.clone(), name: i.name.clone(), user: i.user.clone(), password: s, uri: i.uri.clone(), notes: i.notes.clone() };
            return put_item(a, &e, false);
        }
        Msg::Fill(k) => {
            let d = k != "fillBrowsers";
            let v = !a.prefs.b(k, d);
            a.set(k, json!(v));
        }
        Msg::Backup => {
            let local = a.base().contains("127.0.0.1") || a.base().contains("localhost");
            if local {
                return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "backup", "vaultwarden"])), |o| {
                    Msg::Wrote(if o.ok { Ok(format!("Backed up on your server: {}", o.json().and_then(|j| j.get("backup").and_then(Value::as_str).map(str::to_string)).unwrap_or_default())) } else { Err(format!("The backup didn't work: {}", o.why())) })
                });
            }
            let src = data().join("vault.json");
            let dst = hearth_ui::paths::home().join("Documents").join(format!("hearth-passwords-backup-{}.json", now_iso().get(..10).unwrap_or("today")));
            let _ = std::fs::create_dir_all(hearth_ui::paths::home().join("Documents"));
            a.said = match std::fs::copy(&src, &dst) {
                Ok(_) => format!("A backup of your encrypted vault is in {} (it opens only with your master password).", dst.display()),
                Err(e) => format!("The backup didn't work: {e}"),
            };
        }
        Msg::Fix => {
            let c = vault::check(&a.items);
            if c.is_empty() {
                return Task::none();
            }
            let k = a.fixing % c.len();
            a.fixing += 1;
            if let Some(id) = c[k].2.first() {
                a.open = id.clone();
            }
            a.said = format!("{}: {} ({} of {})", c[k].0, c[k].1, k + 1, c.len());
            return update(a, Msg::Maker);
        }
        Msg::Leaks => {
            // Have I Been Pwned's range check: only the first 5 letters of each password's SHA-1 leave this PC
            let hashes: Vec<String> = a.items.iter().filter(|i| !i.password.is_empty()).map(|i| sha1_hex(&i.password)).collect();
            a.said = "Checking for leaks (only 5 letters of each password's SHA-1 leave this PC)…".into();
            return Task::perform(
                bg(move || -> Result<usize, String> {
                    let mut found = 0;
                    for h in hashes {
                        let o = run::blocking(&run::args(&["curl", "-fsS", "--max-time", "15", &format!("https://api.pwnedpasswords.com/range/{}", &h[..5])]));
                        if !o.ok {
                            return Err("The leak check couldn't reach haveibeenpwned.com".into());
                        }
                        if o.out.lines().any(|l| l.split(':').next().map(|x| x.eq_ignore_ascii_case(&h[5..])).unwrap_or(false)) {
                            found += 1;
                        }
                    }
                    Ok(found)
                }),
                |r| Msg::LeaksDone(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::LeaksDone(r) => match r {
            Ok(n) => {
                a.leaks = Some(n);
                let _ = hearth_ui::settings_file::write_json(&data().join("leaks.json"), &json!({"found": n, "at": hearth_oss::now()}));
                a.said = if n == 0 { "None of your passwords were found in leaks.".into() } else { format!("{n} of your passwords were found in leaks: change them.") };
            }
            Err(e) => a.said = e,
        },
        Msg::Server2(v) => a.srv = v,
        Msg::Install => {
            a.said = "Installing Vaultwarden on this PC (its own image, in your Podman)…".into();
            return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "install", "vaultwarden"])), Msg::Ran);
        }
        Msg::TurnOn => return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "on", "vaultwarden"])), Msg::Ran),
        Msg::Ran(o) => {
            a.said = if o.ok { "Done. Make your account on your server's page, then unlock here.".into() } else { format!("That didn't work: {}", o.why()) };
            return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "status"])), |o| Msg::Server2(o.json().unwrap_or(Value::Null)));
        }
        Msg::OpenServerPage => run::detached(&["xdg-open", &a.base()]),
        Msg::OpenKeepass => {
            return Task::perform(bg(|| run::blocking(&run::args(&["kdialog", "--getopenfilename", &hearth_ui::paths::home().to_string_lossy(), "*.kdbx"]))), |o| Msg::KpPicked(o.filter(|o| o.ok).map(|o| o.out.trim().to_string()).unwrap_or_default()));
        }
        Msg::KpPicked(f) => {
            if !f.is_empty() {
                a.kp_pending = f;
                a.said = "Type that file's master password, then press Open it.".into();
            }
        }
        Msg::KpOpen => {
            if a.kp_pending.is_empty() || a.pw.is_empty() {
                return Task::none();
            }
            a.busy = true;
            let (f, pw) = (a.kp_pending.clone(), std::mem::take(&mut a.pw));
            a.kp_pw = pw.clone();
            return Task::perform(bg(move || { let mut pw = pw; let r = kp::open(&f, &pw); pw.zeroize(); r }), |r| Msg::KpOpened(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::KpOpened(r) => {
            a.busy = false;
            match r {
                Ok((file, items)) => {
                    a.kp_file = file;
                    a.items.retain(|i| i.from == "server");
                    a.items.extend(items);
                    a.open_ok = true;
                    a.kp_pending.clear();
                    if a.open.is_empty() {
                        a.open = a.items.first().map(|i| i.id.clone()).unwrap_or_default();
                    }
                }
                Err(e) => {
                    a.kp_pw.zeroize();
                    a.said = e;
                }
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => {
            a.lock();
            return win::close();
        }
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn sha1_hex(s: &str) -> String {
    use sha1::Digest;
    sha1::Sha1::digest(s.as_bytes()).iter().map(|b| format!("{b:02X}")).collect()
}

// ---------------------------------------------------------------- the window

fn folder_colour(p: &Pal, i: usize) -> Color {
    [p.accent, w::blue(), w::green(), w::pink(), w::yellow(), w::purple()][i % 6]
}

fn side(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let n = |f: &dyn Fn(&Item) -> bool| a.items.iter().filter(|i| f(i)).count().to_string();
    let navs: Vec<(&str, &str, String)> = vec![
        ("all", "Everything", a.items.len().to_string()),
        ("fav", "Favourites", n(&|i| i.favourite)),
        ("logins", "Sign-ins", n(&|i| i.kind() == Kind::Login)),
        ("codes", "2-step codes", n(&|i| !i.totp.is_empty())),
        ("cards", "Cards", n(&|i| i.kind() == Kind::Card)),
        ("wifi", "Wi-Fi", n(&|i| i.kind() == Kind::Wifi)),
        ("notes", "Secure notes", n(&|i| i.kind() == Kind::Note)),
    ];
    let mut kinds = Column::new().spacing(2.0);
    for (id, label, count) in navs {
        kinds = kinds.push(w::nav(&p, None, label, count, a.nav == id, Msg::Nav(id.to_string())));
    }
    let mut folders = Column::new().spacing(2.0).push(w::cap(&p, "Folders"));
    let mut names: Vec<String> = a.folders.iter().map(|f| f.name.clone()).collect();
    if !a.kp_file.is_empty() {
        names.push(format!("KeePass · {}", std::path::Path::new(&a.kp_file).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()));
    }
    for (k, f) in names.iter().enumerate() {
        let id = format!("folder:{f}");
        let c = a.items.iter().filter(|i| &i.folder == f).count();
        folders = folders.push(w::nav(&p, Some(folder_colour(&p, k)), f.clone(), c.to_string(), a.nav == id, Msg::Nav(id.clone())));
    }
    let check = vault::check(&a.items);
    let twice = check.iter().filter(|c| c.0 == "Used twice").count();
    let weak = check.iter().filter(|c| c.0 == "Weak").count();
    let leaks: Element<'_, Msg> = match a.leaks {
        Some(0) => dot_line(&p, w::green(), "none found in leaks".into()),
        Some(n) => dot_line(&p, w::red(), format!("{n} found in leaks")),
        None => mouse_area(dot_line(&p, p.muted, "check for leaks ›".into())).on_press(Msg::Leaks).interaction(mouse::Interaction::Pointer).into(),
    };
    let health = column![
        w::cap(&p, "Health"),
        dot_line(&p, if twice > 0 { w::yellow() } else { w::green() }, format!("{twice} used twice")),
        dot_line(&p, if weak > 0 { w::yellow() } else { w::green() }, format!("{weak} weak")),
        leaks,
        ui::gap(1.0, 12.0),
        mouse_area(row![ui::glyph(G::Pencil, p.muted, 13.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
    ]
    .spacing(6.0);
    let body = hearth_ui::arrange::parts(&a.describe, &p, "side", vec![("kinds", kinds.into()), ("folders", folders.into()), ("health", health.into())], |ps| column![ps.take("kinds"), ps.take("folders"), ps.take("health")].into(), Msg::Describe);
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(250.0)).height(Length::Fill).padding(Padding { top: 14.0, right: 12.0, bottom: 14.0, left: 12.0 }).style(w::side_bg(&p)).into()
}

fn dot_line<'a>(p: &Pal, c: Color, s: String) -> Element<'a, Msg> {
    row![t("●", 12.0, c), t(s, 12.0, p.text)].spacing(4.0).align_y(iced::Center).into()
}

fn place<'a>(p: &Pal, c: Color, title: String, sub: String) -> Element<'a, Msg> {
    w::card2(p, column![row![w::swatch(c, 16.0, 16.0, 4.0), tb(title, 12.0, p.text)].spacing(8.0).align_y(iced::Center), t(sub, 12.0, p.muted)].spacing(6.0)).width(Length::FillPortion(1)).into()
}

fn edit_box<'a>(p: &Pal, k: &'static str, label: &str, v: &str, secure: bool) -> Element<'a, Msg> {
    let mut inp = w::input(p, label, v, 14.0, move |s| Msg::EditField(k, s));
    if secure {
        inp = inp.secure(true);
    }
    fl(p, label, inp.into(), None)
}

fn list(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let top = row![container(w::search(&p, "Search the vault", &a.q, Msg::Search)).width(Length::Fill), w::chip(&p, "＋", true, Sz::Md, Some(Msg::New))].spacing(8.0).align_y(iced::Center);
    let shown = a.shown();
    let count = row![t(format!("{} items", shown.len()), 12.0, p.muted), Space::new().width(Length::Fill), mouse_area(t(if a.sort_name { "Sort: Name ▾" } else { "Sort: Changed ▾" }, 12.0, p.muted)).on_press(Msg::Sort).interaction(mouse::Interaction::Pointer)];
    let mut c = Column::new().spacing(2.0);
    for i in shown {
        let on = i.id == a.open;
        let mut r = row![w::mark(&i.letters(), w::grad_for(&i.name), 34.0), column![t(i.name.clone(), 13.0, p.text), t(i.sub(), 11.0, p.muted)].spacing(2.0).width(Length::Fill)].spacing(12.0).align_y(iced::Center);
        if on {
            r = r.push(t("⧉", 12.0, p.muted));
        }
        let inner = container(r).padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 }).width(Length::Fill);
        c = c.push(mouse_area(w::lit_row(&p, inner.into(), on, 10.0, 54.0)).on_press(Msg::Pick(i.id.clone())).interaction(mouse::Interaction::Pointer));
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "list",
        vec![("search", top.into()), ("count", count.into()), ("items", scrollable(c).height(Length::Fill).into())],
        |ps| column![ps.take("search"), container(ps.take("count")).padding(Padding { top: 12.0, right: 2.0, bottom: 8.0, left: 2.0 }), ps.take("items")].into(),
        Msg::Describe,
    );
    container(body).width(Length::Fixed(420.0)).height(Length::Fill).padding(Padding { top: 16.0, right: 14.0, bottom: 16.0, left: 14.0 }).into()
}

/// .fl: a field's box with its name over it.
fn fl<'a>(p: &Pal, k: &str, v: Element<'a, Msg>, extra: Option<Element<'a, Msg>>) -> Element<'a, Msg> {
    let mut c = column![tb(k.to_uppercase(), 11.0, p.muted), v].spacing(4.0);
    if let Some(e) = extra {
        c = c.push(e);
    }
    container(c).padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 }).width(Length::Fill).style(ui::boxed(p.bg, p.r(10.0), Some((p.line, 1.0)))).into()
}

fn strength_bar<'a>(p: &Pal, pw: &str, w_: f32) -> Element<'a, Msg> {
    let (bits, word, time) = vault::strength(pw);
    let c = if bits >= 70.0 { w::green() } else if bits >= 45.0 { w::yellow() } else { w::red() };
    let frac = (bits / 128.0).min(0.98) as f32;
    row![w::meter(p, frac, Some(c), w_, 6.0), t(if time.is_empty() { String::new() } else { format!("{word} · {}", if w_ > 150.0 { time.to_string() } else { format!("{} characters", pw.chars().count()) }) }, 12.0, c)].spacing(8.0).align_y(iced::Center).into()
}

fn detail(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(i) = a.item() else {
        return container(t(if a.items.is_empty() { "Your vault is empty. Press ＋ to add a sign-in." } else { "Pick an item." }, 14.0, p.muted)).padding(30.0).into();
    };
    let mut meta = Vec::new();
    if !i.folder.is_empty() {
        meta.push(format!("Folder {}", i.folder));
    }
    if !i.changed.is_empty() {
        meta.push(format!("changed {}", day_of(&i.changed)));
    }
    let used = a.used.get(&i.id).and_then(Value::as_i64).unwrap_or(0);
    if used > 0 && a.now - used < 86_400 {
        meta.push("used today".into());
    }
    let mut more = Column::new().push(w::chip(&p, "⋯", false, Sz::Md, Some(Msg::More)));
    if a.menu {
        more = more.push(w::menu(&p, vec![("Make a password for it".into(), false, Msg::Maker), ("Delete (to the bin)".into(), false, Msg::Delete)], 220.0));
    }
    let head = row![
        w::mark(&i.letters(), w::grad_for(&i.name), 64.0),
        column![w::h1(&p, i.name.clone()), t(meta.join(" · "), 12.0, p.muted)].spacing(4.0),
        Space::new().width(Length::Fill),
        w::chip(&p, if i.favourite { "★ Favourite" } else { "☆ Favourite" }, false, Sz::Md, Some(Msg::Favourite)),
        w::chip(&p, "Edit", false, Sz::Md, Some(Msg::Edit)),
        more,
    ]
    .spacing(14.0)
    .align_y(iced::Center);

    // its fields (or the edit boxes)
    let fields: Element<'_, Msg> = if let Some(e) = &a.edit {
        column![
            edit_box(&p, "name", "Name", &e.name, false),
            edit_box(&p, "user", "User name", &e.user, false),
            edit_box(&p, "password", "Password", &e.password, !a.show_pw),
            edit_box(&p, "uri", "Website", &e.uri, false),
            edit_box(&p, "notes", "Notes", &e.notes, false),
            row![w::chip(&p, "Make a password", false, Sz::Sm, Some(Msg::Maker)), Space::new().width(Length::Fill), w::chip(&p, "Cancel", false, Sz::Sm, Some(Msg::CancelEdit)), w::chip(&p, "Save", true, Sz::Sm, Some(Msg::SaveEdit))].spacing(8.0),
        ]
        .spacing(12.0)
        .into()
    } else {
        let mut c = Column::new().spacing(12.0);
        if !i.user.is_empty() {
            c = c.push(fl(&p, "User name", row![t(i.user.clone(), 15.0, p.text).width(Length::Fill), w::icb(&p, "⧉", Some(Msg::Copy("user")))].align_y(iced::Center).into(), None));
        }
        if !i.password.is_empty() {
            let shown: Element<'_, Msg> = if a.show_pw { t(i.password.clone(), 15.0, p.text).font(iced::Font::MONOSPACE).width(Length::Fill).into() } else { t("●".repeat(i.password.chars().count().min(24)), 15.0, p.text).width(Length::Fill).into() };
            c = c.push(fl(&p, "Password", row![shown, w::icb(&p, if a.show_pw { "◌" } else { "◉" }, Some(Msg::ShowPw)), w::icb(&p, "⧉", Some(Msg::Copy("password")))].spacing(10.0).align_y(iced::Center).into(), Some(strength_bar(&p, &i.password, 140.0))));
        }
        if let Some((code, left, of)) = vault::totp_now(&i.totp, a.now as u64) {
            let spaced = if code.len() == 6 { format!("{} {}", &code[..3], &code[3..]) } else { code };
            c = c.push(fl(
                &p,
                "2-step code",
                row![
                    tb(spaced, 24.0, p.accent).width(Length::Fill),
                    canvas(w::Ring { frac: left as f32 / of as f32, color: p.accent, track: p.line, width: 3.0 }).width(Length::Fixed(30.0)).height(Length::Fixed(30.0)),
                    t(format!("{left} s"), 12.0, p.muted),
                    w::icb(&p, "⧉", Some(Msg::Copy("code"))),
                ]
                .spacing(10.0)
                .align_y(iced::Center)
                .into(),
                None,
            ));
        }
        if !i.uri.is_empty() {
            c = c.push(fl(&p, "Website", row![t(i.uri.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string(), 15.0, p.text).width(Length::Fill), w::icb(&p, "↗", Some(Msg::OpenSite))].align_y(iced::Center).into(), None));
        }
        if i.kind() == Kind::Card {
            c = c.push(fl(&p, "Card", t(format!("ends {}", i.card_last4), 15.0, p.text).into(), None));
        }
        c.into()
    };

    let notes = container(column![tb("NOTES", 11.0, p.muted), t(if i.notes.is_empty() { "No notes.".to_string() } else { i.notes.clone() }, 12.0, p.text)].spacing(6.0)).padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 }).width(Length::Fill).height(Length::Fixed(130.0)).style(ui::boxed(p.bg, p.r(10.0), Some((p.line, 1.0))));
    let filling = w::card(
        &p,
        column![
            w::h2(&p, "Filling it in"),
            ui::gap(1.0, 6.0),
            w::kv(&p, "Fill in by itself in browsers", w::sw(&p, a.prefs.b("fillBrowsers", false), Some(Msg::Fill("fillBrowsers")))),
            w::kv(&p, "Ask before filling", w::sw(&p, a.prefs.b("fillAsk", true), Some(Msg::Fill("fillAsk")))),
            w::kv(&p, "Also for apps (Hearth AI, Store)", w::sw(&p, a.prefs.b("fillApps", true), Some(Msg::Fill("fillApps")))),
        ],
    )
    .width(Length::Fill);
    let mut hist: Vec<String> = i.history.iter().map(|(when, what)| format!("{what} · {}", day_of(when))).collect();
    for (when, what) in a.events.get(&i.id).and_then(Value::as_array).cloned().unwrap_or_default().iter().filter_map(|e| Some((e.get("when")?.as_str()?.to_string(), e.get("what")?.as_str()?.to_string()))) {
        hist.push(format!("{what} · {}", day_of(&when)));
    }
    if !i.made.is_empty() {
        hist.push(format!("Made · {}", day_of(&i.made)));
    }
    let history = w::card(&p, column![w::h2(&p, "History"), t(hist.join("\n"), 12.0, p.muted).line_height(1.9)].spacing(8.0)).width(Length::Fill);
    let grid = row![container(fields).width(Length::FillPortion(1)), column![notes, filling, history].spacing(12.0).width(Length::FillPortion(1))].spacing(16.0);

    // where your vault is
    let base = a.base();
    let host = hearth_oss::http::split(&base).map(|x| x.0).unwrap_or_default();
    let how = if host.starts_with("100.") || host.ends_with(".ts.net") { "reached over Tailscale" } else if host == "127.0.0.1" || host == "localhost" { "on this PC" } else { "on your network" };
    let laptop = std::fs::read_to_string("/sys/class/dmi/id/chassis_type").ok().and_then(|s| s.trim().parse::<u32>().ok()).map(|c| matches!(c, 8 | 9 | 10 | 14 | 31 | 32)).unwrap_or(false);
    let others: Vec<String> = a.devices.iter().filter(|d| hearth_oss::ci_str(d, &["identifier", "Identifier"]) != a.device).map(|d| hearth_oss::ci_str(d, &["name", "Name"])).filter(|n| !n.is_empty()).collect();
    let others_t = match others.len() {
        0 => "Your other devices".to_string(),
        1 => others[0].clone(),
        _ => format!("{} and {}", others[..others.len() - 1].join(", "), others[others.len() - 1]),
    };
    let phones = a.devices.iter().any(|d| matches!(hearth_oss::ci(d, &["type", "Type"]).and_then(Value::as_u64), Some(0) | Some(1)));
    let where_ = w::card(
        &p,
        column![
            row![w::h2(&p, "Where your vault is"), Space::new().width(Length::Fill), w::chip(&p, "Back up now", false, Sz::Sm, Some(Msg::Backup))].align_y(iced::Center),
            row![
                place(&p, w::green(), "Your server".into(), format!("{} · {how} · the main copy", a.prefs.s("serverName", "Nvidia PC"))),
                place(&p, w::blue(), if laptop { "This laptop".into() } else { "This PC".into() }, "Locked copy, works with no internet".into()),
                place(&p, p.accent, others_t, if others.is_empty() { "Sign in on them with the same server".into() } else if phones { "Synced · open with fingerprint or PIN".into() } else { "Synced".into() }),
            ]
            .spacing(12.0),
        ]
        .spacing(12.0),
    )
    .width(Length::Fill);

    // the security check
    let checks = vault::check(&a.items);
    let tod = hearth_ui::time::local(a.now).hour;
    let when = if !(5..18).contains(&tod) { "checked tonight" } else if tod < 12 { "checked this morning" } else { "checked this afternoon" };
    let mut cards = Row::new().spacing(12.0);
    for c in checks.iter().take(4) {
        let col = if c.0 == "Add a 2-step code" { w::blue() } else { w::yellow() };
        cards = cards.push(w::card2(&p, column![row![t("●", 12.0, col), tb(c.0, 12.0, p.text)].spacing(5.0), t(c.1.clone(), 12.0, p.muted)].spacing(6.0)).width(Length::FillPortion(1)));
    }
    let security = w::card(
        &p,
        column![
            row![w::h2(&p, "Security check"), t(format!("{} {} worth fixing · {when}", checks.len(), if checks.len() == 1 { "thing" } else { "things" }), 12.0, p.muted), Space::new().width(Length::Fill), w::chip(&p, "Fix them one by one", true, Sz::Sm, (!checks.is_empty()).then_some(Msg::Fix))].spacing(10.0).align_y(iced::Center),
            cards,
        ]
        .spacing(12.0),
    )
    .width(Length::Fill);
    let mut said = Column::new();
    if !a.said.is_empty() {
        said = said.push(t(a.said.clone(), 12.0, p.muted));
    }
    let page = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "item",
        vec![("head", head.into()), ("fields", grid.into()), ("where", where_.into()), ("security", security.into()), ("said", said.into())],
        |ps| column![ps.take("head"), ui::gap(1.0, 22.0), ps.take("fields"), ui::gap(1.0, 6.0), ps.take("where"), ui::gap(1.0, 16.0), ps.take("security"), ui::gap(1.0, 8.0), ps.take("said")].into(),
        Msg::Describe,
    );
    scrollable(container(page).padding(Padding { top: 22.0, right: 28.0, bottom: 22.0, left: 28.0 })).height(Length::Fill).into()
}

/// The password maker (the board's floating card).
fn maker_card(a: &App) -> Option<Element<'_, Msg>> {
    let p = a.p;
    let (m, s) = a.maker.as_ref()?;
    let mut gen = Row::new();
    for ch in s.chars() {
        let c = if ch.is_ascii_digit() { w::blue() } else if ch.is_ascii_alphanumeric() { p.text } else { p.accent };
        gen = gen.push(t(ch.to_string(), 20.0, c).font(iced::Font::MONOSPACE));
    }
    let name = a.item().map(|i| i.name.clone()).unwrap_or_default();
    let how = |v: &'static str, l: &str| (l.to_string(), m.how == v, Some(Msg::MakerHow(v)));
    let sw = |k: &'static str, on: bool| w::sw(&p, on, Some(Msg::MakerSet(k)));
    let body = column![
        row![w::swatch(p.accent, 16.0, 16.0, 4.0), tb("Make a password", 14.0, p.text), Space::new().width(Length::Fill), mouse_area(t("✕", 13.0, p.muted)).on_press(Msg::CloseMaker).interaction(mouse::Interaction::Pointer)].spacing(10.0).align_y(iced::Center),
        container(gen).padding(16.0).width(Length::Fill).style(ui::boxed(p.bg, p.r(10.0), Some((p.line, 1.0)))),
        row![strength_bar(&p, s, 160.0), Space::new().width(Length::Fill), w::chip(&p, "↻ Again", false, Sz::Sm, Some(Msg::Again))].align_y(iced::Center),
        w::seg(&p, vec![how("password", "Password"), how("words", "Words"), how("pin", "PIN")]),
        w::kv(&p, "Length", row![slider(8.0..=64.0, m.length as f32, Msg::MakerLen).step(1.0f32).on_release(Msg::MakerSet("len")).width(Length::Fixed(260.0)).style(w::slider_style(&p, 1.0)), container(t(m.length.to_string(), 12.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right)].spacing(8.0).align_y(iced::Center).into()),
        w::kv(&p, "A–Z", sw("upper", m.upper)),
        w::kv(&p, "a–z", sw("lower", m.lower)),
        w::kv(&p, "0–9", sw("digits", m.digits)),
        w::kv(&p, "! # % & *", sw("symbols", m.symbols)),
        w::kv(&p, "Leave out look-alikes (l, 1, O, 0)", sw("look", m.no_lookalikes)),
        row![w::chip(&p, "Copy", false, Sz::Md, Some(Msg::Copy("maker"))), Space::new().width(Length::Fill), w::chip(&p, "Cancel", false, Sz::Md, Some(Msg::CloseMaker)), w::chip(&p, if name.is_empty() { "Use it".to_string() } else { format!("Use it for {name}") }, true, Sz::Md, (!name.is_empty()).then_some(Msg::UseIt))].spacing(8.0).align_y(iced::Center),
    ]
    .spacing(12.0);
    Some(container(body).padding(Padding { top: 16.0, right: 20.0, bottom: 18.0, left: 20.0 }).width(Length::Fixed(560.0)).style(w::floating(&p, Pal::mix(p.line, p.accent, 0.35))).into())
}

fn lock_screen(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let srv = a.srv.get("vaultwarden").cloned().unwrap_or(Value::Null);
    let mut c = column![
        w::mark("PW", "or", 64.0),
        tb("Hearth Passwords", 26.0, p.text),
        t("Your vault, kept on your own server. Type your master password to open it: it never leaves this PC.", 13.0, p.muted),
        ui::gap(1.0, 6.0),
        w::input(&p, "Your email", &a.email, 14.0, Msg::Email),
        w::secret_input(&p, "Master password", &a.pw, Msg::Password, Msg::Unlock),
    ]
    .spacing(12.0)
    .align_x(iced::Center)
    .width(Length::Fixed(440.0));
    if a.need_code {
        c = c.push(w::input(&p, "2-step code from your authenticator", &a.code, 14.0, Msg::Code).on_submit(Msg::Unlock));
    }
    c = c.push(w::wide(&p, if a.busy { "Opening…" } else { "Unlock" }, (!a.busy && !a.pw.is_empty() && !a.email.trim().is_empty()).then_some(Msg::Unlock)));
    c = c.push(row![t("Your server", 12.0, p.muted), w::input(&p, "http://nvidia-pc:8222", &a.prefs.s("server", "http://127.0.0.1:8222"), 12.0, Msg::Server)].spacing(10.0).align_y(iced::Center));
    if srv.is_object() && !srv.get("installed").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(w::card2(&p, column![t("Vaultwarden isn't on this PC yet. Hearth installs its own image in your Podman, listening only here and on Tailscale.", 12.0, p.text), w::chip(&p, "Install Vaultwarden", true, Sz::Sm, Some(Msg::Install))].spacing(8.0)));
    } else if srv.is_object() && !srv.get("on").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(w::card2(&p, column![t("Vaultwarden is installed but off.", 12.0, p.text), w::chip(&p, "Turn it on", true, Sz::Sm, Some(Msg::TurnOn))].spacing(8.0)));
    }
    c = c.push(row![w::chip(&p, "Make an account on your server", false, Sz::Sm, Some(Msg::OpenServerPage)), w::chip(&p, "Open a KeePass file", false, Sz::Sm, Some(Msg::OpenKeepass))].spacing(8.0));
    if !a.kp_pending.is_empty() {
        c = c.push(row![t(ui::fit(&a.kp_pending, 40), 12.0, p.text), w::chip(&p, "Open it", true, Sz::Sm, (!a.pw.is_empty()).then_some(Msg::KpOpen))].spacing(8.0));
    }
    if !a.said.is_empty() {
        c = c.push(t(a.said.clone(), 12.0, p.muted));
    }
    container(c).center_x(Length::Fill).center_y(Length::Fill).into()
}

use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Passwords", blocks: &[bl("side", "Kinds, folders, health", 0, 0, 2, 11), bl("list", "The vault", 2, 0, 3, 11), bl("item", "The item", 5, 0, 7, 11), bl("credit", "Built on Vaultwarden and KeePassXC", 0, 11, 12, 1)] },
    PageDef { id: "side", title: "Kinds, folders, health", blocks: &[bl("kinds", "Everything, Favourites, Sign-ins…", 0, 0, 12, 5), bl("folders", "Folders", 0, 5, 12, 4), bl("health", "Health", 0, 9, 12, 3)] },
    PageDef { id: "list", title: "The vault", blocks: &[bl("search", "Search and ＋", 0, 0, 12, 1), bl("count", "How many, sort", 0, 1, 12, 1), bls("items", "The items", 0, 2, 12, 10)] },
    PageDef {
        id: "item",
        title: "The item",
        blocks: &[bl("head", "Its name and buttons", 0, 0, 12, 1), bl("fields", "Its fields, notes, filling it in, history", 0, 1, 12, 5), bl("where", "Where your vault is", 0, 6, 12, 2), bl("security", "Security check", 0, 8, 12, 3), bl("said", "What happened", 0, 11, 12, 1)],
    },
];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let base: Element<'_, Msg> = if !a.open_ok {
        lock_screen(a)
    } else {
        let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![
            ("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))),
            ("list", hearth_ui::studio::made(&a.describe, "window", "list", || list(a))),
            ("item", hearth_ui::studio::made(&a.describe, "window", "item", || detail(a))),
            ("credit", w::credit(&p, "Hearth Passwords", "Built on Vaultwarden (AGPL-3) and KeePassXC (GPL-3)")),
        ];
        for (id, which) in [("pane:codes", "codes"), ("pane:check", "check"), ("pane:maker", "maker")] {
            if a.describe.wants_pane(which) {
                items.push((id, studio_pane(a, which)));
            }
        }
        let body = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| column![row![ps.take("side"), ui::vline(p.line), ps.take("list"), ui::vline(p.line), ps.take("item")].height(Length::Fill), ps.take("credit")].into(), Msg::Describe);
        match maker_card(a) {
            Some(mk) => iced::widget::stack![body, container(mk).width(Length::Fill).align_x(iced::alignment::Horizontal::Right).padding(Padding { top: 42.0, right: 30.0, bottom: 0.0, left: 0.0 })].into(),
            None => body,
        }
    };
    let mins = (a.now - a.synced).max(0) / 60;
    let ago = if mins < 1 { "just now".to_string() } else if mins < 60 { format!("{mins} min ago") } else { format!("{} h ago", mins / 60) };
    let tag: Element<'_, Msg> = if !a.open_ok {
        w::tag(&p, "Locked", Tg::Dk)
    } else if a.offline {
        w::tag(&p, format!("The copy on this PC · synced {ago}"), Tg::Warn)
    } else {
        w::tag(&p, format!("Synced · {} · {ago}", a.prefs.s("serverName", "Nvidia PC")), Tg::Ok)
    };
    let middle: Element<'_, Msg> = container(row![t("· your vault, kept on your own server", 14.0, p.muted), Space::new().width(Length::Fill), tag, w::chip(&p, "Lock", false, Sz::Sm, a.open_ok.then_some(Msg::Lock))].spacing(8.0).align_y(iced::Center)).width(Length::FillPortion(30)).into();
    let bar = hearth_ui::describe::title_bar(&p, &a.describe, G::Shield, "Hearth Passwords", Some(middle), TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    let bar: Element<'_, Msg> = if a.describe.zone("titlebar").is_none() { iced::widget::stack![bar, container(w::mark("PW", "or", 22.0)).padding(Padding { top: 9.0, right: 0.0, bottom: 0.0, left: 11.0 })].into() } else { bar };
    let all = container(column![bar, ui::hline(p.line), base]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

use hearth_ui::studio::{tile as ktile, Kit};
const KIT: Kit = Kit {
    name: "Passwords",
    buttons: &[
        ktile("lock", "Lock", G::Shield),
        ktile("new", "Add a sign-in", G::Plus),
        ktile("maker", "Make a password", G::Asterisk),
        ktile("copy-user", "Copy the user name", G::User),
        ktile("copy-password", "Copy the password", G::Doc),
        ktile("copy-code", "Copy the 2-step code", G::Clock),
        ktile("open-site", "Open the website", G::Link),
        ktile("favourite", "Favourite", G::Check),
        ktile("backup", "Back up now", G::Down),
        ktile("fix", "Fix them one by one", G::Shield),
        ktile("leaks", "Check for leaks", G::Search),
        ktile("keepass", "Open a KeePass file", G::Folder),
    ],
    panes: &[ktile("codes", "Every 2-step code", G::Clock), ktile("check", "The security check", G::Shield), ktile("maker", "A new password", G::Asterisk)],
    info: &[ktile("items", "Items", G::List), ktile("weak", "Weak", G::Shield), ktile("twice", "Used twice", G::Sync), ktile("synced", "Synced", G::Server)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(_a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "lock" => Msg::Lock,
        "new" => Msg::New,
        "maker" => Msg::Maker,
        "copy-user" => Msg::Copy("user"),
        "copy-password" => Msg::Copy("password"),
        "copy-code" => Msg::Copy("code"),
        "open-site" => Msg::OpenSite,
        "favourite" => Msg::Favourite,
        "backup" => Msg::Backup,
        "fix" => Msg::Fix,
        "leaks" => Msg::Leaks,
        "keepass" => Msg::OpenKeepass,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let c = vault::check(&a.items);
    a.describe.values(vec![
        ("items", a.items.len().to_string()),
        ("weak", c.iter().filter(|x| x.0 == "Weak").count().to_string()),
        ("twice", c.iter().filter(|x| x.0 == "Used twice").count().to_string()),
        ("synced", if a.synced > 0 { hearth_ui::time::clock(a.synced) } else { String::new() }),
    ]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "codes" => {
            for i in a.items.iter().filter(|i| !i.totp.is_empty()) {
                if let Some((code, left, _)) = vault::totp_now(&i.totp, a.now as u64) {
                    c = c.push(mouse_area(row![t(i.name.clone(), 13.0, p.text), Space::new().width(Length::Fill), tb(code, 13.0, p.accent), t(format!("{left} s"), 11.0, p.muted)].spacing(8.0)).on_press(Msg::Pick(i.id.clone())).interaction(mouse::Interaction::Pointer));
                }
            }
        }
        "check" => {
            for x in vault::check(&a.items) {
                c = c.push(t(format!("{}: {}", x.0, x.1), 12.0, p.text));
            }
        }
        _ => {
            c = c.push(w::chip(&p, "Make a password", true, Sz::Sm, Some(Msg::Maker)));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    // the 2-step code's countdown: once a second, only while one is on the screen
    let codes = a.open_ok && (a.item().map(|i| !i.totp.is_empty()).unwrap_or(false) || a.describe.wants_pane("codes"));
    if codes {
        s.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-passwords-app", "Hearth Passwords: your vault on your own server (built on Vaultwarden, AGPL-3, and KeePassXC, GPL-3).") {
        return Ok(());
    }
    let w = app::window("hearth-passwords", (1840.0, 970.0), (1180.0, 640.0));
    iced::application(App::new, update, shown).title(|_: &App| "Hearth Passwords".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
