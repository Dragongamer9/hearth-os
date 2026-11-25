//! Home server: all its pages, ported from Mantel's SetHome*.qml, SetServerMode.qml and SetAIEngine.qml with the same
//! helpers (hearth-home-server, hearth-server-mode, hearth-server-apps, hearth-colibri, hearth-ai-share), so Debian's
//! Mantel and the Fedora edition's Settings share every setting. The pieces and the running are in hs/kit.rs; the
//! pages in hs/overview.rs (Home server, Set up, Server mode, AI engine), hs/sharing.rs (Shared folders, Files &
//! links, People, Family & home), hs/services.rs (Services, AI chat, AI devices, Server apps, Phone notifications,
//! Gaming) and hs/safe.rs (Drives & undo, Backups & sync, Safety, Alerts & power). See cats/mod.rs for the shape.

pub mod kit;
mod overview;
mod safe;
pub(crate) mod services;
mod sharing;

use crate::shared::Shared;
use crate::App;
use iced::{Element, Subscription, Task};
use kit::{Hooks, Rd, St};

pub const PAGES: &[&str] = &[
    "hs-overview", "hs-server", "hs-aiengine", "hs-setup", "hs-shares", "hs-files", "hs-people", "hs-family", "hs-services", "hs-ai", "hs-aidevices", "hs-apps", "hs-phone",
    "hs-gaming", "hs-drives", "hs-backups", "hs-safety", "hs-alerts",
];

pub type State = St;
pub type Msg = kit::Msg;

fn reads(page: &str, st: &St) -> Vec<Rd> {
    match page {
        "hs-overview" | "hs-setup" | "hs-server" | "hs-aiengine" => overview::reads(page, st),
        "hs-shares" | "hs-files" | "hs-people" | "hs-family" => sharing::reads(page, st),
        "hs-services" | "hs-ai" | "hs-aidevices" | "hs-apps" | "hs-phone" | "hs-gaming" => services::reads(page, st),
        _ => safe::reads(page, st),
    }
}

fn follow(page: &str, slot: &str, st: &St) -> Vec<Rd> {
    match page {
        "hs-aiengine" => overview::follow(page, slot, st),
        "hs-aidevices" => services::follow(page, slot, st),
        "hs-alerts" => safe::follow(page, slot, st),
        _ => Vec::new(),
    }
}

fn submit(st: &mut St, key: &str, text: String) -> Result<Vec<kit::Act>, String> {
    match st.page {
        "hs-overview" | "hs-setup" | "hs-server" | "hs-aiengine" => overview::submit(st, key, text),
        "hs-shares" | "hs-files" | "hs-people" | "hs-family" => sharing::submit(st, key, text),
        "hs-services" | "hs-ai" | "hs-aidevices" | "hs-apps" | "hs-phone" | "hs-gaming" => services::submit(st, key, text),
        _ => safe::submit(st, key, text),
    }
}

fn live(page: &str, st: &St) -> bool {
    match page {
        "hs-setup" | "hs-aiengine" => overview::live(page, st),
        "hs-files" => sharing::live(page, st),
        "hs-services" | "hs-ai" | "hs-apps" => services::live(page, st),
        "hs-backups" => safe::live(page, st),
        _ => false,
    }
}

pub const HOOKS: Hooks = Hooks { reads, follow, submit, live };

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    let part = std::mem::take(&mut sh.part);
    kit::open(st, page, &part, &HOOKS)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    kit::update(sh, st, m, &HOOKS)
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if !PAGES.contains(&page) {
        return None;
    }
    let p = a.sh.p;
    let st = &a.hs;
    Some(match page {
        "hs-overview" => overview::overview(&p, st),
        "hs-setup" => overview::setup(&p, st),
        "hs-server" => overview::server(&p, st),
        "hs-aiengine" => overview::aiengine(&p, st),
        "hs-shares" => sharing::shares(&p, st),
        "hs-files" => sharing::files(&p, st),
        "hs-people" => sharing::people(&p, st),
        "hs-family" => sharing::family(&p, st),
        "hs-services" => services::services(&p, st),
        "hs-ai" => services::ai(&p, st),
        "hs-aidevices" => services::aidevices(&p, st),
        "hs-apps" => services::apps(&p, st),
        "hs-phone" => services::phone(&p, st),
        "hs-gaming" => services::gaming(&p, st),
        "hs-drives" => safe::drives(&p, st),
        "hs-backups" => safe::backups(&p, st),
        "hs-safety" => safe::safety(&p, st),
        _ => safe::alerts(&p, st),
    })
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    kit::subscription(st, &HOOKS)
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

/// Every category's Back is asked, whichever page is open, so this one keeps what's open on its pages (they're
/// closed with their own rows, as in Mantel) and Back goes up a page.
pub fn back(_st: &mut State) -> bool {
    false
}
