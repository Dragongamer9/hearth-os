//! Drag and drop in every Hearth app (drag-drop board 3, "The same in every app"; board 2's side panel for any part).
//! The rules are in studio_model.rs; this is how it looks and how it's done with the pointer, built once here so every
//! Hearth app made with hearth-ui has all of it.
//!
//! Edit layout puts the app's window into editing: EDITING by its name, every part of it outlined (dashed, accent)
//! with a label of what it is and its size ("COMMAND BAR · 48 px", "SIDEBAR · 216 px · DRAG ITS EDGE"), the app's own
//! blocks window ("Files' blocks", "drag into Files": Find a block, All / Buttons / Panes / Info / Your own, its
//! buttons, its panes, MAKE YOUR OWN) and the toolbar (Undo, Redo, Snap to grid · 8 px, Guides, Layers, Start from,
//! Menus…, Cancel, Try it, Keep this). A block is dragged into any bar (a lit line where it lands) or anywhere in the
//! window; a placed block is dragged anywhere else; a part is dragged anywhere in its page (not only to its sides) and
//! sized by its edges, with a guide and a note when it lines up with something ("Preview: 514 px · lined up with
//! Search"). Right-click any part › Customize opens board 2's side panel for it (Look, Shows, Does, Place, When, Motion),
//! and every setting there is applied to the part as you go.
//!
//! Nothing is written until Keep this (hearth-layout app-keep: settings_file, atomic, a .bak, merged); Cancel and Esc
//! leave everything as it was kept; Try it uses the app as it would be, still without keeping it. What's kept can never
//! stop an app from starting: a page whose kept part can't be read is the app's own page.
//!
//! Light at rest: none of this is drawn or measured unless the app is being edited or has something kept here.

use crate::blocks::{self, PageDef};
use crate::describe::{DMsg, Describe};
use crate::run;
use crate::studio_model::{self as sm, Add, Anchors, Draft, PageDraft, Session, Side, Style, R};
use crate::theme::Pal;
use crate::ui::{self, t, tm, ts, G};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::operation::Operation;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{overlay, Clipboard, Renderer as _, Shell, Widget};
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, opaque, pin, row, scrollable, slider, stack, text_input, Column, Row, Space};
use iced::{keyboard, mouse, Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Task, Theme, Vector};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The guides' blue (board 3: the line and the note when an edge lines up).
pub const BLUE: Color = Color::from_rgb(0.36, 0.62, 0.95);

// ------------------------------------------------------------------ each app's own blocks

/// One block in the app's blocks window: its id (the app's action, pane or info by that id), its name, its icon.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: G,
}

pub const fn tile(id: &'static str, title: &'static str, icon: G) -> Tile {
    Tile { id, title, icon }
}

/// One of the app's menus, for Menus…: its id (the zone "menu:<id>"), its name, its items (id, name) in their order.
#[derive(Debug, Clone, Copy)]
pub struct MenuDef {
    pub id: &'static str,
    pub title: &'static str,
    pub items: &'static [(&'static str, &'static str)],
}

/// What an app has to drag in (its blocks window): its buttons (its own actions), its panes (its own views), its info
/// (its own values), the page panes go in, the bars that hand over their pieces one by one (so a block lands between
/// them), and its menus.
#[derive(Debug, Clone, Copy)]
pub struct Kit {
    /// as the blocks window says it: "Files" ("Files' blocks", "drag into Files")
    pub name: &'static str,
    pub buttons: &'static [Tile],
    pub panes: &'static [Tile],
    pub info: &'static [Tile],
    pub pane_page: &'static str,
    pub bars: &'static [(&'static str, &'static str)],
    pub menus: &'static [MenuDef],
}

/// The right-click menu every part has (outside editing), for Menus…
pub const PART_MENU: MenuDef = MenuDef { id: "part", title: "The right-click menu on any part", items: &[("customize", "Customize…"), ("edit-layout", "Edit layout"), ("hide-this", "Hide this"), ("describe-it", "Describe it…")] };

impl Kit {
    fn possessive(&self) -> String {
        if self.name.ends_with('s') { format!("{}'", self.name) } else { format!("{}'s", self.name) }
    }
    fn button(&self, id: &str) -> Option<&'static Tile> {
        self.buttons.iter().find(|t| t.id == id)
    }
    fn tile_of(&self, block: &str) -> Option<&'static Tile> {
        match block.split_once(':') {
            Some(("act", id)) => self.buttons.iter().find(|t| t.id == id),
            Some(("pane", id)) => self.panes.iter().find(|t| t.id == id),
            Some(("info", id)) => self.info.iter().find(|t| t.id == id),
            _ => None,
        }
    }
}

/// The glyph by its name (studio_model ICONS).
pub fn glyph_of(name: &str) -> Option<G> {
    Some(match name {
        "check" => G::Check,
        "arrow" => G::Arrow,
        "plus" => G::Plus,
        "close" => G::Close,
        "search" => G::Search,
        "gear" => G::Gear,
        "chevron" => G::Chevron,
        "dots" => G::Dots,
        "folder" => G::Folder,
        "leaf" => G::Leaf,
        "stop" => G::Stop,
        "play" => G::Play,
        "grid" => G::Grid,
        "pulse" => G::Pulse,
        "list" => G::List,
        "clock" => G::Clock,
        "users" => G::Users,
        "rocket" => G::Rocket,
        "cog" => G::Cog,
        "down" => G::Down,
        "home" => G::Home,
        "wifi" => G::Wifi,
        "bt" => G::Bt,
        "display" => G::Display,
        "palette" => G::Palette,
        "power" => G::Power,
        "shield" => G::Shield,
        "user" => G::User,
        "pad" => G::Pad,
        "server" => G::Server,
        "link" => G::Link,
        "update" => G::Update,
        "access" => G::Access,
        "store" => G::Store,
        "sound" => G::Sound,
        "drive" => G::Drive,
        "moon" => G::Moon,
        "info" => G::Info,
        "camera" => G::Camera,
        "keyboard" => G::Keyboard,
        "mouse" => G::Mouse,
        "menu" => G::Menu,
        "doc" => G::Doc,
        "columns" => G::Columns,
        "asterisk" => G::Asterisk,
        "sync" => G::Sync,
        "pencil" => G::Pencil,
        "help" => G::Help,
        "bell" => G::Bell,
        "tasks" => G::Tasks,
        "reliability" => G::Reliability,
        _ => return None,
    })
}

// ------------------------------------------------------------------ where everything was drawn

/// One thing drawn and where (window coordinates): kind 0 a page (id ""), 1 one of its parts, 2 a block dragged in
/// (id its uid, `part` its bar), 3 a piece of a bar (id the piece's, `part` its bar).
#[derive(Debug, Clone)]
pub struct Rec {
    pub at: Instant,
    pub kind: u8,
    pub page: &'static str,
    pub part: &'static str,
    pub id: String,
    pub title: String,
    pub rect: Rectangle,
}

/// Where the parts were drawn the last time the window being edited was drawn (only while it's edited): what was
/// drawn in the last frame (a part no longer drawn is passed over).
#[derive(Debug, Default)]
pub struct Geo {
    pub recs: Vec<Rec>,
}

fn rr(r: Rectangle) -> R {
    R::new(r.x, r.y, r.width, r.height)
}
fn rect(r: R) -> Rectangle {
    Rectangle { x: r.x, y: r.y, width: r.w.max(0.0), height: r.h.max(0.0) }
}

impl Geo {
    fn put(&mut self, kind: u8, page: &'static str, part: &'static str, id: &str, title: &str, r: Rectangle) {
        let g = Instant::now();
        match self.recs.iter_mut().find(|x| x.kind == kind && x.page == page && x.part == part && x.id == id) {
            Some(x) => {
                x.at = g;
                x.rect = r;
                if x.title != title {
                    x.title = title.to_string();
                }
            }
            None => self.recs.push(Rec { at: g, kind, page, part, id: id.to_string(), title: title.to_string(), rect: r }),
        }
    }
    fn now(&self) -> impl Iterator<Item = &Rec> {
        let last = self.recs.iter().map(|r| r.at).max();
        self.recs.iter().filter(move |r| last.map(|l| l.duration_since(r.at) < Duration::from_millis(60)).unwrap_or(false) && r.rect.width > 0.5 && r.rect.height > 0.5)
    }
    /// The outermost page drawn (the one editing sits over).
    pub fn root(&self) -> Option<&'static str> {
        self.now().filter(|r| r.kind == 0).max_by(|a, b| rr(a.rect).area().total_cmp(&rr(b.rect).area())).map(|r| r.page)
    }
    pub fn page(&self, page: &str) -> Option<R> {
        self.now().find(|r| r.kind == 0 && r.page == page).map(|r| rr(r.rect))
    }
    pub fn part(&self, page: &str, id: &str) -> Option<R> {
        self.now().find(|r| r.kind == 1 && r.page == page && r.id == id).map(|r| rr(r.rect))
    }
    pub fn add(&self, page: &str, uid: &str) -> Option<R> {
        self.now().find(|r| r.kind == 2 && r.page == page && r.id == uid).map(|r| rr(r.rect))
    }
    fn pages(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = Vec::new();
        for r in self.now().filter(|r| r.kind == 0) {
            if !v.contains(&r.page) {
                v.push(r.page);
            }
        }
        v
    }
    /// A part that holds a whole other page (Files' panes, with the sidebar, folder and preview in them).
    fn holder(&self, r: &Rec) -> bool {
        let me = rr(r.rect);
        self.now().any(|o| o.kind == 0 && o.page != r.page && me.holds(&rr(o.rect)) || (o.kind == 0 && o.page != r.page && (rr(o.rect).w - me.w).abs() < 2.0 && (rr(o.rect).h - me.h).abs() < 2.0 && (o.rect.x - me.x).abs() < 2.0 && (o.rect.y - me.y).abs() < 2.0))
    }
    fn hash(&self) -> u64 {
        let mut h: u64 = 1469598103934665603;
        for r in self.now() {
            for v in [r.kind as i64, r.rect.x as i64, r.rect.y as i64, r.rect.width as i64, r.rect.height as i64, r.id.len() as i64] {
                h = (h ^ v as u64).wrapping_mul(1099511628211);
            }
        }
        h
    }
    /// The lines an edge can line up with: every other part's, block's and bar piece's left and right edges (or top
    /// and bottom), with what it is.
    fn lines(&self, across: bool, skip: &str) -> Vec<(f32, String)> {
        let mut v = Vec::new();
        for r in self.now().filter(|r| r.kind != 0 && r.id != skip) {
            let name = short(&r.title);
            if across {
                v.push((r.rect.x, name.clone()));
                v.push((r.rect.x + r.rect.width, name));
            } else {
                v.push((r.rect.y, name.clone()));
                v.push((r.rect.y + r.rect.height, name));
            }
        }
        v
    }
}

/// "The navigation pane" → "Navigation pane" (labels and notes).
fn short(title: &str) -> String {
    let s = title.strip_prefix("The ").or_else(|| title.strip_prefix("the ")).unwrap_or(title);
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Records where its content was drawn (while the window is being edited).
struct Spot<'a, M> {
    content: Element<'a, M>,
    geo: &'a RefCell<Geo>,
    kind: u8,
    page: &'static str,
    part: &'static str,
    id: String,
    title: String,
}

fn spot<'a, M: 'a>(d: &'a Describe, kind: u8, page: &'static str, part: &'static str, id: &str, title: &str, content: Element<'a, M>) -> Element<'a, M> {
    if d.st.is_none() {
        return content;
    }
    Element::new(Spot { content, geo: &d.geo, kind, page, part, id: id.to_string(), title: title.to_string() })
}

impl<'a, M: 'a> Widget<M, Theme, Renderer> for Spot<'a, M> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        if let Ok(mut g) = self.geo.try_borrow_mut() {
            g.put(self.kind, self.page, self.part, &self.id, &self.title, layout.bounds());
        }
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport)
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport)
    }
    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

// ------------------------------------------------------------------ parts anywhere: a page laid out by its parts' places

/// Where one child of [`Free`] goes.
#[derive(Debug, Clone, Copy)]
enum Place {
    /// by its four edges, with a width or height it keeps (Look › Width: Fixed)
    At(Anchors, Option<f32>, Option<f32>),
    /// the app's grid place, until it's moved (x, y, w, h of 12)
    Grid(u8, u8, u8, u8),
}

impl Place {
    fn resolve(&self, w: f32, h: f32) -> R {
        match *self {
            Place::At(a, fw, fh) => {
                let mut r = a.resolve(w, h);
                if let Some(fw) = fw {
                    r.w = fw.min(w - r.x).max(sm::MIN.min(w));
                }
                if let Some(fh) = fh {
                    r.h = fh.min(h - r.y).max(sm::MIN.min(h));
                }
                r
            }
            Place::Grid(x, y, gw, gh) => {
                let c = |v: u8, len: f32, n: u8| (v as f32 * len / n as f32).round();
                let x0 = c(x, w, blocks::COLS);
                let y0 = c(y, h, blocks::ROWS);
                R::new(x0, y0, c(x + gw, w, blocks::COLS) - x0, c(y + gh, h, blocks::ROWS) - y0)
            }
        }
    }
}

/// Children anywhere in it, each at its place (worked out when it's laid out, so a window resized keeps every part
/// where its edges say), drawn in their order (later on top).
struct Free<'a, M> {
    kids: Vec<Element<'a, M>>,
    places: Vec<Place>,
    height: Length,
}

impl<'a, M: 'a> Widget<M, Theme, Renderer> for Free<'a, M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, self.height)
    }
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(Length::Fill).height(self.height);
        let size = limits.resolve(Length::Fill, self.height, Size::ZERO);
        let nodes = self
            .kids
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(self.places.iter())
            .map(|((k, t), p)| {
                let r = p.resolve(size.width, size.height);
                let s = Size::new(r.w.max(0.0), r.h.max(0.0));
                k.as_widget_mut().layout(t, renderer, &layout::Limits::new(s, s)).move_to(Point::new(r.x, r.y))
            })
            .collect();
        layout::Node::with_children(size, nodes)
    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        let Some(vp) = layout.bounds().intersection(viewport) else { return };
        let ls: Vec<Layout<'_>> = layout.children().collect();
        let n = self.kids.len().min(ls.len()).min(tree.children.len());
        // the pointer only for what's on top where it is (like iced's stack)
        let mut top = None;
        for i in (0..n).rev() {
            if cursor.is_over(ls[i].bounds()) && self.kids[i].as_widget().mouse_interaction(&tree.children[i], ls[i], cursor, &vp, renderer) != mouse::Interaction::None {
                top = Some(i);
                break;
            }
        }
        for i in 0..n {
            let c = match top {
                Some(j) if i < j => mouse::Cursor::Unavailable,
                _ => cursor,
            };
            let (k, t, l) = (&self.kids[i], &tree.children[i], ls[i]);
            if i == 0 {
                k.as_widget().draw(t, renderer, theme, style, l, c, &vp);
            } else {
                renderer.with_layer(vp, |r| k.as_widget().draw(t, r, theme, style, l, c, &vp));
            }
        }
    }
    fn children(&self) -> Vec<Tree> {
        self.kids.iter().map(Tree::new).collect()
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.kids);
    }
    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.kids.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()).for_each(|((k, t), l)| k.as_widget_mut().operate(t, l, renderer, operation));
        });
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        let mut cursor = cursor;
        let ls: Vec<Layout<'_>> = layout.children().collect();
        let n = self.kids.len().min(ls.len()).min(tree.children.len());
        for i in (0..n).rev() {
            let l = ls[i];
            self.kids[i].as_widget_mut().update(&mut tree.children[i], event, l, cursor, renderer, clipboard, shell, viewport);
            if shell.is_event_captured() {
                return;
            }
            if cursor.is_over(l.bounds()) && !cursor.is_levitating() && self.kids[i].as_widget().mouse_interaction(&tree.children[i], l, cursor, viewport, renderer) != mouse::Interaction::None {
                cursor = cursor.levitate();
            }
        }
    }
    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let ls: Vec<Layout<'_>> = layout.children().collect();
        let n = self.kids.len().min(ls.len()).min(tree.children.len());
        for i in (0..n).rev() {
            let m = self.kids[i].as_widget().mouse_interaction(&tree.children[i], ls[i], cursor, viewport, renderer);
            if m != mouse::Interaction::None {
                return m;
            }
        }
        mouse::Interaction::None
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        overlay::from_children(&mut self.kids, tree, layout, renderer, viewport, translation)
    }
}

// ------------------------------------------------------------------ a part or block in its look (board 2's Look, When, Does, Motion)

/// How a part or block is drawn, worked out from its [`Style`] once per view.
#[derive(Debug, Clone, Copy, Default)]
struct Paint {
    bg: Option<Color>,
    radius: f32,
    border: Option<(Color, f32)>,
    shadow: Option<Shadow>,
    hover: Option<(Color, Option<Color>)>,
    pressed: Option<Color>,
    /// the page's own colour (a fade-in starts from it)
    page: Color,
}

/// A part or block in its look: its size (Fit, Fixed, Fill), the space around it and inside it, its fill, see-through,
/// corners, edge, shadow, the look when it's pointed at or pressed, shown only on hover (When), what a press, right-click,
/// middle-click, scroll or hover does (Does), and how it appears and bounces (Motion).
struct Dress<'a, M> {
    content: Element<'a, M>,
    width: Length,
    height: Length,
    around: Padding,
    inside: Padding,
    paint: Paint,
    hover_only: bool,
    /// press, right, middle, scroll, hover
    on: [Option<M>; 5],
    motion: &'static str,
    bounce: bool,
    stamp: u64,
    editing: bool,
}

#[derive(Default)]
struct DressState {
    hovered: bool,
    pressed: bool,
    born: Option<Instant>,
    stamp: u64,
    bumped: Option<Instant>,
}

const APPEAR: f32 = 0.22;
const BOUNCE: f32 = 0.45;

impl<'a, M: Clone + 'a> Widget<M, Theme, Renderer> for Dress<'a, M> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<DressState>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(DressState { stamp: self.stamp, ..DressState::default() })
    }
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let pad = Padding { top: self.around.top + self.inside.top, right: self.around.right + self.inside.right, bottom: self.around.bottom + self.inside.bottom, left: self.around.left + self.inside.left };
        let content = &mut self.content;
        let child = &mut tree.children[0];
        layout::padded(limits, self.width, self.height, pad, |l| content.as_widget_mut().layout(child, renderer, l))
    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        let st = tree.state.downcast_ref::<DressState>();
        let b = layout.bounds();
        let inner = Rectangle { x: b.x + self.around.left, y: b.y + self.around.top, width: (b.width - self.around.left - self.around.right).max(0.0), height: (b.height - self.around.top - self.around.bottom).max(0.0) };
        let hovered = cursor.is_over(inner);
        if self.hover_only && !hovered && !self.editing {
            return;
        }
        let now = Instant::now();
        let t_in = st.born.map(|b| (now.duration_since(b).as_secs_f32() / APPEAR).min(1.0)).unwrap_or(if self.motion.is_empty() || self.motion == "none" { 1.0 } else { 0.0 });
        let lift = st.bumped.map(|b| now.duration_since(b).as_secs_f32() / BOUNCE).filter(|x| *x < 1.0).map(|x| -(x * std::f32::consts::PI).sin() * 6.0).unwrap_or(0.0);
        let (dy, scale) = match self.motion {
            "slide" => ((1.0 - t_in) * 14.0, 1.0),
            "pop" => (0.0, 0.85 + 0.15 * t_in),
            _ => (0.0, 1.0),
        };
        let draw_all = |r: &mut Renderer| {
            let mut bg = self.paint.bg;
            let mut border = self.paint.border;
            if st.pressed && hovered {
                if let Some(c) = self.paint.pressed {
                    bg = Some(c);
                }
            } else if hovered {
                if let Some((c, line)) = self.paint.hover {
                    bg = Some(c);
                    if line.is_some() {
                        border = line.map(|l| (l, border.map(|b| b.1).unwrap_or(1.0)));
                    }
                }
            }
            if bg.is_some() || border.is_some() || self.paint.shadow.is_some() {
                let (bc, bw) = border.unwrap_or((Color::TRANSPARENT, 0.0));
                r.fill_quad(
                    renderer::Quad { bounds: inner, border: Border { color: bc, width: bw, radius: self.paint.radius.into() }, shadow: self.paint.shadow.unwrap_or_default(), snap: true },
                    Background::Color(bg.unwrap_or(Color::TRANSPARENT)),
                );
            }
            self.content.as_widget().draw(&tree.children[0], r, theme, style, layout.children().next().unwrap_or(layout), cursor, viewport);
            if self.motion == "fade" && t_in < 1.0 {
                r.with_layer(inner, |r2| r2.fill_quad(renderer::Quad { bounds: inner, ..renderer::Quad::default() }, Background::Color(Pal::a(self.paint.page, 1.0 - t_in))));
            }
        };
        if dy != 0.0 || lift != 0.0 || scale != 1.0 {
            let c = Vector::new(inner.x + inner.width / 2.0, inner.y + inner.height / 2.0);
            let tr = iced::Transformation::translate(c.x, c.y + dy + lift) * iced::Transformation::scale(scale) * iced::Transformation::translate(-c.x, -c.y);
            renderer.with_transformation(tr, |r| draw_all(r));
        } else {
            draw_all(renderer);
        }
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        if let Some(l) = layout.children().next() {
            self.content.as_widget_mut().operate(&mut tree.children[0], l, renderer, operation)
        }
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        let b = layout.bounds();
        let over = cursor.is_over(b);
        {
            let st = tree.state.downcast_mut::<DressState>();
            if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
                let now = Instant::now();
                if st.born.is_none() {
                    st.born = Some(now);
                    if !self.motion.is_empty() && self.motion != "none" {
                        shell.request_redraw();
                    }
                } else if st.born.map(|x| now.duration_since(x).as_secs_f32() < APPEAR).unwrap_or(false) {
                    shell.request_redraw();
                }
                if self.bounce && st.stamp != self.stamp {
                    st.stamp = self.stamp;
                    st.bumped = Some(now);
                }
                if st.bumped.map(|x| now.duration_since(x).as_secs_f32() < BOUNCE).unwrap_or(false) {
                    shell.request_redraw();
                }
            }
            if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                if over != st.hovered {
                    st.hovered = over;
                    if !self.editing {
                        if over {
                            if let Some(m) = &self.on[4] {
                                shell.publish(m.clone());
                            }
                        }
                        if self.paint.hover.is_some() || self.hover_only {
                            shell.request_redraw();
                        }
                    }
                }
            }
        }
        if !self.editing && over {
            // its own right-click, middle-click and scroll, when it has them, before the part's own
            let mine = match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => self.on[1].clone(),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => self.on[2].clone(),
                Event::Mouse(mouse::Event::WheelScrolled { .. }) => self.on[3].clone(),
                _ => None,
            };
            if let Some(m) = mine {
                shell.publish(m);
                shell.capture_event();
                return;
            }
        }
        if let Some(l) = layout.children().next() {
            self.content.as_widget_mut().update(&mut tree.children[0], event, l, cursor, renderer, clipboard, shell, viewport);
        }
        let st = tree.state.downcast_mut::<DressState>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => {
                st.pressed = true;
                if self.paint.pressed.is_some() {
                    shell.request_redraw();
                }
                if !self.editing && !shell.is_event_captured() {
                    if let Some(m) = &self.on[0] {
                        shell.publish(m.clone());
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if st.pressed {
                    st.pressed = false;
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }
    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let own = layout.children().next().map(|l| self.content.as_widget().mouse_interaction(&tree.children[0], l, cursor, viewport, renderer)).unwrap_or_default();
        if own == mouse::Interaction::None && !self.editing && self.on[0].is_some() && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }
        own
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        let l = layout.children().next()?;
        self.content.as_widget_mut().overlay(&mut tree.children[0], l, renderer, viewport, translation)
    }
}

/// What a block starts as (a button looks like one of Hearth's buttons until it's given its own look).
#[derive(Clone, Copy, PartialEq)]
enum Base {
    Part,
    Button,
    Plain,
    Pane,
}

