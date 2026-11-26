//! hearth-server-more-app: the Home server helper's Game servers, Game cache, Remote play, Media and PCs pages in a
//! small window of their own, laid out as the boards draw the Home server helper (its sidebar on the left). The Home
//! server helper's own window takes these pages with one line each (hearth_server_more::SIDE); until the two meet,
//! this is where they're opened and pictured.
//!
//!   hearth-server-more-app [--page games|cache|play|media|pcs]

use hearth_server_more::kit::{self, pad, Tag, E};
use hearth_server_more::{load, Hub, Msg as HMsg, Part, SIDE};
use hearth_ui::app::{self, win};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::studio::{tile, Kit};
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Space};
use iced::{Color, Element, Length, Subscription, Task};

#[derive(Debug, Clone)]
enum Msg {
    H(HMsg),
    Go(Part),
    Theme,
    Tick,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    hub: Hub,
    part: Part,
    server: String,
    up: String,
}

const WINDOW: PageDef = PageDef { id: "srv-window", title: "Home server helper", blocks: &[bl("side", "The sidebar", 0, 0, 2, 12), bl("page", "The page", 2, 0, 10, 12)] };

const KIT: Kit = Kit {
    name: "Home server helper",
    buttons: &[tile("games", "Game servers", G::Pad), tile("cache", "Game cache", G::Down), tile("play", "Remote play", G::Play), tile("media", "Media", G::Display), tile("pcs", "PCs", G::Server)],
    panes: &[],
    info: &[],
    pane_page: "srv-window",
    bars: &[],
    menus: &[],
};

fn uptime() -> String {
    let secs = std::fs::read_to_string("/proc/uptime").ok().and_then(|s| s.split('.').next().and_then(|x| x.parse::<u64>().ok())).unwrap_or(0);
    let days = std::env::var("HEARTH_SERVER_UP_DAYS").ok().and_then(|x| x.parse::<u64>().ok()).unwrap_or(secs / 86400);
    if days >= 1 { format!("up {days} days") } else { format!("up {} h", secs / 3600) }
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let mut pages = vec![WINDOW];
        pages.extend(hearth_server_more::blocks());
        let pages: &'static [PageDef] = Box::leak(pages.into_boxed_slice());
        let part = app::arg("--page").and_then(|p| Part::of(&p)).unwrap_or(Part::Games);
        let server = std::env::var("HEARTH_SERVER_NAME").unwrap_or_else(|_| std::fs::read_to_string("/etc/hostname").unwrap_or_default().trim().to_string());
        let a = App { describe: hearth_ui::describe::Describe::new("home-server", "Home server helper").with_pages(pages).with_kit(&KIT), p: hearth_ui::theme::load(), hub: Hub::new(), part, server, up: uptime() };
        // the open page first, then the others (their counts in the sidebar)
        let mut tasks = vec![load(part).map(Msg::H)];
        for x in Part::ALL {
            if x != part {
                tasks.push(load(x).map(Msg::H));
            }
        }
        (a, Task::batch(tasks))
    }
}

