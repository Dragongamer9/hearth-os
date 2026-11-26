//! hearth-3d: Hearth 3D (docs/mockups/checklists/open-source-17-3d.md; the approved board
//! docs/mockups/open-source-apps/17-3d.png). Modelling with Blender as the engine: the viewport is drawn here in Rust
//! (view3d.rs: orbit, zoom, pick, Move's arrows, G / R / S, the numbers' views, Wire / Solid / Material / Rendered), the
//! scene tree and the picked object's place, turn, size and changes on top, and a timeline with the animation's keys.
//! Blender (GPL-2+) reads and changes the .blend in the background (scene.rs): every edit is Blender's own, saved the
//! way Blender saves (the last file kept as .blend1). Render, import and export (Send to Godot: glTF; Send to Hearth
//! CAD: STL) are Blender's operators too.
//!
//! Made of blocks: the menus and modes, the tools, the viewport, the scene tree, the properties and the timeline are
//! each a block (Edit layout), with its own buttons, panes and info in its blocks window.
//!   hearth-3d [FILE.blend]

mod scene;
mod view3d;

use hearth_oss_kit::need::{self, Have, Need};
use hearth_oss_kit::{self as k, Ch, Sz, E};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, image, row, scrollable, stack, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme};
use scene::{py, Coll, Scene};
use std::path::PathBuf;
use view3d::{Cam, Shade, VEvent};

const APP: &str = "hearth-3d";
const BLENDER: Need = Need { name: "Blender", cmd: "blender", flatpak: "org.blender.Blender", flatpak_cmd: "", licence: "GPL-2+" };
const MODES: [&str; 5] = ["Model", "Sculpt", "Paint", "Animate", "Render"];
const ADDS: [(&str, &str); 8] = [
    ("Cube", "bpy.ops.mesh.primitive_cube_add(location=(0, 0, 1))"),
    ("Sphere", "bpy.ops.mesh.primitive_uv_sphere_add(location=(0, 0, 1))"),
    ("Cylinder", "bpy.ops.mesh.primitive_cylinder_add(location=(0, 0, 1))"),
    ("Plane", "bpy.ops.mesh.primitive_plane_add(size=4)"),
    ("Cone", "bpy.ops.mesh.primitive_cone_add(location=(0, 0, 1))"),
    ("Monkey", "bpy.ops.mesh.primitive_monkey_add(location=(0, 0, 1))"),
    ("Light", "bpy.ops.object.light_add(type='AREA', location=(4, -4, 6))"),
    ("Camera", "bpy.ops.object.camera_add(location=(12, -12, 8))"),
];
const CHANGES: [(&str, &str); 6] = [("Mirror", "MIRROR"), ("Bevel", "BEVEL"), ("Subdivide", "SUBSURF"), ("Array", "ARRAY"), ("Solidify", "SOLIDIFY"), ("Weighted normals", "WEIGHTED_NORMAL")];
const SWATCHES: [[f32; 3]; 8] = [[0.48, 0.54, 0.42], [0.55, 0.6, 0.48], [0.8, 0.8, 0.8], [0.15, 0.15, 0.15], [0.85, 0.35, 0.2], [0.25, 0.45, 0.8], [0.9, 0.75, 0.3], [0.6, 0.3, 0.7]];