fn paint(p: &Pal, s: &Style, base: Base) -> Paint {
    let see = 1.0 - s.see as f32 / 100.0;
    let own_bg = match base {
        Base::Pane => Some(p.panel),
        _ => None,
    };
    let mut bg = match blocks::colour(&s.fill) {
        Some(c) => Some(Pal::a(c, see)),
        None if s.see > 0 => Some(Pal::a(own_bg.unwrap_or(p.panel), see)),
        None => own_bg,
    };
    if s.blur {
        // iced can't blur what's under it: frosted instead (a lighter, less see-through fill)
        let b = bg.unwrap_or(Pal::a(p.panel, 0.6));
        bg = Some(Color { a: b.a.max(0.82), ..Pal::mix(b, p.text, 0.06) });
    }
    let radius = s.corners.map(|c| c as f32).unwrap_or(match base {
        Base::Button => p.r(6.0),
        Base::Pane => p.r(10.0),
        _ => 0.0,
    });
    let edge_c = blocks::colour(&s.edge_colour);
    let edge_w = if s.edge_w > 0 { s.edge_w as f32 } else { 1.0 };
    let border = match s.edge {
        "none" => None,
        "line" => Some((edge_c.unwrap_or(p.line), edge_w)),
        "glow" => Some((edge_c.unwrap_or(p.accent), edge_w)),
        _ => match base {
            Base::Button | Base::Pane => Some((p.line, 1.0)),
            _ => None,
        },
    };
    let mut shadow = match s.shadow {
        "soft" => Some(Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.35), offset: Vector::new(0.0, 4.0), blur_radius: 12.0 }),
        "deep" => Some(Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.55), offset: Vector::new(0.0, 10.0), blur_radius: 28.0 }),
        _ => None,
    };
    if s.edge == "glow" {
        shadow = Some(Shadow { color: Pal::a(edge_c.unwrap_or(p.accent), 0.7), offset: Vector::new(0.0, 0.0), blur_radius: 14.0 });
    }
    let lit = |c: Option<Color>, k: f32| c.map(|c| Pal::mix(c, Color::WHITE, k)).unwrap_or(Pal::a(p.text, k * 0.8));
    let hover = match s.hover {
        "same" => None,
        "lighter" => Some((lit(bg, 0.08), None)),
        "accent" => Some((Pal::a(p.accent, 0.20), Some(p.accent))),
        "own" => blocks::colour(&s.hover_fill).map(|c| (c, None)),
        _ => match base {
            Base::Button => Some((p.hover(), None)),
            _ => None,
        },
    };
    let pressed = match s.pressed {
        "same" => None,
        "darker" => Some(bg.map(|c| Pal::mix(c, Color::BLACK, 0.25)).unwrap_or(Color::from_rgba(0.0, 0.0, 0.0, 0.22))),
        "own" => blocks::colour(&s.pressed_fill),
        _ => match base {
            Base::Button => Some(Pal::mix(p.hover(), Color::BLACK, 0.15)),
            _ => None,
        },
    };
    Paint { bg, radius, border, shadow, hover, pressed, page: p.bg }
}

/// What a press (or right-click, middle-click, scroll, hover) is set to do, as a message (None: its own).
fn does<M>(s: &Style, i: usize, mp: &Mp<'_, M>) -> Option<M> {
    let a = s.does[i].as_str();
    if a.is_empty() {
        return None;
    }
    Some(mp(DMsg::Studio(SMsg::Do(a.to_string()))))
}

type Mp<'a, M> = Rc<dyn Fn(DMsg) -> M + 'a>;

#[allow(clippy::too_many_arguments)]
fn dress<'a, M: Clone + 'a>(p: &Pal, s: &Style, base: Base, content: Element<'a, M>, press: Option<M>, mp: &Mp<'a, M>, editing: bool, stamp: u64, showing: bool) -> Element<'a, M> {
    let width = match s.wmode {
        "fixed" if s.w > 0 => Length::Fixed(s.w as f32 + s.around[1] as f32 + s.around[3] as f32),
        "fill" => Length::Fill,
        "fit" => Length::Shrink,
        _ => match base {
            Base::Part | Base::Pane => Length::Fill,
            _ => Length::Shrink,
        },
    };
    let height = if s.h > 0 {
        Length::Fixed(s.h as f32 + s.around[0] as f32 + s.around[2] as f32)
    } else {
        match base {
            Base::Part | Base::Pane => Length::Fill,
            _ => Length::Shrink,
        }
    };
    let inside = match (s.inside, base) {
        (Some(i), _) => Padding::from(i as f32),
        (None, Base::Button) => Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 },
        (None, Base::Pane) => Padding::from(12.0),
        _ => Padding::ZERO,
    };
    let around = Padding { top: s.around[0] as f32, right: s.around[1] as f32, bottom: s.around[2] as f32, left: s.around[3] as f32 };
    let mut on: [Option<M>; 5] = [does(s, 0, mp).or(press), does(s, 1, mp), does(s, 2, mp), does(s, 3, mp), does(s, 4, mp)];
    if s.does[0] == "none" {
        on[0] = None;
    }
    let content = if showing { content } else { Space::new().into() };
    Element::new(Dress { content, width, height, around, inside, paint: paint(p, s, base), hover_only: s.when == "hover", on, motion: s.motion, bounce: s.bounce, stamp, editing })
}

// ------------------------------------------------------------------ editing: the state

