//! Hearth Files' terminal panel (F4, like Dolphin's; Mantel's FilesTerminal.qml): a command line under the folder
//! you're in. It runs what you type there (sh) and shows what it prints; "cd somewhere" moves Files there too, and
//! moving in Files moves the panel. Not a full terminal (no programs that take over the screen, like nano or htop):
//! "Open in Terminal" is beside it for those. Up / Down: commands you typed before (this session). Ctrl+C stops the
//! one running. Esc goes back to the files.
//!
//! Light: nothing runs until you press Enter; a command is one program whose output is read on a thread of its own
//! (blocked on the pipe), and it's stopped when the panel or the window closes.

use crate::app::{App, Msg, Zone};
use crate::icons::{ico, Ic};
use futures::channel::mpsc;
use futures::StreamExt;
use hearth_ui::ui::{self, hline, t1};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, operation, row, scrollable, text, text_input, Column};
use iced::{mouse, Element, Length, Padding, Task};
use std::io::{BufRead, BufReader};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Context, Poll};

pub const INPUT_ID: &str = "files-term-input";
pub const OUT_ID: &str = "files-term-out";

/// The command running now (its process group), for Ctrl+C.
static PID: AtomicI32 = AtomicI32::new(0);

#[derive(Debug, Clone, Default)]
pub struct Term {
    pub on: bool,
    /// What it printed: (text, kind) with kind "" (output), "cmd" (what you typed) or "err".
    pub lines: Vec<(String, &'static str)>,
    pub history: Vec<String>,
    pub hpos: i64,
    pub input: String,
    /// The command running: (its number, the program to run).
    pub run: Option<(u64, Vec<String>)>,
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub enum TMsg {
    Toggle,
    Close,
    Input(String),
    Submit,
    Out(Line),
    Stop,
    OpenTerminal,
    Focus,
}

#[derive(Debug, Clone)]
pub enum Line {
    Text(u64, String),
    End(u64, i32),
}

impl Term {
    pub fn busy(&self) -> bool {
        self.run.is_some()
    }
    fn add(&mut self, t: &str, kind: &'static str) {
        self.lines.push((t.to_string(), kind));
        if self.lines.len() > 400 {
            let n = self.lines.len() - 400;
            self.lines.drain(..n);
        }
    }
}

/// The folder the panel is in: the folder shown, else your home folder (Home, This PC).
pub fn folder(a: &App) -> String {
    if a.path.starts_with('/') {
        a.path.clone()
    } else {
        a.home.clone()
    }
}

fn resolve(a: &App, p: &str) -> String {
    let mut p = p.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
    if p.is_empty() || p == "~" {
        return a.home.clone();
    }
    if let Some(r) = p.strip_prefix("~/") {
        p = format!("{}/{r}", a.home);
    }
    if !p.starts_with('/') {
        p = format!("{}/{p}", folder(a).trim_end_matches('/'));
    }
    let mut parts: Vec<&str> = Vec::new();
    for s in p.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            x => parts.push(x),
        }
    }
    format!("/{}", parts.join("/"))
}

fn to_end() -> Task<Msg> {
    operation::snap_to_end(OUT_ID)
}

/// Esc and Ctrl+C while the panel's line has the typing: the text box keeps them for itself (Esc leaves it, Ctrl+C
/// copies), so they're read here as Mantel's panel reads them (Esc closes the panel, Ctrl+C stops the command).
pub fn captured_keys(ev: iced::Event, st: iced::event::Status, _w: iced::window::Id) -> Option<Msg> {
    use iced::keyboard::{key::Named, Event as K, Key};
    if st != iced::event::Status::Captured {
        return None;
    }
    match ev {
        iced::Event::Keyboard(K::KeyPressed { key, modifiers, .. }) => match key.as_ref() {
            Key::Named(Named::Escape) => Some(Msg::Term(TMsg::Close)),
            Key::Character(c) if modifiers.control() && c.eq_ignore_ascii_case("c") => Some(Msg::Term(TMsg::Stop)),
            _ => None,
        },
        _ => None,
    }
}

pub fn show(a: &mut App, on: bool) -> Task<Msg> {
    a.term.on = on;
    if on {
        a.zone = Zone::Term;
        return operation::focus(INPUT_ID);
    }
    a.zone = Zone::Main;
    // Closing the panel stops what it runs (its program is dropped with the subscription).
    a.term.run = None;
    Task::none()
}

