//! Build anything boards 4 and 5 in Mantel in Rust: the block editor, as the QML Mantel's (ScreenEditor.qml). Blocks on
//! the left (press one to add it where there's room), the screen in the middle (drag a block to move it, its corners to
//! size it, on the grid), the picked block's Shows, Look (colours, corners, see-through, font, on focus, sound,
//! animation), Place (above or under windows) and When pressed on the right, "Or say it" under the screen (your AI:
//! hearth-layout's describe_screen, checked; a block Hearth hasn't got is made, boxed in, and asks first: board 5).
//! Undo, Try it full screen, Share it, Keep this. Nothing changes your desktop until Keep this.

use iced::widget::{column, container, mouse_area, pin, row, stack, text, text_input, Column, Row, Space};
use iced::{border, keyboard, mouse, Color, Element, Length, Padding, Point, Task};
use serde_json::{json, Map, Value};

use crate::look::a;
use crate::screen::{self, colour};
use crate::shell::{Message, Shell};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grip {
    Move,
    TopLeft,
    BottomRight,
}

#[derive(Debug, Clone)]
pub enum EMsg {
    Add(String),
    Pick(Option<usize>),
    Grab(usize, Grip),
    Moved(Point),
    Drop,
    Undo,
    Full,
    Share,
    Keep,
    Close,
    Say(String),
    SayGo,
    Said(Value),
    Style(String, Value),
    Field(String, Value),
    Remove,
    Made(String),
}

#[derive(Debug, Default)]
pub struct Editor {
    pub draft: Value,
    pub name: String,
    pub undo: Vec<Value>,
    pub pick: Option<usize>,
    pub said: String,
    pub say: String,
    pub busy: bool,
    pub error: String,
    /// board 5: what your AI made ({block, asks, asked})
    pub made: Option<Value>,
    /// Try it full screen: the draft drawn as your desktop until Escape
    pub trying: bool,
    grab: Option<(usize, Grip, Point, Value)>,
    /// the canvas: where it is on the screen and its size (set by view)
    canvas: std::cell::Cell<(f32, f32, f32, f32)>,
}

const BLOCKS: &[(&str, &str, &str)] = &[
    ("hero", "Hero tile", "window"), ("tilerow", "Tile row", "grid"), ("iconrow", "Icon row", "dot"), ("tabs", "Tabs", "list"),
    ("guide", "Guide menu", "menu"), ("clock", "Clock", "clock"), ("profile", "Profile", "user"), ("battery", "Battery", "batt"),
    ("bar", "Bar", "desktops"), ("dock", "Dock", "home"), ("media", "Media", "headphones"), ("friends", "Friends", "family"),
    ("notes", "Notes", "doc"), ("weather", "Weather", "sun"), ("windows", "Windows", "window"), ("own", "Your own…", "sparkle"),
];
const MORE: &[(&str, &str)] = &[
    ("status", "Status corner"), ("play", "Play"), ("activity", "Activity"), ("widget", "Readout"), ("list", "List"), ("icons", "Desktop icons"),
    ("text", "Words"), ("picture", "Picture"), ("menu", "App menu"), ("gamemode", "Game Mode"), ("search", "Search"), ("downloads", "Downloads"),
];
const COLOURS: &[&str] = &["#ffffff", "#2f9e5a", "#ff7a2f", "#5b8cff", "#b07cff", "#f2c14e", "#e05a6a", "#0b3a44", "#1f3a2e", "#00000000"];
const FONTS: &[&str] = &["", "Inter", "Inter Display", "Noto Serif", "Noto Sans Mono", "Pixel", "Rounded", "Condensed"];
const FOCUS: &[(&str, &str)] = &[("grow", "Grows, white outline"), ("outline", "White outline"), ("glow", "Glows"), ("lift", "Lifts"), ("none", "Nothing")];
const SOUNDS: &[(&str, &str)] = &[("", "The look's own"), ("soft", "Soft click (Hearth's)"), ("click", "Click"), ("pop", "Pop"), ("chime", "Chime"), ("wood", "Wood"), ("retro", "Retro"), ("none", "No sound")];
const ANIMS: &[(&str, &str)] = &[("", "The look's own"), ("slide-left", "Slides in from the left"), ("slide-right", "Slides in from the right"), ("slide-up", "Slides up"), ("slide-down", "Slides down"), ("fade", "Fades in"), ("zoom", "Zooms in"), ("none", "Just there")];
const PRESSES: &[(&str, &str)] = &[("", "What it does anyway"), ("play", "Play it"), ("menu", "Open the app menu"), ("game-mode", "Game Mode"), ("quickaccess", "Quick Access"), ("overview", "All your windows"), ("screenshot", "Take a screenshot"), ("lock", "Lock the screen"), ("none", "Nothing")];

pub fn name_of(b: &str) -> String {
    BLOCKS.iter().find(|x| x.0 == b).map(|x| x.1.to_string()).or_else(|| MORE.iter().find(|x| x.0 == b).map(|x| x.1.to_string())).unwrap_or_else(|| if b.starts_with("own:") { "Made by your AI".into() } else { b.to_string() })
}
fn shows_of(b: &str) -> &'static [(&'static str, &'static str)] {
    match b {
        "hero" => &[("recent", "Your last game, with its art")],
        "tilerow" => &[("recent", "Recent games"), ("games", "All your games"), ("", "Your apps and things"), ("words", "Your apps as words"), ("sticky", "A sticky note of them")],
        "iconrow" => &[("games", "Your games as icons")],
        "clock" => &[("", "The time"), ("date", "The date"), ("ring", "The time in a ring")],
        "widget" => &[("system", "CPU, memory, battery")],
        "list" => &[("games", "Your games, as words"), ("recent", "Recent games, as words")],
        "picture" => &[("", "A picture of yours"), ("fire", "The fireside")],
        _ => &[],
    }
}
fn next<'a>(list: &[(&'a str, &'a str)], v: &str) -> &'a str {
    let i = list.iter().position(|x| x.0 == v).map(|i| i + 1).unwrap_or(0);
    list[i % list.len()].0
}
fn label<'a>(list: &[(&'a str, &'a str)], v: &str) -> &'a str {
    list.iter().find(|x| x.0 == v).map(|x| x.1).unwrap_or("")
}