/// What's picked: one of a page's own parts, or a block dragged in (by its uid).
#[derive(Debug, Clone, PartialEq)]
pub enum Sel {
    Part(&'static str, &'static str),
    Add(&'static str, String),
}

impl Sel {
    pub fn page(&self) -> &'static str {
        match self {
            Sel::Part(p, _) | Sel::Add(p, _) => p,
        }
    }
    fn key(&self) -> String {
        match self {
            Sel::Part(p, id) => format!("{p}/{id}"),
            Sel::Add(p, u) => format!("{p}/+{u}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pop {
    None,
    Layers,
    Start,
    Menus,
}

/// A picker opened in the side panel (its list under its row).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expand {
    None,
    Icon,
    Font,
    Open(usize),
    Panel(usize),
    Source,
    Pages,
}

#[derive(Debug, Clone)]
pub enum DragKind {
    /// a block from the blocks window: what it is, its name
    Tile(String, String),
    Add(&'static str, String),
    Part(&'static str, &'static str),
    Side(Sel, Side),
    Win,
}

/// Where a block being dragged would land.
#[derive(Debug, Clone)]
pub enum Target {
    /// in a bar, after one of its pieces ("" at its start); the lit line, and what the note says
    Bar { page: &'static str, part: &'static str, after: String, line: Rectangle, say: String },
    /// free in a page, at this place in it (and where that is on the screen)
    Free { page: &'static str, at: R, abs: Rectangle },
}

#[derive(Debug, Clone)]
pub struct Guide {
    pub line: Rectangle,
    pub note: String,
    pub at: Point,
}

#[derive(Debug, Clone)]
pub struct Drag {
    pub kind: DragKind,
    pub from: (f32, f32),
    pub before: Draft,
    pub moved: bool,
    /// what's dragged, as it was picked up (window coordinates), and its page
    pub start: R,
    pub page: R,
    pub target: Option<Target>,
    pub guide: Option<Guide>,
}

/// One time editing the app: the model's [`Session`] and how it's shown.
pub struct Studio {
    pub s: Session,
    /// the page the editing sits over (the app's outermost page), and the one keys act on
    pub root: &'static str,
    pub focus: &'static str,
    pub pick: Option<Sel>,
    pub panel: bool,
    pub tab: usize,
    pub menu: Option<(Sel, f32, f32)>,
    pub moveto: bool,
    pub drag: Option<Drag>,
    pub pointer: (f32, f32),
    /// the blocks window's place over the root page (None: its own, on the right)
    pub win: Option<(f32, f32)>,
    pub win_open: bool,
    pub find: String,
    pub kind: usize,
    pub pop: Pop,
    pub expand: Expand,
    pub trying: bool,
    pub start: &'static str,
    pub look: Option<Style>,
    pub note: String,
    pub bad: bool,
    /// Keep this asked hearth-layout to write (the only write; tests count it)
    pub writes: u32,
    pub own_name: String,
    pub last_key: String,
    pub busy: bool,
    /// what the geometry was when the view was last made (a change makes it again)
    pub seen: u64,
    pub refresh: bool,
    pub staged: bool,
}

/// What editing is told (inside [`DMsg::Studio`]).
#[derive(Debug, Clone)]
pub enum SMsg {
    /// Edit layout: the page in front (or this one), this part picked
    Start(Option<&'static str>, Option<&'static str>),
    Tick,
    Refresh,
    Press(Sel, f32, f32),
    PressNothing,
    Move(f32, f32),
    Release,
    RMenu(Sel, f32, f32),
    CloseMenu,
    MoveToOpen,
    GrabTile(String, String),
    GrabSide(Sel, Side),
    GrabWin,
    Find(String),
    Kind(usize),
    WinToggle,
    Undo,
    Redo,
    Snap,
    Grid,
    Guides,
    Pop(Pop),
    Layer(Sel, i16),
    Eye(Sel),
    StartFrom(&'static str),
    MenuMove(&'static str, usize, i32),
    MenuHide(&'static str, String),
    Cancel,
    Try,
    Back,
    Keep,
    Kept(run::Output),
    Customize(Sel),
    ClosePanel,
    Tab(usize),
    Expand(Expand),
    Set(Sel, F),
    Reset(Sel),
    CopyLook(Sel),
    PasteLook(Sel),
    Duplicate(Sel),
    MoveTo(Sel, String),
    Hide(Sel),
    TakeOut(Sel),
    OwnName(String),
    SaveOwn(Sel),
    ApplyAll(Sel),
    /// one of the app's own buttons pressed (the app runs it: [`act`])
    Act(&'static str),
    /// what a part's Does says: act:…, run:…, panel:…
    Do(String),
    /// Hide this (right-click, not editing): kept straight away, with Undo
    HideNow(&'static str, &'static str),
    Key(K),
    /// CI's pictures: the window as it is while a block is dragged, a pane sized, a part customized
    Stage(&'static str),
}

/// A key while editing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum K {
    Esc,
    Undo,
    Redo,
    Delete,
    Next,
    Prev,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Wider,
    Narrower,
    Taller,
    Shorter,
}

/// One setting of board 2's side panel.
#[derive(Debug, Clone)]
pub enum F {
    WMode(&'static str),
    W(String),
    H(String),
    Around(usize, String),
    Inside(f32),
    Fill(String),
    See(f32),
    Blur(bool),
    Corners(f32),
    Edge(&'static str),
    EdgeColour(String),
    EdgeW(String),
    Shadow(&'static str),
    Icon(&'static str),
    IconColour(String),
    IconSize(String),
    Words(String),
    Font(&'static str),
    FontSize(String),
    Weight(&'static str),
    Show(&'static str),
    Hover(&'static str),
    HoverFill(String),
    Pressed(&'static str),
    PressedFill(String),
    Shows(&'static str),
    Source(String),
    Picture(String),
    Does(usize, String),
    Layer(&'static str),
    When(&'static str),
    Page(String),
    Motion(&'static str),
    Bounce(bool),
    /// Place: x, y, width, height in px (in its page)
    X(String),
    Y(String),
    PW(String),
    PH(String),
    /// Place: to a side of its page (an edge is only a snap you can use) or free
    Snap(&'static str),
}

/// A press of one of the app's own buttons (a block dragged in from its blocks window): the app takes it before
/// handing the rest to Describe (`if let Some(id) = studio::act(&dm) { … }`).
pub fn act(m: &DMsg) -> Option<&'static str> {
    match m {
        DMsg::Studio(SMsg::Act(id)) => Some(id),
        _ => None,
    }
}

fn num(s: &str, max: u16) -> Option<u16> {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).take(5).collect();
    if d.is_empty() {
        return Some(0);
    }
    d.parse::<u32>().ok().map(|n| n.min(max as u32) as u16)
}

/// One setting applied to a look (true when it's one the place handles instead).
fn apply(s: &mut Style, f: &F) {
    match f {
        F::WMode(m) => {
            s.wmode = m;
            if *m == "fixed" && s.w == 0 {
                s.w = 160;
            }
        }
        F::W(v) => {
            if let Some(n) = num(v, sm::MAX_PX) {
                s.w = n;
                if n > 0 {
                    s.wmode = "fixed";
                }
            }
        }
        F::H(v) => {
            if let Some(n) = num(v, sm::MAX_PX) {
                s.h = n;
            }
        }
        F::Around(i, v) => {
            if let Some(n) = num(v, 400) {
                if *i < 4 {
                    s.around[*i] = n;
                }
            }
        }
        F::Inside(v) => s.inside = Some(v.round().clamp(0.0, 200.0) as u16),
        F::Fill(c) => {
            if sm::colour_ok(c) {
                s.fill = c.to_lowercase();
            }
        }
        F::See(v) => s.see = v.round().clamp(0.0, 100.0) as u8,
        F::Blur(b) => s.blur = *b,
        F::Corners(v) => s.corners = Some(v.round().clamp(0.0, 48.0) as u8),
        F::Edge(e) => s.edge = e,
        F::EdgeColour(c) => {
            if sm::colour_ok(c) {
                s.edge_colour = c.to_lowercase();
            }
        }
        F::EdgeW(v) => {
            if let Some(n) = num(v, 8) {
                s.edge_w = n as u8;
            }
        }
        F::Shadow(x) => s.shadow = x,
        F::Icon(x) => s.icon = x,
        F::IconColour(c) => {
            if sm::colour_ok(c) {
                s.icon_colour = c.to_lowercase();
            }
        }
        F::IconSize(v) => {
            if let Some(n) = num(v, 96) {
                s.icon_size = n as u8;
            }
        }
        F::Words(w) => s.words = w.chars().take(120).collect(),
        F::Font(x) => s.font = x,
        F::FontSize(v) => {
            if let Some(n) = num(v, 72) {
                s.font_size = n as u8;
            }
        }
        F::Weight(x) => s.weight = x,
        F::Show(x) => s.show = x,
        F::Hover(x) => s.hover = x,
        F::HoverFill(c) => {
            if sm::colour_ok(c) {
                s.hover_fill = c.to_lowercase();
                s.hover = "own";
            }
        }
        F::Pressed(x) => s.pressed = x,
        F::PressedFill(c) => {
            if sm::colour_ok(c) {
                s.pressed_fill = c.to_lowercase();
                s.pressed = "own";
            }
        }
        F::Shows(x) => s.shows = x,
        F::Source(x) => s.source = x.clone(),
        F::Picture(x) => s.picture = x.chars().take(400).filter(|c| *c != '\0').collect(),
        F::Does(i, a) => {
            if *i < 5 && (sm::action_ok(a) || a.starts_with("run:")) {
                s.does[*i] = a.chars().take(404).filter(|c| *c != '\n' && *c != '\r' && *c != '\0').collect();
            }
        }
        F::Layer(x) => s.layer = x,
        F::When(x) => s.when = x,
        F::Page(p) => {
            if let Some(i) = s.pages.iter().position(|x| x == p) {
                s.pages.remove(i);
            } else if s.pages.len() < 32 {
                s.pages.push(p.clone());
            }
            if !s.pages.is_empty() {
                s.when = "pages";
            }
        }
        F::Motion(x) => s.motion = x,
        F::Bounce(b) => s.bounce = *b,
        F::X(_) | F::Y(_) | F::PW(_) | F::PH(_) | F::Snap(_) => {}
    }
}

/// The text fields (each keystroke one step for Undo would be too many: one step a field).
fn typed(f: &F) -> Option<&'static str> {
    Some(match f {
        F::W(_) => "w",
        F::H(_) => "h",
        F::Around(..) => "around",
        F::Fill(_) => "fill",
        F::EdgeColour(_) => "edgec",
        F::EdgeW(_) => "edgew",
        F::IconColour(_) => "iconc",
        F::IconSize(_) => "icons",
        F::Words(_) => "words",
        F::FontSize(_) => "fonts",
        F::HoverFill(_) => "hoverc",
        F::PressedFill(_) => "pressc",
        F::Picture(_) => "pic",
        F::Does(..) => "does",
        F::X(_) | F::Y(_) | F::PW(_) | F::PH(_) => "place",
        F::Inside(_) | F::See(_) | F::Corners(_) => "slider",
        _ => return None,
    })
}

fn style_of(d: &Draft, sel: &Sel) -> Style {
    match sel {
        Sel::Part(p, id) => d.page(p).and_then(|x| x.part(id)).map(|x| x.style.clone()).unwrap_or_default(),
        Sel::Add(p, u) => d.page(p).and_then(|x| x.add(u)).map(|x| x.style.clone()).unwrap_or_default(),
    }
}

fn style_mut<'s>(d: &'s mut Draft, sel: &Sel) -> Option<&'s mut Style> {
    match sel {
        Sel::Part(p, id) => Some(&mut d.page_mut(p).part_mut(id).style),
        Sel::Add(p, u) => d.page_mut(p).add_mut(u).map(|a| &mut a.style),
    }
}

/// The default size of a block dropped free (px).
fn free_size(block: &str, page: R) -> (f32, f32) {
    if block.starts_with("pane:") {
        ((page.w * 0.34).clamp(240.0, 520.0), (page.h * 0.7).clamp(200.0, 720.0))
    } else if block == "own:spacer" {
        (48.0, 32.0)
    } else if block.starts_with("info:") || block == "own:count" || block == "own:words" {
        (180.0, 34.0)
    } else {
        (176.0, 36.0)
    }
}

impl Describe {
    /// The page as it's laid out here: being edited or tried, the draft; else what's kept.
    pub fn studio_page(&self, page: &str) -> Option<&PageDraft> {
        match &self.st {
            Some(st) => st.s.draft.page(page),
            None => self.studio_kept.page(page),
        }
    }

    /// Is the window being edited (Edit layout, not Try it)?
    pub fn studio_editing(&self) -> bool {
        self.st.as_ref().map(|s| !s.trying).unwrap_or(false)
    }

    /// The app's values for its info blocks and {count}-style words, set each time its view is made.
    pub fn values(&self, v: Vec<(&'static str, String)>) {
        if let Ok(mut x) = self.values.try_borrow_mut() {
            *x = v;
        }
    }

    fn value(&self, id: &str) -> String {
        self.values.borrow().iter().find(|x| x.0 == id).map(|x| x.1.clone()).unwrap_or_default()
    }

    /// Does the app need to make this pane (one of its panes is placed in its window, or it's being dragged in)?
    pub fn wants_pane(&self, id: &str) -> bool {
        let block = format!("pane:{id}");
        let page = self.kit.map(|k| k.pane_page).unwrap_or("");
        let placed = self.studio_page(page).map(|p| p.adds.iter().any(|a| a.block == block && !a.hidden)).unwrap_or(false);
        placed && !self.panes_closed.borrow().contains(&block)
    }

    fn grid_pages(&self) -> Vec<&'static str> {
        self.pages.iter().filter(|p| self.kept.pointer(&format!("/layout/blocks/{}", p.id)).is_some()).map(|p| p.id).collect()
    }

    fn end_studio(&mut self) {
        self.st = None;
        self.arr = None;
        if let Ok(mut g) = self.geo.try_borrow_mut() {
            g.recs.clear();
        }
    }

    fn st_start(&mut self, focus: Option<&'static str>, pick: Option<&'static str>) -> Task<DMsg> {
        let shown: Vec<&'static str> = self.ed.shown().into_iter().filter(|p| self.pages.iter().any(|d| d.id == *p)).collect();
        let focus = focus.filter(|p| self.pages.iter().any(|d| d.id == *p)).or(shown.first().copied()).or(self.pages.first().map(|p| p.id));
        let Some(focus) = focus else { return Task::none() };
        if let Some(st) = self.st.as_mut() {
            st.focus = focus;
            st.trying = false;
            if let Some(id) = pick {
                st.pick = Some(Sel::Part(focus, id));
            }
            return Task::none();
        }
        let root = shown.last().copied().unwrap_or(focus);
        self.open = false;
        self.result = None;
        self.error.clear();
        self.said.clear();
        self.ed.menu = None;
        self.ed.note = None;
        let kept = Draft::read(self.kept.get("layout").unwrap_or(&Value::Null));
        self.st = Some(Studio {
            s: Session::new(kept),
            root,
            focus,
            pick: pick.map(|id| Sel::Part(focus, id)),
            panel: false,
            tab: 0,
            menu: None,
            moveto: false,
            drag: None,
            pointer: (0.0, 0.0),
            win: None,
            win_open: true,
            find: String::new(),
            kind: 0,
            pop: Pop::None,
            expand: Expand::None,
            trying: false,
            start: "hearth",
            look: None,
            note: String::new(),
            bad: false,
            writes: 0,
            own_name: String::new(),
            last_key: String::new(),
            busy: false,
            seen: 0,
            refresh: false,
            staged: false,
        });
        let placed = self.laid(focus).cloned().or_else(|| self.pages.iter().find(|p| p.id == focus).map(blocks::defaults)).unwrap_or_default();
        self.arr = Some(crate::describe::Arrange { page: focus, placed, undo: Vec::new(), pick: None, grab: None });
        Task::perform(run::after(90), |_| DMsg::Studio(SMsg::Tick))
    }

    /// Where a block being dragged would land (and the guide when it lines up), from where things were drawn.
    fn target_at(&self, x: f32, y: f32, block: &str, skip: &str) -> (Option<Target>, Option<Guide>) {
        let Some(st) = self.st.as_ref() else { return (None, None) };
        let geo = self.geo.borrow();
        let pane = block.starts_with("pane:");
        // the smallest part under the pointer (not one that holds a whole page)
        let under = geo
            .now()
            .filter(|r| r.kind == 1 && r.rect.contains(Point::new(x, y)) && !geo.holder(r) && self.pages.iter().any(|p| p.id == r.page))
            .min_by(|a, b| rr(a.rect).area().total_cmp(&rr(b.rect).area()))
            .cloned();
        if let (false, Some(u)) = (pane, under.as_ref()) {
            let bar = rr(u.rect);
            if bar.is_bar() {
                let part: &'static str = self.pages.iter().find(|p| p.id == u.page).and_then(|p| p.blocks.iter().find(|b| b.id == u.id)).map(|b| b.id).unwrap_or("");
                let across = bar.across();
                let mut pieces: Vec<&Rec> = geo.now().filter(|r| (r.kind == 3 || r.kind == 2) && r.page == u.page && r.part == part && r.id != skip).collect();
                pieces.sort_by(|a, b| if across { a.rect.x.total_cmp(&b.rect.x) } else { a.rect.y.total_cmp(&b.rect.y) });
                let rs: Vec<R> = pieces.iter().map(|r| rr(r.rect)).collect();
                let strip = !self.kit.map(|k| k.bars.contains(&(u.page, part))).unwrap_or(false);
                let (i, mut line) = sm::drop_at(&rs, across, x, y, bar);
                if strip && rs.is_empty() {
                    line = if across { bar.right() - 10.0 } else { bar.bottom() - 10.0 };
                }
                let after = if i == 0 { String::new() } else { pieces[i - 1].id.clone() };
                let name = short(&u.title).to_lowercase();
                let say = if i == 0 && !strip {
                    format!("Drop: {name}, at the start")
                } else if i == 0 {
                    format!("Drop: {name}, at the end")
                } else {
                    format!("Drop: {name}, after {}", short(&pieces[i - 1].title).to_lowercase())
                };
                let lr = if across { Rectangle { x: line - 1.5, y: bar.y + 6.0, width: 3.0, height: (bar.h - 12.0).max(8.0) } } else { Rectangle { x: bar.x + 6.0, y: line - 1.5, width: (bar.w - 12.0).max(8.0), height: 3.0 } };
                return (Some(Target::Bar { page: u.page, part, after, line: lr, say }), None);
            }
        }
        // free in a page: the pane's own page for a pane, else the page under the pointer
        let page: Option<&'static str> = if pane {
            self.kit.map(|k| k.pane_page).filter(|p| geo.page(p).is_some())
        } else {
            under.as_ref().map(|u| u.page).or_else(|| geo.now().filter(|r| r.kind == 0 && r.rect.contains(Point::new(x, y))).min_by(|a, b| rr(a.rect).area().total_cmp(&rr(b.rect).area())).map(|r| r.page))
        };
        let Some(page) = page else { return (None, None) };
        let Some(pr) = geo.page(page) else { return (None, None) };
        let (w, h) = free_size(block, pr);
        let mut at = sm::dropped(x - pr.x, y - pr.y, w, h, R::new(0.0, 0.0, pr.w, pr.h), st.s.grid());
        let mut guide = None;
        if st.s.guides {
            let lines: Vec<(f32, String)> = geo.lines(true, skip).into_iter().map(|(v, n)| (v - pr.x, n)).collect();
            if let Some((v, n)) = sm::align(at.x, &lines) {
                at.x = v;
                guide = Some((v, n.to_string()));
            } else if let Some((v, n)) = sm::align(at.right(), &lines) {
                at.x = v - at.w;
                guide = Some((v, n.to_string()));
            }
        }
        let abs = Rectangle { x: pr.x + at.x, y: pr.y + at.y, width: at.w, height: at.h };
        let g = guide.map(|(v, n)| Guide { line: Rectangle { x: pr.x + v, y: pr.y, width: 1.0, height: pr.h }, note: format!("Lined up with {n}"), at: Point::new(pr.x + v + 6.0, abs.y + abs.height + 6.0) });
        (Some(Target::Free { page, at, abs }), g)
    }

    /// The first move of a part off where the app puts it: every part of its page gets its place from where it was
    /// drawn (so nothing jumps), and from then on it's laid out by its parts' places.
    fn adopt(&mut self, page: &'static str) {
        let Some(def) = self.pages.iter().find(|p| p.id == page) else { return };
        let geo = self.geo.borrow();
        let Some(pr) = geo.page(page) else { return };
        let Some(st) = self.st.as_mut() else { return };
        if st.s.draft.page(page).map(|p| p.moved()).unwrap_or(false) {
            return;
        }
        let pd = st.s.draft.page_mut(page);
        for b in def.blocks {
            let r = match geo.part(page, b.id) {
                Some(r) => R::new(r.x - pr.x, r.y - pr.y, r.w, r.h),
                None => Place::Grid(b.x, b.y, b.w, b.h).resolve(pr.w, pr.h),
            };
            pd.part_mut(b.id).at = Some(Anchors::from_rect(r, pr.w, pr.h));
        }
    }

    /// Each part of a page where it is now (page-relative), for sizing one against its neighbours.
    fn page_rects(&self, page: &'static str) -> Vec<(&'static str, R)> {
        let Some(def) = self.pages.iter().find(|p| p.id == page) else { return Vec::new() };
        let geo = self.geo.borrow();
        let Some(pr) = geo.page(page) else { return Vec::new() };
        let pd = self.studio_page(page);
        def.blocks
            .iter()
            .filter(|b| !pd.and_then(|p| p.part(b.id)).map(|x| x.hidden).unwrap_or(false))
            .map(|b| {
                let r = geo.part(page, b.id).map(|r| R::new(r.x - pr.x, r.y - pr.y, r.w, r.h)).unwrap_or_else(|| Place::Grid(b.x, b.y, b.w, b.h).resolve(pr.w, pr.h));
                (b.id, r)
            })
            .collect()
    }

    fn sel_title(&self, sel: &Sel) -> String {
        match sel {
            Sel::Part(p, id) => self.pages.iter().find(|d| d.id == *p).and_then(|d| d.blocks.iter().find(|b| b.id == *id)).map(|b| short(b.title)).unwrap_or_default(),
            Sel::Add(p, u) => self.studio_page(p).and_then(|x| x.add(u)).map(|a| a.title.clone()).unwrap_or_default(),
        }
    }

    /// A drag goes on: the pointer is at (x, y).
    fn st_move(&mut self, x: f32, y: f32) {
        let Some(st) = self.st.as_mut() else { return };
        st.pointer = (x, y);
        let Some(dr) = st.drag.as_mut() else { return };
        if matches!(dr.kind, DragKind::Win | DragKind::Tile(..)) && !dr.moved && dr.page.w < 0.0 {
            // picked up from a window of its own: where the pointer is now is where it was picked up
            dr.from = (x, y);
            dr.page.w = 0.0;
            return;
        }
        if !dr.moved && (x - dr.from.0).abs() + (y - dr.from.1).abs() < 4.0 {
            return;
        }
        dr.moved = true;
        let (dx, dy) = (x - dr.from.0, y - dr.from.1);
        match dr.kind.clone() {
            DragKind::Win => {
                let (wx, wy) = st.win.unwrap_or((0.0, 0.0));
                st.win = Some((dr.start.x + dx, dr.start.y + dy));
                let _ = (wx, wy);
            }
            DragKind::Tile(block, _) => {
                let (t, g) = self.target_at(x, y, &block, "");
                if let Some(dr) = self.st.as_mut().and_then(|s| s.drag.as_mut()) {
                    dr.target = t;
                    dr.guide = g;
                }
            }
            DragKind::Add(page, uid) => {
                let block = self.studio_page(page).and_then(|p| p.add(&uid)).map(|a| a.block.clone()).unwrap_or_default();
                let (t, g) = self.target_at(x, y, &block, &uid);
                if let Some(dr) = self.st.as_mut().and_then(|s| s.drag.as_mut()) {
                    dr.target = t;
                    dr.guide = g;
                }
            }
            DragKind::Part(page, id) => {
                self.adopt(page);
                let geo = self.geo.borrow();
                let Some(pr) = geo.page(page) else { return };
                let Some(st) = self.st.as_mut() else { return };
                let Some(dr) = st.drag.as_mut() else { return };
                let start = R::new(dr.start.x - pr.x, dr.start.y - pr.y, dr.start.w, dr.start.h);
                let mut r = sm::moved(start, dx, dy, R::new(0.0, 0.0, pr.w, pr.h), st.s.grid());
                let mut guide = None;
                if st.s.guides {
                    let lines: Vec<(f32, String)> = geo.lines(true, id).into_iter().map(|(v, n)| (v - pr.x, n)).collect();
                    if let Some((v, n)) = sm::align(r.x, &lines) {
                        r.x = v;
                        guide = Some((v, n.to_string()));
                    } else if let Some((v, n)) = sm::align(r.right(), &lines) {
                        r.x = v - r.w;
                        guide = Some((v, n.to_string()));
                    }
                }
                dr.guide = guide.map(|(v, n)| Guide { line: Rectangle { x: pr.x + v, y: pr.y, width: 1.0, height: pr.h }, note: format!("Lined up with {n}"), at: Point::new(pr.x + v + 6.0, pr.y + r.bottom() + 6.0) });
                let a = Anchors::from_rect(r, pr.w, pr.h);
                st.s.live(|d| d.page_mut(page).part_mut(id).at = Some(a));
            }
            DragKind::Side(sel, side) => self.side_to(&sel, side, x, y),
        }
    }

    /// A part's or pane's edge dragged to (x, y): sized (its neighbours with it), on the grid, lined up when near.
    fn side_to(&mut self, sel: &Sel, side: Side, x: f32, y: f32) {
        let page = sel.page();
        let across = matches!(side, Side::Left | Side::Right);
        let (pr, r, lines) = {
            let geo = self.geo.borrow();
            let Some(pr) = geo.page(page) else { return };
            let r = match sel {
                Sel::Part(p, id) => geo.part(p, id),
                Sel::Add(p, u) => geo.add(p, u),
            };
            let Some(r) = r else { return };
            let skip = match sel {
                Sel::Part(_, id) => id.to_string(),
                Sel::Add(_, u) => u.clone(),
            };
            (pr, r, geo.lines(across, &skip))
        };
        let Some(st) = self.st.as_ref() else { return };
        let grid = st.s.grid();
        let guides = st.s.guides;
        let v = if across { x } else { y };
        let origin = if across { pr.x } else { pr.y };
        let mut v = sm::snap(v - origin, grid) + origin;
        let mut with = None;
        if guides {
            if let Some((a, n)) = sm::align(v, &lines) {
                v = a;
                with = Some(n.to_string());
            }
        }
        let title = self.sel_title(sel);
        let moved_page = self.studio_page(page).map(|p| p.moved()).unwrap_or(false);
        let size;
        match sel {
            Sel::Part(_, id) if !moved_page => {
                // where the app puts it: its width (or height) set, the parts beside it make room as they do
                size = match side {
                    Side::Right => v - r.x,
                    Side::Left => r.right() - v,
                    Side::Bottom => v - r.y,
                    Side::Top => r.bottom() - v,
                }
                .round()
                .clamp(sm::MIN, sm::MAX_PX as f32);
                let id: &'static str = *id;
                if let Some(st) = self.st.as_mut() {
                    st.s.live(|d| {
                        let s = &mut d.page_mut(page).part_mut(id).style;
                        if across {
                            s.wmode = "fixed";
                            s.w = size as u16;
                        } else {
                            s.h = size as u16;
                        }
                    });
                }
            }
            Sel::Part(_, id) => {
                let mut rects = self.page_rects(page);
                let Some(i) = rects.iter().position(|x| x.0 == *id) else { return };
                let mut rs: Vec<R> = rects.iter().map(|x| x.1).collect();
                let to = sm::size_side(&mut rs, i, side, v - origin, R::new(0.0, 0.0, pr.w, pr.h));
                let _ = to;
                for (k, r2) in rs.iter().enumerate() {
                    rects[k].1 = *r2;
                }
                size = if across { rs[i].w } else { rs[i].h };
                let id: &'static str = *id;
                if let Some(st) = self.st.as_mut() {
                    st.s.live(|d| {
                        let pd = d.page_mut(page);
                        for (pid, r2) in &rects {
                            pd.part_mut(pid).at = Some(Anchors::from_rect(*r2, pr.w, pr.h));
                        }
                        let s = &mut pd.part_mut(id).style;
                        if across && s.wmode == "fixed" {
                            s.w = size.round() as u16;
                        }
                        if !across && s.h > 0 {
                            s.h = size.round() as u16;
                        }
                    });
                }
            }
            Sel::Add(_, uid) => {
                let mut rs = vec![R::new(r.x - pr.x, r.y - pr.y, r.w, r.h)];
                sm::size_side(&mut rs, 0, side, v - origin, R::new(0.0, 0.0, pr.w, pr.h));
                size = if across { rs[0].w } else { rs[0].h };
                let uid = uid.clone();
                let free = self.studio_page(page).and_then(|p| p.add(&uid)).map(|a| a.into.is_empty()).unwrap_or(false);
                if let Some(st) = self.st.as_mut() {
                    st.s.live(|d| {
                        if let Some(a) = d.page_mut(page).add_mut(&uid) {
                            if free {
                                a.at = Some(Anchors::from_rect(rs[0], pr.w, pr.h));
                            } else if across {
                                a.style.wmode = "fixed";
                                a.style.w = rs[0].w.round() as u16;
                            } else {
                                a.style.h = rs[0].h.round() as u16;
                            }
                        }
                    });
                }
            }
        }
        let note = match &with {
            Some(n) => format!("{title}: {} px · lined up with {n}", size.round() as i64),
            None => format!("{title}: {} px", size.round() as i64),
        };
        let line = if across { Rectangle { x: v, y: pr.y, width: 1.0, height: pr.h } } else { Rectangle { x: pr.x, y: v, width: pr.w, height: 1.0 } };
        let at = if across { Point::new(v - 6.0, y) } else { Point::new(x, v + 8.0) };
        if let Some(dr) = self.st.as_mut().and_then(|s| s.drag.as_mut()) {
            dr.guide = Some(Guide { line, note, at });
        }
    }

    fn st_release(&mut self) -> Task<DMsg> {
        let Some(st) = self.st.as_mut() else { return Task::none() };
        let Some(dr) = st.drag.take() else { return Task::none() };
        match dr.kind {
            DragKind::Tile(block, title) => {
                if let (true, Some(t)) = (dr.moved, dr.target) {
                    let uid = st.s.uid();
                    let mut add = Add { uid: uid.clone(), block: block.clone(), title, ..Add::default() };
                    let page = match t {
                        Target::Bar { page, part, after, .. } => {
                            add.into = part.to_string();
                            add.after = after;
                            page
                        }
                        Target::Free { page, at, .. } => {
                            let pr = self.geo.borrow().page(page).unwrap_or_default();
                            add.at = Some(Anchors::from_rect(at, pr.w, pr.h));
                            page
                        }
                    };
                    if let Some(own) = block.strip_prefix("mine:").and_then(|id| st.s.draft.own.iter().find(|o| o.id == id)) {
                        add.style = own.style.clone();
                    }
                    let st = self.st.as_mut().expect("editing");
                    st.s.change(|d| d.page_mut(page).adds.push(add));
                    st.pick = Some(Sel::Add(page, uid));
                    if block == "own:words" {
                        // Count · words · spacer: pick what it is right away
                        st.panel = true;
                        st.tab = 1;
                    }
                    if block == "own:button" {
                        st.panel = true;
                        st.tab = 2;
                    }
                }
            }
            DragKind::Add(page, uid) => {
                if let (true, Some(t)) = (dr.moved, dr.target) {
                    let pr = match &t {
                        Target::Free { page, .. } => self.geo.borrow().page(page).unwrap_or_default(),
                        _ => R::default(),
                    };
                    let st = self.st.as_mut().expect("editing");
                    st.s.change(|d| {
                        let Some(mut a) = d.page(page).and_then(|p| p.add(&uid)).cloned() else { return };
                        d.page_mut(page).adds.retain(|x| x.uid != uid);
                        // what was after it follows the one before it
                        let before = a.after.clone();
                        for x in d.page_mut(page).adds.iter_mut().filter(|x| x.after == uid) {
                            x.after = before.clone();
                        }
                        let to = match t {
                            Target::Bar { page: tp, part, after, .. } => {
                                a.into = part.to_string();
                                a.after = after;
                                a.at = None;
                                tp
                            }
                            Target::Free { page: tp, at, .. } => {
                                a.into.clear();
                                a.after.clear();
                                a.at = Some(Anchors::from_rect(at, pr.w, pr.h));
                                tp
                            }
                        };
                        d.page_mut(to).adds.push(a);
                    });
                    let now = st.s.draft.pages.iter().find(|x| x.1.add(&uid).is_some()).map(|x| x.0.clone());
                    let np = now.and_then(|n| self.pages.iter().find(|d| d.id == n).map(|d| d.id));
                    if let (Some(np), Some(st)) = (np, self.st.as_mut()) {
                        st.pick = Some(Sel::Add(np, uid));
                    }
                }
            }
            DragKind::Part(..) | DragKind::Side(..) => {
                st.s.settle(dr.before);
            }
            DragKind::Win => {}
        }
        Task::none()
    }

    fn st_key(&mut self, k: K) -> Task<DMsg> {
        let Some(st) = self.st.as_mut() else { return Task::none() };
        match k {
            K::Esc => {
                if st.menu.is_some() || st.pop != Pop::None || st.expand != Expand::None {
                    st.menu = None;
                    st.pop = Pop::None;
                    st.expand = Expand::None;
                } else if st.panel {
                    st.panel = false;
                } else {
                    return self.studio(SMsg::Cancel);
                }
            }
            K::Undo => return self.studio(SMsg::Undo),
            K::Redo => return self.studio(SMsg::Redo),
            K::Enter => {
                if let Some(p) = st.pick.clone() {
                    return self.studio(SMsg::Customize(p));
                }
            }
            K::Delete => {
                if let Some(p) = st.pick.clone() {
                    return self.studio(SMsg::Hide(p));
                }
            }
            K::Next | K::Prev => {
                let page = st.pick.as_ref().map(|p| p.page()).unwrap_or(st.focus);
                let Some(def) = self.pages.iter().find(|p| p.id == page) else { return Task::none() };
                let ids: Vec<&'static str> = def.blocks.iter().map(|b| b.id).filter(|id| !st.s.draft.page(page).and_then(|p| p.part(id)).map(|x| x.hidden).unwrap_or(false)).collect();
                if ids.is_empty() {
                    return Task::none();
                }
                let cur = match &st.pick {
                    Some(Sel::Part(_, id)) => ids.iter().position(|x| x == id),
                    _ => None,
                };
                let n = ids.len();
                let i = match (cur, k) {
                    (None, K::Prev) => n - 1,
                    (None, _) => 0,
                    (Some(i), K::Prev) => (i + n - 1) % n,
                    (Some(i), _) => (i + 1) % n,
                };
                st.pick = Some(Sel::Part(page, ids[i]));
            }
            K::Left | K::Right | K::Up | K::Down | K::Wider | K::Narrower | K::Taller | K::Shorter => {
                let Some(Sel::Part(page, id)) = st.pick.clone() else { return Task::none() };
                let g = st.s.grid.max(1) as f32;
                self.adopt(page);
                let geo = self.geo.borrow();
                let Some(pr) = geo.page(page) else { return Task::none() };
                let Some(st) = self.st.as_mut() else { return Task::none() };
                let Some(a) = st.s.draft.page(page).and_then(|p| p.part(id)).and_then(|p| p.at) else { return Task::none() };
                let r = a.resolve(pr.w, pr.h);
                let pg = R::new(0.0, 0.0, pr.w, pr.h);
                let n = match k {
                    K::Left => sm::moved(r, -g, 0.0, pg, None),
                    K::Right => sm::moved(r, g, 0.0, pg, None),
                    K::Up => sm::moved(r, 0.0, -g, pg, None),
                    K::Down => sm::moved(r, 0.0, g, pg, None),
                    K::Wider => R { w: (r.w + g).min(pr.w - r.x), ..r },
                    K::Narrower => R { w: (r.w - g).max(sm::MIN), ..r },
                    K::Taller => R { h: (r.h + g).min(pr.h - r.y), ..r },
                    _ => R { h: (r.h - g).max(sm::MIN), ..r },
                };
                let a2 = Anchors::from_rect(n, pr.w, pr.h);
                st.s.change(|d| d.page_mut(page).part_mut(id).at = Some(a2));
            }
        }
        Task::none()
    }

    /// What a part's Does says to do.
    fn st_do(&mut self, a: &str) -> Task<DMsg> {
        if let Some(id) = a.strip_prefix("act:") {
            if let Some(t) = self.kit.and_then(|k| k.button(id)) {
                return Task::done(DMsg::Studio(SMsg::Act(t.id)));
            }
        } else if let Some(c) = a.strip_prefix("run:") {
            // Button · runs anything: the person's own command, started and left to run
            run::detached(&["sh", "-c", c]);
        } else if let Some(p) = a.strip_prefix("panel:") {
            // Show a panel: one of the app's panes placed in the window, shown or put away (not a setting)
            let b = format!("pane:{p}");
            let mut c = self.panes_closed.borrow_mut();
            if let Some(i) = c.iter().position(|x| *x == b) {
                c.remove(i);
            } else {
                c.push(b);
            }
        }
        Task::none()
    }

    /// Editing (drag and drop): every message of board 3 and board 2's side panel.
    pub(crate) fn studio(&mut self, m: SMsg) -> Task<DMsg> {
        match m {
            SMsg::Start(f, p) => return self.st_start(f, p),
            SMsg::Act(_) => return Task::none(),
            SMsg::Do(a) => return self.st_do(&a),
            SMsg::Kept(o) => {
                if o.ok {
                    self.reload_kept();
                    let editing = self.st.is_some();
                    self.end_studio();
                    if editing {
                        self.said = "Kept. Edit layout again any time; Undo is in Describe it's box.".into();
                    } else if self.ed.note.is_some() {
                        return self.note_later();
                    }
                } else if let Some(st) = self.st.as_mut() {
                    st.busy = false;
                    st.bad = true;
                    st.note = if o.err.contains("No such file") { "Keeping it needs hearth-layout, which comes with Hearth OS.".into() } else { format!("It couldn't be kept: {}", o.why()) };
                } else {
                    self.ed.note = self.ed.note.take().map(|(p, _)| (p, o.why()));
                }
                return Task::none();
            }
            SMsg::HideNow(page, id) => {
                let mut s = Session::new(Draft::read(self.kept.get("layout").unwrap_or(&Value::Null)));
                s.change(|d| d.page_mut(page).part_mut(id).hidden = true);
                let title = self.pages.iter().find(|p| p.id == page).and_then(|p| p.blocks.iter().find(|b| b.id == id)).map(|b| b.title).unwrap_or(id);
                let Some(c) = s.keep_change(&self.grid_pages()) else { return Task::none() };
                self.ed.note = Some((page, format!("{} is hidden.", short(title))));
                return Task::perform(run::spawn(self.keep_cmd_studio(&c)), |o| DMsg::Studio(SMsg::Kept(o)));
            }
            SMsg::Move(x, y) => {
                self.st_move(x, y);
                return Task::none();
            }
            SMsg::Release => return self.st_release(),
            SMsg::Key(k) => return self.st_key(k),
            SMsg::Stage(w) => return self.stage(w),
            _ => {}
        }
        let geo_page = |d: &Describe, p: &str| d.geo.borrow().page(p);
        let Some(st) = self.st.as_mut() else { return Task::none() };
        match m {
            SMsg::Tick => {
                st.refresh = false;
                // editing sits over the outermost page drawn (the whole window)
                if let Some(r) = self.geo.try_borrow().ok().and_then(|g| g.root()).filter(|r| self.pages.iter().any(|p| p.id == *r)) {
                    st.root = r;
                }
                // CI's pictures: HEARTH_STUDIO_STAGE=<app>:<drag|size|customize|menu|move> shows that moment, once,
                // when the window has been drawn
                if !st.staged {
                    st.staged = true;
                    if let Ok(v) = std::env::var("HEARTH_STUDIO_STAGE") {
                        if let Some((app, what)) = v.split_once(':') {
                            let what: &'static str = match what {
                                "drag" => "drag",
                                "size" => "size",
                                "customize" => "customize",
                                "menu" => "menu",
                                "move" => "move",
                                _ => "",
                            };
                            if app == self.app && !what.is_empty() {
                                return Task::perform(run::after(900), move |_| DMsg::Studio(SMsg::Stage(what)));
                            }
                        }
                    }
                }
            }
            SMsg::Refresh => {
                if !st.refresh {
                    st.refresh = true;
                    return Task::perform(run::after(40), |_| DMsg::Studio(SMsg::Tick));
                }
            }
            SMsg::Press(sel, x, y) => {
                st.menu = None;
                st.pop = Pop::None;
                st.moveto = false;
                st.pointer = (x, y);
                if st.panel && st.pick.as_ref() != Some(&sel) {
                    st.expand = Expand::None;
                }
                st.pick = Some(sel.clone());
                st.focus = sel.page();
                let before = st.s.draft.clone();
                let (start, page) = {
                    let geo = self.geo.borrow();
                    let r = match &sel {
                        Sel::Part(p, id) => geo.part(p, id),
                        Sel::Add(p, u) => geo.add(p, u),
                    };
                    (r.unwrap_or_default(), geo.page(sel.page()).unwrap_or_default())
                };
                let kind = match &sel {
                    Sel::Part(p, id) => DragKind::Part(p, id),
                    Sel::Add(p, u) => DragKind::Add(p, u.clone()),
                };
                if let Some(st) = self.st.as_mut() {
                    st.drag = Some(Drag { kind, from: (x, y), before, moved: false, start, page, target: None, guide: None });
                }
            }
            SMsg::PressNothing => {
                st.pick = None;
                st.menu = None;
                st.pop = Pop::None;
                st.expand = Expand::None;
            }
            SMsg::RMenu(sel, x, y) => {
                st.pick = Some(sel.clone());
                st.menu = Some((sel, x, y));
                st.moveto = false;
                st.pop = Pop::None;
            }
            SMsg::CloseMenu => {
                st.menu = None;
                st.moveto = false;
            }
            SMsg::MoveToOpen => st.moveto = !st.moveto,
            SMsg::GrabTile(block, title) => {
                let p = st.pointer;
                st.drag = Some(Drag { kind: DragKind::Tile(block, title), from: p, before: st.s.draft.clone(), moved: false, start: R::default(), page: R::new(0.0, 0.0, -1.0, 0.0), target: None, guide: None });
            }
            SMsg::GrabSide(sel, side) => {
                st.pick = Some(sel.clone());
                let p = st.pointer;
                st.drag = Some(Drag { kind: DragKind::Side(sel, side), from: (p.0 - 10.0, p.1 - 10.0), before: st.s.draft.clone(), moved: true, start: R::default(), page: R::default(), target: None, guide: None });
            }
            SMsg::GrabWin => {
                let root = self.geo.borrow().page(st.root).unwrap_or_default();
                let (wx, wy) = st.win.unwrap_or_else(|| win_default(root));
                let p = st.pointer;
                st.drag = Some(Drag { kind: DragKind::Win, from: p, before: Draft::default(), moved: false, start: R::new(wx, wy, 0.0, 0.0), page: R::new(0.0, 0.0, -1.0, 0.0), target: None, guide: None });
            }
            SMsg::Find(s) => st.find = s.chars().take(60).collect(),
            SMsg::Kind(k) => st.kind = k.min(4),
            SMsg::WinToggle => st.win_open = !st.win_open,
            SMsg::Undo => {
                if !st.s.undo() {
                    st.note = "Nothing to undo.".into();
                } else {
                    st.note.clear();
                }
                st.last_key.clear();
            }
            SMsg::Redo => {
                if !st.s.redo() {
                    st.note = "Nothing to redo.".into();
                } else {
                    st.note.clear();
                }
                st.last_key.clear();
            }
            SMsg::Snap => st.s.snap = !st.s.snap,
            SMsg::Grid => {
                let i = sm::GRIDS.iter().position(|g| *g == st.s.grid).map(|i| (i + 1) % sm::GRIDS.len()).unwrap_or(0);
                st.s.grid = sm::GRIDS[i];
                st.s.snap = true;
            }
            SMsg::Guides => st.s.guides = !st.s.guides,
            SMsg::Pop(p) => st.pop = if st.pop == p { Pop::None } else { p },
            SMsg::Layer(sel, dz) => {
                st.s.change(|d| match &sel {
                    Sel::Part(p, id) => {
                        let z = &mut d.page_mut(p).part_mut(id).z;
                        *z = (*z + dz).clamp(-999, 999);
                    }
                    Sel::Add(p, u) => {
                        if let Some(a) = d.page_mut(p).add_mut(u) {
                            a.z = (a.z + dz).clamp(-999, 999);
                        }
                    }
                });
            }
            SMsg::Eye(sel) => {
                st.s.change(|d| match &sel {
                    Sel::Part(p, id) => {
                        let h = &mut d.page_mut(p).part_mut(id).hidden;
                        *h = !*h;
                    }
                    Sel::Add(p, u) => {
                        if let Some(a) = d.page_mut(p).add_mut(u) {
                            a.hidden = !a.hidden;
                        }
                    }
                });
            }
            SMsg::StartFrom(kind) => {
                let pages: Vec<&'static str> = self.geo.borrow().pages();
                let mut made: Vec<(&'static str, PageDraft)> = Vec::new();
                let kept = self.st.as_ref().map(|s| s.s.kept.clone()).unwrap_or_default();
                for page in pages {
                    let rects = self.page_rects(page);
                    let Some(pr) = geo_page(self, page) else { continue };
                    let ids: Vec<&str> = rects.iter().map(|x| x.0).collect();
                    let rs: Vec<R> = rects.iter().map(|x| x.1).collect();
                    made.push((page, sm::start_from(kind, kept.page(page), &ids, &rs, pr.w, pr.h)));
                }
                let Some(st) = self.st.as_mut() else { return Task::none() };
                st.s.change(|d| {
                    if kind == "kept" {
                        d.pages = kept.pages.clone();
                        d.menus = kept.menus.clone();
                        d.hidden_items = kept.hidden_items.clone();
                        return;
                    }
                    for (p, pd) in made {
                        *d.page_mut(p) = pd;
                    }
                });
                st.start = kind;
                st.pop = Pop::None;
                st.pick = None;
            }
            SMsg::MenuMove(menu, i, dir) => {
                let menus: &'static [MenuDef] = self.kit.map(|k| k.menus).unwrap_or(&[]);
                let Some(def) = menus.iter().chain(std::iter::once(&PART_MENU)).find(|m| m.id == menu) else { return Task::none() };
                let Some(st) = self.st.as_mut() else { return Task::none() };
                let mut order: Vec<String> = order_of(&st.s.draft, def);
                let j = i as i32 + dir;
                if i < order.len() && j >= 0 && (j as usize) < order.len() {
                    order.swap(i, j as usize);
                    st.s.change(|d| match d.menus.iter_mut().find(|m| m.0 == menu) {
                        Some(m) => m.1 = order,
                        None => d.menus.push((menu.to_string(), order)),
                    });
                }
            }
            SMsg::MenuHide(_menu, item) => {
                st.s.change(|d| {
                    if let Some(i) = d.hidden_items.iter().position(|x| *x == item) {
                        d.hidden_items.remove(i);
                    } else {
                        d.hidden_items.push(item);
                    }
                });
            }
            SMsg::Cancel => {
                self.end_studio();
                self.said = "Nothing was changed.".into();
            }
            SMsg::Try => {
                st.trying = true;
                st.menu = None;
                st.pop = Pop::None;
                st.panel = false;
            }
            SMsg::Back => st.trying = false,
            SMsg::Keep => {
                let grid = self.grid_pages();
                let Some(st) = self.st.as_ref() else { return Task::none() };
                if st.busy {
                    return Task::none();
                }
                match st.s.keep_change(&grid) {
                    None => self.end_studio(),
                    Some(c) => {
                        let cmd = self.keep_cmd_studio(&c);
                        if let Some(st) = self.st.as_mut() {
                            st.writes += 1;
                            st.busy = true;
                            st.note = "Keeping it…".into();
                            st.bad = false;
                        }
                        return Task::perform(run::spawn(cmd), |o| DMsg::Studio(SMsg::Kept(o)));
                    }
                }
            }
            SMsg::Customize(sel) => {
                st.pick = Some(sel);
                st.panel = true;
                st.menu = None;
                st.pop = Pop::None;
                st.expand = Expand::None;
            }
            SMsg::ClosePanel => {
                st.panel = false;
                st.expand = Expand::None;
            }
            SMsg::Tab(i) => {
                st.tab = i.min(5);
                st.expand = Expand::None;
            }
            SMsg::Expand(e) => st.expand = if st.expand == e { Expand::None } else { e },
            SMsg::Set(sel, f) => return self.st_set(sel, f),
            SMsg::Reset(sel) => {
                st.s.change(|d| match &sel {
                    Sel::Part(p, id) => {
                        let pt = d.page_mut(p).part_mut(id);
                        pt.style = Style::default();
                    }
                    Sel::Add(p, u) => {
                        if let Some(a) = d.page_mut(p).add_mut(u) {
                            a.style = Style::default();
                        }
                    }
                });
            }
            SMsg::CopyLook(sel) => {
                st.look = Some(style_of(&st.s.draft, &sel).look_of());
                st.menu = None;
                st.note = "Its look is copied: Paste a look on any part.".into();
            }
            SMsg::PasteLook(sel) => {
                if let Some(l) = st.look.clone() {
                    st.s.change(|d| {
                        if let Some(s) = style_mut(d, &sel) {
                            *s = s.with_look(&l);
                        }
                    });
                }
                st.menu = None;
            }
            SMsg::Duplicate(sel) => {
                st.menu = None;
                if let Sel::Add(p, u) = sel {
                    let uid = st.s.uid();
                    st.s.change(|d| {
                        let Some(mut a) = d.page(p).and_then(|x| x.add(&u)).cloned() else { return };
                        a.after = u.clone();
                        a.uid = uid.clone();
                        if let Some(at) = a.at.as_mut() {
                            at.l.px = at.l.px.saturating_add(16);
                            at.r.px = if at.r.end { at.r.px.saturating_sub(16) } else { at.r.px.saturating_add(16) };
                            at.t.px = at.t.px.saturating_add(16);
                            at.b.px = if at.b.end { at.b.px.saturating_sub(16) } else { at.b.px.saturating_add(16) };
                        }
                        d.page_mut(p).adds.push(a);
                    });
                    st.pick = Some(Sel::Add(p, uid));
                }
            }
            SMsg::MoveTo(sel, to) => {
                st.menu = None;
                st.moveto = false;
                match sel {
                    Sel::Add(p, u) => {
                        let to_part: Option<&'static str> = self.pages.iter().find(|d| d.id == p).and_then(|d| d.blocks.iter().find(|b| b.id == to)).map(|b| b.id);
                        let pr = geo_page(self, p).unwrap_or_default();
                        let Some(st) = self.st.as_mut() else { return Task::none() };
                        st.s.change(|d| {
                            if let Some(a) = d.page_mut(p).add_mut(&u) {
                                match to_part {
                                    Some(part) => {
                                        a.into = part.to_string();
                                        a.after = String::new();
                                        a.at = None;
                                    }
                                    None => {
                                        a.into.clear();
                                        a.after.clear();
                                        let (w, h) = free_size(&a.block, pr);
                                        a.at = Some(Anchors::from_rect(R::new(((pr.w - w) / 2.0).max(0.0), ((pr.h - h) / 2.0).max(0.0), w, h), pr.w, pr.h));
                                    }
                                }
                            }
                        });
                    }
                    Sel::Part(p, id) => {
                        self.adopt(p);
                        let pr = geo_page(self, p).unwrap_or_default();
                        let cur = self.page_rects(p).into_iter().find(|x| x.0 == id).map(|x| x.1).unwrap_or_default();
                        let r = match to.as_str() {
                            "left" => R::new(0.0, 0.0, cur.w, pr.h),
                            "right" => R::new(pr.w - cur.w, 0.0, cur.w, pr.h),
                            "top" => R::new(0.0, 0.0, pr.w, cur.h),
                            "bottom" => R::new(0.0, pr.h - cur.h, pr.w, cur.h),
                            _ => R::new(((pr.w - cur.w) / 2.0).max(0.0), ((pr.h - cur.h) / 2.0).max(0.0), cur.w, cur.h),
                        };
                        let Some(st) = self.st.as_mut() else { return Task::none() };
                        st.s.change(|d| d.page_mut(p).part_mut(id).at = Some(Anchors::from_rect(r, pr.w, pr.h)));
                    }
                }
            }
            SMsg::Hide(sel) => {
                st.menu = None;
                st.s.change(|d| match &sel {
                    Sel::Part(p, id) => d.page_mut(p).part_mut(id).hidden = true,
                    Sel::Add(p, u) => {
                        if let Some(a) = d.page_mut(p).add_mut(u) {
                            a.hidden = true;
                        }
                    }
                });
                st.note = "Hidden: Layers has it (the eye shows it again).".into();
                if st.pick.as_ref() == Some(&sel) {
                    st.panel = false;
                }
            }
            SMsg::TakeOut(sel) => {
                st.menu = None;
                st.s.change(|d| match &sel {
                    Sel::Part(p, id) => {
                        let pt = d.page_mut(p).part_mut(id);
                        pt.hidden = true;
                        pt.at = None;
                    }
                    Sel::Add(p, u) => {
                        let pd = d.page_mut(p);
                        let after = pd.add(u).map(|a| a.after.clone()).unwrap_or_default();
                        pd.adds.retain(|a| a.uid != *u);
                        for x in pd.adds.iter_mut().filter(|x| x.after == *u) {
                            x.after = after.clone();
                        }
                    }
                });
                if st.pick.as_ref() == Some(&sel) {
                    st.pick = None;
                    st.panel = false;
                }
            }
            SMsg::OwnName(s) => st.own_name = s.chars().take(60).collect(),
            SMsg::SaveOwn(sel) => {
                let name = if st.own_name.trim().is_empty() { "My block".to_string() } else { st.own_name.trim().to_string() };
                let (block, style) = match &sel {
                    Sel::Add(p, u) => st.s.draft.page(p).and_then(|x| x.add(u)).map(|a| (a.block.clone(), a.style.clone())).unwrap_or_default(),
                    Sel::Part(..) => ("own:words".to_string(), style_of(&st.s.draft, &sel).look_of()),
                };
                let block = if block.starts_with("mine:") { "own:button".to_string() } else { block };
                if st.s.draft.own.len() >= sm::MAX_OWN {
                    st.note = "You have 32 blocks of your own: take one out first.".into();
                } else {
                    let n = st.s.draft.own.len() + 1;
                    let id = format!("own{n}");
                    st.s.change(|d| d.own.push(sm::Own { id, name: name.clone(), block, style }));
                    st.note = format!("\"{name}\" is in Your own.");
                    st.own_name.clear();
                }
            }
            SMsg::ApplyAll(sel) => {
                // the look onto every other part (or every block in the same bar)
                let look = style_of(&st.s.draft, &sel).look_of();
                let page = sel.page();
                let ids: Vec<&'static str> = self.pages.iter().find(|d| d.id == page).map(|d| d.blocks.iter().map(|b| b.id).collect()).unwrap_or_default();
                let Some(st) = self.st.as_mut() else { return Task::none() };
                st.s.change(|d| match &sel {
                    Sel::Part(..) => {
                        for id in &ids {
                            let s = &mut d.page_mut(page).part_mut(id).style;
                            *s = s.with_look(&look);
                        }
                    }
                    Sel::Add(p, u) => {
                        let bar = d.page(p).and_then(|x| x.add(u)).map(|a| a.into.clone()).unwrap_or_default();
                        for a in d.page_mut(p).adds.iter_mut().filter(|a| a.into == bar) {
                            a.style = a.style.with_look(&look);
                        }
                    }
                });
            }
            _ => {}
        }
        Task::none()
    }

    /// hearth-layout app-keep with this change (the app's name and blocks in its env, as Describe it's).
    fn keep_cmd_studio(&self, change: &Value) -> Vec<String> {
        vec![
            "env".into(),
            format!("HEARTH_DESCRIBE_APP={}", self.name),
            format!("HEARTH_APP_BLOCKS={}", blocks::catalog(self.pages)),
            "hearth-layout".into(),
            "app-keep".into(),
            self.app.into(),
            change.to_string(),
        ]
    }

    fn st_set(&mut self, sel: Sel, f: F) -> Task<DMsg> {
        let place = matches!(f, F::X(_) | F::Y(_) | F::PW(_) | F::PH(_) | F::Snap(_));
        if place {
            if let F::Snap(to) = &f {
                return self.studio(SMsg::MoveTo(sel, to.to_string()));
            }
            let page = sel.page();
            let pr = self.geo.borrow().page(page).unwrap_or_default();
            if let Sel::Part(p, _) = &sel {
                self.adopt(*p);
            }
            let cur = match &sel {
                Sel::Part(_, id) => self.page_rects(page).into_iter().find(|x| x.0 == *id).map(|x| x.1),
                Sel::Add(p, u) => self.geo.borrow().add(p, u).map(|r| R::new(r.x - pr.x, r.y - pr.y, r.w, r.h)),
            };
            let Some(mut r) = cur else { return Task::none() };
            let v = |s: &str| num(s, sm::MAX_PX).map(|n| n as f32);
            match &f {
                F::X(s) => r.x = v(s).unwrap_or(r.x).min((pr.w - r.w).max(0.0)),
                F::Y(s) => r.y = v(s).unwrap_or(r.y).min((pr.h - r.h).max(0.0)),
                F::PW(s) => r.w = v(s).unwrap_or(r.w).clamp(sm::MIN, pr.w.max(sm::MIN)),
                F::PH(s) => r.h = v(s).unwrap_or(r.h).clamp(sm::MIN, pr.h.max(sm::MIN)),
                _ => {}
            }
            let a = Anchors::from_rect(r, pr.w, pr.h);
            let Some(st) = self.st.as_mut() else { return Task::none() };
            let key = format!("place:{}", sel.key());
            let same = st.last_key == key;
            let f2 = |d: &mut Draft| match &sel {
                Sel::Part(p, id) => d.page_mut(p).part_mut(id).at = Some(a),
                Sel::Add(p, u) => {
                    if let Some(x) = d.page_mut(p).add_mut(u) {
                        if x.into.is_empty() {
                            x.at = Some(a);
                        } else {
                            x.style.w = r.w as u16;
                            x.style.wmode = "fixed";
                            x.style.h = r.h as u16;
                        }
                    }
                }
            };
            if same { st.s.live(f2) } else { st.s.change(f2); }
            st.last_key = key;
            return Task::none();
        }
        let Some(st) = self.st.as_mut() else { return Task::none() };
        // Shows › A spacer: the block becomes a spacer (and back to words)
        if let (F::Shows(x), Sel::Add(p, u)) = (&f, &sel) {
            let x: &'static str = *x;
            let (p, u) = (*p, u.clone());
            st.s.change(|d| {
                if let Some(a) = d.page_mut(p).add_mut(&u) {
                    let own = a.block.starts_with("own:");
                    if own {
                        a.block = match x {
                            "spacer" => "own:spacer".into(),
                            "count" => "own:count".into(),
                            _ if a.block == "own:spacer" || a.block == "own:count" => "own:words".into(),
                            _ => a.block.clone(),
                        };
                    }
                    a.style.shows = if x == "spacer" { "" } else { x };
                }
            });
            st.last_key.clear();
            return Task::none();
        }
        let key = typed(&f).map(|k| format!("{k}:{}", sel.key()));
        let same = key.is_some() && key.as_deref() == Some(st.last_key.as_str());
        let f2 = |d: &mut Draft| {
            if let Some(s) = style_mut(d, &sel) {
                apply(s, &f);
            }
        };
        if same {
            st.s.live(f2);
        } else {
            st.s.change(f2);
        }
        st.last_key = key.unwrap_or_default();
        Task::none()
    }

    /// CI's pictures (HEARTH_STUDIO_STAGE): the moment shown as it is while it's done with the pointer.
    fn stage(&mut self, what: &'static str) -> Task<DMsg> {
        let (bars, panes): (Vec<Rec>, Option<Rec>) = {
            let geo = self.geo.borrow();
            let mut bars: Vec<Rec> = geo.now().filter(|r| r.kind == 1 && rr(r.rect).is_bar() && rr(r.rect).across() && !geo.holder(r)).cloned().collect();
            bars.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y));
            let tall = geo
                .now()
                .filter(|r| r.kind == 1 && !geo.holder(r) && !rr(r.rect).across())
                .max_by(|a, b| a.rect.x.total_cmp(&b.rect.x))
                .or_else(|| geo.now().filter(|r| r.kind == 1 && !geo.holder(r) && !rr(r.rect).is_bar()).max_by(|a, b| a.rect.x.total_cmp(&b.rect.x)))
                .cloned();
            (bars, tall)
        };
        let first_button = self.kit.and_then(|k| k.buttons.first().copied());
        match what {
            "drag" => {
                // a button being dragged into the app's first bar, the lit line where it lands
                let Some(bar) = bars.first().or(bars.get(1)) else { return Task::none() };
                let Some(b) = first_button else { return Task::none() };
                let (x, y) = (bar.rect.x + bar.rect.width * 0.36, bar.rect.y + bar.rect.height * 0.5);
                if let Some(st) = self.st.as_mut() {
                    st.pointer = (x - 40.0, y - 60.0);
                    st.drag = Some(Drag { kind: DragKind::Tile(format!("act:{}", b.id), b.title.to_string()), from: (x - 40.0, y - 60.0), before: st.s.draft.clone(), moved: false, start: R::default(), page: R::default(), target: None, guide: None });
                }
                self.st_move(x, y);
            }
            "size" => {
                // a pane's (or the tallest part's) left edge being dragged, the guide and the note
                let Some(p) = panes else { return Task::none() };
                let Some(page) = self.pages.iter().find(|d| d.id == p.page).map(|d| d.id) else { return Task::none() };
                let Some(id) = self.pages.iter().find(|d| d.id == page).and_then(|d| d.blocks.iter().find(|b| b.id == p.id)).map(|b| b.id) else { return Task::none() };
                let sel = Sel::Part(page, id);
                let x = p.rect.x - (p.rect.width * 0.2).min(120.0);
                let y = p.rect.y + p.rect.height * 0.5;
                if let Some(st) = self.st.as_mut() {
                    st.pick = Some(sel.clone());
                    st.pointer = (p.rect.x, y);
                    st.drag = Some(Drag { kind: DragKind::Side(sel, Side::Left), from: (p.rect.x, y), before: st.s.draft.clone(), moved: true, start: rr(p.rect), page: R::default(), target: None, guide: None });
                }
                self.st_move(x, y);
            }
            "customize" | "menu" => {
                let pick = bars.first().and_then(|b| self.pages.iter().find(|d| d.id == b.page).and_then(|d| d.blocks.iter().find(|x| x.id == b.id)).map(|x| Sel::Part(d_id(self.pages, b.page), x.id)));
                let Some(sel) = pick else { return Task::none() };
                if what == "menu" {
                    let r = bars.first().map(|b| b.rect).unwrap_or_default();
                    return self.studio(SMsg::RMenu(sel, r.x + r.width * 0.45, r.y + r.height * 0.6));
                }
                if let Some(st) = self.st.as_mut() {
                    st.s.change(|d| {
                        if let Some(s) = style_mut(d, &sel) {
                            s.corners = Some(12);
                            s.edge = "line";
                            s.edge_colour = "#ff7a2f".into();
                            s.fill = "#3b2a1d".into();
                            s.see = 15;
                            s.around = [6, 8, 6, 8];
                        }
                    });
                }
                return self.studio(SMsg::Customize(sel));
            }
            "move" => {
                // a part moved off its side, anywhere in the window
                let Some(p) = panes else { return Task::none() };
                let Some(page) = self.pages.iter().find(|d| d.id == p.page).map(|d| d.id) else { return Task::none() };
                let Some(id) = self.pages.iter().find(|d| d.id == page).and_then(|d| d.blocks.iter().find(|b| b.id == p.id)).map(|b| b.id) else { return Task::none() };
                let (x, y) = (p.rect.x + 40.0, p.rect.y + 40.0);
                let _ = self.studio(SMsg::Press(Sel::Part(page, id), x, y));
                self.st_move(x - p.rect.width * 0.4, y + 60.0);
            }
            _ => {}
        }
        Task::none()
    }
}

/// Where the blocks window starts (over the root page): on the right, a little down.
fn win_default(root: R) -> (f32, f32) {
    ((root.w - 400.0).max(16.0), (root.h * 0.22).max(60.0).min((root.h - 640.0).max(60.0)))
}

fn d_id(pages: &[PageDef], id: &str) -> &'static str {
    pages.iter().find(|p| p.id == id).map(|p| p.id).unwrap_or("")
}

/// A menu's items in the order the draft has them (the rest after, as the app has them).
fn order_of(d: &Draft, m: &MenuDef) -> Vec<String> {
    let mut out: Vec<String> = d.menu(m.id).cloned().unwrap_or_default().into_iter().filter(|i| m.items.iter().any(|x| x.0 == i.as_str())).collect();
    for (id, _) in m.items {
        if !out.iter().any(|x| x.as_str() == *id) {
            out.push(id.to_string());
        }
    }
    out
}

// ------------------------------------------------------------------ the parts and blocks as they're drawn

/// {count}, {time} and the app's other values in a block's words.
fn fill_words(d: &Describe, words: &str, source: &str) -> String {
    let mut s = words.replace("{count}", &d.value(source)).replace("{time}", &crate::time::clock(crate::time::now()));
    if let Some(k) = d.kit {
        for i in k.info {
            let key = format!("{{{}}}", i.id);
            if s.contains(&key) {
                s = s.replace(&key, &d.value(i.id));
            }
        }
    }
    s
}

struct Clock(Color, f32, iced::Font);
impl<M> canvas::Program<M> for Clock {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, _b: Rectangle, _c: mouse::Cursor) -> Option<canvas::Action<M>> {
        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            return Some(canvas::Action::request_redraw_at(iced::time::Instant::now() + Duration::from_secs(15)));
        }
        None
    }
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        f.fill_text(canvas::Text { content: crate::time::clock(crate::time::now()), position: Point::new(0.0, b.height / 2.0), color: self.0, size: iced::Pixels(self.1), font: self.2, align_y: iced::alignment::Vertical::Center, ..canvas::Text::default() });
        vec![f.into_geometry()]
    }
}

/// The text a block shows, in its font (made here, so Look › Font is really its font).
fn faced<R2>(s: &Style, f: impl FnOnce() -> R2) -> R2 {
    let face = if s.font.is_empty() || s.font == "Inter" { None } else { Some(s.font) };
    let w = match s.weight {
        "light" => Some(iced::font::Weight::Light),
        "medium" => Some(iced::font::Weight::Medium),
        "bold" => Some(iced::font::Weight::Bold),
        _ => None,
    };
    let scale = if s.font_size > 0 { s.font_size as f32 / 13.0 } else { 1.0 };
    ui::with_style(face, scale, w, f)
}

/// A part's own content made in its font (Look › Font, its size and weight): the app makes a part's view inside this.
pub fn made<'a, M: 'a>(d: &Describe, page: &str, id: &str, f: impl FnOnce() -> Element<'a, M>) -> Element<'a, M> {
    match d.studio_page(page).and_then(|p| p.part(id)) {
        Some(p) if !p.style.font.is_empty() || !p.style.weight.is_empty() || p.style.font_size > 0 => faced(&p.style, f),
        _ => f(),
    }
}

/// Icon and words (Look › ICON AND WORDS; Show: Icon, Words or Both).
fn icon_words<'a, M: 'a>(p: &Pal, s: &Style, icon: Option<G>, words: String, base_colour: Color) -> Element<'a, M> {
    let ic = if s.icon.is_empty() { icon } else { glyph_of(s.icon) };
    let ic_c = blocks::colour(&s.icon_colour).unwrap_or(base_colour);
    let ic_sz = if s.icon_size > 0 { s.icon_size as f32 } else { 14.0 };
    let show = if s.show.is_empty() { if words.is_empty() { "icon" } else if ic.is_some() && s.icon.is_empty() { "words" } else { "both" } } else { s.show };
    faced(s, || {
        let mut r = Row::new().spacing(7.0).align_y(iced::Center);
        if matches!(show, "icon" | "both") {
            if let Some(g) = ic {
                r = r.push(ui::glyph(g, ic_c, ic_sz, 1.8));
            }
        }
        if matches!(show, "words" | "both") && !words.is_empty() {
            r = r.push(tm(words, 13.0, p.text).wrapping(Wrapping::None));
        }
        r.into()
    })
}

/// A part of one of the app's pages, as the person set it: hidden, in its look, its icon and words over it, the
/// blocks dragged into it (when it's a bar that doesn't hand over its pieces one by one), recorded while editing.
pub(crate) fn part_el<'a, M: Clone + 'a>(d: &'a Describe, pal: &Pal, page: &'static str, id: &'static str, title: &'static str, across: bool, el: Element<'a, M>, map: Mp<'a, M>) -> Element<'a, M> {
    let p = *pal;
    let part = d.studio_page(page).and_then(|x| x.part(id));
    if part.map(|x| x.hidden).unwrap_or(false) {
        return Space::new().into();
    }
    let def = Style::default();
    let s = part.map(|x| &x.style).unwrap_or(&def);
    let editing = d.studio_editing();
    let showing = editing || visible_now(d, s, "");
    let mut body = el;
    // Look › ICON AND WORDS and Shows on a part: a line over it with them
    let shows_more = !s.icon.is_empty() || !s.words.is_empty() || matches!(s.shows, "count" | "clock" | "list" | "picture" | "words");
    if shows_more {
        let head = shows_el(d, &p, s, None, String::new(), true);
        body = column![container(head).padding(Padding { top: 6.0, right: 10.0, bottom: 4.0, left: 10.0 }), body].into();
    }
    // blocks dragged into it, at its end (a bar whose pieces aren't handed over one by one)
    let strip_bar = !d.kit.map(|k| k.bars.contains(&(page, id))).unwrap_or(false);
    if strip_bar {
        let own: [&str; 0] = [];
        let adds: Vec<&Add> = d.studio_page(page).map(|x| x.in_bar(id, &own)).unwrap_or_default();
        if !adds.is_empty() {
            let els: Vec<Element<'a, M>> = adds.into_iter().map(|a| add_el(d, &p, page, a, None, map.clone())).collect();
            body = if across {
                row![container(body).width(Length::Fill).height(Length::Fill), Row::with_children(els).spacing(6.0).align_y(iced::Center).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 6.0 })].align_y(iced::Center).into()
            } else {
                column![container(body).width(Length::Fill).height(Length::Fill), Column::with_children(els).spacing(6.0).padding(Padding { top: 6.0, right: 0.0, bottom: 8.0, left: 0.0 })].into()
            };
        }
    }
    let dressed = dress(&p, s, Base::Part, body, None, &map, editing, 0, showing);
    spot(d, 1, page, "", id, title, dressed)
}

/// Is it shown now (When: always, on hover, only when there's something to show, only on some pages)?
fn visible_now(d: &Describe, s: &Style, value: &str) -> bool {
    match s.when {
        "something" => !value.trim().is_empty() && value.trim() != "0",
        "pages" if !s.pages.is_empty() => {
            let shown = d.ed.shown();
            s.pages.iter().any(|p| shown.contains(&p.as_str()))
        }
        _ => true,
    }
}

/// What a block (or a part's line) shows: Shows › words, a count, a clock, a picture, a list; else its icon and words.
fn shows_el<'a, M: 'a>(d: &Describe, p: &Pal, s: &Style, icon: Option<G>, words: String, part: bool) -> Element<'a, M> {
    let colour = blocks::colour(&s.icon_colour).unwrap_or(if part { p.muted } else { p.text });
    match s.shows {
        "count" => {
            let w = if s.words.is_empty() { "{count}".to_string() } else { s.words.clone() };
            icon_words(p, s, icon, fill_words(d, &w, &s.source), colour)
        }
        "clock" => {
            let size = if s.font_size > 0 { s.font_size as f32 } else { 13.0 };
            let font = faced(s, || ui::weight(iced::font::Weight::Medium));
            let c: Element<'a, M> = canvas::Canvas::new(Clock(p.text, size, font)).width(Length::Fixed(size * 5.6)).height(Length::Fixed(size * 1.6)).into();
            match if s.icon.is_empty() { None } else { glyph_of(s.icon) } {
                Some(g) => row![ui::glyph(g, colour, if s.icon_size > 0 { s.icon_size as f32 } else { 14.0 }, 1.8), c].spacing(7.0).align_y(iced::Center).into(),
                None => c,
            }
        }
        "picture" if !s.picture.is_empty() => {
            let path = if let Some(rest) = s.picture.strip_prefix("~/") { crate::paths::home().join(rest) } else { std::path::PathBuf::from(&s.picture) };
            let h = if s.h > 0 { s.h as f32 } else { 96.0 };
            iced::widget::image(iced::widget::image::Handle::from_path(path)).height(Length::Fixed(h)).content_fit(iced::ContentFit::Contain).into()
        }
        "list" => {
            let mut c = Column::new().spacing(3.0);
            if let Some(k) = d.kit {
                for i in k.info {
                    let v = d.value(i.id);
                    c = c.push(faced(s, || row![t(i.title, 12.0, p.muted).wrapping(Wrapping::None), Space::new().width(Length::Fill), tm(v, 12.0, p.text).wrapping(Wrapping::None)].spacing(10.0)));
                }
            }
            c.into()
        }
        _ => {
            let w = if !s.words.is_empty() { fill_words(d, &s.words, &s.source) } else { words };
            icon_words(p, s, icon, w, colour)
        }
    }
}

/// A block dragged in from the blocks window, drawn: one of the app's buttons (a press runs it), a pane (the app's
/// view), info (its value), or one made here (Button · runs anything, a count, words, a spacer, a saved own one).
fn add_el<'a, M: Clone + 'a>(d: &'a Describe, pal: &Pal, page: &'static str, a: &Add, pane: Option<&dyn Fn(&str) -> Option<Element<'a, M>>>, map: Mp<'a, M>) -> Element<'a, M> {
    let p = *pal;
    let s = &a.style;
    let editing = d.studio_editing();
    let tile = d.kit.and_then(|k| k.tile_of(&a.block));
    let (kind, id) = a.block.split_once(':').unwrap_or(("", ""));
    let source = if s.source.is_empty() && kind == "info" { id.to_string() } else { s.source.clone() };
    let value = if source.is_empty() { String::new() } else { d.value(&source) };
    let showing = editing || visible_now(d, s, if s.when == "something" { &value } else { "x" });
    let mut press: Option<M> = None;
    let (content, base): (Element<'a, M>, Base) = match (kind, a.block.as_str()) {
        ("pane", _) => {
            let title = a.title.to_uppercase();
            let inner: Element<'a, M> = match pane.and_then(|f| f(&a.block)) {
                Some(el) => el,
                None => container(t(format!("{} shows here in {}.", a.title, d.kit.map(|k| k.name).unwrap_or("the app")), 12.0, p.muted)).padding(8.0).into(),
            };
            let head: Element<'a, M> = if s.shows.is_empty() && s.icon.is_empty() && s.words.is_empty() {
                ts(title, 11.0, p.muted).wrapping(Wrapping::None).into()
            } else {
                shows_el(d, &p, s, tile.map(|t| t.icon), a.title.clone(), true)
            };
            (column![head, container(inner).width(Length::Fill).height(Length::Fill)].spacing(10.0).into(), Base::Pane)
        }
        (_, "own:spacer") => {
            let w = if s.w > 0 { s.w as f32 } else { 32.0 };
            let h = if s.h > 0 { s.h as f32 } else { 24.0 };
            let sp: Element<'a, M> = if editing {
                container(t("space", 10.0, Pal::a(p.muted, 0.7))).center_x(Length::Fixed(w)).center_y(Length::Fixed(h)).style(ui::boxed(Pal::a(p.accent, 0.06), 4.0, Some((Pal::a(p.accent, 0.35), 1.0)))).into()
            } else {
                Space::new().width(Length::Fixed(w)).height(Length::Fixed(h)).into()
            };
            (sp, Base::Plain)
        }
        ("act", id) => {
            if let Some(t) = d.kit.and_then(|k| k.button(id)) {
                press = Some(map(DMsg::Studio(SMsg::Act(t.id))));
            }
            (shows_el(d, &p, s, tile.map(|t| t.icon), a.title.clone(), false), Base::Button)
        }
        ("info", _) => {
            let w = if s.words.is_empty() { format!("{}: {}", a.title, value) } else { fill_words(d, &s.words, &source) };
            let st2 = Style { words: String::new(), ..s.clone() };
            let el = if s.shows.is_empty() || s.shows == "words" { icon_words(&p, &st2, None, w, p.text) } else { shows_el(d, &p, s, None, w, false) };
            (el, Base::Plain)
        }
        (_, "own:count") => {
            let w = if s.words.is_empty() { format!("{} {}", value, a.title) } else { fill_words(d, &s.words, &source) };
            let st2 = Style { words: String::new(), ..s.clone() };
            (if s.shows.is_empty() || s.shows == "count" { icon_words(&p, &st2, None, w, p.text) } else { shows_el(d, &p, s, None, w, false) }, Base::Plain)
        }
        (_, "own:words") => {
            let w = if s.words.is_empty() { a.title.clone() } else { fill_words(d, &s.words, &source) };
            let st2 = Style { words: String::new(), ..s.clone() };
            (if s.shows.is_empty() || s.shows == "words" { icon_words(&p, &st2, None, w, p.text) } else { shows_el(d, &p, s, None, w, false) }, Base::Plain)
        }
        ("mine", id) => {
            let own = d.st.as_ref().map(|x| &x.s.draft.own).unwrap_or(&d.studio_kept.own).iter().find(|o| o.id == id).cloned();
            let made_of = own.as_ref().map(|o| o.block.clone()).unwrap_or_default();
            if let Some(t) = made_of.strip_prefix("act:").and_then(|i| d.kit.and_then(|k| k.button(i))) {
                press = Some(map(DMsg::Studio(SMsg::Act(t.id))));
            }
            let base = if made_of.starts_with("act:") || made_of == "own:button" { Base::Button } else { Base::Plain };
            (shows_el(d, &p, s, None, a.title.clone(), false), base)
        }
        _ => (shows_el(d, &p, s, Some(G::Play), a.title.clone(), false), Base::Button),
    };
    let stamp = {
        let mut h: u64 = 7;
        for b in value.bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as u64);
        }
        h
    };
    let el = dress(&p, s, base, content, press, &map, editing, stamp, showing);
    spot(d, 2, page, crate::studio::static_part(d, page, &a.into), &a.uid, &a.title, el)
}

