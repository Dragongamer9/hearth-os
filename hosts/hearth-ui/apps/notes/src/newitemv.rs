//! New item (page 7): the wizard that follows the workspace's CLAUDE.md: project and kind, the name check
//! (lowercase-with-dashes, a suggestion, not taken), status and release, and a preview of exactly what it will make:
//! the README from its template, the INDEX line and the day's log line.

use crate::look::{self, accent2, bx, click, f, pad, project_color, tone, z, E, MONO};
use crate::{App, Msg, NAME_ID};
use hearth_notes::newitem::{self, KINDS, PROJECTS, STATUSES};
use hearth_notes::day;
use hearth_ui::ui::{gap, t, t1, tb, tm};
use hearth_ui::Pal;
use iced::widget::{column, container, row, text, text_input, Column, Row, Space};
use iced::{Color, Length};

/// The releases offered: "not yet assigned" and the newest two the vault's own tags name (#1.16, #1.17).
fn releases(a: &App) -> Vec<String> {
    let mut v: Vec<String> = a.vault.tag_counts().into_iter().map(|x| x.0).filter(|t| t.chars().all(|c| c.is_ascii_digit() || c == '.') && t.contains('.')).collect();
    v.sort_by(|x, y| {
        let k = |s: &str| s.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
        k(y).cmp(&k(x))
    });
    v.truncate(2);
    v.reverse();
    let mut out = vec!["not yet assigned".to_string()];
    out.extend(v);
    out
}

