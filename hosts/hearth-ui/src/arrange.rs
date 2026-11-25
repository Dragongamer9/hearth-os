//! A page made of blocks, drawn (blocks.rs has the rules; describe.rs the state): [`page`] is what every Hearth app
//! puts where its page was. Nothing customised and not being edited: the app's own view, untouched. Laid out: each
//! block in its place on the 12 × 12 grid, in its look.
//!
//! Edit layout (the owner, Nov 24: "editing an app's layout must feel natural, never awkward"): right-click any block
//! for Edit layout / Hide this / Describe it… (the menu opens at the pointer), or pick Edit layout in the app's menu
//! (Describe it's box has it too). The live app stays on the screen exactly as it draws itself: each block gets a soft
//! outline and its name under the pointer. At the first change each block starts on the grid from where it really was
//! drawn; drag anywhere on a block to move it, its right or bottom edge or its corner to size it, snapped to the grid,
//! the others pushed out of the way as it goes (and back when it moves on). The bar over the page has the hidden blocks
//! (press one to add it back, drag it where it goes), the picked block's Look, Undo, Back to the original and Done.
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
use iced::widget::{column, container, mouse_area, opaque, pin, row, stack, Column, Row, Space};
use iced::{keyboard, mouse, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Task, Theme, Vector};
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
    /// the right-click menu closed (Some: opened on that block at its corner, for the older callers)
    Menu(Option<(&'static str, &'static str)>),
    /// the right-click menu on a block, at the pointer: page, block, the pointer in the block, the block's size
    MenuAt(&'static str, &'static str, f32, f32, f32, f32),
    /// the grid cell under the pointer while a block is held (others pushed aside as it goes)
    Cell(u8, u8),
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
    /// where the right-click menu opens in its block (x, y) and the block's size
    pub menu_at: (f32, f32, f32, f32),
    /// the page being edited in place: drawn exactly as the app draws it until something changes (then each block's
    /// starting place comes from where it really was: [`Self::base`])
    pub fresh: Option<&'static str>,
    /// the blocks where the app really drew them, on the grid (worked out at the first change)
    base: Vec<Placed>,
    /// a block being dragged back from the hidden tray (it follows the pointer)
    tray: Option<&'static str>,
    /// where each block of the page edited in place was drawn ("" the page itself): recorded while it's drawn
    rects: RefCell<Vec<(&'static str, &'static str, Rectangle)>>,
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
    fn keep_cmd(&self, page: &'static str, placed: &[Placed], own: bool) -> Vec<String> {
        let pj = match self.def_of(page) {
            _ if own => json!({}),
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
        // back where the app draws everything itself: kept as the app's own ({}), not as the grid's nearest
        let own = self.in_place(a.page);
        let c = self.keep_cmd(a.page, &a.placed, own);
        Task::perform(run::spawn(c), |o| DMsg::Edit(EMsg::Saved(o)))
    }

    /// Is this page being edited in place (drawn exactly as the app draws it, nothing changed on it yet)?
    pub fn in_place(&self, page: &str) -> bool {
        let (Some(a), Some(def)) = (self.arr.as_ref(), self.def_of(page)) else { return false };
        self.ed.fresh == Some(page) && a.page == page && if self.ed.base.is_empty() { blocks::is_default(def, &a.placed) } else { a.placed == self.ed.base }
    }

    /// The first change on a page edited in place: each block's starting place on the grid is where the app really
    /// drew it (recorded while it was drawn), tidied so nothing overlaps. Nothing moves until this change.
    fn adopt(&mut self) {
        let Some(page) = self.ed.fresh else { return };
        if !self.in_place(page) {
            return;
        }
        if self.ed.base.is_empty() {
            let rects = self.ed.rects.borrow().clone();
            let Some(pr) = rects.iter().find(|r| r.0 == page && r.1.is_empty()).map(|r| r.2) else { return };
            if pr.width < 1.0 || pr.height < 1.0 {
                return;
            }
            let Some(a) = self.arr.as_ref() else { return };
            let mut base = a.placed.clone();
            let cell = |v: f32, o: f32, len: f32, n: u8| -> u8 { (((v - o) / len) * n as f32).round().clamp(0.0, n as f32) as u8 };
            for p in base.iter_mut() {
                if let Some(r) = rects.iter().find(|r| r.0 == page && r.1 == p.id).map(|r| r.2) {
                    let x = cell(r.x, pr.x, pr.width, COLS).min(COLS - 1);
                    let x2 = cell(r.x + r.width, pr.x, pr.width, COLS).max(x + 1).min(COLS);
                    let y = cell(r.y, pr.y, pr.height, ROWS).min(ROWS - 1);
                    let y2 = cell(r.y + r.height, pr.y, pr.height, ROWS).max(y + 1).min(ROWS);
                    p.x = x;
                    p.w = x2 - x;
                    p.y = y;
                    p.h = y2 - y;
                }
            }
            blocks::push_aside(&mut base, None);
            self.ed.base = base;
        }
        let base = self.ed.base.clone();
        if let Some(a) = self.arr.as_mut() {
            a.placed = base;
        }
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
                let _ = self.arrange(DMsg::Arrange(Some(page)));
                // nothing kept for it: edited in place, exactly as the app draws it
                self.ed.fresh = if self.laid(page).is_none() { Some(page) } else { None };
                self.ed.base.clear();
                self.ed.tray = None;
                if let Ok(mut r) = self.ed.rects.try_borrow_mut() {
                    r.retain(|x| x.0 != page);
                }
                if let Some(b) = pick {
                    let _ = self.update(DMsg::Pick(b));
                }
                saved
            }
            EMsg::Menu(m) => {
                self.ed.menu = m;
                self.ed.menu_at = (0.0, 0.0, 0.0, 0.0);
                Task::none()
            }
            EMsg::MenuAt(page, id, x, y, w, h) => {
                self.ed.menu = Some((page, id));
                self.ed.menu_at = (x, y, w, h);
                Task::none()
            }
            EMsg::Cell(c, r) => {
                let _ = self.update(DMsg::Cell(c, r));
                let tray = self.ed.tray;
                if let Some(a) = self.arr.as_mut() {
                    let held = a.grab.as_ref().map(|g| g.0);
                    if let (Some(id), Some(mut base)) = (held, a.undo.last().cloned()) {
                        if let Some(i) = a.placed.iter().position(|p| p.id == id).filter(|i| *i < base.len()) {
                            let mut moved = a.placed[i].clone();
                            if tray == Some(id) {
                                // dragged back from the tray: it follows the pointer
                                moved.x = (c as i16 - (moved.w / 2) as i16).clamp(0, (COLS - moved.w) as i16) as u8;
                                moved.y = r.min(ROWS - moved.h);
                            }
                            base[i] = moved;
                            blocks::push_aside(&mut base, Some(i));
                            a.placed = base;
                        }
                    }
                }
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
                let c = self.keep_cmd(page, &placed, false);
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
            EMsg::Grab(id, g) => {
                self.adopt();
                self.update(DMsg::Grab(id, g))
            }
            EMsg::Drop => {
                self.ed.tray = None;
                let held = self.arr.as_ref().map(|a| a.grab.is_some()).unwrap_or(false);
                let n = self.undo_len();
                let _ = self.update(DMsg::Drop);
                if held && n > 0 && self.undo_len() == n { self.changed() } else { Task::none() }
            }
            EMsg::Hide(id) => {
                self.adopt();
                let n = self.undo_len();
                let _ = self.update(DMsg::Hide(id));
                if self.undo_len() > n { self.changed() } else { Task::none() }
            }
            EMsg::Tray(id) => {
                // added back where it was (the others make way), and held: drag it where it goes
                self.adopt();
                let n = self.undo_len();
                let _ = self.update(DMsg::Show(id));
                let t = if self.undo_len() > n { self.changed() } else { Task::none() };
                if let Some(a) = self.arr.as_mut() {
                    if let Some(i) = a.placed.iter().position(|p| p.id == id) {
                        blocks::push_aside(&mut a.placed, Some(i));
                    }
                }
                let _ = self.update(DMsg::Grab(id, Grip::Move));
                self.ed.tray = Some(id);
                t
            }
            EMsg::Look(part) => {
                self.adopt();
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
                let Some(page) = self.arr.as_ref().map(|a| a.page) else { return Task::none() };
                let Some(def) = self.def_of(page) else { return Task::none() };
                if self.in_place(page) {
                    return Task::none();
                }
                // back to where the app draws everything itself: in place again
                let own = if self.ed.fresh == Some(page) && !self.ed.base.is_empty() { self.ed.base.clone() } else { blocks::defaults(def) };
                if self.ed.fresh != Some(page) {
                    self.ed.fresh = Some(page);
                    self.ed.base.clear();
                }
                if let Some(a) = self.arr.as_mut() {
                    a.undo.push(a.placed.clone());
                    a.placed = own;
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
            Key::Next | Key::Prev => {}
            _ => self.adopt(),
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
        let mut base = a.placed.clone();
        base[j] = n;
        blocks::push_aside(&mut base, Some(j));
        a.undo.push(a.placed.clone());
        a.placed = base;
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

/// Round every page (and every block while editing or with its right-click menu): Edit layout's keys while it's being
/// edited, closing the right-click menu (a click anywhere on the page, or Esc), the right-click itself (at the pointer),
/// and where a block edited in place is drawn. At rest it only passes everything on.
type KeysFn<'a, M> = Box<dyn Fn(Key) -> Option<M> + 'a>;
type WrapFn<'a, M> = Box<dyn Fn(&'static str, Element<'a, M>) -> Element<'a, M> + 'a>;
type RClickFn<'a, M> = Box<dyn Fn(Point, Size) -> M + 'a>;
type Rects = RefCell<Vec<(&'static str, &'static str, Rectangle)>>;

struct Watch<'a, M> {
    content: Element<'a, M>,
    keys: Option<KeysFn<'a, M>>,
    click: Option<M>,
    rclick: Option<RClickFn<'a, M>>,
    spot: Option<(&'a Rects, &'static str, &'static str)>,
}

fn watch<'a, M>(content: Element<'a, M>) -> Watch<'a, M> {
    Watch { content, keys: None, click: None, rclick: None, spot: None }
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
        if let Some((rects, page, id)) = self.spot {
            if let Ok(mut v) = rects.try_borrow_mut() {
                let b = layout.bounds();
                match v.iter_mut().find(|r| r.0 == page && r.1 == id) {
                    Some(r) => r.2 = b,
                    None => v.push((page, id, b)),
                }
            }
        }
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
        if let (Some(f), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))) = (&self.rclick, event) {
            let b = layout.bounds();
            if let Some(pt) = cursor.position_in(b) {
                shell.publish(f(pt, b.size()));
                shell.capture_event();
                return;
            }
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

/// A block with its right-click menu (Edit layout / Hide this / Describe it…): the block as it is, and the menu where
/// it was right-clicked while it's open.
fn wrap<'a, M: Clone + 'a, F: Fn(DMsg) -> M + Clone + 'a>(d: &Describe, p: &Pal, page: &'static str, id: &'static str, el: Element<'a, M>, map: &F) -> Element<'a, M> {
    let mut s = stack![el];
    if d.ed.menu == Some((page, id)) {
        let (x, y, w, h) = d.ed.menu_at;
        // at the pointer, kept inside the block (190 × 112: the menu's own size)
        let px = if w > 0.0 { x.min(w - 198.0).max(0.0) } else { 8.0 };
        let py = if h > 0.0 { y.min(h - 120.0).max(0.0) } else { 8.0 };
        s = s.push(pin(block_menu(p, map, page, id)).x(px).y(py));
    }
    let m = map.clone();
    let mut w = watch(s.into());
    w.rclick = Some(Box::new(move |pt: Point, sz: Size| m(DMsg::Edit(EMsg::MenuAt(page, id, pt.x, pt.y, sz.width, sz.height)))) as RClickFn<'a, M>);
    Element::new(w)
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
    page_inner(d, pal, page, height, false, original, block, map)
}

/// The page, laid out, edited (in place when `in_place`: the app's own view with each block's grips over it, from
/// [`parts`]) or as the app draws it.
#[allow(clippy::too_many_arguments)]
fn page_inner<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page: &'static str,
    height: Option<f32>,
    in_place: bool,
    original: impl FnOnce() -> Element<'a, M>,
    block: impl Fn(&'static str) -> Element<'a, M> + 'a,
    map: impl Fn(DMsg) -> M + Clone + 'a,
) -> Element<'a, M> {
    let h = height.map(Length::Fixed).unwrap_or(Length::Fill);
    let editing = d.arr.as_ref().filter(|a| a.page == page);
    let el: Element<'a, M> = if let Some(a) = editing {
        if in_place && d.in_place(page) {
            // exactly where the app draws everything: the page itself measured, each block by its grips (parts)
            let mut board = watch(original());
            board.spot = Some((&d.ed.rects, page, ""));
            let board = container(Element::new(board)).width(Length::Fill).height(h);
            let e = map.clone();
            mouse_area(column![bar(d, a, pal, &map), board].spacing(8.0).width(Length::Fill)).on_release(e(DMsg::Edit(EMsg::Drop))).into()
        } else {
            editor(d, a, pal, &block, &map, h)
        }
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
    let mut w = watch(el);
    w.keys = keys;
    w.click = if menu_here { Some(map(DMsg::Edit(EMsg::Menu(None)))) } else { None };
    Element::new(w)
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

/// One block's grips in Edit layout, over the block as it's drawn: a wash, the mover (drag anywhere), its name and ✕
/// Hide, its right and bottom edges and its corner; and the line round it.
fn chrome<'a, M: Clone + 'a>(d: &Describe, a: &Arrange, p: &Pal, id: &'static str, title: &'static str, map: &dyn Fn(DMsg) -> M) -> (Vec<Element<'a, M>>, (Color, f32)) {
    let p = *p;
    let e = |m: EMsg| map(DMsg::Edit(m));
    let picked = a.pick == Some(id);
    let hov = d.ed.hover == Some(id);
    let held = a.grab.as_ref().map(|g| g.0 == id).unwrap_or(false);
    let wash: Element<'a, M> = container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui::boxed(Pal::a(p.accent, if held { 0.10 } else if hov || picked { 0.05 } else { 0.0 }), 0.0, None))
        .into();
    let mover: Element<'a, M> = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
        .on_press(e(EMsg::Grab(id, Grip::Move)))
        .on_right_press(map(DMsg::Pick(id)))
        .on_enter(e(EMsg::Hover(id)))
        .on_exit(e(EMsg::Unhover(id)))
        .interaction(if held { mouse::Interaction::Grabbing } else { mouse::Interaction::Grab })
        .into();
    // its name only while the pointer is on it (the picked one's name and ✕ Hide are in the bar), so nothing of the
    // block is covered while it just sits there
    let mut top = Row::new().spacing(6.0).align_y(iced::Center).width(Length::Fill);
    if hov || held {
        let bg = if picked { p.accent } else { Pal::a(p.panel3, 0.96) };
        let fg = if picked { p.ink } else { p.text };
        top = top.push(
            container(row![t("⠿", 12.0, fg), ts(title, 12.0, fg).wrapping(Wrapping::None)].spacing(6.0).align_y(iced::Center))
                .padding(Padding { top: 3.0, right: 9.0, bottom: 3.0, left: 7.0 })
                .style(ui::boxed(bg, p.r(8.0), None)),
        );
    }
    let tag: Element<'a, M> = container(top).padding(6.0).width(Length::Fill).into();
    let edge_c = Pal::a(p.accent, if picked || hov { 0.55 } else { 0.0 });
    let right: Element<'a, M> = row![
        Space::new().width(Length::Fill),
        mouse_area(container(Space::new()).width(Length::Fixed(7.0)).height(Length::Fill).style(ui::boxed(edge_c, 3.0, None)))
            .on_press(e(EMsg::Grab(id, Grip::Wide)))
            .interaction(mouse::Interaction::ResizingHorizontally)
    ]
    .height(Length::Fill)
    .into();
    let bottom: Element<'a, M> = column![
        Space::new().height(Length::Fill),
        mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fixed(7.0)).style(ui::boxed(edge_c, 3.0, None)))
            .on_press(e(EMsg::Grab(id, Grip::Tall)))
            .interaction(mouse::Interaction::ResizingVertically)
    ]
    .width(Length::Fill)
    .into();
    let handle = mouse_area(container(Space::new().width(Length::Fixed(13.0)).height(Length::Fixed(13.0))).style(ui::boxed(p.accent, 3.0, Some((p.ink, 1.0)))))
        .on_press(e(EMsg::Grab(id, Grip::Size)))
        .interaction(mouse::Interaction::ResizingDiagonallyDown);
    let corner: Element<'a, M> = column![Space::new().height(Length::Fill), row![Space::new().width(Length::Fill), handle]].width(Length::Fill).height(Length::Fill).into();
    let line = if picked { (p.accent, 2.0) } else if hov || held { (Pal::a(p.accent, 0.8), 1.5) } else { (Pal::a(p.accent, 0.35), 1.0) };
    (vec![wash, mover, tag, right, bottom, corner], line)
}

