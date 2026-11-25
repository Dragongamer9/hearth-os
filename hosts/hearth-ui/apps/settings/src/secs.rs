//! Every Settings page made of its parts (the owner, Nov 24: "Modular for all Hearth apps is required as well"): a
//! page's sections, each heading (widgets::head) and what's under it, are its blocks (hearth_ui::blocks), moved, sized,
//! hidden, added back and restyled in Arrange, kept per page in ~/.config/nimbus/apps/settings.json by hearth-layout
//! (settings_file: atomic, .bak). The frame (frame.rs, "window") and Home (cats/home.rs) name theirs themselves.
//!
//! A page's column is a [`Secs`] (pushed to like a Column). Nothing customised: exactly the column it always was, the
//! same children with the same spacing (each section a column of its own inside it, which draws the same). Laid out:
//! each section where it was put. A page's sections become its blocks the first time it's drawn ([`sync`], at the next
//! message; worked out once, never per frame): a section's id is its heading in plain words ("Volume" → volume; "Mouse
//! · saving…" is still mouse), the lines before the first heading are "top". At most 12 a page (the grid's rows):
//! any after those stay with the 12th.

use crate::App;
use hearth_ui::blocks::{bls, BlockDef, PageDef};
use hearth_ui::describe::{DMsg, Describe};
use hearth_ui::Pal;
use iced::widget::Column;
use iced::Element;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    /// The heading just made (widgets::head), for the next Secs::push to start a section with.
    static HEAD: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The sections each page showed when it was last drawn: (page, [(id, title)]); read by sync().
    static SEEN: RefCell<Vec<(&'static str, Vec<(String, String)>)>> = const { RefCell::new(Vec::new()) };
    /// The pages whose blocks were drawn in this frame (a page lays its blocks out once).
    static DRAWN: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// A heading was made (widgets::head): the next part pushed to a page starts a section.
pub fn heading(title: &str) {
    HEAD.with(|h| *h.borrow_mut() = Some(title.to_string()));
}

/// A new frame is being drawn (frame.rs).
pub fn begin() {
    DRAWN.with(|d| d.borrow_mut().clear());
    HEAD.with(|h| h.borrow_mut().take());
}

/// A heading as its block's title: the words before " · " ("Mouse · saving…" is Mouse).
pub fn clean(t: &str) -> String {
    t.split(" · ").next().unwrap_or("").trim_end_matches('…').trim().to_string()
}

/// A block's id from its title: lowercase letters and digits with dashes ("Play sound through" → play-sound-through).
pub fn slug(t: &str) -> String {
    let mut s = String::new();
    for ch in t.chars() {
        if ch.is_ascii_alphanumeric() {
            s.push(ch.to_ascii_lowercase());
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
        if s.len() >= 40 {
            break;
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "part".into() } else { s }
}

/// The sections' ids and titles, in order, each id once ("top" for the lines before the first heading).
pub fn named(titles: &[String]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for t in titles {
        let (mut id, title) = if t.is_empty() { ("top".to_string(), "The top of the page".to_string()) } else { let c = clean(t); (slug(&c), c) };
        if out.iter().any(|(i, _)| *i == id) {
            let mut n = 2;
            while out.iter().any(|(i, _)| *i == format!("{id}-{n}")) {
                n += 1;
            }
            id = format!("{id}-{n}");
        }
        out.push((id, title));
    }
    out
}

/// Where a page's n blocks go by default: one under another, the 12 rows shared out (the first ones a row more).
pub fn rows(n: usize) -> Vec<(u8, u8)> {
    let n = n.clamp(1, 12);
    let (base, extra) = (12 / n, 12 % n);
    let mut y = 0usize;
    (0..n)
        .map(|i| {
            let h = base + usize::from(i < extra);
            let r = (y as u8, h as u8);
            y += h;
            r
        })
        .collect()
}

/// A page's column, made of its sections (see the module's notes). Pushed to like a Column; `.into()` draws it.
pub struct Secs<'a, M> {
    d: &'a Describe,
    pal: Pal,
    page: &'static str,
    spacing: f32,
    /// `hearth-settings-app --page PAGE:from:N` (CI's pictures of a long page's lower part): from the Nth line, as
    /// it was (no blocks)
    skip: usize,
    secs: Vec<(String, Vec<Element<'a, M>>)>,
}

impl<'a, M: 'a> Secs<'a, M> {
    pub fn on(a: &'a App, skip: usize) -> Self {
        HEAD.with(|h| h.borrow_mut().take());
        Secs { d: &a.describe, pal: a.sh.p, page: a.page, spacing: 10.0, skip, secs: Vec::new() }
    }
    pub fn spacing(mut self, s: f32) -> Self {
        self.spacing = s;
        self
    }
    pub fn push(mut self, e: impl Into<Element<'a, M>>) -> Self {
        let e = e.into();
        match HEAD.with(|h| h.borrow_mut().take()) {
            Some(t) => self.secs.push((t, vec![e])),
            None => match self.secs.last_mut() {
                Some(s) => s.1.push(e),
                None => self.secs.push((String::new(), vec![e])),
            },
        }
        self
    }
}

impl<'a, M: Clone + From<DMsg> + 'a> From<Secs<'a, M>> for Element<'a, M> {
    fn from(s: Secs<'a, M>) -> Element<'a, M> {
        let Secs { d, pal, page, spacing, skip, secs } = s;
        let flat = move |secs: Vec<(String, Vec<Element<'a, M>>)>| -> Element<'a, M> {
            let all: Vec<Element<'a, M>> = secs.into_iter().flat_map(|x| x.1).collect();
            let skip = skip.min(all.len().saturating_sub(1));
            Column::with_children(all.into_iter().skip(skip)).spacing(spacing).into()
        };
        if skip > 0 || secs.is_empty() {
            return flat(secs);
        }
        let first = DRAWN.with(|v| {
            let mut v = v.borrow_mut();
            if v.contains(&page) {
                false
            } else {
                v.push(page);
                true
            }
        });
        if !first {
            return flat(secs);
        }
        let titles: Vec<String> = secs.iter().map(|x| x.0.clone()).collect();
        let named = named(&titles);
        SEEN.with(|v| {
            let mut v = v.borrow_mut();
            v.retain(|x| x.0 != page);
            v.push((page, named.clone()));
        });
        let Some(def) = d.pages.iter().find(|p| p.id == page) else { return flat(secs) };
        let Some(first_block) = def.blocks.first() else { return flat(secs) };
        // each section to its block; one the page doesn't name (yet) stays with the one before
        let mut groups: Vec<(&'static str, Vec<Element<'a, M>>)> = Vec::new();
        let mut n = 0usize;
        for ((id, _), (_, els)) in named.iter().zip(secs) {
            n += els.len();
            let b = def.blocks.iter().find(|b| b.id == id.as_str() && !groups.iter().any(|g| g.0 == b.id));
            match b {
                Some(b) => groups.push((b.id, els)),
                None => match groups.last_mut() {
                    Some(g) => g.1.extend(els),
                    None => groups.push((first_block.id, els)),
                },
            }
        }
        let ids: Vec<&'static str> = groups.iter().map(|g| g.0).collect();
        let items: Vec<(&'static str, Element<'a, M>)> = groups
            .into_iter()
            .map(|(id, els)| {
                let el: Element<'a, M> = Column::with_children(els).spacing(spacing).into();
                (id, hearth_ui::arrange::right_edit(el, M::from(DMsg::Pick(id))))
            })
            .collect();
        let h = (n as f32 * 56.0).clamp(420.0, 4200.0);
        hearth_ui::arrange::parts_in(
            d,
            &pal,
            page,
            Some(h),
            items,
            move |ps| {
                let mut c = Column::new().spacing(spacing);
                for id in ids {
                    c = c.push(ps.take(id));
                }
                c.into()
            },
            |dm| M::from(dm),
        )
    }
}

static OPENED: AtomicBool = AtomicBool::new(false);

/// The page's sections as its blocks, from when it was last drawn (once per new section, never per frame): its
/// PageDef made or grown, and Describe told (it lays the page out again from what's kept for it).
pub fn sync(a: &mut App) {
    let page = a.page;
    let Some(seen) = SEEN.with(|v| v.borrow().iter().find(|x| x.0 == page).map(|x| x.1.clone())) else { return };
    // Home and the window name their own blocks
    if crate::STATIC_BLOCKS.iter().any(|p| p.id == page) {
        return;
    }
    let mut list: Vec<(String, String)> = a.describe.pages.iter().find(|p| p.id == page).map(|p| p.blocks.iter().map(|b| (b.id.to_string(), b.title.to_string())).collect()).unwrap_or_default();
    let mut grew = false;
    for (id, t) in seen {
        if list.len() >= 12 {
            break;
        }
        if !list.iter().any(|x| x.0 == id) {
            list.push((id, t));
            grew = true;
        }
    }
    if grew {
        let places = rows(list.len());
        let blocks: Vec<BlockDef> = list
            .into_iter()
            .zip(places)
            .map(|((id, t), (y, h))| {
                let id: &'static str = Box::leak(id.into_boxed_str());
                let t: &'static str = Box::leak(t.into_boxed_str());
                bls(id, t, 0, y, 12, h)
            })
            .collect();
        let title = crate::registry::page(page).map(|p| p.label).unwrap_or(page);
        let def = PageDef { id: page, title, blocks: Box::leak(blocks.into_boxed_slice()) };
        let mut all: Vec<PageDef> = a.describe.pages.iter().filter(|p| p.id != page).copied().collect();
        all.push(def);
        a.describe.pages = Box::leak(all.into_boxed_slice());
        // worked out again with what's kept for it
        let _ = a.describe.update(DMsg::Drop);
    }
    // CI's pictures: HEARTH_ARRANGE_OPEN=settings:<page> opens Arrange on it once it's drawn
    if !OPENED.load(Ordering::Relaxed) {
        if let Ok(v) = std::env::var("HEARTH_ARRANGE_OPEN") {
            if v == format!("settings:{page}") && a.describe.arr.is_none() {
                OPENED.store(true, Ordering::Relaxed);
                let pick = std::env::var("HEARTH_ARRANGE_PICK").ok().and_then(|b| a.describe.pages.iter().find(|p| p.id == page).and_then(|p| p.blocks.iter().find(|x| x.id == b)).map(|x| x.id));
                let _ = hearth_ui::arrange::edit_at(&mut a.describe, page, pick);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sections_are_named_by_their_headings() {
        let n = named(&["".into(), "Volume".into(), "Mouse · saving…".into(), "Volume".into(), "Wi-Fi & more".into(), "Ça".into()]);
        let ids: Vec<&str> = n.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(ids, vec!["top", "volume", "mouse", "volume-2", "wi-fi-more", "a"]);
        assert_eq!(n[2].1, "Mouse");
        assert_eq!(slug("!!!"), "part");
    }
    #[test]
    fn the_rows_are_shared_out() {
        assert_eq!(rows(1), vec![(0, 12)]);
        assert_eq!(rows(5), vec![(0, 3), (3, 3), (6, 2), (8, 2), (10, 2)]);
        assert_eq!(rows(30).len(), 12);
        let r = rows(7);
        assert_eq!(r.last().map(|x| x.0 + x.1), Some(12));
    }
}
