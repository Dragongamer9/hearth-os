//! A page made of blocks, drawn (blocks.rs and studio_model.rs have the rules; describe.rs the state): [`page`] and
//! [`parts`] are what every Hearth app puts where its page was. Nothing customised and not being edited: the app's own
//! view, untouched. Laid out on the old grid (kept before drag and drop): each block in its place on the 12 × 12 grid.
//! Laid out by drag and drop (studio.rs, drag-drop board 3), or being edited: studio::render.
//!
//! Edit layout (the owner, Nov 24: "editing an app's layout must feel natural, never awkward"; Nov 26, board 3: the
//! same drag and drop in every app): right-click any block for Customize… / Edit layout / Hide this / Describe it…
//! (the menu opens at the pointer), or pick Edit layout in the app's menu (Describe it's box has it too). The window is
//! then edited the board 3 way (studio.rs): nothing is kept until Keep this; Cancel and Esc leave it as it was.
//!
//! Light at rest: nothing runs and nothing is measured per frame; the right-click menu is drawn only while it's open.

use crate::blocks::{self, Placed, COLS, ROWS};
use crate::describe::{outline, DMsg, Describe};
use crate::run;
use crate::studio::{self, SMsg, K};
use crate::theme::Pal;
use crate::ui::{self, t};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::operation::Operation;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{overlay, Clipboard, Shell, Widget};
use iced::widget::text::Wrapping;
use iced::widget::{container, mouse_area, opaque, pin, row, stack, Column, Row, Space};
use iced::{keyboard, mouse, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Task, Theme, Vector};
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

/// A block's own view in its (old grid) look: its font and text size while it's made, and its card when it has one.
fn dressed<'a, M: 'a>(pal: &Pal, pl: &Placed, block: &dyn Fn(&'static str) -> Element<'a, M>) -> Element<'a, M> {
    let el = ui::with_face(pl.look.face(), pl.look.scale(), || block(pl.id));
    let el: Element<'a, M> = if pl.scroll { iced::widget::scrollable(el).width(Length::Fill).height(Length::Fill).into() } else { el };
    match blocks::card_style(pal, &pl.look) {
        None => container(el).width(Length::Fill).height(Length::Fill).into(),
        Some((bg, r, line)) => container(el).width(Length::Fill).height(Length::Fill).padding((r * 0.35).max(2.0)).clip(true).style(ui::boxed(bg, r, line)).into(),
    }
}

// ------------------------------------------------------------------ the right-click menu (not editing)

/// What the right-click menu (and the ways into editing) is told (inside [`DMsg::Edit`]).
#[derive(Debug, Clone)]
pub enum EMsg {
    /// Edit layout from a menu: the page in front
    Start,
    /// Edit layout on this page, with this block picked
    StartAt(&'static str, Option<&'static str>),
    /// Customize…: editing, with this block's side panel open (board 2's)
    Customize(&'static str, &'static str),
    /// the right-click menu closed (Some: opened on that block at its corner)
    Menu(Option<(&'static str, &'static str)>),
    /// the right-click menu on a block, at the pointer: page, block, the pointer in the block, the block's size
    MenuAt(&'static str, &'static str, f32, f32, f32, f32),
    /// Hide this, from the right-click menu (kept, with Undo)
    HideNow(&'static str, &'static str),
    /// Describe it…, from the right-click menu
    Describe,
    /// Undo by "… is hidden.": the last thing kept, undone
    Undo,
    NoteGone(u64),
}

