//! Personalization (SetAppearance.qml), My looks (SetMyLooks.qml) and Window frames & sign-in splash
//! (SetLookMore.qml). What each choice writes is subthemes.js's: picking a look sets the look and its first sub
//! theme; a sub theme only the sub theme; Match sub theme only wallMatch (and, turned on over a wallpaper you chose,
//! the wallpaper back to the sub theme's); Sunset is off unless you turn it on; From wallpaper works the colour out
//! once, when pressed. On Plasma, hearth-plasma-looks puts them on as the look's Global Theme, and what is Plasma's own
//! (the wallpaper on the desktop, the accent from the wallpaper, the splash) goes through KDE's tools too.

use super::kit::{self, dropdown, iact, ilink, irow, small, Ic};
use super::{kde, save, For, Msg, State};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, t, t1, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Task};
use serde_json::{json, Map, Value};

type E<'a> = Element<'a, Msg>;


/// The pictures a wallpaper can be (as SetAppearance's list): Hearth's, your own wallpaper file, Pictures › Wallpapers.
pub const PICTURES: &str = "ls -1 /usr/share/backgrounds/hearth/*.jpg /usr/share/backgrounds/hearth/*.png \"$HOME\"/.config/nimbus/wallpaper.jpg \"$HOME\"/.config/nimbus/wallpaper.png \"$HOME\"/Pictures/Wallpapers/*.jpg \"$HOME\"/Pictures/Wallpapers/*.jpeg \"$HOME\"/Pictures/Wallpapers/*.png \"$HOME\"/Pictures/Wallpapers/*.webp 2>/dev/null; f=$(sed -n 's/.*\"wallpaper\": *\"\\([^\"]*\\)\".*/\\1/p' \"$HOME/.local/share/nimbus/desktop.json\" 2>/dev/null); case \"$f\" in /*) [ -f \"$f\" ] && echo \"$f\";; esac";

pub fn status_cmd() -> Vec<String> {
    if !kit::on_plasma() {
        return vec!["true".into()];
    }
    let path = std::env::var("HEARTH_PLASMA_LOOKS").unwrap_or_else(|_| kit::lx("hearth-plasma-looks"));
    vec!["timeout".into(), "15".into(), "python3".into(), path, "status".into()]
}

const MOVING: &[&str] = &["", "anim:lava", "anim:aurora", "anim:tide", "anim:ridges", "anim:motes"];
const SCENES: &[&str] = &["anim:neon", "anim:zen", "anim:glass", "anim:synth"];
const BUILTINS: &[&str] = &["builtin:slate-dusk", "builtin:slate-contours", "builtin:slate-horizon", "builtin:slate-facets", "builtin:slate-nebula", "builtin:slate-ridges"];
const NAMES: &[(&str, &str)] = &[
    ("anim:neon", "Neon HUD"), ("anim:zen", "Brush Circle"), ("anim:glass", "Aurora Glass"), ("anim:synth", "Sunset Grid"), ("anim:lava", "Lava"), ("anim:aurora", "Aurora"),
    ("anim:tide", "Slate Tide"), ("anim:ridges", "Slate Ridges"), ("anim:motes", "Slate Motes"),
    ("builtin:slate-dusk", "Slate Dusk"), ("builtin:slate-contours", "Slate Contours"), ("builtin:slate-horizon", "Slate Horizon"),
    ("builtin:slate-facets", "Slate Facets"), ("builtin:slate-nebula", "Slate Nebula"), ("builtin:slate-ridges", "Slate Ridges (still)"), ("fireside", "Fireside (still)"), ("embers", "Embers"),
    ("ridge", "Dusk Ridge"), ("mark", "The Mark"), ("kiln", "Kiln"), ("cat", "Hearth Cat"), ("cabin", "Night Cabin"), ("lanterns", "Lanterns"), ("bloom", "Ember Bloom"),
    ("peaks", "Low Poly Peaks"), ("forge", "Forge Grid"), ("topo", "Cinder Topo"), ("fox", "Fox Moon"),
];
/// Plasma shows still pictures: an animated look's wallpaper is the Hearth still closest to it (hearth-plasma-looks).
const ANIM_STILL: &[(&str, &str)] = &[("tide", "ridge"), ("neon", "forge"), ("zen", "peaks"), ("glass", "bloom"), ("synth", "fox")];

fn titlecase(s: &str) -> String {
    s.split(|c| c == '-' || c == '_' || c == ' ').filter(|w| !w.is_empty()).map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() }).collect::<Vec<_>>().join(" ")
}
pub fn name_of(sh: &Shared, f: &str) -> String {
    if f.is_empty() {
        let (look, sub) = (look_label(sh), sub_label(sh));
        return format!("{}'s own", if sub_own(sh) { look } else { sub });
    }
    if let Some(n) = NAMES.iter().find(|x| x.0 == f) {
        return n.1.to_string();
    }
    let b = kit::base(f);
    let stem = b.rsplit_once('.').map(|x| x.0).unwrap_or(&b).to_string();
    let id = stem.trim_start_matches("hearth-");
    if f.contains("/backgrounds/hearth/") {
        if let Some(n) = NAMES.iter().find(|x| x.0 == id) {
            return n.1.to_string();
        }
    }
    titlecase(&stem)
}

