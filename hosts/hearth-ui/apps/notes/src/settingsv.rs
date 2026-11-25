//! Settings (page 8): GitHub (the PC's own Git sign-in, never a password or token; the repository; commits), Sync (by
//! itself, warn about secrets), Editor (font, size, how notes open), Looks and Today (theme following Hearth's looks,
//! today's log with its "day starts at" time and the optional trackers), Graph modes, About. Each one is saved the
//! moment it's changed, with a backup (settings_file.rs); nothing here changes by itself.

use crate::look::{self, accent2, bx, click, f, pad, tone, z, E, MONO};
use crate::{App, Msg, Page};
use hearth_notes::git::Auth;
use hearth_notes::prefs::day_start_words;
use hearth_ui::ui::{gap, t, t1, tb, tm};
use hearth_ui::Pal;
use iced::widget::{column, container, row, scrollable, text, Column, Space};
use iced::{Color, Length, Task};
use serde_json::json;

pub const SCROLL: &str = "notes-settings";
const SECTIONS: [&str; 7] = ["GitHub", "Sync", "Editor", "Looks", "Today", "Graph modes", "About"];
/// Where each section starts on the page (for the sidebar's links).
const AT: [f32; 7] = [0.0, 240.0, 380.0, 380.0, 470.0, 560.0, 640.0];

pub fn scroll_to(i: usize) -> Task<Msg> {
    iced::widget::operation::scroll_to(SCROLL, iced::widget::operation::AbsoluteOffset { x: None, y: Some(AT.get(i).copied().unwrap_or(0.0)) })
}

pub fn side(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(4.0)).push(container(look::caps(&p, "Settings")).padding(pad(z(4.0), 0.0, z(8.0), z(8.0))));
    for (i, s) in SECTIONS.iter().enumerate() {
        let on = i == a.settings_sec;
        c = c.push(click(container(t1(*s, f(17.0), if on { tn.text } else { Pal::mix(tn.text, tn.muted, 0.15) })).padding(pad(0.0, z(14.0), 0.0, z(14.0))).center_y(Length::Fixed(z(44.0))).width(Length::Fill).style(bx(if on { Pal::mix(tn.side, p.text, 0.07) } else { Color::TRANSPARENT }, 6.0, None)), Some(Msg::SetSec(i))));
    }
    container(c).padding(pad(0.0, z(12.0), 0.0, z(12.0))).into()
}

