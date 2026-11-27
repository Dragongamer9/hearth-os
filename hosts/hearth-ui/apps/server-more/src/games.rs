//! Game servers (board docs/mockups/server-expansion/05-game-servers.png; checklist server-game-servers.md): one
//! click makes a server on this PC (hearth-server-apps' rootless Podman; SteamCMD's anonymous login inside the
//! dedicated servers' images, never a Steam account), friends join over Tailscale, nothing open to the internet (the
//! Home server helper's firewall rule). Players online (Steam's server query, Minecraft's list ping), mods from
//! Thunderstore, invites, world backups, start and stop on a schedule, your own games' headless builds.

use crate::kit::{self, pad, Tag, E};
use crate::{act, at, bars, boxed, cols, colour, head, line, list, num, on, panel, s, sv, sw, two, Hub, Msg, Part, ROWLINE};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::describe::Describe;
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, text_input, Column, Row, Space};
use iced::{Color, Length};
use serde_json::Value;

const P: Part = Part::Games;

pub const PAGE: PageDef = PageDef {
    id: "srv-games",
    title: "Game servers",
    blocks: &[
        bl("head", "Game servers and New server", 0, 0, 12, 1),
        bl("cards", "The servers", 0, 1, 12, 3),
        bl("detail", "The open server: players and mods", 0, 4, 6, 5),
        bl("invite", "Invite friends", 6, 4, 6, 1),
        bl("backups", "World backups", 6, 5, 6, 2),
        bl("schedule", "Start and stop on a schedule", 6, 7, 6, 2),
        bl("week", "Players this week", 0, 9, 6, 3),
        bl("own", "Your games' servers", 6, 9, 6, 3),
    ],
};
pub const PANEL: PageDef = PageDef {
    id: "srv-games-side",
    title: "Game servers' right side",
    blocks: &[bl("new", "New server", 0, 0, 12, 5), bl("helper", "The game server helper", 0, 5, 12, 2), bl("fair", "Using the PC fairly", 0, 7, 12, 3), bl("use", "Using the servers", 0, 10, 12, 2)],
};

fn card<'a>(h: &'a Hub, p: Pal, c: &'a Value, sel: bool) -> E<'a, Msg> {
    let id = s(c, "id");
    let arts = list(c, "art");
    let a = colour(arts.first().unwrap_or(&Value::Null), p.panel2);
    let b = colour(arts.get(1).unwrap_or(&Value::Null), p.bg);
    let running = s(c, "state") == "running";
    let tag = if running { kit::tag(&p, s(c, "tag"), Tag::Ok) } else { kit::tag(&p, s(c, "tag"), Tag::Dark) };
    let top = container(column![row![tag], Space::new().height(Length::Fill), tb(s(c, "name"), 17.0, Color::WHITE)].spacing(4.0))
        .padding(pad(10.0, 14.0, 10.0, 14.0))
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(kit::gradient(135.0, a, b, 0.0, None));
    let ln = list(c, "line");
    let mut buttons = Row::new().spacing(6.0);
    for bt in list(c, "buttons") {
        let label = sv(at(bt, "/0"));
        let what = sv(at(bt, "/1"));
        let m = match what.as_str() {
            "console" => act(P, &["game-console", &id], ""),
            "build" => act(P, &["game-build", &id], "The newest build is in place."),
            "make" => act(P, &["game-make", &id], "Your game's server build is made by its own build:"),
            "back" => act(P, &["game-back", &id], "Back on this PC: its world came home first."),
            w => act(P, &["game", w, &id], match w {
                "stop" => "Stopped. Its world was backed up first.",
                "start" => "Starting: friends can join in a minute.",
                _ => "Restarting.",
            }),
        };
        let m = if h.busy { None } else { Some(m) };
        buttons = buttons.push(if what == "start" { kit::chip_acc(&p, label, m) } else { kit::chip(&p, label, m) });
    }
    let inner = column![
        row![t(sv(ln.first().unwrap_or(&Value::Null)), 13.0, p.text), Space::new().width(Length::Fill), t(sv(ln.get(1).unwrap_or(&Value::Null)), 12.0, p.muted)].align_y(iced::Center),
        t(s(c, "sub"), 12.0, p.muted),
        Space::new().height(Length::Fill),
        buttons
    ]
    .spacing(4.0);
    let body = column![top, container(inner).padding(pad(12.0, 14.0, 12.0, 14.0)).height(Length::Fixed(112.0))];
    let line_c = if sel { p.accent } else { p.line };
    let bg = if sel { kit::rgb(0x2e, 0x22, 0x18) } else { p.panel };
    kit::click(container(body).clip(true).width(Length::Fill).style(ui::boxed(bg, 12.0, Some((line_c, if sel { 2.0 } else { 1.0 })))), Some(Msg::Set(P, "sel", format!("\"{id}\""))))
}

