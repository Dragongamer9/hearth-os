//! Bluetooth & devices › Drawing tablet & touchscreen (Mantel's SetKwinTablet and KwinCalibrate, docs/KWIN-SETTINGS.md):
//! a pen tablet's screen and the part of it the tablet covers, which part of the tablet is used, left-handed, the
//! pressure curve and range, and what the pen's buttons do; a touchscreen's screen, rotation and calibration (four
//! crosses, the whole screen). Each device's settings go to KWin over D-Bus (hearth-kwin-settings input-set: KWin's
//! org.kde.KWin.InputDevice, which keeps them in kcminputrc); the pen buttons are KWin's button rebinds
//! (kcminputrc [ButtonRebinds][TabletTool]); "One pointer for the pen and the mouse" is kcminputrc [Tablet]
//! SyncWithMouse, which KWin 6.7 and newer read.

use super::kw::{self, Kw};
use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{tb, t, G};
use iced::widget::canvas::{self, Action, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, Column};
use iced::{mouse, touch, Color, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme};
use serde_json::{json, Value};

pub const IDENTITY: &str = "1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1";

#[derive(Default)]
pub struct S {
    pub kw: Kw,
    pub devices: Vec<Value>,
    pub outs: Vec<Value>,
    pub saving: bool,
    pub open: String,
    pub cal: Option<Value>,
    pub points: Vec<(f32, f32, f32, f32)>,
    /// Esc pressed while calibrating: put back as it was (on the next moment, as Back can't start anything itself).
    pub cancel: bool,
}

