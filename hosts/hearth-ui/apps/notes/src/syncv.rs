//! Sync (page 5) and Conflict (page 6). Sync: what changed with its diff, the commit message, committing as the PC's own
//! Git name, Sync now, Sync by itself (2/5/10/30 min, as drawn on 5), and the workspace's check with its exact
//! problems and fixes, blocking the push while it isn't tidy. Conflict: one note changed on two PCs, side by side;
//! Keep mine, Keep theirs or Merge both, with both versions always kept as copies.

use crate::look::{self, bx, click, f, pad, tone, z, E, MONO};
use crate::{App, Msg, MSG_ID};
use hearth_notes::check::{Check, Fix};
use hearth_notes::conflict::{self, Pick};
use hearth_notes::day;
use hearth_notes::git::Auth;
use hearth_ui::ui::{gap, glyph, t, t1, tb, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, scrollable, text, text_input, Column, Space};
use iced::{Color, Length};

fn clock_of(ts_: i64) -> String {
    let c = day::central(ts_);
    day::clock(c.hour, c.min)
}

pub fn who(a: &App) -> String {
    match &a.sync.auth {
        Some(Auth::Gh(u)) if !u.is_empty() => u.clone(),
        _ => {
            if a.sync.who.name.is_empty() { "you".into() } else { a.sync.who.name.clone() }
        }
    }
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let s = &a.sync;
    let n = s.changes.len();
    let mut title = row![tb("Sync with GitHub", f(40.0), tn.text), gap(z(20.0), 1.0)].align_y(iced::Center);
    if n > 0 {
        title = title.push(look::chip(&p, format!("{n} change{} to push", crate::plural(n)), look::Chip::Accent));
    } else if s.is_repo {
        title = title.push(look::chip(&p, "Up to date", look::Chip::Ok));
    }
    let branch = if s.repo.branch.is_empty() { "main".to_string() } else { s.repo.branch.clone() };
    let last = if s.last_sync > 0 { format!(" · last synced {}", clock_of(s.last_sync)) } else { String::new() };
    title = title.push(Space::new().width(Length::Fill)).push(t1(format!("{} · {branch}{last}", if s.repo.remote.is_empty() { hearth_notes::paths::tilde(&a.root) } else { s.repo.remote.clone() }), f(15.0), tn.muted));
    if !s.is_repo {
        let body = column![title, look::card(&p, column![tm("This folder isn't a Git repository", f(19.0), tn.text), t("Notes here are saved on this PC only. Pick the hearth-workspace clone in Settings › Repository to sync with GitHub.", f(15.0), tn.muted)].spacing(z(8.0)), false)].spacing(z(24.0));
        return container(body).padding(pad(z(28.0), z(40.0), z(28.0), z(40.0))).into();
    }
    // Changes.
    let mut ch = Column::new().spacing(z(6.0)).push(row![ts("Changes", f(16.0), Pal::mix(tn.text, tn.muted, 0.2)), Space::new().width(Length::Fill), look::link(&p, "Pull only", f(15.0), Msg::SyncPull)].align_y(iced::Center));
    for (i, c) in s.changes.iter().enumerate().take(14) {
        let on = i == s.sel;
        let (lc, lbg) = match c.code {
            'A' => (look::GOOD_TX, look::GOOD_LINE),
            'D' => (look::BAD, Color::from_rgb8(0x7a, 0x33, 0x33)),
            'U' => (look::WARN, Color::from_rgb8(0x8a, 0x6a, 0x1f)),
            _ => (p.accent, Pal::a(p.accent, 0.7)),
        };
        let code = container(t(c.code.to_string(), f(12.0), lc)).center_x(Length::Fixed(z(30.0))).center_y(Length::Fixed(z(24.0))).style(bx(Color::TRANSPARENT, z(12.0), Some((lbg, 1.0))));
        let counts = row![t1(if c.added > 0 { format!("+{}", c.added) } else { String::new() }, f(15.0), look::GOOD), gap(z(10.0), 1.0), t1(if c.removed > 0 || c.code == 'M' { format!("−{}", c.removed) } else { String::new() }, f(15.0), look::BAD)];
        let r = container(row![code, gap(z(18.0), 1.0), text(c.path.clone()).size(f(15.5)).font(MONO).color(tn.text).wrapping(text::Wrapping::None), Space::new().width(Length::Fill), counts].align_y(iced::Center)).padding(pad(z(10.0), z(18.0), z(10.0), z(18.0))).width(Length::Fill).style(bx(if on { Pal::mix(tn.card, tn.text, 0.04) } else { Pal::mix(tn.card, tn.page, 0.3) }, 8.0, if on { Some((p.accent, 2.0)) } else { None }));
        ch = ch.push(click(r, Some(Msg::SyncSel(i))));
    }
    if s.changes.is_empty() {
        ch = ch.push(t("Nothing changed on this PC since the last sync", f(15.0), tn.muted));
    }
    let changes = look::card(&p, ch, false);
    let left = column![changes, check_card(a), commit_card(a)].spacing(z(18.0)).width(Length::FillPortion(1));
    let right = column![diff_card(a), auto_card(a), tidy_card(a)].spacing(z(18.0)).width(Length::FillPortion(1));
    let mut body = column![title].spacing(z(24.0));
    if !s.secrets.is_empty() {
        body = body.push(secrets_card(a));
    }
    body = body.push(row![left, right].spacing(z(26.0)));
    scrollable(container(body).padding(pad(z(28.0), z(40.0), z(28.0), z(40.0)))).height(Length::Fill).into()
}

