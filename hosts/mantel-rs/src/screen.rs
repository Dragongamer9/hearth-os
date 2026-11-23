//! Build anything in Mantel in Rust (build-anything boards 1-3, 5, 7): the same whole-screen desktop the QML Mantel
//! draws (Screen.qml), from the same file (~/.local/share/nimbus/screen.json, hearth-layout screen.rs's shape) and the
//! same ready-made desktops (screen.rs READY), on the bottom layer under your windows. When it covers the bar and dock
//! ("bars": "hidden"), its Guide, menu and Game Mode blocks do what they did. Blocks your AI made run boxed in
//! (`hearth-layout run-block`), asking before they reach anything. The first start's "How do you want your desktop?"
//! (first.rs) and Settings › Your desktop change it; nothing here writes your settings except your own choice there,
//! merged with settings_file (atomic, a .bak). If the file can't be read, nothing is shown and the bar and dock stay.

use iced::widget::{column, container, image, mouse_area, pin, row, stack, text, Column, Row, Space};
use iced::{border, mouse, Color, Element, Length, Padding};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::look::{self, a};
use crate::shell::{Message, Shell};

pub fn file() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".local/share")).join("nimbus/screen.json")
}

#[derive(Debug, Clone, Default)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub cover: String,
    pub wide: String,
    pub update: bool,
}

#[derive(Debug, Default)]
pub struct State {
    pub on: bool,
    pub chosen: bool,
    pub unreadable: bool,
    pub screen: Value,
    pub from: String,
    pub games: Vec<Game>,
    pub picked: usize,
    pub tab: String,
    pub guide: bool,
    /// what each of your AI's blocks said last ("own:<id>" → run-block's answer)
    pub own: HashMap<String, Value>,
    /// what you started last ("game:<id>" first), from desk.json, as the QML Mantel's DeskState.recent
    pub recent: Vec<String>,
}

impl State {
    pub fn hides_bars(&self) -> bool {
        self.on && self.screen.get("bars").and_then(Value::as_str) == Some("hidden")
    }
    pub fn blocks(&self) -> Vec<Value> {
        self.screen.get("blocks").and_then(Value::as_array).cloned().unwrap_or_default()
    }
    pub fn own_ids(&self) -> Vec<String> {
        self.blocks().iter().filter_map(|b| b.get("block").and_then(Value::as_str).and_then(|x| x.strip_prefix("own:")).map(str::to_string)).collect()
    }
}

/// What's in your file now (on, the screen, chosen); a file that can't be read shows nothing and is never written over.
pub fn read(st: &mut State) {
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config"));
    st.recent = hearth_ui::settings_file::read_json(&cfg.join("nimbus/desk.json")).get("recent").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let p = file();
    match std::fs::read(&p) {
        Err(_) => {
            st.unreadable = false;
            st.on = false;
        }
        Ok(b) => match serde_json::from_slice::<Value>(&b) {
            Ok(v) if v.is_object() => {
                st.unreadable = false;
                st.screen = v.get("screen").cloned().unwrap_or(Value::Null);
                st.on = v.get("on") == Some(&json!(true)) && st.screen.get("blocks").map(Value::is_array).unwrap_or(false);
                st.chosen = v.get("chosen") == Some(&json!(true));
                st.from = v.get("from").and_then(Value::as_str).unwrap_or("").to_string();
            }
            _ => {
                st.unreadable = !b.iter().all(|c| c.is_ascii_whitespace());
                st.on = false;
            }
        },
    }
}

/// Your choice (the first start, Settings › Your desktop): only these keys merged into the file.
pub fn choose(st: &mut State, what: &str) {
    if st.unreadable {
        return;
    }
    let mut m = Map::new();
    if what == "mantel" {
        m.insert("on".into(), json!(false));
    } else if let Some((_, t)) = hearth_layout::screen::READY.iter().find(|(id, _)| *id == what) {
        let v: Value = serde_json::from_str(t).unwrap_or(Value::Null);
        m.insert("on".into(), json!(true));
        m.insert("from".into(), json!(what));
        m.insert("said".into(), v.get("said").cloned().unwrap_or(json!("")));
        m.insert("screen".into(), v);
    }
    m.insert("chosen".into(), json!(true));
    if let Some(d) = file().parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = hearth_ui::settings_file::merge_json(&file(), &m);
    read(st);
}

/// Your games with their pictures (hearth-games cached: what it said last, at once).
pub fn load_games() -> Vec<Game> {
    let o = std::process::Command::new("python3").arg(look::shell_file("hearth-games")).arg("cached").output();
    let Ok(o) = o else { return Vec::new() };
    let t = String::from_utf8_lossy(&o.stdout);
    let v: Value = t.trim().lines().last().and_then(|l| serde_json::from_str(l).ok()).unwrap_or(Value::Null);
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|g| {
                    let s = |k: &str| g.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                    (!s("title").is_empty()).then(|| Game { id: s("id"), title: s("title"), cover: s("cover"), wide: s("wide"), update: g.get("update") == Some(&json!(true)) })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One boxed-in look of one of your AI's blocks (`hearth-layout run-block`: its own small program).
pub fn run_own(id: &str, once: bool) -> Value {
    let mut c = std::process::Command::new(layout_bin());
    c.arg("run-block").arg(id);
    if once {
        c.arg("--once");
    }
    c.output().ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().lines().last().and_then(|l| serde_json::from_str(l).ok())).unwrap_or(json!({ "ok": false, "error": "It stopped by itself (too big or too slow)." }))
}
pub fn layout_bin() -> PathBuf {
    // the Store's copy of Mantel in Rust carries its own, next to it
    if let Some(p) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("hearth-layout"))).filter(|p| p.exists()) {
        return p;
    }
    let own = look::shell_file("../bin/hearth-layout");
    if own.exists() {
        own
    } else {
        PathBuf::from("hearth-layout")
    }
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn n(v: &Value, k: &str) -> f32 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32
}
pub fn colour(c: &str, d: Color) -> Color {
    let h = c.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f32 / 255.0;
    match h.len() {
        6 => Color::from_rgb(p(0), p(2), p(4)),
        8 => Color::from_rgba(p(0), p(2), p(4), p(6)),
        _ => d,
    }
}

/// A block's sound (Hearth's own, block-*.wav), unless Settings › Sounds is off.
pub fn sound(name: &str) {
    if name.is_empty() || name == "none" {
        return;
    }
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(".config"));
    if hearth_ui::settings_file::read_json(&cfg.join("nimbus/sounds.json")).get("set").and_then(Value::as_str) == Some("off") {
        return;
    }
    let f = look::shell_file(&format!("../sounds/hearth/stereo/block-{name}.wav"));
    if f.exists() {
        let _ = std::process::Command::new("pw-play").arg("--media-role=Notification").arg(&f).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    }
}

