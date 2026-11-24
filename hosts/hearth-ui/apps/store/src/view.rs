//! The Store's pages, drawn as the approved mockups show them (docs/mockups/hearth-store/store-1…9; checklist
//! docs/mockups/checklists/hearth-store.md): the title bar, the sidebar, each page, the ⋯ menu, the uninstall dialog,
//! the controller look (store-7) and the buttons' hints at the bottom.

use crate::look::{self, app_icon, btn, bullets, card, chip, click, f, hint, hint2, icon_box, kv, ring, sec, seg, srow, switch, tone, z, Chip, B, E, ORANGE, ORANGE2, PURPLE};
use crate::{w11, App, JobKind, Msg, Page, State, SEARCH_ID};
use hearth_store::catalog::{Entry, Kind};
use hearth_store::{mb, parts, paths, short_version};
use hearth_ui::ui::{self, gap, t, t1, tb, tm, ts};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, image, row, scrollable, stack, text_input, Column, Row, Space};
use iced::{Color, Length, Theme};

/// Something the keyboard and controller can be on: its id, its row (up / down move between rows), what Enter / A do.
pub struct Target {
    pub id: String,
    pub row: u16,
    pub enter: Option<Msg>,
}

fn tg(id: impl Into<String>, row: u16, enter: Option<Msg>) -> Target {
    Target { id: id.into(), row, enter }
}

fn lit(a: &App, id: &str) -> bool {
    a.focus == id && a.dlg.is_none() && !a.menu
}

/// The one thing pressing an app does from a list: install, update (or open, for an app that updates itself), open.
pub fn primary(e: &Entry, st: &State) -> Option<Msg> {
    match st {
        State::Get(_) => Some(Msg::Start(e.id.clone(), JobKind::Install)),
        State::Ready(..) if e.self_updates => Some(Msg::Launch(e.id.clone())),
        State::Ready(..) => Some(Msg::Start(e.id.clone(), JobKind::Update)),
        State::UpToDate(_) | State::BuiltIn => Some(Msg::Launch(e.id.clone())),
        State::Coming => Some(Msg::OpenApp(e.id.clone())),
    }
}

/// The home page's featured app: a new version ready, else a new app to install, else one of yours.
pub fn featured(a: &App) -> Option<(Entry, State)> {
    let all: Vec<(Entry, State)> = a.cat.apps.iter().map(|e| (e.clone(), a.state(e))).collect();
    all.iter().find(|(e, s)| matches!(s, State::Ready(..)) && !e.self_updates).cloned()
        .or_else(|| all.iter().find(|(_, s)| matches!(s, State::Ready(..))).cloned())
        .or_else(|| all.iter().find(|(_, s)| matches!(s, State::Get(_))).cloned())
        .or_else(|| all.iter().find(|(_, s)| matches!(s, State::UpToDate(_))).cloned())
}

/// What's new is shown under the first update by default (store-3), and wherever it was asked for.
pub fn new_shown(a: &App, id: &str) -> bool {
    if a.shown_new.contains(id) {
        return true;
    }
    if a.hidden_new.contains(id) {
        return false;
    }
    // The first update on the page shows what's new: a quick update first (they're listed first), else an app's.
    if let Some((e, _)) = a.part_offers().first() {
        return e.id == id;
    }
    a.ready().iter().find(|(e, _, _)| !e.self_updates).map(|(e, _, _)| e.id == id).unwrap_or(false)
}

/// Hearth's own apps and Mantel that went back to the built-in one (their update didn't start twice).
fn parts_gave_up(a: &App) -> Vec<Entry> {
    a.cat.apps.iter().filter(|e| e.part.is_some() && a.parts.get(&e.id).map(|h| h.gave_up()).unwrap_or(false)).cloned().collect()
}

fn installed(a: &App) -> Vec<Entry> {
    a.cat.apps.iter().filter(|e| a.inst.contains_key(&e.id)).cloned().collect()
}

fn matches_search(a: &App, e: &Entry) -> bool {
    let q = a.search.trim().to_lowercase();
    q.is_empty() || [&e.name, &e.short, &e.card, &e.tagline].iter().any(|s| s.to_lowercase().contains(&q))
}

/// Controller home (store-7): the featured app, then three more big tiles, then the rest.
fn pad_rows(a: &App) -> (Option<(Entry, State)>, Vec<Entry>, Vec<Entry>) {
    let hero = featured(a);
    let hid = hero.as_ref().map(|h| h.0.id.clone()).unwrap_or_default();
    let mut top: Vec<Entry> = installed(a).into_iter().filter(|e| e.id != hid).collect();
    top.extend(a.cat.apps.iter().filter(|e| e.kind == Kind::BuiltIn).cloned());
    top.truncate(3);
    let mut more: Vec<Entry> = a.cat.apps.iter().filter(|e| e.id != hid && !top.iter().any(|x| x.id == e.id) && matches!(a.state(e), State::Coming)).cloned().collect();
    let rest: Vec<Entry> = a.cat.apps.iter().filter(|e| e.id != hid && !top.iter().any(|x| x.id == e.id) && !more.iter().any(|x| x.id == e.id)).cloned().collect();
    more.extend(rest);
    (hero, top, more)
}

pub fn targets(a: &App) -> Vec<Target> {
    let mut v = Vec::new();
    match &a.page {
        Page::Home if a.pad => {
            let (hero, top, more) = pad_rows(a);
            if let Some((e, s)) = hero {
                v.push(tg("hero", 0, primary(&e, &s)));
            }
            for e in top {
                v.push(tg(format!("top-{}", e.id), 0, Some(Msg::OpenApp(e.id.clone()))));
            }
            for e in more {
                v.push(tg(format!("more-{}", e.id), 1, Some(Msg::OpenApp(e.id.clone()))));
            }
        }
        Page::Home => {
            if a.offline() {
                v.push(tg("try-again", 0, Some(Msg::TryAgain)));
            } else if let Some((e, s)) = featured(a) {
                if a.search.is_empty() {
                    v.push(tg("hero", 1, primary(&e, &s)));
                }
            }
            if a.search.is_empty() {
                for e in installed(a).into_iter().take(2) {
                    let s = a.state(&e);
                    v.push(tg(format!("your-{}", e.id), 2, primary(&e, &s)));
                }
            }
            for (i, e) in a.cat.apps.iter().filter(|e| matches_search(a, e)).enumerate() {
                v.push(tg(format!("tile-{}", e.id), 3 + (i / 4) as u16, Some(Msg::OpenApp(e.id.clone()))));
            }
            v.push(tg("flathub", 90, Some(Msg::OpenFlathub)));
        }
        Page::Yours => {
            for (i, e) in installed(a).into_iter().enumerate() {
                v.push(tg(format!("y-{}", e.id), i as u16, Some(Msg::OpenApp(e.id.clone()))));
            }
        }
        Page::BuiltIn => {
            for (i, e) in a.cat.apps.iter().filter(|e| e.kind == Kind::BuiltIn).enumerate() {
                v.push(tg(format!("b-{}", e.id), i as u16, Some(Msg::Launch(e.id.clone()))));
            }
        }
        Page::Updates => {
            let mut r = 0u16;
            for (e, _) in a.part_offers() {
                v.push(tg(format!("upd-{}", e.id), r, Some(Msg::PartUpdate(e.id.clone()))));
                r += 1;
            }
            for (e, _, _) in a.ready().into_iter().filter(|(e, _, _)| !e.self_updates) {
                v.push(tg(format!("upd-{}", e.id), r, Some(Msg::Start(e.id.clone(), JobKind::Update))));
                r += 1;
            }
            for e in installed(a).into_iter().filter(|e| e.self_updates) {
                let s = a.state(&e);
                v.push(tg(format!("self-{}", e.id), r, if matches!(s, State::Ready(..)) { Some(Msg::Launch(e.id.clone())) } else { None }));
                r += 1;
            }
            for e in parts_gave_up(a) {
                v.push(tg(format!("retry-{}", e.id), r, Some(Msg::PartRetry(e.id.clone()))));
                v.push(tg(format!("rm-{}", e.id), r, Some(Msg::PartRemove(e.id.clone()))));
                r += 1;
            }
            v.push(tg("test-link", r, Some(Msg::Go(Page::Settings))));
        }
        Page::Settings => {
            v.push(tg("s-daily", 0, Some(Msg::SetDaily(!a.prefs.daily))));
            v.push(tg("s-notify", 1, Some(Msg::SetFlag("notify", !a.prefs.notify))));
            v.push(tg("s-metered", 2, Some(Msg::SetFlag("metered", !a.prefs.metered))));
            v.push(tg("s-test", 3, Some(Msg::SetFlag("test", !a.prefs.test))));
            v.push(tg("r-flathub", 4, Some(Msg::OpenFlathub)));
            v.push(tg("r-folder", 4, Some(Msg::OpenFolder)));
            v.push(tg("r-updates", 4, Some(Msg::OpenHearthUpdates)));
        }
        Page::App(id) => {
            if let Some(e) = a.entry(id) {
                let s = a.state(e);
                v.push(tg("open", 0, primary(e, &s)));
                if a.inst.contains_key(id) {
                    v.push(tg("more", 0, Some(Msg::Menu)));
                }
                if e.part.is_some() {
                    v.push(tg("builtin-one", 1, Some(Msg::PartRemove(id.clone()))));
                    v.push(tg("whats-new", 1, Some(Msg::ToggleNew(id.clone()))));
                }
            }
        }
        Page::Work => {
            if let Some(j) = &a.job {
                if j.done() {
                    v.push(tg("open", 0, Some(Msg::Launch(j.id.clone()))));
                    v.push(tg("back", 0, Some(Msg::Back)));
                } else if !j.error.is_empty() {
                    v.push(tg("back", 0, Some(Msg::Back)));
                } else if j.can_cancel() {
                    v.push(tg("cancel", 0, Some(Msg::Cancel)));
                }
            }
        }
        Page::Apps | Page::Tests | Page::System => {}
        Page::Repair(_) => {
            v.push(tg("repair-now", 0, Some(Msg::RepairNow)));
            v.push(tg("not-now", 0, Some(Msg::Back)));
        }
    }
    v
}

pub fn first_target(a: &App) -> String {
    targets(a).first().map(|t| t.id.clone()).unwrap_or_default()
}

/// A download's size, worded as the mockups word it: "about 120 MB" for an app the Store installs and updates,
/// "129 MB" for one that updates itself.
fn dl_size(e: &Entry, n: u64) -> String {
    mb(n, !e.self_updates)
}

fn ago(a: &App, ts: i64) -> String {
    let d = (a.now - ts).max(0);
    if ts <= 0 {
        "not yet".into()
    } else if d < 60 {
        "just now".into()
    } else if d < 3600 {
        format!("{} min ago", d / 60)
    } else if d < 86_400 {
        format!("{} h ago", d / 3600)
    } else {
        format!("on {}", hearth_ui::time::day(ts))
    }
}

fn ago_long(a: &App, ts: i64) -> String {
    let d = (a.now - ts).max(0);
    if d < 60 {
        "just now".into()
    } else if d < 120 {
        "a minute ago".into()
    } else if d < 3600 {
        format!("{} minutes ago", d / 60)
    } else {
        day_time(ts)
    }
}

/// "today at 9:12 AM", "Nov 12 at 9:12 AM".
fn day_time(ts: i64) -> String {
    let now = hearth_ui::time::local(hearth_ui::time::now());
    let t = hearth_ui::time::local(ts);
    if (t.year, t.mon, t.day) == (now.year, now.mon, now.day) {
        format!("today at {}", hearth_ui::time::clock(ts))
    } else {
        format!("{} at {}", hearth_ui::time::day(ts), hearth_ui::time::clock(ts))
    }
}

pub fn released(date: &str) -> String {
    match hearth_ui::time::parse_date(date) {
        Some((y, m, d)) => format!("released {} {d}, {y}", hearth_ui::time::month(m)),
        None => String::new(),
    }
}

fn this_pc(a: &App) -> String {
    let v = if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) };
    if a.edition.is_empty() { v } else { format!("{v} · {}", a.edition) }
}

pub fn day_time_pub(ts: i64) -> String {
    day_time(ts)
}
pub fn ago_pub(a: &App, ts: i64) -> String {
    ago(a, ts)
}
pub fn this_pc_pub(a: &App) -> String {
    this_pc(a)
}
pub fn hero_pub<'a>(a: &'a App, e: &Entry, st: &State) -> E<'a, Msg> {
    hero(a, e, st)
}

