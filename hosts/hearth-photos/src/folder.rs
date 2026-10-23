//! The pictures in a folder, in the order they're shown (the opened picture's place among them).

use crate::formats::{is_picture, natural_cmp};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Debug, Clone)]
pub struct Pic {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// Last changed, in milliseconds since 1970.
    pub mtime: u64,
}

#[derive(Serialize, Debug)]
pub struct Listing {
    pub dir: String,
    pub pics: Vec<Pic>,
    /// Where the opened picture is in `pics` (0 when a folder was opened).
    pub index: usize,
}

fn hidden(name: &str) -> bool { name.starts_with('.') }

/// The pictures next to `target` (a picture), or in it (a folder). Hidden files are left out unless the opened
/// picture is one of them.
pub fn list(target: &Path) -> Result<Listing, String> {
    let meta = std::fs::metadata(target).map_err(|e| crate::friendly_io(&e))?;
    let (dir, picked): (PathBuf, Option<PathBuf>) = if meta.is_dir() {
        (target.to_path_buf(), None)
    } else {
        (target.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")), Some(target.to_path_buf()))
    };
    let show_hidden = picked.as_ref().and_then(|p| p.file_name()).map(|n| hidden(&n.to_string_lossy())).unwrap_or(false);
    let mut pics = Vec::new();
    match std::fs::read_dir(&dir) {
        Ok(rd) => {
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                if (!show_hidden && hidden(&name)) || name.ends_with(".hearth-photos-tmp") || !is_picture(&p) { continue; }
                let Ok(m) = e.metadata() else { continue };
                // A link to a picture counts; a folder called "x.jpg" doesn't.
                let m = if m.file_type().is_symlink() { match std::fs::metadata(&p) { Ok(m) => m, Err(_) => continue } } else { m };
                if !m.is_file() { continue; }
                let mtime = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
                pics.push(Pic { name, path: p.to_string_lossy().into_owned(), size: m.len(), mtime });
            }
        }
        // A folder Photos can't look in: just the picture that was opened.
        Err(_) => {}
    }
    pics.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    let mut index = 0;
    if let Some(p) = picked {
        let want = p.to_string_lossy().into_owned();
        match pics.iter().position(|x| x.path == want) {
            Some(i) => index = i,
            None => {
                // Opened with a name the listing didn't have (an unusual extension, or the folder couldn't be read).
                let size = meta.len();
                let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let at = pics.iter().position(|x| natural_cmp(&x.name, &name).is_gt()).unwrap_or(pics.len());
                pics.insert(at, Pic { name, path: want, size, mtime });
                index = at;
            }
        }
    }
    Ok(Listing { dir: dir.to_string_lossy().into_owned(), pics, index })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_a_folder() {
        let d = std::env::temp_dir().join(format!("hp-folder-{}", std::process::id()));
        std::fs::create_dir_all(d.join("sub.jpg")).unwrap();
        for n in ["b10.png", "b2.JPG", "a.webp", ".hidden.png", "notes.txt", "c.heic"] { std::fs::write(d.join(n), b"x").unwrap(); }
        let l = list(&d.join("b2.JPG")).unwrap();
        let names: Vec<_> = l.pics.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a.webp", "b2.JPG", "b10.png", "c.heic"]);
        assert_eq!(l.index, 1);
        let all = list(&d).unwrap();
        assert_eq!(all.index, 0);
        assert_eq!(all.pics.len(), 4);
        // A hidden picture that was opened shows the hidden ones too.
        assert_eq!(list(&d.join(".hidden.png")).unwrap().pics.len(), 5);
        // An odd extension opened directly is still shown.
        std::fs::write(d.join("odd.pic"), b"x").unwrap();
        let o = list(&d.join("odd.pic")).unwrap();
        assert_eq!(o.pics[o.index].name, "odd.pic");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
