//! Emulated games: ROMs in your ROMs folder (EmuDeck/ES-DE folder names), played with RetroArch; box art from
//! the libretro thumbnail project by the ROM's own name. Port of library/emulators.js.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const FLATPAK: &str = "org.libretro.RetroArch";
const THUMBS: &str = "https://thumbnails.libretro.com/";

pub struct System { pub dirs: &'static [&'static str], pub name: &'static str, pub ext: &'static [&'static str], pub cores: &'static [&'static str], pub thumbs: &'static str }

pub const SYSTEMS: &[System] = &[
    System { dirs: &["nes", "famicom"], name: "NES", ext: &["nes", "fds", "unf", "zip", "7z"], cores: &["mesen", "nestopia", "fceumm"], thumbs: "Nintendo - Nintendo Entertainment System" },
    System { dirs: &["snes", "sfc"], name: "Super Nintendo", ext: &["sfc", "smc", "zip", "7z"], cores: &["snes9x", "bsnes"], thumbs: "Nintendo - Super Nintendo Entertainment System" },
    System { dirs: &["n64"], name: "Nintendo 64", ext: &["n64", "z64", "v64", "zip", "7z"], cores: &["mupen64plus_next", "parallel_n64"], thumbs: "Nintendo - Nintendo 64" },
    System { dirs: &["gb"], name: "Game Boy", ext: &["gb", "zip", "7z"], cores: &["gambatte", "sameboy", "mgba"], thumbs: "Nintendo - Game Boy" },
    System { dirs: &["gbc"], name: "Game Boy Color", ext: &["gbc", "zip", "7z"], cores: &["gambatte", "sameboy", "mgba"], thumbs: "Nintendo - Game Boy Color" },
    System { dirs: &["gba"], name: "Game Boy Advance", ext: &["gba", "zip", "7z"], cores: &["mgba", "vbam", "gpsp"], thumbs: "Nintendo - Game Boy Advance" },
    System { dirs: &["nds"], name: "Nintendo DS", ext: &["nds", "zip", "7z"], cores: &["melondsds", "melonds", "desmume"], thumbs: "Nintendo - Nintendo DS" },
    System { dirs: &["genesis", "megadrive"], name: "Genesis / Mega Drive", ext: &["md", "gen", "smd", "bin", "zip", "7z"], cores: &["genesis_plus_gx", "picodrive"], thumbs: "Sega - Mega Drive - Genesis" },
    System { dirs: &["mastersystem"], name: "Master System", ext: &["sms", "zip", "7z"], cores: &["genesis_plus_gx", "picodrive"], thumbs: "Sega - Master System - Mark III" },
    System { dirs: &["gamegear"], name: "Game Gear", ext: &["gg", "zip", "7z"], cores: &["genesis_plus_gx"], thumbs: "Sega - Game Gear" },
    System { dirs: &["segacd", "megacd"], name: "Sega CD", ext: &["chd", "cue", "m3u"], cores: &["genesis_plus_gx", "picodrive"], thumbs: "Sega - Mega-CD - Sega CD" },
    System { dirs: &["saturn"], name: "Saturn", ext: &["chd", "cue", "m3u"], cores: &["mednafen_saturn", "yabasanshiro"], thumbs: "Sega - Saturn" },
    System { dirs: &["dreamcast"], name: "Dreamcast", ext: &["chd", "gdi", "cdi", "m3u"], cores: &["flycast"], thumbs: "Sega - Dreamcast" },
    System { dirs: &["psx", "ps1"], name: "PlayStation", ext: &["chd", "cue", "pbp", "m3u"], cores: &["swanstation", "mednafen_psx_hw", "mednafen_psx", "pcsx_rearmed"], thumbs: "Sony - PlayStation" },
    System { dirs: &["psp"], name: "PSP", ext: &["iso", "cso", "chd"], cores: &["ppsspp"], thumbs: "Sony - PlayStation Portable" },
    System { dirs: &["pcengine", "tg16"], name: "TurboGrafx-16", ext: &["pce", "zip", "7z"], cores: &["mednafen_pce_fast", "mednafen_pce"], thumbs: "NEC - PC Engine - TurboGrafx 16" },
    System { dirs: &["atari2600"], name: "Atari 2600", ext: &["a26", "bin", "zip"], cores: &["stella", "stella2014"], thumbs: "Atari - 2600" },
    System { dirs: &["ngp", "ngpc"], name: "Neo Geo Pocket", ext: &["ngp", "ngc", "zip"], cores: &["mednafen_ngp"], thumbs: "SNK - Neo Geo Pocket Color" },
];

pub fn rom_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots = vec![home.join("ROMs"), home.join("roms"), home.join("Emulation/roms")];
    let user = std::env::var("USER").unwrap_or_default();
    for m in [format!("/run/media/{user}"), "/run/media".into()] {
        if let Ok(rd) = std::fs::read_dir(&m) { for e in rd.flatten() { roots.push(e.path().join("Emulation/roms")); } }
    }
    let mut seen = std::collections::HashSet::new();
    roots.into_iter().filter(|r| r.is_dir() && seen.insert(r.clone())).collect()
}

