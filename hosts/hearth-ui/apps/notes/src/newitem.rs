//! New item (page 7): the workspace's rules from its CLAUDE.md. An item is `<project>/<kind>/<item>/`, its name is
//! lowercase words with dashes (a version or date leads when it matters: `1.13-features`, `2026-11-15-recap`), it has a
//! README.md (what it is, its status, which release, how to rebuild it), a line in INDEX.md, and a line in the day's log.

use std::path::Path;

pub use crate::vault::{KINDS, PROJECTS};

pub const STATUSES: [&str; 3] = ["draft", "approved", "posted"];

/// The name rule (check.py's: `^[a-z0-9][a-z0-9._-]*$`).
pub fn name_ok(n: &str) -> bool {
    let mut c = n.chars();
    match c.next() {
        Some(f) if f.is_ascii_lowercase() || f.is_ascii_digit() => {}
        _ => return false,
    }
    c.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')) && !n.ends_with('-')
}

/// "Hearth Notes" → "hearth-notes"
pub fn suggest(n: &str) -> String {
    let mut out = String::new();
    for ch in n.trim().chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches(['-', '.', '_']).to_string()
}

/// "hearth-notes" → "Hearth Notes"
pub fn title_of(name: &str) -> String {
    name.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn taken(vault: &Path, project: &str, kind: &str, name: &str) -> bool {
    vault.join(project).join(kind).join(name).exists()
}

/// The README template.
pub fn readme(title: &str, status: &str, belongs: &str) -> String {
    format!("# {title}\n\nWhat it is: …\nStatus: **{status}**\nBelongs to: {belongs}\n\n## Rebuild\n…\n")
}

/// INDEX.md's heading for a project.
pub fn index_heading(project: &str) -> &'static str {
    match project {
        "hearth" => "## Hearth OS",
        _ => "## Other",
    }
}

/// "- `hearth/mockups/hearth-notes`: … Status: draft."
pub fn index_line(item: &str, status: &str) -> String {
    format!("- `{item}`: … Status: {status}.")
}

/// INDEX.md with `line` added at the end of its project's section (the section made if it isn't there).
pub fn index_with(index: &str, project: &str, line: &str) -> String {
    let head = index_heading(project);
    let lines: Vec<&str> = index.lines().collect();
    let Some(h) = lines.iter().position(|l| l.trim() == head) else {
        let mut t = index.trim_end().to_string();
        t.push_str(&format!("\n\n{head}\n{line}\n"));
        return t;
    };
    // The section ends at the next "## " heading; the line goes after its last list line.
    let end = lines.iter().enumerate().skip(h + 1).find(|(_, l)| l.starts_with("## ")).map(|(i, _)| i).unwrap_or(lines.len());
    let mut at = h + 1;
    for (i, l) in lines.iter().enumerate().take(end).skip(h + 1) {
        if !l.trim().is_empty() {
            at = i + 1;
        }
    }
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    out.insert(at, line.to_string());
    let mut t = out.join("\n");
    t.push('\n');
    t
}

/// "- 03:12 Made: hearth/mockups/hearth-notes (draft)."
pub fn log_line(hhmm: &str, item: &str, status: &str) -> String {
    format!("- {hhmm} Made: {item} ({status}).")
}

/// A log's text with a line added (the log made with its "# YYYY-MM-DD" title if it's new).
pub fn log_with(log: Option<&str>, day: &str, line: &str) -> String {
    match log {
        Some(t) if !t.trim().is_empty() => {
            let mut t = t.trim_end().to_string();
            t.push('\n');
            t.push_str(line);
            t.push('\n');
            t
        }
        _ => format!("# {day}\n\n{line}\n"),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    pub item: String,
    pub readme: String,
}

/// Makes the item: its folder and README, the INDEX line, the log line. Nothing is overwritten.
pub fn make(vault: &Path, project: &str, kind: &str, name: &str, status: &str, belongs: &str, day: &str, hhmm: &str) -> Result<Made, String> {
    if !PROJECTS.contains(&project) || !KINDS.contains(&kind) {
        return Err("Pick a project and a kind from the workspace's layout.".into());
    }
    if !name_ok(name) {
        return Err("Lowercase words with dashes, no spaces.".into());
    }
    if taken(vault, project, kind, name) {
        return Err(format!("{project}/{kind}/{name} is already there."));
    }
    let item = format!("{project}/{kind}/{name}");
    let readme_rel = format!("{item}/README.md");
    crate::save::note(vault, &readme_rel, &readme(&title_of(name), status, belongs)).map_err(|e| e.to_string())?;
    let idx = std::fs::read_to_string(vault.join("INDEX.md")).unwrap_or_else(|_| "# Index\n".into());
    if !idx.contains(&format!("`{item}`")) {
        crate::save::note(vault, "INDEX.md", &index_with(&idx, project, &index_line(&item, status))).map_err(|e| e.to_string())?;
    }
    let log_rel = format!("log/{day}.md");
    let old = std::fs::read_to_string(vault.join(&log_rel)).ok();
    crate::save::note(vault, &log_rel, &log_with(old.as_deref(), day, &log_line(hhmm, &item, status))).map_err(|e| e.to_string())?;
    Ok(Made { item, readme: readme_rel })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_workspace_rules() {
        assert!(!name_ok("Hearth Notes"));
        assert!(name_ok("hearth-notes"));
        assert!(name_ok("1.13-features"));
        assert!(!name_ok("-x") && !name_ok("x-") && !name_ok(""));
        assert_eq!(suggest("Hearth Notes"), "hearth-notes");
        assert_eq!(suggest("  2026-11-15 Recap!  "), "2026-11-15-recap");
        assert_eq!(title_of("hearth-notes"), "Hearth Notes");
    }

    #[test]
    fn makes_the_item_its_index_line_and_log_line() {
        let _l = crate::env_lock();
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_DATA_HOME", d.path().join("data"));
        let v = d.path().join("ws");
        std::fs::create_dir_all(&v).unwrap();
        std::fs::write(v.join("INDEX.md"), "# Index\n\nEvery item.\n\n## Hearth OS\n- `hearth/videos/a`: A. Status: draft.\n\n## Guides and notes\n- x\n").unwrap();
        let m = make(&v, "hearth", "mockups", "hearth-notes", "draft", "not yet assigned", "2026-11-20", "03:12").unwrap();
        assert_eq!(m.item, "hearth/mockups/hearth-notes");
        let r = std::fs::read_to_string(v.join("hearth/mockups/hearth-notes/README.md")).unwrap();
        assert!(r.starts_with("# Hearth Notes\n") && r.contains("Status: **draft**") && r.contains("## Rebuild"));
        let idx = std::fs::read_to_string(v.join("INDEX.md")).unwrap();
        assert!(idx.contains("- `hearth/videos/a`: A. Status: draft.\n- `hearth/mockups/hearth-notes`: … Status: draft.\n\n## Guides"), "{idx}");
        let log = std::fs::read_to_string(v.join("log/2026-11-20.md")).unwrap();
        assert_eq!(log, "# 2026-11-20\n\n- 03:12 Made: hearth/mockups/hearth-notes (draft).\n");
        assert!(make(&v, "hearth", "mockups", "hearth-notes", "draft", "", "2026-11-20", "03:13").is_err(), "taken");
        assert!(make(&v, "hearth", "mockups", "Bad Name", "draft", "", "2026-11-20", "03:13").is_err());
        std::env::remove_var("XDG_DATA_HOME");
    }
}
