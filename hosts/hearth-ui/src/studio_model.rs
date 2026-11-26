//! Drag and drop in every Hearth app (drag-drop board 3, "The same in every app"; board 2's side panel): the rules,
//! with nothing of iced in them, so hearth-layout checks what's kept with the very same code (it includes this file by
//! path, as hearth-ui includes settings_file.rs).
//!
//! What's kept is `layout.studio` in the app's file (`~/.config/nimbus/apps/<app>.json`, written only by hearth-layout
//! app-keep: settings_file, atomic, a .bak, merged): `{"pages": {<page>: {"parts": {<part id>: Part}, "adds": [Add]}},
//! "own": [Own]}`. A part is one of the app's own (its blocks.rs blocks: the sidebar, the command bar…), placed anywhere
//! in its page by its four edges (each measured from the nearer side, so a sidebar keeps its width and the folder next
//! to it fills the rest when the window is resized), hidden, layered and restyled. An add is a block dragged in from
//! the app's blocks window: one of its buttons, panes or info, or one made here (Button · runs anything; Count, words,
//! spacer), in one of its bars (after one of the bar's own pieces) or free anywhere in the page.
//!
//! Nothing here writes anything: an editing [`Session`] changes its draft, Undo and Redo step through it, and only
//! [`Session::keep_change`] says what Keep this hands to hearth-layout. A page whose kept part can't be read is the
//! app's own page again (never a broken app).

use serde_json::{json, Map, Value};

/// The smallest a part or pane can be made (px).
pub const MIN: f32 = 24.0;
/// The grid parts snap to by default (px), and the sizes Snap to grid steps through.
pub const GRID: u16 = 8;
pub const GRIDS: &[u16] = &[8, 4, 16, 24];
/// How near an edge must come to another to line up with it (px).
pub const ALIGN: f32 = 6.0;
/// The most adds a page can hold and own blocks an app can keep (a file stays small).
pub const MAX_ADDS: usize = 64;
pub const MAX_OWN: usize = 32;
/// The biggest size anything can be (px; a 8K screen).
pub const MAX_PX: u16 = 8000;

pub const WIDTHS: &[&str] = &["", "fit", "fixed", "fill"];
pub const EDGES: &[&str] = &["", "none", "line", "glow"];
pub const SHADOWS: &[&str] = &["", "none", "soft", "deep"];
pub const WEIGHTS: &[&str] = &["", "light", "medium", "bold"];
pub const SHOW: &[&str] = &["", "icon", "words", "both"];
pub const HOVER: &[&str] = &["", "same", "lighter", "accent", "own"];
pub const PRESSED: &[&str] = &["", "same", "darker", "own"];
/// What a block shows (board 2's Shows): its own, words (with {values}), a count, a clock, a picture, a list.
pub const SHOWS: &[&str] = &["", "words", "count", "clock", "picture", "list"];
pub const WHEN: &[&str] = &["", "always", "hover", "something", "pages"];
pub const MOTION: &[&str] = &["", "none", "fade", "slide", "pop"];
pub const LAYER: &[&str] = &["", "above", "under"];
/// The fonts Hearth OS has ("" is the part's own; Inter is Hearth's).
pub const FONTS: &[&str] = &["", "Inter", "Inter Display", "Noto Serif", "Noto Sans Mono"];
/// The icons a block can have (hearth_ui::ui::G by name).
pub const ICONS: &[&str] = &[
    "", "check", "arrow", "plus", "close", "search", "gear", "chevron", "dots", "folder", "leaf", "stop", "play", "grid", "pulse", "list", "clock",
    "users", "rocket", "cog", "down", "home", "wifi", "bt", "display", "palette", "power", "shield", "user", "pad", "server", "link", "update",
    "access", "store", "sound", "drive", "moon", "info", "camera", "keyboard", "mouse", "menu", "doc", "columns", "asterisk", "sync", "pencil",
    "help", "bell", "tasks", "reliability",
];
/// The ways a block is pointed at or pressed that "Does" can set: press, right-click, middle-click, scroll, hover.
pub const DOES: &[&str] = &["press", "right", "middle", "scroll", "hover"];
/// The blocks you make yourself (MAKE YOUR OWN).
pub const OWN_KINDS: &[&str] = &["own:button", "own:count", "own:words", "own:spacer"];

// ------------------------------------------------------------------ places

/// One edge of a part: px from the start side (left or top), or from the end side (right or bottom) when `end`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edge {
    pub px: u16,
    pub end: bool,
}

/// A part's or pane's place in its page by its four edges.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Anchors {
    pub l: Edge,
    pub t: Edge,
    pub r: Edge,
    pub b: Edge,
}

/// A rectangle in a page (px from its top-left corner).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl R {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
    pub fn area(&self) -> f32 {
        self.w.max(0.0) * self.h.max(0.0)
    }
    /// Does `o` sit wholly inside this one?
    pub fn holds(&self, o: &R) -> bool {
        o.x >= self.x - 0.5 && o.y >= self.y - 0.5 && o.right() <= self.right() + 0.5 && o.bottom() <= self.bottom() + 0.5 && o.area() < self.area()
    }
    /// Is it a bar (long and thin: a strip blocks line up in) rather than an area?
    pub fn is_bar(&self) -> bool {
        self.w.min(self.h) <= 96.0 && self.w.max(self.h) >= self.w.min(self.h) * 2.0
    }
    pub fn across(&self) -> bool {
        self.w >= self.h
    }
}

fn edge_at(v: f32, len: f32, near_start: bool) -> Edge {
    let v = v.clamp(0.0, len.max(0.0));
    if near_start { Edge { px: v.round().min(MAX_PX as f32) as u16, end: false } } else { Edge { px: (len - v).round().clamp(0.0, MAX_PX as f32) as u16, end: true } }
}

