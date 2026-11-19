//! Privacy & security: the category's page, Player count (SetAbout.qml's privacy part), Firewall (SetFirewall.qml),
//! Vaults (SetVaults.qml), Recent files (SetRecentFiles.qml), Firmware & device security and its history
//! (SetFirmware.qml, SetFirmwareHistory.qml) and Passwords (SetPasswords.qml), with Mantel's helpers: hearth-firewall
//! and hearth-admin firewall-* (pkexec), hearth-vaults, hearth-files recent-*, hearth-firmware and hearth-admin
//! firmware-*, hearth-secrets. A vault's password goes on the helper's stdin only. Opening a page only reads (no
//! timer reads again while it's open: a page is read again after each change, and when it's opened).

use crate::cats::pers::kit::{self, chip, iact, iinfo, ilink, indent, irow, irow_if, said, secret, small, Ic, Kind};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{t, G};
use iced::widget::Column;
use iced::{Element, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const PAGES: &[&str] = &["privacy", "player-count", "firewall", "vaults", "recent-files", "firmware", "firmware-history", "passwords"];

type E<'a> = Element<'a, Msg>;


#[derive(Default)]
pub struct State {
    page: &'static str,
    skip: usize,
    msg: String,
    bad: bool,
    f: HashMap<&'static str, String>,
    busy: &'static str,
    confirm: String,
    picked: String,
    // Firewall
    fw: Option<Value>,
    proto: String,
    from: String,
    // Vaults
    vaults: Option<Value>,
    asking: String,
    creating: bool,
    backend: String,
    idle: i64,
    lock_on_lock: Option<bool>,
    // Recent files
    recent: Option<Value>,
    confirm_all: bool,
    // Firmware
    fwst: Option<Value>,
    devs: Option<Value>,
    sec: Option<Value>,
    sec_open: bool,
    restart: bool,
    history: Option<Value>,
    history_err: String,
    // Passwords
    secrets: Option<Value>,
    items: Vec<Value>,
    locked: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(&'static str),
    F(&'static str, String),
    Did(run::Output),
    CountMe(bool),
    // Firewall
    Fw(run::Output),
    FwAct(Vec<String>, String, String, &'static str),
    FwDid(String, run::Output),
    Proto(String),
    From(String),
    Confirm(String),
    // Vaults
    Vaults(run::Output),
    Pick(String),
    Ask(String),
    Unlock(String, String),
    VaultDid(String, run::Output),
    VaultJob(Vec<String>, Value, String),
    Creating(bool),
    Backend(String),
    Idle(i64),
    LockOnLock(bool),
    Create,
    OpenFolder(String),
    // Recent files
    Recent(run::Output),
    RecentSet(Value, String),
    Forget(String, String),
    RecentDid(String, run::Output),
    ConfirmAll,
    Activity(String, String, String),
    LeaveFolder,
    // Firmware
    FwSt(run::Output),
    Devs(run::Output),
    Sec(run::Output),
    SecOpen,
    FwNotify(bool),
    Refresh,
    Update(String, String),
    FirmDid(&'static str, String, run::Output),
    Reboot,
    History(run::Output),
    // Passwords
    Secrets(run::Output),
    List(run::Output),
    Secret(Vec<String>, String),
    SecretDid(String, run::Output),
}

fn say(st: &mut State, s: impl Into<String>, bad: bool) {
    st.msg = s.into();
    st.bad = bad;
}
fn job(helper: &str, args: &[&str], input: String, f: impl FnOnce(run::Output) -> Msg + Send + 'static) -> Task<Msg> {
    Task::perform(kit::run_in(kit::job(helper, args), input), f)
}
fn read_fw() -> Task<Msg> {
    job(&kit::lx("hearth-firewall"), &["status"], String::new(), Msg::Fw)
}
fn read_vaults() -> Task<Msg> {
    job("hearth-vaults", &["list"], String::new(), Msg::Vaults)
}
fn read_recent() -> Task<Msg> {
    job("hearth-files", &["recent-prefs"], String::new(), Msg::Recent)
}
fn read_firm() -> Task<Msg> {
    Task::batch([job("hearth-firmware", &["status"], String::new(), Msg::FwSt), job("hearth-firmware", &["devices"], String::new(), Msg::Devs)])
}
fn read_secrets() -> Task<Msg> {
    job("hearth-secrets", &["status"], String::new(), Msg::Secrets)
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.page = page;
    let part = std::mem::take(&mut sh.part);
    st.skip = kit::skip_of(&part);
    st.msg.clear();
    st.confirm.clear();
    st.picked.clear();
    st.f.clear();
    st.confirm_all = false;
    if st.proto.is_empty() {
        st.proto = "tcp".into();
        st.from = "local".into();
        st.backend = "gocryptfs".into();
        st.idle = 15;
    }
    match page {
        "firewall" => read_fw(),
        "vaults" => {
            // Opened at vaults:<id> (the bar's "Unlock…") or vaults:new.
            let a = kit::part_word(&part);
            st.creating = a == "new";
            if !a.is_empty() && a != "new" && a.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                st.picked = a.clone();
                st.asking = a;
            }
            read_vaults()
        }
        "recent-files" => read_recent(),
        "firmware" => Task::batch([read_firm(), job("hearth-firmware", &["security"], String::new(), Msg::Sec)]),
        "firmware-history" => job("hearth-firmware", &["history"], String::new(), Msg::History),
        "passwords" => read_secrets(),
        _ => Task::none(),
    }
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Go(id) => sh.go(id),
        Msg::F(k, v) => {
            st.f.insert(k, v);
        }
        Msg::Did(o) => sh.result("That setting couldn't be saved", &o),
        Msg::CountMe(on) => {
            say(st, if on { "You're in the player count" } else { "You're no longer counted" }, false);
            let cmd = sh.desktop_cmd(json!({"countMe": on}));
            return Task::perform(run::spawn(cmd), Msg::Did);
        }
        // ---------------------------------------------------------- Firewall
        Msg::Fw(o) => match kit::answer(&o) {
            Ok(v) => {
                let needs = kit::b_of(&v, "needsAdmin");
                st.fw = Some(v);
                // ufw only answers root: its rules come through hearth-admin (no prompt: Hearth's polkit rule).
                if needs {
                    return Task::perform(kit::run_in(kit::admin(&["firewall-status"]), String::new()), |o| Msg::FwDid(String::new(), o));
                }
            }
            Err(e) => say(st, e, true),
        },
        Msg::FwAct(args, input, ok, label) => {
            st.busy = label;
            st.msg.clear();
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return Task::perform(kit::run_in(kit::admin(&a), input), move |o| Msg::FwDid(ok.clone(), o));
        }
        Msg::FwDid(ok, o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(v) => {
                    if v.is_object() {
                        st.fw = Some(v);
                    }
                    st.confirm.clear();
                    if !ok.is_empty() {
                        say(st, ok, false);
                    }
                }
                Err(e) => say(st, e, true),
            }
        }
        Msg::Proto(v) => st.proto = v,
        Msg::From(v) => st.from = v,
        Msg::Confirm(id) => st.confirm = if st.confirm == id { String::new() } else { id },
        // ---------------------------------------------------------- Vaults
        Msg::Vaults(o) => match kit::answer(&o) {
            Ok(v) => st.vaults = Some(v),
            Err(e) => say(st, e, true),
        },
        Msg::Pick(id) => {
            st.picked = if st.picked == id { String::new() } else { id };
            st.confirm.clear();
            st.asking.clear();
        }
        Msg::Ask(id) => st.asking = id,
        Msg::Unlock(id, name) => {
            let pw = st.f.remove("vaultPw").unwrap_or_default();
            st.busy = "unlock";
            let mount = vault(st, &id).map(|v| kit::s_of(&v, "mount")).unwrap_or_default();
            return job("hearth-vaults", &["unlock", &id], json!({"password": pw}).to_string(), move |o| Msg::VaultDid(format!("{name} is unlocked: its files are in {mount}."), o));
        }
        Msg::VaultJob(args, input, ok) => {
            st.busy = "vault";
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return job("hearth-vaults", &a, if input.is_null() { String::new() } else { input.to_string() }, move |o| Msg::VaultDid(ok.clone(), o));
        }
        Msg::VaultDid(ok, o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(v) => {
                    let kept = kit::s_of(&v, "kept");
                    if v.get("vaults").is_some() {
                        st.vaults = Some(v);
                    }
                    st.asking.clear();
                    st.confirm.clear();
                    if ok.starts_with("Taken off") {
                        st.picked.clear();
                        say(st, format!("Taken off the list. Its files are still in {kept}."), false);
                    } else if ok.starts_with("made:") {
                        st.creating = false;
                        st.f.remove("vaultName");
                        say(st, format!("{} is ready and unlocked. Put your private files in it, then lock it.", &ok[5..]), false);
                    } else if !ok.is_empty() {
                        say(st, ok, false);
                    }
                }
                Err(e) => say(st, e, true),
            }
        }
        Msg::Creating(on) => {
            st.creating = on;
            if !on {
                st.f.remove("newPw");
                st.f.remove("newPw2");
            }
        }
        Msg::Backend(b) => {
            st.idle = if b == "cryfs" { 0 } else { 15 };
            st.backend = b;
        }
        Msg::Idle(n) => st.idle = n,
        Msg::LockOnLock(on) => st.lock_on_lock = Some(on),
        Msg::Create => {
            let (pw, pw2) = (st.f.remove("newPw").unwrap_or_default(), st.f.remove("newPw2").unwrap_or_default());
            if pw != pw2 {
                say(st, "The passwords don't match.", true);
                return Task::none();
            }
            let name = st.f.get("vaultName").cloned().unwrap_or_default().trim().to_string();
            st.busy = "create";
            let input = json!({"name": name, "backend": st.backend, "password": pw, "idle": st.idle, "lockOnLock": st.lock_on_lock.unwrap_or(true)});
            return job("hearth-vaults", &["create"], input.to_string(), move |o| Msg::VaultDid(format!("made:{name}"), o));
        }
        Msg::OpenFolder(p) => run::detached(&["xdg-open", &p]),
        // ---------------------------------------------------------- Recent files
        Msg::Recent(o) => match kit::answer(&o) {
            Ok(v) => st.recent = Some(v),
            Err(e) => say(st, e, true),
        },
        Msg::RecentSet(change, ok) => {
            let c = change.to_string();
            return job("hearth-files", &["recent-prefs-set", &c], String::new(), move |o| Msg::RecentDid(ok.clone(), o));
        }
        Msg::Forget(what, ok) => {
            st.confirm_all = false;
            return job("hearth-files", &["recent-forget", &what], String::new(), move |o| Msg::RecentDid(format!("{ok}|forget"), o));
        }
        Msg::RecentDid(ok, o) => {
            match kit::answer(&o) {
                Ok(r) => {
                    let n = kit::n_of(&r, "forgot", 0.0) as i64;
                    let msg = if let Some(base) = ok.strip_suffix("|forget") { format!("{base} ({n})") } else if n > 0 { format!("{ok} · forgot {n} {}", if n == 1 { "file" } else { "files" }) } else { ok };
                    say(st, msg, false);
                }
                Err(e) => say(st, e, true),
            }
            return read_recent();
        }
        Msg::ConfirmAll => {
            if st.confirm_all {
                return update(sh, st, Msg::Forget("all".into(), "Forgot every recent file".into()));
            }
            st.confirm_all = true;
        }
        Msg::Activity(id, mode, name) => {
            // Mantel's activities (activities.json "act:<id>".recent, as Activities.setRecent).
            let path = hearth_ui::paths::home().join(".config/nimbus/activities.json");
            let mut act = match hearth_ui::settings_file::read_json(&path) {
                Value::Object(m) => m.get(&format!("act:{id}")).cloned().unwrap_or(json!({})),
                _ => json!({}),
            };
            if !act.is_object() {
                act = json!({});
            }
            act["recent"] = json!(mode);
            let cmd = sh.json_cmd(&path, json!({ format!("act:{id}"): act }));
            say(st, format!("{name}: {}", match mode.as_str() { "own" => "its own recent files", "all" => "every activity's recent files", _ => "files opened on it aren't remembered" }), false);
            let mut t = vec![Task::perform(run::spawn(cmd), Msg::Did)];
            if mode == "off" {
                t.push(job("hearth-files", &["activity-tidy"], String::new(), Msg::Did));
            }
            t.push(Task::perform(run::after(1500), |_| ()).then(|_| read_recent()));
            return Task::batch(t);
        }
        Msg::LeaveFolder => {
            let f = st.f.get("folder").cloned().unwrap_or_default().trim().trim_end_matches('/').to_string();
            if !f.starts_with('/') {
                say(st, "Type the folder's full path, starting with /", true);
                return Task::none();
            }
            st.f.remove("folder");
            let mut lo = left_out(st);
            lo.push(f.clone());
            return update(sh, st, Msg::RecentSet(json!({"leaveOut": lo}), format!("Files in “{f}” aren't remembered")));
        }
        // ---------------------------------------------------------- Firmware
        Msg::FwSt(o) => st.fwst = kit::answer(&o).ok().filter(|v| v.is_object()),
        Msg::Devs(o) => st.devs = Some(match kit::answer(&o) { Ok(v) if v.is_object() => v, Ok(_) => json!({"devices": []}), Err(e) => json!({"devices": [], "error": e}) }),
        Msg::Sec(o) => st.sec = kit::answer(&o).ok().filter(|v| v.is_object()),
        Msg::SecOpen => st.sec_open = !st.sec_open,
        Msg::FwNotify(on) => {
            let cmd = sh.desktop_cmd(json!({"firmwareNotify": on}));
            return Task::perform(run::spawn(cmd), Msg::Did);
        }
        Msg::Refresh => {
            st.busy = "refresh";
            st.msg.clear();
            return Task::perform(kit::run_in(kit::admin(&["firmware-refresh"]), String::new()), |o| Msg::FirmDid("refresh", "Firmware list is up to date.".into(), o));
        }
        Msg::Update(id, name) => {
            st.busy = "update";
            st.msg.clear();
            st.confirm.clear();
            return Task::perform(kit::run_in(kit::admin(&["firmware-update", &id]), String::new()), move |o| Msg::FirmDid("update", name.clone(), o));
        }
        Msg::FirmDid(what, ok, o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(r) => {
                    if what == "update" {
                        let rs = kit::b_of(&r, "restart");
                        st.restart = st.restart || rs;
                        say(st, format!("{ok} is updated{}.", if rs { ". It's put in when you restart" } else { "" }), false);
                    } else {
                        say(st, ok, false);
                    }
                }
                Err(e) => say(st, e, true),
            }
            return read_firm();
        }
        Msg::Reboot => run::detached(&["systemctl", "reboot"]),
        Msg::History(o) => match kit::answer(&o) {
            Ok(v) if v.is_object() => st.history = Some(v),
            Ok(_) => st.history = Some(json!({"history": []})),
            Err(e) => {
                st.history = Some(json!({"history": []}));
                st.history_err = e;
            }
        },
        // ---------------------------------------------------------- Passwords
        Msg::Secrets(o) => match kit::answer(&o) {
            Ok(v) => {
                let running = kit::b_of(&v, "running");
                st.secrets = Some(v);
                if running {
                    return job("hearth-secrets", &["list"], String::new(), Msg::List);
                }
            }
            Err(e) => say(st, e, true),
        },
        Msg::List(o) => match kit::answer(&o) {
            Ok(v) => {
                st.items = kit::arr(&v, "items").into_iter().cloned().collect();
                st.locked = kit::strs(&v, "lockedKeyrings");
            }
            Err(e) => say(st, e, true),
        },
        Msg::Secret(args, ok) => {
            st.confirm.clear();
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return Task::perform(kit::run_in(kit::job_t("hearth-secrets", &a, 300), String::new()), move |o| Msg::SecretDid(ok.clone(), o));
        }
        Msg::SecretDid(ok, o) => {
            match kit::answer(&o) {
                Ok(r) => say(st, if kit::b_of(&r, "prompted") { kit::s_of(&r, "message") } else { ok }, false),
                Err(e) => say(st, e, true),
            }
            return read_secrets();
        }
    }
    Task::none()
}

