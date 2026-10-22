//! A real shell through the pseudo-terminal (a Unix pty on Linux, ConPTY on Windows): type a command, read its
//! output back, resize, and see the end. CI runs these on ubuntu-24.04 and windows-latest.

use hearth_terminal::pty::{Event, Pty, SpawnOptions};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(30);

struct Session {
    pty: Pty,
    rx: mpsc::Receiver<Event>,
    raw: String,
    exited: Option<Option<u32>>,
}

/// The output without escape sequences (colors, cursor moves, titles), which ConPTY sprinkles in.
fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\x1b' { if c != '\r' { out.push(c); } continue; }
        match it.next() {
            // ConPTY moves the cursor to the next line instead of sending a new line: count that as one.
            Some('[') => { while let Some(n) = it.next() { if ('@'..='~').contains(&n) { if n == 'H' || n == 'f' { out.push('\n'); } break; } } }
            Some(']') => {
                while let Some(n) = it.next() {
                    if n == '\x07' { break; }
                    if n == '\x1b' { it.next(); break; }
                }
            }
            _ => {}
        }
    }
    out
}

impl Session {
    fn start(cols: u16, rows: u16) -> Session {
        let (program, args) = if cfg!(windows) {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
            (format!(r"{root}\System32\cmd.exe"), vec!["/q".to_string()])
        } else {
            ("/bin/sh".to_string(), vec![])
        };
        let (tx, rx) = mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        let opts = SpawnOptions { program, args, cols, rows, ..Default::default() };
        let pty = Pty::spawn(opts, move |e| { let _ = tx.lock().unwrap().send(e); }).expect("the shell starts");
        Session { pty, rx, raw: String::new(), exited: None }
    }

    // The shell's prompt ("$ ") can land at the start of an output line when the command was typed before the
    // first prompt appeared, so the checks look at how lines end.
    fn send(&self, s: &str) { self.pty.write(s.as_bytes()).expect("typing works"); }

    /// Reads output until `done` says the plain text is complete. Answers ConPTY's cursor question on the way
    /// (it asks where the cursor is before showing anything, as xterm.js answers in the app).
    fn until(&mut self, what: &str, done: impl Fn(&str) -> bool) -> String {
        let end = Instant::now() + WAIT;
        let mut answered = 0;
        loop {
            let asks = self.raw.matches("\x1b[6n").count();
            while answered < asks { self.send("\x1b[1;1R"); answered += 1; }
            let text = plain(&self.raw);
            if done(&text) { return text; }
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() { panic!("no {what} after {WAIT:?}; the terminal showed:\n{text}\n(raw: {:?})", self.raw); }
            match self.rx.recv_timeout(left.min(Duration::from_millis(200))) {
                Ok(Event::Data(d)) => self.raw.push_str(&d),
                Ok(Event::Exit(code)) => self.exited = Some(code),
                Err(_) => {}
            }
        }
    }
}

#[test]
fn echo_hello_comes_back() {
    let mut s = Session::start(80, 24);
    // The quote (^ in Command Prompt) keeps the typed line from matching: only the real output says "hello".
    s.send(if cfg!(windows) { "echo hel^lo\r" } else { "echo hel''lo\r" });
    s.until("hello", |t| t.lines().any(|l| l.trim_end().ends_with("hello") && !l.contains("hel'") && !l.contains("hel^")));
}

#[test]
fn resize_reaches_the_shell() {
    let mut s = Session::start(80, 24);
    s.pty.resize(100, 40).expect("resize works");
    assert_eq!(s.pty.size(), Some((100, 40)));
    if cfg!(windows) {
        // Command Prompt's own report of the console size.
        s.send("mode con\r");
        s.until("the new width from `mode con`", |t| {
            t.split("Columns").nth(1).map(|rest| rest.lines().next().unwrap_or("").contains("100")).unwrap_or(false)
        });
    } else {
        s.send("stty size\r");
        s.until("`40 100` from stty", |t| t.lines().any(|l| l.trim_end().ends_with("40 100") && !l.contains("stty")));
    }
}

#[test]
fn exit_is_reported() {
    let mut s = Session::start(80, 24);
    s.send("exit\r");
    let end = Instant::now() + WAIT;
    while s.exited.is_none() {
        assert!(Instant::now() < end, "the shell's end wasn't reported; output:\n{}", plain(&s.raw));
        match s.rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Event::Data(d)) => {
                s.raw.push_str(&d);
                if s.raw.contains("\x1b[6n") && !s.raw.contains("answered") { s.send("\x1b[1;1R"); s.raw.push_str("answered"); }
            }
            Ok(Event::Exit(code)) => s.exited = Some(code),
            Err(_) => {}
        }
    }
    assert_eq!(s.exited, Some(Some(0)));
    assert!(s.pty.has_exited());
}

#[cfg(unix)]
#[test]
fn utf8_and_colors_pass_through() {
    let mut s = Session::start(80, 24);
    // A check mark (three bytes) and a 24-bit color: both arrive intact.
    s.send("printf '\\342\\234\\223 \\033[38;2;255;138;61mok\\033[0m\\n'\r");
    s.until("the check mark", |t| t.lines().any(|l| l.trim_end().ends_with("✓ ok") && !l.contains("printf")));
    assert!(s.raw.contains("\x1b[38;2;255;138;61m"));
}

#[cfg(target_os = "linux")]
#[test]
fn cwd_follows_the_shell() {
    let mut s = Session::start(80, 24);
    s.send("cd /tmp && echo moved''here\r");
    s.until("the cd", |t| t.lines().any(|l| l.trim_end().ends_with("movedhere") && !l.contains("moved'")));
    let cwd = s.pty.cwd().expect("the shell's folder");
    assert_eq!(cwd.canonicalize().unwrap(), std::path::Path::new("/tmp").canonicalize().unwrap());
}
