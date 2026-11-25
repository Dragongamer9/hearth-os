//! Apps › App store (Mantel's SetStore.qml): Flathub's popular, trending and new apps, its categories and search; an
//! app's page (description, rating and reviews, size, license, Install / Open / Remove); Updates (your Flatpaks, ones
//! asking for new permissions wait for your yes, "Tell me when app updates are there"); Sources (Flatpak remotes);
//! Drivers. All through hearth-store, except Drivers: on the Fedora edition the system is an image (bootc), so the
//! PC's parts are read with lspci and rpm (dnf's packages) and drivers come with Hearth OS's updates.
//! `hearth-settings store:updates`, `store:sources`, `store:drivers`, `store:app-org_kde_krita` open at that part.
//! The apps' icons and screenshots are drawn as the pictures themselves (SetStore.qml's Images): hearth-store pics
//! saves each one once in its cache (Flathub's https:// addresses; a file path is used as it is), pics.rs reads it off
//! the window's thread, and the page is drawn again when it's ready. Until then (or offline) the store mark stands in.

use super::kit::{self, *};
use super::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, tb, G};
use iced::widget::{column, row, Column, Row};
use iced::{Length, Task};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum View {
    #[default]
    Discover,
    List,
    App,
    Updates,
    Sources,
    Drivers,
}

#[derive(Default)]
pub struct St {
    pub view: View,
    pub back: View,
    pub home: Value,
    pub apps: Vec<Value>,
    pub cat_label: String,
    pub app: Value,
    pub app_id: String,
    pub ups: Value,
    pub prefs: Value,
    pub reviewing: String,
    pub review_next: String,
    pub srcs: Vec<Value>,
    pub drivers: Value,
    pub loading: bool,
    pub adding: bool,
    pub src_url: String,
    pub query: String,
    pub confirm: String,
    pub say: Say,
    pub busy: Busy,
    /// The pictures' files: Flathub's address → the file hearth-store saved it in.
    pub pics: std::collections::HashMap<String, String>,
    /// Addresses already asked for (each once while Settings is open).
    pub asked: std::collections::HashSet<String>,
    /// The screenshot in front on an app's page (SetStore.qml's shotsCard.at).
    pub shot: usize,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Show(View),
    Home(Res),
    Category(String, String),
    Query(String),
    Search,
    Listed(Res),
    App(String),
    GotApp(Res),
    Updates,
    GotUpdates(Res),
    Prefs(Res),
    Notify(bool),
    Sources,
    GotSources(Res),
    Drivers,
    GotDrivers(Res),
    Review(String),
    PrefsSet(String, Res),
    Work(Vec<String>, String, Then),
    Ev(String, Then, Ev),
    Install,
    OpenApp,
    Remove,
    Website,
    Adding,
    SrcUrl(String),
    AddSource,
    RemoveSource(String),
    // (feature/hearth-store) "Hearth's own apps": opens the Hearth Store (hearth-store-app, Fedora image only).
    HearthStore,
    /// hearth-store pics answered (address → file).
    Pics(Res),
    /// Pictures read: draw again.
    PicsReady,
    /// The screenshots: one back (-1) or on (1).
    Shot(i32),
}

/// What to read again after a job.
#[derive(Debug, Clone, Copy)]
pub enum Then {
    App,
    Updates,
    Sources,
}

fn hs(args: &[&str], secs: u32) -> Vec<String> {
    py("hearth-store", args, secs)
}

pub fn open(sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    st.confirm.clear();
    let part = std::mem::take(&mut sh.part);
    match part.as_str() {
        "updates" => update(sh, st, M::Updates),
        "sources" => update(sh, st, M::Sources),
        "drivers" => update(sh, st, M::Drivers),
        a if a.starts_with("review-") => {
            st.review_next = a[7..].replace('_', ".");
            update(sh, st, M::Updates)
        }
        a if a.starts_with("app-") => {
            let id = a[4..].replace('_', ".");
            let t = load_home(st);
            Task::batch([t, update(sh, st, M::App(id))])
        }
        _ => {
            st.view = View::Discover;
            load_home(st)
        }
    }
}

fn load_home(st: &mut St) -> Task<M> {
    st.loading = true;
    read(hs(&["home"], 60), M::Home)
}

fn show(st: &mut St, v: View) {
    st.say.clear();
    st.confirm.clear();
    st.view = v;
}

