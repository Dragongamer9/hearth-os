//! Moving through Settings with a controller or the arrow keys (the approved new Settings, settings-B1/B2, and
//! Mantel's SetBase.qml / SettingsNav.qml): every row and control the pages draw with widgets.rs is a place the focus
//! can be. Each frame, while the pages are drawn, the widgets say what they are (a switch, ‹ value › choices, a slider,
//! a link, a button, a text field) and what A, left and right do on them; the frame draws the focused one lifted with
//! the accent outline, and main.rs moves the focus (Up / Down), changes the focused setting (Left / Right), selects it
//! (A / Enter) and goes back (B / Esc). Nothing here runs between frames: no timers, no polling.
//!
//! How a control and its row find each other: a control (switch, choices, slider, button) is drawn before the row it
//! sits in (it's an argument of srow), so it waits as "pending" and the row takes it. A control that no row takes (a
//! button on its own in a card) becomes a place of its own when the next card or heading is drawn.
//!
//! Also here: the controller look's timing (when a controller was last used, which buttons' clicks to hold back for a
//! moment) and the Gate, the window's outermost widget, which holds back the click a controller's A sends as a mouse
//! click (hearth-pad on Plasma, Steam's desktop layout) and says when the mouse is really used again.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::operation::{Operation, Outcome, Scrollable};
use iced::advanced::widget::{tree, Id, Tree};
use iced::advanced::{overlay, Clipboard, Shell, Widget};
use iced::{mouse, Element, Event, Length, Rectangle, Size, Vector};
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

/// What A, left or right sends: a page's own message (its module's Msg, or the frame's), lifted to the app's Msg by
/// main.rs (`lift`).
pub type Act = Box<dyn Any + Send>;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Kind {
    /// A row with nothing to change (a fact, "Coming next").
    #[default]
    Plain,
    /// A switch, on or off.
    Toggle(bool),
    /// ‹ value › choices.
    Choice,
    Slider,
    /// Opens something (another page, an app): ›.
    Open,
    /// A button.
    Press,
    /// A text field: A puts the typing there.
    Field,
}

pub struct Item {
    pub title: String,
    pub sub: String,
    pub kind: Kind,
    pub a: Vec<Act>,
    pub left: Vec<Act>,
    pub right: Vec<Act>,
    pub field: Option<Id>,
    /// A tile (Quick, Home's tiles): left and right without a use of their own move to the next one.
    pub grid: bool,
    /// Its widget's id, to find it on the screen (scrolling it into view).
    pub key: Id,
}

struct Pending {
    kind: Kind,
    a: Vec<Act>,
    left: Vec<Act>,
    right: Vec<Act>,
    field: Option<Id>,
    slot: usize,
    label: String,
    value: String,
}

struct Nav {
    items: Vec<Item>,
    pending: Vec<Pending>,
    /// A control's slot → the place it is when no row took it (its own outline is drawn then).
    slots: Vec<Option<usize>>,
    focus: usize,
    /// The outline shows (a key or the controller moved; the mouse hides it in the keyboard-and-mouse look).
    shown: bool,
    /// The focus is in the page (not in the categories / the tab's pages on the left).
    in_page: bool,
    /// The first place of the dialog in front (0: the page).
    layer: usize,
    /// After the next frame, put the focus on the place with this title (a subpage's row after Back, `--focus`).
    want: String,
}

static NAV: Mutex<Nav> = Mutex::new(Nav { items: Vec::new(), pending: Vec::new(), slots: Vec::new(), focus: 0, shown: false, in_page: false, layer: 0, want: String::new() });

fn nav() -> MutexGuard<'static, Nav> {
    NAV.lock().unwrap_or_else(|e| e.into_inner())
}

/// The page's scrollable (to keep the focused row in view).
pub const PAGE: &str = "hs-page";
/// The search field (A's left list; the controller look's search box).
pub const SEARCH: &str = "hs-search";

// ------------------------------------------------------------------ while a frame is drawn

