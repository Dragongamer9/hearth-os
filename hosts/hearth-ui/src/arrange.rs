//! A page made of blocks, drawn (blocks.rs has the rules; describe.rs the state): [`page`] is what every Hearth app
//! puts where its page was. Nothing customised and not being edited: the app's own view, untouched. Laid out: each
//! block in its place on the 12 × 12 grid, in its look.
//!
//! Edit layout (the owner, Nov 24: "editing an app's layout must feel natural, never awkward"): right-click any block
//! for Edit layout / Hide this / Describe it…, or pick Edit layout in the app's menu (Describe it's box has it too).
//! The live app stays on the screen: each block gets a soft outline and its name under the pointer; drag anywhere on a
//! block to move it, its right or bottom edge or its corner to size it, snapped to the grid and drawn as it goes. The
//! bar over the page has the hidden blocks to add back, the picked block's Look, Undo, Back to the original and Done.
//! Everything is saved as you go (hearth-layout app-keep: settings_file, atomic, .bak), so there's no Keep step.
//! Keys: Tab / Shift+Tab pick a block, arrows move it, Shift+arrows size it, Page Up / Down shrink or grow it, Delete
//! hides it, Ctrl+Z undoes, Esc or Done finishes. The controller is the same mode: hearth-pad's D-pad, A, B and LB / RB
//! arrive as those keys, and an app that reads the controller itself hands its presses to [`pad`].
//!
//! Light at rest: nothing runs and nothing is measured per frame; a save waits 450 ms for more changes (one thread
//! that sleeps), and the right-click menu, the outline and the names are drawn only while they're asked for.

use crate::blocks::{self, Grip, LookPart, Placed, COLS, ROWS};
use crate::describe::{outline, Arrange, DMsg, Describe};
use crate::run;
use crate::theme::Pal;
use crate::ui::{self, t, ts};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::operation::Operation;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{overlay, Clipboard, Shell, Widget};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, opaque, row, stack, Column, Row, Space};
use iced::{keyboard, mouse, Color, Element, Event, Length, Padding, Rectangle, Renderer, Size, Task, Theme, Vector};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// One block put in its place on the grid (each block is a layer the size of the page; the spaces round it take its
/// place's share, so nothing needs the window's size and nothing is measured per frame).
fn place<'a, M: 'a>(p: &Placed, el: Element<'a, M>) -> Element<'a, M> {
    let mut r = Row::new().width(Length::Fill).height(Length::FillPortion(p.h as u16));
    if p.x > 0 {
        r = r.push(Space::new().width(Length::FillPortion(p.x as u16)).height(Length::Fill));
    }
    r = r.push(container(el).width(Length::FillPortion(p.w as u16)).height(Length::Fill).clip(true));
    let right = COLS - p.x - p.w;
    if right > 0 {
        r = r.push(Space::new().width(Length::FillPortion(right as u16)).height(Length::Fill));
    }
    let mut c = Column::new().width(Length::Fill).height(Length::Fill);
    if p.y > 0 {
        c = c.push(Space::new().width(Length::Fill).height(Length::FillPortion(p.y as u16)));
    }
    c = c.push(r);
    let below = ROWS - p.y - p.h;
    if below > 0 {
        c = c.push(Space::new().width(Length::Fill).height(Length::FillPortion(below as u16)));
    }
    c.into()
}

/// A block's own view in its look: its font and text size while it's made, and its card when it has one.
fn dressed<'a, M: 'a>(pal: &Pal, pl: &Placed, block: &dyn Fn(&'static str) -> Element<'a, M>) -> Element<'a, M> {
    let el = ui::with_face(pl.look.face(), pl.look.scale(), || block(pl.id));
    let el: Element<'a, M> = if pl.scroll { iced::widget::scrollable(el).width(Length::Fill).height(Length::Fill).into() } else { el };
    match blocks::card_style(pal, &pl.look) {
        None => container(el).width(Length::Fill).height(Length::Fill).into(),
        Some((bg, r, line)) => container(el).width(Length::Fill).height(Length::Fill).padding((r * 0.35).max(2.0)).clip(true).style(ui::boxed(bg, r, line)).into(),
    }
}

struct GridLines(Color);
impl<M> canvas::Program<M> for GridLines {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let st = || Stroke::default().with_color(self.0).with_width(1.0);
        for i in 1..COLS {
            let x = (b.width * i as f32 / COLS as f32).round() + 0.5;
            f.stroke(&Path::line(iced::Point::new(x, 0.0), iced::Point::new(x, b.height)), st());
        }
        for j in 1..ROWS {
            let y = (b.height * j as f32 / ROWS as f32).round() + 0.5;
            f.stroke(&Path::line(iced::Point::new(0.0, y), iced::Point::new(b.width, y)), st());
        }
        vec![f.into_geometry()]
    }
}

// ------------------------------------------------------------------ Edit layout: its messages and state