/// A part's id as the app has it (a &'static str), from its name in what's kept.
pub(crate) fn static_part(d: &Describe, page: &str, id: &str) -> &'static str {
    d.pages.iter().find(|p| p.id == page).and_then(|p| p.blocks.iter().find(|b| b.id == id)).map(|b| b.id).unwrap_or("")
}

/// A bar's own pieces handed over one by one (the app's toolbar, its address bar…), with the blocks dragged into it
/// between them, each piece recorded while editing so a block lands between them. The app lays them out as before.
pub fn pieces<'a, M: Clone + 'a>(d: &'a Describe, pal: &Pal, page: &'static str, part: &'static str, items: Vec<(&'static str, &'static str, Element<'a, M>)>, map: impl Fn(DMsg) -> M + Clone + 'a) -> Vec<Element<'a, M>> {
    let pd = d.studio_page(page);
    let no_adds = pd.map(|p| p.adds.iter().all(|a| a.into != part)).unwrap_or(true);
    if d.st.is_none() && no_adds {
        return items.into_iter().map(|x| x.2).collect();
    }
    let mp: Mp<'a, M> = Rc::new(map);
    let ids: Vec<&str> = items.iter().map(|x| x.0).collect();
    let adds: Vec<Add> = pd.map(|p| p.in_bar(part, &ids).into_iter().cloned().collect()).unwrap_or_default();
    let mut out: Vec<Element<'a, M>> = Vec::new();
    let mut placed: Vec<String> = Vec::new();
    let push_after = |after: &str, out: &mut Vec<Element<'a, M>>, placed: &mut Vec<String>| {
        let mut cur = after.to_string();
        loop {
            let next: Vec<&Add> = adds.iter().filter(|a| a.after == cur && !placed.contains(&a.uid)).collect();
            let Some(a) = next.first() else { break };
            placed.push(a.uid.clone());
            out.push(add_el(d, pal, page, a, None, mp.clone()));
            cur = a.uid.clone();
        }
    };
    push_after("", &mut out, &mut placed);
    for (id, title, el) in items {
        out.push(spot(d, 3, page, part, id, title, el));
        push_after(id, &mut out, &mut placed);
    }
    for a in &adds {
        if !placed.contains(&a.uid) {
            out.push(add_el(d, pal, page, a, None, mp.clone()));
        }
    }
    out
}