/// A frame starts: what the last one registered goes.
pub fn begin() {
    let mut n = nav();
    n.items.clear();
    n.pending.clear();
    n.slots.clear();
    n.layer = 0;
}

/// A control is drawn (its row will take it). Returns its slot, for its own outline when no row does.
pub fn control(kind: Kind, a: Vec<Act>, left: Vec<Act>, right: Vec<Act>, label: String, value: String) -> usize {
    let mut n = nav();
    let slot = n.slots.len();
    n.slots.push(None);
    let field = if kind == Kind::Field { Some(field_id(slot)) } else { None };
    n.pending.push(Pending { kind, a, left, right, field, slot, label, value });
    slot
}

/// The widget id of a control's own box (its outline when no row took it).
pub fn slot_id(slot: usize) -> Id {
    Id::from(format!("hs-slot-{slot}"))
}
/// A text field's id (to put the typing there).
pub fn field_id(slot: usize) -> Id {
    Id::from(format!("hs-field-{slot}"))
}

/// A row as it's drawn: its widget id, whether it's the focused one, and the value its slider shows (the controller
/// look shows it next to the slider, so the row doesn't say it twice).
pub struct Row {
    pub key: Id,
    pub lit: bool,
    pub slider_value: Option<String>,
}

fn better(k: Kind, than: Kind) -> bool {
    let rank = |k: Kind| match k {
        Kind::Plain => 0,
        Kind::Open => 1,
        Kind::Press => 2,
        Kind::Field => 3,
        Kind::Toggle(_) => 4,
        Kind::Choice => 5,
        Kind::Slider => 6,
    };
    rank(k) > rank(than)
}

/// A row is drawn: it takes the controls drawn for it. Returns its place and whether it's the focused one (outlined).
pub fn item(title: &str, sub: &str, grid: bool) -> Row {
    let mut n = nav();
    let i = n.items.len();
    let key = Id::from(format!("hs-item-{i}"));
    let mut it = Item { title: title.to_string(), sub: sub.to_string(), kind: Kind::Plain, a: Vec::new(), left: Vec::new(), right: Vec::new(), field: None, grid, key: key.clone() };
    let mut slider_value = None;
    for p in std::mem::take(&mut n.pending) {
        if p.kind == Kind::Slider && slider_value.is_none() {
            slider_value = Some(p.value.clone());
        }
        if better(p.kind, it.kind) {
            it.kind = p.kind;
        }
        if it.a.is_empty() {
            it.a = p.a;
        }
        if it.left.is_empty() {
            it.left = p.left;
        }
        if it.right.is_empty() {
            it.right = p.right;
        }
        if it.field.is_none() {
            it.field = p.field;
        }
    }
    n.items.push(it);
    let lit = n.shown && n.in_page && n.focus == i && i >= n.layer;
    Row { key, lit, slider_value }
}

/// Controls no row took become places of their own (a card or a heading is drawn, or the frame ends).
pub fn flush() {
    let mut n = nav();
    for p in std::mem::take(&mut n.pending) {
        let i = n.items.len();
        if let Some(s) = n.slots.get_mut(p.slot) {
            *s = Some(i);
        }
        let title = if p.label.is_empty() { p.value.clone() } else { p.label };
        n.items.push(Item { title, sub: String::new(), kind: p.kind, a: p.a, left: p.left, right: p.right, field: p.field, grid: true, key: slot_id(p.slot) });
    }
}

/// Is this control, on its own (no row took it), the focused place? Asked while drawing, after the frame was built.
pub fn slot_lit(slot: usize) -> bool {
    let n = nav();
    matches!(n.slots.get(slot), Some(Some(i)) if *i == n.focus && n.shown && n.in_page && *i >= n.layer)
}

pub fn len() -> usize {
    nav().items.len()
}

/// A dialog is drawn over the page from this place on: the focus stays in it.
pub fn set_layer(from: usize) {
    let mut n = nav();
    n.layer = from;
}