// ---------------------------------------------------------------- looks and sub themes (subthemes.js)
pub fn cur_look(sh: &Shared) -> String {
    let l = sh.d_str("look");
    let pal = hearth_ui::theme::palette_map();
    if !l.is_empty() && pal.get(&l).and_then(|v| v.get("dark")).map(|d| d.is_object()).unwrap_or(false) { l } else { "hearth".into() }
}
pub fn look_label(sh: &Shared) -> String {
    let pal = hearth_ui::theme::palette_map();
    let l = cur_look(sh);
    pal.get(&l).and_then(|v| v.get("label")).and_then(Value::as_str).unwrap_or(&l).to_string()
}
#[derive(Clone)]
pub struct Sub {
    pub id: String,
    pub label: String,
    pub desc: String,
    pub black: bool,
    pub wallpaper: String,
    pub colours: Vec<Color>,
}
/// The look's sub themes, in order: its own colourways, then the black set it has wallpapers for.
pub fn subs(look: &str, mode: &str) -> Vec<Sub> {
    let pal = hearth_ui::theme::palette_map();
    let Some(l) = pal.get(look).and_then(Value::as_object) else { return vec![] };
    let Some(list) = l.get("subThemes").and_then(Value::as_array) else { return vec![] };
    let mut out: Vec<Sub> = vec![];
    let base = l.get(if mode == "light" { "light" } else { "dark" }).or_else(|| l.get("dark")).cloned().unwrap_or(json!({}));
    let add = |out: &mut Vec<Sub>, s: &Value, black: bool, wall: String| {
        let id = kit::s_of(s, "id");
        if id.is_empty() || out.iter().any(|x| x.id == id) {
            return;
        }
        let c = s.get(if mode == "light" { "light" } else { "dark" }).cloned().unwrap_or(json!({}));
        let colour = |k: &str| c.get(k).and_then(Value::as_str).and_then(hearth_ui::theme::hex).or_else(|| base.get(k).and_then(Value::as_str).and_then(hearth_ui::theme::hex));
        let colours = ["ember", "flame", "third"].iter().filter_map(|k| colour(k)).collect();
        let label = { let x = kit::s_of(s, "label"); if x.is_empty() { id.clone() } else { x } };
        out.push(Sub { id, label, desc: kit::s_of(s, "desc"), black, wallpaper: wall, colours });
    };
    for s in list {
        add(&mut out, s, false, kit::s_of(s, "wallpaper"));
    }
    if out.is_empty() {
        return out;
    }
    if let (Some(bw), Some(set)) = (l.get("blackWallpapers").and_then(Value::as_object), pal.get("blackSet").and_then(Value::as_array)) {
        for b in set {
            let id = kit::s_of(b, "id");
            if let Some(w) = bw.get(&id) {
                add(&mut out, b, true, w.as_str().unwrap_or("").to_string());
            }
        }
    }
    out
}
fn cur_sub(sh: &Shared) -> Option<Sub> {
    let l = subs(&cur_look(sh), &sh.d_str("mode"));
    let id = sh.d_str("subTheme");
    l.iter().find(|s| s.id == id).cloned().or_else(|| l.first().cloned())
}
pub fn sub_own(sh: &Shared) -> bool {
    let l = subs(&cur_look(sh), &sh.d_str("mode"));
    let id = sh.d_str("subTheme");
    l.is_empty() || id.is_empty() || l.first().map(|f| f.id == id).unwrap_or(true) || !l.iter().any(|s| s.id == id)
}
pub fn sub_label(sh: &Shared) -> String {
    cur_sub(sh).map(|s| s.label).unwrap_or_else(|| "Default".into())
}
fn match_shown(sh: &Shared) -> bool {
    sh.d_bool("wallMatch", true) && sh.d_str("wallpaper").is_empty()
}

/// The picture Plasma shows for a wallpaper choice (KDE takes a picture file or a wallpaper package).
pub fn plasma_wall(sh: &Shared, chosen: &str) -> String {
    let shell = run::shell_dir();
    let res = |w: &str| -> String {
        if let Some(b) = w.strip_prefix("builtin:") {
            return shell.join("images/wallpapers").join(format!("{b}.jpg")).to_string_lossy().to_string();
        }
        if let Some(a) = w.strip_prefix("anim:") {
            let id = ANIM_STILL.iter().find(|x| x.0 == a).map(|x| x.1).unwrap_or("fireside");
            return format!("/usr/share/wallpapers/Hearth-{id}");
        }
        if w.is_empty() {
            return "/usr/share/wallpapers/Hearth-fireside".into();
        }
        w.to_string()
    };
    if !chosen.is_empty() {
        return res(chosen);
    }
    let pal = hearth_ui::theme::palette_map();
    let lw = pal.get(&cur_look(sh)).and_then(|v| v.get("wallpaper")).and_then(Value::as_str).unwrap_or("").to_string();
    if sh.d_bool("wallMatch", true) && !sub_own(sh) {
        if let Some(s) = cur_sub(sh).filter(|s| !s.wallpaper.is_empty()) {
            return res(&s.wallpaper);
        }
    }
    res(&lw)
}
fn apply_wall_cmd(path: &str) -> Vec<String> {
    vec!["timeout".into(), "20".into(), "plasma-apply-wallpaperimage".into(), path.into()]
}
fn kwrite(file: &str, group: &str, key: &str, val: Option<&str>) -> String {
    match val {
        Some(v) => format!("kwriteconfig6 --notify --file {file} --group {group} --key {key} {}", crate::data::q(v)),
        None => format!("kwriteconfig6 --notify --file {file} --group {group} --key {key} --delete"),
    }
}