/// A row's items (screen.rs ITEMS): (Hearth's icon, label, action, target).
pub fn item(id: &str) -> (&'static str, String, &'static str, String) {
    let o = |g: &'static str, l: &str, a: &'static str, t: &str| (g, l.to_string(), a, t.to_string());
    match id {
        "games" => o("pad", "Games", "games", ""),
        "store" => o("store", "Hearth Store", "store", ""),
        "files" => o("folder", "Files", "files", ""),
        "browser" => o("globe", "Browser", "browser", ""),
        "music" => o("headphones", "Music", "music", ""),
        "friends" => o("family", "Friends", "friends", ""),
        "downloads" => o("update", "Downloads", "store", ""),
        "settings" => o("gear", "Settings", "settings", ""),
        "capture" => o("cam", "Capture", "capture", ""),
        "notes" => o("doc", "Notes", "notes", ""),
        "gamemode" => o("pad", "Game Mode", "game-mode", ""),
        "power" => o("power", "Power", "power", ""),
        "quick" => o("menu", "Quick settings", "quickaccess", ""),
        "search" => o("search", "Search", "search", ""),
        "menu" => o("home", "Start", "menu", ""),
        "windows" => o("window", "Desktop", "tab", "windows"),
        "bin" => o("trash", "Bin", "open-folder", "trash:///"),
        "home" => o("home", "My Hearth", "open-folder", "~"),
        other => {
            let a = other.strip_prefix("app:").unwrap_or(other);
            ("", a.to_string(), "open-app", a.to_string())
        }
    }
}

/// What a press does (Hearth's own actions, as the QML Mantel's).
pub fn act(sh: &mut Shell, a: &str, arg: &str) -> iced::Task<Message> {
    let run = crate::shell::run_cmd;
    let q = crate::apps::shell_quote;
    match a {
        "play" => {
            let g = if arg.is_empty() { sh.screen.games.get(sh.screen.picked).map(|g| g.id.clone()).unwrap_or_default() } else { arg.to_string() };
            if !g.is_empty() {
                run(&format!("python3 {} play {}", q(&look::shell_file("hearth-games").to_string_lossy()), q(&g)));
            }
        }
        "games" | "menu" | "search" | "power" => return iced::Task::done(Message::Flame),
        "settings" | "open-settings" => run(&format!("hearth-settings-app {}", if arg.is_empty() { String::new() } else { format!("--page {}", q(arg)) })),
        "quick" | "quickaccess" => run("hearth-settings-app --page quick"),
        "files" => run("hearth-files-app"),
        "open-folder" => run(&format!("hearth-files-app {}", q(&arg.replacen('~', &hearth_ui::paths::home().to_string_lossy(), 1)))),
        "store" => run("hearth-store-app"),
        "capture" | "screenshot" => run("hearth-capture-app"),
        "notes" => run("hearth-notes-app"),
        "friends" => run("hearth-family-app"),
        "browser" => run("xdg-open https://hearthos.fun"),
        "open-url" if arg.starts_with("https://") => run(&format!("xdg-open {}", q(arg))),
        "music" => run(&format!("xdg-open {}", q(&hearth_ui::paths::home().join("Music").to_string_lossy()))),
        "gamemode" | "game-mode" => run("nimbus-mode game"),
        "lock" => run("loginctl lock-session"),
        "overview" => return iced::Task::done(Message::Overview),
        "guide" => sh.screen.guide = !sh.screen.guide,
        "tab" | "show-desktop" => sh.screen.tab = if arg.is_empty() { "windows".into() } else { arg.to_string() },
        "open-app" | "app" => {
            if let Some(ap) = crate::apps::find(&sh.apps, arg).cloned() {
                crate::apps::launch(&ap);
            }
        }
        "mantel" => {
            choose(&mut sh.screen, "mantel");
            return iced::Task::done(Message::DeskCheck);
        }
        "edit" => return iced::Task::done(Message::ScreenEdit),
        _ => {}
    }
    iced::Task::none()
}

fn recent(sh: &Shell) -> Vec<Game> {
    let ids: Vec<String> = sh.screen.recent.iter().filter_map(|r| r.strip_prefix("game:").map(str::to_string)).collect();
    let mut out: Vec<Game> = ids.iter().filter_map(|id| sh.screen.games.iter().find(|g| &g.id == id).cloned()).collect();
    out.extend(sh.screen.games.iter().filter(|g| !ids.contains(&g.id)).cloned());
    out
}

fn pic<'a>(path: &str, w: f32, h: f32) -> Element<'a, Message> {
    let p = path.trim_start_matches("file://");
    if !p.is_empty() && std::path::Path::new(p).exists() {
        image(look::image_handle(std::path::Path::new(p))).width(w).height(h).content_fit(iced::ContentFit::Cover).into()
    } else {
        Space::new().width(w).height(h).into()
    }
}

fn press<'a>(e: impl Into<Element<'a, Message>>, a: &str, arg: &str, snd: &str) -> Element<'a, Message> {
    mouse_area(e).on_press(Message::ScreenAct(a.to_string(), arg.to_string(), snd.to_string())).interaction(mouse::Interaction::Pointer).into()
}