/// A page laid out here (studio): the app's own view with the parts as the person set them and the blocks free in it
/// over it, or, once a part was moved, every part at its place (anywhere in the page, not only its sides), by layer;
/// while editing, the page in edit (outlines, labels, the blocks window, the toolbar) when it's the window's own.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page: &'static str,
    h: Length,
    original: impl FnOnce() -> Element<'a, M>,
    take: &dyn Fn(&'static str) -> Element<'a, M>,
    pane: &dyn Fn(&str) -> Option<Element<'a, M>>,
    map: Mp<'a, M>,
) -> Element<'a, M> {
    let p = *pal;
    let def = d.pages.iter().find(|x| x.id == page);
    let pd = d.studio_page(page);
    let moved = pd.map(|x| x.moved()).unwrap_or(false);
    let mut free: Vec<&Add> = pd.map(|x| x.adds.iter().filter(|a| a.into.is_empty() && a.at.is_some() && !a.hidden && !d.panes_closed.borrow().contains(&a.block)).collect()).unwrap_or_default();
    free.sort_by_key(|a| (a.style.layer == "under") as i8 * -1 + a.z.signum() as i8);
    let body: Element<'a, M> = match def {
        Some(def) if moved => {
            let mut parts: Vec<(i16, &'static str)> = def.blocks.iter().map(|b| (pd.and_then(|x| x.part(b.id)).map(|x| x.z + if x.style.layer == "under" { -500 } else if x.style.layer == "above" { 500 } else { 0 }).unwrap_or(0), b.id)).collect();
            parts.sort_by_key(|x| x.0);
            let mut kids: Vec<Element<'a, M>> = Vec::new();
            let mut places: Vec<Place> = Vec::new();
            let mut adds_under: Vec<&Add> = Vec::new();
            let mut adds_over: Vec<&Add> = Vec::new();
            for a in &free {
                if a.style.layer == "under" || a.z < 0 {
                    adds_under.push(*a);
                } else {
                    adds_over.push(*a);
                }
            }
            for a in adds_under {
                places.push(Place::At(a.at.unwrap_or_default(), None, None));
                kids.push(container(add_el(d, &p, page, a, Some(pane), map.clone())).width(Length::Fill).height(Length::Fill).clip(true).into());
            }
            for (_, id) in parts {
                let Some(b) = def.blocks.iter().find(|x| x.id == id) else { continue };
                let part = pd.and_then(|x| x.part(id));
                if part.map(|x| x.hidden).unwrap_or(false) {
                    continue;
                }
                let s = part.map(|x| &x.style);
                let fw = s.filter(|s| s.wmode == "fixed" && s.w > 0).map(|s| s.w as f32 + s.around[1] as f32 + s.around[3] as f32);
                let fh = s.filter(|s| s.h > 0).map(|s| s.h as f32 + s.around[0] as f32 + s.around[2] as f32);
                places.push(match part.and_then(|x| x.at) {
                    Some(a) => Place::At(a, fw, fh),
                    None => Place::Grid(b.x, b.y, b.w, b.h),
                });
                kids.push(container(take(b.id)).width(Length::Fill).height(Length::Fill).clip(true).into());
            }
            for a in adds_over {
                places.push(Place::At(a.at.unwrap_or_default(), None, None));
                kids.push(container(add_el(d, &p, page, a, Some(pane), map.clone())).width(Length::Fill).height(Length::Fill).clip(true).into());
            }
            Element::new(Free { kids, places, height: h })
        }
        _ => {
            let el = original();
            if free.is_empty() {
                el
            } else {
                let mut kids: Vec<Element<'a, M>> = Vec::new();
                let mut places: Vec<Place> = Vec::new();
                for a in &free {
                    places.push(Place::At(a.at.unwrap_or_default(), None, None));
                    kids.push(container(add_el(d, &p, page, a, Some(pane), map.clone())).width(Length::Fill).height(Length::Fill).clip(true).into());
                }
                stack![el, Element::new(Free { kids, places, height: Length::Fill })].height(h).into()
            }
        }
    };
    let title = def.map(|x| x.title).unwrap_or("");
    let body = spot(d, 0, page, "", "", title, body);
    match &d.st {
        Some(st) if st.root == page && !st.trying => editor(d, &p, body, map),
        Some(st) if st.root == page => trying_bar(&p, body, map),
        _ => body,
    }
}

// ------------------------------------------------------------------ the window being edited

/// What the editor draws over the page (outlines, the lit line, the guide), in window coordinates.
#[derive(Debug, Clone, Copy)]
enum Mark {
    Dash(Rectangle, Color),
    Fill(Rectangle, Color),
    Ring(Rectangle, Color, f32),
}

/// The page being edited: presses on its parts pick them, start moving them and open their right-click menu (the app
/// itself doesn't take them while it's edited), keys edit, and what's drawn over it is drawn.
struct Board<'a, M> {
    content: Element<'a, M>,
    hits: Vec<(Rectangle, Sel)>,
    marks: Vec<Mark>,
    on: Rc<dyn Fn(SMsg) -> M + 'a>,
    geo: &'a RefCell<Geo>,
    seen: u64,
}

#[derive(Default)]
struct BoardState {
    seen: u64,
    asked: bool,
    primed: bool,
}

fn key_of(key: &keyboard::Key, m: keyboard::Modifiers) -> Option<K> {
    use keyboard::key::Named as N;
    use keyboard::Key as Kk;
    Some(match key.as_ref() {
        Kk::Named(N::Escape) => K::Esc,
        Kk::Character(c) if m.command() && c.eq_ignore_ascii_case("z") && m.shift() => K::Redo,
        Kk::Character(c) if m.command() && c.eq_ignore_ascii_case("z") => K::Undo,
        Kk::Character(c) if m.command() && c.eq_ignore_ascii_case("y") => K::Redo,
        Kk::Named(N::Delete) => K::Delete,
        Kk::Named(N::Tab) => if m.shift() { K::Prev } else { K::Next },
        Kk::Named(N::Enter) => K::Enter,
        Kk::Named(N::ArrowLeft) => if m.shift() { K::Narrower } else { K::Left },
        Kk::Named(N::ArrowRight) => if m.shift() { K::Wider } else { K::Right },
        Kk::Named(N::ArrowUp) => if m.shift() { K::Shorter } else { K::Up },
        Kk::Named(N::ArrowDown) => if m.shift() { K::Taller } else { K::Down },
        _ => return None,
    })
}

impl<'a, M: Clone + 'a> Widget<M, Theme, Renderer> for Board<'a, M> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<BoardState>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(BoardState::default())
    }
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        // the app as it is, without the pointer (it isn't used while it's edited)
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, mouse::Cursor::Unavailable, viewport);
        let b = layout.bounds();
        renderer.with_layer(b, |r| {
            for m in &self.marks {
                match *m {
                    Mark::Fill(rc, c) => r.fill_quad(renderer::Quad { bounds: rc, ..renderer::Quad::default() }, Background::Color(c)),
                    Mark::Ring(rc, c, w) => r.fill_quad(renderer::Quad { bounds: rc, border: Border { color: c, width: w, radius: 4.0.into() }, ..renderer::Quad::default() }, Background::Color(Color::TRANSPARENT)),
                    Mark::Dash(rc, c) => dashes(r, rc, c),
                }
            }
        });
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        {
            // what was drawn changed since the view was made (a part grew, the window was resized): made again
            let st = tree.state.downcast_mut::<BoardState>();
            if st.seen != self.seen {
                st.seen = self.seen;
                st.asked = false;
            }
            let now = self.geo.try_borrow().map(|g| g.hash()).unwrap_or(self.seen);
            if now != self.seen && !st.asked {
                st.asked = true;
                shell.publish((self.on)(SMsg::Refresh));
            }
            if let Event::Window(iced::window::Event::Resized(_)) = event {
                shell.publish((self.on)(SMsg::Refresh));
            }
            // drawn once: one more frame, so what was drawn is measured and everything put over it in its place
            if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
                if !st.primed {
                    st.primed = true;
                    shell.request_redraw();
                }
            }
        }
        let b = layout.bounds();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(btn)) => {
                let Some(pt) = cursor.position_over(b) else { return };
                let hit = self.hits.iter().find(|(r, _)| r.contains(pt)).map(|(_, s)| s.clone());
                let m = match (btn, hit) {
                    (mouse::Button::Left, Some(s)) => Some(SMsg::Press(s, pt.x, pt.y)),
                    (mouse::Button::Left, None) => Some(SMsg::PressNothing),
                    (mouse::Button::Right, Some(s)) => Some(SMsg::RMenu(s, pt.x, pt.y)),
                    (mouse::Button::Right, None) => Some(SMsg::CloseMenu),
                    _ => None,
                };
                if let Some(m) = m {
                    shell.publish((self.on)(m));
                }
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(_)) if cursor.is_over(b) => {
                shell.capture_event();
                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if let Some(k) = key_of(key, *modifiers) {
                    shell.publish((self.on)(SMsg::Key(k)));
                    shell.capture_event();
                    return;
                }
            }
            _ => {}
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }
    fn mouse_interaction(&self, _tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, _viewport: &Rectangle, _renderer: &Renderer) -> mouse::Interaction {
        match cursor.position_over(layout.bounds()) {
            Some(pt) if self.hits.iter().any(|(r, _)| r.contains(pt)) => mouse::Interaction::Grab,
            _ => mouse::Interaction::None,
        }
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

/// A dashed outline (6 on, 4 off, 1.5 px).
fn dashes(r: &mut Renderer, rc: Rectangle, c: Color) {
    let (on, off, w) = (6.0, 4.0, 1.5);
    let q = |r: &mut Renderer, x: f32, y: f32, ww: f32, hh: f32| r.fill_quad(renderer::Quad { bounds: Rectangle { x, y, width: ww, height: hh }, ..renderer::Quad::default() }, Background::Color(c));
    let mut x = rc.x;
    while x < rc.x + rc.width {
        let len = on.min(rc.x + rc.width - x);
        q(r, x, rc.y, len, w);
        q(r, x, rc.y + rc.height - w, len, w);
        x += on + off;
    }
    let mut y = rc.y;
    while y < rc.y + rc.height {
        let len = on.min(rc.y + rc.height - y);
        q(r, rc.x, y, w, len);
        q(r, rc.x + rc.width - w, y, w, len);
        y += on + off;
    }
}

/// A little caps label in the accent ("COMMAND BAR · 48 px").
fn label<'a, M: 'a>(p: &Pal, s: String) -> Element<'a, M> {
    container(ts(s, 10.5, p.ink).wrapping(Wrapping::None)).padding(Padding { top: 2.0, right: 7.0, bottom: 2.0, left: 7.0 }).style(ui::boxed(p.accent, 3.0, None)).into()
}

