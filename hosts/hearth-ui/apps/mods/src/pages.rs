//! The Mod Manager's pages, one per board (docs/mockups/mod-manager): a game's mods (01), Add mods (02), Find mods
//! (03), Updates and going back (04), Crashes (05), Describe it (06), the controller view (07) and Every game (08),
//! with Modlists, Organizer instances, Accounts and Settings as Hearth Mods had them. Each card is a block.

use crate::{back, logic, now, pad, sources, App, Msg, Page};
use hearth_oss_kit_a::{self as k, Ch, Sz, Tag, E};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, scrollable, stack, Column, Row, Space};
use iced::{Color, Length, Task, Theme};
use serde_json::{json, Value};

const MONO: iced::Font = iced::Font::with_name("DejaVu Sans Mono");

fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        o => o.to_string(),
    }
}
fn n(v: &Value) -> u64 {
    v.as_u64().unwrap_or(0)
}
fn gap<'a>(h: f32) -> E<'a, Msg> {
    k::gap(1.0, h)
}
fn commas(x: u64) -> String {
    k::commas(x)
}

/// A game's picture: its colours (art isn't drawn in the side, as on the boards).
pub fn colours(title: &str) -> (Color, Color) {
    let l = title.to_lowercase();
    let pick = |a: &str, b: &str| (k::rgb(a), k::rgb(b));
    if l.contains("skyrim") {
        pick("#5a6a86", "#1d2333")
    } else if l.contains("fallout") {
        pick("#6a7a3a", "#1e2614")
    } else if l.contains("cyberpunk") {
        pick("#d8b83a", "#6a1d4a")
    } else if l.contains("baldur") {
        pick("#8a3a6a", "#2a0d24")
    } else if l.contains("stardew") {
        pick("#5aa84a", "#1d3a14")
    } else if l.contains("valheim") {
        pick("#3a5a86", "#0d1a2a")
    } else if l.contains("minecraft") {
        pick("#4a8a3a", "#1a3a14")
    } else if l.contains("middle-earth") || l.contains("bfme") {
        pick("#8a7a4a", "#2a2414")
    } else {
        let h = title.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        let pal = [("#3a6aa8", "#0d1424"), ("#c8a050", "#3a2a10"), ("#a85a3a", "#2a120a"), ("#7a4aa8", "#1a0d2a")];
        let (x, y) = pal[(h as usize) % pal.len()];
        pick(x, y)
    }
}

pub fn thumb<'a>(title: &str, s: f32) -> E<'a, Msg> {
    banner(title, s, s, 5.0)
}

fn banner<'a>(title: &str, w: f32, h: f32, r: f32) -> E<'a, Msg> {
    let (x, y) = colours(title);
    let gr = iced::gradient::Linear::new(iced::Degrees(150.0)).add_stop(0.0, x).add_stop(1.0, y);
    let wl = if w <= 0.0 { Length::Fill } else { Length::Fixed(w) };
    container(Space::new()).width(wl).height(Length::Fixed(h)).style(move |_: &Theme| container::Style { background: Some(iced::Background::Gradient(iced::Gradient::Linear(gr))), border: iced::Border { radius: r.into(), ..iced::Border::default() }, ..container::Style::default() }).into()
}

/// A source's tag (NEXUS MODS, THUNDERSTORE, CURSEFORGE, MOD.IO, HOME SERVER, YOURS…).
pub fn src_tag<'a>(p: &Pal, src: &str) -> E<'a, Msg> {
    let (label, kind) = match src {
        "nexus" | "Nexus Mods" => ("Nexus Mods", Tag::Acc),
        "thunderstore" | "Thunderstore" => ("Thunderstore", Tag::Bl),
        "curseforge" | "CurseForge" => ("CurseForge", Tag::Bad),
        "modio" | "mod.io" => ("mod.io", Tag::Ok),
        "modrinth" | "Modrinth" => ("Modrinth", Tag::Ok),
        "home" | "Home server" => ("Home server", Tag::Dk),
        "profile" | "Your profile" => ("Your profile", Tag::Dk),
        "skse" => ("SKSE's own site", Tag::Dk),
        "" | "yours" => ("Yours", Tag::Dk),
        _ => ("Elsewhere", Tag::Dk),
    };
    k::tag(p, label, kind)
}

fn mod_source(m: &Value) -> &'static str {
    if m["nexusId"].as_u64().is_some() {
        "nexus"
    } else {
        let f = m["installedFrom"].as_str().unwrap_or("").to_lowercase();
        if f.contains("thunderstore") {
            "thunderstore"
        } else if f.contains("curseforge") {
            "curseforge"
        } else if f.contains("mod.io") || f.contains("modio") {
            "modio"
        } else {
            "yours"
        }
    }
}

fn heading<'a>(p: &Pal, title: impl Into<String>, right: E<'a, Msg>) -> E<'a, Msg> {
    row![k::h2::<Msg>(p, title), k::fill(), right].spacing(8.0).align_y(iced::Center).into()
}

fn none<'a>() -> E<'a, Msg> {
    Space::new().width(Length::Shrink).height(Length::Shrink).into()
}

fn kvl<'a>(p: &Pal, l: impl Into<String>, r: impl Into<String>, c: Color) -> E<'a, Msg> {
    row![t(l.into(), 12.0, p.text).wrapping(Wrapping::None), k::fill(), t(r.into(), 12.0, c).wrapping(Wrapping::None)].spacing(8.0).into()
}

fn box_note<'a>(p: &Pal, title: impl Into<String>, words: impl Into<String>, c: Color) -> E<'a, Msg> {
    container(column![tb(title.into(), 12.0, p.text), k::para(p.muted, words.into(), 11.0)].spacing(3.0)).padding(10.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, c, 0.12), p.r(9.0), Some((Pal::mix(p.line, c, 0.5), 1.0)))).into()
}

/// The page that's open, made of its blocks.
pub fn page(a: &App) -> E<'_, Msg> {
    let body: E<'_, Msg> = match a.page {
        Page::Game => game_page(a),
        Page::Downloads => install_page(a),
        Page::Find => find_page(a),
        Page::Updates => updates_page(a),
        Page::Crashes => crash_page(a),
        Page::Describe => describe_page(a),
        Page::Every => every_page(a),
        Page::Modlists => modlists_page(a),
        Page::Organizer => organizer_page(a),
        Page::Accounts => accounts_page(a),
        Page::Settings => settings_page(a),
    };
    let mut c = Column::new().push(container(body).height(Length::Fill));
    if !a.said.is_empty() {
        c = c.push(container(k::said(&a.p, &a.said)).padding(k::pad(0.0, 16.0, 6.0, 16.0)));
    }
    c.into()
}

// ================================================================== board 1: a game's mods

pub fn health(a: &App) -> (Vec<(Color, String, String, &'static str)>, usize, usize) {
    let mut v: Vec<(Color, String, String, &'static str)> = Vec::new();
    let plugins = a.plugins();
    let missing: Vec<&Value> = plugins.iter().filter(|p| p["active"] == true && p["missing"].as_array().map(|m| !m.is_empty()).unwrap_or(false)).collect();
    if !missing.is_empty() {
        let names: Vec<String> = missing.iter().take(2).map(|p| s(&p["name"])).collect();
        let masters: Vec<String> = missing.iter().flat_map(|p| p["missing"].as_array().cloned().unwrap_or_default()).map(|x| s(&x)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        v.push((k::RED, format!("{} plugin{} miss a master", missing.len(), if missing.len() == 1 { "" } else { "s" }), format!("{} need {}, which is off. Fix it turns it back on.", names.join(" and "), masters.join(", ")), "fix"));
    }
    for w in logic::masterlist_warnings(&logic::plugins_of(&a.detail.s), &a.detail.rules).into_iter().take(2) {
        v.push((k::YELLOW, "LOOT's masterlist has a note".into(), w, ""));
    }
    let losers: Vec<String> = a.mods().iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true && n(&m["files"]) > 0 && n(&m["conflicts"]["winning"]) == 0 && n(&m["conflicts"]["losing"]) > 0).map(|m| s(&m["name"])).collect();
    if !losers.is_empty() {
        v.push((k::BLUE, format!("{} mod{} fully lose every file", losers.len(), if losers.len() == 1 { "" } else { "s" }), "They do nothing as placed. Turn them off, or move them down.".into(), "show"));
    }
    for w in a.detail.s["warnings"].as_array().cloned().unwrap_or_default() {
        let w = s(&w);
        if !w.contains("masters that are missing") {
            v.push((k::YELLOW, w, String::new(), ""));
        }
    }
    let fix = v.iter().filter(|x| x.0 == k::RED).count() + v.iter().filter(|x| x.0 == k::YELLOW).count();
    let look = v.iter().filter(|x| x.0 == k::BLUE).count();
    (v, fix, look)
}

fn hero(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let Some(g) = a.game().cloned() else { return k::mu(&p, "No games yet.", 13.0) };
    let (_, fix, _) = health(a);
    let mut tags = Row::new().spacing(8.0);
    if a.detail.organizer {
        tags = tags.push(k::tag(&p, "Hearth Mod Organizer · works like Mod Organizer 2", Tag::Dk));
    }
    if fix > 0 {
        tags = tags.push(k::tag(&p, format!("{fix} thing{} to fix", if fix == 1 { "" } else { "s" }), Tag::Warn));
    }
    let mods = a.mods();
    let total = mods.iter().filter(|m| m["kind"] == "mod").count();
    let on = mods.iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true).count();
    let pl = a.plugins().len();
    let ext = a.detail.s["executables"].as_array().and_then(|e| e.iter().find(|x| x["extender"] == true)).map(|x| s(&x["title"])).unwrap_or_default();
    let since = a.detail.s["deployed"]["since"].as_i64().or(a.detail.s["deployedAt"].as_i64()).map(|t| format!(" · mods went in {}", hearth_ui::time::when(if t > 10_000_000_000 { t / 1000 } else { t }))).unwrap_or_default();
    let mut line = format!("{total} mods · {on} on");
    if pl > 0 {
        line.push_str(&format!(" · {pl} plugins"));
    }
    if !ext.is_empty() {
        line.push_str(&format!(" · {ext}"));
    }
    line.push_str(&since);
    let words = column![tags, gap(6.0), tb(g.title.clone(), 30.0, p.text).wrapping(Wrapping::None), t(line, 12.0, p.muted).wrapping(Wrapping::None)].spacing(2.0);
    let profile = k::field(&p, format!("Profile  {} ▾", a.profile()), 200.0, Some(Msg::Go(Page::Game)));
    let profiles: Vec<Value> = a.detail.s["profiles"].as_array().cloned().unwrap_or_default();
    let next = profiles.iter().map(|x| s(&x["name"])).find(|x| *x != a.profile());
    let profile = k::click(profile, next.map(Msg::Profile));
    let btns = row![profile, k::chip(&p, "▶ Play modded", Ch::Acc, Sz::Lg, Some(Msg::Play)), k::chip(&p, "Add to Steam", Ch::Plain, Sz::Lg, Some(Msg::AddSteam)), k::chip(&p, "+ Add mods", Ch::Plain, Sz::Lg, Some(Msg::Go(Page::Downloads))), k::chip(&p, "⋯", Ch::Plain, Sz::Lg, Some(Msg::More))].spacing(8.0).align_y(iced::Center);
    let mut starts = Row::new().spacing(0.0).align_y(iced::Center).push(t("Starts", 11.0, p.muted)).push(k::gap(8.0, 1.0));
    let exes: Vec<(String, bool, Msg)> = a.detail.s["executables"].as_array().cloned().unwrap_or_default().iter().take(6).map(|x| (s(&x["title"]), s(&x["title"]) == a.starts, Msg::Starts(s(&x["title"])))).collect();
    if !exes.is_empty() {
        starts = starts.push(k::seg(&p, exes));
    }
    let right = column![btns, starts].spacing(10.0).align_x(iced::Right);
    let front = container(row![words, k::fill(), right].align_y(iced::Center)).padding(k::pad(14.0, 22.0, 14.0, 22.0)).width(Length::Fill);
    let (x, _) = colours(&g.title);
    let gr = iced::gradient::Linear::new(iced::Degrees(100.0)).add_stop(0.0, Pal::mix(p.bg, x, 0.45)).add_stop(0.7, Pal::mix(p.bg, x, 0.15)).add_stop(1.0, p.bg);
    let back = container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_: &Theme| container::Style { background: Some(iced::Background::Gradient(iced::Gradient::Linear(gr))), ..container::Style::default() });
    container(stack![back, front]).width(Length::Fill).height(Length::Fixed(128.0)).into()
}

/// The mods shown under the filter: (row or separator, folded away?).
pub fn shown_mods(a: &App) -> Vec<Value> {
    let q = a.filter_text.to_lowercase();
    let ups: Vec<String> = a.updates["updates"].as_array().cloned().unwrap_or_default().iter().map(|u| s(&u["mod"]).to_lowercase()).collect();
    let mut out = Vec::new();
    let mut sep = String::new();
    // MO2's modlist.txt is highest priority first; the ladder shows lowest first ("lower wins")
    let mut mods = a.mods();
    mods.sort_by_key(|m| m["priority"].as_i64().unwrap_or(0));
    for m in mods {
        let kind = m["kind"].as_str().unwrap_or("mod");
        if kind == "separator" {
            sep = s(&m["name"]);
            out.push(m);
            continue;
        }
        if kind == "unmanaged" {
            continue;
        }
        if a.folded.contains(&sep) {
            continue;
        }
        let name = s(&m["name"]).to_lowercase();
        if !q.is_empty() && !name.contains(&q) {
            continue;
        }
        let ok = match a.filter {
            1 => m["enabled"] == true,
            2 => m["enabled"] == false,
            3 => n(&m["conflicts"]["losing"]) > 0 || !m["conflicts"]["winsOver"].as_array().map(|x| x.is_empty()).unwrap_or(true),
            4 => ups.contains(&name),
            5 => n(&m["conflicts"]["winning"]) == 0 && n(&m["conflicts"]["losing"]) > 0,
            _ => true,
        };
        if ok {
            out.push(m);
        }
    }
    out
}

