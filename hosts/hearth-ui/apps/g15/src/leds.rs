//! The keyboard's lights, through the kernel's LED files (hid-lg-g15). The key light and the screen light have three
//! levels (0 off, 1 low, 2 high: max_brightness 2); M1–M3 and MR are on or off. Reading `brightness` asks the keyboard
//! (a USB request), so it's read once and then followed through `brightness_hw_changed`, which the kernel updates when
//! the light button on the keyboard changes it. Writing needs the video group (70-hearth-g15.rules; hearth-groups
//! puts every account in it).

use crate::root::{read, read_u32};
use std::io::Write;
use std::path::Path;

pub fn level(dir: &Path) -> Option<u32> {
    read_u32(&dir.join("brightness"))
}

pub fn max(dir: &Path) -> u32 {
    read_u32(&dir.join("max_brightness")).unwrap_or(1).max(1)
}

/// The level the light button last set (None until it's been pressed, or without the file).
pub fn hw_changed(dir: &Path) -> Option<u32> {
    read(&dir.join("brightness_hw_changed"))?.parse().ok()
}

pub fn set(dir: &Path, v: u32) -> std::io::Result<()> {
    let v = v.min(max(dir));
    let mut f = std::fs::OpenOptions::new().write(true).truncate(true).open(dir.join("brightness"))?;
    f.write_all(v.to_string().as_bytes())
}

pub fn set_on(dir: &Option<std::path::PathBuf>, on: bool) {
    if let Some(d) = dir {
        if let Err(e) = set(d, on as u32) {
            crate::log(&format!("couldn't set {}: {e}", d.display()));
        }
    }
}
