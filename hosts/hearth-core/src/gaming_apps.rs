//! Hearth's gaming apps (hosts/hearth-gaming): Controller Lab, Saves and Mods, separate programs on Hearth OS
//! (/usr/bin/hearth-controller-lab, hearth-saves, hearth-mods). Game Mode opens them from Settings › Controls
//! (Controller Lab) and a game's ⋯ menu (Back up saves, Mods): full screen in Game Mode, on that game.

use std::path::PathBuf;

/// The page's name for each app → its program.
pub const APPS: [(&str, &str); 3] = [("controller-lab", "hearth-controller-lab"), ("saves", "hearth-saves"), ("mods", "hearth-mods")];

/// Where a program is: on PATH, or in /usr/bin.
pub fn find(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).chain([PathBuf::from("/usr/bin"), PathBuf::from("/usr/local/bin")]).map(|d| d.join(program)).find(|p| p.is_file())
}

/// Whether any of them is installed (the page then shows its entries: host.info feature "gamingApps").
pub fn available() -> bool { APPS.iter().any(|(_, p)| find(p).is_some()) }

/// The program and its arguments for one app, opened on one game (a library id, checked) or none.
pub fn command(app: &str, game: Option<&str>, fullscreen: bool) -> Result<(&'static str, Vec<String>), String> {
    let program = APPS.iter().find(|(a, _)| *a == app).map(|(_, p)| *p).ok_or("Unknown app")?;
    let mut args = Vec::new();
    if fullscreen { args.push("--fullscreen".to_string()); }
    if let Some(g) = game.filter(|g| !g.is_empty()) {
        if !crate::desktop_games::valid_id(g) { return Err("Unknown game".into()); }
        if app != "controller-lab" { args.push("--game".into()); args.push(g.to_string()); }
    }
    Ok((program, args))
}

/// Opens one of them (full screen in Game Mode).
pub fn open(app: &str, game: Option<&str>, fullscreen: bool) -> Result<(), String> {
    let (program, args) = command(app, game, fullscreen)?;
    let exe = find(program).ok_or_else(|| format!("{program} isn't installed. It comes with Hearth OS."))?;
    crate::bridge::log(&format!("gaming apps: opening {program} {}", args.join(" ")));
    std::process::Command::new(exe).args(&args).current_dir(crate::paths::home())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map(|_| ()).map_err(|e| format!("Couldn't open it: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands() {
        assert_eq!(command("saves", Some("steam:620"), true).unwrap(), ("hearth-saves", vec!["--fullscreen".to_string(), "--game".into(), "steam:620".into()]));
        assert_eq!(command("controller-lab", Some("steam:620"), false).unwrap(), ("hearth-controller-lab", vec![]));
        assert_eq!(command("mods", None, false).unwrap().1, Vec::<String>::new());
        assert!(command("mods", Some("bad\"id"), false).is_err());
        assert!(command("rm", None, false).is_err());
    }
}