fn page_name(pg: &Page, a: &App) -> String {
    match pg {
        Page::Home => "Home".into(),
        Page::Apps => "Apps".into(),
        Page::Tests => "Tests".into(),
        Page::Yours => "Your apps".into(),
        Page::Updates => "Updates".into(),
        Page::BuiltIn => "Hearth parts".into(),
        Page::System => "Library › Hearth OS".into(),
        Page::Settings => "Store settings".into(),
        Page::App(id) => a.entry(id).map(|e| e.name.clone()).unwrap_or_default(),
        Page::Work => "Installing".into(),
        Page::Repair(_) => "Check and repair".into(),
    }
}

// ------------------------------------------------------------------ the frame
/// The Store's blocks (hearth_ui::blocks).
pub const BLOCKS: &[hearth_ui::blocks::PageDef] = &[hearth_ui::blocks::PageDef {
    id: "window",
    title: "Hearth Store",
    // The Windows 11 frame draws the rail and the title bar itself (store-win11); the page is the block here.
    blocks: &[hearth_ui::blocks::bl("page", "The page", 0, 0, 12, 12)],
}];

/// The Store with Describe it's box under it while it's open.
pub fn shown(a: &App) -> E<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    if a.pad {
        return ui::edges(pad_frame(a), Msg::Resize);
    }
    // The Windows 11 Store's frame (store-win11): the title bar with the search, the rail, the page on its layer
    let body: E<Msg> = match &a.page {
        Page::Home => w11::home(a),
        Page::Apps => w11::apps(a),
        Page::Updates | Page::Yours => w11::library(a),
        Page::BuiltIn => w11::parts(a),
        Page::System => w11::system(a),
        Page::Tests => w11::tests(a),
        Page::Settings => w11::settings(a),
        Page::App(id) if a.dlg.is_none() => match a.entry(id) {
            Some(e) if e.test => w11::test_page(a, e),
            Some(e) if e.part.is_none() => w11::app_page(a, e),
            _ => page_body(a),
        },
        _ => page_body(a),
    };
    let body: E<Msg> = if a.note.is_empty() || matches!(a.page, Page::App(_) | Page::Work | Page::Repair(_) | Page::Settings | Page::BuiltIn) {
        body
    } else {
        let c = if a.note_bad { look::BAD } else { ORANGE2 };
        column![container(t(a.note.clone(), f(15.0), c)).padding(look::pad(0.0, 0.0, z(14.0), 0.0)), body].into()
    };
    let _ = (tn, sidebar as fn(&App) -> E<'_, Msg>, title_bar as fn(&App) -> E<'_, Msg>, bottom as fn(&App) -> E<'_, Msg>);
    // Every Hearth app made of blocks (hearth_ui::blocks): the page moved, sized or restyled; as it was when nothing is.
    let body = hearth_ui::arrange::parts(&a.describe, &p, "window", vec![("page", body)], |ps| ps.take("page"), Msg::Describe);
    ui::edges(overlays(a, w11::frame(a, body)), Msg::Resize)
}

/// The ⋯ menu and the uninstall dialog, over the page (the sidebar stays as it is, as in store-2 and store-6).
fn overlays<'a>(a: &'a App, area: E<'a, Msg>) -> E<'a, Msg> {
    let area = if a.menu { stack![area, menu(a)].into() } else { area };
    match &a.dlg {
        Some(_) => ui::dialog(area, uninstall_dialog(a), Some(Msg::DlgCancel)),
        None => area,
    }
}

fn page_body(a: &App) -> E<'_, Msg> {
    let content: E<Msg> = match &a.page {
        Page::Home => home(a),
        Page::Apps => w11::apps(a),
        Page::Tests => w11::tests(a),
        Page::Yours => yours(a),
        Page::Updates => updates(a),
        Page::BuiltIn => built_in(a),
        Page::System => w11::system(a),
        Page::Settings => settings(a),
        Page::App(id) => match a.entry(id) {
            Some(e) => if a.dlg.is_some() { app_head(a, e) } else { app_page(a, e) },
            None => t("That app isn't in the list.", f(16.0), tone(&a.p).muted).into(),
        },
        Page::Work => work(a),
        Page::Repair(id) => match a.entry(id) {
            Some(e) => repair(a, e),
            None => gap(1.0, 1.0),
        },
    };
    if a.note.is_empty() {
        content
    } else {
        let c = if a.note_bad { look::BAD } else { ORANGE2 };
        column![container(t(a.note.clone(), f(16.0), c)).padding(look::pad(0.0, 0.0, z(14.0), 0.0)), content].into()
    }
}

fn title_bar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let name = row![gap(z(22.0), 1.0), look::mark(z(30.0)), gap(z(14.0), 1.0), ts("Hearth Store", f(18.0), tn.text), gap(z(10.0), 1.0), t("· Hearth", f(15.0), tn.muted)].align_y(iced::Center);
    // a right-click on the bar opens Describe it (and from it, Move and resize blocks)
    let drag = iced::widget::mouse_area(container(name).width(Length::Fill).center_y(Length::Fixed(z(56.0)))).on_press(Msg::Drag).on_double_click(Msg::Maximize).on_right_press(Msg::Describe(hearth_ui::describe::DMsg::Open));
    let wbtn = |g: ui::G, m: Msg| -> E<'static, Msg> {
        click(container(ui::glyph(g, tn.muted, 10.0, 1.8)).center_x(Length::Fixed(z(56.0))).center_y(Length::Fixed(z(56.0))), Some(m))
    };
    container(row![drag, wbtn(ui::G::Minimize, Msg::Minimize), wbtn(ui::G::Maximize, Msg::Maximize), wbtn(ui::G::Close, Msg::Close), gap(z(10.0), 1.0)].align_y(iced::Center))
        .width(Length::Fill)
        .style(look::bx(tn.side, 0.0, None))
        .into()
}

fn search_box(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let field = text_input("Search apps", &a.search)
        .id(SEARCH_ID)
        .on_input(Msg::Search)
        .size(f(17.0))
        .padding(0.0)
        .style(move |_t: &Theme, _s| text_input::Style {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
            icon: tn.muted,
            placeholder: tn.muted,
            value: tn.text,
            selection: Pal::a(ORANGE, 0.35),
        });
    let cap = container(tm("Y", f(15.0), tn.muted)).padding(look::pad(1.0, z(7.0), 1.0, z(7.0))).style(look::bx(Color::TRANSPARENT, 4.0, Some((Pal::mix(tn.line, tn.text, 0.12), 1.0))));
    container(row![t("⌕", f(14.0), tn.muted), field.width(Length::Fill), cap].spacing(z(10.0)).align_y(iced::Center))
        .padding(look::pad(z(12.0), z(14.0), z(12.0), z(14.0)))
        .style(look::bx(tn.page, 7.0, Some((tn.line, 1.0))))
        .into()
}

fn sidebar(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n_up = a.ready().len() + a.part_offers().len();
    let mut nav = Column::new().spacing(z(6.0));
    for (pg, icon, label) in [(Page::Home, "⌂", "Home"), (Page::Yours, "▦", "Your apps"), (Page::Updates, "⟳", "Updates"), (Page::BuiltIn, "▣", "Built into Hearth"), (Page::Settings, "⚙", "Store settings")] {
        let here = a.page == pg
            || (pg == Page::Yours && matches!(&a.page, Page::App(id) | Page::Repair(id) if a.inst.contains_key(id)))
            || (pg == Page::Home && matches!(&a.page, Page::App(id) if !a.inst.contains_key(id)))
            || (pg == Page::Updates && a.page == Page::Work);
        let bar = container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(z(22.0))).style(look::bx(if here { ORANGE } else { Color::TRANSPARENT }, 2.0, None));
        let mut r = row![container(t(icon, f(16.0), tn.muted)).center_x(Length::Fixed(z(22.0))), gap(z(14.0), 1.0), t(label, f(18.0), Pal::mix(tn.text, tn.muted, 0.12))].align_y(iced::Center);
        if pg == Page::Updates && n_up > 0 {
            r = r.push(Space::new().width(Length::Fill)).push(container(tb(n_up.to_string(), f(13.0), look::INK)).padding(look::pad(1.0, z(9.0), 1.0, z(9.0))).style(look::bx(ORANGE, 8.0, None)));
        }
        let item = container(r).padding(look::pad(z(13.0), z(16.0), z(13.0), z(16.0))).width(Length::Fill).style(look::bx(if here { tn.card2 } else { Color::TRANSPARENT }, 7.0, None));
        nav = nav.push(row![bar, click(item, Some(Msg::Go(pg)))].align_y(iced::Center).spacing(0.0));
    }
    let foot = column![
        t(format!("This PC: {}", this_pc(a)), f(14.0), Pal::mix(tn.text, tn.muted, 0.35)),
        t("Apps for Hearth OS: Dragongamer9's own, and open-source apps from other makers.", f(14.0), tn.muted)
    ]
    .spacing(3.0);
    container(column![search_box(a), gap(1.0, z(14.0)), nav, Space::new().height(Length::Fill), container(foot).padding(z(12.0))])
        .padding(look::pad(z(22.0), z(18.0), z(22.0), z(14.0)))
        .width(Length::Fixed(z(330.0)))
        .height(Length::Fill)
        .style(look::bx(tn.side, 0.0, None))
        .into()
}

fn bottom(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let pages = || hint2(&p, "LB", "RB", "Previous / next page");
    let (l, r): (Vec<E<Msg>>, Vec<E<Msg>>) = if a.dlg.is_some() {
        (vec![hint(&p, "A", "Toggle"), hint(&p, "B", "Cancel")], vec![hint(&p, "X", "Uninstall")])
    } else {
        match &a.page {
            Page::Home if a.offline() => (vec![hint(&p, "A", "Try again"), hint(&p, "B", "Back")], vec![pages()]),
            Page::Home | Page::Yours | Page::BuiltIn | Page::Apps | Page::Tests => (vec![hint(&p, "A", "Select"), hint(&p, "B", "Back"), hint(&p, "Y", "Search"), hint(&p, "X", "Update all")], vec![pages()]),
            Page::System => (vec![hint(&p, "A", "Select"), hint(&p, "B", "Back")], vec![pages()]),
            Page::Updates => (vec![hint(&p, "A", "Update"), hint(&p, "B", "Back"), hint(&p, "X", "Update all"), hint(&p, "Y", "What's new")], vec![pages()]),
            Page::Settings => (vec![hint(&p, "A", "Toggle"), hint(&p, "B", "Back")], vec![pages()]),
            Page::App(_) => (vec![hint(&p, "A", "Select"), hint(&p, "B", "Back"), hint(&p, "≡", "More")], vec![hint2(&p, "LB", "RB", "Previous / next app")]),
            Page::Work => (vec![hint(&p, "B", "Back (it carries on)")], vec![hint(&p, "≡", "Show details")]),
            Page::Repair(_) => (vec![hint(&p, "A", "Repair now"), hint(&p, "B", "Back")], vec![hint(&p, "Y", "Show every file")]),
        }
    };
    look::hints(&p, l, r)
}

fn h1<'a>(p: &Pal, s: impl Into<String>, right: Option<E<'a, Msg>>) -> E<'a, Msg> {
    let tn = tone(p);
    let mut r = row![ts(s.into(), f(40.0), tn.text)].align_y(iced::Center);
    if let Some(x) = right {
        r = r.push(Space::new().width(Length::Fill)).push(x);
    }
    container(r).padding(look::pad(0.0, 0.0, z(22.0), 0.0)).width(Length::Fill).into()
}

// ------------------------------------------------------------------ 1. Home (store-1), offline / empty (store-8)
fn status_chip<'a>(a: &App, e: &Entry, st: &State, focused_btn: bool) -> E<'a, Msg> {
    let p = a.p;
    match st {
        State::UpToDate(_) => chip(&p, "Installed", Chip::Ok),
        State::Ready(..) => click(chip(&p, if e.self_updates { "Open to update" } else { "Update" }, Chip::Up), primary(e, st)),
        State::BuiltIn if e.test => chip(&p, "TEST", Chip::Test),
        State::BuiltIn => chip(&p, "Built in", Chip::Built),
        State::Coming => look::dashed(chip(&p, "Coming", Chip::Coming), 13.0, tone(&p).line),
        State::Get(_) => btn(&p, "Install", B::Normal, false, focused_btn, if a.offline() { None } else { primary(e, st) }),
    }
}