fn ladder(a: &App, big: bool) -> E<'static, Msg> {
    let p = a.p;
    let fs = if big { 18.0 } else { 13.0 };
    let rh = if big { 50.0 } else { 26.0 };
    let mods = a.mods();
    let all: Vec<&Value> = mods.iter().filter(|m| m["kind"] == "mod").collect();
    let picked = mods.iter().find(|m| m["id"].as_str() == Some(a.picked.as_str())).cloned().unwrap_or(Value::Null);
    let beats: Vec<String> = picked["conflicts"]["winsOver"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
    let beaten: Vec<String> = picked["conflicts"]["losesTo"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
    let ups = a.updates["updates"].as_array().cloned().unwrap_or_default();
    let mut list = Column::new().spacing(if big { 6.0 } else { 0.0 });
    let shown = shown_mods(a);
    // the separators' counts
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut cur = String::new();
    let mut sorted = mods.clone();
    sorted.sort_by_key(|m| m["priority"].as_i64().unwrap_or(0));
    for m in &sorted {
        if m["kind"] == "separator" {
            cur = s(&m["name"]);
        } else if m["kind"] == "mod" {
            *counts.entry(cur.clone()).or_default() += 1;
        }
    }
    for m in shown {
        let name = s(&m["name"]);
        if m["kind"] == "separator" {
            let folded = a.folded.contains(&name);
            let c = counts.get(&name).copied().unwrap_or(0);
            let sep = row![t(format!("{} {}", if folded { "▸" } else { "▾" }, name.to_uppercase()), if big { 13.0 } else { 10.5 }, p.accent).font(iced::Font { weight: iced::font::Weight::Bold, ..app_font() }), t(if big { String::new() } else { format!("· {c} mods") }, 10.0, p.muted)].spacing(6.0);
            list = list.push(k::click(container(sep).padding(k::pad(if big { 14.0 } else { 7.0 }, 4.0, 3.0, 8.0)), Some(Msg::Fold(name))));
            continue;
        }
        let id = s(&m["id"]);
        let on = m["enabled"] == true;
        let prio = m["priority"].as_i64().unwrap_or(0);
        let is_pick = id == a.picked;
        let lifted = a.lifted.as_ref().map(|l| l.0 == id).unwrap_or(false);
        let up = ups.iter().find(|u| s(&u["mod"]).eq_ignore_ascii_case(&name));
        let (mark, mc): (String, Color) = if lifted {
            ("picked up".into(), p.accent)
        } else if is_pick {
            ("PICKED".into(), p.accent)
        } else if beats.contains(&name) {
            (format!("+ {}", commas(n(&picked["conflicts"]["winning"]).min(n(&m["files"])))), k::GREEN)
        } else if beaten.contains(&name) {
            (format!("− {}", commas(n(&m["conflicts"]["winning"]))), k::RED)
        } else if up.is_some() {
            ("UPDATE".into(), k::BLUE)
        } else if m["plugins"].as_array().map(|pl| pl.iter().any(|x| a.plugins().iter().any(|q| q["name"] == *x && q["active"] == true && q["missing"].as_array().map(|mm| !mm.is_empty()).unwrap_or(false)))).unwrap_or(false) {
            ("NEEDS A FIX".into(), k::YELLOW)
        } else {
            (String::new(), p.muted)
        };
        let ver = s(&m["version"]);
        let handle = mouse_area(container(t("⋮⋮", fs - 2.0, p.muted)).width(Length::Fixed(22.0))).on_press(Msg::Lift(id.clone()));
        let namec = if on { p.text } else { p.muted };
        let r: E<'static, Msg> = if big {
            let right = if let Some(u) = up { format!("{} → {} ready", s(&u["from"]), s(&u["to"])) } else if lifted { format!("{ver} · picked up") } else { ver.clone() };
            row![handle, k::switch(&p, on, Some(Msg::Toggle(id.clone(), !on))), container(t(prio.to_string(), 14.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::Right), t(name.clone(), fs, namec).wrapping(Wrapping::None), k::fill(), t(right, 14.0, if up.is_some() { k::BLUE } else { p.muted })].spacing(14.0).align_y(iced::Center).into()
        } else {
            row![
                handle,
                k::switch(&p, on, Some(Msg::Toggle(id.clone(), !on))),
                container(t(prio.to_string(), 11.0, p.muted)).width(Length::Fixed(34.0)).align_x(iced::Right),
                container(t(ui::fit(&name, 34), fs, namec).wrapping(Wrapping::None)).width(Length::FillPortion(6)),
                container(t(ui::fit(&ver, 8), 11.0, p.muted)).width(Length::FillPortion(1)),
                container(src_tag(&p, mod_source(&m))).width(Length::FillPortion(2)),
                container(t(mark, 10.5, mc).font(iced::Font { weight: iced::font::Weight::Bold, ..app_font() })).width(Length::FillPortion(2)),
            ]
            .spacing(8.0)
            .align_y(iced::Center)
            .into()
        };
        let bg = if lifted || is_pick {
            Pal::mix(p.panel, p.accent, 0.16)
        } else if beats.contains(&name) {
            Pal::mix(p.panel, k::GREEN, 0.07)
        } else if beaten.contains(&name) {
            Pal::mix(p.panel, k::RED, 0.08)
        } else if big {
            p.panel
        } else {
            Color::TRANSPARENT
        };
        let line = if lifted || (big && is_pick) { Some((p.accent, 2.0)) } else { None };
        let cell = container(r).padding(k::pad(0.0, 8.0, 0.0, 6.0)).height(Length::Fixed(rh)).center_y(Length::Fixed(rh)).width(Length::Fill).style(ui::boxed(bg, p.r(if big { 10.0 } else { 4.0 }), line));
        let cell = mouse_area(cell).on_release(Msg::DropOn(prio)).on_press(Msg::Pick(id.clone()));
        list = list.push(cell);
    }
    if all.is_empty() {
        list = list.push(k::mu(&p, if a.detail.organizer { "No mods in this profile yet: + Add mods, or Find mods." } else { "No mods for this game yet: + Add mods, or Find mods. (Every game › Set up mods gives it a Hearth Mod Organizer instance: separators, plugins, profiles.)" }, 12.0));
    }
    scrollable(list).height(Length::Fill).into()
}

fn mods_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mods = a.mods();
    let total = mods.iter().filter(|m| m["kind"] == "mod").count();
    let archives = a.detail.s["archives"].as_array().map(|x| x.len()).unwrap_or(0);
    let ow = mods.iter().find(|m| m["kind"] == "overwrite").map(|m| n(&m["files"])).unwrap_or(0);
    let conflicts = mods.iter().filter(|m| m["kind"] == "mod" && (n(&m["conflicts"]["losing"]) > 0 || m["conflicts"]["winsOver"].as_array().map(|x| !x.is_empty()).unwrap_or(false))).count();
    let ups = a.updates["updates"].as_array().map(|u| u.iter().filter(|x| s(&x["game"]) == a.game().map(|g| g.title.clone()).unwrap_or_default()).count()).unwrap_or(0);
    let head = row![
        k::h2::<Msg>(&p, "Mods"),
        k::gap(10.0, 1.0),
        k::tabs(&p, vec![(format!("Mods {total}"), a.tab == 0, Msg::Tab(0)), (format!("Plugins {}", a.plugins().len()), a.tab == 1, Msg::Tab(1)), (format!("Archives {archives}"), a.tab == 2, Msg::Tab(2)), (format!("Overwrite {ow}"), a.tab == 3, Msg::Tab(3))]),
        k::fill(),
        k::mu1::<Msg>(&p, "lower wins", 11.0)
    ]
    .align_y(iced::Center);
    let filters = row![
        container(k::input(&p, &format!("⌕ Filter {total} mods"), &a.filter_text, Msg::FilterText)).width(Length::Fill),
        k::seg(&p, vec![("All".into(), a.filter == 0, Msg::Filter(0)), ("On".into(), a.filter == 1, Msg::Filter(1)), ("Off".into(), a.filter == 2, Msg::Filter(2)), (format!("Conflicts {conflicts}"), a.filter == 3, Msg::Filter(3)), (format!("Updates {ups}"), a.filter == 4, Msg::Filter(4))])
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    let cols = row![k::gap(70.0, 1.0), container(k::cap::<Msg>(&p, "#   Mod")).width(Length::FillPortion(6)), container(k::cap::<Msg>(&p, "Version")).width(Length::FillPortion(1)), container(k::cap::<Msg>(&p, "From")).width(Length::FillPortion(2)), container(k::cap::<Msg>(&p, "Conflicts")).width(Length::FillPortion(2))].spacing(8.0);
    let body: E<'static, Msg> = match a.tab {
        1 => plugins_list(a, true),
        2 => {
            let mut c = Column::new().spacing(3.0);
            for x in a.detail.s["archives"].as_array().cloned().unwrap_or_default() {
                c = c.push(kvl(&p, s(&x["name"]), s(&x["mod"]), p.muted));
            }
            scrollable(c).height(Length::Fill).into()
        }
        3 => k::para(p.muted, format!("{ow} files the game wrote while it ran (its own folder stays as it was). Move them into a mod of their own from the old page's Overwrite, or leave them here."), 12.0),
        _ => column![cols, ladder(a, false)].spacing(4.0).into(),
    };
    let foot = row![k::mu1::<Msg>(&p, "Drag by ⋮⋮ · controller: A picks up, ↑↓ moves, A drops", 11.0), k::fill(), k::tag(&p, "+ beats the picked mod", Tag::Ok), k::tag(&p, "− beats it", Tag::Bad)].spacing(6.0).align_y(iced::Center);
    k::card(&p, column![head, gap(10.0), filters, gap(8.0), container(body).height(Length::Fill), gap(6.0), foot])
}

fn health_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let (v, fix, look) = health(a);
    let mut c = Column::new().spacing(8.0).push(heading(&p, "Health", k::tag(&p, format!("{fix} to fix · {look} to look at"), if fix > 0 { Tag::Warn } else { Tag::Ok })));
    if v.is_empty() {
        c = c.push(box_note(&p, "All healthy", "No missing masters, nothing that loses every file, nothing the masterlist warns about.", k::GREEN));
    }
    for (col, title, words, act) in v.into_iter().take(4) {
        let btn: E<'static, Msg> = match act {
            "fix" => k::chip(&p, "Fix it", Ch::Acc, Sz::Sm, Some(Msg::FixMasters)),
            "show" => k::click(t("Show them", 11.0, p.accent), Some(Msg::ShowLosers)),
            _ => none(),
        };
        let inner = column![row![k::dot(col, 8.0), tb(title, 12.0, p.text), k::fill(), btn].spacing(8.0).align_y(iced::Center), k::para(p.muted, words, 11.0)].spacing(4.0);
        c = c.push(container(inner).padding(10.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, col, 0.08), p.r(9.0), Some((Pal::mix(p.line, col, 0.6), 1.0)))));
    }
    k::card(&p, scrollable(c).height(Length::Fill))
}

fn plugins_list(a: &App, long: bool) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(1.0).push(row![container(k::cap::<Msg>(&p, "Idx")).width(Length::Fixed(52.0)), container(k::cap::<Msg>(&p, "Plugin")).width(Length::FillPortion(5)), container(k::cap::<Msg>(&p, "Kind")).width(Length::FillPortion(2)), container(k::cap::<Msg>(&p, "Masters")).width(Length::FillPortion(3))].spacing(6.0));
    let (mut full, mut light) = (0u32, 0u32);
    let list = a.plugins();
    for x in list.iter().take(if long { 400 } else { 9 }) {
        let active = x["active"] == true;
        let is_light = x["light"] == true;
        let idx = if !active {
            "—".to_string()
        } else if is_light {
            light += 1;
            format!("FE:{:03X}", light - 1)
        } else {
            full += 1;
            format!("{:02X}", full - 1)
        };
        let name = s(&x["name"]);
        let lower = name.to_lowercase();
        let kind = if !active { "off" } else if lower.ends_with(".esm") { "ESM" } else if lower.ends_with(".esl") { "ESL" } else if is_light { "ESL" } else if x["master"] == true { "ESM-flag" } else { "ESP" };
        let missing: Vec<String> = x["missing"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
        let masters: Vec<String> = x["masters"].as_array().cloned().unwrap_or_default().iter().map(|m| s(m).trim_end_matches(".esm").trim_end_matches(".esp").to_string()).collect();
        let mtext = if !missing.is_empty() { format!("{} off", missing.join(", ")) } else if masters.is_empty() { "—".into() } else if masters.len() > 2 { format!("{} masters", masters.len()) } else { masters.join(", ") };
        let r = row![
            container(t(idx, 10.5, p.muted).font(MONO)).width(Length::Fixed(52.0)),
            container(k::click(t(ui::fit(&name, 24), 12.0, if active { p.text } else { p.muted }).wrapping(Wrapping::None), Some(Msg::PluginToggle(name.clone(), !active)))).width(Length::FillPortion(5)),
            container(k::tag::<Msg>(&p, kind, if kind == "off" { Tag::Dk } else if kind == "ESL" { Tag::Ok } else { Tag::Dk })).width(Length::FillPortion(2)),
            container(t(ui::fit(&mtext, 22), 11.0, if missing.is_empty() { p.muted } else { k::RED }).wrapping(Wrapping::None)).width(Length::FillPortion(3)),
        ]
        .spacing(6.0)
        .align_y(iced::Center);
        c = c.push(container(r).padding(k::pad(3.0, 0.0, 3.0, 0.0)));
    }
    let l = list.iter().filter(|x| x["light"] == true).count();
    c = c.push(gap(4.0)).push(k::mu(&p, format!("{} plugins · {l} light (FE) · masters always first", list.len()), 11.0));
    if long {
        scrollable(c).height(Length::Fill).into()
    } else {
        c.into()
    }
}

fn plugins_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    k::card(&p, column![heading(&p, "Plugins · load order", k::chip(&p, "Sort", Ch::Acc, Sz::Sm, Some(Msg::Sort))), gap(8.0), plugins_list(a, false)])
}

fn sort_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let src = if a.detail.masterlist.is_empty() { "the plugins' own masters".to_string() } else { a.detail.masterlist.clone() };
    let mut c = Column::new().spacing(6.0).push(heading(&p, "Sort · what it would change", k::mu1(&p, src, 11.0)));
    match &a.sort {
        None => c = c.push(k::mu(&p, "Sort shows here what it would move and why, before anything moves. Your own order wins wherever nothing says otherwise.", 11.0)),
        Some((order, moves)) => {
            for m in moves.iter().take(4) {
                c = c.push(kvl(&p, format!("↓ {} after {}", ui::fit(&m.plugin, 26), ui::fit(&m.other, 18)), m.why.clone(), p.muted));
            }
            let kept = order.len().saturating_sub(moves.len());
            c = c.push(kvl(&p, format!("= {kept} others kept where you put them"), "your order wins", p.muted));
            c = c.push(row![k::chip(&p, format!("Apply {} move{}", moves.len(), if moves.len() == 1 { "" } else { "s" }), Ch::Acc, Sz::Md, (!moves.is_empty()).then_some(Msg::ApplySort)), k::chip(&p, "Not now", Ch::Plain, Sz::Md, Some(Msg::NotNow)), k::fill(), k::mu1::<Msg>(&p, "a snapshot is made first", 11.0)].spacing(8.0).align_y(iced::Center));
        }
    }
    k::card_fit(&p, c)
}