fn vault(st: &State, id: &str) -> Option<Value> {
    st.vaults.as_ref().and_then(|v| kit::arr(v, "vaults").into_iter().find(|x| kit::s_of(x, "id") == id).cloned())
}
fn left_out(st: &State) -> Vec<String> {
    st.recent.as_ref().and_then(|r| r.get("prefs")).map(|p| kit::strs(p, "leaveOut")).unwrap_or_default()
}

// ------------------------------------------------------------------ pages
pub fn view<'a>(a: &'a App, page: &'static str) -> Option<E<'a>> {
    let p = a.sh.p;
    let st = &a.privacy;
    let mut c = kit::Col::new(st.skip);
    match page {
        "privacy" => {
            c = c.push(t("What Hearth OS shares, what it keeps, and who can reach this PC.", sz(14.0), p.muted));
            let count_me = a.sh.d_bool("countMe", true);
            for (id, ic) in [("player-count", Ic::G(G::Users)), ("firewall", Ic::G(G::Shield)), ("vaults", Ic::Vault), ("recent-files", Ic::G(G::Clock)), ("firmware", Ic::Cpu), ("passwords", Ic::Key)] {
                if let Some(pg) = crate::registry::page(id) {
                    if id == "player-count" {
                        c = c.push(iced::widget::mouse_area(irow(&p, ic, pg.label, pg.desc, E::from(iced::widget::row![chip(&p, if count_me { "On" } else { "Off" }, if count_me { Kind::Ok } else { Kind::Off }), hearth_ui::ui::glyph(G::Chevron, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center)))).on_press(Msg::Go(pg.id)).interaction(iced::mouse::Interaction::Pointer));
                    } else {
                        c = c.push(ilink(&p, ic, pg.label, pg.desc, "", Msg::Go(pg.id)));
                    }
                }
            }
            c = c.push(head(&p, "Also about privacy"));
            c = c.push(ilink(&p, Ic::Menu, "Clipboard history", "What you copy is kept for Super+V. Turn it off or clear it", "", Msg::Go("clipboard")));
            c = c.push(ilink(&p, Ic::Lock, "Lock after sleep", "Ask for your password when this PC wakes · Accounts › Sign-in options", "", Msg::Go("signin")));
            c = c.push(ilink(&p, Ic::G(G::Server), "Home server safety", "Who can reach what, open ports, ransomware guard · Home server", "", Msg::Go("hs-safety")));
        }
        "player-count" => {
            c = c.push(head(&p, "Privacy"));
            let on = a.sh.d_bool("countMe", true);
            c = c.push(irow(&p, Ic::G(G::User), "Count me in the player count", "Hearth OS tells its counter it's running about every 10 minutes: a number made from this device, the edition and the version. No names, games or accounts.", switch(&p, on, Some(Msg::CountMe(!on)))));
        }
        "firewall" => c = firewall(a, c),
        "vaults" => c = vaults(a, c),
        "recent-files" => c = recent(a, c),
        "firmware" => c = firmware(a, c),
        "firmware-history" => c = history(a, c),
        "passwords" => c = passwords(a, c),
        _ => return None,
    }
    if !st.msg.is_empty() {
        c = c.push(said(&p, &st.msg, st.bad));
    }
    Some(c.into())
}

fn firewall<'a>(a: &'a App, mut c: kit::Col<'a, Msg>) -> kit::Col<'a, Msg> {
    let p = a.sh.p;
    let st = &a.privacy;
    c = c.push(head(&p, "Firewall"));
    let Some(fw) = st.fw.as_ref() else { return c.push(small(&p, "Reading the firewall…")) };
    let be = kit::s_of(fw, "backend");
    if be == "none" {
        return c.push(iinfo(&p, Ic::G(G::Info), "There's no firewall on this system", "Hearth OS doesn't ship one: your router already keeps the internet out. With firewalld or ufw installed, it's set here", None));
    }
    let name = if be == "firewalld" { "firewalld" } else if be == "ufw" { "ufw" } else { "" };
    let on = kit::b_of(fw, "on");
    let ssh = kit::b_of(fw, "ssh");
    let sub = if st.busy == "act" { "Changing…".to_string() } else if on { "On: only what's listed below can connect to this PC".into() } else { "Off: anything on your network can reach this PC's open ports".into() };
    let ok_msg = if on { "The firewall is off.".to_string() } else { format!("The firewall is on.{}", if ssh { " SSH stays open (this PC's SSH server runs)." } else { "" }) };
    c = c.push(irow(&p, Ic::G(G::Shield), format!("Firewall ({name})"), sub, switch(&p, on, Some(Msg::FwAct(vec![if on { "firewall-off".into() } else { "firewall-on".into() }], String::new(), ok_msg, "act")))));
    if !on {
        return c;
    }
    let inc = { let x = kit::s_of(fw, "incoming"); if x.is_empty() { "deny".to_string() } else { x } };
    let isub = if be == "firewalld" { format!("firewalld's zone: {}", { let z = kit::s_of(fw, "zone"); if z.is_empty() { "public".to_string() } else { z } }) } else { "What happens to a connection nothing below lets in".into() };
    c = c.push(irow(&p, Ic::G(G::Shield), "Other devices connecting in", isub, seg(&p, vec![
        ("Blocked unless listed".into(), inc == "deny", Some(Msg::FwAct(vec!["firewall-incoming".into(), "deny".into()], String::new(), "Only what's listed is let in.".into(), "act"))),
        ("All let in".into(), inc == "allow", Some(Msg::FwAct(vec!["firewall-incoming".into(), "allow".into()], String::new(), "Everything is let in now.".into(), "act"))),
    ])));
    let ts = kit::b_of(fw, "tailscale");
    c = c.push(iinfo(&p, Ic::G(G::Link), if ts { "Tailscale is trusted" } else { "Tailscale" }, if ts { "Your own devices reach this PC through Tailscale whatever the rules say" } else { "Turn the firewall off and on again to trust it" }, None));
    c = c.push(head(&p, "Let in"));
    let rules = kit::arr(fw, "rules");
    for r in &rules {
        let id = kit::s_of(r, "id");
        let label = kit::s_of(r, "label");
        let port = r.get("port").map(|x| x.as_str().map(String::from).unwrap_or_else(|| x.to_string())).unwrap_or_default();
        let proto = kit::s_of(r, "proto");
        let from = kit::s_of(r, "from");
        let hearth = kit::b_of(r, "hearth");
        let title = if label.is_empty() { format!("Port {port}") } else if !proto.is_empty() { format!("{label}  ·  {port}") } else { label.clone() };
        let sub = format!("{}{}{}", if proto.is_empty() { String::new() } else { format!("{} · ", proto.to_uppercase()) }, match from.as_str() { "local" => "From this home network".to_string(), "any" => "From anywhere".into(), f => format!("From {f}") }, if hearth { " · opened by Hearth while the app runs" } else { "" });
        c = c.push(iced::widget::mouse_area(irow(&p, if hearth { Ic::Sparkle } else { Ic::G(G::Link) }, title, sub, E::from(iced::widget::Space::new()))).on_press(Msg::Confirm(id.clone())).interaction(iced::mouse::Interaction::Pointer));
        if st.confirm == id {
            c = c.push(indent(iact(&p, Ic::Trash, "Stop letting this in", "", true, Some(Msg::FwAct(vec!["firewall-remove".into()], json!({"id": id}).to_string(), "Removed.".into(), "remove")))));
        }
    }
    if rules.is_empty() {
        c = c.push(small(&p, if kit::b_of(fw, "needsAdmin") { "Reading ufw's rules…" } else { "Nothing is let in. Nearby Share, Hearth Connect and Hearth Link open their own ports while they run." }));
    }
    c = c.push(head(&p, "Let a port in"));
    let port = st.f.get("port").cloned().unwrap_or_default();
    let label = st.f.get("label").cloned().unwrap_or_default();
    c = c.push(field(&p, "Port, like 25565 (a Minecraft server) or a range like 6000-6010", &port, |v| Msg::F("port", v)));
    c = c.push(field(&p, "What it's for (optional), like Minecraft", &label, |v| Msg::F("label", v)));
    c = c.push(irow(&p, Ic::G(G::Link), "Kind", "", seg(&p, [("TCP", "tcp"), ("UDP", "udp"), ("Both", "both")].iter().map(|(l, v)| (l.to_string(), st.proto == *v, Some(Msg::Proto(v.to_string())))).collect())));
    c = c.push(irow(&p, Ic::Globe, "From", "Anywhere matters only if your router forwards the port", seg(&p, [("This home network", "local"), ("Anywhere", "any")].iter().map(|(l, v)| (l.to_string(), st.from == *v, Some(Msg::From(v.to_string())))).collect())));
    let input = json!({"port": port.trim(), "proto": st.proto, "from": st.from, "label": label.trim()}).to_string();
    c = c.push(iact(&p, Ic::G(G::Plus), if st.busy == "allow" { "Letting it in…" } else { "Let it in" }, "Asks for an administrator's password", false, Some(Msg::FwAct(vec!["firewall-allow".into()], input, "Let in.".into(), "allow"))));
    c
}

fn idle_word(m: i64) -> String {
    if m == 0 { "Never by itself".into() } else if m < 60 { format!("{m} minutes unused") } else if m == 60 { "1 hour unused".into() } else { format!("{} hours unused", m / 60) }
}

fn vaults<'a>(a: &'a App, mut c: kit::Col<'a, Msg>) -> kit::Col<'a, Msg> {
    let p = a.sh.p;
    let st = &a.privacy;
    c = c.push(head(&p, "Your vaults"));
    let list: Vec<Value> = st.vaults.as_ref().map(|v| kit::arr(v, "vaults").into_iter().cloned().collect()).unwrap_or_default();
    for v in &list {
        let id = kit::s_of(v, "id");
        let name = kit::s_of(v, "name");
        let open = kit::b_of(v, "open");
        let mount = kit::s_of(v, "mount");
        let cry = kit::s_of(v, "backend") == "cryfs";
        let sub = format!("{} · {}", if open { format!("Unlocked · {mount}") } else { "Locked".into() }, if cry { "CryFS" } else { "gocryptfs" });
        c = c.push(iced::widget::mouse_area(irow(&p, Ic::Vault, name.clone(), sub, if open { chip(&p, "Open", Kind::Ok) } else { E::from(iced::widget::Space::new()) })).on_press(Msg::Pick(id.clone())).interaction(iced::mouse::Interaction::Pointer));
        if st.picked != id {
            continue;
        }
        let mut inner = Column::new().spacing(8.0);
        if !open && st.asking != id {
            inner = inner.push(iact(&p, Ic::Lock, "Unlock…", "", false, Some(Msg::Ask(id.clone()))));
        }
        if !open && st.asking == id {
            inner = inner.push(secret(&p, "The vault's password", st.f.get("vaultPw").map(String::as_str).unwrap_or(""), |s| Msg::F("vaultPw", s)).on_submit(Msg::Unlock(id.clone(), name.clone())));
            inner = inner.push(iact(&p, Ic::Lock, if st.busy == "unlock" { "Unlocking…" } else { "Unlock" }, "", false, Some(Msg::Unlock(id.clone(), name.clone()))));
        }
        if open {
            inner = inner.push(iact(&p, Ic::G(G::Folder), "Open in Files", "", false, Some(Msg::OpenFolder(mount.clone()))));
            inner = inner.push(iact(&p, Ic::Lock, if st.busy == "vault" { "Locking…" } else { "Lock now" }, "", false, Some(Msg::VaultJob(vec!["lock".into(), id.clone()], Value::Null, format!("{name} is locked.")))));
        }
        let idle = kit::n_of(v, "idle", 15.0) as i64;
        inner = inner.push(irow(&p, Ic::G(G::Clock), "Lock by itself", if cry { "CryFS doesn't check for open files: save your work before it's due (from the next unlock)" } else { "After nobody used its files for a while (from the next unlock)" }, seg(&p, [("5 min", 5i64), ("15 min", 15), ("1 hour", 60), ("Never", 0)].iter().map(|(l, n)| (l.to_string(), idle == *n, Some(Msg::VaultJob(vec!["set".into(), id.clone()], json!({"idle": n}), format!("Saved: {}.", idle_word(*n)))))).collect())));
        let lol = kit::b_of(v, "lockOnLock");
        inner = inner.push(irow(&p, Ic::Lock, "Lock it when the screen locks", "", switch(&p, lol, Some(Msg::VaultJob(vec!["set".into(), id.clone()], json!({"lockOnLock": !lol}), String::new())))));
        if !open && st.confirm != id {
            inner = inner.push(iact(&p, Ic::Trash, "Take it off the list…", "", true, Some(Msg::Confirm(id.clone()))));
        }
        if st.confirm == id {
            inner = inner.push(iact(&p, Ic::Trash, format!("Yes, take {name} off the list"), format!("Its encrypted files stay in {} (nothing is deleted)", kit::s_of(v, "data")), true, Some(Msg::VaultJob(vec!["forget".into(), id.clone()], Value::Null, "Taken off".into()))));
        }
        c = c.push(indent(inner));
    }
    if st.vaults.is_some() && list.is_empty() {
        c = c.push(small(&p, "No vaults yet. A vault is a folder only your password opens: for tax papers, passwords exported from a browser, anything private."));
    }
    if st.vaults.is_none() {
        c = c.push(small(&p, "Reading your vaults…"));
    }
    c = c.push(head(&p, "New vault"));
    let backends = st.vaults.as_ref().map(|v| kit::strs(v, "backends")).unwrap_or_default();
    let folder = st.vaults.as_ref().map(|v| kit::s_of(v, "folder")).filter(|f| !f.is_empty()).unwrap_or_else(|| "~/Vaults".into());
    if !st.creating {
        let none = st.vaults.is_some() && backends.is_empty();
        c = c.push(iact(&p, Ic::G(G::Plus), "Make a vault…", if none { "Needs gocryptfs or CryFS, which aren't on this system".to_string() } else { format!("It shows in {folder} while it's unlocked") }, false, Some(Msg::Creating(true))));
    } else {
        let g = |k: &str| st.f.get(k).cloned().unwrap_or_default();
        c = c.push(field(&p, "Its name, like Private", &g("vaultName"), |s| Msg::F("vaultName", s)));
        c = c.push(secret(&p, "A password (6 letters or more). It can't be brought back if you forget it", &g("newPw"), |s| Msg::F("newPw", s)));
        c = c.push(secret(&p, "The password again", &g("newPw2"), |s| Msg::F("newPw2", s)));
        let cry = st.backend == "cryfs";
        c = c.push(irow(&p, Ic::Vault, "Kind", if cry { "CryFS also hides how big your files are and how many there are" } else { "gocryptfs is fast and works well with backups" }, seg(&p, ["gocryptfs", "cryfs"].iter().map(|b| (if *b == "cryfs" { "CryFS".to_string() } else { "gocryptfs".to_string() }, st.backend == *b, if backends.iter().any(|x| x == b) { Some(Msg::Backend(b.to_string())) } else { None })).collect())));
        c = c.push(irow(&p, Ic::G(G::Clock), "Lock by itself", if cry { "Off for CryFS: its own auto-lock doesn't check for open files, so a file you're still editing could be cut off" } else { "After nobody used its files for a while" }, seg(&p, [("5 min", 5i64), ("15 min", 15), ("1 hour", 60), ("Never", 0)].iter().map(|(l, n)| (l.to_string(), st.idle == *n, Some(Msg::Idle(*n)))).collect())));
        let lol = st.lock_on_lock.unwrap_or(true);
        c = c.push(irow(&p, Ic::Lock, "Lock it when the screen locks", "", switch(&p, lol, Some(Msg::LockOnLock(!lol)))));
        c = c.push(iact(&p, Ic::G(G::Check), if st.busy == "create" { "Making it…" } else { "Make the vault" }, "", false, Some(Msg::Create)));
        c = c.push(iact(&p, Ic::G(G::Close), "Cancel", "", false, Some(Msg::Creating(false))));
    }
    c
}

fn recent<'a>(a: &'a App, mut c: kit::Col<'a, Msg>) -> kit::Col<'a, Msg> {
    let p = a.sh.p;
    let st = &a.privacy;
    c = c.push(t("Files you open show on Files' Home page, in the launcher's Recommended and in apps' jump lists. Kept on this PC only.", sz(13.0), p.muted));
    let prefs = st.recent.as_ref().and_then(|r| r.get("prefs")).cloned().unwrap_or(json!({"remember": true, "keepDays": 0, "leaveOut": []}));
    let remember = prefs.get("remember").and_then(Value::as_bool) != Some(false);
    let ready = st.recent.is_some();
    c = c.push(irow_if(&p, ready, Ic::G(G::Clock), "Remember the files I open", if remember { "Recent files show in Files, the launcher and jump lists" } else { "Off: nothing is kept, and what was kept is forgotten" }, switch(&p, remember, if ready { Some(Msg::RecentSet(json!({"remember": !remember}), if remember { "Recent files are off".into() } else { "Recent files are remembered".into() })) } else { None })));
    if remember {
        let kd = kit::n_of(&prefs, "keepDays", 0.0) as i64;
        let words = |n: i64| match n { 0 => "always", 1 => "for a day", 7 => "for a week", 30 => "for a month", 90 => "for 3 months", _ => "for a year" };
        c = c.push(irow(&p, Ic::Calendar, "Keep them for", "Older ones are forgotten", seg(&p, [("Always", 0i64), ("A day", 1), ("A week", 7), ("A month", 30), ("3 months", 90), ("A year", 365)].iter().map(|(l, n)| (l.to_string(), kd == *n, if ready { Some(Msg::RecentSet(json!({"keepDays": n}), format!("Kept {}", words(*n)))) } else { None })).collect())));
    }
    c = c.push(head(&p, "Forget"));
    c = c.push(iact(&p, Ic::Undo, "Forget the last hour", "", false, Some(Msg::Forget("hour".into(), "Forgot the last hour".into()))));
    c = c.push(iact(&p, Ic::Undo, "Forget today", "The last 24 hours", false, Some(Msg::Forget("day".into(), "Forgot the last 24 hours".into()))));
    c = c.push(iact(&p, Ic::Trash, if st.confirm_all { "Select again to forget every recent file" } else { "Forget all of them…" }, "Only the list: your files stay where they are", true, Some(Msg::ConfirmAll)));
    let acts: Vec<Value> = st.recent.as_ref().map(|r| kit::arr(r, "activities").into_iter().cloned().collect()).unwrap_or_default();
    if acts.len() > 1 {
        c = c.push(head(&p, "Activities"));
        c = c.push(small(&p, "A file belongs to the activity you were on when you opened it. Change these here or in the activity switcher (X)."));
        for ac in &acts {
            let id = kit::s_of(ac, "id");
            let name = kit::s_of(ac, "name");
            let files = kit::n_of(ac, "files", 0.0) as i64;
            let mode = { let m = kit::s_of(ac, "recent"); if m.is_empty() { "own".to_string() } else { m } };
            c = c.push(irow(&p, Ic::Desktops, name.clone(), format!("{files} {}", if files == 1 { "recent file" } else { "recent files" }), seg(&p, [("Its own list", "own"), ("Shared", "all"), ("Don't remember", "off")].iter().map(|(l, v)| (l.to_string(), mode == *v, Some(Msg::Activity(id.clone(), v.to_string(), name.clone())))).collect())));
            if files > 0 {
                c = c.push(iact(&p, Ic::Undo, format!("Forget {name}'s recent files"), "Only the list: your files stay where they are", false, Some(Msg::Forget(format!("activity:{id}"), format!("Forgot {name}'s recent files")))));
            }
        }
    }
    c = c.push(head(&p, "Left out"));
    c = c.push(small(&p, "Files in these folders, or opened with these apps, are never remembered."));
    let lo = left_out(st);
    for x in &lo {
        let rest: Vec<String> = lo.iter().filter(|y| *y != x).cloned().collect();
        c = c.push(iact(&p, if x.starts_with('/') { Ic::G(G::Folder) } else { Ic::G(G::Grid) }, x.clone(), "Select to remember it again", false, Some(Msg::RecentSet(json!({"leaveOut": rest}), format!("“{x}” is remembered again")))));
    }
    let apps: Vec<String> = st.recent.as_ref().map(|r| kit::strs(r, "apps")).unwrap_or_default().into_iter().filter(|a| !lo.contains(a)).take(12).collect();
    for ap in apps {
        let mut more = lo.clone();
        more.push(ap.clone());
        c = c.push(irow(&p, Ic::G(G::Grid), ap.clone(), "Remember what it opens", switch(&p, true, Some(Msg::RecentSet(json!({"leaveOut": more}), format!("Files opened with {ap} aren't remembered"))))));
    }
    c = c.push(field(&p, "Leave out a folder: type its full path (/home/you/Private)", st.f.get("folder").map(String::as_str).unwrap_or(""), |s| Msg::F("folder", s)).on_submit(Msg::LeaveFolder));
    c
}

fn firmware<'a>(a: &'a App, mut c: kit::Col<'a, Msg>) -> kit::Col<'a, Msg> {
    let p = a.sh.p;
    let st = &a.privacy;
    c = c.push(head(&p, "Device security"));
    let s = st.fwst.as_ref();
    let sb = s.map(|x| kit::s_of(x, "secureBoot")).unwrap_or_default();
    let sb_text = match (s, sb.as_str()) {
        (None, _) => "Checking…".to_string(),
        (_, "on") => "On: only signed start-up programs run".into(),
        (_, "off") => "Off. Hearth OS starts either way; turn it on in your PC's firmware setup (UEFI) if you like".into(),
        (Some(x), "unsupported") => if kit::b_of(x, "uefi") { "This PC's firmware doesn't have it".into() } else { "This PC starts the old way (BIOS), without Secure Boot".into() },
        _ => "Couldn't tell".into(),
    };
    c = c.push(iinfo(&p, Ic::Lock, "Secure Boot", sb_text, match sb.as_str() { "on" => Some(("On".into(), Kind::Ok)), "off" => Some(("Off".into(), Kind::Warn)), _ => None }));
    let tpm = s.and_then(|x| x.get("tpm")).cloned().unwrap_or(json!({}));
    let has = kit::b_of(&tpm, "present");
    c = c.push(iinfo(&p, Ic::Key, "Security chip (TPM)", if s.is_none() { "Checking…".to_string() } else if has { format!("TPM {} is there and working", kit::s_of(&tpm, "version")) } else { "None found (or it's off in the firmware setup)".into() }, s.map(|_| if has { ("Found".to_string(), Kind::Ok) } else { ("None".to_string(), Kind::Warn) })));
    if let Some(sec) = st.sec.as_ref() {
        let lv = kit::n_of(sec, "level", 0.0) as i64;
        c = c.push(ilink(&p, Ic::G(G::Shield), format!("Platform security: {}", kit::s_of(sec, "hsi")), format!("{} · fwupd's checks", if lv >= 2 { "Well protected against firmware attacks" } else if lv == 1 { "Basic protection" } else { "Some protections are off" }), "", Msg::SecOpen));
        if st.sec_open {
            let mut inner = Column::new().spacing(6.0);
            for it in kit::arr(sec, "items").into_iter().take(24) {
                let l = kit::n_of(it, "level", 0.0) as i64;
                inner = inner.push(iinfo(&p, if kit::b_of(it, "ok") { Ic::G(G::Check) } else { Ic::Warn }, kit::s_of(it, "name"), format!("{}{}", if l > 0 { format!("Level {l} · ") } else { String::new() }, kit::s_of(it, "result").replace('-', " ")), None));
            }
            c = c.push(indent(inner));
        }
    }
    c = c.push(head(&p, if st.busy == "update" { "Firmware updates · installing…" } else { "Firmware updates" }));
    let fwupd = s.map(|x| kit::b_of(x, "fwupd")).unwrap_or(false);
    if s.is_some() && !fwupd {
        c = c.push(iinfo(&p, Ic::G(G::Info), "Firmware updates aren't on this system", "fwupd is missing", None));
    }
    let devs: Vec<Value> = st.devs.as_ref().map(|d| kit::arr(d, "devices").into_iter().cloned().collect()).unwrap_or_default();
    let ups = devs.iter().filter(|d| d.get("update").map(|u| u.is_object()).unwrap_or(false)).count();
    if fwupd {
        let n = a.sh.d_bool("firmwareNotify", true);
        c = c.push(irow(&p, Ic::Bell, "Tell me about firmware updates", "A notification once a day at most when a part of this PC has new firmware", switch(&p, n, Some(Msg::FwNotify(!n)))));
        c = c.push(iact(&p, Ic::Sync, if st.busy == "refresh" { "Looking…" } else { "Check for firmware updates" }, if ups > 0 { format!("{ups} {} ready", if ups == 1 { "update" } else { "updates" }) } else { "From the Linux Vendor Firmware Service (LVFS), the way your PC's maker publishes them".into() }, false, Some(Msg::Refresh)));
    }
    for d in &devs {
        let id = kit::s_of(d, "id");
        let name = kit::s_of(d, "name");
        let up = d.get("update").filter(|u| u.is_object()).cloned();
        let system = kit::b_of(d, "system");
        let sub = format!("{}{}{}", { let v = kit::s_of(d, "vendor"); if v.is_empty() { v } else { format!("{v} · ") } }, { let v = kit::s_of(d, "version"); if v.is_empty() { v } else { format!("Version {v}") } }, match &up { Some(u) => format!(" → {}", kit::s_of(u, "version")), None if kit::b_of(d, "updatable") => " · up to date".into(), None => String::new() });
        let ch = up.as_ref().map(|_| chip(&p, if system { "Update (not from Hearth)" } else { "Update" }, Kind::Warn)).unwrap_or_else(|| E::from(iced::widget::Space::new()));
        c = c.push(iced::widget::mouse_area(irow(&p, if system { Ic::Cpu } else { Ic::Box }, name.clone(), sub, ch)).on_press(Msg::Confirm(format!("pick:{id}"))).interaction(iced::mouse::Interaction::Pointer));
        if st.confirm == format!("pick:{id}") || st.confirm == id {
            let mut inner = Column::new().spacing(8.0);
            if let Some(u) = &up {
                let desc = kit::s_of(u, "description");
                if !desc.is_empty() {
                    let sm = kit::s_of(u, "summary");
                    inner = inner.push(t(format!("{}{desc}", if sm.is_empty() { String::new() } else { format!("{sm}. ") }), sz(13.0), p.muted));
                }
                if system {
                    inner = inner.push(iinfo(&p, Ic::G(G::Info), "Not installed from Hearth", "System firmware changes how this PC starts. Back up first (Updates & recovery › Backup and help), then use your PC maker's update tool or its firmware setup", None));
                } else if kit::b_of(d, "installable") {
                    if st.confirm == id {
                        inner = inner.push(iact(&p, Ic::Warn, format!("Yes, update {name}"), "Asks for an administrator's password; nothing restarts by itself", false, Some(Msg::Update(id.clone(), name.clone()))));
                        inner = inner.push(iact(&p, Ic::G(G::Close), "Cancel", "", false, Some(Msg::Confirm(format!("pick:{id}")))));
                    } else {
                        inner = inner.push(iact(&p, Ic::G(G::Update), format!("Update to {}", kit::s_of(u, "version")), "Takes a minute; keep the device connected", false, Some(Msg::Confirm(id.clone()))));
                    }
                }
            }
            c = c.push(indent(inner));
        }
    }
    if let Some(d) = st.devs.as_ref() {
        if devs.is_empty() {
            let e = kit::s_of(d, "error");
            c = c.push(small(&p, if e.is_empty() { "fwupd found no parts it can update.".to_string() } else { e }));
        }
    }
    if fwupd {
        c = c.push(ilink(&p, Ic::G(G::Clock), "Update history", "Every firmware update put in, and whether it worked", "", Msg::Go("firmware-history")));
    }
    if st.restart {
        c = c.push(iact(&p, Ic::G(G::Restore), "Restart to finish", "The new firmware goes in while the PC restarts", false, Some(Msg::Reboot)));
    }
    c
}

fn history<'a>(a: &'a App, mut c: kit::Col<'a, Msg>) -> kit::Col<'a, Msg> {
    let p = a.sh.p;
    let st = &a.privacy;
    let list: Vec<Value> = st.history.as_ref().map(|h| kit::arr(h, "history").into_iter().cloned().collect()).unwrap_or_default();
    c = c.push(head(&p, if st.history.is_none() { "Reading…".to_string() } else if list.is_empty() { "No firmware updates yet".into() } else { format!("{} {}", list.len(), if list.len() == 1 { "update" } else { "updates" }) }));
    if st.history.is_some() && list.is_empty() {
        c = c.push(iinfo(&p, Ic::G(G::Info), if st.history_err.is_empty() { "Nothing has been updated on this PC" } else { "Couldn't read the history" }, if st.history_err.is_empty() { "Updates you put in from Firmware & device security show here".to_string() } else { st.history_err.clone() }, None));
    }
    for h in &list {
        let state = kit::s_of(h, "state");
        let (from, to) = (kit::s_of(h, "from"), kit::s_of(h, "to"));
        let title = format!("{}{}", kit::s_of(h, "name"), if to.is_empty() { String::new() } else { format!(" · {}{to}", if !from.is_empty() && from != to { format!("{from} → ") } else { String::new() }) });
        let when = kit::n_of(h, "when", 0.0) as i64;
        let detail = if state == "failed" && !kit::s_of(h, "error").is_empty() { kit::s_of(h, "error") } else { kit::s_of(h, "summary") };
        let sub = [kit::s_of(h, "vendor"), if when > 0 { hearth_ui::time::when(when) } else { String::new() }, detail].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
        let ic = match state.as_str() { "failed" => Ic::Warn, "done" => Ic::G(G::Check), "restart" => Ic::G(G::Restore), _ => Ic::G(G::Update) };
        let ch = match state.as_str() { "done" => Some(("Done".to_string(), Kind::Ok)), "failed" => Some(("Didn't work".to_string(), Kind::Bad)), "restart" => Some(("At restart".to_string(), Kind::Warn)), "pending" => Some(("Waiting".to_string(), Kind::Warn)), _ => None };
        c = c.push(iinfo(&p, ic, title, sub, ch));
    }
    c
}

fn passwords<'a>(a: &'a App, mut c: kit::Col<'a, Msg>) -> kit::Col<'a, Msg> {
    let p = a.sh.p;
    let st = &a.privacy;
    c = c.push(head(&p, "Password store"));
    let s = st.secrets.as_ref();
    let running = s.map(|x| kit::b_of(x, "running")).unwrap_or(false);
    let title = match s { None => "Checking…".to_string(), Some(x) if running => kit::s_of(x, "provider"), _ => "No password store is running".into() };
    let n = st.items.len();
    let sub = match s { None => String::new(), Some(_) if running => format!("Apps keep their sign-ins here (browsers, Discord, network shares, Hearth's stores). {n} {}", if n == 1 { "item saved." } else { "items saved." }), _ => "Apps that keep sign-ins in the system's password store can't save them until it runs. Signing out and in again starts it.".into() };
    let pam = s.and_then(|x| x.get("pam")).map(|x| kit::b_of(x, "auth")).unwrap_or(false);
    c = c.push(iinfo(&p, Ic::Key, title, sub, if running && pam { Some(("Unlocks when you sign in".into(), Kind::Ok)) } else { None }));
    let keyrings: Vec<Value> = s.map(|x| kit::arr(x, "keyrings").into_iter().filter(|k| !kit::b_of(k, "session")).cloned().collect()).unwrap_or_default();
    for k in &keyrings {
        let id = kit::s_of(k, "id");
        let locked = kit::b_of(k, "locked");
        let cnt = kit::n_of(k, "count", 0.0) as i64;
        let prot = k.get("protected").and_then(Value::as_bool);
        let label = { let l = kit::s_of(k, "label"); if l.is_empty() { "Keyring".to_string() } else { l } };
        let sub = format!("{} · {cnt} {}{}", if locked { "Locked" } else { "Unlocked" }, if cnt == 1 { "item" } else { "items" }, match prot { Some(false) => " · no password: kept unencrypted on this drive", Some(true) => " · protected with a password", None => "" });
        let right = iced::widget::row![if prot == Some(false) { chip(&p, "No password", Kind::Warn) } else { E::from(iced::widget::Space::new()) }, crate::widgets::btn(&p, if locked { "Unlock" } else { "Lock" }, hearth_ui::ui::Kind::Normal, false, Some(Msg::Secret(vec![if locked { "unlock".into() } else { "lock".into() }, id.clone()], if locked { "Unlocked.".into() } else { "Locked.".into() })))].spacing(10.0).align_y(iced::Center);
        c = c.push(irow(&p, if locked { Ic::Lock } else { Ic::Key }, format!("{label}{}", if kit::b_of(k, "default") { " (default)" } else { "" }), sub, right.into()));
    }
    let unprot: Vec<&Value> = keyrings.iter().filter(|k| k.get("protected").and_then(Value::as_bool) == Some(false)).collect();
    if let (Some(u), true) = (unprot.first(), s.map(|x| kit::b_of(x, "prompter")).unwrap_or(false)) {
        c = c.push(iact(&p, Ic::G(G::Shield), format!("Protect “{}” with a password…", kit::s_of(u, "label")), "Use your sign-in password so it unlocks when you sign in. With automatic sign-in, apps ask for it once after each start.", false, Some(Msg::Secret(vec!["protect".into(), kit::s_of(u, "id")], "Done.".into()))));
    }
    if running {
        c = c.push(head(&p, "Saved sign-ins"));
        let filter = st.f.get("filter").cloned().unwrap_or_default().to_lowercase();
        if n > 6 {
            c = c.push(field(&p, "Find a saved sign-in", st.f.get("filter").map(String::as_str).unwrap_or(""), |s| Msg::F("filter", s)));
        }
        if !st.locked.is_empty() {
            c = c.push(iinfo(&p, Ic::Lock, "Some are in a locked keyring", format!("Unlock {} above to see them", st.locked.join(", ")), None));
        }
        if n == 0 && st.locked.is_empty() {
            c = c.push(iinfo(&p, Ic::Key, "Nothing saved yet", "", None));
        }
        for it in st.items.iter().filter(|i| filter.is_empty() || format!("{} {} {} {}", kit::s_of(i, "label"), kit::s_of(i, "app"), kit::s_of(i, "account"), kit::s_of(i, "where")).to_lowercase().contains(&filter)) {
            let id = kit::s_of(it, "id");
            let created = kit::n_of(it, "created", 0.0) as i64;
            let sub = [kit::s_of(it, "app"), kit::s_of(it, "account"), kit::s_of(it, "where"), if created > 0 { format!("saved {}", hearth_ui::time::day(created / 1000)) } else { String::new() }, kit::s_of(it, "keyring")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
            let title = { let l = kit::s_of(it, "label"); if l.is_empty() { kit::s_of(it, "app") } else { l } };
            c = c.push(kit::ilink(&p, Ic::Key, title, sub, if st.confirm == id { "" } else { "Remove…" }, Msg::Confirm(id.clone())));
            if st.confirm == id {
                c = c.push(indent(iact(&p, Ic::Trash, "Remove it", "The app forgets this sign-in and asks again next time", true, Some(Msg::Secret(vec!["remove".into(), id.clone()], "Removed.".into())))));
            }
        }
    }
    c = c.push(small(&p, "Hearth never shows or copies a saved password here. Apps' own password managers (like your browser's) are in those apps."));
    c
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(E<'a>, Option<Msg>)> {
    None
}

pub fn back(st: &mut State) -> bool {
    if !st.confirm.is_empty() || !st.picked.is_empty() || st.creating {
        st.confirm.clear();
        st.picked.clear();
        st.creating = false;
        return true;
    }
    false
}

