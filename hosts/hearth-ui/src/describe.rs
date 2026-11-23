//! Describe it in a Hearth app (desktop-layouts board 11; part 12: every Hearth app takes this). The box at the bottom
//! where you say how you'd like the app, the chips of what it'll do (Layout: …, New feature: …), Undo, Try again and
//! Keep this, the EDITING mark for the title, and the dashed outline round the parts being changed. The asking and the
//! checking are hearth-layout's (`hearth-layout describe-app APP …`, its app.rs: the app's own knobs, places, sources
//! and actions); what you keep is `~/.config/nimbus/apps/<app>.json`, written only by hearth-layout (settings_file).
//!
//! An app holds a [`Describe`], sends it [`DMsg`]s, draws [`bar`] at its bottom while it's open, and lays itself out from
//! [`Describe::layout`] and [`Describe::features`] (what's being tried, else what was kept).

use crate::run;
use crate::theme::Pal;
use crate::ui::{self, t};
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, stack, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Rectangle, Renderer, Task, Theme};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub enum DMsg {
    Open,
    Close,
    Text(String),
    Make,
    Got(run::Output),
    Keep,
    Kept(run::Output),
    Undo,
    Reset,
    Send,
    Sent(run::Output),
}

pub struct Describe {
    pub app: &'static str,
    pub name: &'static str,
    pub open: bool,
    pub text: String,
    pub busy: bool,
    /// the AI's checked answer, being tried (nothing kept yet)
    pub result: Option<Value>,
    pub error: String,
    pub said: String,
    /// what's kept (the app's file)
    pub kept: Value,
}

/// `hearth-layout ARGS…`, or nothing when it isn't installed (said plainly).
fn layout_cmd(args: Vec<String>) -> Vec<String> {
    let mut c = vec!["hearth-layout".to_string()];
    c.extend(args);
    c
}

pub fn kept_file(app: &str) -> std::path::PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".config"));
    cfg.join("nimbus/apps").join(format!("{app}.json"))
}

/// What this app kept: {"layout": {...}, "features": [...]} (empty when nothing, or the file can't be read; never written here).
pub fn read_kept(app: &str) -> Value {
    let j = crate::settings_file::read_json(&kept_file(app));
    json!({"layout": j.get("layout").cloned().unwrap_or(json!({})), "features": j.get("features").cloned().unwrap_or(json!([]))})
}

impl Describe {
    pub fn new(app: &'static str, name: &'static str) -> Describe {
        // HEARTH_DESCRIBE_OPEN=<app> opens the box at start (the CI pictures).
        let open = std::env::var("HEARTH_DESCRIBE_OPEN").map(|v| v == app).unwrap_or(false);
        Describe { app, name, open, text: String::new(), busy: false, result: None, error: String::new(), said: String::new(), kept: read_kept(app) }
    }

    /// A change being tried (an "app" answer), not yet kept.
    pub fn trying(&self) -> Option<&Value> {
        self.result.as_ref().filter(|r| r.get("kind").and_then(Value::as_str) == Some("app"))
    }

    /// One of the app's knobs: what's being tried, else what's kept, else "" (the app's own way).
    pub fn knob(&self, key: &str) -> String {
        if let Some(v) = self.trying().and_then(|r| r.pointer(&format!("/layout/{key}"))).and_then(Value::as_str) {
            return v.to_string();
        }
        self.kept.pointer(&format!("/layout/{key}")).and_then(Value::as_str).unwrap_or("").to_string()
    }