fn lens_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mods = a.mods();
    let m = mods.iter().find(|m| m["id"].as_str() == Some(a.picked.as_str())).cloned().unwrap_or(Value::Null);
    let name = s(&m["name"]);
    let files: Vec<Value> = a.lens["files"].as_array().cloned().unwrap_or_default();
    let wins = files.iter().filter(|f| f["status"] == "wins").count();
    let loses = files.iter().filter(|f| f["status"] == "loses").count();
    let alone = files.iter().filter(|f| f["status"] == "alone").count();
    let mut c = Column::new().spacing(5.0).push(heading(&p, format!("Conflict lens · {}", ui::fit(&name, 24)), k::seg(&p, vec![("Files".into(), a.lens_tab == 0, Msg::LensTab(0)), ("Mods".into(), a.lens_tab == 1, Msg::LensTab(1))])));
    c = c.push(row![k::tag::<Msg>(&p, format!("Wins {} files", commas(wins as u64)), Tag::Ok), k::tag::<Msg>(&p, format!("Loses {}", commas(loses as u64)), Tag::Bad), k::tag::<Msg>(&p, format!("Alone {}", commas(alone as u64)), Tag::Dk)].spacing(6.0));
    // by mod: whom it beats and loses to, with file counts
    let mut by: std::collections::BTreeMap<(bool, String), usize> = std::collections::BTreeMap::new();
    for f in &files {
        if f["status"] == "wins" {
            for o in f["over"].as_array().cloned().unwrap_or_default() {
                *by.entry((true, s(&o))).or_default() += 1;
            }
        } else if f["status"] == "loses" {
            *by.entry((false, s(&f["to"]))).or_default() += 1;
        }
    }
    let mut pairs: Vec<((bool, String), usize)> = by.into_iter().collect();
    pairs.sort_by(|x, y| y.0 .0.cmp(&x.0 .0).then(y.1.cmp(&x.1)));
    for ((win, other), cnt) in pairs.iter().take(4) {
        c = c.push(row![k::tag::<Msg>(&p, if *win { "+" } else { "−" }, if *win { Tag::Ok } else { Tag::Bad }), t(format!("{} {}", if *win { "beats" } else { "loses to" }, other), 12.0, p.text), k::fill(), t(format!("{} files", commas(*cnt as u64)), 11.0, p.muted)].spacing(8.0).align_y(iced::Center));
    }
    if a.lens_tab == 0 {
        c = c.push(gap(4.0)).push(k::cap(&p, "File by file"));
        for f in files.iter().filter(|f| f["status"] != "alone").take(5).chain(files.iter().filter(|f| f["status"] == "alone").take(1)) {
            let (col, what) = match f["status"].as_str().unwrap_or("") {
                "wins" => (k::GREEN, format!("wins over {}", f["over"].as_array().map(|o| if o.len() == 1 { s(&o[0]) } else { format!("{} mods", o.len()) }).unwrap_or_default())),
                "loses" => (k::RED, format!("loses to {}", ui::fit(&s(&f["to"]), 16))),
                "hidden" => (p.muted, "hidden".into()),
                _ => (p.muted, "alone".into()),
            };
            let path = s(&f["path"]);
            c = c.push(row![k::click(t(ui::fit(&path, 44), 11.0, col).font(MONO).wrapping(Wrapping::None), Some(Msg::HideFile(path.clone()))), k::fill(), t(what, 11.0, p.muted).wrapping(Wrapping::None)].spacing(8.0));
        }
        c = c.push(k::click(t(format!("All {} files ›", commas(files.len() as u64)), 11.0, p.accent), Some(Msg::Filter(3))));
    }
    c = c.push(gap(4.0)).push(k::cap(&p, "Change who wins"));
    let first_loss = pairs.iter().find(|x| !x.0 .0).map(|x| x.0 .1.clone());
    let mut btns = Row::new().spacing(6.0);
    if let Some(o) = first_loss.clone() {
        btns = btns.push(k::chip(&p, format!("Move below {}", ui::fit(&o, 20)), Ch::Plain, Sz::Md, Some(Msg::MoveBelow(o))));
    }
    let first_file = files.iter().find(|f| f["status"] == "wins").map(|f| s(&f["path"]));
    btns = btns.push(k::chip(&p, "Hide one file in this mod", Ch::Plain, Sz::Md, first_file.map(Msg::HideFile)));
    c = c.push(btns).push(k::chip(&p, "Make a patch with the winners", Ch::Plain, Sz::Md, None)).push(k::mu(&p, "Hiding renames nothing in the game · undo any time", 11.0));
    if name.is_empty() {
        return k::card(&p, column![k::h2::<Msg>(&p, "Conflict lens"), k::mu(&p, "Pick a mod in the list: what it wins and loses, file by file.", 12.0)].spacing(8.0));
    }
    k::card(&p, scrollable(c).height(Length::Fill))
}

fn profiles_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(5.0).push(heading(&p, "Profiles", k::click(t("+ New from this one", 11.0, p.accent), Some(Msg::ProfileNew))));
    for x in a.detail.s["profiles"].as_array().cloned().unwrap_or_default() {
        let on = x["active"] == true;
        let words = format!("{} · {}", if x["localSaves"] == true { "own saves" } else { "shared saves" }, if x["localInis"] == true { "own INIs" } else { "shared INIs" });
        c = c.push(k::click(row![k::dot(if on { k::GREEN } else { p.muted }, 7.0), t(s(&x["name"]), 12.0, if on { p.text } else { p.muted }), k::fill(), k::mu1::<Msg>(&p, words, 11.0)].spacing(8.0).align_y(iced::Center), Some(Msg::Profile(s(&x["name"])))));
    }
    k::card_fit(&p, c)
}

fn folder_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let g = a.game().map(|g| g.short()).unwrap_or_default();
    let dep = &a.detail.s["deployed"];
    let mode = dep["mode"].as_str().unwrap_or("overlay");
    let words = if mode == "overlay" || dep.is_null() { format!("While {g} runs the mods are a layer over its folder; when it closes the layer goes away. Steam's checks always pass.") } else { format!("{g}'s mods are linked in while it runs (no fuse-overlayfs here); every game file they replace is set aside and put back after.") };
    let ow = a.mods().iter().find(|m| m["kind"] == "overwrite").map(|m| n(&m["files"])).unwrap_or(0);
    k::card_fit(&p, column![heading(&p, "The game folder", k::tag(&p, "Never touched", Tag::Ok)), k::para(p.muted, words, 11.0), kvl(&p, "Game files changed", "0", k::GREEN), kvl(&p, "Written by the game last time", format!("{ow} files → Overwrite"), p.muted)].spacing(6.0))
}

fn game_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    if a.game().is_none() {
        return container(k::mu(&p, if a.loading { "Looking for your games…" } else { "No moddable games found yet: Every game › + Add a game." }, 13.0)).padding(40.0).into();
    }
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("hero", hero(a)), ("ladder", mods_card(a)), ("health", health_card(a)), ("lens", lens_card(a)), ("plugins", plugins_card(a)), ("sort", sort_card(a)), ("profiles", profiles_card(a)), ("folder", folder_card(a))];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "game",
        items,
        move |ps| {
            let mid = column![container(ps.take("health")).height(Length::FillPortion(4)), container(ps.take("lens")).height(Length::FillPortion(5))].spacing(12.0).width(Length::FillPortion(4));
            let right = column![ps.take("plugins"), ps.take("sort"), ps.take("profiles"), ps.take("folder")].spacing(12.0).width(Length::FillPortion(4));
            let cards = row![container(ps.take("ladder")).width(Length::FillPortion(6)).height(Length::Fill), mid, scrollable(right).height(Length::Fill).width(Length::FillPortion(4))].spacing(12.0).height(Length::Fill);
            column![ps.take("hero"), ui::hline(p.line), container(cards).padding(k::pad(12.0, 16.0, 8.0, 16.0)).height(Length::Fill)].into()
        },
        Msg::Describe,
    )
}

fn app_font() -> iced::Font {
    hearth_ui::app::INTER
}

// ================================================================== board 2: Add mods

fn install_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let g = a.game().map(|g| g.short()).unwrap_or_default();
    let drop = container(column![tb("Drop an archive or a folder", 15.0, p.text), k::mu(&p, ".7z · .zip · .rar · .fomod · a mod's folder", 11.0)].spacing(4.0).align_x(iced::Center)).center_x(Length::Fill).center_y(Length::Fixed(150.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.05), p.r(10.0), Some((Pal::mix(p.line, p.accent, 0.5), 1.0))));
    let add = k::card_fit(&p, column![heading(&p, "Add mods", k::mu1(&p, format!("to {g}"), 11.0)), drop, row![k::chip(&p, "Choose a file…", Ch::Plain, Sz::Md, Some(Msg::PickFile)), container(k::input(&p, "Paste a mod's address", &a.addr, Msg::Addr).on_submit(Msg::AddrGo)).width(Length::Fill)].spacing(8.0)].spacing(10.0));
    // From links: the download queue (nxm links from "Mod Manager Download")
    let mut links = Column::new().spacing(8.0).push(heading(&p, "From links", k::tag(&p, "nxm links on", Tag::Ok))).push(k::para(p.muted, "\"Mod Manager Download\" on a Nexus Mods page comes straight here, with the mod's page remembered for updates.", 11.0));
    for i in a.downloads["items"].as_array().cloned().unwrap_or_default().into_iter().take(5) {
        let st = s(&i["state"]);
        let right: E<'static, Msg> = if st == "done" {
            k::tag(&p, "Ready", Tag::Ok)
        } else if st == "downloading" || st == "paused" {
            let frac = n(&i["done"]) as f32 / n(&i["size"]).max(1) as f32;
            row![k::bar(&p, frac, 5.0, Length::Fixed(140.0)), k::mu1::<Msg>(&p, format!("{}%", (frac * 100.0) as u32), 11.0)].spacing(6.0).align_y(iced::Center).into()
        } else {
            k::mu1(&p, st, 11.0)
        };
        links = links.push(row![src_tag(&p, if s(&i["domain"]).is_empty() { "yours" } else { "nexus" }), t(ui::fit(&s(&i["name"]), 26), 12.0, p.text), k::fill(), right].spacing(8.0).align_y(iced::Center));
    }
    let links = k::card_fit(&p, links);
    let mut wait = Column::new().spacing(6.0).push(heading(&p, "Waiting to go in", k::click(t(format!("Install all {}", a.waiting.len()), 11.0, p.accent), (!a.waiting.is_empty()).then_some(Msg::InstallAll))));
    for w in a.waiting.iter().take(6) {
        let name = std::path::Path::new(w).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        wait = wait.push(row![column![t(name, 12.0, p.text), k::mu(&p, "an archive · what's inside is read when it goes in", 10.0)], k::fill(), k::chip(&p, "Install", Ch::Plain, Sz::Sm, Some(Msg::Install(w.clone())))].align_y(iced::Center));
    }
    if a.waiting.is_empty() {
        wait = wait.push(k::mu(&p, "Nothing waiting. Finished downloads wait here.", 11.0));
    }
    wait = wait.push(k::mu(&p, "Your answers are kept: installing again asks only what's new.", 11.0));
    let server = match sources::home_server() {
        Some(d) => box_note(&p, "Archives kept on your home server", format!("Each file is downloaded once into {}; the Ally and the laptop take it from there.", k::tilde(&d)), k::GREEN),
        None => box_note(&p, "Archives kept on your home server", "Set up Hearth's home server (Settings › Home server) and every file is downloaded once; your other devices take it from there.", p.muted),
    };
    let wait = k::card(&p, column![wait, k::fill(), server].spacing(8.0));
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("add", add), ("links", links), ("waiting", wait), ("installer", installer(a)), ("before", before_card(a)), ("where", where_card(a)), ("already", already_card(a))];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "install",
        items,
        |ps| {
            let l = column![ps.take("add"), ps.take("links"), container(ps.take("waiting")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(3));
            let r = column![ps.take("before"), ps.take("where"), container(ps.take("already")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(3));
            container(row![l, container(ps.take("installer")).width(Length::FillPortion(6)).height(Length::Fill), r].spacing(14.0)).padding(k::pad(14.0, 16.0, 8.0, 16.0)).into()
        },
        Msg::Describe,
    )
}

fn installer(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let f = &a.fomod["fomod"];
    if f.is_null() {
        return k::card(&p, column![k::h2::<Msg>(&p, "The installer"), k::para(p.muted, "A mod with a FOMOD installer asks its questions here, in Hearth, with the author's own pictures and words, big enough for the TV. BAIN packages and plain archives go straight in.", 12.0)].spacing(8.0));
    }
    let steps: Vec<Value> = f["steps"].as_array().cloned().unwrap_or_default();
    let ev = &a.fomod["evaluation"];
    let si = a.fomod_step.min(steps.len().saturating_sub(1));
    let mut sl = Column::new().spacing(4.0).push(k::cap(&p, "Steps"));
    for (i, st) in steps.iter().enumerate() {
        if ev["steps"][i]["visible"] == false {
            continue;
        }
        let (mk, bg) = if i < si { ("✓".to_string(), k::GREEN) } else { ((i + 1).to_string(), if i == si { p.accent } else { p.muted }) };
        let r = row![container(tb(mk, 10.0, if i == si { p.ink } else { bg })).center_x(Length::Fixed(20.0)).center_y(Length::Fixed(20.0)).style(ui::boxed(if i == si { p.accent } else { Color::TRANSPARENT }, 10.0, Some((bg, 1.0)))), t(s(&st["name"]), 12.0, p.text)].spacing(8.0).align_y(iced::Center);
        sl = sl.push(k::click(container(r).padding(6.0).width(Length::Fill).style(ui::boxed(if i == si { Pal::mix(p.panel, p.accent, 0.1) } else { Color::TRANSPARENT }, p.r(7.0), None)), Some(Msg::FomodStep(i))));
    }
    sl = sl.push(gap(10.0)).push(k::cap(&p, "From last time")).push(k::para(p.muted, "Choices you make now are kept with the mod.", 11.0)).push(gap(6.0)).push(k::cap(&p, "Also reads")).push(k::para(p.muted, "FOMOD steps and pictures · BAIN packages · plain archives", 11.0));
    let st = steps.get(si).cloned().unwrap_or(Value::Null);
    let mut body = Column::new().spacing(10.0);
    for (gi, g) in st["groups"].as_array().cloned().unwrap_or_default().iter().enumerate() {
        let kind = s(&g["kind"]);
        let one = kind.contains("Exactly") || kind.contains("AtMost");
        body = body.push(heading(&p, s(&g["name"]), k::mu1(&p, if one { "pick one" } else { "ticked for you · change any" }, 11.0)));
        let sel: Vec<usize> = ev["choices"][si][gi].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_u64().map(|v| v as usize)).collect();
        let types: Vec<String> = ev["steps"][si]["groups"][gi]["types"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
        let plugins = g["plugins"].as_array().cloned().unwrap_or_default();
        let pictures = plugins.iter().any(|x| !s(&x["imagePath"]).is_empty());
        if pictures && one {
            let mut r = Row::new().spacing(10.0);
            for (pi, pl) in plugins.iter().enumerate().take(4) {
                let on = sel.contains(&pi);
                let tp = types.get(pi).cloned().unwrap_or_default();
                let img: E<'static, Msg> = {
                    let ip = s(&pl["imagePath"]);
                    if !ip.is_empty() && std::path::Path::new(&ip).is_file() {
                        iced::widget::image(iced::widget::image::Handle::from_path(ip)).width(Length::Fill).height(Length::Fixed(150.0)).content_fit(iced::ContentFit::Cover).into()
                    } else {
                        banner(&s(&pl["name"]), 0.0, 150.0, 8.0)
                    }
                };
                let mut col = Column::new().push(img).spacing(6.0);
                if tp == "Recommended" {
                    col = col.push(k::tag(&p, "Suggested", Tag::Ok));
                }
                col = col.push(row![tb(if on { "◉" } else { "○" }, 14.0, if on { p.accent } else { p.muted }), tb(s(&pl["name"]), 15.0, p.text)].spacing(6.0)).push(k::para(p.muted, ui::fit(&s(&pl["description"]), 90), 11.0));
                let usable = tp != "NotUsable";
                r = r.push(k::click(container(col).padding(8.0).width(Length::FillPortion(1)).style(ui::boxed(p.panel2, p.r(10.0), Some((if on { p.accent } else { p.line }, if on { 2.0 } else { 1.0 })))), usable.then_some(Msg::FomodPick(si, gi, pi))));
            }
            body = body.push(r);
        } else {
            for (pi, pl) in plugins.iter().enumerate() {
                let on = sel.contains(&pi);
                let tp = types.get(pi).cloned().unwrap_or_default();
                let usable = tp != "NotUsable";
                let state: E<'static, Msg> = if !usable { k::tag(&p, "Not usable", Tag::Dk) } else if tp == "Recommended" || tp == "Required" { k::tag(&p, "Found", Tag::Ok) } else { none() };
                let words = if usable { ui::fit(&s(&pl["description"]), 70) } else { format!("greyed: {}", ui::fit(&s(&pl["description"]), 60)) };
                let r = row![k::check::<Msg>(&p, on, "", Msg::FomodPick(si, gi, pi)), column![t(s(&pl["name"]), 13.0, if usable { p.text } else { p.muted }), k::mu(&p, words, 11.0)].width(Length::Fill), state].spacing(8.0).align_y(iced::Center);
                body = body.push(k::click(r, usable.then_some(Msg::FomodPick(si, gi, pi))));
            }
        }
    }
    let errs: Vec<String> = ev["errors"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
    if !errs.is_empty() {
        body = body.push(k::said(&p, &errs.join(" ")));
    }
    body = body.push(box_note(&p, "Picture from the installer", "every choice shows the author's own picture and words, big enough for the TV", k::BLUE));
    let last = si + 1 >= steps.len();
    let nav = row![k::chip(&p, "‹ Back", Ch::Plain, Sz::Lg, (si > 0).then_some(Msg::FomodBack)), k::mu1::<Msg>(&p, "B back · A choose · RB next step", 11.0), k::fill(), k::chip(&p, "Cancel", Ch::Plain, Sz::Lg, Some(Msg::FomodCancel)), k::chip(&p, if last { "Install" } else { "Next ›" }, Ch::Acc, Sz::Lg, Some(Msg::FomodNext))].spacing(10.0).align_y(iced::Center);
    let info = &f["info"];
    let head = container(column![row![k::tag::<Msg>(&p, "Installer (FOMOD) · done in Hearth", Tag::Dk), src_tag(&p, "nexus")].spacing(6.0), row![tb(format!("{} {}", s(&a.fomod["name"]), s(&info["version"])), 26.0, p.text), k::fill(), k::chip(&p, "Mod page", Ch::Plain, Sz::Md, (!s(&info["website"]).is_empty()).then(|| Msg::Open(s(&info["website"])))), k::chip(&p, "Read me", Ch::Plain, Sz::Md, None)].align_y(iced::Center).spacing(8.0), k::mu(&p, format!("by {} · {}", if s(&info["author"]).is_empty() { "its author".into() } else { s(&info["author"]) }, ui::fit(&s(&info["description"]), 70)), 11.0)].spacing(6.0))
        .padding(16.0)
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.panel, k::rgb("#c8803a"), 0.18), p.r(10.0), None));
    k::card(&p, column![head, gap(10.0), row![container(sl).width(Length::Fixed(180.0)), ui::vline(p.line), container(scrollable(body)).width(Length::Fill).height(Length::Fill)].spacing(12.0).height(Length::Fill), gap(8.0), nav])
}

