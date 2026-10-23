//! Comic books (.cbz: a zip of pictures, one per page, in name order).

use std::io::Read;
use std::path::Path;

const PICTURES: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "avif"];

fn is_picture(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !lower.starts_with("__macosx/") && !lower.rsplit('/').next().unwrap_or("").starts_with('.')
        && PICTURES.iter().any(|e| lower.ends_with(&format!(".{e}")))
}

fn open(path: &Path) -> Result<zip::ZipArchive<std::fs::File>, String> {
    let f = std::fs::File::open(path).map_err(|e| crate::friendly_io(&e))?;
    zip::ZipArchive::new(f).map_err(|_| "This comic book file looks damaged (it isn't a zip archive)".to_string())
}

/// The pages' names, in reading order.
pub fn pages(path: &Path) -> Result<Vec<String>, String> {
    let z = open(path)?;
    let mut names: Vec<String> = z.file_names().filter(|n| is_picture(n)).map(str::to_string).collect();
    names.sort_by(|a, b| crate::natural_cmp(a, b));
    if names.is_empty() { return Err("There are no pages (pictures) in this comic book file".into()); }
    Ok(names)
}

/// One page's picture: (bytes, MIME type).
pub fn page(path: &Path, index: usize) -> Result<(Vec<u8>, &'static str), String> {
    let names = pages(path)?;
    let name = names.get(index).ok_or("No such page")?;
    let mut z = open(path)?;
    let mut f = z.by_name(name).map_err(|e| e.to_string())?;
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let lower = name.to_ascii_lowercase();
    let mime = match lower.rsplit('.').next().unwrap_or("") {
        "png" => "image/png", "gif" => "image/gif", "webp" => "image/webp", "bmp" => "image/bmp", "avif" => "image/avif",
        _ => "image/jpeg",
    };
    Ok((buf, mime))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write;

    pub fn make_cbz(path: &Path) {
        let f = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        for name in ["Comic/page10.png", "Comic/page2.png", "Comic/page1.png", "Comic/.hidden.png", "__MACOSX/Comic/._page1.png", "Comic/notes.txt"] {
            z.start_file(name, o).unwrap();
            z.write_all(name.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn reads_pages_in_order() {
        let p = std::env::temp_dir().join(format!("hd-{}.cbz", std::process::id()));
        make_cbz(&p);
        assert_eq!(pages(&p).unwrap(), vec!["Comic/page1.png", "Comic/page2.png", "Comic/page10.png"]);
        let (b, mime) = page(&p, 2).unwrap();
        assert_eq!(b, b"Comic/page10.png");
        assert_eq!(mime, "image/png");
        assert!(page(&p, 3).is_err());
        std::fs::write(&p, b"not a zip").unwrap();
        assert!(pages(&p).is_err());
        std::fs::remove_file(&p).unwrap();
    }
}