    /// One of the app's zones (its bars' pieces, in order): what's being tried, else what's kept, else None (the app's
    /// own). A piece the zones don't name anywhere stays in its own zone.
    pub fn zone(&self, zone: &str) -> Option<Vec<String>> {
        let get = |v: &Value| v.pointer(&format!("/layout/zones/{zone}")).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect::<Vec<_>>());
        self.trying().and_then(get).or_else(|| get(&self.kept))
    }

    /// Every piece placed somewhere by the zones (tried, else kept): a piece named here leaves its own zone.
    pub fn placed(&self) -> Vec<String> {
        let all = |v: &Value| -> Vec<String> {
            v.pointer("/layout/zones").and_then(Value::as_object).map(|m| m.values().flat_map(|a| a.as_array().cloned().unwrap_or_default()).filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
        };
        let mut out = self.trying().map(all).unwrap_or_default();
        for x in all(&self.kept) {
            if !out.contains(&x) {
                out.push(x);
            }
        }
        out
    }

    /// Is this zone being changed right now (its dashed outline)?
    pub fn zone_changing(&self, zone: &str) -> bool {
        self.open && self.trying().and_then(|r| r.pointer(&format!("/layout/zones/{zone}"))).is_some()
    }

    /// Is this knob being changed right now (the dashed outline goes round its part)?
    pub fn changing(&self, key: &str) -> bool {
        self.open && self.trying().and_then(|r| r.pointer(&format!("/layout/{key}"))).is_some()
    }

    /// The features in a place ("<app>:<place>"): kept ones, with the ones being tried (marked new: true).
    pub fn features(&self, place: &str) -> Vec<(Value, bool)> {
        let part = format!("{}:{place}", self.app);
        let mut out: Vec<(Value, bool)> = Vec::new();
        let trying: Vec<Value> = self.trying().and_then(|r| r.get("features")).and_then(Value::as_array).cloned().unwrap_or_default();
        for f in self.kept.get("features").and_then(Value::as_array).cloned().unwrap_or_default() {
            if f.get("part").and_then(Value::as_str) == Some(part.as_str()) && f.get("on") != Some(&json!(false)) && !trying.iter().any(|x| x.get("id") == f.get("id")) {
                out.push((f, false));
            }
        }
        for f in trying {
            if f.get("part").and_then(Value::as_str) == Some(part.as_str()) {
                out.push((f, true));
            }
        }
        out
    }

    pub fn has_kept(&self) -> bool {
        self.kept.get("layout").and_then(Value::as_object).map(|m| !m.is_empty()).unwrap_or(false) || self.kept.get("features").and_then(Value::as_array).map(|a| !a.is_empty()).unwrap_or(false)
    }

    pub fn update(&mut self, m: DMsg) -> Task<DMsg> {
        match m {
            DMsg::Open => {
                self.open = true;
                self.error.clear();
                self.said.clear();
            }
            DMsg::Close => {
                self.open = false;
                self.result = None;
                self.busy = false;
            }
            DMsg::Text(s) => self.text = s.chars().take(2000).collect(),
            DMsg::Make => {
                if self.busy || self.text.trim().is_empty() {
                    return Task::none();
                }
                self.busy = true;
                self.error.clear();
                self.result = None;
                let c = layout_cmd(vec!["describe-app".into(), self.app.into(), self.text.clone()]);
                return Task::perform(run::spawn(with_app(c, self.name)), DMsg::Got);
            }
            DMsg::Got(o) => {
                self.busy = false;
                match o.json() {
                    Some(v) if v.get("ok") == Some(&json!(true)) => self.result = Some(v),
                    Some(v) => self.error = v.get("error").and_then(Value::as_str).unwrap_or("That didn't work.").to_string(),
                    None => self.error = if o.err.contains("No such file") { "Describe it needs hearth-layout, which comes with Hearth OS.".into() } else { o.why() },
                }
            }
            DMsg::Keep => {
                let Some(r) = self.trying() else { return Task::none() };
                let change = json!({"layout": r.get("layout").cloned().unwrap_or(json!({})), "features": r.get("features").cloned().unwrap_or(json!([]))});
                return Task::perform(run::spawn(layout_cmd(vec!["app-keep".into(), self.app.into(), change.to_string()])), DMsg::Kept);
            }
            DMsg::Kept(o) => {
                if o.ok {
                    self.kept = read_kept(self.app);
                    self.result = None;
                    self.text.clear();
                    self.said = "Kept. Undo is here, and View › Layout › Back to the original any time.".into();
                } else {
                    self.error = o.why();
                }
            }
            DMsg::Undo => {
                // a change being tried: just not tried any more; else the last Keep this undone
                if self.result.take().is_some() {
                    return Task::none();
                }
                return Task::perform(run::spawn(layout_cmd(vec!["app-undo".into(), self.app.into()])), DMsg::Kept);
            }
            DMsg::Reset => {
                self.result = None;
                return Task::perform(run::spawn(layout_cmd(vec!["app-reset".into(), self.app.into()])), DMsg::Kept);
            }
            DMsg::Send => {
                let Some(i) = self.result.as_ref().and_then(|r| r.get("idea")).cloned() else { return Task::none() };
                self.busy = true;
                return Task::perform(run::spawn(layout_cmd(vec!["send-idea".into(), i.to_string()])), DMsg::Sent);
            }
            DMsg::Sent(o) => {
                self.busy = false;
                self.said = if o.ok { "Sent to Hearth's makers. Thank you!".into() } else { format!("It couldn't be sent: {}", o.why()) };
            }
        }
        Task::none()
    }
}