/// The right-click menu's state, kept in [`Describe::ed`].
#[derive(Default)]
pub struct EditState {
    /// the right-click menu that's open (page, block)
    pub menu: Option<(&'static str, &'static str)>,
    /// after Hide this (not editing): the page and what was said, with Undo
    pub note: Option<(&'static str, String)>,
    /// where the right-click menu opens in its block (x, y) and the block's size
    pub menu_at: (f32, f32, f32, f32),
    pub(crate) note_gen: u64,
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

    /// The pages on the screen (made in the last view), innermost first (a page inside another is made first).
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

/// The controller while editing, for an app that reads it itself: "up" "down" "left" "right" move the picked part,
/// "lb"/"lt" make it narrower and "rb"/"rt" wider, "a" picks the next part, "x" hides it, "y" undoes, "b" cancels,
/// "start" keeps it. None when not editing (the press is the app's own); Some when editing took it.
pub fn pad(d: &mut Describe, button: &str) -> Option<Task<DMsg>> {
    d.st.as_ref()?;
    let k = match button.to_ascii_lowercase().as_str() {
        "up" => K::Up,
        "down" => K::Down,
        "left" => K::Left,
        "right" => K::Right,
        "lb" | "lt" | "l1" | "l2" => K::Narrower,
        "rb" | "rt" | "r1" | "r2" => K::Wider,
        "a" => K::Next,
        "x" => K::Delete,
        "y" => K::Undo,
        "b" => K::Esc,
        "start" | "menu" => return Some(d.update(DMsg::Studio(SMsg::Keep))),
        _ => return Some(Task::none()),
    };
    Some(d.update(DMsg::Studio(SMsg::Key(k))))
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
    /// Is the window being edited (Edit layout)?
    pub fn is_editing(&self) -> bool {
        self.st.is_some()
    }

    pub(crate) fn edit(&mut self, m: EMsg) -> Task<DMsg> {
        match m {
            EMsg::Start => self.studio(SMsg::Start(None, None)),
            EMsg::StartAt(page, pick) => self.studio(SMsg::Start(Some(page), pick)),
            EMsg::Customize(page, id) => {
                self.ed.menu = None;
                let t = self.studio(SMsg::Start(Some(page), Some(id)));
                let t2 = self.studio(SMsg::Customize(studio::Sel::Part(page, id)));
                Task::batch([t, t2])
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
            EMsg::HideNow(page, id) => {
                self.ed.menu = None;
                self.studio(SMsg::HideNow(page, id))
            }
            EMsg::Describe => {
                self.ed.menu = None;
                if self.st.as_ref().map(|s| s.s.dirty()).unwrap_or(false) {
                    if let Some(st) = self.st.as_mut() {
                        st.note = "Keep this or Cancel first; then Describe it.".into();
                    }
                    return Task::none();
                }
                if self.st.is_some() {
                    let _ = self.studio(SMsg::Cancel);
                }
                self.update(DMsg::Open)
            }
            EMsg::Undo => {
                self.ed.note = None;
                Task::perform(run::spawn(vec!["hearth-layout".into(), "app-undo".into(), self.app.into()]), DMsg::Kept)
            }
            EMsg::NoteGone(g) => {
                if g == self.ed.note_gen {
                    self.ed.note = None;
                }
                Task::none()
            }
        }
    }

    /// After Hide this was kept: its note goes after 8 s.
    pub(crate) fn note_later(&mut self) -> Task<DMsg> {
        self.ed.note_gen += 1;
        let g = self.ed.note_gen;
        Task::perform(later(8000), move |_| DMsg::Edit(EMsg::NoteGone(g)))
    }
}

/// Round every page: closing the right-click menu (a click anywhere on the page, or Esc) and the right-click itself
/// (at the pointer). At rest it only passes everything on.
type RClickFn<'a, M> = Box<dyn Fn(Point, Size) -> M + 'a>;
type WrapFn<'a, M> = Box<dyn Fn(&'static str, Element<'a, M>) -> Element<'a, M> + 'a>;

struct Watch<'a, M> {
    content: Element<'a, M>,
    esc: Option<M>,
    click: Option<M>,
    rclick: Option<RClickFn<'a, M>>,
}

fn watch<'a, M>(content: Element<'a, M>) -> Watch<'a, M> {
    Watch { content, esc: None, click: None, rclick: None }
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
        if let (Some(f), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))) = (&self.rclick, event) {
            let b = layout.bounds();
            if let Some(pt) = cursor.position_in(b) {
                shell.publish(f(pt, b.size()));
                shell.capture_event();
                return;
            }
        }
        if let (Some(m), Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Escape), .. })) = (&self.esc, event) {
            shell.publish(m.clone());
            shell.capture_event();
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

/// A block with its right-click menu (Customize… / Edit layout / Hide this / Describe it…): the block as it is, and the
/// menu where it was right-clicked while it's open.
fn wrap<'a, M: Clone + 'a>(d: &Describe, p: &Pal, page: &'static str, id: &'static str, el: Element<'a, M>, map: &Rc<dyn Fn(DMsg) -> M + 'a>) -> Element<'a, M> {
    let mut s = stack![el];
    if d.ed.menu == Some((page, id)) {
        let (x, y, w, h) = d.ed.menu_at;
        // at the pointer, kept inside the block (190 × 150: the menu's own size)
        let px = if w > 0.0 { x.min(w - 198.0).max(0.0) } else { 8.0 };
        let py = if h > 0.0 { y.min(h - 158.0).max(0.0) } else { 8.0 };
        s = s.push(pin(block_menu(d, p, map, page, id)).x(px).y(py));
    }
    let m = map.clone();
    let mut w = watch(s.into());
    w.rclick = Some(Box::new(move |pt: Point, sz: Size| m(DMsg::Edit(EMsg::MenuAt(page, id, pt.x, pt.y, sz.width, sz.height)))) as RClickFn<'a, M>);
    Element::new(w)
}

fn block_menu<'a, M: Clone + 'a>(d: &Describe, p: &Pal, map: &Rc<dyn Fn(DMsg) -> M + 'a>, page: &'static str, id: &'static str) -> Element<'a, M> {
    let p = *p;
    let item = |label: &'static str, m: M| -> Element<'a, M> {
        mouse_area(container(t(label, 13.0, p.text).wrapping(Wrapping::None)).padding(Padding { top: 7.0, right: 14.0, bottom: 7.0, left: 12.0 }).width(Length::Fill))
            .on_press(m)
            .interaction(mouse::Interaction::Pointer)
            .into()
    };
    // in the order Menus… left them (studio::PART_MENU)
    let items: Vec<(String, Element<'a, M>)> = vec![
        ("customize".into(), item("Customize…", map(DMsg::Edit(EMsg::Customize(page, id))))),
        ("edit-layout".into(), item("Edit layout", map(DMsg::Edit(EMsg::StartAt(page, Some(id)))))),
        ("hide-this".into(), item("Hide this", map(DMsg::Edit(EMsg::HideNow(page, id))))),
        ("describe-it".into(), item("Describe it…", map(DMsg::Edit(EMsg::Describe)))),
    ];
    let order = d.zone("menu:part").unwrap_or_default();
    let mut rest = items;
    let mut out: Vec<Element<'a, M>> = Vec::new();
    for k in order {
        if let Some(i) = rest.iter().position(|x| x.0 == k) {
            out.push(rest.remove(i).1);
        }
    }
    out.extend(rest.into_iter().map(|x| x.1));
    opaque(container(Column::with_children(out).width(Length::Fixed(190.0))).padding(4.0).style(ui::boxed(p.panel, p.r(10.0), Some((p.line, 1.0)))))
}

/// The page with "… is hidden. Undo · Edit layout" along its bottom after Hide this (gone after 8 s).
fn noted<'a, M: Clone + 'a>(d: &Describe, p: &Pal, page: &'static str, el: Element<'a, M>, map: &Rc<dyn Fn(DMsg) -> M + 'a>) -> Element<'a, M> {
    let mut s = stack![el];
    if let (None, Some((pg, said))) = (&d.st, &d.ed.note) {
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

/// Is this page laid out (or being edited) by drag and drop?
fn studio_on(d: &Describe, page: &str) -> bool {
    d.st.is_some() || d.studio_kept.page(page).is_some()
}

/// The page: the app's own view (`original`) when nothing is customised, else its blocks where they were put; being
/// edited, board 3's editing over it. `block` makes one block's view by its id.
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
    let mp: Rc<dyn Fn(DMsg) -> M + 'a> = Rc::new(map);
    let pal2 = *pal;
    let mp2 = mp.clone();
    let def = d.pages.iter().find(|p| p.id == page);
    let wrapped = move |id: &'static str| -> Element<'a, M> {
        let el = block(id);
        match def.and_then(|x| x.blocks.iter().find(|b| b.id == id)) {
            Some(b) if studio_on(d, page) => studio::part_el(d, &pal2, page, b.id, b.title, b.w >= b.h, el, mp2.clone()),
            _ => el,
        }
    };
    page_inner(d, pal, page, height, original, wrapped, |_: &str| None, mp)
}

/// The page, as the person laid it out or is editing it, or as the app draws it.
#[allow(clippy::too_many_arguments)]
fn page_inner<'a, M: Clone + 'a>(
    d: &'a Describe,
    pal: &Pal,
    page: &'static str,
    height: Option<f32>,
    original: impl FnOnce() -> Element<'a, M>,
    block: impl Fn(&'static str) -> Element<'a, M> + 'a,
    pane: impl Fn(&str) -> Option<Element<'a, M>> + 'a,
    map: Rc<dyn Fn(DMsg) -> M + 'a>,
) -> Element<'a, M> {
    let h = height.map(Length::Fixed).unwrap_or(Length::Fill);
    let el: Element<'a, M> = if studio_on(d, page) {
        studio::render(d, pal, page, h, original, &block, &pane, map.clone())
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
    let mut w = watch(el);
    if menu_here {
        w.esc = Some(map(DMsg::Edit(EMsg::Menu(None))));
        w.click = Some(map(DMsg::Edit(EMsg::Menu(None))));
    }
    Element::new(w)
}

/// A page's blocks as they're already made (each app's view makes its pieces, then puts them together): `original`
/// puts them together the app's own way (exactly as before, when nothing is customised), else each goes in its place.
/// In the app's own way each block still has its right-click menu (Customize… / Edit layout / Hide this / Describe it…).
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
    /// A piece that's there only when it's asked for (one of the app's panes: "pane:<id>"), as it is.
    fn take_opt(&mut self, id: &str) -> Option<Element<'a, M>> {
        self.items.iter_mut().find(|x| x.0 == id).and_then(|x| x.1.take())
    }
}

/// [`page`] for pieces already made: `items` by their block ids, `original` the app's own way of putting them together.
/// Items named "pane:<id>" are the app's panes (studio Kit), placed only where the person dragged them in.
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
    let mp: Rc<dyn Fn(DMsg) -> M + 'a> = Rc::new(map);
    let on = studio_on(d, page_id);
    let editing = d.st.is_some();
    let w: Option<WrapFn<'a, M>> = match d.pages.iter().find(|p| p.id == page_id) {
        Some(def) => {
            let pal = *pal;
            let m = mp.clone();
            Some(Box::new(move |id: &'static str, el: Element<'a, M>| -> Element<'a, M> {
                match def.blocks.iter().find(|b| b.id == id) {
                    Some(b) if on => {
                        let across = d.geo.try_borrow().ok().and_then(|g| g.part(page_id, b.id)).map(|r| r.across()).unwrap_or(b.w >= b.h);
                        let el = studio::part_el(d, &pal, page_id, b.id, b.title, across, el, m.clone());
                        if editing { el } else { wrap(d, &pal, page_id, b.id, el, &m) }
                    }
                    Some(b) => wrap(d, &pal, page_id, b.id, el, &m),
                    None => el,
                }
            }) as WrapFn<'a, M>)
        }
        None => None,
    };
    let cell = Rc::new(RefCell::new(Parts { items: items.into_iter().map(|(i, e)| (i, Some(e))).collect(), wrap: w }));
    let c2 = cell.clone();
    let c3 = cell.clone();
    page_inner(
        d,
        pal,
        page_id,
        height,
        move || {
            let mut ps = cell.borrow_mut();
            original(&mut ps)
        },
        move |id| c2.borrow_mut().take(id),
        move |id: &str| c3.borrow_mut().take_opt(id),
        mp,
    )
}

