//! The BIOS check: the BIOS files the consoles need, looked for in every BIOS folder RetroDECK, EmuDeck, RetroArch
//! and Hearth Retro use, and checked against the sums RetroDECK's and libretro's lists give (a wrong dump or a file
//! that was renamed is "wrong file (bad check)"). Only reads.

use std::path::PathBuf;

/// One BIOS the check looks for: the console, the file names that will do, their good sums (MD5; empty: any file of
/// that name will do), and the name to show.
#[derive(Debug, Clone, Copy)]
pub struct Need {
    pub console: &'static str,
    pub files: &'static [&'static str],
    pub sums: &'static [&'static str],
    /// the emulator's own BIOS folder name under bios/ (PCSX2's "ps2"), "" for bios/ itself
    pub sub: &'static str,
}

pub const NEEDS: &[Need] = &[
    Need { console: "PlayStation", files: &["scph5501.bin", "scph5500.bin", "scph5502.bin", "scph1001.bin"], sums: &["490f666e1afb15b7362b406ed1cea246", "8dd7d5296a650fac7319bce665a6a53c", "32736f17079d0b2b7024407c39bd3050", "924e392ed05558ffdb115408c263dccf"], sub: "" },
    Need { console: "PlayStation 2", files: &["scph39001.bin", "scph70012.bin", "scph77001.bin", "ps2-0230a-20080220.bin", "SCPH-70012_BIOS_V12_USA_200.BIN"], sums: &[], sub: "ps2" },
    Need { console: "Saturn", files: &["sega_101.bin", "mpr-17933.bin"], sums: &["85ec9ca47d8f6807718151cbcca8b964", "3240872c70984b6cbfda1586cab68dbe"], sub: "" },
    Need { console: "Dreamcast", files: &["dc_boot.bin"], sums: &["e10c53c2f8b90bab96ead2d368858623"], sub: "dc" },
    Need { console: "Dreamcast flash", files: &["dc_flash.bin"], sums: &[], sub: "dc" },
    Need { console: "Game Boy Advance", files: &["gba_bios.bin"], sums: &["a860e8c0b6d573d191e4ec7db1b1e4f6"], sub: "" },
    Need { console: "Nintendo DS (ARM7)", files: &["bios7.bin"], sums: &["df692a80a5b1bc90728bc3dfc76cd948"], sub: "" },
    Need { console: "Nintendo DS (ARM9)", files: &["bios9.bin"], sums: &["a392174eb3e572fed6447e956bde4b25"], sub: "" },
    Need { console: "Sega CD", files: &["bios_CD_U.bin"], sums: &["2efd74e3232ff260e371b99f84024f7f"], sub: "" },
    Need { console: "PC Engine CD", files: &["syscard3.pce"], sums: &["38179df8f4ac870017db21ebcbf53114"], sub: "" },
    Need { console: "Neo Geo", files: &["neogeo.zip"], sums: &[], sub: "" },
];

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// found and its sum is good (the file's name)
    Good(String),
    /// not in any BIOS folder
    Missing,
    /// found, but its sum isn't one of the good ones (the file, its sum)
    Bad(String, String),
}

#[derive(Debug, Clone)]
pub struct Check {
    pub need: &'static Need,
    pub state: State,
    pub at: Option<PathBuf>,
}

/// Every BIOS folder, Hearth Retro's own first (where "Where to put it" points).
pub fn dirs() -> Vec<PathBuf> {
    let h = crate::home();
    vec![
        crate::retro_dir().join("bios"),
        h.join("retrodeck/bios"),
        h.join("Emulation/bios"),
        h.join(".config/retroarch/system"),
        h.join(".var/app/org.libretro.RetroArch/config/retroarch/system"),
    ]
}

/// The good sums: the list's, plus the fixture's (CI's pretend BIOS files, which can't be the real ones).
fn good(n: &Need) -> Vec<String> {
    let mut v: Vec<String> = n.sums.iter().map(|s| s.to_string()).collect();
    if let Some(extra) = crate::fixture().get("biosSums").and_then(|x| x.get(n.console)).and_then(|x| x.as_array()) {
        v.extend(extra.iter().filter_map(|x| x.as_str().map(String::from)));
    }
    v
}

/// Reads every BIOS folder (the files are small; MD5 of each found one). Off the window's thread.
pub fn check() -> Vec<Check> {
    let dirs = dirs();
    NEEDS
        .iter()
        .map(|n| {
            let mut bad: Option<(PathBuf, String)> = None;
            for d in &dirs {
                for f in n.files {
                    let mut tries = vec![d.join(f)];
                    if !n.sub.is_empty() {
                        tries.push(d.join(n.sub).join(f));
                    }
                    for p in tries {
                        let Ok(meta) = std::fs::metadata(&p) else { continue };
                        if !meta.is_file() {
                            continue;
                        }
                        let ok = good(n);
                        if ok.is_empty() {
                            return Check { need: n, state: State::Good(f.to_string()), at: Some(p) };
                        }
                        if meta.len() > 64 * 1024 * 1024 {
                            continue;
                        }
                        let sum = std::fs::read(&p).map(|b| crate::md5::hex(&b)).unwrap_or_default();
                        if ok.iter().any(|s| *s == sum) {
                            return Check { need: n, state: State::Good(f.to_string()), at: Some(p) };
                        }
                        bad.get_or_insert((p, sum));
                    }
                }
            }
            match bad {
                Some((p, sum)) => Check { need: n, state: State::Bad(p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default(), sum), at: Some(p) },
                None => Check { need: n, state: State::Missing, at: None },
            }
        })
        .collect()
}

/// The order the BIOS check card shows them: good ones first, then missing, then wrong files.
pub fn shown(c: &[Check]) -> Vec<&Check> {
    let mut v: Vec<&Check> = c.iter().filter(|x| matches!(x.state, State::Good(_))).take(1).collect();
    v.extend(c.iter().filter(|x| x.state == State::Missing));
    v.extend(c.iter().filter(|x| matches!(x.state, State::Bad(..))));
    v
}