/// What Edit layout is told (inside [`DMsg::Edit`]).
#[derive(Debug, Clone)]
pub enum EMsg {
    /// Edit layout from a menu: the page in front (the innermost page on the screen)
    Start,
    /// Edit layout on this page, with this block picked
    StartAt(&'static str, Option<&'static str>),
    /// the right-click menu on a block (page, block; None: closed)
    Menu(Option<(&'static str, &'static str)>),
    /// Hide this, from the right-click menu (saved, with Undo)
    HideNow(&'static str, &'static str),
    /// Describe it…, from the right-click menu or the bar
    Describe,
    Hover(&'static str),
    Unhover(&'static str),
    Grab(&'static str, Grip),
    Drop,
    Hide(&'static str),
    /// a hidden block added back from the tray
    Tray(&'static str),
    Look(LookPart),
    Undo,
    Reset,
    Done,
    Key(Key),
    /// save now if nothing changed since this change (the 450 ms wait)
    Flush(u64),
    Saved(run::Output),
    NoteGone(u64),
}

/// A key (or the controller's button) in Edit layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Wider,
    Narrower,
    Taller,
    Shorter,
    Grow,
    Shrink,
    Next,
    Prev,
    Hide,
    Undo,
    Done,
}

/// Edit layout's own state, kept in [`Describe::ed`].
#[derive(Default)]
pub struct EditState {
    /// the right-click menu that's open (page, block)
    pub menu: Option<(&'static str, &'static str)>,
    /// the block under the pointer while editing
    pub hover: Option<&'static str>,
    /// "Saved", by Undo
    pub toast: String,
    /// after Hide this (not editing): the page and what was said, with Undo
    pub note: Option<(&'static str, String)>,
    /// the other pages on the screen when editing began (Edit the window, …)
    pub chips: Vec<&'static str>,
    gen: u64,
    dirty: bool,
    note_gen: u64,
    /// the pages drawn and when (which page is in front, for Edit layout from a menu)
    drawn: RefCell<Vec<(&'static str, Instant)>>,
}

impl EditState {
    fn mark(&self, page: &'static str) {
        if let Ok(mut v) = self.drawn.try_borrow_mut() {
            let now = Instant::now();
            match v.iter_mut().find(|x| x.0 == page) {
                Some(e) => e.1 = now,
                None => v.push((page, now)),
            }
        }
    }

    /// The pages on the screen (drawn in the last view), innermost first (a page inside another is drawn first).
    pub fn shown(&self) -> Vec<&'static str> {
        let Ok(v) = self.drawn.try_borrow() else { return Vec::new() };
        let Some(last) = v.iter().map(|x| x.1).max() else { return Vec::new() };
        let mut r: Vec<(&'static str, Instant)> = v.iter().copied().filter(|x| last.duration_since(x.1) < Duration::from_millis(250)).collect();
        r.sort_by_key(|x| x.1);
        r.into_iter().map(|x| x.0).collect()
    }
}

/// What a menu's "Edit layout" sends: Edit layout on the page in front.
pub fn edit_layout() -> DMsg {
    DMsg::Edit(EMsg::Start)
}

/// Edit layout on this page.
pub fn edit_page(page: &'static str) -> DMsg {
    DMsg::Edit(EMsg::StartAt(page, None))
}

/// The controller in Edit layout, for an app that reads it itself: "up" "down" "left" "right" move the picked block,
/// "lb"/"lt" shrink it and "rb"/"rt" grow it, "a" picks the next block, "x" hides it, "y" undoes, "b" finishes. None
/// when not editing (the press is the app's own); Some when Edit layout took it.
pub fn pad(d: &mut Describe, button: &str) -> Option<Task<DMsg>> {
    d.arr.as_ref()?;
    let k = match button.to_ascii_lowercase().as_str() {
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "lb" | "lt" | "l1" | "l2" => Key::Shrink,
        "rb" | "rt" | "r1" | "r2" => Key::Grow,
        "a" => Key::Next,
        "x" => Key::Hide,
        "y" => Key::Undo,
        "b" | "start" | "menu" => Key::Done,
        _ => return Some(Task::none()),
    };
    Some(d.update(DMsg::Edit(EMsg::Key(k))))
}

/// "✎ Edit layout" for an app's title, header or toolbar (always findable; `msg` is the app's message for
/// [`edit_layout`]).
pub fn edit_button<'a, M: Clone + 'a>(p: &Pal, msg: M) -> Element<'a, M> {
    let p = *p;
    mouse_area(
        container(row![ui::glyph(ui::G::Pencil, p.muted, 13.0, 1.8), t("Edit layout", 12.0, p.muted).wrapping(Wrapping::None)].spacing(6.0).align_y(iced::Center))
            .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 8.0 })
            .style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((Pal::a(p.line, 0.8), 1.0)))),
    )
    .on_press(msg)
    .interaction(mouse::Interaction::Pointer)
    .into()
}

/// Waits `ms` on a thread that sleeps (nothing polls).
async fn later(ms: u64) {
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    let _ = std::thread::Builder::new().name("hearth-ui-later".into()).stack_size(64 * 1024).spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        let _ = tx.send(());
    });
    let _ = rx.await;
}