fn check_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let s = &a.sync;
    let pill: E<Msg> = container(text("tools/check.py").size(f(13.0)).font(MONO).color(Pal::mix(tn.text, tn.muted, 0.2))).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(Color::TRANSPARENT, z(13.0), Some((tn.line, 1.0)))).into();
    let when = if s.checked_at > 0 { format!("Checked {} · ", clock_of(s.checked_at)) } else { String::new() };
    let again = row![t(when, f(14.0), tn.muted), look::link(&p, "Check again", f(14.0), Msg::CheckAgain)];
    match &s.check {
        Some(Check::Untidy(probs)) => {
            let mut c = Column::new().spacing(z(10.0));
            c = c.push(row![look::dot(look::BAD, z(10.0)), gap(z(14.0), 1.0), tb(format!("Workspace check: not tidy, {} problem{}", probs.len(), crate::plural(probs.len())), f(19.0), tn.text), Space::new().width(Length::Fill), pill].align_y(iced::Center));
            c = c.push(t("Nothing is pushed until it's tidy. Your changes are saved on this PC meanwhile.", f(15.0), Pal::mix(tn.text, tn.muted, 0.2)));
            for (i, pr) in probs.iter().enumerate() {
                let label = format!("{}{}{}", pr.path, if pr.path.is_empty() { "" } else { ": " }, pr.what);
                let fix: E<Msg> = match &pr.fix {
                    Some(Fix::Readme(_)) => look::btn(&p, "Make README from template", if i == 0 { look::B::Primary } else { look::B::Normal }, false, false, Some(Msg::Fix(i))),
                    Some(Fix::Index(_)) => look::btn(&p, "Add the INDEX line", if i == 0 { look::B::Primary } else { look::B::Normal }, false, false, Some(Msg::Fix(i))),
                    None => t("Fix it in the workspace", f(13.0), tn.muted).into(),
                };
                c = c.push(container(row![text(label).size(f(14.5)).font(MONO).color(Color::from_rgb8(0xff, 0xb0, 0xa8)).width(Length::Fill), fix].align_y(iced::Center)).padding(pad(z(10.0), z(14.0), z(10.0), z(16.0))).width(Length::Fill).style(bx(Color::from_rgb8(0x2a, 0x1c, 0x1c), 7.0, Some((Color::from_rgb8(0x5a, 0x2e, 0x2e), 1.0)))));
            }
            c = c.push(again);
            container(c).padding(pad(z(18.0), z(20.0), z(18.0), z(20.0))).width(Length::Fill).style(bx(Color::from_rgb8(0x24, 0x18, 0x19), 10.0, Some((Color::from_rgb8(0x6a, 0x33, 0x33), 1.0)))).into()
        }
        Some(Check::Failed(e)) => look::card(&p, column![row![look::dot(look::BAD, z(10.0)), gap(z(14.0), 1.0), tb("Workspace check couldn't run", f(19.0), tn.text), Space::new().width(Length::Fill), pill].align_y(iced::Center), t(format!("Nothing is pushed until it runs: {e}"), f(15.0), tn.muted), again].spacing(z(10.0)), false),
        Some(Check::Tidy(sum)) => look::card(&p, column![row![look::dot(look::GOOD, z(10.0)), gap(z(14.0), 1.0), tb("Workspace check: tidy", f(19.0), tn.text), Space::new().width(Length::Fill), pill].align_y(iced::Center), t(sum.clone(), f(15.0), tn.muted), again].spacing(z(10.0)), false),
        Some(Check::None) => look::card(&p, column![tb("No workspace check here", f(19.0), tn.text), t("This folder has no tools/check.py, so nothing is checked before a push.", f(15.0), tn.muted)].spacing(z(8.0)), false),
        None => look::card(&p, row![t("Running the workspace check…", f(15.0), tn.muted), Space::new().width(Length::Fill), pill].align_y(iced::Center), false),
    }
}

