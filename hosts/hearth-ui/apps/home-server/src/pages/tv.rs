//! The TV view (being built: docs/mockups/checklists/server-*.md).

use super::Page;
use crate::{App, E};
use hearth_ui::blocks::{bl, PageDef};

pub const PAGE: Page = Page { id: "tv", title: "The TV view", side: "dash", back: "", engines: &["dash"], every: 0, bare: false, view, tags: super::no_tags, key: super::no_key };
pub const BLOCKS: PageDef = PageDef { id: "tv", title: "The TV view", blocks: &[bl("main", "The TV view", 0, 0, 12, 12)] };

fn view(a: &App) -> E<'_> {
    hearth_oss::w::h1(&a.p, "The TV view")
}

pub const PHONE: Page = Page { id: "phone", title: "Dashboard on the phone", side: "dash", back: "dash", engines: &["dash"], every: 0, bare: true, view, tags: super::no_tags, key: super::no_key };
pub const PHONE_BLOCKS: PageDef = PageDef { id: "phone", title: "The phone", blocks: &[bl("main", "The phone", 0, 0, 12, 12)] };
