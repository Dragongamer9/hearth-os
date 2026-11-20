//! Gaming: every page of Mantel's Gaming category, for the Fedora edition (Steam Game Mode with Decky built in, Hearth
//! Game Mode as the choice it is on Debian, Handheld Daemon on handhelds, KDE Plasma's KWin). Each page is its own file
//! in cats/gaming/ and uses Mantel's own helpers and Hearth's core, so Debian's Mantel and these pages share every
//! setting. Hearth never signs in to Steam or uses Steam accounts. See cats/mod.rs for the shape.

use crate::shared::Shared;
use crate::App;
use iced::{Element, Subscription, Task};

pub mod boot;
mod controller;
mod discord;
mod family;
mod games;
mod handheld;
mod kwingames;
mod landing;
mod perf;
mod remote;
mod saves;
mod steam;

macro_rules! pages {
    ($(($id:literal, $f:ident, $v:ident, $m:ident)),* $(,)?) => {
        pub const PAGES: &[&str] = &[$($id),*];

        #[derive(Default)]
        pub struct State {
            $(pub $f: $m::St,)*
        }

        #[derive(Debug, Clone)]
        pub enum Msg {
            $($v($m::M),)*
        }

        pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
            match page {
                $($id => $m::open(sh, &mut st.$f).map(Msg::$v),)*
                _ => Task::none(),
            }
        }

        pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
            match m {
                $(Msg::$v(x) => $m::update(sh, &mut st.$f, x).map(Msg::$v),)*
            }
        }

        pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
            let st = &a.gaming;
            match page {
                $($id => Some($m::view(&a.sh, &st.$f).map(Msg::$v)),)*
                _ => None,
            }
        }
    };
}

pages! {
    ("gaming", landing, Landing, landing),
    ("savedgames", saves, Saves, saves),
    ("gameperf", perf, Perf, perf),
    ("steam", steam, Steam, steam),
    ("startup-animation", boot, Boot, boot),
    ("games", games, Games, games),
    ("discord", discord, Discord, discord),
    ("remote", remote, Remote, remote),
    ("family", family, Family, family),
    ("controller", controller, Controller, controller),
    ("kwin-games", kwingames, KwinGames, kwingames),
    ("handheld", handheld, Handheld, handheld),
}

/// The page of this category on screen (set while its dialog is asked for, which the frame does every time it draws),
/// so the two pages that read again on their own (Remote play while someone can connect, Handheld while HHD runs)
/// stop as soon as you leave them.
static SHOWN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

fn shown() -> &'static str {
    PAGES.get(SHOWN.load(std::sync::atomic::Ordering::Relaxed)).copied().unwrap_or("")
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    match shown() {
        "remote" if remote::rd_on(&st.remote) => iced::time::every(std::time::Duration::from_secs(5)).map(|_| Msg::Remote(remote::M::Tick)),
        "family" => match family::live(&st.family) {
            Some(secs) => iced::time::every(std::time::Duration::from_secs(secs)).map(|_| Msg::Family(family::M::Tick)),
            None => Subscription::none(),
        },
        "handheld" if handheld::live(&st.handheld) => iced::time::every(std::time::Duration::from_secs(6)).map(|_| Msg::Handheld(handheld::M::Tick)),
        // A startup animation's picture plays (its preview, or the card under the mouse or the focus).
        "startup-animation" => match boot::live(&st.boot) {
            Some(d) => iced::time::every(d).map(|_| Msg::Boot(boot::M::Tick)),
            None => Subscription::none(),
        },
        _ => Subscription::none(),
    }
}

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    SHOWN.store(PAGES.iter().position(|p| *p == a.page).unwrap_or(usize::MAX), std::sync::atomic::Ordering::Relaxed);
    if a.page == "startup-animation" {
        return boot::dialog(&a.sh, &a.gaming.boot).map(|(d, o)| (d.map(Msg::Boot), o.map(Msg::Boot)));
    }
    if a.page == "family" {
        return family::dialog(&a.sh, &a.gaming.family).map(|(d, o)| (d.map(Msg::Family), o.map(Msg::Family)));
    }
    None
}

/// B / Esc closes what's open inside a page first: the animation's preview, a button being remapped, a long list of
/// Handheld Daemon's choices.
pub fn back(st: &mut State) -> bool {
    match shown() {
        "startup-animation" => boot::back(&mut st.boot),
        "controller" => controller::back(&mut st.controller),
        "handheld" => handheld::back(&mut st.handheld),
        "family" => family::back(&mut st.family),
        _ => false,
    }
}
