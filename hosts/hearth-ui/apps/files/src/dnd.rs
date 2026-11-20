//! Drag and drop in Hearth Files (FileManager.qml's startDrag / dropOn): drag what's picked onto a folder, a tab, a
//! step of the address, a place in the left menu, the split view or a Briefcase, and let go: moved (copied onto
//! another drive, or with Ctrl held; Ctrl+Shift makes links there; onto the Trash: moved to the Trash). Files dropped
//! from other windows are copied in (app.rs, the window's FileDropped).
//!
//! iced draws the drag itself here: a press that moves more than a few pixels picks the items up, the place under the
//! pointer lights up, letting go drops them. Nothing runs while nothing is dragged.

use crate::app::{App, Msg};
use crate::backend;
use crate::icons::{ico, Ic};
use crate::model::{self, base, parent_of};
use hearth_ui::ui::{self, t1};
use iced::widget::{column, container, row};
use iced::{Element, Point, Task};

#[derive(Debug, Clone)]
pub struct Pending {
    pub paths: Vec<String>,
    pub label: String,
    pub start: Point,
    pub from_split: bool,
}

#[derive(Debug, Clone)]
pub struct Drag {
    pub paths: Vec<String>,
    pub label: String,
    pub at: Point,
    pub from_split: bool,
    /// What's under the pointer: an item (a folder, a place, a tab, an address step), and the area it's in (the
    /// folder shown, the split view).
    pub item: Option<String>,
    pub area: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DMsg {
    /// 0: an item, 1: an area; the folder it stands for.
    Over(u8, String),
    Out(u8, String),
    Release,
}

/// A press on something that can be dragged: it's picked up once the pointer moves.
pub fn pending(a: &mut App, paths: Vec<String>, name: &str, from_split: bool) {
    if paths.is_empty() || a.overlay.is_some() || a.picking() {
        return;
    }
    let label = if paths.len() == 1 { name.to_string() } else { model::plural(paths.len(), "item", "items") };
    a.drag_pending = Some(Pending { paths, label, start: a.cursor, from_split });
}

/// The pointer moved (the window's on_move).
pub fn moved(a: &mut App, p: Point) {
    if let Some(d) = a.drag.as_mut() {
        d.at = p;
        return;
    }
    if let Some(pd) = &a.drag_pending {
        let (dx, dy) = (p.x - pd.start.x, p.y - pd.start.y);
        if dx * dx + dy * dy > 36.0 {
            let pd = a.drag_pending.take().unwrap();
            a.drag = Some(Drag { paths: pd.paths, label: pd.label, at: p, from_split: pd.from_split, item: None, area: None });
        }
    }
}

/// Where it would go now.
pub fn target(a: &App) -> Option<String> {
    let d = a.drag.as_ref()?;
    d.item.clone().or_else(|| d.area.clone())
}

pub fn update(a: &mut App, m: DMsg) -> Task<Msg> {
    match m {
        DMsg::Over(k, p) => {
            if let Some(d) = a.drag.as_mut() {
                if k == 0 { d.item = Some(p) } else { d.area = Some(p) }
            }
        }
        DMsg::Out(k, p) => {
            if let Some(d) = a.drag.as_mut() {
                let slot = if k == 0 { &mut d.item } else { &mut d.area };
                if slot.as_deref() == Some(p.as_str()) {
                    *slot = None;
                }
            }
        }
        DMsg::Release => {
            a.drag_pending = None;
            let Some(d) = a.drag.take() else { return Task::none() };
            let Some(dest) = d.item.clone().or(d.area.clone()) else { return Task::none() };
            return drop_on(a, d.paths, &dest);
        }
    }
    Task::none()
}

/// What letting go does (FileManager.qml's dropOn).
pub fn drop_on(a: &mut App, paths: Vec<String>, dest: &str) -> Task<Msg> {
    let paths: Vec<String> = paths.into_iter().filter(|p| !p.is_empty() && p != dest && !model::under(dest, p)).collect();
    if paths.is_empty() || dest.is_empty() || !dest.starts_with('/') {
        return Task::none();
    }
    // Into Hearth (C:)'s view folders (Program Files, C:\ …), Windows or ProgramData: refused, saying why.
    let why = crate::drive::c_refusal(dest, &a.ctx());
    if a.cdrive_on() && !why.is_empty() {
        return a.say(&why, true);
    }
    // Into a Briefcase: added (or moved from another one).
    if let Some(t) = crate::briefcase::drop(a, &paths, dest) {
        return t;
    }
    let (ctrl, shift) = (a.mods.control(), a.mods.shift());
    let trash = a.trash_path();
    if ctrl && shift && dest != trash {
        let mut args = vec!["link".to_string()];
        args.extend(paths.iter().cloned());
        args.push(dest.to_string());
        let n = paths.len();
        let first = base(&paths[0]);
        let msg = if n == 1 { format!("Made a link to “{first}”") } else { format!("Made {n} links") };
        return Task::perform(backend::call(args, 30), move |r| Msg::Did(crate::app::Then::Linked(msg.clone()), r));
    }
    if dest == trash {
        return a.trash_paths(paths);
    }
    let copy = ctrl || a.root_of(&paths[0]).0 != a.root_of(dest).0;
    let list: Vec<String> = if copy { paths } else { paths.into_iter().filter(|p| parent_of(p) != dest).collect() };
    if list.is_empty() {
        return Task::none();
    }
    a.transfer_to(if copy { "copy" } else { "move" }, list, dest.to_string())
}

/// What's being dragged, beside the pointer: what it is and what letting go would do.
pub fn ghost(a: &App) -> Option<(Point, Element<'_, Msg>)> {
    let d = a.drag.as_ref()?;
    let p = a.p;
    let dest = target(a);
    let what = match &dest {
        Some(t) => {
            let (ctrl, shift) = (a.mods.control(), a.mods.shift());
            let name = if *t == a.trash_path() { "the Trash".to_string() } else { format!("“{}”", a.title_of(t)) };
            if *t == a.trash_path() {
                format!("Move to {name}")
            } else if crate::briefcase::briefcase_at(a, t).is_some() {
                format!("Add to {name}")
            } else if ctrl && shift {
                format!("Link in {name}")
            } else if ctrl || d.paths.first().map(|x| a.root_of(x).0 != a.root_of(t).0).unwrap_or(false) {
                format!("Copy to {name}")
            } else {
                format!("Move to {name}")
            }
        }
        None => "Drop it on a folder".to_string(),
    };
    let icon = if d.paths.len() == 1 && d.paths.first().map(|x| std::path::Path::new(x).is_dir()).unwrap_or(false) { Ic::Folder } else { Ic::File };
    let card = container(
        row![ico::<Msg>(icon, p.text, 22.0), column![t1(ui::fit(&d.label, 34), 13.0, p.text).font(ui::weight(iced::font::Weight::Semibold)), t1(ui::fit(&what, 40), 12.0, if dest.is_some() { p.accent } else { p.muted })].spacing(1.0)]
            .spacing(10.0)
            .align_y(iced::Center),
    )
    .padding(iced::Padding { top: 7.0, right: 12.0, bottom: 7.0, left: 10.0 })
    .style(ui::boxed(p.panel, p.r(8.0), Some((p.accent, 1.0))));
    Some((Point::new(d.at.x + 16.0, d.at.y + 14.0), card.into()))
}