/// A tile's second line: what it is (or, offline, what's known of it: store-8).
fn tile_sub(a: &App, e: &Entry, st: &State) -> String {
    if !a.offline() {
        return e.card.clone();
    }
    match st {
        State::BuiltIn => "Comes with Hearth OS".into(),
        State::Coming => "Not out yet".into(),
        State::Get(r) | State::Ready(_, r) => format!("AppImage · {}", dl_size(e, r.size)),
        State::UpToDate(i) => format!("AppImage · {}", mb(i.size, false)),
    }
}

fn tile<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let st = a.state(e);
    let id = format!("tile-{}", e.id);
    let title = if e.tag == "rust" { format!("{}  ·  Rust", e.short) } else { e.short.clone() };
    let txt = column![ts(title, f(18.0), tn.text), container(t(tile_sub(a, e, &st), f(14.0), tn.muted)).padding(look::pad(z(4.0), 0.0, z(12.0), 0.0)), status_chip(a, e, &st, false)];
    let c = container(row![app_icon(e, z(56.0)), txt.width(Length::Fill)].spacing(z(16.0)).align_y(iced::Top))
        .padding(look::pad(z(18.0), z(20.0), z(18.0), z(20.0)))
        .width(Length::Fill)
        .height(Length::Fixed(if a.offline() { z(150.0) } else { z(168.0) }))
        .style(look::bx(tn.card, 10.0, None));
    ring(lit(a, &id), 10.0, click(c, Some(Msg::OpenApp(e.id.clone()))))
}

fn grid<'a>(items: Vec<E<'a, Msg>>, cols: usize, gapx: f32) -> E<'a, Msg> {
    let mut c = Column::new().spacing(gapx);
    let mut it = items.into_iter().peekable();
    while it.peek().is_some() {
        let mut r = Row::new().spacing(gapx);
        for _ in 0..cols {
            match it.next() {
                Some(x) => r = r.push(container(x).width(Length::FillPortion(1))),
                None => r = r.push(Space::new().width(Length::FillPortion(1))),
            }
        }
        c = c.push(r);
    }
    c.into()
}

fn your_card<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let st = a.state(e);
    let (sub, end): (String, E<Msg>) = match &st {
        State::UpToDate(i) => (
            if e.self_updates { format!("{} · updates itself", i.version) } else { format!("{} · up to date", i.version) },
            row![chip(&p, "Up to date", Chip::Ok), btn(&p, "Open", B::Normal, false, false, Some(Msg::Launch(e.id.clone())))].spacing(z(10.0)).align_y(iced::Center).into(),
        ),
        State::Ready(i, r) => (
            if e.self_updates { format!("{} → {} · updates itself", i.version, r.version) } else { format!("{} → {} · {}", i.version, r.version, mb(r.size, true)) },
            row![chip(&p, "Update ready", Chip::Up), btn(&p, if e.self_updates { "Open to update" } else { "Update" }, B::Normal, false, false, primary(e, &st))].spacing(z(10.0)).align_y(iced::Center).into(),
        ),
        _ => (String::new(), gap(1.0, 1.0)),
    };
    let id = format!("your-{}", e.id);
    srow(&p, app_icon(e, z(54.0)), e.name.clone(), sub, end, lit(a, &id), Some(Msg::OpenApp(e.id.clone())))
}

fn hero<'a>(a: &'a App, e: &Entry, st: &State) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let from = look::hex(&e.from, ORANGE);
    let to = look::hex(&e.to, ORANGE);
    let (kicker, title, button): (&str, String, &str) = match st {
        State::Ready(_, r) => ("NEW VERSION · UPDATE READY", format!("{} {}", e.name, short_version(&r.version)), if e.self_updates { "Open to update" } else { "Update" }),
        State::Get(r) => ("NEW IN THE STORE", format!("{} {}", e.name, short_version(&r.version)), "Install"),
        _ => ("YOUR APP", e.name.clone(), "Open"),
    };
    let text_lines = if !e.hero.is_empty() {
        e.hero.clone()
    } else {
        match st {
            State::Ready(_, r) | State::Get(r) => r.notes.first().cloned().unwrap_or_else(|| e.about.clone()),
            _ => e.about.clone(),
        }
    };
    let left = column![
        ts(kicker, f(15.0), PURPLE),
        container(row![app_icon(e, z(78.0)), column![tb(title, f(34.0), tn.text), t(e.tagline.clone(), f(17.0), tn.muted)].spacing(z(4.0))].spacing(z(20.0)).align_y(iced::Center)).padding(look::pad(z(16.0), 0.0, z(12.0), 0.0)),
        container(t(text_lines, f(17.0), Pal::mix(tn.text, tn.muted, 0.12))).max_width(z(760.0)),
        Space::new().height(Length::Fill),
        row![btn(&p, button, B::Primary, false, false, primary(e, st)), btn(&p, "See what's new", B::Normal, false, false, Some(Msg::OpenApp(e.id.clone())))].spacing(z(12.0)),
    ];
    let left = container(left).padding(look::pad(z(34.0), z(38.0), z(34.0), z(38.0))).width(Length::Fill).height(Length::Fill).style(look::grad(120.0, Pal::mix(tn.card, from, 0.16), tn.card, 0.0, None));
    let art = container(canvas(look::Brain).width(Length::Fill).height(Length::Fill)).width(Length::Fixed(z(560.0))).height(Length::Fill).style(look::bx(Pal::mix(Color::from_rgb8(0x10, 0x10, 0x18), to, 0.08), 0.0, None));
    let inner = container(row![left, art]).height(Length::Fixed(z(300.0))).width(Length::Fill).clip(true).style(look::bx(tn.card, 10.0, None));
    ring(lit(a, "hero"), 10.0, inner)
}

fn home(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new();
    if a.offline() {
        c = c.push(h1(&p, "Hearth Store", None));
        let warn = canvas(look::Art { kind: "warn".into() }).width(Length::Fixed(z(26.0))).height(Length::Fixed(z(26.0)));
        let banner = container(
            row![
                warn,
                column![t("You're offline", f(19.0), Color::from_rgb8(0xf5, 0xdc, 0x9a)), t(format!("This is the list the Store saw {}. Installing and updates wait until you're back online.", day_time(a.seen.checked)), f(15.0), Pal::mix(tn.text, tn.muted, 0.15))].spacing(z(4.0)).width(Length::Fill),
                btn(&p, "Try again", B::Normal, false, lit(a, "try-again"), Some(Msg::TryAgain)),
            ]
            .spacing(z(16.0))
            .align_y(iced::Center),
        )
        .padding(look::pad(z(16.0), z(20.0), z(16.0), z(20.0)))
        .width(Length::Fill)
        .style(look::bx(Color::from_rgb8(0x2a, 0x25, 0x16), 9.0, Some((Color::from_rgb8(0x5d, 0x4c, 0x1c), 1.0))));
        c = c.push(banner);
    } else {
        let checked = if a.checking && a.seen.checked == 0 { "Looking for updates…".to_string() } else { format!("Checked for updates {}", ago(a, a.seen.checked)) };
        c = c.push(h1(&p, "Hearth Store", Some(chip(&p, checked, Chip::Plain))));
        if a.search.is_empty() {
            if let Some((e, st)) = featured(a) {
                c = c.push(hero(a, &e, &st));
            }
        }
    }
    if a.search.is_empty() {
        let mine = installed(a);
        if mine.is_empty() {
            c = c.push(sec(&p, "Your apps", None));
            let empty = container(
                row![
                    look::dashed(Space::new().width(Length::Fixed(z(42.0))).height(Length::Fixed(z(42.0))), 3.0, tn.muted),
                    column![t("Nothing installed from the Store yet", f(22.0), tn.text), t("Apps you install here show up here, with their updates. On this PC they install as AppImages in your home folder: no password, no restart, and nothing on the system changes.", f(16.0), tn.muted)].spacing(z(6.0)).width(Length::Fill)
                ]
                .spacing(z(32.0))
                .align_y(iced::Center),
            )
            .padding(look::pad(z(40.0), z(40.0), z(40.0), z(40.0)))
            .width(Length::Fill);
            c = c.push(look::dashed(empty, 10.0, tn.line));
        } else {
            c = c.push(sec(&p, "Your apps", Some(look::link("See all ›", f(16.0), Msg::Go(Page::Yours)))));
            c = c.push(grid(mine.iter().take(2).map(|e| your_card(a, e)).collect(), 2, z(16.0)));
        }
    }
    let every = if !a.search.trim().is_empty() {
        sec(&p, format!("Apps matching “{}”", a.search.trim()), None)
    } else if a.offline() {
        container(row![ts("Every app", f(17.0), tn.muted), gap(z(8.0), 1.0), t(format!("· from {}", day_time(a.seen.checked)), f(15.0), tn.muted)].align_y(iced::Center)).padding(look::pad(z(22.0), 0.0, z(12.0), 0.0)).into()
    } else {
        sec(&p, "Every app", None)
    };
    c = c.push(every);
    let tiles: Vec<E<Msg>> = a.cat.apps.iter().filter(|e| matches_search(a, e)).map(|e| tile(a, e)).collect();
    if tiles.is_empty() {
        c = c.push(t("Nothing here by that name.", f(16.0), tn.muted));
    } else {
        c = c.push(grid(tiles, 4, z(16.0)));
    }
    if a.offline() {
        c = c.push(container(t("Pictures you haven't seen yet aren't shown while offline. Apps you've installed keep working offline; only installs and updates wait.", f(16.0), tn.muted)).padding(look::pad(z(26.0), 0.0, 0.0, 0.0)));
    } else {
        let row_ = container(
            row![
                look::dashed(container(t("⬚", f(18.0), tn.muted)).center_x(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0))).style(look::bx(tn.card2, 7.0, None)), 7.0, Color::TRANSPARENT),
                column![t("Looking for other apps?", f(17.0), tn.text), t("Flathub's apps are in Settings › Apps › App store", f(15.0), tn.muted)].spacing(z(4.0)).width(Length::Fill),
                btn(&p, "Open App store ›", B::Normal, false, false, Some(Msg::OpenFlathub)),
            ]
            .spacing(z(18.0))
            .align_y(iced::Center),
        )
        .padding(look::pad(z(18.0), z(22.0), z(18.0), z(22.0)))
        .width(Length::Fill);
        c = c.push(gap(1.0, z(18.0)));
        c = c.push(ring(lit(a, "flathub"), 9.0, look::dashed(row_, 9.0, tn.line)));
    }
    c.into()
}

// ------------------------------------------------------------------ Your apps, Built into Hearth (their own lists)
fn yours(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(10.0)).push(h1(&p, "Your apps", None));
    let mine = installed(a);
    if mine.is_empty() {
        c = c.push(t("Nothing installed from the Store yet. Apps you install show up here, with their updates.", f(16.0), tn.muted));
    }
    for e in mine {
        let st = a.state(&e);
        let (sub, end): (String, E<Msg>) = match &st {
            State::Ready(i, r) => (format!("{} → {} · AppImage in your home folder", i.version, r.version), row![chip(&p, "Update ready", Chip::Up), btn(&p, if e.self_updates { "Open to update" } else { "Update" }, B::Normal, false, false, primary(&e, &st))].spacing(z(10.0)).align_y(iced::Center).into()),
            State::UpToDate(i) => (format!("{} · {}", i.version, if e.self_updates { "updates itself" } else { "AppImage in your home folder" }), row![chip(&p, "Up to date", Chip::Ok), btn(&p, "Open", B::Normal, false, false, Some(Msg::Launch(e.id.clone())))].spacing(z(10.0)).align_y(iced::Center).into()),
            _ => (String::new(), gap(1.0, 1.0)),
        };
        let id = format!("y-{}", e.id);
        c = c.push(srow(&p, app_icon(&e, z(54.0)), e.name.clone(), sub, end, lit(a, &id), Some(Msg::OpenApp(e.id.clone()))));
    }
    c.into()
}