/// "Legend of Zelda, The - A Link to the Past (USA) [!].sfc" → "The Legend of Zelda - A Link to the Past"
pub fn clean_title(file: &str) -> String {
    let base = Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let stem = match base.rfind('.') { Some(i) if i > 0 => base[..i].to_string(), _ => base.clone() };
    let t = regex::Regex::new(r"\s*[(\[][^)\]]*[)\]]").unwrap().replace_all(&stem, "").replace('_', " ");
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let re = regex::Regex::new(r"^(.*?), (The|A|An)(\b.*)$").unwrap();
    let t = match re.captures(&t) { Some(c) => format!("{} {}{}", &c[2], &c[1], &c[3]), None => t };
    if t.is_empty() { base } else { t }
}
/// libretro's thumbnail name for a ROM: &*/:`<>?\| become _.
pub fn thumb_name(file: &str) -> String {
    let base = Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let stem = match base.rfind('.') { Some(i) if i > 0 => base[..i].to_string(), _ => base };
    stem.chars().map(|c| if "&*/:`<>?\\|".contains(c) { '_' } else { c }).collect()
}
fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() { if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { o.push(b as char); } else { o.push_str(&format!("%{b:02X}")); } }
    o
}
pub fn thumb_url(sys: &System, file: &str, kind: &str) -> String { format!("{THUMBS}{}/{kind}/{}.png", enc(sys.thumbs), enc(&thumb_name(file))) }

fn scan_dir(dir: &Path, sys: &System, depth: u32) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let entries: Vec<_> = rd.flatten().collect();
    let files: Vec<String> = entries.iter().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
    // Discs listed in a playlist (.m3u), and .bin tracks of a .cue, aren't games of their own.
    let mut in_playlist = std::collections::HashSet::new();
    for f in files.iter().filter(|f| f.to_lowercase().ends_with(".m3u")) {
        for l in std::fs::read_to_string(dir.join(f)).unwrap_or_default().lines() { let l = l.trim(); if !l.is_empty() { in_playlist.insert(Path::new(l).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()); } }
    }
    let has_cue = files.iter().any(|f| f.to_lowercase().ends_with(".cue"));
    let mut out = Vec::new();
    for f in &files {
        let ext = f.rsplit('.').next().unwrap_or("").to_lowercase();
        if !sys.ext.contains(&ext.as_str()) || in_playlist.contains(&f.to_lowercase()) { continue; }
        if ext == "bin" && has_cue && sys.ext.contains(&"cue") { continue; }
        out.push(dir.join(f));
    }
    if depth > 0 {
        for e in entries.iter().filter(|e| e.path().is_dir()) {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with('.') || ["media", "images", "videos", "manuals", "saves", "states"].contains(&n.to_lowercase().as_str()) { continue; }
            out.extend(scan_dir(&e.path(), sys, depth - 1));
        }
    }
    out
}

/// Every ROM in the ROMs folders, as library games.
pub fn scan(roots: &[PathBuf]) -> Vec<crate::library::Game> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        let mut dirs: Vec<String> = rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
        dirs.sort();
        for d in dirs {
            let Some(sys) = SYSTEMS.iter().find(|s| s.dirs.contains(&d.to_lowercase().as_str())) else { continue };
            let base = root.join(&d);
            let mut files = scan_dir(&base, sys, 2);
            files.sort();
            for f in files {
                let rel = f.strip_prefix(&base).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let id = format!("emu:{}:{rel}", sys.dirs[0]);
                if !seen.insert(id.clone()) { continue; }
                let fs = f.to_string_lossy().to_string();
                let added = std::fs::metadata(&f).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
                out.push(crate::library::Game {
                    id, title: clean_title(&fs), source: "emu", install_path: Some(f.clone()), size_bytes: None, added, store_minutes: 0, store_last_played: 0,
                    installed: true, state: "installed", bytes_done: None, bytes_total: None, launch_uri: None, exe: Some(fs.clone()), args: String::new(),
                    art: vec![("cover", thumb_url(sys, &fs, "Named_Boxarts")), ("wide", thumb_url(sys, &fs, "Named_Snaps")), ("hero", thumb_url(sys, &fs, "Named_Snaps"))],
                    system: Some(sys.name), system_key: Some(sys.dirs[0]), ..Default::default()
                });
            }
        }
    }
    out
}