impl Describe {
    /// Is a page being edited (Edit layout)?
    pub fn is_editing(&self) -> bool {
        self.arr.is_some()
    }

    fn def_of(&self, page: &str) -> Option<&'static blocks::PageDef> {
        self.pages.iter().find(|p| p.id == page)
    }

    /// hearth-layout app-keep for one page (an untouched page is kept as {}: the app's own again).
    fn keep_cmd(&self, page: &'static str, placed: &[Placed]) -> Vec<String> {
        let pj = match self.def_of(page) {
            Some(d) if blocks::is_default(d, placed) => json!({}),
            _ => blocks::to_json(placed),
        };
        let mut b = serde_json::Map::new();
        b.insert(page.to_string(), pj);
        let change = json!({"layout": {"blocks": b}, "features": []});
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

    fn undo_len(&self) -> usize {
        self.arr.as_ref().map(|a| a.undo.len()).unwrap_or(0)
    }

    /// Something changed while editing: saved once nothing more changes for 450 ms.
    fn changed(&mut self) -> Task<DMsg> {
        self.ed.dirty = true;
        self.ed.gen += 1;
        self.ed.toast.clear();
        self.error.clear();
        let g = self.ed.gen;
        Task::perform(later(450), move |_| DMsg::Edit(EMsg::Flush(g)))
    }

    fn save_now(&mut self) -> Task<DMsg> {
        self.ed.dirty = false;
        let Some(a) = self.arr.as_ref() else { return Task::none() };
        let c = self.keep_cmd(a.page, &a.placed);
        Task::perform(run::spawn(c), |o| DMsg::Edit(EMsg::Saved(o)))
    }

    pub(crate) fn edit(&mut self, m: EMsg) -> Task<DMsg> {
        match m {
            EMsg::Start => {
                let shown = self.ed.shown();
                let page = shown.into_iter().find(|p| self.def_of(p).is_some()).or_else(|| self.pages.first().map(|p| p.id));
                match page {
                    Some(p) => self.edit(EMsg::StartAt(p, None)),
                    None => Task::none(),
                }
            }
            EMsg::StartAt(page, pick) => {
                if self.def_of(page).is_none() {
                    return Task::none();
                }
                // a page edited before this one: what changed on it saved first
                let saved = if self.ed.dirty { self.save_now() } else { Task::none() };
                self.ed.menu = None;
                self.ed.note = None;
                self.ed.hover = None;
                self.ed.toast.clear();
                let chips: Vec<&'static str> = self.ed.shown().into_iter().filter(|p| self.def_of(p).is_some()).collect();
                self.ed.chips = chips;
                let _ = self.update(DMsg::Arrange(Some(page)));
                if let Some(b) = pick {
                    let _ = self.update(DMsg::Pick(b));
                }
                saved
            }
            EMsg::Menu(m) => {
                self.ed.menu = m;
                Task::none()
            }
            EMsg::HideNow(page, id) => {
                self.ed.menu = None;
                let Some(def) = self.def_of(page) else { return Task::none() };
                let mut placed = self.laid(page).cloned().unwrap_or_else(|| blocks::defaults(def));
                if placed.iter().filter(|p| !p.hidden && p.id != id).count() == 0 {
                    self.ed.note = Some((page, "One block stays, or the page would be empty.".into()));
                    return Task::none();
                }
                let title = def.blocks.iter().find(|b| b.id == id).map(|b| b.title).unwrap_or(id);
                for p in placed.iter_mut().filter(|p| p.id == id) {
                    p.hidden = true;
                }
                self.ed.note = Some((page, format!("{title} is hidden.")));
                let c = self.keep_cmd(page, &placed);
                Task::perform(run::spawn(c), |o| DMsg::Edit(EMsg::Saved(o)))
            }
            EMsg::Describe => {
                self.ed.menu = None;
                let saved = if self.arr.is_some() { self.edit(EMsg::Done) } else { Task::none() };
                let open = self.update(DMsg::Open);
                Task::batch([saved, open])
            }
            EMsg::Hover(id) => {
                self.ed.hover = Some(id);
                Task::none()
            }
            EMsg::Unhover(id) => {
                if self.ed.hover == Some(id) {
                    self.ed.hover = None;
                }
                Task::none()
            }
            EMsg::Grab(id, g) => self.update(DMsg::Grab(id, g)),
            EMsg::Drop => {
                let held = self.arr.as_ref().map(|a| a.grab.is_some()).unwrap_or(false);
                let n = self.undo_len();
                let _ = self.update(DMsg::Drop);
                if held && n > 0 && self.undo_len() == n { self.changed() } else { Task::none() }
            }
            EMsg::Hide(id) => {
                let n = self.undo_len();
                let _ = self.update(DMsg::Hide(id));
                if self.undo_len() > n { self.changed() } else { Task::none() }
            }
            EMsg::Tray(id) => {
                let n = self.undo_len();
                let _ = self.update(DMsg::Show(id));
                if self.undo_len() > n { self.changed() } else { Task::none() }
            }
            EMsg::Look(part) => {
                let n = self.undo_len();
                let _ = self.update(DMsg::Look(part));
                if self.undo_len() > n { self.changed() } else { Task::none() }
            }
            EMsg::Undo => {
                if self.arr.is_none() {
                    // Undo by "… is hidden.": the last thing kept, undone
                    self.ed.note = None;
                    return Task::perform(run::spawn(vec!["hearth-layout".into(), "app-undo".into(), self.app.into()]), DMsg::Kept);
                }
                if self.undo_len() == 0 {
                    // nothing to step back while editing: the last thing kept before, undone
                    return self.update(DMsg::ArrUndo);
                }
                let _ = self.update(DMsg::ArrUndo);
                self.changed()
            }
            EMsg::Reset => {
                let Some(def) = self.arr.as_ref().and_then(|a| self.def_of(a.page)) else { return Task::none() };
                if let Some(a) = self.arr.as_mut() {
                    if blocks::is_default(def, &a.placed) {
                        return Task::none();
                    }
                    a.undo.push(a.placed.clone());
                    a.placed = blocks::defaults(def);
                    a.pick = None;
                }
                self.changed()
            }
            EMsg::Done => {
                let t = if self.ed.dirty { self.save_now() } else { Task::none() };
                self.ed.hover = None;
                self.ed.menu = None;
                let _ = self.update(DMsg::Arrange(None));
                t
            }
            EMsg::Key(k) => self.key(k),
            EMsg::Flush(g) => {
                if g == self.ed.gen && self.ed.dirty { self.save_now() } else { Task::none() }
            }
            EMsg::Saved(o) => {
                if o.ok {
                    self.kept = crate::describe::read_kept(self.app);
                    if self.arr.is_some() {
                        self.ed.toast = "Saved".into();
                    } else if self.ed.note.is_some() {
                        self.ed.note_gen += 1;
                        let g = self.ed.note_gen;
                        return Task::perform(later(8000), move |_| DMsg::Edit(EMsg::NoteGone(g)));
                    }
                } else {
                    let why = if o.err.contains("No such file") { "Saving it needs hearth-layout, which comes with Hearth OS.".to_string() } else { o.why() };
                    match self.ed.note.as_mut() {
                        Some(n) if self.arr.is_none() => n.1 = why,
                        _ => self.error = why,
                    }
                }
                Task::none()
            }
            EMsg::NoteGone(g) => {
                if g == self.ed.note_gen {
                    self.ed.note = None;
                }
                Task::none()
            }
        }
    }

    fn key(&mut self, k: Key) -> Task<DMsg> {
        match k {
            Key::Done => return self.edit(EMsg::Done),
            Key::Undo => return self.edit(EMsg::Undo),
            _ => {}
        }
        let Some(a) = self.arr.as_mut() else { return Task::none() };
        let vis: Vec<&'static str> = a.placed.iter().filter(|p| !p.hidden).map(|p| p.id).collect();
        if vis.is_empty() {
            return Task::none();
        }
        let cur = a.pick.and_then(|id| vis.iter().position(|x| *x == id));
        let i = match (k, cur) {
            (Key::Next | Key::Prev, _) | (_, None) => {
                let n = vis.len();
                let i = match (cur, k) {
                    (None, Key::Prev) => n - 1,
                    (None, _) => 0,
                    (Some(i), Key::Prev) => (i + n - 1) % n,
                    (Some(i), _) => (i + 1) % n,
                };
                a.pick = Some(vis[i]);
                return Task::none();
            }
            (_, Some(i)) => i,
        };
        let id = vis[i];
        if k == Key::Hide {
            return self.edit(EMsg::Hide(id));
        }
        let Some(j) = a.placed.iter().position(|p| p.id == id) else { return Task::none() };
        let p = &a.placed[j];
        let n = match k {
            Key::Left => blocks::nudge(p, -1, 0),
            Key::Right => blocks::nudge(p, 1, 0),
            Key::Up => blocks::nudge(p, 0, -1),
            Key::Down => blocks::nudge(p, 0, 1),
            Key::Wider => blocks::grow(p, 1, 0),
            Key::Narrower => blocks::grow(p, -1, 0),
            Key::Taller => blocks::grow(p, 0, 1),
            Key::Shorter => blocks::grow(p, 0, -1),
            Key::Grow => blocks::grow(p, 1, 1),
            Key::Shrink => blocks::grow(p, -1, -1),
            _ => return Task::none(),
        };
        if n == *p {
            return Task::none();
        }
        a.undo.push(a.placed.clone());
        a.placed[j] = n;
        self.changed()
    }
}