pub fn back(st: &mut St) -> bool {
    match st.view {
        View::App => {
            st.view = st.back;
            true
        }
        View::Discover => false,
        _ => {
            st.view = View::Discover;
            true
        }
    }
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::HearthStore => kit::open(&["hearth-store-app"]),
        M::Go(id) => sh.go(id),
        M::Show(v) => {
            show(st, v);
            if v == View::Discover && st.home.is_null() {
                return load_home(st);
            }
        }
        M::Home(r) => {
            st.loading = false;
            if r.ok {
                if gb(&r.v, "cached") {
                    st.say.ok("Offline: showing what Flathub showed last time.");
                }
                st.home = r.v;
                return want_pics(st);
            } else {
                st.say.bad(r.err);
            }
        }
        M::Category(id, label) => {
            st.cat_label = label;
            st.apps.clear();
            show(st, View::List);
            st.loading = true;
            return read(hs(&["category", &id], 60), M::Listed);
        }
        M::Query(q) => st.query = q,
        M::Search => {
            let q = st.query.trim().to_string();
            if q.chars().count() < 2 {
                return Task::none();
            }
            st.cat_label = format!("“{q}”");
            st.apps.clear();
            show(st, View::List);
            st.loading = true;
            return read(hs(&["search", &q], 60), M::Listed);
        }
        M::Listed(r) => {
            st.loading = false;
            if r.ok {
                st.apps = ga(&r.v, "apps").to_vec();
                return want_pics(st);
            } else {
                st.say.bad(r.err);
            }
        }
        M::App(id) => {
            if st.view != View::App {
                st.back = st.view;
            }
            st.app_id = id.clone();
            st.app = Value::Null;
            show(st, View::App);
            st.loading = true;
            return read(hs(&["app", &id], 60), M::GotApp);
        }
        M::GotApp(r) => {
            st.loading = false;
            if r.ok {
                st.app = r.v;
                st.shot = 0;
                return want_pics(st);
            } else {
                st.say.bad(r.err);
            }
        }
        M::Updates => {
            show(st, View::Updates);
            st.reviewing.clear();
            st.loading = true;
            return Task::batch([read(hs(&["updates"], 120), M::GotUpdates), read(hs(&["prefs"], 30), M::Prefs)]);
        }
        M::GotUpdates(r) => {
            st.loading = false;
            if r.ok {
                st.ups = r.v;
                st.reviewing = std::mem::take(&mut st.review_next);
            } else {
                st.say.bad(r.err);
            }
        }
        M::Prefs(r) => {
            if r.ok {
                st.prefs = go(&r.v, "prefs").clone();
            }
        }
        M::Notify(on) => {
            let ok = if on { "Hearth tells you when app updates are there." } else { "No more app update notices." };
            sh.reread_history = true;
            return read(hs(&["prefs-set", "notifyUpdates", if on { "on" } else { "off" }], 30), move |r| M::PrefsSet(ok.to_string(), r));
        }
        M::Sources => {
            show(st, View::Sources);
            st.loading = true;
            return read(hs(&["sources"], 60), M::GotSources);
        }
        M::GotSources(r) => {
            st.loading = false;
            if r.ok {
                st.srcs = ga(&r.v, "sources").to_vec();
            } else {
                st.say.bad(r.err);
            }
        }
        M::Drivers => {
            show(st, View::Drivers);
            st.drivers = Value::Null;
            return read(prog(vec!["sh".into(), "-c".into(), FEDORA_DRIVERS.into()], 40), M::GotDrivers);
        }
        M::GotDrivers(r) => st.drivers = drivers(&r),
        M::PrefsSet(ok, r) => {
            if r.ok {
                st.prefs = go(&r.v, "prefs").clone();
                st.say.ok(ok);
            } else {
                st.say.bad(r.err);
            }
        }
        M::Review(id) => st.reviewing = if st.reviewing == id { String::new() } else { id },
        M::Work(args, ok, then) => {
            if st.busy.on {
                return Task::none();
            }
            st.confirm.clear();
            st.say.clear();
            st.busy.start();
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return job(hs(&a, 1800), None, move |e| M::Ev(ok.clone(), then, e));
        }
        M::Ev(ok, then, e) => match e {
            Ev::Line(f, l) => st.busy.step(f, l),
            Ev::Done(r) => {
                st.busy.stop();
                if !r.ok {
                    st.say.bad(r.err);
                    return Task::none();
                }
                let waiting = ga(&r.v, "waiting");
                let text = if !ok.is_empty() {
                    ok
                } else if !waiting.is_empty() {
                    let names: Vec<String> = waiting.iter().map(|x| gs(x, "name")).collect();
                    format!("Updated. {} {} for you: new permissions to look at.", names.join(", "), if waiting.len() == 1 { "waits" } else { "wait" })
                } else {
                    "Your apps are up to date.".into()
                };
                st.say.ok(text);
                return match then {
                    Then::App => {
                        let id = st.app_id.clone();
                        let saved = std::mem::take(&mut st.say);
                        let t = update(sh, st, M::App(id));
                        st.say = saved;
                        t
                    }
                    Then::Updates => {
                        let saved = std::mem::take(&mut st.say);
                        let t = update(sh, st, M::Updates);
                        st.say = saved;
                        t
                    }
                    Then::Sources => {
                        let saved = std::mem::take(&mut st.say);
                        let t = update(sh, st, M::Sources);
                        st.say = saved;
                        t
                    }
                };
            }
        },
        M::Install => {
            let name = gs(&st.app, "name");
            return update(sh, st, M::Work(vec!["install".into(), st.app_id.clone()], format!("{name} is installed."), Then::App));
        }
        M::OpenApp => kit::open(&["flatpak", "run", &st.app_id]),
        M::Remove => {
            if st.confirm != "remove" {
                st.confirm = "remove".into();
            } else {
                let name = gs(&st.app, "name");
                return update(sh, st, M::Work(vec!["remove".into(), st.app_id.clone()], format!("{name} was removed."), Then::App));
            }
        }
        M::Website => {
            let h = gs(&st.app, "homepage");
            if !h.is_empty() {
                kit::open(&["xdg-open", &h]);
            }
        }
        M::Adding => st.adding = true,
        M::SrcUrl(u) => st.src_url = u,
        M::AddSource => {
            let u = st.src_url.trim().to_string();
            let base = u.rsplit('/').next().unwrap_or("").trim_end_matches(".flatpakrepo").to_string();
            let name: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
            st.adding = false;
            st.src_url.clear();
            return update(sh, st, M::Work(vec!["source-add".into(), name.clone(), u], format!("{name} was added."), Then::Sources));
        }
        M::Pics(r) => {
            if let Some(m) = r.v.get("pics").and_then(Value::as_object) {
                for (u, f) in m {
                    if let Some(f) = f.as_str() {
                        st.pics.insert(u.clone(), f.to_string());
                    }
                }
            }
            return draw_pics(st);
        }
        M::PicsReady => {}
        M::Shot(d) => {
            let n = ga(&st.app, "screenshots").len();
            if n > 0 {
                st.shot = (st.shot as i64 + d as i64).clamp(0, n as i64 - 1) as usize;
            }
        }
        M::RemoveSource(n) => {
            let key = format!("src:{n}");
            if st.confirm != key {
                st.confirm = key;
            } else {
                return update(sh, st, M::Work(vec!["source-remove".into(), n.clone()], format!("{n} was removed."), Then::Sources));
            }
        }
    }
    Task::none()
}

