//! hearth-forge-app: Hearth Forge (docs/mockups/checklists/hearth-forge.md; the approved boards
//! docs/mockups/hearth-forge/01-make.png and 02-models.png). Real 3D models from a photo or a few words, made on the
//! owner's Nvidia PC (RTX 5070, 12 GB) by the open models (Hunyuan3D, TRELLIS, TripoSR and the others, each under its
//! own licence), or online by Tripo3D with his own key. Private: only on his own devices (the Store's private list,
//! never its public one).
//!
//! The making happens in Hearth Forge's own service on the Nvidia PC (helper/hearth-forge.py, built in, run as you by
//! hearth-forge.service only once you set it up; 127.0.0.1 and the PC's Tailscale address only, port 8191). Every model
//! made is kept in ~/Hearth/Forge with its photo, words and settings. Your choices: ~/.config/nimbus/forge.json (only
//! what you change, through settings_file). Keys (Tripo3D, Hugging Face): the system's password store, never a file.
//!
//!   --page make|models|made|waiting|settings      which page opens          $HEARTH_FORGE_DIR: the models made

mod forge;
mod models;
mod view3d;

use forge::{Catalog, Model};
use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::canvas::Cache;
use iced::widget::{canvas, column, container, image, mouse_area, row, scrollable, text_editor, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};
use std::collections::HashMap;
use view3d::{Cam, Look, Mesh};

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Status(Result<Value, String>),
    Made(Vec<Value>),
    Image(String, Option<Vec<u8>>),
    Preview(String, Option<Mesh>),
    Page(&'static str),
    Mode(&'static str),
    PickPhoto,
    Picked(Option<String>),
    Dropped(std::path::PathBuf),
    Paste,
    TakeOne,
    Photo(Option<(String, Vec<u8>)>),
    Words(text_editor::Action),
    Menu(&'static str),
    PickModel(String),
    Set(&'static str, Value),
    RealSize(String),
    NewSeed,
    Make,
    MakeAgain,
    ChangeWords(String),
    ChangeGo,
    Queued(Result<Value, String>),
    Open(String),
    Stop(String),
    Cam(Cam),
    Turn,
    Spin,
    Send(&'static str),
    SaveAs(&'static str),
    Sent(Result<String, String>),
    OpenFolder,
    Get(String),
    Remove(String),
    GetBest,
    Search(String),
    Show(&'static str),
    Sort(&'static str),
    TripoUse,
    TripoAsk,
    TripoGo,
    TripoNo,
    TripoBalance(Option<u64>),
    TripoDone(run::Output),
    KeyEdit(&'static str, String),
    KeySave(&'static str),
    KeySaved(&'static str, run::Output),
    Server(String),
    ServerDone,
    SetUp,
    TurnOff,
    Local(Value),
    Ran(run::Output),
    Folder(&'static str, String),
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub prefs: Prefs,
    pub cat: Catalog,
    pub base: String,
    pub server_edit: Option<String>,
    pub up: Option<bool>,
    pub status: Value,
    pub local: Value,
    pub made: Vec<Value>,
    pub page: &'static str,
    pub open: String,
    pub mesh: Option<(String, Mesh)>,
    pub cam: Cam,
    pub cache: Cache,
    pub imgs: HashMap<String, iced::widget::image::Handle>,
    pub asked: std::collections::HashSet<String>,
    pub photos: Vec<(String, Vec<u8>)>,
    pub words: text_editor::Content,
    pub menu: Option<&'static str>,
    pub said: String,
    pub change: Option<String>,
    pub real_edit: Option<String>,
    pub tripo: Option<u64>,
    pub tripo_confirm: bool,
    pub tripo_busy: bool,
    pub keys: HashMap<&'static str, String>,
    pub search: String,
    pub turning: bool,
    pub exported: Option<std::path::PathBuf>,
}

fn read(p: &std::path::Path) -> Value {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = Prefs::load("forge");
        let draft = read(&forge::data().join("draft.json"));
        let mut photos = Vec::new();
        if let Some(name) = draft.get("photo").and_then(Value::as_str) {
            if let Ok(b) = std::fs::read(forge::data().join("draft-photo")) {
                photos.push((name.to_string(), b));
            }
        }
        let page = match app::arg("--page").as_deref() {
            Some("models") => "models",
            Some("made") => "made",
            Some("waiting") => "waiting",
            Some("settings") => "settings",
            _ => "make",
        };
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("forge", "Hearth Forge").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            base: prefs.s("server", &format!("http://127.0.0.1:{}", forge::PORT)).trim_end_matches('/').to_string(),
            prefs,
            cat: forge::catalog(),
            server_edit: None,
            up: None,
            status: Value::Null,
            local: Value::Null,
            made: Vec::new(),
            page,
            open: draft.get("open").and_then(Value::as_str).unwrap_or("").to_string(),
            mesh: None,
            cam: Cam::default(),
            cache: Cache::new(),
            imgs: HashMap::new(),
            asked: Default::default(),
            photos,
            words: text_editor::Content::with_text(draft.get("words").and_then(Value::as_str).unwrap_or("")),
            menu: None,
            said: String::new(),
            change: None,
            real_edit: None,
            tripo: None,
            tripo_confirm: false,
            tripo_busy: false,
            keys: HashMap::new(),
            search: String::new(),
            turning: false,
            exported: None,
        };
        a.made = forge::local_made();
        let _ = forge::helper();
        menu_entry();
        let t = Task::batch([ping(&a.base), local_status(), a.balance()]);
        (a, t)
    }

    pub fn pref(&self, k: &str, d: &str) -> String {
        self.prefs.s(k, d)
    }
    fn set(&mut self, k: &str, v: Value) {
        if let Err(e) = self.prefs.set(k, v) {
            self.said = e;
        }
    }
    pub fn model(&self) -> &Model {
        let id = self.prefs.s("model", "");
        self.cat.models.iter().find(|m| m.id == id).or_else(|| self.cat.models.iter().find(|m| m.default)).or(self.cat.models.first()).expect("models.json has models")
    }
    pub fn mstate(&self, id: &str) -> Value {
        self.status.pointer(&format!("/models/{id}")).cloned().unwrap_or(Value::Null)
    }
    pub fn installed(&self, id: &str) -> bool {
        self.mstate(id).get("installed").and_then(Value::as_bool).unwrap_or(false)
    }
    pub fn vram_gb(&self) -> f64 {
        let v = self.status.pointer("/gpu/vramTotal").and_then(Value::as_f64).unwrap_or(0.0);
        if v > 0.0 {
            (v / 1073741824.0).round()
        } else {
            12.0
        }
    }
    pub fn card(&self) -> String {
        let n = self.status.pointer("/gpu/name").and_then(Value::as_str).unwrap_or("");
        n.replace("NVIDIA ", "").replace("GeForce ", "").trim().to_string()
    }
    fn seed(&self) -> u64 {
        self.prefs.f("seed", 71204.0) as u64
    }
    fn busy(&self) -> bool {
        self.status.get("running").map(|r| !r.is_null()).unwrap_or(false)
            || self.status.get("queue").and_then(Value::as_array).map(|q| !q.is_empty()).unwrap_or(false)
            || self.status.get("models").and_then(Value::as_object).map(|o| o.values().any(|m| m.pointer("/getting/busy").and_then(Value::as_bool).unwrap_or(false))).unwrap_or(false)
            || self.tripo_busy
    }
    pub fn item(&self, id: &str) -> Option<&Value> {
        self.made.iter().find(|m| m.get("id").and_then(Value::as_str) == Some(id))
    }
    /// The one on show: the one you opened, else the newest made.
    pub fn shown(&self) -> Option<&Value> {
        self.item(&self.open).or_else(|| self.made.iter().find(|m| m.pointer("/done/ok").and_then(Value::as_bool) == Some(true)))
    }
    fn tripo_cost(&self) -> u64 {
        let c = &self.cat.tripo_costs;
        let tex = self.pref("textures", "pbr");
        let base = c.get(match tex.as_str() { "none" => "shape", "colour" => "colour", _ => "pbr" }).and_then(Value::as_u64).unwrap_or(30);
        base + if self.pref("detail", "high") == "highest" { c.get("highDetail").and_then(Value::as_u64).unwrap_or(10) } else { 0 }
    }
    fn balance(&self) -> Task<Msg> {
        Task::perform(run::spawn(forge::helper_argv(&["tripo", "balance"])), |o| Msg::TripoBalance(o.json().and_then(|j| j.get("balance").and_then(Value::as_u64))))
    }
    fn save_draft(&self) {
        let mut d = json!({"words": self.words.text().trim_end(), "open": self.open});
        if let Some((name, b)) = self.photos.first() {
            d["photo"] = json!(name);
            let _ = std::fs::create_dir_all(forge::data());
            let tmp = forge::data().join(".draft-photo.tmp");
            if std::fs::write(&tmp, b).is_ok() {
                let _ = std::fs::rename(&tmp, forge::data().join("draft-photo"));
            }
        }
        let _ = hearth_ui::settings_file::write_json(&forge::data().join("draft.json"), &d);
    }
    /// What's asked for now: the photos, the words and every choice, as the service keeps them with the model.
    fn ask(&self, seed: u64) -> Value {
        let words = self.words.text().trim().to_string();
        let title = words.split([',', '.']).next().unwrap_or("").trim().to_string();
        let title = if title.is_empty() { self.photos.first().map(|p| p.0.rsplit_once('.').map(|x| x.0).unwrap_or(p.0.as_str()).replace(['-', '_'], " ")).unwrap_or_else(|| "A model".into()) } else { title };
        json!({"title": ui::fit(&title, 60), "mode": self.pref("mode", "photo"), "words": words, "model": self.model().id, "detail": self.pref("detail", "high"),
               "textures": self.pref("textures", "pbr"), "textureSize": self.pref("texsize", "2k"), "realSize": self.pref("realSize", "30 cm tall"), "seed": seed,
               "photoName": self.photos.first().map(|p| p.0.clone()).unwrap_or_default()})
    }
    fn fetch_for(&mut self, id: &str) -> Task<Msg> {
        let Some(it) = self.item(id).cloned() else { return Task::none() };
        let files: Vec<String> = it.get("files").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        let mut t = Vec::new();
        for f in ["photo-cut.png", "photo.png", "map-colour.png", "map-roughness.png", "map-metal.png", "map-bumps.png"] {
            let key = format!("{id}/{f}");
            if files.iter().any(|x| x == f) && self.asked.insert(key.clone()) {
                let (b, id2, f2) = (self.base.clone(), id.to_string(), f.to_string());
                t.push(Task::perform(forge::bg(move || forge::made_file(&b, &id2, &f2)), move |r| Msg::Image(key.clone(), r.flatten())));
            }
        }
        if files.iter().any(|x| x == "preview.json") && self.mesh.as_ref().map(|m| m.0 != id).unwrap_or(true) && self.asked.insert(format!("{id}/preview")) {
            let (b, id2) = (self.base.clone(), id.to_string());
            t.push(Task::perform(
                forge::bg(move || forge::made_file(&b, &id2, "preview.json").and_then(|x| serde_json::from_slice::<Value>(&x).ok()).and_then(|j| Mesh::from_preview(&j))),
                {
                    let id3 = id.to_string();
                    move |m| Msg::Preview(id3.clone(), m.flatten())
                },
            ));
        }
        Task::batch(t)
    }
    pub fn img(&self, id: &str, f: &str) -> Option<&iced::widget::image::Handle> {
        self.imgs.get(&format!("{id}/{f}"))
    }
    pub fn thumb(&self, id: &str) -> Option<&iced::widget::image::Handle> {
        self.img(id, "photo-cut.png").or_else(|| self.img(id, "photo.png"))
    }
}

/// Private (hidden in everyone's menu): the first time it opens on a device, it puts its own entry in this person's
/// menu, so the owner finds it there after opening it from the Store's private list. Written once, never changed.
fn menu_entry() {
    let Some(dir) = forge::data().parent().and_then(|p| p.parent()).map(|p| p.join("applications")) else { return };
    let f = dir.join("hearth-forge.desktop");
    if f.exists() {
        return;
    }
    let exe = if std::path::Path::new("/usr/bin/hearth-forge-app").is_file() { "/usr/bin/hearth-forge-app".to_string() } else { std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "hearth-forge-app".into()) };
    let text = format!("[Desktop Entry]\nType=Application\nName=Hearth Forge\nGenericName=3D models from a photo or words\nComment=Real 3D models from a photo or a few words, on your own Nvidia PC\nExec={exe}\nIcon=hearth-forge\nCategories=Graphics;3DGraphics;\nStartupWMClass=hearth-forge\n");
    if std::fs::create_dir_all(&dir).is_ok() {
        let tmp = dir.join(".hearth-forge.desktop.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &f);
        }
    }
}

fn ping(base: &str) -> Task<Msg> {
    let b = base.to_string();
    Task::perform(forge::bg(move || forge::get_json(&b, "/status")), |r| Msg::Status(r.unwrap_or_else(|| Err("it stopped".into()))))
}
fn load_made(base: &str) -> Task<Msg> {
    let b = base.to_string();
    Task::perform(
        forge::bg(move || {
            let mut all = forge::get_json(&b, "/made").ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
            for l in forge::local_made() {
                if !all.iter().any(|x| x.get("id") == l.get("id")) {
                    all.push(l);
                }
            }
            all.sort_by_key(|x| -x.get("at").and_then(Value::as_i64).unwrap_or(0));
            all
        }),
        |v| Msg::Made(v.unwrap_or_default()),
    )
}
fn local_status() -> Task<Msg> {
    Task::perform(run::spawn(forge::helper_argv(&["status"])), |o| Msg::Local(o.json().unwrap_or(Value::Null)))
}

fn photo_from(path: String) -> Task<Msg> {
    Task::perform(
        forge::bg(move || {
            let b = std::fs::read(&path).ok()?;
            let name = std::path::Path::new(&path).file_name()?.to_string_lossy().to_string();
            Some((name, b))
        }),
        |r| Msg::Photo(r.flatten()),
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
        Msg::Tick => return Task::batch([ping(&a.base), load_made(&a.base)]),
        Msg::Spin => {
            a.cam.yaw += 0.02;
            a.cache.clear();
        }
        Msg::Status(r) => match r {
            Ok(j) => {
                let was = a.up;
                a.up = Some(true);
                a.status = j;
                if was != Some(true) {
                    return load_made(&a.base);
                }
            }
            Err(_) => {
                a.up = Some(false);
                return load_made(&a.base);
            }
        },
        Msg::Local(v) => a.local = v,
        Msg::Made(v) => {
            a.made = v;
            let mut t = Vec::new();
            let ids: Vec<String> = a.made.iter().take(8).filter_map(|x| x.get("id").and_then(Value::as_str).map(str::to_string)).collect();
            for id in ids {
                t.push(a.fetch_for(&id));
            }
            if let Some(id) = a.shown().and_then(|x| x.get("id")).and_then(Value::as_str).map(str::to_string) {
                t.push(a.fetch_for(&id));
            }
            return Task::batch(t);
        }
        Msg::Image(k, b) => {
            if let Some(b) = b {
                a.imgs.insert(k, iced::widget::image::Handle::from_bytes(b));
            }
        }
        Msg::Preview(id, m) => {
            if let Some(m) = m {
                a.mesh = Some((id, m));
                a.cache.clear();
            }
        }
        Msg::Page(p) => {
            a.page = p;
            a.menu = None;
        }
        Msg::Mode(md) => a.set("mode", json!(md)),
        Msg::PickPhoto => return Task::perform(hearth_oss_kit_a::pick_file("A photo of it", &["*.png", "*.jpg", "*.jpeg", "*.webp"]), Msg::Picked),
        Msg::Picked(p) => {
            if let Some(p) = p {
                return photo_from(p);
            }
        }
        Msg::Dropped(p) => {
            let low = p.to_string_lossy().to_lowercase();
            if [".png", ".jpg", ".jpeg", ".webp"].iter().any(|x| low.ends_with(x)) {
                return photo_from(p.to_string_lossy().to_string());
            }
        }
        Msg::Paste => {
            return Task::perform(run::spawn(run::args(&["sh", "-c", "wl-paste --list-types 2>/dev/null | grep -q '^image/png' && wl-paste --type image/png | base64 -w0"])), |o| {
                Msg::Photo(if o.ok && !o.out.trim().is_empty() { Some(("pasted.png".to_string(), hearth_oss::b64::decode(o.out.trim()))) } else { None })
            })
        }
        Msg::TakeOne => {
            let out = forge::data().join("taken.jpg");
            let o = out.to_string_lossy().to_string();
            a.said = "Taking a photo with the camera…".into();
            return Task::perform(
                run::spawn(vec!["sh".into(), "-c".into(), format!("ls /dev/video* >/dev/null 2>&1 || exit 3; ffmpeg -loglevel error -y -f v4l2 -i /dev/video0 -frames:v 1 '{}'", o.replace('\'', ""))]),
                move |r| {
                    if r.ok {
                        Msg::Photo(std::fs::read(&out).ok().map(|b| ("taken.jpg".to_string(), b)))
                    } else {
                        Msg::Sent(Err(if r.code == 3 { "There's no camera on this device.".into() } else { format!("The camera didn't take one: {}", r.why()) }))
                    }
                },
            );
        }
        Msg::Photo(p) => match p {
            Some((name, b)) => {
                if a.pref("mode", "photo") == "photos" {
                    a.photos.push((name, b));
                    a.photos.truncate(6);
                } else {
                    a.photos = vec![(name, b)];
                }
                a.said.clear();
                a.save_draft();
            }
            None => a.said = "There was no picture to paste.".into(),
        },
        Msg::Words(act) => {
            let edit = act.is_edit();
            a.words.perform(act);
            if edit {
                a.save_draft();
            }
        }
        Msg::Menu(m) => a.menu = if a.menu == Some(m) { None } else { Some(m) },
        Msg::PickModel(id) => {
            a.menu = None;
            a.set("model", json!(id));
        }
        Msg::Set(k, v) => {
            a.menu = None;
            a.set(k, v);
            if k == "view" || k == "light" || k == "floor" {
                a.cache.clear();
            }
        }
        Msg::RealSize(s) => a.real_edit = Some(s),
        Msg::NewSeed => {
            let s = (hearth_oss::now() as u64).wrapping_mul(6364136223846793005).wrapping_add(std::process::id() as u64) % 99_999 + 1;
            a.set("seed", json!(s));
        }
        Msg::Make => return make(a, None, a.seed()),
        Msg::MakeAgain => {
            let s = a.seed() + 1;
            a.set("seed", json!(s));
            return make(a, None, s);
        }
        Msg::ChangeWords(s) => a.change = Some(s),
        Msg::ChangeGo => {
            if let Some(c) = a.change.take() {
                if !c.trim().is_empty() {
                    return make(a, Some(c), a.seed());
                }
            }
        }
        Msg::Queued(r) => {
            match r {
                Ok(j) => {
                    a.said = "It's in the queue.".into();
                    if let Some(id) = j.get("id").and_then(Value::as_str) {
                        a.open = id.to_string();
                        a.save_draft();
                    }
                }
                Err(e) => a.said = format!("It couldn't be started: {e}"),
            }
            return Task::batch([ping(&a.base), load_made(&a.base)]);
        }
        Msg::Open(id) => {
            a.open = id.clone();
            a.page = "make";
            a.save_draft();
            a.cache.clear();
            return a.fetch_for(&id);
        }
        Msg::Stop(id) => {
            let b = a.base.clone();
            return Task::perform(forge::bg(move || forge::post_json(&b, &format!("/made/{id}/stop"), &json!({})).map(|_| "Stopped.".to_string())), |r| Msg::Sent(r.unwrap_or_else(|| Err("it stopped".into()))));
        }
        Msg::Cam(c) => {
            a.cam = c;
            a.cache.clear();
        }
        Msg::Turn => a.turning = !a.turning,
        Msg::Send(target) => return send(a, target, None),
        Msg::SaveAs(fmt) => {
            a.menu = None;
            return send(a, "save", Some(fmt));
        }
        Msg::Sent(r) => {
            a.said = match r {
                Ok(s) | Err(s) => s,
            }
        }
        Msg::OpenFolder => {
            if let Some(d) = a.exported.clone() {
                run::detached(&["xdg-open", &d.to_string_lossy()]);
            }
        }
        Msg::Get(id) => {
            let b = a.base.clone();
            a.said = format!("Getting {}: only its own files, each checked.", a.cat.models.iter().find(|m| m.id == id).map(|m| m.name.clone()).unwrap_or_default());
            return Task::perform(forge::bg(move || forge::post_json(&b, &format!("/models/{id}/get"), &json!({}))), |r| Msg::Queued(r.unwrap_or_else(|| Err("it stopped".into())).map(|_| json!({}))));
        }
        Msg::Remove(id) => {
            let b = a.base.clone();
            return Task::perform(forge::bg(move || forge::post_json(&b, &format!("/models/{id}/remove"), &json!({}))), |r| Msg::Queued(r.unwrap_or_else(|| Err("it stopped".into())).map(|_| json!({}))));
        }
        Msg::GetBest => {
            let vram = a.vram_gb();
            let mut best: Vec<&Model> = a.cat.models.iter().filter(|m| m.best > 0 && (m.vram_all <= vram || m.fit != "bad")).collect();
            best.sort_by_key(|m| m.best);
            let ids: Vec<String> = best.iter().take(4).filter(|m| !a.installed(&m.id)).map(|m| m.id.clone()).collect();
            if ids.is_empty() {
                a.said = "The best 4 for your PC are already here.".into();
                return Task::none();
            }
            a.said = format!("Getting {}.", best.iter().take(4).filter(|m| ids.contains(&m.id)).map(|m| m.name.clone()).collect::<Vec<_>>().join(", "));
            let b = a.base.clone();
            return Task::perform(
                forge::bg(move || {
                    for id in &ids {
                        forge::post_json(&b, &format!("/models/{id}/get"), &json!({}))?;
                    }
                    Ok(json!({}))
                }),
                |r| Msg::Queued(r.unwrap_or_else(|| Err("it stopped".into()))),
            );
        }
        Msg::Search(s) => a.search = s,
        Msg::Show(s) => a.set("show", json!(s)),
        Msg::Sort(s) => {
            a.menu = None;
            a.set("sort", json!(s));
        }
        Msg::TripoUse => {
            let on = !a.prefs.b("tripoOn", false);
            a.set("tripoOn", json!(on));
        }
        Msg::TripoAsk => {
            if a.tripo.is_none() {
                a.said = "Add your Tripo3D key first (Settings); it goes into the system's password store.".into();
                a.page = "settings";
            } else {
                a.tripo_confirm = true;
            }
        }
        Msg::TripoNo => a.tripo_confirm = false,
        Msg::TripoGo => {
            a.tripo_confirm = false;
            return tripo_make(a);
        }
        Msg::TripoBalance(b) => a.tripo = b,
        Msg::TripoDone(o) => {
            a.tripo_busy = false;
            a.said = if o.ok { "Tripo3D made it.".into() } else { o.json().and_then(|j| j.get("error").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| o.why()) };
            return Task::batch([load_made(&a.base), a.balance()]);
        }
        Msg::KeyEdit(k, v) => {
            a.keys.insert(k, v);
        }
        Msg::KeySave(k) => {
            let Some(v) = a.keys.remove(k).filter(|v| !v.trim().is_empty()) else { return Task::none() };
            let label = if k == "tripo3d" { "Hearth Forge: your Tripo3D key" } else { "Hearth Forge: your Hugging Face key" };
            return Task::perform(hearth_oss::secret::store("forge", k, label, v.trim().to_string()), move |o| Msg::KeySaved(k, o));
        }
        Msg::KeySaved(k, o) => {
            a.said = if o.ok { "Kept in the system's password store.".into() } else { format!("It couldn't be kept: {}", o.why()) };
            if k == "tripo3d" && o.ok {
                return a.balance();
            }
        }
        Msg::Server(s) => a.server_edit = Some(s),
        Msg::ServerDone => {
            if let Some(s) = a.server_edit.take() {
                let s = s.trim().trim_end_matches('/').to_string();
                let s = if s.starts_with("http://") { s } else { format!("http://{s}") };
                let s = if s.rsplit(':').next().map(|x| x.parse::<u16>().is_ok()).unwrap_or(false) { s } else { format!("{s}:{}", forge::PORT) };
                a.set("server", json!(s));
                a.base = s;
                a.up = None;
                return ping(&a.base);
            }
        }
        Msg::SetUp => {
            a.said = "Setting up Hearth Forge on this PC (its own service, off until now; nothing is downloaded until you press Get).".into();
            return Task::perform(run::spawn(forge::helper_argv(&["on"])), Msg::Ran);
        }
        Msg::TurnOff => return Task::perform(run::spawn(forge::helper_argv(&["off"])), Msg::Ran),
        Msg::Ran(o) => {
            if !o.ok {
                a.said = format!("That didn't work: {}", o.json().and_then(|j| j.get("error").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| o.why()));
            }
            let b = a.base.clone();
            return Task::batch([
                local_status(),
                Task::perform(
                    forge::bg(move || {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        forge::get_json(&b, "/status")
                    }),
                    |r| Msg::Status(r.unwrap_or_else(|| Err("it stopped".into()))),
                ),
            ]);
        }
        Msg::Folder(k, v) => a.set(k, json!(v)),
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => {
            if let Some(v) = a.real_edit.take() {
                a.set("realSize", json!(v));
            }
            return win::close();
        }
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

/// A new model asked of the service on the Nvidia PC: the photos (or none, for words) and every choice.
fn make(a: &mut App, change: Option<String>, seed: u64) -> Task<Msg> {
    if let Some(v) = a.real_edit.take() {
        a.set("realSize", json!(v));
    }
    if a.up != Some(true) {
        a.said = "Hearth Forge isn't reachable on the Nvidia PC yet (Settings).".into();
        return Task::none();
    }
    let mode = a.pref("mode", "photo");
    let mut ask = a.ask(seed);
    let mut photos: Vec<String> = a.photos.iter().map(|p| hearth_oss::b64::encode(&p.1)).collect();
    if let Some(c) = &change {
        // Change it with words…: the one on show, its photo changed by the words
        let Some(cur) = a.shown().cloned() else { return Task::none() };
        let id = cur.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let mut old = cur.get("ask").cloned().unwrap_or(Value::Null);
        old["change"] = json!(c);
        old["seed"] = json!(seed);
        old["title"] = json!(format!("{}, {}", old.get("title").and_then(Value::as_str).unwrap_or("A model"), ui::fit(c, 30)));
        ask = old;
        photos = forge::made_file(&a.base, &id, "photo.png").map(|b| vec![hearth_oss::b64::encode(&b)]).unwrap_or_default();
    } else if (mode == "photo" || mode == "photos") && photos.is_empty() {
        a.said = "Drop a photo first (or paste one, or take one).".into();
        return Task::none();
    } else if (mode == "words" || mode == "picture") && a.words.text().trim().is_empty() {
        a.said = "Say what to make first.".into();
        return Task::none();
    }
    if mode == "words" || mode == "picture" {
        photos.clear();
    }
    let m = a.model().clone();
    if !a.installed(&m.id) {
        a.said = format!("Get {} first (All 22 models ›).", m.name);
        return Task::none();
    }
    if !m.runner {
        a.said = format!("{} can be got and kept, but Hearth Forge can't make with it yet (its own code isn't in Hearth Forge).", m.name);
        return Task::none();
    }
    ask["photos"] = json!(photos);
    a.said = "Asking the Nvidia PC…".into();
    let b = a.base.clone();
    Task::perform(forge::bg(move || forge::post_json(&b, "/make", &ask)), |r| Msg::Queued(r.unwrap_or_else(|| Err("it stopped".into()))))
}

/// Tripo3D online with your own key: its folder in ~/Hearth/Forge first (the photo, words and settings), then
/// hearth-forge.py asks Tripo3D (it reads the key from the password store itself) and keeps the model there.
fn tripo_make(a: &mut App) -> Task<Msg> {
    let mut ask = a.ask(a.seed());
    ask["model"] = json!("tripo3d");
    ask["at"] = json!(hearth_oss::now());
    let title = ask.get("title").and_then(Value::as_str).unwrap_or("model").to_string();
    let slug: String = title.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
    let base = format!("{}-tripo", if slug.is_empty() { "model".to_string() } else { slug.chars().take(34).collect::<String>() });
    let mut name = base.clone();
    let mut n = 2;
    while forge::forge_dir().join(&name).exists() {
        name = format!("{base}-{n}");
        n += 1;
    }
    let d = forge::forge_dir().join(&name);
    if std::fs::create_dir_all(&d).is_err() {
        a.said = "~/Hearth/Forge can't be written".into();
        return Task::none();
    }
    if let Some((_, b)) = a.photos.first() {
        if a.pref("mode", "photo") != "words" {
            let _ = std::fs::write(d.join("photo.png"), b);
            ask["photos"] = json!(["photo.png"]);
        }
    }
    let _ = hearth_ui::settings_file::write_json(&d.join("ask.json"), &ask);
    a.tripo_busy = true;
    a.open = name;
    a.said = "Tripo3D is making it…".into();
    Task::perform(run::spawn(forge::helper_argv(&["tripo", "make", &d.to_string_lossy()])), Msg::TripoDone)
}

/// Send it to: Hearth 3D (a .blend it opens), Godot (.glb into Gravesleep's assets), O3DE (.fbx into Noclip
/// Protocol's Assets), Hearth CAD (an .stl ready for the Anycubic Chiron), or Save as.
fn send(a: &mut App, target: &'static str, fmt: Option<&'static str>) -> Task<Msg> {
    let Some(it) = a.shown().cloned() else {
        a.said = "Make a model first.".into();
        return Task::none();
    };
    let id = it.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let faces: u64 = match a.pref("game", "laptop").as_str() {
        "phone" => 5000,
        "laptop" => 15000,
        "pc" => 50000,
        _ => 0,
    };
    let tex: u64 = match a.pref("texpx", "1024").as_str() {
        "1024" => 1024,
        "2048" => 2048,
        _ => 0,
    };
    let height = forge::real_size_m(it.pointer("/ask/realSize").and_then(Value::as_str).unwrap_or(""));
    let (solid, flat, hollow) = (a.prefs.b("solid", false), a.prefs.b("flat", false), a.prefs.b("hollow", false));
    let base = a.base.clone();
    let godot = a.pref("godotProject", "");
    let o3de = a.pref("o3deProject", "");
    a.said = "Getting it ready…".into();
    Task::perform(
        forge::bg(move || -> Result<(String, std::path::PathBuf), String> {
            let d = forge::ensure_local(&base, &id)?;
            let ex = d.join("exports");
            let run_export = |fmt: &str, out: &std::path::Path, game: bool, print: bool| -> Result<(), String> {
                let mut args: Vec<String> = vec!["export".into(), d.to_string_lossy().into(), fmt.into(), out.to_string_lossy().into()];
                if game {
                    if faces > 0 {
                        args.extend(["--faces".into(), faces.to_string()]);
                    }
                    if tex > 0 {
                        args.extend(["--texture".into(), tex.to_string()]);
                    }
                }
                if print {
                    if solid {
                        args.push("--solid".into());
                    }
                    if flat {
                        args.push("--flat".into());
                    }
                    if hollow {
                        args.extend(["--hollow".into(), "2".into()]);
                    }
                }
                if height > 0.0 {
                    args.extend(["--height".into(), format!("{height}")]);
                }
                let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                let o = run::blocking(&forge::helper_argv(&refs));
                if o.ok {
                    Ok(())
                } else {
                    Err(o.json().and_then(|j| j.get("error").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| o.why()))
                }
            };
            match target {
                "hearth3d" => {
                    let out = ex.join(format!("{id}.blend"));
                    run_export("blend", &out, true, false)?;
                    run::detached(&["hearth-3d", &out.to_string_lossy()]);
                    Ok((format!("Opened in Hearth 3D ({}).", out.display()), ex))
                }
                "godot" => {
                    let out = ex.join(format!("{id}.glb"));
                    run_export("glb", &out, true, false)?;
                    let proj = if godot.is_empty() { forge::find_project("project.godot", "gravesleep") } else { Some(std::path::PathBuf::from(&godot)) };
                    match proj {
                        Some(p) => {
                            let to = forge::copy_into(&out, &p.join("assets/forge"), &format!("{id}.glb"))?;
                            Ok((format!("In Gravesleep: {} (Godot imports it when you look).", to.display()), ex))
                        }
                        None => Ok((format!("Saved {} (Gravesleep's Godot project wasn't found: set it in Settings).", out.display()), ex)),
                    }
                }
                "o3de" => {
                    let out = ex.join(format!("{id}.fbx"));
                    run_export("fbx", &out, true, false)?;
                    let proj = if o3de.is_empty() { forge::find_project("project.json", "noclip") } else { Some(std::path::PathBuf::from(&o3de)) };
                    match proj {
                        Some(p) => {
                            let to = forge::copy_into(&out, &p.join("Assets/Forge"), &format!("{id}.fbx"))?;
                            Ok((format!("In Noclip Protocol: {} (O3DE's Asset Processor picks it up).", to.display()), ex))
                        }
                        None => Ok((format!("Saved {} (Noclip Protocol's O3DE project wasn't found: set it in Settings).", out.display()), ex)),
                    }
                }
                "cad" => {
                    let out = ex.join(format!("{id}.stl"));
                    run_export("stl", &out, false, true)?;
                    if hearth_oss::which(&["hearth-cad"], &[]).is_some() {
                        run::detached(&["hearth-cad", &out.to_string_lossy()]);
                        Ok((format!("Opened in Hearth CAD for the Anycubic Chiron ({}).", out.display()), ex))
                    } else {
                        Ok((format!("Saved for the Anycubic Chiron: {} (Hearth CAD isn't on this device).", out.display()), ex))
                    }
                }
                _ => {
                    let f = fmt.unwrap_or("glb");
                    let out = ex.join(format!("{id}.{f}"));
                    run_export(f, &out, f != "stl", f == "stl")?;
                    Ok((format!("Saved: {}", out.display()), ex))
                }
            }
        }),
        |r| match r.unwrap_or_else(|| Err("it stopped".into())) {
            Ok((s, dir)) => Msg::Sent(Ok(format!("{s}\u{1}{}", dir.display()))),
            Err(e) => Msg::Sent(Err(e)),
        },
    )
}

// ---------------------------------------------------------------- the window: board 1, Make a model

pub fn cap<'a>(p: &Pal, s: &str) -> Element<'a, Msg> {
    w::cap(p, s)
}

fn segp<'a>(a: &'a App, k: &'static str, d: &str, items: &[(&'static str, &str)]) -> Element<'a, Msg> {
    let cur = a.pref(k, d);
    w::seg(&a.p, items.iter().map(|(v, l)| (l.to_string(), cur == *v, Some(Msg::Set(k, json!(v))))).collect())
}
fn togp<'a>(a: &'a App, k: &'static str, l: &str) -> Element<'a, Msg> {
    let on = a.prefs.b(k, false);
    w::chip(&a.p, l.to_string(), on, Sz::Sm, Some(Msg::Set(k, json!(!on))))
}
fn target<'a>(p: &Pal, label: &str, m: Msg) -> Element<'a, Msg> {
    mouse_area(container(row![t(label.to_string(), 14.0, p.text), Space::new().width(Length::Fill), t("›", 14.0, p.muted)].align_y(iced::Center)).padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 }).center_y(Length::Fixed(36.0)).width(Length::Fill).style(ui::boxed(p.panel2, p.r(10.0), Some((p.line, 1.0)))))
        .on_press(m)
        .interaction(mouse::Interaction::Pointer)
        .into()
}

fn make_side(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mode = a.pref("mode", "photo");
    let modes = [("photo", "Photo"), ("words", "Words"), ("photos", "Several photos"), ("picture", "Words → picture")];
    let mut head = column![w::h2(&p, "Make a model"), ui::gap(1.0, 12.0), w::seg(&p, modes.iter().map(|(k, l)| (l.to_string(), mode == *k, Some(Msg::Mode(k)))).collect())];
    if a.up == Some(false) {
        head = head.push(ui::gap(1.0, 10.0)).push(setup_card(a));
    }
    // PHOTO: the drop area, the photo, its name
    let photo: Element<'_, Msg> = if mode == "words" || mode == "picture" {
        let words = if mode == "words" { "Your words make it: the model takes words itself, or Hearth makes a picture from them first." } else { "Hearth makes a picture from your words first (Hearth Imagine's ComfyUI on the Nvidia PC), then the model from it." };
        column![cap(&p, "Words"), container(t(words, 12.0, p.muted)).padding(14.0).width(Length::Fill).style(ui::boxed(Color::from_rgb8(0x1a, 0x16, 0x14), p.r(12.0), Some((Color::from_rgb8(0x6a, 0x5a, 0x4e), 1.5))))].into()
    } else {
        let thumb: Element<'_, Msg> = match a.photos.first() {
            Some((_, b)) => image(iced::widget::image::Handle::from_bytes(b.clone())).content_fit(iced::ContentFit::Cover).width(Length::Fixed(110.0)).height(Length::Fixed(110.0)).into(),
            None => container(ui::glyph(G::Camera, p.muted, 28.0, 1.6)).center_x(Length::Fixed(110.0)).center_y(Length::Fixed(110.0)).into(),
        };
        let thumb = container(thumb).clip(true).style(ui::boxed(Color::from_rgb8(0x2e, 0x24, 0x1a), p.r(10.0), None));
        let name = match a.photos.len() {
            0 => "No photo yet".to_string(),
            1 => a.photos[0].0.clone(),
            n => format!("{} and {} more", a.photos[0].0, n - 1),
        };
        let take = mouse_area(t("Take one", 12.0, p.muted)).on_press(Msg::TakeOne).interaction(mouse::Interaction::Pointer);
        let words = column![
            tb(name, 12.0, p.text),
            ui::gap(1.0, 4.0),
            t("Background taken out by itself", 12.0, p.muted),
            row![mouse_area(t("Drop a photo · or paste · or ", 12.0, p.muted)).on_press(Msg::PickPhoto).interaction(mouse::Interaction::Pointer), take],
        ];
        let drop = container(row![thumb, words].spacing(14.0).align_y(iced::Center))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(150.0))
            .style(ui::boxed(Color::from_rgb8(0x1a, 0x16, 0x14), p.r(12.0), Some((Color::from_rgb8(0x6a, 0x5a, 0x4e), 1.5))));
        column![cap(&p, if mode == "photos" { "Photos" } else { "Photo" }), mouse_area(drop).on_press(Msg::PickPhoto).interaction(mouse::Interaction::Pointer)].into()
    };
    let more = column![
        cap(&p, if mode == "words" || mode == "picture" { "What to make" } else { "Tell it more (optional)" }),
        container(text_editor(&a.words).on_action(Msg::Words).height(Length::Fixed(58.0)).size(12.0).padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 })).style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0)))),
    ];
    let m = a.model();
    let pick = if m.pick.is_empty() { format!("{} · {}", m.name, m.makes) } else { m.pick.clone() };
    let mut model = column![cap(&p, "Model"), mouse_area(container(t(format!("{pick} ▾"), 12.0, p.text)).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).center_y(Length::Fixed(30.0)).width(Length::Fill).style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0))))).on_press(Msg::Menu("model")).interaction(mouse::Interaction::Pointer)];
    if a.menu == Some("model") {
        let items: Vec<(String, bool, Msg)> = a.cat.models.iter().filter(|x| a.installed(&x.id) || x.id == m.id).map(|x| (format!("{} · {}", x.name, if x.runner { x.makes.clone() } else { "kept, can't make yet".into() }), x.id == m.id, Msg::PickModel(x.id.clone()))).collect();
        model = model.push(w::menu(&p, items, 0.0));
    }
    let note = if m.fits_note.is_empty() { m.fit_words.clone() } else { m.fits_note.clone() };
    model = model.push(container(row![t(format!("{note} · "), 12.0, p.muted), mouse_area(t(format!("All {} models ›", a.cat.models.len()), 12.0, p.accent)).on_press(Msg::Page("models")).interaction(mouse::Interaction::Pointer)]).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    let real_size = a.real_edit.clone().unwrap_or_else(|| a.pref("realSize", "30 cm tall"));
    let real = column![
        cap(&p, "How real"),
        w::kv(&p, "Detail", segp(a, "detail", "high", &[("draft", "Draft"), ("good", "Good"), ("high", "High"), ("highest", "Highest")])),
        w::kv(&p, "Textures", segp(a, "textures", "pbr", &[("none", "None"), ("colour", "Colour"), ("pbr", "Real materials (PBR)")])),
        w::kv(&p, "Texture size", segp(a, "texsize", "2k", &[("1k", "1K"), ("2k", "2K"), ("4k", "4K")])),
        w::kv(&p, "Real size", w::input(&p, "30 cm tall", &real_size, 12.0, Msg::RealSize).on_submit(Msg::Set("realSize", json!(real_size.clone()))).width(Length::Fixed(78.0)).into()),
        w::kv(&p, "Same result again (seed)", w::field(&p, format!("{} ↻", forge::spaced(a.seed())), 0.0, Some(Msg::NewSeed))),
    ];
    let can = a.up == Some(true);
    let go = column![
        ui::gap(1.0, 14.0),
        w::wide(&p, "✦ Make the model", can.then_some(Msg::Make)),
        container(t("On the Nvidia PC · nothing leaves your network · free", 12.0, p.muted)).width(Length::Fill).align_x(iced::alignment::Horizontal::Center).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    ];
    let credits = match a.tripo {
        Some(b) => format!("{} credits · this one {}", forge::commas(b), a.tripo_cost()),
        None => format!("your key · this one {}", a.tripo_cost()),
    };
    let mut online = column![container(mouse_area(row![t("Or online with Tripo3D (your key)", 12.0, p.text), Space::new().width(Length::Fill), t(credits, 12.0, p.muted)].align_y(iced::Center)).on_press(Msg::TripoAsk).interaction(mouse::Interaction::Pointer)).padding(Padding { top: 12.0, right: 0.0, bottom: 0.0, left: 0.0 })];
    if a.tripo_confirm {
        let left = a.tripo.unwrap_or(0);
        online = online.push(ui::gap(1.0, 8.0)).push(w::card2(
            &p,
            column![
                t(format!("This one costs {} of your {} Tripo3D credits. Make it?", a.tripo_cost(), forge::commas(left)), 12.0, p.text),
                ui::gap(1.0, 8.0),
                row![w::chip(&p, "Make it with Tripo3D", true, Sz::Sm, (left >= a.tripo_cost()).then_some(Msg::TripoGo)), w::chip(&p, "Not now", false, Sz::Sm, Some(Msg::TripoNo))].spacing(8.0),
            ],
        ));
    }
    if !a.said.is_empty() && a.page == "make" {
        online = online.push(container(t(a.said.split('\u{1}').next().unwrap_or("").to_string(), 12.0, p.text)).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    }
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "make",
        vec![("head", head.into()), ("photo", photo), ("more", more.into()), ("model", model.into()), ("real", real.into()), ("go", go.into()), ("online", online.into())],
        |ps| column![ps.take("head"), ps.take("photo"), ps.take("more"), ps.take("model"), ps.take("real"), ps.take("go"), ps.take("online")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(420.0)).height(Length::Fill).padding(Padding { top: 18.0, right: 20.0, bottom: 18.0, left: 20.0 }).style(w::side_bg(&p)).into()
}

fn setup_card(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let l = &a.local;
    let here = l.get("nvidia").and_then(Value::as_bool).unwrap_or(false);
    let mut c = Column::new().spacing(8.0);
    if here && !l.get("on").and_then(Value::as_bool).unwrap_or(false) {
        c = c.push(t("Hearth Forge makes models on this PC's graphics card with its own service, off until you set it up. Nothing is downloaded until you press Get.", 12.0, p.text));
        c = c.push(w::chip(&p, "Set up Hearth Forge on this PC", true, Sz::Sm, Some(Msg::SetUp)));
    } else if here {
        c = c.push(t("Hearth Forge's service is starting on this PC…", 12.0, p.text));
    } else {
        c = c.push(t(format!("Hearth Forge doesn't answer at {}. On this device, type the Nvidia PC's address.", a.base), 12.0, p.text));
    }
    let srv = a.server_edit.clone().unwrap_or_else(|| a.base.clone());
    c = c.push(row![w::input(&p, "http://nvidia-pc:8191", &srv, 12.0, Msg::Server).on_submit(Msg::ServerDone), w::chip(&p, "Use it", false, Sz::Sm, Some(Msg::ServerDone))].spacing(6.0).align_y(iced::Center));
    w::card2(&p, c).into()
}

fn note_box<'a>(p: &Pal, s: &str) -> Element<'a, Msg> {
    container(t(s.to_string(), 13.0, p.text)).padding(Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 }).style(ui::boxed(Pal::mix(p.bg, w::blue(), 0.08), p.r(8.0), Some((Pal::mix(p.line, w::blue(), 0.6), 1.0)))).into()
}

