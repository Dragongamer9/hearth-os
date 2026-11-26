//! The consoles and their games: every ROM folder RetroDECK, EmuDeck and Hearth Retro use (ES-DE's folder names:
//! snes, psx, n64…), read once when the app opens and again on Read again. Nothing here writes anything.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One console: its id, the folder names it has in ES-DE / RetroDECK / EmuDeck, its name, its emulator and the files
/// that are its games.
#[derive(Debug, Clone, Copy)]
pub struct System {
    pub id: &'static str,
    pub folders: &'static [&'static str],
    pub name: &'static str,
    /// the emulator Hearth Retro starts its games in (emus::EMUS by name)
    pub emu: &'static str,
    pub exts: &'static [&'static str],
    /// RetroAchievements' console id (its API), 0: none
    pub ra: u32,
}

pub const SYSTEMS: &[System] = &[
    System { id: "snes", folders: &["snes", "sfc", "snesna"], name: "SNES", emu: "Snes9x", exts: &["sfc", "smc", "fig", "swc", "zip", "7z"], ra: 3 },
    System { id: "psx", folders: &["psx", "ps1", "playstation"], name: "PlayStation", emu: "DuckStation", exts: &["cue", "chd", "pbp", "m3u", "ecm", "iso"], ra: 12 },
    System { id: "n64", folders: &["n64"], name: "Nintendo 64", emu: "Mupen64Plus", exts: &["z64", "n64", "v64", "zip", "7z"], ra: 2 },
    System { id: "gba", folders: &["gba"], name: "Game Boy Advance", emu: "mGBA", exts: &["gba", "zip", "7z"], ra: 5 },
    System { id: "gc", folders: &["gc", "gamecube", "ngc"], name: "GameCube", emu: "Dolphin", exts: &["iso", "rvz", "gcm", "gcz", "ciso", "wbfs"], ra: 16 },
    System { id: "ps2", folders: &["ps2"], name: "PlayStation 2", emu: "PCSX2", exts: &["iso", "chd", "cso", "gz", "m3u"], ra: 21 },
    System { id: "dreamcast", folders: &["dreamcast", "dc"], name: "Dreamcast", emu: "Flycast", exts: &["chd", "gdi", "cdi", "m3u"], ra: 40 },
    System { id: "arcade", folders: &["arcade", "mame", "fbneo"], name: "Arcade", emu: "MAME", exts: &["zip", "7z"], ra: 27 },
    System { id: "nes", folders: &["nes", "famicom"], name: "NES", emu: "RetroArch", exts: &["nes", "fds", "unf", "zip", "7z"], ra: 7 },
    System { id: "genesis", folders: &["genesis", "megadrive"], name: "Mega Drive", emu: "RetroArch", exts: &["md", "gen", "smd", "bin", "zip", "7z"], ra: 1 },
    System { id: "psp", folders: &["psp"], name: "PSP", emu: "PPSSPP", exts: &["iso", "cso", "pbp", "chd"], ra: 41 },
    System { id: "nds", folders: &["nds"], name: "Nintendo DS", emu: "melonDS", exts: &["nds", "zip", "7z"], ra: 18 },
    System { id: "saturn", folders: &["saturn"], name: "Saturn", emu: "Beetle Saturn", exts: &["cue", "chd", "m3u", "ccd"], ra: 39 },
    System { id: "wii", folders: &["wii"], name: "Wii", emu: "Dolphin", exts: &["wbfs", "rvz", "iso", "wad"], ra: 19 },
    System { id: "wiiu", folders: &["wiiu"], name: "Wii U", emu: "Cemu", exts: &["wua", "wud", "wux", "rpx"], ra: 0 },
    System { id: "gb", folders: &["gb"], name: "Game Boy", emu: "mGBA", exts: &["gb", "zip", "7z"], ra: 4 },
    System { id: "gbc", folders: &["gbc"], name: "Game Boy Color", emu: "mGBA", exts: &["gbc", "zip", "7z"], ra: 6 },
    System { id: "scummvm", folders: &["scummvm"], name: "ScummVM", emu: "ScummVM", exts: &["scummvm", "svm"], ra: 0 },
];

pub fn system(id: &str) -> Option<&'static System> {
    SYSTEMS.iter().find(|s| s.id == id)
}

/// One game: its file, its console, the name it's shown by (the file's name without "(USA)" and the like, or
/// ES-DE's gamelist name) and its year when ES-DE's gamelist knows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub path: PathBuf,
    pub system: &'static str,
    pub title: String,
    /// the file's name without its extension ("Super Metroid (USA)"): art and saves are named by it
    pub stem: String,
    pub year: String,
}

/// Every ROM folder (the first that has a console's folder wins its name; games from all of them are shown, once).
pub fn roots() -> Vec<PathBuf> {
    let h = crate::home();
    let mut v = vec![crate::retro_dir().join("roms")];
    for extra in [h.join("retrodeck").join("roms"), h.join("Emulation").join("roms"), h.join("ROMs")] {
        if !v.contains(&extra) {
            v.push(extra);
        }
    }
    // the folder Retro to Steam (or the person) picked, as Hearth Retro's own prefs say
    if let Some(p) = crate::prefs::read().get("romsFolder").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
        let p = expand(p);
        if !v.contains(&p) {
            v.insert(0, p);
        }
    }
    v
}

/// "~/x" as a path.
pub fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => crate::home().join(rest),
        None if p == "~" => crate::home(),
        None => PathBuf::from(p),
    }
}

