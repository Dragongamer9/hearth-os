//! The Home server helper's pages, one module each. A new page is its module here and one line in [`PAGES`] and one
//! in [`BLOCKS`] (its blocks for Edit layout); its part's sidebar row comes from hearth_server_parts::parts.

pub mod backups;
pub mod dash;
pub mod home;
pub mod more;
pub mod tv;
pub mod vault;
pub mod vault_checks;

use crate::{App, Msg, Tags, E};
use hearth_ui::blocks::PageDef;
use iced::keyboard::Key;

pub struct Page {
    pub id: &'static str,
    /// in the title bar after "Home server helper ·"
    pub title: &'static str,
    /// the sidebar row it lights
    pub side: &'static str,
    /// where B (Esc) goes ("" nowhere)
    pub back: &'static str,
    /// the engines whose `status` it shows
    pub engines: &'static [&'static str],
    /// read again every this many seconds while it's open (0: only when it opens and after a change)
    pub every: u64,
    /// drawn on its own, without the window around it (the TV, the phone)
    pub bare: bool,
    pub view: fn(&App) -> E<'_>,
    pub tags: fn(&App) -> Tags,
    pub key: fn(&App, Key<&str>, bool) -> Option<Msg>,
}

pub fn no_tags(_a: &App) -> Tags {
    Vec::new()
}
pub fn no_key(_a: &App, _k: Key<&str>, _shift: bool) -> Option<Msg> {
    None
}

pub const PAGES: &[Page] = &[
    home::PAGE,
    dash::PAGE,
    tv::PAGE,
    tv::PHONE,
    vault::PAGE,
    vault_checks::PAGE,
    backups::PAGE,
    more::GAMES,
    more::CACHE,
    more::PLAY,
    more::MEDIA,
    more::PCS,
];

pub const BLOCKS: &[PageDef] = &[
    crate::WINDOW,
    home::BLOCKS,
    dash::BLOCKS,
    tv::BLOCKS,
    tv::PHONE_BLOCKS,
    vault::BLOCKS,
    vault_checks::BLOCKS,
    backups::BLOCKS,
    hearth_server_more::games::PAGE,
    hearth_server_more::games::PANEL,
    hearth_server_more::cache::PAGE,
    hearth_server_more::cache::PANEL,
    hearth_server_more::play::PAGE,
    hearth_server_more::play::PANEL,
    hearth_server_more::media::PAGE,
    hearth_server_more::media::PANEL,
    hearth_server_more::pcs::PAGE,
    hearth_server_more::pcs::PANEL,
];
