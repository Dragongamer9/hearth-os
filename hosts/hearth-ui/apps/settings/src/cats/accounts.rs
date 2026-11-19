//! Accounts: the category's page, Your account and other people (SetUsers.qml), Sign-in options (SetSignin.qml with
//! LoginScreenSwitch.qml), Online accounts (SetAccounts.qml) and Login screen (SetLoginScreen.qml), with the same
//! helpers as Mantel's: hearth-system (accounts, pictures), hearth-admin through pkexec (password, user-add,
//! user-remove, user-type, oem-finish, autologin, fingerprint-places), hearth-fingerprint, hearth_accounts.py,
//! hearth-loginscreen and hearth-login-screen. What a helper needs (a password) goes to it on its stdin, never on a
//! command line. Fedora edition: "Ask for my password after sleep" is also KDE's own lock on resume
//! (kscreenlockerrc), as Plasma's screen locker reads it.

use crate::cats::pers::kit::{self, chip, dropdown, iact, iinfo, ilink, indent, irow, said, secret, small, Ic, Kind, Line};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, ts, G};
use crate::widgets::btn;
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Space};
use iced::{Element, Length, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;

pub const PAGES: &[&str] = &["accounts", "users", "signin", "online-accounts", "login-screen"];

type E<'a> = Element<'a, Msg>;

#[derive(Default)]
pub struct State {
    page: &'static str,
    skip: usize,
    msg: String,
    bad: bool,
    f: HashMap<&'static str, String>,
    busy: &'static str,
    // Users
    users: Option<Value>,
    pictures: Vec<String>,
    picking: bool,
    picked: String,
    confirm: String,
    new_admin: bool,
    // Sign-in options
    can_admin: bool,
    autologin: bool,
    host: String,
    ls: Option<Value>,
    ls_msg: (String, bool),
    fp: Option<Value>,
    fp_msg: (String, bool),
    fp_confirm: String,
    finger: String,
    enrolling: Option<(f32, String)>,
    fp_pid: Arc<AtomicI32>,
    // Online accounts
    accounts: Vec<Value>,
    prov: Value,
    loaded: bool,
    adding: String,
    flow_line: String,
    flow_login: String,
    flowing: bool,
    flow_what: &'static str,
    flow_result: Option<Value>,
    flow_err: String,
    flow_pid: Arc<AtomicI32>,
    open_acct: String,
    // Login screen
    lsc: Option<Value>,
    pick: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(&'static str),
    F(&'static str, String),
    Users(run::Output),
    Pictures(run::Output),
    TogglePictures,
    PicSet(String),
    PicDid(run::Output),
    Pick(String),
    Confirm(String),
    UserType(String, bool),
    Remove(String, bool),
    NewAdmin(bool),
    Add,
    OemFinish,
    Password,
    AdminDid(&'static str, String, run::Output),
    Info(run::Output),
    Autologin(bool),
    LockOnWake(bool),
    Did(run::Output),
    Ls(run::Output),
    LsSet(bool),
    LsDid(bool, run::Output),
    LsPreview,
    Fp(run::Output),
    Finger(String),
    Enroll,
    FpLine(Line),
    FpConfirm(String),
    FpDelete(String),
    FpDid(&'static str, run::Output),
    FpPlaces(&'static str, bool),
    FpPlacesDid(String, run::Output),
    Open(&'static str),
    Close,
    // Online accounts
    Accounts(run::Output),
    Prov(run::Output),
    Form(String),
    Flow(Vec<String>, &'static str),
    FlowLine(Line),
    Cancel,
    AddPw(&'static str, &'static str),
    Added(&'static str, run::Output),
    Use(String, &'static str, bool),
    RemoveAcct(String, String),
    Mount(String, String),
    Mounted(run::Output),
    AcctOpen(String),
    AcctDid(run::Output),
    // Login screen
    Lsc(run::Output),
    LscUse(String),
    LscDid(String, run::Output),
    LscChoose,
    LscPicked(run::Output),
    /// Pictures read (pics.rs): drawn again with them.
    Pics,
}

const SYS: &str = "hearth-system";
fn login_screen_path() -> String {
    kit::lx("hearth-login-screen")
}

fn get(st: &State, k: &str) -> String {
    st.f.get(k).cloned().unwrap_or_default()
}
fn clear(st: &mut State, keys: &[&'static str]) {
    for k in keys {
        st.f.remove(k);
    }
}
fn say(st: &mut State, s: impl Into<String>, bad: bool) {
    st.msg = s.into();
    st.bad = bad;
}
fn job(helper: &str, args: &[&str], input: String, f: impl FnOnce(run::Output) -> Msg + Send + 'static) -> Task<Msg> {
    Task::perform(kit::run_in(kit::job(helper, args), input), f)
}
fn admin(args: &[&str], input: String, label: &'static str, ok: String) -> Task<Msg> {
    Task::perform(kit::run_in(kit::admin(args), input), move |o| Msg::AdminDid(label, ok.clone(), o))
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.page = page;
    let part = std::mem::take(&mut sh.part);
    st.skip = kit::skip_of(&part);
    st.pick = if page == "signin" && kit::part_word(&part) == "finger" { Some("finger") } else { None };
    let word = kit::part_word(&part);
    st.msg.clear();
    st.picking = false;
    st.confirm.clear();
    // Typed passwords don't stay when you leave a page.
    st.f.clear();
    match page {
        "users" => {
            // users:<name>: that account's options open (CI's pictures).
            st.picked = word;
            job(SYS, &["users"], String::new(), Msg::Users)
        }
        "signin" => {
            let script = format!("[ -x {a} ] && echo admin=1; [ -e /etc/nimbus/no-autologin ] && echo autologin=0; echo host=$(cat /etc/hostname 2>/dev/null || hostname)", a = kit::lx("hearth-admin"));
            Task::batch([
                Task::perform(kit::run_in(kit::sh_t(&script, 5), String::new()), Msg::Info),
                Task::perform(kit::run_in(kit::sh_t(&format!("[ -x {l} ] || exit 3; exec timeout 10 {l} status", l = login_screen_path()), 15), String::new()), Msg::Ls),
                job("hearth-fingerprint", &["status"], String::new(), Msg::Fp),
            ])
        }
        "online-accounts" => {
            let t = Task::batch([job("hearth_accounts.py", &["list"], String::new(), Msg::Accounts), job("hearth_accounts.py", &["providers"], String::new(), Msg::Prov)]);
            if let Some(k) = kit::part_word(&part).strip_prefix("add-").map(String::from) {
                if KINDS.iter().any(|x| x.0 == k) {
                    st.adding = k;
                }
            }
            t
        }
        "login-screen" => job("hearth-loginscreen", &["status"], String::new(), Msg::Lsc),
        _ => Task::none(),
    }
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Go(id) => sh.go(id),
        Msg::F(k, v) => {
            st.f.insert(k, v);
        }
        Msg::Open(id) => st.pick = Some(id),
        Msg::Close => st.pick = None,
        // ---------------------------------------------------------- Users
        Msg::Users(o) => match kit::answer(&o) {
            Ok(v) if v.is_object() => {
                let want: Vec<(String, crate::pics::Cut)> = kit::arr(&v, "users").iter().map(|x| (kit::s_of(x, "picture"), crate::pics::ROUND)).collect();
                st.users = Some(v);
                return pics_task(want);
            }
            Ok(_) => {}
            Err(e) => say(st, e, true),
        },
        Msg::Pics => {}
        Msg::TogglePictures => {
            st.picking = !st.picking;
            if st.picking {
                return job(SYS, &["pictures"], String::new(), Msg::Pictures);
            }
        }
        Msg::Pictures(o) => {
            st.pictures = kit::answer(&o).map(|v| kit::strs(&v, "pictures")).unwrap_or_default();
            return pics_task(st.pictures.iter().map(|f| (f.clone(), crate::pics::ROUND)).collect());
        }
        Msg::PicSet(p) => {
            st.busy = "picture";
            return job(SYS, &["picture-set", &p], String::new(), Msg::PicDid);
        }
        Msg::PicDid(o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(r) => {
                    st.picking = false;
                    // The account picture's file stays where it is with a new picture in it: read it again.
                    for x in st.users.as_ref().map(|u| kit::arr(u, "users")).unwrap_or_default() {
                        crate::pics::forget(&kit::s_of(&x, "picture"));
                    }
                    let removed = kit::s_of(&r, "picture").is_empty();
                    say(st, if removed { "Picture removed." } else if kit::b_of(&r, "loginScreen") { "Picture changed." } else { "Picture changed (the login screen keeps its own until Hearth's admin helper is there)." }, false);
                }
                Err(e) => say(st, e, true),
            }
            return job(SYS, &["users"], String::new(), Msg::Users);
        }
        Msg::Pick(n) => {
            st.picked = if st.picked == n { String::new() } else { n };
            st.confirm.clear();
        }
        Msg::Confirm(n) => st.confirm = n,
        Msg::UserType(n, adm) => {
            let full = display_name(st, &n);
            return admin(&["user-type", &n, if adm { "admin" } else { "standard" }], String::new(), "type", format!("{full}{}", if adm { " is an administrator now." } else { " has a standard account now." }));
        }
        Msg::Remove(n, keep) => {
            let home = st.users.as_ref().and_then(|u| kit::arr(u, "users").into_iter().find(|x| kit::s_of(x, "name") == n).map(|x| kit::s_of(x, "home"))).unwrap_or_default();
            let ok = if keep { format!("{n} was removed. Their files are still in {home}.") } else { format!("{n} and their files were removed.") };
            st.busy = "remove";
            return admin(&["user-remove", &n, if keep { "keep-home" } else { "delete-home" }], String::new(), "remove", ok);
        }
        Msg::NewAdmin(a) => st.new_admin = a,
        Msg::Add => {
            let n = get(st, "newName").trim().to_string();
            let ok_name = !n.is_empty() && n.len() <= 32 && n.chars().next().map(|c| c.is_ascii_lowercase() || c == '_').unwrap_or(false) && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
            if !ok_name {
                say(st, "Account names use small letters, numbers, - and _ (up to 32), starting with a letter.", true);
                return Task::none();
            }
            let (pw, pw2) = (get(st, "addPw"), get(st, "addPw2"));
            if pw.chars().count() < 4 {
                say(st, "The password needs at least 4 characters.", true);
                return Task::none();
            }
            if pw != pw2 {
                say(st, "The passwords don't match.", true);
                return Task::none();
            }
            st.busy = "add";
            let input = json!({"password": pw, "fullname": get(st, "newFull").trim()}).to_string() + "\n";
            let kind = if st.new_admin { "admin" } else { "standard" };
            return admin(&["user-add", &n, kind], input, "add", format!("{n} can now sign in."));
        }
        Msg::OemFinish => {
            let name = get(st, "oemName").trim().to_string();
            if name.is_empty() || name.contains([':', ',', '=', '\\']) {
                say(st, "Type your name (no : , = or \\ in it).", true);
                return Task::none();
            }
            let (pw, pw2) = (get(st, "oemPw"), get(st, "oemPw2"));
            if pw.chars().count() < 4 {
                say(st, "The password needs at least 4 characters.", true);
                return Task::none();
            }
            if pw != pw2 {
                say(st, "The passwords don't match.", true);
                return Task::none();
            }
            st.busy = "oem";
            return admin(&["oem-finish"], json!({"full": name, "password": pw}).to_string() + "\n", "oem", "All set: your name and password are saved.".into());
        }
        Msg::Password => {
            let (old, new, new2) = (get(st, "oldPw"), get(st, "newPw"), get(st, "newPw2"));
            if new.chars().count() < 4 {
                say(st, "The new password needs at least 4 characters.", true);
                return Task::none();
            }
            if new != new2 {
                say(st, "The new passwords don't match.", true);
                return Task::none();
            }
            st.busy = "password";
            return admin(&["password"], json!({"old": old, "new": new}).to_string() + "\n", "password", "Password changed.".into());
        }
        Msg::AdminDid(label, ok, o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(_) => {
                    say(st, ok, false);
                    match label {
                        "password" => clear(st, &["oldPw", "newPw", "newPw2"]),
                        "add" => {
                            clear(st, &["newName", "newFull", "addPw", "addPw2"]);
                            st.new_admin = false;
                        }
                        "oem" => clear(st, &["oemName", "oemPw", "oemPw2"]),
                        "remove" => {
                            st.picked.clear();
                            st.confirm.clear();
                        }
                        _ => {}
                    }
                }
                Err(e) => say(st, e, true),
            }
            return match st.page {
                "users" => job(SYS, &["users"], String::new(), Msg::Users),
                "signin" => Task::perform(kit::run_in(kit::sh_t(&format!("[ -x {a} ] && echo admin=1; [ -e /etc/nimbus/no-autologin ] && echo autologin=0; echo host=$(cat /etc/hostname 2>/dev/null || hostname)", a = kit::lx("hearth-admin")), 5), String::new()), Msg::Info),
                _ => Task::none(),
            };
        }
        // ---------------------------------------------------------- Sign-in options
        Msg::Info(o) => {
            st.can_admin = o.out.lines().any(|l| l == "admin=1");
            st.autologin = !o.out.lines().any(|l| l == "autologin=0");
            st.host = o.out.lines().find_map(|l| l.strip_prefix("host=")).unwrap_or("").trim().to_string();
        }
        Msg::Autologin(on) => {
            st.busy = "autologin";
            let t = Task::perform(kit::run_in(kit::admin(&["autologin", if on { "on" } else { "off" }]), String::new()), move |o| Msg::AdminDid("autologin", if on { "Hearth will sign you in automatically.".into() } else { "The login screen will show at start.".into() }, o));
            return t;
        }
        Msg::LockOnWake(on) => {
            let cmd = sh.desktop_cmd(json!({"lockOnWake": on}));
            let mut t = vec![Task::perform(run::spawn(cmd), Msg::Did)];
            if kit::on_plasma() {
                // KDE's screen locker locks on resume by this (System Settings › Screen Locking).
                t.push(Task::perform(kit::run_in(kit::sh_t(&format!("kwriteconfig6 --notify --file kscreenlockerrc --group Daemon --key LockOnResume {}", on), 10), String::new()), Msg::Did));
            }
            return Task::batch(t);
        }
        Msg::Did(o) => sh.result("That setting couldn't be saved", &o),
        Msg::Ls(o) => st.ls = if o.code == 3 { None } else { serde_json::from_str::<Value>(o.out.trim()).ok().or_else(|| kit::answer(&o).ok()).filter(|v| v.is_object()) },
        Msg::LsSet(on) => {
            st.busy = "ls";
            let cmd = vec!["timeout".into(), "60".into(), "pkexec".into(), login_screen_path(), if on { "on".into() } else { "off".into() }];
            return Task::perform(kit::run_in(cmd, String::new()), move |o| Msg::LsDid(on, o));
        }
        Msg::LsDid(on, o) => {
            st.busy = "";
            st.ls_msg = match kit::answer(&o) {
                Ok(_) => (if on { "Hearth's login screen shows from the next start. If it doesn't work, the usual one comes back by itself.".into() } else { "The usual login screen (SDDM) shows from the next start.".into() }, false),
                Err(e) => (e, true),
            };
            return Task::perform(kit::run_in(kit::sh_t(&format!("exec timeout 10 {} status", login_screen_path()), 15), String::new()), Msg::Ls);
        }
        Msg::LsPreview => run::detached(&[&login_screen_path(), "preview"]),
        Msg::Fp(o) => match kit::answer(&o) {
            Ok(v) if v.is_object() => {
                st.fp = Some(v);
                let free = free_fingers(st);
                if !free.contains(&st.finger.as_str()) {
                    st.finger = free.first().map(|s| s.to_string()).unwrap_or_default();
                }
            }
            Ok(_) => {}
            Err(e) => st.fp_msg = (e, true),
        },
        Msg::Finger(f) => {
            st.finger = f;
            st.pick = None;
        }
        Msg::Enroll => {
            if st.enrolling.is_some() || st.finger.is_empty() {
                return Task::none();
            }
            st.fp_msg = (String::new(), false);
            st.enrolling = Some((0.03, "Touch the fingerprint reader".into()));
            let (rx, pid) = kit::lines(kit::job_t("hearth-fingerprint", &["enroll", &st.finger.clone()], 300), String::new());
            st.fp_pid = pid;
            return Task::run(rx, Msg::FpLine);
        }
        Msg::FpLine(l) => match l {
            Line::Progress(n, w) => {
                if let Some(e) = st.enrolling.as_mut() {
                    if n >= 0.0 {
                        e.0 = n.max(0.03);
                    }
                    if !w.is_empty() {
                        e.1 = w;
                    }
                }
            }
            Line::Json(v) => {
                st.fp = Some(v);
                st.fp_msg = (format!("{} is saved. Touch the reader to unlock where it's turned on below.", finger_name(&st.finger)), false);
            }
            Line::Err(e) => st.fp_msg = (e, true),
            Line::End(_) => {
                st.enrolling = None;
                return job("hearth-fingerprint", &["status"], String::new(), Msg::Fp);
            }
        },
        Msg::FpConfirm(f) => st.fp_confirm = if st.fp_confirm == f { String::new() } else { f },
        Msg::FpDelete(f) => return job("hearth-fingerprint", &["delete", &f], String::new(), |o| Msg::FpDid("delete", o)),
        Msg::FpDid(_, o) => {
            match kit::answer(&o) {
                Ok(v) => {
                    if v.is_object() {
                        st.fp = Some(v);
                    }
                    st.fp_confirm.clear();
                    st.fp_msg = ("Forgotten.".into(), false);
                }
                Err(e) => st.fp_msg = (e, true),
            }
        }
        Msg::FpPlaces(which, on) => {
            let ok = match (which, on) {
                ("lock", true) => "Touch the reader to unlock. Your password still works.",
                ("lock", false) => "The lock screen asks for your password only.",
                (_, true) => "Administrator prompts take your finger too.",
                _ => "Administrator prompts ask for your password only.",
            };
            return Task::perform(kit::run_in(kit::admin(&["fingerprint-places"]), json!({ which: on }).to_string()), move |o| Msg::FpPlacesDid(ok.to_string(), o));
        }
        Msg::FpPlacesDid(ok, o) => {
            st.fp_msg = match kit::answer(&o) {
                Ok(_) => (ok, false),
                Err(e) => (e, true),
            };
            return job("hearth-fingerprint", &["status"], String::new(), Msg::Fp);
        }
        // ---------------------------------------------------------- Online accounts
        Msg::Accounts(o) => {
            st.loaded = true;
            match kit::answer(&o) {
                Ok(v) => st.accounts = kit::arr(&v, "accounts").into_iter().cloned().collect(),
                Err(e) => say(st, e, true),
            }
        }
        Msg::Prov(o) => st.prov = kit::answer(&o).unwrap_or(json!({})),
        Msg::Form(k) => {
            if st.flowing {
                return Task::none();
            }
            st.adding = if st.adding == k { String::new() } else { k };
            st.msg.clear();
        }
        Msg::Flow(args, what) => {
            if st.flowing {
                return Task::none();
            }
            st.msg.clear();
            st.flow_line = "Starting…".into();
            st.flow_login.clear();
            st.flow_what = what;
            st.flow_result = None;
            st.flow_err.clear();
            st.flowing = true;
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            let (rx, pid) = kit::lines(kit::job_t("hearth_accounts.py", &a, 900), String::new());
            st.flow_pid = pid;
            return Task::run(rx, Msg::FlowLine);
        }
        Msg::FlowLine(l) => match l {
            Line::Progress(_, w) => st.flow_line = w,
            Line::Json(v) => {
                if kit::b_of(&v, "waiting") {
                    st.flow_login = kit::s_of(&v, "login");
                } else {
                    st.flow_result = Some(v);
                }
            }
            Line::Err(e) => st.flow_err = e,
            Line::End(code) => {
                st.flowing = false;
                if let Some(r) = st.flow_result.take().filter(|r| r.get("account").is_some()) {
                    let w = st.flow_what;
                    return added(st, &r, w);
                } else if !st.flow_err.is_empty() {
                    st.flow_line.clear();
                    let e = std::mem::take(&mut st.flow_err);
                    say(st, e, true);
                } else if code != 0 {
                    st.flow_line.clear();
                    if st.msg.is_empty() {
                        say(st, "Sign-in stopped.", false);
                    }
                }
            }
        },
        Msg::Cancel => {
            kit::stop(&st.flow_pid);
            st.flow_line.clear();
            st.flow_login.clear();
            say(st, "Sign-in cancelled.", false);
        }
        Msg::AddPw(cmd, what) => {
            let details = match cmd {
                "add-imap" => json!({"host": get(st, "imapHost").trim(), "port": get(st, "imapPort").trim().parse::<i64>().unwrap_or(993), "user": get(st, "imapUser").trim(), "password": get(st, "imapPw"), "ssl": true}),
                _ => json!({"server": get(st, "davServer").trim(), "user": get(st, "davUser").trim(), "password": get(st, "davPw")}),
            };
            st.busy = "adding";
            return job("hearth_accounts.py", &[cmd], details.to_string() + "\n", move |o| Msg::Added(what, o));
        }
        Msg::Added(what, o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(r) => return added(st, &r, what),
                Err(e) => say(st, e, true),
            }
        }
        Msg::Use(id, what, on) => return job("hearth_accounts.py", &["use", &id, what, if on { "on" } else { "off" }], String::new(), Msg::AcctDid),
        Msg::RemoveAcct(id, name) => {
            say(st, format!("{name} is removed from this device (and its password from the keyring)."), false);
            return job("hearth_accounts.py", &["remove", &id], String::new(), Msg::AcctDid);
        }
        Msg::Mount(id, name) => {
            say(st, format!("Opening {name}'s files…"), false);
            return job("hearth_accounts.py", &["mount", &id], String::new(), Msg::Mounted);
        }
        Msg::Mounted(o) => match kit::answer(&o) {
            Ok(r) => {
                st.msg.clear();
                let p = kit::s_of(&r, "path");
                if !p.is_empty() {
                    run::detached(&["hearth-filemanager", &p]);
                }
            }
            Err(e) => say(st, e, true),
        },
        Msg::AcctOpen(id) => st.open_acct = if st.open_acct == id { String::new() } else { id },
        Msg::AcctDid(o) => {
            if let Err(e) = kit::answer(&o) {
                say(st, e, true);
            }
            return job("hearth_accounts.py", &["list"], String::new(), Msg::Accounts);
        }
        // ---------------------------------------------------------- Login screen
        Msg::Lsc(o) => match kit::answer(&o) {
            Ok(v) if v.is_object() => {
                let mut want: Vec<(String, crate::pics::Cut)> = kit::strs(&v, "pictures").into_iter().map(|f| (f, crate::pics::ROW)).collect();
                want.push((kit::s_of(&v, "background"), crate::pics::SCREEN));
                st.lsc = Some(v);
                return pics_task(want);
            }
            Ok(_) => {}
            Err(e) => say(st, e, true),
        },
        Msg::LscUse(p) => {
            st.busy = "set";
            st.msg.clear();
            let p2 = p.clone();
            return job("hearth-loginscreen", &["set", &p], String::new(), move |o| Msg::LscDid(p2.clone(), o));
        }
        Msg::LscDid(p, o) => {
            st.busy = "";
            match kit::answer(&o) {
                Ok(r) => {
                    if r.is_object() {
                        st.lsc = Some(r);
                    }
                    say(st, if p.is_empty() { "Back to Hearth's fireside." } else { "The login screen shows your picture now." }, false);
                }
                Err(e) => say(st, e, true),
            }
        }
        Msg::LscChoose => return Task::perform(kit::run_in(kit::file_picker("Login screen picture", false, "image/jpeg image/png"), String::new()), Msg::LscPicked),
        Msg::LscPicked(o) => {
            if o.code == 3 {
                say(st, "No file picker here: put the picture in Pictures and pick it below.", true);
            } else if !o.out.trim().is_empty() {
                return update(sh, st, Msg::LscUse(o.out.trim().to_string()));
            }
        }
    }
    Task::none()
}

fn added(st: &mut State, r: &Value, what: &str) -> Task<Msg> {
    st.adding.clear();
    st.flow_line.clear();
    st.flow_login.clear();
    clear(st, &["davServer", "davUser", "davPw", "imapHost", "imapUser", "imapPw", "ncServer", "imapPort"]);
    let a = r.get("account").cloned().unwrap_or(json!({}));
    let n = { let x = kit::s_of(&a, "name"); if x.is_empty() { what.to_string() } else { x } };
    say(st, format!("{n} is added. Pick below what Hearth uses it for."), false);
    job("hearth_accounts.py", &["list"], String::new(), Msg::Accounts)
}

fn display_name(st: &State, n: &str) -> String {
    st.users.as_ref().and_then(|u| kit::arr(u, "users").into_iter().find(|x| kit::s_of(x, "name") == n).map(|x| { let f = kit::s_of(x, "fullname"); if f.is_empty() { n.to_string() } else { f } })).unwrap_or_else(|| n.to_string())
}

const FINGERS: &[(&str, &str)] = &[
    ("right-index-finger", "Right index finger"), ("left-index-finger", "Left index finger"), ("right-thumb", "Right thumb"), ("left-thumb", "Left thumb"),
    ("right-middle-finger", "Right middle finger"), ("left-middle-finger", "Left middle finger"), ("right-ring-finger", "Right ring finger"), ("left-ring-finger", "Left ring finger"),
    ("right-little-finger", "Right little finger"), ("left-little-finger", "Left little finger"),
];
fn finger_name(f: &str) -> String {
    FINGERS.iter().find(|x| x.0 == f).map(|x| x.1.to_string()).unwrap_or_else(|| f.to_string())
}
fn free_fingers(st: &State) -> Vec<&'static str> {
    let have = st.fp.as_ref().map(|f| kit::strs(f, "fingers")).unwrap_or_default();
    FINGERS.iter().map(|x| x.0).filter(|f| !have.iter().any(|h| h == f)).collect()
}

const KINDS: &[(&str, Ic, &str, &str)] = &[
    ("nextcloud", Ic::Cloud, "Nextcloud", "Calendar, contacts and files · sign in in your browser"),
    ("google", Ic::Globe, "Google", "Calendar, contacts and Gmail · sign in in your browser"),
    ("microsoft", Ic::Window, "Microsoft", "Outlook calendar and mail · sign in in your browser"),
    ("dav", Ic::Calendar, "CalDAV / CardDAV", "Any calendar and contacts server (Fastmail, iCloud, Radicale, Baïkal…)"),
    ("webdav", Ic::G(G::Folder), "WebDAV files", "Files on a WebDAV server, in Hearth Files"),
    ("imap", Ic::Mail, "IMAP mail", "Any mail server, for Hearth's mail apps"),
];
const USES: &[(&str, Ic, &str, &str)] = &[
    ("calendar", Ic::Calendar, "Calendar", "Events in the calendar widget, the clock and Hearth Time"),
    ("contacts", Ic::G(G::Users), "Contacts", "Address books in Hearth Contacts"),
    ("files", Ic::G(G::Folder), "Files", "Opens in Hearth Files' left menu"),
    ("mail", Ic::Mail, "Mail", "Mail apps can use it (IMAP)"),
];

// ------------------------------------------------------------------ pages
pub fn view<'a>(a: &'a App, page: &'static str) -> Option<E<'a>> {
    Some(match page {
        "accounts" => landing(a),
        "users" => users(a),
        "signin" => signin(a),
        "online-accounts" => online(a),
        "login-screen" => login_screen(a),
        _ => return None,
    })
}

fn landing(a: &App) -> E<'_> {
    let p = a.sh.p;
    let mut c = Column::new().spacing(8.0).push(container(t("You and the other people on this PC, and how you sign in.", sz(14.0), p.muted)).padding(iced::Padding { top: 0.0, right: 0.0, bottom: 8.0, left: 0.0 }));
    for (id, ic) in [("users", Ic::G(G::Users)), ("signin", Ic::Lock), ("online-accounts", Ic::Cloud), ("login-screen", Ic::Image)] {
        if let Some(pg) = crate::registry::page(id) {
            c = c.push(ilink(&p, ic, pg.label, pg.desc, "", Msg::Go(pg.id)));
        }
    }
    c = c.push(head(&p, "Also here"));
    c = c.push(ilink(&p, Ic::G(G::Display), "Device name", "This PC's name on your network: rename it on Home · Home", "", Msg::Go("home")));
    c.into()
}

/// Reads these pictures off the window's thread (pics.rs), then draws the page again.
fn pics_task(paths: Vec<(String, crate::pics::Cut)>) -> Task<Msg> {
    match crate::pics::load(paths) {
        Some(f) => Task::perform(f, |_| Msg::Pics),
        None => Task::none(),
    }
}

/// Your picture in its circle (as Mantel shows it), on `bg`; your initial until it's read or when there's none.
fn avatar<'a>(p: &Pal, name: &str, pic: &str, s: f32) -> E<'a> {
    if let Some(h) = crate::pics::get(pic, crate::pics::ROUND) {
        return crate::pics::round(h, s);
    }
    let letter = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "·".into());
    container(tb(letter, s * 0.4, p.text)).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(p.panel3, s / 2.0, None)).into()
}

fn users(a: &App) -> E<'_> {
    let p = a.sh.p;
    let st = &a.accounts;
    let mut c = kit::Col::new(st.skip);
    let Some(u) = st.users.as_ref() else {
        return column![small(&p, "Reading the accounts on this device…"), if st.msg.is_empty() { E::from(Space::new()) } else { said(&p, &st.msg, st.bad) }].into();
    };
    let list = kit::arr(u, "users");
    let me = list.iter().find(|x| kit::b_of(x, "me")).cloned().cloned();
    let can_admin = kit::b_of(u, "canAdmin");
    let can_manage = can_admin && kit::b_of(u, "admin");
    let oem = kit::b_of(u, "oemSetup");
    if let Some(m) = &me {
        let full = { let f = kit::s_of(m, "fullname"); if f.is_empty() { kit::s_of(m, "name") } else { f } };
        c = c.push(container(row![avatar(&p, &full, &kit::s_of(m, "picture"), 56.0), column![ts(full.clone(), sz(17.0), p.text), t(format!("{} · {}", kit::s_of(m, "name"), if kit::b_of(m, "admin") { "Administrator" } else { "Standard account" }), sz(12.0), p.muted)].spacing(3.0)].spacing(14.0).align_y(iced::Center)).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None)));
        c = c.push(ilink(&p, Ic::Camera, if st.picking { "Choose a picture below" } else { "Change my picture" }, "Shown here and on the login screen", "", Msg::TogglePictures));
        if st.picking {
            let mut g = Column::new().spacing(6.0);
            for f in &st.pictures {
                let pr = pick_row(&p, kit::base(f), Msg::PicSet(f.clone()));
                g = g.push(match crate::pics::get(f, crate::pics::ROUND) {
                    Some(h) => row![crate::pics::round(h, 40.0), pr].spacing(10.0).align_y(iced::Center).into(),
                    None => pr,
                });
            }
            if st.pictures.is_empty() {
                g = g.push(small(&p, "Put a PNG or JPEG picture in your Pictures folder to use it here."));
            } else {
                g = g.push(small(&p, "The picture is shown in a circle: one with your face in the middle works best."));
            }
            if !kit::s_of(m, "picture").is_empty() {
                g = g.push(iact(&p, Ic::G(G::Close), "Remove my picture", "", false, Some(Msg::PicSet(String::new()))));
            }
            c = c.push(indent(g));
        }
    }
    if oem {
        c = c.push(head(&p, "Finish setting up: your name and password"));
        c = c.push(small(&p, "This PC came with Hearth on it, so its account is still the stand-in one, with no password. Type your name and a password here, or do it in first-time setup at the next start."));
        c = c.push(field(&p, "Your name", &get(st, "oemName"), |v| Msg::F("oemName", v)));
        c = c.push(secret(&p, "Password (at least 4 characters)", &get(st, "oemPw"), |v| Msg::F("oemPw", v)));
        c = c.push(secret(&p, "Password again", &get(st, "oemPw2"), |v| Msg::F("oemPw2", v)).on_submit(Msg::OemFinish));
        c = c.push(iact(&p, Ic::G(G::Check), if st.busy == "oem" { "Saving…" } else { "Finish setting up" }, "", false, Some(Msg::OemFinish)));
    }
    if can_admin {
        c = c.push(head(&p, "My password"));
        c = c.push(password_fields(&p, st));
    }
    if !st.msg.is_empty() {
        c = c.push(said(&p, &st.msg, st.bad));
    }
    c = c.push(head(&p, "Accounts on this device"));
    for x in &list {
        let name = kit::s_of(x, "name");
        let full = { let f = kit::s_of(x, "fullname"); if f.is_empty() { name.clone() } else { f } };
        let me_ = kit::b_of(x, "me");
        let player = kit::b_of(x, "player");
        let removable = can_manage && !me_ && !player;
        let kind = if player { "Multi-seat player" } else if kit::b_of(x, "admin") { "Administrator" } else { "Standard account" };
        let row_ = container(row![avatar(&p, &full, &kit::s_of(x, "picture"), 38.0), column![ts(format!("{full}{}", if me_ { "  (you)" } else { "" }), sz(15.0), p.text), t(format!("{name} · {kind}"), sz(12.0), p.muted)].spacing(3.0).width(Length::Fill), if removable { E::from(ui::glyph(G::Chevron, p.muted, 14.0, 2.0)) } else { Space::new().into() }].spacing(14.0).align_y(iced::Center)).padding(14.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None));
        c = c.push(if removable { E::from(iced::widget::mouse_area(row_).on_press(Msg::Pick(name.clone())).interaction(iced::mouse::Interaction::Pointer)) } else { row_.into() });
        if st.picked == name && st.confirm.is_empty() {
            let adm = kit::b_of(x, "admin");
            c = c.push(indent(column![
                irow(&p, Ic::G(G::Shield), "Account type", "Administrators can install apps, change the system and other accounts. From their next sign-in", seg(&p, vec![("Standard".into(), !adm, Some(Msg::UserType(name.clone(), false))), ("Administrator".into(), adm, Some(Msg::UserType(name.clone(), true)))])),
                iact(&p, Ic::Trash, "Remove this account…", "", true, Some(Msg::Confirm(name.clone()))),
            ].spacing(8.0)));
        }
        if st.confirm == name {
            let home = kit::s_of(x, "home");
            c = c.push(indent(column![
                t(format!("Remove {name}? They won't be able to sign in any more."), sz(14.0), p.text),
                iact(&p, Ic::Trash, if st.busy == "remove" { "Removing…" } else { "Remove, but keep their files" }, format!("Their home folder stays ({home})"), false, Some(Msg::Remove(name.clone(), true))),
                iact(&p, Ic::Trash, "Remove and delete their files", "Their home folder, games and saves are deleted for good", true, Some(Msg::Remove(name.clone(), false))),
                iact(&p, Ic::G(G::Close), "Cancel", "", false, Some(Msg::Confirm(String::new()))),
            ].spacing(8.0)));
        }
    }
    if can_manage {
        c = c.push(head(&p, "Add an account"));
        c = c.push(field(&p, "Account name (small letters, like alex)", &get(st, "newName"), |v| Msg::F("newName", v)));
        c = c.push(field(&p, "Full name (optional)", &get(st, "newFull"), |v| Msg::F("newFull", v)));
        c = c.push(secret(&p, "Password (at least 4 characters)", &get(st, "addPw"), |v| Msg::F("addPw", v)));
        c = c.push(secret(&p, "Password again", &get(st, "addPw2"), |v| Msg::F("addPw2", v)).on_submit(Msg::Add));
        c = c.push(irow(&p, Ic::G(G::Shield), "Account type", "Administrators can change system settings, install system updates and manage accounts", seg(&p, vec![("Standard".into(), !st.new_admin, Some(Msg::NewAdmin(false))), ("Administrator".into(), st.new_admin, Some(Msg::NewAdmin(true)))])));
        c = c.push(iact(&p, Ic::G(G::User), if st.busy == "add" { "Adding…" } else { "Add account" }, "", false, Some(Msg::Add)));
    } else {
        c = c.push(small(&p, if !can_admin { "Accounts are managed here on Hearth, the OS." } else { "Only administrators can add or remove accounts." }));
    }
    c.into()
}

