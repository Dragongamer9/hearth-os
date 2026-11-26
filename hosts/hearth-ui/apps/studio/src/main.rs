//! Hearth Studio, in pure Rust (hearth-ui, iced): the approved board docs/mockups/open-source-apps/12-studio.png
//! (checklist docs/mockups/checklists/open-source-12-studio.md). Built on OBS Studio (GPL-2), driven through its own
//! remote control, obs-websocket (obs.rs): what viewers see (OBS's program, as pictures OBS makes), going live and
//! recording, the stream's health, your scenes and what's in them, the sound mixer with OBS's own meters and the mic's
//! filters, and how scenes switch. Everything it shows comes from OBS and everything it does, OBS does.
//!
//! Hearth's own on top: also a 9:16 copy of each recording for phones (ffmpeg crops it when the recording ends), the
//! stream title on your title overlay, Twitch's viewer count (decapi.me: no account or key), Hearth's Ember
//! transition. Hotkeys while it's open: Ctrl+1–6 scenes, Ctrl+Shift+R save the last 30 s; the controller's ≡ + Y the
//! next scene (read straight from /dev/input, Hearth Notes' pad.rs).
//!
//! At rest (not connected): no timers; the connection thread sleeps between tries. Connected: OBS's numbers once a
//! second and its picture once a second, only while the window is in front.

#[path = "../../notes/src/pad.rs"]
#[allow(dead_code)]
mod pad;
mod obs;
mod view;
mod ws;

use hearth_ui::app::{self, win};
use hearth_ui::{run, settings_file, Pal};
use iced::keyboard::{Event as KEvent, Key};
use iced::{Element, Subscription, Task};
use serde_json::{json, Map, Value};
use std::collections::VecDeque;
use std::path::PathBuf;

