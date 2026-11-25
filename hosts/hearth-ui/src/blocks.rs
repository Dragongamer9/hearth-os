//! Every Hearth app made of blocks (the owner, Nov 24: "Modular for all Hearth apps is required as well"), the way
//! Build anything does it for the desktop (build-anything board 4): each app names its pages' blocks ([`PageDef`],
//! [`BlockDef`]: an id, a title, its place and size on a 12 × 12 grid), and [`page`] lays a page out from what the
//! person kept for it (`~/.config/nimbus/apps/<app>.json`, `layout.blocks.<page>`, written only by hearth-layout's
//! app-keep: settings_file, atomic, .bak). In Arrange (Describe it › Move and resize blocks) a block is dragged by its
//! bar, sized by its corner, hidden (✕) or added back (+), and restyled (Look: corners, see-through, font, text size,
//! accent); Describe it can change the same through hearth-layout describe-app. Undo, Back to the original, Keep this.
//!
//! Nothing customised: the app's own view, exactly as it was (no change to its look). A page whose kept blocks can't be
//! read or don't check comes back as the app's own (never a broken app). Where the blocks go is worked out once per
//! change ([`resolve`], kept in Describe), never per frame.

use crate::theme::Pal;
use iced::Color;
use serde_json::{json, Map, Value};

/// The grid every page is laid on: 12 columns, 12 rows (a block's x, y, w, h count these).
pub const COLS: u8 = 12;
pub const ROWS: u8 = 12;

/// One block of an app's page: its id, its title (in Arrange and to your AI) and where it is by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockDef {
    pub id: &'static str,
    pub title: &'static str,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    /// laid out on its own, it scrolls (a piece of a page that scrolls as a whole in the app's own view)
    pub scroll: bool,
}

pub const fn bl(id: &'static str, title: &'static str, x: u8, y: u8, w: u8, h: u8) -> BlockDef {
    BlockDef { id, title, x, y, w, h, scroll: false }
}
/// A block that scrolls when it's laid out on its own.
pub const fn bls(id: &'static str, title: &'static str, x: u8, y: u8, w: u8, h: u8) -> BlockDef {
    BlockDef { id, title, x, y, w, h, scroll: true }
}

/// One page of an app made of blocks.
#[derive(Debug, Clone, Copy)]
pub struct PageDef {
    pub id: &'static str,
    pub title: &'static str,
    pub blocks: &'static [BlockDef],
}

/// A block's look (build-anything board 4, Look): "" or None is the block's own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Look {
    /// rounded corners, 0-48 px (None: the block's own, no card)
    pub corners: Option<u8>,
    /// see-through, 0-90 (%)
    pub see: u8,
    /// a font family from [`FONTS`] ("" Inter)
    pub font: &'static str,
    /// "" | "smaller" | "bigger"
    pub size: &'static str,
    /// "#rrggbb" ("" the look's own)
    pub accent: String,
}

impl Look {
    pub fn is_own(&self) -> bool {
        *self == Look::default()
    }
    /// The text size this look asks for (1.0: as the block has it).
    pub fn scale(&self) -> f32 {
        match self.size {
            "smaller" => 0.88,
            "bigger" => 1.18,
            _ => 1.0,
        }
    }
    pub fn face(&self) -> Option<&'static str> {
        if self.font.is_empty() { None } else { Some(self.font) }
    }
    pub fn accent_colour(&self) -> Option<Color> {
        colour(&self.accent)
    }
}

/// The fonts a block can have (Hearth OS has these; "" is Inter).
pub const FONTS: &[&str] = &["", "Noto Serif", "Noto Sans Mono", "Inter Display"];
pub const SIZES: &[&str] = &["", "bigger", "smaller"];
/// Corners and see-through as Arrange steps through them.
pub const CORNERS: &[Option<u8>] = &[None, Some(8), Some(16), Some(28), Some(0)];
pub const SEE: &[u8] = &[0, 15, 35, 60];
/// Hearth's accents (the editor's swatches; build-anything's own list).
pub const ACCENTS: &[&str] = &["", "#ff7a2f", "#5b8cff", "#2f9e5a", "#b07cff", "#f2c14e", "#e05a6a"];