impl Anchors {
    /// Where it is in a page this big (kept inside the page, never smaller than [`MIN`] where the page allows).
    pub fn resolve(&self, w: f32, h: f32) -> R {
        let at = |e: Edge, len: f32| if e.end { len - e.px as f32 } else { e.px as f32 };
        let fit = |a: f32, b: f32, len: f32| -> (f32, f32) {
            let min = MIN.min(len.max(0.0));
            let a = a.clamp(0.0, (len - min).max(0.0));
            let b = b.clamp(a + min, len.max(a + min));
            (a, b.min(len.max(a + min)))
        };
        let (x0, x1) = fit(at(self.l, w), at(self.r, w), w);
        let (y0, y1) = fit(at(self.t, h), at(self.b, h), h);
        R { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }

    /// The edges of `r` in a page this big, each measured from its nearer side (a sidebar at the left keeps its width;
    /// a part that reaches the right keeps reaching it).
    pub fn from_rect(r: R, w: f32, h: f32) -> Anchors {
        let x0 = r.x;
        let x1 = r.x + r.w;
        let y0 = r.y;
        let y1 = r.y + r.h;
        Anchors {
            l: edge_at(x0, w, x0 <= w - x0),
            r: edge_at(x1, w, x1 < w - x1),
            t: edge_at(y0, h, y0 <= h - y0),
            b: edge_at(y1, h, y1 < h - y1),
        }
    }

    fn to_json(&self) -> Value {
        let s = |e: Edge| if e.end { 'e' } else { 's' };
        json!({"at": [self.l.px, self.t.px, self.r.px, self.b.px], "from": format!("{}{}{}{}", s(self.l), s(self.t), s(self.r), s(self.b))})
    }

    fn read(o: &Map<String, Value>) -> Result<Option<Anchors>, String> {
        let (Some(at), Some(from)) = (o.get("at"), o.get("from")) else {
            return if o.contains_key("at") || o.contains_key("from") { Err("a place needs both \"at\" and \"from\"".into()) } else { Ok(None) };
        };
        let a = at.as_array().filter(|a| a.len() == 4).ok_or("\"at\" is four numbers")?;
        let mut px = [0u16; 4];
        for (i, v) in a.iter().enumerate() {
            px[i] = v.as_u64().filter(|n| *n <= MAX_PX as u64).ok_or("a place is in px, 0 to 8000")? as u16;
        }
        let f: Vec<char> = from.as_str().ok_or("\"from\" is four letters")?.chars().collect();
        if f.len() != 4 || f.iter().any(|c| *c != 's' && *c != 'e') {
            return Err("\"from\" is four of s (from the start) or e (from the end)".into());
        }
        let e = |i: usize| Edge { px: px[i], end: f[i] == 'e' };
        Ok(Some(Anchors { l: e(0), t: e(1), r: e(2), b: e(3) }))
    }
}

// ------------------------------------------------------------------ the look and the rest (board 2's side panel)

/// Everything board 2's side panel sets on a part or a block. "" / 0 / None is the part's own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    // Look · SIZE AND PLACE
    pub wmode: &'static str,
    pub w: u16,
    pub h: u16,
    /// space around it: top, right, bottom, left
    pub around: [u16; 4],
    pub inside: Option<u16>,
    // Look · BACKGROUND AND EDGE
    pub fill: String,
    pub see: u8,
    pub blur: bool,
    pub corners: Option<u8>,
    pub edge: &'static str,
    pub edge_colour: String,
    pub edge_w: u8,
    pub shadow: &'static str,
    // Look · ICON AND WORDS
    pub icon: &'static str,
    pub icon_colour: String,
    pub icon_size: u8,
    pub words: String,
    pub font: &'static str,
    pub font_size: u8,
    pub weight: &'static str,
    pub show: &'static str,
    // Look · WHEN IT'S POINTED AT · PRESSED
    pub hover: &'static str,
    pub hover_fill: String,
    pub pressed: &'static str,
    pub pressed_fill: String,
    // Shows
    pub shows: &'static str,
    pub source: String,
    pub picture: String,
    // Does: press, right, middle, scroll, hover ("" its own, "none", "act:<id>", "run:<command>", "panel:<pane>")
    pub does: [String; 5],
    // Place (where it is is its part's or add's own; above or under the others)
    pub layer: &'static str,
    // When
    pub when: &'static str,
    pub pages: Vec<String>,
    // Motion
    pub motion: &'static str,
    pub bounce: bool,
}

pub fn colour_ok(s: &str) -> bool {
    s.is_empty() || (s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit()))
}

fn id_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 48 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// What "Does" can be: "" (its own), "none", "act:<one of the app's buttons>", "run:<a command>", "panel:<a pane>".
pub fn action_ok(s: &str) -> bool {
    if s.is_empty() || s == "none" {
        return true;
    }
    if let Some(c) = s.strip_prefix("run:") {
        return !c.trim().is_empty() && c.len() <= 400 && !c.chars().any(|ch| ch == '\0' || ch == '\n' || ch == '\r');
    }
    match s.split_once(':') {
        Some(("act", id)) | Some(("panel", id)) => id_ok(id),
        _ => false,
    }
}

fn one_of(list: &'static [&'static str], v: Option<&Value>, what: &str) -> Result<&'static str, String> {
    match v {
        None | Some(Value::Null) => Ok(""),
        Some(Value::String(s)) => list.iter().copied().find(|x| *x == s.as_str()).ok_or_else(|| format!("{what} can't be \"{s}\"")),
        _ => Err(format!("{what} is a word")),
    }
}

fn num(v: Option<&Value>, max: u64, what: &str) -> Result<Option<u64>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(n) => n.as_u64().filter(|n| *n <= max).map(Some).ok_or_else(|| format!("{what} is a number up to {max}")),
    }
}

fn text(v: Option<&Value>, max: usize, what: &str) -> Result<String, String> {
    match v {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) if s.chars().count() <= max && !s.contains('\0') => Ok(s.clone()),
        _ => Err(format!("{what} is words, up to {max} letters")),
    }
}

fn colour(v: Option<&Value>, what: &str) -> Result<String, String> {
    let s = text(v, 7, what)?;
    if colour_ok(&s) { Ok(s.to_lowercase()) } else { Err(format!("{what} is a colour like #ff7a2f")) }
}

const STYLE_KEYS: &[&str] = &[
    "width", "w", "h", "around", "inside", "fill", "see", "blur", "corners", "edge", "edgeColour", "edgeW", "shadow", "icon", "iconColour",
    "iconSize", "words", "font", "fontSize", "weight", "show", "hover", "hoverFill", "pressed", "pressedFill", "shows", "source", "picture",
    "does", "layer", "when", "pages", "motion", "bounce",
];

impl Style {
    pub fn is_own(&self) -> bool {
        *self == Style::default()
    }