pub fn prefs_path() -> PathBuf {
    hearth_ui::paths::home().join(".config/hearth/studio.json")
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub on: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Audio {
    pub name: String,
    pub kind: String,
    pub db: f64,
    pub muted: bool,
    /// the loudest of its channels now (0–1, OBS's peak)
    pub level: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Out {
    pub active: bool,
    pub paused: bool,
    pub timecode: String,
    pub bytes: u64,
    pub skipped: u64,
    pub total: u64,
    pub congestion: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dlg {
    NewScene,
    AddSource,
    Filters,
    Where,
    Password,
    Ember,
    Said(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Obs(obs::Ev),
    Every,
    Viewers(run::Output),
    Scene(String),
    PickItem(i64),
    ToggleItem(i64, bool),
    LockItem(i64, bool),
    Volume(String, f32),
    Mute(String, bool),
    Transition(String),
    Duration(String),
    DurationDone,
    Title(String),
    TitleDone,
    Quality(u32, u32),
    Vertical(bool),
    Stream,
    Record,
    Pause,
    Replay,
    VirtualCam,
    StudioMode,
    Open(Dlg),
    Close,
    NewName(String),
    CreateScene,
    AddSource(&'static str, &'static str),
    Filter(String, bool),
    AddFilter(&'static str, &'static str),
    Service(&'static str),
    Key(String),
    SaveService,
    Password(String),
    SavePassword,
    Install,
    Installed(run::Output),
    StartObs,
    Cropped(run::Output),
    Pad(pad::Button),
    KeyEv(KEvent),
    Focus(bool),
    Drag,
    Max,
    Min,
    Quit,
    Resize(iced::window::Direction),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub up: bool,
    pub down_why: String,
    pub scenes: Vec<String>,
    pub scene: String,
    pub items: Vec<Item>,
    pub item_sel: Option<i64>,
    pub audio: Vec<Audio>,
    pub stream: Out,
    pub record: Out,
    pub replay: bool,
    pub vcam: bool,
    pub studio_mode: bool,
    pub fps: f64,
    pub cpu: f64,
    pub encoder: String,
    pub transitions: Vec<String>,
    pub transition: String,
    pub duration: String,
    pub base: (u32, u32),
    pub output: (u32, u32, u32),
    pub services: Vec<String>,
    pub shot: Option<iced::widget::image::Handle>,
    pub record_dir: String,
    pub mic_filters: Vec<(String, String, bool)>,
    pub mic: String,
    pub rate: VecDeque<f32>,
    pub last_bytes: Option<(u64, std::time::Instant)>,
    pub mbps: f64,
    pub viewers: Option<u64>,
    pub prefs: Map<String, Value>,
    pub title_edit: String,
    pub dlg: Option<Dlg>,
    pub new_name: String,
    pub key_edit: String,
    pub service_pick: &'static str,
    pub pw_edit: String,
    pub busy: String,
    pub said: String,
    pub focused: bool,
    pub menu_at: Option<std::time::Instant>,
    pub ticks: u64,
}

pub fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
fn n(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

/// "00:42:13.123" (obs-websocket's timecode) → "0:42:13".
pub fn short_tc(t: &str) -> String {
    let t = t.split('.').next().unwrap_or("");
    t.strip_prefix('0').filter(|x| x.len() >= 7).unwrap_or(t).to_string()
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = settings_file::read(&prefs_path()).map().unwrap_or_default();
        if let Some(pw) = prefs.get("password").and_then(Value::as_str) {
            obs::set_password(pw);
        }
        let a = App {
            describe: hearth_ui::describe::Describe::new("studio", "Hearth Studio").with_pages(view::BLOCKS).with_kit(&view::KIT),
            p: hearth_ui::theme::load(),
            up: false,
            down_why: "connecting".into(),
            scenes: vec![],
            scene: String::new(),
            items: vec![],
            item_sel: None,
            audio: vec![],
            stream: Out::default(),
            record: Out::default(),
            replay: false,
            vcam: false,
            studio_mode: false,
            fps: 0.0,
            cpu: 0.0,
            encoder: String::new(),
            transitions: vec![],
            transition: String::new(),
            duration: String::new(),
            base: (1920, 1080),
            output: (1920, 1080, 60),
            services: vec![],
            shot: None,
            record_dir: String::new(),
            mic_filters: vec![],
            mic: String::new(),
            rate: VecDeque::new(),
            last_bytes: None,
            mbps: 0.0,
            viewers: None,
            title_edit: prefs.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
            prefs,
            dlg: None,
            new_name: String::new(),
            key_edit: String::new(),
            service_pick: "Twitch",
            pw_edit: String::new(),
            busy: String::new(),
            said: String::new(),
            focused: true,
            menu_at: None,
            ticks: 0,
        };
        (a, Task::none())
    }

    pub fn vertical(&self) -> bool {
        self.prefs.get("vertical").and_then(Value::as_bool).unwrap_or(true)
    }

    fn save(&mut self, f: impl FnOnce(&mut Map<String, Value>)) {
        if let Some(d) = prefs_path().parent() {
            let _ = std::fs::create_dir_all(d);
        }
        match settings_file::update_json(&prefs_path(), |m| f(m)) {
            Ok((m, _)) => self.prefs = m,
            Err(e) => self.said = format!("It couldn't be saved: {e}"),
        }
    }

    /// The source whose text shows the stream's title ("Overlay: title", else the first text source named Overlay…).
    fn title_source(&self) -> Option<String> {
        let names: Vec<&Item> = self.items.iter().filter(|i| i.kind.starts_with("text_")).collect();
        names.iter().find(|i| i.name == "Overlay: title").or_else(|| names.iter().find(|i| i.name.to_lowercase().starts_with("overlay"))).map(|i| i.name.clone())
    }
}

fn first_reads() {
    for r in ["GetSceneList", "GetSceneTransitionList", "GetStreamServiceSettings", "GetVideoSettings", "GetRecordDirectory", "GetStudioModeEnabled", "GetVirtualCamStatus", "GetReplayBufferStatus", "GetInputList", "GetStreamStatus", "GetRecordStatus", "GetStats"] {
        obs::req(r, json!({}));
    }
    obs::request("GetProfileParameter", "encoder-mode", json!({"parameterCategory": "Output", "parameterName": "Mode"}));
}

fn read_items(scene: &str) {
    if !scene.is_empty() {
        obs::req("GetSceneItemList", json!({"sceneName": scene}));
    }
}

fn read_shot(a: &App) {
    if !a.scene.is_empty() {
        obs::req("GetSourceScreenshot", json!({"sourceName": a.scene, "imageFormat": "jpg", "imageWidth": 960, "imageCompressionQuality": 70}));
    }
}

fn read_audio(a: &mut App, inputs: &[Value]) {
    a.audio.retain(|x| inputs.iter().any(|i| s(i, "inputName") == x.name));
    for i in inputs {
        let name = s(i, "inputName").to_string();
        if !a.audio.iter().any(|x| x.name == name) {
            a.audio.push(Audio { name: name.clone(), kind: s(i, "inputKind").to_string(), db: -100.0, ..Audio::default() });
        }
        obs::request("GetInputVolume", &format!("vol\u{1}{name}"), json!({"inputName": name}));
        obs::request("GetInputMute", &format!("mute\u{1}{name}"), json!({"inputName": name}));
    }
    // the mic: the first microphone input (its filters: noise removal, leveller)
    a.mic = inputs.iter().find(|i| s(i, "inputKind").contains("input_capture")).map(|i| s(i, "inputName").to_string()).unwrap_or_default();
    if !a.mic.is_empty() {
        obs::request("GetSourceFilterList", "mic-filters", json!({"sourceName": a.mic}));
    }
}

/// One of OBS's answers.
fn answer(a: &mut App, d: &Value) {
    let kind = s(d, "requestType");
    let id = s(d, "requestId");
    let ok = d.get("requestStatus").map(|x| b(x, "result")).unwrap_or(false);
    let r = d.get("responseData").cloned().unwrap_or(Value::Null);
    if !ok {
        // an input that has no sound isn't in the mixer
        if let Some(name) = id.strip_prefix("vol\u{1}") {
            a.audio.retain(|x| x.name != name);
        } else if !matches!(kind, "GetSourceScreenshot" | "GetInputMute" | "GetSourceFilterList" | "GetProfileParameter" | "GetReplayBufferStatus" | "GetVirtualCamStatus") {
            let c = d.get("requestStatus").and_then(|x| x.get("comment")).and_then(Value::as_str).unwrap_or("");
            a.said = format!("OBS said no to {kind}{}", if c.is_empty() { String::new() } else { format!(": {c}") });
        }
        return;
    }
    match kind {
        "GetSceneList" => {
            let mut v: Vec<String> = r.get("scenes").and_then(Value::as_array).map(|x| x.iter().map(|s0| s(s0, "sceneName").to_string()).collect()).unwrap_or_default();
            let idx: Vec<i64> = r.get("scenes").and_then(Value::as_array).map(|x| x.iter().map(|s0| s0.get("sceneIndex").and_then(Value::as_i64).unwrap_or(0)).collect()).unwrap_or_default();
            let mut pairs: Vec<(i64, String)> = idx.into_iter().zip(v).collect();
            pairs.sort_by_key(|p| std::cmp::Reverse(p.0));
            a.scenes = pairs.into_iter().map(|p| p.1).collect();
            a.scene = s(&r, "currentProgramSceneName").to_string();
            read_items(&a.scene.clone());
        }
        "GetSceneItemList" => {
            let mut v: Vec<Item> = r
                .get("sceneItems")
                .and_then(Value::as_array)
                .map(|x| {
                    x.iter()
                        .map(|i| Item { id: i.get("sceneItemId").and_then(Value::as_i64).unwrap_or(0), name: s(i, "sourceName").to_string(), kind: s(i, "inputKind").to_string(), on: b(i, "sceneItemEnabled"), locked: b(i, "sceneItemLocked") })
                        .collect()
                })
                .unwrap_or_default();
            // OBS lists them bottom first: the top one first, as its own list shows
            v.reverse();
            if let Some(want) = app::arg("--pick-item") {
                a.item_sel = v.iter().find(|i| i.name == want).map(|i| i.id);
            }
            a.items = v;
        }
        "GetInputList" => {
            let inputs = r.get("inputs").and_then(Value::as_array).cloned().unwrap_or_default();
            read_audio(a, &inputs);
        }
        "GetInputVolume" => {
            if let Some(name) = id.strip_prefix("vol\u{1}") {
                if let Some(x) = a.audio.iter_mut().find(|x| x.name == name) {
                    x.db = n(&r, "inputVolumeDb");
                }
            }
        }
        "GetInputMute" => {
            if let Some(name) = id.strip_prefix("mute\u{1}") {
                if let Some(x) = a.audio.iter_mut().find(|x| x.name == name) {
                    x.muted = b(&r, "inputMuted");
                }
            }
        }
        "GetSourceFilterList" => {
            a.mic_filters = r.get("filters").and_then(Value::as_array).map(|x| x.iter().map(|f| (s(f, "filterName").to_string(), s(f, "filterKind").to_string(), b(f, "filterEnabled"))).collect()).unwrap_or_default();
        }
        "GetSceneTransitionList" => {
            a.transitions = r.get("transitions").and_then(Value::as_array).map(|x| x.iter().map(|t| s(t, "transitionName").to_string()).collect()).unwrap_or_default();
            a.transition = s(&r, "currentSceneTransitionName").to_string();
            if let Some(ms) = r.get("currentSceneTransitionDuration").and_then(Value::as_f64) {
                a.duration = format!("{} ms", ms as i64);
            }
        }
        "GetStreamServiceSettings" => {
            let st = r.get("streamServiceSettings").cloned().unwrap_or(Value::Null);
            let svc = s(&st, "service");
            let name = if svc.is_empty() { if s(&r, "streamServiceType").contains("custom") { "Custom server".to_string() } else { String::new() } } else { svc.split(" - ").next().unwrap_or(svc).to_string() };
            a.services = if name.is_empty() { vec![] } else { vec![name] };
        }
        "GetVideoSettings" => {
            a.base = (n(&r, "baseWidth") as u32, n(&r, "baseHeight") as u32);
            let fps = (n(&r, "fpsNumerator") / n(&r, "fpsDenominator").max(1.0)).round() as u32;
            a.output = (n(&r, "outputWidth") as u32, n(&r, "outputHeight") as u32, fps);
        }
        "GetRecordDirectory" => a.record_dir = s(&r, "recordDirectory").to_string(),
        "GetStudioModeEnabled" => a.studio_mode = b(&r, "studioModeEnabled"),
        "GetVirtualCamStatus" => a.vcam = b(&r, "outputActive"),
        "GetReplayBufferStatus" => a.replay = b(&r, "outputActive"),
        "GetStreamStatus" => {
            a.stream = Out {
                active: b(&r, "outputActive"),
                paused: false,
                timecode: short_tc(s(&r, "outputTimecode")),
                bytes: n(&r, "outputBytes") as u64,
                skipped: n(&r, "outputSkippedFrames") as u64,
                total: n(&r, "outputTotalFrames") as u64,
                congestion: n(&r, "outputCongestion"),
            };
            let now = std::time::Instant::now();
            if let Some((was, at)) = a.last_bytes {
                let secs = now.duration_since(at).as_secs_f64();
                if secs > 0.2 && a.stream.bytes >= was {
                    a.mbps = (a.stream.bytes - was) as f64 * 8.0 / secs / 1_000_000.0;
                }
            }
            if !a.stream.active {
                a.mbps = 0.0;
            }
            a.last_bytes = Some((a.stream.bytes, now));
            if a.rate.len() >= 60 {
                a.rate.pop_front();
            }
            a.rate.push_back(a.mbps as f32);
        }
        "GetRecordStatus" => {
            a.record = Out { active: b(&r, "outputActive"), paused: b(&r, "outputPaused"), timecode: short_tc(s(&r, "outputTimecode")), bytes: n(&r, "outputBytes") as u64, ..Out::default() };
        }
        "GetStats" => {
            a.fps = n(&r, "activeFps");
            a.cpu = n(&r, "cpuUsage");
        }
        "GetSourceScreenshot" => {
            let data = s(&r, "imageData");
            if let Some(i) = data.find(";base64,") {
                a.shot = Some(iced::widget::image::Handle::from_bytes(ws::unb64(&data[i + 8..])));
            }
        }
        "GetProfileParameter" => {
            if id == "encoder-mode" {
                let mode = s(&r, "parameterValue");
                let (cat, name) = if mode == "Advanced" { ("AdvOut", "Encoder") } else { ("SimpleOutput", "StreamEncoder") };
                obs::request("GetProfileParameter", "encoder", json!({"parameterCategory": cat, "parameterName": name}));
            } else if id == "encoder" {
                let e = s(&r, "parameterValue").to_lowercase();
                a.encoder = if e.contains("nvenc") || e.contains("vaapi") || e.contains("amf") || e.contains("qsv") || e.contains("ffmpeg_") && !e.contains("x264") {
                    "GPU encoder".into()
                } else if e.is_empty() {
                    String::new()
                } else {
                    "CPU encoder".into()
                };
            }
        }
        "SaveReplayBuffer" => a.said = "Saved the last 30 s.".into(),
        _ => {}
    }
}

/// One of OBS's events.
fn event(a: &mut App, d: &Value) -> Task<Msg> {
    let kind = s(d, "eventType");
    let e = d.get("eventData").cloned().unwrap_or(Value::Null);
    match kind {
        "InputVolumeMeters" => {
            for i in e.get("inputs").and_then(Value::as_array).cloned().unwrap_or_default() {
                let name = s(&i, "inputName");
                let peak = i.get("inputLevelsMul").and_then(Value::as_array).map(|ch| ch.iter().filter_map(|c| c.get(1).and_then(Value::as_f64)).fold(0.0f64, f64::max)).unwrap_or(0.0);
                if let Some(x) = a.audio.iter_mut().find(|x| x.name == name) {
                    x.level = peak as f32;
                }
            }
        }
        "CurrentProgramSceneChanged" => {
            a.scene = s(&e, "sceneName").to_string();
            read_items(&a.scene.clone());
            read_shot(a);
        }
        "SceneListChanged" | "SceneCreated" | "SceneRemoved" | "SceneNameChanged" => obs::req("GetSceneList", json!({})),
        "SceneItemCreated" | "SceneItemRemoved" | "SceneItemListReindexed" => read_items(&a.scene.clone()),
        "SceneItemEnableStateChanged" => {
            let id = e.get("sceneItemId").and_then(Value::as_i64).unwrap_or(-1);
            if let Some(x) = a.items.iter_mut().find(|x| x.id == id) {
                x.on = b(&e, "sceneItemEnabled");
            }
        }
        "SceneItemLockStateChanged" => {
            let id = e.get("sceneItemId").and_then(Value::as_i64).unwrap_or(-1);
            if let Some(x) = a.items.iter_mut().find(|x| x.id == id) {
                x.locked = b(&e, "sceneItemLocked");
            }
        }
        "InputVolumeChanged" => {
            if let Some(x) = a.audio.iter_mut().find(|x| x.name == s(&e, "inputName")) {
                x.db = n(&e, "inputVolumeDb");
            }
        }
        "InputMuteStateChanged" => {
            if let Some(x) = a.audio.iter_mut().find(|x| x.name == s(&e, "inputName")) {
                x.muted = b(&e, "inputMuted");
            }
        }
        "InputCreated" | "InputRemoved" | "InputNameChanged" => obs::req("GetInputList", json!({})),
        "SourceFilterCreated" | "SourceFilterRemoved" | "SourceFilterEnableStateChanged" => {
            if !a.mic.is_empty() {
                obs::request("GetSourceFilterList", "mic-filters", json!({"sourceName": a.mic}));
            }
        }
        "StreamStateChanged" => {
            a.stream.active = b(&e, "outputActive");
            obs::req("GetStreamStatus", json!({}));
        }
        "RecordStateChanged" => {
            a.record.active = b(&e, "outputActive");
            obs::req("GetRecordStatus", json!({}));
            // a recording ended: its 9:16 copy for phones, when that's on
            if s(&e, "outputState") == "OBS_WEBSOCKET_OUTPUT_STOPPED" && a.vertical() {
                let path = s(&e, "outputPath").to_string();
                if !path.is_empty() {
                    return crop(a, &path);
                }
            }
        }
        "RecordStateChanged2" => {}
        "ReplayBufferStateChanged" => a.replay = b(&e, "outputActive"),
        "ReplayBufferSaved" => a.said = format!("Saved the last 30 s: {}", s(&e, "savedReplayPath")),
        "VirtualcamStateChanged" => a.vcam = b(&e, "outputActive"),
        "StudioModeStateChanged" => a.studio_mode = b(&e, "studioModeEnabled"),
        "CurrentSceneTransitionChanged" => a.transition = s(&e, "transitionName").to_string(),
        "CurrentSceneTransitionDurationChanged" => a.duration = format!("{} ms", n(&e, "transitionDuration") as i64),
        "ExitStarted" => {
            a.up = false;
            a.down_why = "not running".into();
        }
        _ => {}
    }
    Task::none()
}

/// A 9:16 copy of a recording (the middle of the picture, full height), beside it: "<name>-vertical.mp4".
fn crop(a: &mut App, path: &str) -> Task<Msg> {
    let src = PathBuf::from(path);
    let stem = src.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
    let out = src.with_file_name(format!("{stem}-vertical.mp4"));
    a.busy = "Making the 9:16 copy for phones…".into();
    let cmd = vec![
        "ffmpeg".into(),
        "-nostdin".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        path.to_string(),
        "-vf".into(),
        "crop=ih*9/16:ih,scale=1080:1920".into(),
        "-c:a".into(),
        "copy".into(),
        out.display().to_string(),
    ];
    Task::perform(run::spawn(cmd), Msg::Cropped)
}

fn every(a: &mut App) -> Task<Msg> {
    if !a.up {
        return Task::none();
    }
    a.ticks += 1;
    obs::req("GetStats", json!({}));
    obs::req("GetStreamStatus", json!({}));
    obs::req("GetRecordStatus", json!({}));
    // what viewers see: every other second, only while Hearth Studio is in front
    if a.focused && a.ticks % 2 == 0 {
        read_shot(a);
    }
    // the viewers once a minute while live (Twitch, through decapi.me: no account or key)
    let ch = a.prefs.get("twitch").and_then(Value::as_str).unwrap_or("").to_string();
    if a.stream.active && !ch.is_empty() && a.ticks % 60 == 1 && std::env::var_os("HEARTH_STUDIO_VIEWERS").is_none() {
        let url = format!("https://decapi.me/twitch/viewercount/{}", ch.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect::<String>());
        return Task::perform(run::spawn(vec!["curl".into(), "-fsS".into(), "--max-time".into(), "10".into(), url]), Msg::Viewers);
    }
    Task::none()
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(view::block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Obs(ev) => match ev {
            obs::Ev::Up => {
                a.up = true;
                a.down_why.clear();
                // CI's pictures: Twitch's count can't be asked there
                if let Ok(v) = std::env::var("HEARTH_STUDIO_VIEWERS") {
                    a.viewers = v.parse().ok();
                }
                first_reads();
                match app::arg("--show").as_deref() {
                    Some("filters") => a.dlg = Some(Dlg::Filters),
                    Some("add") => a.dlg = Some(Dlg::AddSource),
                    Some("where") => a.dlg = Some(Dlg::Where),
                    _ => {}
                }
            }
            obs::Ev::Down(why) => {
                a.up = false;
                a.down_why = why;
                a.audio.iter_mut().for_each(|x| x.level = 0.0);
            }
            obs::Ev::Got(v) => {
                let d = v.get("d").cloned().unwrap_or(Value::Null);
                if v.get("op").and_then(Value::as_u64) == Some(7) {
                    answer(a, &d);
                } else {
                    return event(a, &d);
                }
            }
        },
        Msg::Every => return every(a),
        Msg::Viewers(o) => a.viewers = o.out.trim().parse::<u64>().ok(),
        Msg::Scene(sn) => {
            if a.studio_mode {
                obs::req("SetCurrentPreviewScene", json!({"sceneName": sn}));
            } else {
                obs::req("SetCurrentProgramScene", json!({"sceneName": sn}));
            }
        }
        Msg::PickItem(id) => a.item_sel = Some(id),
        Msg::ToggleItem(id, on) => obs::req("SetSceneItemEnabled", json!({"sceneName": a.scene, "sceneItemId": id, "sceneItemEnabled": on})),
        Msg::LockItem(id, on) => obs::req("SetSceneItemLocked", json!({"sceneName": a.scene, "sceneItemId": id, "sceneItemLocked": on})),
        Msg::Volume(name, x) => {
            // the fader: cubic (as OBS's own): x³ is the volume, so 60·log10(x) dB
            let db = if x <= 0.001 { -100.0 } else { (60.0 * (x as f64).log10()).max(-100.0) };
            if let Some(au) = a.audio.iter_mut().find(|au| au.name == name) {
                au.db = db;
            }
            obs::req("SetInputVolume", json!({"inputName": name, "inputVolumeDb": db}));
        }
        Msg::Mute(name, on) => obs::req("SetInputMute", json!({"inputName": name, "inputMuted": on})),
        Msg::Transition(t) => {
            if a.transitions.iter().any(|x| *x == t) {
                obs::req("SetCurrentSceneTransition", json!({"transitionName": t}));
            } else if t == "Ember" {
                a.dlg = Some(Dlg::Ember);
            } else {
                a.said = format!("OBS has no {t} transition in this scene collection.");
            }
        }
        Msg::Duration(d) => a.duration = d,
        Msg::DurationDone => {
            let ms: String = a.duration.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = ms.parse::<u64>() {
                obs::req("SetCurrentSceneTransitionDuration", json!({"transitionDuration": v.clamp(50, 20000)}));
            }
        }
        Msg::Title(t) => a.title_edit = t,
        Msg::TitleDone => {
            let t = a.title_edit.clone();
            a.save(|m| {
                m.insert("title".into(), Value::String(t.clone()));
            });
            match a.title_source() {
                Some(src) => obs::req("SetInputSettings", json!({"inputName": src, "inputSettings": {"text": t}})),
                None => a.said = "Your title is kept. Add a text source called \"Overlay: title\" to show it on the stream.".into(),
            }
        }
        Msg::Quality(w, h) => {
            if a.stream.active || a.record.active {
                a.said = "The quality changes once you've stopped streaming and recording.".into();
            } else {
                let fps = if h == 1080 { 60 } else { a.output.2.max(30) };
                obs::req("SetVideoSettings", json!({"outputWidth": w, "outputHeight": h, "fpsNumerator": fps, "fpsDenominator": 1}));
                obs::req("GetVideoSettings", json!({}));
            }
        }
        Msg::Vertical(on) => a.save(|m| {
            m.insert("vertical".into(), Value::Bool(on));
        }),
        Msg::Stream => obs::req(if a.stream.active { "StopStream" } else { "StartStream" }, json!({})),
        Msg::Record => obs::req(if a.record.active { "StopRecord" } else { "StartRecord" }, json!({})),
        Msg::Pause => {
            if a.record.active {
                obs::req(if a.record.paused { "ResumeRecord" } else { "PauseRecord" }, json!({}));
            }
        }
        Msg::Replay => {
            if a.replay {
                obs::req("SaveReplayBuffer", json!({}));
            } else {
                obs::req("StartReplayBuffer", json!({}));
                a.said = "Keeping the last 30 s from now on: press it again to save them.".into();
            }
        }
        Msg::VirtualCam => obs::req("ToggleVirtualCam", json!({})),
        Msg::StudioMode => obs::req("SetStudioModeEnabled", json!({"studioModeEnabled": !a.studio_mode})),
        Msg::Open(d) => {
            a.new_name.clear();
            a.key_edit.clear();
            a.dlg = Some(d);
        }
        Msg::Close => a.dlg = None,
        Msg::NewName(sn) => a.new_name = sn,
        Msg::CreateScene => {
            let nm = a.new_name.trim().to_string();
            if !nm.is_empty() {
                obs::req("CreateScene", json!({"sceneName": nm}));
            }
            a.dlg = None;
        }
        Msg::AddSource(kind, label) => {
            let name = if a.new_name.trim().is_empty() { label.to_string() } else { a.new_name.trim().to_string() };
            obs::req("CreateInput", json!({"sceneName": a.scene, "inputName": name, "inputKind": kind, "inputSettings": {}, "sceneItemEnabled": true}));
            a.dlg = None;
        }
        Msg::Filter(name, on) => obs::req("SetSourceFilterEnabled", json!({"sourceName": a.mic, "filterName": name, "filterEnabled": on})),
        Msg::AddFilter(kind, name) => obs::req("CreateSourceFilter", json!({"sourceName": a.mic, "filterName": name, "filterKind": kind, "filterSettings": {}})),
        Msg::Service(sv) => a.service_pick = sv,
        Msg::Key(k) => a.key_edit = k,
        Msg::SaveService => {
            let (service, server) = match a.service_pick {
                "YouTube" => ("YouTube - RTMPS", "rtmps://a.rtmps.youtube.com:443/live2"),
                "Kick" => ("Kick", "auto"),
                _ => ("Twitch", "auto"),
            };
            let mut st = json!({"service": service, "server": server});
            if !a.key_edit.trim().is_empty() {
                st["key"] = Value::String(a.key_edit.trim().to_string());
            }
            obs::req("SetStreamServiceSettings", json!({"streamServiceType": "rtmp_common", "streamServiceSettings": st}));
            obs::req("GetStreamServiceSettings", json!({}));
            a.key_edit.clear();
            a.dlg = None;
        }
        Msg::Password(pw) => a.pw_edit = pw,
        Msg::SavePassword => {
            let pw = a.pw_edit.clone();
            obs::set_password(&pw);
            a.save(|m| {
                m.insert("password".into(), Value::String(pw.clone()));
            });
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(prefs_path(), std::fs::Permissions::from_mode(0o600));
            }
            a.pw_edit.clear();
            a.dlg = None;
        }
        Msg::Install => {
            a.busy = "Installing OBS Studio from Flathub (for you only)…".into();
            return Task::perform(run::spawn(obs::install_cmd()), Msg::Installed);
        }
        Msg::Installed(o) => {
            a.busy.clear();
            a.said = if o.ok { "OBS Studio is installed.".into() } else { format!("It couldn't be installed: {}", o.why()) };
        }
        Msg::StartObs => {
            if let Err(e) = obs::start() {
                a.said = format!("OBS couldn't be started: {e}");
            } else {
                a.said = "Starting OBS…".into();
            }
            a.dlg = None;
        }
        Msg::Cropped(o) => {
            a.busy.clear();
            a.said = if o.ok { "The 9:16 copy for phones is made, beside the recording.".into() } else if o.code == -1 { "The 9:16 copy needs ffmpeg (Hearth Store › ffmpeg).".into() } else { format!("The 9:16 copy couldn't be made: {}", o.why()) };
        }
        Msg::Pad(b) => {
            // ≡ + Y: the next scene (≡ pressed, then Y within a moment)
            match b {
                pad::Button::Menu => a.menu_at = Some(std::time::Instant::now()),
                pad::Button::Y if a.menu_at.map(|t| t.elapsed().as_millis() < 1500).unwrap_or(false) => {
                    a.menu_at = None;
                    if let Some(i) = a.scenes.iter().position(|x| *x == a.scene) {
                        let next = a.scenes[(i + 1) % a.scenes.len()].clone();
                        return update(a, Msg::Scene(next));
                    }
                }
                _ => {}
            }
        }
        Msg::KeyEv(ev) => {
            if let KEvent::KeyPressed { key, modifiers, .. } = ev {
                if let Some(dm) = hearth_ui::arrange::arrange_key(&a.describe, &key, modifiers) {
                    return update(a, Msg::Describe(dm));
                }
                if modifiers.control() {
                    if let Key::Character(c) = key.as_ref() {
                        if modifiers.shift() && c.eq_ignore_ascii_case("r") {
                            return update(a, Msg::Replay);
                        }
                        if let Some(d) = c.chars().next().and_then(|ch| ch.to_digit(10)) {
                            if (1..=6).contains(&d) {
                                if let Some(sn) = a.scenes.get(d as usize - 1).cloned() {
                                    return update(a, Msg::Scene(sn));
                                }
                            }
                        }
                    }
                }
                if let Key::Named(iced::keyboard::key::Named::Escape) = key.as_ref() {
                    a.dlg = None;
                }
            }
        }
        Msg::Focus(f) => {
            if a.focused != f && a.up {
                obs::meters(f);
                if !f {
                    a.audio.iter_mut().for_each(|x| x.level = 0.0);
                }
            }
            a.focused = f;
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Quit => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn window_events(ev: iced::Event, _s: iced::event::Status, _id: iced::window::Id) -> Option<Msg> {
    match ev {
        iced::Event::Window(iced::window::Event::Focused) => Some(Msg::Focus(true)),
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Msg::Focus(false)),
        _ => None,
    }
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut subs = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        Subscription::run(obs::stream).map(Msg::Obs),
        iced::keyboard::listen().map(Msg::KeyEv),
        iced::event::listen_with(window_events),
        Subscription::run(pad::presses).map(Msg::Pad),
    ];
    if a.up {
        subs.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Every));
    }
    Subscription::batch(subs)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view::view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-studio-app", "Hearth Studio: going live and recording with OBS Studio (obs-websocket).\n  --show filters|add|where") {
        return Ok(());
    }
    iced::application(App::new, update, shown)
        .title(|_: &App| "Hearth Studio".to_string())
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-studio", (1600.0, 900.0), (1100.0, 680.0)))
        .default_font(app::INTER)
        .run()
}
