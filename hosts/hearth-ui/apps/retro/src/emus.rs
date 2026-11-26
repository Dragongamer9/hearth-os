//! The emulators RetroDECK and EmuDeck set up, whether each is here, and how a game starts in it.
//!
//! An emulator is "here" when any of its ways is: RetroDECK's Flatpak (net.retrodeck.retrodeck carries them all), its
//! own Flathub app, its program on the system, or (for RetroArch's cores) the core's file. "Set up in one go" installs
//! RetroDECK from Flathub for you (flatpak --user, nothing as root) and writes Hearth Retro's one RetroArch config
//! (~/Hearth/Retro/hearth-retroarch.cfg: the same hotkeys, saves and states in ~/Hearth/Retro), given to RetroArch
//! with --appendconfig when Hearth starts a game, so RetroArch's own retroarch.cfg is never changed.

use crate::library::{Game, System};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const RETRODECK: &str = "net.retrodeck.retrodeck";

/// One emulator: its name, its RetroArch core (when it's one), its Flathub app, its programs, and what RetroDECK's
/// Flatpak calls it (`flatpak run --command=…`).
#[derive(Debug, Clone, Copy)]
pub struct Emu {
    pub name: &'static str,
    pub core: &'static str,
    pub flatpak: &'static str,
    pub cmds: &'static [&'static str],
    pub retrodeck_cmd: &'static str,
    /// its arguments before the game's file
    pub args: &'static [&'static str],
}

pub const EMUS: &[Emu] = &[
    Emu { name: "RetroArch", core: "", flatpak: "org.libretro.RetroArch", cmds: &["retroarch"], retrodeck_cmd: "retroarch", args: &[] },
    Emu { name: "Snes9x", core: "snes9x", flatpak: "com.snes9x.Snes9x", cmds: &["snes9x-gtk"], retrodeck_cmd: "", args: &[] },
    Emu { name: "DuckStation", core: "swanstation", flatpak: "org.duckstation.DuckStation", cmds: &["duckstation-qt"], retrodeck_cmd: "", args: &["-fullscreen", "--"] },
    Emu { name: "PCSX2", core: "", flatpak: "net.pcsx2.PCSX2", cmds: &["pcsx2-qt"], retrodeck_cmd: "pcsx2-qt", args: &["-fullscreen", "--"] },
    Emu { name: "Dolphin", core: "", flatpak: "org.DolphinEmu.dolphin-emu", cmds: &["dolphin-emu"], retrodeck_cmd: "dolphin-emu", args: &["-b", "-e"] },
    Emu { name: "PPSSPP", core: "", flatpak: "org.ppsspp.PPSSPP", cmds: &["PPSSPPSDL", "ppsspp"], retrodeck_cmd: "PPSSPPSDL", args: &["--fullscreen"] },
    Emu { name: "Flycast", core: "flycast", flatpak: "org.flycast.Flycast", cmds: &["flycast"], retrodeck_cmd: "", args: &[] },
    Emu { name: "melonDS", core: "melonds", flatpak: "net.kuribo64.melonDS", cmds: &["melonDS"], retrodeck_cmd: "melonDS", args: &[] },
    Emu { name: "mGBA", core: "mgba", flatpak: "io.mgba.mGBA", cmds: &["mgba-qt"], retrodeck_cmd: "", args: &["-f"] },
    Emu { name: "Mupen64Plus", core: "mupen64plus_next", flatpak: "", cmds: &["mupen64plus"], retrodeck_cmd: "", args: &[] },
    Emu { name: "Beetle Saturn", core: "mednafen_saturn", flatpak: "", cmds: &[], retrodeck_cmd: "", args: &[] },
    Emu { name: "Cemu", core: "", flatpak: "info.cemu.Cemu", cmds: &["cemu", "Cemu"], retrodeck_cmd: "Cemu-wrapper", args: &["-f", "-g"] },
    Emu { name: "ScummVM", core: "scummvm", flatpak: "org.scummvm.ScummVM", cmds: &["scummvm"], retrodeck_cmd: "scummvm", args: &[] },
    Emu { name: "MAME", core: "mame", flatpak: "org.mamedev.MAME", cmds: &["mame"], retrodeck_cmd: "mame", args: &[] },
];

pub fn emu(name: &str) -> Option<&'static Emu> {
    EMUS.iter().find(|e| e.name == name)
}

/// What's on this PC: the Flatpak apps installed (yours and the system's) and RetroArch's core folders.
#[derive(Debug, Clone, Default)]
pub struct Here {
    pub flatpaks: Vec<String>,
    pub cmds: Vec<String>,
    pub cores: Vec<String>,
    /// the fixture's word (CI): these emulators are here
    pub fixed: Option<Vec<String>>,
}

fn on_path(c: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(c).is_file())).unwrap_or(false)
}

