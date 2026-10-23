//! Edit as administrator (Linux): saving a file you can't write, such as one in /etc. The bytes go to
//! `pkexec /usr/libexec/nimbus/hearth-admin write-file PATH` on its standard input. polkit asks for an
//! administrator's password every time (Hearth's rule, 49-hearth-admin.rules, never lets write-file through
//! without one), and hearth-admin only replaces an existing regular file at a plain path (no links, no devices).
//! Hearth Text itself never runs as root.

use std::path::Path;

pub const HELPER: &str = "/usr/libexec/nimbus/hearth-admin";

/// Whether this system can save as administrator (Hearth OS on Linux, with pkexec).
pub fn available() -> bool {
    cfg!(target_os = "linux") && Path::new(HELPER).exists() && which("pkexec")
}

fn which(prog: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
        .unwrap_or(false)
}

/// What hearth-admin said, turned into a sentence for the person.
pub fn explain(code: Option<i32>, stdout: &str, stderr: &str) -> String {
    if let Some(line) = stdout.lines().rev().find_map(|l| l.strip_prefix("error: ")) { return line.to_string(); }
    match code {
        // pkexec: 126 = the password dialog was closed or the password was wrong; 127 = not allowed at all.
        Some(126) => "Not saved: the administrator password wasn't given.".into(),
        Some(127) => "Not saved: this account isn't allowed to change system files.".into(),
        _ => {
            let e = stderr.trim();
            if e.is_empty() { "Not saved: the administrator helper failed.".into() } else { format!("Not saved: {e}") }
        }
    }
}

/// Saves `bytes` as `path` through pkexec and hearth-admin.
#[cfg(target_os = "linux")]
pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if !available() { return Err("Saving as administrator needs Hearth OS (pkexec and hearth-admin).".into()); }
    // hearth-admin refuses links: give it the real file.
    let real = std::fs::canonicalize(path).map_err(|e| crate::files::friendly(&e, path))?;
    let mut child = Command::new("pkexec")
        .arg(HELPER).arg("write-file").arg(&real)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("Couldn't start pkexec: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let data = bytes.to_vec();
    let writer = std::thread::spawn(move || { let _ = stdin.write_all(&data); });
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = writer.join();
    if out.status.success() { return Ok(()); }
    Err(explain(out.status.code(), &String::from_utf8_lossy(&out.stdout), &String::from_utf8_lossy(&out.stderr)))
}

#[cfg(not(target_os = "linux"))]
pub fn save(_path: &Path, _bytes: &[u8]) -> Result<(), String> {
    Err("Saving as administrator is only on Linux. Use Save as… to keep a copy somewhere you can write.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explains_failures() {
        assert_eq!(explain(Some(1), "log: x\nerror: only regular files can be saved this way\n", ""), "only regular files can be saved this way");
        assert!(explain(Some(126), "", "").contains("password"));
        assert!(explain(Some(127), "", "").contains("allowed"));
    }
}