/// A note in the guides' blue ("Preview: 514 px · lined up with Search", "Drop: command bar, after Sort").
fn note<'a, M: 'a>(p: &Pal, s: String) -> Element<'a, M> {
    container(t(s, 13.0, p.text).wrapping(Wrapping::None)).padding(Padding { top: 5.0, right: 10.0, bottom: 5.0, left: 10.0 }).style(ui::boxed(Pal::mix(p.bg, BLUE, 0.14), 5.0, Some((BLUE, 1.0)))).into()
}

/// A toolbar button (board 1 and 3's: dark, rounded; Keep this in the accent).
fn tbtn<'a, M: Clone + 'a>(p: &Pal, label: String, on: bool, primary: bool, msg: Option<M>) -> Element<'a, M> {
    let p = *p;
    let fg = if primary { p.ink } else if on { p.accent } else { p.text };
    let bg = if primary { p.accent } else { Pal::mix(p.panel, p.text, 0.04) };
    let c = container(tm(label, 14.0, fg).wrapping(Wrapping::None)).padding(Padding { top: 7.0, right: 14.0, bottom: 7.0, left: 14.0 }).style(ui::boxed(bg, p.r(8.0), Some((if primary { p.accent } else if on { Pal::a(p.accent, 0.6) } else { Pal::a(p.line, 0.9) }, 1.0))));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

fn vsep<'a, M: 'a>(p: &Pal) -> Element<'a, M> {
    container(Space::new()).width(Length::Fixed(1.0)).height(Length::Fixed(26.0)).style(ui::boxed(p.line, 0.0, None)).into()
}

/// A part's label: what it is and its size, as board 3 has them.
fn part_label(title: &str, r: &R) -> String {
    let name = short(title).to_uppercase();
    if r.is_bar() && r.across() {
        format!("{name} · {} px", r.h.round() as i64)
    } else if !r.across() && r.w <= 420.0 {
        format!("{name} · {} px · DRAG ITS EDGE", r.w.round() as i64)
    } else {
        format!("{name} · {} × {}", r.w.round() as i64, r.h.round() as i64)
    }
}

/// The page being edited, with everything board 3 puts over it.
fn editor<'a, M: Clone + 'a>(d: &'a Describe, p: &Pal, body: Element<'a, M>, map: Mp<'a, M>) -> Element<'a, M> {
    let p = *p;
    let Some(st) = d.st.as_ref() else { return body };
    let geo = d.geo.borrow();
    let root = geo.page(st.root).unwrap_or_default();
    let (ox, oy) = (root.x, root.y);
    let sm_ = {
        let m = map.clone();
        Rc::new(move |x: SMsg| m(DMsg::Studio(x))) as Rc<dyn Fn(SMsg) -> M + 'a>
    };
    let s = |x: SMsg| sm_(x);
    // what can be picked: blocks dragged in first (they're small), then the parts, smallest first
    let mut hits: Vec<(Rectangle, Sel)> = Vec::new();
    let mut marks: Vec<Mark> = Vec::new();
    let mut layer: Vec<Element<'a, M>> = Vec::new();
    let at = |e: Element<'a, M>, x: f32, y: f32| -> Element<'a, M> { pin(e).x(x - ox).y(y - oy).into() };
    let mut adds: Vec<&Rec> = geo.now().filter(|r| r.kind == 2).collect();
    adds.sort_by(|a, b| rr(a.rect).area().total_cmp(&rr(b.rect).area()));
    for r in &adds {
        if let Some(pg) = d.pages.iter().find(|x| x.id == r.page) {
            hits.push((r.rect, Sel::Add(pg.id, r.id.clone())));
            let picked = st.pick == Some(Sel::Add(pg.id, r.id.clone()));
            marks.push(if picked { Mark::Ring(r.rect, p.accent, 2.0) } else { Mark::Dash(r.rect, Pal::a(p.accent, 0.55)) });
        }
    }
    let mut parts: Vec<&Rec> = geo.now().filter(|r| r.kind == 1 && d.pages.iter().any(|x| x.id == r.page)).collect();
    parts.sort_by(|a, b| rr(a.rect).area().total_cmp(&rr(b.rect).area()));
    let mut handles: Vec<(f32, f32)> = Vec::new();
    for r in &parts {
        let holder = geo.holder(r);
        let Some(id) = d.pages.iter().find(|x| x.id == r.page).and_then(|x| x.blocks.iter().find(|b| b.id == r.id)).map(|b| b.id) else { continue };
        let page = d_id(d.pages, r.page);
        let sel = Sel::Part(page, id);
        let picked = st.pick.as_ref() == Some(&sel);
        if holder {
            marks.push(Mark::Dash(r.rect, Pal::a(p.accent, 0.25)));
            continue;
        }
        hits.push((r.rect, sel.clone()));
        let inset = Rectangle { x: r.rect.x + 2.0, y: r.rect.y + 2.0, width: (r.rect.width - 4.0).max(0.0), height: (r.rect.height - 4.0).max(0.0) };
        marks.push(Mark::Dash(inset, p.accent));
        if picked {
            marks.push(Mark::Ring(inset, p.accent, 2.0));
            marks.push(Mark::Fill(inset, Pal::a(p.accent, 0.06)));
        }
        let rect_r = rr(r.rect);
        // its label: under it, inside the window
        let txt = part_label(&r.title, &rect_r);
        let lx = r.rect.x + 2.0;
        let below = r.rect.y + r.rect.height + 2.0;
        let ly = if below + 20.0 <= root.y + root.h { below } else { (r.rect.y + r.rect.height - 22.0).max(r.rect.y) };
        layer.push(at(label(&p, txt), lx, ly));
        // its edges' handles (a part's sides that aren't the window's), and all four on the one picked
        let sides: Vec<Side> = if picked {
            vec![Side::Left, Side::Right, Side::Top, Side::Bottom]
        } else if rect_r.is_bar() && rect_r.across() {
            Vec::new()
        } else {
            vec![Side::Left, Side::Right]
        };
        for side in sides {
            let (hx, hy) = match side {
                Side::Left => (r.rect.x, r.rect.y + r.rect.height / 2.0),
                Side::Right => (r.rect.x + r.rect.width, r.rect.y + r.rect.height / 2.0),
                Side::Top => (r.rect.x + r.rect.width / 2.0, r.rect.y),
                Side::Bottom => (r.rect.x + r.rect.width / 2.0, r.rect.y + r.rect.height),
            };
            let at_edge = match side {
                Side::Left => (hx - root.x).abs() < 3.0,
                Side::Right => (root.x + root.w - hx).abs() < 3.0,
                Side::Top => (hy - root.y).abs() < 3.0,
                Side::Bottom => (root.y + root.h - hy).abs() < 3.0,
            };
            if (at_edge && !picked) || handles.iter().any(|(x, y)| (x - hx).abs() < 12.0 && (y - hy).abs() < 40.0) {
                continue;
            }
            handles.push((hx, hy));
            let cur = if matches!(side, Side::Left | Side::Right) { mouse::Interaction::ResizingHorizontally } else { mouse::Interaction::ResizingVertically };
            let h: Element<'a, M> = mouse_area(container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(ui::boxed(p.accent, 2.0, Some((p.ink, 1.0)))))
                .on_press(s(SMsg::GrabSide(sel.clone(), side)))
                .interaction(cur)
                .into();
            layer.push(at(h, hx - 5.0, hy - 5.0));
        }
    }
    // a pane dragged in: its edges size it too
    if let Some(Sel::Add(pg, u)) = &st.pick {
        if let Some(r) = geo.add(pg, u) {
            for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
                let (hx, hy) = match side {
                    Side::Left => (r.x, r.y + r.h / 2.0),
                    Side::Right => (r.right(), r.y + r.h / 2.0),
                    Side::Top => (r.x + r.w / 2.0, r.y),
                    Side::Bottom => (r.x + r.w / 2.0, r.bottom()),
                };
                let cur = if matches!(side, Side::Left | Side::Right) { mouse::Interaction::ResizingHorizontally } else { mouse::Interaction::ResizingVertically };
                let h: Element<'a, M> = mouse_area(container(Space::new().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(ui::boxed(p.accent, 2.0, Some((p.ink, 1.0)))))
                    .on_press(s(SMsg::GrabSide(Sel::Add(pg, u.clone()), side)))
                    .interaction(cur)
                    .into();
                layer.push(at(h, hx - 5.0, hy - 5.0));
            }
        }
    }
    // the drag: the lit line where it lands (and where), the free place, the guide and its note, the block itself
    if let Some(dr) = &st.drag {
        if dr.moved {
            match &dr.target {
                Some(Target::Bar { line, say, .. }) => {
                    marks.push(Mark::Fill(*line, p.accent));
                    marks.push(Mark::Fill(Rectangle { x: line.x - 2.0, y: line.y - 2.0, width: line.width + 4.0, height: line.height + 4.0 }, Pal::a(p.accent, 0.25)));
                    layer.push(at(note(&p, say.clone()), line.x + 14.0, line.y + line.height + 10.0));
                }
                Some(Target::Free { abs, .. }) => {
                    marks.push(Mark::Fill(*abs, Pal::a(p.accent, 0.08)));
                    marks.push(Mark::Dash(*abs, p.accent));
                }
                None => {}
            }
            if let Some(g) = &dr.guide {
                marks.push(Mark::Fill(g.line, BLUE));
                let w = (g.note.chars().count() as f32 * 7.2 + 24.0).min(520.0);
                let x = if g.at.x - w < root.x + 8.0 { g.at.x + 12.0 } else { g.at.x - w };
                let x = if matches!(dr.kind, DragKind::Side(..)) { x } else { g.at.x };
                layer.push(at(note(&p, g.note.clone()), x, g.at.y));
            }
            let ghost = match &dr.kind {
                DragKind::Tile(block, title) => Some((block.clone(), title.clone())),
                DragKind::Add(pg, u) => d.studio_page(pg).and_then(|x| x.add(u)).map(|a| (a.block.clone(), a.title.clone())),
                _ => None,
            };
            if let Some((block, title)) = ghost {
                layer.push(at(ghost_el(&p, &block, title), st.pointer.0 - 30.0, st.pointer.1 - 26.0));
            }
        }
    }
    // the blocks window (its own window, moved by its title), unless the side panel is open where it sits
    if st.win_open && !st.panel {
        let (wx, wy) = st.win.unwrap_or_else(|| win_default(root));
        layer.push(pin(blocks_window(d, st, &p, &sm_)).x(wx).y(wy).into());
    }
    // the side panel (Right-click any part › Customize)
    if st.panel {
        if let Some(sel) = st.pick.clone() {
            let w = 520.0_f32.min(root.w * 0.5);
            let panel = side_panel(d, st, &p, &sel, &sm_);
            layer.push(pin(container(panel).width(Length::Fixed(w)).height(Length::Fixed((root.h - 96.0).max(200.0)))).x((root.w - w - 16.0).max(0.0)).y(16.0).into());
        }
    }
    // the toolbar along the bottom, its line over it, and what it says
    let bar = toolbar(d, st, &p, &sm_);
    let line = container(ts("THE SAME IN EVERY HEARTH APP · SETTINGS · TASK MANAGER · CAPTURE · NOTES · STORE … · RIGHT-CLICK ANY PART › CUSTOMIZE", 10.5, p.muted).wrapping(Wrapping::None))
        .padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 })
        .style(ui::boxed(Pal::a(p.bg, 0.85), 4.0, Some((Pal::a(p.line, 0.8), 1.0))));
    let mut foot = Column::new().spacing(8.0).align_x(iced::Center);
    if !st.note.is_empty() {
        foot = foot.push(container(t(st.note.clone(), 13.0, if st.bad { p.danger } else { p.text })).padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 }).style(ui::boxed(Pal::a(p.panel, 0.95), 6.0, Some((if st.bad { p.danger } else { p.line }, 1.0)))));
    }
    if st.pop != Pop::None {
        foot = foot.push(opaque(pop_el(d, st, &p, &sm_)));
    }
    foot = foot.push(line).push(opaque(bar));
    layer.push(container(foot).width(Length::Fill).height(Length::Fill).align_x(iced::Center).align_y(iced::Bottom).padding(Padding { top: 0.0, right: 0.0, bottom: 14.0, left: 0.0 }).into());
    // the right-click menu
    if let Some((sel, mx, my)) = &st.menu {
        let m = part_menu(d, st, &p, sel, &sm_);
        let x = (mx - ox).min(root.w - 250.0).max(0.0);
        let y = (my - oy).min(root.h - 300.0).max(0.0);
        layer.push(pin(opaque(m)).x(x).y(y).into());
    }
    // while something's dragged: everything the pointer does goes to it
    if st.drag.is_some() {
        let m1 = sm_.clone();
        layer.push(
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_move(move |pt: Point| m1(SMsg::Move(pt.x + ox, pt.y + oy)))
                .on_release(s(SMsg::Release))
                .interaction(mouse::Interaction::Grabbing)
                .into(),
        );
    }
    let seen = geo.hash();
    drop(geo);
    let board = Element::new(Board { content: body, hits, marks, on: sm_.clone(), geo: &d.geo, seen });
    stack![board, stack(layer).width(Length::Fill).height(Length::Fill)].into()
}

/// What's dragged, following the pointer: tilted a little, with a shadow.
fn ghost_el<'a, M: 'a>(p: &Pal, block: &str, title: String) -> Element<'a, M> {
    struct Tilt(Color, Color, f32);
    impl<M> canvas::Program<M> for Tilt {
        type State = ();
        fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
            let mut f = Frame::new(renderer, b.size());
            f.with_save(|f| {
                f.translate(Vector::new(b.width / 2.0, b.height / 2.0));
                f.rotate(-0.035);
                let sz = Size::new(b.width - 12.0, b.height - 12.0);
                let tl = Point::new(-sz.width / 2.0, -sz.height / 2.0);
                f.fill(&Path::rounded_rectangle(Point::new(tl.x + 3.0, tl.y + 5.0), sz, 8.0.into()), Color::from_rgba(0.0, 0.0, 0.0, 0.35));
                f.fill(&Path::rounded_rectangle(tl, sz, 8.0.into()), self.0);
                f.stroke(&Path::rounded_rectangle(tl, sz, 8.0.into()), canvas::Stroke::default().with_color(self.1).with_width(self.2));
            });
            vec![f.into_geometry()]
        }
    }
    let p = *p;
    let w = (title.chars().count() as f32 * 7.6 + 64.0).clamp(120.0, 320.0);
    let sq = if block.starts_with("own:") || block.starts_with("mine:") { p.accent } else { p.accent };
    let face: Element<'a, M> = container(row![container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(sq, 3.0, None)), t(title, 14.0, p.text).wrapping(Wrapping::None)].spacing(10.0).align_y(iced::Center))
        .center_y(Length::Fill)
        .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 20.0 })
        .into();
    stack![canvas::Canvas::new(Tilt(Pal::mix(p.panel, p.accent, 0.10), p.accent, 1.5)).width(Length::Fixed(w)).height(Length::Fixed(46.0)), face].width(Length::Fixed(w)).height(Length::Fixed(46.0)).into()
}

/// Trying it: the app as it would be, still not kept, with a bar to go back, cancel or keep.
fn trying_bar<'a, M: Clone + 'a>(p: &Pal, body: Element<'a, M>, map: Mp<'a, M>) -> Element<'a, M> {
    let p = *p;
    let s = |x: SMsg| map(DMsg::Studio(x));
    let bar = container(
        row![
            t("Trying it · nothing is kept until Keep this", 13.0, p.text).wrapping(Wrapping::None),
            tbtn(&p, "Back to editing".into(), false, false, Some(s(SMsg::Back))),
            tbtn(&p, "Cancel".into(), false, false, Some(s(SMsg::Cancel))),
            tbtn(&p, "Keep this".into(), false, true, Some(s(SMsg::Keep))),
        ]
        .spacing(10.0)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 16.0 })
    .style(ui::boxed(p.panel, p.r(12.0), Some((p.accent, 1.5))));
    stack![body, container(opaque(bar)).width(Length::Fill).height(Length::Fill).align_x(iced::Center).align_y(iced::Bottom).padding(16.0)].into()
}

/// The toolbar: Undo, Redo, Snap to grid · 8 px, Guides, Layers, Start from, Menus…, Cancel, Try it, Keep this.
fn toolbar<'a, M: Clone + 'a>(_d: &'a Describe, st: &'a Studio, p: &Pal, sm_: &Rc<dyn Fn(SMsg) -> M + 'a>) -> Element<'a, M> {
    let p = *p;
    let s = |x: SMsg| sm_(x);
    let start = sm::STARTS.iter().find(|x| x.0 == st.start).map(|x| x.1).unwrap_or("Hearth");
    let start = if start.len() > 18 { start.split(' ').next().unwrap_or(start) } else { start };
    let snap = {
        let fg = if st.s.snap { p.accent } else { p.text };
        let c = container(
            row![
                mouse_area(tm("▦ Snap to grid", 14.0, fg).wrapping(Wrapping::None)).on_press(s(SMsg::Snap)).interaction(mouse::Interaction::Pointer),
                mouse_area(tm(format!(" · {} px", st.s.grid), 14.0, fg).wrapping(Wrapping::None)).on_press(s(SMsg::Grid)).interaction(mouse::Interaction::Pointer)
            ]
            .align_y(iced::Center),
        )
        .padding(Padding { top: 7.0, right: 14.0, bottom: 7.0, left: 14.0 })
        .style(ui::boxed(Pal::mix(p.panel, p.text, 0.04), p.r(8.0), Some((if st.s.snap { Pal::a(p.accent, 0.6) } else { Pal::a(p.line, 0.9) }, 1.0))));
        Element::from(c)
    };
    let r = row![
        tbtn(&p, "↶ Undo".into(), false, false, Some(s(SMsg::Undo))),
        tbtn(&p, "↷ Redo".into(), false, false, Some(s(SMsg::Redo))),
        vsep(&p),
        snap,
        tbtn(&p, "┼ Guides".into(), st.s.guides, false, Some(s(SMsg::Guides))),
        tbtn(&p, "◫ Layers".into(), st.pop == Pop::Layers, false, Some(s(SMsg::Pop(Pop::Layers)))),
        vsep(&p),
        tbtn(&p, format!("Start from: {start} ▾"), st.pop == Pop::Start, false, Some(s(SMsg::Pop(Pop::Start)))),
        tbtn(&p, "Menus…".into(), st.pop == Pop::Menus, false, Some(s(SMsg::Pop(Pop::Menus)))),
        vsep(&p),
        tbtn(&p, "Cancel".into(), false, false, Some(s(SMsg::Cancel))),
        tbtn(&p, "Try it".into(), false, false, Some(s(SMsg::Try))),
        tbtn(&p, if st.busy { "Keeping…".into() } else { "Keep this".into() }, false, true, if st.busy { None } else { Some(s(SMsg::Keep)) }),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    container(r).padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 }).style(ui::boxed(Pal::a(p.bg2, 0.97), p.r(12.0), Some((p.line, 1.0)))).into()
}

/// The toolbar's pop-ups: Layers, Start from, Menus….
fn pop_el<'a, M: Clone + 'a>(d: &'a Describe, st: &'a Studio, p: &Pal, sm_: &Rc<dyn Fn(SMsg) -> M + 'a>) -> Element<'a, M> {
    let p = *p;
    let s = |x: SMsg| sm_(x);
    let small = |label: &'static str, m: M| -> Element<'a, M> {
        mouse_area(container(t(label, 12.0, p.text)).padding(Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 }).style(ui::boxed(Color::TRANSPARENT, 4.0, Some((p.line, 1.0))))).on_press(m).interaction(mouse::Interaction::Pointer).into()
    };
    let mut c = Column::new().spacing(6.0).width(Length::Fixed(560.0));
    match st.pop {
        Pop::Layers => {
            c = c.push(ts("LAYERS · top first · the eye hides or shows", 11.0, p.muted));
            let pages = d.geo.borrow().pages();
            for pg in pages.iter().rev() {
                let Some(def) = d.pages.iter().find(|x| x.id == *pg) else { continue };
                c = c.push(tm(def.title, 13.0, p.muted));
                let pd = st.s.draft.page(pg);
                let mut rows: Vec<(i16, Sel, String, bool)> = Vec::new();
                for b in def.blocks {
                    let part = pd.and_then(|x| x.part(b.id));
                    let z = part.map(|x| x.z + if x.style.layer == "under" { -500 } else if x.style.layer == "above" { 500 } else { 0 }).unwrap_or(0);
                    rows.push((z, Sel::Part(def.id, b.id), short(b.title), part.map(|x| x.hidden).unwrap_or(false)));
                }
                for a in pd.map(|x| x.adds.iter().collect::<Vec<_>>()).unwrap_or_default() {
                    rows.push((a.z + 1, Sel::Add(def.id, a.uid.clone()), format!("{} (yours)", a.title), a.hidden));
                }
                rows.sort_by(|a, b| b.0.cmp(&a.0));
                for (_, sel, name, hidden) in rows {
                    let picked = st.pick.as_ref() == Some(&sel);
                    c = c.push(
                        row![
                            mouse_area(container(t(if hidden { "◌" } else { "◉" }, 14.0, if hidden { p.muted } else { p.accent })).width(Length::Fixed(22.0))).on_press(s(SMsg::Eye(sel.clone()))).interaction(mouse::Interaction::Pointer),
                            mouse_area(container(t(name, 13.0, if picked { p.accent } else if hidden { p.muted } else { p.text }).wrapping(Wrapping::None)).width(Length::Fill)).on_press(s(SMsg::Customize(sel.clone()))).interaction(mouse::Interaction::Pointer),
                            small("Up", s(SMsg::Layer(sel.clone(), 1))),
                            small("Down", s(SMsg::Layer(sel, -1))),
                        ]
                        .spacing(8.0)
                        .align_y(iced::Center),
                    );
                }
            }
        }
        Pop::Start => {
            c = c.push(ts("START FROM · a ready-made layout (Undo has the one before)", 11.0, p.muted));
            for (id, name) in sm::STARTS {
                let on = st.start == *id;
                c = c.push(mouse_area(container(t(*name, 13.0, if on { p.accent } else { p.text })).padding(Padding { top: 5.0, right: 8.0, bottom: 5.0, left: 8.0 }).width(Length::Fill).style(ui::boxed(if on { Pal::a(p.accent, 0.10) } else { Color::TRANSPARENT }, 6.0, None))).on_press(s(SMsg::StartFrom(id))).interaction(mouse::Interaction::Pointer));
            }
        }
        Pop::Menus => {
            let name = d.kit.map(|k| k.possessive()).unwrap_or_else(|| format!("{}'s", d.name));
            c = c.push(ts(format!("MENUS · {} menus: their items in order, Up and Down to move, Hide to take one out", name.to_uppercase()), 11.0, p.muted));
            let menus: Vec<&MenuDef> = d.kit.map(|k| k.menus.iter().collect::<Vec<&MenuDef>>()).unwrap_or_default().into_iter().chain(std::iter::once(&PART_MENU)).collect();
            for m in menus {
                c = c.push(tm(m.title, 13.0, p.text));
                let order = order_of(&st.s.draft, m);
                let n = order.len();
                for (i, id) in order.into_iter().enumerate() {
                    let label = m.items.iter().find(|x| x.0 == id).map(|x| x.1).unwrap_or("");
                    let hidden = st.s.draft.hidden_items.contains(&id);
                    let mut r = Row::new().spacing(8.0).align_y(iced::Center).push(container(t(label, 13.0, if hidden { p.muted } else { p.text })).width(Length::Fill));
                    if i > 0 {
                        r = r.push(small("Up", s(SMsg::MenuMove(m.id, i, -1))));
                    }
                    if i + 1 < n {
                        r = r.push(small("Down", s(SMsg::MenuMove(m.id, i, 1))));
                    }
                    if m.id != PART_MENU.id {
                        r = r.push(small(if hidden { "Show" } else { "Hide" }, s(SMsg::MenuHide(m.id, id.clone()))));
                    }
                    c = c.push(container(r).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 12.0 }));
                }
            }
        }
        Pop::None => {}
    }
    container(scrollable(c).height(Length::Shrink)).max_height(420.0).padding(12.0).style(ui::boxed(Pal::a(p.panel, 0.98), p.r(12.0), Some((p.line, 1.0)))).into()
}