#[derive(Debug, Clone, PartialEq)]
pub enum Pop {
    Menu(&'static str),
    AddChange,
    Keys,
    Overlays,
    Mod(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Have(Have),
    Open,
    Opened(Option<String>),
    Loaded(Result<Box<Scene>, String>),
    View(VEvent),
    Mode(usize),
    Tool(usize),
    Shade(Shade),
    Persp,
    Pick(String),
    Fold(String),
    Hide(String),
    NoRender(String),
    Find(String),
    Tab(usize),
    Field(usize, usize, String),
    Commit(usize, usize),
    ModShow(String),
    ModRemove(String),
    AddChange(&'static str),
    Colour(usize),
    SceneField(usize, String),
    SceneCommit,
    Pop(Pop),
    Close,
    Add(usize),
    Duplicate,
    Delete,
    HidePicked,
    Undo,
    Save,
    Import,
    Imported(Option<String>),
    Render,
    Rendered(Result<String, String>),
    Export(&'static str),
    Exported(&'static str, Result<String, String>),
    Frame(i64),
    Play,
    Step(i64),
    Tick,
    KeysOf(String),
    AddKey,
    Key(iced::keyboard::Event),
    GetBlender,
    GotBlender(run::Output),
    Drag,
    Max,
    Min,
    WClose,
    Resize(iced::window::Direction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Grab {
    None,
    Move,
    Turn,
    Size,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    have: Have,
    file: Option<PathBuf>,
    scene: Scene,
    loading: bool,
    gen: u64,
    cam: Cam,
    shade: Shade,
    mode: usize,
    tool: usize,
    picked: Vec<String>,
    folded: Vec<String>,
    find: String,
    tab: usize,
    fields: [[String; 3]; 3],
    scene_fields: [String; 3],
    pop: Option<Pop>,
    frame: i64,
    playing: bool,
    keys_of: String,
    grab: Grab,
    grab_from: Option<([f32; 3], [f32; 3], [f32; 3])>,
    overlays: [bool; 3],
    undo: Vec<String>,
    render: Option<PathBuf>,
    said: String,
    busy: bool,
}

fn cache_dir() -> PathBuf {
    let h = k::home();
    std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".cache")).join("hearth/3d")
}

/// Blender in the background: the edits (saved), then the scene read back.
fn blender(have: &Have, file: &PathBuf, edits: String, save: bool) -> Task<Msg> {
    let out = cache_dir().join(format!("scene-{}.json", std::process::id()));
    let _ = std::fs::create_dir_all(cache_dir());
    let script = scene::script(&edits, save);
    let f = file.to_string_lossy().to_string();
    let o = out.to_string_lossy().to_string();
    let cmd = need::argv(&BLENDER, have, &["-b", &f, "--python-expr", &script, "--", &o]);
    Task::perform(run::spawn(cmd), move |r| {
        if !r.ok && !std::path::Path::new(&out).exists() {
            return Msg::Loaded(Err(r.why()));
        }
        let j = std::fs::read_to_string(&out).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let _ = std::fs::remove_file(&out);
        match j {
            Some(j) => Msg::Loaded(Ok(Box::new(scene::parse(&j)))),
            None => Msg::Loaded(Err(if r.ok { "Blender didn't say what's in the file.".into() } else { r.why() })),
        }
    })
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let set = k::read_settings(APP);
        let file = std::env::args().skip(1).find(|a| !a.starts_with("--") && a.ends_with(".blend")).map(PathBuf::from).or_else(|| set.get("last").and_then(|v| v.as_str()).map(PathBuf::from).filter(|p| p.is_file()));
        let a = App {
            describe: hearth_ui::describe::Describe::new(APP, "Hearth 3D").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            have: Have::Looking,
            file,
            scene: Scene::default(),
            loading: false,
            gen: 0,
            cam: Cam::default(),
            shade: match app::arg("--shade").as_deref() {
                Some("wire") => Shade::Wire,
                Some("material") => Shade::Material,
                _ => Shade::Solid,
            },
            mode: 0,
            tool: 0,
            picked: app::arg("--pick").map(|s| s.split(',').map(str::to_string).collect()).unwrap_or_default(),
            folded: Vec::new(),
            find: String::new(),
            tab: 0,
            fields: Default::default(),
            scene_fields: Default::default(),
            pop: None,
            frame: 1,
            playing: false,
            keys_of: String::new(),
            grab: Grab::None,
            grab_from: None,
            overlays: [true, true, true],
            undo: Vec::new(),
            render: None,
            said: String::new(),
            busy: false,
        };
        (a, Task::perform(need::probe(&BLENDER), Msg::Have))
    }
    fn obj(&self, n: &str) -> Option<&scene::Obj> {
        self.scene.objects.iter().find(|o| o.name == n)
    }
    fn picked_obj(&self) -> Option<&scene::Obj> {
        self.picked.first().and_then(|n| self.obj(n))
    }
    fn fill_fields(&mut self) {
        let Some(o) = self.picked_obj().cloned() else { return };
        let fmt = |v: f32, unit: &str| {
            let r = (v * 100.0).round() / 100.0;
            if unit == "m" { format!("{} m", trim(r)) } else if unit == "°" { format!("{}°", trim(r.round())) } else { format!("{r:.2}") }
        };
        for i in 0..3 {
            self.fields[0][i] = fmt(o.loc[i], "m");
            self.fields[1][i] = fmt(o.rot[i], "°");
            self.fields[2][i] = fmt(o.scale[i], "");
        }
        self.scene_fields = [self.scene.start.to_string(), self.scene.end.to_string(), self.scene.fps.to_string()];
    }
    /// Edits Blender makes and saves (then the scene is read again); `undo` is what puts it back.
    fn edit(&mut self, code: String, undo: Option<String>) -> Task<Msg> {
        let Some(f) = self.file.clone() else { return Task::none() };
        if let Some(u) = undo {
            self.undo.push(u);
        }
        self.busy = true;
        self.render = None;
        blender(&self.have, &f, code, true)
    }
}

fn trim(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
fn num(s: &str) -> Option<f32> {
    s.trim().trim_end_matches('°').trim_end_matches('m').trim().replace(',', ".").parse().ok()
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Have(h) => {
            a.have = h;
            if a.have.ok() {
                if let Some(f) = a.file.clone() {
                    a.loading = true;
                    return blender(&a.have, &f, String::new(), false);
                }
            }
        }
        Msg::Open => return Task::perform(k::pick_file("A Blender file", &["*.blend"]), Msg::Opened),
        Msg::Opened(Some(f)) => {
            a.file = Some(PathBuf::from(&f));
            let _ = k::save_settings(APP, [("last".to_string(), serde_json::json!(f))].into_iter().collect());
            a.loading = true;
            a.picked.clear();
            a.undo.clear();
            return blender(&a.have, &PathBuf::from(f), String::new(), false);
        }
        Msg::Opened(None) => {}
        Msg::Loaded(r) => {
            a.loading = false;
            a.busy = false;
            match r {
                Ok(s) => {
                    a.scene = *s;
                    a.gen += 1;
                    if a.frame < a.scene.start || a.frame > a.scene.end || a.gen == 1 {
                        a.frame = a.scene.frame;
                    }
                    a.picked.retain(|n| a.scene.objects.iter().any(|o| &o.name == n));
                    // a collection of many is folded at first (the board's "▸ Legs · 8")
                    if a.gen == 1 {
                        fn many(c: &Coll, out: &mut Vec<String>) {
                            for ch in &c.children {
                                if ch.objects.len() >= 8 {
                                    out.push(ch.name.clone());
                                }
                                many(ch, out);
                            }
                        }
                        many(&a.scene.root, &mut a.folded);
                    }
                    if a.keys_of.is_empty() {
                        a.keys_of = a.scene.root.children.iter().flat_map(|c| c.children.iter()).find(|c| c.objects.iter().any(|n| a.scene.objects.iter().any(|o| &o.name == n && !o.keys.is_empty()))).map(|c| c.name.clone()).unwrap_or_default();
                    }
                    a.fill_fields();
                }
                Err(e) => a.said = format!("Blender couldn't do that: {e}"),
            }
        }
        Msg::View(ev) => match ev {
            VEvent::Orbit(dx, dy) => {
                a.cam.yaw -= dx * 0.008;
                a.cam.pitch = (a.cam.pitch + dy * 0.008).clamp(-1.5, 1.5);
            }
            VEvent::Pan(dx, dy) => {
                let v = view3d::View::new(&a.cam, iced::Size::new(900.0, 600.0));
                let d = v.screen_to_world(a.cam.target, -dx, -dy);
                a.cam.target = [a.cam.target[0] + d[0], a.cam.target[1] + d[1], a.cam.target[2] + d[2]];
            }
            VEvent::Zoom(d) => a.cam.dist = (a.cam.dist * (1.0 - d * 0.1)).clamp(1.0, 400.0),
            VEvent::Pick(n, shift) => match n {
                Some(n) => {
                    if shift {
                        if !a.picked.contains(&n) {
                            a.picked.push(n);
                        }
                    } else {
                        a.picked = vec![n];
                    }
                    a.fill_fields();
                }
                None if !shift => a.picked.clear(),
                None => {}
            },
            VEvent::Drag(dx, dy) => {
                // the picked object follows the pointer, as Move / Turn / Size do (written to the file at the end)
                let Some(o) = a.picked_obj().cloned() else { return Task::none() };
                let (l0, r0, s0) = *a.grab_from.get_or_insert((o.loc, o.rot, o.scale));
                let how = match (a.grab, a.tool) {
                    (Grab::None, 2) | (Grab::Turn, _) => Grab::Turn,
                    (Grab::None, 3) | (Grab::Size, _) => Grab::Size,
                    _ => Grab::Move,
                };
                let v = view3d::View::new(&a.cam, iced::Size::new(900.0, 600.0));
                if let Some(ob) = a.scene.objects.iter_mut().find(|x| x.name == o.name) {
                    match how {
                        Grab::Turn => ob.rot = [r0[0], r0[1], r0[2] - dx * 0.5],
                        Grab::Size => {
                            let s = (1.0 - dy * 0.005).max(0.01);
                            ob.scale = [s0[0] * s, s0[1] * s, s0[2] * s];
                        }
                        _ => {
                            let d = v.screen_to_world(l0, dx, dy);
                            ob.loc = [l0[0] + d[0], l0[1] + d[1], l0[2] + d[2]];
                        }
                    }
                    ob.mat = trs(ob.loc, ob.rot, ob.scale);
                    ob.anim.clear();
                }
                a.gen += 1;
                a.fill_fields();
            }
            VEvent::DragEnd => {
                a.grab = Grab::None;
                let Some((l0, r0, s0)) = a.grab_from.take() else { return Task::none() };
                let Some(o) = a.picked_obj().cloned() else { return Task::none() };
                let set = |l: [f32; 3], r: [f32; 3], s: [f32; 3]| format!("o = ob({})\no.location = ({}, {}, {})\no.rotation_euler = (math.radians({}), math.radians({}), math.radians({}))\no.scale = ({}, {}, {})", py(&o.name), l[0], l[1], l[2], r[0], r[1], r[2], s[0], s[1], s[2]);
                return a.edit(set(o.loc, o.rot, o.scale), Some(set(l0, r0, s0)));
            }
            VEvent::Size(..) => {}
        },
        Msg::Mode(i) => {
            a.mode = i;
            match i {
                1 | 2 => a.said = "Sculpt and Paint aren't in Hearth 3D yet: Model, Animate and Render are.".into(),
                3 => a.said.clear(),
                4 => return update(a, Msg::Render),
                _ => a.said.clear(),
            }
        }
        Msg::Tool(i) => {
            a.tool = i;
            match i {
                5 => {
                    // look through the scene's camera
                    if let Some(c) = a.scene.objects.iter().find(|o| o.kind == "CAMERA") {
                        let p = [c.mat[12], c.mat[13], c.mat[14]];
                        let d = [p[0] - a.cam.target[0], p[1] - a.cam.target[1], p[2] - a.cam.target[2]];
                        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1.0);
                        a.cam = Cam { yaw: d[1].atan2(d[0]), pitch: (d[2] / dist).asin(), dist, target: a.cam.target, persp: true };
                    }
                }
                6 => {
                    if let Some(o) = a.picked_obj() {
                        a.cam.target = [o.mat[12], o.mat[13], o.mat[14]];
                        a.cam.dist = 14.0;
                    }
                }
                7 => a.cam = Cam::default(),
                _ => {}
            }
        }
        Msg::Shade(s) => {
            a.shade = s;
            if s == Shade::Rendered && a.render.is_none() {
                return update(a, Msg::Render);
            }
        }
        Msg::Persp => a.cam.persp = !a.cam.persp,
        Msg::Pick(n) => {
            a.picked = vec![n];
            a.fill_fields();
        }
        Msg::Fold(n) => {
            if let Some(i) = a.folded.iter().position(|x| *x == n) {
                a.folded.remove(i);
            } else {
                a.folded.push(n);
            }
        }
        Msg::Hide(n) => {
            let h = a.obj(&n).map(|o| o.hidden).unwrap_or(false);
            return a.edit(format!("ob({}).hide_set({})", py(&n), if h { "False" } else { "True" }), Some(format!("ob({}).hide_set({})", py(&n), if h { "True" } else { "False" })));
        }
        Msg::NoRender(n) => {
            let r = a.obj(&n).map(|o| o.render).unwrap_or(true);
            return a.edit(format!("ob({}).hide_render = {}", py(&n), if r { "True" } else { "False" }), Some(format!("ob({}).hide_render = {}", py(&n), if r { "False" } else { "True" })));
        }
        Msg::Find(s) => a.find = s,
        Msg::Tab(i) => a.tab = i,
        Msg::Field(r, c, s) => a.fields[r][c] = s,
        Msg::Commit(r, c) => {
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            let Some(v) = num(&a.fields[r][c]) else {
                a.fill_fields();
                return Task::none();
            };
            let (attr, val, old) = match r {
                0 => ("location", v.to_string(), o.loc[c].to_string()),
                1 => ("rotation_euler", format!("math.radians({v})"), format!("math.radians({})", o.rot[c])),
                _ => ("scale", v.to_string(), o.scale[c].to_string()),
            };
            return a.edit(format!("ob({}).{attr}[{c}] = {val}", py(&o.name)), Some(format!("ob({}).{attr}[{c}] = {old}", py(&o.name))));
        }
        Msg::ModShow(m) => {
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            let code = format!("md = ob({}).modifiers.get({})\nmd.show_viewport = not md.show_viewport", py(&o.name), py(&m));
            return a.edit(code.clone(), Some(code));
        }
        Msg::ModRemove(m) => {
            a.pop = None;
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            return a.edit(format!("o = ob({})\no.modifiers.remove(o.modifiers.get({}))", py(&o.name), py(&m)), None);
        }
        Msg::AddChange(kind) => {
            a.pop = None;
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            let name = CHANGES.iter().find(|c| c.1 == kind).map(|c| c.0).unwrap_or(kind);
            return a.edit(format!("o = ob({})\nmd = o.modifiers.new({}, {})", py(&o.name), py(name), py(kind)), Some(format!("o = ob({})\nmd = o.modifiers[-1] if len(o.modifiers) else None\nif md: o.modifiers.remove(md)", py(&o.name))));
        }
        Msg::Colour(i) => {
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            let c = SWATCHES[i];
            let code = format!(
                "o = ob({})\nm = o.active_material or bpy.data.materials.new(o.name)\nif not o.material_slots: o.data.materials.append(m)\nm.use_nodes = True\nn = m.node_tree.nodes.get('Principled BSDF')\nif n: n.inputs['Base Color'].default_value = ({}, {}, {}, 1)\nm.diffuse_color = ({}, {}, {}, 1)",
                py(&o.name), c[0], c[1], c[2], c[0], c[1], c[2]
            );
            return a.edit(code, None);
        }
        Msg::SceneField(i, s) => a.scene_fields[i] = s,
        Msg::SceneCommit => {
            let v: Vec<i64> = a.scene_fields.iter().map(|s| s.trim().parse().unwrap_or(0)).collect();
            if v[0] < 0 || v[1] <= v[0] || v[2] < 1 {
                a.fill_fields();
                return Task::none();
            }
            let old = format!("sc = bpy.context.scene\nsc.frame_start = {}\nsc.frame_end = {}\nsc.render.fps = {}", a.scene.start, a.scene.end, a.scene.fps);
            return a.edit(format!("sc = bpy.context.scene\nsc.frame_start = {}\nsc.frame_end = {}\nsc.render.fps = {}", v[0], v[1], v[2]), Some(old));
        }
        Msg::Pop(p) => a.pop = Some(p),
        Msg::Close => a.pop = None,
        Msg::Add(i) => {
            a.pop = None;
            return a.edit(ADDS[i].1.to_string(), None);
        }
        Msg::Duplicate => {
            a.pop = None;
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            return a.edit(format!("o = ob({})\nc = o.copy()\nc.data = o.data.copy() if o.data else None\nc.location.x += 2\nfor col in o.users_collection: col.objects.link(c)", py(&o.name)), None);
        }
        Msg::Delete => {
            a.pop = None;
            let names: Vec<String> = a.picked.clone();
            if names.is_empty() {
                return Task::none();
            }
            a.picked.clear();
            return a.edit(names.iter().map(|n| format!("bpy.data.objects.remove(ob({}), do_unlink=True)", py(n))).collect::<Vec<_>>().join("\n"), None);
        }
        Msg::HidePicked => {
            a.pop = None;
            if let Some(n) = a.picked.first().cloned() {
                return update(a, Msg::Hide(n));
            }
        }
        Msg::Undo => {
            a.pop = None;
            if let Some(u) = a.undo.pop() {
                let Some(f) = a.file.clone() else { return Task::none() };
                a.busy = true;
                return blender(&a.have, &f, u, true);
            }
            a.said = "Nothing to undo.".into();
        }
        Msg::Save => {
            a.pop = None;
            return a.edit(String::new(), None);
        }
        Msg::Import => {
            a.pop = None;
            return Task::perform(k::pick_file("A 3D model", &["*.glb", "*.gltf", "*.obj", "*.stl", "*.fbx"]), Msg::Imported);
        }
        Msg::Imported(Some(f)) => {
            let low = f.to_lowercase();
            let op = if low.ends_with(".glb") || low.ends_with(".gltf") {
                "bpy.ops.import_scene.gltf(filepath=F)"
            } else if low.ends_with(".obj") {
                "bpy.ops.wm.obj_import(filepath=F)"
            } else if low.ends_with(".stl") {
                "try:\n    bpy.ops.wm.stl_import(filepath=F)\nexcept Exception:\n    bpy.ops.import_mesh.stl(filepath=F)"
            } else {
                "bpy.ops.import_scene.fbx(filepath=F)"
            };
            return a.edit(format!("F = {}\n{op}", py(&f)), None);
        }
        Msg::Imported(None) => {}
        Msg::Render => {
            a.pop = None;
            let Some(f) = a.file.clone() else { return Task::none() };
            let out = cache_dir().join(format!("render-{}.png", a.frame));
            let _ = std::fs::create_dir_all(cache_dir());
            let code = format!("sc = bpy.context.scene\nsc.frame_set({})\nsc.render.filepath = {}\nsc.render.image_settings.file_format = 'PNG'\nbpy.ops.render.render(write_still=True)", a.frame, py(&out.to_string_lossy()));
            let cmd = need::argv(&BLENDER, &a.have, &["-b", &f.to_string_lossy(), "--python-expr", &code]);
            a.busy = true;
            a.said = format!("Blender is rendering frame {}…", a.frame);
            return Task::perform(run::spawn(cmd), move |o| Msg::Rendered(if out.is_file() { Ok(out.to_string_lossy().to_string()) } else { Err(o.why()) }));
        }
        Msg::Rendered(r) => {
            a.busy = false;
            match r {
                Ok(p) => {
                    a.render = Some(PathBuf::from(p));
                    a.shade = Shade::Rendered;
                    a.said.clear();
                }
                Err(e) => a.said = format!("The render didn't finish: {e}"),
            }
        }
        Msg::Export(to) => {
            a.pop = None;
            let Some(f) = a.file.clone() else { return Task::none() };
            let stem = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "scene".into());
            let (out, code) = if to == "godot" {
                let out = f.with_extension("glb");
                (out.clone(), format!("bpy.ops.export_scene.gltf(filepath={}, export_format='GLB')", py(&out.to_string_lossy())))
            } else {
                let d = k::home().join("Documents/Hearth CAD");
                let _ = std::fs::create_dir_all(&d);
                let out = d.join(format!("{stem}.stl"));
                let p = py(&out.to_string_lossy());
                (out, format!("try:\n    bpy.ops.wm.stl_export(filepath={p})\nexcept Exception:\n    bpy.ops.export_mesh.stl(filepath={p})"))
            };
            let cmd = need::argv(&BLENDER, &a.have, &["-b", &f.to_string_lossy(), "--python-expr", &code]);
            a.busy = true;
            return Task::perform(run::spawn(cmd), move |o| Msg::Exported(to, if out.is_file() { Ok(k::tilde(&out)) } else { Err(o.why()) }));
        }
        Msg::Exported(to, r) => {
            a.busy = false;
            a.said = match (to, r) {
                ("godot", Ok(p)) => format!("Ready for Godot: {p} (glTF, with its materials and animation). Put it in your Godot project's folder."),
                (_, Ok(p)) => {
                    run::detached(&["hearth-cad", &p.replace('~', &k::home().to_string_lossy())]);
                    format!("Sent to Hearth CAD: {p} (STL).")
                }
                (_, Err(e)) => format!("It couldn't be written: {e}"),
            };
        }
        Msg::Frame(f) => {
            a.frame = f.clamp(a.scene.start, a.scene.end.max(a.scene.start));
            a.gen += 1;
        }
        Msg::Play => a.playing = !a.playing,
        Msg::Step(d) => {
            let n = if d < 0 { a.scene.start } else { a.scene.end };
            return update(a, Msg::Frame(n));
        }
        Msg::Tick => {
            let next = if a.frame >= a.scene.end { a.scene.start } else { a.frame + 1 };
            return update(a, Msg::Frame(next));
        }
        Msg::KeysOf(n) => {
            a.pop = None;
            a.keys_of = n;
        }
        Msg::AddKey => {
            let Some(o) = a.picked_obj().cloned() else { return Task::none() };
            let f = a.frame;
            return a.edit(format!("o = ob({})\nfor p in ('location', 'rotation_euler', 'scale'):\n    o.keyframe_insert(p, frame={f})", py(&o.name)), Some(format!("o = ob({})\nfor p in ('location', 'rotation_euler', 'scale'):\n    try:\n        o.keyframe_delete(p, frame={f})\n    except Exception:\n        pass", py(&o.name))));
        }
        Msg::Key(ev) => {
            use iced::keyboard::{key::Named, Event as KE, Key};
            let KE::KeyPressed { key, modifiers, .. } = ev else { return Task::none() };
            if let Some(dm) = hearth_ui::arrange::arrange_key(&a.describe, &key, modifiers) {
                return a.describe.update(dm).map(Msg::Describe);
            }
            match key.as_ref() {
                Key::Character("g") => a.grab = Grab::Move,
                Key::Character("r") => a.grab = Grab::Turn,
                Key::Character("s") if !modifiers.control() => a.grab = Grab::Size,
                Key::Character("s") => return update(a, Msg::Save),
                Key::Character("z") if modifiers.control() => return update(a, Msg::Undo),
                Key::Named(Named::Tab) => a.said = "Editing a mesh's points (Tab) isn't in Hearth 3D yet.".into(),
                Key::Named(Named::Escape) => a.grab = Grab::None,
                Key::Named(Named::Space) => a.playing = !a.playing,
                Key::Named(Named::Delete) => return update(a, Msg::Delete),
                Key::Character(c) => {
                    // the numbers' views: 1 front, 3 side, 7 top, 9 bottom, 5 flat / perspective, 2 4 6 8 turn
                    let pi = std::f32::consts::PI;
                    match c {
                        "1" => (a.cam.yaw, a.cam.pitch) = (-pi / 2.0, 0.0),
                        "3" => (a.cam.yaw, a.cam.pitch) = (0.0, 0.0),
                        "7" => (a.cam.yaw, a.cam.pitch) = (-pi / 2.0, 1.5),
                        "9" => a.cam.pitch = -a.cam.pitch,
                        "5" => a.cam.persp = !a.cam.persp,
                        "4" => a.cam.yaw += pi / 12.0,
                        "6" => a.cam.yaw -= pi / 12.0,
                        "8" => a.cam.pitch = (a.cam.pitch + pi / 12.0).min(1.5),
                        "2" => a.cam.pitch = (a.cam.pitch - pi / 12.0).max(-1.5),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Msg::GetBlender => {
            a.busy = true;
            return Task::perform(run::spawn(need::install_argv(&BLENDER)), Msg::GotBlender);
        }
        Msg::GotBlender(o) => {
            a.busy = false;
            a.said = if o.ok { "Blender is here.".into() } else { format!("Couldn't get Blender: {}", o.why()) };
            return Task::perform(need::probe(&BLENDER), Msg::Have);
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::WClose => return win::close(),
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

/// Place, turn (degrees, XYZ) and size as a matrix (column-major), for an object moved here before Blender has it.
fn trs(l: [f32; 3], r: [f32; 3], s: [f32; 3]) -> [f32; 16] {
    let (x, y, z) = (r[0].to_radians(), r[1].to_radians(), r[2].to_radians());
    let (cx, sx, cy, sy, cz, sz) = (x.cos(), x.sin(), y.cos(), y.sin(), z.cos(), z.sin());
    // R = Rz · Ry · Rx (Blender's XYZ Euler)
    let m = [
        [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
        [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
        [-sy, cy * sx, cy * cx],
    ];
    [m[0][0] * s[0], m[1][0] * s[0], m[2][0] * s[0], 0.0, m[0][1] * s[1], m[1][1] * s[1], m[2][1] * s[1], 0.0, m[0][2] * s[2], m[1][2] * s[2], m[2][2] * s[2], 0.0, l[0], l[1], l[2], 1.0]
}

// ------------------------------------------------------------------ the window

fn rail(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let icons = [G::Search, G::Arrow, G::Sync, G::Maximize, G::Grid, G::Camera, G::Display, G::Home];
    let names = ["Select", "Move", "Turn", "Size", "Box select", "Look through the camera", "Frame the picked", "Back to the start view"];
    let mut c = Column::new().spacing(7.0).align_x(iced::Center);
    for (i, g) in icons.iter().enumerate() {
        if i == 4 {
            c = c.push(container(ui::hline(p.line)).width(Length::Fixed(28.0)).padding(k::pad(3.0, 0.0, 6.0, 0.0)));
        }
        let on = a.tool == i;
        let tile = container(ui::glyph(*g, if on { p.accent } else { p.muted }, 15.0, 1.8)).center_x(Length::Fixed(32.0)).center_y(Length::Fixed(32.0)).style(ui::boxed(if on { Pal::mix(p.panel2, p.accent, 0.2) } else { Color::TRANSPARENT }, 8.0, Some((if on { p.accent } else { Color::TRANSPARENT }, 1.0))));
        c = c.push(iced::widget::tooltip(k::click(tile, Some(Msg::Tool(i))), container(t(names[i], 11.0, p.text)).padding(5.0).style(ui::boxed(p.panel2, 6.0, Some((p.line, 1.0)))), iced::widget::tooltip::Position::Right));
    }
    container(c).width(Length::Fixed(44.0)).height(Length::Fill).padding(k::pad(10.0, 6.0, 10.0, 6.0)).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.2), 0.0, None)).into()
}

/// The path of collections down to an object ("Mech › Body").
fn path_of(c: &Coll, name: &str) -> Option<Vec<String>> {
    if c.objects.iter().any(|o| o == name) {
        return Some(vec![]);
    }
    for ch in &c.children {
        if let Some(mut v) = path_of(ch, name) {
            v.insert(0, ch.name.clone());
            return Some(v);
        }
    }
    None
}

fn viewport(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let canvas: E<'_, Msg> = if a.shade == Shade::Rendered && a.render.is_some() {
        container(image(image::Handle::from_path(a.render.clone().unwrap_or_default())).content_fit(iced::ContentFit::Contain)).center_x(Length::Fill).center_y(Length::Fill).into()
    } else {
        canvas::Canvas::new(view3d::Viewport { scene: &a.scene, cam: a.cam, shade: a.shade, picked: &a.picked, frame: a.frame, accent: p.accent, gen: a.gen, overlays: a.overlays, on: Msg::View }).width(Length::Fill).height(Length::Fill).into()
    };
    let name = a.picked.first().cloned().unwrap_or_default();
    let coll = path_of(&a.scene.root, &name).map(|mut v| {
        v.retain(|x| x != "Scene Collection");
        v.push(name.clone());
        v.join(" › ")
    });
    let tl = row![
        k::chip(&p, if a.cam.persp { "User view · perspective" } else { "User view · flat" }, Ch::Plain, Sz::Sm, Some(Msg::Persp)),
        k::chip::<Msg>(&p, format!("Collection: {}", coll.unwrap_or_else(|| "nothing picked".into())), Ch::Plain, Sz::Sm, None),
    ]
    .spacing(6.0);
    let sh = |s: Shade, l: &str| (l.to_string(), a.shade == s, Msg::Shade(s));
    let tr = row![k::seg(&p, vec![sh(Shade::Wire, "Wire"), sh(Shade::Solid, "Solid"), sh(Shade::Material, "Material"), sh(Shade::Rendered, "Rendered")]), k::chip(&p, "Overlays ▾", Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::Overlays)))].spacing(6.0).align_y(iced::Center);
    let faces: u64 = a.scene.objects.iter().map(|o| o.faces).sum();
    let mut stats = Column::new().spacing(3.0);
    if let Some(o) = a.picked_obj() {
        stats = stats.push(k::mu1::<Msg>(&p, format!("{} · {} faces · {} points", o.name, k::commas(o.faces), k::commas(o.points)), 11.0));
    }
    stats = stats.push(k::mu1::<Msg>(&p, format!("Scene: {} faces · {} objects picked of {}", k::commas(faces), a.picked.len(), a.scene.objects.len()), 11.0));
    let hint = container(t("G move · R turn · S size · Tab edit · numbers 1–9 views", 11.0, p.text).wrapping(Wrapping::None)).padding(k::pad(5.0, 10.0, 5.0, 10.0)).style(ui::boxed(Pal::a(k::rgb("#173040"), 0.9), 6.0, Some((k::BLUE, 1.0))));
    let mut layers = stack![canvas];
    if a.loading || a.busy {
        layers = layers.push(container(k::mu::<Msg>(&p, if a.loading { "Blender is opening the file…" } else { "Blender is at work…" }, 12.0)).center_x(Length::Fill).padding(k::pad(52.0, 0.0, 0.0, 0.0)));
    }
    if a.file.is_none() && a.have.ok() {
        layers = layers.push(container(column![k::h2::<Msg>(&p, "Open a Blender file"), k::chip(&p, "Open…", Ch::Acc, Sz::Md, Some(Msg::Open))].spacing(10.0).align_x(iced::Center)).center_x(Length::Fill).center_y(Length::Fill));
    }
    if !a.have.ok() && a.have != Have::Looking {
        layers = layers.push(container(column![k::h2::<Msg>(&p, "Hearth 3D works with Blender"), k::para(p.muted, need::missing_words(&BLENDER), 12.0), k::chip(&p, if a.busy { "Getting Blender…" } else { "Get Blender" }, Ch::Acc, Sz::Md, (!a.busy).then_some(Msg::GetBlender))].spacing(10.0).max_width(460.0)).center_x(Length::Fill).center_y(Length::Fill));
    }
    let overlay = column![row![tl, k::fill(), tr].align_y(iced::Center), Space::new().height(Length::Fill), row![stats, k::fill(), hint].align_y(iced::Bottom)].padding(12.0);
    layers.push(overlay).width(Length::Fill).height(Length::Fill).into()
}

fn tree_rows<'a>(a: &'a App, c: &Coll, depth: usize, out: &mut Column<'a, Msg>) {
    let p = a.p;
    let q = a.find.to_lowercase();
    let row_of = |label: String, d: usize, on: bool, msg: Msg, eye: Option<(bool, bool, String)>| -> E<'a, Msg> {
        let sq = container(Space::new()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(ui::boxed(if on { p.accent } else { Pal::mix(p.muted, p.bg, 0.4) }, 2.5, None));
        let mut r = Row::new().spacing(8.0).align_y(iced::Center).push(k::gap(d as f32 * 13.0, 1.0)).push(sq).push(t(label, 12.0, if on { p.text } else { p.text }).wrapping(Wrapping::None)).push(k::fill());
        if let Some((hidden, render, name)) = eye {
            r = r.push(k::click(t(if hidden { "◌" } else { "◉" }, 11.0, p.muted), Some(Msg::Hide(name.clone())))).push(k::click(t(if render { "▣" } else { "▢" }, 11.0, p.muted), Some(Msg::NoRender(name))));
        } else {
            r = r.push(t("◉ ▣", 11.0, p.muted));
        }
        k::click(container(r).padding(k::pad(4.0, 6.0, 4.0, 4.0)).width(Length::Fill).style(ui::boxed(if on { Pal::mix(p.panel2, p.accent, 0.18) } else { Color::TRANSPARENT }, 6.0, None)), Some(msg))
    };
    let folded = a.folded.contains(&c.name);
    let n_inside = c.objects.len() + c.children.len();
    let shown = if depth == 0 && c.name == "Scene Collection" { "Scene collection".to_string() } else { c.name.clone() };
    let label = if folded { format!("▸ {shown}{}", if n_inside > 1 { format!(" · {n_inside}") } else { String::new() }) } else { format!("▾ {shown}") };
    *out = std::mem::replace(out, Column::new()).push(row_of(label, depth, false, Msg::Fold(c.name.clone()), None));
    if folded {
        return;
    }
    for ch in &c.children {
        tree_rows(a, ch, depth + 1, out);
    }
    for n in &c.objects {
        if !q.is_empty() && !n.to_lowercase().contains(&q) {
            continue;
        }
        let o = a.obj(n);
        let on = a.picked.contains(n);
        *out = std::mem::replace(out, Column::new()).push(row_of(n.clone(), depth + 1, on, Msg::Pick(n.clone()), Some((o.map(|o| o.hidden).unwrap_or(false), o.map(|o| o.render).unwrap_or(true), n.clone()))));
    }
}

fn scene_panel(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let mut rows = Column::new().spacing(1.0);
    tree_rows(a, &a.scene.root, 0, &mut rows);
    column![row![k::cap::<Msg>(&p, "Scene"), k::fill(), container(k::search(&p, "Find", &a.find, Msg::Find)).width(Length::Fixed(140.0))].align_y(iced::Center), k::gap(1.0, 6.0), scrollable(rows).height(Length::Fill)].padding(k::pad(10.0, 10.0, 8.0, 10.0)).into()
}

fn xyz<'a>(a: &'a App, r: usize, label: &str) -> E<'a, Msg> {
    let p = a.p;
    let cols = [k::RED, k::GREEN, k::BLUE];
    let mut row_ = Row::new().spacing(5.0).align_y(iced::Center).push(container(t(label.to_string(), 12.0, p.text)).width(Length::Fixed(52.0)));
    for c in 0..3 {
        let inp = k::input(&p, "", &a.fields[r][c], move |s| Msg::Field(r, c, s)).on_submit(Msg::Commit(r, c)).width(Length::Fill);
        row_ = row_.push(container(row![tb(["X", "Y", "Z"][c], 11.0, cols[c]), inp].spacing(4.0).align_y(iced::Center)).padding(k::pad(0.0, 0.0, 0.0, 8.0)).width(Length::FillPortion(1)).style(ui::boxed(p.bg, p.r(7.0), Some((p.line, 1.0)))));
    }
    row_.into()
}

fn mods_col<'a>(a: &'a App, c: Column<'a, Msg>) -> Column<'a, Msg> {
    let p = a.p;
    let Some(o) = a.picked_obj() else { return c };
        let mut c = c.push(container(k::cap::<Msg>(&p, format!("Changes on top · {}", o.mods.len()))).padding(k::pad(8.0, 0.0, 2.0, 0.0)));
    let cols = [k::BLUE, k::GREEN, k::PINK, k::YELLOW, k::RED];
    for (i, m) in o.mods.iter().enumerate() {
        let label = if m.words.is_empty() { m.name.clone() } else { format!("{} · {}", m.name, m.words) };
        let r = row![
            container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(cols[i % cols.len()], 3.0, None)),
            t(label, 12.0, if m.show { p.text } else { p.muted }).wrapping(Wrapping::None),
            k::fill(),
            k::click(t(if m.show { "◉" } else { "◌" }, 11.0, p.muted), Some(Msg::ModShow(m.name.clone()))),
            k::click(t("⌄", 12.0, p.muted), Some(Msg::Pop(Pop::Mod(m.name.clone())))),
        ]
        .spacing(9.0)
        .align_y(iced::Center);
        c = c.push(container(r).padding(k::pad(7.0, 10.0, 7.0, 10.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(8.0), Some((p.line, 1.0)))));
    }
    c.push(row![k::chip(&p, "＋ Add a change", Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::AddChange)))])
}