/// The whole screen (the Screen surface on the bottom layer).
pub fn view(sh: &Shell) -> Element<'_, Message> {
    let st = &sh.screen;
    // Try it full screen (the block editor): the draft, until Escape
    let lay = if sh.editor.trying { &sh.editor.draft } else { &st.screen };
    let (w, h) = (sh.screen_size.width.max(1.0), sh.screen_size.height.max(1.0));
    let grid = lay.get("grid").cloned().unwrap_or(json!({}));
    let (cols, rows) = (n(&grid, "cols").max(1.0), n(&grid, "rows").max(1.0));
    let margin = (w.min(h) * 0.035).round();
    let (cw, chh) = ((w - margin * 2.0) / cols, (h - margin * 2.0) / rows);
    let look_v = lay.get("look").cloned().unwrap_or(json!({}));
    let accent = colour(s(&look_v, "accent"), Color::WHITE);
    let ink = colour(s(&look_v, "text"), Color::WHITE);
    let l = &sh.look;
    let desk_tab = st.tab == "windows";
    // the background
    let bg = lay.get("background").cloned().unwrap_or(json!({}));
    let kind = s(&bg, "kind").to_string();
    let from = colour(s(&bg, "from"), Color::from_rgb8(0x10, 0x18, 0x20));
    let to = if kind == "colour" { from } else { colour(s(&bg, "to"), from) };
    let mut layers: Vec<Element<Message>> = Vec::new();
    if !desk_tab && kind != "wallpaper" && kind != "none" {
        layers.push(container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, from).add_stop(1.0, to)).into()), ..Default::default() }).into());
        if kind == "art" {
            if let Some(g) = st.games.get(st.picked) {
                layers.push(container(pic(if g.wide.is_empty() { &g.cover } else { &g.wide }, w, h)).style(|_| container::Style::default()).into());
                layers.push(container(Space::new()).width(Length::Fill).height(Length::Fill).style(|_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2).add_stop(0.0, a(Color::BLACK, 0.75)).add_stop(0.6, a(Color::BLACK, 0.2)).add_stop(1.0, a(Color::BLACK, 0.05))).into()), ..Default::default() }).into());
            }
        }
        if kind == "picture" && !s(&bg, "value").is_empty() {
            layers.push(pic(&s(&bg, "value").replacen('~', &hearth_ui::paths::home().to_string_lossy(), 1), w, h));
        }
    }
    let rec = recent(sh);
    let blocks: Vec<Value> = lay.get("blocks").and_then(Value::as_array).cloned().unwrap_or_default();
    for b in blocks.iter().cloned() {
        let blk = s(&b, "block").to_string();
        if desk_tab && !["tabs", "clock", "status", "bar"].contains(&blk.as_str()) {
            continue;
        }
        if blk == "guide" && !st.guide {
            continue;
        }
        let stv = b.get("style").cloned().unwrap_or(json!({}));
        let (bx, by, bw, bh) = (margin + n(&b, "x") * cw, margin + n(&b, "y") * chh, n(&b, "w").max(1.0) * cw, n(&b, "h").max(1.0) * chh);
        let content = block(sh, &b, &stv, &rec, bw, bh, accent, ink);
        layers.push(pin(container(content).width(bw).height(bh)).x(bx).y(by).into());
    }
    // without a guide block, the Guide on the right
    if st.guide && !blocks.iter().any(|b| s(b, "block") == "guide") {
        let gw = (w * 0.24).min(420.0);
        layers.push(pin(container(guide(sh, &json!({}), accent)).width(gw).height(h)).x(w - gw).y(0.0).into());
    }
    let _ = l;
    stack(layers).width(Length::Fill).height(Length::Fill).into()
}

