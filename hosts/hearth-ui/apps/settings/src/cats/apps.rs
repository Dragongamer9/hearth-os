//! Apps: every page of Mantel's Apps category, for the Fedora edition (Flatpak and Bazaar, the Debian box for .deb
//! files, drivers read with lspci and rpm, KDE Plasma on KWin). Each page is its own file in cats/apps/ and uses
//! Mantel's own helpers (hosts/linux/desktop/shell), so Debian's Mantel and these pages share every setting.
//! See cats/mod.rs for the shape; cats/apps/kit.rs runs the helpers.

use crate::shared::Shared;
use crate::App;
use iced::{Element, Subscription, Task};

pub mod kit;
mod autostart;
mod browser;
mod claude;
mod defaults;
mod filetypes;
mod installed;
mod menued;
mod perms;
mod rules;
mod services;
mod store;
mod winapps;

/// Every page here once: (its id, its field in State, its Msg variant, its file).
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
            let st = &a.apps;
            match page {
                $($id => Some($m::view(&a.sh, &st.$f).map(Msg::$v)),)*
                _ => None,
            }
        }
    };
}

pages! {
    ("apps", installed, Installed, installed),
    ("store", store, Store, store),
    ("winapps", winapps, WinApps, winapps),
    ("defaults", defaults, Defaults, defaults),
    ("file-types", filetypes, FileTypes, filetypes),
    ("autostart", autostart, Autostart, autostart),
    ("menu-editor", menued, MenuEditor, menued),
    ("app-permissions", perms, Perms, perms),
    ("services", services, Services, services),
    ("kwin-rules", rules, Rules, rules),
    ("browser", browser, Browser, browser),
    ("claude", claude, Claude, claude),
}

/// The page of this category on screen (set while its dialog is asked for, which the frame does every time it draws).
static SHOWN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

fn shown() -> &'static str {
    PAGES.get(SHOWN.load(std::sync::atomic::Ordering::Relaxed)).copied().unwrap_or("")
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    SHOWN.store(PAGES.iter().position(|p| *p == a.page).unwrap_or(usize::MAX), std::sync::atomic::Ordering::Relaxed);
    None
}

/// B / Esc inside the App store goes back from an app to the list it came from, and from a list to Discover.
pub fn back(st: &mut State) -> bool {
    match shown() {
        "store" => store::back(&mut st.store),
        _ => false,
    }
}