    pub fn read(v: &Value) -> Result<Style, String> {
        let o = v.as_object().ok_or("a look is {…}")?;
        if let Some(k) = o.keys().find(|k| !STYLE_KEYS.contains(&k.as_str())) {
            return Err(format!("a part has no \"{k}\" to set"));
        }
        let mut s = Style { wmode: one_of(WIDTHS, o.get("width"), "Width")?, ..Style::default() };
        s.w = num(o.get("w"), MAX_PX as u64, "Width")?.unwrap_or(0) as u16;
        s.h = num(o.get("h"), MAX_PX as u64, "Height")?.unwrap_or(0) as u16;
        if let Some(a) = o.get("around") {
            let a = a.as_array().filter(|a| a.len() == 4).ok_or("Space around it is four numbers")?;
            for (i, x) in a.iter().enumerate() {
                s.around[i] = x.as_u64().filter(|n| *n <= 400).ok_or("Space around it is 0 to 400 px")? as u16;
            }
        }
        s.inside = num(o.get("inside"), 200, "Space inside")?.map(|n| n as u16);
        s.fill = colour(o.get("fill"), "Fill")?;
        s.see = num(o.get("see"), 100, "See-through")?.unwrap_or(0) as u8;
        s.blur = match o.get("blur") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            _ => return Err("Blur behind it is yes or no".into()),
        };
        s.corners = num(o.get("corners"), 48, "Corners")?.map(|n| n as u8);
        s.edge = one_of(EDGES, o.get("edge"), "Edge")?;
        s.edge_colour = colour(o.get("edgeColour"), "The edge's colour")?;
        s.edge_w = num(o.get("edgeW"), 8, "The edge's width")?.unwrap_or(0) as u8;
        s.shadow = one_of(SHADOWS, o.get("shadow"), "Shadow")?;
        s.icon = one_of(ICONS, o.get("icon"), "Icon")?;
        s.icon_colour = colour(o.get("iconColour"), "The icon's colour")?;
        s.icon_size = num(o.get("iconSize"), 96, "The icon's size")?.unwrap_or(0) as u8;
        s.words = text(o.get("words"), 120, "Words")?;
        s.font = one_of(FONTS, o.get("font"), "Font")?;
        s.font_size = num(o.get("fontSize"), 72, "The font's size")?.unwrap_or(0) as u8;
        s.weight = one_of(WEIGHTS, o.get("weight"), "The font's weight")?;
        s.show = one_of(SHOW, o.get("show"), "Show")?;
        s.hover = one_of(HOVER, o.get("hover"), "Hover look")?;
        s.hover_fill = colour(o.get("hoverFill"), "The hover colour")?;
        s.pressed = one_of(PRESSED, o.get("pressed"), "Pressed look")?;
        s.pressed_fill = colour(o.get("pressedFill"), "The pressed colour")?;
        s.shows = one_of(SHOWS, o.get("shows"), "Shows")?;
        s.source = text(o.get("source"), 48, "What it counts")?;
        if !s.source.is_empty() && !id_ok(&s.source) {
            return Err("What it counts is named by its id".into());
        }
        s.picture = text(o.get("picture"), 400, "The picture")?;
        if let Some(d) = o.get("does") {
            let d = d.as_object().ok_or("Does is {press, right, middle, scroll, hover}")?;
            for (k, v) in d {
                let i = DOES.iter().position(|x| x == k).ok_or_else(|| format!("Does has no \"{k}\""))?;
                let a = text(Some(v), 404, "What it does")?;
                if !action_ok(&a) {
                    return Err(format!("\"{a}\" isn't something a block can do"));
                }
                s.does[i] = a;
            }
        }
        s.layer = one_of(LAYER, o.get("layer"), "Layer")?;
        s.when = one_of(WHEN, o.get("when"), "When")?;
        if let Some(p) = o.get("pages") {
            let p = p.as_array().filter(|a| a.len() <= 32).ok_or("Pages is a list")?;
            for x in p {
                let x = x.as_str().filter(|x| id_ok(x)).ok_or("a page is named by its id")?;
                s.pages.push(x.to_string());
            }
        }
        s.motion = one_of(MOTION, o.get("motion"), "Motion")?;
        s.bounce = match o.get("bounce") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            _ => return Err("Bounces is yes or no".into()),
        };
        Ok(s)
    }

    /// Only what differs from the part's own.
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        let mut put = |k: &str, v: Value| {
            m.insert(k.to_string(), v);
        };
        if !self.wmode.is_empty() {
            put("width", json!(self.wmode));
        }
        if self.w > 0 {
            put("w", json!(self.w));
        }
        if self.h > 0 {
            put("h", json!(self.h));
        }
        if self.around != [0; 4] {
            put("around", json!(self.around));
        }
        if let Some(i) = self.inside {
            put("inside", json!(i));
        }
        for (k, v) in [("fill", &self.fill), ("edgeColour", &self.edge_colour), ("iconColour", &self.icon_colour), ("words", &self.words), ("hoverFill", &self.hover_fill), ("pressedFill", &self.pressed_fill), ("source", &self.source), ("picture", &self.picture)] {
            if !v.is_empty() {
                put(k, json!(v));
            }
        }
        if self.see > 0 {
            put("see", json!(self.see));
        }
        if self.blur {
            put("blur", json!(true));
        }
        if let Some(c) = self.corners {
            put("corners", json!(c));
        }
        if self.edge_w > 0 {
            put("edgeW", json!(self.edge_w));
        }
        if self.icon_size > 0 {
            put("iconSize", json!(self.icon_size));
        }
        if self.font_size > 0 {
            put("fontSize", json!(self.font_size));
        }
        for (k, v) in [("edge", self.edge), ("shadow", self.shadow), ("icon", self.icon), ("font", self.font), ("weight", self.weight), ("show", self.show), ("hover", self.hover), ("pressed", self.pressed), ("shows", self.shows), ("layer", self.layer), ("when", self.when), ("motion", self.motion)] {
            if !v.is_empty() {
                put(k, json!(v));
            }
        }
        if self.does.iter().any(|d| !d.is_empty()) {
            let mut d = Map::new();
            for (i, a) in self.does.iter().enumerate() {
                if !a.is_empty() {
                    d.insert(DOES[i].to_string(), json!(a));
                }
            }
            put("does", Value::Object(d));
        }
        if !self.pages.is_empty() {
            put("pages", json!(self.pages));
        }
        if self.bounce {
            put("bounce", json!(true));
        }
        Value::Object(m)
    }

    /// Copy its look: everything of the Look tab (not what it shows, does, or when).
    pub fn look_of(&self) -> Style {
        Style { shows: "", source: String::new(), picture: String::new(), does: Default::default(), when: "", pages: Vec::new(), layer: "", ..self.clone() }
    }

    /// Paste a look onto this: the look's Look tab, this one's own Shows, Does, Place and When.
    pub fn with_look(&self, look: &Style) -> Style {
        Style { shows: self.shows, source: self.source.clone(), picture: self.picture.clone(), does: self.does.clone(), when: self.when, pages: self.pages.clone(), layer: self.layer, ..look.clone() }
    }
}

// ------------------------------------------------------------------ a page's parts and adds

/// One of the app's own parts, as it's set in its page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Part {
    pub id: String,
    /// moved (None: where the app puts it)
    pub at: Option<Anchors>,
    pub hidden: bool,
    pub z: i16,
    pub style: Style,
}

/// A block dragged in from the app's blocks window.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Add {
    pub uid: String,
    /// "act:<button>", "pane:<pane>", "info:<info>", "own:button" | "own:count" | "own:words" | "own:spacer", "mine:<own block>"
    pub block: String,
    /// its name (the blocks window's, or the one you gave it)
    pub title: String,
    /// the part (bar) it's in; "" free in the page at `at`
    pub into: String,
    /// in its bar: after this piece of the bar (one of its own, or another add's uid); "" at the start
    pub after: String,
    pub at: Option<Anchors>,
    pub hidden: bool,
    pub z: i16,
    pub style: Style,
}

/// A block you made and saved (Save as my own block…): its name, what it's made from and its look.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Own {
    pub id: String,
    pub name: String,
    pub block: String,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageDraft {
    pub parts: Vec<Part>,
    pub adds: Vec<Add>,
}

