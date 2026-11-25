//! A page made of blocks, drawn (blocks.rs has the rules; describe.rs the state): [`page`] is what every Hearth app
//! puts where its page was. Nothing customised and not being arranged: the app's own view, untouched. Laid out: each
//! block in its place on the 12 × 12 grid, in its look. Arranging (build-anything board 4, for an app): the grid, each
//! block with its bar (drag it to move the block, ✕ hides it) and its corner (drag it to size the block), the picked
//! block outlined, and over the page Arrange's own bar: the hidden blocks to add back, the picked block's Look (corners,
//! see-through, font, text size, accent), Undo, Back to the original, Done and Keep this.

use crate::blocks::{self, Grip, LookPart, Placed, COLS, ROWS};
use crate::describe::{outline, Arrange, DMsg, Describe};
use crate::theme::Pal;
use crate::ui::{self, t, ts};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, stack, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Rectangle, Renderer, Theme};

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

/// The page: the app's own view (`original`) when nothing is customised, else its blocks where they were put; with
/// Arrange's grid and grips while it's being arranged. `block` makes one block's view by its id.
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

/// [`page`] for a page that sits in something that scrolls (Settings' pages): laid out or arranged, its blocks take
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
    if let Some(a) = d.arr.as_ref().filter(|a| a.page == page) {
        return editor(d, a, pal, &block, map, h);
    }
    let Some(placed) = d.laid(page) else { return original() };
    let mut layers: Vec<Element<'a, M>> = Vec::with_capacity(placed.len());
    for pl in placed.iter().filter(|p| !p.hidden) {
        let el = dressed(pal, pl, &block);
        layers.push(place(pl, outline(pal, d.block_changing(page, pl.id), el)));
    }
    stack(layers).width(Length::Fill).height(h).into()
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

fn chip<'a, M: Clone + 'a>(p: &Pal, label: String, on: bool, msg: M) -> Element<'a, M> {
    let p = *p;
    mouse_area(
        container(t(label, 12.0, if on { p.accent } else { p.text }).wrapping(iced::widget::text::Wrapping::None))
            .padding(Padding { top: 4.0, right: 10.0, bottom: 4.0, left: 10.0 })
            .style(ui::boxed(if on { Pal::a(p.accent, 0.10) } else { Color::TRANSPARENT }, 12.0, Some((if on { p.accent } else { p.line }, 1.0)))),
    )
    .on_press(msg)
    .interaction(mouse::Interaction::Pointer)
    .into()
}