fn view_middle(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let view = a.pref("view", "real");
    let mut looks = vec![("shape", "Shape"), ("clay", "Clay"), ("real", "Real"), ("wire", "Wire"), ("textures", "Textures")];
    let light = a.pref("light", "workshop");
    let mut light_c = Column::new().push(w::chip(&p, format!("Light: {light} ▾"), false, Sz::Sm, Some(Msg::Menu("light"))));
    if a.menu == Some("light") {
        light_c = light_c.push(w::menu(&p, view3d::LIGHTS.iter().map(|l| (l.0.to_string(), l.0 == light, Msg::Set("light", json!(l.0)))).collect(), 140.0));
    }
    let floor = a.prefs.b("floor", true);
    let shown = a.shown().cloned();
    let has = shown.as_ref().map(|s| s.pointer("/done/ok").and_then(Value::as_bool) == Some(true)).unwrap_or(false);
    let mut change_c = Column::new().push(w::chip(&p, "Change it with words…", false, Sz::Sm, has.then_some(Msg::ChangeWords(String::new()))));
    if let Some(c) = &a.change {
        change_c = change_c.push(container(row![w::input(&p, "rustier, a brass handle", c, 12.0, Msg::ChangeWords).on_submit(Msg::ChangeGo).width(Length::Fixed(220.0)), w::chip(&p, "Make", true, Sz::Sm, Some(Msg::ChangeGo))].spacing(6.0).align_y(iced::Center)).padding(6.0).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0)))));
    }
    // a splat model: its own points as a view of their own
    let sid = shown.as_ref().and_then(|s| s.get("id")).and_then(Value::as_str).unwrap_or("");
    if a.mesh.as_ref().map(|m| m.0 == sid && !m.1.s.is_empty()).unwrap_or(false) {
        looks.push(("splat", "Splat"));
    }
    let tools = row![
        w::seg(&p, looks.iter().map(|(k, l)| (l.to_string(), view == *k, Some(Msg::Set("view", json!(k))))).collect()),
        Space::new().width(Length::Fill),
        w::chip(&p, "↻ Turn it", a.turning, Sz::Sm, Some(Msg::Turn)),
        light_c,
        w::chip(&p, "Floor", false, Sz::Sm, Some(Msg::Set("floor", json!(!floor)))),
        w::chip(&p, "Make again", false, Sz::Sm, has.then_some(Msg::MakeAgain)),
        change_c,
    ]
    .spacing(8.0);
    // the viewport: the model, the note, its card, how it was made
    let id = shown.as_ref().and_then(|s| s.get("id")).and_then(Value::as_str).unwrap_or("").to_string();
    let mesh = a.mesh.as_ref().filter(|m| m.0 == id).map(|m| &m.1);
    let li = view3d::LIGHTS.iter().position(|l| l.0 == light).unwrap_or(0);
    let look = match view.as_str() {
        "shape" => Look::Shape,
        "clay" => Look::Clay,
        "wire" => Look::Wire,
        "splat" if mesh.map(|m| !m.s.is_empty()).unwrap_or(false) => Look::Splat,
        _ => Look::Real,
    };
    let mut layers: Vec<Element<'_, Msg>> = Vec::new();
    if view == "textures" {
        let im: Element<'_, Msg> = match a.img(&id, "map-colour.png") {
            Some(h) => image(h.clone()).content_fit(iced::ContentFit::Contain).width(Length::Fill).height(Length::Fill).into(),
            None => container(t("This one has no textures.", 13.0, p.muted)).center(Length::Fill).into(),
        };
        layers.push(container(im).padding(40.0).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color::from_rgb8(0x14, 0x11, 0x10), 0.0, None)).into());
    } else {
        layers.push(canvas(view3d::Viewport { mesh, cam: a.cam, look, light: li, floor, cache: &a.cache, on_cam: Msg::Cam }).width(Length::Fill).height(Length::Fill).into());
    }
    if a.prefs.b("compare", true) {
        if let Some(h) = a.thumb(&id) {
            layers.push(container(image(h.clone()).opacity(0.10).content_fit(iced::ContentFit::Contain).height(Length::Fixed(420.0))).center(Length::Fill).into());
        }
    }
    let mut top = Row::new().push(container(note_box(&p, "Drag to turn it · scroll to zoom · right-drag to move")).align_y(iced::alignment::Vertical::Top));
    top = top.push(Space::new().width(Length::Fill));
    let running = a.status.get("running").filter(|r| !r.is_null()).cloned();
    if let Some(s) = &shown {
        let st = s.get("stats").cloned().unwrap_or(Value::Null);
        let ask = s.get("ask").cloned().unwrap_or(Value::Null);
        let done = s.get("done").cloned().unwrap_or(Value::Null);
        let mut lines: Vec<String> = Vec::new();
        if let (Some(f), Some(pt)) = (st.get("faces").and_then(Value::as_u64), st.get("points").and_then(Value::as_u64)) {
            lines.push(format!("{} faces · {} points", forge::spaced(f), forge::spaced(pt)));
        }
        let maps: Vec<String> = st.get("maps").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        if maps.len() >= 4 {
            lines.push("Real materials: colour, roughness, metal, bumps".into());
        } else if !maps.is_empty() {
            lines.push(format!("Its materials: {}", maps.join(", ")));
        } else if st.is_object() {
            lines.push("No textures (its shape only)".into());
        }
        if st.get("splat").and_then(Value::as_bool) == Some(true) {
            lines.push(if st.get("meshFromSplat").and_then(Value::as_bool) == Some(true) { "A splat (splat.ply) · its mesh made from it".into() } else { "Its splat kept too (splat.ply)".to_string() });
        }
        if let Some(sz) = st.get("size").and_then(Value::as_array) {
            let v: Vec<f64> = sz.iter().filter_map(Value::as_f64).collect();
            if v.len() == 3 && v[1] > 0.0 {
                let h = forge::real_size_m(ask.get("realSize").and_then(Value::as_str).unwrap_or(""));
                let k = if h > 0.0 { h * 100.0 / v[1] } else { 100.0 };
                let closed = st.get("closed").and_then(Value::as_bool).unwrap_or(false);
                lines.push(format!("{:.1} × {:.1} × {:.1} cm · {}", v[1] * k, v[0] * k, v[2] * k, if closed { "closed (prints solid)" } else { "open (Make it solid to print)" }));
            }
        }
        if let Some(secs) = done.get("secs").and_then(Value::as_u64) {
            lines.push(format!("Made in {} · {}", forge::min_s(secs), done.get("model").and_then(Value::as_str).unwrap_or("")));
        }
        let card = container(column![tb(s.get("title").and_then(Value::as_str).unwrap_or("").to_string(), 12.0, p.text), ui::gap(1.0, 6.0), t(lines.join("\n"), 12.0, p.muted).line_height(1.7)]).padding(16.0).width(Length::Fixed(230.0)).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))));
        top = top.push(card);
    }
    let mut bottom = Row::new().align_y(iced::Center);
    let ok = shown.as_ref().and_then(|s| s.pointer("/done/ok")).and_then(Value::as_bool);
    let is_running = running.as_ref().and_then(|r| r.get("id")).and_then(Value::as_str) == Some(id.as_str()) && !id.is_empty();
    if is_running {
        let r = running.clone().unwrap_or(Value::Null);
        bottom = bottom.push(w::tag(&p, "Making", Tg::Warn)).push(container(t(format!("{} · {} %", r.get("stage").and_then(Value::as_str).unwrap_or(""), r.get("progress").and_then(Value::as_f64).unwrap_or(0.0).round()), 12.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 10.0 }));
    } else if ok == Some(true) {
        let tm = shown.as_ref().and_then(|s| s.pointer("/done/timing")).cloned().unwrap_or(Value::Null);
        let mut parts = Vec::new();
        if let Some(x) = tm.get("shape").and_then(Value::as_u64) {
            parts.push(format!("Shape {}", forge::mmss(x)));
        }
        if let Some(x) = tm.get("textures").and_then(Value::as_u64) {
            parts.push(format!("textures {}", forge::mmss(x)));
        }
        if let Some(x) = tm.get("texturesFailed").and_then(Value::as_str) {
            parts.push(x.to_string());
        }
        if let Some(x) = tm.get("note").and_then(Value::as_str) {
            parts.push(x.to_string());
        }
        bottom = bottom.push(w::tag(&p, "Done", Tg::Ok)).push(container(t(parts.join(" · "), 12.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 10.0 }));
    } else if ok == Some(false) {
        let e = shown.as_ref().and_then(|s| s.pointer("/done/error")).and_then(Value::as_str).unwrap_or("").to_string();
        bottom = bottom.push(w::tag(&p, "Stopped", Tg::Bad)).push(container(t(ui::fit(&e, 90), 12.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 10.0 }));
    } else if shown.is_none() {
        bottom = bottom.push(t("Your model turns here as soon as it's made.", 12.0, p.muted));
    }
    bottom = bottom.push(Space::new().width(Length::Fill)).push(t("Compare with your photo", 12.0, p.muted)).push(ui::gap(8.0, 1.0)).push(w::sw(&p, a.prefs.b("compare", true), Some(Msg::Set("compare", json!(!a.prefs.b("compare", true))))));
    layers.push(container(column![top, Space::new().height(Length::Fill), bottom]).padding(Padding { top: 16.0, right: 16.0, bottom: 16.0, left: 20.0 }).width(Length::Fill).height(Length::Fill).into());
    let vp: Element<'_, Msg> = container(iced::widget::Stack::with_children(layers)).clip(true).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color::from_rgb8(0x14, 0x11, 0x10), p.r(12.0), Some((p.line, 1.0)))).into();
    // ITS MATERIALS (PBR)
    let mut maps = Row::new().spacing(8.0);
    for (f, l, g) in [("map-colour.png", "Colour", ("#6b4a2a", "#2a1e14")), ("map-roughness.png", "Roughness", ("#888888", "#333333")), ("map-metal.png", "Metal", ("#222222", "#bbbbbb")), ("map-bumps.png", "Bumps", ("#8080ff", "#6070f0"))] {
        let inner: Element<'_, Msg> = match a.img(&id, f) {
            Some(h) => image(h.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(Length::Fixed(64.0)).into(),
            None => {
                let lin = iced::gradient::Linear::new(iced::Radians(2.356)).add_stop(0.0, w::hex(g.0)).add_stop(1.0, w::hex(g.1));
                container(Space::new()).width(Length::Fill).height(Length::Fixed(64.0)).style(move |_: &iced::Theme| container::Style { background: Some(iced::Background::Gradient(iced::Gradient::Linear(lin))), ..Default::default() }).into()
            }
        };
        let lab = container(container(t(l, 10.0, Color::WHITE)).padding(Padding { top: 1.0, right: 5.0, bottom: 1.0, left: 5.0 }).style(ui::boxed(Color::from_rgba(0.0, 0.0, 0.0, 0.66), 4.0, None))).width(Length::Fill).height(Length::Fixed(64.0)).align_y(iced::alignment::Vertical::Bottom).padding(Padding { top: 0.0, right: 0.0, bottom: 5.0, left: 6.0 });
        maps = maps.push(container(iced::widget::stack![inner, lab]).clip(true).width(Length::FillPortion(1)).style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((p.line, 1.0)))));
    }
    let materials = container(column![container(tb("ITS MATERIALS (PBR)", 11.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 8.0, left: 0.0 }), maps]).padding(16.0).width(Length::Fixed(520.0)).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))));
    // READY IT FOR
    let ready = container(column![
        container(tb("READY IT FOR", 11.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 4.0, left: 0.0 }),
        w::kv(&p, "For a game", segp(a, "game", "laptop", &[("phone", "Phone · 5K faces"), ("laptop", "Laptop · 15K"), ("pc", "PC · 50K"), ("all", "Keep all")])),
        w::kv(&p, "Textures for a small laptop", segp(a, "texpx", "1024", &[("1024", "1024 px"), ("2048", "2048 px"), ("keep", "Keep")])),
        w::kv(&p, "For printing", row![togp(a, "solid", "Make it solid"), togp(a, "flat", "Flat bottom"), togp(a, "hollow", "Hollow, 2 mm walls")].spacing(6.0).into()),
    ])
    .padding(16.0)
    .width(Length::Fill)
    .style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))));
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "view",
        vec![("tools", tools.into()), ("viewport", vp), ("materials", materials.into()), ("ready", ready.into())],
        |ps| column![ps.take("tools"), ui::gap(1.0, 10.0), ps.take("viewport"), ui::gap(1.0, 12.0), row![ps.take("materials"), ps.take("ready")].spacing(12.0).height(Length::Fixed(150.0))].into(),
        Msg::Describe,
    )
}