// ---------------------------------------------------------------- the apps' pictures

/// Every picture the page shows now: (its address, its shape).
fn shown_pics(st: &St) -> Vec<(String, crate::pics::Cut)> {
    let mut v: Vec<(String, crate::pics::Cut)> = Vec::new();
    let icon = |a: &Value, v: &mut Vec<(String, crate::pics::Cut)>| {
        let u = gs(a, "icon");
        if !u.is_empty() {
            v.push((u, crate::pics::ICON));
        }
    };
    for k in ["popular", "trending", "new"] {
        for a in ga(&st.home, k).iter().take(8) {
            icon(a, &mut v);
        }
    }
    for a in &st.apps {
        icon(a, &mut v);
    }
    if !st.app.is_null() {
        icon(&st.app, &mut v);
        for s in ga(&st.app, "screenshots") {
            let u = gs(s, "url");
            if !u.is_empty() {
                v.push((u, crate::pics::SHOT));
            }
        }
    }
    v
}

/// The file a picture is in (a file path is used as it is), once it's known.
fn file_of(st: &St, url: &str) -> Option<String> {
    if url.starts_with('/') {
        return Some(url.to_string());
    }
    st.pics.get(url).cloned()
}

/// Asks hearth-store for the pictures not asked for yet (Flathub's addresses, one a line on its stdin), and reads
/// the ones already saved.
fn want_pics(st: &mut St) -> Task<M> {
    let mut ask: Vec<String> = Vec::new();
    for (u, _) in shown_pics(st) {
        if !u.starts_with('/') && !st.pics.contains_key(&u) && st.asked.insert(u.clone()) {
            ask.push(u);
        }
    }
    let draw = draw_pics(st);
    if ask.is_empty() {
        return draw;
    }
    Task::batch([draw, kit::read_in(hs(&["pics"], 90), Some(ask.join("\n") + "\n"), M::Pics)])
}