// ---------------------------------------------------------------- changes
fn choose_accent(sh: &mut Shared, st: &mut State, c: String) -> Task<Msg> {
    let from_wall = sh.d_str("accentFrom") == "wallpaper";
    let mut t = vec![save(sh, json!({"accentFrom": "", "accent": c}))];
    if kit::on_plasma() && from_wall {
        t.push(kde(kit::sh_t(&kwrite("kdeglobals", "General", "accentColorFromWallpaper", Some("false")), 10)));
    }
    if sh.d_bool("kbdLights", false) {
        let hx = if c.is_empty() { kit::hex_of(sh.p.accent) } else { c.clone() };
        t.push(kde(kit::job("hearth-kbd-lights", &["apply", &hx])));
    }
    st.hue = None;
    Task::batch(t)
}
fn choose_wallpaper(sh: &mut Shared, f: String) -> Task<Msg> {
    let mut t = vec![save(sh, json!({"wallpaper": f}))];
    if kit::on_plasma() {
        let path = plasma_wall(sh, &f);
        t.push(kde(apply_wall_cmd(&path)));
    }
    Task::batch(t)
}
/// From wallpaper: worked out once, now. On Plasma it's KDE's own (Colors › accent from the wallpaper).
fn from_wallpaper(sh: &mut Shared, st: &mut State) -> Task<Msg> {
    if kit::on_plasma() {
        let t = vec![save(sh, json!({"accentFrom": "wallpaper", "accent": ""})), kde(kit::sh_t(&kwrite("kdeglobals", "General", "accentColorFromWallpaper", Some("true")), 10))];
        return Task::batch(t);
    }
    let w = { let x = sh.d_str("wallpaper"); if x.is_empty() { hearth_ui::theme::palette_map().get(&cur_look(sh)).and_then(|v| v.get("wallpaper")).and_then(Value::as_str).unwrap_or("").to_string() } else { x } };
    let fixed: &[(&str, [&str; 4])] = &[("", ["#ff8a3d", "#ffd166", "#c2410c", "#7c2d12"]), ("anim:lava", ["#ff5a36", "#ffb347", "#b3264a", "#5a1830"]), ("anim:aurora", ["#ff9d5c", "#ffd9a0", "#8a3a2c", "#2a1410"]), ("anim:tide", ["#1a9fff", "#66c0f4", "#2a475e", "#171a21"]), ("anim:ridges", ["#4f9fe0", "#8fc4ef", "#2a475e", "#1b2838"]), ("anim:motes", ["#66c0f4", "#1a9fff", "#32404f", "#171a21"])];
    match fixed.iter().find(|x| x.0 == w) {
        Some((_, sw)) => save(sh, json!({"wallSwatches": sw, "accentFrom": "wallpaper", "accent": sw[0]})),
        None => {
            st.msg = "Couldn't read colours from this wallpaper here".into();
            st.msg_bad = true;
            Task::none()
        }
    }
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Look(l) => return save(sh, json!({"look": l, "subTheme": ""})),
        Msg::Sub(id) => {
            st.pick = None;
            let mut t = vec![save(sh, json!({"subTheme": id}))];
            if kit::on_plasma() && sh.d_str("wallpaper").is_empty() && sh.d_bool("wallMatch", true) {
                let path = plasma_wall(sh, "");
                t.push(kde(apply_wall_cmd(&path)));
            }
            return Task::batch(t);
        }
        Msg::Mode(md) => {
            let mut t = vec![if md == "sunset" { save(sh, json!({"sunset": true})) } else { save(sh, json!({"sunset": false, "mode": md})) }];
            // A light and a dark wallpaper (Wallpaper per screen…): the one for this mode, on Plasma.
            let pair = kit::obj(sh, "wallPair");
            if kit::on_plasma() && md != "sunset" && pair.get("on").and_then(Value::as_bool).unwrap_or(false) {
                if let Some(f) = pair.get(md).and_then(Value::as_str).filter(|f| !f.is_empty()) {
                    t.push(kde(apply_wall_cmd(f)));
                }
            }
            return Task::batch(t);
        }
        Msg::Shift(key, d) => {
            let cur = sh.d_str(key);
            let (h, mi) = cur.split_once(':').and_then(|(a, b)| Some((a.parse::<i32>().ok()?, b.parse::<i32>().ok()?))).unwrap_or(if key == "sunsetLight" { (7, 0) } else { (19, 30) });
            let tt = (h * 60 + mi + d * 30 + 1440) % 1440;
            return save(sh, json!({ key: format!("{:02}:{:02}", tt / 60, tt % 60) }));
        }
        Msg::Match(on) => {
            let chosen = sh.d_str("wallpaper");
            let ch = if !on { json!({"wallMatch": false}) } else if !chosen.is_empty() { json!({"wallMatch": true, "wallpaper": ""}) } else { json!({"wallMatch": true}) };
            let mut t = vec![save(sh, ch)];
            if kit::on_plasma() && sh.d_str("wallpaper").is_empty() {
                let path = plasma_wall(sh, "");
                t.push(kde(apply_wall_cmd(&path)));
            }
            return Task::batch(t);
        }
        Msg::Wall(f) => {
            let t = choose_wallpaper(sh, f);
            if sh.d_str("accentFrom") == "wallpaper" && !kit::on_plasma() {
                return Task::batch([t, from_wallpaper(sh, st)]);
            }
            return t;
        }
        Msg::Choose(For::Wall) => {
            if !st.picking {
                st.picking = true;
                return Task::perform(kit::run_in(kit::file_picker("Choose a wallpaper", false, "image/jpeg image/png image/webp"), String::new()), |o| Msg::Picked(For::Wall, o));
            }
        }
        Msg::Picked(For::Wall, o) => {
            st.picking = false;
            if o.code == 3 {
                st.msg = "No file picker installed: put pictures in Pictures › Wallpapers instead".into();
                st.msg_bad = true;
            } else {
                let f = o.out.trim().to_string();
                if !f.is_empty() {
                    if !st.pictures.contains(&f) {
                        st.pictures.push(f.clone());
                    }
                    return update(sh, st, Msg::Wall(f));
                }
            }
        }
        Msg::Pictures(o) => {
            let mut v: Vec<String> = vec![];
            for l in o.out.lines().map(str::trim).filter(|l| !l.is_empty()) {
                if !v.iter().any(|x| x == l) {
                    v.push(l.to_string());
                }
            }
            st.pictures = v;
        }
        Msg::WallOpen => st.wall_open = !st.wall_open,
        Msg::AccentMore => st.accent_more = !st.accent_more,
        Msg::Accent(c) => return choose_accent(sh, st, c),
        Msg::Hex(s) => st.hex = s,
        Msg::HexGo => {
            let v = st.hex.trim().trim_start_matches('#').to_string();
            if v.len() == 6 && v.chars().all(|c| c.is_ascii_hexdigit()) {
                st.hex.clear();
                return choose_accent(sh, st, format!("#{}", v.to_lowercase()));
            }
            st.msg = "A colour looks like #1a9fff".into();
            st.msg_bad = true;
        }
        Msg::Hue(h) => st.hue = Some(h),
        Msg::HueSet => {
            if let Some(h) = st.hue {
                let c = kit::hex_of(kit::hsv(h.min(0.999) * 360.0, 0.85, 1.0));
                return choose_accent(sh, st, c);
            }
        }
        Msg::FromWall => return from_wallpaper(sh, st),
        Msg::Kbd(on) => {
            let mut t = vec![save(sh, json!({"kbdLights": on}))];
            if on {
                let hx = { let a = sh.d_str("accent"); if a.is_empty() { kit::hex_of(sh.p.accent) } else { a } };
                t.push(kde(kit::job("hearth-kbd-lights", &["apply", &hx])));
            }
            return Task::batch(t);
        }
        Msg::KbdSt(o) => st.kbd = kit::answer(&o).ok().filter(|v| v.is_object()),
        Msg::LooksSt(o) => st.looks_st = kit::answer(&o).ok().filter(|v| v.is_object()),
        Msg::UseHearthLook => {
            let look = cur_look(sh);
            let md = if sh.d_str("mode") == "light" { "Light" } else { "Dark" };
            let sub = if sub_own(sh) { String::new() } else { format!("-{}", sh.d_str("subTheme")) };
            let script = format!("plasma-apply-lookandfeel -a org.hearthos.{look} && plasma-apply-colorscheme HearthOS-{look}{sub}-{md}");
            return kde(kit::sh_t(&script, 60)).chain(Task::perform(kit::run_in(status_cmd(), String::new()), Msg::LooksSt));
        }
        Msg::Kde(kcm) => run::detached(&["systemsettings", kcm]),
        Msg::DeskClock(v) => return super::desk::desk_clock(sh, v),
        Msg::Glass(v) => return super::desk::glass(sh, st, v),
        // ---- My looks
        Msg::MyFonts(o) => st.my_fonts = kit::answer(&o).ok().and_then(|v| v.get("saved").cloned()).or(Some(json!({}))),
        Msg::MyName(s) => st.my_name = s,
        Msg::MySave => {
            let name = st.my_name.trim().to_string();
            if name.is_empty() {
                st.msg = "Give it a name first".into();
                st.msg_bad = true;
                return Task::none();
            }
            let mut d = Map::new();
            for k in MY_KEYS {
                if let Some(v) = sh.desktop.get(*k) {
                    d.insert(k.to_string(), v.clone());
                }
            }
            let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
            let id = format!("look:{}", slug.chars().take(40).collect::<String>());
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            let entry = json!({ "name": name.chars().take(60).collect::<String>(), "saved": now, "desktop": d, "fonts": st.my_fonts.clone().unwrap_or(json!({})), "accentShown": kit::hex_of(sh.p.accent), "lookLabel": look_label(sh) });
            let cmd = sh.json_cmd(&my_path(), json!({ id: entry }));
            st.msg = format!("Saved as {name}.");
            st.msg_bad = false;
            st.my_name.clear();
            return Task::perform(run::spawn(cmd), Msg::MyDid);
        }
        Msg::MyUse(id) => {
            let Some(l) = st.my.iter().find(|x| kit::s_of(x, "id") == id).cloned() else { return Task::none() };
            let mut ch = Map::new();
            if let Some(d) = l.get("desktop").and_then(Value::as_object) {
                for k in MY_KEYS {
                    if let Some(v) = d.get(*k) {
                        ch.insert(k.to_string(), v.clone());
                    }
                }
            }
            // Fonts, cursor and icons: what that look had; anything it didn't set goes back to Hearth's default.
            let mut f = Map::new();
            for k in ["font", "fontSize", "mono", "monoSize", "antialias", "hinting", "dpi", "cursor", "cursorSize", "icons", "smallFont", "smallSize", "titleFont", "titleSize"] {
                f.insert(k.into(), l.get("fonts").and_then(|x| x.get(k)).cloned().unwrap_or(Value::Null));
            }
            st.msg = format!("{} is back.", kit::s_of(&l, "name"));
            st.msg_bad = false;
            let mut t = vec![save(sh, Value::Object(ch)), Task::perform(kit::run_in(kit::job("hearth-lookset", &["set"]), Value::Object(f).to_string()), Msg::MyDid)];
            if kit::on_plasma() {
                let w = l.get("desktop").and_then(|d| d.get("wallpaper")).and_then(Value::as_str).unwrap_or("").to_string();
                t.push(kde(apply_wall_cmd(&plasma_wall(sh, &w))));
            }
            return Task::batch(t);
        }
        Msg::MyDelete(id) => {
            let name = st.my.iter().find(|x| kit::s_of(x, "id") == id).map(|x| kit::s_of(x, "name")).unwrap_or_default();
            let cmd = sh.json_cmd(&my_path(), json!({ id: null }));
            st.msg = format!("{name} deleted.");
            st.msg_bad = false;
            return Task::perform(run::spawn(cmd), Msg::MyDid);
        }
        Msg::MyDid(o) => {
            if let Err(e) = kit::answer(&o) {
                st.msg = e;
                st.msg_bad = true;
            }
            st.my = my_looks();
        }
        // ---- Window frames & sign-in splash
        Msg::Tint(v) => {
            st.tint = Some((v * 20.0).round() * 5.0);
            if !st.tint_due {
                st.tint_due = true;
                return Task::perform(run::after(400), |_| Msg::TintFlush);
            }
        }
        Msg::TintFlush => {
            st.tint_due = false;
            if let Some(t) = st.tint {
                return save(sh, json!({"frameTint": t as i64}));
            }
        }
        Msg::AppsFollow(on) => return save(sh, json!({"appsFollow": on})),
        Msg::Splash(v) => {
            let mut t = vec![save(sh, json!({"signinSplash": v}))];
            if kit::on_plasma() {
                // KDE's splash after signing in (ksplashrc): none, or the look's (its colours, Hearth's logo).
                let script = if v.is_empty() { kwrite("ksplashrc", "KSplash", "Engine", Some("none")) } else { format!("{} && {}", kwrite("ksplashrc", "KSplash", "Engine", Some("KSplashQML")), kwrite("ksplashrc", "KSplash", "Theme", Some(&format!("org.hearthos.{}", cur_look(sh))))) };
                t.push(kde(kit::sh_t(&script, 10)));
            }
            return Task::batch(t);
        }
        Msg::SplashNow => {
            if kit::on_plasma() {
                let lnf = format!("org.hearthos.{}", cur_look(sh));
                run::detached(&["ksplashqml", &lnf, "--test"]);
            } else {
                let v = sh.d_str("signinSplash");
                let dir = run::shell_dir().to_string_lossy().to_string();
                run::detached(&["quickshell", "ipc", "-p", &dir, "call", "splash", "preview", &v]);
            }
        }
        _ => {}
    }
    Task::none()
}