fn commit_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let s = &a.sync;
    let input = text_input("What changed", &s.msg).id(MSG_ID).on_input(Msg::SyncMsg).on_submit(Msg::SyncNow).size(f(17.0)).padding(pad(z(14.0), z(16.0), z(30.0), z(16.0))).style(look::field_style(&p, false));
    let blocked = matches!(s.check, Some(Check::Untidy(_)) | Some(Check::Failed(_)));
    let signed = !matches!(s.auth, Some(Auth::None));
    let name = who(a);
    let line: E<Msg> = if signed {
        row![look::dot(look::GOOD, z(9.0)), gap(z(10.0), 1.0), t("Committing as ", f(14.0), tn.muted), tb(name, f(14.0), tn.text), t(", from this PC's Git sign-in · nothing is added to the message", f(14.0), tn.muted)].align_y(iced::Center).into()
    } else {
        row![look::dot(look::WARN, z(9.0)), gap(z(10.0), 1.0), t(hearth_notes::git::SIGN_IN_HELP, f(14.0), tn.muted)].align_y(iced::Center).into()
    };
    let probs = if let Some(Check::Untidy(pp)) = &s.check { pp.len() } else { 0 };
    let sync_btn = look::btn(&p, if s.busy { "⇅ Syncing…" } else { "⇅ Sync now" }, if blocked { look::B::Dim } else { look::B::Primary }, true, false, if blocked || s.busy { None } else { Some(Msg::SyncNow) });
    let mut r = row![sync_btn, gap(z(14.0), 1.0)].align_y(iced::Center);
    if blocked && probs > 0 {
        r = r.push(t(format!("Fix the {probs} problem{} first", crate::plural(probs)), f(15.0), Color::from_rgb8(0xff, 0x8a, 0x80)));
    } else if !s.said.is_empty() {
        r = r.push(t(s.said.clone(), f(15.0), if s.bad { Color::from_rgb8(0xff, 0x8a, 0x80) } else { tn.muted }));
    }
    look::card(&p, column![t("Commit message", f(14.0), tn.muted), input, line, r].spacing(z(12.0)), false)
}