fn props(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let tabs = k::tabs(&p, ["Object", "Changes", "Material", "Scene", "World"].iter().enumerate().map(|(i, s)| (s.to_string(), a.tab == i, Msg::Tab(i))).collect());
    let mut c = Column::new().spacing(6.0).push(tabs);
    let o = a.picked_obj();
    match (a.tab, o) {
        (_, None) if a.tab < 3 => c = c.push(k::mu(&p, "Pick something in the viewport or the scene list.", 12.0)),
        (0, Some(o)) => {
            c = c.push(container(k::cap::<Msg>(&p, format!("Where it is · {}", o.name))).padding(k::pad(8.0, 0.0, 2.0, 0.0)));
            c = c.push(xyz(a, 0, "Place")).push(xyz(a, 1, "Turn")).push(xyz(a, 2, "Size"));
            c = mods_col(a, c);
        }
        (1, Some(_)) => c = mods_col(a, c),
        (2, Some(o)) => {
            c = c.push(container(k::cap::<Msg>(&p, format!("{}'s colour", o.name))).padding(k::pad(8.0, 0.0, 4.0, 0.0)));
            let mut r = Row::new().spacing(7.0);
            for (i, s) in SWATCHES.iter().enumerate() {
                let near = (s[0] - o.color[0]).abs() + (s[1] - o.color[1]).abs() + (s[2] - o.color[2]).abs() < 0.08;
                r = r.push(k::click(container(Space::new()).width(Length::Fixed(24.0)).height(Length::Fixed(24.0)).style(ui::boxed(Color::from_rgb(s[0], s[1], s[2]), 6.0, Some((if near { Color::WHITE } else { p.line }, if near { 2.0 } else { 1.0 })))), Some(Msg::Colour(i))));
            }
            c = c.push(r);
        }
        (3, _) => {
            c = c.push(container(k::cap::<Msg>(&p, "The animation")).padding(k::pad(8.0, 0.0, 4.0, 0.0)));
            for (i, l) in ["Starts at frame", "Ends at frame", "Frames a second"].iter().enumerate() {
                c = c.push(k::kv(&p, *l, k::input(&p, "", &a.scene_fields[i], move |s| Msg::SceneField(i, s)).on_submit(Msg::SceneCommit).width(Length::Fixed(80.0))));
            }
        }
        _ => {
            let w = a.scene.world;
            c = c.push(container(k::cap::<Msg>(&p, "The world")).padding(k::pad(8.0, 0.0, 4.0, 0.0))).push(k::kv(&p, "Its colour", container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(20.0)).style(ui::boxed(Color::from_rgb(w[0], w[1], w[2]), 4.0, Some((p.line, 1.0))))));
        }
    }
    if !a.said.is_empty() {
        c = c.push(k::gap(1.0, 6.0)).push(k::said(&p, &a.said));
    }
    scrollable(c.padding(k::pad(10.0, 12.0, 10.0, 12.0))).height(Length::Fill).into()
}