pub fn colour(s: &str) -> Option<Color> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(Color::from_rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// A block where it goes now.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub id: &'static str,
    pub title: &'static str,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    pub hidden: bool,
    pub look: Look,
    pub scroll: bool,
}

impl Placed {
    pub fn of(d: &BlockDef) -> Placed {
        Placed { id: d.id, title: d.title, x: d.x, y: d.y, w: d.w, h: d.h, hidden: false, look: Look::default(), scroll: d.scroll }
    }
}

/// The page as the app draws it by default.
pub fn defaults(def: &PageDef) -> Vec<Placed> {
    def.blocks.iter().map(Placed::of).collect()
}

fn num(v: &Value, k: &str) -> Result<Option<u8>, ()> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(n) => n.as_u64().filter(|n| *n <= 255).map(|n| Some(n as u8)).ok_or(()),
    }
}

fn pick(list: &[&'static str], v: Option<&Value>) -> Result<&'static str, ()> {
    match v {
        None | Some(Value::Null) => Ok(""),
        Some(Value::String(s)) => list.iter().copied().find(|x| *x == s.as_str()).ok_or(()),
        _ => Err(()),
    }
}

/// One block's kept entry read onto its default, or Err when it doesn't check (the page then stays the app's own).
fn read_one(d: &BlockDef, v: &Value) -> Result<Placed, ()> {
    let o = v.as_object().ok_or(())?;
    for k in o.keys() {
        if !matches!(k.as_str(), "x" | "y" | "w" | "h" | "hidden" | "corners" | "seeThrough" | "font" | "size" | "accent") {
            return Err(());
        }
    }
    let mut p = Placed::of(d);
    p.x = num(v, "x")?.unwrap_or(d.x);
    p.y = num(v, "y")?.unwrap_or(d.y);
    p.w = num(v, "w")?.unwrap_or(d.w);
    p.h = num(v, "h")?.unwrap_or(d.h);
    if p.w == 0 || p.h == 0 || p.x as u16 + p.w as u16 > COLS as u16 || p.y as u16 + p.h as u16 > ROWS as u16 {
        return Err(());
    }
    p.hidden = match v.get("hidden") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        _ => return Err(()),
    };
    p.look.corners = num(v, "corners")?;
    if p.look.corners.map(|c| c > 48).unwrap_or(false) {
        return Err(());
    }
    p.look.see = num(v, "seeThrough")?.unwrap_or(0);
    if p.look.see > 90 {
        return Err(());
    }
    p.look.font = pick(FONTS, v.get("font"))?;
    p.look.size = pick(SIZES, v.get("size"))?;
    p.look.accent = match v.get("accent") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) if s.is_empty() || colour(s).is_some() => s.to_lowercase(),
        _ => return Err(()),
    };
    Ok(p)
}

/// Where a page's blocks go, from what's kept for it (`layout.blocks.<page>`: {block id: {x, y, w, h, hidden, corners,
/// seeThrough, font, size, accent}}). None: the app's own view (nothing kept for the page, an empty page, one that
/// can't be read or doesn't check, or every block hidden). A block the page doesn't name keeps its default place; an
/// id the app no longer has is passed over (an older file still opens).
pub fn resolve(def: &PageDef, saved: Option<&Value>) -> Option<Vec<Placed>> {
    let m = saved?.as_object()?;
    if m.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(def.blocks.len());
    for d in def.blocks {
        match m.get(d.id) {
            Some(v) => out.push(read_one(d, v).ok()?),
            None => out.push(Placed::of(d)),
        }
    }
    if out.iter().all(|p| p.hidden) {
        return None;
    }
    Some(out)
}

/// What's tried over what's kept, block by block: a page named in `over` takes its blocks over `base`'s (an empty page
/// in `over` is "back to the app's own" for that page).
pub fn merge(base: Option<&Value>, over: Option<&Value>) -> Option<Value> {
    match (base.and_then(Value::as_object), over.and_then(Value::as_object)) {
        (_, Some(o)) if o.is_empty() => None,
        (Some(b), Some(o)) => {
            let mut m = b.clone();
            for (k, v) in o {
                m.insert(k.clone(), v.clone());
            }
            Some(Value::Object(m))
        }
        (None, Some(o)) => Some(Value::Object(o.clone())),
        (Some(b), None) => Some(Value::Object(b.clone())),
        (None, None) => None,
    }
}