/// Reads (off the window's thread) the pictures whose files are known, then draws again.
fn draw_pics(st: &St) -> Task<M> {
    let items: Vec<(String, crate::pics::Cut)> = shown_pics(st).into_iter().filter_map(|(u, c)| file_of(st, &u).map(|f| (f, c))).collect();
    match crate::pics::load(items) {
        Some(f) => Task::perform(f, |_| M::PicsReady),
        None => Task::none(),
    }
}

/// An app's icon in its rounded box (SetStore.qml: 48 in a list, 96 on its page), the store mark until it's read.
fn icon_pic<'a>(p: &hearth_ui::Pal, st: &St, a: &Value, size: f32) -> E<'a, M> {
    let h = file_of(st, &gs(a, "icon")).and_then(|f| crate::pics::get(&f, crate::pics::ICON));
    let inner: E<'a, M> = match h {
        Some(h) => crate::pics::fit(h, size, size),
        None => hearth_ui::ui::glyph(G::Store, p.text, sz(size * 0.45), 1.8),
    };
    iced::widget::container(inner)
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(hearth_ui::ui::boxed(p.panel3, p.r(if size > 60.0 { 20.0 } else { 12.0 }), None))
        .into()
}

fn stars(n: f64) -> String {
    let f = (n.round() as usize).min(5);
    format!("{}{}", "★".repeat(f), "☆".repeat(5 - f))
}

/// One app in a list (SetStore.qml's AppLine): its icon, name (✓ verified), summary · developer, ›.
fn app_line<'a>(p: &hearth_ui::Pal, st: &St, a: &Value) -> E<'a, M> {
    let mut name = gs(a, "name");
    if gb(a, "verified") {
        name.push_str("  ✓");
    }
    let dev = gs(a, "developer");
    let sub = if dev.is_empty() { gs(a, "summary") } else { format!("{} · {dev}", gs(a, "summary")) };
    let msg = M::App(gs(a, "id"));
    crate::nav::control(crate::nav::Kind::Open, vec![Box::new(msg.clone()) as crate::nav::Act], Vec::new(), Vec::new(), String::new(), String::new());
    let nr = crate::nav::item(&name, &sub, false);
    let words = column![tb(name, sz(15.0), p.text), t(sub, sz(12.0), p.muted)].spacing(2.0).width(Length::Fill);
    let line = row![icon_pic(p, st, a, 48.0), words, hearth_ui::ui::glyph(G::Chevron, p.muted, 14.0, 2.0)].spacing(14.0).align_y(iced::Center);
    let c = iced::widget::container(line)
        .id(nr.key)
        .padding(iced::Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 16.0 })
        .width(Length::Fill)
        .style(row_style(*p, p.panel, p.r(10.0), nr.lit));
    iced::widget::mouse_area(c).on_press(msg).interaction(iced::mouse::Interaction::Pointer).into()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut v = crate::secs::SecVec::new();
    v.push(field(&p, "Search Flathub (Enter)", &st.query, M::Query).on_submit(M::Search).into());
    let n_up = ga(&st.ups, "flatpaks").len();
    let tab = |label: String, on: bool, m: M| -> E<'a, M> { button(&p, label, on, Some(m)) };
    v.push(
        Row::new()
            .spacing(8.0)
            .push(tab("Discover".into(), matches!(st.view, View::Discover | View::List), M::Show(View::Discover)))
            .push(tab(if n_up > 0 { format!("Updates ({n_up})") } else { "Updates".into() }, st.view == View::Updates, M::Updates))
            .push(tab("Sources".into(), st.view == View::Sources, M::Sources))
            .push(tab("Drivers".into(), st.view == View::Drivers, M::Drivers))
            .push(pill(&p, "Installed apps", false, Some(M::Go("apps"))))
            .align_y(iced::Center)
            .into(),
    );
    opt(&mut v, busy_view(&p, &st.busy, "Working…"));
    opt(&mut v, say_view(&p, &st.say));
    if st.loading {
        v.push(words(&p, "Asking Flathub…", false));
    }
    match st.view {
        View::Discover => discover(&p, st, &mut v),
        View::List => {
            let mut g: Vec<E<M>> = Vec::new();
            if !st.loading && st.apps.is_empty() {
                g.push(r(G::Search, "Nothing found", "").view(&p));
            }
            for a in &st.apps {
                g.push(app_line(&p, st, a));
            }
            v.push(group(&p, st.cat_label.clone(), g));
        }
        View::App => app_page(&p, st, &mut v),
        View::Updates => updates(&p, st, &mut v),
        View::Sources => sources(&p, st, &mut v),
        View::Drivers => drivers_view(&p, st, &mut v),
    }
    page(v)
}

