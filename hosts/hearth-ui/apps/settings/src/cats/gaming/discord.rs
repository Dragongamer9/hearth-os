//! Gaming › Discord (Mantel's SetDiscord.qml, docs/DISCORD.md): which Discord the bar opens (the official app, Discord
//! in a web window, Dorion: the three choices of Mantel's chooser, here on the page as KDE Plasma has no Hearth bar),
//! opening it, "Playing <game>" on Discord with its one-step setup (the Discord application ID), Ask to join / Join for
//! friends, and the invite link for what you're playing. Everything changes only when you press it (hearth-discord,
//! which merges one key of ~/.config/nimbus/discord.json). Hearth never reads messages or signs in for you.

use crate::cats::apps::kit::{self, *};
use crate::cats::apps::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub choosing: bool,
    pub id: String,
    pub busy: Busy,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Choosing,
    Choose(&'static str),
    Install(&'static str),
    Ev(Ev),
    OpenFull,
    Id(String),
    SaveId,
    RemoveId,
    Set(&'static str, String),
    Copy,
}

fn hd(args: &[&str], secs: u32) -> Vec<String> {
    py("hearth-discord", args, secs)
}
fn name(c: &str) -> &'static str {
    match c {
        "official" => "Discord (the app)",
        "web" => "Discord in a web window",
        "dorion" => "Dorion (unofficial)",
        _ => "Discord",
    }
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    st.choosing = false;
    read(hd(&["status"], 30), M::Got)
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.st = r.v
            } else {
                st.say.bad(r.err)
            }
        }
        M::Choosing => st.choosing = !st.choosing,
        M::Choose(c) => {
            st.choosing = false;
            sh.reread_history = true;
            return read(hd(&["choose", c], 30), M::Got);
        }
        M::Install(c) => {
            st.busy.start();
            return job(hd(&["install", c], 1800), None, M::Ev);
        }
        M::Ev(e) => match e {
            Ev::Line(f, l) => st.busy.step(f, l),
            Ev::Done(r) => {
                st.busy.stop();
                if r.ok {
                    st.say.ok("Installed.");
                } else {
                    st.say.bad(r.err);
                }
                return read(hd(&["status"], 30), M::Got);
            }
        },
        M::OpenFull => {
            let s = shell("hearth-discord");
            kit::open(&["python3", &s, "open", "--full"]);
            st.say.ok("Opening Discord…");
        }
        M::Id(t) => st.id = t.chars().filter(|c| c.is_ascii_digit()).collect(),
        M::SaveId => {
            let id = st.id.clone();
            return update(sh, st, M::Set("appId", id));
        }
        M::RemoveId => {
            st.id.clear();
            return update(sh, st, M::Set("appId", String::new()));
        }
        M::Set(k, v) => {
            sh.reread_history = true;
            return read(hd(&["set", k, &v], 30), M::Got);
        }
        M::Copy => {
            let link = gs(go(&st.st, "invite"), "link");
            kit::open(&["wl-copy", &link]);
            st.say.ok("Invite link copied");
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let client = gs(s, "client");
    let clients = go(s, "clients");
    let mut v = crate::secs::SecVec::new();
    v.push(head(&p, "Your Discord"));
    let sub = if client.is_empty() {
        "Not picked yet: pick one below".to_string()
    } else {
        format!("{}{}. Change it", name(&client), if clients.is_object() && !gb(clients, &client) { " · not installed yet" } else { "" })
    };
    v.push(r(G::Users, "Which Discord", sub).chev().on(M::Choosing).view(&p));
    if st.choosing || client.is_empty() {
        let cards: [(&'static str, &str, &str, &str); 3] = [
            ("official", "Discord", "Recommended", "The official Discord app: everything Discord has. Installed from Flathub when you press Install."),
            ("web", "Discord in a web window", "Official · nothing to install", "Discord's own web app (discord.com) in a small Hearth Browser window. You sign in on Discord's page."),
            ("dorion", "Dorion", "Lighter · unofficial", "Uses less memory. It's unofficial: Discord's rules don't allow third-party apps, so there's a small risk to your account."),
        ];
        for (id, title, tag, body) in cards {
            let installed = id == "web" || gb(clients, id);
            let mut row = r(G::Users, title, body).chip(tag, if id == "dorion" { Chip::Warn } else { Chip::New }).indent();
            row = if client == id { row.value("In use") } else { row.value("Pick").on(M::Choose(id)) };
            v.push(row.view(&p));
            if client == id && !installed && !st.busy.on {
                v.push(r(G::Update, format!("Install {title}"), if id == "official" { "From Flathub" } else { "Its official GitHub release, checked against GitHub's SHA-256" }).indent().on(M::Install(id)).view(&p));
            }
        }
    }
    opt(&mut v, busy_view(&p, &st.busy, "Installing…"));
    v.push(r(G::Grid, "Open Discord", "Full size").chev().on(M::OpenFull).view(&p));
    v.push(small(&p, "Hearth only opens, places and hides Discord's window. It never reads your messages or friends, and never asks for your Discord password. Discord's notifications show as Hearth's."));
    v.push(head(&p, "What you're playing"));
    let app_id = gs(s, "appId");
    v.push(r(G::Shield, "Discord application ID", if app_id.is_empty() { "One step: make an application at discord.com/developers/applications (its name is what Discord shows, e.g. \"Hearth OS\"), copy its Application ID and paste it here".to_string() } else { format!("Set ({app_id}). Discord shows \"Playing\" with the game's name and picture") }).view(&p));
    let mut bs = vec![field(&p, "Paste the Application ID (numbers only)", &st.id, M::Id).on_submit(M::SaveId).width(iced::Length::Fill).into(), button(&p, "Save", true, Some(M::SaveId))];
    if !app_id.is_empty() {
        bs.push(button(&p, "Remove", false, Some(M::RemoveId)));
    }
    v.push(buttons(bs));
    let en = s.is_null() || gb(s, "enabled");
    v.push(sw(&p, G::Play, "Show what I'm playing on Discord", "Rich Presence: the game, its picture and how long you've played, while Discord is running", en, Some(M::Set("enabled", if en { "off" } else { "on" }.into()))));
    let ask = s.is_null() || gb(s, "askToJoin");
    v.push(sw(&p, G::Users, "Let friends ask to join", "Friends see Ask to join / Join on your game in Discord. Joining opens the game on their Hearth, or Play together when Remote play (Sunshine) is on", ask, Some(M::Set("askToJoin", if ask { "off" } else { "on" }.into()))));
    let inv = go(s, "invite");
    if gb(inv, "link") {
        let t = if gb(inv, "playTogether") && gb(inv, "pin") { format!("Invite link · Play together PIN {}", gs(inv, "pin")) } else { "Invite link".into() };
        v.push(r(G::Link, t, "Copy a link to paste in a Discord chat yourself. A Play together PIN in it works once, for 10 minutes").on(M::Copy).view(&p));
    }
    opt(&mut v, say_view(&p, &st.say));
    page(v)
}