fn avatar<'a>(name: &str) -> E<'a, Msg> {
    let c = match name.chars().next().unwrap_or('A').to_ascii_uppercase() {
        'D' => kit::rgb(0xff, 0x8a, 0x3d),
        'S' => kit::rgb(0x3b, 0xa5, 0x5c),
        'J' => kit::rgb(0x2c, 0x7f, 0xd8),
        _ => kit::rgb(0x8a, 0x5a, 0xc8),
    };
    container(tb(name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(), 12.0, Color::WHITE))
        .center_x(Length::Fixed(28.0))
        .center_y(Length::Fixed(28.0))
        .style(ui::boxed(c, 14.0, None))
        .into()
}

fn detail<'a>(h: &'a Hub, p: Pal, d: &'a Value) -> E<'a, Msg> {
    let id = s(d, "id");
    let arts = list(d, "art");
    let mark = container(tb(s(d, "mark"), 15.0, Color::WHITE))
        .center_x(Length::Fixed(44.0))
        .center_y(Length::Fixed(44.0))
        .style(kit::gradient(135.0, colour(arts.first().unwrap_or(&Value::Null), p.panel2), colour(arts.get(1).unwrap_or(&Value::Null), p.bg), 10.0, None));
    let tabs = ["Players", "Mods", "Worlds", "Settings"];
    let seg = kit::seg(&p, tabs.iter().map(|x| (x.to_string(), h.gtab == *x, Some(Msg::GTab(*x)))).collect());
    let top = row![mark, column![ts(s(d, "title"), 18.0, p.text), t(s(d, "sub"), 12.0, p.muted)].spacing(2.0).width(Length::Fill), seg].spacing(10.0).align_y(iced::Center);
    let mut c = Column::new().spacing(6.0).push(top);
    match h.gtab {
        "Mods" => c = c.push(mods(h, p, d, true)),
        "Worlds" => c = c.push(worlds_tab(h, p)),
        "Settings" => c = c.push(settings_tab(h, p, d)),
        "Console" => {
            let mut lines = Column::new().spacing(2.0);
            for l in h.console.iter().rev().take(18).rev() {
                lines = lines.push(t(l.clone(), 11.0, p.muted).font(iced::Font::MONOSPACE));
            }
            c = c.push(container(kit::caps(&p, "Console · its last lines")).padding(pad(10.0, 0.0, 4.0, 0.0))).push(boxed(p, lines));
        }
        _ => {
            let pl = list(d, "players");
            c = c.push(container(kit::caps(&p, format!("Playing now · {}", pl.len()))).padding(pad(10.0, 0.0, 4.0, 0.0)));
            for x in pl {
                let name = s(x, "name");
                let right: E<'a, Msg> = if on(x, "admin") {
                    kit::tag(&p, "Admin", Tag::Dark)
                } else if on(x, "kick") {
                    kit::click(t("Kick", 12.0, p.accent), Some(act(P, &["game-kick", &id, &name], "Kicked.")))
                } else {
                    Space::new().into()
                };
                let r = row![avatar(&name), row![t(name.clone(), 13.0, p.text), t(format!(" · {}", s(x, "where")), 13.0, p.muted)].width(Length::Fill), t(s(x, "time"), 13.0, p.muted), right]
                    .spacing(10.0)
                    .align_y(iced::Center);
                c = c.push(container(r).padding(pad(7.0, 0.0, 7.0, 0.0))).push(ui::hline(ROWLINE));
            }
            if pl.is_empty() {
                c = c.push(t(if on(d, "running") { "Nobody's on right now." } else { "It isn't running." }, 13.0, p.muted));
            }
            if !at(d, "/mods").is_null() {
                c = c.push(mods(h, p, d, false));
            }
        }
    }
    kit::card(&p, c).into()
}

