//! Hearth Retro's library, shared with Retro to Steam (apps/retro-to-steam): the consoles and their ROM folders
//! (Hearth's own ~/Hearth/Retro/roms, RetroDECK's ~/retrodeck/roms and EmuDeck's ~/Emulation/roms, the same folder
//! names: ES-DE's), the emulators RetroDECK and EmuDeck set up and how each game starts in one, the BIOS check, the
//! controllers, and what Hearth Retro keeps (its own settings, written only the safe way: settings_file, red flag 9).

pub mod bios;
pub mod emus;
pub mod library;
pub mod md5;
pub mod pads;
pub mod prefs;

/// The home folder.
pub fn home() -> std::path::PathBuf {
    hearth_ui::paths::home()
}

/// ~/Hearth/Retro: Hearth Retro's own folder (ROMs, BIOS, saves, states, art), synced with Hearth Link.
pub fn retro_dir() -> std::path::PathBuf {
    std::env::var_os("HEARTH_RETRO_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| home().join("Hearth").join("Retro"))
}

/// What CI's pictures pretend (HEARTH_RETRO_FIXTURE: installed emulators, controllers, achievements); never set on a
/// PC. Null without it.
pub fn fixture() -> serde_json::Value {
    std::env::var("HEARTH_RETRO_FIXTURE")
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null)
}
