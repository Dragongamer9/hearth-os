//! Sort my Downloads (Hearth OS 1.17; the owner, Nov 21: "add that to the files app in the downloads section a auto
//! sort downloads button"; mockups docs/mockups/1.17-sort-downloads, checklist sort-downloads.md).
//!
//! What's loose at the top of Downloads goes into the same five folders as the Nov 15 tidy: Hearth, Installers, Media,
//! Documents and _to_delete (copies of a download that are the same size as it, "name (1).ext", for you to empty).
//! It only moves: never deletes, never overwrites (a name that's already there is left where it is), and one Undo
//! (the note's button, or Ctrl+Z) puts everything back.
//!
//! What the rules don't know (Documents/Unsorted) is asked of Sort my Downloads' AI while the sheet is open (Settings ›
//! AI › What uses your AI: its own pick, else Hearth's AI; `hearth-layout sort-ask`, names only, nothing in the files):
//! its answer moves those to Installers, Media or Documents' folders. No AI, or no answer: the rules' place stays.

use crate::app::{App, Msg, Op, Overlay};
use hearth_ui::ui::{self, btn, t1, ts, Kind};
use iced::widget::{column, container, row, scrollable, Column, Space};
use iced::{Element, Length, Task};
use std::path::{Path, PathBuf};

pub const TOP: [&str; 5] = ["Hearth", "Installers", "Media", "Documents", "_to_delete"];
/// Each group's colour (the mockup's).
pub const COLOURS: [(u8, u8, u8); 5] = [(0xe9, 0xa4, 0x3d), (0x3d, 0x7a, 0xb8), (0x7a, 0x5a, 0xb0), (0x3f, 0x8f, 0x6a), (0xb0, 0x56, 0x3a)];
const TITLES: [&str; 5] = ["Hearth", "Installers", "Media", "Documents", "To delete"];

#[derive(Debug, Clone)]
pub enum SMsg {
    Open,
    /// Sort my Downloads' AI answered for what the rules don't know (hearth-layout sort-ask)
    Ai(hearth_ui::run::Output),
    Every,
    Go,
    Cancel,
    Undo,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// (from, the folder it goes into, its group 0…4)
    pub moves: Vec<(String, String, usize)>,
}