/// The right-click menu on a part while editing (board 2's).
fn part_menu<'a, M: Clone + 'a>(d: &'a Describe, st: &'a Studio, p: &Pal, sel: &Sel, sm_: &Rc<dyn Fn(SMsg) -> M + 'a>) -> Element<'a, M> {
    let p = *p;
    let s = |x: SMsg| sm_(x);
    let item = |label: String, c: Color, m: Option<M>| -> Element<'a, M> {
        let e = container(t(label, 14.0, if m.is_some() { c } else { p.muted }).wrapping(Wrapping::None)).padding(Padding { top: 7.0, right: 16.0, bottom: 7.0, left: 14.0 }).width(Length::Fill);
        match m {
            Some(m) => mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into(),
            None => e.into(),
        }
    };
    let is_add = matches!(sel, Sel::Add(..));
    let mut c = Column::new().width(Length::Fixed(240.0));
    c = c.push(item("Customize…".into(), p.text, Some(s(SMsg::Customize(sel.clone())))));
    c = c.push(item("Copy its look".into(), p.text, Some(s(SMsg::CopyLook(sel.clone())))));
    c = c.push(item("Paste a look".into(), p.text, st.look.as_ref().map(|_| s(SMsg::PasteLook(sel.clone())))));
    c = c.push(item("Duplicate".into(), p.text, if is_add { Some(s(SMsg::Duplicate(sel.clone()))) } else { None }));
    c = c.push(item(format!("Move to {}", if st.moveto { "▾" } else { "›" }), p.text, Some(s(SMsg::MoveToOpen))));
    if st.moveto {
        let page = sel.page();
        let mut targets: Vec<(String, String)> = Vec::new();
        if is_add {
            if let Some(def) = d.pages.iter().find(|x| x.id == page) {
                let geo = d.geo.borrow();
                for b in def.blocks {
                    if geo.part(page, b.id).map(|r| r.is_bar()).unwrap_or(false) {
                        targets.push((b.id.to_string(), short(b.title)));
                    }
                }
            }
            targets.push((String::new(), "Free in the window".into()));
        } else {
            for (k, n) in [("left", "The left side"), ("right", "The right side"), ("top", "The top"), ("bottom", "The bottom"), ("free", "Free, in the middle")] {
                targets.push((k.to_string(), n.to_string()));
            }
        }
        for (k, n) in targets {
            c = c.push(container(item(format!("› {n}"), p.text, Some(s(SMsg::MoveTo(sel.clone(), k))))).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 14.0 }));
        }
    }
    c = c.push(item("Hide (keep it here)".into(), p.text, Some(s(SMsg::Hide(sel.clone())))));
    c = c.push(item("Take it out".into(), p.danger, Some(s(SMsg::TakeOut(sel.clone())))));
    container(c).padding(6.0).style(ui::boxed(Pal::a(p.panel, 0.99), p.r(12.0), Some((p.line, 1.0)))).into()
}

/// The app's own blocks window: "Files' blocks", "drag into Files", Find a block (the count), All, Buttons, Panes,
/// Info, Your own, and the blocks to drag in.
fn blocks_window<'a, M: Clone + 'a>(d: &'a Describe, st: &'a Studio, p: &Pal, sm_: &Rc<dyn Fn(SMsg) -> M + 'a>) -> Element<'a, M> {
    let p = *p;
    let s = |x: SMsg| sm_(x);
    let kit = d.kit;
    let name = kit.map(|k| k.name).unwrap_or(d.name);
    let poss = kit.map(|k| k.possessive()).unwrap_or_else(|| format!("{name}'s"));
    let own_kinds: [(&str, &str); 2] = [("own:button", "Button · runs anything"), ("own:words", "Count · words · spacer")];
    let mine: Vec<(String, String)> = st.s.draft.own.iter().map(|o| (format!("mine:{}", o.id), o.name.clone())).collect();
    let n_all = kit.map(|k| k.buttons.len() + k.panes.len() + k.info.len()).unwrap_or(0) + own_kinds.len() + mine.len();
    let q = st.find.to_lowercase();
    let hit = |title: &str| q.is_empty() || title.to_lowercase().contains(&q);
    let tile_el = |block: String, title: String, own: bool| -> Element<'a, M> {
        let sq = container(Space::new()).width(Length::Fixed(18.0)).height(Length::Fixed(18.0)).style(ui::boxed(if own { p.accent } else { Pal::mix(p.text, p.bg, 0.12) }, 3.0, None));
        let c = container(column![sq, t(title.clone(), 12.5, p.text).wrapping(Wrapping::None)].spacing(8.0).align_x(iced::Center))
            .center_x(Length::Fill)
            .padding(Padding { top: 12.0, right: 6.0, bottom: 11.0, left: 6.0 })
            .style(ui::boxed(Pal::mix(p.panel, p.text, 0.035), p.r(8.0), Some((if own { Pal::a(p.accent, 0.25) } else { Pal::a(p.line, 0.9) }, 1.0))));
        mouse_area(c).on_press(s(SMsg::GrabTile(block, title))).interaction(mouse::Interaction::Grab).into()
    };
    let grid = |tiles: Vec<Element<'a, M>>| -> Element<'a, M> {
        let mut c = Column::new().spacing(8.0);
        let mut it = tiles.into_iter();
        loop {
            let Some(a) = it.next() else { break };
            let b: Element<'a, M> = it.next().unwrap_or_else(|| Space::new().width(Length::Fill).into());
            c = c.push(row![container(a).width(Length::FillPortion(1)), container(b).width(Length::FillPortion(1))].spacing(8.0));
        }
        c.into()
    };
    let head = |s2: &'static str| -> Element<'a, M> { container(ts(s2, 11.0, p.muted).wrapping(Wrapping::None)).padding(Padding { top: 6.0, right: 0.0, bottom: 2.0, left: 0.0 }).into() };
    let mut body = Column::new().spacing(8.0);
    let k = st.kind;
    if let Some(kit) = kit {
        if k == 0 || k == 1 {
            let tiles: Vec<Element<'a, M>> = kit.buttons.iter().filter(|x| hit(x.title)).map(|x| tile_el(format!("act:{}", x.id), x.title.to_string(), false)).collect();
            if !tiles.is_empty() {
                body = body.push(head("BUTTONS")).push(grid(tiles));
            }
        }
        if k == 0 || k == 2 {
            let tiles: Vec<Element<'a, M>> = kit.panes.iter().filter(|x| hit(x.title)).map(|x| tile_el(format!("pane:{}", x.id), x.title.to_string(), false)).collect();
            if !tiles.is_empty() {
                body = body.push(head("PANES")).push(grid(tiles));
            }
        }
        if k == 0 || k == 3 {
            let tiles: Vec<Element<'a, M>> = kit.info.iter().filter(|x| hit(x.title)).map(|x| tile_el(format!("info:{}", x.id), x.title.to_string(), false)).collect();
            if !tiles.is_empty() {
                body = body.push(head("INFO")).push(grid(tiles));
            }
        }
    }
    if k == 0 || k == 4 {
        let tiles: Vec<Element<'a, M>> = mine.iter().filter(|x| hit(&x.1)).map(|x| tile_el(x.0.clone(), x.1.clone(), true)).collect();
        if !tiles.is_empty() {
            body = body.push(head("YOUR OWN")).push(grid(tiles));
        }
        let tiles: Vec<Element<'a, M>> = own_kinds.iter().filter(|x| hit(x.1)).map(|x| tile_el(x.0.to_string(), x.1.to_string(), true)).collect();
        if !tiles.is_empty() {
            body = body.push(head("MAKE YOUR OWN")).push(grid(tiles));
        }
    }
    let kinds = ["All", "Buttons", "Panes", "Info", "Your own"];
    let mut kr = Row::new().spacing(6.0);
    for (i, kname) in kinds.iter().enumerate() {
        let on = i == st.kind;
        kr = kr.push(mouse_area(container(t(*kname, 12.0, if on { p.ink } else { p.muted }).wrapping(Wrapping::None)).padding(Padding { top: 4.0, right: 9.0, bottom: 4.0, left: 9.0 }).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 6.0, Some((if on { p.accent } else { p.line }, 1.0))))).on_press(s(SMsg::Kind(i))).interaction(mouse::Interaction::Pointer));
    }
    let find = text_input(&format!("⌕ Find a block ({n_all})"), &st.find).on_input({
        let m = sm_.clone();
        move |x| m(SMsg::Find(x))
    })
    .size(13.0)
    .padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 });
    let title = mouse_area(
        row![
            container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(p.accent, 6.0, None)),
            ts(format!("{poss} blocks"), 14.0, p.text).wrapping(Wrapping::None),
            Space::new().width(Length::Fill),
            t(format!("drag into {name}"), 13.0, p.muted).wrapping(Wrapping::None),
            mouse_area(t("  ✕", 13.0, p.muted)).on_press(s(SMsg::WinToggle)).interaction(mouse::Interaction::Pointer),
        ]
        .spacing(10.0)
        .align_y(iced::Center),
    )
    .on_press(s(SMsg::GrabWin))
    .interaction(mouse::Interaction::Grab);
    let col = column![
        container(title).padding(Padding { top: 12.0, right: 14.0, bottom: 12.0, left: 14.0 }),
        ui::hline(Pal::a(p.line, 0.8)),
        container(column![find, kr, scrollable(body).height(Length::Fixed(470.0))].spacing(12.0)).padding(Padding { top: 12.0, right: 14.0, bottom: 14.0, left: 14.0 })
    ];
    opaque(container(col).width(Length::Fixed(380.0)).style(ui::boxed(Pal::a(p.bg2, 0.98), p.r(12.0), Some((p.line, 1.0)))))
}

// ------------------------------------------------------------------ board 2's side panel, for any part

fn seg<'a, M: Clone + 'a>(p: &Pal, opts: &[(&'static str, &'static str)], cur: &str, f: impl Fn(&'static str) -> M) -> Element<'a, M> {
    let p = *p;
    let mut r = Row::new().spacing(0.0);
    for (v, label) in opts {
        let on = *v == cur;
        r = r.push(mouse_area(container(t(*label, 12.5, if on { p.ink } else { p.muted }).wrapping(Wrapping::None)).padding(Padding { top: 4.0, right: 9.0, bottom: 4.0, left: 9.0 }).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 4.0, None))).on_press(f(*v)).interaction(mouse::Interaction::Pointer));
    }
    container(r).padding(1.0).style(ui::boxed(Color::TRANSPARENT, 5.0, Some((p.line, 1.0)))).into()
}

fn field<'a, M: Clone + 'a>(p: &Pal, value: String, suffix: &'static str, w: f32, on: impl Fn(String) -> M + 'a) -> Element<'a, M> {
    let p = *p;
    let i = text_input("", &value).on_input(on).size(12.5).padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 8.0 }).width(Length::Fixed(w));
    if suffix.is_empty() {
        i.into()
    } else {
        row![i, t(suffix, 12.0, p.muted)].spacing(4.0).align_y(iced::Center).into()
    }
}

fn swatch<'a, M: Clone + 'a>(p: &Pal, c: Color, on: bool, m: M) -> Element<'a, M> {
    let p = *p;
    mouse_area(container(Space::new()).width(Length::Fixed(20.0)).height(Length::Fixed(20.0)).style(ui::boxed(c, 4.0, Some((if on { p.text } else { p.line }, if on { 2.0 } else { 1.0 }))))).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

