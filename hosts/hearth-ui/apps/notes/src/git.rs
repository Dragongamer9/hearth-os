//! Git, as the PC already has it: the person's own Git config (user.name, user.email) and sign-in (GitHub CLI's, or
//! Git's credential helper, or an SSH key). Hearth Notes never asks for, stores or sends a password or token: with
//! GIT_TERMINAL_PROMPT=0 Git can't ask either, it just says it isn't signed in. Every run has a time limit and runs
//! off the window's thread.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct Out {
    pub ok: bool,
    pub code: i32,
    pub out: String,
    pub err: String,
}

impl Out {
    pub fn why(&self) -> String {
        let t = format!("{}\n{}", self.err, self.out);
        let lines: Vec<&str> = t.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("hint:")).collect();
        lines.iter().rev().take(2).rev().cloned().collect::<Vec<_>>().join(" ").trim_start_matches("fatal: ").trim_start_matches("error: ").to_string()
    }
}

/// Runs a program with a time limit (killed after it).
pub fn run(cmd: &mut Command, secs: u64) -> Out {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Out { ok: false, code: -1, out: String::new(), err: e.to_string() },
    };
    // Read both pipes on their own threads so a big output can't fill a pipe and stall.
    let mut so = child.stdout.take();
    let mut se = child.stderr.take();
    let t1 = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = so.as_mut() {
            let _ = std::io::Read::read_to_string(p, &mut s);
        }
        s
    });
    let t2 = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = se.as_mut() {
            let _ = std::io::Read::read_to_string(p, &mut s);
        }
        s
    });
    let end = Instant::now() + Duration::from_secs(secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if Instant::now() >= end => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => break None,
        }
    };
    let out = t1.join().unwrap_or_default();
    let err = t2.join().unwrap_or_default();
    match status {
        Some(s) => Out { ok: s.success(), code: s.code().unwrap_or(-1), out, err },
        None => Out { ok: false, code: -1, out, err: format!("{err}\nit took longer than {secs} s and was stopped") },
    }
}

pub fn git(dir: &Path, args: &[&str], secs: u64) -> Out {
    let mut c = Command::new("git");
    c.arg("-C").arg(dir).args(args).env("GIT_TERMINAL_PROMPT", "0").env("LC_ALL", "C").env("GIT_EDITOR", "true").env("GIT_MERGE_AUTOEDIT", "no");
    run(&mut c, secs)
}

pub fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists() && git(dir, &["rev-parse", "--is-inside-work-tree"], 10).out.trim() == "true"
}

pub fn has_git() -> bool {
    run(Command::new("git").arg("--version"), 10).ok
}

#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// 'M' changed, 'A' new, 'D' deleted, 'R' renamed, 'U' in a conflict.
    pub code: char,
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