/// A block of a page edited in place: drawn where the app draws it, its grips over it, and where it is recorded.
fn in_place_block<'a, M: Clone + 'a>(d: &'a Describe, p: &Pal, page: &'static str, id: &'static str, title: &'static str, el: Element<'a, M>, map: &dyn Fn(DMsg) -> M) -> Element<'a, M> {
    let Some(a) = d.arr.as_ref() else { return el };
    let (layers, line) = chrome(d, a, p, id, title, map);
    let mut s = stack![el];
    for l in layers {
        s = s.push(l);
    }
    let framed: Element<'a, M> = container(s).style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some(line))).into();
    let mut w = watch(framed);
    w.spot = Some((&d.ed.rects, page, id));
    Element::new(w)
}

/// Edit layout once something has changed: each block in its place on the grid, as it is with its grips, moved by
/// dragging it anywhere, sized by its right edge, its bottom edge or its corner (the others make way); the grid shows
/// only while one is held.
fn editor<'a, M: Clone + 'a>(d: &'a Describe, a: &'a Arrange, pal: &Pal, block: &dyn Fn(&'static str) -> Element<'a, M>, map: &dyn Fn(DMsg) -> M, h: Length) -> Element<'a, M> {
    let p = *pal;
    let e = |m: EMsg| map(DMsg::Edit(m));
    let mut layers: Vec<Element<'a, M>> = Vec::with_capacity(a.placed.len() + 2);
    if a.grab.is_some() {
        layers.push(canvas::Canvas::new(GridLines(Pal::a(p.accent, 0.16))).width(Length::Fill).height(Length::Fill).into());
    }
    for pl in a.placed.iter().filter(|x| !x.hidden) {
        let content = dressed(&p, pl, block);
        let (chrome_layers, line) = chrome(d, a, &p, pl.id, pl.title, map);
        let mut s = stack![content];
        for l in chrome_layers {
            s = s.push(l);
        }
        let framed = container(s).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some(line)));
        layers.push(place(pl, framed.into()));
    }
    if a.grab.is_some() {
        // while a block is held: the cell under the pointer, so it follows on the grid (snapped, the others making way)
        let mut rows = Column::new().width(Length::Fill).height(Length::Fill);
        for r in 0..ROWS {
            let mut line = Row::new().width(Length::Fill).height(Length::Fill);
            for c in 0..COLS {
                line = line.push(mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_enter(e(EMsg::Cell(c, r))).interaction(mouse::Interaction::Grabbing));
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
            under = under.push(chip(&p, format!("✕ Hide {}", pl.title), false, e(EMsg::Hide(pl.id))));
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
            under = under.push(t("Drag a block to move it, its edge or corner to size it (the others make way) · Tab picks one, arrows move it, Shift+arrows size it · Ctrl+Z undoes · Esc when you're done · saved as you go", 12.0, p.muted));
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
    // edited in place: each block where the app draws it, with its grips; else each with its right-click menu
    let fresh = d.in_place(page_id);
    let w: Option<WrapFn<'a, M>> = match d.pages.iter().find(|p| p.id == page_id) {
        Some(def) => {
            let pal = *pal;
            let m = map.clone();
            Some(Box::new(move |id: &'static str, el: Element<'a, M>| -> Element<'a, M> {
                match def.blocks.iter().find(|b| b.id == id) {
                    Some(b) if fresh => in_place_block(d, &pal, page_id, b.id, b.title, el, &m),
                    Some(b) => wrap(d, &pal, page_id, b.id, el, &m),
                    None => el,
                }
            }) as WrapFn<'a, M>)
        }
        None => None,
    };
    let cell = Rc::new(RefCell::new(Parts { items: items.into_iter().map(|(i, e)| (i, Some(e))).collect(), wrap: None }));
    let c2 = cell.clone();
    page_inner(
        d,
        pal,
        page_id,
        height,
        true,
        move || {
            let mut ps = cell.borrow_mut();
            ps.wrap = w;
            original(&mut ps)
        },
        move |id| c2.borrow_mut().take(id),
        map,
    )
}
