//! hearth-cloud-app: Hearth Cloud (docs/mockups/checklists/open-source-13-cloud.md; the approved board
//! docs/mockups/open-source-apps/13-cloud.png). Your own cloud on your Nvidia PC: Nextcloud (AGPL-3) from
//! hearth-oss-server (its own image in your Podman, 127.0.0.1 and Tailscale only), spoken to with WebDAV, CalDAV,
//! CardDAV and its OCS API (dav.rs): recent files, today's calendar, devices, contacts, what's shared, space, the bin.
//! Photos stay in the Home server helper; Hearth Cloud keeps files, calendar and contacts.
//!
//! Signing in is Nextcloud's own Login Flow v2: you approve this PC on Nextcloud's page, and the app password it hands
//! back is kept in your keyring (secret.rs), never in a file. Your choices: ~/.config/nimbus/cloud.json.
//! $HEARTH_CLOUD_FIXTURE: made-up answers (CI's pictures; nothing is sent).

mod dav;

use dav::{Acct, Contact, Entry, Event};
use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
struct Share {
    id: String,
    name: String,
    with: String,
    kind: u64,
    can_change: bool,
    expires: String,
    url: String,
    password: bool,
}

#[derive(Debug, Clone, Default)]
struct Device {
    name: String,
    what: String,
    when: String,
    shared: bool,
}