pub fn ordinal(i: usize) -> String {
    match i {
        0 => "next".into(),
        1 => "2nd".into(),
        2 => "3rd".into(),
        k => format!("{}th", k + 1),
    }
}

/// Each one waiting, as the board writes it: its name, and how it's made (from words, 3 photos).
pub fn waiting(a: &App) -> Vec<(String, String)> {
    let q: Vec<Value> = a.status.get("queue").and_then(Value::as_array).cloned().unwrap_or_default();
    let running = a.status.pointer("/running/id").and_then(Value::as_str).map(str::to_string);
    q.iter()
        .filter(|x| x.get("id").and_then(Value::as_str).map(str::to_string) != running)
        .map(|x| {
            let id = x.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let it = a.item(&id);
            let ask = it.and_then(|i| i.get("ask")).cloned().unwrap_or(Value::Null);
            let n = ask.get("photos").and_then(Value::as_array).map(|p| p.len()).unwrap_or(0);
            let title = x.get("title").and_then(Value::as_str).unwrap_or(&id).to_string();
            let how = match ask.get("mode").and_then(Value::as_str) {
                Some("words") | Some("picture") => format!("{title} · from words"),
                _ if n > 1 => format!("{title}, {n} photos"),
                _ => title,
            };
            (how, id)
        })
        .collect()
}