/// Arranging: the grid, the blocks with their bars and corners, Arrange's bar over the page.
fn editor<'a, M: Clone + 'a>(d: &'a Describe, a: &'a Arrange, pal: &Pal, block: &dyn Fn(&'static str) -> Element<'a, M>, map: impl Fn(DMsg) -> M + Clone + 'a, h: Length) -> Element<'a, M> {
    let p = *pal;
    let mut layers: Vec<Element<'a, M>> = vec![canvas::Canvas::new(GridLines(Pal::a(p.text, 0.07))).width(Length::Fill).height(Length::Fill).into()];
    for pl in a.placed.iter().filter(|x| !x.hidden) {
        let picked = a.pick == Some(pl.id);
        let content = dressed(&p, pl, block);
        let bar_bg = if picked { p.accent } else { Pal::a(p.panel3, 0.96) };
        let fg = if picked { p.ink } else { p.text };
        let grip = mouse_area(
            container(row![t("⠿", 14.0, fg), ts(pl.title, 12.0, fg).wrapping(iced::widget::text::Wrapping::None), Space::new().width(Length::Fill)].spacing(8.0).align_y(iced::Center))
                .padding(Padding { top: 3.0, right: 6.0, bottom: 3.0, left: 8.0 })
                .width(Length::Fill),
        )
        .on_press(map(DMsg::Grab(pl.id, Grip::Move)))
        .interaction(mouse::Interaction::Grab);
        let hide = mouse_area(container(t("✕", 12.0, fg)).padding(Padding { top: 3.0, right: 9.0, bottom: 3.0, left: 6.0 })).on_press(map(DMsg::Hide(pl.id))).interaction(mouse::Interaction::Pointer);
        let strip = container(row![grip, hide].align_y(iced::Center)).width(Length::Fill).style(ui::boxed(bar_bg, 0.0, None));
        let handle = mouse_area(container(Space::new().width(Length::Fixed(14.0)).height(Length::Fixed(14.0))).style(ui::boxed(p.accent, 3.0, Some((p.ink, 1.0)))))
            .on_press(map(DMsg::Grab(pl.id, Grip::Size)))
            .interaction(mouse::Interaction::ResizingDiagonallyDown);
        let chrome = column![strip, Space::new().height(Length::Fill), row![Space::new().width(Length::Fill), handle]].width(Length::Fill).height(Length::Fill);
        let framed = container(stack![content, chrome]).width(Length::Fill).height(Length::Fill).style(ui::boxed(Color::TRANSPARENT, 0.0, Some((if picked { p.accent } else { Pal::a(p.accent, 0.45) }, if picked { 2.0 } else { 1.0 }))));
        layers.push(place(pl, outline(&p, picked, framed.into())));
    }
    if a.grab.is_some() {
        // while a block is held: the cell under the pointer, so it follows on the grid
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
    let board = mouse_area(container(stack(layers).width(Length::Fill).height(Length::Fill)).width(Length::Fill).height(h).style(ui::boxed(Pal::a(p.bg2, 0.6), 0.0, None)))
        .on_release(map(DMsg::Drop));
    column![bar(d, a, &p, map), board].spacing(8.0).width(Length::Fill).into()
}

/// Arrange's bar: which page, the hidden blocks to add back, the picked block's Look, and Undo / Back to the original /
/// Done / Keep this.
fn bar<'a, M: Clone + 'a>(d: &'a Describe, a: &'a Arrange, p: &Pal, map: impl Fn(DMsg) -> M + Clone + 'a) -> Element<'a, M> {
    let p = *p;
    let title = d.pages.iter().find(|x| x.id == a.page).map(|x| x.title).unwrap_or("");
    let mut top = Row::new().spacing(10.0).align_y(iced::Center).push(crate::describe::editing::<M>(&p)).push(ts(format!("{} · {title}", d.name), 14.0, p.text).wrapping(iced::widget::text::Wrapping::None));
    if d.pages.len() > 1 {
        for pg in d.pages {
            top = top.push(chip(&p, pg.title.to_string(), pg.id == a.page, map(DMsg::Arrange(Some(pg.id)))));
        }
    }
    let buttons = row![
        ui::btn(&p, "Undo", ui::Kind::Normal, false, Some(map(DMsg::ArrUndo))),
        ui::btn(&p, "Back to the original", ui::Kind::Normal, false, Some(map(DMsg::ArrReset))),
        ui::btn(&p, "Done", ui::Kind::Normal, false, Some(map(DMsg::Arrange(None)))),
        ui::btn(&p, "Keep this", ui::Kind::Primary, false, Some(map(DMsg::ArrKeep))),
    ]
    .spacing(10.0)
    .align_y(iced::Center);

    let mut add = Row::new().spacing(8.0).align_y(iced::Center).push(t("Drag a block by its bar to move it, by its corner to size it; ✕ hides it.", 12.0, p.muted));
    let hidden: Vec<&Placed> = a.placed.iter().filter(|x| x.hidden).collect();
    if !hidden.is_empty() {
        add = add.push(container(t("Add back:", 12.0, p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 8.0 }));
        for h in hidden {
            add = add.push(chip(&p, format!("+ {}", h.title), false, map(DMsg::Show(h.id))));
        }
    }
    let mut body = column![top.wrap(), add.wrap()].spacing(10.0);
    match a.pick.and_then(|id| a.placed.iter().find(|x| x.id == id)) {
        Some(pl) => {
            let mut look = Row::new().spacing(8.0).align_y(iced::Center).push(ts(format!("Look of {}:", pl.title), 12.0, p.text));
            for part in [LookPart::Corners, LookPart::See, LookPart::Font, LookPart::Size, LookPart::Accent] {
                let changed = match part {
                    LookPart::Corners => pl.look.corners.is_some(),
                    LookPart::See => pl.look.see > 0,
                    LookPart::Font => !pl.look.font.is_empty(),
                    LookPart::Size => !pl.look.size.is_empty(),
                    LookPart::Accent => !pl.look.accent.is_empty(),
                };
                look = look.push(chip(&p, blocks::look_label(&pl.look, part), changed, map(DMsg::Look(part))));
            }
            if let Some(c) = pl.look.accent_colour() {
                look = look.push(container(Space::new().width(Length::Fixed(14.0)).height(Length::Fixed(14.0))).style(ui::boxed(c, 7.0, None)));
            }
            body = body.push(look.wrap());
        }
        None => body = body.push(t("Pick a block (press its bar) to change its look.", 12.0, p.muted)),
    }
    let status: Element<'a, M> = if !d.error.is_empty() {
        t(d.error.clone(), 12.0, p.danger).into()
    } else if !d.said.is_empty() {
        t(d.said.clone(), 12.0, p.good).into()
    } else {
        Space::new().into()
    };
    body = body.push(row![container(status).width(Length::Fill), buttons].spacing(12.0).align_y(iced::Center));
    container(body).padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 16.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.accent, 2.0)))).into()
}