fn mods<'a>(h: &'a Hub, p: Pal, d: &'a Value, add: bool) -> E<'a, Msg> {
    let m = at(d, "/mods");
    if m.is_null() {
        return t("This game has no mods from Thunderstore.", 13.0, p.muted).into();
    }
    let id = s(d, "id");
    let mut c = Column::new().spacing(8.0).push(container(kit::caps(&p, s(m, "caption"))).padding(pad(10.0, 0.0, 2.0, 0.0)));
    let items: Vec<&Value> = list(m, "list").iter().collect();
    for pair in items.chunks(2) {
        let mut r = Vec::new();
        for x in pair {
            let mid = s(x, "id");
            let now = on(x, "on");
            r.push(
                boxed(p, row![t(s(x, "name"), 13.0, p.text).width(Length::Fill), kit::switch(&p, now, Some(act(P, &["mod-set", &id, &mid, if now { "off" } else { "on" }], if now { "That mod is off." } else { "That mod is on." })))].align_y(iced::Center))
                    .into(),
            );
        }
        if r.len() == 1 {
            r.push(Space::new().into());
        }
        c = c.push(cols(r, 8.0));
    }
    if add {
        let full = h.typed.trim().to_string();
        c = c.push(
            row![
                text_input("A Thunderstore mod, as Namespace-Name", &h.typed).on_input(Msg::Typed).size(13.0).padding(pad(7.0, 10.0, 7.0, 10.0)).style(kit::field_style(&p)),
                kit::chip_acc(&p, "Add", if full.is_empty() { None } else { Some(act(P, &["mod-add", &id, &full], "Added and checked on Thunderstore.")) })
            ]
            .spacing(8.0)
            .align_y(iced::Center),
        );
    }
    c = c.push(t(s(m, "note"), 12.0, p.muted));
    // Friends without Hearth: the same mods through a Thunderstore profile code (r2modman imports it).
    let code = s(m, "code");
    if !items.is_empty() {
        let r: E<'a, Msg> = if code.is_empty() {
            row![t("Friends without Hearth: a profile code for r2modman", 12.0, p.muted).width(Length::Fill), kit::click(t("Make one", 12.0, p.accent), Some(act(P, &["mods-code", &id], "Made: friends import it in r2modman.")))].align_y(iced::Center).into()
        } else {
            row![t(format!("Profile code for r2modman · {code}"), 12.0, p.muted).width(Length::Fill), kit::click(t("Copy", 12.0, p.accent), Some(Msg::Copy(code.clone())))].align_y(iced::Center).into()
        };
        c = c.push(r);
    }
    c.into()
}

fn worlds_tab<'a>(h: &'a Hub, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let id = s(at(v, "/detail"), "id");
    let mut c = Column::new().spacing(4.0).push(container(kit::caps(&p, "Its worlds")).padding(pad(10.0, 0.0, 4.0, 0.0)));
    for r in list(at(v, "/backups"), "rows") {
        let f = s(r, "file");
        c = c.push(line(t(s(r, "label"), 13.0, p.text).into(), row![t(s(r, "size"), 12.0, p.muted), kit::click(t("Restore", 12.0, p.accent), Some(act(P, &["world-restore", &id, &f], "Rolled back: today's world was kept as a backup first.")))].spacing(14.0).into()));
    }
    c = c.push(row![kit::chip(&p, "Back up now", Some(act(P, &["world-backup", &id], "Backed up.")))]);
    c.into()
}

fn settings_tab<'a>(h: &'a Hub, p: Pal, d: &'a Value) -> E<'a, Msg> {
    let id = s(d, "id");
    let mut c = Column::new().spacing(8.0).push(container(kit::caps(&p, "Its choices")).padding(pad(10.0, 0.0, 4.0, 0.0)));
    for o in list(d, "opts") {
        let key = s(o, "key");
        let now = s(o, "value");
        let segs: Vec<(String, bool, Option<Msg>)> = list(o, "choices")
            .iter()
            .map(|ch| {
                let v = sv(ch);
                let m = if h.busy || v == now { None } else { Some(act(P, &["game-opt", &id, &key, &v], "Kept: it restarts with it.")) };
                (v.clone(), v == now, m)
            })
            .collect();
        c = c.push(line(t(s(o, "label"), 13.0, p.text).into(), kit::seg(&p, segs)));
    }
    c.into()
}