/// A page's blocks as they're kept: every block's place, and only what differs from its own look.
pub fn to_json(placed: &[Placed]) -> Value {
    let mut m = Map::new();
    for p in placed {
        let mut o = Map::new();
        o.insert("x".into(), json!(p.x));
        o.insert("y".into(), json!(p.y));
        o.insert("w".into(), json!(p.w));
        o.insert("h".into(), json!(p.h));
        if p.hidden {
            o.insert("hidden".into(), json!(true));
        }
        if let Some(c) = p.look.corners {
            o.insert("corners".into(), json!(c));
        }
        if p.look.see > 0 {
            o.insert("seeThrough".into(), json!(p.look.see));
        }
        if !p.look.font.is_empty() {
            o.insert("font".into(), json!(p.look.font));
        }
        if !p.look.size.is_empty() {
            o.insert("size".into(), json!(p.look.size));
        }
        if !p.look.accent.is_empty() {
            o.insert("accent".into(), json!(p.look.accent));
        }
        m.insert(p.id.to_string(), Value::Object(o));
    }
    Value::Object(m)
}

/// Is this the page exactly as the app has it (nothing to keep)?
pub fn is_default(def: &PageDef, placed: &[Placed]) -> bool {
    placed.len() == def.blocks.len() && placed.iter().zip(def.blocks).all(|(p, d)| *p == Placed::of(d))
}

/// What hearth-layout is told of the app's blocks (HEARTH_APP_BLOCKS: what your AI may move, and the check).
pub fn catalog(pages: &[PageDef]) -> Value {
    json!({"cols": COLS, "rows": ROWS, "pages": pages.iter().map(|p| json!({
        "id": p.id, "title": p.title,
        "blocks": p.blocks.iter().map(|b| json!({"id": b.id, "title": b.title, "x": b.x, "y": b.y, "w": b.w, "h": b.h})).collect::<Vec<_>>()
    })).collect::<Vec<_>>()})
}

/// What a grip does while it's held.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grip {
    Move,
    Size,
    /// the right edge: wider or narrower only (Edit layout)
    Wide,
    /// the bottom edge: taller or shorter only (Edit layout)
    Tall,
}

/// A held block moved or sized to the grid cell under the pointer: `start` is the block as it was picked up, `from` the
/// cell it was picked up in. The block stays whole and on the grid.
pub fn drag(start: &Placed, grip: Grip, from: (u8, u8), to: (u8, u8)) -> Placed {
    let mut p = start.clone();
    match grip {
        Grip::Move => {
            let dx = to.0 as i16 - from.0 as i16;
            let dy = to.1 as i16 - from.1 as i16;
            p.x = (start.x as i16 + dx).clamp(0, (COLS - start.w) as i16) as u8;
            p.y = (start.y as i16 + dy).clamp(0, (ROWS - start.h) as i16) as u8;
        }
        Grip::Size => {
            p.w = (to.0 as i16 - start.x as i16 + 1).clamp(1, (COLS - start.x) as i16) as u8;
            p.h = (to.1 as i16 - start.y as i16 + 1).clamp(1, (ROWS - start.y) as i16) as u8;
        }
        Grip::Wide => p.w = (to.0 as i16 - start.x as i16 + 1).clamp(1, (COLS - start.x) as i16) as u8,
        Grip::Tall => p.h = (to.1 as i16 - start.y as i16 + 1).clamp(1, (ROWS - start.y) as i16) as u8,
    }
    p
}