fn send_side(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut targets = column![
        target(&p, "Hearth 3D · open and keep working", Msg::Send("hearth3d")),
        target(&p, "Godot · glTF (.glb) · Gravesleep", Msg::Send("godot")),
        target(&p, "Noclip Protocol (O3DE) · .fbx", Msg::Send("o3de")),
        target(&p, "Hearth CAD · STL · Anycubic Chiron", Msg::Send("cad")),
        target(&p, "Save as… .glb .obj .fbx .stl .usdz", Msg::Menu("saveas")),
    ]
    .spacing(8.0);
    if a.menu == Some("saveas") {
        targets = targets.push(w::menu(&p, ["glb", "obj", "fbx", "stl", "usdz"].iter().map(|f| (format!(".{f}"), false, Msg::SaveAs(f))).collect(), 0.0));
    }
    if let Some((msg, _)) = a.said.split_once('\u{1}') {
        targets = targets.push(column![t(msg.to_string(), 12.0, p.text), mouse_area(t("Open the folder ›", 12.0, p.accent)).on_press(Msg::OpenFolder).interaction(mouse::Interaction::Pointer)].spacing(4.0));
    }
    let head = column![w::h2(&p, "Send it to"), ui::gap(1.0, 10.0)];
    let mut wait = Column::new().spacing(6.0).push(container(cap(&p, "Waiting")).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    let ws = waiting(a);
    if ws.is_empty() {
        wait = wait.push(t("Nothing else waiting.", 12.0, p.muted));
    }
    for (i, (title, _)) in ws.iter().enumerate() {
        wait = wait.push(container(row![t(title.clone(), 12.0, p.text), Space::new().width(Length::Fill), t(ordinal(i), 12.0, p.muted)]).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).center_y(Length::Fixed(30.0)).width(Length::Fill).style(ui::boxed(p.bg, p.r(8.0), Some((p.line, 1.0)))));
    }
    let mut lately = Column::new().spacing(8.0).push(container(cap(&p, "Made lately")).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    let done: Vec<&Value> = a.made.iter().filter(|m| m.pointer("/done/ok").and_then(Value::as_bool) == Some(true)).take(6).collect();
    for pair in done.chunks(2) {
        let mut r = Row::new().spacing(8.0);
        for it in pair {
            r = r.push(container(lately_tile(a, it)).width(Length::FillPortion(1)));
        }
        if pair.len() == 1 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        lately = lately.push(r);
    }
    let kept = container(row![t("Every model kept in ~/Hearth/Forge with its photo, words and settings · ", 12.0, p.muted), mouse_area(t("Edit layout", 12.0, p.accent)).on_press(Msg::Describe(hearth_ui::arrange::edit_layout())).interaction(mouse::Interaction::Pointer)].wrap())
        .padding(Padding { top: 14.0, right: 0.0, bottom: 0.0, left: 0.0 });
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "send",
        vec![("head", head.into()), ("targets", targets.into()), ("waiting", wait.into()), ("lately", lately.into()), ("kept", kept.into())],
        |ps| column![ps.take("head"), ps.take("targets"), ps.take("waiting"), ps.take("lately"), ps.take("kept")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(360.0)).height(Length::Fill).padding(18.0).style(w::side_bg(&p)).into()
}

pub fn lately_tile<'a>(a: &'a App, it: &Value) -> Element<'a, Msg> {
    let p = a.p;
    let id = it.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let title = it.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    let inner: Element<'_, Msg> = match a.thumb(&id) {
        Some(h) => image(h.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(Length::Fixed(86.0)).into(),
        None => container(Space::new()).width(Length::Fill).height(Length::Fixed(86.0)).style(ui::boxed(Color::from_rgb8(0x1a, 0x15, 0x12), 0.0, None)).into(),
    };
    let lab = container(t(ui::fit(&title, 26), 10.0, p.muted)).width(Length::Fill).height(Length::Fixed(86.0)).align_y(iced::alignment::Vertical::Bottom).padding(Padding { top: 0.0, right: 6.0, bottom: 5.0, left: 6.0 });
    let on = a.shown().and_then(|s| s.get("id")).and_then(Value::as_str) == Some(id.as_str());
    let c = container(iced::widget::stack![inner, lab]).clip(true).style(ui::boxed(Color::TRANSPARENT, p.r(10.0), Some((if on { p.accent } else { p.line }, 1.0))));
    mouse_area(c).on_press(Msg::Open(id)).interaction(mouse::Interaction::Pointer).into()
}