#[derive(Debug, Clone)]
pub enum M {
    KwStatus(Res),
    KwGot(Res),
    Input(Res),
    Outputs(Res),
    Set(String, Value),
    SetDone(Res),
    Sync(bool),
    KwSaved(Res),
    Open(String),
    Rebind(String, String, String),
    Rebound(Res),
    Calibrate(String),
    CalPoint(f32, f32),
    CalDone,
    Range(String, &'static str, f32),
    RangeSave(String, &'static str),
    Go(&'static str),
}

/// Where the crosses are, as parts of the screen.
pub const TARGETS: [(f32, f32); 4] = [(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9)];
pub const AREAS: &[(&str, &str, [f64; 4])] = &[("all", "All of it", [0.0, 0.0, 1.0, 1.0]), ("left", "Left half", [0.0, 0.0, 0.5, 1.0]), ("right", "Right half", [0.5, 0.0, 0.5, 1.0]), ("middle", "The middle", [0.25, 0.25, 0.5, 0.5])];
pub const CURVES: &[(&str, &str)] = &[("0,0;1,1;", "Even"), ("0,0.35;0.4,1;", "Soft (light strokes count more)"), ("0.35,0;1,0.6;", "Firm (press harder)")];
/// MouseButton,N is the Linux button code (BTN_RIGHT 273, BTN_MIDDLE 274, BTN_SIDE 275), as KWin's rebinds send it.
pub const BUTTON_ACTIONS: &[(&str, &str)] = &[("", "As it is"), ("MouseButton,273", "Right-click"), ("MouseButton,274", "Middle-click"), ("MouseButton,275", "Back"), ("Key,Ctrl+Z", "Undo (Ctrl+Z)"), ("Key,Ctrl+Shift+Z", "Redo"), ("Key,E", "Eraser key (E)"), ("Disabled", "Nothing")];

fn area_of(r: &Value) -> &'static str {
    let v: Vec<f64> = r.as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
    if v.len() != 4 {
        return "all";
    }
    AREAS.iter().find(|a| a.2.iter().zip(&v).all(|(x, y)| (x - y).abs() < 0.01)).map(|a| a.0).unwrap_or("custom")
}

/// The four touches as KWin's calibration matrix: for each of x and y, a·rx + b·ry + c = target, least squares
/// (KwinCalibrate.qml's solve), written column by column as a 4×4 matrix (KWin's serializeMatrix).
pub fn solve(points: &[(f32, f32, f32, f32)]) -> String {
    fn row(points: &[(f32, f32, f32, f32)], y: bool) -> Option<[f64; 3]> {
        let mut m = [[0.0f64; 3]; 3];
        let mut v = [0.0f64; 3];
        for &(tx, ty, rx, ry) in points {
            let r = [rx as f64, ry as f64, 1.0];
            let t = if y { ty } else { tx } as f64;
            for i in 0..3 {
                v[i] += r[i] * t;
                for j in 0..3 {
                    m[i][j] += r[i] * r[j];
                }
            }
        }
        for i in 0..3 {
            let mut piv = i;
            for k in i + 1..3 {
                if m[k][i].abs() > m[piv][i].abs() {
                    piv = k;
                }
            }
            m.swap(i, piv);
            v.swap(i, piv);
            if m[i][i].abs() < 1e-9 {
                return None;
            }
            for k in 0..3 {
                if k != i {
                    let f = m[k][i] / m[i][i];
                    for j in 0..3 {
                        m[k][j] -= f * m[i][j];
                    }
                    v[k] -= f * v[i];
                }
            }
        }
        Some([v[0] / m[0][0], v[1] / m[1][1], v[2] / m[2][2]])
    }
    let (Some(a), Some(b)) = (row(points, false), row(points, true)) else { return String::new() };
    let r = |x: f64| {
        let v = (x * 10000.0).round() / 10000.0;
        if v == v.trunc() { format!("{}", v as i64) } else { format!("{v}") }
    };
    [a[0], b[0], 0.0, 0.0, a[1], b[1], 0.0, 0.0, a[2], b[2], 1.0, 0.0, 0.0, 0.0, 0.0, 1.0].iter().map(|x| r(*x)).collect::<Vec<_>>().join(",")
}

fn load() -> Task<M> {
    Task::batch([
        Task::perform(kit::job(kw::H, kit::a(&["input"]), String::new()), M::Input),
        Task::perform(kit::job(kw::H, kit::a(&["outputs"]), String::new()), M::Outputs),
    ])
}

pub fn open(_sh: &mut Shared, s: &mut S, _part: &str) -> Task<M> {
    s.kw.say.clear();
    s.cal = None;
    Task::perform(kw::status(), M::KwStatus)
}

fn set_dev(s: &mut S, sys: &str, props: Value) -> Task<M> {
    // Shown at once; KWin keeps it.
    for d in s.devices.iter_mut() {
        if kit::st(d, "sysName") == sys {
            if let (Some(o), Some(pm)) = (d.as_object_mut(), props.as_object()) {
                for (k, v) in pm {
                    o.insert(k.clone(), v.clone());
                }
            }
        }
    }
    s.saving = true;
    Task::perform(kit::job(kw::H, vec!["input-set".into(), sys.to_string()], props.to_string()), M::SetDone)
}

fn window(mode: iced::window::Mode) -> Task<M> {
    iced::window::latest().and_then(move |id| iced::window::set_mode(id, mode))
}

pub fn update(_sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::KwStatus(r) => match r {
            Res::Ok(v) => {
                let live = kit::bo(&v, "live");
                s.kw.st = Some(v);
                if live {
                    return Task::batch([Task::perform(kw::get(), M::KwGot), load()]);
                }
            }
            Res::Err(e) => {
                s.kw.st = Some(json!({ "live": false }));
                s.kw.say.bad(e);
            }
        },
        M::KwGot(r) => s.kw.take(&r),
        M::Input(r) => match r {
            Res::Ok(v) => s.devices = kit::ar(&v, "devices").to_vec(),
            Res::Err(e) => s.kw.say.bad(e),
        },
        M::Outputs(r) => {
            if let Res::Ok(v) = r {
                s.outs = kit::ar(&v, "outputs").iter().filter(|o| !kit::st(o, "name").is_empty()).cloned().collect();
            }
        }
        M::Set(sys, props) => {
            s.open.clear();
            return set_dev(s, &sys, props);
        }
        M::SetDone(r) => {
            s.saving = false;
            if let Res::Err(e) = r {
                s.kw.say.bad(e);
                return load();
            }
        }
        M::Sync(on) => {
            s.kw.put("kcminputrc", "Tablet", "SyncWithMouse", Some(if on { "true" } else { "false" }));
            return Task::perform(kw::set("kcminputrc", json!({ "Tablet": { "SyncWithMouse": on } })), M::KwSaved);
        }
        M::KwSaved(r) => s.kw.take(&r),
        M::Open(k) => s.open = if s.open == k { String::new() } else { k },
        M::Rebind(pen, button, action) => {
            s.open.clear();
            let input = if action.is_empty() { " ".to_string() } else { action };
            return Task::perform(kit::job(kw::H, vec!["rebind".into(), pen, button], input), M::Rebound);
        }
        M::Rebound(r) => {
            if let Res::Err(e) = r {
                s.kw.say.bad(e);
            }
            return Task::perform(kw::get(), M::KwGot);
        }
        M::Calibrate(sys) => {
            let Some(d) = s.devices.iter().find(|d| kit::st(d, "sysName") == sys).cloned() else { return Task::none() };
            s.points.clear();
            s.cal = Some(d);
            return Task::batch([set_dev(s, &sys, json!({ "calibrationMatrix": IDENTITY })), window(iced::window::Mode::Fullscreen)]);
        }
        M::CalPoint(rx, ry) => {
            if s.cal.is_none() || s.points.len() >= 4 {
                return Task::none();
            }
            let (tx, ty) = TARGETS[s.points.len()];
            s.points.push((tx, ty, rx, ry));
            if s.points.len() == 4 {
                let mtx = solve(&s.points);
                return finish(s, mtx);
            }
        }
        M::CalDone => return finish(s, String::new()),
        M::Range(sys, k, v) => {
            // A slider moves in small steps: shown at once, saved when it's let go.
            let other = if k == "pressureRangeMin" { "pressureRangeMax" } else { "pressureRangeMin" };
            for d in s.devices.iter_mut() {
                if kit::st(d, "sysName") == sys {
                    let o = kit::nu(d, other).unwrap_or(if k == "pressureRangeMin" { 1.0 } else { 0.0 });
                    let v = if k == "pressureRangeMin" { (v as f64).min(o - 0.1).max(0.0) } else { (v as f64).max(o + 0.1).min(1.0) };
                    d[k] = json!((v * 100.0).round() / 100.0);
                }
            }
        }
        M::RangeSave(sys, k) => {
            let v = s.devices.iter().find(|d| kit::st(d, "sysName") == sys).map(|d| kit::at(d, k).clone()).unwrap_or(Value::Null);
            if !v.is_null() {
                let mut props = serde_json::Map::new();
                props.insert(k.to_string(), v);
                return set_dev(s, &sys, Value::Object(props));
            }
        }
        M::Go(id) => _sh.go(id),
    }
    Task::none()
}

/// Calibration done (or given up): the new matrix, else the one from before, else KWin's default.
fn finish(s: &mut S, mtx: String) -> Task<M> {
    let Some(d) = s.cal.take() else { return Task::none() };
    let before = kit::st(&d, "calibrationMatrix");
    let dflt = kit::st(&d, "defaultCalibrationMatrix");
    let m = if !mtx.is_empty() { mtx } else if !before.is_empty() { before } else if !dflt.is_empty() { dflt } else { IDENTITY.to_string() };
    s.points.clear();
    s.cancel = false;
    Task::batch([set_dev(s, &kit::st(&d, "sysName"), json!({ "calibrationMatrix": m })), window(iced::window::Mode::Windowed)])
}

/// The calibration screen: one cross at a time; a touch (or a click) where it is.
pub struct Cal {
    pub step: usize,
    pub accent: Color,
}

impl canvas::Program<M> for Cal {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<M>> {
        let at = |p: Point| M::CalPoint((p.x - bounds.x) / bounds.width.max(1.0), (p.y - bounds.y) / bounds.height.max(1.0));
        match event {
            canvas::Event::Touch(touch::Event::FingerLifted { position, .. }) => Some(Action::publish(at(*position)).and_capture()),
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => cursor.position().map(|p| Action::publish(at(p)).and_capture()),
            _ => None,
        }
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        f.fill_rectangle(Point::ORIGIN, bounds.size(), Color { r: 0.063, g: 0.078, b: 0.094, a: 0.88 });
        if let Some(&(x, y)) = TARGETS.get(self.step) {
            let (cx, cy) = (bounds.width * x, bounds.height * y);
            f.fill_rectangle(Point::new(cx - 30.0, cy - 1.5), Size::new(60.0, 3.0), self.accent);
            f.fill_rectangle(Point::new(cx - 1.5, cy - 30.0), Size::new(3.0, 60.0), self.accent);
            f.stroke(&Path::circle(Point::new(cx, cy), 9.0), Stroke::default().with_color(Color::WHITE).with_width(2.0));
        }
        vec![f.into_geometry()]
    }
}

pub fn dialog<'a>(sh: &'a Shared, s: &'a S) -> Option<(E<'a, M>, Option<M>)> {
    s.cal.as_ref()?;
    let p = sh.p;
    let words_c = container(column![
        tb("Touch the centre of each cross", sz(26.0), Color::WHITE),
        t(format!("{} of 4 · Esc cancels, and nothing is changed", s.points.len() + 1), sz(15.0), Color { a: 0.75, ..Color::WHITE }),
    ].spacing(8.0).align_x(iced::Center))
    .center_x(Length::Fill)
    .center_y(Length::Fill);
    let cal = iced::widget::stack![canvas::Canvas::new(Cal { step: s.points.len(), accent: p.accent }).width(Length::Fill).height(Length::Fill), words_c];
    Some((container(cal).width(Length::Fill).height(Length::Fill).into(), None))
}