// ---------------------------------------------------------------- Personalization
pub fn appearance(a: &App) -> E<'_> {
    let sh = &a.sh;
    let st = &a.pers;
    let p = sh.p;
    let pal = hearth_ui::theme::palette_map();
    let plasma = kit::on_plasma();
    let cur_look = cur_look(sh);
    let cur_mode = sh.d_str("mode");
    let cur_accent = sh.d_str("accent").to_lowercase();
    let mut c = kit::Col::new(st.skip);
    c = c.push(t("Hearth's look for the whole desktop: KDE's Global Theme, its colours and Hearth's apps all follow it. A Global Theme you pick in KDE's System Settings shows here too.", sz(14.0), p.muted));
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.msg_bad));
    }
    // KDE's Global Theme now (hearth-plasma-looks status): Hearth's look, or one picked in KDE.
    if plasma {
        let lnf = st.looks_st.as_ref().and_then(|s| s.get("kde")).map(|k| kit::s_of(k, "lnf")).unwrap_or_default();
        let theirs = !lnf.is_empty() && !lnf.starts_with("org.hearthos.");
        let shown = if lnf.is_empty() { "Reading…".to_string() } else if let Some(l) = lnf.strip_prefix("org.hearthos.") { format!("Hearth OS · {}", pal.get(l).and_then(|v| v.get("label")).and_then(Value::as_str).unwrap_or(l)) } else { lnf.rsplit('.').next().unwrap_or(&lnf).to_string() };
        let mut r = Row::new().spacing(10.0).align_y(iced::Center);
        if theirs {
            r = r.push(btn(&p, "Use Hearth's look", ui::Kind::Primary, false, Some(Msg::UseHearthLook)));
        }
        r = r.push(btn(&p, "Global Themes in KDE", ui::Kind::Normal, false, Some(Msg::Kde("kcm_lookandfeel"))));
        c = c.push(irow(&p, Ic::G(G::Palette), format!("KDE's Global Theme: {shown}"), if theirs { "Picked in KDE's System Settings: Hearth's apps follow it. Pick a look below to go back to Hearth's" } else { "Hearth's look, as a Global Theme: KDE's panel, window frames, colours and splash follow it" }, r.into()));
    }
    c = c.push(head(&p, "Look"));
    let mut looks: Vec<(String, String, i64, Map<String, Value>)> = pal
        .iter()
        .filter(|(k, v)| !k.starts_with('_') && v.get("dark").map(|d| d.is_object()).unwrap_or(false))
        .map(|(k, v)| (k.clone(), v.get("label").and_then(|l| l.as_str()).unwrap_or(k).to_string(), v.get("order").and_then(|o| o.as_i64()).unwrap_or(99), v.get("dark").and_then(|d| d.as_object()).cloned().unwrap_or_default()))
        .collect();
    looks.sort_by_key(|l| l.2);
    let mut grid = Column::new().spacing(14.0);
    let mut line = Row::new().spacing(14.0);
    for (n, (id, label, _, dark)) in looks.iter().enumerate() {
        let col = |k: &str, d: &str| dark.get(k).and_then(|v| v.as_str()).and_then(hearth_ui::theme::hex).unwrap_or_else(|| hearth_ui::theme::hex(d).unwrap());
        let (bg, panel, accent, text) = (col("bg", "#111111"), col("panel2", "#222222"), col("ember", "#ff8a3d"), col("text", "#eeeeee"));
        let on = *id == cur_look;
        let preview = container(column![
            container(Space::new()).width(Length::Fixed(110.0)).height(Length::Fixed(6.0)).style(ui::boxed(text, 3.0, None)),
            container(Space::new()).width(Length::Fixed(80.0)).height(Length::Fixed(5.0)).style(ui::boxed(Pal::a(text, 0.5), 3.0, None)),
            row![container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(14.0)).style(ui::boxed(accent, 3.0, None)), container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(14.0)).style(ui::boxed(Pal::mix(panel, text, 0.1), 3.0, None))].spacing(6.0)
        ].spacing(8.0))
        .padding(16.0)
        .width(Length::Fill)
        .height(Length::Fixed(sz(96.0)))
        .style(ui::boxed(bg, p.r(8.0), Some((if on { p.accent } else { Pal::mix(bg, text, 0.15) }, if on { 3.0 } else { 1.0 }))));
        let n_sub = subs(id, &cur_mode).len();
        let name = if on { ts(label.clone(), sz(14.0), p.text) } else { t(label.clone(), sz(14.0), p.text) };
        let under = column![container(name).center_x(Length::Fill), container(t1(if n_sub > 0 { format!("{n_sub} sub themes") } else { String::new() }, sz(11.0), p.muted)).center_x(Length::Fill)].spacing(1.0);
        let tile = mouse_area(column![preview, under].spacing(8.0)).on_press(Msg::Look(id.clone())).interaction(mouse::Interaction::Pointer);
        line = line.push(container(tile).width(Length::FillPortion(1)));
        if n % 4 == 3 {
            grid = grid.push(line);
            line = Row::new().spacing(14.0);
        }
    }
    let rest = looks.len() % 4;
    if rest != 0 {
        for _ in rest..4 {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    c = c.push(card(&p, grid));
    // The sub theme: the look's colourways, each with its own wallpaper.
    let sl = subs(&cur_look, &cur_mode);
    let sub_dots: Row<'_, Msg> = cur_sub(sh).map(|s| s.colours.iter().fold(Row::new().spacing(4.0), |r, c| r.push(container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(*c, 6.0, None))))).unwrap_or_else(|| Row::new());
    c = c.push(irow(&p, Ic::G(G::Palette), "Sub theme", if sl.is_empty() { format!("{} has one set of colours", look_label(sh)) } else { format!("The colours of {}, each with its own wallpaper", look_label(sh)) }, row![sub_dots, dropdown(&p, if sl.is_empty() { "Default".to_string() } else { sub_label(sh) }, if sl.is_empty() { None } else { Some(Msg::Open("sub")) })].spacing(12.0).align_y(iced::Center).into()));

    c = c.push(head(&p, "Your style"));
    let sunset = sh.d_bool("sunset", false);
    let mode_v = if sunset { "sunset" } else if cur_mode == "light" { "light" } else { "dark" };
    c = c.push(irow(&p, Ic::G(G::Moon), "Mode", "Auto (optional): light by day and dark by night, on the times below. Off unless you choose it.", seg(&p, vec![
        ("Dark".into(), mode_v == "dark", Some(Msg::Mode("dark"))),
        ("Light".into(), mode_v == "light", Some(Msg::Mode("light"))),
        ("Auto".into(), mode_v == "sunset", Some(Msg::Mode("sunset"))),
    ])));
    if sunset {
        for (key, title, d) in [("sunsetLight", "Light from", "07:00"), ("sunsetDark", "Dark from", "19:30")] {
            let v = { let x = sh.d_str(key); if x.is_empty() { d.to_string() } else { x } };
            c = c.push(kit::indent(irow(&p, Ic::G(G::Clock), title, "", row![btn(&p, "−", ui::Kind::Normal, false, Some(Msg::Shift(key, -1))), container(ts(v, sz(15.0), p.text)).width(Length::Fixed(sz(56.0))).center_x(Length::Fixed(sz(56.0))), btn(&p, "+", ui::Kind::Normal, false, Some(Msg::Shift(key, 1)))].spacing(8.0).align_y(iced::Center).into())));
        }
        if plasma {
            c = c.push(small(&p, "KDE Plasma changes between light and dark at these times only while Hearth's desktop helper runs them (Mantel); on Plasma KDE's own night-and-day switching is in Global Themes."));
        }
    }
    let mine = !sh.d_str("wallpaper").is_empty();
    let own_name = if sub_own(sh) { look_label(sh) } else { sub_label(sh) };
    let msub = if !mine && sh.d_bool("wallMatch", true) { format!("The wallpaper is {own_name}'s own. Pick your own any time; it stays when you change sub theme.") } else if mine { format!("You picked your own wallpaper ({}). Turn this on to use the sub theme's instead.", name_of(sh, &sh.d_str("wallpaper"))) } else { "Off: the look's own wallpaper, whatever the sub theme".to_string() };
    c = c.push(irow(&p, Ic::Image, "Match sub theme", msub, switch(&p, match_shown(sh), Some(Msg::Match(!match_shown(sh))))));
    c = c.push(ilink(&p, Ic::Image, "Wallpaper", format!("{} · pictures in Pictures › Wallpapers show up here too", name_of(sh, &sh.d_str("wallpaper"))), if st.wall_open { "Close" } else { "" }, Msg::WallOpen));
    if st.wall_open {
        let mut list: Vec<String> = vec![];
        if !plasma {
            list.extend(MOVING.iter().map(|s| s.to_string()));
            list.extend(SCENES.iter().map(|s| s.to_string()));
        } else {
            list.push(String::new());
        }
        list.extend(BUILTINS.iter().map(|s| s.to_string()));
        list.extend(st.pictures.iter().cloned());
        let chosen = sh.d_str("wallpaper");
        let mut g = Column::new().spacing(10.0);
        let mut r = Row::new().spacing(10.0);
        for (i, f) in list.iter().enumerate() {
            let lit = *f == chosen;
            let bgc = kit::name_colour(&p, if f.is_empty() { &cur_look } else { f });
            let anim = MOVING.contains(&f.as_str()) && !f.is_empty();
            let lbl = format!("{}{}", name_of(sh, f), if anim { "  ·  animated" } else { "" });
            let tile = mouse_area(column![kit::swatch(bgc, p.accent, Color::WHITE, sz(190.0), sz(96.0), if lit { Some(p.accent) } else { None }), row![t1(ui::fit(&lbl, 26), sz(12.0), p.text), Space::new().width(Length::Fill), if lit { E::from(ui::glyph(G::Check, p.accent, 14.0, 2.4)) } else { Space::new().width(Length::Fixed(0.0)).into() }].align_y(iced::Center)].spacing(6.0).width(Length::Fixed(sz(190.0))))
                .on_press(Msg::Wall(f.clone()))
                .interaction(mouse::Interaction::Pointer);
            r = r.push(tile);
            if i % 4 == 3 {
                g = g.push(r);
                r = Row::new().spacing(10.0);
            }
        }
        g = g.push(r);
        let mut inner = column![g].spacing(10.0);
        if plasma {
            inner = inner.push(small(&p, "KDE Plasma shows still pictures: an animated wallpaper is the Hearth picture closest to it."));
        }
        inner = inner.push(iact(&p, Ic::Image, if st.picking { "Choosing…" } else { "Choose a picture…" }, "Any picture on this device. In the Files app you can also right-click a picture › Set as Hearth wallpaper.", false, Some(Msg::Choose(For::Wall))));
        c = c.push(kit::indent(inner));
    }
    // The accent: the look's own ("A"), then the presets.
    let accents: Vec<(String, String)> = pal.get("accents").and_then(|v| v.as_array()).map(|l| l.iter().filter_map(|a| Some((kit::s_of(a, "color").to_lowercase(), kit::s_of(a, "name")))).filter(|x| !x.0.is_empty()).collect()).unwrap_or_default();
    let from_wall = sh.d_str("accentFrom") == "wallpaper";
    let accent_sub = if cur_accent.is_empty() && from_wall && plasma {
        "From your wallpaper (KDE works it out)".to_string()
    } else if cur_accent.is_empty() {
        if sub_own(sh) { "The look's own".to_string() } else { format!("From the sub theme ({})", sub_label(sh)) }
    } else if from_wall {
        format!("From wallpaper {cur_accent}")
    } else {
        accents.iter().find(|a| a.0 == cur_accent).map(|a| a.1.clone()).unwrap_or_else(|| format!("Custom {cur_accent}"))
    };
    let own_on = cur_accent.is_empty();
    let mut sw = Row::new().spacing(8.0).align_y(iced::Center);
    let s = sz(28.0);
    sw = sw.push(mouse_area(container(ts("A", sz(12.0), p.ink)).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(p.accent, s / 2.0, Some((if own_on { p.text } else { p.line }, if own_on { 2.0 } else { 1.0 }))))).on_press(Msg::Accent(String::new())).interaction(mouse::Interaction::Pointer));
    for (hex, _) in &accents {
        let Some(cc) = hearth_ui::theme::hex(hex) else { continue };
        let on = cur_accent == *hex;
        sw = sw.push(mouse_area(container(if on { E::from(ui::glyph(G::Check, Color::WHITE, 13.0, 2.6)) } else { Space::new().into() }).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(cc, s / 2.0, Some((if on { p.text } else { p.line }, if on { 2.0 } else { 1.0 }))))).on_press(Msg::Accent(hex.clone())).interaction(mouse::Interaction::Pointer));
    }
    if !cur_accent.is_empty() && !accents.iter().any(|a| a.0 == cur_accent) {
        if let Some(cc) = hearth_ui::theme::hex(&cur_accent) {
            sw = sw.push(container(Space::new()).width(Length::Fixed(s)).height(Length::Fixed(s)).style(ui::boxed(cc, s / 2.0, Some((p.text, 2.0)))));
        }
    }
    c = c.push(irow(&p, Ic::G(G::Palette), "Accent colour", format!("{accent_sub} · focus rings, selections and highlights, in apps too"), sw.into()));
    let more_val = if from_wall { "From wallpaper".to_string() } else if !cur_accent.is_empty() && !accents.iter().any(|a| a.0 == cur_accent) { cur_accent.clone() } else { String::new() };
    c = c.push(ilink(&p, Ic::Edit, "More accent colours", "From your wallpaper, any hue, or a colour you type", more_val, Msg::AccentMore));
    if st.accent_more {
        let mut inner = Column::new().spacing(8.0);
        let fw_sub = if from_wall { "Worked out from your wallpaper when you picked it. It doesn't change by itself.".to_string() } else { "Take the accent from the wallpaper (once, now)".to_string() };
        let mut fw = Row::new().spacing(6.0).align_y(iced::Center);
        if from_wall {
            for sw in kit::strs(&Value::Object(sh.desktop.clone()), "wallSwatches") {
                if let Some(cc) = hearth_ui::theme::hex(&sw) {
                    let lit = cur_accent == sw.to_lowercase();
                    fw = fw.push(mouse_area(container(Space::new()).width(Length::Fixed(26.0)).height(Length::Fixed(26.0)).style(ui::boxed(cc, 13.0, Some((if lit { p.text } else { p.line }, if lit { 2.0 } else { 1.0 }))))).on_press(Msg::Set(json!({"accent": sw}))).interaction(mouse::Interaction::Pointer));
                }
            }
        }
        fw = fw.push(btn(&p, if from_wall { "Again" } else { "From wallpaper" }, ui::Kind::Normal, false, Some(Msg::FromWall)));
        inner = inner.push(irow(&p, Ic::Image, "From wallpaper", fw_sub, fw.into()));
        let hue = st.hue.unwrap_or_else(|| if hearth_ui::theme::hex(&cur_accent).is_some() { kit::hue_of(&cur_accent) } else { 0.58 });
        let hv = if let Some(h) = st.hue { kit::hex_of(kit::hsv(h.min(0.999) * 360.0, 0.85, 1.0)) } else if hearth_ui::theme::hex(&cur_accent).is_some() { cur_accent.clone() } else { "Pick a hue".into() };
        inner = inner.push(irow(&p, Ic::G(G::Palette), "Custom accent", "", row![iced::widget::slider(0.0..=1.0, hue, Msg::Hue).step(1.0f32 / 36.0).on_release(Msg::HueSet).width(Length::Fixed(sz(240.0))), container(t1(hv, sz(13.0), p.muted)).width(Length::Fixed(sz(80.0)))].spacing(12.0).align_y(iced::Center).into()));
        inner = inner.push(field(&p, "Or type a colour, like #1a9fff, then Enter", &st.hex, Msg::Hex).on_submit(Msg::HexGo));
        c = c.push(kit::indent(inner));
    }
    // 1.14: the keyboard's RGB lights follow the accent. Opening the page only asks hearth-kbd-lights which way.
    let kb = st.kbd.as_ref();
    let way = kb.map(|k| kit::s_of(k, "way")).unwrap_or_default();
    let kb_sub = match (kb, way.as_str()) {
        (None, _) => "Colours an RGB keyboard with your accent colour".to_string(),
        (_, "openrgb") => "Through OpenRGB: keyboards only, your mouse and the rest stay as they are".into(),
        (Some(k), "kernel") => format!("This keyboard's own coloured light{}", if kit::arr(k, "leds").iter().any(|l| !kit::b_of(l, "writable")) { " (it needs Hearth's backlight permission: restart once after the update)" } else { "" }),
        _ => "No coloured keyboard found here (OpenRGB, or a laptop keyboard with coloured lights)".into(),
    };
    let kbd = sh.d_bool("kbdLights", false);
    c = c.push(irow(&p, Ic::Kbd, "Keyboard lights follow the accent", kb_sub, switch(&p, kbd, Some(Msg::Kbd(!kbd)))));

    c = c.push(head(&p, "Desktop"));
    c = c.push(ilink(&p, Ic::Window, "Bar and dock", if plasma { "KDE's panel: where it goes, auto-hide, floating, the app menu" } else { "Where the bar goes, the dock, auto-hide, the app menu and the right-click menu" }, super::bar_value(sh), Msg::Go("bar-dock")));
    let dc = sh.d_str("deskClock");
    c = c.push(irow(&p, Ic::G(G::Clock), "Desktop clock and weather", if plasma { "KDE's clock and weather widgets on the desktop" } else { "" }, seg(&p, vec![("Look's own".into(), dc.is_empty(), Some(Msg::DeskClock(String::new()))), ("On".into(), dc == "on", Some(Msg::DeskClock("on".into()))), ("Off".into(), dc == "off", Some(Msg::DeskClock("off".into())))])));
    let pg = sh.d_str("panelGlass");
    c = c.push(irow(&p, Ic::Eye, "Panel transparency", if plasma { "KDE's panel: adaptive, solid or see-through" } else { "" }, seg(&p, vec![("Look's own".into(), pg.is_empty(), Some(Msg::Glass(String::new()))), ("Solid".into(), pg == "solid", Some(Msg::Glass("solid".into()))), ("Frosted".into(), pg == "frosted", Some(Msg::Glass("frosted".into()))), ("Clear".into(), pg == "clear", Some(Msg::Glass("clear".into())))])));
    c = c.push(ilink(&p, Ic::Kbd, "On-screen keyboard style", "Its looks (Ember, Frost, Steam, Warm…) are in Bluetooth & devices › Keyboard", "", Msg::Go("keyboard")));
    c.into()
}

