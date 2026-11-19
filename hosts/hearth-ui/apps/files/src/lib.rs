//! Hearth Files in pure Rust (hearth-ui, iced): Mantel's Hearth Files (hosts/linux/desktop/shell/FileManager.qml and
//! its Files*.qml pages) for the Fedora edition, where KDE Plasma runs and Quickshell doesn't. The same backend
//! (hearth-files), the same settings (~/.config/nimbus/files.json), the same words; built to the approved mockups
//! (docs/mockups/next-update/files-*.png, cdrive-1/3; checklist docs/mockups/checklists/files-rust.md).
//!
//! Two programs share it: hearth-files-app (the file manager) and hearth-file-chooser (every app's Open / Save
//! window, started by hearth-portal with the request in HEARTH_PICK_REQUEST, the answer to HEARTH_PICK_RESULT).

pub mod app;
pub mod backend;
pub mod drive;
pub mod icons;
pub mod menus;
pub mod model;
pub mod thumbs;
pub mod view;

pub use app::{run, Mode};