fn key_of(key: &keyboard::Key, m: keyboard::Modifiers) -> Option<Key> {
    use keyboard::key::Named as N;
    use keyboard::Key as K;
    Some(match key.as_ref() {
        K::Named(N::Escape) => Key::Done,
        K::Character(c) if m.command() && c.eq_ignore_ascii_case("z") => Key::Undo,
        K::Named(N::ArrowLeft) => if m.shift() { Key::Narrower } else { Key::Left },
        K::Named(N::ArrowRight) => if m.shift() { Key::Wider } else { Key::Right },
        K::Named(N::ArrowUp) => if m.shift() { Key::Shorter } else { Key::Up },
        K::Named(N::ArrowDown) => if m.shift() { Key::Taller } else { Key::Down },
        K::Named(N::Tab) => if m.shift() { Key::Prev } else { Key::Next },
        K::Named(N::Enter) => Key::Next,
        K::Named(N::PageUp) => Key::Shrink,
        K::Named(N::PageDown) => Key::Grow,
        K::Named(N::Delete) => Key::Hide,
        _ => return None,
    })
}

/// Round every page: Edit layout's keys while it's being edited, and closing its right-click menu (a click anywhere
/// on the page, or Esc). At rest it only passes everything on.
type KeysFn<'a, M> = Box<dyn Fn(Key) -> Option<M> + 'a>;
type WrapFn<'a, M> = Box<dyn Fn(&'static str, Element<'a, M>) -> Element<'a, M> + 'a>;