/// What's changed on this PC since the last commit (untracked files count as new, with their lines).
pub fn changes(dir: &Path) -> Vec<Change> {
    let st = git(dir, &["status", "--porcelain=v1", "-z", "--untracked-files=all"], 30);
    if !st.ok {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut parts = st.out.split('\0').filter(|s| !s.is_empty());
    while let Some(entry) = parts.next() {
        if entry.len() < 4 {
            continue;
        }
        let (x, y) = (entry.as_bytes()[0] as char, entry.as_bytes()[1] as char);
        let path = entry[3..].to_string();
        if x == 'R' || x == 'C' {
            parts.next();
        }
        let code = match (x, y) {
            ('?', '?') => 'A',
            ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D') => 'U',
            ('A', _) => 'A',
            ('D', _) | (_, 'D') => 'D',
            ('R', _) => 'R',
            _ => 'M',
        };
        out.push(Change { code, path, added: 0, removed: 0 });
    }
    let ns = git(dir, &["diff", "HEAD", "--numstat"], 30);
    for l in ns.out.lines() {
        let f: Vec<&str> = l.splitn(3, '\t').collect();
        if f.len() == 3 {
            if let Some(c) = out.iter_mut().find(|c| c.path == f[2]) {
                c.added = f[0].parse().unwrap_or(0);
                c.removed = f[1].parse().unwrap_or(0);
            }
        }
    }
    for c in out.iter_mut().filter(|c| c.code == 'A' && c.added == 0) {
        if let Ok(t) = std::fs::read(dir.join(&c.path)) {
            c.added = String::from_utf8_lossy(&t).lines().count() as u32;
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    /// ' ' unchanged, '+' added, '-' removed.
    pub kind: char,
    /// Its line number in the new file (or the old one for a removed line).
    pub no: u32,
    pub text: String,
}

/// A file's changes since the last commit, with a few lines around each.
pub fn diff(dir: &Path, path: &str) -> Vec<DiffLine> {
    let tracked = git(dir, &["ls-files", "--error-unmatch", "--", path], 10).ok;
    if !tracked {
        let t = std::fs::read(dir.join(path)).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        return t.lines().enumerate().map(|(i, l)| DiffLine { kind: '+', no: i as u32 + 1, text: l.to_string() }).collect();
    }
    let d = git(dir, &["diff", "HEAD", "-U3", "--no-color", "--", path], 30);
    parse_diff(&d.out)
}

pub fn parse_diff(text: &str) -> Vec<DiffLine> {
    let mut out = Vec::new();
    let (mut old, mut new) = (0u32, 0u32);
    let mut in_hunk = false;
    for l in text.lines() {
        if let Some(h) = l.strip_prefix("@@ ") {
            // @@ -a,b +c,d @@
            let mut it = h.split_whitespace();
            old = it.next().and_then(|s| s.trim_start_matches('-').split(',').next()?.parse().ok()).unwrap_or(1);
            new = it.next().and_then(|s| s.trim_start_matches('+').split(',').next()?.parse().ok()).unwrap_or(1);
            in_hunk = true;
            continue;
        }
        if !in_hunk || l.starts_with("\\ ") {
            continue;
        }
        if let Some(t) = l.strip_prefix('+') {
            out.push(DiffLine { kind: '+', no: new, text: t.to_string() });
            new += 1;
        } else if let Some(t) = l.strip_prefix('-') {
            out.push(DiffLine { kind: '-', no: old, text: t.to_string() });
            old += 1;
        } else {
            out.push(DiffLine { kind: ' ', no: new, text: l.strip_prefix(' ').unwrap_or(l).to_string() });
            old += 1;
            new += 1;
        }
    }
    out
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Repo {
    pub branch: String,
    /// "Dragongamer9/hearth-workspace" (from origin's address), or the address itself.
    pub remote: String,
    pub upstream: bool,
    pub ahead: u32,
    pub behind: u32,
}

/// owner/repo from a GitHub address (https or ssh).
pub fn slug(url: &str) -> String {
    let u = url.trim().trim_end_matches('/').trim_end_matches(".git");
    for p in ["https://github.com/", "http://github.com/", "git@github.com:", "ssh://git@github.com/"] {
        if let Some(r) = u.strip_prefix(p) {
            return r.to_string();
        }
    }
    u.to_string()
}

pub fn repo(dir: &Path) -> Repo {
    let branch = git(dir, &["rev-parse", "--abbrev-ref", "HEAD"], 10).out.trim().to_string();
    let remote = slug(&git(dir, &["config", "--get", "remote.origin.url"], 10).out);
    let ab = git(dir, &["rev-list", "--left-right", "--count", "HEAD...@{u}"], 10);
    let mut r = Repo { branch, remote, upstream: ab.ok, ..Default::default() };
    if ab.ok {
        let n: Vec<u32> = ab.out.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        if n.len() == 2 {
            r.ahead = n[0];
            r.behind = n[1];
        }
    }
    r
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

/// Who commits: the person's own Git config.
pub fn identity(dir: &Path) -> Identity {
    Identity { name: git(dir, &["config", "user.name"], 10).out.trim().to_string(), email: git(dir, &["config", "user.email"], 10).out.trim().to_string() }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Auth {
    /// GitHub CLI is signed in (as this account).
    Gh(String),
    /// Git's credential helper (its name) keeps the sign-in.
    Helper(String),
    /// The address is SSH: the PC's key.
    Ssh,
    None,
}

impl Auth {
    pub fn signed_in(&self) -> bool {
        !matches!(self, Auth::None)
    }
}

/// How this PC signs in to GitHub (asked, never stored).
pub fn auth(dir: &Path) -> Auth {
    let gh = run(Command::new("gh").args(["auth", "status", "--hostname", "github.com"]).env("GH_PROMPT_DISABLED", "1").env("NO_COLOR", "1"), 15);
    if gh.ok {
        let t = format!("{}\n{}", gh.out, gh.err);
        let user = t
            .lines()
            .find_map(|l| l.split(" account ").nth(1).or_else(|| l.split("Logged in to github.com as ").nth(1)))
            .map(|r| r.split_whitespace().next().unwrap_or("").to_string())
            .unwrap_or_default();
        return Auth::Gh(user);
    }
    let helper = git(dir, &["config", "--get", "credential.helper"], 10).out.trim().to_string();
    if !helper.is_empty() {
        return Auth::Helper(helper);
    }
    let url = git(dir, &["config", "--get", "remote.origin.url"], 10).out;
    if url.trim().starts_with("git@") || url.trim().starts_with("ssh://") {
        return Auth::Ssh;
    }
    Auth::None
}

/// How to sign in, in words (when Auth::None).
pub const SIGN_IN_HELP: &str = "Sign in once with GitHub's own tool in a terminal: gh auth login (or set up Git's credential helper). Hearth Notes then uses that sign-in; it never asks for or keeps a password or token.";

/// Files in a conflict (both PCs changed them).
pub fn conflicts(dir: &Path) -> Vec<String> {
    git(dir, &["diff", "--name-only", "--diff-filter=U"], 15).out.lines().map(str::to_string).filter(|l| !l.is_empty()).collect()
}

/// A conflicted file's version: 2 is this PC's, 3 is the other one's.
pub fn stage(dir: &Path, n: u8, path: &str) -> Option<String> {
    let o = git(dir, &["show", &format!(":{n}:{path}")], 15);
    o.ok.then_some(o.out)
}

/// The commit the other side came from (short) and when, for the conflict page.
pub fn theirs_commit(dir: &Path) -> (String, i64) {
    let o = git(dir, &["log", "-1", "--format=%h %ct", "MERGE_HEAD"], 10);
    let mut it = o.out.split_whitespace();
    (it.next().unwrap_or("").to_string(), it.next().and_then(|s| s.parse().ok()).unwrap_or(0))
}

/// Everything changed, in one commit with the person's own name and message (nothing added to it).
pub fn commit_all(dir: &Path, msg: &str) -> Result<bool, String> {
    let a = git(dir, &["add", "-A"], 60);
    if !a.ok {
        return Err(a.why());
    }
    if git(dir, &["diff", "--cached", "--quiet"], 30).ok {
        return Ok(false);
    }
    let c = git(dir, &["commit", "-q", "-m", msg], 60);
    if !c.ok {
        return Err(c.why());
    }
    Ok(true)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pull {
    Done(bool),
    Conflicts(Vec<String>),
    Failed(String),
}

/// GitHub's changes, merged (never rebased, so nothing of this PC's is rewritten).
pub fn pull(dir: &Path) -> Pull {
    let before = git(dir, &["rev-parse", "HEAD"], 10).out;
    let p = git(dir, &["pull", "--no-rebase", "--no-edit", "--autostash", "-q"], 120);
    let c = conflicts(dir);
    if !c.is_empty() {
        return Pull::Conflicts(c);
    }
    if !p.ok {
        return Pull::Failed(p.why());
    }
    Pull::Done(git(dir, &["rev-parse", "HEAD"], 10).out != before)
}

pub fn push(dir: &Path) -> Result<(), String> {
    let p = git(dir, &["push", "-q"], 120);
    if p.ok { Ok(()) } else { Err(p.why()) }
}

pub fn fetch(dir: &Path) -> Result<(), String> {
    let p = git(dir, &["fetch", "-q"], 60);
    if p.ok { Ok(()) } else { Err(p.why()) }
}

/// When this PC last had GitHub's version (the upstream's newest commit it knows).
pub fn last_sync(dir: &Path) -> i64 {
    let fh = dir.join(".git/FETCH_HEAD");
    if let Ok(m) = std::fs::metadata(&fh) {
        if let Some(t) = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
            return t.as_secs() as i64;
        }
    }
    git(dir, &["log", "-1", "--format=%ct", "@{u}"], 10).out.trim().parse().unwrap_or(0)
}

/// Clones `slug_or_url` into `dest` with the PC's sign-in: GitHub CLI when it's signed in, else plain Git.
pub fn clone(slug_or_url: &str, dest: &Path, auth: &Auth) -> Result<(), String> {
    if dest.exists() && std::fs::read_dir(dest).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(format!("{} already has files in it", dest.display()));
    }
    let o = match auth {
        Auth::Gh(_) if !slug_or_url.contains("://") && !slug_or_url.starts_with("git@") => {
            run(Command::new("gh").args(["repo", "clone", slug_or_url]).arg(dest).env("GIT_TERMINAL_PROMPT", "0").env("GH_PROMPT_DISABLED", "1"), 600)
        }
        _ => {
            let url = if slug_or_url.contains("://") || slug_or_url.starts_with("git@") || slug_or_url.starts_with('/') { slug_or_url.to_string() } else { format!("https://github.com/{slug_or_url}") };
            run(Command::new("git").arg("clone").arg("-q").arg(&url).arg(dest).env("GIT_TERMINAL_PROMPT", "0"), 600)
        }
    };
    if o.ok { Ok(()) } else { Err(o.why()) }
}