/// The timeline's track: a line a frame, the keys as diamonds (the group's on top, the picked object's under them),
/// the frame marker with its number. A press sets the frame.
struct Track<'a> {
    a: &'a App,
}
impl<'a> canvas::Program<Msg> for Track<'a> {
    type State = ();
    fn update(&self, _s: &mut (), ev: &iced::Event, b: Rectangle, c: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        if let iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = ev {
            let p = c.position_in(b)?;
            let s = &self.a.scene;
            let n = (s.end - s.start).max(1) as f32;
            let f = s.start + ((p.x / b.width) * n).round() as i64;
            return Some(canvas::Action::publish(Msg::Frame(f)).and_capture());
        }
        None
    }
    fn draw(&self, _s: &(), r: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let a = self.a;
        let s = &a.scene;
        let n = (s.end - s.start).max(1) as f32;
        let x_of = |fr: i64| (fr - s.start) as f32 / n * (b.width - 12.0) + 6.0;
        let line = Color::from_rgb8(0x3a, 0x32, 0x2c);
        f.fill_rectangle(Point::new(0.0, 8.0), iced::Size::new(b.width, b.height - 8.0), Color { a: 0.25, ..Color::BLACK });
        for fr in s.start..=s.end {
            let x = x_of(fr).round() + 0.5;
            f.stroke(&Path::line(Point::new(x, 8.0), Point::new(x, b.height)), canvas::Stroke::default().with_color(Color { a: if (fr - s.start) % 4 == 0 { 0.9 } else { 0.35 }, ..line }).with_width(1.0));
        }
        let diamond = |f: &mut Frame, x: f32, y: f32, c: Color| {
            f.fill(&Path::new(|p| {
                p.move_to(Point::new(x, y - 7.0));
                p.line_to(Point::new(x + 7.0, y));
                p.line_to(Point::new(x, y + 7.0));
                p.line_to(Point::new(x - 7.0, y));
                p.close();
            }), c);
        };
        let group: Vec<&scene::Obj> = {
            fn names(c: &Coll, want: &str, inside: bool, out: &mut Vec<String>) {
                let me = inside || c.name == want;
                if me {
                    out.extend(c.objects.iter().cloned());
                }
                for ch in &c.children {
                    names(ch, want, me, out);
                }
            }
            let mut v = Vec::new();
            names(&s.root, &a.keys_of, false, &mut v);
            s.objects.iter().filter(|o| v.contains(&o.name) || o.name == a.keys_of).collect()
        };
        let mut top: Vec<i64> = group.iter().flat_map(|o| o.keys.iter().copied()).collect();
        top.sort();
        top.dedup();
        for k in top {
            diamond(&mut f, x_of(k), 22.0, a.p.accent);
        }
        if let Some(o) = a.picked_obj() {
            for k in &o.keys {
                diamond(&mut f, x_of(*k), 46.0, k::BLUE);
            }
        }
        let x = x_of(a.frame);
        f.fill_rectangle(Point::new(x - 1.0, 4.0), iced::Size::new(2.0, b.height - 4.0), a.p.accent);
        f.fill(&Path::rounded_rectangle(Point::new(x - 12.0, -2.0), iced::Size::new(24.0, 14.0), 4.0.into()), a.p.accent);
        f.fill_text(canvas::Text { content: a.frame.to_string(), position: Point::new(x - 7.0, -1.0), color: a.p.ink, size: iced::Pixels(10.0), font: iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::with_name("Inter") }, ..canvas::Text::default() });
        vec![f.into_geometry()]
    }
}