fn built_in(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(10.0)).push(h1(&p, "Built into Hearth", None));
    c = c.push(container(t(format!("These come with {} and update with it (Settings › Updates & recovery). Nothing to install.", if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) }), f(16.0), tn.muted)).padding(look::pad(0.0, 0.0, z(10.0), 0.0)));
    for e in a.cat.apps.iter().filter(|e| e.kind == Kind::BuiltIn) {
        let here = a.here.contains(&e.id);
        let id = format!("b-{}", e.id);
        if e.test {
            // a test version: marked TEST, opened to its own page (Get / Remove)
            let got = a.parts.contains_key(&e.id);
            let end: E<Msg> = row![chip(&p, "TEST", Chip::Test), btn(&p, if got { "Remove…" } else { "Get…" }, B::Normal, false, false, Some(Msg::OpenApp(e.id.clone())))].spacing(z(10.0)).align_y(iced::Center).into();
            c = c.push(srow(&p, app_icon(e, z(54.0)), e.name.clone(), e.card.clone(), end, lit(a, &id), Some(Msg::OpenApp(e.id.clone()))));
            continue;
        }
        let end: E<Msg> = row![chip(&p, "Built in", Chip::Built), btn(&p, "Open", B::Normal, false, false, if here { Some(Msg::Launch(e.id.clone())) } else { None })].spacing(z(10.0)).align_y(iced::Center).into();
        c = c.push(srow(&p, app_icon(e, z(54.0)), e.name.clone(), if here { e.card.clone() } else { format!("{} · not on this PC", e.card) }, end, lit(a, &id), Some(Msg::OpenApp(e.id.clone()))));
    }
    c.into()
}

// ------------------------------------------------------------------ 2. An app's page (store-2)
fn back_link<'a>(a: &'a App) -> E<'a, Msg> {
    let tn = tone(&a.p);
    let to = match &a.page {
        Page::Repair(id) => a.entry(id).map(|e| e.name.clone()).unwrap_or_default(),
        Page::Work => "Updates".to_string(),
        Page::App(id) => a.trail.last().map(|p| page_name(p, a)).unwrap_or_else(|| if a.inst.contains_key(id) { "Your apps".into() } else { "Home".into() }),
        _ => a.trail.last().map(|p| page_name(p, a)).unwrap_or_else(|| "Home".into()),
    };
    container(click(t(format!("‹ {to}"), f(16.0), tn.muted), Some(Msg::Back))).padding(look::pad(0.0, 0.0, z(16.0), 0.0)).into()
}

fn app_chips<'a>(a: &App, e: &Entry, st: &State) -> E<'a, Msg> {
    let p = a.p;
    let mut r = Row::new().spacing(z(10.0));
    match st {
        State::UpToDate(i) => r = r.push(chip(&p, format!("Installed · {}", i.version), Chip::Ok)),
        State::Ready(i, _) => r = r.push(chip(&p, format!("Installed · {}", i.version), Chip::Ok)).push(chip(&p, "Update ready", Chip::Up)),
        State::BuiltIn if e.test => r = r.push(chip(&p, "TEST", Chip::Test)),
        State::BuiltIn => r = r.push(chip(&p, "Built in", Chip::Built)),
        State::Coming => r = r.push(look::dashed(chip(&p, "Coming", Chip::Coming), 13.0, tone(&p).line)),
        State::Get(_) => {}
    }
    if e.self_updates {
        r = r.push(chip(&p, "Updates itself", Chip::Plain));
    }
    let test = match st {
        State::Get(r) | State::Ready(_, r) => r.test,
        _ => false,
    };
    if e.private {
        r = r.push(chip(&p, "Only on your devices", Chip::Plain));
    }
    if matches!(st, State::Get(_) | State::Ready(..) | State::UpToDate(_)) {
        r = r.push(if test { chip(&p, "TEST", Chip::Test) } else { chip(&p, "Stable", Chip::Plain) });
    }
    r.into()
}

pub fn open_label(e: &Entry, st: &State) -> &'static str {
    match st {
        State::Get(_) => "Install",
        State::Ready(..) if e.self_updates => "Open to update",
        State::Ready(..) => "Update",
        State::Coming => "Coming",
        _ => "Open",
    }
}

fn app_head<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let st = a.state(e);
    let line = if e.test {
        format!("Test version · by {}", e.author)
    } else if e.tag == "rust" {
        format!("Rust · from the Hearth Store · by {}", e.author)
    } else if e.part.is_some() {
        format!("Built into Hearth OS · by {}", e.author)
    } else if e.tagline.is_empty() {
        e.author.clone()
    } else {
        format!("{} · {}", e.author, e.tagline)
    };
    let mut right = Row::new().spacing(z(12.0));
    if e.test {
        right = right.push(look::chip(&p, "TEST", look::Chip::Test));
    } else if a.dlg.is_none() {
        let on = if matches!(st, State::Coming) { None } else { primary(e, &st) };
        right = right.push(btn(&p, open_label(e, &st), B::Primary, true, lit(a, "open"), if a.offline() && matches!(st, State::Get(_) | State::Ready(..)) && !e.self_updates { None } else { on }));
        if a.inst.contains_key(&e.id) {
            right = right.push(btn(&p, "⋯", B::Normal, true, lit(a, "more") || a.menu, Some(Msg::Menu)));
        }
    }
    let head = row![
        app_icon(e, z(104.0)),
        if e.part.is_some() {
            column![tb(e.name.clone(), f(38.0), tn.text), container(t(line, f(17.0), tn.muted)).padding(look::pad(z(6.0), 0.0, 0.0, 0.0))].width(Length::Fill)
        } else {
            column![tb(e.name.clone(), f(38.0), tn.text), container(t(line, f(17.0), tn.muted)).padding(look::pad(z(6.0), 0.0, z(12.0), 0.0)), app_chips(a, e, &st)].width(Length::Fill)
        },
        container(right).padding(look::pad(z(10.0), 0.0, 0.0, 0.0)),
    ]
    .spacing(z(26.0))
    .align_y(iced::Center);
    column![back_link(a), container(head).padding(look::pad(0.0, 0.0, z(22.0), 0.0))].into()
}

/// One of Hearth's own apps or Mantel (mockup 1.17-quick-updates 2): what a quick update is, which one runs, and
/// going back to the built-in one.
/// A test version (Mantel in Rust): what it is, Get or Remove. Shown only with "Get test versions" on.
fn test_page<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let have = a.parts.get(&e.id);
    let pt = e.part.clone();
    let mut left = Column::new().push(container(ts("A test version", f(21.0), tn.text)).padding(look::pad(0.0, 0.0, z(10.0), 0.0)));
    let about = if e.about.is_empty() {
        "Mantel drawn in Rust: the same bar, dock, wallpaper and app menu, lighter and quicker. Still being built: Quick Access, notifications, tray icons and right-click menus aren't in it yet.".to_string()
    } else {
        e.about.clone()
    };
    left = left.push(t(about, f(16.0), Pal::mix(tn.text, tn.muted, 0.12)));
    left = left.push(container(bullets(&p, &[
        "Only on this PC, only when you get it here: the usual one stays in Hearth OS, unchanged.".to_string(),
        "If it doesn't start, the usual one starts instead, right away.".to_string(),
        "Remove it here any time: the usual one is back at once. Your settings stay as they are.".to_string(),
    ], f(16.0))).padding(look::pad(z(12.0), 0.0, 0.0, 0.0)));
    if let Some(pt) = pt.as_ref().filter(|pt| !pt.notes.is_empty()) {
        left = left.push(container(ts(format!("What's new in {}", pt.version), f(18.0), tn.text)).padding(look::pad(z(18.0), 0.0, z(8.0), 0.0))).push(bullets(&p, &pt.notes, f(16.0)));
    }
    let mut facts = Column::new();
    let status = match have {
        Some(h) if h.gave_up() => format!("{} · didn't start, the usual one is used", h.version),
        Some(h) if h.failed_now => format!("{} · didn't start this time", h.version),
        Some(h) => format!("{} · in use", h.version),
        None => "Not got".to_string(),
    };
    facts = facts.push(kv(&p, "This PC", status, None, false));
    if let Some(pt) = pt.as_ref() {
        facts = facts.push(kv(&p, "Newest", pt.version.clone(), Some(format!("for Hearth OS {}", pt.os)), false));
    }
    facts = facts.push(kv(&p, "Installs as", "Files in your home folder", Some("no password, no restart".into()), false));
    facts = facts.push(kv(&p, "Checked with", "The signed list from hearthos.fun", None, true));
    let busy = a.part_busy.contains(&e.id);
    let fits = pt.as_ref().map(|pt| parts::test_fits(pt, &a.hearth)).unwrap_or(false);
    let mut buttons = Row::new().spacing(z(10.0));
    if have.is_some() {
        if let Some(up) = a.part_offer(e) {
            buttons = buttons.push(btn(&p, if busy { "Updating…" } else { "Update" }, B::Primary, false, lit(a, "test-update"), if busy || a.offline() { None } else { Some(Msg::PartUpdate(e.id.clone())) }));
            let _ = up;
        }
        if have.map(|h| h.gave_up() || h.failed_now).unwrap_or(false) {
            buttons = buttons.push(btn(&p, "Try it again", B::Normal, false, lit(a, "test-retry"), if busy { None } else { Some(Msg::PartRetry(e.id.clone())) }));
        }
        buttons = buttons.push(btn(&p, if busy { "Removing…" } else { "Remove" }, B::Normal, false, lit(a, "test-remove"), if busy { None } else { Some(Msg::PartRemove(e.id.clone())) }));
    } else {
        buttons = buttons.push(btn(&p, if busy { "Getting…" } else { "Get" }, B::Primary, false, lit(a, "test-get"), if busy || a.offline() || !fits { None } else { Some(Msg::PartGet(e.id.clone())) }));
        if !fits {
            buttons = buttons.push(t(format!("Not made for Hearth OS {} yet", a.hearth), f(15.0), tn.muted));
        }
    }
    facts = facts.push(container(buttons).padding(look::pad(z(16.0), 0.0, 0.0, 0.0)));
    let lower = row![card(&p, left, false).into_width(140), card(&p, facts, false).into_width(100)].spacing(z(18.0));
    Column::new().push(app_head(a, e)).push(lower).into()
}

fn part_page<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let have = a.parts.get(&e.id);
    let mut left = Column::new().push(container(ts("Quick updates", f(21.0), tn.text)).padding(look::pad(0.0, 0.0, z(10.0), 0.0)));
    left = left.push(t("Fixes for Hearth's own apps and desktops come here as soon as they're ready, without waiting for the next Hearth OS update. The new version goes in your home folder, next to the one built into Hearth OS, which is never changed.", f(16.0), Pal::mix(tn.text, tn.muted, 0.12)));
    left = left.push(container(bullets(&p, &[
        "Used only when it's signed with Dragongamer9's key and made for your Hearth OS.".to_string(),
        "If it doesn't start, the built-in one opens instead, right away, and you're told once.".to_string(),
        "Your settings stay as they are: the app reads the same settings files either way.".to_string(),
    ], f(16.0))).padding(look::pad(z(12.0), 0.0, 0.0, 0.0)));
    if new_shown(a, &e.id) {
        if let Some(pt) = e.part.as_ref().filter(|pt| !pt.notes.is_empty()) {
            left = left.push(container(ts(format!("What's new in {}", pt.version), f(18.0), tn.text)).padding(look::pad(z(18.0), 0.0, z(8.0), 0.0))).push(bullets(&p, &pt.notes, f(16.0)));
        }
    }
    let running = parts::running(have, &a.hearth);
    let quick = have.map(|h| h.version == running && running != a.hearth).unwrap_or(false);
    let run_line = if quick {
        let day = have.map(|h| day_time(h.installed)).unwrap_or_default();
        let day = day.split(" at ").next().unwrap_or("").trim().to_string();
        if day.is_empty() { format!("{running} · quick update") } else { format!("{running} · quick update, {day}") }
    } else {
        format!("{running} · built into Hearth OS")
    };
    let mut facts = Column::new();
    facts = facts.push(kv(&p, "Running", run_line, None, false));
    facts = facts.push(kv(&p, "Built into Hearth OS", if a.hearth.is_empty() { "Hearth OS".to_string() } else { a.hearth.clone() }, None, false));
    facts = facts.push(kv(&p, "Installs as", "Files in your home folder", Some("no password, no restart".into()), false));
    facts = facts.push(kv(&p, "Checked with", "The signed list from hearthos.fun", None, true));
    let busy = a.part_busy.contains(&e.id);
    let buttons = row![
        btn(&p, "Use the built-in one", B::Normal, false, lit(a, "builtin-one"), if quick && !busy { Some(Msg::PartRemove(e.id.clone())) } else { None }),
        btn(&p, "What's new", B::Normal, false, lit(a, "whats-new"), Some(Msg::ToggleNew(e.id.clone())))
    ]
    .spacing(z(10.0));
    facts = facts.push(container(buttons).padding(look::pad(z(16.0), 0.0, 0.0, 0.0)));
    let lower = row![card(&p, left, false).into_width(140), card(&p, facts, false).into_width(100)].spacing(z(18.0));
    Column::new().push(app_head(a, e)).push(lower).into()
}