fn before_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let files: Vec<Value> = a.fomod["evaluation"]["files"].as_array().cloned().unwrap_or_default();
    let tile = |l: &str, v: String, c: Color| -> E<'static, Msg> { container(column![k::cap::<Msg>(&p, l.to_string()), tb(v, 20.0, c)].spacing(2.0)).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))).into() };
    let c = column![
        heading(&p, "Before it goes in", k::mu1(&p, "with your choices so far", 11.0)),
        row![tile("Files", commas(files.len() as u64), p.text), tile("New", "—".into(), k::GREEN), tile("Overwrites", "—".into(), k::YELLOW)].spacing(8.0),
        k::cap(&p, "The game folder"),
        kvl(&p, "Game files changed", "none · the layer only", k::GREEN)
    ]
    .spacing(8.0);
    k::card_fit(&p, c)
}

fn where_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let pl: Vec<String> = a.fomod["evaluation"]["files"].as_array().cloned().unwrap_or_default().iter().map(|f| s(&f["destination"])).filter(|d| d.to_lowercase().ends_with(".esp") || d.to_lowercase().ends_with(".esm") || d.to_lowercase().ends_with(".esl")).collect();
    let free = hearth_gaming::wabbajack::free_space(&a.mods_dir());
    k::card_fit(&p, column![k::h2::<Msg>(&p, "Where it goes"), kvl(&p, "In the list", "at the bottom (it wins); move it after", p.muted), kvl(&p, "Plugins added", if pl.is_empty() { "none".into() } else { pl.join(" + ") }, p.muted), kvl(&p, "Load order", "placed by Sort's rules", p.muted), kvl(&p, "Space", free.map(|f| format!("{} free", k::size_words(f))).unwrap_or_default(), p.muted)].spacing(6.0))
}

fn already_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let opt = |i: u8, title: &str, words: &str| -> E<'static, Msg> { k::click(row![tb(if a.if_in == i { "◉" } else { "○" }, 14.0, if a.if_in == i { p.accent } else { p.muted }), column![t(title.to_string(), 13.0, p.text), k::mu(&p, words.to_string(), 11.0)]].spacing(8.0), Some(Msg::IfIn(i))) };
    k::card(&p, column![k::h2::<Msg>(&p, "If the mod is already in"), opt(0, "Install as a new version", "the old one stays, off, until you remove it"), opt(1, "Replace it", "a snapshot is made first"), opt(2, "Merge the files in", ""), gap(6.0), k::cap(&p, "Safe either way"), k::para(p.muted, "A snapshot of the whole setup is made before it goes in. Removing it later takes every file back out; Updates & going back can return to tonight's setup in one step.", 11.0)].spacing(8.0))
}

// ================================================================== board 3: Find mods

pub fn shown_hits(a: &App) -> Vec<sources::Hit> {
    let src = match a.find_src {
        1 => "nexus",
        2 => "thunderstore",
        3 => "curseforge",
        4 => "modio",
        _ => "",
    };
    a.hits.iter().filter(|h| src.is_empty() || h.source == src).filter(|h| !a.find_flags[1] || !h.adult).filter(|h| !a.find_flags[0] || !h.check.starts_with("! needs")).cloned().collect()
}

fn find_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let total: usize = a.counts.iter().filter_map(|c| c.1.as_ref().ok()).sum();
    let search = row![container(row![k::input(&p, "⌕ Search every source", &a.find_q, Msg::FindQ).on_submit(Msg::FindGo), k::mu1::<Msg>(&p, format!("one search across every source · for all {} of your games", a.games.len()), 11.0)].spacing(10.0).align_y(iced::Center)).width(Length::Fill), k::field(&p, "All your games ▾", 170.0, None::<Msg>), k::field(&p, "Most endorsed ▾", 170.0, None::<Msg>)].spacing(10.0).align_y(iced::Center);
    let mut tabs = vec![(format!("All sources · {}", commas(total as u64)), a.find_src == 0, Msg::FindSrc(0))];
    for (i, (id, label)) in sources::SOURCES.iter().enumerate() {
        let c = a.counts.iter().find(|c| c.0 == *id).map(|c| match &c.1 {
            Ok(n) => commas(*n as u64),
            Err(_) => "key".into(),
        });
        tabs.push((format!("{} {}", label, c.unwrap_or_default()), a.find_src == (i as u8 + 1), Msg::FindSrc(i as u8 + 1)));
    }
    let flags = row![
        k::chip(&p, "Works with my mods", if a.find_flags[0] { Ch::Acc } else { Ch::Plain }, Sz::Md, Some(Msg::FindFlag(0))),
        k::chip(&p, "Hide adult mods", if a.find_flags[1] { Ch::Acc } else { Ch::Plain }, Sz::Md, Some(Msg::FindFlag(1))),
        k::chip(&p, "Fits the Ally", if a.find_flags[2] { Ch::Acc } else { Ch::Plain }, Sz::Md, Some(Msg::FindFlag(2)))
    ]
    .spacing(6.0);
    let head = column![search, row![k::tabs(&p, tabs), k::fill(), flags].align_y(iced::Center)].spacing(10.0);
    let hits = shown_hits(a);
    let mut grid = Column::new().spacing(10.0);
    for pair in hits.chunks(2).take(6).enumerate() {
        let mut r = Row::new().spacing(10.0);
        for (j, h) in pair.1.iter().enumerate() {
            let i = pair.0 * 2 + j;
            let on = i == a.find_pick;
            let stat = if h.endorsed > 0 { format!("{} endorsed · {} downloads", commas(h.endorsed), commas(h.downloads)) } else { format!("{} downloads", commas(h.downloads)) };
            let cc = if h.check.starts_with('!') { k::YELLOW } else { k::GREEN };
            let inner = row![banner(&h.game, 110.0, 80.0, 7.0), column![row![src_tag(&p, h.source), t(h.game.clone(), 11.0, p.muted)].spacing(6.0).align_y(iced::Center), tb(ui::fit(&h.name, 40), 14.0, p.text), k::mu(&p, stat, 11.0), t(format!("{} {}", if cc == k::GREEN { "✓" } else { "" }, ui::fit(&h.check, 60)), 11.0, cc)].spacing(3.0)].spacing(12.0).align_y(iced::Center);
            r = r.push(k::click(container(inner).padding(10.0).width(Length::FillPortion(1)).style(ui::boxed(p.panel, p.r(10.0), Some((if on { p.accent } else { p.line }, if on { 2.0 } else { 1.0 })))), Some(Msg::FindPick(i))));
        }
        if pair.1.len() == 1 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
    }
    if hits.is_empty() {
        let mut why = Column::new().spacing(4.0).push(k::mu(&p, if a.find_q.is_empty() { "Type what you're looking for: every source is asked at once, for all your games." } else { "Nothing found yet." }, 12.0));
        for (id, r) in &a.counts {
            if let Err(e) = r {
                why = why.push(row![src_tag(&p, id), k::mu1::<Msg>(&p, e.clone(), 11.0)].spacing(6.0));
            }
        }
        grid = grid.push(why);
    }
    // whole sets: Wabbajack's gallery (Nexus Collections, Thunderstore packs and CurseForge modpacks show with their source when found)
    let lists = a.sets["lists"].as_array().or(a.sets.as_array()).cloned().unwrap_or_default();
    let mut sets = Row::new().spacing(10.0);
    for (i, l) in lists.iter().take(4).enumerate() {
        let title = s(&l["title"]);
        let card = column![
            stack![banner(&s(&l["game"]), 0.0, 100.0, 8.0), container(k::tag::<Msg>(&p, "Wabbajack list", Tag::Dk)).padding(8.0)],
            tb(ui::fit(&title, 26), 15.0, p.text),
            k::mu(&p, format!("{} · {}", s(&l["game"]), s(&l["size"])), 11.0),
            k::chip(&p, "Install the list", Ch::Acc, Sz::Md, Some(Msg::SetInstall(i)))
        ]
        .spacing(5.0);
        sets = sets.push(container(card).padding(8.0).width(Length::FillPortion(1)).style(ui::boxed(p.panel2, p.r(10.0), Some((p.line, 1.0)))));
    }
    let sets = k::card(&p, column![row![tb("Whole sets, installed in one go", 15.0, p.text), k::mu1::<Msg>(&p, "collections, modlists and modpacks · each goes into a new profile, your others stay as they are", 11.0), k::fill(), k::click(t("All sets ›", 11.0, p.accent), Some(Msg::Go(Page::Modlists)))].spacing(10.0).align_y(iced::Center), gap(8.0), sets]);
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("search", head.into()), ("results", scrollable(grid).height(Length::Fill).into()), ("sets", sets), ("picked", picked_card(a, &hits))];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "find",
        items,
        |ps| container(column![ps.take("search"), row![column![container(ps.take("results")).height(Length::FillPortion(3)), container(ps.take("sets")).height(Length::FillPortion(2))].spacing(12.0).width(Length::FillPortion(8)), container(ps.take("picked")).width(Length::FillPortion(4))].spacing(14.0).height(Length::Fill)].spacing(12.0)).padding(k::pad(14.0, 16.0, 8.0, 16.0)).into(),
        Msg::Describe,
    )
}

fn picked_card(a: &App, hits: &[sources::Hit]) -> E<'static, Msg> {
    let p = a.p;
    let Some(h) = hits.get(a.find_pick).cloned() else {
        return k::card(&p, column![k::h2::<Msg>(&p, "The picked mod"), k::mu(&p, "Pick a result: what it needs, whether it works with your mods, and which file.", 12.0)].spacing(8.0));
    };
    let stat = |l: &str, v: String| -> E<'static, Msg> { column![k::cap::<Msg>(&p, l.to_string()), tb(v, 16.0, p.text)].spacing(1.0).into() };
    let top = stack![banner(&h.game, 0.0, 160.0, 10.0), container(column![row![src_tag(&p, h.source), k::tag::<Msg>(&p, h.game.clone(), Tag::Dk)].spacing(6.0), tb(ui::fit(&h.name, 30), 24.0, Color::WHITE)].spacing(6.0)).padding(16.0).align_y(iced::alignment::Vertical::Bottom).height(Length::Fixed(160.0))];
    let mut c = Column::new().spacing(8.0).push(top).push(row![stat("Endorsed", commas(h.endorsed)), stat("Downloads", commas(h.downloads)), stat("Version", h.version.clone()), stat("Updated", h.updated.clone())].spacing(18.0)).push(k::para(p.muted, ui::fit(&h.summary, 200), 11.0));
    c = c.push(k::cap(&p, "Needs · pulled in by themselves"));
    if h.needs.is_empty() {
        c = c.push(k::mu(&p, "Nothing else.", 11.0));
    }
    for nd in &h.needs {
        c = c.push(kvl(&p, nd.clone(), "you have it", p.muted));
    }
    c = c.push(k::cap(&p, "Works with your other mods?"));
    let total = a.mods().iter().filter(|m| m["kind"] == "mod").count();
    c = c.push(if h.check.starts_with('!') { box_note(&p, "Not yet", h.check.clone(), k::YELLOW) } else { box_note(&p, format!("Yes, checked against your {total} mods"), "No missing masters. Clashes with nothing you have on.", k::GREEN) });
    let same = hits.iter().filter(|x| x.name != h.name && x.game == h.game && x.name.split_whitespace().last() == h.name.split_whitespace().last()).map(|x| x.name.clone()).next();
    if let Some(o) = same {
        c = c.push(box_note(&p, "One to choose", format!("{o} (in results) does the same job: have one or the other."), k::YELLOW));
    }
    if !h.files.is_empty() {
        c = c.push(k::cap(&p, "Which file")).push(k::seg(&p, h.files.iter().enumerate().take(4).map(|(i, f)| (ui::fit(&f.0, 22), i == a.find_file, Msg::FindFile(i))).collect()));
    }
    let install_words = if h.needs.is_empty() { "Install".to_string() } else { format!("Install · {} more mod{} with it", h.needs.len(), if h.needs.len() == 1 { "" } else { "s" }) };
    c = c.push(k::fill()).push(row![k::chip(&p, install_words, Ch::Acc, Sz::Lg, Some(Msg::FindInstall)), k::chip(&p, "Endorse", Ch::Plain, Sz::Lg, (h.source == "nexus").then_some(Msg::Endorse)), k::chip(&p, "Page", Ch::Plain, Sz::Lg, Some(Msg::Open(h.page.clone())))].spacing(8.0));
    c = c.push(k::mu(&p, "Free account: one click on \"Mod Manager Download\" · Premium: straight in", 11.0));
    k::card(&p, scrollable(c).height(Length::Fill))
}

// ================================================================== board 4: Updates and going back

pub fn snap_name(s: &logic::Snap) -> String {
    if !s.name.is_empty() {
        s.name.clone()
    } else {
        format!("{}'s setup", hearth_ui::time::when(s.at).split(',').next().unwrap_or(""))
    }
}

fn updates_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let checked = a.updates["checked"].as_i64().map(|t| format!("checked {}", hearth_ui::time::when(t))).unwrap_or_else(|| "not checked yet".into());
    let banner_ = container(row![k::dot(k::GREEN, 8.0), tb("Nothing updates by itself.", 13.0, p.text), t("Hearth checks every mod's page each night and tells you; you pick what goes in, and a snapshot is made first.", 12.0, p.muted), k::fill(), k::mu1::<Msg>(&p, checked, 11.0), k::chip(&p, "Check now", Ch::Plain, Sz::Md, Some(Msg::CheckNow))].spacing(10.0).align_y(iced::Center))
        .padding(12.0)
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.panel, k::GREEN, 0.08), p.r(10.0), Some((Pal::mix(p.line, k::GREEN, 0.4), 1.0))));
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("banner", banner_.into()), ("updates", updates_card(a)), ("snapshots", snaps_card(a)), ("saves", saves_card(a)), ("how", how_card(a))];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "updates",
        items,
        |ps| container(column![ps.take("banner"), row![container(ps.take("updates")).width(Length::FillPortion(4)), container(ps.take("snapshots")).width(Length::FillPortion(4)), column![ps.take("saves"), container(ps.take("how")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(4))].spacing(14.0).height(Length::Fill)].spacing(12.0)).padding(k::pad(14.0, 16.0, 8.0, 16.0)).into(),
        Msg::Describe,
    )
}