fn with_app(mut c: Vec<String>, name: &str) -> Vec<String> {
    let mut e = vec!["env".to_string(), format!("HEARTH_DESCRIBE_APP={name}")];
    e.append(&mut c);
    e
}

/// "EDITING", for the title while the box is open (board 11).
pub fn editing<'a, M: 'a>(p: &Pal) -> Element<'a, M> {
    container(t("EDITING", 12.0, p.accent).wrapping(iced::widget::text::Wrapping::None))
        .padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 })
        .style(ui::boxed(Color::TRANSPARENT, 11.0, Some((p.accent, 1.2))))
        .into()
}

/// "NEW", by a feature being tried.
pub fn new_chip<'a, M: 'a>(p: &Pal) -> Element<'a, M> {
    container(t("NEW", 12.0, p.accent).wrapping(iced::widget::text::Wrapping::None))
        .padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 })
        .style(ui::boxed(Color::TRANSPARENT, 11.0, Some((p.accent, 1.2))))
        .into()
}

struct Dashes(Color);
impl<M> canvas::Program<M> for Dashes {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let r = Path::rectangle(iced::Point::new(1.0, 1.0), iced::Size::new((b.width - 2.0).max(0.0), (b.height - 2.0).max(0.0)));
        f.stroke(&r, Stroke { line_dash: LineDash { segments: &[6.0, 4.0], offset: 0 }, ..Stroke::default().with_color(self.0).with_width(1.5) });
        vec![f.into_geometry()]
    }
}

/// The dashed outline round a part being changed (board 11), or the part as it is.
pub fn outline<'a, M: 'a>(p: &Pal, on: bool, inner: Element<'a, M>) -> Element<'a, M> {
    if !on {
        return inner;
    }
    stack![inner, canvas::Canvas::new(Dashes(p.accent)).width(Length::Fill).height(Length::Fill)].into()
}

/// A feature's own value, from what the app can count (its sources): "Pictures here: 38".
pub fn feature_text(f: &Value, value_of: &dyn Fn(&str) -> String) -> String {
    let shows: Vec<String> = f
        .get("shows")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|s| {
                    let src = s.get("source").and_then(Value::as_str).unwrap_or("");
                    let label = s.get("label").and_then(Value::as_str).unwrap_or("");
                    let v = value_of(src);
                    if label.is_empty() { v } else if v.is_empty() { label.to_string() } else { format!("{label}: {v}") }
                })
                .collect()
        })
        .unwrap_or_default();
    if shows.is_empty() { f.get("name").and_then(Value::as_str).unwrap_or("").to_string() } else { shows.join(" · ") }
}

