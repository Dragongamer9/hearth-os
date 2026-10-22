//! The shells a new tab can run. Linux: your login shell first, then the others in /etc/shells.
//! Windows: PowerShell 7 (pwsh) when it's installed, Windows PowerShell, Command Prompt, and WSL when a Linux
//! distribution is installed.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Shell {
    /// Stable name saved in settings ("login", "bash", "pwsh", "cmd", ...).
    pub id: String,
    /// What the menu shows.
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    /// Extra environment for this shell (Command Prompt's prompt that reports the folder).
    #[serde(skip)]
    pub env: Vec<(String, String)>,
}

/// Every shell found on this computer, the default one first.
pub fn available() -> Vec<Shell> { platform::available() }

/// The shell a new tab runs when settings don't pick one (or the picked one is gone).
pub fn default_shell() -> Shell {
    available().into_iter().next().unwrap_or_else(platform::fallback)
}

/// The shell saved in settings, if it's still installed; otherwise the default.
pub fn pick(id: Option<&str>) -> Shell {
    let all = available();
    if let Some(id) = id.filter(|s| !s.is_empty()) {
        if let Some(s) = all.iter().find(|s| s.id == id) { return s.clone(); }
    }
    all.into_iter().next().unwrap_or_else(platform::fallback)
}

#[cfg(unix)]
mod platform {
    use super::Shell;
    use std::path::Path;

    fn title(base: &str) -> String {
        match base {
            "bash" => "Bash".into(),
            "zsh" => "Zsh".into(),
            "fish" => "Fish".into(),
            "nu" => "Nushell".into(),
            "dash" => "Dash".into(),
            other => other.to_string(),
        }
    }

    fn shell(id: &str, program: &str, mine: bool) -> Shell {
        let base = Path::new(program).file_name().and_then(|s| s.to_str()).unwrap_or(program);
        let name = if mine { format!("{} (your shell)", title(base)) } else { title(base) };
        Shell { id: id.into(), name, program: program.into(), args: vec![], env: vec![] }
    }

    /// $SHELL, or the shell in your account (passwd) when $SHELL isn't set.
    pub fn login_shell() -> Option<String> {
        if let Ok(s) = std::env::var("SHELL") {
            if !s.is_empty() && Path::new(&s).exists() { return Some(s); }
        }
        // getpwuid_r: the account's shell, as login does.
        unsafe {
            let mut pw: libc::passwd = std::mem::zeroed();
            let mut buf = vec![0 as libc::c_char; 4096];
            let mut out: *mut libc::passwd = std::ptr::null_mut();
            if libc::getpwuid_r(libc::getuid(), &mut pw, buf.as_mut_ptr(), buf.len(), &mut out) == 0
                && !out.is_null() && !pw.pw_shell.is_null()
            {
                let s = std::ffi::CStr::from_ptr(pw.pw_shell).to_string_lossy().into_owned();
                if !s.is_empty() && Path::new(&s).exists() { return Some(s); }
            }
        }
        None
    }

    pub fn available() -> Vec<Shell> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if let Some(s) = login_shell() {
            let base = Path::new(&s).file_name().and_then(|b| b.to_str()).unwrap_or("").to_string();
            seen.insert(base);
            out.push(shell("login", &s, true));
        }
        let listed = std::fs::read_to_string("/etc/shells").unwrap_or_default();
        for line in listed.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') || !Path::new(line).exists() { continue; }
            let base = Path::new(line).file_name().and_then(|b| b.to_str()).unwrap_or("").to_string();
            // nologin and the like aren't shells you can type into; /usr/bin and /bin copies are the same shell.
            if base.is_empty() || base.contains("nologin") || base == "false" || base == "git-shell" || !seen.insert(base.clone()) { continue; }
            out.push(shell(&base, line, false));
        }
        if out.is_empty() { out.push(fallback()); }
        out
    }

    pub fn fallback() -> Shell { shell("sh", "/bin/sh", false) }
}

#[cfg(windows)]
mod platform {
    use super::Shell;
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// PowerShell's prompt, wrapped so it also tells the terminal the current folder (OSC 9;9, the same
    /// sequence Windows Terminal uses). New tabs and splits then open in that folder.
    const PS_PROMPT: &str = r#"$global:__hearthPrompt = $function:prompt
function global:prompt {
  $loc = $executionContext.SessionState.Path.CurrentLocation
  $out = ''
  if ($loc.Provider.Name -eq 'FileSystem') { $out = "$([char]27)]9;9;`"$($loc.ProviderPath)`"$([char]27)\" }
  $out + (& $global:__hearthPrompt)
}"#;

    fn ps_args() -> Vec<String> {
        let utf16: Vec<u8> = PS_PROMPT.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(utf16);
        vec!["-NoLogo".into(), "-NoExit".into(), "-EncodedCommand".into(), b64]
    }

    fn system32() -> PathBuf {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        Path::new(&root).join("System32")
    }

    fn in_path(exe: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path).map(|d| d.join(exe)).find(|p| p.is_file())
    }

    fn pwsh() -> Option<PathBuf> {
        in_path("pwsh.exe").or_else(|| {
            let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
            let p = Path::new(&pf).join("PowerShell").join("7").join("pwsh.exe");
            p.is_file().then_some(p)
        })
    }

    /// WSL is only offered when a Linux distribution is installed (wsl.exe alone just offers to install one).
    fn wsl() -> Option<PathBuf> {
        let exe = system32().join("wsl.exe");
        if !exe.is_file() { return None; }
        let out = std::process::Command::new(system32().join("reg.exe"))
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout).contains("Lxss\\{").then_some(exe)
    }

    pub fn available() -> Vec<Shell> {
        let mut out = Vec::new();
        if let Some(p) = pwsh() {
            out.push(Shell { id: "pwsh".into(), name: "PowerShell".into(), program: p.to_string_lossy().into(), args: ps_args(), env: vec![] });
        }
        let winps = system32().join("WindowsPowerShell").join("v1.0").join("powershell.exe");
        if winps.is_file() {
            out.push(Shell { id: "powershell".into(), name: "Windows PowerShell".into(), program: winps.to_string_lossy().into(), args: ps_args(), env: vec![] });
        }
        out.push(fallback());
        if let Some(w) = wsl() {
            out.push(Shell { id: "wsl".into(), name: "WSL (Linux)".into(), program: w.to_string_lossy().into(), args: vec!["--cd".into(), "~".into()], env: vec![] });
        }
        out
    }

    pub fn fallback() -> Shell {
        Shell {
            id: "cmd".into(),
            name: "Command Prompt".into(),
            program: system32().join("cmd.exe").to_string_lossy().into(),
            args: vec![],
            // The usual prompt ($P$G), plus the folder for the terminal (OSC 9;9).
            env: vec![("PROMPT".into(), "$E]9;9;$P$E\\$P$G".into())],
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn there_is_always_a_shell() {
        let all = super::available();
        assert!(!all.is_empty());
        assert_eq!(super::default_shell(), all[0]);
        assert_eq!(super::pick(Some("no-such-shell")), all[0]);
        assert!(std::path::Path::new(&all[0].program).exists(), "{:?}", all[0]);
    }
}
