//! Sync now (and Sync by itself): the order things happen in, so nothing untidy or secret ever leaves the PC.
//!
//! 1. A conflict still open? Then that's shown first (nothing else happens).
//! 2. "Warn if a note looks like a secret": every changed text file is looked through; found, it stops and you decide.
//! 3. The workspace check (tools/check.py): while it finds problems, nothing is committed or pushed; your changes stay
//!    saved on this PC.
//! 4. Your changes in one commit, as you (your Git config), with your message and nothing added to it.
//! 5. GitHub's changes merged in (pull). Both PCs changed a note: the conflict page.
//! 6. Push.

use crate::check::{self, Check};
use crate::git;
use crate::secrets::{self, Hit};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// The vault isn't a Git repository (any folder can be a vault; it syncs only when it's one).
    NotRepo,
    Secrets(Vec<(String, Hit)>),
    Untidy(Vec<check::Problem>),
    CheckFailed(String),
    Conflict(Vec<String>),
    Done { committed: bool, pulled: bool, pushed: bool },
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub warn_secrets: bool,
    /// The person looked at what was found and said to go on.
    pub allow_secrets: bool,
    pub push: bool,
}

fn texty(path: &str) -> bool {
    let l = path.to_lowercase();
    !(crate::md::is_picture(&l) || [".mp4", ".webm", ".mkv", ".mov", ".zip", ".gz", ".xz", ".pdf", ".ttf", ".otf", ".woff2", ".mp3", ".flac", ".ogg", ".wav"].iter().any(|e| l.ends_with(e)))
}

/// What looks like a secret in the changed files (each file at most 2 MB).
pub fn secret_hits(dir: &Path, changes: &[git::Change]) -> Vec<(String, Hit)> {
    let mut out = Vec::new();
    for c in changes.iter().filter(|c| c.code != 'D' && texty(&c.path)) {
        let p = dir.join(&c.path);
        let Ok(m) = std::fs::metadata(&p) else { continue };
        if !m.is_file() || m.len() > 2 * 1024 * 1024 {
            continue;
        }
        let Ok(b) = std::fs::read(&p) else { continue };
        if b.contains(&0) {
            continue;
        }
        for h in secrets::scan(&String::from_utf8_lossy(&b)) {
            out.push((c.path.clone(), h));
        }
    }
    out
}

pub fn sync(dir: &Path, msg: &str, o: Options) -> Step {
    if !git::is_repo(dir) {
        return Step::NotRepo;
    }
    let open = git::conflicts(dir);
    if !open.is_empty() {
        return Step::Conflict(open);
    }
    let ch = git::changes(dir);
    if o.warn_secrets && !o.allow_secrets {
        let hits = secret_hits(dir, &ch);
        if !hits.is_empty() {
            return Step::Secrets(hits);
        }
    }
    if o.push || !ch.is_empty() {
        match check::run(dir) {
            Check::Untidy(p) => return Step::Untidy(p),
            Check::Failed(e) => return Step::CheckFailed(e),
            _ => {}
        }
    }
    let committed = if ch.is_empty() {
        false
    } else {
        let m = if msg.trim().is_empty() { "Notes from Hearth Notes" } else { msg.trim() };
        match git::commit_all(dir, m) {
            Ok(c) => c,
            Err(e) => return Step::Failed(e),
        }
    };
    let pulled = match git::pull(dir) {
        git::Pull::Conflicts(c) => return Step::Conflict(c),
        git::Pull::Failed(e) => return Step::Failed(e),
        git::Pull::Done(p) => p,
    };
    if o.push {
        let r = git::repo(dir);
        if committed || r.ahead > 0 || !r.upstream {
            if let Err(e) = git::push(dir) {
                return Step::Failed(e);
            }
            return Step::Done { committed, pulled, pushed: true };
        }
    }
    Step::Done { committed, pulled, pushed: false }
}

