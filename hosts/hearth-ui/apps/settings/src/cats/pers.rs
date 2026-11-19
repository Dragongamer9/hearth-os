//! Personalization: every page of Mantel's (SetAppearance, SetBarDock, SetFonts, SetWallpaperMore, SetMyLooks,
//! SetLookMore and the KWin pages), for KDE Plasma on the Fedora edition.
//!
//! Hearth's own settings are desktop.json's (through hearth_settings_io.py, shared with Mantel on Debian). On Plasma,
//! Hearth's looks are Global Themes (org.hearthos.<look>): hearth-plasma-looks puts the look, mode, accent and sub
//! theme chosen here on in Plasma, and a Hearth OS Global Theme picked in KDE shows here; every Hearth app follows
//! the Global Theme picked in KDE (theme.rs). What only Plasma has (its panel, its wallpapers per screen, slideshow
//! and picture of the day, fonts for KDE apps, the splash) is changed with KDE's own tools: plasmashell's scripting,
//! plasma-apply-*, kwriteconfig6. Opening a page writes nothing.
//!
//! Its own files: pers/kit.rs (what these four categories share), pers/kwin.rs (KWin's pages), pers/looks.rs,
//! pers/desk.rs, pers/fonts.rs.

pub mod desk;
pub mod fonts;
pub mod kit;
pub mod kwin;
pub mod looks;

use crate::shared::Shared;
use crate::App;
use hearth_ui::run;
use iced::{Element, Subscription, Task};
use kwin::{KMsg, Kw};
use serde_json::Value;

pub const PAGES: &[&str] = &["appearance", "bar-dock", "fonts", "wallpaper-more", "my-looks", "look-more", "kwin-effects", "kwin-desktops", "kwin-tiling", "kwin-edges", "kwin-behaviour"];