impl PageDraft {
    pub fn is_empty(&self) -> bool {
        self.adds.is_empty() && self.parts.iter().all(|p| p.at.is_none() && !p.hidden && p.z == 0 && p.style.is_own())
    }
    /// Has any part been moved off where the app puts it (the page is then laid out by its parts' places)?
    pub fn moved(&self) -> bool {
        self.parts.iter().any(|p| p.at.is_some())
    }
    pub fn part(&self, id: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.id == id)
    }
    pub fn part_mut(&mut self, id: &str) -> &mut Part {
        if let Some(i) = self.parts.iter().position(|p| p.id == id) {
            return &mut self.parts[i];
        }
        self.parts.push(Part { id: id.to_string(), ..Part::default() });
        self.parts.last_mut().expect("just pushed")
    }
    pub fn add(&self, uid: &str) -> Option<&Add> {
        self.adds.iter().find(|a| a.uid == uid)
    }
    pub fn add_mut(&mut self, uid: &str) -> Option<&mut Add> {
        self.adds.iter_mut().find(|a| a.uid == uid)
    }

    /// The adds in one bar, in order (each after the piece it names: the bar's own pieces in `own`, in their order).
    pub fn in_bar(&self, part: &str, own: &[&str]) -> Vec<&Add> {
        let mut out: Vec<&Add> = Vec::new();
        let mine: Vec<&Add> = self.adds.iter().filter(|a| a.into == part && !a.hidden).collect();
        // at the start, then after each own piece in turn; each add followed by the adds after it
        fn chain<'x>(mine: &[&'x Add], after: &str, out: &mut Vec<&'x Add>, depth: usize) {
            if depth > MAX_ADDS {
                return;
            }
            let next: Vec<&'x Add> = mine.iter().copied().filter(|a| a.after == after && !out.iter().any(|o| o.uid == a.uid)).collect();
            for a in next {
                if out.iter().any(|o| o.uid == a.uid) {
                    continue;
                }
                out.push(a);
                chain(mine, &a.uid, out, depth + 1);
            }
        }
        chain(&mine, "", &mut out, 0);
        for p in own {
            chain(&mine, p, &mut out, 0);
        }
        // anything whose piece isn't there any more: at the end, never lost
        for a in &mine {
            if !out.iter().any(|o| o.uid == a.uid) {
                out.push(a);
            }
        }
        out
    }

    fn to_json(&self) -> Value {
        let mut parts = Map::new();
        for p in &self.parts {
            let mut o = Map::new();
            if let Some(a) = p.at {
                if let Value::Object(m) = a.to_json() {
                    o.extend(m);
                }
            }
            if p.hidden {
                o.insert("hidden".into(), json!(true));
            }
            if p.z != 0 {
                o.insert("z".into(), json!(p.z));
            }
            if !p.style.is_own() {
                o.insert("style".into(), p.style.to_json());
            }
            if !o.is_empty() {
                parts.insert(p.id.clone(), Value::Object(o));
            }
        }
        let adds: Vec<Value> = self
            .adds
            .iter()
            .map(|a| {
                let mut o = Map::new();
                o.insert("uid".into(), json!(a.uid));
                o.insert("block".into(), json!(a.block));
                o.insert("title".into(), json!(a.title));
                if !a.into.is_empty() {
                    o.insert("in".into(), json!(a.into));
                }
                if !a.after.is_empty() {
                    o.insert("after".into(), json!(a.after));
                }
                if let Some(at) = a.at {
                    if let Value::Object(m) = at.to_json() {
                        o.extend(m);
                    }
                }
                if a.hidden {
                    o.insert("hidden".into(), json!(true));
                }
                if a.z != 0 {
                    o.insert("z".into(), json!(a.z));
                }
                if !a.style.is_own() {
                    o.insert("style".into(), a.style.to_json());
                }
                Value::Object(o)
            })
            .collect();
        let mut m = Map::new();
        if !parts.is_empty() {
            m.insert("parts".into(), Value::Object(parts));
        }
        if !adds.is_empty() {
            m.insert("adds".into(), Value::Array(adds));
        }
        Value::Object(m)
    }

    fn read(v: &Value) -> Result<PageDraft, String> {
        let o = v.as_object().ok_or("a page is {parts, adds}")?;
        if let Some(k) = o.keys().find(|k| *k != "parts" && *k != "adds") {
            return Err(format!("a page has no \"{k}\""));
        }
        let mut d = PageDraft::default();
        if let Some(ps) = o.get("parts") {
            let ps = ps.as_object().ok_or("parts is {part: …}")?;
            for (id, p) in ps {
                if !id_ok(id) {
                    return Err(format!("a part is named by its id (\"{id}\")"));
                }
                let po = p.as_object().ok_or("a part is {…}")?;
                if let Some(k) = po.keys().find(|k| !["at", "from", "hidden", "z", "style"].contains(&k.as_str())) {
                    return Err(format!("a part has no \"{k}\""));
                }
                let at = Anchors::read(po)?;
                let hidden = match po.get("hidden") {
                    None | Some(Value::Null) => false,
                    Some(Value::Bool(b)) => *b,
                    _ => return Err("hidden is yes or no".into()),
                };
                let z = match po.get("z") {
                    None | Some(Value::Null) => 0,
                    Some(n) => n.as_i64().filter(|n| n.abs() <= 999).ok_or("a layer is -999 to 999")? as i16,
                };
                let style = match po.get("style") {
                    None | Some(Value::Null) => Style::default(),
                    Some(s) => Style::read(s)?,
                };
                d.parts.push(Part { id: id.clone(), at, hidden, z, style });
            }
        }
        if let Some(adds) = o.get("adds") {
            let adds = adds.as_array().filter(|a| a.len() <= MAX_ADDS).ok_or("adds is a list (up to 64)")?;
            for a in adds {
                let ao = a.as_object().ok_or("an add is {…}")?;
                if let Some(k) = ao.keys().find(|k| !["uid", "block", "title", "in", "after", "at", "from", "hidden", "z", "style"].contains(&k.as_str())) {
                    return Err(format!("a block has no \"{k}\""));
                }
                let uid = text(ao.get("uid"), 48, "A block's id")?;
                if !id_ok(&uid) || d.adds.iter().any(|x| x.uid == uid) {
                    return Err(format!("\"{uid}\" isn't a block's own id"));
                }
                let block = text(ao.get("block"), 64, "What the block is")?;
                let ok = OWN_KINDS.contains(&block.as_str()) || matches!(block.split_once(':'), Some(("act" | "pane" | "info" | "mine", id)) if id_ok(id));
                if !ok {
                    return Err(format!("\"{block}\" isn't a block"));
                }
                let title = text(ao.get("title"), 80, "A block's name")?;
                let into = text(ao.get("in"), 48, "Where it is")?;
                if !into.is_empty() && !id_ok(&into) {
                    return Err("a block is in a part named by its id".into());
                }
                let after = text(ao.get("after"), 48, "What it's after")?;
                if !after.is_empty() && !after.chars().all(|c| c.is_ascii_alphanumeric() || "-_:.".contains(c)) {
                    return Err("a block is after a piece named by its id".into());
                }
                let at = Anchors::read(ao)?;
                if into.is_empty() && at.is_none() {
                    return Err(format!("\"{title}\" is in no bar and has no place"));
                }
                let hidden = match ao.get("hidden") {
                    None | Some(Value::Null) => false,
                    Some(Value::Bool(b)) => *b,
                    _ => return Err("hidden is yes or no".into()),
                };
                let z = match ao.get("z") {
                    None | Some(Value::Null) => 0,
                    Some(n) => n.as_i64().filter(|n| n.abs() <= 999).ok_or("a layer is -999 to 999")? as i16,
                };
                let style = match ao.get("style") {
                    None | Some(Value::Null) => Style::default(),
                    Some(s) => Style::read(s)?,
                };
                d.adds.push(Add { uid, block, title, into, after, at, hidden, z, style });
            }
        }
        Ok(d)
    }
}

// ------------------------------------------------------------------ everything an app has (the draft, what's kept)

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Draft {
    pub pages: Vec<(String, PageDraft)>,
    pub own: Vec<Own>,
    /// the app's menus reordered (Menus…): (menu, its items' ids in order); and the items hidden from every menu
    pub menus: Vec<(String, Vec<String>)>,
    pub hidden_items: Vec<String>,
}

