//! The workspace's own check (`python3 tools/check.py`, its CLAUDE.md red flag 2): run before every push. While it
//! finds problems nothing is pushed (your changes stay saved on this PC), and each problem is shown exactly as the
//! check says it, with a fix for the ones Hearth Notes can make (a README from the template, the INDEX line).

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum Fix {
    /// Make the item's README.md from the template.
    Readme(String),
    /// Add the item's line to INDEX.md.
    Index(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    /// What the check named (an item or a file).
    pub path: String,
    /// What it said is wrong.
    pub what: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    /// "tidy: 52 items, 610 files"
    Tidy(String),
    Untidy(Vec<Problem>),
    /// The vault has no tools/check.py (any folder that isn't the workspace): nothing to run.
    None,
    /// It couldn't run (no python3, it crashed, it took too long): the push waits too.
    Failed(String),
}

impl Check {
    pub fn blocks(&self) -> bool {
        matches!(self, Check::Untidy(_) | Check::Failed(_))
    }
}

pub fn has_check(dir: &Path) -> bool {
    dir.join("tools/check.py").is_file()
}

/// The check's output: "  - <path>: <what's wrong>" lines.
pub fn parse(out: &str) -> Vec<Problem> {
    let mut v = Vec::new();
    for l in out.lines() {
        let Some(item) = l.trim_start().strip_prefix("- ") else { continue };
        let (path, what) = match item.split_once(": ") {
            Some((p, w)) => (p.trim().to_string(), w.trim().to_string()),
            None => (String::new(), item.trim().to_string()),
        };
        let fix = if what == "no README.md" {
            Some(Fix::Readme(path.clone()))
        } else if what == "not listed in INDEX.md" {
            Some(Fix::Index(path.clone()))
        } else {
            None
        };
        v.push(Problem { path, what, fix });
    }
    v
}

pub fn run(dir: &Path) -> Check {
    if !has_check(dir) {
        return Check::None;
    }
    let o = crate::git::run(Command::new("python3").arg("-I").arg("tools/check.py").current_dir(dir).env("LC_ALL", "C.UTF-8"), 60);
    if o.ok {
        return Check::Tidy(o.out.lines().last().unwrap_or("tidy").trim().to_string());
    }
    let probs = parse(&o.out);
    if o.code == 1 && !probs.is_empty() {
        Check::Untidy(probs)
    } else {
        Check::Failed(o.why())
    }
}

/// Makes a fix (README from the template, the INDEX line), saved like any note.
pub fn apply(dir: &Path, fix: &Fix, status: &str) -> Result<(), String> {
    match fix {
        Fix::Readme(item) => {
            let rel = format!("{item}/README.md");
            if dir.join(&rel).exists() {
                return Ok(());
            }
            let name = item.rsplit('/').next().unwrap_or(item);
            let text = crate::newitem::readme(&crate::newitem::title_of(name), status, "not yet assigned");
            crate::save::note(dir, &rel, &text).map(|_| ()).map_err(|e| e.to_string())
        }
        Fix::Index(item) => {
            let idx = std::fs::read_to_string(dir.join("INDEX.md")).unwrap_or_else(|_| "# Index\n".into());
            if idx.contains(item.as_str()) {
                return Ok(());
            }
            let project = item.split('/').next().unwrap_or("");
            let new = crate::newitem::index_with(&idx, project, &crate::newitem::index_line(item, status));
            crate::save::note(dir, "INDEX.md", &new).map(|_| ()).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_checks_problems() {
        let out = "hearth-workspace isn't tidy:\n  - hearth/mockups/hearth-notes: no README.md\n  - hearth/mockups/hearth-notes: not listed in INDEX.md\n  - notes.txt: a loose file at the top (put it in an item folder)\n";
        let p = parse(out);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].fix, Some(Fix::Readme("hearth/mockups/hearth-notes".into())));
        assert_eq!(p[1].fix, Some(Fix::Index("hearth/mockups/hearth-notes".into())));
        assert_eq!(p[2].what, "a loose file at the top (put it in an item folder)");
        assert_eq!(p[2].fix, None);
    }
}
