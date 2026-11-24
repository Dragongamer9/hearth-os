//! The dock along the bottom, drawn as the QML Mantel's Dock.qml (Mantel 2026): floating and rounded (most looks) or
//! attached to the edge as one flat strip (Steam-style, Zen). The Hearth flame (the app menu), a line, your pinned
//! apps (desk.json, the QML Mantel's, only read here) and then every other open app, each app's windows sharing one
//! button, and the thin Show desktop strip at the end. A running app has a dot, the one in front a long accent dot.
//! Click: open it, bring it to the front (in front already: minimize it; several windows: the next one);
//! middle-click: open another one.

use iced::widget::{button, column, container, mouse_area, row, stack, Space};
use iced::{border, Alignment, Background, Element, Length, Padding};

use crate::apps::{self, App};
use crate::look::{self, a, Look};
use crate::shell::{Message, Shell};
use crate::wm;

/// One button: the app (when it's installed), its windows, pinned or not.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub app: Option<App>,
    pub key: String,
    pub name: String,
    pub wins: Vec<wm::Win>,
    pub pinned: bool,
}

/// The buttons, in order: pins first (in pin order), then the other open apps by when they opened.
pub fn items(apps: &[App], pins: &[String], wins: &[wm::Win]) -> Vec<Item> {
    let mut v: Vec<Item> = Vec::new();
    for p in pins {
        let Some(app) = apps::find(apps, p).cloned() else { continue };
        if v.iter().any(|i| i.key == app.id) {
            continue;
        }
        v.push(Item { key: app.id.clone(), name: app.name.clone(), app: Some(app), wins: Vec::new(), pinned: true });
    }
    for w in wins {
        let app = apps::for_window(apps, &w.app_id);
        let key = app.map(|x| x.id.clone()).unwrap_or_else(|| format!("app:{}", if w.app_id.is_empty() { &w.title } else { &w.app_id }));
        if key == "app:" || key.contains("hearth-mantel") {
            continue;
        }
        if let Some(it) = v.iter_mut().find(|i| i.key == key) {
            it.wins.push(w.clone());
        } else {
            v.push(Item { key, name: app.map(|x| x.name.clone()).unwrap_or_else(|| w.title.clone()), app: app.cloned(), wins: vec![w.clone()], pinned: false });
        }
    }
    v
}

/// A click (and Super+number): open it, bring it to the front, or (in front) minimize it; several windows: the next.
pub fn cycle(it: &Item) {
    let ws = &it.wins;
    if ws.is_empty() {
        if let Some(a) = &it.app {
            apps::launch(a);
        }
        return;
    }
    let i = ws.iter().position(|w| w.active);
    if ws.len() == 1 {
        if i == Some(0) {
            wm::minimize(ws[0].id);
        } else {
            wm::activate(ws[0].id);
        }
    } else {
        let next = i.map(|i| (i + 1) % ws.len()).unwrap_or(0);
        wm::activate(ws[next].id);
    }
}

pub struct Geom {
    pub attached: bool,
    pub tile: f32,
    pub icon: f32,
    pub body_h: f32,
    pub gap: f32,
}

impl Geom {
    /// `style`: a layout's dockStyle ("" the look's own, "floating", "attached").
    pub fn new(l: &Look, compact: bool, style: &str) -> Geom {
        // Console's shelf of big tiles isn't in this test yet: it's drawn as the floating dock.
        let attached = style == "attached" || (style.is_empty() && l.style == "bar");
        let tile = if compact { 68.0 } else { 56.0 };
        Geom { attached, tile, icon: if compact { 46.0 } else { 38.0 }, body_h: if attached { tile + 8.0 } else { tile + 16.0 }, gap: if attached { 0.0 } else { 10.0 } }
    }
}

/// The dock's surface height and the space it keeps at the bottom.
pub fn height(l: &Look, compact: bool, style: &str) -> u32 {
    let g = Geom::new(l, compact, style);
    (g.body_h + g.gap + 2.0).round() as u32
}
pub fn reserved(l: &Look, compact: bool, style: &str) -> u32 {
    let g = Geom::new(l, compact, style);
    (g.body_h + g.gap + if g.attached { 0.0 } else { 2.0 }).round() as u32
}

fn dock_button<'a>(l: &Look, g: &Geom, face: Element<'a, Message>, active: bool, running: bool, count: usize, msg: Message) -> Element<'a, Message> {
    let p = l.p;
    let r = l.r(14.0);
    let tile = g.tile;
    // the running dot under the icon: long and lit when in front
    let dot: Element<'a, Message> = if active || running {
        let len = if active { 18.0 } else if count > 1 { 10.0 } else { 5.0 };
        let c = if active { p.accent } else { p.muted };
        container(Space::new().width(Length::Fixed(len)).height(Length::Fixed(4.0))).style(move |_| container::Style { background: Some(c.into()), border: border::rounded(2.0), ..container::Style::default() }).into()
    } else {
        Space::new().height(Length::Fixed(4.0)).into()
    };
    let layers = stack![
        container(face).center(Length::Fixed(tile)),
        container(column![Space::new().height(Length::Fill), container(dot).center_x(Length::Fill), Space::new().height(Length::Fixed(2.0))]).width(Length::Fixed(tile)).height(Length::Fixed(tile)),
    ];
    button(layers)
        .padding(0)
        .width(Length::Fixed(tile))
        .height(Length::Fixed(tile))
        .on_press(msg)
        .style(move |_, st| button::Style {
            background: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                Some(Background::Color(p.panel3))
            } else if active {
                Some(Background::Color(a(p.text, 0.08)))
            } else {
                None
            },
            border: border::rounded(r),
            text_color: p.text,
            ..button::Style::default()
        })
        .into()
}

