//! Dashboard (being built: docs/mockups/checklists/server-*.md).

use super::Page;
use crate::{App, E};
use hearth_ui::blocks::{bl, PageDef};

pub const PAGE: Page = Page { id: "dash", title: "Dashboard", side: "dash", back: "", engines: &["dash"], every: 0, bare: false, view, tags: super::no_tags, key: super::no_key };
pub const BLOCKS: PageDef = PageDef { id: "dash", title: "Dashboard", blocks: &[bl("main", "Dashboard", 0, 0, 12, 12)] };

fn view(a: &App) -> E<'_> {
    hearth_oss::w::h1(&a.p, "Dashboard")
}