fn updates_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let all: Vec<Value> = a.updates["updates"].as_array().cloned().unwrap_or_default();
    let game = a.game().map(|g| g.title.clone()).unwrap_or_default();
    let pinned: Vec<String> = a.settings["pinned"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
    let this: Vec<&Value> = all.iter().filter(|u| s(&u["game"]) == game).collect();
    let shown: Vec<Value> = match a.upd_tab {
        0 => this.iter().map(|x| (*x).clone()).collect(),
        2 => Vec::new(),
        _ => all.clone(),
    };
    let picked = all.iter().filter(|u| u["picked"] == true).count();
    let mut c = Column::new().spacing(6.0).push(row![k::h2::<Msg>(&p, format!("Updates · {}", all.len())), k::gap(8.0, 1.0), k::tabs(&p, vec![(format!("This game {}", this.len()), a.upd_tab == 0, Msg::UpdTab(0)), (format!("All games {}", all.len()), a.upd_tab == 1, Msg::UpdTab(1)), (format!("Pinned {}", pinned.len()), a.upd_tab == 2, Msg::UpdTab(2))]), k::fill(), k::chip(&p, format!("Update {picked} picked"), Ch::Acc, Sz::Sm, (picked > 0).then_some(Msg::UpdateApply))].spacing(6.0).align_y(iced::Center));
    let pick = if a.upd_pick.is_empty() { shown.first().map(|u| s(&u["id"])).unwrap_or_default() } else { a.upd_pick.clone() };
    for u in &shown {
        let id = s(&u["id"]);
        let tag = match s(&u["tag"]).as_str() {
            "crash" => k::tag::<Msg>(&p, "Fixes your crash", Tag::Bad),
            "new-game" => k::tag::<Msg>(&p, "New game best", Tag::Warn),
            _ => k::tag::<Msg>(&p, "Safe", Tag::Ok),
        };
        let other = if s(&u["game"]) != game { format!(" · {}", s(&u["game"])) } else { String::new() };
        let size = u["size"].as_u64().map(|b| format!(" · {}", k::size_words(b))).unwrap_or_default();
        let r = row![k::check::<Msg>(&p, u["picked"] == true, "", Msg::UpdTick(id.clone())), column![row![tb(s(&u["mod"]), 13.0, p.text), k::mu1::<Msg>(&p, other, 11.0)], row![t(format!("{} → ", s(&u["from"])), 11.0, p.muted), t(s(&u["to"]), 11.0, k::GREEN), k::mu1::<Msg>(&p, format!("{size} · {}", s(&u["date"]).chars().take(10).collect::<String>()), 11.0)]].width(Length::Fill), src_tag(&p, &s(&u["source"])), tag].spacing(8.0).align_y(iced::Center);
        let on = id == pick;
        c = c.push(k::click(container(r).padding(7.0).width(Length::Fill).style(ui::boxed(if on { Pal::mix(p.panel, p.accent, 0.12) } else { Color::TRANSPARENT }, p.r(8.0), None)), Some(Msg::UpdPick(id.clone()))));
        if on {
            let mut log = Column::new().spacing(3.0);
            for l in u["changelog"].as_array().cloned().unwrap_or_default().iter().take(5) {
                log = log.push(t(format!("• {}", s(l)), 12.0, p.text));
            }
            if u["changelog"].as_array().map(|x| x.is_empty()).unwrap_or(true) {
                log = log.push(k::mu(&p, "Its author wrote no changelog for this version.", 11.0));
            }
            c = c.push(k::cap(&p, "What changed (its author's changelog)")).push(container(log).padding(10.0).width(Length::Fill).style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0)))));
        }
    }
    if shown.is_empty() {
        c = c.push(k::mu(&p, if a.upd_tab == 2 { "Nothing pinned." } else { "Nothing new on the mods' pages. Check now asks again." }, 12.0));
    }
    for gone in a.updates["gone"].as_array().cloned().unwrap_or_default().iter().take(1) {
        c = c.push(k::cap(&p, "Gone from its site")).push(box_note(&p, format!("{} was taken down", s(&gone["mod"])), "Your copy stays on the home server, so this setup and its snapshots still work.", k::BLUE));
    }
    let d = back::read_own("devices.json");
    let devs = d["devices"].as_array().cloned().unwrap_or_default();
    if !devs.is_empty() {
        c = c.push(k::cap(&p, "Same updates on your other devices"));
        for x in devs.iter().take(3) {
            c = c.push(kvl(&p, format!("{} · {}", s(&x["name"]), s(&x["profile"])), "gets them when you pick them here", p.muted));
        }
    }
    if !pinned.is_empty() {
        c = c.push(row![k::mu1::<Msg>(&p, format!("Pinned and never offered: {}", pinned.join(", ")), 11.0), k::click(t(" · Change", 11.0, p.accent), Some(Msg::UpdTab(2)))]);
    }
    k::card(&p, scrollable(c).height(Length::Fill))
}

fn snaps_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(4.0).push(row![k::h2::<Msg>(&p, "Snapshots"), k::mu1::<Msg>(&p, "the whole setup, before every change", 11.0), k::fill(), k::click(t("+ Take one now", 11.0, p.accent), Some(Msg::TakeSnap))].spacing(8.0).align_y(iced::Center));
    let snaps = &a.detail.snaps;
    for (i, sn) in snaps.iter().enumerate().take(8) {
        let on = i == a.snap_pick;
        let dot = container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(if i == 0 { k::GREEN } else if on { p.accent } else { p.bg }, 6.0, Some((if on || i == 0 { Color::TRANSPARENT } else { p.muted }, 1.5))));
        let when = hearth_ui::time::when(sn.at);
        let title = if sn.name.is_empty() { when } else { format!("{when} · {}", sn.name) };
        let words = format!("{} · {} mods · {}", sn.why, sn.mods, sn.profile);
        let r = row![dot, column![tb(title, 13.0, p.text), k::mu(&p, words, 11.0)].spacing(1.0)].spacing(12.0).align_y(iced::Center);
        c = c.push(k::click(container(r).padding(8.0).width(Length::Fill).style(ui::boxed(if on { Pal::mix(p.panel, p.accent, 0.12) } else { Color::TRANSPARENT }, p.r(8.0), on.then_some((Pal::mix(p.line, p.accent, 0.5), 1.0)))), Some(Msg::SnapPick(i))));
    }
    if snaps.is_empty() {
        c = c.push(k::mu(&p, if a.detail.organizer { "No snapshots yet: one is made before every change, or + Take one now." } else { "Snapshots are kept for games with a Hearth Mod Organizer instance." }, 12.0));
    }
    let mut go: Option<String> = None;
    if let Some(sn) = snaps.get(a.snap_pick).filter(|_| a.snap_pick > 0) {
        let name = snap_name(sn);
        let mut d = Column::new().spacing(4.0).push(k::cap(&p, format!("Going back to {name} changes")));
        for (l, r) in logic::diff(sn, &a.profile_dir(), &a.mods_dir()) {
            d = d.push(kvl(&p, l, r, p.muted));
        }
        let since = a.detail.saves.iter().filter(|x| x.2 as i64 > sn.at).count();
        d = d.push(kvl(&p, "Saves made since", format!("{since}, kept"), p.muted));
        c = c.push(gap(6.0)).push(container(d).padding(12.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(9.0), Some((p.line, 1.0)))));
        go = Some(name);
    }
    c = c.push(k::fill());
    if let Some(name) = go {
        c = c.push(container(k::chip(&p, format!("↶ Go back to {name}"), Ch::Acc, Sz::Lg, Some(Msg::GoBack))).center_x(Length::Fill)).push(container(k::mu(&p, "Tonight's setup becomes a snapshot too, so you can come forward again", 11.0)).center_x(Length::Fill));
    }
    k::card(&p, scrollable(c).height(Length::Fill))
}

fn saves_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let on: Vec<String> = a.plugins().iter().filter(|x| x["active"] == true).map(|x| s(&x["name"]).to_lowercase()).collect();
    let mut c = Column::new().spacing(6.0).push(heading(&p, "Saves that need a mod", k::mu1(&p, format!("{} · {}", a.profile(), a.detail.saves.len()), 11.0))).push(k::para(p.muted, "Each save remembers the mods it was made with. Going back or turning a mod off shows which saves it would break.", 11.0));
    let mut needing = None;
    for (_, sv, mt) in a.detail.saves.iter().take(4) {
        let missing: Vec<&String> = sv.plugins.iter().filter(|x| !on.contains(&x.to_lowercase())).collect();
        let tag = if let Some(m) = missing.first() {
            needing = Some(m.trim_end_matches(".esp").trim_end_matches(".esm").to_string());
            k::tag::<Msg>(&p, format!("Needs {}", m.trim_end_matches(".esp").trim_end_matches(".esm")), Tag::Warn)
        } else {
            k::tag::<Msg>(&p, "Works now", Tag::Ok)
        };
        c = c.push(row![banner(&sv.place, 60.0, 34.0, 5.0), column![tb(sv.place.clone(), 12.0, p.text), k::mu(&p, format!("{} · level {}", hearth_ui::time::when(*mt as i64), sv.level), 10.0)].width(Length::Fill), tag].spacing(10.0).align_y(iced::Center));
    }
    if a.detail.saves.is_empty() {
        c = c.push(k::mu(&p, "No saves in this profile yet.", 11.0));
    }
    if let Some(m) = needing {
        c = c.push(box_note(&p, format!("Loading a marked save without {m}: Hearth asks first"), format!("and offers to turn {m} on for that start only."), k::YELLOW));
    }
    k::card_fit(&p, c)
}

fn how_card(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let every = match a.settings["checkUpdates"].as_str().unwrap_or("night") {
        "never" => 0,
        "open" => 2,
        _ => 1,
    };
    let sw = |label: &str, key: &'static str, d: bool| -> E<'static, Msg> {
        let on = a.setting(key, d);
        row![t(label.to_string(), 12.0, p.text), k::fill(), k::switch(&p, on, Some(Msg::Setting(key, !on)))].align_y(iced::Center).into()
    };
    let mut c = Column::new().spacing(8.0).push(k::h2::<Msg>(&p, "How updates work"));
    c = c.push(row![t("Check for updates", 12.0, p.text), k::fill(), k::seg(&p, vec![("Never".into(), every == 0, Msg::CheckEvery(0)), ("Every night".into(), every == 1, Msg::CheckEvery(1)), ("When I open it".into(), every == 2, Msg::CheckEvery(2))])].align_y(iced::Center));
    c = c.push(sw("Install updates by themselves", "autoInstall", false)).push(sw("Tell me on the Ally and the TV", "tellDevices", true));
    c = c.push(kvl(&p, "Keep snapshots", "60 days · up to 20 GB", p.muted)).push(sw("Old versions kept on the home server", "keepOld", true));
    c = c.push(k::para(p.muted, "Snapshots are tiny: they note what's on and where; mod files are kept once.", 11.0));
    c = c.push(k::cap(&p, "Or go back one mod only"));
    let mods = a.mods();
    for m in mods.iter().filter(|m| m["kind"] == "mod").filter(|m| a.updates["updates"].as_array().map(|u| u.iter().any(|x| x["mod"] == m["name"])).unwrap_or(false)).take(2) {
        c = c.push(row![tb(s(&m["name"]), 12.0, p.text), k::mu1::<Msg>(&p, " · versions kept", 11.0), k::fill(), k::seg(&p, vec![(format!("{} now", s(&m["version"])), true, Msg::Tick)])].align_y(iced::Center));
    }
    k::card(&p, scrollable(c).height(Length::Fill))
}

// ================================================================== board 5: Crashes