use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Forge", blocks: &[bl("make", "Make a model", 0, 0, 3, 11), bl("view", "The model and getting it ready", 3, 0, 6, 11), bl("send", "Send it to", 9, 0, 3, 11), bl("credit", "Built on the open models", 0, 11, 12, 1)] },
    PageDef {
        id: "make",
        title: "Make a model",
        blocks: &[
            bl("head", "Make a model: Photo, Words, Several photos, Words → picture", 0, 0, 12, 1),
            bl("photo", "Photo", 0, 1, 12, 2),
            bl("more", "Tell it more", 0, 3, 12, 1),
            bl("model", "Model", 0, 4, 12, 1),
            bl("real", "How real", 0, 5, 12, 3),
            bl("go", "Make the model", 0, 8, 12, 2),
            bls("online", "Or online with Tripo3D", 0, 10, 12, 2),
        ],
    },
    PageDef { id: "view", title: "The model", blocks: &[bl("tools", "Shape, Clay, Real, Wire, Textures and the tools", 0, 0, 12, 1), bl("viewport", "The model turning", 0, 1, 12, 8), bl("materials", "Its materials (PBR)", 0, 9, 6, 3), bl("ready", "Ready it for", 6, 9, 6, 3)] },
    PageDef { id: "send", title: "Send it to", blocks: &[bl("head", "Send it to", 0, 0, 12, 1), bl("targets", "Hearth 3D, Godot, O3DE, Hearth CAD, Save as", 0, 1, 12, 4), bl("waiting", "Waiting", 0, 5, 12, 2), bl("lately", "Made lately", 0, 7, 12, 4), bls("kept", "Where every model is kept", 0, 11, 12, 1)] },
    PageDef { id: "models", title: "Every model", blocks: &[bl("side", "Pages, Show, Tripo3D", 0, 0, 2, 11), bl("list", "Every model Hearth Forge can use", 2, 0, 10, 11), bl("credit", "Each model's own licence", 0, 11, 12, 1)] },
    PageDef { id: "mside", title: "The side", blocks: &[bl("nav", "Make a model, Models, Made, Waiting, Settings", 0, 0, 12, 4), bl("show", "Show", 0, 4, 12, 4), bl("online", "Online (your key)", 0, 8, 12, 3), bls("kept", "Where models are kept", 0, 11, 12, 1)] },
    PageDef { id: "mlist", title: "Every model", blocks: &[bl("head", "Find, Sort, Get the best 4", 0, 0, 12, 1), bl("about", "What Fits means", 0, 1, 12, 1), bl("table", "The models", 0, 2, 12, 10)] },
];

