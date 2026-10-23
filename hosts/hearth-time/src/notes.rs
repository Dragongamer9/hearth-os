//! Hearth Notes' notes: plain Markdown files in ~/Documents/Notes, so any editor (and any other notes app) can open
//! them, and they copy to a USB drive or another PC as they are.
//!   <Title>.md      a note; its first line is its title; checklists are "- [ ] item" / "- [x] done"
//!   images/         pictures pasted into notes, linked from them as ![](images/<file>.png)
//!   .pinned         the pinned notes' file names, one per line
//!   .trash/         deleted notes (delete them from there for good, or move them back)
//! A note's file is renamed to follow its title when the title changes.

use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MAX_NOTE: u64 = 4 * 1024 * 1024;
const MAX_IMAGE: usize = 25 * 1024 * 1024;

pub fn dir() -> PathBuf { crate::paths::notes_dir() }

/// A note's file name is a plain name in the Notes folder: no folders, no hidden files.
fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', '\0']) && name != ".." && (name.ends_with(".md") || name.ends_with(".txt"));
    if ok { Ok(()) } else { Err("That isn't a note's file name.".into()) }
}

/// A note's title: its first line that has text, without Markdown's heading marks.
pub fn title_of(text: &str) -> String {
    text.lines().map(|l| l.trim()).find(|l| !l.is_empty())
        .map(|l| l.trim_start_matches('#').trim().trim_start_matches("- [ ]").trim_start_matches("- [x]").trim().to_string())
        .unwrap_or_default()
}

/// A file name from a title: characters file systems dislike become spaces; at most 60 characters.
pub fn file_name_for(title: &str) -> String {
    let clean: String = title.chars().map(|c| if c.is_control() || "/\\:*?\"<>|#".contains(c) { ' ' } else { c }).collect();
    let mut s: String = clean.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect();
    s = s.trim_matches(['.', ' ']).to_string();
    if s.is_empty() { s = "Note".into(); }
    format!("{s}.md")
}

fn pinned_file(d: &Path) -> PathBuf { d.join(".pinned") }
fn pinned(d: &Path) -> Vec<String> {
    std::fs::read_to_string(pinned_file(d)).unwrap_or_default().lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}
fn set_pinned(d: &Path, list: &[String]) -> Result<(), String> {
    let mut l: Vec<String> = Vec::new();
    for x in list { if !l.contains(x) { l.push(x.clone()); } }
    crate::store::write_bytes(&pinned_file(d), (l.join("\n") + "\n").as_bytes())
}

/// A free file name: "Shopping.md", else "Shopping 2.md", "Shopping 3.md"...
fn free_name(d: &Path, wanted: &str, except: Option<&str>) -> String {
    let (stem, ext) = wanted.rsplit_once('.').unwrap_or((wanted, "md"));
    let mut name = wanted.to_string();
    let mut n = 2;
    while d.join(&name).exists() && Some(name.as_str()) != except {
        name = format!("{stem} {n}.{ext}");
        n += 1;
    }
    name
}

fn modified(p: &Path) -> i64 { crate::store::mtime(p) }

/// Every note, pinned first, then the most recently changed. Each with its text (for search and previews).
pub fn list_in(d: &Path) -> Vec<Value> {
    let pins = pinned(d);
    let mut v: Vec<Value> = std::fs::read_dir(d).into_iter().flatten().flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        if check_name(&name).is_err() { return None; }
        let p = e.path();
        if !p.is_file() || std::fs::metadata(&p).map(|m| m.len() > MAX_NOTE).unwrap_or(true) { return None; }
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let open = text.lines().filter(|l| l.trim_start().starts_with("- [ ]")).count();
        let done = text.lines().filter(|l| { let t = l.trim_start(); t.starts_with("- [x]") || t.starts_with("- [X]") }).count();
        let mut title = title_of(&text);
        if title.is_empty() { title = name.trim_end_matches(".md").trim_end_matches(".txt").to_string(); }
        Some(json!({"name": name, "title": title, "text": text, "modified": modified(&p), "pinned": pins.contains(&name), "open": open, "done": done}))
    }).collect();
    v.sort_by(|a, b| (b["pinned"].as_bool(), b["modified"].as_i64()).cmp(&(a["pinned"].as_bool(), a["modified"].as_i64())));
    v
}
pub fn list() -> Vec<Value> { list_in(&dir()) }

pub fn read_in(d: &Path, name: &str) -> Result<String, String> {
    check_name(name)?;
    std::fs::read_to_string(d.join(name)).map_err(|_| "That note isn't there anymore.".to_string())
}

/// Saves a note. An empty `name` makes a new file from the title. When the title changed, the file is renamed to
/// match (if that name is free). Returns {name, modified}.
pub fn write_in(d: &Path, name: &str, text: &str) -> Result<Value, String> {
    std::fs::create_dir_all(d).map_err(|e| format!("Couldn't make the Notes folder: {e}"))?;
    if text.len() as u64 > MAX_NOTE { return Err("That note is too long to save (over 4 MB).".into()); }
    let wanted = file_name_for(&title_of(text));
    let mut name = name.to_string();
    if name.is_empty() {
        name = free_name(d, &wanted, None);
    } else {
        check_name(&name)?;
        if name.ends_with(".md") && wanted != name && d.join(&name).exists() {
            let stem_now = name.trim_end_matches(".md");
            let stem_wanted = wanted.trim_end_matches(".md");
            // "Shopping 2.md" for a note titled "Shopping" is already right.
            let numbered = stem_now.strip_prefix(stem_wanted).map(|r| r.trim().parse::<u32>().is_ok()).unwrap_or(false);
            if !numbered {
                let new = free_name(d, &wanted, Some(&name));
                if new != name && std::fs::rename(d.join(&name), d.join(&new)).is_ok() {
                    let mut pins = pinned(d);
                    if let Some(p) = pins.iter_mut().find(|p| **p == name) { *p = new.clone(); let _ = set_pinned(d, &pins); }
                    name = new;
                }
            }
        }
    }
    crate::store::write_bytes(&d.join(&name), text.as_bytes())?;
    Ok(json!({"name": name, "modified": modified(&d.join(&name))}))
}