pub fn view(s: &Shell) -> Element<'_, Message> {
    let l = &s.look;
    let p = l.p;
    let g = Geom::new(l, s.compact(), &s.desk.dock_style);

    let mut r = row![dock_button(l, &g, look::flame(28.0), false, false, 0, Message::Flame)].spacing(8).align_y(Alignment::Center);
    let line = p.line;
    r = r.push(container(Space::new().width(Length::Fixed(1.0)).height(Length::Fixed(g.tile * 0.6))).style(move |_| container::Style { background: Some(line.into()), ..container::Style::default() }));
    for (i, it) in items(&s.apps, &s.pins, &s.wm.wins).into_iter().enumerate() {
        let active = it.wins.iter().any(|w| w.active);
        let face = look::app_pic(it.app.as_ref().and_then(|a| a.icon_path.as_deref()), g.icon);
        let b = dock_button(l, &g, face, active, !it.wins.is_empty(), it.wins.len(), Message::Task(i));
        r = r.push(mouse_area(b).on_middle_press(Message::TaskMiddle(i)));
    }
    // Show desktop: the thin strip at the end
    let (txt, p3) = (p.text, p.panel3);
    let strip = button(Space::new().width(Length::Fixed(8.0)).height(Length::Fixed(g.tile * 0.62)))
        .padding(0)
        .on_press(Message::ShowDesktop)
        .style(move |_, st| button::Style {
            background: Some(Background::Color(if matches!(st, button::Status::Hovered | button::Status::Pressed) { p3 } else { a(txt, 0.12) })),
            border: border::rounded(3.0),
            ..button::Style::default()
        });
    r = r.push(strip);

    let glass = l.glass();
    if g.attached {
        // attached to the edge: one flat strip, a line along its top
        let body = column![
            container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(move |_| container::Style { background: Some(line.into()), ..container::Style::default() }),
            container(r).center_x(Length::Fill).center_y(Length::Fill),
        ];
        let strip = container(body).width(Length::Fill).height(Length::Fixed(g.body_h)).style(move |_| container::Style { background: Some(glass.into()), ..container::Style::default() });
        column![Space::new().height(Length::Fill), strip].into()
    } else {
        let g2 = crate::bar::Geom::new(l, s.compact());
        let shelf = container(r)
            .padding(Padding { left: 10.0, right: 10.0, ..Padding::ZERO })
            .height(Length::Fixed(g.body_h))
            .align_y(Alignment::Center)
            .style(crate::bar::frame(l, &crate::bar::Geom { floating: true, ..g2 }, if l.gaming() { 0.0 } else { l.r(22.0) }, false));
        container(shelf).center_x(Length::Fill).padding(Padding { bottom: g.gap, ..Padding::ZERO }).height(Length::Fill).align_y(Alignment::End).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> App {
        crate::apps::parse(id, &format!("[Desktop Entry]\nName={id}\nExec={id}\n")).unwrap()
    }

    #[test]
    fn pins_first_then_open_apps_sharing_a_button() {
        let apps = vec![app("org.a.files"), app("firefox"), app("steam")];
        let pins = vec!["firefox".to_string(), "gone.app".to_string()];
        let wins = vec![
            wm::Win { id: 1, app_id: "steam".into(), title: "Steam".into(), ..Default::default() },
            wm::Win { id: 2, app_id: "firefox".into(), title: "Web".into(), active: true, ..Default::default() },
            wm::Win { id: 3, app_id: "steam".into(), title: "Friends".into(), ..Default::default() },
        ];
        let v = items(&apps, &pins, &wins);
        assert_eq!(v.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), ["firefox", "steam"]);
        assert!(v[0].pinned && v[0].wins.len() == 1);
        assert_eq!(v[1].wins.len(), 2);
    }

    #[test]
    fn steam_style_dock_is_attached_and_hearth_floats() {
        let mut l = crate::look::get();
        l.style = "bar".into();
        assert_eq!(height(&l, false, ""), 66);
        assert_eq!(reserved(&l, false, ""), 64);
        assert_eq!(height(&l, false, "floating"), 84, "a layout's floating dock in a bar look");
        l.style = "floating".into();
        assert_eq!(height(&l, false, ""), 84);
        assert_eq!(reserved(&l, false, ""), 84);
    }
}
