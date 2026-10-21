//! Hearth's windows, whoever draws them. The core (hearth-core and Game Mode) asks for window things through
//! `WindowHost`; two hosts answer:
//!   - Tauri (main.rs, `TauriHost`): the system's WebKit, in this process. The usual one.
//!   - Electron (`IpcHost`, `hearth --core-stdio`): Chromium, in the Electron app (hosts/electron), which starts this
//!     process windowless and talks to it over its standard input and output, one JSON object a line. Picked on
//!     Nvidia graphics (WebKit's fast drawing fails on RTX 50 cards): docs/RENDERERS.md.
//!
//! The channel (every line is one JSON object with "t"):
//!   core → window app (stdout)
//!     {"t":"msg","to":"main"|"overlay"|"*","msg":{…}}           a reply or event for a page ("*": every window)
//!     {"t":"do","op":"show"|"hide"|"focus"|"unminimize"|"destroy","win":label}
//!     {"t":"do","op":"eval","win":label,"js":"…"}
//!     {"t":"do","op":"exit","code":n} | {"t":"do","op":"restart"} | {"t":"do","op":"zoom","pct":n}
//!     {"t":"do","op":"overlay.create"} | {"t":"do","op":"hotkey"} (Ctrl+Alt+Q, sent back as {"t":"hotkey"})
//!     {"t":"do","op":"stream.open","url","title","windowed"} | {"t":"do","op":"stream.close"}
//!     {"t":"do","op":"signin.open","url","title","script","pad"} | {"t":"do","op":"signin.close"}
//!     {"t":"ask","id":n,"op":"pickFile","title"} | {"t":"ask","id":n,"op":"pickFolder","title","start"}
//!     {"t":"art","id":n,"path":"…"|null,"mime":"…"}           answer to an art request
//!   window app → core (stdin)
//!     {"t":"page","win":label,"msg":{…}}                        what a page posted (nimbusHost.postMessage)
//!     {"t":"win","win":label,"exists":b,"visible":b,"focused":b,"title":"…"}   a window's state, on every change
//!     {"t":"answer","id":n,"value":…}                           a picked file or folder (null: cancelled)
//!     {"t":"signin.nav","url":"…"} | {"t":"signin.closed"}       the sign-in window went somewhere, or was closed
//!     {"t":"stream.exit"} (its page's Exit) | {"t":"stream.closed"} (core_ipc.rs) | {"t":"hotkey"}
//!     {"t":"art","id":n,"uri":"nimbus-art://…"}                  which file a picture is (the window app reads it)
//! Lines that aren't JSON objects with "t" are ignored on both sides (a stray print can't break the channel).

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

pub type Hotkey = Arc<dyn Fn() + Send + Sync>;

pub trait WindowHost: Send + Sync {
    /// A page message (reply or event) to one window ("main", "overlay"...).
    fn emit_to(&self, label: &str, v: Value);
    /// To every window.
    fn emit_all(&self, v: Value);
    fn exists(&self, label: &str) -> bool;
    fn show(&self, label: &str);
    fn hide(&self, label: &str);
    fn focus(&self, label: &str);
    fn unminimize(&self, label: &str);
    fn is_visible(&self, label: &str) -> bool;
    fn is_focused(&self, label: &str) -> bool;
    fn title(&self, label: &str) -> Option<String>;
    fn eval(&self, label: &str, js: &str);
    fn destroy(&self, label: &str);
    /// Leave (0: the session goes on to Desktop Mode or ends; 75: the session's watchdog starts Hearth again).
    fn exit(&self, code: i32);
    /// Start again (outside the session).
    fn restart(&self);
    fn pick_file(&self, title: &str) -> Option<String>;
    fn pick_folder(&self, title: &str, start: &str) -> Option<String>;
    /// A store's sign-in page in its own window; blocks until it ends with a code, or is closed.
    fn sign_in(&self, page: &hearth_core::stores::SignIn) -> Option<String>;
    /// Xbox Cloud Gaming's window: open (url, title) or close (None).
    fn stream(&self, open: Option<(&str, &str)>, windowed: bool);
    fn zoom(&self, pct: u32);
    /// Quick Access's own see-through window over games (not on the Handheld Daemon editions).
    fn create_overlay(&self) -> bool;
    /// Ctrl+Alt+Q, the keyboard's Guide button.
    fn register_hotkey(&self, on: Hotkey);
}

/// What the window app last said about a window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WinState { pub exists: bool, pub visible: bool, pub focused: bool, pub title: String }

/// One line of the channel, as written.
pub fn line(v: &Value) -> String { let mut s = v.to_string(); s.push('\n'); s }

/// One line read: a JSON object with "t", or None (blank, not JSON, or not ours).
pub fn parse(l: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(l.trim()).ok()?;
    if v.is_object() && v["t"].is_string() { Some(v) } else { None }
}

/// The sign-in window's answer from where it went: Some(Some(code)) done, Some(None) cancelled, None keep going.
pub fn signin_outcome(url: &str, done: fn(&str) -> Option<String>) -> Option<Option<String>> {
    if url.starts_with("hearth-signin://cancel") { return Some(None); }
    if url.starts_with("hearth-signin://done") { return Some(hearth_core::http::query_param(url, "code")); }
    done(url).map(Some)
}

