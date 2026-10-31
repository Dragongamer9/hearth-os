//! The games the Hearth Mod Organizer knows, after Mod Organizer 2's game plugins (modorganizer-game_*,
//! GPL-3.0): where their mods go, how their plugins.txt is written, which plugins they always load, where their
//! inis and saves are, and their script extenders. A game it doesn't know is "generic": mods go into the game
//! folder itself and there are no plugins.

use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kind {
    pub id: &'static str,
    /// MO2's gameName (first) and other names for it (Wabbajack's, short ones).
    pub names: &'static [&'static str],
    pub steam: u64,
    /// The folder mods go into, inside the game folder ("" = the game folder).
    pub data: &'static str,
    /// plugins.txt lists every plugin with * before the active ones (Skyrim SE, Fallout 4, Starfield); otherwise it
    /// lists only the active ones and loadorder.txt has the order.
    pub star: bool,
    /// The record flag of a light plugin (0: the game has none).
    pub light: u32,
    /// The size of the TES4 record's header (20 for Oblivion, 24 later; 0: no plugins).
    pub header: usize,
    /// Plugins the game always loads first, in this order.
    pub implicit: &'static [&'static str],
    /// Its folder in Documents/My Games and AppData/Local.
    pub folder: &'static str,
    pub inis: &'static [&'static str],
    pub exe: &'static str,
    pub launcher: &'static str,
    /// The script extender's loader, in the game folder, and its name.
    pub extender: &'static str,
    pub extender_name: &'static str,
    /// The Creation Club list of always-loaded plugins, in the game folder.
    pub ccc: &'static str,
}

const SKYRIM_SE_MASTERS: &[&str] = &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"];

pub const KINDS: &[Kind] = &[
    Kind { id: "skyrimse", names: &["Skyrim Special Edition", "SkyrimSpecialEdition", "SkyrimSE", "Skyrim Anniversary Edition"], steam: 489830, data: "Data", star: true, light: 0x200, header: 24,
           implicit: SKYRIM_SE_MASTERS, folder: "Skyrim Special Edition", inis: &["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimCustom.ini"],
           exe: "SkyrimSE.exe", launcher: "SkyrimSELauncher.exe", extender: "skse64_loader.exe", extender_name: "SKSE", ccc: "Skyrim.ccc" },
    Kind { id: "skyrim", names: &["Skyrim", "Skyrim Legendary Edition", "SkyrimLE"], steam: 72850, data: "Data", star: false, light: 0, header: 24,
           implicit: &["Skyrim.esm", "Update.esm"], folder: "Skyrim", inis: &["Skyrim.ini", "SkyrimPrefs.ini"],
           exe: "TESV.exe", launcher: "SkyrimLauncher.exe", extender: "skse_loader.exe", extender_name: "SKSE", ccc: "" },
    Kind { id: "skyrimvr", names: &["Skyrim VR", "SkyrimVR"], steam: 611670, data: "Data", star: true, light: 0x200, header: 24,
           implicit: &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm", "SkyrimVR.esm"], folder: "Skyrim VR", inis: &["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimVR.ini"],
           exe: "SkyrimVR.exe", launcher: "", extender: "sksevr_loader.exe", extender_name: "SKSEVR", ccc: "" },
    Kind { id: "enderalse", names: &["Enderal Special Edition", "EnderalSpecialEdition", "EnderalSE"], steam: 976620, data: "Data", star: true, light: 0x200, header: 24,
           implicit: &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm", "Enderal - Forgotten Stories.esm"], folder: "Enderal Special Edition",
           inis: &["Enderal.ini", "EnderalPrefs.ini"], exe: "SkyrimSE.exe", launcher: "Enderal Launcher.exe", extender: "skse64_loader.exe", extender_name: "SKSE", ccc: "" },
    Kind { id: "fallout4", names: &["Fallout 4", "Fallout4"], steam: 377160, data: "Data", star: true, light: 0x200, header: 24,
           implicit: &["Fallout4.esm", "DLCRobot.esm", "DLCworkshop01.esm", "DLCCoast.esm", "DLCworkshop02.esm", "DLCworkshop03.esm", "DLCNukaWorld.esm", "DLCUltraHighResolution.esm"],
           folder: "Fallout4", inis: &["Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"], exe: "Fallout4.exe", launcher: "Fallout4Launcher.exe", extender: "f4se_loader.exe", extender_name: "F4SE", ccc: "Fallout4.ccc" },
    Kind { id: "fallout4vr", names: &["Fallout 4 VR", "Fallout4VR"], steam: 611660, data: "Data", star: true, light: 0, header: 24,
           implicit: &["Fallout4.esm", "Fallout4_VR.esm"], folder: "Fallout4VR", inis: &["Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"],
           exe: "Fallout4VR.exe", launcher: "", extender: "f4sevr_loader.exe", extender_name: "F4SEVR", ccc: "" },
    Kind { id: "falloutnv", names: &["New Vegas", "FalloutNV", "FalloutNewVegas", "Fallout New Vegas", "TTW"], steam: 22380, data: "Data", star: false, light: 0, header: 24,
           implicit: &["FalloutNV.esm"], folder: "FalloutNV", inis: &["Fallout.ini", "FalloutPrefs.ini", "FalloutCustom.ini"],
           exe: "FalloutNV.exe", launcher: "FalloutNVLauncher.exe", extender: "nvse_loader.exe", extender_name: "NVSE", ccc: "" },
    Kind { id: "fallout3", names: &["Fallout 3", "Fallout3"], steam: 22300, data: "Data", star: false, light: 0, header: 24,
           implicit: &["Fallout3.esm"], folder: "Fallout3", inis: &["Fallout.ini", "FalloutPrefs.ini"],
           exe: "Fallout3.exe", launcher: "FalloutLauncher.exe", extender: "fose_loader.exe", extender_name: "FOSE", ccc: "" },
    Kind { id: "oblivion", names: &["Oblivion"], steam: 22330, data: "Data", star: false, light: 0, header: 20,
           implicit: &["Oblivion.esm"], folder: "Oblivion", inis: &["Oblivion.ini"],
           exe: "Oblivion.exe", launcher: "OblivionLauncher.exe", extender: "obse_loader.exe", extender_name: "OBSE", ccc: "" },
    Kind { id: "starfield", names: &["Starfield"], steam: 1716740, data: "Data", star: true, light: 0x100, header: 24,
           implicit: &["Starfield.esm", "Constellation.esm", "OldMars.esm", "BlueprintShips-Starfield.esm", "SFBGS003.esm", "SFBGS004.esm", "SFBGS006.esm", "SFBGS007.esm", "SFBGS008.esm"],
           folder: "Starfield", inis: &["StarfieldCustom.ini", "StarfieldPrefs.ini"], exe: "Starfield.exe", launcher: "", extender: "sfse_loader.exe", extender_name: "SFSE", ccc: "Starfield.ccc" },
];

