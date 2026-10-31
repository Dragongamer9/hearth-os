//! Hearth Theater (hosts/hearth-theater), your own wall of video sites, opened from Game Mode's menu (Theater). It
//! opens full screen inside gamescope like Bazaar (its window is a program Hearth opened, so it goes in front). It reads
//! the controller itself (the viewer has its own pointer), so the controller mouse stays off.

use std::path::{Path, PathBuf};

pub const BIN: &str = "hearth-theater";

/// Where Theater is: the system image's /usr/bin (or /usr/local/bin), looking in `root` ("/" normally).
pub fn find_in(root: &Path) -> Option<PathBuf> {
    ["usr/bin", "usr/local/bin"].iter().map(|d| root.join(d).join(BIN)).find(|b| b.is_file())
}

pub fn find() -> Option<PathBuf> { find_in(Path::new("/")) }

/// Whether Game Mode shows Theater in its menu: when the image has it.
pub fn available() -> bool { cfg!(target_os = "linux") && find().is_some() }

/// Opens Theater (full screen in Game Mode). Returns {ok}.
pub fn open(game_mode: bool) -> Result<serde_json::Value, String> {
    let bin = find().ok_or("Hearth Theater isn't on this system yet")?;
    let args: Vec<&str> = if game_mode { vec!["--fullscreen"] } else { vec![] };
    crate::bridge::log(&format!("theater: opening {} {}", bin.display(), args.join(" ")));
    std::process::Command::new(&bin).args(&args).current_dir(crate::paths::home())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map_err(|e| format!("Couldn't open Theater: {e}"))?;
    Ok(serde_json::json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_where_it_is() {
        let root = std::env::temp_dir().join(format!("hearth-theater-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(find_in(&root), None);
        std::fs::create_dir_all(root.join("usr/local/bin")).unwrap();
        std::fs::write(root.join("usr/local/bin").join(BIN), "").unwrap();
        assert_eq!(find_in(&root), Some(root.join("usr/local/bin").join(BIN)));
        std::fs::create_dir_all(root.join("usr/bin")).unwrap();
        std::fs::write(root.join("usr/bin").join(BIN), "").unwrap();
        assert_eq!(find_in(&root), Some(root.join("usr/bin").join(BIN)));
        let _ = std::fs::remove_dir_all(&root);
    }
}
