//! feature/server-2's pages in the Home server helper: Game servers, Game cache, Remote play, Media and PCs
//! (hearth_server_more, boards docs/mockups/server-expansion/05, 07, 08, 09, 10). Each is drawn by its own module in
//! hosts/hearth-ui/apps/server-more and does its work through Mantel's hearth-server-hub; the window keeps one
//! [`hearth_server_more::Hub`] (App::more) and passes its messages on (Msg::More).

use super::Page;
use crate::{App, Msg, Tags, E};
use hearth_oss::w::Tg;
use hearth_server_more::Part;
use iced::Task;

fn tags_of(a: &App, part: Part) -> Tags {
    let t = a.more.tag(part);
    if t.is_empty() { Vec::new() } else { vec![(t, Tg::Ok)] }
}

macro_rules! page {
    ($name:ident, $view:ident, $tags:ident, $part:expr, $id:literal, $title:literal) => {
        fn $view(a: &App) -> E<'_> {
            a.more.view($part, &a.describe, a.p).map(Msg::More)
        }
        fn $tags(a: &App) -> Tags {
            tags_of(a, $part)
        }
        pub const $name: Page = Page { id: $id, title: $title, side: $id, back: "home", engines: &[], every: 15, bare: false, view: $view, tags: $tags, key: super::no_key };
    };
}

page!(GAMES, games_view, games_tags, Part::Games, "games", "Game servers");
page!(CACHE, cache_view, cache_tags, Part::Cache, "cache", "Game cache");
page!(PLAY, play_view, play_tags, Part::Play, "play", "Remote play");
page!(MEDIA, media_view, media_tags, Part::Media, "media", "Media");
page!(PCS, pcs_view, pcs_tags, Part::Pcs, "pcs", "PCs");

/// Opening one of these pages: its view read from hearth-server-hub.
pub fn open(id: &str) -> Option<Task<Msg>> {
    Part::of(id).map(|p| hearth_server_more::load(p).map(Msg::More))
}

/// Every one of these pages' views, for the sidebar's counts (once at the start).
pub fn load_all() -> Task<Msg> {
    Task::batch(Part::ALL.iter().map(|p| hearth_server_more::load(*p).map(Msg::More)))
}

/// The sidebar's count for one of these parts ("3 on", "612 GB"), "" for the others.
pub fn count(a: &App, id: &str) -> String {
    Part::of(id).map(|p| a.more.count(p)).unwrap_or_default()
}