/// The box at the bottom (board 11): your words, the chips, Undo / Try again / Keep this, and what it can't change.
pub fn bar<'a, M: Clone + 'a>(p: &Pal, d: &'a Describe, map: impl Fn(DMsg) -> M + Clone + 'a) -> Element<'a, M> {
    let p = *p;
    let m = map.clone();
    let input = text_input(&format!("Say how you'd like {}…", d.name), &d.text)
        .on_input(move |s| m(DMsg::Text(s)))
        .on_submit(map(DMsg::Make))
        .size(19.0)
        .padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 0.0 })
        .style(move |_: &Theme, _| text_input::Style {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
            icon: p.muted,
            placeholder: p.muted,
            value: p.text,
            selection: Pal::a(p.accent, 0.35),
        });
    let mut chips = Row::new().spacing(10.0).align_y(iced::Center);
    let chip = |s: String, feat: bool| -> Element<'a, M> {
        container(t(s, 13.0, if feat { p.accent } else { p.text }).wrapping(iced::widget::text::Wrapping::None))
            .padding(Padding { top: 4.0, right: 11.0, bottom: 4.0, left: 11.0 })
            .style(ui::boxed(if feat { Pal::a(p.accent, 0.08) } else { Color::TRANSPARENT }, 13.0, Some((if feat { p.accent } else { p.line }, 1.0))))
            .into()
    };
    let mut buttons = Row::new().spacing(10.0).align_y(iced::Center);
    let mut note = format!("{} only · changes how it's laid out and adds from its own parts; what its buttons do stays the same · Back to the original: View › Layout", d.name);
    match &d.result {
        Some(r) if r.get("kind").and_then(Value::as_str) == Some("app") => {
            for c in r.get("chips").and_then(Value::as_array).cloned().unwrap_or_default() {
                chips = chips.push(chip(c.get("text").and_then(Value::as_str).unwrap_or("").to_string(), c.get("kind").and_then(Value::as_str) == Some("feature")));
            }
            buttons = buttons.push(ui::btn(&p, "Undo", ui::Kind::Normal, false, Some(map(DMsg::Undo))));
            buttons = buttons.push(ui::btn(&p, "Try again", ui::Kind::Normal, false, Some(map(DMsg::Make))));
            buttons = buttons.push(ui::btn(&p, "Keep this", ui::Kind::Primary, false, Some(map(DMsg::Keep))));
        }
        Some(r) => {
            // it needs something the app doesn't have: what, and Send this idea to Hearth
            let needs: Vec<String> = r.get("needs").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            chips = chips.push(chip(format!("Needs something new: {}", needs.join(", ")), true));
            note = r.get("preview").and_then(Value::as_str).unwrap_or("").replace('\n', " · ");
            buttons = buttons.push(ui::btn(&p, "Not now", ui::Kind::Normal, false, Some(map(DMsg::Undo))));
            buttons = buttons.push(ui::btn(&p, "Send this idea to Hearth", ui::Kind::Primary, false, (!d.busy && d.said.is_empty()).then(|| map(DMsg::Send))));
        }
        None => {
            if d.has_kept() {
                buttons = buttons.push(ui::btn(&p, "Undo", ui::Kind::Normal, false, Some(map(DMsg::Undo))));
            }
            buttons = buttons.push(ui::btn(&p, "Done", ui::Kind::Normal, false, Some(map(DMsg::Close))));
            buttons = buttons.push(ui::btn(&p, if d.busy { "Making it…" } else { "Make it" }, ui::Kind::Primary, false, (!d.busy && !d.text.trim().is_empty()).then(|| map(DMsg::Make))));
        }
    }
    let status: Element<'a, M> = if !d.error.is_empty() {
        t(d.error.clone(), 13.0, p.danger).into()
    } else if !d.said.is_empty() {
        t(d.said.clone(), 13.0, p.good).into()
    } else {
        t(note, 13.0, p.muted).into()
    };
    // the chips wrap onto more lines (a whole app rearranged makes many); the buttons stay on the right under them
    let body: Column<'a, M> = column![input, chips.wrap(), row![container(status).width(Length::Fill), buttons].spacing(16.0).align_y(iced::Center)].spacing(14.0);
    let close = mouse_area(t("✕", 14.0, p.muted)).on_press(map(DMsg::Close)).interaction(mouse::Interaction::Pointer);
    container(stack![
        container(body).padding(Padding { top: 18.0, right: 22.0, bottom: 16.0, left: 22.0 }),
        container(close).align_right(Length::Fill).padding(Padding { top: 8.0, right: 12.0, bottom: 0.0, left: 0.0 })
    ])
    .width(Length::Fill)
    .style(ui::boxed(p.panel, p.r(16.0), Some((p.accent, 2.0))))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> Describe {
        Describe { app: "files", name: "Hearth Files", open: true, text: String::new(), busy: false, result: None, error: String::new(), said: String::new(), kept: json!({"layout": {"sidebar": "hidden"}, "features": []}) }
    }

    #[test]
    fn what_is_tried_wins_over_what_was_kept() {
        let mut d = d();
        assert_eq!(d.knob("sidebar"), "hidden");
        assert_eq!(d.knob("preview"), "");
        d.result = Some(json!({"ok": true, "kind": "app", "layout": {"sidebar": "icons"}, "features": [{"id": "c", "part": "files:downloads", "name": "Counts"}]}));
        assert_eq!(d.knob("sidebar"), "icons");
        assert!(d.changing("sidebar") && !d.changing("preview"));
        assert_eq!(d.features("downloads").len(), 1);
        assert!(d.features("downloads")[0].1, "marked new");
        assert!(d.features("folder").is_empty());
    }

    #[test]
    fn a_feature_reads_like_board_11() {
        let f = json!({"name": "Counts", "shows": [{"source": "folder-pictures", "label": "Pictures here"}]});
        assert_eq!(feature_text(&f, &|s| if s == "folder-pictures" { "38".into() } else { String::new() }), "Pictures here: 38");
    }
}