fn app_page<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    if e.test {
        return test_page(a, e);
    }
    if e.part.is_some() {
        return part_page(a, e);
    }
    let p = a.p;
    let tn = tone(&p);
    let st = a.state(e);
    let mut c = Column::new().push(app_head(a, e));
    // Pictures (once seen, kept in the cache; offline, only those).
    let shots: Vec<&image::Handle> = e.pictures.iter().filter_map(|u| a.pics.get(u)).take(3).collect();
    if !shots.is_empty() {
        let mut r = Row::new().spacing(z(16.0));
        for (i, h) in shots.into_iter().enumerate() {
            let im = image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::Contain);
            let w = if i == 0 { 125 } else { 100 };
            r = r.push(container(im).width(Length::FillPortion(w)).height(Length::Fixed(z(300.0))).clip(true).style(look::bx(Color::from_rgb8(0x0f, 0x11, 0x15), 9.0, Some((tn.line, 1.0)))));
        }
        c = c.push(container(r).padding(look::pad(0.0, 0.0, z(22.0), 0.0)));
    }
    let rel = match &st {
        State::Get(r) | State::Ready(_, r) => Some(r.clone()),
        _ => a.seen.rel(&e.id),
    };
    let mut left = Column::new();
    if let Some(r) = rel.as_ref().filter(|r| !r.notes.is_empty()) {
        left = left.push(container(ts(format!("What's new in {}", short_version(&r.version)), f(21.0), tn.text)).padding(look::pad(0.0, 0.0, z(10.0), 0.0))).push(bullets(&p, &r.notes, f(16.0)));
        left = left.push(gap(1.0, z(20.0)));
    }
    left = left.push(container(ts("About", f(21.0), tn.text)).padding(look::pad(0.0, 0.0, z(8.0), 0.0))).push(t(if e.about.is_empty() { e.card.clone() } else { e.about.clone() }, f(16.0), Pal::mix(tn.text, tn.muted, 0.12)));
    let mut facts = Column::new();
    let pc_line = format!("This PC: {} · no password, no restart", if a.edition.is_empty() { "Hearth OS".to_string() } else { a.edition.clone() });
    match &st {
        State::BuiltIn => {
            facts = facts.push(kv(&p, "Version", format!("Comes with {}", if a.hearth.is_empty() { "Hearth OS".into() } else { format!("Hearth OS {}", a.hearth) }), None, false));
            facts = facts.push(kv(&p, "Installs as", "Part of Hearth OS", None, false));
            facts = facts.push(kv(&p, "Updates", "With Hearth OS (Settings › Updates & recovery)", None, true));
        }
        State::Coming => {
            facts = facts.push(kv(&p, "Version", "Not out yet", None, false));
            facts = facts.push(kv(&p, "Installs as", "AppImage in your home folder", Some(pc_line), true));
        }
        _ => {
            let (ver, date, size) = match (&st, rel.as_ref()) {
                (State::UpToDate(i), r) => (i.version.clone(), r.filter(|r| r.version == i.version).map(|r| r.date.clone()).unwrap_or_default(), i.size),
                (_, Some(r)) => (r.version.clone(), r.date.clone(), r.size),
                _ => (String::new(), String::new(), 0),
            };
            let rd = released(&date);
            facts = facts.push(kv(&p, "Version", if rd.is_empty() { ver } else { format!("{ver} · {rd}") }, None, false));
            facts = facts.push(kv(&p, "Download", mb(size, false), None, false));
            facts = facts.push(kv(&p, "Installs as", "AppImage in your home folder", Some(pc_line), false));
            facts = facts.push(kv(&p, "Updates", if e.self_updates { "By itself, from the same release" } else { "From the Store, when you say so" }, None, false));
            facts = facts.push(kv(&p, "Checked with", if e.pin.is_some() { "The SHA-256 in Hearth OS's list" } else { "The release's SHA-256 list" }, None, true));
        }
    }
    let lower = row![card(&p, left, false).into_width(135), card(&p, facts, false).into_width(100)].spacing(z(22.0));
    c = c.push(lower);
    c.into()
}

trait Portion<'a> {
    fn into_width(self, n: u16) -> E<'a, Msg>;
}
impl<'a> Portion<'a> for E<'a, Msg> {
    fn into_width(self, n: u16) -> E<'a, Msg> {
        container(self).width(Length::FillPortion(n)).into()
    }
}

/// The ⋯ menu (store-2): Check and repair files, Show its files, Release page on GitHub, Uninstall…
fn menu(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Page::App(id) = &a.page else { return gap(1.0, 1.0) };
    let items: [(&str, &str, Msg, bool); 4] = [
        ("✓", "Check and repair files", Msg::Go(Page::Repair(id.clone())), false),
        ("▤", "Show its files", Msg::ShowFiles(id.clone()), false),
        ("↗", "Release page on GitHub", Msg::ReleasePage(id.clone()), false),
        ("trash", "Uninstall…", Msg::AskUninstall(id.clone()), true),
    ];
    let mut c = Column::new().spacing(2.0);
    for (i, (sym, label, m, red)) in items.into_iter().enumerate() {
        if i == 3 {
            c = c.push(container(ui::hline(Pal::mix(tn.line, tn.text, 0.06))).padding(look::pad(z(6.0), z(4.0), z(6.0), z(4.0))));
        }
        let on = a.menu_at == i;
        let fg = if red { Color::from_rgb8(0xff, 0x9d, 0x9d) } else { tn.text };
        let icon: E<Msg> = if sym == "trash" {
            canvas(look::Art { kind: "trash".into() }).width(Length::Fixed(z(20.0))).height(Length::Fixed(z(20.0))).into()
        } else {
            container(t(sym, f(15.0), tn.muted)).center_x(Length::Fixed(z(20.0))).into()
        };
        let r = container(row![icon, t(label, f(17.0), fg)].spacing(z(12.0)).align_y(iced::Center))
            .padding(look::pad(z(13.0), z(16.0), z(13.0), z(16.0)))
            .width(Length::Fill)
            .style(look::bx(if on { Pal::mix(tn.card2, tn.text, 0.06) } else { Color::TRANSPARENT }, 6.0, if on { Some((ORANGE, 2.0)) } else { None }));
        c = c.push(click(r, Some(m)));
    }
    let m = container(c).padding(z(8.0)).width(Length::Fixed(z(360.0))).style(look::bx(Pal::mix(tn.card, tn.text, 0.03), 9.0, Some((Pal::mix(tn.line, tn.text, 0.08), 1.0))));
    container(iced::widget::opaque(m)).width(Length::Fill).align_x(iced::Right).padding(look::pad(z(200.0), z(50.0), 0.0, 0.0)).into()
}

// ------------------------------------------------------------------ 3. Updates (store-3)
fn updates(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let ready: Vec<_> = a.ready().into_iter().filter(|(e, _, _)| !e.self_updates).collect();
    let offers = a.part_offers();
    let n = ready.len() + offers.len();
    let mut c = Column::new();
    c = c.push(h1(&p, "Updates", Some(btn(&p, format!("Update all ({n})"), B::Primary, true, false, if n > 0 && !a.offline() { Some(Msg::UpdateAll) } else { None }))));
    let when = if a.offline() { format!("You're offline. Last checked {}.", day_time(a.seen.checked)) } else { format!("Checked {}.", ago_long(a, a.seen.checked)) };
    let how = if a.prefs.daily { "The Store looks again when you open it, and at most once a day." } else { "The Store looks again when you open it." };
    c = c.push(container(t(format!("{when} {how}"), f(16.0), tn.muted)).padding(look::pad(0.0, 0.0, z(20.0), 0.0)));
    let mut list = Column::new().spacing(z(10.0));
    // Quick updates for Hearth's own apps and Mantel (mockup 1.17-quick-updates 1): first, the first one open.
    for (e, pt) in &offers {
        let id = format!("upd-{}", e.id);
        let shown = new_shown(a, &e.id);
        let busy = a.part_busy.contains(&e.id);
        let from = parts::running(a.parts.get(&e.id), &a.hearth);
        let how = if pt.id == "mantel" { "quick update: no Hearth OS update, no restart" } else { "quick update · opens with it next time" };
        let pill = click(
            container(t(if shown { "Hide what's new ▴" } else { "What's new ▾" }, f(14.0), Pal::mix(tn.text, tn.muted, 0.2))).padding(look::pad(z(8.0), z(14.0), z(8.0), z(14.0))).style(look::bx(Color::TRANSPARENT, 14.0, Some((Pal::mix(tn.line, tn.text, 0.08), 1.0)))),
            Some(Msg::ToggleNew(e.id.clone())),
        );
        let big = shown;
        let mut body = Column::new().push(t(e.name.clone(), f(if big { 22.0 } else { 19.0 }), tn.text)).push(t(format!("{from} → {} · {} · {how}", pt.version, mb(pt.size, false)), f(15.0), tn.muted));
        if shown {
            body = body.push(container(ts(format!("What's new in {}", pt.version), f(18.0), tn.text)).padding(look::pad(z(22.0), 0.0, z(8.0), 0.0)));
            if !pt.notes.is_empty() {
                body = body.push(bullets(&p, &pt.notes, f(16.0)));
            }
            let note = if pt.id == "mantel" {
                format!("Your desktop restarts in a moment to use it: open windows stay open, your settings stay as they are. If it doesn't start, the Mantel built into Hearth OS {} comes back by itself.", a.hearth)
            } else {
                format!("Your settings stay as they are. If it doesn't start, the {} built into Hearth OS {} opens instead.", e.name, a.hearth)
            };
            body = body.push(container(t(note, f(15.0), tn.muted)).padding(look::pad(z(14.0), 0.0, 0.0, 0.0)));
        }
        let go = btn(&p, if busy { "Updating…" } else { "Update" }, B::Primary, false, false, if a.offline() || busy { None } else { Some(Msg::PartUpdate(e.id.clone())) });
        let icon = app_icon(e, z(if big { 64.0 } else { 54.0 }));
        let rowx = row![icon, body.spacing(z(4.0)).width(Length::Fill), row![pill, go].spacing(z(10.0)).align_y(iced::Center)].spacing(z(20.0)).align_y(if big { iced::alignment::Vertical::Top } else { iced::alignment::Vertical::Center });
        let pad = if big { z(24.0) } else { z(18.0) };
        list = list.push(ring(lit(a, &id), 10.0, container(rowx).padding(look::pad(pad, z(24.0), pad, z(24.0))).width(Length::Fill).style(look::bx(tn.card, 10.0, None))));
    }
    for (e, i, r) in &ready {
        let id = format!("upd-{}", e.id);
        let shown = new_shown(a, &e.id);
        let pill = click(
            container(t(if shown { "Hide what's new ▴" } else { "What's new ▾" }, f(14.0), Pal::mix(tn.text, tn.muted, 0.2))).padding(look::pad(z(8.0), z(14.0), z(8.0), z(14.0))).style(look::bx(Color::TRANSPARENT, 14.0, Some((Pal::mix(tn.line, tn.text, 0.08), 1.0)))),
            Some(Msg::ToggleNew(e.id.clone())),
        );
        let mut body = Column::new().push(t(e.name.clone(), f(22.0), tn.text)).push(t(format!("{} → {} · {} · AppImage in your home folder", i.version, r.version, dl_size(e, r.size)), f(15.0), tn.muted));
        if shown {
            body = body.push(container(ts(format!("What's new in {}", short_version(&r.version)), f(18.0), tn.text)).padding(look::pad(z(22.0), 0.0, z(8.0), 0.0)));
            body = body.push(bullets(&p, &r.notes, f(16.0)));
            if !e.update_note.is_empty() {
                body = body.push(container(t(e.update_note.clone(), f(15.0), tn.muted)).padding(look::pad(z(14.0), 0.0, 0.0, 0.0)));
            }
        }
        let rowx = row![app_icon(e, z(64.0)), body.spacing(z(4.0)).width(Length::Fill), row![pill, btn(&p, "Update", B::Primary, false, false, if a.offline() { None } else { Some(Msg::Start(e.id.clone(), JobKind::Update)) })].spacing(z(10.0)).align_y(iced::Center)]
            .spacing(z(20.0))
            .align_y(iced::Top);
        list = list.push(ring(lit(a, &id), 10.0, container(rowx).padding(look::pad(z(24.0), z(24.0), z(24.0), z(24.0))).width(Length::Fill).style(look::bx(tn.card, 10.0, None))));
    }
    for e in installed(a).into_iter().filter(|e| e.self_updates) {
        let st = a.state(&e);
        let id = format!("self-{}", e.id);
        let ver = match &st {
            State::UpToDate(i) | State::Ready(i, _) => i.version.clone(),
            _ => String::new(),
        };
        let end: E<Msg> = match &st {
            State::Ready(_, r) => row![chip(&p, format!("{} is ready", r.version), Chip::Up), btn(&p, "Open to update", B::Normal, false, false, Some(Msg::Launch(e.id.clone())))].spacing(z(10.0)).align_y(iced::Center).into(),
            _ => chip(&p, "Up to date", Chip::Ok),
        };
        list = list.push(srow(&p, app_icon(&e, z(54.0)), e.name.clone(), format!("{ver} · this app updates itself. When it has a new version, the button here is \"Open to update\"."), end, lit(a, &id), None));
    }
    for e in installed(a).into_iter().filter(|e| !e.self_updates && matches!(a.state(e), State::UpToDate(_))) {
        let v = a.inst.get(&e.id).map(|i| i.version.clone()).unwrap_or_default();
        list = list.push(srow(&p, app_icon(&e, z(54.0)), e.name.clone(), format!("{v} · AppImage in your home folder"), chip(&p, "Up to date", Chip::Ok), false, None));
    }
    c = c.push(list);
    // Hearth's own apps and desktops (mockup 1.17-quick-updates 1): the ones up to date in one row, and each one
    // that went back to the built-in one, with Try it again and Remove the update.
    let gave_up = parts_gave_up(a);
    let up: Vec<&Entry> = a.cat.apps.iter().filter(|e| e.kind == Kind::BuiltIn && !e.test && a.part_offer(e).is_none() && !gave_up.iter().any(|g| g.id == e.id)).collect();
    if !up.is_empty() || !gave_up.is_empty() {
        c = c.push(sec(&p, "Hearth's own apps and desktops", None));
        let os = if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) };
        if let Some(first) = up.first() {
            let names: Vec<String> = up.iter().take(6).map(|e| e.name.clone()).collect();
            let mut title = names.join(" · ");
            if up.len() > 6 {
                title = format!("{title} and {} more", up.len() - 6);
            }
            c = c.push(srow(&p, app_icon(first, z(46.0)), title, format!("Up to date for {os} · they get quick updates here, between Hearth OS updates"), chip(&p, "Up to date", Chip::Ok), false, None));
        }
        for e in &gave_up {
            let v = a.parts.get(&e.id).map(|h| h.version.clone()).unwrap_or_default();
            let busy = a.part_busy.contains(&e.id);
            let end: E<Msg> = row![
                btn(&p, "Try it again", B::Normal, false, lit(a, &format!("retry-{}", e.id)), if busy { None } else { Some(Msg::PartRetry(e.id.clone())) }),
                btn(&p, "Remove the update", B::Normal, false, lit(a, &format!("rm-{}", e.id)), if busy { None } else { Some(Msg::PartRemove(e.id.clone())) })
            ]
            .spacing(z(10.0))
            .align_y(iced::Center)
            .into();
            let short = if e.short.is_empty() { e.name.clone() } else { e.short.clone() };
            c = c.push(srow(&p, app_icon(e, z(46.0)), format!("{} {v} · went back to the built-in one", e.name), format!("Its update didn't start twice, so the {short} built into Hearth OS is used until the next update"), end, false, None));
        }
    }
    c = c.push(sec(&p, "Test versions", None));
    let (tt, ss) = if a.prefs.test {
        ("Test versions are on on this PC", "New versions reach this PC first, marked TEST, before everyone else. Store settings can turn them off.")
    } else {
        ("Test versions are off on this PC", "New versions go to the test channel first and reach everyone once they've been tried on a device. Store settings can turn them on for this PC.")
    };
    c = c.push(srow(&p, icon_box(&p, "⚑"), tt, ss, btn(&p, "Store settings ›", B::Normal, false, lit(a, "test-link"), Some(Msg::Go(Page::Settings))), false, None));
    c.into()
}