// ------------------------------------------------------------------ Edit layout, right there
// The owner: editing an app's layout must feel natural, not buried behind Describe it › Move and resize blocks. A
// right-click on any part opens editing in place with that part picked, an app's own Edit layout entry does the same
// for its page, and Esc / Ctrl+Z work while editing.

/// Edit layout right there: editing opens on `page` in place (the live app stays), with `pick` picked. Already
/// editing, only the pick changes, so nothing done is lost.
pub fn edit_at(d: &mut Describe, page: &'static str, pick: Option<&'static str>) -> iced::Task<DMsg> {
    d.update(DMsg::Studio(SMsg::Start(Some(page), pick)))
}

/// A part that a right-click edits (`msg`, the app's own message for [`edit_at`]); it looks and lays out as before
/// (a press the part itself takes, its own right-click menu, stays the part's).
pub fn right_edit<'a, M: Clone + 'a>(el: Element<'a, M>, msg: M) -> Element<'a, M> {
    mouse_area(el).on_right_press(msg).into()
}

/// A key the app hears while its window is edited. Editing takes its own keys on the window itself (Esc cancels,
/// Ctrl+Z undoes, Ctrl+Shift+Z or Ctrl+Y redoes), so for the app this is always None: the key isn't the app's.
pub fn arrange_key(d: &Describe, key: &iced::keyboard::Key, mods: iced::keyboard::Modifiers) -> Option<DMsg> {
    let _ = (key, mods);
    d.st.as_ref()?;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_controller_is_editings_while_editing() {
        const PAGES: &[blocks::PageDef] = &[blocks::PageDef { id: "main", title: "Main", blocks: &[blocks::bl("side", "Side", 0, 0, 2, 12), blocks::bl("list", "List", 2, 0, 10, 12)] }];
        let mut d = Describe::new("ideas", "Hearth Ideas").with_pages(PAGES);
        assert!(pad(&mut d, "a").is_none(), "not editing: the app's own");
        let _ = d.update(edit_page("main"));
        assert!(d.is_editing());
        assert!(pad(&mut d, "a").is_some());
        assert_eq!(d.st.as_ref().unwrap().pick, Some(studio::Sel::Part("main", "side")), "A picks the first part");
        let _ = pad(&mut d, "b");
        assert!(!d.is_editing(), "B cancels: nothing kept");
    }
}
