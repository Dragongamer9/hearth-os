//! One module per category, each with the same shape (so each can be worked on by itself):
//!
//! ```ignore
//! pub const PAGES: &[&str];                       // the pages it draws (registry.rs: "built")
//! #[derive(Default)] pub struct State { .. }      // what it has read
//! #[derive(Debug, Clone)] pub enum Msg { .. }     // what its pages ask for
//! pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg>   // a page of it opens: read
//! pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg>
//! pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>>     // None: not one of its pages
//! pub fn subscription(st: &State) -> Subscription<Msg>                             // timers while a page needs one
//! pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)>       // a dialog over the page, and
//!                                                                                  // what a click outside sends
//! pub fn back(st: &mut State) -> bool                                             // Back closed something of its own
//! ```
//! Going to another page: `sh.go(id)`. Changing Hearth's settings: `sh.desktop_cmd(json)` (and `json_cmd`,
//! `conf_cmd`), run with hearth_ui::run::spawn; never write a settings file directly (red flag 9).

pub mod a11y;
pub mod ai;
pub mod accounts;
pub mod apps;
pub mod bt;
pub mod couch;
pub mod g15;
pub mod gaming;
pub mod home;
pub mod kde;
pub mod hs;
pub mod hs_board;
pub mod link;
pub mod net;
pub mod pers;
pub mod privacy;
pub mod sections;
pub mod system;
pub mod updates;

pub fn built(id: &str) -> bool {
    [home::PAGES, sections::PAGES, ai::PAGES, system::PAGES, couch::PAGES, net::PAGES, bt::PAGES, g15::PAGES, link::PAGES, pers::PAGES, apps::PAGES, gaming::PAGES, accounts::PAGES, hs::PAGES, privacy::PAGES, a11y::PAGES, updates::PAGES, kde::PAGES]
        .iter()
        .any(|l| l.contains(&id))
}