impl Draft {
    pub fn page(&self, page: &str) -> Option<&PageDraft> {
        self.pages.iter().find(|p| p.0 == page).map(|p| &p.1)
    }
    pub fn page_mut(&mut self, page: &str) -> &mut PageDraft {
        if let Some(i) = self.pages.iter().position(|p| p.0 == page) {
            return &mut self.pages[i].1;
        }
        self.pages.push((page.to_string(), PageDraft::default()));
        &mut self.pages.last_mut().expect("just pushed").1
    }
    pub fn menu(&self, kind: &str) -> Option<&Vec<String>> {
        self.menus.iter().find(|m| m.0 == kind).map(|m| &m.1)
    }

    /// `layout.studio` as kept.
    pub fn studio_json(&self) -> Value {
        let mut pages = Map::new();
        for (id, p) in &self.pages {
            if !p.is_empty() {
                pages.insert(id.clone(), p.to_json());
            }
        }
        let own: Vec<Value> = self.own.iter().map(|o| json!({"id": o.id, "name": o.name, "block": o.block, "style": o.style.to_json()})).collect();
        json!({"pages": pages, "own": own})
    }

    /// What's kept for the app (its file's `layout`): a page that can't be read is passed over (the app's own page);
    /// own blocks that can't be read too. Never fails.
    pub fn read(layout: &Value) -> Draft {
        let mut d = Draft::default();
        if let Some(pages) = layout.pointer("/studio/pages").and_then(Value::as_object) {
            for (id, p) in pages {
                if id_ok(id) {
                    if let Ok(pd) = PageDraft::read(p) {
                        d.pages.push((id.clone(), pd));
                    }
                }
            }
        }
        if let Some(own) = layout.pointer("/studio/own").and_then(Value::as_array) {
            for o in own.iter().take(MAX_OWN) {
                if let Ok(x) = read_own(o) {
                    d.own.push(x);
                }
            }
        }
        if let Some(z) = layout.get("zones").and_then(Value::as_object) {
            for (k, v) in z {
                let list: Vec<String> = v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
                if let Some(m) = k.strip_prefix("menu:") {
                    d.menus.push((m.to_string(), list));
                } else if k == "hidden" {
                    d.hidden_items = list;
                }
            }
        }
        d
    }
}

fn read_own(v: &Value) -> Result<Own, String> {
    let o = v.as_object().ok_or("an own block is {…}")?;
    if let Some(k) = o.keys().find(|k| !["id", "name", "block", "style"].contains(&k.as_str())) {
        return Err(format!("an own block has no \"{k}\""));
    }
    let id = text(o.get("id"), 48, "Its id")?;
    if !id_ok(&id) {
        return Err("an own block is named by its id".into());
    }
    let name = text(o.get("name"), 80, "Its name")?;
    if name.trim().is_empty() {
        return Err("an own block has a name".into());
    }
    let block = text(o.get("block"), 64, "What it's made from")?;
    if !(OWN_KINDS.contains(&block.as_str()) || matches!(block.split_once(':'), Some(("act" | "pane" | "info", id)) if id_ok(id))) {
        return Err(format!("\"{block}\" isn't a block"));
    }
    let style = match o.get("style") {
        None | Some(Value::Null) => Style::default(),
        Some(s) => Style::read(s)?,
    };
    Ok(Own { id, name, block, style })
}

/// hearth-layout's check of `layout.studio` (the same rules the app reads with): {"pages": {page: {} | page}, "own":
/// [...]} given back as it's kept, or every problem. An empty page is "that page as the app has it".
pub fn check(v: &Value) -> Result<Value, Vec<String>> {
    let Some(o) = v.as_object() else { return Err(vec!["The app's layout couldn't be read.".into()]) };
    let mut errs = Vec::new();
    let mut out = Map::new();
    for (k, x) in o {
        match k.as_str() {
            "pages" => {
                let Some(pm) = x.as_object() else {
                    errs.push("The pages couldn't be read.".into());
                    continue;
                };
                let mut po = Map::new();
                for (id, p) in pm {
                    if !id_ok(id) {
                        errs.push(format!("A page is named by its id (\"{id}\")."));
                        continue;
                    }
                    match PageDraft::read(p) {
                        Ok(d) => {
                            po.insert(id.clone(), d.to_json());
                        }
                        Err(e) => errs.push(format!("\"{id}\": {e}.")),
                    }
                }
                out.insert("pages".into(), Value::Object(po));
            }
            "own" => {
                let Some(list) = x.as_array().filter(|l| l.len() <= MAX_OWN) else {
                    errs.push("Your own blocks are a list (up to 32).".into());
                    continue;
                };
                let mut ol = Vec::new();
                for b in list {
                    match read_own(b) {
                        Ok(b) => ol.push(json!({"id": b.id, "name": b.name, "block": b.block, "style": b.style.to_json()})),
                        Err(e) => errs.push(format!("{e}.")),
                    }
                }
                out.insert("own".into(), Value::Array(ol));
            }
            _ => errs.push(format!("The layout has no \"{k}\".")),
        }
    }
    if errs.is_empty() { Ok(Value::Object(out)) } else { Err(errs) }
}

/// hearth-layout's keep of `layout.studio`: the pages named replace those pages (an empty one is the app's own page
/// again), the others stay; "own", when named, replaces the own blocks.
pub fn merge(kept: Option<&Value>, change: &Value) -> Value {
    let mut pages = kept.and_then(|k| k.get("pages")).and_then(Value::as_object).cloned().unwrap_or_default();
    if let Some(pm) = change.get("pages").and_then(Value::as_object) {
        for (id, p) in pm {
            if p.as_object().map(|m| m.is_empty()).unwrap_or(true) {
                pages.remove(id);
            } else {
                pages.insert(id.clone(), p.clone());
            }
        }
    }
    let own = change.get("own").cloned().or_else(|| kept.and_then(|k| k.get("own")).cloned()).unwrap_or(json!([]));
    json!({"pages": pages, "own": own})
}

// ------------------------------------------------------------------ the rules of dragging

/// `v` on the grid (none: as it is).
pub fn snap(v: f32, grid: Option<u16>) -> f32 {
    match grid {
        Some(g) if g > 0 => (v / g as f32).round() * g as f32,
        _ => v,
    }
}

/// The nearest line `v` lines up with (within [`ALIGN`] px): its place and what it belongs to.
pub fn align<'a>(v: f32, lines: &'a [(f32, String)]) -> Option<(f32, &'a str)> {
    lines.iter().filter(|(x, _)| (x - v).abs() <= ALIGN).min_by(|a, b| (a.0 - v).abs().total_cmp(&(b.0 - v).abs())).map(|(x, n)| (*x, n.as_str()))
}

/// Where a block dropped on a bar goes: between which of the bar's pieces (their rectangles, in order along the bar).
/// The index it lands at (0: before the first) and where its lit line is drawn (x for a bar across, y for one down).
pub fn drop_at(pieces: &[R], across: bool, x: f32, y: f32, bar: R) -> (usize, f32) {
    let v = if across { x } else { y };
    let mut i = 0;
    for p in pieces {
        let mid = if across { p.x + p.w / 2.0 } else { p.y + p.h / 2.0 };
        if v > mid {
            i += 1;
        }
    }
    let line = if pieces.is_empty() {
        if across { bar.x + 8.0 } else { bar.y + 8.0 }
    } else if i == 0 {
        let f = &pieces[0];
        if across { f.x - 4.0 } else { f.y - 4.0 }
    } else if i >= pieces.len() {
        let l = &pieces[pieces.len() - 1];
        if across { l.right() + 4.0 } else { l.bottom() + 4.0 }
    } else {
        let (a, b) = (&pieces[i - 1], &pieces[i]);
        if across { (a.right() + b.x) / 2.0 } else { (a.bottom() + b.y) / 2.0 }
    };
    (i, line)
}

