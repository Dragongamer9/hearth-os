//! Game Mode (mockups 5 and 5b): the picture full screen, a top info pill (source, 1080p60, delay, Low delay) and the
//! controller hints that fade after 3 s (any button on this PC's controller shows them again; the Xbox's controller
//! keeps playing the Xbox), and the quick menu ☰ opens on the right. Hold B to leave.

use crate::desk::{self, card_name, delay_words, fit_seg, mode_short, volume_slider};
use crate::look::*;
use crate::{App, Msg, Page};
use iced::widget::{column, container, row, stack, Column, Space};
use iced::{Color, Length};
use serde_json::json;

const HINT: Color = Color { r: 0xe3 as f32 / 255.0, g: 0xe6 as f32 / 255.0, b: 0xeb as f32 / 255.0, a: 1.0 };

pub fn viewer(a: &App) -> E<'_, Msg> {
    let mut layers: Vec<E<'_, Msg>> = vec![desk::picture(a)];
    if a.menu {
        // The picture dimmed behind the menu.
        layers.push(container(Space::new()).width(Length::Fill).height(Length::Fill).style(bx(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.45 }, 0.0, None)).into());
    }
    let shown = (a.pill || a.shows("pill")) && !a.menu;
    if let Some(p) = desk::rec_pill(a, 16.0) {
        if !a.menu {
            layers.push(container(p).center_x(Length::Fill).padding(pad(26.0, 0.0, 0.0, 0.0)).into());
        }
    } else if shown {
        let ld = if a.prefs.low_delay { "Low delay on" } else { "Low delay off" };
        let pill = container(row![dotted(OK, 9.0), column![one(t6(format!("{} · {}", card_name(a), mode_short(a)), 17.0, TX)), one(t(format!("Delay {} · {ld}", delay_words(a)), 14.0, MUT))].spacing(2.0)].spacing(12.0).align_y(iced::Center))
            .padding(pad(12.0, 18.0, 12.0, 18.0))
            .style(shadowed(Color { a: 0.91, ..Color::from_rgb8(0x12, 0x14, 0x19) }, 12.0, Some((Color { r: 1.0, g: 1.0, b: 1.0, a: 0.11 }, 1.0)), 40.0));
        layers.push(container(pill).center_x(Length::Fill).padding(pad(26.0, 0.0, 0.0, 0.0)).into());
    }
    if shown {
        let strip = container(
            row![hint(&["☰"], "Quick menu", 17.0), hint(&["X"], "Screenshot", 17.0), hint(&["Y"], "Record", 17.0), hint(&["◀▶"], "Volume", 17.0), fill(), hint(&["B"], "Hold to leave", 17.0)].spacing(30.0).align_y(iced::Center),
        )
        .padding(pad(0.0, 30.0, 0.0, 30.0))
        .center_y(Length::Fixed(64.0))
        .width(Length::Fill)
        .style(grad(180.0, Color::TRANSPARENT, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.8 }, 0.0));
        layers.push(container(strip).align_bottom(Length::Fill).width(Length::Fill).into());
    }
    if !a.menu {
        if let Some(tst) = desk::shot_toast(a, if shown { 80.0 } else { 30.0 }) {
            layers.push(tst);
        }
    }
    if a.menu {
        layers.push(quick_menu(a));
    }
    let _ = HINT;
    stack(layers).width(Length::Fill).height(Length::Fill).into()
}