#[allow(clippy::too_many_arguments)]
fn block<'a>(sh: &'a Shell, b: &Value, stv: &Value, rec: &[Game], bw: f32, bh: f32, accent: Color, ink: Color) -> Element<'a, Message> {
    let l = &sh.look;
    let st = &sh.screen;
    let blk = s(b, "block");
    let snd = s(stv, "sound").to_string();
    let size = |d: f32| if n(stv, "size") > 0.0 { n(stv, "size") } else { d };
    let fg = colour(s(stv, "text"), ink);
    let fill = colour(s(stv, "colour"), a(Color::WHITE, 0.06));
    let corners = if stv.get("corners").is_some() { n(stv, "corners") } else { 8.0 };
    let boxed = move |e: Element<'a, Message>| -> Element<'a, Message> {
        container(e).width(Length::Fill).height(Length::Fill).clip(true).style(move |_| container::Style { background: Some(fill.into()), border: border::rounded(corners), ..Default::default() }).into()
    };
    let items: Vec<String> = b.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let title = s(b, "title").to_string();
    let shows = s(b, "shows").to_string();
    match blk {
        "hero" => {
            let g = rec.first().cloned();
            let name = g.as_ref().map(|g| g.title.clone()).unwrap_or_else(|| "Your games show here".into());
            let art = g.as_ref().map(|g| pic(if g.wide.is_empty() { &g.cover } else { &g.wide }, bw, bh)).unwrap_or_else(|| Space::new().into());
            let words = container(l.tb(name, size(26.0), Color::WHITE)).padding(18).align_bottom(Length::Fill).width(Length::Fill);
            let tile = boxed(stack![art, words].into());
            let body: Element<Message> = if title.is_empty() { tile } else { column![l.t(title, 17.0, a(ink, 0.85)), tile].spacing(10).into() };
            press(body, "play", &g.map(|g| g.id).unwrap_or_default(), &snd)
        }
        "tilerow" | "list" => {
            let games = shows == "recent" || shows == "games";
            let skip = n(b, "skip") as usize;
            let count = if n(b, "count") > 0.0 { n(b, "count") as usize } else { 99 };
            let mut r = Row::new().spacing(if shows == "words" { 34 } else { 14 });
            let mut col = Column::new().spacing(6);
            if games {
                let src: Vec<Game> = if shows == "recent" { rec.to_vec() } else { st.games.clone() };
                let list: Vec<Game> = src.into_iter().skip(skip).take(count).collect();
                let tw = if list.is_empty() { bw } else { (bw - 14.0 * (list.len() as f32 - 1.0)) / list.len() as f32 };
                for g in list {
                    if blk == "list" {
                        col = col.push(press(l.t(g.title.to_uppercase(), size(15.0), fg), "play", &g.id, &snd));
                    } else {
                        let tile = container(stack![pic(&g.cover, tw, bh), container(l.tb(g.title.clone(), 15.0, Color::WHITE)).padding(14).align_bottom(Length::Fill)]).width(tw).height(Length::Fill).clip(true).style(move |_| container::Style { background: Some(Color::from_rgb8(0x24, 0x32, 0x4a).into()), border: border::rounded(corners), ..Default::default() });
                        r = r.push(press(tile, "play", &g.id, &snd));
                    }
                }
            } else {
                for i in &items {
                    let (g, lab, act, arg) = item(i);
                    if blk == "list" || shows == "sticky" {
                        col = col.push(press(l.t(lab, size(18.0), fg), act, &arg, &snd));
                    } else if shows == "words" {
                        r = r.push(press(row![look::icon(g, size(18.0) * 1.1, fg), l.t(lab, size(18.0), fg)].spacing(8).align_y(iced::Center), act, &arg, &snd));
                    } else {
                        let tw = if i == "friends" { (bh * 2.1).max(200.0) } else { bh.max(110.0) };
                        let inner = column![look::icon(g, 22.0, fg), l.t(lab, size(16.0), fg)].spacing(4);
                        let tile = container(inner).padding(Padding { left: 22.0, ..Padding::ZERO }).center_y(Length::Fill).width(tw).style(move |_| container::Style { background: Some(fill.into()), border: border::rounded(corners), ..Default::default() });
                        r = r.push(press(tile, act, &arg, &snd));
                    }
                }
            }
            let body: Element<Message> = if blk == "list" {
                col.into()
            } else if shows == "sticky" {
                {
                let paper = colour(s(stv, "colour"), Color::from_rgb8(0xff, 0xf3, 0xa0));
                container(col).padding(20).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(paper.into()), ..Default::default() }).into()
            }
            } else {
                r.height(Length::Fill).into()
            };
            if title.is_empty() { body } else { column![l.t(title, 17.0, a(ink, 0.85)), body].spacing(12).into() }
        }
        "iconrow" => {
            let mut r = Row::new().spacing(16).align_y(iced::Center);
            let (big, small) = (bh, bh * 0.62);
            for (i, g) in st.games.iter().take(if n(b, "count") > 0.0 { n(b, "count") as usize } else { 8 }).enumerate() {
                let on = i == st.picked;
                let sz = if on { big } else { small };
                let t = container(pic(&g.cover, sz, sz)).width(sz).height(sz).clip(true).style(move |_| container::Style { border: iced::Border { color: if on { Color::WHITE } else { Color::TRANSPARENT }, width: if on { 4.0 } else { 0.0 }, radius: (sz * 0.14).into() }, background: Some(Color::from_rgb8(0x3a, 0x2a, 0x5a).into()), ..Default::default() });
                r = r.push(mouse_area(t).on_press(if on { Message::ScreenAct("play".into(), g.id.clone(), snd.clone()) } else { Message::ScreenPick(i) }).interaction(mouse::Interaction::Pointer));
            }
            r = r.push(press(container(look::icon("grid", small * 0.34, ink)).center_x(small).center_y(small).style(move |_| container::Style { background: Some(a(Color::WHITE, 0.12).into()), border: border::rounded(small * 0.14), ..Default::default() }), "games", "", &snd));
            r.height(Length::Fill).into()
        }
        "tabs" => {
            let list = if items.is_empty() { vec!["games".to_string(), "music".into(), "windows".into()] } else { items };
            let mut r = Row::new().spacing(36);
            for (i, it) in list.iter().enumerate() {
                let key = if i == 0 { String::new() } else { it.clone() };
                let on = st.tab == key;
                let lab = match it.as_str() { "music" => "Media".to_string(), "windows" => "Desktop".into(), x => item(x).1 };
                r = r.push(mouse_area(if on { l.tb(lab, 22.0, ink) } else { l.t(lab, 22.0, a(ink, 0.75)) }).on_press(Message::ScreenTab(key)).interaction(mouse::Interaction::Pointer));
            }
            r.into()
        }
        "clock" => {
            let now = chrono::Local::now();
            let t = match shows.as_str() {
                "date" => now.format("%A, %-d %B").to_string(),
                "ring" => now.format("%H:%M").to_string(),
                _ => if l.clock24 { now.format("%H:%M").to_string() } else { now.format("%-I:%M %p").to_string() },
            };
            let tx = text(t).size(size(22.0)).color(fg).font(l.font);
            if shows == "ring" {
                let d = bw.min(bh);
                container(container(tx).center_x(d).center_y(d).style(move |_| container::Style { border: iced::Border { color: a(accent, 0.8), width: 3.0, radius: (d / 2.0).into() }, ..Default::default() })).center_x(Length::Fill).center_y(Length::Fill).into()
            } else {
                let right = n(b, "x") > 12.0;
                container(tx).align_x(if right { iced::alignment::Horizontal::Right } else { iced::alignment::Horizontal::Left }).center_y(Length::Fill).width(Length::Fill).into()
            }
        }
        "profile" => {
            let user = std::env::var("USER").unwrap_or_else(|_| "you".into());
            let name = format!("{}{}", user.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(), user.chars().skip(1).collect::<String>());
            let dot = container(l.tb(name.chars().next().map(|c| c.to_string()).unwrap_or_default(), 19.0, Color::from_rgb8(0x0b, 0x1a, 0x10))).center_x(46).center_y(46).style(move |_| container::Style { background: Some(accent.into()), border: border::rounded(23), ..Default::default() });
            press(row![dot, column![l.tb(name, 17.0, ink), l.t("Online", 13.0, a(ink, 0.8))]].spacing(14).align_y(iced::Center), "guide", "", &snd)
        }
        "status" | "battery" => {
            let now = chrono::Local::now();
            let t = if l.clock24 { now.format("%H:%M").to_string() } else { now.format("%-I:%M %p").to_string() };
            let batt = sh.status.battery.map(|(p, _)| format!("{p}%")).unwrap_or_default();
            if blk == "battery" {
                return container(l.t(if batt.is_empty() { "Plugged in".into() } else { batt }, 17.0, fg)).align_right(Length::Fill).center_y(Length::Fill).into();
            }
            container(row![press(look::icon("search", 22.0, ink), "search", "", ""), press(look::icon("gear", 22.0, ink), "settings", "", ""), l.t(batt, 17.0, a(ink, 0.85)), l.t(t, 19.0, ink)].spacing(22).align_y(iced::Center)).align_right(Length::Fill).center_y(Length::Fill).into()
        }
        "menu" | "search" | "gamemode" => {
            let (g, lab, act) = match blk { "menu" => ("grid", "", "menu"), "search" => ("search", "Search", "search"), _ => ("pad", "Game Mode", "game-mode") };
            let pa = b.get("press").and_then(|p| p.get("action")).and_then(Value::as_str).unwrap_or(act).to_string();
            let mut r = Row::new().spacing(8).align_y(iced::Center).push(look::icon(g, (bh * 0.45).min(28.0), fg));
            if !lab.is_empty() {
                r = r.push(l.t(lab, (bh * 0.38).min(22.0), fg));
            }
            press(boxed(container(r).center_x(Length::Fill).center_y(Length::Fill).into()), &pa, "", &snd)
        }
        "play" => {
            let g = st.games.get(st.picked).cloned();
            let name = g.as_ref().map(|g| g.title.clone()).unwrap_or_else(|| "Pick a game".into());
            let playb = container(text("Play").size(20).color(Color::from_rgb8(0x11, 0x11, 0x11)).font(l.bold())).center_x(150).center_y(56).style(|_| container::Style { background: Some(Color::WHITE.into()), border: border::rounded(28), ..Default::default() });
            let more = container(l.t("⋯", 20.0, ink)).center_x(56).center_y(56).style(|_| container::Style { background: Some(a(Color::WHITE, 0.16).into()), border: border::rounded(28), ..Default::default() });
            column![text(name).size(72).color(ink).font(iced::Font { weight: iced::font::Weight::Light, ..l.font }), row![press(playb, "play", &g.map(|g| g.id).unwrap_or_default(), &snd), press(more, "games", "", "")].spacing(14)].spacing(26).into()
        }
        "text" => text(s(b, "value").to_string()).size(size(18.0)).color(fg).font(l.font).into(),
        "notes" => {
            let win = !s(stv, "colour").is_empty() && s(stv, "colour") != "#00000000";
            let mut c = Column::new().spacing(8);
            if win {
                c = c.push(container(l.tb(if title.is_empty() { "Notes".into() } else { title.clone() }, 14.0, Color::WHITE)).padding(Padding { left: 8.0, top: 4.0, bottom: 4.0, right: 8.0 }).width(Length::Fill).style(move |_| container::Style { background: Some(accent.into()), ..Default::default() }));
            } else if !title.is_empty() {
                c = c.push(l.t(title, size(22.0) * 1.05, fg));
            }
            // "☐ " / "☑ " lines are a checklist: Hearth draws the boxes (the fonts haven't got those signs)
            let fs = size(18.0);
            for line in s(b, "value").lines() {
                let (mark, rest) = if let Some(r) = line.strip_prefix('☐') { (Some(false), r.trim_start()) } else if let Some(r) = line.strip_prefix('☑') { (Some(true), r.trim_start()) } else { (None, line) };
                match mark {
                    Some(done) => {
                        let bx = fs * 0.8;
                        let tick: Element<Message> = if done { container(Space::new()).width(bx * 0.55).height(bx * 0.55).style(move |_| container::Style { background: Some(fg.into()), border: border::rounded(2.0), ..Default::default() }).into() } else { Space::new().into() };
                        let boxv = container(tick).width(bx).height(bx).center_x(bx).center_y(bx).style(move |_| container::Style { border: iced::Border { color: fg, width: 1.5, radius: 3.0.into() }, ..Default::default() });
                        c = c.push(row![boxv, l.t(rest.to_string(), fs, fg)].spacing(fs * 0.5).align_y(iced::Center));
                    }
                    None => c = c.push(l.t(line.to_string(), fs, fg)),
                }
            }
            if win { boxed(container(c).padding(6).into()) } else { c.into() }
        }
        "picture" => {
            if shows == "fire" {
                let gr = bw;
                let glow = container(Space::new()).width(bw * 0.7).height(bh * 0.8).style(move |_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, Color::from_rgb8(0xff, 0xcf, 0x7a)).add_stop(0.45, Color::from_rgb8(0xff, 0x7a, 0x2f)).add_stop(1.0, Color::TRANSPARENT)).into()), border: border::rounded(gr), ..Default::default() });
                container(glow).center_x(Length::Fill).center_y(Length::Fill).style(move |_| container::Style { background: Some(Color::from_rgb8(0x2a, 0x14, 0x08).into()), border: border::rounded(corners), ..Default::default() }).into()
            } else {
                pic(&s(b, "value").replacen('~', &hearth_ui::paths::home().to_string_lossy(), 1), bw, bh)
            }
        }
        "activity" => {
            let played = rec.first().filter(|_| sh.screen.recent.iter().any(|r| r.starts_with("game:"))).map(|g| g.title.clone()).unwrap_or_else(|| "—".into());
            let cards = [("Played last", played, ""), ("Your games", st.games.len().to_string(), "in Hearth's library"), ("Next up", if st.games.iter().any(|g| g.update) { "An update is waiting".into() } else { "All up to date".to_string() }, "")];
            let mut r = Row::new().spacing(18).height(Length::Fill);
            for (k, v, sub) in cards {
                r = r.push(container(column![l.t(k, 14.0, a(ink, 0.8)), l.t(v, 30.0, ink), l.t(sub, 14.0, a(ink, 0.8))].spacing(6)).padding(20).width(Length::FillPortion(1)).height(Length::Fill).style(move |_| container::Style { background: Some(fill.into()), border: border::rounded(corners), ..Default::default() }));
            }
            column![l.t(if title.is_empty() { "Your activity".to_string() } else { title }, 17.0, a(ink, 0.85)), r].spacing(12).into()
        }
        "windows" => {
            let mut c = Column::new().spacing(8).push(l.t(if title.is_empty() { "Your windows".to_string() } else { title }, 17.0, a(ink, 0.85)));
            for w in &sh.wm.wins {
                c = c.push(mouse_area(container(l.t(w.title.clone(), 15.0, ink)).padding(12).width(Length::Fill).style(|_| container::Style { background: Some(a(Color::WHITE, 0.06).into()), border: border::rounded(6), ..Default::default() })).on_press(Message::Win(w.id)));
            }
            c.into()
        }
        "media" => boxed(container(column![l.t("Nothing playing", 13.0, a(ink, 0.8)), l.t("Music, videos and podcasts show here", 17.0, ink)].spacing(4)).padding(18).into()),
        "friends" => press(boxed(container(column![l.t("Friends", 14.0, accent), l.t("Family and friends", 16.0, ink), l.t("Who's around", 13.0, a(ink, 0.75))].spacing(6)).padding(20).into()), "friends", "", &snd),
        "downloads" => press(boxed(container(column![look::icon("update", 22.0, ink), l.t(if st.games.iter().any(|g| g.update) { format!("{} updates waiting", st.games.iter().filter(|g| g.update).count()) } else { "Downloads".into() }, 16.0, ink)].spacing(4)).padding(22).center_y(Length::Fill).into()), "store", "", &snd),
        "dock" => {
            let mut r = Row::new().spacing(12);
            for p in sh.pins.iter().take(10) {
                if let Some(ap) = crate::apps::find(&sh.apps, p) {
                    let ic: Element<Message> = ap.icon_path.clone().map(|path| image(look::image_handle(&path)).width(bh * 0.6).height(bh * 0.6).into()).unwrap_or_else(|| Space::new().into());
                    r = r.push(press(container(ic).center_x(bh).center_y(bh).style(|_| container::Style { background: Some(a(Color::WHITE, 0.06).into()), border: border::rounded(12), ..Default::default() }), "open-app", &ap.id, &snd));
                }
            }
            r.into()
        }
        "guide" => guide(sh, stv, accent),
        "weather" => l.t("Pick your place in Settings › Display for the weather", size(18.0), fg).into(),
        "widget" => {
            let (load, mem) = readings();
            let batt = sh.status.battery.map(|(p, _)| format!("{p}%")).unwrap_or_else(|| "AC".into());
            let mono = look::mono();
            column![text(format!("CPU {load}")).size(size(15.0)).color(fg).font(mono), text(format!("MEM {mem}")).size(size(15.0)).color(fg).font(mono), text(format!("BAT {batt}")).size(size(15.0)).color(fg).font(mono)].spacing(6).into()
        }
        "icons" => {
            let mut c = Column::new().spacing(22);
            for pair in items.chunks(2) {
                let mut r = Row::new().spacing(30);
                for i in pair {
                    let (g, lab, act, arg) = item(i);
                    r = r.push(press(column![look::icon(g, 30.0, fg), l.t(lab, size(14.0), fg)].spacing(4).align_x(iced::Center).width(90), act, &arg, &snd));
                }
                c = c.push(r);
            }
            c.into()
        }
        "bar" => {
            let mut r = Row::new().spacing(6).align_y(iced::Center);
            for i in items.iter().filter(|i| *i != "windows") {
                let (g, lab, act, arg) = item(i);
                r = r.push(press(container(row![look::icon(g, 16.0, fg), l.tb(lab, size(14.0), fg)].spacing(6).align_y(iced::Center)).padding(Padding { left: 12.0, right: 12.0, top: 2.0, bottom: 2.0 }).style(|_| container::Style { border: iced::Border { color: Color::from_rgb8(0x80, 0x80, 0x80), width: 1.0, radius: 0.0.into() }, ..Default::default() }), act, &arg, &snd));
            }
            if items.iter().any(|i| i == "windows") {
                for w in &sh.wm.wins {
                    r = r.push(mouse_area(container(l.t(w.title.chars().take(22).collect::<String>(), 13.0, fg)).width(160).padding(Padding { left: 8.0, top: 2.0, bottom: 2.0, right: 8.0 }).style(|_| container::Style { border: iced::Border { color: Color::from_rgb8(0x80, 0x80, 0x80), width: 1.0, radius: 0.0.into() }, ..Default::default() })).on_press(Message::Win(w.id)));
                }
            }
            let now = chrono::Local::now();
            let t = if l.clock24 { now.format("%H:%M").to_string() } else { now.format("%-I:%M %p").to_string() };
            boxed(container(row![r, Space::new().width(Length::Fill), l.t(t, 14.0, fg)].align_y(iced::Center)).padding(Padding { left: 6.0, right: 12.0, ..Padding::ZERO }).center_y(Length::Fill).into())
        }
        own if own.starts_with("own:") => own_block(sh, &own[4..], stv, ink),
        _ => Space::new().into(),
    }
}