// ------------------------------------------------------------------ 4. Installing / updating (store-4)
fn step_circle<'a>(p: &Pal, n: usize, state: u8) -> E<'a, Msg> {
    // state: 0 waiting, 1 now, 2 done
    let tn = tone(p);
    let d = z(36.0);
    let (bg, inner): (Color, E<'a, Msg>) = match state {
        2 => (Color::from_rgb8(0x1f, 0x4a, 0x31), ui::glyph(ui::G::Check, look::OK_TX, z(18.0), 2.6)),
        1 => (ORANGE, tb(n.to_string(), f(16.0), look::INK).into()),
        _ => (Pal::mix(tn.card2, tn.text, 0.05), tb(n.to_string(), f(16.0), tn.muted).into()),
    };
    container(inner).center_x(Length::Fixed(d)).center_y(Length::Fixed(d)).style(look::bx(bg, d / 2.0, None)).into()
}

fn step_row<'a>(p: &Pal, n: usize, state: u8, title: String, sub: String, right: String, extra: Option<E<'a, Msg>>, last: bool) -> E<'a, Msg> {
    let tn = tone(p);
    let mut txt = Column::new().push(t(title, f(18.0), tn.text)).push(t(sub, f(15.0), tn.muted)).spacing(z(4.0));
    if let Some(x) = extra {
        txt = txt.push(container(x).padding(look::pad(z(10.0), 0.0, 0.0, 0.0)));
    }
    let r = row![step_circle(p, n, state), txt.width(Length::Fill), t1(right, f(15.0), tn.muted)].spacing(z(18.0)).align_y(iced::Top);
    let mut c = Column::new().push(container(r).padding(look::pad(z(16.0), 0.0, z(16.0), 0.0)));
    if !last {
        c = c.push(ui::hline(tn.line));
    }
    c.into()
}

fn speed_left(j: &crate::Job) -> String {
    if let Some(fr) = j.demo {
        // CI's still picture: the mockup's "about 10 seconds left".
        let left = ((1.0 - fr) / fr.max(0.01) * 16.0).round().max(1.0) as u64;
        return format!("about {left} seconds left");
    }
    let secs = j.t0.elapsed().as_secs_f64().max(0.5);
    let rate = j.got as f64 / secs;
    if rate < 1.0 || j.got == 0 {
        return "starting…".into();
    }
    let left = ((j.total.saturating_sub(j.got)) as f64 / rate).round() as u64;
    if left < 60 {
        format!("about {} seconds left", left.max(1))
    } else {
        format!("about {} minutes left", left.div_ceil(60))
    }
}

fn work(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(j) = &a.job else { return t("Nothing is installing.", f(16.0), tn.muted).into() };
    let Some(e) = a.entry(&j.id) else { return gap(1.0, 1.0) };
    let update = j.kind == JobKind::Update;
    let repo = e.github.rsplit('/').next().unwrap_or("").to_string();
    let verb = if update { "Updating" } else { "Installing" };
    let title = if j.done() { format!("{} {}", e.name, if update { "is updated" } else { "is installed" }) } else { format!("{verb} {}", e.name) };
    let from = if update { format!("{} → {} · ", j.from, j.rel.version) } else { format!("{} · ", j.rel.version) };
    let sub = format!("{from}AppImage in your home folder · from the {repo} release on GitHub");
    let mut right = Row::new().spacing(z(12.0));
    if j.done() {
        right = right.push(btn(&p, "Open", B::Primary, true, lit(a, "open"), Some(Msg::Launch(e.id.clone())))).push(btn(&p, "Back", B::Normal, true, lit(a, "back"), Some(Msg::Back)));
    } else if !j.error.is_empty() {
        right = right.push(btn(&p, "Back", B::Normal, true, lit(a, "back"), Some(Msg::Back)));
    } else {
        right = right.push(btn(&p, "Cancel", B::Normal, true, lit(a, "cancel"), if j.can_cancel() { Some(Msg::Cancel) } else { None }));
    }
    let head = row![app_icon(e, z(96.0)), column![tb(title, f(38.0), tn.text), t(sub, f(17.0), tn.muted)].spacing(z(6.0)).width(Length::Fill), right].spacing(z(24.0)).align_y(iced::Center);
    let frac = if j.total > 0 { j.got as f32 / j.total as f32 } else { 0.0 };
    let pct = (frac * 100.0).round() as u32;
    let (big, small) = match j.step {
        0 => (format!("Downloading... {pct}%"), format!("{} of {} · {}", mb(j.got, false).replace(" MB", ""), mb(j.total, false), speed_left(j))),
        1 => ("Checking the download...".to_string(), String::new()),
        2 => (format!("Closing {}...", e.name), String::new()),
        3 => ("Putting it in place...".to_string(), String::new()),
        4 => (if j.was_running { format!("Opening {} again...", e.name) } else { "Writing the menu entry...".to_string() }, String::new()),
        _ => ("Done".to_string(), if j.kept { format!("{} is kept until {} has started fine.", j.from, j.rel.version) } else { String::new() }),
    };
    let overall = if j.step == 0 { frac } else { (0.8 + j.step as f32 * 0.04).min(1.0) };
    let mut prog = column![
        row![ts(big, f(22.0), if j.error.is_empty() { tn.text } else { look::BAD }), Space::new().width(Length::Fill), t(small, f(16.0), tn.muted)].align_y(iced::Center),
        container(look::progress(&p, if j.error.is_empty() { overall } else { 0.0 }, z(12.0), Length::Fill)).padding(look::pad(z(14.0), 0.0, z(14.0), 0.0)),
    ];
    prog = prog.push(t(if j.error.is_empty() { "Cancel works until the new version is put in place. That last step is a single swap, so the app is never left half updated.".to_string() } else { j.error.clone() }, f(15.0), if j.error.is_empty() { tn.muted } else { look::BAD }));
    let prog = container(prog).padding(look::pad(z(26.0), z(24.0), z(22.0), z(24.0))).width(Length::Fill).style(look::bx(tn.card, 10.0, None));
    let st = |n: u8| -> u8 {
        if j.step > n || j.done() { 2 } else if j.step == n && j.error.is_empty() { 1 } else { 0 }
    };
    let right_of = |n: u8| -> String {
        if j.step > n || j.done() {
            "Done".into()
        } else if j.step == n {
            if !j.error.is_empty() { "Stopped".into() } else if n == 0 { format!("{pct}%") } else { "Now".into() }
        } else {
            "Waiting".into()
        }
    };
    let pron_close = if e.close_note.is_empty() { "It saves first".to_string() } else { e.close_note.clone() };
    let check_src = if e.pin.is_some() { "Hearth OS's list" } else { "the release's SHA256SUMS.txt" };
    let mut steps = Column::new();
    let bar: E<Msg> = container(look::progress(&p, if j.step == 0 { frac } else { 1.0 }, z(12.0), Length::Fill)).width(Length::Fixed(z(520.0))).into();
    steps = steps.push(step_row(&p, 1, st(0), "Download".into(), format!("{} · {}", j.rel.asset, if update { "next to the old one, which keeps working meanwhile" } else { "into your home folder" }), right_of(0), if j.step == 0 { Some(bar) } else { None }, false));
    steps = steps.push(step_row(&p, 2, st(1), "Check the download".into(), format!("Its SHA-256 must match the release's list ({check_src}); if not, it's deleted"), right_of(1), None, false));
    let mut n = 3;
    if update {
        let close_right = if j.step > 2 && !j.was_running { "It wasn't open".to_string() } else { right_of(2) };
        steps = steps.push(step_row(&p, n, st(2), format!("Close {}", e.name), pron_close, close_right, None, false));
        n += 1;
    }
    let place = if update { format!("The new AppImage takes the old one's place in one step; {} is kept until {} has started fine", j.from, j.rel.version) } else { "The AppImage goes into its own folder in one step".to_string() };
    steps = steps.push(step_row(&p, n, st(3), "Put it in place".into(), place, right_of(3), None, false));
    n += 1;
    let (t5, s5) = if update {
        (if e.reopen_title.is_empty() { "Update the menu entry and open it again".to_string() } else { e.reopen_title.clone() }, if e.reopen_note.is_empty() { format!("{} opens where you left it", e.name) } else { e.reopen_note.clone() })
    } else {
        ("Add its menu entry".to_string(), format!("{} is in your apps menu, ready to open", e.name))
    };
    steps = steps.push(step_row(&p, n, st(4), t5, s5, right_of(4), None, true));
    let steps = container(steps).padding(look::pad(z(14.0), z(24.0), z(14.0), z(24.0))).width(Length::Fill).style(look::bx(tn.card, 10.0, None));
    let mut c = column![back_link(a), container(head).padding(look::pad(0.0, 0.0, z(26.0), 0.0)), prog, gap(1.0, z(20.0)), steps];
    if j.details {
        c = c.push(container(card(&p, column![kv(&p, "From", j.rel.url.clone(), None, false), kv(&p, "To", paths::tilde(&hearth_store::files::app_file(e)), None, false), kv(&p, "SHA-256", j.rel.sha256.clone(), None, true)], false)).padding(look::pad(z(20.0), 0.0, 0.0, 0.0)));
    }
    c = c.push(container(t("Everything happens in your home folder: no password, no restart, and nothing that starts your PC is changed.", f(16.0), tn.muted)).padding(look::pad(z(26.0), 0.0, 0.0, 0.0)));
    c.into()
}