pub fn view(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tn = tone(&p);
    let Some(n) = a.new_item.as_ref() else { return gap(1.0, 1.0) };
    let project = PROJECTS[n.project];
    let kind = KINDS[n.kind];
    let pchip = |i: usize, name: &'static str| -> E<'static, Msg> {
        let on = i == n.project;
        let c = container(row![look::dot(project_color(&p, name), z(10.0)), gap(z(8.0), 1.0), t1(name, f(15.0), if on { accent2(&p) } else { Pal::mix(tn.text, tn.muted, 0.15) })].align_y(iced::Center)).padding(pad(z(5.0), z(14.0), z(5.0), z(14.0))).style(bx(if on { Pal::a(p.accent, 0.1) } else { Color::TRANSPARENT }, z(16.0), Some((if on { p.accent } else { tn.line }, 1.0))));
        click(c, Some(Msg::NewProject(i)))
    };
    let mut pr = Row::new().spacing(z(10.0));
    for (i, pj) in PROJECTS.iter().enumerate() {
        pr = pr.push(pchip(i, pj));
    }
    let mut kr = Row::new().spacing(z(10.0));
    for (i, k) in KINDS.iter().enumerate() {
        let on = i == n.kind;
        kr = kr.push(click(container(t1(*k, f(15.0), if on { accent2(&p) } else { Pal::mix(tn.text, tn.muted, 0.15) })).padding(pad(z(5.0), z(14.0), z(5.0), z(14.0))).style(bx(if on { Pal::a(p.accent, 0.1) } else { Color::TRANSPARENT }, z(16.0), Some((if on { p.accent } else { tn.line }, 1.0)))), Some(Msg::NewKind(i))));
    }
    let name_in = text_input("its name", &n.name).id(NAME_ID).on_input(Msg::NewName).on_submit(Msg::NewMake).size(f(19.0)).padding(pad(z(12.0), z(16.0), z(12.0), z(16.0))).style(look::field_style(&p, true));
    let ok = newitem::name_ok(&n.name);
    let sug = newitem::suggest(&n.name);
    let taken = !n.name.is_empty() && newitem::taken(&a.root, project, kind, if ok { &n.name } else { &sug });
    let mut checks = Column::new().spacing(z(6.0));
    if !n.name.is_empty() && !ok {
        checks = checks.push(row![t("✕ Lowercase words with dashes, no spaces", f(15.0), Color::from_rgb8(0xff, 0x8a, 0x80)), Space::new().width(Length::Fill), click(container(row![t("Use ", f(14.0), tn.text), text(sug.clone()).size(f(14.0)).font(iced::Font { weight: iced::font::Weight::Bold, ..MONO }).color(tn.text)]).padding(pad(z(4.0), z(12.0), z(4.0), z(12.0))).style(bx(Color::TRANSPARENT, 5.0, Some((tn.line, 1.0)))), Some(Msg::NewSuggested))].align_y(iced::Center));
    } else if ok {
        checks = checks.push(t("✓ Lowercase words with dashes", f(15.0), look::GOOD_TX));
    }
    if !n.name.is_empty() {
        checks = checks.push(t(if taken { format!("✕ already in {project}/{kind}") } else { format!("✓ not taken in {project}/{kind}") }, f(14.0), if taken { Color::from_rgb8(0xff, 0x8a, 0x80) } else { Pal::mix(tn.text, tn.muted, 0.25) }));
    }
    checks = checks.push(row![t("· a version or date leads when it matters, like ", f(14.0), tn.muted), text("1.13-features").size(f(13.5)).font(MONO).color(tn.muted), t(" or ", f(14.0), tn.muted), text("2026-11-15-recap").size(f(13.5)).font(MONO).color(tn.muted)]);
    let st = look::seg(&p, STATUSES.iter().enumerate().map(|(i, s)| (s.to_string(), i == n.status, Msg::NewStatus(i))).collect(), false);
    let rel = look::seg(&p, releases(a).into_iter().map(|r| (r.clone(), r == n.release, Msg::NewRelease(r))).collect(), false);
    let left = column![
        tm("1 · Project", f(17.0), tn.text),
        pr.wrap(),
        tm("2 · Kind", f(17.0), tn.text),
        kr,
        tm("3 · Name", f(17.0), tn.text),
        name_in,
        checks,
        tm("4 · Status and release", f(17.0), tn.text),
        row![st, gap(z(14.0), 1.0), rel].align_y(iced::Center),
    ]
    .spacing(z(10.0))
    .width(Length::FillPortion(11));
    let final_name = if ok { n.name.clone() } else if sug.is_empty() { "…".into() } else { sug.clone() };
    let item = format!("{project}/{kind}/{final_name}");
    let status = STATUSES[n.status];
    let hhmm = day::hhmm(a.central_now());
    let readme = newitem::readme(&newitem::title_of(&final_name), status, &n.release);
    let code = |head: String, body: String| -> E<'static, Msg> {
        container(column![text(head).size(f(14.0)).font(MONO).color(accent2(&p)), text(body).size(f(14.0)).font(MONO).color(Pal::mix(tn.text, tn.muted, 0.1))].spacing(z(8.0))).padding(pad(z(14.0), z(18.0), z(14.0), z(18.0))).width(Length::Fill).style(bx(Pal::mix(tn.page, tn.card, 0.4), 8.0, None)).into()
    };
    let readme_short: String = readme.lines().take(8).collect::<Vec<_>>().join("\n");
    let right = column![
        tm("It will make", f(17.0), tn.text),
        code(format!("{item}/README.md"), readme_short),
        code(format!("INDEX.md · under \"{}\"", newitem::index_heading(project)), format!("+ {}", newitem::index_line(&item, status))),
        code(format!("log/{}.md", day::ymd(a.today)), format!("+ {}", newitem::log_line(&hhmm, &item, status))),
        Space::new().height(Length::Fill),
        row![Space::new().width(Length::Fill), look::btn(&p, "Cancel", look::B::Normal, true, false, Some(Msg::NewCancel)), look::btn(&p, "Make it", if ok && !taken { look::B::Primary } else { look::B::Dim }, true, false, if ok && !taken { Some(Msg::NewMake) } else { None })].spacing(z(12.0)),
    ]
    .spacing(z(10.0))
    .width(Length::FillPortion(10));
    let mut body = column![
        row![tb("New item", f(30.0), tn.text), gap(z(16.0), 1.0), t("Follows the workspace's rules: <project>/<kind>/<item>/, a README, an INDEX line", f(16.0), tn.muted)].align_y(iced::Center),
        row![left, gap(z(30.0), 1.0), right].height(Length::Fixed(z(560.0))),
    ]
    .spacing(z(14.0));
    if !n.error.is_empty() {
        body = body.push(t(n.error.clone(), f(15.0), Color::from_rgb8(0xff, 0x8a, 0x80)));
    }
    container(body).padding(pad(z(26.0), z(40.0), z(30.0), z(40.0))).width(Length::Fixed(z(1530.0))).style(bx(tn.card, 12.0, Some((tn.line, 1.0)))).into()
}