impl Editor {
    pub fn start(from: Value, name: &str) -> Editor {
        let mut d = if from.is_object() { from } else { json!({}) };
        if d.get("grid").is_none() {
            d["grid"] = json!({ "cols": 24, "rows": 14 });
        }
        if !d.get("blocks").map(Value::is_array).unwrap_or(false) {
            d["blocks"] = json!([]);
        }
        let said = d.get("said").and_then(Value::as_str).unwrap_or("").to_string();
        let pick = if d["blocks"].as_array().map(|a| a.is_empty()).unwrap_or(true) { None } else { Some(0) };
        Editor { draft: d, name: if name.is_empty() { "Your desktop".into() } else { name.into() }, said, pick, ..Default::default() }
    }
    fn blocks(&self) -> Vec<Value> {
        self.draft.get("blocks").and_then(Value::as_array).cloned().unwrap_or_default()
    }
    fn cur(&self) -> Option<Value> {
        self.pick.and_then(|i| self.blocks().get(i).cloned())
    }
    fn grid(&self) -> (u32, u32) {
        let g = &self.draft["grid"];
        (g.get("cols").and_then(Value::as_u64).unwrap_or(24).max(1) as u32, g.get("rows").and_then(Value::as_u64).unwrap_or(14).max(1) as u32)
    }
    fn change(&mut self, f: impl FnOnce(&mut Value)) {
        self.undo.push(self.draft.clone());
        if self.undo.len() > 60 {
            self.undo.remove(0);
        }
        f(&mut self.draft);
    }
    pub fn own_ids(&self) -> Vec<String> {
        self.made.as_ref().and_then(|m| m.pointer("/block/id")).and_then(Value::as_str).map(|s| vec![s.to_string()]).unwrap_or_default()
    }
    fn free_spot(&self, w: u32, h: u32) -> (u32, u32) {
        let (c, r) = self.grid();
        let bl = self.blocks();
        let n = |b: &Value, k: &str| b.get(k).and_then(Value::as_u64).unwrap_or(0) as u32;
        for y in 0..=r.saturating_sub(h) {
            for x in 0..=c.saturating_sub(w) {
                if !bl.iter().any(|b| x < n(b, "x") + n(b, "w") && n(b, "x") < x + w && y < n(b, "y") + n(b, "h") && n(b, "y") < y + h) {
                    return (x, y);
                }
            }
        }
        (0, 0)
    }
}