pub fn update(a: &mut App, m: TMsg) -> Task<Msg> {
    match m {
        TMsg::Toggle => {
            let on = !a.term.on;
            return show(a, on);
        }
        TMsg::Close => return show(a, false),
        TMsg::Focus => {
            a.zone = Zone::Term;
            return operation::focus(INPUT_ID);
        }
        TMsg::Input(t) => {
            a.term.input = t;
            a.zone = Zone::Term;
        }
        TMsg::Submit => {
            let cmd = std::mem::take(&mut a.term.input).trim().to_string();
            if cmd.is_empty() {
                return Task::none();
            }
            a.term.history.retain(|h| *h != cmd);
            a.term.history.push(cmd.clone());
            if a.term.history.len() > 50 {
                a.term.history.remove(0);
            }
            a.term.hpos = -1;
            a.term.add(&format!("$ {cmd}"), "cmd");
            if cmd == "clear" {
                a.term.lines.clear();
                return Task::none();
            }
            if cmd == "cd" || cmd.starts_with("cd ") || cmd.starts_with("cd\t") {
                let to = resolve(a, cmd.get(2..).unwrap_or(""));
                if std::path::Path::new(&to).is_dir() {
                    return a.navigate(&to, "");
                }
                a.term.add(&format!("cd: {to}: no such folder"), "err");
                return to_end();
            }
            if a.term.busy() {
                a.term.add("Still running the last one (Ctrl+C stops it)", "err");
                return to_end();
            }
            a.term.seq += 1;
            let f = folder(a);
            let prog = vec![
                "sh".to_string(),
                "-c".into(),
                "cd -- \"$1\" 2>/dev/null || cd; shift; exec sh -c \"$1\" 2>&1".into(),
                "sh".into(),
                f,
                cmd,
            ];
            a.term.run = Some((a.term.seq, prog));
            return to_end();
        }
        TMsg::Out(l) => match l {
            Line::Text(s, t) => {
                if a.term.run.as_ref().map(|r| r.0) == Some(s) {
                    a.term.add(&t, "");
                    return to_end();
                }
            }
            Line::End(s, code) => {
                if a.term.run.as_ref().map(|r| r.0) == Some(s) {
                    a.term.run = None;
                    if code != 0 {
                        a.term.add(&format!("(exit code {code})"), "err");
                    }
                    return to_end();
                }
            }
        },
        TMsg::Stop => {
            if a.term.busy() {
                let pid = PID.load(Ordering::SeqCst);
                if pid > 0 {
                    unsafe {
                        libc::kill(-pid, libc::SIGINT);
                    }
                }
                a.term.add("^C", "err");
            }
        }
        TMsg::OpenTerminal => {
            let f = folder(a);
            return Task::perform(crate::backend::call(crate::backend::a(&["terminal", &f]), 15), |r| Msg::Did(crate::app::Then::Nothing, r));
        }
    }
    Task::none()
}

/// The keys the panel takes while it has the typing (from menus.rs's key handler).
pub fn key(a: &mut App, named: Option<iced::keyboard::key::Named>, ctrl: bool, ch: Option<&str>) -> Option<Task<Msg>> {
    use iced::keyboard::key::Named;
    if ctrl && ch == Some("c") && a.term.busy() {
        return Some(update(a, TMsg::Stop));
    }
    match named {
        Some(Named::Escape) => Some(show(a, false)),
        Some(Named::ArrowUp) => {
            let n = a.term.history.len() as i64;
            if n == 0 {
                return Some(Task::none());
            }
            a.term.hpos = if a.term.hpos < 0 { n - 1 } else { (a.term.hpos - 1).max(0) };
            a.term.input = a.term.history[a.term.hpos as usize].clone();
            Some(operation::move_cursor_to_end(INPUT_ID))
        }
        Some(Named::ArrowDown) => {
            if a.term.hpos < 0 {
                return Some(Task::none());
            }
            a.term.hpos += 1;
            if a.term.hpos >= a.term.history.len() as i64 {
                a.term.hpos = -1;
                a.term.input.clear();
            } else {
                a.term.input = a.term.history[a.term.hpos as usize].clone();
            }
            Some(operation::move_cursor_to_end(INPUT_ID))
        }
        _ => None,
    }
}