pub const GENERIC: Kind = Kind { id: "generic", names: &["Other game"], steam: 0, data: "", star: false, light: 0, header: 0, implicit: &[], folder: "", inis: &[],
    exe: "", launcher: "", extender: "", extender_name: "", ccc: "" };

impl Kind {
    pub fn by_id(id: &str) -> Kind { KINDS.iter().find(|k| k.id == id).copied().unwrap_or(GENERIC) }
    /// From MO2's gameName or Wabbajack's game name.
    pub fn by_name(name: &str) -> Option<Kind> {
        let n = name.trim();
        KINDS.iter().find(|k| k.names.iter().any(|x| x.eq_ignore_ascii_case(n)) || k.id.eq_ignore_ascii_case(n)).copied()
    }
    pub fn by_steam(app: u64) -> Option<Kind> {
        // New Vegas' low-violence edition has its own app.
        let app = if app == 22490 { 22380 } else { app };
        KINDS.iter().find(|k| k.steam == app).copied()
    }
    /// From what's in a game folder (its exe), when nothing else says.
    pub fn by_folder(dir: &Path) -> Option<Kind> {
        KINDS.iter().find(|k| !k.exe.is_empty() && crate::organizer::vfs::ci_find(dir, k.exe).is_some() && crate::organizer::vfs::ci_find(dir, k.data).is_some()).copied()
    }
    pub fn mo2_name(&self) -> &'static str { self.names[0] }
    pub fn has_plugins(&self) -> bool { self.header > 0 }
    pub fn is_generic(&self) -> bool { self.id == "generic" }
    pub fn public(&self) -> Value {
        json!({"id": self.id, "name": self.mo2_name(), "steamApp": if self.steam > 0 { json!(self.steam) } else { Value::Null }, "dataFolder": self.data, "plugins": self.has_plugins(),
               "lightPlugins": self.light != 0, "extender": if self.extender.is_empty() { Value::Null } else { json!({"name": self.extender_name, "loader": self.extender}) }})
    }
}

pub fn all_public() -> Value { Value::Array(KINDS.iter().map(|k| k.public()).chain(std::iter::once(GENERIC.public())).collect()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_apps() {
        assert_eq!(Kind::by_name("Skyrim Special Edition").unwrap().id, "skyrimse");
        assert_eq!(Kind::by_name("SkyrimSpecialEdition").unwrap().id, "skyrimse");
        assert_eq!(Kind::by_name("fallout 4").unwrap().id, "fallout4");
        assert_eq!(Kind::by_name("New Vegas").unwrap().id, "falloutnv");
        assert!(Kind::by_name("Stardew Valley").is_none());
        assert_eq!(Kind::by_steam(22490).unwrap().id, "falloutnv");
        assert_eq!(Kind::by_id("nope").id, "generic");
        assert!(KINDS.iter().all(|k| k.implicit.iter().all(|p| p.to_lowercase().ends_with(".esm"))));
    }
}