fn diff_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let s = &a.sync;
    let path = s.changes.get(s.sel).map(|c| c.path.clone()).unwrap_or_default();
    let added = s.diff.iter().filter(|l| l.kind == '+').count();
    let removed = s.diff.iter().filter(|l| l.kind == '-').count();
    let sum = format!("Changes since GitHub · {} line{} added{}", added, crate::plural(added), if removed > 0 { format!(", {removed} removed") } else { String::new() });
    let head = container(row![text(path).size(f(14.5)).font(MONO).color(tn.text), Space::new().width(Length::Fill), t(sum, f(13.5), tn.muted)].align_y(iced::Center)).padding(pad(z(12.0), z(18.0), z(12.0), z(18.0)));
    let mut c = Column::new();
    let shown: Vec<&hearth_notes::git::DiffLine> = {
        let first_change = s.diff.iter().position(|l| l.kind != ' ').unwrap_or(0);
        s.diff.iter().skip(first_change.saturating_sub(4)).take(10).collect()
    };
    for l in shown {
        let (bg, mark, col) = match l.kind {
            '+' => (Color::from_rgba8(0x2a, 0x6b, 0x3a, 0.45), "+", Color::from_rgb8(0xd8, 0xf5, 0xdc)),
            '-' => (Color::from_rgba8(0x7a, 0x2a, 0x2a, 0.45), "−", Color::from_rgb8(0xff, 0xd0, 0xd0)),
            _ => (Color::TRANSPARENT, " ", Pal::mix(tn.text, tn.muted, 0.25)),
        };
        let r = container(row![container(text(l.no.to_string()).size(f(13.5)).font(MONO).color(tn.faint)).width(Length::Fixed(z(40.0))).align_right(Length::Fixed(z(40.0))), gap(z(12.0), 1.0), text(mark).size(f(13.5)).font(MONO).color(col), gap(z(10.0), 1.0), container(text(hearth_ui::ui::fit(&l.text, 74)).size(f(13.5)).font(MONO).color(col).wrapping(text::Wrapping::None)).clip(true)].align_y(iced::Center)).padding(pad(z(4.0), z(14.0), z(4.0), z(10.0))).width(Length::Fill).style(bx(bg, 0.0, None));
        c = c.push(r);
    }
    if s.diff.is_empty() {
        c = c.push(container(t("Pick a change to see it", f(14.0), tn.muted)).padding(z(16.0)));
    }
    container(column![head, look::divider(&p), container(c).padding(pad(z(6.0), 0.0, z(8.0), 0.0))]).width(Length::Fill).style(bx(Pal::mix(tn.page, tn.card, 0.6), 10.0, Some((tn.line, 1.0)))).clip(true).into()
}

fn auto_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let every = a.prefs.sync_every;
    let next = ((a.sync.next_auto - a.now).max(60) / 60).max(1);
    let line = if every == 0 { "Off: sync with Sync now.".to_string() } else { format!("Pulls and pushes when tidy; skipped while a game runs. Next in {next} min.") };
    let choices: Vec<(String, bool, Msg)> = [2u32, 5, 10, 30].iter().map(|m| (format!("{m} min"), every == *m, Msg::SyncEvery(*m))).collect();
    let r = row![look::sym(&p, "⟳"), gap(z(16.0), 1.0), column![t("Sync by itself", f(19.0), tn.text), t(line, f(14.0), tn.muted)].spacing(z(4.0)).width(Length::Fill), look::seg(&p, choices, false), gap(z(14.0), 1.0), look::switch(&p, every > 0, false, Some(Msg::SyncEvery(if every > 0 { 0 } else { 5 })))].align_y(iced::Center);
    look::card(&p, r, false)
}

fn tidy_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let when = if a.sync.last_sync > 0 { clock_of(a.sync.last_sync) } else { "—".into() };
    let r = row![
        container(glyph(G::Check, look::GOOD_TX, z(18.0), 2.2)).center_x(Length::Fixed(z(42.0))).center_y(Length::Fixed(z(42.0))).style(bx(Pal::a(look::GOOD, 0.1), 7.0, None)),
        gap(z(16.0), 1.0),
        column![t("When it's tidy and pushed", f(19.0), tn.text), t("The bar says \"Up to date\" with the time, like on every other page", f(14.0), tn.muted)].spacing(z(4.0)).width(Length::Fill),
        look::chip(&p, format!("Up to date · {when}"), look::Chip::Ok),
    ]
    .align_y(iced::Center);
    container(r).padding(pad(z(18.0), z(20.0), z(18.0), z(20.0))).width(Length::Fill).style(bx(Color::from_rgb8(0x17, 0x26, 0x1d), 10.0, Some((look::GOOD_LINE, 1.0)))).into()
}