fn crash_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let Some((file, c, at)) = a.detail.crashes.get(a.crash_pick).cloned() else {
        return container(column![k::h1::<Msg>(&p, "No crashes"), k::para(p.muted, "When a game crashes, its crash log (Crash Logger SSE / F4SE, .NET Script Framework) is read here in plain words, with what probably did it and fixes you can try in one tap. Settings › When a game crashes can get Crash Logger for the games that need it.", 13.0)].spacing(10.0).max_width(640.0)).padding(40.0).into();
    };
    let mods_dir = a.mods_dir();
    let suspects: Vec<(String, String)> = c.suspects.iter().map(|m| (m.clone(), logic::mod_with_file(&mods_dir, m).unwrap_or_else(|| m.trim_end_matches(".dll").to_string()))).collect();
    let first = suspects.first().map(|x| x.1.clone()).unwrap_or_else(|| "the game itself".into());
    let ups = a.updates["updates"].as_array().cloned().unwrap_or_default();
    let fix = ups.iter().find(|u| s(&u["mod"]).eq_ignore_ascii_case(&first)).cloned();
    let objects = c.objects.first().cloned().unwrap_or_default();
    let place = if objects.to_lowercase().contains("menu") { format!("{}'s menu", first) } else { first.clone() };
    let title = match &fix {
        Some(u) => format!("Crashed in {place}: update {} to {}", s(&u["mod"]), s(&u["to"])),
        None => format!("Crashed in {place}"),
    };
    let ver = a.mods().iter().find(|m| s(&m["name"]).eq_ignore_ascii_case(&first)).map(|m| s(&m["version"])).unwrap_or_default();
    let mut words = format!("{} {} while {}. The crash log points at {first} {ver}", a.game().map(|g| g.short()).unwrap_or_default(), logic::exception_words(&c.exception), if objects.is_empty() { "it ran".into() } else { format!("{} was open", objects) });
    if let Some(u) = &fix {
        words.push_str(&format!(", and its own changelog for {} is out.", s(&u["to"])));
    } else {
        words.push('.');
    }
    let head = container(column![
        row![k::tag::<Msg>(&p, format!("Crashed {}", hearth_ui::time::when(at as i64)), Tag::Bad), k::tag::<Msg>(&p, a.profile(), Tag::Dk), k::tag::<Msg>(&p, "Crash log read", Tag::Dk)].spacing(6.0),
        row![column![tb(title, 28.0, p.text), k::para(p.muted, words, 13.0)].spacing(6.0).width(Length::Fill), k::chip(&p, if fix.is_some() { "Update and play" } else { "Test start without it" }, Ch::Acc, Sz::Lg, Some(Msg::FixStep(if fix.is_some() { 1 } else { 2 }))), k::chip(&p, "Send logs", Ch::Plain, Sz::Lg, Some(Msg::SendLogs))].spacing(10.0).align_y(iced::Center)
    ]
    .spacing(8.0))
    .padding(16.0)
    .width(Length::Fill)
    .style(ui::boxed(Pal::mix(p.panel, k::RED, 0.12), p.r(12.0), Some((Pal::mix(p.line, k::RED, 0.4), 1.0))));
    let step = |i: u8, title: String, words: &str, btn: &str| -> E<'static, Msg> {
        let r = row![container(tb(i.to_string(), 11.0, if i == 1 { p.ink } else { p.text })).center_x(Length::Fixed(24.0)).center_y(Length::Fixed(24.0)).style(ui::boxed(if i == 1 { p.accent } else { p.panel2 }, 12.0, None)), column![tb(title, 13.0, p.text), k::mu(&p, words.to_string(), 11.0)].width(Length::Fill), k::chip(&p, btn.to_string(), if i == 1 { Ch::Acc } else { Ch::Plain }, Sz::Sm, Some(Msg::FixStep(i)))].spacing(10.0).align_y(iced::Center);
        container(r).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(9.0), Some((if i == 1 { p.accent } else { p.line }, 1.0)))).into()
    };
    let last_good = a.detail.snaps.iter().find(|x| !x.crashed && x.at < at as i64).map(snap_name).unwrap_or_else(|| "the last setup".into());
    let fixc = k::card_fit(
        &p,
        column![
            heading(&p, "Fix it in one tap", k::mu1(&p, "a snapshot comes first", 11.0)),
            step(1, fix.as_ref().map(|u| format!("Update {} to {}", s(&u["mod"]), s(&u["to"]))).unwrap_or_else(|| format!("Look for an update to {first}")), "fixes this crash, says its changelog", "Do it"),
            step(2, format!("Turn {first} off for one test start"), "to the main menu and your last save, then back on", "Try"),
            step(3, format!("Go back to {last_good}"), "the last one that never crashed", "Go back"),
            step(4, "Find it by halves".into(), "turns half of tonight's changes off each start until it's found", "Start")
        ]
        .spacing(8.0),
    );
    let mut which = Column::new().spacing(6.0).push(k::h2::<Msg>(&p, "Which mod probably did it"));
    for (i, (_, m)) in suspects.iter().enumerate().take(3) {
        let (w, frac, col) = match i {
            0 => ("very likely", 0.85, k::RED),
            1 => ("maybe", 0.45, k::YELLOW),
            _ => ("unlikely", 0.1, p.muted),
        };
        let why = if i == 0 { "named on the crash line".to_string() } else { "in the call stack".to_string() };
        which = which.push(row![tb(m.clone(), 13.0, p.text), k::fill(), t(w, 12.0, col)]).push(k::bar(&p, frac, 5.0, Length::Fill)).push(k::mu(&p, why, 11.0));
    }
    if suspects.is_empty() {
        which = which.push(k::mu(&p, "The log names no mod's code: it stopped in the game's own. Find it by halves narrows it down.", 11.0));
    }
    let which = k::card(&p, which);
    // the log
    let mut lines = Column::new().spacing(2.0);
    let shown: Vec<(usize, String)> = c.lines.iter().cloned().enumerate().take(90).collect();
    for (i, l) in shown {
        let mk = c.marked.contains(&i);
        let col = if l.contains("Unhandled") { k::RED } else if mk { k::YELLOW } else { k::rgb("#cfc5bc") };
        lines = lines.push(container(t(l, 11.0, col).font(MONO).wrapping(Wrapping::None)).width(Length::Fill).style(ui::boxed(if mk { Pal::mix(p.bg, k::YELLOW, 0.12) } else { Color::TRANSPARENT }, 2.0, None)));
    }
    let what = box_note(&p, "What happened", format!("{} {} inside {}.", a.game().map(|g| g.short()).unwrap_or_default(), logic::exception_words(&c.exception), if c.at.is_empty() { first.clone() } else { c.at.clone() }), p.muted);
    let logc = k::card(
        &p,
        column![
            row![k::h2::<Msg>(&p, "The crash log"), k::gap(8.0, 1.0), k::tabs(&p, vec![("Crash Logger's log".into(), a.crash_tab == 0, Msg::CrashTab(0)), ("Proton's log".into(), a.crash_tab == 1, Msg::CrashTab(1)), ("What ran".into(), a.crash_tab == 2, Msg::CrashTab(2))]), k::fill(), k::chip(&p, "Copy", Ch::Plain, Sz::Sm, Some(Msg::CopyLog))].spacing(6.0).align_y(iced::Center),
            what,
            container(scrollable(lines).height(Length::Fill).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::default(), horizontal: scrollable::Scrollbar::default() })).padding(10.0).height(Length::Fill).width(Length::Fill).style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0)))),
            k::mu(&p, format!("Read from the log {} writes · lines that matter are marked · {}", if c.logger.is_empty() { "the crash logger" } else { "Crash Logger" }, file.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()), 11.0)
        ]
        .spacing(8.0),
    );
    let mut lately = Column::new().spacing(6.0).push(heading(&p, "Crashes lately", k::mu1(&p, format!("{}", a.detail.crashes.len()), 11.0)));
    for (i, (_, cc, t2)) in a.detail.crashes.iter().enumerate().take(4) {
        let m = cc.suspects.first().cloned().unwrap_or_else(|| "the game".into());
        lately = lately.push(k::click(row![k::dot(if i == 0 { k::RED } else { p.muted }, 7.0), column![tb(format!("{} · {}", hearth_ui::time::when(*t2 as i64), m), 12.0, p.text), k::mu(&p, a.game().map(|g| g.short()).unwrap_or_default(), 10.0)].width(Length::Fill), k::tag::<Msg>(&p, if i == 0 { "Fix ready" } else { "Seen" }, if i == 0 { Tag::Warn } else { Tag::Ok })].spacing(8.0).align_y(iced::Center), Some(Msg::CrashPick(i))));
    }
    let lately = k::card_fit(&p, lately);
    let send = k::card_fit(&p, column![k::h2::<Msg>(&p, "Send logs"), k::para(p.muted, "Sends this crash log, Proton's log, the mod list and load order, so it can be looked at. Nothing else from the PC.", 11.0), k::input(&p, "Note: what you were doing", &a.note, Msg::Note), k::chip(&p, "Send logs", Ch::Acc, Sz::Lg, Some(Msg::SendLogs))].spacing(8.0));
    let sw = |label: &str, key: &'static str, d: bool| -> E<'static, Msg> {
        let on = a.setting(key, d);
        row![t(label.to_string(), 12.0, p.text), k::fill(), k::switch(&p, on, Some(Msg::Setting(key, !on)))].align_y(iced::Center).into()
    };
    let when = k::card(&p, column![k::h2::<Msg>(&p, "When a game crashes"), sw("Read the crash log and tell me", "readCrash", true), sw("Show it in Game Mode too", "crashGameMode", true), sw("Get Crash Logger for games that need it", "getCrashLogger", true), sw("Fix things by itself", "autoFix", false), k::mu(&p, "Off: every fix waits for you", 11.0), k::cap(&p, "After a fix"), k::para(p.muted, "A test start: to the main menu, loads your last save, opens the menu it crashed in, 60 seconds. Passes → you're told it's fixed.", 11.0)].spacing(8.0));
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("head", head.into()), ("fix", fixc), ("which", which), ("log", logc), ("lately", lately), ("send", send), ("when", when)];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "crashes",
        items,
        |ps| container(column![ps.take("head"), row![column![ps.take("fix"), container(ps.take("which")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(3)), container(ps.take("log")).width(Length::FillPortion(5)), column![ps.take("lately"), ps.take("send"), container(ps.take("when")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(3))].spacing(14.0).height(Length::Fill)].spacing(12.0)).padding(k::pad(14.0, 16.0, 8.0, 16.0)).into(),
        Msg::Describe,
    )
}

/// Board 5's one-tap fixes (a snapshot first).
pub fn fix_step(a: &mut App, i: u8) -> Task<Msg> {
    let Some((_, c, _)) = a.detail.crashes.get(a.crash_pick).cloned() else { return Task::none() };
    let mods_dir = a.mods_dir();
    let first = c.suspects.first().and_then(|m| logic::mod_with_file(&mods_dir, m));
    match i {
        1 => {
            if let Some(m) = &first {
                if let Some(u) = a.updates["updates"].as_array().and_then(|x| x.iter().find(|u| s(&u["mod"]).eq_ignore_ascii_case(m))).map(|u| s(&u["id"])) {
                    if let Some(list) = a.updates["updates"].as_array_mut() {
                        for x in list.iter_mut() {
                            x["picked"] = json!(s(&x["id"]) == u);
                        }
                    }
                    return Task::done(Msg::UpdateApply);
                }
            }
            Task::done(Msg::CheckNow)
        }
        2 => {
            let Some(m) = first else {
                a.said = "The log names no mod to turn off.".into();
                return Task::none();
            };
            if !a.snapshot(&format!("before a test start without {m}")) {
                return Task::none();
            }
            a.said = format!("{m} is off for this start; turn it back on in Mods after.");
            Task::batch([Task::done(Msg::Toggle(m, false)), Task::done(Msg::Play)])
        }
        3 => {
            if let Some(i) = a.detail.snaps.iter().position(|x| !x.crashed) {
                a.snap_pick = i;
                a.page = Page::Updates;
            }
            Task::none()
        }
        _ => {
            // by halves: tonight's changes (since the last snapshot) half off, then a start
            let Some(sn) = a.detail.snaps.first().cloned() else {
                a.said = "There's no earlier snapshot to compare with yet.".into();
                return Task::none();
            };
            let then = std::fs::read_to_string(sn.dir.join("modlist.txt")).unwrap_or_default();
            let now_l = std::fs::read_to_string(a.profile_dir().join("modlist.txt")).unwrap_or_default();
            let new: Vec<String> = now_l.lines().filter(|l| l.starts_with('+') && !then.lines().any(|x| x == *l)).map(|l| l[1..].to_string()).collect();
            if new.is_empty() {
                a.said = "Nothing changed since the last snapshot: Go back finds older ones.".into();
                return Task::none();
            }
            if !a.snapshot("before finding it by halves") {
                return Task::none();
            }
            let half: Vec<String> = new.iter().take(new.len().div_ceil(2)).cloned().collect();
            a.said = format!("Half of tonight's changes are off ({}). Start the game: if it still crashes, it's in the other half.", half.join(", "));
            let mut t: Vec<Task<Msg>> = half.into_iter().map(|m| Task::done(Msg::Toggle(m, false))).collect();
            t.push(Task::done(Msg::Play));
            Task::batch(t)
        }
    }
}

// ================================================================== board 6: Describe it

fn describe_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let dev = a.settings["describeFor"].as_str().unwrap_or("the ROG Ally").to_string();
    let mut talk = Column::new().spacing(10.0).push(heading(&p, "Say what you want", k::mu1(&p, format!("for {} · {dev}", a.game().map(|g| g.short()).unwrap_or_default()), 11.0)));
    for x in a.talk["talk"].as_array().cloned().unwrap_or_default() {
        if let Some(y) = x["you"].as_str() {
            talk = talk.push(row![k::fill(), container(t(y.to_string(), 13.0, p.text)).padding(12.0).max_width(420.0).style(ui::boxed(Pal::mix(p.panel2, p.accent, 0.15), p.r(10.0), Some((Pal::mix(p.line, p.accent, 0.4), 1.0))))]);
        } else if let Some(r) = x["ai"].as_str() {
            talk = talk.push(container(column![k::para(p.text, r.to_string(), 13.0), k::mu(&p, "Nothing is installed until you say.", 11.0)].spacing(8.0)).padding(12.0).max_width(440.0).style(ui::boxed(p.panel2, p.r(10.0), Some((p.line, 1.0)))));
        }
    }
    if a.asking {
        talk = talk.push(k::mu(&p, "Hearth AI is thinking (on the Nvidia PC)…", 12.0));
    }
    let chips = row![k::chip(&p, "Lighter, 70 fps", Ch::Plain, Sz::Md, Some(Msg::Suggest("Lighter, 70 fps".into()))), k::chip(&p, "No grass mods", Ch::Plain, Sz::Md, Some(Msg::Suggest("No grass mods".into()))), k::chip(&p, "Only free sources", Ch::Plain, Sz::Md, Some(Msg::Suggest("Only free sources".into())))].spacing(6.0);
    let say = k::card(
        &p,
        column![
            scrollable(talk).height(Length::Fill),
            chips,
            k::chip(&p, "Explain the order", Ch::Plain, Sz::Md, Some(Msg::Suggest("Explain the order".into()))),
            k::cap(&p, "You can also ask"),
            k::mu(&p, "\"Why does Skyrim stutter in Riften?\"", 11.0),
            k::mu(&p, "\"Make Fallout 4 run on the Ally\"", 11.0),
            k::mu(&p, "\"Give my Valheim the mods my server uses\"", 11.0),
            row![container(k::input(&p, "Tell it more…", &a.ask, Msg::AskText).on_submit(Msg::Ask)).width(Length::Fill), k::chip(&p, "Speak", Ch::Plain, Sz::Md, Some(Msg::Speak)), k::chip(&p, "Send", Ch::Acc, Sz::Md, Some(Msg::Ask))].spacing(6.0).align_y(iced::Center),
            k::mu(&p, "Hearth AI on the Nvidia PC · reads mod pages, your setup and the Ally's numbers · nothing leaves home", 11.0)
        ]
        .spacing(8.0),
    );
    let set = &a.talk["set"];
    let groups = set["groups"].as_array().cloned().unwrap_or_default();
    let count: usize = groups.iter().map(|g| g["mods"].as_array().map(|m| m.len()).unwrap_or(0)).sum();
    let mut sc = Column::new().spacing(4.0).push(heading(&p, format!("The proposed set · {count} mods"), row![k::tag::<Msg>(&p, "Not installed", Tag::Warn), k::mu1::<Msg>(&p, "in this order · lower wins", 11.0)].spacing(8.0).into()));
    sc = sc.push(row![container(k::cap::<Msg>(&p, "#  Mod")).width(Length::FillPortion(6)), container(k::cap::<Msg>(&p, "From")).width(Length::FillPortion(2)), container(k::cap::<Msg>(&p, "Ally fps")).width(Length::FillPortion(1)), container(k::cap::<Msg>(&p, "Size")).width(Length::FillPortion(1))].spacing(6.0));
    let mut i = 0;
    let mut fps = 0i64;
    for g in &groups {
        sc = sc.push(t(s(&g["name"]).to_uppercase(), 11.0, p.accent));
        for m in g["mods"].as_array().cloned().unwrap_or_default() {
            i += 1;
            let f = m["fps"].as_i64().unwrap_or(0);
            fps += f;
            sc = sc.push(row![container(t(format!("{i:>2}  {}", s(&m["name"])), 12.0, p.text).wrapping(Wrapping::None)).width(Length::FillPortion(6)), container(src_tag(&p, &s(&m["source"]))).width(Length::FillPortion(2)), container(t(if f > 0 { format!("+{f}") } else if f < 0 { format!("−{}", -f) } else { "0".into() }, 12.0, if f < 0 { k::RED } else { k::GREEN })).width(Length::FillPortion(1)), container(t(s(&m["size"]), 11.0, p.muted)).width(Length::FillPortion(1))].spacing(6.0).align_y(iced::Center));
        }
    }
    if groups.is_empty() {
        sc = sc.push(k::mu(&p, "Say what you want on the left: Hearth AI proposes a set of mods, in their order, with what each costs on the device. Nothing is installed until you say.", 12.0));
    } else {
        let mut why = Column::new().spacing(3.0).push(k::cap(&p, "Why this order"));
        for w in set["why"].as_array().cloned().unwrap_or_default() {
            why = why.push(t(s(&w), 12.0, p.text));
        }
        sc = sc.push(gap(6.0)).push(container(why).padding(12.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(9.0), Some((p.line, 1.0)))));
        sc = sc.push(k::fill()).push(row![k::mu1::<Msg>(&p, format!("All in: {fps} fps · {count} mods"), 11.0), k::fill(), k::click(t("Every file it changes ›", 11.0, p.accent), None)]);
    }
    let set_card = k::card(&p, scrollable(sc).height(Length::Fill));
    let m = back::read_own("measured.json");
    let mut ally = Column::new().spacing(5.0).push(heading(&p, format!("On {}", dev.trim_start_matches("the ")), k::mu1(&p, s(&m["settings"]), 11.0)));
    for pl in m["places"].as_array().cloned().unwrap_or_default() {
        let f = pl["fps"].as_f64().unwrap_or(0.0);
        ally = ally.push(row![container(t(s(&pl["name"]), 12.0, p.text)).width(Length::Fixed(110.0)), k::bar(&p, (f / 90.0) as f32, 8.0, Length::Fill), tb(format!("{f:.0} fps"), 12.0, p.text)].spacing(8.0).align_y(iced::Center));
    }
    if m["places"].as_array().map(|x| x.is_empty()).unwrap_or(true) {
        ally = ally.push(k::mu(&p, "Not measured on the device yet: Measure it first starts the game there with the set and counts fps in a few places.", 11.0));
    }
    ally = ally.push(k::chip(&p, format!("Measure it on {} first", dev.trim_start_matches("the ")), Ch::Plain, Sz::Md, None));
    let ally = k::card_fit(&p, ally);
    let looks = k::card_fit(&p, column![heading(&p, "How it looks", k::mu1(&p, "from the mods' pages", 11.0)), row![stack![banner(&a.game().map(|g| g.title.clone()).unwrap_or_default(), 0.0, 130.0, 8.0), container(k::tag::<Msg>(&p, "Now", Tag::Dk)).padding(8.0)], stack![banner("valheim", 0.0, 130.0, 8.0), container(k::tag::<Msg>(&p, "With the set", Tag::Dk)).padding(8.0)]].spacing(8.0)].spacing(8.0));
    let profile = s(&set["profile"]);
    let go = k::card(
        &p,
        column![
            box_note(&p, "Where Hearth AI runs", "On the Nvidia PC (Colibri). From the Ally it asks the PC over Hearth Link; with the PC off, Describe it waits.", p.muted),
            box_note(&p, "Nothing installed until you say.", format!("It goes into a new profile{}. A snapshot comes first.", if profile.is_empty() { String::new() } else { format!(", \"{profile}\"") }), k::GREEN),
            k::fill(),
            k::chip(&p, format!("Install the {count} mods"), Ch::Acc, Sz::Lg, (count > 0).then_some(Msg::InstallSet)),
            row![k::chip(&p, "Change something", Ch::Plain, Sz::Lg, Some(Msg::Suggest("Change something: ".into()))), k::chip(&p, "Save for later", Ch::Plain, Sz::Lg, Some(Msg::SaveLater))].spacing(8.0)
        ]
        .spacing(10.0),
    );
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("say", say), ("set", set_card), ("ally", ally), ("looks", looks), ("go", go)];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "describe",
        items,
        |ps| container(row![container(ps.take("say")).width(Length::FillPortion(4)), container(ps.take("set")).width(Length::FillPortion(5)), column![ps.take("ally"), ps.take("looks"), container(ps.take("go")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(4))].spacing(14.0)).padding(k::pad(14.0, 16.0, 8.0, 16.0)).into(),
        Msg::Describe,
    )
}