fn invite<'a>(h: &'a Hub, p: Pal, v: &'a Value, sel: &str) -> E<'a, Msg> {
    let i = at(v, "/invite");
    let who = h.typed.trim().to_string();
    // Whom it's for (kept: Shared with), when you say; the link itself is Tailscale's share page.
    let make: E<'a, Msg> = if h.pick == "invite" {
        row![
            text_input("Who is it for?", &h.typed).on_input(Msg::Typed).size(12.0).width(Length::Fixed(130.0)).padding(pad(5.0, 8.0, 5.0, 8.0)).style(kit::field_style(&p)),
            kit::chip_acc(&p, "Make an invite link", Some(act(P, if who.is_empty() { vec!["invite", sel] } else { vec!["invite", sel, who.as_str()] }.as_slice(), "Share it from Tailscale's page:")))
        ]
        .spacing(6.0)
        .align_y(iced::Center)
        .into()
    } else {
        kit::chip_acc(&p, "Make an invite link", Some(Msg::Pick("invite")))
    };
    let mut joins = Column::new().spacing(2.0);
    for j in list(v, "join") {
        let (peer, gid) = (s(j, "peer"), s(j, "id"));
        let mods = at(j, "/mods").as_u64().unwrap_or(0);
        joins = joins.push(line(
            t(format!("On {}: {}{}", s(j, "pc"), s(j, "name"), if mods > 0 { format!(" · its {mods} mods first") } else { String::new() }), 12.0, p.text).into(),
            kit::click(t("Join from here", 12.0, p.accent), Some(act(P, &["game-join", &peer, &gid], "Joining: its mods are in, the game is starting."))),
        ));
    }
    kit::card(
        &p,
        column![
            kit::caps(&p, "Invite friends · over Tailscale"),
            line(two(p, "Share this server with a friend's Tailscale".into(), "they only see this one game, nothing else of yours".into()), make),
            container(row![t(s(i, "address"), 13.0, p.text).width(Length::Fill), kit::click(t("Copy", 12.0, p.accent), Some(Msg::Copy(s(i, "address").split(" · ").next().unwrap_or("").to_string())))].align_y(iced::Center))
                .padding(pad(7.0, 10.0, 7.0, 10.0))
                .style(ui::boxed(p.bg2, 8.0, Some((p.line, 1.0)))),
            line(t("Shared with", 12.0, p.muted).into(), t(s(i, "shared"), 13.0, p.text).into()),
            joins,
        ]
        .spacing(6.0),
    )
    .into()
}

fn backups<'a>(p: Pal, v: &'a Value, sel: &str) -> E<'a, Msg> {
    let b = at(v, "/backups");
    let mut c = Column::new().spacing(2.0).push(row![kit::caps(&p, "World backups"), Space::new().width(Length::Fill), t(s(b, "when"), 12.0, p.muted)].align_y(iced::Center));
    for r in list(b, "rows") {
        let f = s(r, "file");
        c = c.push(
            row![t(s(r, "label"), 13.0, p.text).width(Length::FillPortion(5)), t(s(r, "size"), 12.0, p.muted).width(Length::FillPortion(2)), kit::click(t("Restore", 12.0, p.accent), Some(act(P, &["world-restore", sel, &f], "Rolled back: today's world was kept as a backup first.")))]
                .align_y(iced::Center)
                .padding(pad(3.0, 0.0, 3.0, 0.0)),
        );
    }
    kit::card(&p, c).into()
}

fn schedule<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let sc = at(v, "/schedule");
    const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let cell = |s_: String, on_: bool, m: Option<Msg>| -> E<'a, Msg> {
        let c = container(t(s_, 11.0, if on_ { p.accent } else { p.muted })).padding(pad(3.0, 6.0, 3.0, 6.0)).width(Length::Fill).height(Length::Fixed(22.0));
        let c = if on_ { c.style(ui::boxed(kit::rgb(0x3b, 0x2a, 0x1d), 5.0, Some((kit::rgb(0x5a, 0x3b, 0x22), 1.0)))) } else { c };
        kit::click(c, m)
    };
    let mut hdr = Row::new().spacing(4.0).push(container(Space::new()).width(Length::Fixed(70.0)));
    for d in DAYS {
        hdr = hdr.push(cell(d.into(), false, None));
    }
    let mut c = Column::new().spacing(4.0).push(row![kit::caps(&p, "Start and stop on a schedule"), Space::new().width(Length::Fill), t(s(sc, "note"), 12.0, p.muted)].align_y(iced::Center)).push(hdr);
    for r in list(sc, "rows") {
        let gid = s(r, "id");
        let mut rr = Row::new().spacing(4.0).push(container(t(s(r, "name"), 11.0, p.muted)).width(Length::Fixed(70.0)).padding(pad(3.0, 0.0, 0.0, 0.0)));
        for (i, d) in list(r, "days").iter().enumerate() {
            let w = sv(d);
            let day = DAYS[i.min(6)];
            // a press: a filled day off, an empty one the evening (6 PM to 11 PM)
            let m = if w.is_empty() { act(P, &["schedule", &gid, day, "18:00", "23:00"], "Kept: it starts and stops by itself then.") } else { act(P, &["schedule", &gid, day, "off"], "That day is off.") };
            rr = rr.push(cell(w.clone(), !w.is_empty(), Some(m)));
        }
        c = c.push(rr);
    }
    kit::card(&p, c).into()
}