#[derive(Debug, Clone, Default)]
struct Data {
    recent: Vec<Entry>,
    events: Vec<Event>,
    contacts: Vec<Contact>,
    shares: Vec<Share>,
    devices: Vec<Device>,
    used: u64,
    total: u64,
    files: u64,
    bin: usize,
    folder: Vec<Entry>,
}

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Status(Value),
    Secret(String),
    Loaded(Result<Data, String>),
    Nav(&'static str),
    Search(String),
    Tab(&'static str),
    SignIn,
    LoginStarted(Result<(String, String, String), String>),
    Poll,
    LoggedIn(Option<(String, String)>),
    Server(String),
    Upload,
    Uploaded(Result<String, String>),
    NewFolder,
    FolderName(String),
    MakeFolder,
    AddDevice,
    NewEvent,
    EventTitle(String),
    EventTime(String),
    SaveEvent,
    Row(usize),
    Open(usize),
    Delete(usize),
    Link(usize),
    Manage(usize),
    CanChange(usize),
    Unshare(usize),
    CopyLink(usize),
    Done(Result<String, String>),
    Install,
    TurnOn,
    Ran(run::Output),
    Into(String),
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    prefs: Prefs,
    acct: Option<Acct>,
    status: Value,
    d: Data,
    loading: bool,
    problem: String,
    nav: &'static str,
    tab: &'static str,
    q: String,
    login: Option<(String, String)>,
    menu: Option<usize>,
    managing: Option<usize>,
    new_folder: Option<String>,
    new_event: Option<(String, String)>,
    adding_device: bool,
    said: String,
    path: String,
    fixture: bool,
}

fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-cloud".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

/// How far local time is from UTC, in seconds.
fn utc_offset() -> i64 {
    let now = hearth_oss::now();
    let t = hearth_ui::time::local(now);
    let local = dav::days_from_civil(t.year as i64, t.mon as i64 + 1, t.day as i64) * 86_400 + (t.hour * 3600 + t.min * 60 + t.sec) as i64;
    ((local - now) as f64 / 900.0).round() as i64 * 900
}
fn today_start() -> i64 {
    let off = utc_offset();
    (hearth_oss::now() + off).div_euclid(86_400) * 86_400 - off
}

/// Everything the Home page shows, read from your cloud.
fn load(a: Acct, path: String) -> Result<Data, String> {
    let mut d = Data::default();
    let files = a.files();
    d.recent = dav::multistatus(&a.dav("SEARCH", "/remote.php/dav/", "infinity", &dav::search_body(&a.user, 30, ""))?);
    if let Some(q) = dav::multistatus(&a.dav("PROPFIND", &format!("{files}/"), "0", dav::QUOTA)?).first() {
        d.used = q.used;
        d.total = if q.avail >= 0 { q.used + q.avail as u64 } else { 0 };
    }
    d.folder = dav::multistatus(&a.dav("PROPFIND", &format!("{files}/{}", hearth_oss::http::enc_path(&path)), "1", "")?).into_iter().skip(1).collect();
    // the week's events in every calendar (Monday to Sunday)
    let today = today_start();
    let wd = hearth_ui::time::local(hearth_oss::now()).wday;
    let monday = today - ((wd + 6) % 7) as i64 * 86_400;
    let cals = dav::multistatus(&a.dav("PROPFIND", &format!("/remote.php/dav/calendars/{}/", hearth_oss::http::enc(&a.user)), "1", dav::CALS).unwrap_or_default());
    let off = utc_offset();
    for c in cals.iter().skip(1).filter(|c| !c.href.contains("/inbox") && !c.href.contains("/outbox") && !c.href.contains("trashbin")) {
        if let Ok(x) = a.dav("REPORT", &c.href.replace("/remote.php", "/remote.php"), "1", &dav::calendar_query(monday - 7 * 86_400, monday + 7 * 86_400)) {
            for r in dav::multistatus(&x) {
                for mut e in dav::events(&r.data, off) {
                    e.color = c.color.clone();
                    e.calendar = c.name.clone();
                    d.events.push(e);
                }
            }
        }
    }
    d.events.sort_by_key(|e| (!e.all_day, e.start));
    if let Ok(x) = a.dav("REPORT", &format!("/remote.php/dav/addressbooks/users/{}/contacts/", hearth_oss::http::enc(&a.user)), "1", dav::CARD_QUERY) {
        d.contacts = dav::multistatus(&x).iter().flat_map(|r| dav::contacts(&r.data)).collect();
    }
    if let Ok(s) = a.ocs("/ocs/v2.php/apps/files_sharing/api/v1/shares") {
        d.shares = s.as_array().cloned().unwrap_or_default().iter().map(|x| {
            let g = |k: &str| x.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            Share { id: g("id"), name: g("file_target").trim_start_matches('/').to_string(), with: g("share_with_displayname"), kind: x.get("share_type").and_then(Value::as_u64).unwrap_or(0), can_change: x.get("permissions").and_then(Value::as_u64).unwrap_or(1) & 2 != 0, expires: g("expiration"), url: g("url"), password: x.get("password").map(|v| !v.is_null()).unwrap_or(false) }
        }).collect();
    }
    if let Ok(i) = a.ocs("/ocs/v2.php/apps/serverinfo/api/v1/info") {
        d.files = i.pointer("/nextcloud/storage/num_files").and_then(Value::as_u64).unwrap_or(0);
    }
    d.bin = dav::multistatus(&a.dav("PROPFIND", &format!("/remote.php/dav/trashbin/{}/trash", hearth_oss::http::enc(&a.user)), "1", "").unwrap_or_default()).len().saturating_sub(1);
    // this account's devices: the app passwords and sessions Nextcloud keeps (Settings › Security)
    if let Ok(t) = hearth_oss::http::send(&hearth_oss::http::Req::get(&format!("{}/index.php/settings/personal/authtokens", a.base)).basic(&a.user, &a.pass).header("OCS-APIRequest", "true").timeout(15)) {
        if let Some(arr) = t.json().and_then(|j| j.as_array().cloned()) {
            for x in arr {
                let name = x.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let last = x.get("lastActivity").and_then(Value::as_i64).unwrap_or(0);
                d.devices.push(Device { name, what: "files, calendar, contacts".into(), when: ago(last), shared: false });
            }
        }
    }
    for s in d.shares.iter().filter(|s| s.kind == 0) {
        d.devices.push(Device { name: format!("{}'s PC", s.with), what: format!("shared folder \"{}\"", s.name), when: String::new(), shared: true });
    }
    Ok(d)
}

fn ago(t: i64) -> String {
    let s = (hearth_oss::now() - t).max(0);
    if t <= 0 {
        String::new()
    } else if s < 90 {
        "just now".into()
    } else if s < 3600 {
        format!("{} min ago", s / 60)
    } else if s < 86_400 {
        format!("{} h ago", s / 3600)
    } else {
        hearth_ui::time::day(t)
    }
}

fn fixture() -> Option<Data> {
    let f = std::env::var("HEARTH_CLOUD_FIXTURE").ok()?;
    let j: Value = serde_json::from_str(&std::fs::read_to_string(f).ok()?).ok()?;
    let now = hearth_oss::now();
    let g = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let n = |v: &Value, k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
    let today = today_start();
    Some(Data {
        recent: j["recent"].as_array().cloned().unwrap_or_default().iter().map(|r| Entry { href: format!("/remote.php/dav/files/me/{}/{}", g(r, "where"), g(r, "name")), name: g(r, "name"), modified: now - n(r, "ago"), from: g(r, "from"), ..Entry::default() }).collect(),
        events: j["events"].as_array().cloned().unwrap_or_default().iter().map(|e| Event { title: g(e, "title"), start: today + n(e, "at"), end: today + n(e, "at") + n(e, "mins") * 60, all_day: e.get("allDay").and_then(Value::as_bool).unwrap_or(false), place: g(e, "place"), note: g(e, "note"), repeats: g(e, "repeats"), color: g(e, "color"), calendar: g(e, "calendar") }).collect(),
        contacts: j["contacts"].as_array().cloned().unwrap_or_default().iter().map(|c| Contact { name: g(c, "name"), phone: c.get("phone").and_then(Value::as_bool).unwrap_or(false), email: c.get("email").and_then(Value::as_bool).unwrap_or(false), address: c.get("address").and_then(Value::as_bool).unwrap_or(false), other: c.get("other").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(), groups: c.get("groups").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default() }).collect(),
        shares: j["shares"].as_array().cloned().unwrap_or_default().iter().map(|s| Share { id: g(s, "id"), name: g(s, "name"), with: g(s, "with"), kind: s.get("kind").and_then(Value::as_u64).unwrap_or(0), can_change: s.get("canChange").and_then(Value::as_bool).unwrap_or(false), expires: g(s, "expires"), url: g(s, "url"), password: true }).collect(),
        devices: j["devices"].as_array().cloned().unwrap_or_default().iter().map(|d| Device { name: g(d, "name"), what: g(d, "what"), when: g(d, "when"), shared: d.get("shared").and_then(Value::as_bool).unwrap_or(false) }).collect(),
        used: j["used"].as_u64().unwrap_or(0),
        total: j["total"].as_u64().unwrap_or(0),
        files: j["files"].as_u64().unwrap_or(0),
        bin: j["bin"].as_u64().unwrap_or(0) as usize,
        folder: Vec::new(),
    })
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = Prefs::load("cloud");
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("cloud", "Hearth Cloud").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            prefs,
            acct: None,
            status: Value::Null,
            d: Data::default(),
            loading: true,
            problem: String::new(),
            nav: "home",
            tab: "all",
            q: String::new(),
            login: None,
            menu: None,
            managing: None,
            new_folder: None,
            new_event: None,
            adding_device: false,
            said: String::new(),
            path: String::new(),
            fixture: false,
        };
        if let Some(d) = fixture() {
            a.fixture = true;
            a.d = d;
            a.loading = false;
            a.acct = Some(Acct { base: a.base(), user: "me".into(), pass: String::new() });
            a.status = json!({"nextcloud": {"installed": true, "on": true, "active": true}});
            return (a, Task::none());
        }
        let user = a.prefs.s("user", "");
        let mut t = vec![Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "status"])), |o| Msg::Status(o.json().unwrap_or(Value::Null)))];
        if !user.is_empty() {
            t.push(Task::perform(hearth_oss::secret::lookup("hearth-cloud", &user), Msg::Secret));
        } else {
            a.loading = false;
        }
        (a, Task::batch(t))
    }
    fn base(&self) -> String {
        self.prefs.s("server", "http://127.0.0.1:8090").trim_end_matches('/').to_string()
    }
    fn reload(&mut self) -> Task<Msg> {
        let Some(acct) = self.acct.clone() else { return Task::none() };
        if self.fixture {
            return Task::none();
        }
        self.loading = true;
        let path = self.path.clone();
        Task::perform(bg(move || load(acct, path)), |r| Msg::Loaded(r.unwrap_or_else(|| Err("it stopped".into()))))
    }
    fn set(&mut self, k: &str, v: Value) {
        if let Err(e) = self.prefs.set(k, v) {
            self.said = e;
        }
    }
    fn today(&self) -> Vec<&Event> {
        let t = today_start();
        let wd = hearth_ui::time::local(hearth_oss::now()).wday;
        self.d.events.iter().filter(|e| dav::on_day(e, t, wd)).collect()
    }
    fn recent(&self) -> Vec<&Entry> {
        let me = self.acct.as_ref().map(|a| a.user.clone()).unwrap_or_default();
        self.d
            .recent
            .iter()
            .filter(|e| match self.tab {
                "docs" => dav::folder_of(&e.href, &me).starts_with("Documents"),
                "projects" => dav::folder_of(&e.href, &me).starts_with("Projects"),
                _ => true,
            })
            .filter(|e| self.q.trim().is_empty() || e.name.to_lowercase().contains(&self.q.trim().to_lowercase()))
            .take(7)
            .collect()
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Status(v) => a.status = v,
        Msg::Secret(s) => {
            if s.is_empty() {
                a.loading = false;
                a.problem = "Sign in to your cloud (your keyring didn't have its app password).".into();
                return Task::none();
            }
            a.acct = Some(Acct { base: a.base(), user: a.prefs.s("user", ""), pass: s });
            return a.reload();
        }
        Msg::Loaded(r) => {
            a.loading = false;
            match r {
                Ok(d) => {
                    a.d = d;
                    a.problem.clear();
                }
                Err(e) => a.problem = e,
            }
        }
        Msg::Nav(n) => {
            a.nav = n;
            a.menu = None;
        }
        Msg::Search(q) => a.q = q,
        Msg::Tab(t) => a.tab = t,
        Msg::Server(s) => {
            a.prefs.m.insert("server".into(), json!(s.trim()));
        }
        Msg::SignIn => {
            let b = a.base();
            let saved = Prefs::load("cloud").s("server", "");
            if saved != b {
                a.set("server", json!(b));
            }
            return Task::perform(bg(move || dav::login_start(&b)), |r| Msg::LoginStarted(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::LoginStarted(r) => match r {
            Ok((url, token, endpoint)) => {
                run::detached(&["xdg-open", &url]);
                a.said = "Approve this PC on your cloud's own page (it just opened); Hearth Cloud waits for it.".into();
                a.login = Some((token, endpoint));
            }
            Err(e) => a.said = e,
        },
        Msg::Poll => {
            if let Some((tok, ep)) = a.login.clone() {
                return Task::perform(bg(move || dav::login_poll(&ep, &tok)), |r| Msg::LoggedIn(r.flatten()));
            }
        }
        Msg::LoggedIn(r) => {
            if let Some((user, pass)) = r {
                a.login = None;
                a.set("user", json!(user));
                a.acct = Some(Acct { base: a.base(), user: user.clone(), pass: pass.clone() });
                a.said = if hearth_oss::secret::available() { "Signed in. Its app password is kept in your keyring.".into() } else { "Signed in. (No keyring here: you'll sign in again next time.)".into() };
                let local = a.base().contains("127.0.0.1") || a.base().contains("localhost");
                let mut t = vec![Task::perform(hearth_oss::secret::store("hearth-cloud", &user, "Hearth Cloud", pass), |_| Msg::Done(Ok(String::new()))), a.reload()];
                if local {
                    // links that ask for a password and end by themselves: Nextcloud's own sharing rules
                    t.push(Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "nextcloud-links"])), |_| Msg::Done(Ok(String::new()))));
                }
                return Task::batch(t);
            }
        }
        Msg::Upload => {
            let Some(acct) = a.acct.clone() else { return Task::none() };
            let dir = a.path.clone();
            let device = a.prefs.s("deviceName", &hostname());
            return Task::perform(
                bg(move || -> Result<String, String> {
                    let o = run::blocking(&run::args(&["kdialog", "--getopenfilename", &hearth_ui::paths::home().to_string_lossy()]));
                    let f = o.out.trim().to_string();
                    if !o.ok || f.is_empty() {
                        return Err(String::new());
                    }
                    let name = std::path::Path::new(&f).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                    let bytes = std::fs::read(&f).map_err(|e| e.to_string())?;
                    let path = format!("{}/{}{}", acct.files(), if dir.is_empty() { String::new() } else { format!("{}/", hearth_oss::http::enc_path(&dir)) }, hearth_oss::http::enc(&name));
                    acct.send("PUT", &path, bytes, "application/octet-stream", &[])?;
                    // which Hearth device it came from (Hearth's own WebDAV property, shown in From)
                    let pp = format!(r#"<?xml version="1.0"?><d:propertyupdate xmlns:d="DAV:" xmlns:h="{}"><d:set><d:prop><h:device>{}</h:device></d:prop></d:set></d:propertyupdate>"#, dav::NS, dav::xml_escape(&device));
                    let _ = acct.send("PROPPATCH", &path, pp.into_bytes(), "application/xml", &[]);
                    Ok(format!("{name} is in your cloud."))
                }),
                |r| Msg::Uploaded(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Uploaded(r) => {
            match r {
                Ok(s) => a.said = s,
                Err(e) if !e.is_empty() => a.said = e,
                _ => {}
            }
            return a.reload();
        }
        Msg::NewFolder => a.new_folder = Some(String::new()),
        Msg::FolderName(s) => a.new_folder = Some(s),
        Msg::MakeFolder => {
            let (Some(acct), Some(name)) = (a.acct.clone(), a.new_folder.take()) else { return Task::none() };
            if name.trim().is_empty() {
                return Task::none();
            }
            let dir = a.path.clone();
            return Task::perform(
                bg(move || acct.send("MKCOL", &format!("{}/{}{}", acct.files(), if dir.is_empty() { String::new() } else { format!("{}/", hearth_oss::http::enc_path(&dir)) }, hearth_oss::http::enc(name.trim())), Vec::new(), "", &[]).map(|_| format!("The folder {} is made.", name.trim()))),
                |r| Msg::Uploaded(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::AddDevice => a.adding_device = !a.adding_device,
        Msg::NewEvent => a.new_event = Some((String::new(), "18:00".into())),
        Msg::EventTitle(s) => {
            if let Some(e) = a.new_event.as_mut() {
                e.0 = s;
            }
        }
        Msg::EventTime(s) => {
            if let Some(e) = a.new_event.as_mut() {
                e.1 = s;
            }
        }
        Msg::SaveEvent => {
            let (Some(acct), Some((title, time))) = (a.acct.clone(), a.new_event.take()) else { return Task::none() };
            let (h, mi) = time.split_once(':').map(|(h, m)| (h.trim().parse::<i64>().unwrap_or(18), m.trim().parse::<i64>().unwrap_or(0))).unwrap_or((18, 0));
            let start = today_start() + h * 3600 + mi * 60;
            let uid = format!("hearth-{}-{}", hearth_oss::now(), std::process::id());
            let ics = dav::new_event(&uid, &title, start, 60);
            let cal = a.prefs.s("calendar", "personal");
            return Task::perform(
                bg(move || acct.send("PUT", &format!("/remote.php/dav/calendars/{}/{}/{uid}.ics", hearth_oss::http::enc(&acct.user), cal), ics.into_bytes(), "text/calendar; charset=utf-8", &[]).map(|_| format!("\"{title}\" is on today's calendar."))),
                |r| Msg::Uploaded(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Row(i) => a.menu = if a.menu == Some(i) { None } else { Some(i) },
        Msg::Open(i) => {
            a.menu = None;
            let (Some(acct), Some(e)) = (a.acct.clone(), a.recent().get(i).map(|e| (*e).clone())) else { return Task::none() };
            return Task::perform(
                bg(move || -> Result<String, String> {
                    let r = hearth_oss::http::send(&hearth_oss::http::Req::get(&format!("{}{}", acct.base, hearth_oss::http::enc_path(&e.href))).basic(&acct.user, &acct.pass).timeout(120))?;
                    if !r.ok() {
                        return Err(format!("Your cloud answered {}", r.status));
                    }
                    let d = hearth_ui::paths::home().join("Downloads");
                    let _ = std::fs::create_dir_all(&d);
                    let f = d.join(&e.name);
                    std::fs::write(&f, &r.body).map_err(|x| x.to_string())?;
                    run::detached(&["xdg-open", &f.to_string_lossy()]);
                    Ok(format!("{} is in Downloads.", e.name))
                }),
                |r| Msg::Done(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Delete(i) => {
            a.menu = None;
            let (Some(acct), Some(e)) = (a.acct.clone(), a.recent().get(i).map(|e| (*e).clone())) else { return Task::none() };
            return Task::perform(bg(move || acct.send("DELETE", &hearth_oss::http::enc_path(&e.href), Vec::new(), "", &[]).map(|_| format!("{} is in the bin.", e.name))), |r| Msg::Uploaded(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::Link(i) => {
            a.menu = None;
            let (Some(acct), Some(e)) = (a.acct.clone(), a.recent().get(i).map(|e| (*e).clone())) else { return Task::none() };
            let me = acct.user.clone();
            return Task::perform(
                bg(move || -> Result<String, String> {
                    // a link that asks for a password and ends by itself (a week)
                    let mut pw = [0u8; 9];
                    let _ = std::fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut pw));
                    let pass: String = hearth_oss::b64::encode(&pw).chars().filter(|c| c.is_alphanumeric()).take(10).collect();
                    let path = format!("/{}/{}", dav::folder_of(&e.href, &me), e.name).replace("//", "/");
                    let (y, mo, d) = dav::civil((hearth_oss::now() + 7 * 86_400).div_euclid(86_400));
                    let ends = format!("{y:04}-{mo:02}-{d:02}");
                    let r = hearth_oss::http::send(&hearth_oss::http::Req::new("POST", &format!("{}/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json", acct.base)).basic(&acct.user, &acct.pass).header("OCS-APIRequest", "true").form(&[("path", path.as_str()), ("shareType", "3"), ("password", pass.as_str()), ("expireDate", ends.as_str())]).timeout(20))?;
                    let url = r.json().and_then(|j| j.pointer("/ocs/data/url").and_then(Value::as_str).map(str::to_string)).ok_or_else(|| format!("Your cloud said no ({})", r.status))?;
                    Ok(format!("{url}\npassword: {pass}"))
                }),
                |r| match r {
                    Some(Ok(s)) => Msg::Into(s),
                    Some(Err(e)) => Msg::Done(Err(e)),
                    None => Msg::Done(Err("it stopped".into())),
                },
            );
        }
        Msg::Into(s) => {
            a.said = "The link and its password are on the clipboard (it ends in a week).".into();
            return Task::batch([iced::clipboard::write(s), a.reload()]);
        }
        Msg::Manage(i) => a.managing = if a.managing == Some(i) { None } else { Some(i) },
        Msg::CanChange(i) => {
            let (Some(acct), Some(s)) = (a.acct.clone(), a.d.shares.get(i).cloned()) else { return Task::none() };
            let perm = if s.can_change { "1" } else { "31" };
            return Task::perform(
                bg(move || hearth_oss::http::send(&hearth_oss::http::Req::new("PUT", &format!("{}/ocs/v2.php/apps/files_sharing/api/v1/shares/{}?format=json", acct.base, s.id)).basic(&acct.user, &acct.pass).header("OCS-APIRequest", "true").form(&[("permissions", perm)]).timeout(20)).map(|_| "Changed.".to_string())),
                |r| Msg::Uploaded(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Unshare(i) => {
            let (Some(acct), Some(s)) = (a.acct.clone(), a.d.shares.get(i).cloned()) else { return Task::none() };
            a.managing = None;
            return Task::perform(
                bg(move || hearth_oss::http::send(&hearth_oss::http::Req::new("DELETE", &format!("{}/ocs/v2.php/apps/files_sharing/api/v1/shares/{}", acct.base, s.id)).basic(&acct.user, &acct.pass).header("OCS-APIRequest", "true").timeout(20)).map(|_| format!("{} isn't shared any more.", s.name))),
                |r| Msg::Uploaded(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::CopyLink(i) => {
            if let Some(s) = a.d.shares.get(i) {
                a.said = "The link is on the clipboard.".into();
                return iced::clipboard::write(s.url.clone());
            }
        }
        Msg::Done(r) => match r {
            Ok(s) if !s.is_empty() => a.said = s,
            Err(e) => a.said = e,
            _ => {}
        },
        Msg::Install => return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "install", "nextcloud"])), Msg::Ran),
        Msg::TurnOn => return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "on", "nextcloud"])), Msg::Ran),
        Msg::Ran(o) => {
            a.said = if o.ok { "Done. Finish Nextcloud's own first-time setup on its page (Open my cloud's page), then sign in here.".into() } else { format!("That didn't work: {}", o.why()) };
            return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-oss-server"), "status"])), |o| Msg::Status(o.json().unwrap_or(Value::Null)));
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).unwrap_or_else(|_| "This PC".into())
}

// ---------------------------------------------------------------- the window

fn side(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let today_n = a.today().len();
    let navs: Vec<(&'static str, &str, String)> = vec![
        ("home", "Home", String::new()),
        ("files", "Files", if a.d.files > 0 { group(a.d.files) } else { String::new() }),
        ("calendar", "Calendar", format!("{today_n} today")),
        ("contacts", "Contacts", a.d.contacts.len().to_string()),
        ("shared", "Shared with others", a.d.shares.len().to_string()),
        ("devices", "Devices", a.d.devices.len().to_string()),
        ("bin", "Bin", a.d.bin.to_string()),
    ];
    let mut c = Column::new().spacing(2.0);
    for (id, label, n) in navs {
        c = c.push(w::nav(&p, None, label, n, a.nav == id, Msg::Nav(id)));
    }
    let frac = if a.d.total > 0 { a.d.used as f32 / a.d.total as f32 } else { 0.0 };
    let space = column![
        w::cap(&p, "Space on the server"),
        w::meter(&p, frac, None, 0.0, 8.0),
        container(t(if a.d.total > 0 { format!("{} of {} used", hearth_oss::size_words(a.d.used), hearth_oss::size_words(a.d.total)) } else { format!("{} used", hearth_oss::size_words(a.d.used)) }, 12.0, p.muted)).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        container(t("Photos live in the Home server helper. Hearth Cloud keeps files, calendar and contacts.", 12.0, p.muted)).padding(Padding { top: 14.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    ];
    let outside = column![
        w::cap(&p, "From outside home"),
        {
            use iced::widget::text::Span;
            let spans: Vec<Span<'_, (), iced::Font>> = vec![iced::widget::span("Reached over Tailscale only · ").color(p.text).size(12.0), iced::widget::span("not open to the internet").color(p.muted).size(12.0)];
            Element::from(iced::widget::rich_text(spans))
        },
        ui::gap(1.0, 14.0),
        mouse_area(row![ui::glyph(G::Pencil, p.muted, 13.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
    ];
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "side",
        vec![("search", w::search(&p, "Search files, events, people", &a.q, Msg::Search)), ("places", c.into()), ("space", space.into()), ("outside", outside.into())],
        |ps| column![ps.take("search"), ui::gap(1.0, 12.0), ps.take("places"), ps.take("space"), ps.take("outside")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(250.0)).height(Length::Fill).padding(Padding { top: 14.0, right: 12.0, bottom: 14.0, left: 12.0 }).style(w::side_bg(&p)).into()
}

/// 48,210
fn group(n: u64) -> String {
    let s = n.to_string();
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            o.push(',');
        }
        o.push(c);
    }
    o
}

fn file_colour(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_lowercase().as_str() {
        "odt" | "doc" | "docx" => "bl",
        "3mf" | "stl" | "obj" => "pk",
        "ods" | "xls" | "xlsx" | "csv" => "gr",
        "png" | "jpg" | "jpeg" | "webp" => "or",
        "txt" => "yl",
        "pdf" => "rd",
        "md" => "pu",
        _ => "gy",
    }
}

fn hdr<'a>(p: &Pal, s: &str, wd: f32) -> Element<'a, Msg> {
    container(tb(s.to_uppercase(), 11.0, p.muted)).width(if wd > 0.0 { Length::Fixed(wd) } else { Length::Fill }).into()
}

fn home(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let me = a.acct.as_ref().map(|x| x.user.clone()).unwrap_or_default();
    let newest = a.d.recent.first();
    let sub = match newest {
        Some(e) => format!("Everything synced · last change {}{}", hearth_ui::time::clock(e.modified), if e.from.is_empty() { String::new() } else { format!(" from the {}", e.from.to_lowercase()) }),
        None if a.loading => "Reading your cloud…".into(),
        None => a.problem.clone(),
    };
    let mut head_btns = row![
        w::chip(&p, "＋ Upload", false, Sz::Md, a.acct.as_ref().map(|_| Msg::Upload)),
        w::chip(&p, "＋ New folder", false, Sz::Md, a.acct.as_ref().map(|_| Msg::NewFolder)),
        w::chip(&p, "Add a device", true, Sz::Md, Some(Msg::AddDevice)),
    ]
    .spacing(10.0);
    if let Some(n) = &a.new_folder {
        head_btns = head_btns.push(row![w::input(&p, "Folder name", n, 13.0, Msg::FolderName).on_submit(Msg::MakeFolder).width(Length::Fixed(180.0)), w::chip(&p, "Make it", true, Sz::Sm, Some(Msg::MakeFolder))].spacing(6.0).align_y(iced::Center));
    }
    let head = row![column![w::h1(&p, "Home"), t(sub, 12.0, p.muted)].spacing(4.0), Space::new().width(Length::Fill), head_btns].align_y(iced::Center);
    let mut device_help = Column::new();
    if a.adding_device {
        let url = a.base();
        device_help = device_help.push(w::card2(&p, column![tb("Add a device", 13.0, p.text), t(format!("On a phone: Nextcloud's own app (files), DAVx⁵ (calendar and contacts). On a Hearth device: Hearth Cloud. Sign in at {url} (over Tailscale) and approve it here."), 12.0, p.muted)].spacing(6.0)));
    }

    // recent files
    let tabs = w::tabs(&p, vec![("All".into(), a.tab == "all", Some(Msg::Tab("all"))), ("Documents".into(), a.tab == "docs", Some(Msg::Tab("docs"))), ("Projects".into(), a.tab == "projects", Some(Msg::Tab("projects")))]);
    let mut table = Column::new().push(container(row![hdr(&p, "Name", 0.0), hdr(&p, "Where", 220.0), hdr(&p, "Changed", 120.0), hdr(&p, "From", 120.0), Space::new().width(Length::Fixed(24.0))]).padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 })).push(ui::hline(p.line));
    for (i, e) in a.recent().into_iter().enumerate() {
        let when = if hearth_oss::now() - e.modified < 86_400 && hearth_ui::time::local(e.modified).day == hearth_ui::time::local(hearth_oss::now()).day {
            hearth_ui::time::clock(e.modified)
        } else if hearth_oss::now() - e.modified < 2 * 86_400 {
            "Yesterday".into()
        } else {
            hearth_ui::time::day(e.modified)
        };
        let mut r = Column::new().push(
            container(
                row![
                    container(row![w::swatch_g(file_colour(&e.name), 22.0, 18.0, 5.0), t(e.name.clone(), 13.0, p.text)].spacing(10.0).align_y(iced::Center)).width(Length::Fill),
                    container(t(dav::folder_of(&e.href, &me), 13.0, p.muted)).width(Length::Fixed(220.0)),
                    container(t(when, 13.0, p.muted)).width(Length::Fixed(120.0)),
                    container(t(e.from.clone(), 13.0, p.muted)).width(Length::Fixed(120.0)),
                    container(mouse_area(t("⋯", 13.0, p.muted)).on_press(Msg::Row(i)).interaction(mouse::Interaction::Pointer)).width(Length::Fixed(24.0)).align_x(iced::alignment::Horizontal::Right),
                ]
                .align_y(iced::Center),
            )
            .padding(Padding { top: 9.0, right: 10.0, bottom: 9.0, left: 10.0 }),
        );
        if a.menu == Some(i) {
            r = r.push(row![Space::new().width(Length::Fill), w::menu(&p, vec![("Open".into(), false, Msg::Open(i)), ("Share a link (a password, ends in a week)".into(), false, Msg::Link(i)), ("Delete (to the bin)".into(), false, Msg::Delete(i))], 300.0)]);
        }
        table = table.push(r).push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
    }
    let recent = w::card(&p, column![row![w::h2(&p, "Recent files"), Space::new().width(Length::Fill), tabs].align_y(iced::Center), table].spacing(8.0)).width(Length::FillPortion(145));

    // today
    let now = hearth_ui::time::local(hearth_oss::now());
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mut week = Row::new().spacing(4.0);
    let monday = today_start() - ((now.wday + 6) % 7) as i64 * 86_400;
    for k in 0..7 {
        let ds = monday + k as i64 * 86_400;
        let tm = hearth_ui::time::local(ds + 43_200);
        let is_today = tm.day == now.day && tm.mon == now.mon;
        let has = a.d.events.iter().any(|e| dav::on_day(e, ds, tm.wday)) && !is_today;
        let letter = ["S", "M", "T", "W", "T", "F", "S"][tm.wday];
        let cell = column![
            container(t(letter, 11.0, p.muted)).center_x(Length::Fill).height(Length::Fixed(26.0)).align_y(iced::alignment::Vertical::Center),
            container(if is_today { tb(tm.day.to_string(), 11.0, p.ink) } else { t(tm.day.to_string(), 11.0, p.text) }).center_x(Length::Fill).height(Length::Fixed(26.0)).align_y(iced::alignment::Vertical::Center).style(ui::boxed(if is_today { p.accent } else { Color::TRANSPARENT }, p.r(6.0), None)),
            container(Space::new()).width(Length::Fill).height(Length::Fixed(3.0)).style(ui::boxed(if has { w::blue() } else { Color::TRANSPARENT }, 2.0, None)),
        ]
        .spacing(4.0)
        .width(Length::FillPortion(1));
        week = week.push(cell);
    }
    let mut evs = Column::new().spacing(8.0);
    for e in a.today() {
        let c = if e.color.is_empty() { p.accent } else { w::hex(&e.color.chars().take(7).collect::<String>()) };
        let mut small = Vec::new();
        if !e.calendar.is_empty() && e.calendar != "Personal" {
            small.push(e.calendar.clone());
        }
        if !e.place.is_empty() {
            small.push(e.place.clone());
        }
        if !e.note.is_empty() {
            small.push(e.note.clone());
        }
        if !e.all_day && e.end > e.start + 3 * 3600 {
            small.push(format!("until {}", hearth_ui::time::clock(e.end)));
        }
        let rep = dav::repeats_words(&e.repeats);
        if !rep.is_empty() {
            small.push(rep);
        }
        let when = if e.all_day { "All day".to_string() } else { hearth_ui::time::clock(e.start) };
        let inner = row![container(t(when, 13.0, p.muted)).width(Length::Fixed(62.0)), column![t(e.title.clone(), 13.0, p.text), t(small.join(" · "), 11.0, p.muted)].spacing(3.0)].spacing(12.0);
        evs = evs.push(container(row![container(Space::new()).width(Length::Fixed(4.0)).height(Length::Fixed(44.0)).style(ui::boxed(c, 2.0, None)), container(inner).padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 }).width(Length::Fill)].align_y(iced::Center)).clip(true).style(ui::boxed(p.panel2, p.r(10.0), None)));
    }
    if a.today().is_empty() {
        evs = evs.push(t("Nothing on today.", 12.0, p.muted));
    }
    let mut ev_head = row![w::h2(&p, format!("Today · {} {} {}", DAYS[now.wday], hearth_ui::time::month(now.mon), now.day)), Space::new().width(Length::Fill), w::chip(&p, "＋ Event", false, Sz::Sm, a.acct.as_ref().map(|_| Msg::NewEvent))].align_y(iced::Center);
    if let Some((title, time)) = &a.new_event {
        ev_head = ev_head.push(row![w::input(&p, "What", title, 12.0, Msg::EventTitle).width(Length::Fixed(140.0)), w::input(&p, "18:00", time, 12.0, Msg::EventTime).width(Length::Fixed(64.0)).on_submit(Msg::SaveEvent), w::chip(&p, "Add", true, Sz::Sm, Some(Msg::SaveEvent))].spacing(6.0));
    }
    let today = w::card(&p, column![ev_head, container(week).padding(Padding { top: 12.0, right: 0.0, bottom: 14.0, left: 0.0 }), evs]).width(Length::FillPortion(100));

    // devices, contacts, shared
    let mut devs = Column::new().spacing(8.0);
    for (k, d) in a.d.devices.iter().enumerate() {
        let c = [p.accent, w::blue(), w::pink(), w::green()][k % 4];
        let sub = if d.when.is_empty() { d.what.clone() } else { format!("{} · {}", d.what, d.when) };
        devs = devs.push(w::card2(&p, row![w::swatch(c, 16.0, 16.0, 4.0), column![t(d.name.clone(), 13.0, p.text), t(sub, 11.0, p.muted)].spacing(3.0).width(Length::Fill), w::tag(&p, if d.shared { "Shared" } else { "Synced" }, if d.shared { Tg::Bl } else { Tg::Ok })].spacing(12.0).align_y(iced::Center)));
    }
    let devices = w::card(&p, column![row![w::h2(&p, "Devices"), Space::new().width(Length::Fill), t(if a.d.devices.is_empty() { "none yet" } else { "all up to date" }, 12.0, p.muted)].align_y(iced::Center), devs].spacing(10.0)).width(Length::FillPortion(1));
    let mut cts = Column::new();
    let cols = [Color::from_rgb8(0x3b, 0xa5, 0x5c), Color::from_rgb8(0xb0, 0x4f, 0xc8), Color::from_rgb8(0x2c, 0x7f, 0xd8), Color::from_rgb8(0xf0, 0xa0, 0x20), Color::from_rgb8(0xd8, 0x41, 0x3a)];
    let pinned = a.prefs.list("pinnedContacts");
    let mut shown: Vec<&Contact> = a.d.contacts.iter().filter(|c| pinned.contains(&c.name)).collect();
    if shown.is_empty() {
        shown = a.d.contacts.iter().take(5).collect();
    }
    for (k, c) in shown.iter().take(5).enumerate() {
        let av = container(tb(c.name.chars().next().unwrap_or('?').to_uppercase().to_string(), 12.0, Color::WHITE)).center_x(Length::Fixed(32.0)).center_y(Length::Fixed(32.0)).style(ui::boxed(cols[k % 5], 16.0, None));
        cts = cts.push(container(row![av, column![t(c.name.clone(), 13.0, p.text), t(c.sub(), 12.0, p.muted)].spacing(2.0)].spacing(10.0).align_y(iced::Center)).padding(Padding { top: 8.0, right: 0.0, bottom: 8.0, left: 0.0 }));
        if k + 1 < shown.len().min(5) {
            cts = cts.push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
        }
    }
    let contacts = w::card(&p, column![row![w::h2(&p, "Contacts"), Space::new().width(Length::Fill), t(format!("{} · on every device", a.d.contacts.len()), 12.0, p.muted)].align_y(iced::Center), cts].spacing(6.0)).width(Length::FillPortion(1));
    let mut sh = Column::new();
    for (k, s) in a.d.shares.iter().enumerate().take(6) {
        let sub = match s.kind {
            3 => format!("link · {}", if !s.expires.is_empty() { format!("ends {}", hearth_ui::time::parse_date(s.expires.get(..10).unwrap_or("")).map(|(_, m, d)| format!("{} {d}", hearth_ui::time::month(m))).unwrap_or_default()) } else if s.can_change { "can change".into() } else { "view only".into() }),
            _ => format!("with {}{}", s.with, if s.can_change { " · can change" } else { "" }),
        };
        let btn = if s.kind == 3 { w::chip(&p, "Copy link", false, Sz::Sm, Some(Msg::CopyLink(k))) } else { w::chip(&p, "Manage", false, Sz::Sm, Some(Msg::Manage(k))) };
        sh = sh.push(container(row![w::swatch_g(w::grad_for(&s.name), 30.0, 24.0, 5.0), column![t(s.name.clone(), 13.0, p.text), t(sub, 12.0, p.muted)].spacing(2.0).width(Length::Fill), btn].spacing(10.0).align_y(iced::Center)).padding(Padding { top: 8.0, right: 0.0, bottom: 8.0, left: 0.0 }));
        if a.managing == Some(k) {
            sh = sh.push(row![t("Can change", 12.0, p.text), w::sw(&p, s.can_change, Some(Msg::CanChange(k))), Space::new().width(Length::Fill), w::chip(&p, "Stop sharing", false, Sz::Sm, Some(Msg::Unshare(k)))].spacing(10.0).align_y(iced::Center));
        }
        sh = sh.push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
    }
    let shared = w::card(&p, column![w::h2(&p, "Shared with others"), sh, container(t("Links ask for a password and end by themselves.", 12.0, p.muted)).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 })].spacing(6.0)).width(Length::FillPortion(1));
    let mut said = Column::new();
    if !a.said.is_empty() {
        said = said.push(t(a.said.clone(), 12.0, p.muted));
    }
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "home",
        vec![("head", column![head, device_help].spacing(10.0).into()), ("recent", recent.into()), ("today", today.into()), ("devices", devices.into()), ("contacts", contacts.into()), ("shared", shared.into()), ("said", said.into())],
        |ps| column![ps.take("head"), ui::gap(1.0, 18.0), row![ps.take("recent"), ps.take("today")].spacing(16.0), ui::gap(1.0, 16.0), row![ps.take("devices"), ps.take("contacts"), ps.take("shared")].spacing(16.0), ui::gap(1.0, 10.0), ps.take("said")].into(),
        Msg::Describe,
    )
}

/// The other places in the side: each a plain list of what it has.
fn other_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(8.0);
    let title = match a.nav {
        "files" => "Files",
        "calendar" => "Calendar",
        "contacts" => "Contacts",
        "shared" => "Shared with others",
        "devices" => "Devices",
        _ => "Bin",
    };
    c = c.push(w::h1(&p, title));
    match a.nav {
        "files" => {
            for e in &a.d.folder {
                c = c.push(row![w::swatch_g(if e.dir { "yl" } else { file_colour(&e.name) }, 22.0, 18.0, 5.0), t(e.name.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(if e.dir { String::new() } else { hearth_oss::size_words(e.size) }, 12.0, p.muted)].spacing(10.0));
            }
        }
        "calendar" => {
            for e in &a.d.events {
                c = c.push(row![t(format!("{} {}", hearth_ui::time::day(e.start), if e.all_day { "all day".into() } else { hearth_ui::time::clock(e.start) }), 12.0, p.muted).width(Length::Fixed(140.0)), t(e.title.clone(), 13.0, p.text)].spacing(10.0));
            }
        }
        "contacts" => {
            for x in &a.d.contacts {
                c = c.push(row![t(x.name.clone(), 13.0, p.text).width(Length::Fixed(260.0)), t(x.sub(), 12.0, p.muted)].spacing(10.0));
            }
        }
        "shared" => {
            for s in &a.d.shares {
                c = c.push(row![t(s.name.clone(), 13.0, p.text).width(Length::Fixed(260.0)), t(if s.kind == 3 { "a link".to_string() } else { format!("with {}", s.with) }, 12.0, p.muted)].spacing(10.0));
            }
        }
        "devices" => {
            for d in &a.d.devices {
                c = c.push(row![t(d.name.clone(), 13.0, p.text).width(Length::Fixed(260.0)), t(d.what.clone(), 12.0, p.muted)].spacing(10.0));
            }
        }
        _ => c = c.push(t(format!("{} in the bin. Nextcloud empties it by itself after 30 days.", a.d.bin), 13.0, p.muted)),
    }
    scrollable(c).height(Length::Fill).into()
}

fn signin(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let nc = a.status.get("nextcloud").cloned().unwrap_or(Value::Null);
    let mut c = column![w::mark("CL", "bl", 64.0), tb("Hearth Cloud", 26.0, p.text), t("Your own cloud on your Nvidia PC: files, calendar and contacts on your phone and laptop.", 13.0, p.muted)].spacing(12.0).align_x(iced::Center).width(Length::Fixed(480.0));
    if nc.is_object() && !nc.get("installed").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(w::card2(&p, column![t("Nextcloud isn't on this PC yet. Hearth installs its own image in your Podman, reached only here and over Tailscale.", 12.0, p.text), w::chip(&p, "Install Nextcloud", true, Sz::Sm, Some(Msg::Install))].spacing(8.0)));
    } else if nc.is_object() && !nc.get("on").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(w::card2(&p, column![t("Nextcloud is installed but off.", 12.0, p.text), w::chip(&p, "Turn it on", true, Sz::Sm, Some(Msg::TurnOn))].spacing(8.0)));
    }
    c = c.push(row![t("Your cloud", 12.0, p.muted), w::input(&p, "http://nvidia-pc:8090", &a.prefs.s("server", "http://127.0.0.1:8090"), 12.0, Msg::Server)].spacing(10.0).align_y(iced::Center));
    c = c.push(w::wide(&p, if a.login.is_some() { "Waiting for you to approve it…" } else { "Sign in" }, a.login.is_none().then_some(Msg::SignIn)));
    if !a.said.is_empty() || !a.problem.is_empty() {
        c = c.push(t(if a.said.is_empty() { a.problem.clone() } else { a.said.clone() }, 12.0, p.muted));
    }
    container(c).center_x(Length::Fill).center_y(Length::Fill).into()
}

use hearth_ui::blocks::{bl, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Cloud", blocks: &[bl("side", "Places, space, from outside", 0, 0, 2, 11), bl("page", "The page", 2, 0, 10, 11), bl("credit", "Built on Nextcloud (AGPL-3)", 0, 11, 12, 1)] },
    PageDef { id: "side", title: "Places", blocks: &[bl("search", "Search files, events, people", 0, 0, 12, 1), bl("places", "Home, Files, Calendar…", 0, 1, 12, 6), bl("space", "Space on the server", 0, 7, 12, 3), bl("outside", "From outside home", 0, 10, 12, 2)] },
    PageDef {
        id: "home",
        title: "Home",
        blocks: &[bl("head", "Home and its buttons", 0, 0, 12, 1), bl("recent", "Recent files", 0, 1, 7, 5), bl("today", "Today", 7, 1, 5, 5), bl("devices", "Devices", 0, 6, 4, 5), bl("contacts", "Contacts", 4, 6, 4, 5), bl("shared", "Shared with others", 8, 6, 4, 5), bl("said", "What happened", 0, 11, 12, 1)],
    },
];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let signed = a.acct.is_some();
    let page: Element<'_, Msg> = if !signed && !a.loading { signin(a) } else if a.nav == "home" { home(a) } else { other_page(a) };
    let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![
        ("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))),
        ("page", container(page).padding(Padding { top: 22.0, right: 26.0, bottom: 18.0, left: 26.0 }).width(Length::Fill).height(Length::Fill).into()),
        ("credit", w::credit(&p, "Hearth Cloud", "Built on Nextcloud (AGPL-3)")),
    ];
    for (id, which) in [("pane:today", "today"), ("pane:recent", "recent"), ("pane:space", "space")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let base = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| column![row![ps.take("side"), ui::vline(p.line), ps.take("page")].height(Length::Fill), ps.take("credit")].into(), Msg::Describe);
    let nc = a.status.get("nextcloud").cloned().unwrap_or(Value::Null);
    let tag = if nc.get("active").and_then(Value::as_bool).unwrap_or(false) || (signed && a.problem.is_empty()) { w::tag(&p, "Server on · Nvidia PC", Tg::Ok) } else if nc.get("installed").and_then(Value::as_bool).unwrap_or(false) { w::tag(&p, "Server off", Tg::Warn) } else { w::tag(&p, "No server yet", Tg::Dk) };
    let middle: Element<'_, Msg> = container(row![t("· your own cloud, on your Nvidia PC", 14.0, p.muted), Space::new().width(Length::Fill), tag].spacing(8.0).align_y(iced::Center)).width(Length::FillPortion(30)).into();
    let bar = hearth_ui::describe::title_bar(&p, &a.describe, G::Server, "Hearth Cloud", Some(middle), TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    let bar: Element<'_, Msg> = if a.describe.zone("titlebar").is_none() { iced::widget::stack![bar, container(w::mark("CL", "bl", 22.0)).padding(Padding { top: 9.0, right: 0.0, bottom: 0.0, left: 11.0 })].into() } else { bar };
    let all = container(column![bar, ui::hline(p.line), base]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

use hearth_ui::studio::{tile as ktile, Kit};
const KIT: Kit = Kit {
    name: "Cloud",
    buttons: &[
        ktile("upload", "Upload", G::Plus),
        ktile("new-folder", "New folder", G::Folder),
        ktile("add-device", "Add a device", G::Display),
        ktile("new-event", "New event", G::Clock),
        ktile("home", "Home", G::Home),
        ktile("files", "Files", G::Folder),
        ktile("calendar", "Calendar", G::Clock),
        ktile("contacts", "Contacts", G::Users),
        ktile("shared", "Shared with others", G::Link),
        ktile("bin", "Bin", G::Close),
    ],
    panes: &[ktile("today", "Today's calendar", G::Clock), ktile("recent", "Recent files", G::Doc), ktile("space", "Space on the server", G::Drive)],
    info: &[ktile("used", "Space used", G::Drive), ktile("today", "Events today", G::Clock), ktile("contacts", "Contacts", G::Users), ktile("devices", "Devices", G::Display)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(_a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "upload" => Msg::Upload,
        "new-folder" => Msg::NewFolder,
        "add-device" => Msg::AddDevice,
        "new-event" => Msg::NewEvent,
        "home" => Msg::Nav("home"),
        "files" => Msg::Nav("files"),
        "calendar" => Msg::Nav("calendar"),
        "contacts" => Msg::Nav("contacts"),
        "shared" => Msg::Nav("shared"),
        "bin" => Msg::Nav("bin"),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    a.describe.values(vec![("used", hearth_oss::size_words(a.d.used)), ("today", a.today().len().to_string()), ("contacts", a.d.contacts.len().to_string()), ("devices", a.d.devices.len().to_string())]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "today" => {
            for e in a.today() {
                c = c.push(row![t(if e.all_day { "All day".into() } else { hearth_ui::time::clock(e.start) }, 12.0, p.muted).width(Length::Fixed(70.0)), t(e.title.clone(), 13.0, p.text)].spacing(8.0));
            }
        }
        "recent" => {
            for e in a.d.recent.iter().take(7) {
                c = c.push(t(e.name.clone(), 13.0, p.text));
            }
        }
        _ => {
            c = c.push(t(format!("{} of {}", hearth_oss::size_words(a.d.used), hearth_oss::size_words(a.d.total)), 13.0, p.text));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    if a.login.is_some() {
        s.push(iced::time::every(std::time::Duration::from_secs(3)).map(|_| Msg::Poll));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-cloud-app", "Hearth Cloud: your own cloud on your Nvidia PC (built on Nextcloud, AGPL-3).") {
        return Ok(());
    }
    let w = app::window("hearth-cloud", (1840.0, 880.0), (1180.0, 620.0));
    iced::application(App::new, update, shown).title(|_: &App| "Hearth Cloud".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