pub fn update(sh: &mut Shell, m: EMsg) -> Task<Message> {
    let ed = &mut sh.editor;
    match m {
        EMsg::Add(b) => {
            if b == "own" {
                ed.say = "A block that ".into();
                return Task::none();
            }
            let (w, h) = match b.as_str() {
                "hero" => (8, 5), "tilerow" => (16, 4), "iconrow" => (14, 3), "tabs" => (10, 1), "guide" => (6, 14), "clock" => (5, 1), "profile" => (6, 1),
                "battery" => (3, 1), "bar" => (24, 1), "dock" => (10, 2), "media" => (8, 2), "friends" => (6, 3), "notes" => (8, 6), "weather" => (8, 1),
                "windows" => (8, 6), "status" => (7, 1), "play" => (14, 3), "activity" => (15, 3), "widget" => (6, 4), "list" => (6, 5), "icons" => (6, 6),
                "text" => (8, 1), "picture" => (8, 6), "menu" => (2, 2), "gamemode" => (4, 1), "search" => (6, 1), _ => (4, 3),
            };
            let (gc, gr) = ed.grid();
            let (w, h) = (w.min(gc), h.min(gr));
            let (x, y) = ed.free_spot(w, h);
            let ids: Vec<String> = ed.blocks().iter().filter_map(|b| b.get("id").and_then(Value::as_str).map(str::to_string)).collect();
            let base = b.trim_start_matches("own:").to_string();
            let mut id = base.clone();
            let mut k = 2;
            while ids.contains(&id) {
                id = format!("{base}-{k}");
                k += 1;
            }
            let mut nb = json!({ "id": id, "block": b, "x": x, "y": y, "w": w, "h": h });
            match b.as_str() {
                "tilerow" => nb["items"] = json!(["games", "store", "files", "browser", "music"]),
                "tabs" => nb["items"] = json!(["games", "music", "windows"]),
                "bar" => nb["items"] = json!(["menu", "windows"]),
                "icons" => nb["items"] = json!(["home", "bin", "files", "games"]),
                "notes" => { nb["title"] = json!("Notes"); nb["value"] = json!("☐ "); }
                "text" => nb["value"] = json!("Your words"),
                "widget" => nb["shows"] = json!("system"),
                "iconrow" | "list" => nb["shows"] = json!("games"),
                "hero" => nb["shows"] = json!("recent"),
                _ => {}
            }
            ed.change(|d| d["blocks"].as_array_mut().map(|a| a.push(nb)).unwrap_or(()));
            ed.pick = Some(ed.blocks().len() - 1);
        }
        EMsg::Pick(i) => ed.pick = i,
        EMsg::Grab(i, g) => {
            ed.pick = Some(i);
            let b = ed.blocks().get(i).cloned().unwrap_or(Value::Null);
            ed.grab = Some((i, g, Point::new(-1.0, -1.0), b));
            ed.undo.push(ed.draft.clone());
        }
        EMsg::Moved(p) => {
            let Some((i, g, start, orig)) = ed.grab.clone() else { return Task::none() };
            if start.x < 0.0 {
                ed.grab = Some((i, g, p, orig));
                return Task::none();
            }
            let (_, _, cw, ch) = ed.canvas.get();
            let (gc, gr) = ed.grid();
            let (cellw, cellh) = (cw / gc as f32, ch / gr as f32);
            let dx = ((p.x - start.x) / cellw).round() as i64;
            let dy = ((p.y - start.y) / cellh).round() as i64;
            let n = |k: &str| orig.get(k).and_then(Value::as_i64).unwrap_or(0);
            let (ox, oy, ow, oh) = (n("x"), n("y"), n("w"), n("h"));
            let (gc, gr) = (gc as i64, gr as i64);
            let (x, y, w, h) = match g {
                Grip::Move => ((ox + dx).clamp(0, gc - ow), (oy + dy).clamp(0, gr - oh), ow, oh),
                Grip::BottomRight => (ox, oy, (ow + dx).clamp(1, gc - ox), (oh + dy).clamp(1, gr - oy)),
                Grip::TopLeft => {
                    let nx = (ox + dx).clamp(0, ox + ow - 1);
                    let ny = (oy + dy).clamp(0, oy + oh - 1);
                    (nx, ny, ow + (ox - nx), oh + (oy - ny))
                }
            };
            if let Some(b) = ed.draft["blocks"].get_mut(i) {
                b["x"] = json!(x);
                b["y"] = json!(y);
                b["w"] = json!(w);
                b["h"] = json!(h);
            }
        }
        EMsg::Drop => {
            if let Some((_, _, _, orig)) = ed.grab.take() {
                // nothing moved: nothing to undo
                if ed.pick.and_then(|i| ed.blocks().get(i).cloned()) == Some(orig) {
                    ed.undo.pop();
                }
            }
        }
        EMsg::Undo => {
            if let Some(d) = ed.undo.pop() {
                ed.draft = d;
                let n = ed.blocks().len();
                ed.pick = ed.pick.filter(|i| *i < n);
            }
        }
        EMsg::Full => {
            ed.trying = true;
            let t = crate::shell::try_full(sh, true);
            return t;
        }
        EMsg::Share => {
            let idea = json!({ "format": 1, "part": "screen", "asked": if ed.said.is_empty() { ed.name.clone() } else { ed.said.clone() }, "needs": [format!("A desktop shared with Hearth: {}", ed.name)], "sketch": false, "hearth": "", "app": "Mantel in Rust", "kept": true });
            let _ = std::process::Command::new(screen::layout_bin()).args(["send-idea", &idea.to_string()]).spawn();
            ed.error = "Sent to Hearth's makers: if they share it, it shows under Made by people ✓".into();
        }
        EMsg::Keep => {
            let mut d = ed.draft.clone();
            d["said"] = json!(if ed.said.is_empty() { d.get("said").cloned().unwrap_or(json!("")) } else { json!(ed.said) });
            if let Err(e) = hearth_layout::screen::parse(&d) {
                ed.error = e.join(" ");
                return Task::none();
            }
            if sh.screen.unreadable {
                sh.editor.error = "Your desktop's file couldn't be read, so nothing was written over it.".into();
                return Task::none();
            }
            let mut m = Map::new();
            m.insert("on".into(), json!(true));
            m.insert("chosen".into(), json!(true));
            m.insert("from".into(), json!(""));
            m.insert("said".into(), d["said"].clone());
            m.insert("screen".into(), d);
            if let Some(dir) = screen::file().parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = hearth_ui::settings_file::merge_json(&screen::file(), &m);
            return Task::batch([crate::shell::close_editor(sh), Task::done(Message::DeskCheck)]);
        }
        EMsg::Close => {
            if ed.made.is_some() {
                ed.made = None;
                return Task::none();
            }
            return crate::shell::close_editor(sh);
        }
        EMsg::Say(t) => ed.say = t,
        EMsg::SayGo => {
            let t = ed.say.trim().to_string();
            if t.is_empty() || ed.busy {
                return Task::none();
            }
            ed.busy = true;
            ed.error.clear();
            let cur = ed.draft.clone();
            return Task::perform(async move { let mut v = hearth_layout::describe::describe_screen(&t, None, &cur); v["_asked"] = json!(t); v }, |v| Message::Editor(EMsg::Said(v)));
        }
        EMsg::Said(r) => {
            ed.busy = false;
            if r.get("ok") != Some(&json!(true)) {
                ed.error = r.get("error").and_then(Value::as_str).unwrap_or("Your AI didn't answer.").to_string();
                return Task::none();
            }
            let asked = r.get("_asked").and_then(Value::as_str).unwrap_or("").to_string();
            let scr = r.get("screen").cloned().unwrap_or(Value::Null);
            ed.change(|d| *d = scr);
            ed.said = asked.clone();
            ed.pick = if ed.blocks().is_empty() { None } else { Some(0) };
            if let Some(made) = r.get("made").and_then(Value::as_array).filter(|a| !a.is_empty()) {
                for b in made {
                    if let Ok(m) = hearth_layout::made::parse(b) {
                        let _ = hearth_layout::made::keep(&m);
                    }
                }
                let first = made[0].clone();
                let id = first.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                ed.pick = ed.blocks().iter().position(|b| b.get("block").and_then(Value::as_str) == Some(&format!("own:{id}")));
                ed.made = Some(json!({ "block": first, "asks": r.get("asks").and_then(|a| a.get(0)).cloned().unwrap_or(Value::Null), "asked": asked }));
                return Task::done(Message::OwnTick);
            }
        }
        EMsg::Style(k, v) => {
            let Some(i) = ed.pick else { return Task::none() };
            if k == "sound" {
                screen::sound(v.as_str().unwrap_or(""));
            }
            ed.change(|d| {
                if let Some(b) = d["blocks"].get_mut(i) {
                    if !b.get("style").map(Value::is_object).unwrap_or(false) {
                        b["style"] = json!({});
                    }
                    let st = b["style"].as_object_mut().unwrap();
                    if v.is_null() || v == json!("") || v == json!(false) {
                        st.remove(&k);
                    } else {
                        st.insert(k, v);
                    }
                }
            });
        }
        EMsg::Field(k, v) => {
            let Some(i) = ed.pick else { return Task::none() };
            ed.change(|d| {
                if let Some(b) = d["blocks"].get_mut(i).and_then(Value::as_object_mut) {
                    if v.is_null() || v == json!("") {
                        b.remove(&k);
                    } else {
                        b.insert(k, v);
                    }
                }
            });
        }
        EMsg::Remove => {
            let Some(i) = ed.pick else { return Task::none() };
            ed.change(|d| {
                if let Some(a) = d["blocks"].as_array_mut() {
                    if i < a.len() {
                        a.remove(i);
                    }
                }
            });
            let n = ed.blocks().len();
            ed.pick = if n == 0 { None } else { Some(i.min(n - 1)) };
        }
        EMsg::Made(what) => {
            let made = ed.made.clone().unwrap_or(Value::Null);
            let id = made.pointer("/block/id").and_then(Value::as_str).unwrap_or("").to_string();
            let asked = made.get("asked").and_then(Value::as_str).unwrap_or("").to_string();
            match what.as_str() {
                "keep" => ed.made = None,
                "change" => {
                    ed.say = format!("{asked}, and ");
                    ed.made = None;
                }
                "again" => {
                    ed.say = asked;
                    ed.made = None;
                    return Task::done(Message::Editor(EMsg::SayGo));
                }
                "undo" => {
                    let _ = hearth_layout::made::set_on(&id, false);
                    ed.made = None;
                    if let Some(d) = ed.undo.pop() {
                        ed.draft = d;
                    }
                }
                "share" => {
                    let idea = json!({ "format": 1, "part": "screen", "asked": asked, "needs": [format!("A block made by your AI: {}", made.pointer("/block/name").and_then(Value::as_str).unwrap_or(""))], "sketch": false, "hearth": "", "app": "Mantel in Rust", "kept": true });
                    let _ = std::process::Command::new(screen::layout_bin()).args(["send-idea", &idea.to_string()]).spawn();
                    ed.error = "Sent to Hearth's makers ✓".into();
                }
                a @ ("always" | "no") => {
                    let _ = hearth_layout::made::allow(&id, a);
                    return Task::done(Message::OwnTick);
                }
                "once" => return Task::done(Message::OwnAllow(id, "once".into())),
                _ => {}
            }
        }
    }
    Task::none()
}