impl Plan {
    pub fn count(&self, g: usize) -> usize {
        self.moves.iter().filter(|m| m.2 == g).count()
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// Your Downloads folder (user-dirs.dirs' XDG_DOWNLOAD_DIR, else ~/Downloads).
pub fn downloads() -> String {
    let h = home();
    if let Ok(t) = std::fs::read_to_string(h.join(".config/user-dirs.dirs")) {
        for l in t.lines() {
            if let Some(v) = l.trim().strip_prefix("XDG_DOWNLOAD_DIR=") {
                let v = v.trim().trim_matches('"').replace("$HOME", &h.to_string_lossy());
                if v.starts_with('/') && v.trim_end_matches('/') != h.to_string_lossy().trim_end_matches('/') {
                    return v.trim_end_matches('/').to_string();
                }
            }
        }
    }
    h.join("Downloads").to_string_lossy().into_owned()
}

pub fn here(a: &App) -> bool {
    !a.path.is_empty() && a.path.trim_end_matches('/') == downloads()
}

/// The things at the top of Downloads that aren't one of the five folders (hidden ones stay as they are).
pub fn loose(a: &App) -> usize {
    a.entries.iter().filter(|e| !e.name.starts_with('.') && !TOP.contains(&e.name.as_str())).count()
}

/// "name (1).ext" / "name(2).ext" → "name.ext" (a browser's second copy of the same download).
pub fn original_of(name: &str) -> Option<String> {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let stem_t = stem.trim_end();
    if !stem_t.ends_with(')') {
        return None;
    }
    let open = stem_t.rfind('(')?;
    let n = &stem_t[open + 1..stem_t.len() - 1];
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let base = stem_t[..open].trim_end();
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}{ext}"))
}

/// Where one thing goes (the folder under Downloads, and its group).
pub fn place(dir: &Path, name: &str, is_dir: bool) -> (String, usize) {
    let l = name.to_lowercase();
    let ext = |xs: &[&str]| xs.iter().any(|x| l.ends_with(x));
    if let Some(orig) = original_of(name) {
        let (a, b) = (std::fs::metadata(dir.join(name)), std::fs::metadata(dir.join(&orig)));
        if let (Ok(a), Ok(b)) = (a, b) {
            if a.is_file() && b.is_file() && a.len() == b.len() {
                return ("_to_delete/Old copies".into(), 4);
            }
        }
    }
    if l.starts_with("hearth-ai-command-center") || l.contains("command-center") {
        return ("Hearth/AI Command Center".into(), 0);
    }
    if l.starts_with("hearth-console-remote-play") || l.starts_with("console-remote-play") {
        return ("Hearth/Console Remote Play".into(), 0);
    }
    if l.starts_with("capture-card") || l.starts_with("hearth-capture-card") {
        return ("Hearth/Capture Card".into(), 0);
    }
    if l.starts_with("hearth-os") || l.starts_with("hearth-recovery") || (l.starts_with("hearth") && l.ends_with(".iso")) {
        return ("Hearth/Hearth OS".into(), 0);
    }
    if l.starts_with("hearth") || l.starts_with("nimbus") || l.starts_with("ithil") {
        return ("Hearth/Other".into(), 0);
    }
    if is_dir {
        return ("Documents/Folders".into(), 3);
    }
    if ext(&[".iso", ".img"]) {
        return ("Installers/ISOs".into(), 1);
    }
    if ext(&[".appimage", ".deb", ".rpm", ".flatpak", ".flatpakref", ".exe", ".msi", ".run"]) || l.contains("installer") {
        return ("Installers".into(), 1);
    }
    if ext(&[".zip", ".7z", ".rar", ".tar", ".tgz", ".tar.gz", ".tar.xz", ".tar.zst", ".gz", ".xz"]) {
        return ("Installers/Archives".into(), 1);
    }
    if ext(&[".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".heic", ".bmp", ".avif"]) {
        return ("Media/Images".into(), 2);
    }
    if ext(&[".mp4", ".mkv", ".mov", ".webm", ".avi"]) {
        return ("Media/Videos".into(), 2);
    }
    if ext(&[".mp3", ".wav", ".flac", ".ogg", ".m4a", ".opus"]) {
        return ("Media/Sound".into(), 2);
    }
    if ext(&[".sh", ".py"]) {
        return ("Documents/Scripts".into(), 3);
    }
    if ext(&[".pdf", ".txt", ".md", ".doc", ".docx", ".odt", ".csv", ".xlsx", ".ods", ".json", ".html", ".log"]) {
        return ("Documents".into(), 3);
    }
    ("Documents/Unsorted".into(), 3)
}

/// A place Sort my Downloads' AI may give (hearth-layout describe.rs SORT_PLACES) and its group; anything else: none.
pub fn ai_place(p: &str) -> Option<(&'static str, usize)> {
    [("Installers", 1), ("Installers/Archives", 1), ("Media/Images", 2), ("Media/Videos", 2), ("Media/Sound", 2), ("Documents", 3), ("Documents/Scripts", 3), ("Documents/Unsorted", 3)]
        .into_iter()
        .find(|(x, _)| *x == p)
}

/// What would move, from what's in the folder now (read again, not from the window's list).
pub fn plan(dir: &str) -> Plan {
    let d = Path::new(dir);
    let mut moves = Vec::new();
    let Ok(rd) = std::fs::read_dir(d) else { return Plan::default() };
    let mut names: Vec<(String, bool)> = rd.filter_map(|e| e.ok()).filter_map(|e| {
        let n = e.file_name().to_string_lossy().into_owned();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        (!n.starts_with('.') && !TOP.contains(&n.as_str())).then_some((n, is_dir))
    }).collect();
    names.sort();
    for (n, is_dir) in names {
        let (to, g) = place(d, &n, is_dir);
        moves.push((d.join(&n).to_string_lossy().into_owned(), d.join(to).to_string_lossy().into_owned(), g));
    }
    Plan { moves }
}

/// Do it: each thing into its folder, only when nothing of that name is there. (from, to) of what moved.
pub fn apply(p: &Plan) -> (Vec<(String, String)>, usize) {
    let mut done = Vec::new();
    let mut skipped = 0;
    for (from, dir, _) in &p.moves {
        let f = Path::new(from);
        let Some(name) = f.file_name() else { continue };
        let to = Path::new(dir).join(name);
        if std::fs::symlink_metadata(f).is_err() {
            continue;
        }
        if std::fs::symlink_metadata(&to).is_ok() || std::fs::create_dir_all(dir).is_err() {
            skipped += 1;
            continue;
        }
        // rename(2) never replaces a folder that appeared meanwhile only when it's empty; checked just above.
        if std::fs::rename(f, &to).is_ok() {
            done.push((from.clone(), to.to_string_lossy().into_owned()));
        } else {
            skipped += 1;
        }
    }
    (done, skipped)
}

/// Back (or again, for Redo): only when nothing of that name is where it goes.
pub fn swap(moves: &[(String, String)], back: bool) -> usize {
    let mut n = 0;
    for (from, to) in moves {
        let (a, b) = if back { (to, from) } else { (from, to) };
        if std::fs::symlink_metadata(a).is_ok() && std::fs::symlink_metadata(b).is_err() {
            if let Some(parent) = Path::new(b).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::rename(a, b).is_ok() {
                n += 1;
            }
        }
    }
    n
}

pub fn update(a: &mut App, m: SMsg) -> Task<Msg> {
    match m {
        SMsg::Open => {
            if !here(a) {
                return Task::none();
            }
            let p = plan(&a.path);
            if p.moves.is_empty() {
                return a.say("Nothing is loose in Downloads", false);
            }
            let unknown: Vec<String> = p.moves.iter().filter(|m| m.1.ends_with("Documents/Unsorted")).filter_map(|m| Path::new(&m.0).file_name().map(|n| n.to_string_lossy().into_owned())).collect();
            a.sort_plan = Some(p);
            a.sort_every = false;
            a.overlay = Some(Overlay::Sort);
            if !unknown.is_empty() {
                let ask = serde_json::json!({ "names": unknown }).to_string();
                return Task::perform(hearth_ui::run::spawn(vec!["hearth-layout".into(), "sort-ask".into(), ask]), |o| Msg::Sort(SMsg::Ai(o)));
            }
        }
        SMsg::Ai(o) => {
            let places = o.json().and_then(|v| v.get("places").and_then(|x| x.as_object()).cloned()).unwrap_or_default();
            if let Some(p) = a.sort_plan.as_mut() {
                for m in p.moves.iter_mut() {
                    if !m.1.ends_with("Documents/Unsorted") {
                        continue;
                    }
                    let name = Path::new(&m.0).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if let Some((to, g)) = places.get(&name).and_then(|x| x.as_str()).and_then(ai_place) {
                        let base = Path::new(&m.0).parent().map(|d| d.to_path_buf()).unwrap_or_default();
                        m.1 = base.join(to).to_string_lossy().into_owned();
                        m.2 = g;
                    }
                }
            }
        }
        SMsg::Every => a.sort_every = !a.sort_every,
        SMsg::Cancel => {
            a.overlay = None;
            a.sort_plan = None;
        }
        SMsg::Go => {
            a.overlay = None;
            let Some(p) = a.sort_plan.take() else { return Task::none() };
            let (done, skipped) = apply(&p);
            if !done.is_empty() {
                a.push_undo(Op::Sort { moves: done.clone() });
            }
            let folders = TOP.iter().filter(|t| Path::new(&a.path).join(t).is_dir()).count();
            a.sorted = Some((done.len(), folders, skipped));
            let reload = a.reload();
            let n = a.sorted_seq.wrapping_add(1);
            a.sorted_seq = n;
            return Task::batch(vec![reload, Task::perform(hearth_ui::run::after(12000), move |_| Msg::SortNoteGone(n))]);
        }
        SMsg::Undo => {
            a.sorted = None;
            return a.run_undo(true);
        }
    }
    Task::none()
}

fn swatch<'a>(c: (u8, u8, u8), size: f32) -> Element<'a, Msg> {
    container(Space::new().width(Length::Fixed(size)).height(Length::Fixed(size))).style(ui::boxed(iced::Color::from_rgb8(c.0, c.1, c.2), size * 0.24, None)).into()
}

/// One of the five folders' icon in Downloads: its colour, as the mockups show them.
pub fn folder_icon<'a>(name: &str, size: f32) -> Element<'a, Msg> {
    let g = TOP.iter().position(|t| *t == name).unwrap_or(0);
    container(swatch(COLOURS[g], (size * 0.86).max(8.0))).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).into()
}

/// The preview (mockup 1).
pub fn dialog(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(plan) = &a.sort_plan else { return Space::new().into() };
    let n = plan.moves.len();
    let what: [&str; 5] = [
        "Hearth OS, AI Command Center, Remote Play and Hearth's other downloads",
        "AppImages, .rpm, .deb and Flatpaks · disc images in ISOs · archives in Archives",
        "Images, Videos and Sound",
        "PDFs, text and spreadsheets · scripts in Scripts · the rest in Unsorted",
        "",
    ];
    let mut col = Column::new().spacing(10.0);
    col = col.push(ts("Sort my Downloads", 22.0, p.text));
    col = col.push(ui::t(format!("The {n} things loose in Downloads go into folders by what they are. Nothing is deleted or overwritten, and one click puts everything back."), 14.0, p.muted).width(Length::Fill));
    for g in 0..5 {
        let c = plan.count(g);
        if c == 0 && g == 4 {
            continue;
        }
        let sub = if g == 4 {
            let names: Vec<String> = plan.moves.iter().filter(|m| m.2 == 4).map(|m| crate::model::base(&m.0)).collect();
            let first = names.first().cloned().unwrap_or_default();
            if names.len() > 1 { format!("Old copies Files found ({first} and {} more) · kept for you to empty yourself", names.len() - 1) } else { format!("Old copies Files found ({first}) · kept for you to empty yourself") }
        } else {
            what[g].to_string()
        };
        let mut body = column![t1(TITLES[g], 15.0, p.text), t1(sub, 13.0, p.muted)].spacing(3.0);
        if a.sort_every {
            for (from, dir, _) in plan.moves.iter().filter(|m| m.2 == g) {
                let rel = dir.strip_prefix(&format!("{}/", a.path.trim_end_matches('/'))).unwrap_or(dir);
                body = body.push(t1(format!("{} → {rel}", crate::model::base(from)), 12.5, p.muted));
            }
        }
        let danger = g == 4;
        let r = row![swatch(COLOURS[g], 30.0), body.width(Length::Fill), t1(crate::model::plural(c, "item", "items"), 13.0, p.muted)].spacing(14.0).align_y(iced::Center);
        let bg = if danger { hearth_ui::Pal::a(p.danger, 0.06) } else { iced::Color::TRANSPARENT };
        let line = if danger { hearth_ui::Pal::a(p.danger, 0.45) } else { p.line };
        col = col.push(container(r).padding(12.0).width(Length::Fill).style(ui::boxed(bg, 10.0, Some((line, 1.0)))));
    }
    let buttons = row![
        Space::new().width(Length::Fill),
        btn(&p, if a.sort_every { "Hide the files" } else { "Show every file" }, Kind::Normal, false, Some(Msg::Sort(SMsg::Every))),
        btn(&p, "Cancel", Kind::Normal, false, Some(Msg::Sort(SMsg::Cancel))),
        btn(&p, format!("Sort {n} items"), Kind::Primary, false, Some(Msg::Sort(SMsg::Go))),
    ]
    .spacing(12.0)
    .align_y(iced::Center);
    let body: Element<Msg> = if a.sort_every { scrollable(col).height(Length::Fixed(520.0)).into() } else { col.into() };
    ui::dialog_box(&p, 780.0, column![body, container(buttons).padding(iced::Padding { top: 12.0, right: 0.0, bottom: 0.0, left: 0.0 })])
}

/// The note after sorting, with Undo (mockup 2).
pub fn note(a: &App) -> Option<Element<'_, Msg>> {
    let p = a.p;
    let (n, folders, skipped) = a.sorted?;
    let mut msg = format!("✓ Sorted {} into {}.", crate::model::plural(n, "item", "items"), crate::model::plural(folders, "folder", "folders"));
    if skipped > 0 {
        msg = format!("{msg} {skipped} stayed (a name was already there).");
    }
    let r = row![t1(msg, 14.0, p.text), btn(&p, "Undo", Kind::Normal, false, Some(Msg::Sort(SMsg::Undo))), t1("Ctrl+Z also puts them back", 13.0, p.muted)].spacing(18.0).align_y(iced::Center);
    Some(container(r).padding(iced::Padding { top: 12.0, right: 18.0, bottom: 12.0, left: 18.0 }).style(ui::boxed(p.panel, 12.0, Some((p.line, 1.0)))).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_are_found() {
        assert_eq!(original_of("AppImage(2).zip").as_deref(), Some("AppImage.zip"));
        assert_eq!(original_of("photo (1).jpg").as_deref(), Some("photo.jpg"));
        assert_eq!(original_of("notes.txt"), None);
        assert_eq!(original_of("(1).txt"), None);
        assert_eq!(original_of("v(1.2).txt"), None);
    }

    #[test]
    fn sorts_only_moves_and_undo_puts_back() {
        let d = std::env::temp_dir().join(format!("hearth-sortdl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("Media/Images")).unwrap();
        for (n, b) in [("a.png", "x"), ("a (1).png", "x"), ("b.AppImage", "y"), ("hearth-os-anatase.iso", "z"), ("notes.pdf", "n"), ("Media/Images/c.png", "c"), ("c.png", "other")] {
            std::fs::write(d.join(n), b).unwrap();
        }
        let p = plan(&d.to_string_lossy());
        assert!(p.moves.iter().any(|m| m.0.ends_with("a (1).png") && m.2 == 4));
        assert!(p.moves.iter().any(|m| m.0.ends_with("hearth-os-anatase.iso") && m.1.ends_with("Hearth/Hearth OS")));
        assert!(!p.moves.iter().any(|m| m.0.ends_with("/Media")));
        let (done, skipped) = apply(&p);
        assert_eq!(skipped, 1); // c.png: one is already in Media/Images
        assert_eq!(std::fs::read_to_string(d.join("Media/Images/c.png")).unwrap(), "c"); // never overwritten
        assert!(d.join("Installers/b.AppImage").exists() && d.join("_to_delete/Old copies/a (1).png").exists());
        assert_eq!(swap(&done, true), done.len());
        assert!(d.join("b.AppImage").exists() && d.join("a (1).png").exists() && d.join("c.png").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