struct Watch<'a, M> {
    content: Element<'a, M>,
    keys: Option<KeysFn<'a, M>>,
    click: Option<M>,
}

impl<'a, M: Clone + 'a> Widget<M, Theme, Renderer> for Watch<'a, M> {
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
    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation)
    }
    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        if let (Some(m), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))) = (&self.click, event) {
            shell.publish(m.clone());
        }
        if shell.is_event_captured() {
            return;
        }
        if let (Some(f), Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })) = (&self.keys, event) {
            if let Some(m) = key_of(key, *modifiers).and_then(|k| f(k)) {
                shell.publish(m);
                shell.capture_event();
            }
        }
    }
    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut Tree, layout: Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: Vector) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

// ------------------------------------------------------------------ drawing

/// A block with its right-click menu (Edit layout / Hide this / Describe it…): the block as it is, and the menu over
/// its top-left corner while it's open.
fn wrap<'a, M: Clone + 'a>(d: &Describe, p: &Pal, page: &'static str, id: &'static str, el: Element<'a, M>, map: &dyn Fn(DMsg) -> M) -> Element<'a, M> {
    let mut s = stack![el];
    if d.ed.menu == Some((page, id)) {
        s = s.push(container(block_menu(p, map, page, id)).padding(8.0));
    }
    mouse_area(s).on_right_press(map(DMsg::Edit(EMsg::Menu(Some((page, id)))))).into()
}

fn block_menu<'a, M: Clone + 'a>(p: &Pal, map: &dyn Fn(DMsg) -> M, page: &'static str, id: &'static str) -> Element<'a, M> {
    let p = *p;
    let item = |label: &'static str, m: M| -> Element<'a, M> {
        mouse_area(container(t(label, 13.0, p.text).wrapping(Wrapping::None)).padding(Padding { top: 7.0, right: 14.0, bottom: 7.0, left: 12.0 }).width(Length::Fill))
            .on_press(m)
            .interaction(mouse::Interaction::Pointer)
            .into()
    };
    opaque(
        container(
            column![
                item("Edit layout", map(DMsg::Edit(EMsg::StartAt(page, Some(id))))),
                item("Hide this", map(DMsg::Edit(EMsg::HideNow(page, id)))),
                item("Describe it…", map(DMsg::Edit(EMsg::Describe))),
            ]
            .width(Length::Fixed(190.0)),
        )
        .padding(4.0)
        .style(ui::boxed(p.panel, p.r(10.0), Some((p.line, 1.0)))),
    )
}

/// The page with "… is hidden. Undo · Edit layout" along its bottom after Hide this (gone after 8 s).
fn noted<'a, M: Clone + 'a>(d: &Describe, p: &Pal, page: &'static str, el: Element<'a, M>, map: &dyn Fn(DMsg) -> M) -> Element<'a, M> {
    let mut s = stack![el];
    if let (None, Some((pg, said))) = (&d.arr, &d.ed.note) {
        if *pg == page {
            let p = *p;
            let card = container(
                row![
                    t(said.clone(), 13.0, p.text),
                    ui::btn(&p, "Undo", ui::Kind::Normal, false, Some(map(DMsg::Edit(EMsg::Undo)))),
                    ui::btn(&p, "Edit layout", ui::Kind::Primary, false, Some(map(DMsg::Edit(EMsg::StartAt(page, None))))),
                ]
                .spacing(12.0)
                .align_y(iced::Center),
            )
            .padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 16.0 })
            .style(ui::boxed(p.panel, p.r(12.0), Some((p.accent, 1.5))));
            s = s.push(container(opaque(card)).width(Length::Fill).height(Length::Fill).align_x(iced::Center).align_y(iced::Bottom).padding(16.0));
        }
    }
    s.into()
}