fn block_action(id: &str) -> Option<Msg> {
    Part::of(id).map(Msg::Go)
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::H(HMsg::D(dm)) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(|d| Msg::H(HMsg::D(d)));
        }
        Msg::H(hm) => return a.hub.update(hm).map(Msg::H),
        Msg::Go(p) => {
            a.part = p;
            a.hub.said.clear();
            a.hub.pick = "";
            return load(p).map(Msg::H);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {
            if !a.hub.busy {
                return load(a.part).map(Msg::H);
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let item = |label: &'static str, count: String, part: Option<Part>| -> E<'static, Msg> {
        let on = part == Some(a.part);
        let fg = if on { p.text } else if part.is_some() { p.text } else { p.muted };
        let inner = row![kit::square(if on { Color::from_rgb8(0xee, 0xe6, 0xdf) } else { p.muted }, 16.0, 4.0, None), t(label, 13.0, fg), Space::new().width(Length::Fill), t(count, 11.0, p.muted)].spacing(10.0).align_y(iced::Center);
        let body = container(inner).padding(pad(0.0, 10.0, 0.0, 10.0)).center_y(Length::Fixed(36.0)).width(Length::Fill);
        let el: E<'static, Msg> = if on {
            container(row![container(Space::new()).width(Length::Fixed(3.0)).height(Length::Fixed(36.0)).style(ui::boxed(p.accent, 0.0, None)), body]).clip(true).style(ui::boxed(Color::from_rgb8(0x3b, 0x2a, 0x1d), 9.0, None)).into()
        } else {
            body.into()
        };
        kit::click(el, part.map(Msg::Go))
    };
    let mut c = Column::new()
        .spacing(2.0)
        .push(container(t("⌕ Search the server", 13.0, p.muted)).padding(pad(9.0, 12.0, 9.0, 12.0)).width(Length::Fill).style(ui::boxed(p.panel, 8.0, Some((p.line, 1.0)))))
        .push(container(kit::caps(&p, "Home server")).padding(pad(18.0, 0.0, 6.0, 0.0)))
        // The Home server helper's own pages (Server home, Dashboard, Model vault, Backups) are its window's own.
        .push(item("Server home", String::new(), None))
        .push(item("Dashboard", String::new(), None))
        .push(item("Model vault", String::new(), None));
    let mut seen_backups = false;
    for (_, label, part) in SIDE {
        if *part == Part::Cache && !seen_backups {
            c = c.push(item("Backups", String::new(), None));
            seen_backups = true;
        }
        c = c.push(item(label, a.hub.count(*part), Some(*part)));
    }
    c = c
        .push(container(kit::caps(&p, "This server")).padding(pad(22.0, 0.0, 6.0, 0.0)))
        .push(row![hearth_server_more::kit::dot(kit::GREEN, 9.0), t(a.server.clone(), 12.0, p.text)].spacing(6.0).align_y(iced::Center))
        .push(t(format!("Hearth OS · {}", a.up), 12.0, p.muted))
        .push(container(row![t("●", 12.0, p.accent), t("Updates in the Hearth Store", 12.0, p.text)].spacing(6.0)).padding(pad(12.0, 0.0, 0.0, 0.0)))
        .push(t("every part is its own Store app · never a Hearth OS update", 12.0, p.muted).line_height(1.5))
        .push(Space::new().height(Length::Fill))
        .push(kit::click(row![t("Made of blocks · ", 12.0, p.muted), tb("Edit layout", 12.0, p.accent)], Some(Msg::H(HMsg::D(hearth_ui::arrange::edit_layout())))));
    c.into()
}

fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let page = a.hub.view(a.part, &a.describe, p).map(Msg::H);
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![
        ("side", hearth_ui::studio::made(&a.describe, "srv-window", "side", || container(side(a)).width(Length::Fixed(230.0)).height(Length::Fill).padding(pad(14.0, 12.0, 16.0, 12.0)).into())),
        ("page", page),
    ];
    let body = hearth_ui::arrange::parts(&a.describe, &p, "srv-window", items, move |ps| row![ps.take("side"), ui::vline(p.line), container(ps.take("page")).width(Length::Fill).height(Length::Fill)].height(Length::Fill).into(), |d| Msg::H(HMsg::D(d)));
    let tag_s = a.hub.tag(a.part);
    let status = if tag_s.is_empty() { kit::tag(&p, "Looking…", Tag::Dark) } else { kit::tag(&p, tag_s, Tag::Ok) };
    let bar = kit::title_bar(
        &p,
        "HS",
        Color::from_rgb8(0xff, 0xb3, 0x6b),
        Color::from_rgb8(0xff, 0x6a, 0x1f),
        "Home server helper",
        a.part.title(),
        vec![status],
        TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close },
        Msg::H(HMsg::D(hearth_ui::describe::DMsg::Open)),
    );
    let all = container(column![bar, container(body).height(Length::Fill)]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, |d| Msg::H(HMsg::D(d)))
}

fn subscription(_a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    // what's playing and who's on, every 15 seconds while the window is open (one small read of the open page)
    if std::env::var_os("HEARTH_SERVER_HUB_FIXTURE").is_none() {
        s.push(iced::time::every(std::time::Duration::from_secs(15)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-server-more-app", "The Home server helper's game servers, game cache, remote play, media and PCs.\n  --page games|cache|play|media|pcs") {
        return Ok(());
    }
    let w = app::window("hearth-server-more", (1840.0, 980.0), (1100.0, 700.0));
    iced::application(App::new, update, shown)
        .title(|a: &App| format!("Home server helper · {}", a.part.title()))
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(w)
        .default_font(app::INTER)
        .run()
}