fn overlaps(a: &Placed, b: &Placed) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// The other blocks moved out of the way of block `fixed` (down below what's in the way, else to its right, else made
/// shorter or narrower to fit), so nothing overlaps; with `fixed` None, the page is tidied as it reads (top to bottom,
/// left to right). Hidden blocks are in nobody's way. Edit layout runs it on every move from the page as it was when
/// the block was picked up, so blocks pushed aside come back when the held one moves on (live reflow).
pub fn push_aside(placed: &mut [Placed], fixed: Option<usize>) {
    let mut order: Vec<usize> = (0..placed.len()).filter(|i| Some(*i) != fixed && !placed[*i].hidden).collect();
    order.sort_by_key(|i| (placed[*i].y, placed[*i].x));
    let mut settled: Vec<usize> = fixed.filter(|i| *i < placed.len() && !placed[*i].hidden).into_iter().collect();
    for j in order {
        let mut p = placed[j].clone();
        for _ in 0..(COLS as usize + ROWS as usize) * 2 {
            let Some(s) = settled.iter().map(|k| &placed[*k]).find(|s| overlaps(&p, s)).cloned() else { break };
            if s.y + s.h + p.h <= ROWS {
                p.y = s.y + s.h;
            } else if s.x + s.w + p.w <= COLS {
                p.x = s.x + s.w;
            } else if s.y + s.h < ROWS {
                p.y = s.y + s.h;
                p.h = ROWS - p.y;
            } else if s.x + s.w < COLS {
                p.x = s.x + s.w;
                p.w = COLS - p.x;
            } else {
                break;
            }
        }
        placed[j] = p;
        settled.push(j);
    }
}

/// A block moved by whole cells (the keys and the controller in Edit layout), staying whole and on the grid.
pub fn nudge(p: &Placed, dx: i8, dy: i8) -> Placed {
    let mut n = p.clone();
    n.x = (p.x as i16 + dx as i16).clamp(0, (COLS - p.w) as i16) as u8;
    n.y = (p.y as i16 + dy as i16).clamp(0, (ROWS - p.h) as i16) as u8;
    n
}

/// A block sized by whole cells from its top-left corner (never smaller than a cell, never off the grid).
pub fn grow(p: &Placed, dw: i8, dh: i8) -> Placed {
    let mut n = p.clone();
    n.w = (p.w as i16 + dw as i16).clamp(1, (COLS - p.x) as i16) as u8;
    n.h = (p.h as i16 + dh as i16).clamp(1, (ROWS - p.y) as i16) as u8;
    n
}

/// Which part of the look a press in Arrange steps on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LookPart {
    Corners,
    See,
    Font,
    Size,
    Accent,
}

fn step<T: PartialEq + Clone>(list: &[T], v: &T) -> T {
    let i = list.iter().position(|x| x == v).map(|i| i + 1).unwrap_or(0);
    list[i % list.len()].clone()
}

/// The look with one part stepped on to its next value (round to the block's own again).
pub fn next_look(l: &Look, part: LookPart) -> Look {
    let mut n = l.clone();
    match part {
        LookPart::Corners => n.corners = step(CORNERS, &l.corners),
        LookPart::See => n.see = step(SEE, &l.see),
        LookPart::Font => n.font = step(FONTS, &l.font),
        LookPart::Size => n.size = step(SIZES, &l.size),
        LookPart::Accent => {
            let cur: &str = ACCENTS.iter().copied().find(|a| *a == l.accent).unwrap_or("");
            n.accent = step(ACCENTS, &cur).to_string();
        }
    }
    n
}

/// How the look's parts read in Arrange.
pub fn look_label(l: &Look, part: LookPart) -> String {
    match part {
        LookPart::Corners => format!("Corners: {}", l.corners.map(|c| if c == 0 { "square".to_string() } else { format!("{c}") }).unwrap_or_else(|| "its own".into())),
        LookPart::See => format!("See-through: {}", if l.see == 0 { "no".to_string() } else { format!("{}%", l.see) }),
        LookPart::Font => format!("Font: {}", if l.font.is_empty() { "Inter" } else { l.font }),
        LookPart::Size => format!("Text: {}", if l.size.is_empty() { "as it is" } else { l.size }),
        LookPart::Accent => format!("Accent: {}", if l.accent.is_empty() { "the look's" } else { l.accent.as_str() }),
    }
}