/// A page's blocks as they're already made (each app's view makes its pieces, then puts them together): `original`
/// puts them together the app's own way (exactly as before, when nothing is customised), else each goes in its place.
pub struct Parts<'a, M> {
    items: Vec<(&'static str, Option<Element<'a, M>>)>,
}

impl<'a, M: 'a> Parts<'a, M> {
    /// The piece by its id (taken once; nothing when it's gone already).
    pub fn take(&mut self, id: &str) -> Element<'a, M> {
        self.items.iter_mut().find(|(i, _)| *i == id).and_then(|(_, e)| e.take()).unwrap_or_else(|| Space::new().into())
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
    let cell = std::rc::Rc::new(std::cell::RefCell::new(Parts { items: items.into_iter().map(|(i, e)| (i, Some(e))).collect() }));
    let c2 = cell.clone();
    page_in(d, pal, page_id, height, move || original(&mut cell.borrow_mut()), move |id| c2.borrow_mut().take(id), map)
}

// ------------------------------------------------------------------ Edit layout, right there (additive, Nov 24)
// The owner: editing an app's layout must feel natural, not buried behind Describe it › Move and resize blocks. These
// are what an app needs for that with Arrange as it is: a right-click on any part opens Arrange in place with that part
// picked, an app's own Edit layout entry does the same for its page, and Esc / Ctrl+Z work while arranging.

/// Edit layout right there: Arrange opens on `page` in place (the live app stays), with `pick` picked. Already
/// arranging that page, only the pick changes, so nothing done is lost.
pub fn edit_at(d: &mut Describe, page: &'static str, pick: Option<&'static str>) -> iced::Task<DMsg> {
    let mut t = iced::Task::none();
    if d.arr.as_ref().map(|a| a.page) != Some(page) {
        t = d.update(DMsg::Arrange(Some(page)));
    }
    if let Some(id) = pick {
        let _ = d.update(DMsg::Pick(id));
    }
    t
}

/// A part that a right-click edits (`msg`, the app's own message for [`edit_at`]); it looks and lays out as before
/// (a press the part itself takes, its own right-click menu, stays the part's).
pub fn right_edit<'a, M: Clone + 'a>(el: Element<'a, M>, msg: M) -> Element<'a, M> {
    mouse_area(el).on_right_press(msg).into()
}

/// A key while a page is being arranged: Esc keeps what you did and leaves (Keep this), Ctrl+Z undoes. None when
/// nothing is being arranged, or for any other key (the app's own).
pub fn arrange_key(d: &Describe, key: &iced::keyboard::Key, mods: iced::keyboard::Modifiers) -> Option<DMsg> {
    use iced::keyboard::{key::Named, Key};
    d.arr.as_ref()?;
    match key.as_ref() {
        Key::Named(Named::Escape) => Some(DMsg::ArrKeep),
        Key::Character(c) if (mods.control() || mods.command()) && c.eq_ignore_ascii_case("z") => Some(DMsg::ArrUndo),
        _ => None,
    }
}
