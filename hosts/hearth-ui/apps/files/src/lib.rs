//! Hearth Files in pure Rust (hearth-ui, iced): Mantel's Hearth Files (hosts/linux/desktop/shell/FileManager.qml and
//! its Files*.qml pages) for the Fedora edition, where KDE Plasma runs and Quickshell doesn't. The same backend
//! (hearth-files), the same settings (~/.config/nimbus/files.json), the same words; built to the approved mockups
//! (docs/mockups/next-update/files-*.png, cdrive-1/3; checklist docs/mockups/checklists/files-rust.md).
//!
//! Two programs share it: hearth-files-app (the file manager) and hearth-file-chooser (every app's Open / Save
//! window, started by hearth-portal with the request in HEARTH_PICK_REQUEST, the answer to HEARTH_PICK_RESULT).

pub mod app;
pub mod backend;
pub mod briefcase;
pub mod dnd;
pub mod dragout;
pub mod drive;
pub mod extras;
pub mod icons;
pub mod ldescribe;
pub mod menus;
pub mod model;
pub mod osk_view;
pub mod pad;
pub mod sortdl;
pub mod split;
pub mod terminal;
pub mod thumbs;
pub mod view;
// Hearth's on-screen keyboard engine and its looks, shared with hearth-pad-host (the same files, not copies): the
// Open / Save window draws the keyboard inside itself in Game Mode (pad.rs).
#[path = "../../pad/src/osk.rs"]
pub mod osk;
#[path = "../../pad/src/looks.rs"]
pub mod looks;

pub use app::{run, Mode};