/// Pull only: GitHub's changes, nothing of this PC's committed or pushed.
pub fn pull_only(dir: &Path) -> Step {
    if !git::is_repo(dir) {
        return Step::NotRepo;
    }
    match git::pull(dir) {
        git::Pull::Conflicts(c) => Step::Conflict(c),
        git::Pull::Failed(e) => Step::Failed(e),
        git::Pull::Done(p) => Step::Done { committed: false, pulled: p, pushed: false },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CHECK: &str = "import os, subprocess, sys\nROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))\nfiles = subprocess.run(['git', 'ls-files', '--cached', '--others', '--exclude-standard'], cwd=ROOT, capture_output=True, text=True).stdout.split()\nitems = {'/'.join(p.split('/')[:3]) for p in files if len(p.split('/')) >= 4}\nbad = [f'{i}: no README.md' for i in sorted(items) if f'{i}/README.md' not in files]\nif bad:\n    print(\"hearth-workspace isn't tidy:\"); [print('  - ' + b) for b in bad]; sys.exit(1)\nprint(f'tidy: {len(items)} items')\n";

    fn g(dir: &Path, args: &[&str]) {
        let o = git::git(dir, args, 60);
        assert!(o.ok, "git {args:?}: {}", o.why());
    }

    /// GitHub (a local bare repository), this PC and the laptop, as Dragongamer9.
    fn world() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let bare = d.path().join("github.git");
        let pc = d.path().join("pc");
        let laptop = d.path().join("laptop");
        std::fs::create_dir_all(&bare).unwrap();
        g(&bare, &["init", "-q", "--bare"]);
        g(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        std::fs::create_dir_all(pc.join("tools")).unwrap();
        std::fs::create_dir_all(pc.join("log")).unwrap();
        std::fs::write(pc.join("tools/check.py"), CHECK).unwrap();
        std::fs::write(pc.join("log/2026-11-20.md"), "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n").unwrap();
        g(&pc, &["init", "-q", "-b", "main"]);
        for dir in [&pc] {
            g(dir, &["config", "user.name", "Dragongamer9"]);
            g(dir, &["config", "user.email", "Dragongamer9@users.noreply.github.com"]);
        }
        g(&pc, &["add", "-A"]);
        g(&pc, &["commit", "-q", "-m", "first"]);
        g(&pc, &["remote", "add", "origin", bare.to_str().unwrap()]);
        g(&pc, &["push", "-q", "-u", "origin", "main"]);
        g(d.path(), &["clone", "-q", bare.to_str().unwrap(), laptop.to_str().unwrap()]);
        g(&laptop, &["config", "user.name", "Dragongamer9"]);
        g(&laptop, &["config", "user.email", "Dragongamer9@users.noreply.github.com"]);
        (d, pc, laptop)
    }

    fn head(dir: &Path) -> String {
        git::git(dir, &["rev-parse", "HEAD"], 10).out.trim().to_string()
    }

    const PUSH: Options = Options { warn_secrets: true, allow_secrets: false, push: true };

    #[test]
    fn sync_commits_as_the_person_and_pushes() {
        let (_d, pc, laptop) = world();
        std::fs::write(pc.join("log/2026-11-20.md"), "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 03:04 Made: Hearth Notes mockups\n").unwrap();
        let ch = git::changes(&pc);
        assert_eq!(ch, vec![git::Change { code: 'M', path: "log/2026-11-20.md".into(), added: 1, removed: 0 }]);
        assert!(git::diff(&pc, "log/2026-11-20.md").iter().any(|l| l.kind == '+' && l.text == "- 03:04 Made: Hearth Notes mockups"));
        assert_eq!(sync(&pc, "Hearth Notes mockups: first draft; log", PUSH), Step::Done { committed: true, pulled: false, pushed: true });
        let msg = git::git(&pc, &["log", "-1", "--format=%an <%ae>%n%B"], 10).out;
        assert_eq!(msg.trim(), "Dragongamer9 <Dragongamer9@users.noreply.github.com>\nHearth Notes mockups: first draft; log", "his name, his message, nothing added");
        assert!(matches!(pull_only(&laptop), Step::Done { pulled: true, .. }));
        assert!(std::fs::read_to_string(laptop.join("log/2026-11-20.md")).unwrap().contains("03:04"));
    }

    #[test]
    fn the_workspace_check_blocks_the_push_until_its_tidy() {
        let (_d, pc, _laptop) = world();
        let before = head(&pc);
        std::fs::create_dir_all(pc.join("hearth/mockups/hearth-notes")).unwrap();
        std::fs::write(pc.join("hearth/mockups/hearth-notes/notes.html"), "<html>").unwrap();
        match sync(&pc, "x", PUSH) {
            Step::Untidy(p) => {
                assert_eq!(p.len(), 1);
                assert_eq!(p[0].path, "hearth/mockups/hearth-notes");
                assert_eq!(p[0].fix, Some(check::Fix::Readme("hearth/mockups/hearth-notes".into())));
            }
            s => panic!("{s:?}"),
        }
        assert_eq!(head(&pc), before, "nothing committed while it's untidy");
        assert!(pc.join("hearth/mockups/hearth-notes/notes.html").exists(), "the change stays saved on this PC");
        // The fix the page offers makes it tidy, and then it goes.
        let _l = crate::env_lock();
        std::env::set_var("XDG_DATA_HOME", _d.path().join("data"));
        check::apply(&pc, &check::Fix::Readme("hearth/mockups/hearth-notes".into()), "draft").unwrap();
        std::env::remove_var("XDG_DATA_HOME");
        assert!(matches!(sync(&pc, "x", PUSH), Step::Done { pushed: true, .. }));
    }

    #[test]
    fn a_secret_stops_the_sync_until_the_person_says() {
        let (_d, pc, _laptop) = world();
        let before = head(&pc);
        std::fs::write(pc.join("log/2026-11-20.md"), format!("# 2026-11-20\n\nthe key ghp_{}\n", "Zz9Yy8Xx7Ww6Vv5Uu4Tt3Ss2Rr1Qq0Pp9Oo8")).unwrap();
        match sync(&pc, "x", PUSH) {
            Step::Secrets(h) => assert_eq!(h[0].0, "log/2026-11-20.md"),
            s => panic!("{s:?}"),
        }
        assert_eq!(head(&pc), before, "nothing committed");
        assert!(matches!(sync(&pc, "x", Options { allow_secrets: true, ..PUSH }), Step::Done { pushed: true, .. }));
    }

    #[test]
    fn a_note_changed_on_two_pcs_keeps_both() {
        let _l = crate::env_lock();
        let (d, pc, laptop) = world();
        std::env::set_var("XDG_DATA_HOME", d.path().join("data"));
        std::fs::write(laptop.join("log/2026-11-20.md"), "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 03:06 Changed: laptop autostart\n").unwrap();
        assert!(matches!(sync(&laptop, "laptop", PUSH), Step::Done { pushed: true, .. }));
        std::fs::write(pc.join("log/2026-11-20.md"), "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 03:04 Made: Hearth Notes mockups\n").unwrap();
        assert_eq!(sync(&pc, "pc", PUSH), Step::Conflict(vec!["log/2026-11-20.md".into()]));
        assert!(git::stage(&pc, 2, "log/2026-11-20.md").unwrap().contains("03:04"));
        assert!(git::stage(&pc, 3, "log/2026-11-20.md").unwrap().contains("03:06"));
        assert!(crate::conflict::settle(&pc, "log/2026-11-20.md", crate::conflict::Pick::Both, "2026-11-20-0310").unwrap());
        let t = std::fs::read_to_string(pc.join("log/2026-11-20.md")).unwrap();
        assert_eq!(t, "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 03:04 Made: Hearth Notes mockups\n- 03:06 Changed: laptop autostart\n");
        assert!(crate::paths::kept().join("2026-11-20-0310/log/2026-11-20.md.this-pc").exists());
        assert!(crate::paths::kept().join("2026-11-20-0310/log/2026-11-20.md.github").exists());
        assert!(matches!(sync(&pc, "pc", PUSH), Step::Done { pushed: true, .. }));
        assert!(matches!(pull_only(&laptop), Step::Done { .. }));
        assert_eq!(std::fs::read_to_string(laptop.join("log/2026-11-20.md")).unwrap(), t, "both PCs have both lines");
        std::env::remove_var("XDG_DATA_HOME");
    }
}
