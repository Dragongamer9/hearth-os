//! Hearth Notes' workings, apart from its window (main.rs and the page files), so they're tested on their own:
//!
//! * [`md`]: Markdown as the editor shows it: blocks, inline marks, `[[links]]`, tags, headings, checklists, pictures;
//! * [`vault`]: the folder of notes (any folder; the hearth-workspace clone by default), its links and backlinks;
//! * [`day`]: US Central dates and times (the workspace's logs), with the "day starts at" setting (6:00 AM);
//! * [`trackers`]: Today's optional productivity and habits, on this PC, apart from the log (its format never changes);
//! * [`prefs`]: the app's settings (settings_file.rs: atomic, .bak, merged, never written by opening it);
//! * [`save`]: saving a note right away, atomically, with its .bak outside the workspace;
//! * [`git`] and [`sync`]: pull, commit and push with the PC's own Git sign-in and config, and the order they run in;
//! * [`check`]: the workspace's own `tools/check.py`, which blocks a push until it's tidy, and its fixes;
//! * [`secrets`]: what looks like a token, password or key, found before anything is committed;
//! * [`conflict`]: one note changed on two PCs: keep mine, keep theirs, or merge both, with copies always kept;
//! * [`newitem`]: the New item wizard's rules (the workspace's CLAUDE.md layout and names);
//! * [`board`]: the kanban made from checklists and "Waiting:" lines, and moving a card;
//! * [`search`]: full-text search with filters, and the quick switcher's fuzzy match;
//! * [`graph`]: the notes and links as a graph (layout, clusters, most linked, orphans, a note's neighbourhood).

pub mod board;
pub mod check;
pub mod conflict;
pub mod day;
pub mod git;
pub mod graph;
pub mod md;
pub mod newitem;
pub mod paths;
pub mod prefs;
pub mod save;
pub mod search;
pub mod secrets;
pub mod sync;
pub mod trackers;
pub mod vault;

/// A test lock for the tests that change the process's environment (HOME, XDG_*): one at a time.
#[cfg(test)]
pub(crate) fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    L.lock().unwrap_or_else(|e| e.into_inner())
}