/// One side of a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

/// A part's side dragged to `to` (snapped already): the part sized, and each other part whose facing side was against
/// it sized with it (the folder next to the sidebar gets narrower as the sidebar gets wider). Nothing goes under [`MIN`]
/// or off the page.
pub fn size_side(rects: &mut [R], i: usize, side: Side, to: f32, page: R) -> f32 {
    let r = rects[i];
    let (lo, hi) = match side {
        Side::Left => (page.x, r.right() - MIN),
        Side::Right => (r.x + MIN, page.right()),
        Side::Top => (page.y, r.bottom() - MIN),
        Side::Bottom => (r.y + MIN, page.bottom()),
    };
    let mut to = to.clamp(lo.min(hi), hi.max(lo));
    let was = match side {
        Side::Left => r.x,
        Side::Right => r.right(),
        Side::Top => r.y,
        Side::Bottom => r.bottom(),
    };
    // the neighbours against it, and how far they let it go
    let near = |a: f32, b: f32| (a - b).abs() <= 1.5;
    let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| a0.max(b0) < a1.min(b1) - 0.5;
    let mut with: Vec<usize> = Vec::new();
    for (j, o) in rects.iter().enumerate() {
        if j == i {
            continue;
        }
        let against = match side {
            Side::Right => near(o.x, was) && overlap(o.y, o.bottom(), r.y, r.bottom()),
            Side::Left => near(o.right(), was) && overlap(o.y, o.bottom(), r.y, r.bottom()),
            Side::Bottom => near(o.y, was) && overlap(o.x, o.right(), r.x, r.right()),
            Side::Top => near(o.bottom(), was) && overlap(o.x, o.right(), r.x, r.right()),
        };
        if against {
            with.push(j);
            to = match side {
                Side::Right => to.min(o.right() - MIN),
                Side::Left => to.max(o.x + MIN),
                Side::Bottom => to.min(o.bottom() - MIN),
                Side::Top => to.max(o.y + MIN),
            };
        }
    }
    let set = |q: &mut R, s: Side, v: f32| match s {
        Side::Left => {
            let right = q.right();
            q.x = v;
            q.w = right - v;
        }
        Side::Right => q.w = v - q.x,
        Side::Top => {
            let bottom = q.bottom();
            q.y = v;
            q.h = bottom - v;
        }
        Side::Bottom => q.h = v - q.y,
    };
    set(&mut rects[i], side, to);
    let facing = match side {
        Side::Left => Side::Right,
        Side::Right => Side::Left,
        Side::Top => Side::Bottom,
        Side::Bottom => Side::Top,
    };
    for j in with {
        set(&mut rects[j], facing, to);
    }
    to
}

/// A part or block moved by (dx, dy), kept in the page, its top-left on the grid.
pub fn moved(r: R, dx: f32, dy: f32, page: R, grid: Option<u16>) -> R {
    let x = snap(r.x + dx - page.x, grid) + page.x;
    let y = snap(r.y + dy - page.y, grid) + page.y;
    R { x: x.clamp(page.x, (page.right() - r.w).max(page.x)), y: y.clamp(page.y, (page.bottom() - r.h).max(page.y)), w: r.w, h: r.h }
}

/// A new free block or pane dropped with its middle at (x, y): its size, on the grid, inside the page.
pub fn dropped(x: f32, y: f32, w: f32, h: f32, page: R, grid: Option<u16>) -> R {
    let w = w.min(page.w).max(MIN.min(page.w));
    let h = h.min(page.h).max(MIN.min(page.h));
    moved(R { x: x - w / 2.0, y: y - h / 2.0, w, h }, 0.0, 0.0, page, grid)
}

// ------------------------------------------------------------------ editing: the draft, Undo and Redo

/// One time editing an app (from Edit layout to Keep this or Cancel): what was kept when it began, the draft, and
/// what Undo and Redo step through. Nothing is written while it lasts.
#[derive(Debug, Clone, Default)]
pub struct Session {
    pub kept: Draft,
    pub draft: Draft,
    pub undo: Vec<Draft>,
    pub redo: Vec<Draft>,
    pub snap: bool,
    pub grid: u16,
    pub guides: bool,
    /// the next new block's number
    pub seq: u32,
}

impl Session {
    pub fn new(kept: Draft) -> Session {
        let seq = kept.pages.iter().flat_map(|p| p.1.adds.iter()).filter_map(|a| a.uid.strip_prefix('b').and_then(|n| n.parse::<u32>().ok())).max().unwrap_or(0) + 1;
        Session { draft: kept.clone(), kept, undo: Vec::new(), redo: Vec::new(), snap: true, grid: GRID, guides: true, seq }
    }

    /// The grid while snapping is on.
    pub fn grid(&self) -> Option<u16> {
        if self.snap { Some(self.grid) } else { None }
    }