/// The system's readings for a readout: the load (how busy the processor is) and memory in use.
fn readings() -> (String, String) {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f32;
    let load = std::fs::read_to_string("/proc/loadavg").ok().and_then(|t| t.split_whitespace().next().and_then(|x| x.parse::<f32>().ok())).map(|l| format!("{}%", ((l / cores) * 100.0).min(100.0).round())).unwrap_or_else(|| "—".into());
    let mi = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let kb = |k: &str| mi.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|x| x.parse::<f32>().ok());
    let mem = match (kb("MemTotal:"), kb("MemAvailable:")) {
        (Some(t), Some(a)) if t > 0.0 => format!("{}%", (((t - a) / t) * 100.0).round()),
        _ => "—".into(),
    };
    (load, mem)
}

fn guide<'a>(sh: &'a Shell, stv: &Value, accent: Color) -> Element<'a, Message> {
    let l = &sh.look;
    let bgc = colour(s(stv, "colour"), Color::from_rgba(0.07, 0.11, 0.09, 0.97));
    let items = [("Home", "guide", ""), ("Quick settings", "quick", ""), ("Friends and chat", "friends", ""), ("Capture and share", "capture", ""), ("Your windows", "tab", "windows"), ("Game Mode", "game-mode", ""), ("Settings", "settings", ""), ("Power", "power", ""), ("Change this desktop", "edit", ""), ("Back to Hearth OS Mantel", "mantel", "")];
    let mut c = Column::new().spacing(6).push(container(l.t("THE GUIDE · HOME BUTTON", 13.0, look::lighter(accent, 1.2))).padding(Padding { bottom: 18.0, ..Padding::ZERO }));
    for (i, (t, act, arg)) in items.iter().enumerate() {
        let lab = if *act == "tab" { format!("{t} ({} open)", sh.wm.wins.len()) } else { t.to_string() };
        let on = i == 0;
        c = c.push(mouse_area(container(l.t(lab, 17.0, if on { Color::from_rgb8(0x0b, 0x1a, 0x10) } else { Color::from_rgb8(0xe8, 0xef, 0xe9) })).padding(Padding { left: 16.0, ..Padding::ZERO }).center_y(48).width(Length::Fill).style(move |_| container::Style { background: Some(if on { accent } else { Color::TRANSPARENT }.into()), border: border::rounded(4), ..Default::default() })).on_press(Message::ScreenGuideAct(act.to_string(), arg.to_string())).interaction(mouse::Interaction::Pointer));
    }
    container(c).padding(30).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(bgc.into()), ..Default::default() }).into()
}

