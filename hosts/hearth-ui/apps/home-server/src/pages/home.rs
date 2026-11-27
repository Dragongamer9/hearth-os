//! Server home: the same board as Settings › Home server (board 11; hearth_server_parts::home): the overall state, a
//! tile for each part, your devices, Updates for the server and what else is here. Each tile is its own block.

use super::Page;
use crate::{App, Msg, E};
use hearth_server_parts::home::{self as h, HAct};
use hearth_server_parts::parts;
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::ui;
use iced::widget::{column, container, row, Column};
use iced::Length;

pub const PAGE: Page = Page { id: "home", title: "Server home", side: "home", back: "", engines: &[], every: 0, bare: false, view, tags, key: super::no_key };

/// Every part's tile is a block (the parts' ids; a part that isn't here just isn't drawn).
pub const BLOCKS: PageDef = PageDef {
    id: "home",
    title: "Server home",
    blocks: &[
        bl("state", "How the server is doing", 0, 0, 9, 1),
        bl("dash", "Dashboard", 0, 1, 2, 4),
        bl("vault", "Model vault", 2, 1, 2, 4),
        bl("games", "Game servers", 4, 1, 2, 4),
        bl("backup", "Backups", 6, 1, 3, 4),
        bl("cache", "Game cache", 0, 5, 2, 4),
        bl("play", "Remote play", 2, 5, 2, 4),
        bl("media", "Media", 4, 5, 2, 4),
        bl("pcs", "PCs", 6, 5, 3, 4),
        bl("devices", "Your devices", 0, 9, 9, 2),
        bl("foot", "Each tile is a block", 0, 11, 9, 1),
        bl("updates", "Updates for the server", 9, 0, 3, 6),
        bl("also", "Also here", 9, 6, 3, 6),
    ],
};

fn tags(a: &App) -> crate::Tags {
    let (ok, tot) = h::healthy(&a.home);
    if tot > 0.0 {
        vec![(format!("{ok} of {tot} healthy"), if ok < tot { hearth_oss::w::Tg::Warn } else { hearth_oss::w::Tg::Ok })]
    } else {
        Vec::new()
    }
}

fn m(x: HAct) -> Msg {
    Msg::HomeAct(x)
}

/// The rows of Also here: Settings' Home server pages (they open in Settings).
pub fn also_rows(a: &App) -> Vec<(&'static str, String, &'static str)> {
    let o = a.home.overall();
    let v = |k: &str, d: &str| {
        let x = hearth_server_parts::s(o, &format!("also.{k}"));
        if x.is_empty() { d.to_string() } else { x }
    };
    vec![
        ("Shared folders", v("shares", ""), "hs-shares"),
        ("Files & links", v("files", ""), "hs-files"),
        ("People", v("people", ""), "hs-people"),
        ("Drives & undo", v("drives", ""), "hs-drives"),
        ("Safety", v("safety", ""), "hs-safety"),
        ("Alerts & power", v("alerts", ""), "hs-alerts"),
        ("The other services", v("others", "Cloud, Home, Shield…"), "hs-services"),
    ]
}

fn view(a: &App) -> E<'_> {
    let p = a.p;
    let d = &a.home;
    let sel = a.ui_n("homeSel");
    let mut items: Vec<(&'static str, E<'_>)> = vec![("state", h::state_card(&p, d, m))];
    let order: Vec<&'static parts::Part> = parts::sorted();
    for (i, pt) in order.iter().copied().enumerate() {
        items.push((pt.id, h::tile(&p, d, pt, i == sel, m)));
    }
    items.push(("devices", h::devices(&p, d)));
    items.push(("foot", h::foot(&p, m)));
    let notes = a.ui_s("notes", "");
    items.push(("updates", h::updates(&p, d, &notes, m)));
    items.push(("also", h::also(&p, also_rows(a), m)));
    let ids: Vec<&'static str> = order.iter().map(|x| x.id).collect();
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "home",
        items,
        move |ps| {
            let tiles: Vec<E<'_>> = ids.iter().map(|id| ps.take(id)).collect();
            let main = column![
                container(column![hearth_oss::w::h1(&p, "Server home"), hearth_oss::w::mu(&p, "This PC serving your home: AI, games, backups, media and more · every part is its own Hearth Store app", 12.0)].spacing(4.0)).padding(crate::home_pad(0.0, 0.0, 14.0, 0.0)),
                ps.take("state"),
                container(h::tiles_grid(tiles, 4, 293.0)).padding(crate::home_pad(16.0, 0.0, 14.0, 0.0)),
                ps.take("devices"),
                container(ps.take("foot")).padding(crate::home_pad(10.0, 0.0, 0.0, 0.0)),
                crate::said(a),
            ];
            let side = Column::new().push(ps.take("updates")).push(ps.take("also"));
            row![
                container(iced::widget::scrollable(container(main).padding(crate::home_pad(20.0, 26.0, 20.0, 26.0)))).width(Length::Fill).height(Length::Fill),
                ui::vline(p.line),
                container(side).padding(20.0).width(Length::Fixed(360.0)).height(Length::Fill),
            ]
            .into()
        },
        Msg::Describe,
    );
    body
}