struct SignInWait { done: fn(&str) -> Option<String>, tx: Sender<Option<String>> }

/// The Electron window app's side, reached through this process's standard output (answers come in through
/// `IpcHost::incoming`, fed from standard input by core_ipc.rs).
pub struct IpcHost {
    out: Mutex<Box<dyn Write + Send>>,
    wins: Mutex<HashMap<String, WinState>>,
    next: AtomicU64,
    asks: Mutex<HashMap<u64, Sender<Value>>>,
    signin: Mutex<Option<SignInWait>>,
    hotkey: Mutex<Option<Hotkey>>,
}

impl IpcHost {
    pub fn new(out: Box<dyn Write + Send>) -> IpcHost {
        IpcHost { out: Mutex::new(out), wins: Mutex::new(HashMap::new()), next: AtomicU64::new(1), asks: Mutex::new(HashMap::new()),
                  signin: Mutex::new(None), hotkey: Mutex::new(None) }
    }
    pub fn send(&self, v: Value) {
        if let Ok(mut o) = self.out.lock() { let _ = o.write_all(line(&v).as_bytes()); let _ = o.flush(); }
    }
    fn act(&self, op: &str, win: &str) { self.send(json!({"t": "do", "op": op, "win": win})); }
    fn state(&self, label: &str) -> WinState { self.wins.lock().ok().and_then(|w| w.get(label).cloned()).unwrap_or_default() }
    /// Asks the window app and waits for its answer (a file picker can stay open as long as it likes).
    fn ask(&self, mut v: Value) -> Value {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = channel();
        self.asks.lock().unwrap().insert(id, tx);
        v["t"] = json!("ask"); v["id"] = json!(id);
        self.send(v);
        let r = rx.recv().unwrap_or(Value::Null);
        self.asks.lock().unwrap().remove(&id);
        r
    }

    /// A line from the window app that isn't a page message (core_ipc.rs handles those): window state, answers,
    /// the sign-in window's navigation, the hotkey. Returns false when it wasn't one of these.
    pub fn incoming(&self, v: &Value) -> bool {
        match v["t"].as_str().unwrap_or("") {
            "win" => {
                let label = v["win"].as_str().unwrap_or("").to_string();
                let st = WinState { exists: v["exists"] != false, visible: v["visible"] == true, focused: v["focused"] == true,
                                    title: v["title"].as_str().unwrap_or("").to_string() };
                if let Ok(mut w) = self.wins.lock() { w.insert(label, st); }
            }
            "answer" => {
                let id = v["id"].as_u64().unwrap_or(0);
                if let Some(tx) = self.asks.lock().unwrap().remove(&id) { let _ = tx.send(v["value"].clone()); }
            }
            "signin.nav" => {
                let url = v["url"].as_str().unwrap_or("");
                let mut s = self.signin.lock().unwrap();
                if let Some(w) = s.as_ref() {
                    if let Some(answer) = signin_outcome(url, w.done) { let _ = w.tx.send(answer); *s = None; }
                }
            }
            "signin.closed" => { if let Some(w) = self.signin.lock().unwrap().take() { let _ = w.tx.send(None); } }
            "hotkey" => { if let Some(h) = self.hotkey.lock().unwrap().clone() { std::thread::spawn(move || h()); } }
            "stream.exit" => self.stream(None, false),
            _ => return false,
        }
        true
    }
}

