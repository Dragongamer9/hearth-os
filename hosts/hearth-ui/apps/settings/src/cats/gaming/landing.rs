//! Gaming (Mantel's SetGaming.qml with GameModeChoice.qml and StartInChoice.qml): Game Mode up top, which Game Mode
//! this PC goes into (Steam Game Mode or Hearth Game Mode: hearth-game-mode, session.conf's HEARTH_GAME_MODE_CHOICE
//! line), Start in (session.conf's NIMBUS_START_MODE through hearth_settings_io conf-set), Decky and the startup
//! animation, your games, controls, and Light mode while you play (the Home server's own key, hearth-home-server).
//! The pages Mantel reaches from elsewhere (Performance, Saved games, Discord, Full-screen games) are listed too.
//! Opening it only reads.

use crate::cats::apps::kit::{self, *};
use crate::cats::apps::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{self, t, tb, tm, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Row, Space};
use iced::{mouse, Color, Length, Padding, Task};
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub gm: Value,
    pub picked: String,
    pub msg: String,
    pub start: String,
    pub server_on: bool,
    pub steam: Value,
    pub boot: Value,
    pub hhd: Value,
    pub hs: Value,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Gm(Res),
    Pick(&'static str),
    Picked(String, Res),
    SteamGames(bool),
    Switch(&'static str),
    Start(&'static str),
    Started(Res),
    Server(Res),
    Steam(Result<Value, String>),
    Boot(Res),
    Hhd(Res),
    Hs(Res),
    Light(bool),
    LightDone(Res),
}

fn hgm(args: &[&str]) -> Vec<String> {
    py("hearth-game-mode", args, 30)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.picked.clear();
    st.msg.clear();
    st.start = crate::data::start_mode();
    Task::batch([
        read(hgm(&["get"]), M::Gm),
        read(py("hearth-server-mode", &["session-check"], 20), M::Server),
        core("steamMode.status", serde_json::json!({}), 30, M::Steam),
        read(py(&libexec("hearth-boot-look"), &["status"], 30), M::Boot),
        read(py("hearth-hhd", &["page"], 20), M::Hhd),
        read(py("hearth-home-server", &["status"], 30), M::Hs),
    ])
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::Gm(r) => {
            if r.ok {
                st.gm = r.v
            } else {
                st.msg = r.err
            }
        }
        M::Pick(v) => {
            if gs(&st.gm, "choice") == v && gb(&st.gm, "saved") {
                st.picked.clear();
                st.msg.clear();
                return Task::none();
            }
            sh.reread_history = true;
            return read(hgm(&["set", v]), move |r| M::Picked(v.to_string(), r));
        }
        M::Picked(v, r) => {
            if r.ok {
                st.gm = r.v;
                st.picked = v;
                st.msg.clear();
            } else {
                st.msg = r.err;
            }
        }
        M::SteamGames(on) => {
            sh.reread_history = true;
            return read(hgm(&["steam-games", if on { "on" } else { "off" }]), |r| M::Picked(String::new(), r));
        }
        M::Switch(mode) => kit::open(&["nimbus-mode", mode]),
        M::Start(mode) => {
            st.start = mode.to_string();
            let cmd = sh.conf_cmd(&crate::data::session_conf(), "NIMBUS_START_MODE", mode);
            return read(prog(cmd, 30), M::Started);
        }
        M::Started(r) => {
            if !r.ok {
                st.msg = format!("Couldn't save Start in: {}", r.err);
            }
        }
        M::Server(r) => st.server_on = r.out.trim() == "on",
        M::Steam(r) => st.steam = r.unwrap_or(Value::Null),
        M::Boot(r) => st.boot = r.v,
        M::Hhd(r) => st.hhd = if r.ok { r.v } else { serde_json::json!({"running": false}) },
        M::Hs(r) => st.hs = r.v,
        M::Light(on) => {
            sh.reread_history = true;
            return read(py("hearth-home-server", &["set", "lightMode", if on { "on" } else { "off" }], 60), M::LightDone);
        }
        M::LightDone(r) => {
            if !r.ok {
                st.msg = r.err;
            }
            return read(py("hearth-home-server", &["status"], 30), M::Hs);
        }
    }
    Task::none()
}

fn movie_label(boot: &Value) -> String {
    let m = go(boot, "movie");
    let cur = gs(m, "current");
    if m.is_null() {
        return String::new();
    }
    if cur == "steam" {
        return "Steam's own".into();
    }
    ga(m, "list").iter().find(|x| gs(x, "id") == cur).map(|x| gs(x, "label")).unwrap_or_default()
}

/// A small picture of each Game Mode (GameModeChoice.qml's): Steam's library under its bar, or Hearth's big tile.
fn picture<'a>(steam: bool) -> E<'a, M> {
    let bx = |w: f32, h: f32, c: &str, r: f32| -> E<'a, M> { container(Space::new()).width(Length::Fixed(w)).height(Length::Fixed(h)).style(ui::boxed(hearth_ui::theme::hex(c).unwrap_or(Color::BLACK), r, None)).into() };
    let inner: E<'a, M> = if steam {
        let covers = ["#3b6ea5", "#6b4fa0", "#2f8f72", "#a5793b", "#a54a3b", "#3b8aa5"];
        let mut cr = Row::new().spacing(6.0);
        for c in covers {
            cr = cr.push(bx(34.0, 50.0, c, 4.0));
        }
        column![row![bx(46.0, 10.0, "#1a9fff", 3.0), container(Space::new()).width(Length::Fill).height(Length::Fixed(10.0)).style(ui::boxed(hearth_ui::theme::hex("#23364c").unwrap_or(Color::BLACK), 3.0, None))], cr, bx(64.0, 14.0, "#1a9fff", 7.0)].spacing(10.0).into()
    } else {
        let tiles = ["#e08a3c", "#9b5a3a", "#5e7f9e", "#7d9e5e", "#b0624f", "#6a5e9e"];
        let mut tr = Row::new().spacing(6.0);
        for c in tiles {
            tr = tr.push(bx(30.0, 32.0, c, 4.0));
        }
        let big = container(container(bx(40.0, 10.0, "#ffd9a8", 5.0)).padding(Padding { top: 34.0, right: 0.0, bottom: 0.0, left: 8.0 })).width(Length::Fixed(150.0)).height(Length::Fixed(52.0)).style(ui::boxed(hearth_ui::theme::hex("#a44a24").unwrap_or(Color::BLACK), 6.0, None));
        column![row![big, column![bx(70.0, 12.0, "#4a2c1e", 4.0), bx(70.0, 12.0, "#4a2c1e", 4.0), bx(70.0, 12.0, "#4a2c1e", 4.0)].spacing(6.0)].spacing(14.0), tr].spacing(8.0).into()
    };
    container(inner).padding(10.0).width(Length::Fill).height(Length::Fixed(112.0)).style(ui::boxed(hearth_ui::theme::hex(if steam { "#14202e" } else { "#2a1810" }).unwrap_or(Color::BLACK), 8.0, None)).into()
}

