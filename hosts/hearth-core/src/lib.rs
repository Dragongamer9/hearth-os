//! Hearth's host logic in Rust, without a window: the Tauri app (hosts/hearth-rs) puts the UI on screen and
//! passes its messages here. Replaces the Electron host (hosts/linux) a part at a time; see docs/RUST.md.
/// Hearth's version (this crate's, which the owner sets with Hearth's): Hearth Link reports it for device health.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod access;   // Settings › Accessibility (desktop/shell/hearth-access)
pub mod admin;
pub mod appid;
pub mod appchannel;   // Hearth app updates without a system update (docs/APP-UPDATES.md)
pub mod appupdate;
pub mod version;   // MAJOR.MINOR as people see it (1.0.0 -> "1.0") and the release date
pub mod artcache;
pub mod backup;
pub mod bazaar;   // Bazaar, Hearth OS's app store, opened from Game Mode's menu
pub mod battlenet;   // Battle.net through Faugus Launcher, or in Hearth's own Proton prefix (Library › Stores)
pub mod beacon;
pub mod binkv;
pub mod bridge;
pub mod defaults;   // Settings › Default apps (desktop/shell/hearth-system)
pub mod collections;
pub mod desktop_games;
pub mod discord;   // Discord: "Playing X" (Rich Presence; needs the owner's application ID, /etc/hearth/discord.conf)
pub mod display;
pub mod diskspace;   // Settings › Storage: drives, games by size, moving games, clean up, low space (docs/STORAGE.md)
pub mod emulators;
pub mod friends;
// ---- Hearth's gaming apps (Controller Lab, Saves, Mods): opened from Game Mode
pub mod gaming_apps;
// ---- Hearth Mods in Game Mode (its worker, over a socket)
pub mod hearth_mods;
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
// ---- Hearth Link: paired Hearth devices, data sync and game copies (hosts/hearth-link)
pub mod link;
// ---- end of Hearth Link
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
pub mod osk;   // the on-screen keyboard's settings and learned words (ui/osk, docs/OSK.md)
pub mod overlay_link;
pub mod paths;
pub mod perf;
pub mod picker;   // file and folder pickers, off the bridge
pub mod play;   // Discover, game details, achievements, artwork, profile, downloads (docs/HYDRA-FEATURES.md)
pub mod power;
pub mod procs;
pub mod recomps;   // Hearth Recomps: native PC ports of console games (docs/RECOMPS.md)
pub mod profile;
pub mod profiles;
pub mod remote;
pub mod renderer;
pub mod signin_handoff;
// ---- Store sign-ins kept through updates (backed up, put back when missing, said in the log at start)
pub mod signins;
pub mod space;
pub mod settings_file;   // the safe way settings files are read and written (docs/SETTINGS-PERSISTENCE.md)
pub mod seat_share;   // Multi-seat: games shared between the players (docs/MULTI-SEAT.md)
pub mod steam;
pub mod steam_ctl;
pub mod settings_cli;   // Mantel's Settings asking Hearth's core (`--settings-call`, docs/SETTINGS-MOVE.md)
pub mod core_service;   // Hearth's core without a window in Steam Game Mode, for the Hearth Decky plugin (`--core-service`)
pub mod steam_takeover;   // Steam Game Mode takeover: what the Hearth Decky plugin's pages show (`--steam-mode-data`)
pub mod steam_mode;   // Steam Game Mode (Experimental): Steam's gamepad UI in gamescope with the Hearth Decky plugin (docs/STEAM-GAME-MODE.md)
pub mod steam_launch;   // Steam games in Game Mode: Steam kept ready, its windows in front, the starting screen's stages
pub mod steam_engine;   // Settings › Library › Steam engine: Lightweight (hosts/hearth-steam), with fallback to the normal client
pub mod steam_owned;
pub mod steam_thaw;   // Steam stopped by Handheld Daemon while Quick Access is open over a game: let run again
pub mod store;
// ---- Game Mode › Store (browsing Steam, GOG, Epic and Xbox; buying on their own pages)
pub mod storefront;
// ---- end of Game Mode › Store
pub mod stores;
pub mod system;
pub mod sysinfo;
pub mod theater;   // Hearth Theater, your wall of video sites, opened from Game Mode's menu
pub mod tools;
pub mod vdf;
pub mod watchdog;