/// Keys in the editor: Escape (back, or close), Delete (the picked block), Ctrl+Z (Undo).
pub fn key(sh: &mut Shell, k: keyboard::Event) -> Task<Message> {
    if let keyboard::Event::KeyPressed { key, modifiers, .. } = k {
        match key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::Escape) => {
                if sh.editor.trying {
                    sh.editor.trying = false;
                    return crate::shell::try_full(sh, false);
                }
                return update(sh, EMsg::Close);
            }
            keyboard::Key::Named(keyboard::key::Named::Delete) if sh.editor.say.is_empty() => return update(sh, EMsg::Remove),
            keyboard::Key::Character("z") if modifiers.control() => return update(sh, EMsg::Undo),
            _ => {}
        }
    }
    Task::none()
}

fn e<'a>(m: EMsg) -> Message {
    Message::Editor(m)
}

fn chip<'a>(sh: &'a Shell, t: String, m: EMsg) -> Element<'a, Message> {
    let l = &sh.look;
    mouse_area(container(l.t(t, 13.0, Color::from_rgb8(0xe8, 0xdd, 0xd4))).padding(Padding { left: 12.0, right: 12.0, top: 6.0, bottom: 6.0 }).style(|_| container::Style { border: iced::Border { color: a(Color::WHITE, 0.25), width: 1.0, radius: 15.0.into() }, ..Default::default() }))
        .on_press(e(m))
        .interaction(mouse::Interaction::Pointer)
        .into()
}
fn line<'a>(sh: &'a Shell, k: &'static str, v: String, m: EMsg) -> Element<'a, Message> {
    let l = &sh.look;
    row![
        container(l.t(k, 14.0, Color::from_rgb8(0xc9, 0xbf, 0xb6))).width(120),
        mouse_area(container(l.t(v, 13.0, Color::from_rgb8(0xe8, 0xdd, 0xd4))).padding(Padding { left: 12.0, right: 12.0, top: 6.0, bottom: 6.0 }).width(Length::Fill).style(|_| container::Style { border: iced::Border { color: a(Color::WHITE, 0.2), width: 1.0, radius: 8.0.into() }, ..Default::default() })).on_press(e(m)).interaction(mouse::Interaction::Pointer)
    ]
    .align_y(iced::Center)
    .into()
}
fn slider<'a>(sh: &'a Shell, k: &'static str, v: f32, max: f32, key: &'static str) -> Element<'a, Message> {
    let l = &sh.look;
    let steps: Vec<f32> = (0..=8).map(|i| max * i as f32 / 8.0).collect();
    let mut r = Row::new().spacing(3).align_y(iced::Center);
    for s in steps {
        let on = s <= v;
        r = r.push(mouse_area(container(Space::new()).width(22).height(6).style(move |_| container::Style { background: Some(if on { Color::from_rgb8(0xff, 0x7a, 0x2f) } else { a(Color::WHITE, 0.15) }.into()), border: border::rounded(2), ..Default::default() })).on_press(e(EMsg::Style(key.into(), json!(s.round() as u32)))).interaction(mouse::Interaction::Pointer));
    }
    row![container(l.t(k, 14.0, Color::from_rgb8(0xc9, 0xbf, 0xb6))).width(120), r].align_y(iced::Center).into()
}
fn btn<'a>(sh: &'a Shell, t: &'static str, prim: bool, m: EMsg) -> Element<'a, Message> {
    let l = &sh.look;
    let orange = Color::from_rgb8(0xff, 0x7a, 0x2f);
    mouse_area(container(l.tb(t, 15.0, if prim { Color::from_rgb8(0x1b, 0x10, 0x06) } else { Color::WHITE })).padding(Padding { left: 18.0, right: 18.0, top: 8.0, bottom: 8.0 }).style(move |_| container::Style { background: Some(if prim { orange } else { a(Color::WHITE, 0.07) }.into()), border: iced::Border { color: a(Color::WHITE, if prim { 0.0 } else { 0.18 }), width: 1.0, radius: 6.0.into() }, ..Default::default() }))
        .on_press(e(m))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

pub fn view(sh: &Shell) -> Element<'_, Message> {
    let ed = &sh.editor;
    let l = &sh.look;
    let (sw, shh) = (sh.screen_size.width.max(1280.0), sh.screen_size.height.max(720.0));
    let muted = Color::from_rgb8(0xa5, 0x9a, 0x91);
    let ink = Color::from_rgb8(0xe8, 0xdd, 0xd4);
    let orange = Color::from_rgb8(0xff, 0x7a, 0x2f);
    if let Some(made) = &ed.made {
        return made_view(sh, made);
    }
    // ---- the top bar
    let top = container(row![
        l.tb("Your desktop", 17.0, Color::WHITE),
        container(l.t(format!("\"{}\"", ed.name), 14.0, ink)).padding(Padding { left: 13.0, right: 13.0, top: 4.0, bottom: 4.0 }).style(|_| container::Style { border: iced::Border { color: a(Color::WHITE, 0.25), width: 1.0, radius: 14.0.into() }, ..Default::default() }),
        Space::new().width(Length::Fill),
        btn(sh, "Undo", false, EMsg::Undo),
        btn(sh, "Try it full screen", false, EMsg::Full),
        btn(sh, "Share it", false, EMsg::Share),
        btn(sh, "Keep this", true, EMsg::Keep)
    ]
    .spacing(12)
    .align_y(iced::Center))
    .padding(Padding { left: 24.0, right: 24.0, ..Padding::ZERO })
    .center_y(56)
    .width(Length::Fill)
    .style(|_| container::Style { background: Some(Color::from_rgb8(0x21, 0x1c, 0x19).into()), ..Default::default() });
    // ---- the blocks
    let mut grid = Column::new().spacing(10);
    for pair in BLOCKS.chunks(2) {
        let mut r = Row::new().spacing(10);
        for (b, t, g) in pair {
            r = r.push(mouse_area(container(row![crate::look::icon(g, 15.0, ink), l.t(*t, 14.0, ink)].spacing(8).align_y(iced::Center)).padding(Padding { left: 12.0, ..Padding::ZERO }).center_y(40).width(125).style(|_| container::Style { background: Some(Color::from_rgb8(0x2a, 0x24, 0x20).into()), border: border::rounded(6), ..Default::default() })).on_press(e(EMsg::Add(b.to_string()))).interaction(mouse::Interaction::Pointer));
        }
        grid = grid.push(r);
    }
    let mut more = Column::new().spacing(6);
    for chunk in MORE.chunks(3) {
        let mut r = Row::new().spacing(6);
        for (b, t) in chunk {
            r = r.push(mouse_area(container(l.t(*t, 12.0, ink)).padding(Padding { left: 9.0, right: 9.0, top: 5.0, bottom: 5.0 }).style(|_| container::Style { background: Some(Color::from_rgb8(0x2a, 0x24, 0x20).into()), border: border::rounded(14), ..Default::default() })).on_press(e(EMsg::Add(b.to_string()))).interaction(mouse::Interaction::Pointer));
        }
        more = more.push(r);
    }
    let left = container(column![
        l.t("BLOCKS · DRAG ONTO THE SCREEN", 12.0, muted),
        grid,
        l.t("MORE", 12.0, muted),
        more,
        mouse_area(container(l.t("+ Describe a block that isn't here, and your AI makes it", 14.0, Color::from_rgb8(0xff, 0x9a, 0x52))).padding(14).width(Length::Fill).style(move |_| container::Style { border: iced::Border { color: orange, width: 1.0, radius: 8.0.into() }, ..Default::default() })).on_press(e(EMsg::Add("own".into()))).interaction(mouse::Interaction::Pointer),
        l.t("MADE BY PEOPLE", 12.0, muted),
        l.t("Blocks people share with Hearth show here once Hearth's makers have checked them.", 13.0, Color::from_rgb8(0x8a, 0x80, 0x79)),
    ]
    .spacing(10))
    .padding(20)
    .width(300)
    .height(Length::Fill)
    .style(|_| container::Style { background: Some(Color::from_rgb8(0x1f, 0x1a, 0x17).into()), ..Default::default() });
    // ---- the screen
    let mid_w = sw - 300.0 - 380.0 - 60.0;
    let mid_h = shh - 56.0 - 60.0 - 90.0;
    let cw = mid_w.min(mid_h * 16.0 / 9.0);
    let ch = cw * 9.0 / 16.0;
    ed.canvas.set((300.0 + 30.0 + (mid_w - cw) / 2.0, 56.0 + 30.0, cw, ch));
    let (gc, gr) = ed.grid();
    let (cellw, cellh) = (cw / gc as f32, ch / gr as f32);
    let bg = ed.draft.get("background").cloned().unwrap_or(json!({}));
    let from = colour(bg.get("from").and_then(Value::as_str).unwrap_or("#10243a"), Color::from_rgb8(0x10, 0x24, 0x3a));
    let to = if bg.get("kind").and_then(Value::as_str) == Some("colour") { from } else { colour(bg.get("to").and_then(Value::as_str).unwrap_or("#1a0d07"), from) };
    let mut layers: Vec<Element<Message>> = vec![mouse_area(container(Space::new()).width(cw).height(ch).style(move |_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, from).add_stop(1.0, to)).into()), border: border::rounded(10), ..Default::default() })).on_press(e(EMsg::Pick(None))).into()];
    for i in 1..gc {
        layers.push(pin(container(Space::new()).width(1).height(ch).style(|_| container::Style { background: Some(a(Color::WHITE, 0.05).into()), ..Default::default() })).x(i as f32 * cellw).y(0.0).into());
    }
    for j in 1..gr {
        layers.push(pin(container(Space::new()).width(cw).height(1).style(|_| container::Style { background: Some(a(Color::WHITE, 0.05).into()), ..Default::default() })).x(0.0).y(j as f32 * cellh).into());
    }
    for (i, b) in ed.blocks().iter().enumerate() {
        let n = |k: &str| b.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let (bx, by, bw, bh) = (n("x") * cellw + 4.0, n("y") * cellh + 4.0, (n("w") * cellw - 8.0).max(8.0), (n("h") * cellh - 8.0).max(8.0));
        let on = ed.pick == Some(i);
        let blk = b.get("block").and_then(Value::as_str).unwrap_or("");
        let st = b.get("style").cloned().unwrap_or(json!({}));
        let fill = st.get("colour").and_then(Value::as_str).filter(|c| *c != "#00000000").map(|c| { let c = colour(c, Color::BLACK); a(c, 0.45) }).unwrap_or(a(Color::BLACK, 0.25));
        let title = b.get("title").and_then(Value::as_str).unwrap_or("");
        let items = b.get("items").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
        let lab = if blk == "hero" { "Hero tile · your last game".to_string() } else { format!("{}{}", name_of(blk), if !title.is_empty() { format!(" · {title}") } else if items > 0 { format!(" · {items} things") } else { String::new() }) };
        let bxv = container(if blk == "hero" { container(l.tb(lab, 16.0, Color::WHITE)).padding(12).align_bottom(Length::Fill) } else { container(l.t(lab, 14.0, Color::WHITE)).center_x(Length::Fill).center_y(Length::Fill) })
            .width(bw)
            .height(bh)
            .style(move |_| container::Style { background: Some(fill.into()), border: iced::Border { color: if on { orange } else { a(Color::WHITE, 0.35) }, width: if on { 2.0 } else { 1.0 }, radius: 6.0.into() }, ..Default::default() });
        layers.push(pin(mouse_area(bxv).on_press(e(EMsg::Grab(i, Grip::Move))).interaction(mouse::Interaction::Grab)).x(bx).y(by).into());
        if on {
            let handle = |g: Grip| mouse_area(container(Space::new()).width(12).height(12).style(move |_| container::Style { background: Some(orange.into()), border: border::rounded(2), ..Default::default() })).on_press(e(EMsg::Grab(i, g))).interaction(mouse::Interaction::ResizingDiagonallyDown);
            layers.push(pin(handle(Grip::TopLeft)).x(bx - 6.0).y(by - 6.0).into());
            layers.push(pin(handle(Grip::BottomRight)).x(bx + bw - 6.0).y(by + bh - 6.0).into());
        }
    }
    let canvas = mouse_area(stack(layers).width(cw).height(ch)).on_move(|p| e(EMsg::Moved(p))).on_release(e(EMsg::Drop));
    let note: Element<Message> = if ed.busy { l.t("Your AI is making it…", 14.0, muted).into() } else if !ed.error.is_empty() { l.t(ed.error.clone(), 14.0, Color::from_rgb8(0xff, 0x9a, 0x52)).into() } else { Space::new().into() };
    let say = container(text_input("Or say it: \"make the hero tile bigger and add my printer's camera in the corner\"…", &ed.say).on_input(|t| e(EMsg::Say(t))).on_submit(e(EMsg::SayGo)).size(16).padding(14).font(l.font))
        .width(cw)
        .style(move |_| container::Style { background: Some(Color::from_rgb8(0x21, 0x1c, 0x19).into()), border: iced::Border { color: orange, width: 2.0, radius: 10.0.into() }, ..Default::default() });
    let middle = container(column![canvas, note, Space::new().height(Length::Fill), say].spacing(12).align_x(iced::Center)).padding(30).width(Length::Fill).height(Length::Fill);
    // ---- the picked block
    let right: Element<Message> = match ed.cur() {
        None => container(l.t("Pick a block on the screen, or add one from the left.", 14.0, muted)).center_x(380).center_y(Length::Fill).into(),
        Some(b) => {
            let blk = b.get("block").and_then(Value::as_str).unwrap_or("").to_string();
            let st = b.get("style").cloned().unwrap_or(json!({}));
            let sv = |k: &str| st.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            let nv = |k: &str| st.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let shows = b.get("shows").and_then(Value::as_str).unwrap_or("").to_string();
            let mut c = Column::new().spacing(10).push(l.t(name_of(&blk).to_uppercase(), 13.0, ink));
            c = c.push(l.t("SHOWS", 12.0, muted));
            let so = shows_of(&blk);
            if !so.is_empty() {
                c = c.push(chip(sh, label(so, &shows).to_string(), EMsg::Field("shows".into(), json!(next(so, &shows)))));
            }
            if ["notes", "text", "tilerow", "hero", "windows", "activity"].contains(&blk.as_str()) {
                let t = b.get("title").and_then(Value::as_str).unwrap_or("").to_string();
                c = c.push(text_input("Its title (\"Jump back in\")", &t).on_input(|v| e(EMsg::Field("title".into(), json!(v)))).size(13).padding(8));
            }
            if ["notes", "text"].contains(&blk.as_str()) {
                let v = b.get("value").and_then(Value::as_str).unwrap_or("").to_string();
                c = c.push(text_input("Your words", &v).on_input(|v| e(EMsg::Field("value".into(), json!(v)))).size(13).padding(8));
            }
            c = c.push(l.t("LOOK", 12.0, muted));
            let mut sw_r = Row::new().spacing(8);
            for col in COLOURS.iter().take(4) {
                let cc = colour(col, Color::WHITE);
                let on = sv("colour") == *col;
                sw_r = sw_r.push(mouse_area(container(Space::new()).width(28).height(28).style(move |_| container::Style { background: Some(cc.into()), border: iced::Border { color: if on { orange } else { a(Color::WHITE, 0.3) }, width: if on { 3.0 } else { 1.0 }, radius: 14.0.into() }, ..Default::default() })).on_press(e(EMsg::Style("colour".into(), json!(col)))).interaction(mouse::Interaction::Pointer));
            }
            let cur_c = sv("colour");
            let nxt = COLOURS[(COLOURS.iter().position(|x| *x == cur_c).map(|i| i + 1).unwrap_or(0)) % COLOURS.len()];
            sw_r = sw_r.push(mouse_area(container(Space::new()).width(28).height(28).style(|_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2).add_stop(0.0, Color::from_rgb8(0xff, 0x4d, 0x4d)).add_stop(0.33, Color::from_rgb8(0xff, 0xd2, 0x4d)).add_stop(0.66, Color::from_rgb8(0x4d, 0xff, 0x88)).add_stop(1.0, Color::from_rgb8(0x4d, 0x7a, 0xff))).into()), border: border::rounded(14), ..Default::default() })).on_press(e(EMsg::Style("colour".into(), json!(nxt)))).interaction(mouse::Interaction::Pointer));
            c = c.push(sw_r);
            c = c.push(slider(sh, "Corners", nv("corners"), 64.0, "corners"));
            c = c.push(slider(sh, "See-through", nv("seeThrough"), 100.0, "seeThrough"));
            let f = sv("font");
            let fnext = FONTS[(FONTS.iter().position(|x| *x == f).map(|i| i + 1).unwrap_or(0)) % FONTS.len()];
            c = c.push(line(sh, "Font", if f.is_empty() { "The look's own".into() } else { f.clone() }, EMsg::Style("font".into(), json!(fnext))));
            let fo = if sv("focus").is_empty() { "grow".to_string() } else { sv("focus") };
            c = c.push(line(sh, "On focus", label(FOCUS, &fo).into(), EMsg::Style("focus".into(), json!(next(FOCUS, &fo)))));
            c = c.push(line(sh, "Sound", label(SOUNDS, &sv("sound")).into(), EMsg::Style("sound".into(), json!(next(SOUNDS, &sv("sound"))))));
            c = c.push(line(sh, "Animation", label(ANIMS, &sv("animation")).into(), EMsg::Style("animation".into(), json!(next(ANIMS, &sv("animation"))))));
            c = c.push(l.t("PLACE", 12.0, muted));
            let n = |k: &str| b.get(k).and_then(Value::as_u64).unwrap_or(0);
            c = c.push(l.t(format!("Column {}, row {} · {} × {} squares", n("x") + 1, n("y") + 1, n("w"), n("h")), 14.0, Color::from_rgb8(0xc9, 0xbf, 0xb6)));
            let above = st.get("aboveWindows") == Some(&json!(true));
            c = c.push(mouse_area(l.t(format!("Above other windows: {}", if above { "yes (over them)" } else { "no (under them)" }), 14.0, Color::from_rgb8(0xc9, 0xbf, 0xb6))).on_press(e(EMsg::Style("aboveWindows".into(), json!(!above)))).interaction(mouse::Interaction::Pointer));
            c = c.push(l.t("WHEN PRESSED", 12.0, muted));
            let pa = b.pointer("/press/action").and_then(Value::as_str).unwrap_or("").to_string();
            let pn = next(PRESSES, &pa);
            c = c.push(chip(sh, label(PRESSES, &pa).into(), EMsg::Field("press".into(), if pn.is_empty() { Value::Null } else { json!({ "action": pn }) })));
            c = c.push(Space::new().height(14)).push(chip(sh, "Remove this block".into(), EMsg::Remove));
            container(iced::widget::scrollable(c)).padding(22).width(380).height(Length::Fill).into()
        }
    };
    let right = container(right).width(380).height(Length::Fill).style(|_| container::Style { background: Some(Color::from_rgb8(0x1f, 0x1a, 0x17).into()), ..Default::default() });
    container(column![top, row![left, middle, right].height(Length::Fill)]).width(Length::Fill).height(Length::Fill).style(|_| container::Style { background: Some(Color::from_rgb8(0x1b, 0x17, 0x15).into()), ..Default::default() }).into()
}