/// Install the set: a snapshot, a new profile from this one, then each mod found on Nexus Mods queued (nothing
/// before this press).
pub fn install_set(a: &mut App) -> Task<Msg> {
    let profile = { let x = s(&a.talk["set"]["profile"]); if x.is_empty() { "Described".to_string() } else { x } };
    if !a.snapshot("before Describe it's set went in") {
        return Task::none();
    }
    let from = a.profile();
    let names: Vec<String> = a.talk["set"]["groups"].as_array().cloned().unwrap_or_default().iter().flat_map(|g| g["mods"].as_array().cloned().unwrap_or_default()).map(|m| s(&m["name"])).collect();
    a.find_q = names.first().cloned().unwrap_or_default();
    a.said = format!("A new profile \"{profile}\" is made from {from}; Find mods looks up each of the {} mods to download.", names.len());
    let t1 = a_org(a, &["profile-create", &profile, "--from", &from]);
    Task::batch([t1, Task::done(Msg::Go(Page::Find)), Task::done(Msg::FindGo)])
}

fn a_org(a: &App, args: &[&str]) -> Task<Msg> {
    let Some(id) = a.inst_id() else { return Task::none() };
    let mut v = vec![id];
    v.extend(args.iter().map(|x| x.to_string()));
    Task::perform(back::organizer(v), Msg::Said)
}

// ================================================================== board 8: Every game

fn every_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let hidden: Vec<String> = a.settings["hidden"].as_array().cloned().unwrap_or_default().iter().map(s).collect();
    let games: Vec<&back::Game> = a.games.iter().filter(|g| !hidden.contains(&g.key)).collect();
    let modded: Vec<&&back::Game> = games.iter().filter(|g| g.mods > 0 || g.inst.is_some() || g.beacon).collect();
    let total: usize = modded.iter().map(|g| g.mods).sum();
    let insts = a.instances["instances"].as_array().map(|x| x.len()).unwrap_or(0);
    let head = row![column![k::h1::<Msg>(&p, "Every game, one place"), k::mu(&p, format!("{} games · {} mods · {insts} organizer instances · each game modded the way its players do it", modded.len(), commas(total as u64)), 12.0)].spacing(4.0), k::fill(), k::chip(&p, "Check every game", Ch::Plain, Sz::Lg, Some(Msg::CheckAll)), k::chip(&p, "+ Add a game", Ch::Acc, Sz::Lg, Some(Msg::Go(Page::Organizer)))].spacing(10.0).align_y(iced::Center);
    let ups = a.updates["updates"].as_array().cloned().unwrap_or_default();
    let mut grid = Column::new().spacing(12.0);
    for chunk in modded.chunks(4).take(3) {
        let mut r = Row::new().spacing(12.0);
        for g in chunk {
            let u = ups.iter().filter(|x| s(&x["game"]) == g.title).count();
            let (state, kind) = if u > 0 { ("Updates", Tag::Bl) } else if g.dot == 1 { ("Needs a look", Tag::Warn) } else { ("Healthy", Tag::Ok) };
            let health = if u > 0 { (k::BLUE, format!("{u} updates")) } else if g.dot == 1 { (k::YELLOW, "needs a look".to_string()) } else { (k::GREEN, "healthy".to_string()) };
            let top = stack![banner(&g.title, 0.0, 100.0, 10.0), container(column![k::tag::<Msg>(&p, state, kind), k::fill(), tb(ui::fit(&g.title, 26), 16.0, Color::WHITE)]).padding(12.0).height(Length::Fixed(100.0))];
            let devs = row![k::tag::<Msg>(&p, "PC", Tag::Dk)].spacing(4.0);
            let card = column![
                top,
                container(column![row![tb(format!("{} mods", g.mods), 16.0, p.text), k::fill(), devs].align_y(iced::Center), row![k::dot(health.0, 7.0), t(health.1, 11.0, p.muted)].spacing(6.0).align_y(iced::Center), k::mu(&p, g.how(), 11.0), row![k::chip(&p, "▶ Play modded", Ch::Acc, Sz::Md, Some(Msg::PickGame(g.key.clone()))), k::chip(&p, "Mods", Ch::Plain, Sz::Md, Some(Msg::PickGame(g.key.clone())))].spacing(6.0)].spacing(5.0)).padding(12.0)
            ];
            let on = g.key == a.pick;
            r = r.push(container(card).width(Length::FillPortion(1)).style(ui::boxed(p.panel, p.r(12.0), Some((if on { p.accent } else { p.line }, if on { 2.0 } else { 1.0 })))));
        }
        for _ in chunk.len()..4 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(r);
    }
    if modded.is_empty() {
        grid = grid.push(k::mu(&p, if a.loading { "Looking for your games…" } else { "No modded games yet. Found by themselves (below) lists the moddable games on this PC." }, 13.0));
    }
    let mut found = Column::new().spacing(6.0).push(heading(&p, "Found by themselves", k::mu1(&p, "moddable games on your devices", 11.0)));
    for g in games.iter().filter(|g| g.mods == 0 && g.inst.is_none() && !g.beacon).take(4) {
        let lib = &g.lib;
        let kind = hearth_gaming::organizer::kinds::KINDS.iter().find(|k2| Some(k2.steam) == lib["steamApp"].as_u64()).map(|k2| format!("works like {}'s", k2.names[0])).unwrap_or_else(|| "mods into its folder".into());
        found = found.push(row![thumb(&g.title, 30.0), column![tb(g.title.clone(), 12.0, p.text), k::mu(&p, format!("{} · {kind}", s(&lib["source"])), 10.0)].width(Length::Fill), k::chip(&p, "Set up mods", Ch::Plain, Sz::Sm, Some(Msg::SetUp(g.key.clone())))].spacing(8.0).align_y(iced::Center));
    }
    found = found.push(row![k::mu1::<Msg>(&p, "Scans Steam, Heroic, Lutris, Hearth's library and Beacon each start ·", 11.0), k::click(t(" Not a game? Hide it", 11.0, p.accent), games.iter().find(|g| g.mods == 0).map(|g| Msg::HideGame(g.key.clone())))]);
    found = found.push(container(row![hearth_oss_kit_a::mark::<Msg>("MM", k::rgb("#ffb36b"), k::rgb("#ff7a2e"), 18.0), tb("Updates in the Hearth Store", 12.0, p.text), k::mu1::<Msg>(&p, "· Mod Manager is its own Store app · up to date", 11.0)].spacing(8.0).align_y(iced::Center)).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))));
    let found = k::card(&p, found);
    let d = back::read_own("devices.json");
    let mut dev = Column::new().spacing(4.0).push(heading(&p, "On your other devices", k::mu1(&p, "same mods, each download once", 11.0)));
    dev = dev.push(row![container(k::cap::<Msg>(&p, "Device")).width(Length::FillPortion(2)), container(k::cap::<Msg>(&p, "Games modded")).width(Length::FillPortion(3)), container(k::cap::<Msg>(&p, "Profile")).width(Length::FillPortion(2)), container(k::cap::<Msg>(&p, "State")).width(Length::FillPortion(1))].spacing(6.0));
    let here = s(&d["here"]);
    dev = dev.push(row![container(tb(format!("{} · here", if here.is_empty() { "This PC".into() } else { here }), 12.0, p.text)).width(Length::FillPortion(2)), container(t(format!("all {}", modded.len()), 12.0, p.text)).width(Length::FillPortion(3)), container(t(a.profile(), 12.0, p.muted)).width(Length::FillPortion(2)), container(k::dot(k::GREEN, 7.0)).width(Length::FillPortion(1))].spacing(6.0));
    for x in d["devices"].as_array().cloned().unwrap_or_default() {
        let st = s(&x["state"]);
        dev = dev.push(row![container(tb(s(&x["name"]), 12.0, p.text)).width(Length::FillPortion(2)), container(t(s(&x["games"]), 12.0, p.text)).width(Length::FillPortion(3)), container(t(s(&x["profile"]), 12.0, p.muted)).width(Length::FillPortion(2)), container(row![k::dot(if st == "away" { p.muted } else if st.is_empty() { k::GREEN } else { k::YELLOW }, 7.0), t(st, 11.0, p.muted)].spacing(4.0)).width(Length::FillPortion(1))].spacing(6.0));
    }
    dev = dev.push(gap(6.0)).push(row![t("Send a game's mods to a device", 12.0, p.text), k::fill(), k::chip(&p, "Choose…", Ch::Plain, Sz::Sm, None)].align_y(iced::Center)).push(k::mu(&p, "Saves made with mods go along with the profile · settings stay each device's own", 11.0));
    let dev = k::card(&p, dev);
    let srv = sources::home_server();
    let kept = srv.as_ref().map(|d| k::folder_size(d)).unwrap_or(0);
    let tile = |l: &str, v: String, sub: &str, c: Color| -> E<'static, Msg> { container(column![k::cap::<Msg>(&p, l.to_string()), tb(v, 22.0, c), k::mu(&p, sub.to_string(), 11.0)].spacing(2.0)).padding(10.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))).into() };
    let saved = back::read_own("server.json")["savedThisMonth"].as_u64().unwrap_or(0);
    let server = k::card_fit(&p, column![k::h2::<Msg>(&p, "Kept on your home server"), row![tile("Mod files", if srv.is_some() { k::size_words(kept) } else { "—".into() }, "every version kept", p.text), tile("Saved this month", k::size_words(saved), "not downloaded twice", k::GREEN)].spacing(8.0), k::mu(&p, if srv.is_some() { "Taken-down mods stay here · in Backups each night" } else { "Set up Hearth's home server and each mod file is downloaded once for every device." }, 11.0)].spacing(8.0));
    let has = |id: &str| a.keys.iter().find(|x| x.0 == id).map(|x| x.1).unwrap_or(false);
    let acct = |src: &str, words: String, ok: bool| -> E<'static, Msg> { row![src_tag(&p, src), t(words, 12.0, p.text), k::fill(), k::dot(if ok { k::GREEN } else { k::YELLOW }, 8.0)].spacing(8.0).align_y(iced::Center).into() };
    let nexus_name = hearth_gaming::nexus::account().map(|x| format!("{} · {}", x.name, if x.premium { "premium" } else { "free" })).unwrap_or_else(|| "not signed in".into());
    let accounts = k::card_fit(
        &p,
        column![
            heading(&p, "Accounts", k::click(t("Manage", 11.0, p.accent), Some(Msg::Go(Page::Accounts)))),
            acct("nexus", nexus_name, has("nexus")),
            acct("modio", if has("modio") { "key kept in the keyring".into() } else { "no key yet".into() }, has("modio")),
            acct("curseforge", if has("curseforge") { "a key from the keyring".into() } else { "Hearth's own app key (applying)".into() }, has("curseforge")),
            acct("thunderstore", "no account needed".into(), true),
            k::mu(&p, "and LoversLab, MEGA, Google for modlists", 11.0)
        ]
        .spacing(8.0),
    );
    let items: Vec<(&'static str, E<'_, Msg>)> = vec![("head", head.into()), ("cards", scrollable(grid).height(Length::Fill).into()), ("found", found), ("devices", dev), ("server", server), ("accounts", accounts)];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "every",
        items,
        |ps| container(column![ps.take("head"), container(ps.take("cards")).height(Length::FillPortion(5)), row![container(ps.take("found")).width(Length::FillPortion(4)), container(ps.take("devices")).width(Length::FillPortion(5)), column![ps.take("server"), container(ps.take("accounts")).height(Length::Fill)].spacing(12.0).width(Length::FillPortion(3))].spacing(14.0).height(Length::FillPortion(3))].spacing(14.0)).padding(k::pad(14.0, 16.0, 8.0, 16.0)).into(),
        Msg::Describe,
    )
}

// ================================================================== Hearth Mods' other pages

fn modlists_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(8.0).push(k::h1::<Msg>(&p, "Modlists & collections")).push(k::mu(&p, "Wabbajack lists through jackify-engine (GPL-3); each installs into a new instance. Nexus Collections and Thunderstore packs install the same way, into a new profile.", 12.0));
    for j in a.modlists["jobs"].as_array().cloned().unwrap_or_default() {
        c = c.push(kvl(&p, s(&j["title"]), s(&j["state"]), p.accent));
    }
    let lists = a.modlists["gallery"]["lists"].as_array().or(a.modlists["gallery"].as_array()).cloned().unwrap_or_default();
    for (i, l) in lists.iter().enumerate().take(40) {
        c = c.push(row![thumb(&s(&l["game"]), 28.0), column![tb(s(&l["title"]), 13.0, p.text), k::mu(&p, format!("{} · {}", s(&l["game"]), s(&l["author"])), 11.0)].width(Length::Fill), k::chip(&p, "Install the list", Ch::Plain, Sz::Sm, Some(Msg::SetInstall(i)))].spacing(10.0).align_y(iced::Center));
    }
    if lists.is_empty() {
        c = c.push(k::mu(&p, "The gallery comes from the Wabbajack engine (Settings › Get the Wabbajack engine).", 12.0));
    }
    container(scrollable(c)).padding(20.0).into()
}