/// The frame is done: the focus is kept on a place that's there (and one asked for by its title).
pub fn end() {
    flush();
    let mut n = nav();
    // (A page's rows can come a moment later, once what they show was read: the wish waits for them, until the
    // focus is moved by hand.)
    if !n.want.is_empty() {
        if let Some(i) = n.items.iter().position(|x| x.title == n.want) {
            n.focus = i;
            n.in_page = true;
            n.want.clear();
        }
    }
    let (lo, len) = (n.layer, n.items.len());
    if len > lo && (n.focus < lo || n.focus >= len) {
        n.focus = if n.focus >= len { len - 1 } else { lo };
    }
}

// ------------------------------------------------------------------ from main.rs (between frames)

pub fn shown() -> bool {
    nav().shown
}
pub fn set_shown(on: bool) {
    nav().shown = on;
}
pub fn in_page() -> bool {
    nav().in_page
}
pub fn set_in_page(on: bool) {
    let mut n = nav();
    n.in_page = on;
    if !on {
        n.want.clear();
    }
}
pub fn focus() -> usize {
    nav().focus
}
/// The focus to the first place of the page (a page opened).
pub fn reset() {
    let mut n = nav();
    n.focus = n.layer;
}
/// After the next frame, the focus goes to the place called `title` (if there is one).
pub fn want(title: &str) {
    nav().want = title.to_string();
}

/// Up / Down: the focus to the place before or after (staying in the dialog in front). Whether it moved.
pub fn step(d: i32) -> bool {
    let mut n = nav();
    let (lo, len) = (n.layer, n.items.len());
    if len <= lo {
        return false;
    }
    n.want.clear();
    let to = (n.focus as i64 + d as i64).clamp(lo as i64, len as i64 - 1) as usize;
    let moved = to != n.focus;
    n.focus = to;
    moved
}

pub fn at_first() -> bool {
    let n = nav();
    n.focus <= n.layer
}

/// What the focused place is, for the help panel and X.
#[derive(Debug, Clone, Default)]
pub struct Focused {
    pub title: String,
    pub sub: String,
    pub kind: Kind,
    pub grid: bool,
    pub has_a: bool,
    pub has_lr: bool,
}

pub fn focused() -> Option<Focused> {
    let n = nav();
    n.items.get(n.focus).map(|x| Focused { title: x.title.clone(), sub: x.sub.clone(), kind: x.kind, grid: x.grid, has_a: !x.a.is_empty(), has_lr: !x.left.is_empty() || !x.right.is_empty() })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Which {
    A,
    Left,
    Right,
}

/// What the focused place does for A, left or right (taken: the next frame registers them again).
pub fn take(w: Which) -> (Vec<Act>, Option<Id>) {
    let mut n = nav();
    let f = n.focus;
    match n.items.get_mut(f) {
        Some(x) => (
            match w {
                Which::A => std::mem::take(&mut x.a),
                Which::Left => std::mem::take(&mut x.left),
                Which::Right => std::mem::take(&mut x.right),
            },
            if w == Which::A { x.field.clone() } else { None },
        ),
        None => (Vec::new(), None),
    }
}

pub fn focused_key() -> Option<Id> {
    let n = nav();
    n.items.get(n.focus).map(|x| x.key.clone())
}

// ------------------------------------------------------------------ keeping the focused row in view

/// Finds the focused row and the page's scrollable; says how far to scroll so the row shows (None: it shows).
struct Reveal {
    target: Id,
    page: Id,
    item: Option<Rectangle>,
    scroll: Option<(Rectangle, Rectangle, Vector)>,
}

impl Operation<Option<f32>> for Reveal {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<f32>>)) {
        operate(self)
    }
    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.item = Some(bounds);
        }
    }
    fn scrollable(&mut self, id: Option<&Id>, bounds: Rectangle, content: Rectangle, translation: Vector, _state: &mut dyn Scrollable) {
        if id == Some(&self.page) {
            self.scroll = Some((bounds, content, translation));
        }
    }
    fn finish(&self) -> Outcome<Option<f32>> {
        Outcome::Some(match (self.item, self.scroll) {
            (Some(it), Some((b, c, t))) => to_show(it, b, c, t.y),
            _ => None,
        })
    }
}