fn hexs(c: Color) -> String {
    let [r, g, b, _] = c.into_rgba8();
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// board 2's side panel for the picked part or block: its name, where it is, Reset; Look, Shows, Does, Place, When,
/// Motion; Save as my own block…, Copy its look, Apply to every bar item.
fn side_panel<'a, M: Clone + 'a>(d: &'a Describe, st: &'a Studio, p: &Pal, sel: &Sel, sm_: &Rc<dyn Fn(SMsg) -> M + 'a>) -> Element<'a, M> {
    let p = *p;
    let s = |x: SMsg| sm_(x);
    let sty = style_of(&st.s.draft, sel);
    let set = {
        let sel = sel.clone();
        let m = sm_.clone();
        Rc::new(move |f: F| m(SMsg::Set(sel.clone(), f)))
    };
    let title = d.sel_title(sel);
    let page_title = d.pages.iter().find(|x| x.id == sel.page()).map(|x| x.title).unwrap_or("");
    let where_ = match sel {
        Sel::Part(..) => format!("· a part of {}", short(page_title).to_lowercase()),
        Sel::Add(pg, u) => match d.studio_page(pg).and_then(|x| x.add(u)) {
            Some(a) if !a.into.is_empty() => format!("· in the {}", d.pages.iter().find(|x| x.id == *pg).and_then(|x| x.blocks.iter().find(|b| b.id == a.into)).map(|b| short(b.title).to_lowercase()).unwrap_or_default()),
            _ => "· free in the window".to_string(),
        },
    };
    let row_ = |label: &'static str, right: Element<'a, M>| -> Element<'a, M> { row![container(t(label, 13.0, p.text)).width(Length::Fill), right].spacing(10.0).align_y(iced::Center).into() };
    let head = |x: &'static str| -> Element<'a, M> { container(ts(x, 11.0, p.muted).wrapping(Wrapping::None)).padding(Padding { top: 10.0, right: 0.0, bottom: 2.0, left: 0.0 }).into() };
    let mut c = Column::new().spacing(9.0);
    let picker_item = |label: String, on: bool, m: M| -> Element<'a, M> {
        mouse_area(container(t(label, 12.5, if on { p.accent } else { p.text }).wrapping(Wrapping::None)).padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 }).style(ui::boxed(if on { Pal::a(p.accent, 0.12) } else { Color::TRANSPARENT }, 4.0, Some((Pal::a(p.line, 0.7), 1.0))))).on_press(m).interaction(mouse::Interaction::Pointer).into()
    };
    let is_add = matches!(sel, Sel::Add(..));
    let pages_list: Vec<&'static PageDef> = d.pages.iter().collect();
    let action_label = |a: &str| -> String {
        if a.is_empty() {
            "Its own".into()
        } else if a == "none" {
            "Nothing".into()
        } else if let Some(id) = a.strip_prefix("act:") {
            format!("Open: {}", d.kit.and_then(|k| k.button(id)).map(|t| t.title).unwrap_or(id))
        } else if let Some(c2) = a.strip_prefix("run:") {
            format!("Run: {}", ui::fit(c2, 24))
        } else if let Some(pn) = a.strip_prefix("panel:") {
            format!("Show a panel: {}", d.kit.and_then(|k| k.panes.iter().find(|t| t.id == pn)).map(|t| t.title).unwrap_or(pn))
        } else {
            a.to_string()
        }
    };
    match st.tab {
        0 => {
            // Look
            c = c.push(head("SIZE AND PLACE"));
            let st1 = set.clone();
            let st2 = set.clone();
            c = c.push(row_("Width", row![seg(&p, &[("fit", "Fit"), ("fixed", "Fixed"), ("fill", "Fill")], sty.wmode, move |v| st1(F::WMode(v))), field(&p, if sty.w > 0 { sty.w.to_string() } else { String::new() }, "px", 64.0, move |v| st2(F::W(v)))].spacing(8.0).align_y(iced::Center).into()));
            let st3 = set.clone();
            c = c.push(row_("Height", field(&p, if sty.h > 0 { sty.h.to_string() } else { String::new() }, "px", 64.0, move |v| st3(F::H(v)))));
            let mut around = Row::new().spacing(6.0).align_y(iced::Center);
            for (i, arrow) in ["↑", "→", "↓", "←"].iter().enumerate() {
                let stx = set.clone();
                around = around.push(row![t(*arrow, 12.0, p.muted), field(&p, sty.around[i].to_string(), "", 44.0, move |v| stx(F::Around(i, v)))].spacing(3.0).align_y(iced::Center));
            }
            c = c.push(row_("Space around it", around.into()));
            let st4 = set.clone();
            let inside = sty.inside.unwrap_or(0) as f32;
            c = c.push(row_("Space inside", row![slider(0.0..=48.0, inside, move |v| st4(F::Inside(v))).step(1.0).width(Length::Fixed(170.0)), t(format!("{} px", inside as i64), 12.0, p.muted)].spacing(10.0).align_y(iced::Center).into()));
            c = c.push(head("BACKGROUND AND EDGE"));
            let mut fills = Row::new().spacing(6.0).align_y(iced::Center);
            for col in [Color::TRANSPARENT, p.panel2, p.accent, Color::from_rgb8(0x3a, 0x5b, 0xa8), Color::from_rgb8(0xe0, 0x6a, 0x8c)] {
                let hx = if col.a == 0.0 { String::new() } else { hexs(col) };
                let on = sty.fill == hx;
                let stx = set.clone();
                fills = fills.push(swatch(&p, if col.a == 0.0 { p.bg } else { col }, on, stx(F::Fill(hx))));
            }
            let st5 = set.clone();
            fills = fills.push(field(&p, if sty.fill.is_empty() { String::new() } else { sty.fill.to_uppercase() }, "", 84.0, move |v| st5(F::Fill(v))));
            c = c.push(row_("Fill", fills.into()));
            let st6 = set.clone();
            c = c.push(row_("See-through", row![slider(0.0..=100.0, sty.see as f32, move |v| st6(F::See(v))).step(1.0).width(Length::Fixed(170.0)), t(format!("{} %", sty.see), 12.0, p.muted)].spacing(10.0).align_y(iced::Center).into()));
            let st7 = set.clone();
            c = c.push(row_("Blur behind it", iced::widget::toggler(sty.blur).on_toggle(move |b| st7(F::Blur(b))).size(18.0).into()));
            let st8 = set.clone();
            let corners = sty.corners.unwrap_or(0) as f32;
            c = c.push(row_("Corners", row![slider(0.0..=48.0, corners, move |v| st8(F::Corners(v))).step(1.0).width(Length::Fixed(170.0)), t(if corners >= 24.0 { "Round".to_string() } else { format!("{} px", corners as i64) }, 12.0, p.muted)].spacing(10.0).align_y(iced::Center).into()));
            let st9 = set.clone();
            let st10 = set.clone();
            let st11 = set.clone();
            let edge_c = blocks::colour(&sty.edge_colour).unwrap_or(p.accent);
            c = c.push(row_(
                "Edge",
                row![
                    seg(&p, &[("none", "None"), ("line", "Line"), ("glow", "Glow")], if sty.edge.is_empty() { "none" } else { sty.edge }, move |v| st9(F::Edge(v))),
                    swatch(&p, edge_c, false, st10(F::EdgeColour(if sty.edge_colour.is_empty() { hexs(p.accent) } else if sty.edge_colour == hexs(p.accent) { "#5b8cff".into() } else { hexs(p.accent) }))),
                    field(&p, if sty.edge_w > 0 { sty.edge_w.to_string() } else { "1".into() }, "px", 40.0, move |v| st11(F::EdgeW(v)))
                ]
                .spacing(8.0)
                .align_y(iced::Center)
                .into(),
            ));
            let st12 = set.clone();
            c = c.push(row_("Shadow", seg(&p, &[("none", "None"), ("soft", "Soft"), ("deep", "Deep")], if sty.shadow.is_empty() { "none" } else { sty.shadow }, move |v| st12(F::Shadow(v)))));
            c = c.push(head("ICON AND WORDS"));
            let ic_label = if sty.icon.is_empty() { "None".to_string() } else { short(sty.icon) };
            let ic_c = blocks::colour(&sty.icon_colour).unwrap_or(p.accent);
            let st13 = set.clone();
            let st14 = set.clone();
            c = c.push(row_(
                "Icon",
                row![
                    mouse_area(container(row![container(Space::new()).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(ui::boxed(ic_c, 3.0, None)), t(format!("{ic_label} ▾"), 12.5, p.text)].spacing(6.0).align_y(iced::Center)).padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 }).style(ui::boxed(Color::TRANSPARENT, 5.0, Some((p.line, 1.0))))).on_press(s(SMsg::Expand(Expand::Icon))).interaction(mouse::Interaction::Pointer),
                    swatch(&p, ic_c, false, st13(F::IconColour(if sty.icon_colour.is_empty() || sty.icon_colour == hexs(p.accent) { hexs(p.text) } else { hexs(p.accent) }))),
                    field(&p, if sty.icon_size > 0 { sty.icon_size.to_string() } else { "16".into() }, "px", 40.0, move |v| st14(F::IconSize(v)))
                ]
                .spacing(8.0)
                .align_y(iced::Center)
                .into(),
            ));
            if st.expand == Expand::Icon {
                let mut w = Row::new().spacing(6.0);
                for name in sm::ICONS.iter().copied() {
                    let g = glyph_of(name);
                    let on = sty.icon == name;
                    let stx = set.clone();
                    let inner: Element<'a, M> = match g {
                        Some(g) => ui::glyph(g, if on { p.accent } else { p.text }, 16.0, 1.8),
                        None => t("none", 11.0, p.muted).into(),
                    };
                    w = w.push(mouse_area(container(inner).padding(5.0).style(ui::boxed(if on { Pal::a(p.accent, 0.15) } else { Color::TRANSPARENT }, 4.0, Some((p.line, 1.0))))).on_press(stx(F::Icon(name))).interaction(mouse::Interaction::Pointer));
                }
                c = c.push(w.wrap());
            }
            let st15 = set.clone();
            c = c.push(row_("Words", text_input("Words · {count}", &sty.words).on_input(move |v| st15(F::Words(v))).size(12.5).padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 8.0 }).width(Length::Fixed(220.0)).into()));
            let st16 = set.clone();
            let st17 = set.clone();
            let font = if sty.font.is_empty() { "Its own" } else { sty.font };
            c = c.push(row_(
                "Font",
                row![
                    mouse_area(container(t(format!("{font} ▾"), 12.5, p.text).wrapping(Wrapping::None)).padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 }).style(ui::boxed(Color::TRANSPARENT, 5.0, Some((p.line, 1.0))))).on_press(s(SMsg::Expand(Expand::Font))).interaction(mouse::Interaction::Pointer),
                    field(&p, if sty.font_size > 0 { sty.font_size.to_string() } else { "13".into() }, "", 40.0, move |v| st16(F::FontSize(v))),
                    seg(&p, &[("light", "Light"), ("medium", "Medium"), ("bold", "Bold")], sty.weight, move |v| st17(F::Weight(v)))
                ]
                .spacing(8.0)
                .align_y(iced::Center)
                .into(),
            ));
            if st.expand == Expand::Font {
                let mut w = Row::new().spacing(6.0);
                for f in sm::FONTS.iter().copied() {
                    let stx = set.clone();
                    w = w.push(picker_item(if f.is_empty() { "Its own".into() } else { f.to_string() }, sty.font == f, stx(F::Font(f))));
                }
                c = c.push(w.wrap());
            }
            let st18 = set.clone();
            c = c.push(row_("Show", seg(&p, &[("icon", "Icon"), ("words", "Words"), ("both", "Both")], sty.show, move |v| st18(F::Show(v)))));
            c = c.push(head("WHEN IT'S POINTED AT · PRESSED"));
            let st19 = set.clone();
            let st20 = set.clone();
            let mut hover = Row::new().spacing(8.0).align_y(iced::Center).push(seg(&p, &[("same", "Same"), ("lighter", "Lighter"), ("accent", "Accent"), ("own", "Own…")], sty.hover, move |v| st19(F::Hover(v))));
            if sty.hover == "own" {
                hover = hover.push(field(&p, sty.hover_fill.to_uppercase(), "", 80.0, move |v| st20(F::HoverFill(v))));
            }
            c = c.push(row_("Hover look", hover.into()));
            let st21 = set.clone();
            let st22 = set.clone();
            let mut pressed = Row::new().spacing(8.0).align_y(iced::Center).push(seg(&p, &[("same", "Same"), ("darker", "Darker"), ("own", "Own…")], sty.pressed, move |v| st21(F::Pressed(v))));
            if sty.pressed == "own" {
                pressed = pressed.push(field(&p, sty.pressed_fill.to_uppercase(), "", 80.0, move |v| st22(F::PressedFill(v))));
            }
            c = c.push(row_("Pressed look", pressed.into()));
            c = c.push(head("THE OTHER TABS"));
            c = c.push(t("Shows: what's in it (a count, a list, a picture, a clock…) · Does: press, right-click, middle-click, scroll, hover — open, run, show a panel, nothing · Place: where it is in the window, its size, above or under the rest · When: always, only on hover, only when there's something to show, only on some pages · Motion: how it appears, bounces when something new comes", 12.0, p.muted));
        }
        1 => {
            // Shows
            c = c.push(head("WHAT'S IN IT"));
            let st1 = set.clone();
            let opts: &[(&'static str, &'static str)] = &[("", "Its own"), ("words", "Words"), ("count", "A count"), ("clock", "A clock"), ("picture", "A picture"), ("list", "A list")];
            c = c.push(row_("Shows", seg(&p, opts, sty.shows, move |v| st1(F::Shows(v)))));
            if let Sel::Add(pg, u) = sel {
                if d.studio_page(pg).and_then(|x| x.add(u)).map(|a| a.block.starts_with("own:")).unwrap_or(false) {
                    let stx = sm_.clone();
                    let (pg2, u2) = (*pg, u.clone());
                    c = c.push(row_("Or make it", picker_item("A spacer (empty space)".into(), d.studio_page(pg).and_then(|x| x.add(u)).map(|a| a.block == "own:spacer").unwrap_or(false), stx(SMsg::Set(Sel::Add(pg2, u2), F::Shows("spacer"))))));
                }
            }
            if matches!(sty.shows, "count" | "words" | "list" | "") {
                let mut w = Row::new().spacing(6.0);
                if let Some(k) = d.kit {
                    for i in k.info {
                        let stx = set.clone();
                        w = w.push(picker_item(format!("{} ({})", i.title, ui::fit(&d.value(i.id), 14)), sty.source == i.id, stx(F::Source(i.id.to_string()))));
                    }
                }
                c = c.push(row_("What it counts", t("one of the app's own values ({count} in Words)", 12.0, p.muted).into()));
                c = c.push(w.wrap());
            }
            if sty.shows == "picture" {
                let stx = set.clone();
                c = c.push(row_("The picture", text_input("~/Pictures/a-picture.png", &sty.picture).on_input(move |v| stx(F::Picture(v))).size(12.5).padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 8.0 }).width(Length::Fixed(260.0)).into()));
            }
            let stw = set.clone();
            c = c.push(row_("Words", text_input("Words · {count} · {time}", &sty.words).on_input(move |v| stw(F::Words(v))).size(12.5).padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 8.0 }).width(Length::Fixed(260.0)).into()));
            c = c.push(t("Words can hold {count} (what it counts), {time}, and any of the app's own values by name.", 12.0, p.muted));
        }
        2 => {
            // Does
            c = c.push(head("WHAT IT DOES WHEN IT'S…"));
            for (i, (_, name)) in [("press", "Pressed"), ("right", "Right-clicked"), ("middle", "Middle-clicked"), ("scroll", "Scrolled on"), ("hover", "Pointed at")].iter().enumerate() {
                let cur = sty.does[i].clone();
                let mut r = Row::new().spacing(6.0).align_y(iced::Center);
                let s1 = set.clone();
                let s2 = set.clone();
                let s3 = set.clone();
                r = r.push(picker_item("Its own".into(), cur.is_empty(), s1(F::Does(i, String::new()))));
                r = r.push(picker_item("Nothing".into(), cur == "none", s2(F::Does(i, "none".into()))));
                r = r.push(picker_item("Open ▾".into(), cur.starts_with("act:"), s(SMsg::Expand(Expand::Open(i)))));
                r = r.push(picker_item("Run".into(), cur.starts_with("run:"), s3(F::Does(i, if cur.starts_with("run:") { cur.clone() } else { "run:".into() }))));
                r = r.push(picker_item("Show a panel ▾".into(), cur.starts_with("panel:"), s(SMsg::Expand(Expand::Panel(i)))));
                c = c.push(row_(*name, t(action_label(&cur), 12.0, p.muted).into()));
                c = c.push(r.wrap());
                if cur.starts_with("run:") {
                    let s4 = set.clone();
                    c = c.push(text_input("a command, like: notify-send Hello", cur.strip_prefix("run:").unwrap_or("")).on_input(move |v| s4(F::Does(i, format!("run:{v}")))).size(12.5).padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 8.0 }));
                }
                if st.expand == Expand::Open(i) {
                    let mut w = Row::new().spacing(6.0);
                    if let Some(k) = d.kit {
                        for b in k.buttons {
                            let sx = set.clone();
                            let v = format!("act:{}", b.id);
                            w = w.push(picker_item(b.title.to_string(), cur == v, sx(F::Does(i, v))));
                        }
                    }
                    c = c.push(w.wrap());
                }
                if st.expand == Expand::Panel(i) {
                    let mut w = Row::new().spacing(6.0);
                    if let Some(k) = d.kit {
                        for b in k.panes {
                            let sx = set.clone();
                            let v = format!("panel:{}", b.id);
                            w = w.push(picker_item(b.title.to_string(), cur == v, sx(F::Does(i, v))));
                        }
                    }
                    c = c.push(w.wrap());
                }
            }
        }
        3 => {
            // Place
            c = c.push(head("WHERE IT IS"));
            let (pr, r) = {
                let geo = d.geo.borrow();
                let pr = geo.page(sel.page()).unwrap_or_default();
                let r = match sel {
                    Sel::Part(pg, id) => geo.part(pg, id),
                    Sel::Add(pg, u) => geo.add(pg, u),
                }
                .map(|r| R::new(r.x - pr.x, r.y - pr.y, r.w, r.h))
                .unwrap_or_default();
                (pr, r)
            };
            let _ = pr;
            let mut snaps = Row::new().spacing(6.0);
            let opts: &[(&'static str, &'static str)] = if is_add { &[("free", "Free in the window")] } else { &[("left", "Left"), ("right", "Right"), ("top", "Top"), ("bottom", "Bottom"), ("free", "Floating anywhere")] };
            for (k2, n) in opts {
                let sx = set.clone();
                snaps = snaps.push(picker_item(n.to_string(), false, sx(F::Snap(*k2))));
            }
            c = c.push(row_("Put it", snaps.wrap().into()));
            if let Sel::Add(pg, _) = sel {
                if let Some(def) = d.pages.iter().find(|x| x.id == *pg) {
                    let mut bars = Row::new().spacing(6.0);
                    let geo = d.geo.borrow();
                    for b in def.blocks {
                        if geo.part(pg, b.id).map(|r| r.is_bar()).unwrap_or(false) {
                            bars = bars.push(picker_item(short(b.title), false, s(SMsg::MoveTo(sel.clone(), b.id.to_string()))));
                        }
                    }
                    c = c.push(row_("In a bar", bars.wrap().into()));
                }
            }
            let (s1, s2, s3, s4) = (set.clone(), set.clone(), set.clone(), set.clone());
            c = c.push(row_("Across, down", row![field(&p, (r.x.round() as i64).to_string(), "px", 60.0, move |v| s1(F::X(v))), field(&p, (r.y.round() as i64).to_string(), "px", 60.0, move |v| s2(F::Y(v)))].spacing(10.0).into()));
            c = c.push(row_("Width, height", row![field(&p, (r.w.round() as i64).to_string(), "px", 60.0, move |v| s3(F::PW(v))), field(&p, (r.h.round() as i64).to_string(), "px", 60.0, move |v| s4(F::PH(v)))].spacing(10.0).into()));
            let s5 = set.clone();
            c = c.push(row_("Layer", seg(&p, &[("", "As it is"), ("above", "Above the rest"), ("under", "Under the rest")], sty.layer, move |v| s5(F::Layer(v)))));
            c = c.push(t("Drag it anywhere in the window, or its edges to size it; an edge of the window is only a place it can snap to.", 12.0, p.muted));
        }
        4 => {
            // When
            c = c.push(head("WHEN IT SHOWS"));
            let s1 = set.clone();
            c = c.push(row_("Shows", seg(&p, &[("", "Always"), ("hover", "Only on hover"), ("something", "When there's something to show"), ("pages", "On some pages")], sty.when, move |v| s1(F::When(if v == "always" { "" } else { v })))));
            if sty.when == "pages" {
                let mut w = Row::new().spacing(6.0);
                for pg in pages_list {
                    let sx = set.clone();
                    w = w.push(picker_item(pg.title.to_string(), sty.pages.iter().any(|x| x == pg.id), sx(F::Page(pg.id.to_string()))));
                }
                c = c.push(t("Only while one of these is on the screen:", 12.0, p.muted));
                c = c.push(w.wrap());
            }
        }
        _ => {
            // Motion
            c = c.push(head("MOTION"));
            let s1 = set.clone();
            c = c.push(row_("How it appears", seg(&p, &[("none", "At once"), ("fade", "Fades in"), ("slide", "Slides in"), ("pop", "Pops in")], if sty.motion.is_empty() { "none" } else { sty.motion }, move |v| s1(F::Motion(v)))));
            let s2 = set.clone();
            c = c.push(row_("Bounces when something new comes", iced::widget::toggler(sty.bounce).on_toggle(move |b| s2(F::Bounce(b))).size(18.0).into()));
        }
    }
    // the buttons at the bottom
    let own_name = text_input("Its name", &st.own_name).on_input({
        let m = sm_.clone();
        move |v| m(SMsg::OwnName(v))
    })
    .size(12.5)
    .padding(Padding { top: 5.0, right: 6.0, bottom: 5.0, left: 8.0 })
    .width(Length::Fixed(130.0));
    let foot = row![
        own_name,
        tbtn(&p, "Save as my own block…".into(), false, false, Some(s(SMsg::SaveOwn(sel.clone())))),
        tbtn(&p, "Copy its look".into(), false, false, Some(s(SMsg::CopyLook(sel.clone())))),
        tbtn(&p, if is_add { "Apply to every bar item".into() } else { "Apply to every part".into() }, false, false, Some(s(SMsg::ApplyAll(sel.clone())))),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    let tabs = ["Look", "Shows", "Does", "Place", "When", "Motion"];
    let mut tr = Row::new().spacing(6.0);
    for (i, n) in tabs.iter().enumerate() {
        let on = i == st.tab;
        tr = tr.push(mouse_area(container(t(*n, 12.5, if on { p.ink } else { p.muted }).wrapping(Wrapping::None)).padding(Padding { top: 5.0, right: 11.0, bottom: 5.0, left: 11.0 }).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 6.0, Some((if on { p.accent } else { p.line }, 1.0))))).on_press(s(SMsg::Tab(i))).interaction(mouse::Interaction::Pointer));
    }
    let header = row![
        container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(p.accent, 3.0, None)),
        ts(title, 15.0, p.text).wrapping(Wrapping::None),
        t(where_, 13.0, p.muted).wrapping(Wrapping::None),
        Space::new().width(Length::Fill),
        tbtn(&p, "Reset".into(), false, false, Some(s(SMsg::Reset(sel.clone())))),
        mouse_area(t("✕", 14.0, p.muted)).on_press(s(SMsg::ClosePanel)).interaction(mouse::Interaction::Pointer),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    opaque(
        container(column![container(header).padding(Padding { top: 12.0, right: 14.0, bottom: 12.0, left: 16.0 }), ui::hline(Pal::a(p.line, 0.8)), container(column![tr, scrollable(c).height(Length::Fill), foot].spacing(12.0)).padding(Padding { top: 12.0, right: 16.0, bottom: 14.0, left: 16.0 }).height(Length::Fill)].height(Length::Fill))
            .height(Length::Fill)
            .style(ui::boxed(Pal::a(p.bg2, 0.985), p.r(12.0), Some((p.line, 1.0)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGES: &[PageDef] = &[PageDef { id: "window", title: "Hearth Files", blocks: &[blocks::bl("toolbar", "The toolbar", 0, 0, 12, 1), blocks::bl("sidebar", "The sidebar", 0, 1, 2, 11), blocks::bl("folder", "The folder", 2, 1, 10, 11)] }];
    const KIT: Kit = Kit { name: "Files", buttons: &[tile("sort-downloads", "Sort my Downloads", G::Down)], panes: &[tile("preview", "Preview", G::Info)], info: &[tile("count", "Items here", G::List)], pane_page: "window", bars: &[("window", "toolbar")], menus: &[] };

    fn editing() -> Describe {
        let dir = std::env::temp_dir().join(format!("hearth-studio-test-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        let mut d = Describe::new("files", "Hearth Files").with_pages(PAGES).with_kit(&KIT);
        let _ = d.studio(SMsg::Start(Some("window"), None));
        // where it was drawn (as the window draws it: 1200 × 800)
        let mut g = d.geo.borrow_mut();
        g.put(0, "window", "", "", "Hearth Files", Rectangle { x: 0.0, y: 0.0, width: 1200.0, height: 800.0 });
        g.put(1, "window", "", "toolbar", "The toolbar", Rectangle { x: 216.0, y: 0.0, width: 984.0, height: 48.0 });
        g.put(1, "window", "", "sidebar", "The sidebar", Rectangle { x: 0.0, y: 0.0, width: 216.0, height: 800.0 });
        g.put(1, "window", "", "folder", "The folder", Rectangle { x: 216.0, y: 48.0, width: 984.0, height: 752.0 });
        g.put(3, "window", "toolbar", "new", "New", Rectangle { x: 226.0, y: 8.0, width: 70.0, height: 32.0 });
        g.put(3, "window", "toolbar", "sort", "Sort", Rectangle { x: 306.0, y: 8.0, width: 70.0, height: 32.0 });
        g.put(3, "window", "toolbar", "view", "View", Rectangle { x: 386.0, y: 8.0, width: 70.0, height: 32.0 });
        drop(g);
        d
    }

    #[test]
    fn a_block_dropped_on_a_bar_lands_where_its_lit_line_is() {
        let mut d = editing();
        let _ = d.studio(SMsg::GrabTile("act:sort-downloads".into(), "Sort my Downloads".into()));
        let _ = d.studio(SMsg::Move(330.0, 10.0));
        let _ = d.studio(SMsg::Move(384.0, 24.0));
        match d.st.as_ref().and_then(|s| s.drag.as_ref()).and_then(|x| x.target.clone()) {
            Some(Target::Bar { part, after, say, line, .. }) => {
                assert_eq!((part, after.as_str()), ("toolbar", "sort"));
                assert_eq!(say, "Drop: toolbar, after sort");
                assert!((line.x + 1.5 - 381.0).abs() < 0.01, "between Sort and View: {line:?}");
            }
            t => panic!("not on the bar: {t:?}"),
        }
        let _ = d.studio(SMsg::Release);
        let a = &d.st.as_ref().unwrap().s.draft.page("window").unwrap().adds[0];
        assert_eq!((a.block.as_str(), a.into.as_str(), a.after.as_str()), ("act:sort-downloads", "toolbar", "sort"));
        assert_eq!(d.st.as_ref().unwrap().writes, 0, "nothing written until Keep this");
        assert!(!crate::describe::kept_file("files").exists());
    }

    #[test]
    fn a_pane_dropped_free_is_on_the_grid_and_a_side_dragged_sizes_it() {
        let mut d = editing();
        let _ = d.studio(SMsg::GrabTile("pane:preview".into(), "Preview".into()));
        let _ = d.studio(SMsg::Move(800.0, 400.0));
        let _ = d.studio(SMsg::Move(803.0, 413.0));
        let Some(Target::Free { at, .. }) = d.st.as_ref().unwrap().drag.as_ref().unwrap().target.clone() else { panic!("free") };
        assert_eq!(at.x % 8.0, 0.0, "snapped to the 8 px grid: {at:?}");
        let _ = d.studio(SMsg::Release);
        // the sidebar's edge dragged: its width (the app lays out the rest around it), lined up with the toolbar's Sort
        let _ = d.studio(SMsg::GrabSide(Sel::Part("window", "sidebar"), Side::Right));
        let _ = d.studio(SMsg::Move(303.0, 400.0));
        let st = d.st.as_ref().unwrap();
        let s = &st.s.draft.page("window").unwrap().part("sidebar").unwrap().style;
        assert_eq!((s.wmode, s.w), ("fixed", 306), "lined up with Sort's left edge (306), not the grid's 304");
        let note = &st.drag.as_ref().unwrap().guide.as_ref().unwrap().note;
        assert_eq!(note, "Sidebar: 306 px · lined up with Sort");
        let _ = d.studio(SMsg::Release);
        assert_eq!(d.st.as_ref().unwrap().s.undo.len(), 2, "the pane, then the drag: one step each");
        let _ = d.studio(SMsg::Undo);
        assert!(d.st.as_ref().unwrap().s.draft.page("window").unwrap().part("sidebar").map(|p| p.style.w == 0).unwrap_or(true));
        let _ = d.studio(SMsg::Redo);
        assert_eq!(d.st.as_ref().unwrap().s.draft.page("window").unwrap().part("sidebar").unwrap().style.w, 306);
    }

    #[test]
    fn a_part_moves_anywhere_and_cancel_keeps_nothing() {
        let mut d = editing();
        let _ = d.studio(SMsg::Press(Sel::Part("window", "sidebar"), 100.0, 400.0));
        let _ = d.studio(SMsg::Move(500.0, 430.0));
        let _ = d.studio(SMsg::Release);
        let pd = d.st.as_ref().unwrap().s.draft.page("window").unwrap().clone();
        assert!(pd.moved(), "laid out by its parts' places now");
        let side = pd.part("sidebar").unwrap().at.unwrap().resolve(1200.0, 800.0);
        assert_eq!((side.x, side.w), (400.0, 216.0), "in the middle of the window, not on a side");
        let folder = pd.part("folder").unwrap().at.unwrap().resolve(1200.0, 800.0);
        assert_eq!((folder.x, folder.w), (216.0, 984.0), "the others stayed where they were drawn");
        let _ = d.studio(SMsg::Set(Sel::Part("window", "folder"), F::Corners(14.0)));
        let _ = d.studio(SMsg::Set(Sel::Part("window", "folder"), F::Words("Downloads · {count}".into())));
        let _ = d.studio(SMsg::Set(Sel::Part("window", "folder"), F::Words("Downloads · {count} here".into())));
        assert_eq!(d.st.as_ref().unwrap().s.undo.len(), 3, "typing is one step");
        let _ = d.studio(SMsg::Cancel);
        assert!(d.st.is_none() && d.arr.is_none());
        assert!(d.studio_page("window").is_none(), "nothing kept");
        assert!(!crate::describe::kept_file("files").exists());
    }

    #[test]
    fn keep_this_asks_hearth_layout_once_with_the_change() {
        let mut d = editing();
        let _ = d.studio(SMsg::Set(Sel::Part("window", "toolbar"), F::Shadow("soft")));
        let grid = d.grid_pages();
        let c = d.st.as_ref().unwrap().s.keep_change(&grid).unwrap();
        assert_eq!(c["layout"]["studio"]["pages"]["window"]["parts"]["toolbar"]["style"]["shadow"], "soft");
        let cmd = d.keep_cmd_studio(&c);
        assert!(cmd.iter().any(|x| x == "app-keep") && cmd.last().unwrap().contains("\"shadow\":\"soft\""));
        let _ = d.studio(SMsg::Keep);
        assert_eq!(d.st.as_ref().unwrap().writes, 1);
        assert!(d.st.as_ref().unwrap().busy);
    }

    #[test]
    fn every_setting_applies_to_the_look() {
        let mut s = Style::default();
        for f in [F::WMode("fixed"), F::W("158".into()), F::H("30".into()), F::Around(0, "7".into()), F::Inside(12.0), F::Fill("#3B2A1D".into()), F::See(15.0), F::Blur(true), F::Corners(14.0), F::Edge("line"), F::EdgeColour("#ff7a2f".into()), F::EdgeW("1".into()), F::Shadow("soft"), F::Icon("down"), F::IconColour("#ff7a2f".into()), F::IconSize("16".into()), F::Words("Downloads · {count}".into()), F::Font("Inter"), F::FontSize("14".into()), F::Weight("medium"), F::Show("both"), F::Hover("lighter"), F::Pressed("darker"), F::Shows("count"), F::Source("count".into()), F::Does(0, "act:sort-downloads".into()), F::Does(1, "run:notify-send hi".into()), F::Layer("above"), F::Page("window".into()), F::Motion("pop"), F::Bounce(true)] {
            apply(&mut s, &f);
        }
        assert_eq!((s.wmode, s.w, s.h, s.around[0], s.inside, s.fill.as_str(), s.see, s.blur, s.corners), ("fixed", 158, 30, 7, Some(12), "#3b2a1d", 15, true, Some(14)));
        assert_eq!((s.edge, s.edge_colour.as_str(), s.edge_w, s.shadow, s.icon, s.icon_size, s.font, s.font_size, s.weight, s.show), ("line", "#ff7a2f", 1, "soft", "down", 16, "Inter", 14, "medium", "both"));
        assert_eq!((s.hover, s.pressed, s.shows, s.source.as_str(), s.layer, s.when, s.motion, s.bounce), ("lighter", "darker", "count", "count", "above", "pages", "pop", true));
        assert_eq!(s.does[1], "run:notify-send hi");
        assert!(Style::read(&s.to_json()).is_ok(), "everything set is kept the way hearth-layout checks it");
        let p = crate::theme::load();
        let pt = paint(&p, &s, Base::Part);
        assert!(pt.bg.map(|c| (c.a - 0.85).abs() < 0.2).unwrap_or(false) && pt.radius == 14.0 && pt.border.is_some() && pt.shadow.is_some() && pt.hover.is_some() && pt.pressed.is_some());
        apply(&mut s, &F::Fill("orange".into()));
        assert_eq!(s.fill, "#3b2a1d", "not a colour: left as it was");
    }

    #[test]
    fn a_part_menu_can_be_reordered() {
        let mut d = editing();
        let _ = d.studio(SMsg::MenuMove("part", 0, 1));
        let order = order_of(&d.st.as_ref().unwrap().s.draft, &PART_MENU);
        assert_eq!(order[0], "edit-layout");
        assert_eq!(order[1], "customize");
    }
}