/// A block's card in a laid-out page: nothing when the look is its own (the block draws itself as it always has),
/// else the panel (see-through as asked) with its corners and the accent as its edge.
pub fn card_style(p: &Pal, l: &Look) -> Option<(Color, f32, Option<(Color, f32)>)> {
    if l.corners.is_none() && l.see == 0 && l.accent.is_empty() {
        return None;
    }
    let bg = Pal::a(p.panel, 1.0 - l.see as f32 / 100.0);
    let r = l.corners.unwrap_or(0) as f32;
    let line = l.accent_colour().map(|c| (c, 2.0)).or(Some((p.line, 1.0)));
    Some((bg, r, line))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: PageDef = PageDef { id: "main", title: "Main", blocks: &[bl("side", "Side", 0, 0, 2, 12), bl("list", "List", 2, 0, 10, 12)] };

    #[test]
    fn nothing_kept_is_the_apps_own_view() {
        assert!(resolve(&PAGE, None).is_none());
        assert!(resolve(&PAGE, Some(&json!({}))).is_none());
        assert!(resolve(&PAGE, Some(&json!("broken"))).is_none());
        assert!(resolve(&PAGE, Some(&json!([1, 2]))).is_none());
    }

    #[test]
    fn a_kept_page_is_laid_out_with_the_rest_at_their_defaults() {
        let r = resolve(&PAGE, Some(&json!({"list": {"x": 0, "y": 0, "w": 12, "h": 8, "corners": 16, "seeThrough": 35, "font": "Noto Serif", "accent": "#FF7A2F"}}))).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].x, r[0].w), (0, 2), "side not named: its own place");
        assert_eq!((r[1].x, r[1].y, r[1].w, r[1].h), (0, 0, 12, 8));
        assert_eq!(r[1].look.corners, Some(16));
        assert_eq!(r[1].look.see, 35);
        assert_eq!(r[1].look.font, "Noto Serif");
        assert_eq!(r[1].look.accent, "#ff7a2f");
        assert_eq!(r[1].look.accent_colour(), Some(Color::from_rgb8(0xff, 0x7a, 0x2f)));
    }

    #[test]
    fn anything_that_doesnt_check_falls_back_to_the_original() {
        let bad = [
            json!({"list": {"x": 4, "w": 10}}),            // off the grid
            json!({"list": {"w": 0}}),                     // no size
            json!({"list": {"x": -1}}),                    // not a number it can be
            json!({"list": {"corners": 99}}),              // too round
            json!({"list": {"seeThrough": 100}}),          // gone
            json!({"list": {"font": "Wingdings"}}),        // a font Hearth hasn't got
            json!({"list": {"accent": "red"}}),            // not a colour
            json!({"list": {"hidden": "yes"}}),            // not a yes or no
            json!({"list": {"x": 0, "run": "rm -rf ~"}}),  // something else entirely
            json!({"list": "big"}),                        // not a block
            json!({"side": {"hidden": true}, "list": {"hidden": true}}), // nothing left to see
        ];
        for b in bad {
            assert!(resolve(&PAGE, Some(&b)).is_none(), "{b}");
        }
    }

    #[test]
    fn an_id_the_app_no_longer_has_is_passed_over() {
        let r = resolve(&PAGE, Some(&json!({"gone": {"x": 0}, "side": {"hidden": true}}))).unwrap();
        assert!(r[0].hidden && !r[1].hidden);
    }

    #[test]
    fn tried_goes_over_kept_block_by_block() {
        let kept = json!({"side": {"hidden": true}, "list": {"x": 0, "w": 12}});
        let tried = json!({"list": {"x": 2, "w": 10, "corners": 8}});
        let m = merge(Some(&kept), Some(&tried)).unwrap();
        assert_eq!(m["side"]["hidden"], true);
        assert_eq!(m["list"]["corners"], 8);
        assert!(merge(Some(&kept), Some(&json!({}))).is_none(), "an empty page is the app's own again");
        assert_eq!(merge(Some(&kept), None).unwrap(), kept);
        assert!(merge(None, None).is_none());
    }

    #[test]
    fn what_is_kept_reads_back_the_same() {
        let mut r = defaults(&PAGE);
        assert!(is_default(&PAGE, &r));
        r[1].x = 3;
        r[1].w = 9;
        r[1].look = next_look(&r[1].look, LookPart::Corners);
        r[0].hidden = true;
        assert!(!is_default(&PAGE, &r));
        let j = to_json(&r);
        assert_eq!(resolve(&PAGE, Some(&j)).unwrap(), r);
        assert!(j["side"].get("corners").is_none(), "only what differs from its own look");
    }

    #[test]
    fn dragging_stays_on_the_grid() {
        let s = Placed::of(&PAGE.blocks[1]);
        let m = drag(&s, Grip::Move, (5, 5), (1, 7));
        assert_eq!((m.x, m.y), (0, 2), "moved left as far as it goes, down 2");
        let m = drag(&s, Grip::Move, (5, 5), (11, 11));
        assert_eq!((m.x, m.y, m.w, m.h), (2, 0, 10, 12), "a full-height block can't go lower");
        let z = drag(&s, Grip::Size, (11, 11), (6, 4));
        assert_eq!((z.w, z.h), (5, 5));
        let z = drag(&s, Grip::Size, (11, 11), (0, 0));
        assert_eq!((z.w, z.h), (1, 1), "never smaller than one cell");
    }

    #[test]
    fn edges_size_one_way_and_keys_stay_on_the_grid() {
        let s = Placed::of(&PAGE.blocks[1]); // 2, 0, 10 x 12
        let w = drag(&s, Grip::Wide, (11, 3), (7, 9));
        assert_eq!((w.w, w.h), (6, 12), "the right edge: only the width");
        let t = drag(&s, Grip::Tall, (5, 11), (9, 5));
        assert_eq!((t.w, t.h), (10, 6), "the bottom edge: only the height");
        let n = nudge(&s, -5, 3);
        assert_eq!((n.x, n.y), (0, 0), "a full-height block stays where it fits");
        let g = grow(&s, 4, -20);
        assert_eq!((g.w, g.h), (10, 1));
        let g = grow(&grow(&s, -3, -6), 1, 1);
        assert_eq!((g.x, g.w, g.h), (2, 8, 7));
    }

    #[test]
    fn others_make_way_and_nothing_overlaps() {
        const P3: PageDef = PageDef { id: "p", title: "P", blocks: &[bl("a", "A", 0, 0, 6, 2), bl("b", "B", 6, 0, 6, 2), bl("c", "C", 0, 2, 12, 10)] };
        let mut r = defaults(&P3);
        r[0] = drag(&r[0], Grip::Move, (0, 0), (0, 2)); // A dropped onto C
        push_aside(&mut r, Some(0));
        assert_eq!((r[0].y, r[0].h), (2, 2), "the held block stays where it was put");
        assert_eq!((r[2].y, r[2].h), (4, 8), "C goes down below it, shorter to fit");
        assert_eq!((r[1].x, r[1].y), (6, 0), "B wasn't in the way");
        for i in 0..r.len() {
            for j in 0..r.len() {
                assert!(i == j || !overlaps(&r[i], &r[j]), "{} and {} overlap", r[i].id, r[j].id);
            }
        }
        let mut t = defaults(&P3);
        t[1].x = 3; // B half over A: tidied
        push_aside(&mut t, None);
        assert!(!overlaps(&t[0], &t[1]));
    }

    #[test]
    fn the_look_steps_round_to_its_own() {
        let mut l = Look::default();
        for _ in 0..CORNERS.len() {
            l = next_look(&l, LookPart::Corners);
        }
        assert_eq!(l, Look::default());
        let l = next_look(&Look::default(), LookPart::Font);
        assert_eq!(l.face(), Some("Noto Serif"));
        assert!(card_style(&crate::theme::load(), &Look::default()).is_none(), "its own look: no card");
    }

    #[test]
    fn the_catalog_names_every_block() {
        let c = catalog(&[PAGE]);
        assert_eq!(c["pages"][0]["blocks"][1]["id"], "list");
        assert_eq!(c["cols"], 12);
    }
}