pub fn dialog<'a>(a: &'a App, id: &str) -> Option<E<'a>> {
    let sh = &a.sh;
    let p = sh.p;
    match id {
        "sub" => {
            let cur = if sub_own(sh) { String::new() } else { sh.d_str("subTheme") };
            let l = subs(&cur_look(sh), &sh.d_str("mode"));
            let items = l.iter().enumerate().map(|(i, s)| {
                let v = if i == 0 { String::new() } else { s.id.clone() };
                let on = v == cur;
                (format!("{}{}", s.label, if s.black && (i == 0 || !l[i - 1].black) { "  · Black" } else { "" }), s.desc.clone(), on, Msg::Sub(v))
            }).collect();
            Some(kit::pick_dialog(&p, &format!("{}'s sub themes", look_label(sh)), items, Msg::Close))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------- My looks
pub const MY_KEYS: &[&str] = &["look", "mode", "accent", "subTheme", "wallpaper", "wallMatch", "barStyle", "barLayout", "barEdge", "dockMode", "dockFloat", "panelGlass", "appMenu", "menuStyle", "deskClock", "frameTint", "signinSplash", "wallScreens", "wallShow"];

pub fn my_path() -> std::path::PathBuf {
    hearth_ui::paths::home().join(".config/nimbus/my-looks.json")
}
pub fn my_looks() -> Vec<Value> {
    let mut v: Vec<Value> = match hearth_ui::settings_file::read_json(&my_path()) {
        Value::Object(m) => m.into_iter().filter(|(k, x)| k.starts_with("look:") && x.is_object()).map(|(k, mut x)| { x["id"] = json!(k); x }).collect(),
        _ => vec![],
    };
    v.sort_by(|a, b| kit::n_of(b, "saved", 0.0).partial_cmp(&kit::n_of(a, "saved", 0.0)).unwrap_or(std::cmp::Ordering::Equal));
    v
}

pub fn my_looks_page(a: &App) -> E<'_> {
    let sh = &a.sh;
    let st = &a.pers;
    let p = sh.p;
    let mut c = kit::Col::new(st.skip);
    c = c.push(head(&p, "Save how it looks now"));
    c = c.push(row![field(&p, "A name, like Cosy evening", &st.my_name, Msg::MyName).on_submit(Msg::MySave), btn(&p, "Save as my look", ui::Kind::Primary, false, Some(Msg::MySave))].spacing(10.0).align_y(iced::Center));
    c = c.push(small(&p, format!("Keeps {}, {} mode, the accent, the wallpaper, the bar and dock, the menus, the window frames and your fonts, cursor and icons.", look_label(sh), if sh.d_str("mode") == "light" { "light" } else { "dark" })));
    c = c.push(head(&p, if st.my.is_empty() { "No looks saved yet" } else { "Your looks" }));
    let mut g = Column::new().spacing(12.0);
    let mut r = Row::new().spacing(12.0);
    for (i, l) in st.my.iter().enumerate() {
        let d = l.get("desktop").cloned().unwrap_or(json!({}));
        let light = kit::s_of(&d, "mode") == "light";
        let accent = hearth_ui::theme::hex(&kit::s_of(l, "accentShown")).unwrap_or(p.accent);
        let bg = hearth_ui::theme::hex(if light { "#e9ecef" } else { "#16191d" }).unwrap();
        let win = hearth_ui::theme::hex(if light { "#fafafa" } else { "#23272d" }).unwrap();
        let tint = (kit::n_of(&d, "frameTint", 0.0) / 30.0).max(1.0) as f32;
        let pic = container(column![
            container(container(Space::new()).width(Length::Fixed(sz(110.0))).height(Length::Fixed(sz(46.0))).style(ui::boxed(win, 5.0, Some((accent, tint))))).padding(Padding { top: 14.0, right: 0.0, bottom: 0.0, left: 70.0 }),
            Space::new().height(Length::Fill),
            container(row![container(Space::new()).width(Length::Fixed(8.0)).height(Length::Fixed(8.0)).style(ui::boxed(accent, 4.0, None))]).padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 }).width(Length::Fill).style(ui::boxed(if light { Pal::a(Color::WHITE, 0.8) } else { Pal::a(Color::BLACK, 0.67) }, 7.0, None)),
        ])
        .padding(Padding { top: 0.0, right: 10.0, bottom: 8.0, left: 10.0 })
        .width(Length::Fill)
        .height(Length::Fixed(sz(96.0)))
        .style(ui::boxed(bg, 8.0, None));
        let id = kit::s_of(l, "id");
        let label = { let x = kit::s_of(l, "lookLabel"); if x.is_empty() { kit::s_of(&d, "look") } else { x } };
        let tile = container(column![
            pic,
            ts(ui::fit(&kit::s_of(l, "name"), 30), sz(15.0), p.text),
            t(format!("{label} · {}", if light { "Light" } else { "Dark" }), sz(12.0), p.muted),
            row![btn(&p, "Use this look", ui::Kind::Primary, false, Some(Msg::MyUse(id.clone()))), btn(&p, "Delete", ui::Kind::Normal, false, Some(Msg::MyDelete(id)))].spacing(8.0),
        ].spacing(6.0))
        .padding(12.0)
        .width(Length::Fixed(sz(280.0)))
        .style(ui::boxed(p.panel, p.r(10.0), None));
        r = r.push(tile);
        if i % 3 == 2 {
            g = g.push(r);
            r = Row::new().spacing(12.0);
        }
    }
    g = g.push(r);
    c = c.push(g);
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.msg_bad));
    }
    c.into()
}

