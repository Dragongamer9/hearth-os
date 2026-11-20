//! Saving a note: right away (a moment after typing stops), all at once (a temporary file beside it, fsync, one
//! rename), and its last version is kept as a .bak first. The .bak lives outside the vault
//! (~/.local/share/hearth/notes/bak/<vault>/), so the workspace never gets extra files to commit or tidy.

use std::io::Write;
use std::path::{Path, PathBuf};

pub fn bak_for(vault: &Path, rel: &str) -> PathBuf {
    crate::paths::baks().join(crate::paths::vault_key(vault)).join(format!("{rel}.bak"))
}

fn fsync_dir(dir: &Path) {
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
}

/// Writes `bytes` to `path` all at once (keeping its permissions).
pub fn atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir)?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.hn-{}-{}.tmp", std::process::id(), SEQ.fetch_add(1, Ordering::SeqCst)));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        let mode = std::fs::metadata(path).ok().map(|m| m.permissions().mode() & 0o7777).unwrap_or(0o644);
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))?;
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    fsync_dir(&dir);
    r
}

/// Saves a note (unchanged: nothing is written). The old text goes to its .bak first. True: it was written.
pub fn note(vault: &Path, rel: &str, text: &str) -> std::io::Result<bool> {
    if rel.contains("..") || rel.starts_with('/') {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a path in the vault"));
    }
    let path = vault.join(rel);
    let cur = std::fs::read(&path).ok();
    if cur.as_deref() == Some(text.as_bytes()) {
        return Ok(false);
    }
    if let Some(c) = cur.filter(|c| !c.is_empty()) {
        let b = bak_for(vault, rel);
        atomic(&b, &c)?;
    }
    atomic(&path, text.as_bytes())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_atomically_with_a_bak_outside_the_vault() {
        let _l = crate::env_lock();
        let d = tempfile::tempdir().unwrap();
        let vault = d.path().join("hearth-workspace");
        std::env::set_var("XDG_DATA_HOME", d.path().join("data"));
        std::fs::create_dir_all(vault.join("log")).unwrap();
        std::fs::write(vault.join("log/2026-11-20.md"), "# 2026-11-20\n").unwrap();
        assert!(note(&vault, "log/2026-11-20.md", "# 2026-11-20\n\n- 03:04 Made: x\n").unwrap());
        assert_eq!(std::fs::read_to_string(vault.join("log/2026-11-20.md")).unwrap(), "# 2026-11-20\n\n- 03:04 Made: x\n");
        let bak = bak_for(&vault, "log/2026-11-20.md");
        assert!(bak.starts_with(d.path().join("data")), "the .bak is outside the vault");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), "# 2026-11-20\n");
        assert!(!note(&vault, "log/2026-11-20.md", "# 2026-11-20\n\n- 03:04 Made: x\n").unwrap(), "unchanged: not written");
        // Nothing but the note in the vault's folder (no temporary files left).
        let names: Vec<String> = std::fs::read_dir(vault.join("log")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["2026-11-20.md".to_string()]);
        assert!(note(&vault, "../escape.md", "x").is_err());
        std::env::remove_var("XDG_DATA_HOME");
    }
}