    /// A change to the draft: kept for Undo (and Redo cleared) only when something really changed.
    pub fn change(&mut self, f: impl FnOnce(&mut Draft)) -> bool {
        let before = self.draft.clone();
        f(&mut self.draft);
        if self.draft == before {
            return false;
        }
        self.undo.push(before);
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    /// While something is being dragged: the draft changes, and Undo gets the place it was picked up from once (at
    /// the drop, [`Session::settle`]).
    pub fn live(&mut self, f: impl FnOnce(&mut Draft)) {
        f(&mut self.draft);
    }

    /// The drag let go: one step for Undo, from where it was picked up (`before`), if anything changed.
    pub fn settle(&mut self, before: Draft) -> bool {
        if self.draft == before {
            return false;
        }
        self.undo.push(before);
        self.redo.clear();
        true
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(d) => {
                let now = std::mem::replace(&mut self.draft, d);
                self.redo.push(now);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(d) => {
                let now = std::mem::replace(&mut self.draft, d);
                self.undo.push(now);
                true
            }
            None => false,
        }
    }

    /// Anything to keep?
    pub fn dirty(&self) -> bool {
        self.draft != self.kept
    }

    pub fn uid(&mut self) -> String {
        let u = format!("b{}", self.seq);
        self.seq += 1;
        u
    }

    /// What Keep this hands to hearth-layout app-keep (None: nothing changed, nothing to write): the pages that
    /// changed (an emptied one as {}, the app's own again), the own blocks if they changed, the menus that changed,
    /// and the old grid's places cleared for the pages now laid out here (`grid_pages`: pages kept on the grid before).
    pub fn keep_change(&self, grid_pages: &[&str]) -> Option<Value> {
        if !self.dirty() {
            return None;
        }
        let mut pages = Map::new();
        let mut names: Vec<&String> = self.draft.pages.iter().map(|p| &p.0).collect();
        for p in &self.kept.pages {
            if !names.contains(&&p.0) {
                names.push(&p.0);
            }
        }
        let mut blocks = Map::new();
        for name in names {
            let now = self.draft.page(name).cloned().unwrap_or_default();
            let was = self.kept.page(name).cloned().unwrap_or_default();
            if now != was {
                pages.insert(name.clone(), if now.is_empty() { json!({}) } else { now.to_json() });
                if grid_pages.contains(&name.as_str()) {
                    blocks.insert(name.clone(), json!({}));
                }
            }
        }
        let mut studio = Map::new();
        studio.insert("pages".into(), Value::Object(pages));
        if self.draft.own != self.kept.own {
            studio.insert("own".into(), self.draft.studio_json()["own"].clone());
        }
        let mut layout = Map::new();
        layout.insert("studio".into(), Value::Object(studio));
        if !blocks.is_empty() {
            layout.insert("blocks".into(), Value::Object(blocks));
        }
        let mut zones = Map::new();
        for (k, v) in &self.draft.menus {
            if self.kept.menu(k) != Some(v) {
                zones.insert(format!("menu:{k}"), json!(v));
            }
        }
        if self.draft.hidden_items != self.kept.hidden_items {
            zones.insert("hidden".into(), json!(self.draft.hidden_items));
        }
        if !zones.is_empty() {
            layout.insert("zones".into(), Value::Object(zones));
        }
        Some(json!({"layout": layout, "features": []}))
    }
}

// ------------------------------------------------------------------ ready-made layouts (Start from)

/// The ready-made starting points every app has (Start from): the app's own, what you kept, and three made from the
/// app's own parts.
pub const STARTS: &[(&str, &str)] = &[
    ("hearth", "Hearth"),
    ("kept", "What you kept"),
    ("mirror", "Mirrored (sides swapped)"),
    ("roomy", "Roomy"),
    ("compact", "Compact"),
];

/// A page laid out from one of [`STARTS`]: `rects` are its parts where they are now (page-relative), the page's size.
pub fn start_from(kind: &str, kept: Option<&PageDraft>, ids: &[&str], rects: &[R], w: f32, h: f32) -> PageDraft {
    match kind {
        "kept" => kept.cloned().unwrap_or_default(),
        "mirror" => PageDraft {
            parts: ids
                .iter()
                .zip(rects)
                .map(|(id, r)| Part { id: id.to_string(), at: Some(Anchors::from_rect(R { x: w - r.right(), ..*r }, w, h)), ..Part::default() })
                .collect(),
            adds: kept.map(|k| k.adds.clone()).unwrap_or_default(),
        },
        "roomy" | "compact" => {
            let (inside, around) = if kind == "roomy" { (16, 6) } else { (2, 0) };
            PageDraft {
                parts: ids.iter().map(|id| Part { id: id.to_string(), style: Style { inside: Some(inside), around: [around; 4], ..Style::default() }, ..Part::default() }).collect(),
                adds: kept.map(|k| k.adds.clone()).unwrap_or_default(),
            }
        }
        _ => PageDraft::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> R {
        R::new(0.0, 0.0, 1200.0, 800.0)
    }

    #[test]
    fn a_place_keeps_to_its_nearer_sides() {
        // a sidebar 216 px wide on the left, full height: its width stays when the window grows
        let side = Anchors::from_rect(R::new(0.0, 0.0, 216.0, 800.0), 1200.0, 800.0);
        assert_eq!((side.l, side.r), (Edge { px: 0, end: false }, Edge { px: 216, end: false }));
        assert!(side.b.end && side.b.px == 0, "it reaches the bottom and keeps reaching it");
        let wide = side.resolve(1600.0, 900.0);
        assert_eq!((wide.x, wide.w, wide.h), (0.0, 216.0, 900.0));
        // the folder beside it fills the rest
        let folder = Anchors::from_rect(R::new(216.0, 48.0, 984.0, 752.0), 1200.0, 800.0);
        let f = folder.resolve(1600.0, 900.0);
        assert_eq!((f.x, f.w), (216.0, 1384.0));
        // a pane floating in the middle stays where it was put, its own size
        let pane = Anchors::from_rect(R::new(500.0, 300.0, 240.0, 160.0), 1200.0, 800.0);
        let p = pane.resolve(1200.0, 800.0);
        assert_eq!((p.x, p.y, p.w, p.h), (500.0, 300.0, 240.0, 160.0));
        // a window too small: still on the page, never smaller than MIN
        let tiny = side.resolve(100.0, 30.0);
        assert!(tiny.right() <= 100.0 && tiny.w >= MIN && tiny.h <= 30.0);
    }

    #[test]
    fn snapping_and_lining_up() {
        assert_eq!(snap(517.0, Some(8)), 520.0);
        assert_eq!(snap(517.0, None), 517.0);
        assert_eq!(snap(13.0, Some(4)), 12.0);
        let lines = vec![(514.0, "Search".to_string()), (216.0, "the sidebar".to_string())];
        assert_eq!(align(511.0, &lines), Some((514.0, "Search")));
        assert_eq!(align(530.0, &lines), None, "too far: nothing lines up");
        let m = moved(R::new(100.0, 100.0, 200.0, 100.0), 37.0, -500.0, page(), Some(8));
        assert_eq!((m.x, m.y), (136.0, 0.0), "on the grid, kept in the page");
        let d = dropped(1190.0, 790.0, 300.0, 200.0, page(), Some(8));
        assert!(d.right() <= 1200.0 && d.bottom() <= 800.0);
    }

    #[test]
    fn dropping_into_a_bar_lands_between_its_pieces() {
        let bar = R::new(0.0, 0.0, 600.0, 48.0);
        let pieces = [R::new(10.0, 8.0, 60.0, 32.0), R::new(80.0, 8.0, 60.0, 32.0), R::new(150.0, 8.0, 60.0, 32.0)];
        let (i, line) = drop_at(&pieces, true, 130.0, 20.0, bar);
        assert_eq!(i, 2, "after the second piece");
        assert_eq!(line, 145.0, "the lit line between the two");
        assert_eq!(drop_at(&pieces, true, 0.0, 20.0, bar).0, 0);
        assert_eq!(drop_at(&pieces, true, 590.0, 20.0, bar), (3, 214.0));
        assert_eq!(drop_at(&[], true, 300.0, 20.0, bar), (0, 8.0));
        // a bar down the side: by height
        assert_eq!(drop_at(&[R::new(0.0, 0.0, 40.0, 40.0)], false, 10.0, 90.0, R::new(0.0, 0.0, 48.0, 400.0)).0, 1);
    }

    #[test]
    fn adds_in_a_bar_follow_their_pieces() {
        let mut d = PageDraft::default();
        let add = |uid: &str, after: &str| Add { uid: uid.into(), block: "act:share".into(), title: uid.into(), into: "toolbar".into(), after: after.into(), ..Add::default() };
        d.adds.push(add("b2", "sort"));
        d.adds.push(add("b1", ""));
        d.adds.push(add("b3", "b2"));
        d.adds.push(add("b4", "gone"));
        let order: Vec<&str> = d.in_bar("toolbar", &["new", "sort", "view"]).iter().map(|a| a.uid.as_str()).collect();
        assert_eq!(order, vec!["b1", "b2", "b3", "b4"], "start, after Sort (then what's after it), and the lost one last");
    }

    #[test]
    fn sizing_a_side_sizes_its_neighbour() {
        let mut rs = vec![R::new(0.0, 0.0, 216.0, 800.0), R::new(216.0, 0.0, 670.0, 800.0), R::new(886.0, 0.0, 314.0, 800.0)];
        let at = size_side(&mut rs, 2, Side::Left, 686.0, page());
        assert_eq!(at, 686.0);
        assert_eq!((rs[2].x, rs[2].w), (686.0, 514.0), "the preview: 514 px");
        assert_eq!(rs[1].w, 470.0, "the folder gave way");
        assert_eq!(rs[0].w, 216.0, "the sidebar wasn't against it");
        // never past its neighbour's own smallest
        let at = size_side(&mut rs, 0, Side::Right, 2000.0, page());
        assert_eq!(at, 686.0 - MIN);
    }

    #[test]
    fn undo_and_redo_step_through_the_draft() {
        let mut s = Session::new(Draft::default());
        assert!(!s.change(|_| {}), "nothing changed: nothing to undo");
        assert!(s.change(|d| d.page_mut("window").part_mut("sidebar").hidden = true));
        assert!(s.change(|d| d.page_mut("window").part_mut("sidebar").style.corners = Some(12)));
        assert_eq!(s.undo.len(), 2);
        assert!(s.undo());
        assert_eq!(s.draft.page("window").and_then(|p| p.part("sidebar")).map(|p| p.style.corners), Some(None));
        assert!(s.redo());
        assert_eq!(s.draft.page("window").and_then(|p| p.part("sidebar")).map(|p| p.style.corners), Some(Some(12)));
        assert!(s.undo() && s.undo());
        assert!(!s.dirty(), "all undone: as it was kept");
        assert!(!s.undo(), "nothing more");
        // a drag is one step
        let before = s.draft.clone();
        s.live(|d| d.page_mut("window").part_mut("folder").z = 1);
        s.live(|d| d.page_mut("window").part_mut("folder").z = 2);
        assert!(s.settle(before));
        assert_eq!(s.undo.len(), 1);
        assert!(s.redo.is_empty());
    }

    #[test]
    fn nothing_is_kept_until_keep_this() {
        let mut s = Session::new(Draft::default());
        assert!(s.keep_change(&[]).is_none(), "nothing changed: nothing to write");
        let uid = s.uid();
        s.change(|d| {
            d.page_mut("bars").adds.push(Add { uid: uid.clone(), block: "act:sort-downloads".into(), title: "Sort my Downloads".into(), into: "toolbar".into(), after: "sort".into(), ..Add::default() })
        });
        s.change(|d| d.page_mut("window").part_mut("preview").style.w = 514);
        s.change(|d| d.menus.push(("context".into(), vec!["open".into(), "share".into()])));
        let c = s.keep_change(&["window"]).unwrap();
        assert_eq!(c["layout"]["studio"]["pages"]["bars"]["adds"][0]["after"], "sort");
        assert_eq!(c["layout"]["studio"]["pages"]["window"]["parts"]["preview"]["style"]["w"], 514);
        assert_eq!(c["layout"]["blocks"]["window"], json!({}), "the old grid's places cleared for the page laid out here");
        assert_eq!(c["layout"]["zones"]["menu:context"], json!(["open", "share"]));
        assert!(c["layout"]["studio"].get("own").is_none(), "own blocks didn't change: not named");
        // what's kept reads back the same, through the same check hearth-layout makes
        let checked = check(&c["layout"]["studio"]).unwrap();
        let kept = Draft::read(&json!({"studio": merge(None, &checked)}));
        assert_eq!(kept.page("bars"), s.draft.page("bars"));
        assert_eq!(kept.page("window"), s.draft.page("window"));
        // everything put back: the pages emptied, the app's own again
        let mut back = Session::new(kept.clone());
        back.change(|d| d.pages.clear());
        let c = back.keep_change(&[]).unwrap();
        assert_eq!(c["layout"]["studio"]["pages"]["bars"], json!({}));
        let m = merge(Some(&json!({"pages": {"bars": {"adds": []}, "other": {"parts": {"x": {"hidden": true}}}}})), &c["layout"]["studio"]);
        assert!(m["pages"].get("bars").is_none() && m["pages"].get("other").is_some(), "only the pages named change");
    }

    #[test]
    fn every_setting_reads_back_and_bad_ones_are_refused() {
        let s = Style {
            wmode: "fixed",
            w: 158,
            h: 30,
            around: [7, 6, 7, 6],
            inside: Some(12),
            fill: "#3b2a1d".into(),
            see: 15,
            blur: true,
            corners: Some(14),
            edge: "line",
            edge_colour: "#ff7a2f".into(),
            edge_w: 1,
            shadow: "soft",
            icon: "down",
            icon_colour: "#ff7a2f".into(),
            icon_size: 16,
            words: "Downloads · {count}".into(),
            font: "Inter",
            font_size: 14,
            weight: "medium",
            show: "both",
            hover: "lighter",
            hover_fill: String::new(),
            pressed: "darker",
            pressed_fill: String::new(),
            shows: "count",
            source: "folder-count".into(),
            picture: String::new(),
            does: ["act:sort-downloads".into(), "run:notify-send hi".into(), "none".into(), String::new(), "panel:preview".into()],
            layer: "above",
            when: "pages",
            pages: vec!["window".into()],
            motion: "pop",
            bounce: true,
        };
        assert_eq!(Style::read(&s.to_json()).unwrap(), s);
        assert_eq!(Style::default().to_json(), json!({}), "its own look: nothing kept");
        for bad in [
            json!({"width": "huge"}),
            json!({"w": 9000}),
            json!({"fill": "orange"}),
            json!({"see": 101}),
            json!({"corners": 99}),
            json!({"font": "Wingdings"}),
            json!({"icon": "rocketship"}),
            json!({"does": {"press": "rm -rf ~"}}),
            json!({"does": {"kick": "none"}}),
            json!({"does": {"press": "run:a\nb"}}),
            json!({"around": [1, 2, 3]}),
            json!({"colour": "#ffffff"}),
        ] {
            assert!(Style::read(&bad).is_err(), "{bad}");
        }
        assert!(check(&json!({"pages": {"window": {"parts": {"side": {"at": [0, 0, 216], "from": "ssse"}}}}})).is_err());
        assert!(check(&json!({"pages": {"window": {"adds": [{"uid": "b1", "block": "act:x", "title": "X"}]}}})).is_err(), "in no bar and no place");
        assert!(check(&json!({"pages": {"window": {"adds": [{"uid": "b1", "block": "exec:x", "title": "X", "in": "bar"}]}}})).is_err());
        assert!(check(&json!({"pages": {"Window": {}}})).is_err());
        assert!(check(&json!({"pages": {}, "own": [{"id": "mine", "name": "Mine", "block": "own:button"}]})).is_ok());
        // a page that can't be read is the app's own page; the others still open
        let d = Draft::read(&json!({"studio": {"pages": {"bad": {"parts": {"x": {"z": "top"}}}, "good": {"parts": {"x": {"hidden": true}}}}}}));
        assert!(d.page("bad").is_none() && d.page("good").is_some());
    }

    #[test]
    fn start_from_makes_pages_from_the_apps_own_parts() {
        let ids = ["sidebar", "folder"];
        let rects = [R::new(0.0, 0.0, 216.0, 800.0), R::new(216.0, 0.0, 984.0, 800.0)];
        let m = start_from("mirror", None, &ids, &rects, 1200.0, 800.0);
        let side = m.part("sidebar").unwrap().at.unwrap().resolve(1200.0, 800.0);
        assert_eq!((side.x, side.w), (984.0, 216.0), "the sidebar on the right");
        assert!(start_from("hearth", None, &ids, &rects, 1200.0, 800.0).is_empty(), "the app's own");
        assert_eq!(start_from("roomy", None, &ids, &rects, 1200.0, 800.0).part("folder").unwrap().style.inside, Some(16));
    }
}