// ------------------------------------------------------------------ the running command
pub struct Lines {
    rx: mpsc::UnboundedReceiver<Line>,
    pid: i32,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl futures::Stream for Lines {
    type Item = Line;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Line>> {
        self.rx.poll_next_unpin(cx)
    }
}

impl Drop for Lines {
    fn drop(&mut self) {
        // The panel closed or the window is closing: what still runs is stopped.
        if self.pid > 0 && !self.done.load(Ordering::SeqCst) {
            unsafe {
                libc::kill(-self.pid, libc::SIGTERM);
            }
        }
        let _ = PID.compare_exchange(self.pid, 0, Ordering::SeqCst, Ordering::SeqCst);
    }
}

pub fn lines(r: &(u64, Vec<String>)) -> Lines {
    use std::os::unix::process::CommandExt;
    let (tx, rx) = mpsc::unbounded();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (seq, cmd) = (r.0, r.1.clone());
    let child = Command::new(&cmd[0]).args(&cmd[1..]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).process_group(0).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.unbounded_send(Line::Text(seq, format!("{}: {e}", cmd[0])));
            let _ = tx.unbounded_send(Line::End(seq, -1));
            done.store(true, Ordering::SeqCst);
            return Lines { rx, pid: 0, done };
        }
    };
    let pid = child.id() as i32;
    PID.store(pid, Ordering::SeqCst);
    let d2 = done.clone();
    let _ = std::thread::Builder::new().name("hearth-files-term".into()).stack_size(256 * 1024).spawn(move || {
        if let Some(out) = child.stdout.take() {
            let mut r = BufReader::new(out);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match r.read_until(b'\n', &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let s = String::from_utf8_lossy(&buf).trim_end_matches(['\n', '\r']).to_string();
                        if tx.unbounded_send(Line::Text(seq, s)).is_err() {
                            break;
                        }
                    }
                }
            }
        }
        let code = child.wait().ok().map(|s| s.code().unwrap_or(130)).unwrap_or(-1);
        d2.store(true, Ordering::SeqCst);
        let _ = tx.unbounded_send(Line::End(seq, code));
    });
    Lines { rx, pid, done }
}

// ------------------------------------------------------------------ the panel
pub fn view(a: &App, height: f32) -> Element<'_, Msg> {
    let p = a.p;
    let f = folder(a);
    let link = |s: &'static str, m: Msg, c: iced::Color| -> Element<'static, Msg> { mouse_area(t1(s, 12.0, c)).on_press(m).interaction(mouse::Interaction::Pointer).into() };
    let mut head = row![
        ico::<Msg>(Ic::Keyboard, p.accent, 14.0),
        container(t1(ui::fit(&format!("Terminal · {f}"), 90), 12.0, p.muted)).width(Length::Fill).clip(true),
    ]
    .spacing(8.0)
    .align_y(iced::Center);
    if a.term.busy() {
        head = head.push(link("Stop (Ctrl+C)", Msg::Term(TMsg::Stop), p.danger));
    }
    head = head.push(link("Open in Terminal", Msg::Term(TMsg::OpenTerminal), p.accent));
    head = head.push(mouse_area(t1("✕", 13.0, p.muted)).on_press(Msg::Term(TMsg::Close)).interaction(mouse::Interaction::Pointer));
    let mut out = Column::new().spacing(0.0).width(Length::Fill);
    for (l, k) in &a.term.lines {
        let c = match *k {
            "cmd" => p.accent,
            "err" => p.danger,
            _ => p.text,
        };
        out = out.push(text(l.clone()).size(13.0).color(c).font(iced::Font::MONOSPACE).wrapping(text::Wrapping::Glyph));
    }
    let out = scrollable(out).id(OUT_ID).height(Length::Fill).width(Length::Fill);
    let focused = a.zone == Zone::Term;
    let prompt = t1(if a.term.busy() { "…" } else { "$" }, 13.0, p.accent).font(iced::Font::MONOSPACE);
    let pp = p;
    let input = text_input("", &a.term.input)
        .id(INPUT_ID)
        .on_input(|s| Msg::Term(TMsg::Input(s)))
        .on_submit(Msg::Term(TMsg::Submit))
        .size(13.0)
        .font(iced::Font::MONOSPACE)
        .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 2.0 })
        .style(move |_t, _st| text_input::Style {
            background: iced::Background::Color(iced::Color::TRANSPARENT),
            border: iced::Border::default(),
            icon: pp.muted,
            placeholder: pp.muted,
            value: pp.text,
            selection: Pal::a(pp.accent, 0.35),
        });
    let field = container(row![prompt, input].spacing(6.0).align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 })
        .height(Length::Fixed(30.0))
        .center_y(Length::Fixed(30.0))
        .width(Length::Fill)
        .style(ui::boxed(p.panel2, p.r(6.0), Some((if focused { p.accent } else { p.line }, 1.0))));
    let field = mouse_area(field).on_press(Msg::Term(TMsg::Focus));
    let body = column![head, out, field].spacing(4.0).padding(Padding { top: 6.0, right: 8.0, bottom: 8.0, left: 8.0 });
    container(column![hline::<Msg>(p.line), body])
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .style(ui::boxed(p.panel, 0.0, None))
        .into()
}