// ------------------------------------------------------------------ 5. Check and repair (store-5)
fn repair<'a>(a: &'a App, e: &Entry) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let i = a.inst.get(&e.id).cloned().unwrap_or_default();
    let r = a.repair.as_ref().filter(|r| r.id == e.id);
    let c = r.and_then(|r| r.check.clone());
    let head = row![app_icon(e, z(96.0)), column![tb("Check and repair", f(38.0), tn.text), t(format!("{} {} · AppImage in your home folder", e.name, i.version), f(17.0), tn.muted)].spacing(z(6.0))].spacing(z(24.0)).align_y(iced::Center);
    let done = if c.is_some() { 2 } else { 1 };
    let s1: String = match c.as_ref().map(|c| c.matches) {
        None => "Checking…".into(),
        Some(Some(true)) => "Matches".into(),
        Some(Some(false)) => "Doesn't match".into(),
        Some(None) => "No SHA-256 known".into(),
    };
    let menu_n = c.as_ref().map(|c| c.menu_fixes()).unwrap_or(0);
    let menu_sub = match c.as_ref() {
        None => "Its menu entry and icon".to_string(),
        Some(c) if !c.menu && !c.icon => "The menu entry and icon are missing".into(),
        Some(c) if !c.menu => "The menu entry is missing".into(),
        Some(c) if !c.icon => "The icon is missing".into(),
        Some(_) => "Both are there".into(),
    };
    let fuse = c.as_ref().map(|c| c.fuse).unwrap_or(true);
    let fixing = r.map(|r| r.fixing).unwrap_or(false);
    let need = c.as_ref().map(|c| c.needs_download()).unwrap_or(false);
    let nothing = c.as_ref().map(|c| c.wrong.is_empty()).unwrap_or(false);
    let fix_sub = if nothing {
        "Nothing is wrong: nothing to fix".to_string()
    } else if need {
        format!("Download {} again ({}), check its SHA-256, swap it in and write the menu entry again", i.version, mb(i.size, false))
    } else {
        "Allow it to run and write the menu entry again (no download)".to_string()
    };
    let fix_right = if fixing {
        if need && i.size > 0 { format!("{}%", (r.map(|r| r.got).unwrap_or(0) as f64 * 100.0 / i.size as f64).min(100.0) as u32) } else { "Fixing…".into() }
    } else {
        String::new()
    };
    let mut steps = Column::new();
    steps = steps.push(step_row(&p, 1, done, "Check the app".into(), format!("The AppImage against the {} release's SHA-256 list", i.version), s1, None, false));
    steps = steps.push(step_row(&p, 2, done, "Check the menu entry and icon".into(), menu_sub, if c.is_none() { String::new() } else if menu_n == 0 { "Fine".into() } else { format!("{menu_n} to fix") }, None, false));
    steps = steps.push(step_row(&p, 3, done, "Check it can start".into(), if fuse { "AppImage support (FUSE) is there".into() } else { "FUSE isn't here, so it's unpacked to run".into() }, if c.is_none() { String::new() } else if fuse { "Fine".into() } else { "Unpacked to run".into() }, None, false));
    steps = steps.push(step_row(&p, 4, if c.is_some() { 1 } else { 0 }, "Fix what's wrong".into(), fix_sub, fix_right, None, false));
    let said = r.map(|r| r.said.clone()).unwrap_or_default();
    let can = c.is_some() && !fixing;
    let mut left = column![steps, gap(1.0, z(20.0)), row![btn(&p, if fixing { "Repairing…" } else if nothing { "Done" } else { "Repair now" }, B::Primary, true, lit(a, "repair-now"), if can { Some(Msg::RepairNow) } else { None }), btn(&p, "Not now", B::Normal, true, lit(a, "not-now"), Some(Msg::Back))].spacing(z(12.0))];
    if !said.is_empty() {
        left = left.push(container(t(said, f(15.0), look::BAD)).padding(look::pad(z(12.0), 0.0, 0.0, 0.0)));
    }
    let left = container(left).padding(look::pad(z(14.0), z(24.0), z(22.0), z(24.0))).width(Length::FillPortion(84)).style(look::bx(tn.card, 10.0, None));
    let wrong = c.as_ref().map(|c| c.wrong.clone()).unwrap_or_default();
    let every = r.map(|r| r.every_file).unwrap_or(false);
    let mut list = Column::new();
    let mut rows: Vec<(String, Option<hearth_store::files::Wrong>)> = wrong.iter().map(|(n, w)| (n.clone(), Some(*w))).collect();
    if every {
        for (n, ok) in [(e.file.clone(), true), (format!("hearth-store-{}.desktop", e.id), true), (format!("hearth-store-{}.svg", e.id), true)] {
            if ok && !rows.iter().any(|(x, _)| *x == n) {
                rows.push((n, None));
            }
        }
    }
    let nrows = rows.len();
    for (k, (name, w)) in rows.into_iter().enumerate() {
        use hearth_store::files::Wrong as W;
        let (dc, ch): (Color, E<Msg>) = match w {
            Some(W::Changed) => (look::BAD, chip(&p, "Changed", Chip::Bad)),
            Some(W::NotRunnable) => (look::WARN, chip(&p, "Not allowed to run", Chip::Warn)),
            Some(W::Missing) | Some(W::MenuMissing) | Some(W::IconMissing) => (look::WARN, chip(&p, "Missing", Chip::Warn)),
            None => (look::OK_DOT, chip(&p, "Fine", Chip::Ok)),
        };
        let shown = name.clone();
        list = list.push(container(row![look::dot(dc, z(11.0)), look::mono(shown, f(14.0), Pal::mix(tn.text, tn.muted, 0.2)), Space::new().width(Length::Fill), ch].spacing(z(14.0)).align_y(iced::Center)).padding(look::pad(z(11.0), 0.0, z(11.0), 0.0)));
        if k + 1 < nrows {
            list = list.push(ui::hline(tn.line));
        }
    }
    let data_words = if e.data_note.is_empty() { "Its settings and data".to_string() } else { e.data_note.clone() };
    let note = format!("The app is in {}. {} are in their own folders and aren't touched. If the app has updated itself, it's checked against its new version's release.", paths::tilde(&paths::apps()), data_words);
    let title = match c.as_ref() {
        None => "Checking…".to_string(),
        Some(c) if c.wrong.is_empty() => "Nothing is wrong".to_string(),
        Some(c) => format!("What's wrong ({})", c.wrong.len()),
    };
    let right = container(column![ts(title, f(21.0), tn.text), container(list).padding(look::pad(z(14.0), 0.0, z(14.0), 0.0)), t(note, f(15.0), tn.muted)])
        .padding(look::pad(z(26.0), z(24.0), z(24.0), z(24.0)))
        .width(Length::FillPortion(71))
        .height(Length::Shrink)
        .style(look::bx(tn.card, 10.0, None));
    let mid = row![left, right].spacing(z(22.0));
    let same = srow(&p, icon_box(&p, "⟳"), "The same version, put back", "A repair never updates or changes versions. The app's own updater keeps working afterwards.", gap(1.0, 1.0), false, None);
    column![back_link(a), container(head).padding(look::pad(0.0, 0.0, z(26.0), 0.0)), mid, gap(1.0, z(22.0)), same].into()
}

// ------------------------------------------------------------------ 6. Uninstall (store-6)
fn uninstall_dialog(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(d) = &a.dlg else { return gap(1.0, 1.0) };
    let Some(e) = a.entry(&d.id) else { return gap(1.0, 1.0) };
    let i = a.inst.get(&e.id).cloned().unwrap_or_default();
    let size = d.size.map(|n| mb(n, false)).filter(|s| !s.is_empty()).unwrap_or_else(|| if d.size.is_some() { "nothing yet".into() } else { "counting…".into() });
    let note = if e.keep_note.is_empty() { format!("Its settings and data stay in your home folder ({size}) and are there again if you install it later") } else { e.keep_note.replace("{size}", &size) };
    let keep_row = container(row![column![t("Keep my settings and data", f(19.0), tn.text), t(note, f(15.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), switch(&p, d.keep, false, Some(Msg::DlgKeep))].spacing(z(18.0)).align_y(iced::Center))
        .padding(look::pad(z(18.0), z(22.0), z(18.0), z(22.0)))
        .width(Length::Fill)
        .style(look::bx(Pal::mix(tn.card, tn.text, 0.02), 9.0, None));
    let keep_row = ring(d.focus == 0, 9.0, click(keep_row, Some(Msg::DlgKeep)));
    let facts = column![
        kv(&p, "The app", format!("{} {} · AppImage in your home folder · {}", e.name, i.version, mb(i.size, false)), None, false),
        kv(&p, "Its menu entry and icon", "Yes", None, false),
        kv(&p, "Settings and data", if d.keep { "Kept (turn the switch off to move them to the Trash)" } else { "Moved to the Trash (not deleted)" }, None, false),
    ];
    let body = column![
        row![app_icon(e, z(64.0)), tb(format!("Uninstall {}?", e.name), f(30.0), tn.text)].spacing(z(20.0)).align_y(iced::Center),
        gap(1.0, z(22.0)),
        keep_row,
        container(ts("What goes", f(17.0), tn.text)).padding(look::pad(z(20.0), 0.0, z(6.0), 0.0)),
        facts,
        gap(1.0, z(26.0)),
        row![Space::new().width(Length::Fill), btn(&p, "Cancel", B::Normal, true, d.focus == 1, Some(Msg::DlgCancel)), btn(&p, "Uninstall", B::Danger, true, d.focus == 2, Some(Msg::DlgUninstall))].spacing(z(12.0)),
    ];
    container(body).padding(look::pad(z(34.0), z(38.0), z(34.0), z(38.0))).width(Length::Fixed(z(860.0))).style(look::bx(Pal::mix(tn.page, tn.card, 0.6), 12.0, Some((Pal::mix(tn.line, tn.text, 0.08), 1.0)))).into()
}

// ------------------------------------------------------------------ 9. Store settings (store-9)
fn settings(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().push(h1(&p, "Store settings", None));
    c = c.push(container(ts("Updates", f(17.0), tn.muted)).padding(look::pad(0.0, 0.0, z(12.0), 0.0)));
    let daily = seg(&p, vec![("When the Store opens and once a day".into(), a.prefs.daily, Msg::SetDaily(true)), ("Only when the Store opens".into(), !a.prefs.daily, Msg::SetDaily(false))], false);
    let mut rows = Column::new().spacing(z(10.0));
    rows = rows.push(srow(&p, icon_box(&p, "⟳"), "Look for updates", "A few small files from GitHub and hearthos.fun, a few KB. Never while a game runs.", daily, lit(a, "s-daily"), None));
    let bell: E<Msg> = container(canvas(look::Art { kind: "bell".into() }).width(Length::Fixed(z(22.0))).height(Length::Fixed(z(22.0)))).center_x(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0))).style(look::bx(tn.card2, 7.0, None)).into();
    rows = rows.push(srow(&p, bell, "Tell me when an update is ready", "One notice, like \"Hearth AI 0.6.0 is ready\"", switch(&p, a.prefs.notify, false, Some(Msg::SetFlag("notify", !a.prefs.notify))), lit(a, "s-notify"), Some(Msg::SetFlag("notify", !a.prefs.notify))));
    rows = rows.push(srow(&p, icon_box(&p, "⇅"), "Download on metered connections", "Phone hotspots and other connections marked metered", switch(&p, a.prefs.metered, false, Some(Msg::SetFlag("metered", !a.prefs.metered))), lit(a, "s-metered"), Some(Msg::SetFlag("metered", !a.prefs.metered))));
    rows = rows.push(srow(&p, icon_box(&p, "⚑"), "Get test versions", "New versions reach this PC before everyone else, marked TEST. For trying them on your own devices first.", switch(&p, a.prefs.test, false, Some(Msg::SetFlag("test", !a.prefs.test))), lit(a, "s-test"), Some(Msg::SetFlag("test", !a.prefs.test))));
    c = c.push(rows);
    c = c.push(sec(&p, "Where apps go on this PC", None));
    let list_value: E<Msg> = {
        let (what, when) = if a.cat.source == "hearthos.fun" {
            ("hearthos.fun/store · signature checked".to_string(), a.cat.signed_at)
        } else {
            (format!("Built into {}", if a.hearth.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {}", a.hearth) }), a.seen.checked)
        };
        let mut r = row![t(what, f(16.0), tn.text)].spacing(z(12.0)).align_y(iced::Center);
        if when > 0 {
            r = r.push(row![look::dot(if a.offline() { look::WARN } else { look::OK_DOT }, z(9.0)), t(day_time(when).replace("today at ", "today "), f(16.0), tn.text)].spacing(z(7.0)).align_y(iced::Center));
        }
        r.into()
    };
    let facts = column![
        kv(&p, "This PC", this_pc(a), None, false),
        kv(&p, "Apps install as", format!("AppImages in your home folder ({}), no password or restart", paths::tilde(&paths::apps())), None, false),
        row![t1("List of apps", f(16.0), tn.muted), Space::new().width(Length::Fill), list_value].align_y(iced::Center).padding(look::pad(z(10.0), 0.0, z(10.0), 0.0)),
    ];
    c = c.push(card(&p, facts, false));
    c = c.push(sec(&p, "Related", None));
    c = c.push(row![
        container(rel_card(a, "r-flathub", "⬚", "App store (Flathub)", "Settings › Apps", Msg::OpenFlathub)).width(Length::FillPortion(1)),
        container(rel_card(a, "r-folder", "▤", "Your Store apps' folder", "Open in Files", Msg::OpenFolder)).width(Length::FillPortion(1)),
        container(rel_card(a, "r-updates", "⟲", "Hearth OS updates", "Settings › Updates & recovery", Msg::OpenHearthUpdates)).width(Length::FillPortion(1)),
    ]
    .spacing(z(16.0)));
    if a.prefs.unreadable {
        c = c.push(container(t("The Store's settings file couldn't be read, so nothing is written over it (it's left for you to look at).", f(15.0), look::BAD)).padding(look::pad(z(14.0), 0.0, 0.0, 0.0)));
    }
    c.into()
}

fn rel_card<'a>(a: &'a App, id: &'static str, sym: &'static str, title: &'static str, sub: &'static str, m: Msg) -> E<'a, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let r = row![icon_box(&p, sym), column![t(title, f(19.0), tn.text), t(sub, f(15.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), t("›", f(18.0), tn.text)].spacing(z(18.0)).align_y(iced::Center);
    ring(lit(a, id), 9.0, click(container(r).padding(look::pad(z(18.0), z(22.0), z(18.0), z(22.0))).width(Length::Fill).style(look::bx(tn.card, 9.0, None)), Some(m)))
}