fn timeline(a: &App) -> E<'_, Msg> {
    let p = a.p;
    let s = &a.scene;
    let words = format!("{} · frame {} of {} · {} fps", if s.action.is_empty() { "No animation" } else { s.action.as_str() }, a.frame, s.end, s.fps);
    let head = row![
        k::chip(&p, "⏮", Ch::Plain, Sz::Sm, Some(Msg::Step(-1))),
        k::chip(&p, if a.playing { "⏸" } else { "▶" }, Ch::Acc, Sz::Sm, Some(Msg::Play)),
        k::chip(&p, "⏭", Ch::Plain, Sz::Sm, Some(Msg::Step(1))),
        k::mu1::<Msg>(&p, words, 11.0),
        k::fill(),
        k::chip(&p, format!("Keys: {}", if a.keys_of.is_empty() { "none" } else { a.keys_of.as_str() }), Ch::Plain, Sz::Sm, Some(Msg::Pop(Pop::Keys))),
        k::chip(&p, "＋ Key", Ch::Plain, Sz::Sm, a.picked_obj().map(|_| Msg::AddKey)),
    ]
    .spacing(7.0)
    .align_y(iced::Center);
    let track = canvas::Canvas::new(Track { a }).width(Length::Fill).height(Length::Fixed(58.0));
    container(column![head, track].spacing(8.0)).padding(k::pad(8.0, 14.0, 8.0, 14.0)).width(Length::Fill).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.2), 0.0, None)).into()
}