fn credit_line<'a>(p: &Pal, bold: &str, after: &str) -> Element<'a, Msg> {
    container(row![t("Hearth Forge · ", 11.0, p.muted), tb(bold.to_string(), 11.0, Pal::mix(p.muted, p.text, 0.4)), t(after.to_string(), 11.0, p.muted)].align_y(iced::Center))
        .padding(Padding { top: 6.0, right: 18.0, bottom: 6.0, left: 18.0 })
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let base: Element<'_, Msg> = if a.page == "make" {
        let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![
            ("make", hearth_ui::studio::made(&a.describe, "window", "make", || make_side(a))),
            ("view", hearth_ui::studio::made(&a.describe, "window", "view", || container(view_middle(a)).padding(Padding { top: 16.0, right: 18.0, bottom: 16.0, left: 18.0 }).width(Length::Fill).height(Length::Fill).into())),
            ("send", hearth_ui::studio::made(&a.describe, "window", "send", || send_side(a))),
            ("credit", credit_line(&p, "Built on Hunyuan3D, TRELLIS, TripoSR and the other open models (each model's own licence)", " · private, not in the public Store")),
        ];
        for (id, which) in [("pane:waiting", "waiting"), ("pane:lately", "lately")] {
            if a.describe.wants_pane(which) {
                items.push((id, studio_pane(a, which)));
            }
        }
        hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| column![row![ps.take("make"), ps.take("view"), ps.take("send")].height(Length::Fill), ps.take("credit")].into(), Msg::Describe)
    } else {
        hearth_ui::arrange::parts(
            &a.describe,
            &p,
            "models",
            vec![("side", models::side(a)), ("list", models::main(a)), ("credit", credit_line(&p, "each model credited with its own licence", " · numbers are each maker's own"))],
            |ps| column![row![ps.take("side"), ps.take("list")].height(Length::Fill), ps.take("credit")].into(),
            Msg::Describe,
        )
    };
    let sub = match a.page {
        "make" => "· real 3D models from a photo or a few words",
        "models" => "· models",
        "made" => "· made",
        "waiting" => "· waiting",
        _ => "· settings",
    };
    let gpu_tag: Element<'_, Msg> = match a.up {
        Some(true) if !a.card().is_empty() => w::tag(&p, format!("Nvidia PC · {} · {} GB", a.card(), a.vram_gb()), Tg::Ok),
        Some(true) => w::tag(&p, "Hearth Forge is on", Tg::Ok),
        Some(false) => w::tag(&p, "Nvidia PC not reachable", Tg::Warn),
        None => w::tag(&p, "Looking for the Nvidia PC…", Tg::Dk),
    };
    let middle: Element<'_, Msg> = container(row![t(sub, 14.0, p.muted), Space::new().width(Length::Fill), w::tag(&p, "Private · only your devices", Tg::Dk), gpu_tag].spacing(8.0).align_y(iced::Center)).width(Length::FillPortion(30)).into();
    let bar = hearth_ui::describe::title_bar_with(&p, &a.describe, w::mark("HF", "or", 22.0), "Hearth Forge", Some(middle), TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    let all = container(column![bar, ui::hline(p.line), base]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

use hearth_ui::studio::{tile as ktile, Kit};
const KIT: Kit = Kit {
    name: "Hearth Forge",
    buttons: &[
        ktile("make", "Make the model", G::Play),
        ktile("make-again", "Make again", G::Sync),
        ktile("new-seed", "A new seed", G::Sync),
        ktile("turn", "Turn it", G::Sync),
        ktile("pick-photo", "Pick a photo", G::Camera),
        ktile("send-3d", "Send to Hearth 3D", G::Play),
        ktile("send-godot", "Send to Godot", G::Play),
        ktile("send-cad", "Send to Hearth CAD", G::Play),
        ktile("models", "Every model", G::List),
        ktile("make-page", "Make a model", G::Pencil),
        ktile("get-best", "Get the best 4 for my PC", G::Down),
    ],
    panes: &[ktile("waiting", "Waiting", G::Clock), ktile("lately", "Made lately", G::Grid)],
    info: &[ktile("card", "The graphics card", G::Display), ktile("queue", "Waiting", G::Clock), ktile("made", "Made", G::Check), ktile("credits", "Tripo3D credits", G::Drive)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(_a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "make" => Msg::Make,
        "make-again" => Msg::MakeAgain,
        "new-seed" => Msg::NewSeed,
        "turn" => Msg::Turn,
        "pick-photo" => Msg::PickPhoto,
        "send-3d" => Msg::Send("hearth3d"),
        "send-godot" => Msg::Send("godot"),
        "send-cad" => Msg::Send("cad"),
        "models" => Msg::Page("models"),
        "make-page" => Msg::Page("make"),
        "get-best" => Msg::GetBest,
        _ => return None,
    })
}

fn studio_values(a: &App) {
    a.describe.values(vec![
        ("card", a.card()),
        ("queue", waiting(a).len().to_string()),
        ("made", a.made.len().to_string()),
        ("credits", a.tripo.map(forge::commas).unwrap_or_default()),
    ]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    if which == "waiting" {
        for (i, (title, _)) in waiting(a).iter().enumerate() {
            c = c.push(row![t(title.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(ordinal(i), 12.0, p.muted)]);
        }
    } else {
        for it in a.made.iter().take(4) {
            c = c.push(lately_tile(a, it));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![
        Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme),
        iced::event::listen_with(|e, status, _id| match e {
            iced::Event::Window(iced::window::Event::FileDropped(p)) => Some(Msg::Dropped(p)),
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Character(c), modifiers, .. }) if status == iced::event::Status::Ignored && modifiers.control() && c.as_str() == "v" => Some(Msg::Paste),
            _ => None,
        }),
    ];
    // asked again only while something is being made or got (nothing at rest)
    if a.busy() {
        s.push(iced::time::every(std::time::Duration::from_secs(2)).map(|_| Msg::Tick));
    }
    if a.turning {
        s.push(iced::time::every(std::time::Duration::from_millis(40)).map(|_| Msg::Spin));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-forge-app", "Hearth Forge: real 3D models from a photo or a few words, on your Nvidia PC.\n  --page make|models|made|waiting|settings") {
        return Ok(());
    }
    let w = app::window("hearth-forge", (1840.0, 970.0), (1200.0, 680.0));
    iced::application(App::new, update, shown).title(|_: &App| "Hearth Forge".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
