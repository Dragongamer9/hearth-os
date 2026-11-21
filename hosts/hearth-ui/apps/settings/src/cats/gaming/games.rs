//! Gaming › Games & stores (Mantel's SetGames.qml): Hearth Games (GOG, Epic, Battle.net and Recomps), store sign-ins
//! (Epic, GOG and Battle.net in Hearth Games, Xbox through hearth-signin), where your games come from (Library sources
//! with the emulators, Downloads, Store, Import, a game from another Hearth) and Hearth's gaming apps. Apps that exist
//! open themselves; the rest opens in Hearth Game Mode at that page (nimbus-mode hearth-page, never a window). Steam signs in in Steam itself: Hearth
//! never signs in to Steam for you.

use crate::cats::apps::kit::{self, *};
use crate::cats::apps::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;

#[derive(Default)]
pub struct St {
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    App(Vec<&'static str>, &'static str),
    SignIn(&'static str, &'static str),
    Hearth(&'static str, &'static str),
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    Task::none()
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::App(cmd, name) => {
            kit::open(&cmd);
            st.say.ok(format!("Opening {name}…"));
        }
        M::SignIn(store, name) => {
            let s = shell("hearth-signin");
            kit::open(&["python3", &s, "open", store]);
            st.say.ok(format!("Opening {name}'s sign-in…"));
        }
        M::Hearth(page, said) => {
            kit::open_hearth_page(page);
            st.say.ok(said);
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let app = |cmd: Vec<&'static str>, name: &'static str| M::App(cmd, name);
    let mut v: Vec<E<M>> = Vec::new();
    opt(&mut v, say_view(&p, &st.say));
    v.push(head(&p, "Hearth Games"));
    v.push(r(G::Store, "Hearth Games", "Your GOG, Epic Games and Battle.net games: sign in, install, play and uninstall").chev().on(app(vec!["hearth-gamestore"], "Hearth Games")).view(&p));
    v.push(head(&p, "Store accounts"));
    v.push(r(G::Shield, "Epic Games", "Sign in to see, install and play your Epic games (in Hearth Games)").on(app(vec!["hearth-gamestore", "--store", "epic"], "Hearth Games")).view(&p));
    v.push(r(G::Shield, "GOG", "Sign in to see, install and play your GOG games (in Hearth Games)").on(app(vec!["hearth-gamestore", "--store", "gog"], "Hearth Games")).view(&p));
    v.push(r(G::Shield, "Battle.net", "Blizzard's own app, set up through Faugus Launcher (in Hearth Games)").on(app(vec!["hearth-gamestore", "--store", "battlenet"], "Hearth Games")).view(&p));
    v.push(r(G::Shield, "Xbox", "Sign in for Xbox Cloud Gaming and your Xbox PC games").on(M::SignIn("xbox", "Xbox")).view(&p));
    v.push(small(&p, "Steam signs in in Steam itself. Hearth never signs in to Steam for you."));
    v.push(head(&p, "Your games"));
    v.push(r(G::Grid, "Library sources", "Steam, Epic, GOG and emulators: which ones Hearth lists, add a game by hand, look for games again").chev().on(M::Hearth("sources", "Opening Library sources in Hearth Game Mode…")).view(&p));
    v.push(r(G::Update, "Downloads", "Epic and GOG installs: pause, carry on, cancel (in Hearth Games)").chev().on(app(vec!["hearth-gamestore"], "Hearth Games")).view(&p));
    v.push(r(G::Store, "Store", "Deals, new games and search for Steam, GOG, Epic and Xbox").chev().on(M::Hearth("shop", "Opening the Store in Hearth Game Mode…")).view(&p));
    v.push(r(G::Pad, "Recomps", "Native PC versions of classic console games: install, update and play them (in Hearth Games)").chev().on(app(vec!["hearth-gamestore", "--tab", "recomps"], "Recomps in Hearth Games")).view(&p));
    v.push(r(G::Folder, "Import", "Games and files from other drives and systems").chev().on(M::Hearth("import", "Opening Import in Hearth Game Mode…")).view(&p));
    v.push(r(G::Grid, "Get a game from another Hearth", "Copy an installed game from one of your Hearth devices (Settings › Hearth Link pairs them)").chev().on(M::Hearth("library", "Opening your library in Hearth Game Mode: Get from another Hearth…")).view(&p));
    v.push(head(&p, "Apps for your games"));
    v.push(r(G::Grid, "Mods", "Install mods and Nexus or Wabbajack modlists, turn them on and off, choose their order").on(app(vec!["hearth-mods"], "Mods")).view(&p));
    v.push(r(G::Folder, "Saves", "Back up and restore your games' saves, and move them to another PC").on(app(vec!["hearth-saves"], "Saves")).view(&p));
    v.push(r(G::Pad, "Controller Lab", "Test every button, stick and trigger; measure stick drift and calibrate").on(app(vec!["hearth-controller-lab"], "Controller Lab")).view(&p));
    v.push(r(G::Users, "Discord", "Friends, chats and invites in Discord; what you're playing on Discord").chev().on(M::Go("discord")).view(&p));
    v.push(r(G::Display, "Theater", "Videos from the web as tiles, full screen with a controller").on(app(vec!["hearth-theater"], "Theater")).view(&p));
    v.push(r(G::Gear, "Benchmark", "How fast this device is, with scores and history").on(app(vec!["hearth-tools", "--app", "benchmark"], "Benchmark")).view(&p));
    page(v)
}