fn organizer_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(8.0).push(k::h1::<Msg>(&p, "Organizer instances")).push(k::mu(&p, "Hearth Mod Organizer: works like Mod Organizer 2 and reads and writes its files, so an instance opens in both.", 12.0));
    for i in a.instances["instances"].as_array().cloned().unwrap_or_default() {
        let key = if s(&i["libraryId"]).is_empty() { format!("instance:{}", s(&i["id"])) } else { s(&i["libraryId"]) };
        c = c.push(row![thumb(&s(&i["title"]), 28.0), column![tb(s(&i["title"]), 13.0, p.text), k::mu(&p, format!("{} mods · {} on · {}", n(&i["mods"]), n(&i["modsOn"]), s(&i["dir"])), 11.0)].width(Length::Fill), k::chip(&p, "Open", Ch::Plain, Sz::Sm, Some(Msg::PickGame(key)))].spacing(10.0).align_y(iced::Center));
    }
    c = c.push(gap(10.0)).push(k::cap(&p, "Make one for a game"));
    for g in a.games.iter().filter(|g| g.inst.is_none() && !g.beacon).take(12) {
        c = c.push(row![t(g.title.clone(), 12.0, p.text), k::fill(), k::chip(&p, "Set up mods", Ch::Plain, Sz::Sm, Some(Msg::SetUp(g.key.clone())))].align_y(iced::Center));
    }
    container(scrollable(c)).padding(20.0).into()
}

fn accounts_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let has = |id: &str| a.keys.iter().find(|x| x.0 == id).map(|x| x.1).unwrap_or(false);
    let mut c = Column::new().spacing(12.0).push(k::h1::<Msg>(&p, "Accounts"));
    let rows: [(&'static str, &str, &str); 4] = [
        ("nexus", "Nexus Mods", "Your own personal API key (Nexus › Settings › API keys), kept readable by you only, as Hearth Mods always did."),
        ("modio", "mod.io", "mod.io's public API: your API key from mod.io › My account › API access, kept in the keyring."),
        ("curseforge", "CurseForge", "CurseForge's API needs a key Hearth applies for (console.curseforge.com). Until it comes, a key you type here is kept in the keyring and used."),
        ("thunderstore", "Thunderstore", "No account needed."),
    ];
    for (id, name, words) in rows {
        let mut r = Column::new().spacing(6.0).push(row![src_tag(&p, id), tb(name.to_string(), 14.0, p.text), k::fill(), k::tag::<Msg>(&p, if has(id) { "Ready" } else { "Not yet" }, if has(id) { Tag::Ok } else { Tag::Warn })].spacing(8.0).align_y(iced::Center)).push(k::para(p.muted, words.to_string(), 12.0));
        if id != "thunderstore" {
            if a.key_for == id {
                r = r.push(row![container(hearth_oss::w::secret_input(&p, "Paste the key", &a.key_text, Msg::KeyText, Msg::SaveKey)).width(Length::Fill), k::chip(&p, "Keep it", Ch::Acc, Sz::Md, Some(Msg::SaveKey))].spacing(8.0));
            } else {
                r = r.push(k::chip(&p, if has(id) { "Change the key" } else { "Type the key" }, Ch::Plain, Sz::Md, Some(Msg::KeyFor(id))));
            }
        }
        c = c.push(k::card_fit(&p, r));
    }
    c = c.push(k::mu(&p, "LoversLab, MEGA and Google for Wabbajack lists: as before (docs/MODS.md › Accounts).", 12.0));
    container(scrollable(c).height(Length::Fill)).padding(20.0).max_width(900.0).into()
}

fn settings_page(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let sw = |label: &str, key: &'static str, d: bool| -> E<'static, Msg> {
        let on = a.setting(key, d);
        row![t(label.to_string(), 13.0, p.text), k::fill(), k::switch(&p, on, Some(Msg::Setting(key, !on)))].align_y(iced::Center).into()
    };
    let c = column![
        k::h1::<Msg>(&p, "Settings"),
        k::card_fit(&p, column![k::h2::<Msg>(&p, "Mods"), sw("Show adult mods in Find mods", "adult", false), sw("Tell me on the Ally and the TV", "tellDevices", true), sw("Old versions kept on the home server", "keepOld", true)].spacing(10.0)),
        k::card_fit(&p, column![k::h2::<Msg>(&p, "When a game crashes"), sw("Read the crash log and tell me", "readCrash", true), sw("Get Crash Logger for games that need it", "getCrashLogger", true), sw("Fix things by itself", "autoFix", false)].spacing(10.0)),
        k::card_fit(&p, column![k::h2::<Msg>(&p, "Wabbajack"), k::para(p.muted, "jackify-engine (GPL-3) installs Wabbajack lists. Hearth OS has it built in; otherwise Modlists & collections fetches it.", 12.0)].spacing(8.0)),
    ]
    .spacing(12.0);
    container(scrollable(c)).padding(20.0).max_width(900.0).into()
}

// ================================================================== board 7: the controller view (Game Mode, TV, Ally)

pub fn tv(a: &App) -> iced::Element<'_, Msg> {
    let p = a.p;
    let g = a.game().cloned().unwrap_or_default();
    let tabs: Vec<(String, bool, Msg)> = vec![
        ("Mods".into(), a.page == Page::Game, Msg::Go(Page::Game)),
        ("Plugins".into(), a.tab == 1 && a.page == Page::Game, Msg::Tab(1)),
        ("Find".into(), a.page == Page::Find, Msg::Go(Page::Find)),
        (format!("Updates {}", a.updates["updates"].as_array().map(|u| u.len()).unwrap_or(0)), a.page == Page::Updates, Msg::Go(Page::Updates)),
        (format!("Crashes {}", a.detail.crashes.len()), a.page == Page::Crashes, Msg::Go(Page::Crashes)),
        ("Describe it".into(), a.page == Page::Describe, Msg::Go(Page::Describe)),
    ];
    let pill = |s2: &str| -> E<'static, Msg> { container(tb(s2.to_string(), 12.0, p.bg)).padding(k::pad(4.0, 8.0, 4.0, 8.0)).style(ui::boxed(p.text, 6.0, None)).into() };
    let mut tabrow = Row::new().spacing(18.0).align_y(iced::Center).push(pill("LB"));
    for (l, on, m) in tabs {
        tabrow = tabrow.push(k::click(container(t(l, 18.0, if on { p.ink } else { p.muted })).padding(k::pad(8.0, 16.0, 8.0, 16.0)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 10.0, None)), Some(m)));
    }
    tabrow = tabrow.push(pill("RB"));
    let head = row![hearth_oss_kit_a::mark::<Msg>("MM", k::rgb("#ffb36b"), k::rgb("#ff7a2e"), 34.0), column![tb(g.title.clone(), 30.0, p.text), t(format!("Mods · {} · {} mods · on the TV", a.profile(), g.mods), 14.0, p.muted)], k::fill(), tabrow].spacing(14.0).align_y(iced::Center);
    let body: E<'_, Msg> = if a.page == Page::Game || a.page == Page::Every {
        ladder(a, true)
    } else {
        page(a)
    };
    let (_, fix, _) = health(a);
    let (hv, _, _) = health(a);
    let mut hc = Column::new().spacing(8.0).push(k::cap(&p, format!("Health · {fix} to fix")));
    for (c, title, _, _) in hv.iter().take(3) {
        hc = hc.push(row![k::dot(*c, 10.0), t(title.clone(), 16.0, p.text)].spacing(10.0).align_y(iced::Center));
    }
    hc = hc.push(row![pill("Y"), t("Fix both", 14.0, p.text)].spacing(8.0).align_y(iced::Center));
    let play = container(column![k::cap::<Msg>(&p, "Ready to play"), tb("Play modded", 26.0, p.text), k::para(p.muted, "Starts with the script extender · the mods are a layer over the game while it runs", 13.0), k::click(container(tb("▶  Play modded", 18.0, p.ink)).center_x(Length::Fill).center_y(Length::Fixed(52.0)).style(ui::boxed(p.accent, 12.0, None)), Some(Msg::Play))].spacing(8.0)).padding(18.0).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.08), 14.0, Some((p.accent, 1.5))));
    let picked = a.mods().iter().find(|m| m["id"].as_str() == Some(a.picked.as_str())).cloned().unwrap_or(Value::Null);
    let moving = container(column![
        k::cap::<Msg>(&p, format!("Moving {}", s(&picked["name"]))),
        t(format!("Now at {} · wins {} files · loses {}", n(&picked["priority"]), commas(n(&picked["conflicts"]["winning"])), commas(n(&picked["conflicts"]["losing"]))), 16.0, p.text),
        t(format!("beats {}", picked["conflicts"]["winsOver"].as_array().map(|x| x.iter().take(2).map(s).collect::<Vec<_>>().join(", ")).unwrap_or_default()), 14.0, k::GREEN),
        t(format!("loses to {}", picked["conflicts"]["losesTo"].as_array().map(|x| x.iter().take(2).map(s).collect::<Vec<_>>().join(", ")).unwrap_or_default()), 14.0, k::RED),
        k::fill(),
        k::mu(&p, "↑ ↓ to move · A to drop · B puts it back · the conflict lens follows as it moves", 12.0),
        row![k::mu1::<Msg>(&p, "Made of blocks · hold Y to", 12.0), k::click(t(" Edit layout", 12.0, p.accent), Some(Msg::Describe(hearth_ui::arrange::edit_layout())))]
    ]
    .spacing(8.0))
    .padding(18.0)
    .height(Length::Fill)
    .style(ui::boxed(p.panel, 14.0, Some((p.line, 1.0))));
    let mid = column![play, container(hc).padding(18.0).width(Length::Fill).style(ui::boxed(p.panel, 14.0, Some((p.line, 1.0)))), moving].spacing(14.0).width(Length::Fixed(460.0));
    let hint = |b: &str, w: &str| -> E<'static, Msg> { row![container(tb(b.to_string(), 13.0, p.bg)).center_x(Length::Fixed(28.0)).center_y(Length::Fixed(28.0)).style(ui::boxed(if b == "A" { p.accent } else { p.text }, 14.0, None)), t(w.to_string(), 15.0, p.text)].spacing(8.0).align_y(iced::Center).into() };
    let lifted = a.lifted.is_some();
    let hints = row![hint("A", if lifted { "Drop here" } else { "Pick up" }), hint("B", if lifted { "Put it back" } else { "Back" }), hint("X", "On / off"), hint("Y", "More"), hint("≡", "Play modded"), hint("⧉", "Profiles")].spacing(26.0);
    let qa = back::read_own("devices.json");
    let side = column![
        k::cap::<Msg>(&p, "The same on the ROG Ally"),
        tb("Same lists, sized for the screen", 20.0, p.text),
        k::para(p.muted, format!("Mods, plugins, Find, Updates and Crashes all work with the buttons. Its own profile ({}) and the same mod files from the home server.", qa["devices"][0]["profile"].as_str().unwrap_or("Vanilla+")), 13.0),
        gap(10.0),
        container(column![k::cap::<Msg>(&p, "Quick Access on the Ally · Mods"), downloads_mini(a)].spacing(8.0)).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))),
        container(column![k::cap::<Msg>(&p, "Typing, when it's needed"), k::para(p.text, "Find mods and Describe it use Hearth's on-screen keyboard, or your voice. Nothing asks for a file path in Game Mode.", 13.0), kvl(&p, "Adult mods", if a.setting("adult", false) { "shown" } else { "hidden unless shown" }, p.muted), kvl(&p, "Desktop's Mods window", "⋯ › Open on the desktop", p.muted)].spacing(8.0)).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))),
        container(column![k::cap::<Msg>(&p, "In Steam's Game Mode too"), k::para(p.text, format!("\"{} (Modded)\" sits in the Steam library with its art (Add to Steam, through Hearth Sync). Play starts it with the mods.", g.title), 13.0)].spacing(8.0)).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))),
    ]
    .spacing(14.0)
    .width(Length::Fixed(460.0));
    let main = column![head, gap(16.0), row![container(body).width(Length::Fill).height(Length::Fill), mid].spacing(24.0).height(Length::Fill), gap(10.0), hints].padding(k::pad(30.0, 40.0, 20.0, 40.0));
    let all = row![container(main).width(Length::Fill), container(side).padding(30.0).height(Length::Fill).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.2), 0.0, None))];
    container(all).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into()
}

fn downloads_mini(a: &App) -> E<'static, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    let mut any = false;
    for i in a.downloads["items"].as_array().cloned().unwrap_or_default().into_iter().filter(|i| i["state"] != "done").take(3) {
        let frac = n(&i["done"]) as f32 / n(&i["size"]).max(1) as f32;
        any = true;
        c = c.push(kvl(&p, s(&i["name"]), format!("{}% · {}", (frac * 100.0) as u32, if sources::home_server().is_some() { "from the home server" } else { "downloading" }), p.muted)).push(k::bar(&p, frac, 5.0, Length::Fill));
    }
    for u in a.updates["updates"].as_array().cloned().unwrap_or_default().iter().take(1) {
        any = true;
        c = c.push(row![t(format!("{} {}", s(&u["mod"]), s(&u["to"])), 13.0, p.text), k::fill(), t("Update", 13.0, p.accent)]);
    }
    if !a.detail.crashes.is_empty() {
        any = true;
        c = c.push(kvl(&p, "Crash on the PC", "fix ready", p.muted));
    }
    if !any {
        c = c.push(k::mu(&p, "Nothing downloading.", 12.0));
    }
    c.into()
}

/// The controller (board 7): LB / RB pages, ↑↓ through the list, A picks a mod up and drops it, B puts it back,
/// X on/off, Y more, Menu Play modded, View profiles.
pub fn pad(a: &mut App, b: pad::Button) -> Task<Msg> {
    use pad::Button as B;
    const ORDER: &[Page] = &[Page::Game, Page::Find, Page::Updates, Page::Crashes, Page::Describe, Page::Every];
    let at = ORDER.iter().position(|x| *x == a.page).unwrap_or(0);
    let shown: Vec<Value> = shown_mods(a).into_iter().filter(|m| m["kind"] == "mod").collect();
    let i = shown.iter().position(|m| m["id"].as_str() == Some(a.picked.as_str())).unwrap_or(0);
    match b {
        B::Lb => Task::done(Msg::Go(ORDER[(at + ORDER.len() - 1) % ORDER.len()])),
        B::Rb => Task::done(Msg::Go(ORDER[(at + 1) % ORDER.len()])),
        B::Up | B::Down if a.page == Page::Game => {
            let d: i64 = if b == B::Up { -1 } else { 1 };
            if a.lifted.is_some() {
                return Task::done(Msg::MoveBy(d));
            }
            let j = (i as i64 + d).clamp(0, shown.len().saturating_sub(1) as i64) as usize;
            shown.get(j).map(|m| Task::done(Msg::Pick(s(&m["id"])))).unwrap_or(Task::none())
        }
        B::A if a.page == Page::Game => {
            if a.lifted.is_some() {
                a.lifted = None;
                a.said = "Dropped. A snapshot was made before it moved.".into();
                let _ = a.snapshot("before a mod moved (controller)");
                Task::done(Msg::Reload)
            } else {
                Task::done(Msg::Lift(a.picked.clone()))
            }
        }
        B::B => {
            if a.lifted.is_some() {
                Task::done(Msg::PutBack)
            } else {
                Task::done(Msg::Go(Page::Every))
            }
        }
        B::X if a.page == Page::Game => {
            let on = shown.get(i).map(|m| m["enabled"] == true).unwrap_or(false);
            Task::done(Msg::Toggle(a.picked.clone(), !on))
        }
        B::Y => Task::done(Msg::FixMasters),
        B::Menu => Task::done(Msg::Play),
        B::View => {
            let next = a.detail.s["profiles"].as_array().and_then(|x| x.iter().find(|p| p["active"] != true)).map(|p| s(&p["name"]));
            next.map(|n2| Task::done(Msg::Profile(n2))).unwrap_or(Task::none())
        }
        _ => Task::none(),
    }
}

/// One of Mod Manager's own panes (its blocks window).
pub fn studio_pane<'a>(a: &'a App, which: &str) -> E<'a, Msg> {
    match which {
        "ladder" => ladder(a, false),
        "health" => health_card(a),
        _ => updates_card(a),
    }
}

#[allow(dead_code)]
fn unused() -> i64 {
    now()
}