pub fn core_dirs() -> Vec<PathBuf> {
    let h = crate::home();
    vec![
        h.join(".config/retroarch/cores"),
        h.join(".var/app/org.libretro.RetroArch/config/retroarch/cores"),
        h.join(".var/app/net.retrodeck.retrodeck/config/retroarch/cores"),
        PathBuf::from("/usr/lib64/libretro"),
        PathBuf::from("/usr/lib/libretro"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu/libretro"),
    ]
}

/// Looks once (flatpak list, PATH, the core folders). Slow-ish (flatpak): run off the window's thread.
pub fn look() -> Here {
    let fx = crate::fixture();
    if let Some(list) = fx.get("installed").and_then(|v| v.as_array()) {
        return Here { fixed: Some(list.iter().filter_map(|x| x.as_str().map(String::from)).collect()), ..Here::default() };
    }
    let mut h = Here::default();
    if let Ok(o) = Command::new("flatpak").args(["list", "--app", "--columns=application"]).output() {
        h.flatpaks = String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    }
    for e in EMUS {
        for c in e.cmds {
            if on_path(c) {
                h.cmds.push(c.to_string());
            }
        }
    }
    for d in core_dirs() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for f in rd.flatten() {
                let n = f.file_name().to_string_lossy().to_string();
                if let Some(core) = n.strip_suffix("_libretro.so") {
                    h.cores.push(core.to_string());
                }
            }
        }
    }
    h
}

impl Here {
    pub fn retrodeck(&self) -> bool {
        match &self.fixed {
            Some(f) => f.iter().any(|x| x == "RetroDECK"),
            None => self.flatpaks.iter().any(|f| f == RETRODECK),
        }
    }
    fn retroarch(&self) -> bool {
        self.retrodeck() || self.flatpaks.iter().any(|f| f == "org.libretro.RetroArch") || self.cmds.iter().any(|c| c == "retroarch")
    }
    /// Is this emulator here, in any of its ways?
    pub fn has(&self, e: &Emu) -> bool {
        if let Some(f) = &self.fixed {
            return f.iter().any(|x| x == e.name || x == "RetroDECK");
        }
        if self.retrodeck() {
            return true;
        }
        (!e.flatpak.is_empty() && self.flatpaks.iter().any(|f| f == e.flatpak))
            || e.cmds.iter().any(|c| self.cmds.iter().any(|x| x == c))
            || (!e.core.is_empty() && self.retroarch() && self.cores.iter().any(|c| c == e.core))
    }
    pub fn missing(&self) -> Vec<&'static Emu> {
        EMUS.iter().filter(|e| !self.has(e)).collect()
    }
}

/// Hearth Retro's own RetroArch settings, given with --appendconfig (never written into retroarch.cfg): the same
/// hotkeys for every console (≡ + button: ≡ + R1 saves, ≡ + L1 loads, ≡ + B the menu, ≡ + Start quits), saves and
/// states in ~/Hearth/Retro, BIOS from ~/Hearth/Retro/bios.
pub fn retroarch_cfg_text() -> String {
    let r = crate::retro_dir();
    let q = |p: PathBuf| p.display().to_string();
    format!(
        "# Hearth Retro (written by Hearth Retro's Set up in one go; RetroArch's own retroarch.cfg isn't changed)\n\
         savefile_directory = \"{}\"\nsavestate_directory = \"{}\"\nsystem_directory = \"{}\"\n\
         sort_savefiles_by_content_enable = \"false\"\nsort_savestates_by_content_enable = \"false\"\n\
         input_enable_hotkey_btn = \"6\"\ninput_save_state_btn = \"5\"\ninput_load_state_btn = \"4\"\n\
         input_menu_toggle_btn = \"1\"\ninput_exit_emulator_btn = \"7\"\ninput_state_slot_increase_axis = \"+5\"\n\
         savestate_auto_index = \"true\"\nvideo_fullscreen = \"true\"\n",
        q(r.join("saves")),
        q(r.join("states")),
        q(r.join("bios"))
    )
}

pub fn retroarch_cfg() -> PathBuf {
    crate::retro_dir().join("hearth-retroarch.cfg")
}

/// Is Hearth Retro's config written (as it should be)?
pub fn cfg_ok() -> bool {
    if crate::fixture().get("installed").is_some() {
        return true;
    }
    std::fs::read_to_string(retroarch_cfg()).map(|t| t == retroarch_cfg_text()).unwrap_or(false)
}

/// Writes Hearth Retro's config and its folders (saves, states, bios, roms with each console's folder).
pub fn write_cfg() -> Result<(), String> {
    let r = crate::retro_dir();
    for d in ["saves", "states", "bios", "art"] {
        std::fs::create_dir_all(r.join(d)).map_err(|e| e.to_string())?;
    }
    for s in crate::library::SYSTEMS {
        let _ = std::fs::create_dir_all(r.join("roms").join(s.folders[0]));
    }
    hearth_ui::settings_file::write_text(&retroarch_cfg(), &retroarch_cfg_text()).map(|_| ()).map_err(|e| e.to_string())
}