/// The page: the app's own view (`original`) when nothing is customised, else its blocks where they were put; the
/// live page in Edit layout while it's being edited. `block` makes one block's view by its id.
pub fn page<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page: &'static str,
    original: impl FnOnce() -> Element<'a, M>,
    block: impl Fn(&'static str) -> Element<'a, M> + 'a,
    map: impl Fn(DMsg) -> M + Clone + 'a,
) -> Element<'a, M> {
    page_in(d, pal, page, None, original, block, map)
}

/// [`page`] for a page that sits in something that scrolls (Settings' pages): laid out or edited, its blocks take
/// this height (a grid in a scroll has no height of its own).
pub fn page_in<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page: &'static str,
    height: Option<f32>,
    original: impl FnOnce() -> Element<'a, M>,
    block: impl Fn(&'static str) -> Element<'a, M> + 'a,
    map: impl Fn(DMsg) -> M + Clone + 'a,
) -> Element<'a, M> {
    let h = height.map(Length::Fixed).unwrap_or(Length::Fill);
    let editing = d.arr.as_ref().filter(|a| a.page == page);
    let el: Element<'a, M> = if let Some(a) = editing {
        editor(d, a, pal, &block, &map, h)
    } else if let Some(placed) = d.laid(page) {
        let mut layers: Vec<Element<'a, M>> = Vec::with_capacity(placed.len());
        for pl in placed.iter().filter(|p| !p.hidden) {
            let el = dressed(pal, pl, &block);
            let el = wrap(d, pal, page, pl.id, el, &map);
            layers.push(place(pl, outline(pal, d.block_changing(page, pl.id), el)));
        }
        stack(layers).width(Length::Fill).height(h).into()
    } else {
        original()
    };
    let el = noted(d, pal, page, el, &map);
    d.ed.mark(page);
    let menu_here = d.ed.menu.map(|x| x.0) == Some(page);
    let keys: Option<KeysFn<'a, M>> = if editing.is_some() {
        let m = map.clone();
        Some(Box::new(move |k: Key| Some(m(DMsg::Edit(EMsg::Key(k))))) as KeysFn<'a, M>)
    } else if menu_here {
        let m = map.clone();
        Some(Box::new(move |k: Key| if k == Key::Done { Some(m(DMsg::Edit(EMsg::Menu(None)))) } else { None }) as KeysFn<'a, M>)
    } else {
        None
    };
    let click = if menu_here { Some(map(DMsg::Edit(EMsg::Menu(None)))) } else { None };
    Element::new(Watch { content: el, keys, click })
}


fn chip<'a, M: Clone + 'a>(p: &Pal, label: String, on: bool, msg: M) -> Element<'a, M> {
    let p = *p;
    mouse_area(
        container(t(label, 12.0, if on { p.accent } else { p.text }).wrapping(Wrapping::None))
            .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
            .style(ui::boxed(if on { Pal::a(p.accent, 0.10) } else { Color::TRANSPARENT }, 12.0, Some((if on { p.accent } else { p.line }, 1.0)))),
    )
    .on_press(msg)
    .interaction(mouse::Interaction::Pointer)
    .into()
}