/// One of your AI's blocks at a size (the editor's board 5 shows it big).
pub fn own_view<'a>(sh: &'a Shell, id: &str, w: f32, h: f32) -> Element<'a, Message> {
    container(own_block(sh, id, &json!({ "corners": 12 }), Color::WHITE)).width(w).height(h).into()
}

/// One of your AI's blocks: what its last look said (chip, title, progress, lines), or its ask (Allow, Just this once, No).
fn own_block<'a>(sh: &'a Shell, id: &str, stv: &Value, ink: Color) -> Element<'a, Message> {
    let l = &sh.look;
    let out = sh.screen.own.get(id).cloned().unwrap_or(json!({}));
    let fill = colour(s(stv, "colour"), Color::from_rgb8(0x12, 0x12, 0x12));
    let corners = if stv.get("corners").is_some() { n(stv, "corners") } else { 10.0 };
    let mut c = Column::new().spacing(6);
    if let Some(asks) = out.get("asks").filter(|a| !a.is_null() && out.get("no") != Some(&json!(true))) {
        c = c.push(l.ts(s(asks, "title").to_string(), 15.0, Color::WHITE)).push(l.t(s(asks, "sub").to_string(), 12.0, a(Color::WHITE, 0.7)));
        let btn = |t: &'static str, ans: &'static str, prim: bool| -> Element<'a, Message> {
            mouse_area(container(l.tb(t, 13.0, if prim { Color::from_rgb8(0x1b, 0x10, 0x06) } else { Color::WHITE })).padding(Padding { left: 12.0, right: 12.0, top: 6.0, bottom: 6.0 }).style(move |_| container::Style { background: Some(if prim { Color::from_rgb8(0xff, 0x7a, 0x2f) } else { a(Color::WHITE, 0.08) }.into()), border: border::rounded(5), ..Default::default() })).on_press(Message::OwnAllow(id.to_string(), ans.to_string())).interaction(mouse::Interaction::Pointer).into()
        };
        c = c.push(row![btn("Allow", "always", true), btn("Just this once", "once", false), btn("No", "no", false)].spacing(8));
    } else if out.get("ok") == Some(&json!(true)) {
        if !s(&out, "chip").is_empty() {
            c = c.push(container(l.t(format!("● {}", s(&out, "chip")), 13.0, Color::WHITE)).padding(Padding { left: 8.0, right: 8.0, top: 3.0, bottom: 3.0 }).style(|_| container::Style { background: Some(a(Color::BLACK, 0.7).into()), border: border::rounded(6), ..Default::default() }));
        }
        c = c.push(Space::new().height(Length::Fill));
        c = c.push(l.ts(if s(&out, "title").is_empty() { s(&out, "name").to_string() } else { s(&out, "title").to_string() }, 20.0, ink));
        if let Some(p) = out.get("progress").and_then(Value::as_f64) {
            let f = (p as f32 / 100.0).clamp(0.0, 1.0);
            c = c.push(row![container(Space::new()).width(Length::FillPortion((f * 1000.0) as u16 + 1)).height(6).style(|_| container::Style { background: Some(Color::from_rgb8(0xff, 0x7a, 0x2f).into()), ..Default::default() }), container(Space::new()).width(Length::FillPortion(((1.0 - f) * 1000.0) as u16 + 1)).height(6).style(|_| container::Style { background: Some(a(Color::WHITE, 0.18).into()), ..Default::default() })]);
        }
        for line in out.get("lines").and_then(Value::as_array).cloned().unwrap_or_default() {
            c = c.push(l.t(line.as_str().unwrap_or("").to_string(), 13.0, a(Color::WHITE, 0.8)));
        }
    } else {
        let msg = if out.get("off") == Some(&json!(true)) { "Off (Settings › Your desktop › Your blocks)".to_string() } else if out.get("no") == Some(&json!(true)) { "You said No. Settings › Your desktop › Your blocks asks again.".into() } else { s(&out, "error").to_string() };
        c = c.push(container(l.t(msg, 13.0, a(Color::WHITE, 0.7))).center_x(Length::Fill).center_y(Length::Fill));
    }
    container(c).padding(16).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(fill.into()), border: iced::Border { color: Color::from_rgba(1.0, 0.48, 0.18, 0.55), width: 2.0, radius: corners.into() }, ..Default::default() }).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_is_merged_and_back_to_mantel_keeps_the_rest() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_DATA_HOME", d.path());
        std::fs::create_dir_all(d.path().join("nimbus")).unwrap();
        std::fs::write(file(), r#"{"mine": 1}"#).unwrap();
        let mut st = State::default();
        choose(&mut st, "console");
        assert!(st.on && st.hides_bars() && st.chosen && st.from == "console");
        choose(&mut st, "mantel");
        assert!(!st.on && st.chosen);
        let v: Value = serde_json::from_slice(&std::fs::read(file()).unwrap()).unwrap();
        assert_eq!(v["mine"], 1, "the rest of the file stays");
        assert!(v["screen"]["blocks"].is_array(), "your desktop stays, off");
        std::fs::write(file(), "{ broken").unwrap();
        read(&mut st);
        assert!(st.unreadable && !st.on);
        choose(&mut st, "console");
        assert_eq!(std::fs::read_to_string(file()).unwrap(), "{ broken", "never written over a file that can't be read");
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn every_row_item_has_a_name() {
        for (id, _) in hearth_layout::screen::ITEMS {
            assert!(!item(id).1.is_empty(), "{id}");
        }
    }
}