/// The command that installs what's missing the Hearth way: RetroDECK from Flathub, for you only (no password).
pub fn install_cmd() -> Vec<String> {
    let script = format!(
        "flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo && \
         flatpak install --user -y --noninteractive flathub {RETRODECK}"
    );
    vec!["sh".into(), "-c".into(), script]
}

fn retroarch_base(h: &Here) -> Option<Vec<String>> {
    if h.cmds.iter().any(|c| c == "retroarch") {
        Some(vec!["retroarch".into()])
    } else if h.flatpaks.iter().any(|f| f == "org.libretro.RetroArch") {
        Some(vec!["flatpak".into(), "run".into(), "org.libretro.RetroArch".into()])
    } else if h.retrodeck() {
        Some(vec!["flatpak".into(), "run".into(), "--command=retroarch".into(), RETRODECK.into()])
    } else {
        None
    }
}

fn core_file(core: &str) -> Option<PathBuf> {
    core_dirs().into_iter().map(|d| d.join(format!("{core}_libretro.so"))).find(|p| p.is_file())
}

/// The extra RetroArch lines for this game's look (CRT look, 4:3 smooth), written beside the config before it starts.
pub fn look_cfg(crt: bool, smooth: bool) -> String {
    let mut s = String::new();
    if crt {
        for d in shader_dirs() {
            let p = d.join("crt/crt-easymode.slangp");
            if p.is_file() {
                s.push_str(&format!("video_shader_enable = \"true\"\nvideo_shader = \"{}\"\n", p.display()));
                break;
            }
        }
    } else {
        s.push_str("video_shader_enable = \"false\"\n");
    }
    if smooth {
        s.push_str("aspect_ratio_index = \"0\"\nvideo_smooth = \"true\"\n");
    }
    s
}

fn shader_dirs() -> Vec<PathBuf> {
    let h = crate::home();
    vec![
        h.join(".config/retroarch/shaders/shaders_slang"),
        h.join(".var/app/org.libretro.RetroArch/config/retroarch/shaders/shaders_slang"),
        PathBuf::from("/usr/share/libretro/shaders/shaders_slang"),
    ]
}

/// The command that starts `g` in its console's emulator (slot: a save state to load, RetroArch's --entryslot), or
/// why it can't.
pub fn command(g: &Game, sys: &System, h: &Here, slot: Option<i32>, look: (bool, bool)) -> Result<Vec<String>, String> {
    let rom = g.path.display().to_string();
    let e = emu(sys.emu).ok_or("no emulator for it")?;
    // a RetroArch core first (the same hotkeys, saves and states everywhere), then the emulator on its own
    let core = if e.name == "RetroArch" { default_core(sys.id) } else { e.core };
    if !core.is_empty() {
        if let (Some(mut base), Some(cf)) = (retroarch_base(h), core_file(core)) {
            base.extend(["-L".into(), cf.display().to_string()]);
            let cfg = retroarch_cfg();
            if cfg.is_file() {
                let extra = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-retro-look.cfg");
                let _ = std::fs::write(&extra, look_cfg(look.0, look.1));
                base.extend(["--appendconfig".into(), format!("{}|{}", cfg.display(), extra.display())]);
            }
            if let Some(s) = slot {
                base.push(format!("--entryslot={s}"));
            }
            base.push(rom);
            return Ok(base);
        }
    }
    let mut base: Vec<String> = if let Some(c) = e.cmds.iter().find(|c| h.cmds.iter().any(|x| x == *c)) {
        vec![c.to_string()]
    } else if !e.flatpak.is_empty() && h.flatpaks.iter().any(|f| f == e.flatpak) {
        vec!["flatpak".into(), "run".into(), e.flatpak.into()]
    } else if h.retrodeck() && !e.retrodeck_cmd.is_empty() {
        vec!["flatpak".into(), "run".into(), format!("--command={}", e.retrodeck_cmd), RETRODECK.into()]
    } else {
        return Err(format!("{} isn't set up yet: Set up in one go installs it", e.name));
    };
    base.extend(e.args.iter().map(|a| a.to_string()));
    if e.name == "MAME" {
        let dir = g.path.parent().map(Path::to_path_buf).unwrap_or_default();
        base.extend(["-rompath".into(), dir.display().to_string(), g.stem.clone()]);
    } else {
        base.push(rom);
    }
    Ok(base)
}

/// RetroArch's core for a console it runs itself.
pub fn default_core(system: &str) -> &'static str {
    match system {
        "nes" => "mesen",
        "genesis" => "genesis_plus_gx",
        "gb" | "gbc" => "gambatte",
        _ => "",
    }
}