fn password_fields<'a>(p: &Pal, st: &State) -> E<'a> {
    column![
        secret(p, "Current password", &get(st, "oldPw"), |v| Msg::F("oldPw", v)),
        secret(p, "New password (at least 4 characters)", &get(st, "newPw"), |v| Msg::F("newPw", v)),
        secret(p, "New password again", &get(st, "newPw2"), |v| Msg::F("newPw2", v)).on_submit(Msg::Password),
        iact(p, Ic::Lock, if st.busy == "password" { "Changing…" } else { "Change my password" }, "", false, Some(Msg::Password)),
    ]
    .spacing(8.0)
    .into()
}

fn signin(a: &App) -> E<'_> {
    let p = a.sh.p;
    let st = &a.accounts;
    let mut c = kit::Col::new(st.skip);
    c = c.push(head(&p, "Signing in"));
    if st.can_admin {
        c = c.push(irow(&p, Ic::G(G::User), "Sign in automatically", if st.busy == "autologin" { "Saving…" } else { "Off shows the login screen at start, with your password" }, switch(&p, st.autologin, Some(Msg::Autologin(!st.autologin)))));
    }
    // Hearth's own login screen (a test): only where hearth-login-screen is.
    if let (Some(ls), true) = (st.ls.as_ref(), st.can_admin) {
        let on = kit::b_of(ls, "on");
        c = c.push(head(&p, "Login screen (test)"));
        if on && kit::b_of(ls, "tripped") {
            let why = ls.get("last").map(|l| kit::s_of(l, "why")).filter(|w| !w.is_empty()).map(|w| format!(" ({w})")).unwrap_or_default();
            c = c.push(kit::banner(&p, Ic::Warn, "Hearth's login screen paused itself", format!("It didn't work twice in a row{why}, so the usual login screen shows at every start. Your setting is still on."), false, Some(("Turn on again", Msg::LsSet(true)))));
        }
        let sub = if st.busy == "ls" { "Saving…".to_string() } else if !on { "A test, off by default. The usual login screen (SDDM) stays and comes back by itself if Hearth's doesn't work.".into() } else if kit::s_of(ls, "thisStart") == "greetd" { "On: in use at this start.".into() } else if kit::b_of(ls, "fellBack") { format!("On, but it didn't work at this start ({}): the usual one was used.", kit::s_of(ls, "why")) } else if !kit::b_of(ls, "canStart") { format!("On, but it can't start here: {}.", { let x = kit::s_of(ls, "cannot"); if x.is_empty() { "something is missing".to_string() } else { x } }) } else { "On: shows from the next start.".into() };
        c = c.push(irow(&p, Ic::G(G::User), "Hearth's own login screen", sub, switch(&p, on, Some(Msg::LsSet(!on)))));
        c = c.push(ilink(&p, Ic::G(G::Display), "Preview", "Hearth's login screen in a window, to see it (signing in is off there)", "", Msg::LsPreview));
        if !st.ls_msg.0.is_empty() {
            c = c.push(said(&p, &st.ls_msg.0, st.ls_msg.1));
        }
    }
    let low = a.sh.d_bool("lockOnWake", false);
    c = c.push(irow(&p, Ic::Lock, "Ask for my password after sleep", "Locked when it wakes, on the desktop and in Game Mode. Lock any time with Super+L or the power menu.", switch(&p, low, Some(Msg::LockOnWake(!low)))));
    if st.can_admin {
        c = c.push(head(&p, "Password"));
        c = c.push(password_fields(&p, st));
    }
    if !st.msg.is_empty() {
        c = c.push(said(&p, &st.msg, st.bad));
    }
    // ---- Fingerprint
    c = c.push(head(&p, "Fingerprint"));
    let fp = st.fp.as_ref();
    let (title, sub) = match fp {
        None => ("Looking for a fingerprint reader…".to_string(), String::new()),
        Some(f) if !kit::b_of(f, "fprintd") => ("Fingerprint sign-in isn't on this system".into(), "It needs fprintd (in Hearth OS's image)".into()),
        Some(f) if !kit::b_of(f, "reader") => ("No fingerprint reader found".into(), "Plug one in, or this PC doesn't have one".into()),
        Some(f) => {
            let n = kit::arr(f, "fingers").len();
            ({ let d = kit::s_of(f, "device"); if d.is_empty() { "Fingerprint reader".into() } else { d } }, if n == 0 { "No fingers saved yet".into() } else if n == 1 { "1 finger saved".into() } else { format!("{n} fingers saved") })
        }
    };
    c = c.push(iinfo(&p, Ic::Finger, title, sub, None));
    let reader = fp.map(|f| kit::b_of(f, "reader")).unwrap_or(false);
    if reader {
        let fingers = fp.map(|f| kit::strs(f, "fingers")).unwrap_or_default();
        for f in &fingers {
            c = c.push(ilink(&p, Ic::Finger, finger_name(f), "Saved on the reader", "", Msg::FpConfirm(f.clone())));
            if st.fp_confirm == *f {
                c = c.push(indent(iact(&p, Ic::Trash, "Forget this finger", "", true, Some(Msg::FpDelete(f.clone())))));
            }
        }
        if let Some((n, w)) = &st.enrolling {
            c = c.push(column![ts(w.clone(), sz(16.0), p.text), ui::bar(vec![(*n, p.accent)], p.panel3, 10.0, 5.0)].spacing(8.0));
        } else if !free_fingers(st).is_empty() {
            c = c.push(irow(&p, Ic::G(G::Plus), "Add a finger", "", dropdown(&p, finger_name(&st.finger), Some(Msg::Open("finger")))));
            c = c.push(iact(&p, Ic::Finger, "Start", "Then touch the reader again and again with that finger until it's done", false, Some(Msg::Enroll)));
        }
        if !fingers.is_empty() {
            let f = fp.unwrap();
            let places = f.get("places").cloned().unwrap_or(json!({}));
            let pam = f.get("pam").cloned().unwrap_or(json!({}));
            let lock = kit::b_of(&places, "lock");
            let pk = kit::b_of(&places, "polkit");
            c = c.push(irow(&p, Ic::Lock, "Unlock the lock screen with a finger", if kit::b_of(&pam, "lock") { "Your password works there too, always" } else { "Needs Hearth's lock screen file (hearth-fingerprint), missing here" }, switch(&p, lock, Some(Msg::FpPlaces("lock", !lock)))));
            c = c.push(irow(&p, Ic::G(G::Shield), "Use it for administrator prompts", if kit::b_of(&pam, "polkit") { "When an app asks for your password: touch the reader, or wait a moment and type it" } else { "Needs Hearth's polkit setup, missing on this system" }, switch(&p, pk, Some(Msg::FpPlaces("polkit", !pk)))));
        }
        c = c.push(small(&p, "A fingerprint is never the only way in: your password always works. Signing in at the login screen still takes your password (it also unlocks your saved passwords)."));
    }
    if !st.fp_msg.0.is_empty() {
        c = c.push(said(&p, &st.fp_msg.0, st.fp_msg.1));
    }
    c = c.push(head(&p, "Moved"));
    c = c.push(ilink(&p, Ic::G(G::Power), "Start in Game Mode or Desktop Mode", "Now in Gaming › Start in", "", Msg::Go("gaming")));
    c = c.push(ilink(&p, Ic::G(G::Display), "Device name", format!("Rename this PC on Home{}", if st.host.is_empty() { String::new() } else { format!(" · now: {}", st.host) }), "", Msg::Go("home")));
    if !st.can_admin {
        c = c.push(small(&p, "Automatic sign-in and your password are set here on Hearth, the OS."));
    }
    c.into()
}