/// Edit layout on the live page: each block as it is with a soft outline, its name under the pointer, moved by
/// dragging it anywhere, sized by its right edge, its bottom edge or its corner; the grid shows only while one is held.
fn editor<'a, M: Clone + 'a>(d: &'a Describe, a: &'a Arrange, pal: &Pal, block: &dyn Fn(&'static str) -> Element<'a, M>, map: &dyn Fn(DMsg) -> M, h: Length) -> Element<'a, M> {
    let p = *pal;
    let e = |m: EMsg| map(DMsg::Edit(m));
    let mut layers: Vec<Element<'a, M>> = Vec::with_capacity(a.placed.len() + 2);
    if a.grab.is_some() {
        layers.push(canvas::Canvas::new(GridLines(Pal::a(p.accent, 0.16))).width(Length::Fill).height(Length::Fill).into());
    }
    for pl in a.placed.iter().filter(|x| !x.hidden) {
        let picked = a.pick == Some(pl.id);
        let hov = d.ed.hover == Some(pl.id);
        let held = a.grab.as_ref().map(|g| g.0 == pl.id).unwrap_or(false);
        let content = dressed(&p, pl, block);
        let wash: Element<'a, M> = container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui::boxed(Pal::a(p.accent, if held { 0.10 } else if hov || picked { 0.05 } else { 0.0 }), 0.0, None))
            .into();
        let mover: Element<'a, M> = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(e(EMsg::Grab(pl.id, Grip::Move)))
            .on_right_press(map(DMsg::Pick(pl.id)))
            .on_enter(e(EMsg::Hover(pl.id)))
            .on_exit(e(EMsg::Unhover(pl.id)))
            .interaction(if held { mouse::Interaction::Grabbing } else { mouse::Interaction::Grab })
            .into();
        let mut top = Row::new().spacing(6.0).align_y(iced::Center).width(Length::Fill);
        if picked || hov || held {
            let bg = if picked { p.accent } else { Pal::a(p.panel3, 0.96) };
            let fg = if picked { p.ink } else { p.text };
            top = top.push(
                container(row![t("⠿", 12.0, fg), ts(pl.title, 12.0, fg).wrapping(Wrapping::None)].spacing(6.0).align_y(iced::Center))
                    .padding(Padding { top: 3.0, right: 9.0, bottom: 3.0, left: 7.0 })
                    .style(ui::boxed(bg, p.r(8.0), None)),
            );
            top = top.push(Space::new().width(Length::Fill));
            if picked {
                top = top.push(
                    mouse_area(container(t("✕ Hide", 12.0, p.ink).wrapping(Wrapping::None)).padding(Padding { top: 3.0, right: 9.0, bottom: 3.0, left: 9.0 }).style(ui::boxed(p.accent, p.r(8.0), None)))
                        .on_press(e(EMsg::Hide(pl.id)))
                        .interaction(mouse::Interaction::Pointer),
                );
            }
        }
        let tag = container(top).padding(6.0).width(Length::Fill);
        let edge_c = Pal::a(p.accent, if picked || hov { 0.55 } else { 0.0 });
        let right = row![
            Space::new().width(Length::Fill),
            mouse_area(container(Space::new()).width(Length::Fixed(7.0)).height(Length::Fill).style(ui::boxed(edge_c, 3.0, None)))
                .on_press(e(EMsg::Grab(pl.id, Grip::Wide)))
                .interaction(mouse::Interaction::ResizingHorizontally)
        ]
        .height(Length::Fill);
        let bottom = column![
            Space::new().height(Length::Fill),
            mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fixed(7.0)).style(ui::boxed(edge_c, 3.0, None)))
                .on_press(e(EMsg::Grab(pl.id, Grip::Tall)))
                .interaction(mouse::Interaction::ResizingVertically)
        ]
        .width(Length::Fill);
        let handle = mouse_area(container(Space::new().width(Length::Fixed(13.0)).height(Length::Fixed(13.0))).style(ui::boxed(p.accent, 3.0, Some((p.ink, 1.0)))))
            .on_press(e(EMsg::Grab(pl.id, Grip::Size)))
            .interaction(mouse::Interaction::ResizingDiagonallyDown);
        let corner = column![Space::new().height(Length::Fill), row![Space::new().width(Length::Fill), handle]].width(Length::Fill).height(Length::Fill);
        let line = if picked { (p.accent, 2.0) } else if hov || held { (Pal::a(p.accent, 0.8), 1.5) } else { (Pal::a(p.accent, 0.35), 1.0) };
        let framed = container(stack![content, wash, mover, tag, right, bottom, corner]).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some(line)));
        layers.push(place(pl, framed.into()));
    }
    if a.grab.is_some() {
        // while a block is held: the cell under the pointer, so it follows on the grid (snapped, drawn as it goes)
        let mut rows = Column::new().width(Length::Fill).height(Length::Fill);
        for r in 0..ROWS {
            let mut line = Row::new().width(Length::Fill).height(Length::Fill);
            for c in 0..COLS {
                line = line.push(mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_enter(map(DMsg::Cell(c, r))).interaction(mouse::Interaction::Grabbing));
            }
            rows = rows.push(line);
        }
        layers.push(rows.into());
    }
    let board = container(stack(layers).width(Length::Fill).height(Length::Fill)).width(Length::Fill).height(h);
    mouse_area(column![bar(d, a, &p, map), board].spacing(8.0).width(Length::Fill)).on_release(e(EMsg::Drop)).into()
}

