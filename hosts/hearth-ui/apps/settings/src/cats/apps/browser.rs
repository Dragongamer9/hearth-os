//! Apps › Web browser (Mantel's SetBrowser.qml): your browser, Hearth Browser (Firefox with Hearth's settings,
//! /usr/bin/hearth-browser) or Google Chrome (downloaded from Google's signed package site when picked, into your
//! home folder; "Must Have For Xodus Nexus"); Hearth Browser's own choices; and the optional Aloha Browser. Through
//! the hearth-browser helper next to Mantel's pages, as on Debian.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, G};
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub hb: Value,
    pub ch: Value,
    pub busy: Busy,
    pub error: String,
}

#[derive(Debug, Clone)]
pub enum M {
    Status(Res),
    Hb(Res),
    Choice(Res),
    Run(Vec<String>),
    Ev(Ev),
    Hearth(Vec<String>),
    HbDone(Res),
    OpenHb,
}

const HB: &str = "/usr/bin/hearth-browser";

fn hb(args: Vec<String>) -> Vec<String> {
    let mut v = vec![HB.to_string()];
    v.extend(args);
    prog(v, 30)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.error.clear();
    Task::batch([read(py("hearth-browser", &["status"], 60), M::Status), read(hb(vec!["--hearth-status".into()]), M::Hb), read(py("hearth-browser", &["choice"], 30), M::Choice)])
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Status(r) => {
            if !r.v.is_null() {
                st.st = r.v;
            } else if !r.ok {
                st.error = r.err;
            }
        }
        M::Hb(r) => {
            if !r.v.is_null() {
                st.hb = r.v;
            } else {
                st.hb = serde_json::json!({"installed": false});
            }
        }
        M::Choice(r) => {
            if !r.v.is_null() {
                st.ch = r.v;
            }
        }
        M::Run(args) => {
            if st.busy.on {
                return Task::none();
            }
            st.busy.start();
            st.error.clear();
            return job(pyv("hearth-browser", args, 1800), None, M::Ev);
        }
        M::Ev(e) => match e {
            Ev::Line(f, l) => st.busy.step(f, l),
            Ev::Done(r) => {
                st.busy.stop();
                if !r.ok {
                    st.error = r.err;
                }
                if !r.v.is_null() && r.v.get("installed").is_some() {
                    st.st = r.v;
                }
                return Task::batch([read(py("hearth-browser", &["choice"], 30), M::Choice), read(hb(vec!["--hearth-status".into()]), M::Hb), read(py("hearth-browser", &["status"], 60), M::Status)]);
            }
        },
        M::Hearth(args) => return read(hb(args), M::HbDone),
        M::HbDone(r) => {
            if !r.ok {
                st.error = r.err;
            }
            if !r.v.is_null() {
                st.hb = r.v;
            }
        }
        M::OpenHb => {
            if gb(&st.hb, "installed") {
                kit::open(&[HB]);
            }
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let idle = !st.busy.on;
    let mut v: Vec<E<M>> = vec![head(&p, "Your browser")];
    let ch = &st.ch;
    let chrome = go(ch, "chrome");
    let mine = gs(ch, "browser");
    let s = |x: Vec<&str>| x.into_iter().map(String::from).collect::<Vec<_>>();
    v.push(r(G::Link, "Hearth Browser (Firefox)", "Comes with Hearth OS. Google sign-in, Netflix and add-ons work").chip(if mine == "firefox" { "Your browser" } else { "" }, Chip::Ok).on_maybe(idle.then(|| M::Run(s(vec!["pick", "firefox"])))).view(&p));
    let csub = if ch.is_null() {
        "Checking…".to_string()
    } else if gb(chrome, "installed") {
        format!("Installed{} · Hearth looks for updates once a week", if gb(chrome, "version") { format!(" · version {}", gs(chrome, "version")) } else { String::new() })
    } else {
        "Downloaded from Google when you pick it (about 120 MB), checked against Google's signature, kept in your home folder".into()
    };
    v.push(r(G::Link, "Google Chrome", csub).chip(if mine == "chrome" { "Your browser" } else { "" }, Chip::Ok).on_maybe(idle.then(|| M::Run(s(vec!["pick", "chrome"])))).view(&p));
    v.push(iced::widget::container(hearth_ui::ui::ts("Must Have For Xodus Nexus", sz(13.0), p.danger)).padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 56.0 }).into());
    if gb(chrome, "installed") && idle {
        v.push(r(G::Update, "Look for a Chrome update", "Hearth also checks once a week").on(M::Run(s(vec!["chrome", "update"]))).view(&p));
        v.push(r(G::Close, "Remove Google Chrome", "Your browser goes back to Hearth Browser").danger().on(M::Run(s(vec!["chrome", "remove"]))).view(&p));
    }

    v.push(head(&p, "Hearth Browser"));
    let h = &st.hb;
    let hsub = if h.is_null() {
        "Checking…".to_string()
    } else if !gb(h, "installed") {
        "Not part of this version of Hearth OS yet".into()
    } else {
        format!("Built on Firefox{}: Google sign-in, Netflix and add-ons work{}", if gb(h, "version") { format!(" {}", gs(h, "version")) } else { String::new() }, if gb(h, "default") { " · your default browser" } else { "" })
    };
    v.push(r(G::Link, "Open Hearth Browser", hsub).chev().on(M::OpenHb).view(&p));
    if gb(h, "installed") {
        if !gb(h, "default") {
            v.push(r(G::Check, "Make Hearth Browser the default", "Links from other apps open in it").on(M::Hearth(s(vec!["--hearth-default"]))).view(&p));
        }
        let lay = if gs(h, "layout").is_empty() { "auto".to_string() } else { gs(h, "layout") };
        let opts = [("auto", "In Game Mode"), ("big", "Always"), ("normal", "Never")];
        v.push(choice(&p, G::Display, "Bigger buttons and text", "For a controller and a small screen. Applies the next time it opens.", opts.iter().map(|(val, l)| (l.to_string(), lay == *val, Some(M::Hearth(s(vec!["--hearth-layout", *val]))))).collect()));
    }

    v.push(head(&p, "Aloha Browser (optional)"));
    let a = &st.st;
    let asub = if a.is_null() {
        "Checking…".to_string()
    } else if !gb(a, "installed") {
        "Not installed".into()
    } else {
        format!("{}{}", if gb(a, "default") { "Your default browser" } else { "Installed, but another browser opens links" }, if gb(a, "version") { format!(" · version {}", gs(a, "version")) } else { String::new() })
    };
    v.push(r(G::Info, "Aloha Browser", asub).view(&p));
    opt(&mut v, busy_view(&p, &st.busy, "Working…"));
    if !a.is_null() && idle {
        if !gb(a, "installed") {
            v.push(r(G::Update, "Install Aloha Browser", "Downloads the official package from alohabrowser.com (about 150 MB) and makes it the default browser").chev().on(M::Run(s(vec!["install"]))).view(&p));
        } else {
            if !gb(a, "default") {
                v.push(r(G::Check, "Make Aloha the default browser", "").on(M::Run(s(vec!["default"]))).view(&p));
            }
            v.push(r(G::Update, "Look for an update", "Hearth also checks once a week").on(M::Run(s(vec!["update"]))).view(&p));
            v.push(r(G::Close, "Remove Aloha Browser", "Hearth won't install it again by itself").danger().on(M::Run(s(vec!["remove"]))).view(&p));
        }
    }
    if !st.error.is_empty() {
        v.push(t(st.error.clone(), sz(13.0), p.danger).into());
    }
    page(v)
}
