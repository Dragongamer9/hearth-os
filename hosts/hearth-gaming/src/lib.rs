//! Hearth's own gaming utilities, for Hearth OS (Anatase and Peppermint):
//!
//! - Controller Lab (`hearth-controller-lab`): every connected controller live (buttons, sticks with their dead
//!   zone and drift, triggers, D-pad, motion sensors, touchpads, battery), rumble, which driver it's on, stick
//!   and trigger calibration through evdev, and Hearth Input's dead zones. pads.rs.
//! - Saves (`hearth-saves`): finds the saves of the games in Hearth's library with Ludusavi's open manifest
//!   (manifest.rs, locate.rs), backs them up as one zip per game, restores them (with a safety copy first), and
//!   backs up on a schedule. saves.rs.
//! - Mods (`hearth-mods`): a mod manager: install a mod from a zip or folder into the game's folder or its Proton
//!   prefix, turn it on and off without losing the files it replaced, load order. mods.rs.
//!
//! games.rs is the game list Saves and Mods share (Hearth's library through hearth-core). util.rs has the small
//! things all three use. Windows (Ember) comes later: the parts that touch the system are Linux-only for now.

pub mod util;
pub mod manifest;
pub mod locate;
pub mod saves;
pub mod mods;
pub mod nexus;
pub mod downloads;
pub mod wabbajack;
pub mod guided;
pub mod accounts;
pub mod games;
pub mod browse;
#[cfg(unix)]
pub mod service;
#[cfg(target_os = "linux")]
pub mod pads;