// ------------------------------------------------------------------ the window's own bar, for every app
/// The window's own bar as Describe it left it, for any Hearth app (the owner, Nov 22: "EVERY one of our apps needs
/// this"): the app's mark and name, anything in the middle, Minimize, Maximize and Close, in the order of the zone
/// "titlebar" (pieces: mark, title, middle, spacer, minimize, maximize, close), with their look ("winbuttons": "" | dots
/// | square). A right-click on it opens Describe it, so even an app without menus can be changed. The window can always
/// be moved (what's left of the bar moves it), and a window without Close still closes with Alt+F4 or Super+Q.
pub fn title_bar<'a, M: Clone + 'a>(p: &Pal, d: &Describe, mark: ui::G, title: &'a str, middle: Option<Element<'a, M>>, m: ui::TitleMsgs<M>, bg: Color, open: M) -> Element<'a, M> {
    use iced::widget::button;
    let p = *p;
    let style = d.knob("winbuttons");
    let wbtn = |id: &str| -> Element<'a, M> {
        let (g, msg, red, c) = match id {
            "minimize" => (ui::G::Minimize, m.minimize.clone().unwrap_or(m.close.clone()), false, Color::from_rgb8(0xfe, 0xbc, 0x2e)),
            "maximize" => (ui::G::Maximize, m.maximize.clone(), false, Color::from_rgb8(0x28, 0xc8, 0x40)),
            _ => (ui::G::Close, m.close.clone(), true, Color::from_rgb8(0xff, 0x5f, 0x57)),
        };
        if style == "dots" {
            let dot = container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(c, 6.0, None));
            return mouse_area(container(dot).center_x(Length::Fixed(22.0)).center_y(Length::Fixed(40.0))).on_press(msg).interaction(mouse::Interaction::Pointer).into();
        }
        let sq = style == "square";
        button(container(ui::glyph(g, p.muted, 10.0, 2.2)).center_x(Length::Fixed(if sq { 40.0 } else { 46.0 })).center_y(Length::Fixed(if sq { 32.0 } else { 40.0 })))
            .padding(0.0)
            .on_press(msg)
            .style(move |_: &Theme, st: button::Status| button::Style {
                background: Some(iced::Background::Color(match st {
                    button::Status::Hovered | button::Status::Pressed if red => Color::from_rgb8(0xc4, 0x2b, 0x1c),
                    button::Status::Hovered | button::Status::Pressed => p.hover(),
                    _ if sq => Pal::a(p.text, 0.06),
                    _ => Color::TRANSPARENT,
                })),
                text_color: p.text,
                border: iced::Border { radius: if sq { 4.0.into() } else { 0.0.into() }, ..iced::Border::default() },
                shadow: iced::Shadow::default(),
                snap: true,
            })
            .into()
    };
    let has_min = m.minimize.is_some();
    let own: Vec<String> = ["mark", "title", "spacer", "middle", "spacer", "minimize", "maximize", "close"].iter().filter(|x| has_min || !matches!(**x, "minimize" | "maximize")).map(|x| x.to_string()).collect();
    let placed = d.placed();
    let items: Vec<String> = match d.zone("titlebar") {
        Some(z) => z,
        None => own.iter().filter(|i| *i == "spacer" || !placed.contains(i)).cloned().collect(),
    };
    let drag = || -> Element<'a, M> { mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fixed(40.0))).on_press(m.drag.clone()).on_double_click(m.maximize.clone()).on_right_press(open.clone()).into() };
    let mut r = Row::new().align_y(iced::Center);
    let mut middle = middle;
    let mut dragged = false;
    for i in &items {
        match i.as_str() {
            "mark" => r = r.push(container(ui::glyph(mark, p.accent, 16.0, 2.0)).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 14.0 })),
            "title" => {
                let mut tr = Row::new().align_y(iced::Center).push(ui::tm(title, 13.0, p.text).wrapping(iced::widget::text::Wrapping::None));
                if d.open {
                    tr = tr.push(container(editing::<M>(&p)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 10.0 }));
                }
                r = r.push(mouse_area(container(tr).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 4.0 }).center_y(Length::Fixed(40.0))).on_press(m.drag.clone()).on_right_press(open.clone()));
            }
            "middle" => {
                if let Some(mid) = middle.take() {
                    r = r.push(mid);
                }
            }
            "spacer" => {
                dragged = true;
                r = r.push(drag());
            }
            "minimize" | "maximize" if !has_min => {}
            "minimize" | "maximize" | "close" => r = r.push(wbtn(i)),
            _ => {}
        }
    }
    if !dragged {
        r = r.push(drag());
    }
    // the window's buttons an answer took out without putting them anywhere: back at the end
    for b in ["minimize", "maximize", "close"] {
        if (has_min || b == "close") && !items.iter().any(|x| x == b) && !placed.iter().any(|x| x == b) {
            r = r.push(wbtn(b));
        }
    }
    if let Some(fm) = ui::frame_meter() {
        r = r.push(fm);
    }
    let bar: Element<'a, M> = container(r).width(Length::Fill).height(Length::Fixed(40.0)).style(ui::boxed(bg, 0.0, None)).into();
    outline(&p, d.zone_changing("titlebar") || d.changing("winbuttons"), bar)
}