/// Moves a note to .trash (a later note of the same name doesn't overwrite it).
pub fn delete_in(d: &Path, name: &str) -> Result<(), String> {
    check_name(name)?;
    let trash = d.join(".trash");
    std::fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let dest = free_name(&trash, name, None);
    std::fs::rename(d.join(name), trash.join(dest)).map_err(|e| format!("Couldn't delete it: {e}"))?;
    let pins: Vec<String> = pinned(d).into_iter().filter(|p| p != name).collect();
    let _ = set_pinned(d, &pins);
    Ok(())
}

pub fn pin_in(d: &Path, name: &str, on: bool) -> Result<(), String> {
    check_name(name)?;
    let mut pins: Vec<String> = pinned(d).into_iter().filter(|p| p != name).collect();
    if on { pins.insert(0, name.to_string()); }
    set_pinned(d, &pins)
}

/// Saves a pasted picture (base64, or a data: URL) into images/. Returns its path for the note ("images/...").
pub fn save_image_in(d: &Path, data: &str) -> Result<String, String> {
    let (mime, b64) = match data.strip_prefix("data:") {
        Some(rest) => { let (m, b) = rest.split_once(',').ok_or("Not a picture.")?; (m.trim_end_matches(";base64").to_string(), b.to_string()) }
        None => ("image/png".into(), data.to_string()),
    };
    let ext = match mime.as_str() { "image/png" => "png", "image/jpeg" | "image/jpg" => "jpg", "image/gif" => "gif", "image/webp" => "webp", _ => return Err("Only PNG, JPEG, GIF and WebP pictures can go in notes.".into()) };
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| "That picture couldn't be read.")?;
    if bytes.len() > MAX_IMAGE { return Err("That picture is too big (over 25 MB).".into()); }
    let images = d.join("images");
    std::fs::create_dir_all(&images).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let name = free_name(&images, &format!("pasted-{stamp}.{ext}"), None);
    crate::store::write_bytes(&images.join(&name), &bytes)?;
    Ok(format!("images/{name}"))
}

/// A picture from images/ as a data: URL for the page.
pub fn image_in(d: &Path, rel: &str) -> Result<String, String> {
    let rel = rel.trim().trim_start_matches("./");
    let Some(file) = rel.strip_prefix("images/") else { return Err("Only pictures in the Notes folder's images/ show.".into()) };
    if file.is_empty() || file.contains(['/', '\\']) || file.starts_with('.') { return Err("Not a picture.".into()); }
    let ext = file.rsplit_once('.').map(|x| x.1.to_ascii_lowercase()).unwrap_or_default();
    let mime = match ext.as_str() { "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif", "webp" => "image/webp", "svg" => "image/svg+xml", _ => return Err("Not a picture.".into()) };
    let bytes = std::fs::read(d.join("images").join(file)).map_err(|_| "That picture is missing.")?;
    if bytes.len() > MAX_IMAGE { return Err("That picture is too big to show.".into()); }
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_titles() {
        assert_eq!(title_of("\n# Shopping list\n- [ ] milk"), "Shopping list");
        assert_eq!(file_name_for("a/b: c?"), "a b c.md");
        assert_eq!(file_name_for("   "), "Note.md");
        assert!(check_name("../x.md").is_err());
        assert!(check_name(".pinned").is_err());
        assert!(check_name("x.md").is_ok());
    }

    #[test]
    fn write_rename_pin_delete() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path();
        let a = write_in(d, "", "# Shopping\n- [ ] milk\n- [x] eggs\n").unwrap();
        assert_eq!(a["name"], "Shopping.md");
        let b = write_in(d, "", "Shopping\nsecond").unwrap();
        assert_eq!(b["name"], "Shopping 2.md");
        // Already numbered for its title: stays.
        assert_eq!(write_in(d, "Shopping 2.md", "Shopping\nthird").unwrap()["name"], "Shopping 2.md");
        pin_in(d, "Shopping.md", true).unwrap();
        // New title: the file follows, and stays pinned.
        let r = write_in(d, "Shopping.md", "# Groceries\n- [ ] milk").unwrap();
        assert_eq!(r["name"], "Groceries.md");
        let l = list_in(d);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0]["name"], "Groceries.md");
        assert_eq!(l[0]["pinned"], true);
        assert_eq!(l[0]["open"], 1);
        delete_in(d, "Groceries.md").unwrap();
        assert!(d.join(".trash/Groceries.md").exists());
        assert_eq!(list_in(d).len(), 1);
        assert!(read_in(d, "Groceries.md").is_err());
    }

    #[test]
    fn pictures() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path();
        // A 1x1 PNG.
        let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let rel = save_image_in(d, png).unwrap();
        assert!(rel.starts_with("images/pasted-") && rel.ends_with(".png"));
        assert!(image_in(d, &rel).unwrap().starts_with("data:image/png;base64,iVBOR"));
        assert!(image_in(d, "images/../../etc/passwd").is_err());
        assert!(image_in(d, "/etc/passwd").is_err());
        assert!(save_image_in(d, "data:text/html;base64,AAAA").is_err());
    }
}