fn gm_card<'a>(p: &Pal, st: &St, v: &'static str) -> E<'a, M> {
    let steam = v == "steam";
    let choice = if st.gm.is_null() { "steam".to_string() } else { gs(&st.gm, "choice") };
    let chosen = choice == v;
    let part = go(&st.gm, v);
    let usable = st.gm.is_null() || gb(part, "ok");
    let (title, desc) = if steam {
        ("Steam Game Mode", "Steam's own big-screen mode, like a Steam Deck, with Hearth Sync for your other stores' games.")
    } else {
        ("Hearth Game Mode", "Hearth's own screens: every store in one library, Hearth's Store, Quick Access and Settings. Steam games still play through Steam.")
    };
    let mut head = Row::new().spacing(8.0).align_y(iced::Center).push(hearth_ui::ui::glyph(G::Pad, p.text, 20.0, 1.8)).push(tb(title, sz(16.0), p.text));
    if chosen {
        head = head.push(t(if steam && !gb(&st.gm, "saved") { "· In use (default)" } else { "· In use" }, sz(14.0), p.accent));
    }
    let line = if usable { t(desc, sz(13.0), p.muted) } else { t(format!("Can't start here: {}.", gs(part, "why")), sz(13.0), p.danger) };
    let c = container(column![picture(steam), head, line].spacing(10.0))
        .padding(10.0)
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(12.0), Some((if chosen { p.accent } else { Pal::a(p.text, 0.08) }, if chosen { 2.0 } else { 1.0 }))));
    mouse_area(c).on_press(M::Pick(v)).interaction(mouse::Interaction::Pointer).into()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let hearth_gm = gs(&st.gm, "choice") == "hearth";
    let mut v = crate::secs::SecVec::new();
    // ---- the Game Mode card
    let hero = container(
        row![
            container(hearth_ui::ui::glyph(G::Pad, p.accent, 30.0, 1.8)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(Pal::a(p.accent, 0.18), p.r(12.0), None)),
            column![
                tm(if hearth_gm { "Hearth Game Mode" } else { "Steam Game Mode" }, sz(19.0), p.text),
                t(if hearth_gm { "Hearth's own big-screen mode: every store in one library. Press the Game Mode button any time." } else { "Steam's big-screen mode with the Hearth Sync plugin. Press the Game Mode button any time." }, sz(13.0), p.muted)
            ]
            .spacing(3.0)
            .width(Length::Fill),
            button(&p, "Switch to Game Mode", true, Some(M::Switch("game")))
        ]
        .spacing(18.0)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 22.0, right: 20.0, bottom: 22.0, left: 20.0 })
    .width(Length::Fill)
    .style(ui::boxed(Pal::mix(p.panel, p.accent, 0.14), p.r(14.0), None));
    v.push(mouse_area(hero).on_press(M::Switch("game")).interaction(mouse::Interaction::Pointer).into());

    v.push(head(&p, "Game Mode"));
    v.push(row![gm_card(&p, st, "steam"), gm_card(&p, st, "hearth")].spacing(12.0).into());
    let now = gs(&st.gm, "now");
    if !st.picked.is_empty() && !st.gm.is_null() && now != "game" && now != "steam" {
        let h = st.picked == "hearth";
        v.push(r(G::Pad, format!("Switch to {} now", if h { "Hearth Game Mode" } else { "Steam Game Mode" }), "Saved. Game Mode opens this way from now on, the next time it starts. Switching now closes Desktop Mode's windows.").chev().on(M::Switch(if h { "hearth" } else { "game" })).view(&p));
    }
    if hearth_gm {
        let on = not_false(&st.gm, "steamGames");
        v.push(sw(&p, G::Pad, "Play Steam games in Steam Game Mode", "In Hearth Game Mode, Play on a Steam game opens it in Steam Game Mode, then comes back when the game ends", on, Some(M::SteamGames(!on))));
        let (fb, paused) = (gs(&st.gm, "fallback"), gs(&st.gm, "paused"));
        if !fb.is_empty() || !paused.is_empty() {
            let sub = if !paused.is_empty() {
                format!("It failed twice ({paused}), so Game Mode opens Steam Game Mode until you try again. Your choice is still Hearth Game Mode. Press to try Hearth Game Mode again.")
            } else {
                format!("Why: {fb}. Steam Game Mode (or Desktop Mode) opened instead. Your choice is still Hearth Game Mode. Press to try Hearth Game Mode again.")
            };
            v.push(r(G::Info, if paused.is_empty() { "Hearth Game Mode didn't start last time" } else { "Hearth Game Mode is paused" }, sub).danger().on(M::Switch("hearth")).view(&p));
        }
    }
    if !st.msg.is_empty() {
        v.push(words(&p, st.msg.clone(), true));
    }
    let mut starts = vec![("Game Mode".to_string(), st.start == "game", Some(M::Start("game"))), ("Desktop Mode".to_string(), st.start == "desktop", Some(M::Start("desktop")))];
    if st.server_on || st.start == "server" {
        starts.push(("Server mode".into(), st.start == "server", Some(M::Start("server"))));
    }
    v.push(choice(&p, G::Power, "Start in", "What this PC opens when it turns on or you sign in", starts));
    let decky = go(&st.steam, "decky");
    let r1 = r(G::Pad, "Decky Loader and Hearth Sync", "Plugins in Game Mode's Quick Access menu").chev().on(M::Go("steam"));
    let r1 = if decky.is_null() { r1 } else if gb(decky, "installed") { r1.chip("Installed", Chip::Ok) } else { r1.chip("Not installed", Chip::Off) };
    v.push(r1.view(&p));
    v.push(r(G::Play, "Startup animation", "What Steam Game Mode plays while it starts").value(movie_label(&st.boot)).chev().on(M::Go("startup-animation")).view(&p));

    v.push(head(&p, "Your games"));
    v.push(link_row(&p, G::Store, "Games & stores", "Epic Games, GOG and Xbox accounts, library sources, mods and saves", M::Go("games")));
    v.push(link_row(&p, G::Play, "Remote play", "Stream this PC's games to your phone or TV, play another PC here, or use this PC from anywhere", M::Go("remote")));
    v.push(link_row(&p, G::Users, "Family and friends", "Let family and friends connect to this PC with Moonlight, and connect to theirs", M::Go("family")));
    v.push(link_row(&p, G::Drive, "Game storage", "Games by size, moving games between drives, low space warning · System › Storage", M::Go("storage")));
    v.push(link_row(&p, G::Update, "Saved games", "Every game's saves, backed up and put back (Ludusavi)", M::Go("savedgames")));

    v.push(head(&p, "Controls & performance"));
    v.push(link_row(&p, G::Pad, "Controller", "Controller mouse, button layout, what games see, remapping", M::Go("controller")));
    let no_hhd = st.hhd.get("running").and_then(Value::as_bool) == Some(false);
    v.push(r(G::Power, "Handheld", "Power limit, fans, lights and back buttons (Handheld Daemon)").value(if no_hhd { "No handheld on this PC" } else { "" }).chev().on(M::Go("handheld")).view(&p));
    v.push(link_row(&p, G::Rocket, "Performance", "Game Mode's resolution, refresh rate, frame limit and HDR", M::Go("gameperf")));
    // a KWin page: off KWin it could never work (the owner, Nov 28)
    if crate::shared::kwin_desktop() {
    v.push(link_row(&p, G::Display, "Full-screen games & X11 apps", "Tearing in full-screen games, keys X11 apps can see, scaling X11 apps", M::Go("kwin-games")));
    }
    let cfg = go(&st.hs, "settings");
    if !cfg.is_null() {
        let on = not_false(cfg, "lightMode");
        v.push(sw(&p, G::Server, "Light mode while you play", "Home server jobs wait while a game runs. Sharing keeps working.", on, Some(M::Light(!on))));
    }
    v.push(head(&p, "Friends"));
    v.push(link_row(&p, G::Users, "Discord", "Which Discord the bar opens, what you're playing on Discord, friends joining", M::Go("discord")));
    page(v)
}