/// The quick menu (5b): opened with ☰ (Menu) on this PC's controller, or Esc. B closes it.
pub fn quick_menu(a: &App) -> E<'_, Msg> {
    let p = &a.prefs;
    let fo = |i: usize| a.menu_focus == i;
    // On a smaller screen (the Ally's 1280 × 800) the same menu, a little tighter, so all of it fits.
    let c = a.size.1 < 1000.0;
    let (pv, gp, ts, top) = if c { (10.0, 6.0, 17.0, 20.0) } else { (16.0, 8.0, 19.0, 30.0) };
    let mrow = move |inner: E<'static, Msg>, focused: bool| -> E<'static, Msg> { ring(focused, 12.0, container(inner).padding(pad(pv, 20.0, pv, 20.0)).width(Length::Fill).style(bx(if focused { FOCUSROW } else { CARD }, 12.0, None))) };
    let title = move |s: &str| -> E<'static, Msg> { t(s.to_string(), ts, TX).into() };
    let fmt = a.mode.as_ref().map(|m| m.format_words()).unwrap_or("MJPEG");
    let rec_icon: E<'static, Msg> = container(dotted(Color::from_rgb8(0xff, 0x4b, 0x4b), 16.0)).center_x(Length::Fixed(40.0)).center_y(Length::Fixed(40.0)).style(bx(CARD2, 10.0, None)).into();
    let recording = a.rec.is_some();
    let cell = |icon: E<'static, Msg>, label: &str, focused: bool, m: Msg| -> E<'static, Msg> {
        let inner = container(row![icon, t(label.to_string(), ts - 1.0, TX)].spacing(16.0).align_y(iced::Center)).padding(pad(pv, 18.0, pv, 18.0)).width(Length::Fill).style(bx(if focused { FOCUSROW } else { CARD }, 12.0, None));
        container(ring(focused, 12.0, click(inner, Some(m)))).width(Length::FillPortion(1)).into()
    };
    let col: Column<'_, Msg> = column![
        row![app_icon(40.0, 24.0, 11.0), one(t6("Capture Card", 26.0, TX))].spacing(14.0).align_y(iced::Center),
        gap(1.0, 6.0),
        row![dotted(OK, 9.0), gap(7.0, 1.0), one(t(format!("{} · {} · {fmt} · delay {}", card_name(a), mode_short(a), delay_words(a)), 16.0, MUT))].align_y(iced::Center),
        gap(1.0, gp + 6.0),
        click(mrow(row![ri(G::Bolt, 40.0), column![title("Low delay"), t("For playing; off if it stutters", 14.0, MUT)].spacing(3.0), fill(), toggle(p.low_delay, 1.0, false, None)].spacing(16.0).align_y(iced::Center).into(), fo(0)), Some(Msg::Set("lowDelay", json!(!p.low_delay)))),
        gap(1.0, gp),
        mrow(row![ri(G::Vol, 40.0), title("Volume"), fill(), volume_slider(p.volume, 170.0), container(one(t(p.volume.to_string(), 14.0, MUT))).width(Length::Fixed(28.0))].spacing(16.0).align_y(iced::Center).into(), fo(1)),
        gap(1.0, gp),
        click(mrow(row![ri(G::Mute, 40.0), title("Mute"), fill(), one(t(if p.muted { "On" } else { "Off" }, 14.0, MUT)), toggle(p.muted, 1.0, false, None)].spacing(16.0).align_y(iced::Center).into(), fo(2)), Some(Msg::Mute)),
        gap(1.0, gp),
        mrow(row![ri(G::Fit, 40.0), title("Picture"), fill(), fit_seg(a, 15.0, 7.0, 15.0)].spacing(16.0).align_y(iced::Center).into(), fo(3)),
        gap(1.0, gp + 6.0),
        row![cell(ri(G::Cam, 40.0), "Screenshot", fo(4), Msg::Screenshot), cell(rec_icon, if recording { "Stop recording" } else { "Record" }, fo(5), Msg::Record)].spacing(10.0),
        gap(1.0, gp + 8.0),
        click(mrow(row![ri(G::List, 40.0), title("Recordings & screenshots"), fill(), one(t(format!("{} · {}", a.recs(), a.shots()), 14.0, MUT))].spacing(16.0).align_y(iced::Center).into(), fo(6)), Some(Msg::Go(Page::Library))),
        gap(1.0, gp),
        click(mrow(row![ri(G::Gear, 40.0), title("Settings")].spacing(16.0).align_y(iced::Center).into(), fo(7)), Some(Msg::Go(Page::Settings))),
        gap(1.0, gp),
        click(mrow(row![ri(G::Exit, 40.0), title("Leave Capture Card")].spacing(16.0).align_y(iced::Center).into(), fo(8)), Some(Msg::Leave)),
        vfill(),
    ];
    let foot = hints(vec![hint(&["A"], "Select", 16.0), hint(&["◀▶"], "Change", 16.0), hint(&["B"], "Close", 16.0)], vec![], 54.0, 28.0, 28.0, BG);
    let panel = container(column![container(col).padding(pad(top, 32.0, 0.0, 32.0)).height(Length::Fill), foot])
        .width(Length::Fixed(if c { 560.0 } else { 620.0 }))
        .height(Length::Fill)
        .style(shadowed(BG, 0.0, Some((LINE, 1.0)), 80.0));
    container(iced::widget::opaque(panel)).align_right(Length::Fill).height(Length::Fill).into()
}