/// The scroll offset that shows `it` (16 px to spare), or None when it shows already.
pub fn to_show(it: Rectangle, view: Rectangle, content: Rectangle, offset: f32) -> Option<f32> {
    let m = 16.0;
    let top = it.y - content.y;
    let bottom = top + it.height;
    let max = (content.height - view.height).max(0.0);
    if top - m < offset {
        Some((top - m).clamp(0.0, max))
    } else if bottom + m > offset + view.height {
        Some((bottom + m - view.height).clamp(0.0, max))
    } else {
        None
    }
}

/// Scrolls the page so the focused row shows.
pub fn reveal<M: Send + 'static>() -> iced::Task<M> {
    let Some(target) = focused_key() else { return iced::Task::none() };
    iced::advanced::widget::operate(Reveal { target, page: Id::new(PAGE), item: None, scroll: None }).then(|o: Option<f32>| match o {
        Some(y) => iced::widget::operation::scroll_to(Id::new(PAGE), iced::widget::operation::AbsoluteOffset { x: None, y: Some(y) }),
        None => iced::Task::none(),
    })
}

// ------------------------------------------------------------------ the controller's timing

fn start() -> Instant {
    static T: OnceLock<Instant> = OnceLock::new();
    *T.get_or_init(Instant::now)
}
fn now_ms() -> u64 {
    start().elapsed().as_millis() as u64 + 1
}

static PAD_AT: AtomicU64 = AtomicU64::new(0);
static HOLD_CLICKS: AtomicU64 = AtomicU64::new(0);
/// The controller look is showing (the Gate then watches for the mouse).
static PAD_LOOK: AtomicBool = AtomicBool::new(false);
static POINTER_SAID: AtomicBool = AtomicBool::new(false);
/// The look was picked with --layout (it doesn't switch by itself then).
static FIXED: AtomicBool = AtomicBool::new(false);

/// A controller button was pressed (read from the controller itself): its click (hearth-pad's A as a mouse click,
/// Steam's) is held back for a moment, and the keys it types too (main.rs).
pub fn pad_used() {
    let t = now_ms();
    PAD_AT.store(t, Ordering::Relaxed);
    HOLD_CLICKS.store(t + 300, Ordering::Relaxed);
}
/// A controller button was pressed in the last `ms` milliseconds.
pub fn pad_recent(ms: u64) -> bool {
    let at = PAD_AT.load(Ordering::Relaxed);
    at > 0 && now_ms().saturating_sub(at) < ms
}
/// When the look last changed (ms on this clock): it changes back by itself at most once every few seconds.
static LOOK_AT: AtomicU64 = AtomicU64::new(0);
/// The first look was said (set_layout's line, once at start).
pub fn look_said() -> bool {
    static SAID: AtomicBool = AtomicBool::new(false);
    SAID.swap(true, Ordering::Relaxed)
}
pub fn look_age() -> u64 {
    now_ms().saturating_sub(LOOK_AT.load(Ordering::Relaxed))
}
pub fn set_pad_look(on: bool) {
    if PAD_LOOK.load(Ordering::Relaxed) != on {
        LOOK_AT.store(now_ms(), Ordering::Relaxed);
    }
    PAD_LOOK.store(on, Ordering::Relaxed);
    if on {
        POINTER_SAID.store(false, Ordering::Relaxed);
    }
}
pub fn pad_look() -> bool {
    PAD_LOOK.load(Ordering::Relaxed)
}
pub fn set_fixed(on: bool) {
    FIXED.store(on, Ordering::Relaxed);
}

/// The window's outermost widget: holds back mouse clicks for a moment after a controller press (they're the
/// controller's own A as a click), and says (once) when the mouse is used for real while the controller look shows.
/// The pointer moving counts only when no controller button was pressed in the last 1.5 s, as in Mantel
/// (SettingsNav.pointerUsed): the controller's pointer mode moves it too.
pub struct Gate<'a, M> {
    content: Element<'a, M>,
    on_pointer: M,
}