fn discover<'a>(p: &hearth_ui::Pal, st: &'a St, v: &mut crate::secs::SecVec<'a, M>) {
    // (feature/hearth-store) Hearth's own apps are in the Hearth Store (docs/HEARTH-STORE.md); Flathub's stay here.
    if std::path::Path::new("/usr/bin/hearth-store-app").exists() {
        v.push(group(p, "Hearth's own apps", vec![r(G::Store, "Hearth Store", "Dragongamer9's apps for Hearth: Hearth AI, AI Command Center and more").chev().on(M::HearthStore).view(p)]));
    }
    if st.home.is_null() {
        return;
    }
    let mut cats = Row::new().spacing(8.0);
    for c in ga(&st.home, "categories") {
        cats = cats.push(button(p, gs(c, "label"), false, Some(M::Category(gs(c, "id"), gs(c, "label")))));
    }
    v.push(group(p, "Categories", vec![cats.wrap().vertical_spacing(8.0).into()]));
    for (k, title) in [("popular", "Popular"), ("trending", "Trending"), ("new", "New and updated")] {
        let l = ga(&st.home, k);
        if l.is_empty() {
            continue;
        }
        v.push(group(p, title, l.iter().take(8).map(|a| app_line(p, st, a)).collect()));
    }
}

fn app_page<'a>(p: &hearth_ui::Pal, st: &'a St, v: &mut crate::secs::SecVec<'a, M>) {
    let a = &st.app;
    if a.is_null() {
        return;
    }
    let installed = !ga(a, "installed").is_empty();
    let mut name = gs(a, "name");
    if gb(a, "verified") {
        name.push_str("  ✓");
    }
    let rt = go(a, "ratings");
    let facts: Vec<String> = [
        gs(a, "developer"),
        if rt.is_null() { "No ratings yet".into() } else { format!("{} {} ({} ratings)", stars(gn(rt, "average")), gs(rt, "average"), gs(rt, "total")) },
        { let m = mb(gn(a, "size")); if m.is_empty() { m } else { format!("{m} download") } },
    ]
    .into_iter()
    .filter(|x| !x.is_empty())
    .collect();
    v.push(row![icon_pic(p, st, a, 96.0), column![tb(name, sz(26.0), p.text), t(gs(a, "summary"), sz(15.0), p.text), t(facts.join("  ·  "), sz(13.0), p.muted)].spacing(4.0).width(Length::Fill)].spacing(18.0).align_y(iced::Center).into());
    let mut bs: Vec<E<M>> = Vec::new();
    if !installed {
        bs.push(button(p, "Install", true, Some(M::Install)));
    } else {
        bs.push(button(p, "Open", true, Some(M::OpenApp)));
        bs.push(button(p, if st.confirm == "remove" { "Yes, remove it" } else { "Remove…" }, false, Some(M::Remove)));
    }
    bs.push(pill(p, "Back", false, Some(M::Show(st.back))));
    v.push(buttons(bs));
    // Screenshots (SetStore.qml's shotsCard): side by side, the one in front first; Left / Right move through them.
    let shots = ga(a, "screenshots");
    if !shots.is_empty() {
        let n = shots.len();
        let at = st.shot.min(n - 1);
        let h = sz(356.0);
        let w = (h * 16.0 / 9.0).round();
        let mut strip = Row::new().spacing(12.0);
        for s in shots.iter().skip(at) {
            let pic = file_of(st, &gs(s, "url")).and_then(|f| crate::pics::get(&f, crate::pics::SHOT));
            let inner: E<M> = match pic {
                Some(hd) => crate::pics::fit(hd, w, h),
                None => t(if gs(s, "caption").is_empty() { "Screenshot".to_string() } else { gs(s, "caption") }, sz(13.0), p.muted).into(),
            };
            strip = strip.push(iced::widget::container(inner).center_x(Length::Fixed(w)).center_y(Length::Fixed(h)).style(hearth_ui::ui::boxed(p.panel3, p.r(10.0), None)));
        }
        let (prev, next) = (if at > 0 { Some(M::Shot(-1)) } else { None }, if at + 1 < n { Some(M::Shot(1)) } else { None });
        crate::nav::control(crate::nav::Kind::Choice, next.iter().cloned().map(|m| Box::new(m) as crate::nav::Act).collect(), prev.iter().cloned().map(|m| Box::new(m) as crate::nav::Act).collect(), next.iter().cloned().map(|m| Box::new(m) as crate::nav::Act).collect(), String::new(), format!("{} of {n}", at + 1));
        let nr = crate::nav::item("Screenshots", &format!("{} of {n}", at + 1), false);
        let card = column![
            iced::widget::container(strip).height(Length::Fixed(h)).width(Length::Fill).clip(true),
            row![t(format!("{} of {n}", at + 1), sz(12.0), p.muted).width(Length::Fill), button(p, "‹", false, prev), button(p, "›", false, next)].spacing(8.0).align_y(iced::Center),
        ]
        .spacing(10.0);
        v.push(iced::widget::container(card).id(nr.key).padding(12.0).width(Length::Fill).style(row_style(*p, p.panel, p.r(12.0), nr.lit)).into());
    }
    v.push(t(gs(a, "description"), sz(14.0), p.text).into());
    let reviews = ga(a, "reviews");
    if !reviews.is_empty() {
        v.push(group(p, "Reviews", reviews.iter().map(|x| r(G::User, format!("{}  {}", stars(gn(x, "stars")), gs(x, "summary")), format!("{} · {}", gs(x, "who"), gs(x, "text"))).view(p)).collect()));
    }
    let mut about = vec![
        r(G::Info, "License", "").value({ let l = gs(a, "license"); if l.is_empty() { "Not given".into() } else { l } }).view(p),
        r(G::Store, "App id", "").value(st.app_id.clone()).view(p),
    ];
    let home = gs(a, "homepage");
    if !home.is_empty() {
        about.push(r(G::Link, "Website", home).chev().on(M::Website).view(p));
    }
    v.push(group(p, "About", about));
}

