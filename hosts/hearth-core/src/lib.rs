//! Hearth's host logic in Rust, without a window: the Tauri app (hosts/hearth-rs) puts the UI on screen and
//! passes its messages here. Replaces the Electron host (hosts/linux) a part at a time; see docs/RUST.md.
pub mod bridge;
pub mod library;
pub mod paths;
pub mod steam;
pub mod store;
pub mod system;
pub mod vdf;