fn secrets_card(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let mut c = Column::new().spacing(z(8.0)).push(row![look::dot(look::WARN, z(10.0)), gap(z(14.0), 1.0), tb("Something looks like a secret: nothing was committed", f(19.0), tn.text)].align_y(iced::Center));
    c = c.push(t("Look at each line. If it really is a token, password or key, take it out of the note (and change it where it's used). If it isn't, you can go on.", f(15.0), Pal::mix(tn.text, tn.muted, 0.2)));
    for (path, h) in a.sync.secrets.iter().take(8) {
        c = c.push(container(column![text(format!("{path}:{}", h.line)).size(f(13.5)).font(MONO).color(look::WARN), text(format!("{} · {}", h.what, h.shown)).size(f(13.5)).font(MONO).color(tn.text)].spacing(2.0)).padding(z(10.0)).width(Length::Fill).style(bx(Color::from_rgb8(0x2a, 0x24, 0x16), 7.0, Some((Color::from_rgb8(0x6a, 0x55, 0x22), 1.0)))));
    }
    c = c.push(row![look::btn(&p, "It isn't a secret: sync it", look::B::Normal, false, false, Some(Msg::SecretsGo)), look::btn(&p, "Cancel", look::B::Primary, false, false, Some(Msg::SecretsCancel))].spacing(z(10.0)));
    container(c).padding(pad(z(18.0), z(20.0), z(18.0), z(20.0))).width(Length::Fill).style(bx(Color::from_rgb8(0x22, 0x1f, 0x16), 10.0, Some((Color::from_rgb8(0x8a, 0x6a, 0x1f), 1.0)))).into()
}

// ------------------------------------------------------------------ Conflict (6)
fn host() -> String {
    std::env::var("HEARTH_NOTES_PC_NAME").ok().or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).unwrap_or_else(|| "this PC".into())
}