impl S {
    fn rebind(&self, pen: &str, button: &str) -> String {
        self.kw.v("kcminputrc", &format!("ButtonRebinds][TabletTool][{pen}"), button).unwrap_or_default()
    }
    fn can_sync(&self) -> bool {
        let v = self.kw.version();
        let mut it = v.split('.').map(|x| x.parse::<u32>().unwrap_or(0));
        let (a, b) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        a > 6 || (a == 6 && b >= 7)
    }
    fn screens(&self) -> Vec<(String, String)> {
        self.outs.iter().map(|o| { let n = kit::st(o, "name"); let m = kit::st(o, "model"); (n.clone(), if m.is_empty() { n } else { format!("{n} · {m}") }) }).collect()
    }
}

pub fn view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let on = s.kw.live();
    let q = if on { p } else { dim(&p) };
    let mut c = crate::secs::Plain::new(0).spacing(10.0);
    c = c.push(kw::gate(&p, &s.kw, "Drawing tablet and touchscreen settings", M::Go("your-desktop")));
    let pens: Vec<&Value> = s.devices.iter().filter(|d| kit::bo(d, "tabletTool")).collect();
    let pads: Vec<&Value> = s.devices.iter().filter(|d| kit::bo(d, "tabletPad")).collect();
    let touches: Vec<&Value> = s.devices.iter().filter(|d| kit::bo(d, "touch")).collect();
    // ------------------------------------------------ pen tablets
    c = c.push(head(&q, if s.saving { "Drawing tablet · saving…" } else { "Drawing tablet" }));
    if pens.is_empty() {
        c = c.push(words(&q, if s.kw.running() || s.kw.preview() { "No drawing tablet is connected. Its settings show here when one is." } else { "Connect a drawing tablet to set it up." }));
    }
    let sync_ok = on && s.can_sync();
    let ver = s.kw.version();
    c = c.push(switch_row(&p, G::Grid, "One pointer for the pen and the mouse", if s.can_sync() { "The pen moves the mouse's pointer, so the pointer is where the pen left it when you pick up the mouse".to_string() } else { format!("Needs KWin 6.7 or newer{}. Until then the pen and the mouse each have their own pointer", if ver.is_empty() { String::new() } else { format!(" (this PC has KWin {ver})") }) }, s.kw.b("kcminputrc", "Tablet", "SyncWithMouse", false), sync_ok, M::Sync(!s.kw.b("kcminputrc", "Tablet", "SyncWithMouse", false))));
    let screens = s.screens();
    for d in &pens {
        let sys = kit::st(d, "sysName");
        let name = kit::st(d, "name");
        c = c.push(head(&q, name.clone()));
        let ws = kit::bo(d, "mapToWorkspace");
        let outn = kit::st(d, "outputName");
        let cur = if ws { String::new() } else { outn.clone() };
        let key = format!("screen-{sys}");
        let mut opts = vec![("Every screen".to_string(), cur.is_empty(), M::Set(sys.clone(), json!({ "mapToWorkspace": true })))];
        for (n, l) in &screens {
            opts.push((l.clone(), cur == *n, M::Set(sys.clone(), json!({ "mapToWorkspace": false, "outputName": n }))));
        }
        let cur_label = if cur.is_empty() { "Every screen".to_string() } else { screens.iter().find(|x| x.0 == cur).map(|x| x.1.clone()).unwrap_or(cur.clone()) };
        c = c.push(dropdown(&q, G::Display, "Screen it draws on", "", cur_label, s.open == key, M::Open(key.clone()), opts));
        if kit::not_false(d, "supportsOutputArea") {
            let a = area_of(kit::at(d, "outputArea"));
            c = c.push(choice(&q, G::Palette, "Part of the screen", "", AREAS.iter().map(|&(v, l, r)| (l.to_string(), a == v, if on { Some(M::Set(sys.clone(), json!({ "outputArea": r }))) } else { None })).collect()));
        }
        if kit::bo(d, "supportsInputArea") {
            let a = area_of(kit::at(d, "inputArea"));
            c = c.push(choice(&q, G::Grid, "Part of the tablet used", "A smaller part means less arm movement", AREAS.iter().map(|&(v, l, r)| (l.to_string(), a == v, if on { Some(M::Set(sys.clone(), json!({ "inputArea": r }))) } else { None })).collect()));
        }
        if kit::bo(d, "supportsLeftHanded") {
            let lh = kit::bo(d, "leftHanded");
            c = c.push(switch_row(&q, G::Grid, "Left-handed (tablet turned round)", "", lh, on, M::Set(sys.clone(), json!({ "leftHanded": !lh }))));
        }
        let curve = kit::st(d, "pressureCurve");
        let ck = format!("curve-{sys}");
        let cv = CURVES.iter().find(|c| c.0 == curve).map(|c| c.0).unwrap_or("0,0;1,1;");
        c = c.push(dropdown(&q, G::Pulse, "Pressure", "", CURVES.iter().find(|c| c.0 == cv).map(|c| c.1).unwrap_or("Even"), s.open == ck, M::Open(ck.clone()), CURVES.iter().map(|&(v, l)| (l.to_string(), cv == v, M::Set(sys.clone(), json!({ "pressureCurve": v })))).collect()));
        if kit::bo(d, "supportsPressureRange") {
            let lo = kit::nu(d, "pressureRangeMin").unwrap_or(0.0);
            let hi = kit::nu(d, "pressureRangeMax").unwrap_or(1.0);
            let (s1, s2) = (sys.clone(), sys.clone());
            c = c.push(srow(&q, G::Pulse, "Lightest touch that counts", format!("{}%", (lo * 100.0).round()), slider_f32(lo as f32, 0.0..=1.0, 0.01, move |v| M::Range(s1.clone(), "pressureRangeMin", v), M::RangeSave(sys.clone(), "pressureRangeMin"))));
            c = c.push(srow(&q, G::Pulse, "Full pressure at", format!("{}%", (hi * 100.0).round()), slider_f32(hi as f32, 0.0..=1.0, 0.01, move |v| M::Range(s2.clone(), "pressureRangeMax", v), M::RangeSave(sys.clone(), "pressureRangeMax"))));
        }
        // BTN_STYLUS and BTN_STYLUS2 (linux/input-event-codes.h): the pen's lower and upper buttons.
        for (code, label) in [("331", "Pen's lower button"), ("332", "Pen's upper button")] {
            let cur = s.rebind(&name, code);
            let bk = format!("btn-{sys}-{code}");
            let mut opts: Vec<(String, bool, M)> = BUTTON_ACTIONS.iter().map(|&(v, l)| (l.to_string(), cur == v, M::Rebind(name.clone(), code.into(), v.into()))).collect();
            // A setting saved before 1.14's button codes is shown as it is, never changed by itself.
            if !BUTTON_ACTIONS.iter().any(|a| a.0 == cur) {
                opts.push((cur.clone(), true, M::Rebind(name.clone(), code.into(), cur.clone())));
            }
            let shown = BUTTON_ACTIONS.iter().find(|a| a.0 == cur).map(|a| a.1.to_string()).unwrap_or(cur.clone());
            c = c.push(dropdown(&q, G::Grid, label, "", shown, s.open == bk, M::Open(bk.clone()), opts));
        }
    }
    if !pads.is_empty() {
        let list: Vec<String> = pads.iter().map(|d| format!("{}: {}", kit::st(d, "name"), kit::nu(d, "tabletPadButtonCount").unwrap_or(0.0) as i64)).collect();
        c = c.push(words(&q, format!("The tablet's own buttons ({}) work as the tablet's maker set them.", list.join(", "))));
    }
    // ------------------------------------------------ touchscreens
    c = c.push(head(&q, "Touchscreen"));
    if touches.is_empty() {
        c = c.push(words(&q, "No touchscreen found."));
    }
    for d in &touches {
        let sys = kit::st(d, "sysName");
        c = c.push(head(&q, kit::st(d, "name")));
        if kit::bo(d, "supportsDisableEvents") {
            let en = kit::not_false(d, "enabled");
            c = c.push(switch_row(&q, G::Pad, "Use this touchscreen", "", en, on, M::Set(sys.clone(), json!({ "enabled": !en }))));
        }
        let outn = kit::st(d, "outputName");
        let key = format!("tscreen-{sys}");
        let opts: Vec<(String, bool, M)> = screens.iter().map(|(n, l)| (l.clone(), outn == *n, M::Set(sys.clone(), json!({ "outputName": n })))).collect();
        c = c.push(dropdown(&q, G::Display, "Screen it's on", "With more than one screen, touches go to this one", if outn.is_empty() { "—".to_string() } else { screens.iter().find(|x| x.0 == outn).map(|x| x.1.clone()).unwrap_or(outn.clone()) }, s.open == key, M::Open(key.clone()), opts));
        let o = kit::nu(d, "orientationDBus").unwrap_or(0.0) as i64;
        c = c.push(choice(&q, G::Update, "Turned", "If touches land rotated from your finger", [(0, "No"), (1, "90° right"), (2, "Upside down"), (3, "90° left")].iter().map(|&(v, l)| (l.to_string(), o == v, if on { Some(M::Set(sys.clone(), json!({ "orientationDBus": v }))) } else { None })).collect()));
        if kit::bo(d, "supportsCalibrationMatrix") {
            let run = s.kw.running();
            c = c.push(press(&q, G::Search, "Calibrate", if run { "Touch four crosses so touches land under your finger" } else { "Needs KWin running" }, if run && on { Some(M::Calibrate(sys.clone())) } else { None }));
            let cm = kit::st(d, "calibrationMatrix");
            let dm = kit::st(d, "defaultCalibrationMatrix");
            if !cm.is_empty() && cm != dm {
                c = c.push(press(&q, G::Restore, "Undo the calibration", "", Some(M::Set(sys.clone(), json!({ "calibrationMatrix": if dm.is_empty() { IDENTITY.to_string() } else { dm } })))));
            }
        }
    }
    c.push(said(&p, &s.kw.say)).into()
}