fn week<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let w = at(v, "/week");
    let vals: Vec<(f32, f32)> = list(w, "bars").iter().map(|x| (num(x), 0.0)).collect();
    let mut labels = Row::new();
    for (i, d) in ["Fri", "Sat", "Sun", "Mon", "Tue", "Wed", "Thu"].iter().enumerate() {
        if i > 0 {
            labels = labels.push(Space::new().width(Length::Fill));
        }
        labels = labels.push(t(*d, 11.0, p.muted));
    }
    kit::card(&p, column![row![kit::caps(&p, "Players this week"), Space::new().width(Length::Fill), t(s(w, "most"), 12.0, p.muted)].align_y(iced::Center), bars(vals, p.accent, p.accent, 70.0), labels].spacing(6.0))
        .padding(pad(12.0, 16.0, 12.0, 16.0))
        .into()
}

fn own<'a>(p: Pal, v: &'a Value) -> E<'a, Msg> {
    let mut c = Column::new().spacing(2.0).push(kit::caps(&p, "Your games' servers"));
    for r in list(v, "own") {
        let gid = s(r, "id");
        let right: E<'a, Msg> = if on(r, "link") { kit::click(t(s(r, "right"), 12.0, p.accent), Some(act(P, &["game-make", &gid], "Your game's server build is made by its own build:"))) } else { t(s(r, "right"), 12.0, p.muted).into() };
        c = c.push(line(t(s(r, "label"), 13.0, p.text).into(), right));
    }
    kit::card(&p, c).padding(pad(12.0, 16.0, 12.0, 16.0)).into()
}