pub fn conflict(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(c) = a.conflict.as_ref() else {
        let msg = if a.sync.conflicts.is_empty() { "No note is changed on two PCs." } else { "Reading both versions…" };
        return container(t(msg, f(17.0), tn.muted)).center_x(Length::Fill).center_y(Length::Fill).into();
    };
    let rel = c.files.get(c.i).cloned().unwrap_or_default();
    let title = row![tb("One note changed on two PCs", f(40.0), tn.text), Space::new().width(Length::Fill), look::chip(&p, format!("{} of {}", c.i + 1, c.files.len()), look::Chip::Warn)].align_y(iced::Center);
    let sub = t(format!("{rel} was changed here and on another PC since the last sync. Pick what to keep. Nothing is thrown away."), f(16.0), Pal::mix(tn.text, tn.muted, 0.15));
    let (mine, theirs) = conflict::sides(&c.mine, &c.theirs);
    let mtime = std::fs::metadata(a.root.join(&rel)).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
    let side = |head: String, sub_: String, lines: Vec<(String, bool)>, hl: Color, tx: Color| -> E<'static, Msg> {
        let mut col = Column::new();
        for (k, (l, ch)) in lines.iter().enumerate().take(16) {
            col = col.push(container(row![container(text((k + 1).to_string()).size(f(13.5)).font(MONO).color(tn.faint)).width(Length::Fixed(z(36.0))), gap(z(16.0), 1.0), container(text(hearth_ui::ui::fit(l, 70)).size(f(14.0)).font(MONO).color(if *ch { tx } else { Pal::mix(tn.text, tn.muted, 0.25) }).wrapping(text::Wrapping::None)).clip(true)].align_y(iced::Center)).padding(pad(z(4.0), z(14.0), z(4.0), z(14.0))).width(Length::Fill).style(bx(if *ch { hl } else { Color::TRANSPARENT }, 0.0, None)));
        }
        let pad_lines = 9usize.saturating_sub(lines.len());
        for k in 0..pad_lines {
            col = col.push(container(text((lines.len() + k + 1).to_string()).size(f(13.5)).font(MONO).color(tn.faint)).padding(pad(z(4.0), z(14.0), z(4.0), z(14.0))));
        }
        let hd = container(row![tb(head, f(18.0), tn.text), gap(z(14.0), 1.0), t(sub_, f(14.0), tn.muted)].align_y(iced::Center)).padding(pad(z(14.0), z(18.0), z(12.0), z(18.0)));
        container(column![hd, look::divider(&p), container(col).padding(pad(z(8.0), 0.0, z(10.0), 0.0))]).width(Length::FillPortion(1)).style(bx(Pal::mix(tn.page, tn.card, 0.6), 10.0, Some((tn.line, 1.0)))).clip(true).into()
    };
    let here = format!("{} · {} · not pushed", host(), if mtime > 0 { clock_of(mtime) } else { String::new() });
    let there = format!("from another PC · {} · commit {}", if c.theirs_at > 0 { clock_of(c.theirs_at) } else { String::new() }, c.commit);
    let panes = row![
        side("This PC".into(), here, mine, Color::from_rgba8(0x2a, 0x6b, 0x3a, 0.40), Color::from_rgb8(0xd8, 0xf5, 0xdc)),
        side("GitHub".into(), there, theirs, Color::from_rgba8(0x24, 0x48, 0x7a, 0.45), Color::from_rgb8(0xd6, 0xe6, 0xff)),
    ]
    .spacing(z(22.0));
    let pick = c.pick;
    let choices = row![
        look::btn(&p, "Keep mine", look::B::Normal, true, pick == Some(Pick::Mine), Some(Msg::ConflictPick(Pick::Mine))),
        look::btn(&p, "Keep theirs", look::B::Normal, true, pick == Some(Pick::Theirs), Some(Msg::ConflictPick(Pick::Theirs))),
        look::btn(&p, "Merge both", look::B::Primary, true, pick.is_none() || pick == Some(Pick::Both), Some(Msg::ConflictPick(Pick::Both))),
        gap(z(10.0), 1.0),
        t("Merge both keeps every line from each side, in time order; nothing is overwritten.", f(15.0), tn.muted),
    ]
    .spacing(z(12.0))
    .align_y(iced::Center);
    let kept = hearth_notes::paths::tilde(&hearth_notes::paths::kept());
    let safe = container(
        row![
            container(glyph(G::Shield, look::GOOD_TX, z(20.0), 2.0)).center_x(Length::Fixed(z(46.0))).center_y(Length::Fixed(z(46.0))).style(bx(Pal::a(look::GOOD, 0.12), 8.0, None)),
            gap(z(18.0), 1.0),
            column![t("Nothing is lost", f(20.0), tn.text), t(format!("Whichever you pick, both versions are kept as copies in {kept}/ (outside the workspace, so the check stays tidy), and GitHub keeps every earlier version too."), f(15.0), Pal::mix(tn.text, tn.muted, 0.2))].spacing(z(4.0)).width(Length::Fill),
            look::btn(&p, "Open kept copies", look::B::Normal, false, false, Some(Msg::OpenKept)),
        ]
        .align_y(iced::Center),
    )
    .padding(pad(z(18.0), z(22.0), z(18.0), z(22.0)))
    .width(Length::Fill)
    .style(bx(Color::from_rgb8(0x17, 0x26, 0x1d), 10.0, Some((look::GOOD_LINE, 1.0))));
    let body = column![title, sub, panes, choices, safe].spacing(z(20.0));
    scrollable(container(body).padding(pad(z(28.0), z(40.0), z(28.0), z(40.0)))).height(Length::Fill).into()
}