pub fn back(s: &mut S) -> bool {
    if s.cal.is_some() {
        // Esc during calibration: nothing changed (the matrix from before goes back).
        s.points.clear();
        s.cancel = true;
        return true;
    }
    if !s.open.is_empty() {
        s.open.clear();
        return true;
    }
    false
}

pub fn subscription(s: &S) -> Subscription<M> {
    if s.cancel {
        return iced::time::every(std::time::Duration::from_millis(50)).map(|_| M::CalDone);
    }
    if s.cal.is_some() {
        // Mantel gives up after 20 s with no touch.
        iced::time::every(std::time::Duration::from_secs(20)).map(|_| M::CalDone)
    } else {
        Subscription::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibration_matrix() {
        // Touches exactly on the crosses: no change (the identity, in KWin's form).
        let pts: Vec<_> = TARGETS.iter().map(|&(x, y)| (x, y, x, y)).collect();
        assert_eq!(solve(&pts), IDENTITY);
        // Touches that land shifted right by 0.1: x goes back by 0.1.
        let pts: Vec<_> = TARGETS.iter().map(|&(x, y)| (x, y, x + 0.1, y)).collect();
        assert!(solve(&pts).starts_with("1,0,0,0,0,1,0,0,-0.1,0,1,0"));
        assert_eq!(area_of(&json!([0.5, 0.0, 0.5, 1.0])), "right");
    }
}