fn updates<'a>(p: &hearth_ui::Pal, st: &'a St, v: &mut crate::secs::SecVec<'a, M>) {
    let mut g: Vec<E<M>> = Vec::new();
    let on = not_false(&st.prefs, "notifyUpdates");
    g.push(sw(p, G::Moon, "Tell me when app updates are there", if on { "A Hearth notice with Update all, every few hours when there's something new" } else { "Off: look here yourself" }, on, Some(M::Notify(!on))));
    let fl = ga(&st.ups, "flatpaks");
    let asking = fl.iter().filter(|f| !ga(f, "newPerms").is_empty()).count();
    if !st.ups.is_null() && fl.len() > asking {
        let n = fl.len() - asking;
        let mut sub = format!("{n} Flatpak {}", if n == 1 { "update" } else { "updates" });
        if asking > 0 {
            sub.push_str(&format!(" · {asking} {} for you: new permissions", if asking == 1 { "waits" } else { "wait" }));
        }
        g.push(r(G::Update, "Update all apps", sub).on(M::Work(vec!["update".into()], String::new(), Then::Updates)).view(p));
    }
    if !st.ups.is_null() && fl.is_empty() {
        g.push(r(G::Check, "Your apps are up to date", "").view(p));
    }
    for f in fl {
        let id = gs(f, "id");
        let name = if gs(f, "name").is_empty() { id.clone() } else { gs(f, "name") };
        let np = ga(f, "newPerms");
        let mut sub = [gs(f, "version"), gs(f, "size"), if gs(f, "where") == "system" { "for everyone".into() } else { "yours".into() }, gs(f, "origin")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
        if !np.is_empty() {
            sub.push_str(&format!(" · asks for {} new {}", np.len(), if np.len() == 1 { "permission" } else { "permissions" }));
        }
        let row_msg = if np.is_empty() { M::Work(vec!["update".into(), id.clone()], format!("{name} is up to date."), Then::Updates) } else { M::Review(id.clone()) };
        let val = if np.is_empty() { "Update" } else if st.reviewing == id { "Hide" } else { "Look first…" };
        g.push(r(G::Store, name.clone(), sub).chip(if np.is_empty() { "" } else { "New permissions" }, Chip::Warn).value(val).on(row_msg).view(p));
        if st.reviewing == id {
            let mut c = Column::new().spacing(6.0).push(tb(format!("The new version of {name} asks to:"), sz(15.0), p.text));
            for x in np {
                c = c.push(t(format!("•  {}", gs(x, "label")), sz(14.0), p.text));
            }
            c = c.push(t("It can't do these now. Update only if you trust it with them; you can take them away later in Settings › Apps.", sz(12.0), p.muted));
            c = c.push(buttons(vec![button(p, "Update and allow", true, Some(M::Work(vec!["update".into(), id.clone(), "--allow-new".into()], format!("{name} is up to date."), Then::Updates))), pill(p, "Not now", false, Some(M::Review(id.clone())))]));
            g.push(iced::widget::container(card(p, c)).padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 24.0 }).into());
        }
    }
    g.push(r(G::Update, "Hearth OS updates", "The system itself: Settings › Updates").chev().on(M::Go("updates")).view(p));
    g.push(r(G::Info, "Firmware", "Your PC's and devices' firmware: Settings › Firmware").chev().on(M::Go("firmware")).view(p));
    v.push(group(p, "Updates", g));
}

