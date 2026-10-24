//! Hearth's host logic in Rust, without a window: the Tauri app (hosts/hearth-rs) puts the UI on screen and
//! passes its messages here. Replaces the Electron host (hosts/linux) a part at a time; see docs/RUST.md.
pub mod access;   // Settings › Accessibility (desktop/shell/hearth-access)
pub mod admin;
pub mod appid;
pub mod appupdate;
pub mod artcache;
pub mod backup;
pub mod beacon;
pub mod binkv;
pub mod bridge;
pub mod defaults;   // Settings › Default apps (desktop/shell/hearth-system)
pub mod collections;
pub mod desktop_games;
pub mod display;
pub mod emulators;
pub mod friends;
// ---- Hearth's gaming apps (Controller Lab, Saves, Mods): opened from Game Mode
pub mod gaming_apps;
// ---- end of Hearth's gaming apps
pub mod game_options;
pub mod gameinfo;
pub mod gamescope;
pub mod guide;
pub mod hearth_browser;
pub mod hhd;
pub mod http;
pub mod importer;
pub mod input_map;
pub mod inputplumber;
pub mod logs;
pub mod xodus;
pub mod library;
// ---- Nearby Share and Hearth Connect, opened from Game Mode
pub mod link_apps;
// ---- end of Nearby Share and Hearth Connect
pub mod multiplay;
pub mod owned;
pub mod padmouse;
pub mod manual;
pub mod matrix_friends;   // Hearth Friends on Matrix (hosts/hearth-friends)
pub mod newos;
pub mod osinstall;
pub mod overlay_link;
pub mod paths;
pub mod perf;
pub mod power;
pub mod procs;
pub mod profile;
pub mod profiles;
pub mod remote;
pub mod renderer;
pub mod signin_handoff;
pub mod space;
pub mod steam;
pub mod steam_ctl;
pub mod steam_owned;
pub mod store;
pub mod stores;
pub mod system;
pub mod sysinfo;
pub mod tools;
pub mod vdf;
pub mod watchdog;
