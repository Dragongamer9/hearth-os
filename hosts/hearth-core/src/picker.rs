//! File and folder pickers Hearth opens for itself (Library › Stores' install folder, Add a game, a profile
//! picture, Beacon's installers and mods). The picker (Hearth Files through the file-chooser portal, or the
//! window app's own dialog) waits for the person, so it runs on its own thread: the bridge stays free, every other
//! request is answered meanwhile, and the fail-safe (watchdog.rs) never takes a picker left open for "stuck". What
//! was picked comes back to the bridge as {type:'internal', what:'picked'} (bridge.rs `picked`) or goes to `then`.
//! One picker at a time; the page hears {name:'picker', data:{open}} so it leaves the controller to the picker.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type FileFn = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
pub type FolderFn = Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>;

/// What to ask for: a file (title), or a folder (title, the folder it starts in).
#[derive(Debug, Clone, PartialEq)]
pub enum Ask { File(String), Folder(String, String) }

pub const ALREADY_OPEN: &str = "A file picker is open already: choose there, or close it first.";

static OPEN: AtomicBool = AtomicBool::new(false);
/// Tests that open pickers take turns (there's one picker at a time, process-wide).
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
/// Whether one of Hearth's pickers is open now.
pub fn is_open() -> bool { OPEN.load(Ordering::SeqCst) }

/// Puts OPEN back however the picker's thread ends (a picker that panics included).
struct Open;
impl Drop for Open { fn drop(&mut self) { OPEN.store(false, Ordering::SeqCst); } }

fn said(open: bool, what: &str) -> Value { json!({"type": "event", "name": "picker", "data": {"open": open, "what": what}}) }

/// Opens the picker on its own thread and returns at once; `then(choice)` runs on that thread when it closes
/// (None: closed without a choice). Err: a picker is open already (that one is in front; this one isn't opened).
pub fn open(file: &FileFn, folder: &FolderFn, ask: Ask, what: &str, then: impl FnOnce(Option<String>) + Send + 'static) -> Result<(), String> {
    if OPEN.swap(true, Ordering::SeqCst) { return Err(ALREADY_OPEN.into()); }
    let guard = Open;
    let (file, folder, what) = (file.clone(), folder.clone(), what.to_string());
    crate::bridge::log(&format!("picker: {what}: open (Hearth keeps answering meanwhile)"));
    crate::stores::notify(said(true, &what));
    let started = std::time::Instant::now();
    std::thread::Builder::new().name("hearth-picker".into()).spawn(move || {
        let _g = guard;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match &ask {
            Ask::File(title) => file(title),
            Ask::Folder(title, start) => folder(title, start),
        })).unwrap_or(None);
        drop(_g);
        crate::bridge::log(&format!("picker: {what}: {} after {} s", if r.is_some() { "chosen" } else { "closed" }, started.elapsed().as_secs()));
        crate::stores::notify(said(false, &what));
        then(r);
    }).map(|_| ()).map_err(|e| format!("Couldn't open the picker: {e}"))
}

/// The message that hands a choice back to the bridge: the request it answers (id, window) and the call's own args.
pub fn picked_message(method: &str, args: &Value, path: Option<String>, id: &Value, to: &str) -> Value {
    json!({"type": "internal", "what": "picked", "method": method, "args": args, "path": path, "reply": {"id": id, "to": to}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn one_at_a_time_and_off_the_callers_thread() {
        let _turn = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (go_tx, go_rx) = mpsc::channel::<Option<String>>();
        let go_rx = std::sync::Mutex::new(go_rx);
        let folder: FolderFn = Arc::new(move |_t: &str, _s: &str| go_rx.lock().unwrap().recv().unwrap());
        let file: FileFn = Arc::new(|_t: &str| None);
        let (done_tx, done_rx) = mpsc::channel();
        let t = std::time::Instant::now();
        open(&file, &folder, Ask::Folder("Where?".into(), "/".into()), "test", move |c| done_tx.send(c).unwrap()).unwrap();
        assert!(t.elapsed() < std::time::Duration::from_secs(2), "open() waited for the picker");
        assert!(is_open());
        assert_eq!(open(&file, &folder, Ask::File("Another".into()), "test", |_| {}).unwrap_err(), ALREADY_OPEN);
        go_tx.send(Some("/games".into())).unwrap();
        assert_eq!(done_rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(), Some("/games".into()));
        for _ in 0..50 { if !is_open() { break; } std::thread::sleep(std::time::Duration::from_millis(20)); }
        assert!(!is_open());
        // A picker that panics still closes and answers "nothing chosen".
        let bad: FileFn = Arc::new(|_t: &str| panic!("dialog broke"));
        let (tx, rx) = mpsc::channel();
        open(&bad, &folder, Ask::File("x".into()), "test", move |c| tx.send(c).unwrap()).unwrap();
        assert_eq!(rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(), None);
    }
}