fn sources<'a>(p: &hearth_ui::Pal, st: &'a St, v: &mut crate::secs::SecVec<'a, M>) {
    let mut g: Vec<E<M>> = Vec::new();
    for s in &st.srcs {
        let name = gs(s, "name");
        let mut title = if gs(s, "title").is_empty() { name.clone() } else { gs(s, "title") };
        if !gb(s, "enabled") {
            title.push_str(" (off)");
        }
        let yours = gs(s, "where") == "yours";
        let key = format!("src:{name}");
        let row = r(G::Link, title, format!("{} · {}", gs(s, "url"), if gs(s, "where") == "system" { "for everyone" } else { "yours" })).danger_if(st.confirm == key);
        let row = if yours { row.value(if st.confirm == key { "Remove it" } else { "Remove…" }).on(M::RemoveSource(name)) } else { row };
        g.push(row.view(p));
    }
    if !st.adding {
        g.push(r(G::Plus, "Add a source…", "A .flatpakrepo file's https:// address (it carries the key that checks its apps)").on(M::Adding).view(p));
    } else {
        g.push(field(p, "https://…/name.flatpakrepo (Enter adds it)", &st.src_url, M::SrcUrl).on_submit(M::AddSource).into());
    }
    v.push(group(p, "Sources", g));
}

// ---------------------------------------------------------------- Drivers, the Fedora edition's way

/// The PC's parts and the drivers in use (lspci -k), the firmware and graphics packages installed (rpm), and firmware
/// the kernel asked for and couldn't load. Only reads.
pub const FEDORA_DRIVERS: &str = "echo '#@pci'; lspci -k 2>/dev/null; echo '#@rpm'; rpm -q --qf '%{NAME}\\t%{VERSION}-%{RELEASE}\\n' linux-firmware amd-gpu-firmware intel-gpu-firmware nvidia-gpu-firmware amd-ucode-firmware microcode_ctl mesa-dri-drivers mesa-vulkan-drivers nvidia-driver akmod-nvidia kmod-nvidia xorg-x11-drv-nvidia 2>/dev/null | grep -v 'not installed'; echo '#@cpu'; grep -m1 vendor_id /proc/cpuinfo; echo '#@fw'; timeout 10 journalctl -k -b -q --no-pager -o cat --grep 'firmware' 2>/dev/null | grep -iE 'failed|error' | head -20; echo '#@nv'; test -e /proc/driver/nvidia && echo loaded";

fn drivers(r: &Res) -> Value {
    serde_json::json!({ "rows": parse_drivers(&r.out) })
}