/// An app's whole window with Describe it's box under it while it's open (every app takes this around its view).
pub fn with_bar<'a, M: Clone + 'a>(base: Element<'a, M>, p: &Pal, d: &'a Describe, map: impl Fn(DMsg) -> M + Clone + 'a) -> Element<'a, M> {
    if !d.open {
        return base;
    }
    column![container(base).height(Length::Fill), container(bar(p, d, map)).padding(12.0)].into()
}

/// A piece's id from its label ("Efficiency mode ✓" is efficiency-mode, "Set priority ▸" set-priority).
pub fn slug(label: &str) -> String {
    label.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>().split_whitespace().collect::<Vec<_>>().join("-")
}

/// Pieces as Describe it left them (any app's bar, side or menu): the ones in "hidden" left out, the zone's own order
/// first, then the rest as they were. A piece with an empty id (a menu's line) always stays.
pub fn arrange<T>(d: &Describe, zone: &str, items: Vec<(String, T)>) -> Vec<T> {
    let hidden = d.zone("hidden").unwrap_or_default();
    let mut rest: Vec<(String, T)> = items.into_iter().filter(|(id, _)| id.is_empty() || !hidden.contains(id)).collect();
    let mut out = Vec::new();
    if let Some(order) = d.zone(zone) {
        for id in order {
            if let Some(k) = rest.iter().position(|(i, _)| *i == id) {
                out.push(rest.remove(k).1);
            }
        }
    }
    out.extend(rest.into_iter().map(|x| x.1));
    out
}

#[cfg(test)]
mod arrange_tests {
    #[test]
    fn slugs() {
        assert_eq!(super::slug("Efficiency mode ✓"), "efficiency-mode");
        assert_eq!(super::slug("Set priority ▸"), "set-priority");
        assert_eq!(super::slug("Limit CPU / memory…"), "limit-cpu-memory");
        assert_eq!(super::slug("+ New task"), "new-task");
    }
}