fn head<'a>(p: &Pal, s: &str) -> E<'a, Msg> {
    container(tm(s.to_string(), f(18.0), Pal::mix(tone(p).text, tone(p).muted, 0.15))).padding(pad(z(14.0), 0.0, z(4.0), 0.0)).into()
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let pr = &a.prefs;
    let sub = |s: String| -> E<'static, Msg> { t(s, f(15.0), tn.muted).into() };
    let monosub = |s: String| -> E<'static, Msg> { text(s).size(f(14.0)).font(MONO).color(tn.muted).into() };
    let title = row![tb("Settings", f(40.0), tn.text), gap(z(20.0), 1.0), t("Saved the moment you change them, with a backup. Nothing here changes by itself.", f(19.0), tn.muted)].align_y(iced::Center);
    // GitHub.
    let (signed_title, signed_sub, chip): (String, String, Option<&str>) = match &a.sync.auth {
        Some(Auth::Gh(u)) => (format!("Signed in as {}", if u.is_empty() { crate::syncv::who(a) } else { u.clone() }), "Uses the sign-in Git and GitHub CLI already have on this PC. Hearth Notes never asks for or stores a password or token.".into(), Some("this PC's Git sign-in")),
        Some(Auth::Helper(_)) | Some(Auth::Ssh) => (format!("Signed in as {}", crate::syncv::who(a)), "Uses the sign-in Git already has on this PC. Hearth Notes never asks for or stores a password or token.".into(), Some("this PC's Git sign-in")),
        Some(Auth::None) => ("Not signed in to GitHub".into(), hearth_notes::git::SIGN_IN_HELP.into(), None),
        None => ("Checking this PC's Git sign-in…".into(), "Hearth Notes never asks for or stores a password or token.".into(), None),
    };
    let mut st_row = row![t(signed_title, f(19.0), tn.text)].spacing(z(12.0)).align_y(iced::Center);
    if let Some(c) = chip {
        st_row = st_row.push(look::chip(&p, c, look::Chip::Ok));
    }
    let gh = row![look::sym(&p, "⎇"), gap(z(18.0), 1.0), column![st_row, sub(signed_sub)].spacing(z(4.0)).width(Length::Fill), look::btn(&p, "Check sign-in", look::B::Normal, false, false, Some(Msg::CheckSignIn))].align_y(iced::Center);
    let gh = container(gh).padding(pad(z(16.0), z(22.0), z(16.0), z(22.0))).width(Length::Fill).style(bx(tn.card, 9.0, None));
    let repo_line = format!("{} · branch {} · {}", if a.sync.repo.remote.is_empty() { "not a Git repository".to_string() } else { a.sync.repo.remote.clone() }, if a.sync.repo.branch.is_empty() { "main" } else { &a.sync.repo.branch }, hearth_notes::paths::tilde(&a.root));
    let repo = look::srow(&p, look::sym(&p, "▤"), "Repository", monosub(repo_line), look::btn(&p, "Change…", look::B::Normal, false, false, Some(Msg::ChangeVault)), false, None);
    let who = crate::syncv::who(a);
    let private = a.sync.who.email.ends_with("users.noreply.github.com");
    let commits_line = if private { format!("As {who}, with his GitHub private address. No extra lines added to messages.") } else { format!("As {who}, with his Git config's name and address. No extra lines added to messages.") };
    let commits = look::srow(&p, look::sym(&p, "✎"), "Commits", sub(commits_line), look::chip(&p, "Fixed", look::Chip::Plain), false, None);
    // Sync.
    let every = pr.sync_every;
    let sync_choices: Vec<(String, bool, Msg)> = hearth_notes::prefs::SYNC_CHOICES.iter().map(|m| (if *m == 0 { "Off".to_string() } else { format!("{m} min") }, every == *m, Msg::Set("syncEvery", json!(m)))).collect();
    let focus_sync = a.focus == "s-sync" || a.settings_sec == 1;
    let auto = look::srow(&p, look::sym(&p, "⟳"), "Sync by itself", sub("Pulls, checks (tools/check.py), then pushes. Never while a game runs or the check fails.".into()), look::seg(&p, sync_choices, false), focus_sync, None);
    let warn = look::srow(&p, look::sym(&p, "⚠"), "Warn if a note looks like a secret", sub("Stops the push if a note has something that looks like a token, password or private key; you decide what happens".into()), look::switch(&p, pr.warn_secrets, false, Some(Msg::Set("warnSecrets", json!(!pr.warn_secrets)))), false, None);
    // Editor.
    let font = look::seg(&p, vec![("Inter".into(), pr.font == "inter", Msg::Set("font", json!("inter"))), ("Noto Serif".into(), pr.font == "serif", Msg::Set("font", json!("serif"))), ("Mono".into(), pr.font == "mono", Msg::Set("font", json!("mono")))], false);
    let font_row = look::srow(&p, look::sym(&p, "Aa"), "Font", sub("Code in DejaVu Sans Mono".into()), font, false, None);
    let size = iced::widget::slider(11.0..=26.0, pr.font_size, |v| Msg::Set("fontSize", json!(v.round()))).step(1.0).width(Length::Fixed(z(290.0))).style(look::slider_style(&p));
    let size_row = look::srow(&p, look::sym(&p, "↕"), "Size", sub(format!("{} px", pr.font_size.round() as u32)), size.into(), false, None);
    let open_in = look::seg(&p, vec![("Live preview".into(), pr.open_in == "live", Msg::Set("openIn", json!("live"))), ("Source".into(), pr.open_in == "source", Msg::Set("openIn", json!("source"))), ("Reading".into(), pr.open_in == "read", Msg::Set("openIn", json!("read")))], false);
    let open_row = look::srow(&p, look::sym(&p, "◧"), "Opens notes in", gap(1.0, 1.0), open_in, false, None);
    // Looks and Today.
    let theme = look::seg(&p, vec![(format!("Hearth (now: {})", a.look_name), pr.theme == "hearth", Msg::Set("theme", json!("hearth"))), ("Gold".into(), pr.theme == "gold", Msg::Set("theme", json!("gold"))), ("Firehold".into(), pr.theme == "firehold", Msg::Set("theme", json!("firehold")))], false);
    let theme_row = look::srow(&p, look::sym(&p, "✦"), "Theme", sub("Follows Hearth's looks".into()), theme, false, None);
    let today_row = look::srow(&p, look::sym(&p, "◷"), "Today's log", monosub("log/YYYY-MM-DD.md · US Central dates".into()), look::chip(&p, "From the workspace rules", look::Chip::Plain), false, None);
    let step = |s: &'static str, d: i32| -> E<'static, Msg> { click(container(t(s, f(15.0), tn.text)).center_x(Length::Fixed(z(34.0))).center_y(Length::Fixed(z(32.0))).style(bx(Color::TRANSPARENT, 6.0, Some((tn.line, 1.0)))), Some(Msg::DayStartStep(d))) };
    let ds = row![step("‹", -1), container(tm(day_start_words(pr.day_start), f(16.0), tn.text)).center_x(Length::Fixed(z(120.0))), step("›", 1)].align_y(iced::Center);
    let ds_row = look::srow(&p, look::sym(&p, "☾"), "Day starts at", sub("Until then, Today is still the day before (for night shifts)".into()), ds.into(), a.focus == "s-day", None);
    let tr_row = look::srow(&p, look::sym(&p, "▮"), "Productivity and habits on Today", sub("Optional; kept on this PC, apart from the log".into()), look::switch(&p, pr.trackers, false, Some(Msg::Set("trackers", json!(!pr.trackers)))), false, None);
    let gm_line = format!("{} now · still when Reduce motion is on", crate::gview::name_of(&a.g.mode));
    let gm_row = look::srow(&p, look::sym(&p, "⏸"), "Graph modes", sub(gm_line), look::btn(&p, "Graph modes ›", look::B::Normal, false, false, Some(Msg::Rail(Page::Modes))), false, None);
    let about = look::srow(&p, look::sym(&p, "?"), "About Hearth Notes", sub(format!("Pure Rust, part of Hearth OS. Your notes are plain Markdown in {}; the app's own files are in {}.", hearth_notes::paths::tilde(&a.root), hearth_notes::paths::tilde(&hearth_notes::paths::data()))), gap(1.0, 1.0), false, None);
    let unreadable = a.prefs.unreadable;
    let items: Vec<(&'static str, E<Msg>)> = vec![
        ("title", title.into()),
        ("github", column![head(&p, "GitHub"), gh, repo, commits].spacing(z(10.0)).into()),
        ("sync", column![head(&p, "Sync"), auto, warn].spacing(z(10.0)).into()),
        ("editor", column![head(&p, "Editor"), font_row, size_row, open_row].spacing(z(10.0)).width(Length::FillPortion(1)).into()),
        ("looks", column![head(&p, "Looks and Today"), theme_row, today_row, ds_row, tr_row, gm_row].spacing(z(10.0)).width(Length::FillPortion(1)).into()),
        ("about", column![head(&p, "About"), about].spacing(z(10.0)).into()),
        ("unreadable", t("Hearth Notes' settings file couldn't be read, so nothing is written over it; it's kept as it was.", f(15.0), look::BAD).into()),
    ];
    let _ = accent2;
    // its blocks (hearth_ui::blocks "settings"): as it was when nothing is customised
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "settings",
        items,
        move |ps| {
            let mut c = column![ps.take("title"), ps.take("github"), ps.take("sync"), row![ps.take("editor"), ps.take("looks")].spacing(z(18.0)), ps.take("about")].spacing(z(10.0));
            if unreadable {
                c = c.push(ps.take("unreadable"));
            }
            scrollable(container(c).padding(pad(z(28.0), z(44.0), z(30.0), z(44.0)))).id(SCROLL).height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

#[allow(dead_code)]
fn spacer() -> E<'static, Msg> {
    Space::new().into()
}
