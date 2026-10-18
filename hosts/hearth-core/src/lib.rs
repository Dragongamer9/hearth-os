//! Hearth's host logic in Rust, without a window: the Tauri app (hosts/hearth-rs) puts the UI on screen and
//! passes its messages here. Replaces the Electron host (hosts/linux) a part at a time; see docs/RUST.md.
pub mod admin;
pub mod bridge;
pub mod collections;
pub mod game_options;
pub mod library;
pub mod manual;
pub mod paths;
pub mod perf;
pub mod power;
pub mod procs;
pub mod profile;
pub mod profiles;
pub mod steam;
pub mod store;
pub mod system;
pub mod sysinfo;
pub mod vdf;
