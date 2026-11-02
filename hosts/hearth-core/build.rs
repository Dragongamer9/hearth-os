//! This version's release date, built into the program so it's there offline (Settings › About shows
//! "Hearth OS 1.0 · November 2, 2026"). It comes from the version's heading in docs/ROADMAP.md,
//! "## 1.0 (released 2026-11-02)", which the release step writes (docs/UPDATES.md, "Version numbers"). Without one
//! (a build before the release, an older "## 0.6.63 (released)" heading, or no docs/ next to the source) it's empty
//! and only the number is shown. Nothing else is read and nothing can fail here.

fn key(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches('v');
    let base = v.split(|c: char| !(c.is_ascii_digit() || c == '.')).next().unwrap_or("");
    let mut n: Vec<u64> = base.split('.').filter(|s| !s.is_empty()).map(|s| s.parse().unwrap_or(0)).collect();
    while n.len() < 4 { n.push(0); }
    n
}

fn date_for(version: &str, roadmap: &str) -> Option<String> {
    let want = key(version);
    for line in roadmap.lines() {
        let Some(head) = line.trim().strip_prefix("## ") else { continue };
        let mut words = head.split_whitespace();
        let ver = words.next().unwrap_or("");
        if !ver.trim_start_matches('v').starts_with(|c: char| c.is_ascii_digit()) || key(ver) != want { continue; }
        let rest: String = words.collect::<Vec<_>>().join(" ");
        let d = rest.strip_prefix("(released")?.trim().trim_end_matches(')').trim().to_string();
        let ok = d.len() == 10 && d.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
        return ok.then_some(d);
    }
    None
}

fn main() {
    let roadmap = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/ROADMAP.md");
    let date = match std::fs::read_to_string(&roadmap) {
        Ok(text) => {
            println!("cargo:rerun-if-changed={}", roadmap.display());
            date_for(env!("CARGO_PKG_VERSION"), &text).unwrap_or_default()
        }
        Err(_) => String::new(),
    };
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-env=HEARTH_VERSION_DATE={date}");
}