fn body(a: &App) -> E<'_, Msg> {
    let p = a.p;
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "studio",
        vec![("tools", rail(a)), ("viewport", viewport(a)), ("scene", scene_panel(a)), ("properties", props(a)), ("timeline", timeline(a))],
        move |ps| {
            let right = column![container(ps.take("scene")).height(Length::FillPortion(9)), ui::hline(p.line), container(ps.take("properties")).height(Length::FillPortion(10))].width(Length::Fixed(272.0));
            column![row![ps.take("tools"), ui::vline(p.line), ps.take("viewport"), ui::vline(p.line), right].height(Length::Fill), ui::hline(p.line), ps.take("timeline")].into()
        },
        Msg::Describe,
    )
}

fn pop_box(a: &App) -> Option<E<'_, Msg>> {
    let p = a.p;
    Some(match a.pop.clone()? {
        Pop::Menu(m) => {
            let opts: Vec<(String, bool, Msg)> = match m {
                "File" => vec![("Open…".into(), false, Msg::Open), ("Save".into(), false, Msg::Save), ("Import a model…".into(), false, Msg::Import), ("Send to Godot (glTF)".into(), false, Msg::Export("godot")), ("Send to Hearth CAD (STL)".into(), false, Msg::Export("cad"))],
                "Edit" => vec![(format!("Undo{}", if a.undo.is_empty() { "" } else { " the last change" }), false, Msg::Undo)],
                "Add" => ADDS.iter().enumerate().map(|(i, x)| (x.0.to_string(), false, Msg::Add(i))).collect(),
                "Object" => vec![("Duplicate".into(), false, Msg::Duplicate), ("Hide".into(), false, Msg::HidePicked), ("Delete".into(), false, Msg::Delete), ("＋ Key at this frame".into(), false, Msg::AddKey)],
                _ => vec![("Render this frame".into(), false, Msg::Render)],
            };
            k::chooser(&p, m, opts, Msg::Close)
        }
        Pop::AddChange => k::chooser(&p, "Add a change on top", CHANGES.iter().map(|(n, kd)| (n.to_string(), false, Msg::AddChange(kd))).collect(), Msg::Close),
        Pop::Keys => {
            fn all(c: &Coll, out: &mut Vec<String>) {
                for ch in &c.children {
                    out.push(ch.name.clone());
                    all(ch, out);
                }
            }
            let mut v = Vec::new();
            all(&a.scene.root, &mut v);
            v.extend(a.scene.objects.iter().filter(|o| !o.keys.is_empty()).map(|o| o.name.clone()));
            k::chooser(&p, "Keys of", v.into_iter().map(|n| (n.clone(), n == a.keys_of, Msg::KeysOf(n))).collect(), Msg::Close)
        }
        Pop::Overlays => k::chooser(
            &p,
            "Overlays",
            vec![("The floor's grid".into(), a.overlays[0], Msg::Tool(100)), ("Move's arrows".into(), a.overlays[1], Msg::Tool(101)), ("The axes".into(), a.overlays[2], Msg::Tool(102))],
            Msg::Close,
        ),
        Pop::Mod(m) => k::chooser(&p, m.clone(), vec![("Show or hide it".into(), false, Msg::ModShow(m.clone())), ("Take it off".into(), false, Msg::ModRemove(m))], Msg::Close),
    })
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mut items: Vec<(&'static str, E<'_, Msg>)> = vec![("body", body(a))];
    for (id, which) in [("pane:picked", "picked"), ("pane:keys", "keys")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let mut b = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| ps.take("body"), Msg::Describe);
    if let Some(d) = pop_box(a) {
        b = ui::dialog(b, d, Some(Msg::Close));
    }
    let menus = Row::with_children(["File", "Edit", "Add", "Object", "Render"].iter().map(|m| k::chip(&p, *m, Ch::Ghost, Sz::Sm, Some(Msg::Pop(Pop::Menu(m)))))).spacing(4.0);
    let modes = k::seg(&p, MODES.iter().enumerate().map(|(i, m)| (m.to_string(), a.mode == i, Msg::Mode(i))).collect());
    let file = a.file.as_ref().and_then(|f| f.file_name()).map(|f| format!("· {}", f.to_string_lossy())).unwrap_or_default();
    let bar = k::Bar {
        letters: "3D",
        from: k::rgb("#ffb36b"),
        to: k::rgb("#ff6a1f"),
        title: "Hearth 3D",
        line: file,
        middle: Some(row![k::gap(8.0, 1.0), menus, Space::new().width(Length::Fill), modes].spacing(6.0).align_y(iced::Center).width(Length::FillPortion(3)).into()),
        right: vec![k::chip(&p, "Send to Godot", Ch::Plain, Sz::Sm, a.file.as_ref().map(|_| Msg::Export("godot"))), k::chip(&p, "Send to Hearth CAD", Ch::Plain, Sz::Sm, a.file.as_ref().map(|_| Msg::Export("cad")))],
        msgs: TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::WClose },
        open: Msg::Describe(hearth_ui::describe::DMsg::Open),
    };
    k::window(&p, &a.describe, bar, b, ("Hearth 3D", "Built on Blender (GPL-2+)"), Msg::Resize, Msg::Describe)
}