/// Parsed from FEDORA_DRIVERS' sections: rows {device, kind, driver, package, installed, note}.
pub fn parse_drivers(out: &str) -> Vec<Value> {
    let s = crate::data::sections(out);
    let mut rows = Vec::new();
    let empty = String::new();
    let pci = s.get("pci").unwrap_or(&empty);
    let mut cur: Option<(String, String)> = None;
    let mut driver = String::new();
    let flush = |cur: &mut Option<(String, String)>, driver: &mut String, rows: &mut Vec<Value>| {
        if let Some((cls, name)) = cur.take() {
            let c = cls.to_lowercase();
            let kind = if c.contains("vga") || c.contains("3d") || c.contains("display") {
                "Graphics"
            } else if c.contains("network") || c.contains("ethernet") || c.contains("wireless") {
                "Network"
            } else if c.contains("audio") || c.contains("multimedia") {
                "Sound"
            } else {
                ""
            };
            if !kind.is_empty() {
                rows.push(serde_json::json!({"device": format!("{kind}: {name}"), "kind": kind, "driver": driver.clone()}));
            }
        }
        driver.clear();
    };
    for l in pci.lines() {
        if !l.starts_with(char::is_whitespace) && l.len() > 8 {
            flush(&mut cur, &mut driver, &mut rows);
            let rest = &l[l.find(' ').map(|i| i + 1).unwrap_or(0)..];
            let (cls, name) = rest.split_once(": ").unwrap_or((rest, ""));
            let name = name.split(" (rev").next().unwrap_or(name).to_string();
            cur = Some((cls.to_string(), name));
        } else if let Some(d) = l.trim().strip_prefix("Kernel driver in use: ") {
            driver = d.trim().to_string();
        }
    }
    flush(&mut cur, &mut driver, &mut rows);
    for l in s.get("rpm").unwrap_or(&empty).lines() {
        if let Some((n, ver)) = l.split_once('\t') {
            let what = if n.contains("nvidia") {
                "Nvidia graphics"
            } else if n.contains("ucode") || n.contains("microcode") {
                "Processor fixes"
            } else if n.contains("mesa") {
                "Graphics (Mesa: AMD, Intel and others)"
            } else {
                "Firmware for Wi-Fi, Bluetooth, sound and graphics"
            };
            rows.push(serde_json::json!({"device": what, "kind": "Package", "package": n, "installed": ver}));
        }
    }
    for l in s.get("fw").unwrap_or(&empty).lines().filter(|l| !l.trim().is_empty()) {
        rows.push(serde_json::json!({"device": "Firmware the kernel asked for", "kind": "Missing", "note": l.trim()}));
    }
    rows
}

fn drivers_view<'a>(p: &hearth_ui::Pal, st: &'a St, v: &mut crate::secs::SecVec<'a, M>) {
    v.push(words(p, "What your PC's parts need to work their best: graphics drivers, firmware for Wi-Fi, Bluetooth and sound, and fixes for the processor. Hearth OS comes with most of them.", false));
    let d = &st.drivers;
    if d.is_null() {
        v.push(words(p, "Looking at your PC…", false));
        return;
    }
    v.push(t("This edition's system is an image: drivers come with Hearth OS's updates (Settings › Updates).", sz(13.0), p.flame).into());
    let rows = ga(d, "rows");
    let missing = rows.iter().filter(|x| gs(x, "kind") == "Missing").count();
    let mut g: Vec<E<M>> = Vec::new();
    if rows.is_empty() {
        g.push(r(G::Check, "Nothing extra is needed", "Your PC's parts work with what Hearth OS has").view(p));
    }
    for x in rows {
        let kind = gs(x, "kind");
        let gl = match kind.as_str() {
            "Graphics" => G::Display,
            "Network" => G::Wifi,
            "Sound" => G::Sound,
            "Missing" => G::Info,
            _ => G::Drive,
        };
        let row = match kind.as_str() {
            "Package" => r(gl, format!("{}  ·  {}", gs(x, "device"), gs(x, "package")), format!("Installed ({})", gs(x, "installed"))).chip("Installed", Chip::Ok),
            "Missing" => r(gl, gs(x, "device"), format!("{} · it comes with Hearth OS's updates, if the parts' makers publish it", gs(x, "note"))).chip("Not loaded", Chip::Warn),
            _ => {
                let drv = gs(x, "driver");
                if drv.is_empty() {
                    r(gl, gs(x, "device"), "No driver in use").chip("Not in use", Chip::Warn)
                } else {
                    r(gl, gs(x, "device"), format!("Driver in use: {drv}")).chip("In use", Chip::Ok)
                }
            }
        };
        g.push(row.view(p));
    }
    let title = if missing > 0 { format!("Drivers · {missing} firmware not loaded") } else { "Drivers · all installed".into() };
    v.push(group(p, title, g));
    v.push(r(G::Update, "Check again", "After plugging something in").on(M::Drivers).view(p));
    v.push(r(G::Info, "Firmware", "Updates for your PC's own firmware: Settings › Firmware").chev().on(M::Go("firmware")).view(p));
}