/// Edit layout's bar over the page: its name, the other pages on the screen, Undo / Back to the original / Describe
/// it… / Done, and under them the picked block's Look (or how it works) and the hidden blocks to add back.
fn bar<'a, M: Clone + 'a>(d: &'a Describe, a: &'a Arrange, p: &Pal, map: &dyn Fn(DMsg) -> M) -> Element<'a, M> {
    let p = *p;
    let e = |m: EMsg| map(DMsg::Edit(m));
    let title = d.pages.iter().find(|x| x.id == a.page).map(|x| x.title).unwrap_or("");
    let mut top = Row::new().spacing(10.0).align_y(iced::Center).push(crate::describe::editing::<M>(&p)).push(ts(format!("Edit layout · {} · {title}", d.name), 14.0, p.text).wrapping(Wrapping::None));
    for pg in d.ed.chips.iter().copied().filter(|x| *x != a.page) {
        if let Some(def) = d.pages.iter().find(|x| x.id == pg) {
            top = top.push(chip(&p, format!("Edit {}", def.title), false, e(EMsg::StartAt(def.id, None))));
        }
    }
    let (said, c) = if !d.error.is_empty() { (d.error.clone(), p.danger) } else { (d.ed.toast.clone(), p.good) };
    let buttons = row![
        t(said, 12.0, c).wrapping(Wrapping::None),
        ui::btn(&p, "Undo", ui::Kind::Normal, false, Some(e(EMsg::Undo))),
        ui::btn(&p, "Back to the original", ui::Kind::Normal, false, Some(e(EMsg::Reset))),
        ui::btn(&p, "Describe it…", ui::Kind::Normal, false, Some(e(EMsg::Describe))),
        ui::btn(&p, "Done", ui::Kind::Primary, false, Some(e(EMsg::Done))),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    let mut under = Row::new().spacing(8.0).align_y(iced::Center);
    match a.pick.and_then(|id| a.placed.iter().find(|x| x.id == id)) {
        Some(pl) => {
            under = under.push(ts(format!("Look of {}:", pl.title), 12.0, p.text));
            for part in [LookPart::Corners, LookPart::See, LookPart::Font, LookPart::Size, LookPart::Accent] {
                let changed = match part {
                    LookPart::Corners => pl.look.corners.is_some(),
                    LookPart::See => pl.look.see > 0,
                    LookPart::Font => !pl.look.font.is_empty(),
                    LookPart::Size => !pl.look.size.is_empty(),
                    LookPart::Accent => !pl.look.accent.is_empty(),
                };
                under = under.push(chip(&p, blocks::look_label(&pl.look, part), changed, e(EMsg::Look(part))));
            }
            if let Some(c) = pl.look.accent_colour() {
                under = under.push(container(Space::new().width(Length::Fixed(14.0)).height(Length::Fixed(14.0))).style(ui::boxed(c, 7.0, None)));
            }
        }
        None => {
            under = under.push(t("Drag a block to move it, its edge or corner to size it · Tab picks one, arrows move it, Shift+arrows size it · Ctrl+Z undoes · Esc when you're done · saved as you go", 12.0, p.muted));
        }
    }
    let hidden: Vec<&Placed> = a.placed.iter().filter(|x| x.hidden).collect();
    if !hidden.is_empty() {
        under = under.push(container(t("Hidden:", 12.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 8.0 }));
        for hb in hidden {
            under = under.push(chip(&p, format!("+ {}", hb.title), false, e(EMsg::Tray(hb.id))));
        }
    }
    let body = column![row![container(top.wrap()).width(Length::Fill), buttons].spacing(12.0).align_y(iced::Center), under.wrap()].spacing(8.0);
    container(body).padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.accent, 1.5)))).into()
}

/// A page's blocks as they're already made (each app's view makes its pieces, then puts them together): `original`
/// puts them together the app's own way (exactly as before, when nothing is customised), else each goes in its place.
/// In the app's own way each block still has its right-click menu (Edit layout / Hide this / Describe it…).
pub struct Parts<'a, M> {
    items: Vec<(&'static str, Option<Element<'a, M>>)>,
    wrap: Option<WrapFn<'a, M>>,
}

impl<'a, M: 'a> Parts<'a, M> {
    /// The piece by its id (taken once; nothing when it's gone already).
    pub fn take(&mut self, id: &str) -> Element<'a, M> {
        let Some((sid, el)) = self.items.iter_mut().find(|x| x.0 == id).and_then(|x| x.1.take().map(|e| (x.0, e))) else { return Space::new().into() };
        match &self.wrap {
            Some(w) => w(sid, el),
            None => el,
        }
    }
}

/// [`page`] for pieces already made: `items` by their block ids, `original` the app's own way of putting them together.
pub fn parts<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page_id: &'static str,
    items: Vec<(&'static str, Element<'a, M>)>,
    original: impl FnOnce(&mut Parts<'a, M>) -> Element<'a, M>,
    map: impl Fn(DMsg) -> M + Clone + 'a,
) -> Element<'a, M> {
    parts_in(d, pal, page_id, None, items, original, map)
}

/// [`parts`] for a page in something that scrolls ([`page_in`]).
pub fn parts_in<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page_id: &'static str,
    height: Option<f32>,
    items: Vec<(&'static str, Element<'a, M>)>,
    original: impl FnOnce(&mut Parts<'a, M>) -> Element<'a, M>,
    map: impl Fn(DMsg) -> M + Clone + 'a,
) -> Element<'a, M> {
    let w: Option<WrapFn<'a, M>> = match d.pages.iter().find(|p| p.id == page_id) {
        Some(def) => {
            let pal = *pal;
            let m = map.clone();
            Some(Box::new(move |id: &'static str, el: Element<'a, M>| -> Element<'a, M> {
                if def.blocks.iter().any(|b| b.id == id) { wrap(d, &pal, page_id, id, el, &m) } else { el }
            }) as WrapFn<'a, M>)
        }
        None => None,
    };
    let cell = Rc::new(RefCell::new(Parts { items: items.into_iter().map(|(i, e)| (i, Some(e))).collect(), wrap: None }));
    let c2 = cell.clone();
    page_in(
        d,
        pal,
        page_id,
        height,
        move || {
            let mut ps = cell.borrow_mut();
            ps.wrap = w;
            original(&mut ps)
        },
        move |id| c2.borrow_mut().take(id),
        map,
    )
}