/// What a file picker was opened for.
#[derive(Debug, Clone, PartialEq)]
pub enum For {
    Wall,
    Screen(usize),
    Folder,
    Pair(&'static str),
}

#[derive(Default)]
pub struct State {
    pub kw: Kw,
    pub skip: usize,
    pub page: &'static str,
    pub msg: String,
    pub msg_bad: bool,
    pub pick: Option<&'static str>,
    // Personalization
    pub looks_st: Option<Value>,
    pub pictures: Vec<String>,
    pub wall_open: bool,
    pub accent_more: bool,
    pub hex: String,
    pub hue: Option<f32>,
    pub kbd: Option<Value>,
    pub picking: bool,
    // Bar and dock
    pub panels: Option<Value>,
    pub panel_err: String,
    pub panel: usize,
    pub panel_remove: bool,
    // Fonts, cursor & icons
    pub fonts: Option<Value>,
    pub sizes: Option<(&'static str, f64)>,
    pub sizes_due: bool,
    pub small_open: bool,
    pub smooth_open: bool,
    // Wallpaper per screen…
    pub walls: Option<Value>,
    pub walls_err: String,
    // My looks
    pub my: Vec<Value>,
    pub my_name: String,
    pub my_fonts: Option<Value>,
    // Window frames & splash
    pub tint: Option<f32>,
    pub tint_due: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Did(run::Output),
    KdeDid(run::Output),
    Set(Value),
    Look(String),
    Mode(&'static str),
    Accent(String),
    Sub(String),
    Match(bool),
    Shift(&'static str, i32),
    Wall(String),
    Choose(For),
    Picked(For, run::Output),
    WallOpen,
    AccentMore,
    Hex(String),
    HexGo,
    Hue(f32),
    HueSet,
    FromWall,
    Kbd(bool),
    KbdSt(run::Output),
    LooksSt(run::Output),
    UseHearthLook,
    Kde(&'static str),
    Pictures(run::Output),
    /// Wallpapers' pictures read (pics.rs): drawn again with them.
    Pics,
    Open(&'static str),
    Close,
    K(KMsg),
    Go(&'static str),
    // Bar and dock
    Panels(run::Output),
    Panel(usize),
    Js(String),
    JsSet(String, Value),
    JsDone(run::Output),
    EditMode,
    RemovePanel,
    DeskClock(String),
    Glass(String),
    // Fonts
    Fonts(run::Output),
    FontPick(&'static str, Value),
    FontSize(&'static str, f32),
    FontFlush,
    FontSet(&'static str, run::Output),
    SmallOpen,
    SmoothOpen,
    // Wallpaper per screen…
    Walls(run::Output),
    ScreenSame(usize),
    Show(&'static str, Value),
    Potd(&'static str, Value),
    PotdAgain,
    Pair(&'static str, Value),
    // My looks
    MyFonts(run::Output),
    MyName(String),
    MySave,
    MyUse(String),
    MyDelete(String),
    MyDid(run::Output),
    // Window frames & splash
    Tint(f32),
    TintFlush,
    Splash(String),
    SplashNow,
    AppsFollow(bool),
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.page = page;
    st.msg.clear();
    st.pick = None;
    st.picking = false;
    // `PAGE:part`: a list or a closed part opened (as CI's pictures ask), or `from:N` (a long page further down).
    let part = std::mem::take(&mut sh.part);
    st.skip = kit::skip_of(&part);
    st.kw.skip = st.skip;
    st.kw.pick = None;
    let word = kit::part_word(&part);
    match (page, word.as_str()) {
        ("appearance", "sub") => st.pick = Some("sub"),
        ("appearance", "wallpaper") => {
            st.wall_open = true;
            st.accent_more = true;
        }
        ("fonts", "all") => {
            st.small_open = true;
            st.smooth_open = true;
        }
        ("fonts", "font") => st.pick = Some("font"),
        (k, x) if k.starts_with("kwin-") && !x.is_empty() => st.kw.pick = Some(x.to_string()),
        _ => {}
    }
    match page {
        "appearance" => Task::batch([
            Task::perform(kit::run_in(looks::status_cmd(), String::new()), Msg::LooksSt),
            Task::perform(kit::run_in(kit::sh_t(looks::PICTURES, 10), String::new()), Msg::Pictures),
            Task::perform(kit::run_in(kit::job("hearth-kbd-lights", &["status"]), String::new()), Msg::KbdSt),
        ]),
        "bar-dock" => desk::read_panels(),
        "fonts" => Task::perform(kit::run_in(kit::job("hearth-lookset", &["status"]), String::new()), Msg::Fonts),
        "wallpaper-more" => desk::read_walls(),
        "my-looks" => {
            st.my = looks::my_looks();
            Task::perform(kit::run_in(kit::job("hearth-lookset", &["status"]), String::new()), Msg::MyFonts)
        }
        "look-more" => {
            st.tint = None;
            Task::none()
        }
        p if p.starts_with("kwin-") => st.kw.open(p).map(Msg::K),
        _ => Task::none(),
    }
}

/// desktop.json: these keys, merged in (atomic, .bak, the change history: red flag 9).
pub fn save(sh: &mut Shared, v: Value) -> Task<Msg> {
    let cmd = sh.desktop_cmd(v);
    Task::perform(run::spawn(cmd), Msg::Did)
}
/// One of KDE's own tools (Plasma only), its failure shown at the top.
pub fn kde(cmd: Vec<String>) -> Task<Msg> {
    Task::perform(kit::run_in(cmd, String::new()), Msg::KdeDid)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Did(o) => sh.result("That setting couldn't be saved", &o),
        Msg::KdeDid(o) => {
            if !o.ok {
                let w = kit::plasma_why(&o);
                st.msg = if w.is_empty() { "KDE didn't take that change".into() } else { format!("KDE didn't take that change: {w}") };
                st.msg_bad = true;
            }
        }
        Msg::Set(v) => return save(sh, v),
        Msg::K(k) => {
            if let KMsg::Go(id) = k {
                sh.go(id);
                return Task::none();
            }
            if kwin::closes(&k) {
                st.kw.pick = None;
            }
            return st.kw.update(k).map(Msg::K);
        }
        Msg::Go(id) => sh.go(id),
        Msg::Open(id) => st.pick = Some(id),
        Msg::Close => st.pick = None,
        _ => {
            return match st.page {
                "bar-dock" | "wallpaper-more" => desk::update(sh, st, m),
                "fonts" => fonts::update(sh, st, m),
                _ => looks::update(sh, st, m),
            }
        }
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    let p = a.sh.p;
    let st = &a.pers;
    Some(match page {
        "appearance" => looks::appearance(a),
        "my-looks" => looks::my_looks_page(a),
        "look-more" => looks::look_more(a),
        "bar-dock" => desk::bar_dock(a),
        "wallpaper-more" => desk::wallpaper_more(a),
        "fonts" => fonts::page(a),
        "kwin-effects" => kwin::effects(&p, &st.kw).map(Msg::K),
        "kwin-desktops" => kwin::desktops(&p, &st.kw).map(Msg::K),
        "kwin-tiling" => kwin::tiling(&p, &st.kw).map(Msg::K),
        "kwin-edges" => kwin::edges(&p, &st.kw).map(Msg::K),
        "kwin-behaviour" => kwin::behaviour(&p, &st.kw).map(Msg::K),
        _ => return None,
    })
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    let st = &a.pers;
    if !PAGES.contains(&a.page) {
        return None;
    }
    if a.page.starts_with("kwin-") {
        return kwin::dialog(&a.sh.p, &st.kw).map(|(d, o)| (d.map(Msg::K), o.map(Msg::K)));
    }
    let id = st.pick?;
    let d = match a.page {
        "appearance" => looks::dialog(a, id),
        "bar-dock" | "wallpaper-more" => desk::dialog(a, id),
        "fonts" => fonts::dialog(a, id),
        _ => None,
    }?;
    Some((d, Some(Msg::Close)))
}

pub fn back(st: &mut State) -> bool {
    st.pick.take().is_some() || st.kw.pick.take().is_some()
}

/// What Personalization's own row on Home and the category page can show (Bar and dock's state).
#[allow(dead_code)]
pub fn bar_value(sh: &Shared) -> String {
    let b = sh.d_str("barStyle");
    let d = sh.d_str("dockMode");
    if b.is_empty() && d.is_empty() {
        return "Look's own".into();
    }
    let v: Vec<&str> = [match b.as_str() { "bar" => "Full-width bar", "floating" => "Islands", _ => "" }, match d.as_str() { "autohide" => "dock auto-hides", "shown" => "dock shown", _ => "" }].into_iter().filter(|x| !x.is_empty()).collect();
    v.join(", ")
}