/// Board 5: a block your AI made: what it shows (live), what you asked, how it's boxed in, its ask, and what next.
fn made_view<'a>(sh: &'a Shell, made: &Value) -> Element<'a, Message> {
    let l = &sh.look;
    let orange = Color::from_rgb8(0xff, 0x7a, 0x2f);
    let muted = Color::from_rgb8(0xa5, 0x9a, 0x91);
    let id = made.pointer("/block/id").and_then(Value::as_str).unwrap_or("").to_string();
    let name = made.pointer("/block/name").and_then(Value::as_str).unwrap_or("").to_string();
    let by = made.pointer("/block/by").and_then(Value::as_str).unwrap_or("your AI").to_string();
    let asked = made.get("asked").and_then(Value::as_str).unwrap_or("").to_string();
    let w = (sh.screen_size.width - 200.0) * 0.47;
    let preview = container(crate::screen::own_view(sh, &id, w, w * 0.52)).width(w).height(w * 0.52);
    let left = column![l.tb("MADE FOR YOU BY YOUR AI · NEW", 14.0, orange), preview, l.t(format!("{name} as a block on your desktop. Made by {by}; it's yours, no one else's desktop changes."), 15.0, Color::from_rgb8(0xc9, 0xbf, 0xb6))].spacing(14).width(w);
    let boxed = |inner: Element<'a, Message>| container(inner).padding(20).width(Length::Fill).style(|_| container::Style { background: Some(Color::from_rgb8(0x22, 0x1d, 0x1a).into()), border: iced::Border { color: a(Color::WHITE, 0.08), width: 1.0, radius: 8.0.into() }, ..Default::default() });
    let mut r = Column::new().spacing(14);
    r = r.push(l.t("WHAT YOU ASKED", 13.0, muted)).push(boxed(l.t(format!("\"{asked}\""), 19.0, Color::WHITE).into()));
    r = r.push(l.t("IT RUNS ON ITS OWN, SO NOTHING ELSE CAN BREAK", 13.0, muted));
    r = r.push(boxed(column![
        l.t("✓  Its own small program: stopped by itself if it uses too much processor or memory", 15.0, Color::WHITE),
        l.t("✓  Can't touch starting up, Hearth's system or your settings", 15.0, Color::WHITE),
        l.t("✓  Turn it off any time in Settings › Your desktop › Your blocks", 15.0, Color::WHITE)
    ]
    .spacing(12)
    .into()));
    if let Some(asks) = made.get("asks").filter(|x| !x.is_null()) {
        r = r.push(container(column![
            l.tb(asks.get("title").and_then(Value::as_str).unwrap_or("").to_string(), 17.0, Color::WHITE),
            l.t(asks.get("sub").and_then(Value::as_str).unwrap_or("").to_string(), 14.0, Color::from_rgb8(0xc9, 0xbf, 0xb6)),
            row![btn(sh, "Allow", true, EMsg::Made("always".into())), btn(sh, "Just this once", false, EMsg::Made("once".into())), btn(sh, "No", false, EMsg::Made("no".into()))].spacing(10)
        ]
        .spacing(8))
        .padding(20)
        .width(Length::Fill)
        .style(|_| container::Style { background: Some(Color::from_rgb8(0x2c, 0x24, 0x18).into()), border: iced::Border { color: Color::from_rgba(1.0, 0.75, 0.3, 0.5), width: 1.0, radius: 8.0.into() }, ..Default::default() }));
    }
    r = r.push(row![btn(sh, "Keep this", true, EMsg::Made("keep".into())), btn(sh, "Change it…", false, EMsg::Made("change".into())), btn(sh, "Try again", false, EMsg::Made("again".into())), btn(sh, "Undo", false, EMsg::Made("undo".into())), Space::new().width(Length::Fill), btn(sh, "Share it with Hearth…", false, EMsg::Made("share".into()))].spacing(10));
    let top = container(row![l.tb("Editing your desktop", 17.0, Color::WHITE), l.t("· a block that isn't here yet", 15.0, Color::from_rgb8(0xc9, 0xbf, 0xb6)), Space::new().width(Length::Fill), btn(sh, "Done", false, EMsg::Close)].spacing(14).align_y(iced::Center))
        .padding(Padding { left: 24.0, right: 24.0, ..Padding::ZERO })
        .center_y(56)
        .width(Length::Fill)
        .style(|_| container::Style { background: Some(Color::from_rgb8(0x21, 0x1c, 0x19).into()), ..Default::default() });
    container(column![top, container(row![left, r].spacing(60)).padding(Padding { left: 100.0, right: 100.0, top: 50.0, bottom: 0.0 })]).width(Length::Fill).height(Length::Fill).style(|_| container::Style { background: Some(Color::from_rgb8(0x1b, 0x17, 0x15).into()), ..Default::default() }).into()
}