fn side<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let mut cat = Column::new().spacing(6.0);
    for x in list(v, "catalog") {
        let gid = sv(at(x, "/0"));
        let label = sv(at(x, "/1"));
        // Minecraft: Java Edition or Bedrock (Mojang's own Bedrock Dedicated Server), picked under its row.
        let two_kinds = !at(x, "/2").is_null();
        let m = if gid == "more" { Msg::GTab("more") } else if two_kinds { Msg::GTab("mc") } else { act(P, &["game", "install", &gid], "Getting it ready: it's off until you start it.") };
        let c = container(row![t(label, 13.0, p.text), Space::new().width(Length::Fill), t("›", 13.0, p.muted)].align_y(iced::Center)).padding(pad(8.0, 12.0, 8.0, 12.0)).width(Length::Fill).style(ui::boxed(p.panel2, 8.0, Some((p.line, 1.0))));
        cat = cat.push(kit::click(c, Some(m)));
        if two_kinds && h.gtab == "mc" {
            let other = sv(at(x, "/2"));
            cat = cat.push(
                container(row![
                    kit::chip(&p, "Java Edition", Some(act(P, &["game", "install", &gid], "Getting it ready: it's off until you start it."))),
                    kit::chip(&p, "Bedrock Edition", Some(act(P, &["game", "install", &other], "Getting it ready: it's off until you start it.")))
                ]
                .spacing(6.0))
                .padding(pad(0.0, 0.0, 0.0, 16.0)),
            );
        }
        if gid == "more" && h.gtab == "more" {
            for (g, n) in [("terraria", "Terraria"), ("sevendays", "7 Days to Die"), ("ark", "Ark")] {
                cat = cat.push(container(kit::chip(&p, n, Some(act(P, &["game", "install", g], "Getting it ready: it's off until you start it.")))).padding(pad(0.0, 0.0, 0.0, 16.0)));
            }
        }
    }
    let new = column![ts("New server", 18.0, p.text), t("Each game is its own Store app · updates in the Hearth Store", 12.0, p.muted), container(cat).padding(pad(6.0, 0.0, 0.0, 0.0))].spacing(4.0);
    let installed = on(at(v, "/helper"), "installed");
    let helper: E<'a, Msg> = boxed(
        p,
        column![
            row![tb("First server only: ", 12.0, p.text), t("Install the game server helper", 12.0, p.text)],
            t("It adds the system parts it needs (a service and a firewall rule for Tailscale only).", 12.0, p.muted).line_height(1.5),
            container(if installed { kit::tag(&p, "Installed", Tag::Ok) } else { kit::chip_acc(&p, "Install it · asks for your password once", Some(act(P, &["games-setup"], "The game server helper is in."))) }).padding(pad(6.0, 0.0, 0.0, 0.0))
        ]
        .spacing(4.0),
    )
    .into();
    let f = at(v, "/fair");
    let mem_next = match s(f, "mem").as_str() {
        "8 GB" => "12",
        "12 GB" => "16",
        "16 GB" => "24",
        _ => "8",
    };
    let fair = column![
        kit::caps(&p, "Using the PC fairly"),
        line(t("Pause servers' CPU while you play here", 13.0, p.text).into(), sw(p, P, "pauseWhilePlaying", on(f, "pause"))),
        line(t("Most memory for all servers", 13.0, p.text).into(), kit::click(container(t(s(f, "mem"), 13.0, p.text)).padding(pad(6.0, 10.0, 6.0, 10.0)).style(ui::boxed(p.bg2, 8.0, Some((p.line, 1.0)))), Some(Msg::Set(P, "maxMemGB", mem_next.into())))),
        line(t("Run on another PC if this one's busy", 13.0, p.text).into(), sw(p, P, "otherPc", on(f, "other"))),
        t("See PCs to let Dad's PC host one", 12.0, p.muted),
    ]
    .spacing(2.0);
    let u = at(v, "/use");
    let using = column![
        kit::caps(&p, "Using the servers"),
        line(t("CPU · all servers", 13.0, p.text).into(), t(s(u, "cpu"), 12.0, p.muted).into()),
        line(t("Memory · all servers", 13.0, p.text).into(), t(s(u, "mem"), 12.0, p.muted).into()),
        line(t("Worlds and saves", 13.0, p.text).into(), t(s(u, "worlds"), 12.0, p.muted).into()),
    ]
    .spacing(2.0);
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![("new", new.into()), ("helper", helper), ("fair", fair.into()), ("use", using.into())];
    panel(p, hearth_ui::arrange::parts(d, &p, "srv-games-side", items, |ps| column![ps.take("new"), ps.take("helper"), ps.take("fair"), ps.take("use")].spacing(16.0).into(), Msg::D))
}

pub fn view<'a>(h: &'a Hub, d: &'a Describe, p: Pal) -> E<'a, Msg> {
    let v = h.view_of(P);
    let sel = s(v, "sel");
    let hd = head(h, p, "Game servers", "One click makes a server on the Nvidia PC · friends join over Tailscale, nothing open to the internet".into(), vec![
        kit::chip(&p, "Invite friends", Some(act(P, &["invite", &sel], "Share it from Tailscale's page:"))),
        kit::chip_acc(&p, "＋ New server", Some(Msg::GTab("more"))),
    ]);
    let cards: Vec<E<'a, Msg>> = list(v, "cards").iter().map(|c| card(h, p, c, s(c, "id") == sel)).collect();
    let items: Vec<(&'static str, E<'a, Msg>)> = vec![
        ("head", hd),
        ("cards", cols(cards, 12.0)),
        ("detail", detail(h, p, at(v, "/detail"))),
        ("invite", invite(h, p, v, &sel)),
        ("backups", backups(p, v, &sel)),
        ("schedule", schedule(p, v)),
        ("week", week(p, v)),
        ("own", own(p, v)),
    ];
    let main = hearth_ui::arrange::parts(d, &p, "srv-games", items, |ps| {
        iced::widget::scrollable(
            column![
                ps.take("head"),
                ps.take("cards"),
                row![container(ps.take("detail")).width(Length::FillPortion(11)), container(column![ps.take("invite"), ps.take("backups"), ps.take("schedule")].spacing(12.0)).width(Length::FillPortion(10))].spacing(16.0),
                row![container(ps.take("week")).width(Length::FillPortion(11)), container(ps.take("own")).width(Length::FillPortion(10))].spacing(16.0),
            ]
            .spacing(16.0),
        )
        .height(Length::Fill)
        .into()
    }, Msg::D);
    row![container(main).width(Length::Fill).height(Length::Fill).padding(pad(18.0, 22.0, 18.0, 22.0)), ui::vline(p.line), side(h, d, p)].height(Length::Fill).into()
}