/// A path as it's shown: the home folder as ~.
pub fn tilde(p: &Path) -> String {
    let h = crate::home();
    match p.strip_prefix(&h) {
        Ok(r) if r.as_os_str().is_empty() => "~".into(),
        Ok(r) => format!("~/{}", r.display()),
        Err(_) => p.display().to_string(),
    }
}

/// "Chrono Trigger (USA) [!]" → "Chrono Trigger"; "Legend of Zelda, The - A Link to the Past" → "The Legend of Zelda - A Link to the Past".
pub fn clean_title(stem: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for c in stem.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let mut t = out.replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    for art in [", The", ", A", ", An"] {
        if let Some(i) = t.find(art) {
            let (head, rest) = t.split_at(i);
            let rest = &rest[art.len()..];
            t = format!("{} {}{}", &art[2..], head, rest);
            break;
        }
    }
    t.trim().to_string()
}

fn ext_of(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// ES-DE's gamelist.xml names and years (RetroDECK's ~/retrodeck/ES-DE/gamelists/<system>/, EmuDeck's
/// ~/ES-DE/gamelists/<system>/), by the game's file name. Read with a small reader of its own (<game> … <path>,
/// <name>, <releasedate>).
fn gamelist(system_folder: &str) -> BTreeMap<String, (String, String)> {
    let h = crate::home();
    let mut out = BTreeMap::new();
    for base in [h.join("retrodeck/ES-DE/gamelists"), h.join("ES-DE/gamelists"), h.join(".emulationstation/gamelists")] {
        let Ok(text) = std::fs::read_to_string(base.join(system_folder).join("gamelist.xml")) else { continue };
        for block in text.split("<game").skip(1) {
            let tag = |t: &str| -> String {
                let (o, c) = (format!("<{t}>"), format!("</{t}>"));
                block.find(&o).and_then(|i| block[i + o.len()..].find(&c).map(|j| block[i + o.len()..i + o.len() + j].to_string())).unwrap_or_default()
            };
            let path = tag("path");
            let file = path.rsplit('/').next().unwrap_or("").to_string();
            if file.is_empty() {
                continue;
            }
            let date = tag("releasedate");
            let year = if date.len() >= 4 && date[..4].chars().all(|c| c.is_ascii_digit()) { date[..4].to_string() } else { String::new() };
            let name = unxml(&tag("name"));
            out.entry(file).or_insert((name, year));
        }
    }
    out
}

fn unxml(s: &str) -> String {
    s.replace("&amp;", "&").replace("&apos;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">")
}

/// The games of every console with any, in the consoles' order, each console's games by title.
pub fn scan() -> Vec<(&'static System, Vec<Game>)> {
    let roots = roots();
    let mut out = Vec::new();
    for sys in SYSTEMS {
        let mut games: Vec<Game> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for root in &roots {
            for folder in sys.folders {
                let dir = root.join(folder);
                let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                let names = gamelist(folder);
                let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                files.sort();
                // a .cue's .bin, an .m3u's discs: only the one to start from
                let has: std::collections::HashSet<String> = files.iter().map(|p| ext_of(p)).collect();
                for p in files {
                    let e = ext_of(&p);
                    if !sys.exts.contains(&e.as_str()) || (e == "bin" && has.contains("cue")) {
                        continue;
                    }
                    if p.is_dir() && e != "wua" {
                        continue;
                    }
                    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let file = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let (gl_name, year) = names.get(&file).cloned().unwrap_or_default();
                    let title = if gl_name.is_empty() { clean_title(&stem) } else { gl_name };
                    if title.is_empty() || !seen.insert(title.to_lowercase()) {
                        continue;
                    }
                    games.push(Game { path: p, system: sys.id, title, stem, year });
                }
            }
        }
        if !games.is_empty() {
            games.sort_by_key(|g| g.title.to_lowercase());
            out.push((sys, games));
        }
    }
    out
}

/// Box art for a game: Hearth Retro's own (~/Hearth/Retro/art/<system>/<file name>.png, which Retro to Steam fills),
/// else ES-DE's scraped covers (RetroDECK, EmuDeck).
pub fn art(g: &Game) -> Option<PathBuf> {
    let h = crate::home();
    let sys = system(g.system)?;
    let mut dirs = vec![crate::retro_dir().join("art").join(sys.id)];
    for f in sys.folders {
        dirs.push(h.join("retrodeck/downloaded_media").join(f).join("covers"));
        dirs.push(h.join("ES-DE/downloaded_media").join(f).join("covers"));
        dirs.push(h.join("Emulation/tools/downloaded_media").join(f).join("covers"));
    }
    for d in dirs {
        for ext in ["png", "jpg", "jpeg", "webp"] {
            let p = d.join(format!("{}.{ext}", g.stem));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Two colours for a game's tile when it has no art: the same game always gets the same ones.
pub fn tile_colours(title: &str) -> ([u8; 3], [u8; 3]) {
    let mut h: u32 = 2166136261;
    for b in title.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    let hue = (h % 360) as f32;
    let top = hsl(hue, 0.58, 0.55);
    let bottom = hsl(hue, 0.6, 0.11);
    (top, bottom)
}

fn hsl(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}

/// "1284" → "1,284".
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_cleaned() {
        assert_eq!(clean_title("Chrono Trigger (USA)"), "Chrono Trigger");
        assert_eq!(clean_title("Super Metroid (Japan, USA) (En,Ja) [!]"), "Super Metroid");
        assert_eq!(clean_title("Legend of Zelda, The - A Link to the Past (USA)"), "The Legend of Zelda - A Link to the Past");
        assert_eq!(thousands(1284), "1,284");
        assert_eq!(thousands(12), "12");
        assert_eq!(tile_colours("F-Zero"), tile_colours("F-Zero"));
    }
}
