//! hearth-imagine-app: Hearth Imagine (docs/mockups/checklists/open-source-06-imagine.md; the approved board
//! docs/mockups/open-source-apps/06-imagine.png). Pictures from words with ComfyUI (GPL-3) on the Nvidia PC: Type and
//! make on the left (say what you want), the node graph it runs in the middle (change it there if you like), the
//! results, the queue, earlier pictures and the models on the right. Nothing leaves your network: ComfyUI is the Home
//! server helper's (hearth-comfyui.service, 127.0.0.1 and the PC's Tailscale address only); on another device, its
//! address (http://nvidia-pc:8188) is set once in the Type and make panel.
//!
//! Your choices: ~/.config/nimbus/imagine.json (only what you change). What you asked for lately and your own graphs:
//! ~/.local/share/hearth/imagine (written whole, atomically). Finished pictures: ~/Pictures/Imagine too.
//!
//!   --view type|both|graph     which side shows         $HEARTH_IMAGINE_COMFY_DIR: ComfyUI's folder (its models' sizes)

mod comfy;
mod graph;

use comfy::Ask;
use graph::{Node, Wire};
use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, image, mouse_area, row, scrollable, slider, text_editor, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task, Vector};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Stats(Result<Value, String>),
    Info(Vec<String>, Vec<String>),
    History(Value),
    Queue(Value),
    HomeAi(Value),
    Picture(String, Result<(Value, Vec<u8>), String>),
    Earlier(usize, Option<Vec<u8>>),
    View(&'static str),
    Prompt(text_editor::Action),
    Leave(String),
    Look(&'static str),
    Size(&'static str),
    Count(u32),
    Model(String),
    Steps(f32),
    StepsDone,
    NewSeed,
    Make,
    Queued(String, Vec<Result<String, String>>),
    Lately(usize),
    Select(usize),
    MoreLike,
    Bigger,
    Wallpaper,
    Done(Result<String, String>),
    Menu(&'static str),
    AddStep(&'static str),
    StartFrom(String),
    SaveOwn,
    Server(String),
    ServerDone,
    Install,
    TurnOn,
    Ran(run::Output),
    MoveNode(&'static str, f32, f32),
    Pan(f32, f32),
    Zoom(f32),
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

#[derive(Debug, Clone)]
struct Pic {
    /// its place in ComfyUI's queue (the order they were asked for)
    num: i64,
    id: String,
    step: (u32, u32),
    done: Option<(Value, iced::widget::image::Handle, Vec<u8>)>,
    failed: String,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    prefs: Prefs,
    base: String,
    server_edit: Option<String>,
    up: Option<bool>,
    card: String,
    vram: f64,
    home_ai: Value,
    models: Vec<String>,
    upscalers: Vec<String>,
    prompt: text_editor::Content,
    leave: String,
    set_title: String,
    pics: Vec<Pic>,
    sel: usize,
    waiting: Vec<(String, String)>,
    earlier: Vec<(Value, Option<iced::widget::image::Handle>)>,
    ws: Option<Receiver<String>>,
    client: String,
    nodes: Vec<Node>,
    wires: Vec<Wire>,
    zoom: f32,
    pan: Vector,
    menu: Option<&'static str>,
    said: String,
    recent: Vec<Value>,
    started_at: Option<std::time::Instant>,
    steps_live: f32,
}

fn data() -> PathBuf {
    hearth_oss::data_dir("imagine")
}
fn read(p: &std::path::Path) -> Value {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}
fn pictures_dir() -> PathBuf {
    hearth_ui::paths::home().join("Pictures/Imagine")
}
fn comfy_dir() -> PathBuf {
    std::env::var_os("HEARTH_IMAGINE_COMFY_DIR").map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join("ComfyUI"))
}

/// Work off the window's thread.
fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-imagine".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

/// The graph's steps where the board has them.
fn default_nodes() -> Vec<Node> {
    let n = |id: &'static str, title: &str, c: Color, x: f32, y: f32, w: f32| Node { id, title: title.into(), color: c, x, y, w, lines: Vec::new(), ports: Vec::new(), running: None };
    vec![
        n("load", "Load model", w::purple(), 40.0, 80.0, 210.0),
        n("describe", "Describe it", w::yellow(), 420.0, 290.0, 260.0),
        n("leave", "Leave out", w::yellow(), 420.0, 440.0, 260.0),
        n("empty", "Empty picture", w::lime(), 420.0, 570.0, 260.0),
        n("look", "Look: Film", w::lime(), 40.0, 780.0, 210.0),
        n("make", "Make", Color::from_rgb8(0xff, 0x8a, 0x3d), 740.0, 220.0, 220.0),
        n("save", "Turn into pictures", w::pink(), 740.0, 660.0, 220.0),
    ]
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = Prefs::load("imagine");
        let draft = read(&data().join("draft.json"));
        let mut nodes = default_nodes();
        if let Some(pos) = prefs.get("graph").and_then(Value::as_object) {
            for n in nodes.iter_mut() {
                if let Some(xy) = pos.get(n.id).and_then(Value::as_array) {
                    if let (Some(x), Some(y)) = (xy.first().and_then(Value::as_f64), xy.get(1).and_then(Value::as_f64)) {
                        n.x = x as f32;
                        n.y = y as f32;
                    }
                }
            }
        }
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("imagine", "Hearth Imagine").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            base: prefs.s("server", "http://127.0.0.1:8188").trim_end_matches('/').to_string(),
            prefs,
            server_edit: None,
            up: None,
            card: String::new(),
            vram: 0.0,
            home_ai: Value::Null,
            models: Vec::new(),
            upscalers: Vec::new(),
            prompt: text_editor::Content::with_text(draft.get("prompt").and_then(Value::as_str).unwrap_or("")),
            leave: draft.get("leaveOut").and_then(Value::as_str).unwrap_or("people, text, blurry").to_string(),
            set_title: String::new(),
            pics: Vec::new(),
            sel: 0,
            waiting: Vec::new(),
            earlier: Vec::new(),
            ws: None,
            client: format!("hearth-imagine-{}-{}", std::process::id(), hearth_oss::now()),
            nodes,
            wires: Vec::new(),
            zoom: 0.86,
            pan: Vector::new(0.0, 0.0),
            menu: None,
            said: String::new(),
            recent: read(&data().join("recent.json")).as_array().cloned().unwrap_or_default(),
            started_at: None,
            steps_live: 0.0,
        };
        if let Some(v) = app::arg("--view") {
            a.prefs.m.insert("view".into(), json!(v));
        }
        a.steps_live = a.steps() as f32;
        a.refresh_nodes();
        let t = ping(&a.base);
        (a, t)
    }
    fn view_mode(&self) -> String {
        self.prefs.s("view", "both")
    }
    fn look(&self) -> String {
        self.prefs.s("look", "Film")
    }
    fn size(&self) -> (String, u32, u32) {
        let s = self.prefs.s("size", "16:9");
        comfy::SIZES.iter().find(|x| x.0 == s).map(|x| (x.0.to_string(), x.1, x.2)).unwrap_or(("16:9".into(), 1344, 768))
    }
    fn count(&self) -> u32 {
        self.prefs.f("count", 4.0) as u32
    }
    fn steps(&self) -> u32 {
        self.prefs.f("steps", 28.0) as u32
    }
    fn seed(&self) -> u64 {
        self.prefs.f("seed", 482113.0) as u64
    }
    fn model(&self) -> String {
        let m = self.prefs.s("model", "");
        if m.is_empty() || (!self.models.is_empty() && !self.models.contains(&m)) {
            self.models.first().cloned().unwrap_or(m)
        } else {
            m
        }
    }
    fn bigger_step(&self) -> bool {
        self.prefs.b("bigger", false)
    }
    fn ask(&self, seed: u64) -> Ask {
        let (_, w, h) = self.size();
        Ask { prompt: self.prompt.text().trim().to_string(), leave_out: self.leave.clone(), look: self.look(), w, h, model: self.model(), steps: self.steps(), seed, cfg: 6.5, upscale: if self.bigger_step() { self.upscalers.first().cloned().unwrap_or_default() } else { String::new() }, from: None }
    }
    fn set(&mut self, k: &str, v: Value) {
        if let Err(e) = self.prefs.set(k, v) {
            self.said = e;
        }
    }
    fn busy(&self) -> bool {
        self.pics.iter().any(|p| p.done.is_none() && p.failed.is_empty()) || !self.waiting.is_empty()
    }
    /// A short name for what's asked: its first two parts, and the size when it isn't 16:9.
    fn title(&self) -> String {
        let p = self.prompt.text();
        let parts: Vec<&str> = p.split(',').map(str::trim).filter(|x| !x.is_empty()).take(2).collect();
        let mut s = ui::fit(&parts.join(", "), 48);
        let (sz, _, _) = self.size();
        if sz != "16:9" {
            s = format!("{s}, {sz}");
        }
        s
    }
    /// The graph's steps with what they hold now (the choices, the model, how far Make is).
    fn refresh_nodes(&mut self) {
        let ask = self.ask(self.seed());
        let (look, look_sub) = (self.look(), comfy::look_words(&self.look()).1);
        let running = self.pics.iter().find(|p| p.done.is_none() && p.failed.is_empty() && p.step.1 > 0).map(|p| p.step);
        let count = self.count();
        let up = self.upscalers.first().map(|u| comfy::pretty(u)).unwrap_or_else(|| "no upscaler yet".into());
        let prompt = self.prompt.text().trim().to_string();
        let words = if prompt.chars().count() > 60 { format!("{}…", prompt.chars().take(60).collect::<String>().trim_end_matches([',', ' '])) } else { prompt };
        for n in self.nodes.iter_mut() {
            n.running = None;
            match n.id {
                "load" => {
                    n.lines = vec![(comfy::pretty(&ask.model), false)];
                    n.ports = vec![("model".into(), w::purple()), ("text reader".into(), w::yellow()), ("picture maker".into(), w::blue())];
                }
                "describe" => {
                    let mut l: Vec<(String, bool)> = wrap(&words, 34).into_iter().map(|x| (x, true)).collect();
                    l.truncate(2);
                    n.lines = l;
                    n.ports = vec![("words".into(), Color::from_rgb8(0xff, 0x8a, 0x3d))];
                }
                "leave" => {
                    n.lines = vec![(if ask.leave_out.is_empty() { "nothing".into() } else { ui::fit(&ask.leave_out, 34) }, false)];
                    n.ports = Vec::new();
                }
                "empty" => {
                    n.lines = vec![(format!("{} × {} · {count} at once", ask.w, ask.h), false)];
                }
                "look" => {
                    n.title = format!("Look: {look}");
                    n.lines = vec![(look_sub.to_string(), false)];
                }
                "make" => {
                    n.lines = vec![(format!("steps {} · guide {}", ask.steps, ask.cfg), false), (format!("seed {}", comfy::seed_words(ask.seed)), false)];
                    n.running = running;
                }
                "save" => {
                    n.lines = vec![("then save to".into(), false), ("~/Pictures/Imagine".into(), false)];
                }
                "bigger" => {
                    n.lines = vec![(up.clone(), false), ("4 × the size".into(), false)];
                }
                _ => {}
            }
        }
        self.wires = wires(self.bigger_step());
        let has = self.nodes.iter().any(|n| n.id == "bigger");
        if self.bigger_step() && !has {
            self.nodes.push(Node { id: "bigger", title: "Bigger".into(), color: w::blue(), x: 1000.0, y: 480.0, w: 200.0, lines: vec![(up, false), ("4 × the size".into(), false)], ports: Vec::new(), running: None });
        } else if !self.bigger_step() && has {
            self.nodes.retain(|n| n.id != "bigger");
        }
    }
}

fn wrap(s: &str, n: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for w in s.split_whitespace() {
        if !line.is_empty() && line.chars().count() + w.chars().count() + 1 > n {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(w);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn wires(bigger: bool) -> Vec<Wire> {
    let wv = |from: &'static str, fy: f32, to: &'static str, ty: f32, c: Color| Wire { from, fy, to, ty, color: c };
    let orange = Color::from_rgb8(0xff, 0x8a, 0x3d);
    let mut v = vec![
        wv("load", 71.0, "make", 42.0, w::purple()),
        wv("load", 92.0, "describe", 50.0, w::yellow()),
        wv("load", 92.0, "leave", 50.0, w::yellow()),
        wv("load", 114.0, "save", 52.0, w::blue()),
        wv("describe", 92.0, "make", 72.0, orange),
        wv("leave", 52.0, "make", 96.0, orange),
        wv("empty", 50.0, "make", 118.0, orange),
        wv("look", 50.0, "describe", 70.0, w::lime()),
    ];
    if bigger {
        v.push(wv("make", 80.0, "bigger", 50.0, w::pink()));
        v.push(wv("bigger", 50.0, "save", 75.0, w::pink()));
    } else {
        v.push(wv("make", 80.0, "save", 75.0, w::pink()));
    }
    v
}

fn ping(base: &str) -> Task<Msg> {
    let b = base.to_string();
    Task::perform(bg(move || comfy::get(&b, "/system_stats")), |r| Msg::Stats(r.unwrap_or_else(|| Err("it stopped".into()))))
}

fn load_all(base: &str) -> Task<Msg> {
    let (b1, b2, b3) = (base.to_string(), base.to_string(), base.to_string());
    Task::batch([
        Task::perform(
            bg(move || {
                let c = comfy::get(&b1, "/object_info/CheckpointLoaderSimple").map(|j| comfy::choices(&j, "CheckpointLoaderSimple", "ckpt_name")).unwrap_or_default();
                let u = comfy::get(&b1, "/object_info/UpscaleModelLoader").map(|j| comfy::choices(&j, "UpscaleModelLoader", "model_name")).unwrap_or_default();
                (c, u)
            }),
            |r| {
                let (c, u) = r.unwrap_or_default();
                Msg::Info(c, u)
            },
        ),
        Task::perform(bg(move || comfy::get(&b2, "/history?max_items=40").unwrap_or(Value::Null)), |v| Msg::History(v.unwrap_or(Value::Null))),
        Task::perform(bg(move || comfy::get(&b3, "/queue").unwrap_or(Value::Null)), |v| Msg::Queue(v.unwrap_or(Value::Null))),
    ])
}

fn fetch_picture(base: &str, id: String) -> Task<Msg> {
    let b = base.to_string();
    let id2 = id.clone();
    Task::perform(
        bg(move || -> Result<(Value, Vec<u8>), String> {
            let h = comfy::get(&b, &format!("/history/{id2}"))?;
            let outs = h.pointer(&format!("/{id2}/outputs")).and_then(Value::as_object).cloned().unwrap_or_default();
            let img = outs.values().filter_map(|o| o.get("images").and_then(Value::as_array)).flatten().next().cloned().ok_or("ComfyUI made no picture")?;
            let bytes = comfy::view(&b, &img)?;
            // a copy in ~/Pictures/Imagine (ComfyUI keeps its own in its output folder on the Nvidia PC)
            let dir = pictures_dir();
            let _ = std::fs::create_dir_all(&dir);
            if let Some(name) = img.get("filename").and_then(Value::as_str) {
                let _ = std::fs::write(dir.join(name.rsplit('/').next().unwrap_or(name)), &bytes);
            }
            Ok((img, bytes))
        }),
        move |r| Msg::Picture(id.clone(), r.unwrap_or_else(|| Err("it stopped".into()))),
    )
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Stats(r) => match r {
            Ok(j) => {
                a.up = Some(true);
                let d = j.pointer("/devices/0").cloned().unwrap_or(Value::Null);
                a.card = comfy::card(d.get("name").and_then(Value::as_str).unwrap_or(""));
                a.vram = d.get("vram_total").and_then(Value::as_f64).unwrap_or(0.0);
                if a.ws.is_none() {
                    a.ws = Some(hearth_oss::ws::listen(format!("{}/ws?clientId={}", a.base, a.client), Vec::new(), Vec::new()));
                }
                return load_all(&a.base);
            }
            Err(_) => {
                a.up = Some(false);
                return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-home-server"), "home-ai"])), |o| Msg::HomeAi(o.json().unwrap_or(Value::Null)));
            }
        },
        Msg::HomeAi(v) => a.home_ai = v,
        Msg::Info(c, u) => {
            a.models = c;
            a.upscalers = u;
            a.refresh_nodes();
        }
        Msg::History(h) => {
            // finished pictures, newest first: this set's (by its name) and the earlier ones
            let mut items: Vec<(i64, String, Value)> = h.as_object().map(|o| o.iter().map(|(id, v)| (v.pointer("/prompt/0").and_then(Value::as_i64).unwrap_or(0), id.clone(), v.clone())).collect()).unwrap_or_default();
            items.sort_by(|x, y| y.0.cmp(&x.0));
            let mut t = Vec::new();
            let mut earlier = Vec::new();
            for (num, id, v) in items {
                let title = v.pointer("/prompt/3/hearth_title").and_then(Value::as_str).unwrap_or("").to_string();
                if !a.set_title.is_empty() && title == a.set_title {
                    match a.pics.iter_mut().find(|p| p.id == id) {
                        Some(pc) => {
                            pc.num = num;
                            if pc.done.is_none() && pc.failed.is_empty() {
                                t.push(fetch_picture(&a.base, id));
                            }
                        }
                        None => {
                            a.pics.push(Pic { num, id: id.clone(), step: (0, 0), done: None, failed: String::new() });
                            t.push(fetch_picture(&a.base, id));
                        }
                    }
                    continue;
                }
                let img = v.get("outputs").and_then(Value::as_object).and_then(|o| o.values().filter_map(|x| x.get("images").and_then(Value::as_array)).flatten().next().cloned());
                if let Some(img) = img {
                    if earlier.len() < 6 {
                        earlier.push((img, None));
                    }
                }
            }
            a.pics.sort_by_key(|p| p.num);
            a.earlier = earlier;
            for (i, (img, _)) in a.earlier.iter().enumerate() {
                let (b, img) = (a.base.clone(), img.clone());
                t.push(Task::perform(bg(move || comfy::view(&b, &img).ok()), move |r| Msg::Earlier(i, r.flatten())));
            }
            return Task::batch(t);
        }
        Msg::Earlier(i, b) => {
            if let (Some(e), Some(b)) = (a.earlier.get_mut(i), b) {
                e.1 = Some(iced::widget::image::Handle::from_bytes(b));
            }
        }
        Msg::Queue(q) => {
            // what's running and waiting: a set already started (after the app opened again) is picked up by its name
            let mut items: Vec<(i64, String, String, bool)> = Vec::new();
            for (k, running) in [("queue_running", true), ("queue_pending", false)] {
                for it in q.get(k).and_then(Value::as_array).cloned().unwrap_or_default() {
                    let num = it.get(0).and_then(Value::as_i64).unwrap_or(0);
                    let id = it.get(1).and_then(Value::as_str).unwrap_or("").to_string();
                    let title = it.pointer("/3/hearth_title").and_then(Value::as_str).unwrap_or("A picture").to_string();
                    items.push((num, id, title, running));
                }
            }
            items.sort();
            if a.set_title.is_empty() {
                if let Some(r) = items.iter().find(|x| x.3).or(items.first()) {
                    a.set_title = r.2.clone();
                }
            }
            a.waiting.clear();
            let steps = a.steps();
            for (num, id, title, running) in items {
                if title == a.set_title {
                    match a.pics.iter_mut().find(|p| p.id == id) {
                        Some(pc) => {
                            pc.num = num;
                            if running && pc.step.1 == 0 {
                                pc.step = (0, steps);
                            }
                        }
                        None => a.pics.push(Pic { num, id, step: (0, if running { steps } else { 0 }), done: None, failed: String::new() }),
                    }
                } else if !a.waiting.iter().any(|w| w.0 == title) {
                    a.waiting.push((title, id));
                }
            }
            a.pics.sort_by_key(|p| p.num);
            a.refresh_nodes();
            if !a.set_title.is_empty() {
                let b = a.base.clone();
                return Task::perform(bg(move || comfy::get(&b, "/history?max_items=40").unwrap_or(Value::Null)), |v| Msg::History(v.unwrap_or(Value::Null)));
            }
        }
        Msg::Tick => {
            let mut t = Vec::new();
            let mut msgs = Vec::new();
            if let Some(rx) = &a.ws {
                while let Ok(s) = rx.try_recv() {
                    msgs.push(s);
                }
            }
            let mut queue_again = false;
            for s in msgs {
                let j: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
                let kind = j.get("type").and_then(Value::as_str).unwrap_or("");
                let d = j.get("data").cloned().unwrap_or(Value::Null);
                let pid = d.get("prompt_id").and_then(Value::as_str).unwrap_or("").to_string();
                match kind {
                    "progress" => {
                        if let Some(p) = a.pics.iter_mut().find(|p| p.id == pid) {
                            p.step = (d.get("value").and_then(Value::as_u64).unwrap_or(0) as u32, d.get("max").and_then(Value::as_u64).unwrap_or(0) as u32);
                            if a.started_at.is_none() {
                                a.started_at = Some(std::time::Instant::now());
                            }
                        }
                    }
                    "executing" if d.get("node").map(Value::is_null).unwrap_or(false) => {
                        if a.pics.iter().any(|p| p.id == pid && p.done.is_none()) {
                            t.push(fetch_picture(&a.base, pid.clone()));
                        }
                        queue_again = true;
                    }
                    "execution_error" => {
                        if let Some(p) = a.pics.iter_mut().find(|p| p.id == pid) {
                            p.failed = d.get("exception_message").and_then(Value::as_str).unwrap_or("ComfyUI stopped").to_string();
                        }
                        queue_again = true;
                    }
                    "status" => queue_again = true,
                    _ => {}
                }
            }
            a.refresh_nodes();
            if queue_again {
                let b = a.base.clone();
                t.push(Task::perform(bg(move || comfy::get(&b, "/queue").unwrap_or(Value::Null)), |v| Msg::Queue(v.unwrap_or(Value::Null))));
            }
            return Task::batch(t);
        }
        Msg::Picture(id, r) => {
            if let Some(p) = a.pics.iter_mut().find(|p| p.id == id) {
                match r {
                    Ok((img, bytes)) => {
                        p.done = Some((img, iced::widget::image::Handle::from_bytes(bytes.clone()), bytes));
                        if let Some(st) = a.started_at {
                            // how long a step takes on this PC, for the estimate under Make
                            let steps = a.pics.iter().filter(|p| p.done.is_some()).count().max(1) as f64 * a.steps().max(1) as f64;
                            let per = st.elapsed().as_secs_f64() / steps;
                            if per > 0.01 && per < 30.0 {
                                let _ = hearth_ui::settings_file::update_json(&data().join("speed.json"), |m| {
                                    m.insert("secsPerStep".into(), json!(per));
                                });
                            }
                        }
                    }
                    Err(e) => p.failed = e,
                }
            }
            a.refresh_nodes();
        }
        Msg::View(v) => a.set("view", json!(v)),
        Msg::Prompt(act) => {
            a.prompt.perform(act);
            a.refresh_nodes();
        }
        Msg::Leave(s) => {
            a.leave = s;
            a.refresh_nodes();
        }
        Msg::Look(l) => {
            a.set("look", json!(l));
            a.refresh_nodes();
        }
        Msg::Size(s) => {
            a.set("size", json!(s));
            a.refresh_nodes();
        }
        Msg::Count(n) => {
            a.set("count", json!(n));
            a.refresh_nodes();
        }
        Msg::Model(m) => {
            a.menu = None;
            a.set("model", json!(m));
            a.refresh_nodes();
        }
        Msg::Steps(v) => {
            a.steps_live = v.round();
            if let Some(l) = a.nodes.iter_mut().find(|n| n.id == "make").and_then(|n| n.lines.get_mut(0)) {
                l.0 = format!("steps {} · guide 6.5", v.round());
            }
        }
        Msg::StepsDone => {
            let v = a.steps_live;
            a.set("steps", json!(v));
            a.refresh_nodes();
        }
        Msg::NewSeed => {
            let s = (hearth_oss::now() as u64).wrapping_mul(6364136223846793005).wrapping_add(std::process::id() as u64) % 999_999 + 1;
            a.set("seed", json!(s));
            a.refresh_nodes();
        }
        Msg::Make => return make(a, None),
        Msg::Queued(title, ids) => {
            let mut bad = Vec::new();
            for r in ids {
                match r {
                    Ok(id) => {
                        if title == a.set_title && !a.pics.iter().any(|p| p.id == id) {
                            a.pics.push(Pic { num: i64::MAX, id, step: (0, 0), done: None, failed: String::new() });
                        }
                    }
                    Err(e) => bad.push(e),
                }
            }
            if let Some(e) = bad.first() {
                a.said = e.clone();
            }
            let b = a.base.clone();
            return Task::perform(bg(move || comfy::get(&b, "/queue").unwrap_or(Value::Null)), |v| Msg::Queue(v.unwrap_or(Value::Null)));
        }
        Msg::Lately(i) => {
            if let Some(r) = a.lately().get(i).cloned() {
                a.prompt = text_editor::Content::with_text(r.get("prompt").and_then(Value::as_str).unwrap_or(""));
                a.leave = r.get("leaveOut").and_then(Value::as_str).unwrap_or("").to_string();
                for k in ["look", "size", "count", "steps"] {
                    if let Some(v) = r.get(k) {
                        a.set(k, v.clone());
                    }
                }
                a.refresh_nodes();
            }
        }
        Msg::Select(i) => a.sel = i,
        Msg::MoreLike => {
            let Some((_, _, bytes)) = a.pics.get(a.sel).and_then(|p| p.done.clone()) else { return Task::none() };
            return make(a, Some((bytes, format!("More like {}", a.sel + 1))));
        }
        Msg::Bigger => {
            let Some((img, _, bytes)) = a.pics.get(a.sel).and_then(|p| p.done.clone()) else { return Task::none() };
            let Some(up) = a.upscalers.first().cloned() else {
                a.said = "ComfyUI has no upscaler yet: put one in its models/upscale_models folder.".into();
                return Task::none();
            };
            let (b, client, n) = (a.base.clone(), a.client.clone(), a.sel + 1);
            let name = img.get("filename").and_then(Value::as_str).unwrap_or("picture.png").to_string();
            let title = format!("Bigger, picture {n}");
            a.said = format!("Making picture {n} 4 × bigger…");
            let t2 = title.clone();
            return Task::perform(
                bg(move || {
                    let up_name = comfy::upload(&b, &format!("hearth-{name}"), &bytes)?;
                    comfy::queue_prompt(&b, &comfy::bigger(&up_name, &up, "Imagine/bigger"), &client, &t2)
                }),
                move |r| Msg::Queued(title.clone(), vec![r.unwrap_or_else(|| Err("it stopped".into()))]),
            );
        }
        Msg::Wallpaper => {
            let Some((img, _, bytes)) = a.pics.get(a.sel).and_then(|p| p.done.clone()) else { return Task::none() };
            let name = img.get("filename").and_then(Value::as_str).unwrap_or("picture.png").rsplit('/').next().unwrap_or("picture.png").to_string();
            return Task::perform(
                bg(move || {
                    let dir = pictures_dir();
                    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                    let f = dir.join(&name);
                    std::fs::write(&f, &bytes).map_err(|e| e.to_string())?;
                    let o = run::blocking(&run::args(&["plasma-apply-wallpaperimage", &f.to_string_lossy()]));
                    if o.ok {
                        Ok(format!("It's your wallpaper ({}).", f.display()))
                    } else {
                        Err(format!("The wallpaper couldn't be set: {}", o.why()))
                    }
                }),
                |r| Msg::Done(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Done(r) => {
            a.said = match r {
                Ok(s) | Err(s) => s,
            }
        }
        Msg::Menu(m) => a.menu = if a.menu == Some(m) { None } else { Some(m) },
        Msg::AddStep(s) => {
            a.menu = None;
            if s == "bigger" {
                let on = !a.bigger_step();
                a.set("bigger", json!(on));
                a.refresh_nodes();
            }
        }
        Msg::StartFrom(name) => {
            a.menu = None;
            if name == "hearth" {
                a.nodes = default_nodes();
                a.set("bigger", json!(false));
                a.set("graph", json!({}));
            } else {
                let g = read(&data().join("workflows").join(format!("{name}.json")));
                if let Some(s) = g.get("prompt").and_then(Value::as_str) {
                    a.prompt = text_editor::Content::with_text(s);
                }
                a.leave = g.get("leaveOut").and_then(Value::as_str).unwrap_or(&a.leave).to_string();
                for k in ["look", "size", "count", "steps", "bigger", "graph"] {
                    if let Some(v) = g.get(k) {
                        a.set(k, v.clone());
                    }
                }
                a.nodes = default_nodes();
                if let Some(pos) = g.get("graph").and_then(Value::as_object) {
                    for n in a.nodes.iter_mut() {
                        if let Some(xy) = pos.get(n.id).and_then(Value::as_array) {
                            n.x = xy.first().and_then(Value::as_f64).unwrap_or(n.x as f64) as f32;
                            n.y = xy.get(1).and_then(Value::as_f64).unwrap_or(n.y as f64) as f32;
                        }
                    }
                }
            }
            a.refresh_nodes();
        }
        Msg::SaveOwn => {
            let name: String = a.title().chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { ' ' }).collect::<String>().split_whitespace().collect::<Vec<_>>().join("-").to_lowercase();
            let name = if name.is_empty() { format!("graph-{}", hearth_oss::now()) } else { name };
            let pos: serde_json::Map<String, Value> = a.nodes.iter().map(|n| (n.id.to_string(), json!([n.x, n.y]))).collect();
            let v = json!({"title": a.title(), "prompt": a.prompt.text().trim(), "leaveOut": a.leave, "look": a.look(), "size": a.size().0, "count": a.count(), "steps": a.steps(), "bigger": a.bigger_step(), "graph": pos,
                           "comfyui": comfy::graph(&a.ask(a.seed()), "Imagine/picture")});
            let dir = data().join("workflows");
            let _ = std::fs::create_dir_all(&dir);
            a.said = match hearth_ui::settings_file::write_json(&dir.join(format!("{name}.json")), &v) {
                Ok(_) => format!("Saved as your own: \"{}\" (Start from has it; its ComfyUI graph is in it too).", a.title()),
                Err(e) => format!("It couldn't be saved: {e}"),
            };
        }
        Msg::Server(s) => a.server_edit = Some(s),
        Msg::ServerDone => {
            if let Some(s) = a.server_edit.take() {
                let s = s.trim().trim_end_matches('/').to_string();
                let s = if s.starts_with("http://") { s } else { format!("http://{s}") };
                let s = if s.rsplit(':').next().map(|x| x.parse::<u16>().is_ok()).unwrap_or(false) { s } else { format!("{s}:8188") };
                a.set("server", json!(s));
                a.base = s;
                a.ws = None;
                a.up = None;
                return ping(&a.base);
            }
        }
        Msg::Install => {
            a.said = "Installing ComfyUI on this PC (the Home server helper does it in the background; it takes a while).".into();
            return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-home-server"), "pictures-install"])), Msg::Ran);
        }
        Msg::TurnOn => {
            a.said = "Turning ComfyUI on…".into();
            return Task::perform(run::spawn(run::args(&["python3", &run::shell_helper("hearth-home-server"), "set", "picturesOn", "on"])), Msg::Ran);
        }
        Msg::Ran(o) => {
            if !o.ok {
                a.said = format!("That didn't work: {}", o.why());
            }
            let b = a.base.clone();
            return Task::perform(bg(move || {
                std::thread::sleep(std::time::Duration::from_secs(4));
                comfy::get(&b, "/system_stats")
            }), |r| Msg::Stats(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::MoveNode(id, dx, dy) => {
            if let Some(n) = a.nodes.iter_mut().find(|n| n.id == id) {
                n.x += dx;
                n.y += dy;
            }
            // where the steps are is kept once they stop moving (a write per drag would be too many)
            let pos: serde_json::Map<String, Value> = a.nodes.iter().map(|n| (n.id.to_string(), json!([n.x.round(), n.y.round()]))).collect();
            a.prefs.m.insert("graph".into(), Value::Object(pos));
        }
        Msg::Pan(dx, dy) => a.pan = Vector::new(a.pan.x + dx, a.pan.y + dy),
        Msg::Zoom(z) => a.zoom = z.clamp(0.3, 2.0),
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => {
            if let Some(g) = a.prefs.m.get("graph").cloned() {
                let mut pr = Prefs::load("imagine");
                if pr.get("graph") != Some(&g) {
                    let _ = pr.set("graph", g);
                }
            }
            return win::close();
        }
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

impl App {
    /// What you asked for lately (not what's open now).
    fn lately(&self) -> Vec<Value> {
        let now = self.prompt.text().trim().to_string();
        self.recent.iter().filter(|r| r.get("prompt").and_then(Value::as_str).map(|p| p.trim() != now).unwrap_or(false)).take(3).cloned().collect()
    }
}

/// Make N pictures (or N more like one): each its own run in ComfyUI's queue, seeds one after another, so each comes
/// back as soon as it's done.
fn make(a: &mut App, like: Option<(Vec<u8>, String)>) -> Task<Msg> {
    if a.up != Some(true) {
        a.said = "ComfyUI isn't reachable yet.".into();
        return Task::none();
    }
    if a.prompt.text().trim().is_empty() {
        a.said = "Say what you want first (What to make).".into();
        return Task::none();
    }
    if a.model().is_empty() {
        a.said = "ComfyUI has no model yet: put one in its models/checkpoints folder.".into();
        return Task::none();
    }
    let n = a.count().max(1);
    let seed = a.seed();
    let title = match &like {
        Some((_, t)) => t.clone(),
        None => a.title(),
    };
    // what you asked for, kept for Used lately and for the next time the app opens (written whole)
    if like.is_none() {
        let entry = json!({"title": title, "prompt": a.prompt.text().trim(), "leaveOut": a.leave, "look": a.look(), "size": a.size().0, "count": n, "steps": a.steps()});
        a.recent.retain(|r| r.get("prompt") != entry.get("prompt"));
        a.recent.insert(0, entry.clone());
        a.recent.truncate(12);
        let _ = std::fs::create_dir_all(data());
        let _ = hearth_ui::settings_file::write_json(&data().join("recent.json"), &Value::Array(a.recent.clone()));
        let _ = hearth_ui::settings_file::write_json(&data().join("draft.json"), &entry);
    }
    let asks: Vec<Ask> = (0..n as u64).map(|i| a.ask(seed + i)).collect();
    let (b, client) = (a.base.clone(), a.client.clone());
    let prefix = format!("Imagine/{}", title.chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect::<String>().split_whitespace().take(4).collect::<Vec<_>>().join("-").to_lowercase());
    if a.busy() && title != a.set_title {
        a.said = format!("\"{title}\" waits its turn.");
    } else {
        a.set_title = title.clone();
        a.pics.clear();
        a.sel = 0;
        a.started_at = None;
    }
    let t2 = title.clone();
    Task::perform(
        bg(move || {
            let from = match like {
                Some((bytes, _)) => match comfy::upload(&b, &format!("hearth-like-{}.png", hearth_oss::now()), &bytes) {
                    Ok(n) => Some(n),
                    Err(e) => return vec![Err(e)],
                },
                None => None,
            };
            asks.into_iter()
                .map(|mut q| {
                    if let Some(f) = &from {
                        q.from = Some((f.clone(), 0.55));
                    }
                    comfy::queue_prompt(&b, &comfy::graph(&q, &prefix), &client, &t2)
                })
                .collect()
        }),
        move |r| Msg::Queued(title.clone(), r.unwrap_or_default()),
    )
}

// ---------------------------------------------------------------- the window

fn type_and_make(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let head = column![w::h2(&p, "Type and make"), t("Say what you want. The graph on the right is what it runs; change it there if you like.", 12.0, p.muted)].spacing(4.0);
    let mut setup = Column::new();
    if a.up == Some(false) {
        let pic = a.home_ai.get("pictures").cloned().unwrap_or(Value::Null);
        let installed = pic.get("installed").and_then(Value::as_bool).unwrap_or(false);
        let installing = pic.get("installing").and_then(Value::as_bool).unwrap_or(false);
        let on = pic.get("on").and_then(Value::as_bool).unwrap_or(false);
        let (line, btn): (String, Option<Element<'_, Msg>>) = if a.home_ai.is_null() {
            (format!("ComfyUI doesn't answer at {}. On another device, type the Nvidia PC's address.", a.base), None)
        } else if installing {
            (format!("Installing ComfyUI: {}", pic.get("step").and_then(Value::as_str).unwrap_or("…")), None)
        } else if !installed {
            ("ComfyUI isn't on this PC yet. Hearth installs its checked release, as you, in ~/ComfyUI.".into(), Some(w::chip(&p, "Install ComfyUI", true, Sz::Sm, Some(Msg::Install))))
        } else if !on {
            ("ComfyUI is installed but off (it's stopped while you play).".into(), Some(w::chip(&p, "Turn it on", true, Sz::Sm, Some(Msg::TurnOn))))
        } else {
            ("ComfyUI is starting…".into(), None)
        };
        let mut c = column![t(line, 12.0, p.text)].spacing(8.0);
        if let Some(b) = btn {
            c = c.push(b);
        }
        let srv = a.server_edit.clone().unwrap_or_else(|| a.base.clone());
        c = c.push(row![w::input(&p, "http://nvidia-pc:8188", &srv, 12.0, Msg::Server).on_submit(Msg::ServerDone), w::chip(&p, "Use it", false, Sz::Sm, Some(Msg::ServerDone))].spacing(6.0).align_y(iced::Center));
        setup = setup.push(ui::gap(1.0, 10.0)).push(w::card2(&p, c));
    }
    let what = column![
        w::cap(&p, "What to make"),
        container(text_editor(&a.prompt).on_action(Msg::Prompt).height(Length::Fixed(120.0)).size(14.0).padding(12.0)).style(ui::boxed(p.bg, p.r(10.0), Some((p.line, 1.0)))),
    ];
    let leave = column![w::cap(&p, "Leave out"), w::input(&p, "people, text, blurry", &a.leave, 13.0, Msg::Leave)];
    let look = a.look();
    let mut chips = Row::new().spacing(6.0);
    let mut chips2 = Row::new().spacing(6.0);
    for (i, (l, _, _)) in comfy::LOOKS.iter().enumerate() {
        let c = w::chip(&p, *l, look == *l, Sz::Sm, Some(Msg::Look(*l)));
        if i < 5 {
            chips = chips.push(c);
        } else {
            chips2 = chips2.push(c);
        }
    }
    let looks = column![w::cap(&p, "Look"), column![chips, chips2].spacing(6.0)];
    let (sz, sw, sh) = a.size();
    let size = column![
        w::cap(&p, "Size"),
        row![w::seg(&p, comfy::SIZES.iter().map(|s| (s.0.to_string(), s.0 == sz, Some(Msg::Size(s.0)))).collect()), w::field(&p, format!("{sw} × {sh}"), 0.0, None::<Msg>)].spacing(6.0).align_y(iced::Center),
    ];
    let n = a.count();
    let mut model_c = Column::new().push(w::kv(&p, "Model", w::field(&p, format!("{} ▾", if a.models.is_empty() { "none yet".into() } else { comfy::pretty(&a.model()) }), 180.0, Some(Msg::Menu("model")))));
    if a.menu == Some("model") {
        let cur = a.model();
        model_c = model_c.push(row![Space::new().width(Length::Fill), w::menu(&p, a.models.iter().map(|m| (comfy::pretty(m), *m == cur, Msg::Model(m.clone()))).collect(), 240.0)]);
    }
    let numbers = column![
        ui::gap(1.0, 8.0),
        w::kv(&p, "How many", w::seg(&p, [1u32, 2, 4, 8].iter().map(|k| (k.to_string(), *k == n, Some(Msg::Count(*k)))).collect())),
        model_c,
        w::kv(
            &p,
            "Steps",
            row![slider(1.0..=60.0, a.steps_live, Msg::Steps).step(1.0f32).on_release(Msg::StepsDone).width(Length::Fixed(120.0)).style(w::slider_style(&p, 1.0)), container(t(format!("{}", a.steps_live as u32), 12.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right)]
                .spacing(8.0)
                .align_y(iced::Center)
                .into()
        ),
        w::kv(&p, "Same result again (seed)", w::field(&p, format!("{} ↻", comfy::seed_words(a.seed())), 0.0, Some(Msg::NewSeed))),
    ]
    .spacing(2.0);
    let per = read(&data().join("speed.json")).get("secsPerStep").and_then(Value::as_f64).unwrap_or(0.125);
    let secs = (per * a.steps() as f64 * n as f64).round().max(1.0);
    let where_ = "on the Nvidia PC";
    let make_b = column![
        ui::gap(1.0, 16.0),
        w::wide(&p, format!("✦ Make {n} {}", if n == 1 { "picture" } else { "pictures" }), (a.up == Some(true)).then_some(Msg::Make)),
        container(t(format!("About {secs} s {where_} · nothing leaves your network"), 12.0, p.muted)).width(Length::Fill).align_x(iced::alignment::Horizontal::Center).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    ];
    let mut lately = Column::new().spacing(6.0).push(container(w::cap(&p, "Used lately")).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    for (i, r) in a.lately().iter().enumerate() {
        let title = r.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        lately = lately.push(mouse_area(t(format!("{title} ›"), 12.0, p.muted)).on_press(Msg::Lately(i)).interaction(mouse::Interaction::Pointer));
    }
    lately = lately.push(ui::gap(1.0, 8.0)).push(
        mouse_area(row![ui::glyph(G::Pencil, p.muted, 13.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer),
    );
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "type",
        vec![("head", column![head, setup].into()), ("what", what.into()), ("leave", leave.into()), ("look", looks.into()), ("size", size.into()), ("numbers", numbers.into()), ("make", make_b.into()), ("lately", lately.into())],
        |ps| column![ps.take("head"), ps.take("what"), ps.take("leave"), ps.take("look"), ps.take("size"), ps.take("numbers"), ps.take("make"), ps.take("lately")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(400.0)).height(Length::Fill).padding(Padding { top: 18.0, right: 20.0, bottom: 18.0, left: 20.0 }).style(w::side_bg(&p)).into()
}

fn graph_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let bigger = a.bigger_step();
    let g = graph::Graph { nodes: &a.nodes, wires: &a.wires, zoom: a.zoom, pan: a.pan, p, on_move: Msg::MoveNode, on_pan: Msg::Pan, on_zoom: Msg::Zoom };
    let cv: Element<'_, Msg> = canvas(g).width(Length::Fill).height(Length::Fill).into();
    let mut add = Column::new().push(w::chip(&p, "＋ Add a step", false, Sz::Sm, Some(Msg::Menu("add"))));
    if a.menu == Some("add") {
        add = add.push(w::menu(&p, vec![(if bigger { "Take out: Bigger (the upscaler)".to_string() } else { format!("Bigger: {} after Make", a.upscalers.first().map(|u| comfy::pretty(u)).unwrap_or_else(|| "an upscaler".into())) }, bigger, Msg::AddStep("bigger"))], 260.0));
    }
    let mut start = Column::new().push(w::chip(&p, "Start from ▾", false, Sz::Sm, Some(Msg::Menu("start"))));
    if a.menu == Some("start") {
        let mut items = vec![("Hearth's own: words to a picture".to_string(), false, Msg::StartFrom("hearth".into()))];
        if let Ok(rd) = std::fs::read_dir(data().join("workflows")) {
            let mut names: Vec<String> = rd.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(str::to_string)).collect();
            names.sort();
            for nm in names {
                items.push((nm.replace('-', " "), false, Msg::StartFrom(nm)));
            }
        }
        start = start.push(w::menu(&p, items, 260.0));
    }
    let tools = row![add, start, w::chip(&p, "Save as my own", false, Sz::Sm, Some(Msg::SaveOwn))].spacing(8.0);
    let zoom = row![w::chip(&p, "−", false, Sz::Sm, Some(Msg::Zoom(a.zoom - 0.1))), w::chip(&p, format!("{} %", (a.zoom * 100.0).round()), false, Sz::Sm, Some(Msg::Zoom(0.86))), w::chip(&p, "＋", false, Sz::Sm, Some(Msg::Zoom(a.zoom + 0.1)))].spacing(8.0);
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "graph",
        vec![("tools", tools.into()), ("zoom", zoom.into()), ("nodes", cv)],
        |ps| iced::widget::stack![ps.take("nodes"), container(row![ps.take("tools"), Space::new().width(Length::Fill), ps.take("zoom")].align_y(iced::alignment::Vertical::Top)).padding(Padding { top: 16.0, right: 20.0, bottom: 0.0, left: 20.0 }).width(Length::Fill)].into(),
        Msg::Describe,
    )
}

/// A picture still being made: the boards' stripes.
struct Stripes(Color, Color);
impl canvas::Program<Msg> for Stripes {
    type State = ();
    fn draw(&self, _s: &(), r: &iced::Renderer, _t: &iced::Theme, b: iced::Rectangle, _c: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut f = canvas::Frame::new(r, b.size());
        f.fill(&canvas::Path::rectangle(iced::Point::ORIGIN, b.size()), self.0);
        let mut x = 6.0;
        while x < b.width {
            f.fill(&canvas::Path::rectangle(iced::Point::new(x, 0.0), iced::Size::new(6.0, b.height)), self.1);
            x += 12.0;
        }
        vec![f.into_geometry()]
    }
}

fn tile<'a>(p: &Pal, img: Option<&iced::widget::image::Handle>, label: String, h: f32, sel: bool, msg: Option<Msg>) -> Element<'a, Msg> {
    let inner: Element<'a, Msg> = match img {
        Some(hd) => image(hd.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(Length::Fixed(h)).into(),
        None => canvas(Stripes(Color::from_rgb8(0x1a, 0x1c, 0x22), Color::from_rgb8(0x22, 0x25, 0x2c))).width(Length::Fill).height(Length::Fixed(h)).into(),
    };
    let mut layers = vec![inner];
    if !label.is_empty() {
        layers.push(container(container(t(label, 11.0, Color::WHITE)).padding(Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 6.0 }).style(ui::boxed(Color::from_rgba(0.0, 0.0, 0.0, 0.62), 5.0, None))).width(Length::Fill).height(Length::Fixed(h)).align_y(iced::alignment::Vertical::Bottom).padding(8.0).into());
    }
    let c = container(iced::widget::Stack::with_children(layers)).clip(true).style(ui::boxed(Color::TRANSPARENT, p.r(12.0), Some((if sel { p.accent } else { Color::TRANSPARENT }, 2.0))));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

fn results(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let n_set = a.pics.len();
    let head = row![w::h2(&p, "Results"), Space::new().width(Length::Fill), t(if n_set > 0 { format!("this one · {n_set}") } else { String::new() }, 12.0, p.muted)].align_y(iced::Center);
    let mut grid = Column::new().spacing(10.0);
    if a.pics.is_empty() {
        grid = grid.push(container(t(if a.up == Some(true) { "Your pictures come here as soon as each is made." } else { "ComfyUI isn't reachable yet." }, 12.0, p.muted)).padding(Padding { top: 10.0, right: 0.0, bottom: 10.0, left: 0.0 }));
    }
    for (r, pair) in a.pics.chunks(2).enumerate() {
        let mut rw = Row::new().spacing(10.0);
        for (k, pc) in pair.iter().enumerate() {
            let i = r * 2 + k;
            let label = if pc.done.is_some() {
                format!("{} · done", i + 1)
            } else if !pc.failed.is_empty() {
                format!("{} · stopped", i + 1)
            } else if pc.step.1 > 0 {
                format!("{} · step {}", i + 1, pc.step.0)
            } else {
                format!("{} · waiting", i + 1)
            };
            let el = tile(&p, pc.done.as_ref().map(|d| &d.1), label, 120.0, a.sel == i && pc.done.is_some() && n_set > 1, pc.done.as_ref().map(|_| Msg::Select(i)));
            rw = rw.push(container(el).width(Length::FillPortion(1)));
        }
        if pair.len() == 1 {
            rw = rw.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(rw);
    }
    let have = a.pics.get(a.sel).map(|p| p.done.is_some()).unwrap_or(false);
    let k = a.sel + 1;
    let actions = row![
        w::chip(&p, format!("Make more like {k}"), false, Sz::Sm, have.then_some(Msg::MoreLike)),
        w::chip(&p, "Bigger", false, Sz::Sm, have.then_some(Msg::Bigger)),
        w::chip(&p, "Set as wallpaper", false, Sz::Sm, have.then_some(Msg::Wallpaper)),
    ]
    .spacing(6.0);
    let mut waiting = Column::new().spacing(8.0).push(w::cap(&p, "Waiting"));
    if a.waiting.is_empty() {
        waiting = waiting.push(t("Nothing else waiting.", 12.0, p.muted));
    }
    for (i, (title, _)) in a.waiting.iter().enumerate() {
        let when = match i {
            0 => "next".to_string(),
            1 => "2nd".to_string(),
            2 => "3rd".to_string(),
            k => format!("{}th", k + 1),
        };
        waiting = waiting.push(w::card2(&p, row![t(title.clone(), 12.0, p.text), Space::new().width(Length::Fill), t(when, 12.0, p.muted)]).padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 }));
    }
    let mut earlier = Column::new().spacing(8.0).push(w::cap(&p, "Earlier"));
    for chunk in a.earlier.chunks(3) {
        let mut rw = Row::new().spacing(8.0);
        for (img, h) in chunk {
            let _ = img;
            rw = rw.push(container(tile(&p, h.as_ref(), String::new(), 76.0, false, None)).width(Length::FillPortion(1)));
        }
        for _ in chunk.len()..3 {
            rw = rw.push(Space::new().width(Length::FillPortion(1)));
        }
        earlier = earlier.push(rw);
    }
    let mut models = Column::new().spacing(4.0).push(w::cap(&p, "Models on the Nvidia PC"));
    let cd = comfy_dir();
    let size_of = |sub: &str, f: &str| std::fs::metadata(cd.join("models").join(sub).join(f)).map(|m| format!(" · {}", hearth_oss::size_words(m.len()))).unwrap_or_default();
    for m in &a.models {
        models = models.push(t(format!("{}{}", comfy::pretty(m), size_of("checkpoints", m)), 12.0, p.muted));
    }
    for u in &a.upscalers {
        models = models.push(t(format!("{}{}", comfy::pretty(u), size_of("upscale_models", u)), 12.0, p.muted));
    }
    let mut said = Column::new();
    if !a.said.is_empty() {
        said = said.push(container(t(a.said.clone(), 12.0, p.text)).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "results",
        vec![("head", head.into()), ("pictures", column![grid, said].into()), ("actions", actions.into()), ("waiting", waiting.into()), ("earlier", earlier.into()), ("models", models.into())],
        |ps| column![ps.take("head"), ui::gap(1.0, 12.0), ps.take("pictures"), ui::gap(1.0, 10.0), ps.take("actions"), ps.take("waiting"), ps.take("earlier"), ps.take("models")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(420.0)).height(Length::Fill).padding(Padding { top: 18.0, right: 20.0, bottom: 18.0, left: 20.0 }).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.12), 0.0, None)).into()
}

use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Imagine", blocks: &[bl("type", "Type and make", 0, 0, 3, 11), bl("graph", "The node graph", 3, 0, 6, 11), bl("results", "Results", 9, 0, 3, 11), bl("credit", "Built on ComfyUI (GPL-3)", 0, 11, 12, 1)] },
    PageDef {
        id: "type",
        title: "Type and make",
        blocks: &[
            bl("head", "Type and make", 0, 0, 12, 1),
            bl("what", "What to make", 0, 1, 12, 2),
            bl("leave", "Leave out", 0, 3, 12, 1),
            bl("look", "Look", 0, 4, 12, 1),
            bl("size", "Size", 0, 5, 12, 1),
            bl("numbers", "How many, model, steps, seed", 0, 6, 12, 2),
            bl("make", "Make", 0, 8, 12, 2),
            bls("lately", "Used lately", 0, 10, 12, 2),
        ],
    },
    PageDef { id: "graph", title: "The node graph", blocks: &[bl("tools", "Add a step, Start from, Save as my own", 0, 0, 8, 1), bl("zoom", "Zoom", 8, 0, 4, 1), bl("nodes", "The steps and their wires", 0, 1, 12, 11)] },
    PageDef {
        id: "results",
        title: "Results",
        blocks: &[bl("head", "Results", 0, 0, 12, 1), bl("pictures", "This one's pictures", 0, 1, 12, 3), bl("actions", "Make more, Bigger, Set as wallpaper", 0, 4, 12, 1), bl("waiting", "Waiting", 0, 5, 12, 2), bl("earlier", "Earlier", 0, 7, 12, 3), bls("models", "Models on the Nvidia PC", 0, 10, 12, 2)],
    },
];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mode = a.view_mode();
    let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![
        ("type", hearth_ui::studio::made(&a.describe, "window", "type", || type_and_make(a))),
        ("graph", hearth_ui::studio::made(&a.describe, "window", "graph", || graph_page(a))),
        ("results", hearth_ui::studio::made(&a.describe, "window", "results", || results(a))),
        ("credit", w::credit(&p, "Hearth Imagine", "Built on ComfyUI (GPL-3)")),
    ];
    for (id, which) in [("pane:queue", "queue"), ("pane:earlier", "earlier"), ("pane:models", "models")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let base = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        items,
        move |ps| {
            let r: Element<'_, Msg> = match mode.as_str() {
                "type" => row![ps.take("type"), ps.take("results")].height(Length::Fill).into(),
                "graph" => row![ps.take("graph"), ps.take("results")].height(Length::Fill).into(),
                _ => row![ps.take("type"), ps.take("graph"), ps.take("results")].height(Length::Fill).into(),
            };
            column![r, ps.take("credit")].into()
        },
        Msg::Describe,
    );
    let vm = a.view_mode();
    let seg = w::seg(&p, vec![("Type and make".to_string(), vm == "type", Some(Msg::View("type"))), ("Both".to_string(), vm == "both", Some(Msg::View("both"))), ("Node graph".to_string(), vm == "graph", Some(Msg::View("graph")))]);
    let tag: Element<'_, Msg> = match a.up {
        Some(true) => w::tag(&p, format!("Nvidia PC · {}{}", a.card, if a.vram > 0.0 { format!(" · {} GB", (a.vram / 1073741824.0).round()) } else { String::new() }), Tg::Ok),
        Some(false) => w::tag(&p, "ComfyUI not reachable", Tg::Warn),
        None => w::tag(&p, "Looking for ComfyUI…", Tg::Dk),
    };
    let middle: Element<'_, Msg> = container(row![seg, Space::new().width(Length::Fill), tag].spacing(8.0).align_y(iced::Center)).width(Length::FillPortion(30)).into();
    let bar = hearth_ui::describe::title_bar(&p, &a.describe, G::Palette, "Hearth Imagine", Some(middle), TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    let bar: Element<'_, Msg> = if a.describe.zone("titlebar").is_none() { iced::widget::stack![bar, container(w::mark("IM", "pu", 22.0)).padding(Padding { top: 9.0, right: 0.0, bottom: 0.0, left: 11.0 })].into() } else { bar };
    let all = container(column![bar, ui::hline(p.line), base]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

use hearth_ui::studio::{tile as ktile, Kit};
const KIT: Kit = Kit {
    name: "Imagine",
    buttons: &[
        ktile("make", "Make the pictures", G::Play),
        ktile("new-seed", "A new seed", G::Sync),
        ktile("more-like", "Make more like this", G::Plus),
        ktile("bigger", "Bigger", G::Maximize),
        ktile("wallpaper", "Set as wallpaper", G::Display),
        ktile("save-own", "Save as my own", G::Down),
        ktile("add-bigger", "Add or take out Bigger", G::Plus),
        ktile("only-type", "Only Type and make", G::Columns),
        ktile("both", "Both", G::Columns),
        ktile("only-graph", "Only the node graph", G::Grid),
        ktile("zoom-in", "Zoom in", G::Plus),
        ktile("zoom-out", "Zoom out", G::Minimize),
    ],
    panes: &[ktile("queue", "The queue", G::List), ktile("earlier", "Earlier pictures", G::Camera), ktile("models", "Models on the Nvidia PC", G::Drive)],
    info: &[ktile("card", "The graphics card", G::Display), ktile("waiting", "Waiting", G::Clock), ktile("done", "Done in this one", G::Check), ktile("seed", "The seed", G::Sync)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "make" => Msg::Make,
        "new-seed" => Msg::NewSeed,
        "more-like" => Msg::MoreLike,
        "bigger" => Msg::Bigger,
        "wallpaper" => Msg::Wallpaper,
        "save-own" => Msg::SaveOwn,
        "add-bigger" => Msg::AddStep("bigger"),
        "only-type" => Msg::View("type"),
        "both" => Msg::View("both"),
        "only-graph" => Msg::View("graph"),
        "zoom-in" => Msg::Zoom(a.zoom + 0.1),
        "zoom-out" => Msg::Zoom(a.zoom - 0.1),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    a.describe.values(vec![
        ("card", a.card.clone()),
        ("waiting", a.waiting.len().to_string()),
        ("done", a.pics.iter().filter(|p| p.done.is_some()).count().to_string()),
        ("seed", comfy::seed_words(a.seed())),
    ]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "queue" => {
            for (i, (title, _)) in a.waiting.iter().enumerate() {
                c = c.push(row![t(title.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(format!("{}", i + 1), 12.0, p.muted)]);
            }
        }
        "earlier" => {
            for (_, h) in a.earlier.iter().take(3) {
                c = c.push(tile(&p, h.as_ref(), String::new(), 60.0, false, None));
            }
        }
        _ => {
            for m in a.models.iter().chain(a.upscalers.iter()) {
                c = c.push(t(comfy::pretty(m), 13.0, p.text));
            }
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    if a.ws.is_some() && a.busy() {
        s.push(iced::time::every(std::time::Duration::from_millis(300)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-imagine-app", "Hearth Imagine: pictures from words with ComfyUI (GPL-3) on your Nvidia PC.\n  --view type|both|graph") {
        return Ok(());
    }
    let w = app::window("hearth-imagine", (1840.0, 970.0), (1200.0, 640.0));
    iced::application(App::new, update, shown).title(|_: &App| "Hearth Imagine".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
