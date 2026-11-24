//! Describe it in Hearth Files (desktop-layouts board 11): what Files' own sources count (hearth-layout app.rs FILES:
//! folder-pictures, folder-old…), the features' buttons on the bar (in Downloads and in every folder), and what they
//! do when pressed (Files' own actions: Sort my Downloads, select the pictures here…). The box, the chips and Keep this
//! are hearth_ui::describe's; the layout knobs are read where Files draws itself (view.rs, app.rs shown_view).

use crate::app::{App, Msg};
use crate::model::{self, Entry};
use hearth_ui::describe;
use hearth_ui::ui::{self, t};
use iced::widget::{container, mouse_area, Row};
use iced::{mouse, Color, Element, Padding, Task};
use serde_json::Value;

const MONTH: i64 = 30 * 24 * 3600;
const GB: u64 = 1 << 30;

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
fn picture(e: &Entry) -> bool {
    !e.dir && model::kind_of(e) == "image"
}
fn video(e: &Entry) -> bool {
    !e.dir && e.mime.starts_with("video/")
}
fn old(e: &Entry) -> bool {
    !e.dir && e.mtime > 0 && now() - e.mtime > MONTH
}
fn big(e: &Entry) -> bool {
    !e.dir && e.size > GB
}

/// What one of Files' sources says about the folder you're in.
pub fn value_of(a: &App, src: &str) -> String {
    let es = || a.entries.iter().filter(|e| !e.name.starts_with('.'));
    match src {
        "folder-pictures" => es().filter(|e| picture(e)).count().to_string(),
        "folder-videos" => es().filter(|e| video(e)).count().to_string(),
        "folder-old" => es().filter(|e| old(e)).count().to_string(),
        "folder-big" => es().filter(|e| big(e)).count().to_string(),
        "folder-count" => es().count().to_string(),
        "folder-size" => model::fmt_size(es().filter(|e| !e.dir).map(|e| e.size).sum()),
        "disk-free" | "time" | "date" => String::new(),
        _ => String::new(),
    }
}

/// The features' buttons for where you are (Downloads gets its own and every folder's), with NEW by the ones being
/// tried; nothing on Home, This PC and the other special pages.
pub fn buttons(a: &App) -> Option<Element<'_, Msg>> {
    if a.picking() || a.on_special() || a.on_home() {
        return None;
    }
    let mut fs = a.describe.features("folder");
    if crate::sortdl::here(a) {
        fs = [a.describe.features("downloads"), fs].concat();
    }
    if fs.is_empty() {
        return None;
    }
    let p = a.p;
    let mut r = Row::new().spacing(8.0).align_y(iced::Center);
    let mut any_new = false;
    for (f, new) in fs {
        any_new |= new;
        let label = describe::feature_text(&f, &|s| value_of(a, s));
        let act = f.pointer("/click/action").and_then(Value::as_str).unwrap_or("none").to_string();
        let bx = container(t(label, 14.0, p.text).wrapping(iced::widget::text::Wrapping::None))
            .padding(Padding { top: 6.0, right: 14.0, bottom: 6.0, left: 14.0 })
            .style(ui::boxed(Color::TRANSPARENT, 8.0, Some((p.line, 1.0))));
        let el: Element<'_, Msg> = if act == "none" { bx.into() } else { mouse_area(bx).on_press(Msg::DescAct(act)).interaction(mouse::Interaction::Pointer).into() };
        r = r.push(el);
    }
    if any_new {
        r = r.push(describe::new_chip(&p));
    }
    Some(r.into())
}

/// A menu as Describe it left it: the ids its zone "menu:<kind>" names first, in that order ("sep" a line), then the
/// rest as they were; ids in "hidden" left out; features for "menu-<kind>" added at the end as items.
pub fn arrange_menu(a: &App, kind: &str, items: Vec<crate::app::MenuItem>) -> Vec<crate::app::MenuItem> {
    let hidden = a.describe.zone("hidden").unwrap_or_default();
    let mut rest: Vec<crate::app::MenuItem> = items.into_iter().filter(|i| i.id.is_empty() || !hidden.contains(&i.id)).collect();
    let mut out: Vec<crate::app::MenuItem> = Vec::new();
    if let Some(order) = a.describe.zone(&format!("menu:{kind}")) {
        for id in order {
            if id == "sep" {
                out.push(crate::app::MenuItem::sep());
            } else if let Some(k) = rest.iter().position(|i| i.id == id) {
                out.push(rest.remove(k));
            }
        }
        if !out.is_empty() && !rest.is_empty() {
            out.push(crate::app::MenuItem::sep());
        }
    }
    out.append(&mut rest);
    let feats = a.describe.features(&format!("menu-{kind}"));
    if !feats.is_empty() {
        out.push(crate::app::MenuItem::sep());
        for (f, new) in feats {
            let label = describe::feature_text(&f, &|s| value_of(a, s));
            let act = f.pointer("/click/action").and_then(Value::as_str).unwrap_or("none");
            out.push(crate::app::MenuItem::new(&format!("feat:{act}"), &format!("{label}{}", if new { "  · NEW" } else { "" })));
        }
    }
    out
}

/// A feature's button pressed: one of Files' own actions (or Hearth's open-folder / open-app).
pub fn act(a: &mut App, action: &str) -> Task<Msg> {
    let pick = |a: &mut App, f: &dyn Fn(&Entry) -> bool| {
        let paths: Vec<String> = a.entries.iter().filter(|e| f(e)).map(|e| e.path.clone()).collect();
        a.marked.clear();
        for p in paths {
            a.marked.insert(p);
        }
    };
    match action {
        "sort-downloads" if crate::sortdl::here(a) => return crate::sortdl::update(a, crate::sortdl::SMsg::Open),
        "select-pictures" => pick(a, &picture),
        "select-old" => pick(a, &old),
        "select-big" => pick(a, &big),
        _ => {}
    }
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_go_by_kind_age_and_size() {
        let e = |name: &str, mime: &str, mtime: i64, size: u64| Entry { name: name.into(), mime: mime.into(), mtime, size, ..Default::default() };
        assert!(picture(&e("a.png", "image/png", now(), 1)));
        assert!(!picture(&Entry { dir: true, ..e("Pictures", "inode/directory", now(), 0) }));
        assert!(old(&e("x", "", now() - 2 * MONTH, 1)) && !old(&e("x", "", now(), 1)));
        assert!(big(&e("x", "", now(), 2 * GB)));
    }
}