impl WindowHost for IpcHost {
    fn emit_to(&self, label: &str, v: Value) { self.send(json!({"t": "msg", "to": label, "msg": v})); }
    fn emit_all(&self, v: Value) { self.send(json!({"t": "msg", "to": "*", "msg": v})); }
    fn exists(&self, label: &str) -> bool { self.state(label).exists }
    fn show(&self, label: &str) { self.act("show", label); }
    fn hide(&self, label: &str) { self.act("hide", label); }
    fn focus(&self, label: &str) { self.act("focus", label); }
    fn unminimize(&self, label: &str) { self.act("unminimize", label); }
    fn is_visible(&self, label: &str) -> bool { self.state(label).visible }
    fn is_focused(&self, label: &str) -> bool { self.state(label).focused }
    fn title(&self, label: &str) -> Option<String> { let s = self.state(label); if s.exists { Some(s.title) } else { None } }
    fn eval(&self, label: &str, js: &str) { self.send(json!({"t": "do", "op": "eval", "win": label, "js": js})); }
    fn destroy(&self, label: &str) { self.act("destroy", label); }
    fn exit(&self, code: i32) {
        self.send(json!({"t": "do", "op": "exit", "code": code}));
        // The window app closes and so does this process; if it doesn't answer, leave anyway.
        std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(5)); std::process::exit(code); });
    }
    fn restart(&self) { self.send(json!({"t": "do", "op": "restart"})); }
    fn pick_file(&self, title: &str) -> Option<String> {
        self.ask(json!({"op": "pickFile", "title": title})).as_str().filter(|p| p.starts_with('/')).map(str::to_string)
    }
    fn pick_folder(&self, title: &str, start: &str) -> Option<String> {
        self.ask(json!({"op": "pickFolder", "title": title, "start": start})).as_str().filter(|p| p.starts_with('/')).map(str::to_string)
    }
    fn sign_in(&self, page: &hearth_core::stores::SignIn) -> Option<String> {
        let (tx, rx) = channel();
        { let mut s = self.signin.lock().unwrap(); if let Some(old) = s.take() { let _ = old.tx.send(None); } *s = Some(SignInWait { done: page.done, tx }); }
        let pad = crate::signin_pad_wanted();
        self.send(json!({"t": "do", "op": "signin.open", "url": page.url, "title": page.title, "script": page.script.unwrap_or(""),
                         "pad": pad, "fullscreen": crate::in_session() && !hearth_core::signin_handoff::windowed()}));
        if let Some(gm) = crate::GM.get() { gm.to_front(page.title); }
        let answer = rx.recv().ok().flatten();
        self.send(json!({"t": "do", "op": "signin.close"}));
        if let Some(gm) = crate::GM.get() { gm.come_back(); }
        answer
    }
    fn stream(&self, open: Option<(&str, &str)>, windowed: bool) {
        match open {
            Some((url, title)) => self.send(json!({"t": "do", "op": "stream.open", "url": url, "title": title, "windowed": windowed})),
            None => self.send(json!({"t": "do", "op": "stream.close"})),
        }
    }
    fn zoom(&self, pct: u32) { self.send(json!({"t": "do", "op": "zoom", "pct": pct})); }
    fn create_overlay(&self) -> bool { self.send(json!({"t": "do", "op": "overlay.create"})); true }
    fn register_hotkey(&self, on: Hotkey) { *self.hotkey.lock().unwrap() = Some(on); self.send(json!({"t": "do", "op": "hotkey"})); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);
    impl Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { self.0.lock().unwrap().extend_from_slice(b); Ok(b.len()) }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    fn lines(b: &Buf) -> Vec<Value> { String::from_utf8(b.0.lock().unwrap().clone()).unwrap().lines().filter_map(parse).collect() }

    #[test]
    fn framing() {
        assert_eq!(line(&json!({"t": "msg"})), "{\"t\":\"msg\"}\n");
        assert!(parse("{\"t\":\"page\",\"win\":\"main\",\"msg\":{}}").is_some());
        assert!(parse("Gtk-WARNING something").is_none(), "stray output is ignored");
        assert!(parse("{\"no\":1}").is_none() && parse("[1]").is_none() && parse("").is_none());
    }

    #[test]
    fn window_state_and_requests() {
        let b = Buf::default();
        let h = IpcHost::new(Box::new(b.clone()));
        assert!(!h.exists("main") && !h.is_visible("main"));
        assert!(h.incoming(&json!({"t": "win", "win": "main", "exists": true, "visible": true, "focused": false, "title": "Hearth"})));
        assert!(h.exists("main") && h.is_visible("main") && !h.is_focused("main"));
        assert_eq!(h.title("main").as_deref(), Some("Hearth"));
        assert!(h.incoming(&json!({"t": "win", "win": "main", "exists": false})));
        assert!(!h.exists("main") && h.title("main").is_none());
        h.show("main"); h.emit_to("overlay", json!({"type": "event"})); h.emit_all(json!({"x": 1}));
        let out = lines(&b);
        assert_eq!(out[0], json!({"t": "do", "op": "show", "win": "main"}));
        assert_eq!(out[1], json!({"t": "msg", "to": "overlay", "msg": {"type": "event"}}));
        assert_eq!(out[2]["to"], "*");
        assert!(!h.incoming(&json!({"t": "page"})), "page messages are core_ipc's");
    }

    #[test]
    fn asking_waits_for_the_answer() {
        let b = Buf::default();
        let h = Arc::new(IpcHost::new(Box::new(b.clone())));
        let h2 = h.clone();
        let t = std::thread::spawn(move || h2.pick_file("Choose"));
        for _ in 0..100 { if !lines(&b).is_empty() { break; } std::thread::sleep(std::time::Duration::from_millis(10)); }
        let ask = lines(&b)[0].clone();
        assert_eq!((ask["t"].as_str(), ask["op"].as_str()), (Some("ask"), Some("pickFile")));
        h.incoming(&json!({"t": "answer", "id": ask["id"], "value": "/home/me/setup.exe"}));
        assert_eq!(t.join().unwrap().as_deref(), Some("/home/me/setup.exe"));
    }

    #[test]
    fn sign_in_outcomes() {
        fn done(u: &str) -> Option<String> { u.strip_prefix("https://example.com/ok?c=").map(str::to_string) }
        assert_eq!(signin_outcome("https://example.com/login", done), None);
        assert_eq!(signin_outcome("https://example.com/ok?c=abc", done), Some(Some("abc".into())));
        assert_eq!(signin_outcome("hearth-signin://cancel", done), Some(None));
        assert_eq!(signin_outcome("hearth-signin://done?code=xyz", done), Some(Some("xyz".into())));
    }
}