// ---------------------------------------------------------------- Window frames & sign-in splash
pub fn look_more(a: &App) -> E<'_> {
    let sh = &a.sh;
    let st = &a.pers;
    let p = sh.p;
    let plasma = kit::on_plasma();
    let tint = st.tint.unwrap_or_else(|| sh.d_num("frameTint", 0.0) as f32);
    let mut c = kit::Col::new(st.skip);
    c = c.push(head(&p, "Window frames"));
    // Two windows, the one in front and one behind, in the colours the frames will have.
    let win = |active: bool, title: &'static str| -> E<'static> {
        let base = if active { p.panel3 } else { p.panel2 };
        let bar = Pal::mix(base, p.accent, (if active { 0.55 } else { 0.27 }) * tint / 100.0);
        let edge = Pal::mix(p.line, p.accent, (if active { 0.7 } else { 0.35 }) * tint / 100.0);
        let dots = (0..3).fold(Row::new().spacing(8.0), |r, i| r.push(container(Space::new()).width(Length::Fixed(11.0)).height(Length::Fixed(11.0)).style(ui::boxed(Pal::a(if i == 2 && active { p.danger } else { p.muted }, 0.8), 6.0, None))));
        container(column![
            container(row![t1(title, sz(13.0), if active { p.text } else { p.muted }).font(ui::weight(iced::font::Weight::Semibold)), Space::new().width(Length::Fill), dots].align_y(iced::Center)).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 14.0 }).center_y(Length::Fixed(32.0)).width(Length::Fill).style(ui::boxed(bar, 9.0, None)),
            Space::new().height(Length::Fill),
        ])
        .width(Length::FillPortion(5))
        .height(Length::Fixed(140.0))
        .style(ui::boxed(p.bg, 9.0, Some((edge, 1.0))))
        .into()
    };
    let wins = iced::widget::stack![
        container(row![Space::new().width(Length::FillPortion(1)), win(false, "Files"), Space::new().width(Length::FillPortion(4))]).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        container(row![Space::new().width(Length::FillPortion(3)), win(true, "Text editor"), Space::new().width(Length::FillPortion(2))]).padding(Padding { top: 34.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    ]
    .height(Length::Fixed(190.0));
    c = c.push(container(wins).padding(Padding { top: 20.0, right: 10.0, bottom: 20.0, left: 10.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None)));
    let shown = if tint == 0.0 { "Off".to_string() } else { format!("{}%", tint as i64) };
    c = c.push(irow(&p, Ic::G(G::Palette), "Accent colour in the frames", if plasma { "0% keeps the look's own frames. More tints Hearth's window frames with your accent (KDE's frames take the look's colour scheme)" } else { "0% keeps the look's own frames. More tints the title bars and edges with your accent" }, kit::slide(&p, tint / 100.0, 0.0..=1.0, 0.05, shown, Msg::Tint)));
    c = c.push(head(&p, "Apps"));
    let af = sh.d_bool("appsFollow", true);
    c = c.push(irow(&p, Ic::G(G::Palette), "Apps follow Hearth", if af { "Qt, KDE and GTK apps use the look's colours and your accent" } else { "Qt, KDE and GTK apps keep their own colours (Breeze and Adwaita)" }, switch(&p, af, Some(Msg::AppsFollow(!af)))));
    c = c.push(small(&p, "Apps already open change the next time they're opened. Hearth's own apps always follow the look."));
    c = c.push(head(&p, "After you sign in"));
    let cur = sh.d_str("signinSplash");
    let user = std::env::var("USER").unwrap_or_else(|_| "you".into());
    let mut grid = Row::new().spacing(10.0);
    for (v, title, sub) in [("", "None", "Straight to the desktop"), ("flame", "Hearth's flame", "The logo over a glowing fire"), ("logo", "Logo", "Hearth's logo on dark"), ("plain", "Plain welcome", "Your name in the look's colours")] {
        let lit = cur == v;
        let dark = hearth_ui::theme::hex("#0d0a08").unwrap();
        let (bg, inner): (Color, E<'_>) = match v {
            "" => (p.bg, kit::icon(Ic::Desktops, p.muted, 30.0, 1.8)),
            "flame" => (dark, column![kit::icon(Ic::G(G::Leaf), hearth_ui::theme::hex("#ffb347").unwrap(), 34.0, 2.0), container(Space::new()).width(Length::Fixed(90.0)).height(Length::Fixed(10.0)).style(ui::boxed(Pal::a(hearth_ui::theme::hex("#ff8a3d").unwrap(), 0.55), 5.0, None))].spacing(6.0).align_x(iced::Center).into()),
            "logo" => (dark, kit::icon(Ic::G(G::Leaf), hearth_ui::theme::hex("#ff8a3d").unwrap(), 34.0, 2.0)),
            _ => (p.panel, ts(format!("Welcome, {user}"), sz(14.0), p.text).into()),
        };
        let pic = container(inner).center_x(Length::Fill).center_y(Length::Fixed(110.0)).style(ui::boxed(bg, 8.0, None));
        let card_ = container(column![pic, ts(title, sz(14.0), p.text), t1(sub, sz(12.0), p.muted)].spacing(6.0))
            .padding(10.0)
            .width(Length::FillPortion(1))
            .style(ui::boxed(p.panel, p.r(12.0), Some((if lit { p.accent } else { Pal::a(p.text, 0.07) }, if lit { 2.0 } else { 1.0 }))));
        grid = grid.push(mouse_area(card_).on_press(Msg::Splash(v.into())).interaction(mouse::Interaction::Pointer));
    }
    c = c.push(grid);
    if !cur.is_empty() {
        c = c.push(iact(&p, Ic::G(G::Play), "Show it now", if plasma { "A preview of KDE's splash in the look's colours" } else { "A preview: clicks go straight through it" }, false, Some(Msg::SplashNow)));
    }
    c = c.push(small(&p, if plasma { "On KDE Plasma the splash is KDE's own, in the look's colours with Hearth's logo; None turns it off. It never holds anything up. The boot splash (before signing in) is in System › Boot splash." } else { "It shows for two seconds while the desktop comes up and never holds anything up. The boot splash (before signing in) is in System › Boot splash." }));
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.msg_bad));
    }
    c.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_sub_themes() {
        assert_eq!(titlecase("my-cat_photo"), "My Cat Photo");
        // Every look's sub themes come from palettes.json; Hearth has its own colourways then the black set.
        std::env::set_var("HEARTH_PALETTES", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../linux/desktop/theme/palettes.json"));
        let l = subs("hearth", "dark");
        assert!(l.len() >= 5, "{}", l.len());
        assert!(!l[0].black && l.iter().any(|s| s.black));
        assert!(subs("zen", "dark").is_empty());
    }
}
