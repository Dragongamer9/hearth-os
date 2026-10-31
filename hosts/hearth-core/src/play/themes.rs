//! Theme files (docs/HYDRA-FEATURES.md): besides Hearth's built-in themes (Settings › You › Appearance), your own
//! CSS themes in ~/.local/share/nimbus/themes/*.css, picked in the same place. A theme can change Hearth's colours
//! and look (its CSS variables like --accent, --bg, --panel) but loads nothing from the internet: @import lines and
//! web addresses in url() are taken out before the page gets it.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MAX: usize = 64 * 1024;

pub fn folder() -> PathBuf { crate::paths::data_dir().join("themes") }

/// A theme's CSS, safe to put on the page: no @import, no web or file addresses in url(), no closing </style>.
pub fn clean(css: &str) -> String {
    let no_import: String = css.lines().filter(|l| !l.trim_start().to_ascii_lowercase().starts_with("@import")).collect::<Vec<_>>().join("\n");
    let re = regex::Regex::new(r#"(?i)url\(\s*['"]?\s*(https?:|file:|//|ftp:)[^)]*\)"#).unwrap();
    let out = re.replace_all(&no_import, "none").to_string();
    let out = regex::Regex::new(r"(?i)</\s*style").unwrap().replace_all(&out, "").to_string();
    out.chars().take(MAX).collect()
}

/// The themes in a folder: [{id, name, css, accent}] (name from a `/* name: … */` line, else the file's name).
pub fn list_in(dir: &Path) -> Vec<Value> {
    let mut v: Vec<Value> = std::fs::read_dir(dir).into_iter().flatten().flatten().filter_map(|e| {
        let p = e.path();
        if p.extension().map(|x| x != "css").unwrap_or(true) || !p.is_file() { return None; }
        if std::fs::metadata(&p).map(|m| m.len() as usize > MAX * 2).unwrap_or(true) { return None; }
        let text = std::fs::read_to_string(&p).ok()?;
        let id = p.file_stem()?.to_string_lossy().to_string();
        let name = regex::Regex::new(r"(?im)^\s*/\*\s*name:\s*(.+?)\s*\*/").unwrap().captures(&text).map(|c| c[1].to_string()).unwrap_or_else(|| id.replace(['-', '_'], " "));
        let accent = regex::Regex::new(r"--accent\s*:\s*(#[0-9a-fA-F]{3,8})").unwrap().captures(&text).map(|c| c[1].to_string());
        Some(json!({"id": id, "name": name.chars().take(60).collect::<String>(), "css": clean(&text), "accent": accent}))
    }).collect();
    v.sort_by(|a, b| a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase()));
    v
}

/// The folder (made if missing, with an example theme the first time) and its themes.
pub fn list() -> Value {
    let dir = folder();
    if !dir.exists() && std::fs::create_dir_all(&dir).is_ok() { let _ = std::fs::write(dir.join("example-ember.css"), EXAMPLE); }
    json!({"folder": dir.to_string_lossy(), "themes": list_in(&dir)})
}

const EXAMPLE: &str = "/* name: Ember (example) */\n/* Copy this file, change the colours, and pick it in Settings › You › Appearance › Theme files. */\n:root {\n  --accent: #ff8a3d;\n  --accent2: #e06a1f;\n  --accentInk: #2a1204;\n}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_files_load_nothing_from_outside() {
        let css = "@import url(https://evil/x.css);\n:root{--accent:#ff0000}\nbody{background:url('https://track.me/p.png') no-repeat}\n.x{background:url(data:image/png;base64,AAA)}\n</style><script>";
        let c = clean(css);
        assert!(!c.contains("@import") && !c.contains("track.me") && !c.contains("</style"), "{c}");
        assert!(c.contains("--accent:#ff0000") && c.contains("data:image/png"), "{c}");
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("ocean-blue.css"), "/* name: Ocean */\n:root{--accent:#2288ff}").unwrap();
        std::fs::write(d.path().join("plain.css"), ":root{}").unwrap();
        std::fs::write(d.path().join("notes.txt"), "x").unwrap();
        let l = list_in(d.path());
        assert_eq!(l.len(), 2);
        assert_eq!(l[0]["name"], "Ocean");
        assert_eq!(l[0]["accent"], "#2288ff");
        assert_eq!(l[1]["name"], "plain");
        assert!(list_in(&d.path().join("missing")).is_empty());
    }
}