impl<'a, M: Clone + 'a> Gate<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>, on_pointer: M) -> Self {
        Gate { content: content.into(), on_pointer }
    }
}

impl<'a, M: Clone + 'a> Widget<M, iced::Theme, iced::Renderer> for Gate<'a, M> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }
    fn draw(&self, tree: &Tree, renderer: &mut iced::Renderer, theme: &iced::Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport)
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &iced::Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        let click = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_)) | Event::Touch(_));
        if click && now_ms() < HOLD_CLICKS.load(Ordering::Relaxed) {
            shell.capture_event();
            return;
        }
        let pointer = click || matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::WheelScrolled { .. }));
        // Back to the keyboard-and-mouse look only for a real mouse / touchpad (pad.rs desk_in_use): the controller's
        // own pointer and A click arrive as pointer events too, and must not flip the look (the owner, Nov 19).
        if pointer && PAD_LOOK.load(Ordering::Relaxed) && !FIXED.load(Ordering::Relaxed) && !pad_recent(1500) && look_age() > 4000 && crate::pad::desk_in_use() && !POINTER_SAID.swap(true, Ordering::Relaxed) {
            shell.publish(self.on_pointer.clone());
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport)
    }
    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &iced::Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &iced::Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, M: Clone + 'a> From<Gate<'a, M>> for Element<'a, M> {
    fn from(g: Gate<'a, M>) -> Element<'a, M> {
        Element::new(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test touches NAV (it's shared by the whole program).
    #[test]
    fn rows_take_their_controls_and_the_focus_moves() {
        begin();
        set_shown(true);
        set_in_page(true);
        nav().focus = 0;
        control(Kind::Toggle(true), vec![Box::new(1u8)], vec![], vec![], String::new(), "On".into());
        assert!(item("Night light", "Warmer colours", false).lit);
        control(Kind::Slider, vec![], vec![Box::new(2u8)], vec![Box::new(3u8)], String::new(), "70%".into());
        control(Kind::Press, vec![Box::new(4u8)], vec![], vec![], "Mute".into(), String::new());
        let r = item("Volume", "45%", false);
        assert!(!r.lit);
        assert_eq!(r.slider_value.as_deref(), Some("70%"));
        // A button no row took is a place of its own.
        let s = control(Kind::Press, vec![Box::new(5u8)], vec![], vec![], "Rename".into(), String::new());
        end();
        assert_eq!(len(), 3);
        assert!(!slot_lit(s));
        assert!(step(1));
        let f = focused().unwrap();
        assert_eq!(f.title, "Volume");
        assert_eq!(f.kind, Kind::Slider);
        assert!(f.has_a && f.has_lr);
        let (a, _) = take(Which::A);
        assert_eq!(a.len(), 1);
        assert_eq!(*a.into_iter().next().unwrap().downcast::<u8>().unwrap(), 4);
        assert!(step(1));
        assert!(slot_lit(s));
        assert!(!step(1));
        // A dialog keeps the focus in it.
        begin();
        item("Page row", "", false);
        let from = len();
        set_layer(from);
        item("Dialog row", "", false);
        nav().focus = 0;
        end();
        assert_eq!(focused().unwrap().title, "Dialog row");
        want("Page row");
        begin();
        item("Page row", "", false);
        end();
        assert_eq!(focus(), 0);
    }

    #[test]
    fn scrolling_shows_the_row() {
        let view = Rectangle { x: 0.0, y: 100.0, width: 500.0, height: 400.0 };
        let content = Rectangle { x: 0.0, y: 100.0, width: 500.0, height: 1200.0 };
        let row = |y: f32| Rectangle { x: 0.0, y: 100.0 + y, width: 500.0, height: 60.0 };
        assert_eq!(to_show(row(50.0), view, content, 0.0), None);
        assert_eq!(to_show(row(600.0), view, content, 0.0), Some(276.0));
        assert_eq!(to_show(row(10.0), view, content, 300.0), Some(0.0));
    }
}