/// RetroArch: (command, arguments before ours, folders with its cores).
pub fn retroarch(home: &Path) -> Option<(String, Vec<String>, Vec<PathBuf>)> {
    if crate::system::has("retroarch") { return Some(("retroarch".into(), vec![], vec![home.join(".config/retroarch/cores"), "/usr/lib64/libretro".into(), "/usr/lib/libretro".into(), "/usr/lib/x86_64-linux-gnu/libretro".into()])); }
    let fp = [home.join(".local/share/flatpak/app").join(FLATPAK), Path::new("/var/lib/flatpak/app").join(FLATPAK)].iter().any(|p| p.exists());
    if fp && crate::system::has("flatpak") { return Some(("flatpak".into(), vec!["run".into(), FLATPAK.into()], vec![home.join(".var/app").join(FLATPAK).join("config/retroarch/cores")])); }
    None
}
pub fn find_core(sys: &System, dirs: &[PathBuf]) -> Option<PathBuf> {
    sys.cores.iter().flat_map(|c| dirs.iter().map(move |d| d.join(format!("{c}_libretro.so")))).find(|f| f.exists())
}

pub fn launch(rom: &str, system_key: &str, home: &Path) -> Result<u32, String> {
    let (cmd, pre, cores) = retroarch(home).ok_or("Emulated games play with RetroArch. Install it in Settings › Library sources › Emulators.")?;
    let sys = SYSTEMS.iter().find(|s| s.dirs[0] == system_key).ok_or("Unknown system")?;
    let core = find_core(sys, &cores).ok_or(format!("RetroArch needs a {} core: open RetroArch › Online Updater › Core Downloader and get {}.", sys.name, sys.cores[0].replace('_', " ")))?;
    if !Path::new(rom).exists() { return Err(format!("The game file is gone: {rom}")); }
    let mut args = pre; args.extend(["-f".into(), "-L".into(), core.to_string_lossy().to_string(), rom.into()]);
    std::process::Command::new(cmd).args(args).current_dir(Path::new(rom).parent().unwrap_or(Path::new("/")))
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|c| c.id()).map_err(|e| e.to_string())
}

pub fn status(home: &Path) -> Value {
    let ra = retroarch(home);
    json!({"retroarch": ra.is_some(), "via": ra.map(|r| if r.0 == "flatpak" { "flatpak" } else { "system" }), "roots": rom_roots(home), "folder": home.join("ROMs"),
           "systems": SYSTEMS.iter().map(|s| json!({"dir": s.dirs[0], "name": s.name})).collect::<Vec<_>>()})
}
pub fn make_folders(home: &Path) -> Result<PathBuf, String> {
    let root = home.join("ROMs");
    for s in SYSTEMS { std::fs::create_dir_all(root.join(s.dirs[0])).map_err(|e| e.to_string())?; }
    Ok(root)
}
pub fn install_retroarch() -> Result<(), String> {
    if !crate::system::has("flatpak") { return Err("Flatpak is needed to install RetroArch".into()); }
    crate::system::run_timeout("flatpak", &["install", "--user", "-y", "--noninteractive", "flathub", FLATPAK], std::time::Duration::from_secs(1200)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titles_and_thumbnails() {
        assert_eq!(clean_title("Legend of Zelda, The - A Link to the Past (USA) [!].sfc"), "The Legend of Zelda - A Link to the Past");
        assert_eq!(thumb_name("Sonic & Knuckles (World).md"), "Sonic _ Knuckles (World)");
        let snes = SYSTEMS.iter().find(|s| s.dirs[0] == "snes").unwrap();
        assert_eq!(thumb_url(snes, "/r/Super Mario World (USA).sfc", "Named_Boxarts"),
            "https://thumbnails.libretro.com/Nintendo%20-%20Super%20Nintendo%20Entertainment%20System/Named_Boxarts/Super%20Mario%20World%20(USA).png");
    }
    #[test]
    fn scans_a_roms_folder() {
        let root = tempfile::tempdir().unwrap();
        let put = |rel: &str, text: &str| { let f = root.path().join(rel); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, text).unwrap(); };
        put("snes/Super Mario World (USA).sfc", "x"); put("snes/readme.txt", "x");
        put("psx/Final Fantasy VII (USA) (Disc 1).chd", "x"); put("psx/Final Fantasy VII (USA) (Disc 2).chd", "x");
        put("psx/Final Fantasy VII (USA).m3u", "Final Fantasy VII (USA) (Disc 1).chd\nFinal Fantasy VII (USA) (Disc 2).chd\n");
        put("psx/Crash Bandicoot (USA).cue", "x"); put("psx/Crash Bandicoot (USA).bin", "x");
        put("gba/Collections/Metroid Fusion (USA).gba", "x"); put("gba/media/Metroid Fusion (USA).gba", "x");
        let mut titles: Vec<String> = scan(&[root.path().to_path_buf()]).into_iter().map(|g| g.title).collect();
        titles.sort();
        assert_eq!(titles, vec!["Crash Bandicoot", "Final Fantasy VII", "Metroid Fusion", "Super Mario World"]);
    }
}