/// Board 7: the first start after a fresh install, full screen, once (desktop.json made for a new install, nothing
/// chosen yet): Hearth OS Mantel, or Build your own desktop (the ready-made ones, Start from nothing).
/// What the gallery calls a ready-made desktop ("" for one of your own).
pub fn ready_name(id: &str) -> &'static str {
    match id {
        "console" => "Console dashboard",
        "iconrow" => "Icon row home",
        "cockpit" => "Cockpit",
        "cabin" => "Cabin",
        "notebook" => "Paper notebook",
        "retro" => "1995 computer",
        _ => "",
    }
}

pub fn first_due(st: &State) -> bool {
    !st.chosen && !st.unreadable && hearth_ui::settings_file::read_json(&hearth_ui::theme::desktop_json()).get("defaults").and_then(Value::as_i64) == Some(2)
}

pub fn first_view(sh: &Shell) -> Element<'_, Message> {
    let l = &sh.look;
    let orange = Color::from_rgb8(0xff, 0x7a, 0x2f);
    let muted = Color::from_rgb8(0xc9, 0xbf, 0xb6);
    let card_w = ((sh.screen_size.width - 260.0) / 2.0).clamp(420.0, 780.0);
    let head = column![
        l.tb("WELCOME TO HEARTH OS", 16.0, orange),
        l.tb("How do you want your desktop?", 54.0, Color::WHITE),
        l.t("Either way it's all Hearth underneath, and you can change your mind any time in Settings › Your desktop.", 19.0, muted)
    ]
    .spacing(12)
    .align_x(iced::Center);
    let btn = |t: &'static str, prim: bool, m: Message| -> Element<'_, Message> {
        mouse_area(container(l.tb(t, 17.0, if prim { Color::from_rgb8(0x1b, 0x10, 0x06) } else { Color::WHITE })).padding(Padding { left: 24.0, right: 24.0, top: 12.0, bottom: 12.0 }).style(move |_| container::Style { background: Some(if prim { orange } else { a(Color::WHITE, 0.06) }.into()), border: iced::Border { color: a(Color::WHITE, if prim { 0.0 } else { 0.18 }), width: 1.0, radius: 6.0.into() }, ..Default::default() })).on_press(m).interaction(mouse::Interaction::Pointer).into()
    };
    let dock = row![
        container(Space::new()).width(34).height(34).style(|_| container::Style { background: Some(Color::from_rgb8(0xff, 0x8a, 0x3d).into()), border: border::rounded(8), ..Default::default() }),
        container(Space::new()).width(34).height(34).style(|_| container::Style { background: Some(Color::from_rgb8(0x5b, 0x8c, 0xff).into()), border: border::rounded(8), ..Default::default() }),
        container(Space::new()).width(34).height(34).style(|_| container::Style { background: Some(Color::from_rgb8(0x5f, 0xd1, 0x8a).into()), border: border::rounded(8), ..Default::default() }),
        container(Space::new()).width(34).height(34).style(|_| container::Style { background: Some(Color::from_rgb8(0xf2, 0xc1, 0x4e).into()), border: border::rounded(8), ..Default::default() }),
        container(Space::new()).width(34).height(34).style(|_| container::Style { background: Some(Color::from_rgb8(0xb0, 0x7c, 0xff).into()), border: border::rounded(8), ..Default::default() })
    ]
    .spacing(10);
    let mantel_pic = container(column![
        container(row![l.t("Files  ·  Games", 13.0, Color::from_rgb8(0xe8, 0xdd, 0xd4)), Space::new().width(Length::Fill), container(l.tb("Game Mode", 12.0, Color::from_rgb8(0x1b, 0x10, 0x06))).padding(Padding { left: 10.0, right: 10.0, top: 2.0, bottom: 2.0 }).style(move |_| container::Style { background: Some(orange.into()), border: border::rounded(11), ..Default::default() })].align_y(iced::Center)).padding(Padding { left: 14.0, right: 14.0, top: 6.0, bottom: 6.0 }).style(|_| container::Style { background: Some(a(Color::BLACK, 0.45).into()), border: border::rounded(10), ..Default::default() }),
        Space::new().height(Length::Fill),
        container(dock).center_x(Length::Fill)
    ])
    .padding(18)
    .width(Length::Fill)
    .height(card_w * 0.35)
    .style(|_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, Color::from_rgb8(0x2a, 0x1a, 0x12)).add_stop(1.0, Color::from_rgb8(0x6a, 0x34, 0x12))).into()), border: border::rounded(20), ..Default::default() });
    let mantel = container(column![
        mantel_pic,
        container(column![
            row![l.tb("Hearth OS Mantel", 30.0, Color::WHITE), container(l.t("Ready now", 13.0, Color::from_rgb8(0x7f, 0xe0, 0xa3))).padding(Padding { left: 10.0, right: 10.0, top: 3.0, bottom: 3.0 }).style(|_| container::Style { border: iced::Border { color: Color::from_rgb8(0x2f, 0x6b, 0x45), width: 1.0, radius: 13.0.into() }, ..Default::default() })].spacing(14).align_y(iced::Center),
            l.t("Hearth's own desktop, set up and ready: the bar, the dock, Quick Access and Game Mode one press away. Pick a look later if you like.", 16.0, muted),
            btn("Use Hearth OS Mantel", true, Message::FirstChoose("mantel".into()))
        ]
        .spacing(14))
        .padding(34)
    ])
    .width(card_w)
    .style(move |_| container::Style { background: Some(Color::from_rgb8(0x22, 0x1a, 0x16).into()), border: iced::Border { color: orange, width: 3.0, radius: 22.0.into() }, ..Default::default() });
    let tiles = [("console", "Console dashboard", "#1d3557", "#7a3b1d"), ("iconrow", "Icon row home", "#3a2550", "#6b4a3a"), ("cockpit", "Cockpit", "#062a33", "#0b1a2a"), ("cabin", "Cabin", "#4a2f17", "#1f140b"), ("retro", "1995 computer", "#008080", "#006a6a")];
    let tw = (card_w - 44.0) / 3.0;
    let tile = |id: &'static str, name: &'static str, f: &'static str, t: &'static str, h: f32| -> Element<'_, Message> {
        let (fc, tc) = (colour(f, Color::BLACK), colour(t, Color::BLACK));
        mouse_area(container(l.tb(name, 13.0, Color::WHITE)).padding(12).width(tw).height(h).align_bottom(h).style(move |_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, fc).add_stop(1.0, tc)).into()), border: border::rounded(6), ..Default::default() })).on_press(Message::FirstChoose(id.into())).interaction(mouse::Interaction::Pointer).into()
    };
    let nothing = mouse_area(container(l.t("+  Start from nothing", 15.0, orange)).center_x(tw).center_y(tw * 0.63).style(move |_| container::Style { border: iced::Border { color: orange, width: 2.0, radius: 6.0.into() }, ..Default::default() })).on_press(Message::FirstChoose("nothing".into())).interaction(mouse::Interaction::Pointer);
    let gallery = column![
        row![tile(tiles[0].0, tiles[0].1, tiles[0].2, tiles[0].3, tw * 0.47), tile(tiles[1].0, tiles[1].1, tiles[1].2, tiles[1].3, tw * 0.47), tile(tiles[2].0, tiles[2].1, tiles[2].2, tiles[2].3, tw * 0.47)].spacing(8),
        row![tile(tiles[3].0, tiles[3].1, tiles[3].2, tiles[3].3, tw * 0.63), tile(tiles[4].0, tiles[4].1, tiles[4].2, tiles[4].3, tw * 0.63), nothing].spacing(8)
    ]
    .spacing(8);
    let own = container(column![
        container(gallery).padding(14),
        container(column![
            l.tb("Build your own desktop", 30.0, Color::WHITE),
            l.t("Say it, sketch it, or start from one of these and change anything: where things go, how they look, sound and move. If Hearth hasn't got a part, your AI makes it.", 16.0, muted),
            row![btn("Build my own", false, Message::FirstChoose("build".into())), btn("Describe it", false, Message::FirstChoose("nothing".into()))].spacing(12)
        ]
        .spacing(14))
        .padding(34)
    ])
    .width(card_w)
    .style(|_| container::Style { background: Some(Color::from_rgb8(0x1d, 0x1a, 0x19).into()), border: iced::Border { color: a(Color::WHITE, 0.1), width: 1.0, radius: 22.0.into() }, ..Default::default() });
    let foot = row![l.t("A  Choose", 16.0, muted), l.t("◀ ▶  Move", 16.0, muted), l.t("Mouse, keyboard, touch or controller", 16.0, muted), mouse_area(l.t("Not now", 16.0, Color::from_rgb8(0x8a, 0x80, 0x79))).on_press(Message::FirstChoose("later".into())).interaction(mouse::Interaction::Pointer)].spacing(30);
    container(column![head, row![mantel, own].spacing(40), foot].spacing(60).align_x(iced::Center))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style { background: Some(iced::Gradient::Linear(iced::gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, Color::from_rgb8(0x12, 0x0c, 0x08)).add_stop(1.0, Color::from_rgb8(0x2a, 0x15, 0x08))).into()), ..Default::default() })
        .into()
}

/// A wake-up every 10 seconds (your AI's blocks look again), on its own small thread.
pub fn ten_seconds() -> futures::channel::mpsc::UnboundedReceiver<()> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("mantel-blocks".into()).stack_size(64 * 1024).spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        if tx.unbounded_send(()).is_err() {
            return;
        }
    });
    rx
}