// ------------------------------------------------------------------ 7. The controller look (store-7)
fn pad_frame(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let n_up = a.ready().len() + a.part_offers().len();
    let pill = |s: &'static str| -> E<'static, Msg> { container(tb(s, f(13.0), Pal::mix(tn.text, tn.muted, 0.2))).padding(look::pad(z(2.0), z(10.0), z(2.0), z(10.0))).style(look::bx(Color::TRANSPARENT, 12.0, Some((tn.muted, 1.5)))).into() };
    let mut tabs = Row::new().spacing(z(8.0)).align_y(iced::Center).push(pill("LB"));
    for (pg, label) in [(Page::Home, "Home"), (Page::Yours, "Your apps"), (Page::Updates, "Updates"), (Page::BuiltIn, "Built in"), (Page::Settings, "Settings")] {
        let on = a.page == pg;
        let mut r = row![if on { ts(label, f(21.0), tn.text) } else { t(label, f(21.0), Pal::mix(tn.text, tn.muted, 0.25)) }].spacing(z(8.0)).align_y(iced::Center);
        if pg == Page::Updates && n_up > 0 {
            r = r.push(container(tb(n_up.to_string(), f(12.0), look::INK)).center_x(Length::Fixed(z(22.0))).center_y(Length::Fixed(z(22.0))).style(look::bx(ORANGE, z(11.0), None)));
        }
        let c = container(r).padding(look::pad(z(10.0), z(20.0), z(10.0), z(20.0))).style(look::bx(if on { tn.card2 } else { Color::TRANSPARENT }, z(22.0), if on { Some((ORANGE, 1.5)) } else { None }));
        tabs = tabs.push(click(c, Some(Msg::Go(pg))));
    }
    tabs = tabs.push(pill("RB"));
    let top = row![look::mark(z(48.0)), gap(z(20.0), 1.0), tb("Hearth Store", f(30.0), tn.text), gap(z(80.0), 1.0), tabs, Space::new().width(Length::Fill), t(hearth_ui::time::clock(a.now), f(19.0), tn.muted)].align_y(iced::Center);
    let top = container(top).padding(look::pad(z(40.0), z(70.0), z(20.0), z(70.0))).width(Length::Fill);
    let body: E<Msg> = match &a.page {
        Page::Home => pad_home(a),
        _ => {
            let b = page_body(a);
            overlays(a, b)
        }
    };
    let body = container(scrollable(container(body).padding(look::pad(z(16.0), z(70.0), z(30.0), z(70.0))).width(Length::Fill)).height(Length::Fill)).height(Length::Fill);
    let hints: E<Msg> = match &a.page {
        Page::Home => look::hints(&p, vec![hint(&p, "A", "Update"), hint(&p, "Y", "What's new"), hint(&p, "≡", "More (repair, uninstall)"), hint(&p, "X", "Update all"), hint(&p, "B", "Back")], vec![hint2(&p, "LB", "RB", "Pages"), hint(&p, "⧉", "Search")]),
        _ => bottom(a),
    };
    container(column![top, body, hints]).width(Length::Fill).height(Length::Fill).style(look::bx(tn.page, 0.0, None)).into()
}

fn pad_home(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let (hero, top, more) = pad_rows(a);
    let mut c = Column::new();
    let mut first = Row::new().spacing(z(30.0)).align_y(iced::Bottom);
    let mut info: Option<E<Msg>> = None;
    if let Some((e, st)) = &hero {
        let from = look::hex(&e.from, ORANGE);
        let to = look::hex(&e.to, ORANGE);
        let (sub, label) = match st {
            State::Ready(i, r) => (format!("{} → {}{}", i.version, r.version, if e.hero_short.is_empty() { String::new() } else { format!(" · {}", e.hero_short) }), if e.self_updates { "Open to update" } else { "Update" }),
            State::Get(r) => (format!("{} · {}", r.version, e.card), "Install"),
            _ => (e.card.clone(), "Open"),
        };
        let abtn = container(row![container(tb("A", f(12.0), look::INK)).center_x(Length::Fixed(z(26.0))).center_y(Length::Fixed(z(26.0))).style(look::bx(Pal::a(Color::WHITE, 0.35), z(13.0), None)), tb(label, f(19.0), look::INK)].spacing(z(10.0)).align_y(iced::Center))
            .padding(look::pad(z(14.0), z(30.0), z(14.0), z(30.0)))
            .style(look::bx(ORANGE, 8.0, None));
        let ybtn = container(row![container(tb("Y", f(12.0), tn.text)).center_x(Length::Fixed(z(26.0))).center_y(Length::Fixed(z(26.0))).style(look::bx(Pal::a(Color::BLACK, 0.35), z(13.0), None)), t("What's new", f(19.0), tn.text)].spacing(z(10.0)).align_y(iced::Center))
            .padding(look::pad(z(14.0), z(30.0), z(14.0), z(30.0)))
            .style(look::bx(Color::TRANSPARENT, 8.0, Some((Pal::mix(tn.line, tn.text, 0.1), 1.0))));
        let inner = column![
            app_icon(e, z(120.0)),
            gap(1.0, z(30.0)),
            tb(e.name.clone(), f(46.0), tn.text),
            container(t(sub, f(22.0), Pal::mix(PURPLE, tn.text, 0.4))).padding(look::pad(z(6.0), 0.0, z(30.0), 0.0)),
            row![click(abtn, primary(e, st)), click(ybtn, Some(Msg::OpenApp(e.id.clone())))].spacing(z(16.0)),
        ];
        let card_ = container(inner).padding(look::pad(z(44.0), z(44.0), z(44.0), z(44.0))).width(Length::Fixed(z(776.0))).height(Length::Fixed(z(440.0))).style(look::grad(160.0, Pal::mix(tn.card, from, 0.3), Pal::mix(tn.card, to, 0.25), 12.0, None));
        first = first.push(ring(lit(a, "hero"), 12.0, card_));
        if let State::Ready(_, r) | State::Get(r) = st {
            let mut facts = Column::new();
            facts = facts.push(kv(&p, "Installs as", "AppImage in your home folder", None, false));
            facts = facts.push(kv(&p, "Download", dl_size(e, r.size), None, false));
            facts = facts.push(kv(&p, if e.data_note.is_empty() { "Its settings and data".to_string() } else { e.data_note.clone() }, "Stay as they are", None, true));
            let notes: Vec<String> = r.notes.iter().take(2).cloned().collect();
            let left = column![ts(format!("What's new in {} {}", e.name, short_version(&r.version)), f(21.0), tn.text), container(bullets(&p, &notes, f(19.0))).padding(look::pad(z(12.0), 0.0, 0.0, 0.0))].width(Length::FillPortion(107));
            info = Some(
                container(row![left, ui::vline(tn.line), container(facts).padding(look::pad(0.0, 0.0, 0.0, z(30.0))).width(Length::FillPortion(75))].spacing(z(30.0)).height(Length::Shrink))
                    .padding(look::pad(z(26.0), z(32.0), z(26.0), z(32.0)))
                    .width(Length::Fill)
                    .style(look::bx(tn.card, 12.0, None))
                    .into(),
            );
        }
    }
    for e in &top {
        let st = a.state(e);
        let line = match &st {
            State::UpToDate(i) => format!("{} · Installed", i.version),
            State::Ready(i, r) => format!("{} → {}", i.version, r.version),
            State::BuiltIn => "Built in".into(),
            State::Coming => "Coming".into(),
            State::Get(r) => format!("{} · Install", r.version),
        };
        let from = look::hex(&e.from, ORANGE);
        let id = format!("top-{}", e.id);
        let inner = column![app_icon(e, z(96.0)), gap(1.0, z(30.0)), tb(e.short.clone(), f(25.0), tn.text).wrapping(iced::widget::text::Wrapping::None), t(line, f(19.0), tn.muted)].spacing(z(6.0));
        let tile = container(inner).padding(z(26.0)).width(Length::FillPortion(1)).height(Length::Fixed(z(380.0))).style(look::grad(180.0, Pal::mix(tn.card, from, 0.18), Pal::mix(tn.page, tn.card, 0.5), 12.0, None));
        first = first.push(container(ring(lit(a, &id), 12.0, click(tile, Some(Msg::OpenApp(e.id.clone()))))).width(Length::FillPortion(1)));
    }
    c = c.push(first);
    if let Some(i) = info {
        c = c.push(gap(1.0, z(34.0))).push(i);
    }
    c = c.push(container(ts("More apps", f(24.0), tn.text)).padding(look::pad(z(40.0), 0.0, z(20.0), 0.0)));
    let mut r = Row::new().spacing(z(24.0));
    for e in more.iter().take(4) {
        let st = a.state(e);
        let line = match &st {
            State::BuiltIn => "Built in".to_string(),
            State::Coming => "Coming".into(),
            State::UpToDate(_) => "Installed".into(),
            State::Ready(..) => "Update ready".into(),
            State::Get(_) => "Install".into(),
        };
        let id = format!("more-{}", e.id);
        let inner = row![app_icon(e, z(64.0)), column![ts(e.name.clone(), f(22.0), tn.text), t(line, f(17.0), tn.muted)].spacing(z(4.0))].spacing(z(18.0)).align_y(iced::Center);
        r = r.push(container(ring(lit(a, &id), 10.0, click(container(inner).padding(z(20.0)).width(Length::Fill).style(look::bx(tn.card, 10.0, None)), Some(Msg::OpenApp(e.id.clone()))))).width(Length::Fixed(z(424.0))));
    }
    c = c.push(r);
    c.into()
}
