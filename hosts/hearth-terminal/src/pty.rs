//! A shell running in a pseudo-terminal: a Unix pty on Linux, ConPTY on Windows (both through portable-pty).
//! Output arrives as text (whole UTF-8 characters) on a callback; input, resizing and closing are methods.

use crate::utf8::Utf8Chunker;
use anyhow::{Context, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

/// What to run and how big the terminal is.
#[derive(Clone, Debug, Default)]
pub struct SpawnOptions {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    /// Variables of ours the shell shouldn't inherit (the drawing settings main.rs sets for WebKit).
    pub env_remove: Vec<String>,
    pub cols: u16,
    pub rows: u16,
}

/// What the shell sends back.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Output text, including the terminal's escape sequences.
    Data(String),
    /// The program ended (its exit code when it has one). Comes after its last output.
    Exit(Option<u32>),
}

pub struct Pty {
    master: Mutex<Box<dyn MasterPty + Send>>,
    /// Typed text goes to a thread of its own that writes it: a program that isn't reading (a paste while
    /// `sleep` runs) fills the terminal's buffer, and the write then waits; done on the window's thread, that
    /// froze the whole window, every tab with it.
    writer: Mutex<mpsc::Sender<Vec<u8>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    pid: Option<u32>,
    exited: Arc<AtomicBool>,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize { cols: cols.max(2), rows: rows.max(1), pixel_width: 0, pixel_height: 0 }
}

impl Pty {
    /// Starts `opts.program` in a new pseudo-terminal. `on_event` is called from background threads.
    pub fn spawn<F>(opts: SpawnOptions, on_event: F) -> Result<Pty>
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let pair = native_pty_system()
            .openpty(size(if opts.cols == 0 { 80 } else { opts.cols }, if opts.rows == 0 { 24 } else { opts.rows }))
            .context("couldn't open a pseudo-terminal")?;

        let mut cmd = CommandBuilder::new(&opts.program);
        cmd.args(&opts.args);
        // A folder that's gone (or none at all) means your home folder, like a fresh terminal.
        let cwd = opts.cwd.filter(|d| d.is_dir()).or_else(dirs::home_dir);
        if let Some(d) = &cwd { cmd.cwd(d); }
        // What programs look at to decide on colors and features: 256 colors and 24-bit color both work.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "HearthTerminal");
        cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        for k in &opts.env_remove { cmd.env_remove(k); }
        for (k, v) in &opts.env { cmd.env(k, v); }

        let mut child = pair.slave.spawn_command(cmd).with_context(|| format!("couldn't start {}", opts.program))?;
        // The shell has its own copy of the terminal's other end; ours must close so the end of output is seen.
        drop(pair.slave);
        let pid = child.process_id();
        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader().context("couldn't read from the terminal")?;
        let mut writer = pair.master.take_writer().context("couldn't write to the terminal")?;
        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();
        std::thread::Builder::new().name("pty-write".into()).spawn(move || {
            for data in write_rx {
                if writer.write_all(&data).and_then(|_| writer.flush()).is_err() { break; }
            }
        })?;

        let on_event = Arc::new(on_event);
        let exited = Arc::new(AtomicBool::new(false));
        let (done_tx, done_rx) = mpsc::channel::<()>();

        // Output: read, turn into whole characters, hand on.
        {
            let on_event = on_event.clone();
            std::thread::Builder::new().name("pty-read".into()).spawn(move || {
                let mut text = Utf8Chunker::new();
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let s = text.push(&buf[..n]);
                            if !s.is_empty() { on_event(Event::Data(s)); }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,   // Linux says EIO once the shell and everything it started are gone
                    }
                }
                let s = text.finish();
                if !s.is_empty() { on_event(Event::Data(s)); }
                let _ = done_tx.send(());
            })?;
        }

        // The end: wait for the program, let its last output through, then say it ended.
        {
            let exited = exited.clone();
            std::thread::Builder::new().name("pty-wait".into()).spawn(move || {
                let code = child.wait().ok().map(|s| s.exit_code());
                // On Linux the output ends with the program; ConPTY keeps it open until the terminal closes, so
                // only wait a moment for it there.
                let _ = done_rx.recv_timeout(Duration::from_millis(if cfg!(windows) { 300 } else { 1500 }));
                exited.store(true, Ordering::SeqCst);
                on_event(Event::Exit(code));
            })?;
        }

        Ok(Pty {
            master: Mutex::new(pair.master),
            writer: Mutex::new(write_tx),
            killer: Mutex::new(killer),
            pid,
            exited,
        })
    }

    /// Types into the terminal (keys, pasted text, escape sequences). Returns at once; the text is written in
    /// order as the program takes it.
    pub fn write(&self, data: &[u8]) -> Result<()> {
        if data.is_empty() { return Ok(()); }
        self.writer.lock().unwrap_or_else(|e| e.into_inner()).send(data.to_vec())
            .map_err(|_| anyhow::anyhow!("the terminal is closed"))
    }

    /// The terminal changed size: the shell and the programs in it are told (SIGWINCH / ConPTY resize).
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master.lock().unwrap().resize(size(cols, rows))
    }

    /// The size the terminal has now, as (columns, rows).
    pub fn size(&self) -> Option<(u16, u16)> {
        self.master.lock().unwrap().get_size().ok().map(|s| (s.cols, s.rows))
    }

    pub fn pid(&self) -> Option<u32> { self.pid }

    pub fn has_exited(&self) -> bool { self.exited.load(Ordering::SeqCst) }

    /// The shell's current folder, for opening a new tab in the same place. Linux reads it from /proc; on
    /// Windows the page learns it from the prompt instead (OSC 9;9), so this has nothing.
    pub fn cwd(&self) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let pid = self.pid?;
            std::fs::read_link(Path::new("/proc").join(pid.to_string()).join("cwd")).ok()
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = Path::new("");
            None
        }
    }

    /// Ends the program (closing the tab).
    pub fn kill(&self) {
        if !self.has_exited() { let _ = self.killer.lock().unwrap().kill(); }
    }
}

impl Drop for Pty {
    fn drop(&mut self) { self.kill(); }
}