use hearth_ui::blocks::{bl, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth 3D", blocks: &[bl("body", "The studio", 0, 0, 12, 12)] },
    PageDef {
        id: "studio",
        title: "The studio",
        blocks: &[bl("tools", "Tools", 0, 0, 1, 10), bl("viewport", "The viewport", 1, 0, 8, 10), bl("scene", "Scene", 9, 0, 3, 5), bl("properties", "The picked object", 9, 5, 3, 5), bl("timeline", "Timeline", 0, 10, 12, 2)],
    },
];

use hearth_ui::studio::{tile, Kit};
const KIT: Kit = Kit {
    name: "Hearth 3D",
    buttons: &[
        tile("open", "Open…", G::Folder),
        tile("save", "Save", G::Check),
        tile("render", "Render this frame", G::Camera),
        tile("play", "Play the animation", G::Play),
        tile("key", "＋ Key", G::Plus),
        tile("godot", "Send to Godot", G::Rocket),
        tile("cad", "Send to Hearth CAD", G::Link),
        tile("undo", "Undo", G::Update),
        tile("frame", "Frame the picked", G::Display),
    ],
    panes: &[tile("picked", "What's picked", G::List), tile("keys", "The keys", G::Clock)],
    info: &[tile("objects", "Objects", G::Grid), tile("faces", "Faces", G::Grid), tile("frame", "Frame", G::Clock), tile("picked", "Picked", G::Check)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(id: &str) -> Option<Msg> {
    Some(match id {
        "open" => Msg::Open,
        "save" => Msg::Save,
        "render" => Msg::Render,
        "play" => Msg::Play,
        "key" => Msg::AddKey,
        "godot" => Msg::Export("godot"),
        "cad" => Msg::Export("cad"),
        "undo" => Msg::Undo,
        "frame" => Msg::Tool(6),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let faces: u64 = a.scene.objects.iter().map(|o| o.faces).sum();
    a.describe.values(vec![("objects", a.scene.objects.len().to_string()), ("faces", k::commas(faces)), ("frame", a.frame.to_string()), ("picked", a.picked.join(", "))]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> E<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(4.0);
    if which == "picked" {
        for n in &a.picked {
            c = c.push(t(n.clone(), 12.0, p.text));
        }
    } else if let Some(o) = a.picked_obj() {
        for kf in &o.keys {
            c = c.push(k::click(t(format!("Frame {kf}"), 12.0, p.text), Some(Msg::Frame(*kf))));
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key)];
    // only while it plays: a frame each 1/fps s
    if a.playing {
        s.push(iced::time::every(std::time::Duration::from_millis((1000 / a.scene.fps.max(1)) as u64)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-3d", "Hearth 3D: modelling with Blender as the engine.\n  hearth-3d [FILE.blend]   open it\n  --pick NAME,NAME        pick these objects") {
        return Ok(());
    }
    let w = app::window("hearth-3d", (1280.0, 800.0), (960.0, 600.0));
    iced::application(App::new, update_all, view).title(|a: &App| format!("Hearth 3D{}", a.file.as_ref().and_then(|f| f.file_name()).map(|f| format!(" · {}", f.to_string_lossy())).unwrap_or_default())).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}

/// update, with the Overlays list's switches (Tool 100–102) taken first.
fn update_all(a: &mut App, m: Msg) -> Task<Msg> {
    if let Msg::Tool(i @ 100..=102) = m {
        a.overlays[i - 100] = !a.overlays[i - 100];
        a.gen += 1;
        return Task::none();
    }
    update(a, m)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbers_and_matrices() {
        assert_eq!(num("2.4 m"), Some(2.4));
        assert_eq!(num("35°"), Some(35.0));
        assert_eq!(trim(2.40), "2.4");
        assert_eq!(trim(0.0), "0");
        let m = trs([1.0, 2.0, 3.0], [0.0, 0.0, 90.0], [1.0, 1.0, 1.0]);
        let v = view3d::apply(&m, [1.0, 0.0, 0.0]);
        assert!((v[0] - 1.0).abs() < 1e-4 && (v[1] - 3.0).abs() < 1e-4 && (v[2] - 3.0).abs() < 1e-4, "{v:?}");
    }
}