fn online(a: &App) -> E<'_> {
    let p = a.sh.p;
    let st = &a.accounts;
    let mut c = kit::Col::new(st.skip);
    if st.loaded && st.prov.get("secretService").and_then(Value::as_bool) == Some(false) {
        c = c.push(kit::banner(&p, Ic::Lock, "No keyring on this device", "Hearth keeps online accounts' passwords only in the system keyring, and it isn't running. Accounts can't be added until it is.", true, None));
    }
    c = c.push(head(&p, "Your accounts"));
    if st.loaded && st.accounts.is_empty() {
        c = c.push(iinfo(&p, Ic::Cloud, "No online accounts yet", "Sign in once below, and Hearth's calendar, contacts, files and mail use the account. Passwords stay in this device's keyring.", None));
    }
    if !st.loaded {
        c = c.push(small(&p, "Reading your accounts…"));
    }
    let one = st.accounts.len() == 1;
    for ac in &st.accounts {
        let id = kit::s_of(ac, "id");
        let kind = kit::s_of(ac, "kind");
        let icon = KINDS.iter().find(|k| k.0 == kind).map(|k| k.1).unwrap_or(Ic::G(G::User));
        let kl = { let x = kit::s_of(ac, "kindLabel"); if x.is_empty() { KINDS.iter().find(|k| k.0 == kind).map(|k| k.2.to_string()).unwrap_or(kind.clone()) } else { x } };
        let name = kit::s_of(ac, "name");
        let user = kit::s_of(ac, "user");
        let usev = ac.get("use").cloned().unwrap_or(json!({}));
        let can = ac.get("can").cloned().unwrap_or(json!({}));
        let used: Vec<String> = USES.iter().filter(|u| kit::b_of(&usev, u.0)).map(|u| u.2.to_lowercase()).collect();
        let sub = format!("{kl}{} · used for {}", if !user.is_empty() && !name.contains(&user) { format!(" · {user}") } else { String::new() }, if used.is_empty() { "nothing yet".to_string() } else { used.join(", ") });
        let open = one || st.open_acct == id;
        c = c.push(iced::widget::mouse_area(irow(&p, icon, name.clone(), sub, E::from(row![chip(&p, "Signed in", Kind::Ok), ui::glyph(if open { G::Down } else { G::Chevron }, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center)))).on_press(Msg::AcctOpen(id.clone())).interaction(iced::mouse::Interaction::Pointer));
        if open {
            let mut inner = Column::new().spacing(8.0);
            for (u, ic, title, s) in USES.iter().filter(|u| kit::b_of(&can, u.0)) {
                let on = kit::b_of(&usev, u);
                inner = inner.push(irow(&p, *ic, *title, *s, switch(&p, on, Some(Msg::Use(id.clone(), *u, !on)))));
            }
            if kit::b_of(&usev, "files") {
                inner = inner.push(ilink(&p, Ic::G(G::Folder), "Open in Hearth Files", "Mounted with gvfs (davs://), like a drive", "", Msg::Mount(id.clone(), name.clone())));
            }
            inner = inner.push(iact(&p, Ic::Trash, "Remove this account", "Signs this device out: its password or token is taken out of the keyring", true, Some(Msg::RemoveAcct(id.clone(), name.clone()))));
            c = c.push(indent(inner));
        }
    }
    c = c.push(head(&p, "Add an account"));
    for (k, ic, title, sub) in KINDS {
        let needs = (*k == "google" || *k == "microsoft") && st.prov.get(*k).map(|x| !kit::b_of(x, "ready")).unwrap_or(false);
        let right = row![if needs { chip(&p, "Needs setting up", Kind::Off) } else { E::from(Space::new()) }, ui::glyph(if st.adding == *k { G::Down } else { G::Chevron }, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center);
        c = c.push(iced::widget::mouse_area(irow(&p, *ic, *title, *sub, E::from(right))).on_press(Msg::Form(k.to_string())).interaction(iced::mouse::Interaction::Pointer));
    }
    if !st.adding.is_empty() {
        let k = st.adding.as_str();
        let title = KINDS.iter().find(|x| x.0 == k).map(|x| x.2).unwrap_or("");
        let mut f = Column::new().spacing(8.0).push(head(&p, format!("Add {title}")));
        match k {
            "nextcloud" => {
                f = f.push(field(&p, "Your Nextcloud's address (cloud.example.com)", &get(st, "ncServer"), |v| Msg::F("ncServer", v)));
                f = f.push(t("Your browser opens Nextcloud's own sign-in page (two-step sign-in works there). Nextcloud then gives this device its own app password, kept in the keyring; you can revoke it any time in Nextcloud › Settings › Security.", sz(13.0), p.muted));
                let srv = get(st, "ncServer").trim().to_string();
                f = f.push(if st.flowing { btn(&p, "Cancel", ui::Kind::Normal, false, Some(Msg::Cancel)) } else { btn(&p, "Sign in in the browser", ui::Kind::Primary, false, if srv.is_empty() { None } else { Some(Msg::Flow(vec!["add-nextcloud".into(), srv], "Nextcloud")) }) });
            }
            "google" | "microsoft" => {
                let pv = st.prov.get(k).cloned().unwrap_or(json!({}));
                let label = { let x = kit::s_of(&pv, "label"); if x.is_empty() { title.to_string() } else { x } };
                let ready = kit::b_of(&pv, "ready");
                f = f.push(t(if ready { format!("Your browser opens {label}'s sign-in page. Hearth never sees your password: {label} gives this device a sign-in token, kept in the keyring. You can remove its access any time in your {label} account's security settings.") } else { format!("{} Until then, {}", { let w = kit::s_of(&pv, "why"); if w.is_empty() { "This sign-in isn't set up on this device yet.".to_string() } else { w } }, if k == "google" { "Google Calendar works as CalDAV with an app password." } else { "Outlook mail works as IMAP if your account allows app passwords." }) }, sz(13.0), if ready { p.muted } else { p.text }));
                if st.flowing {
                    f = f.push(btn(&p, "Cancel", ui::Kind::Normal, false, Some(Msg::Cancel)));
                } else if ready {
                    f = f.push(btn(&p, "Sign in in the browser", ui::Kind::Primary, false, Some(Msg::Flow(vec!["add-oauth".into(), k.into()], if k == "google" { "Google" } else { "Microsoft" }))));
                }
            }
            "dav" | "webdav" => {
                f = f.push(field(&p, if k == "webdav" { "WebDAV address (https://…)" } else { "Server address (https://dav.example.com)" }, &get(st, "davServer"), |v| Msg::F("davServer", v)));
                f = f.push(field(&p, "User name", &get(st, "davUser"), |v| Msg::F("davUser", v)));
                let (cmd, what) = if k == "webdav" { ("add-webdav", "WebDAV") } else { ("add-dav", "The account") };
                f = f.push(secret(&p, "Password (an app password if your provider has them)", &get(st, "davPw"), |v| Msg::F("davPw", v)).on_submit(Msg::AddPw(cmd, what)));
                f = f.push(btn(&p, if st.busy == "adding" { "Signing in…" } else { "Sign in" }, ui::Kind::Primary, false, Some(Msg::AddPw(cmd, what))));
            }
            _ => {
                f = f.push(field(&p, "Mail server (imap.example.com)", &get(st, "imapHost"), |v| Msg::F("imapHost", v)));
                let port = { let x = get(st, "imapPort"); if st.f.contains_key("imapPort") { x } else { "993".into() } };
                f = f.push(field(&p, "Port", &port, |v| Msg::F("imapPort", v)));
                f = f.push(field(&p, "User name (often your e-mail address)", &get(st, "imapUser"), |v| Msg::F("imapUser", v)));
                f = f.push(secret(&p, "Password", &get(st, "imapPw"), |v| Msg::F("imapPw", v)).on_submit(Msg::AddPw("add-imap", "Mail")));
                f = f.push(btn(&p, if st.busy == "adding" { "Signing in…" } else { "Sign in" }, ui::Kind::Primary, false, Some(Msg::AddPw("add-imap", "Mail"))));
            }
        }
        if st.flowing && !st.flow_line.is_empty() {
            f = f.push(iinfo(&p, Ic::Sync, st.flow_line.clone(), if st.flow_login.is_empty() { "Waiting for the browser…".to_string() } else { format!("If the browser didn't open, go to {}", st.flow_login) }, None));
        }
        c = c.push(f);
    }
    if !st.msg.is_empty() {
        c = c.push(said(&p, &st.msg, st.bad));
    }
    c.into()
}

fn login_screen(a: &App) -> E<'_> {
    let p = a.sh.p;
    let st = &a.accounts;
    let mut c = kit::Col::new(st.skip);
    c = c.push(head(&p, "Picture"));
    let bgp = st.lsc.as_ref().map(|s| kit::s_of(s, "background")).unwrap_or_default();
    // The preview: the login screen as it shows: your picture's colour darkened, or Hearth's fireside.
    let clock = hearth_ui::time::clock(hearth_ui::time::now()).replace(" AM", "").replace(" PM", "");
    let base = if bgp.is_empty() { hearth_ui::theme::hex("#17110e").unwrap() } else { Pal::mix(kit::name_colour(&p, &bgp), iced::Color::BLACK, 0.45) };
    let words = container(column![tb(clock, 44.0, hearth_ui::theme::hex("#f4ece3").unwrap()), t(if bgp.is_empty() { "Hearth's fireside".to_string() } else { kit::base(&bgp) }, sz(14.0), hearth_ui::theme::hex("#a69484").unwrap())].spacing(4.0).align_x(iced::Center)).center_x(Length::Fill).center_y(Length::Fixed(sz(260.0)));
    // Your picture itself behind the clock when it's been read (darkened as the login screen does), else its colour.
    // A screen's shape (16:9), as the login screen is, in the middle of the dark box.
    c = c.push(match crate::pics::get(&bgp, crate::pics::SCREEN) {
        Some(h) => {
            let (w, ht) = (sz(260.0) * 16.0 / 9.0, sz(260.0));
            let shot = iced::widget::stack![
                crate::pics::tile(h, w, ht),
                container(Space::new()).width(Length::Fixed(w)).height(Length::Fixed(ht)).style(ui::boxed(Pal::a(iced::Color::BLACK, 0.45), 0.0, None)),
                words.width(Length::Fixed(w)),
            ];
            E::from(container(shot).center_x(Length::Fill).style(ui::boxed(base, 12.0, None)))
        }
        None => E::from(words.style(ui::boxed(base, 12.0, None))),
    });
    let pics = st.lsc.as_ref().map(|s| kit::strs(s, "pictures")).unwrap_or_default();
    if !pics.is_empty() {
        let mut g = Column::new().spacing(6.0);
        for f in pics {
            let pr = pick_row(&p, kit::base(&f), Msg::LscUse(f.clone()));
            g = g.push(match crate::pics::get(&f, crate::pics::ROW) {
                Some(h) => row![crate::pics::tile(h, 64.0, 40.0), pr].spacing(10.0).align_y(iced::Center).into(),
                None => pr,
            });
        }
        c = c.push(g);
    }
    c = c.push(iact(&p, Ic::Image, if st.busy == "set" { "Saving…" } else { "Choose a picture…" }, "Any PNG or JPEG of yours", false, Some(Msg::LscChoose)));
    if !bgp.is_empty() {
        c = c.push(iact(&p, Ic::Undo, "Use Hearth's fireside", "The login screen's own look", false, Some(Msg::LscUse(String::new()))));
    }
    if !st.msg.is_empty() {
        c = c.push(said(&p, &st.msg, st.bad));
    }
    c = c.push(head(&p, "Signing in"));
    let al = st.lsc.as_ref().map(|s| kit::b_of(s, "autologin"));
    c = c.push(ilink(&p, Ic::Lock, "Sign in automatically", match al { Some(true) => "On: the login screen is skipped when this PC starts · in Sign-in options", Some(false) => "Off: the login screen shows when this PC starts · in Sign-in options", None => "" }, "", Msg::Go("signin")));
    c.into()
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(a: &'a App) -> Option<(E<'a>, Option<Msg>)> {
    let st = &a.accounts;
    if a.page != "signin" || st.pick != Some("finger") {
        return None;
    }
    let p = a.sh.p;
    let items = free_fingers(st).into_iter().map(|f| (finger_name(f), String::new(), f == st.finger, Msg::Finger(f.to_string()))).collect();
    Some((kit::pick_dialog(&p, "Add a finger", items, Msg::Close), Some(Msg::Close)))
}

pub fn back(st: &mut State) -> bool {
    if st.pick.take().is_some() {
        return true;
    }
    if !st.confirm.is_empty() || !st.picked.is_empty() {
        st.confirm.clear();
        st.picked.clear();
        return true;
    }
    false
}

