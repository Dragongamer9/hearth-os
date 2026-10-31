//! Steam paused under a Steam game by Handheld Daemon (the HHD editions: Anatase, Peppermint, Ubuntu, CachyOS).
//!
//! While HHD's overlay app (Hearth's Quick Access over a game, overlay_link.rs) holds the controllers, HHD stops
//! Steam with SIGSTOP so Steam doesn't also act on the presses, and lets it go (SIGCONT) when the controllers are given
//! back: "Froze Steam (to avoid HID device dual input)" / "Unfreezing Steam" in HHD's log
//! (hhd/plugins/overlay/controllers.py device_shortcut_loop; plugins/plugin.py freeze_steam stops the first process
//! whose first or second argument ends in "/steam"). A Steam game waits on Steam (the Steam API, Steam Input, Steam's
//! own overlay), so the game froze for as long as Quick Access was open, and Steam's overlay with it. And HHD only
//! lets Steam go from that same loop: when its overlay thread ends while Steam is stopped ("Overlay thread died",
//! the app crashing, gamescope ending for Desktop Mode) nothing ever sends the SIGCONT, and Steam and its game stay
//! frozen for good.
//!
//! So Hearth lets Steam run again: right away while Quick Access is open over a game, again after it closes, and
//! whenever HHD's overlay app goes away (hearth-rs gamemode.rs, steam_while_open). The game still doesn't get the
//! presses meant for Quick Access: Hearth holds Steam's virtual controllers meanwhile (guide.rs hold_steam_pads).
//! Only this user's processes, only ones stopped by a signal (state T), only the kind HHD stops: nothing else is
//! touched, and a game Hearth put aside itself (Quick Resume) is never Steam's client (procs.rs is_steam_client).

use std::path::Path;

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn getuid() -> u32;
}
const SIGCONT: i32 = 18;

/// The kind of process HHD stops: its first or second argument (a NUL-separated /proc cmdline) ends in "/steam"
/// (Steam's client, ~/.local/share/Steam/ubuntu12_32/steam, or a shell running a script called steam).
pub fn hhd_would_stop(cmdline: &[u8]) -> bool {
    cmdline.split(|b| *b == 0).take(2).any(|a| a.ends_with(b"/steam"))
}

/// The state letter of a /proc/<pid>/stat line ("pid (comm) S ppid ..."; the name can hold spaces and ')').
pub fn state(stat: &str) -> Option<char> {
    let close = stat.rfind(')')?;
    stat.get(close + 2..)?.chars().next()
}

/// Stopped by a signal (SIGSTOP): T. A debugger's stop (t) isn't ours to undo.
pub fn is_stopped(stat: &str) -> bool { state(stat) == Some('T') }

/// This user's processes of the kind HHD stops, running or not (`root` is /proc, or a pretend one in tests).
pub fn steam_processes(root: &Path, uid: u32) -> Vec<u32> {
    use std::os::unix::fs::MetadataExt;
    let Ok(rd) = std::fs::read_dir(root) else { return vec![] };
    let mut out: Vec<u32> = rd.flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok().map(|pid| (pid, e.path())))
        .filter(|(_, dir)| std::fs::metadata(dir).map(|m| m.uid() == uid).unwrap_or(false))
        .filter(|(_, dir)| std::fs::read(dir.join("cmdline")).map(|c| hhd_would_stop(&c)).unwrap_or(false))
        .map(|(pid, _)| pid)
        .collect();
    out.sort_unstable();
    out
}

/// Which of `pids` are stopped now.
pub fn stopped(root: &Path, pids: &[u32]) -> Vec<u32> {
    pids.iter().copied().filter(|p| std::fs::read_to_string(root.join(p.to_string()).join("stat")).map(|s| is_stopped(&s)).unwrap_or(false)).collect()
}

pub fn uid() -> u32 { unsafe { getuid() } }

/// Lets the given stopped processes run again (SIGCONT); returns the ones that were let go.
pub fn let_go(pids: &[u32]) -> Vec<u32> {
    pids.iter().copied().filter(|p| *p > 1 && unsafe { kill(*p as i32, SIGCONT) } == 0).collect()
}

/// Steam stopped by HHD (or anything else) is let go: returns the processes that were.
pub fn let_steam_run() -> Vec<u32> {
    let root = Path::new("/proc");
    let_go(&stopped(root, &steam_processes(root, uid())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fake_proc(name: &str, procs: &[(u32, &[u8], &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("hearth-steam-thaw-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (pid, cmd, stat) in procs {
            let d = root.join(pid.to_string());
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("cmdline"), cmd).unwrap();
            std::fs::write(d.join("stat"), stat).unwrap();
        }
        root
    }

    #[test]
    fn the_processes_hhd_stops() {
        // Steam's client, as Flatpak Steam runs it.
        assert!(hhd_would_stop(b"/home/me/.local/share/Steam/ubuntu12_32/steam\0-silent\0"));
        // A shell running a script called steam (argument 2).
        assert!(hhd_would_stop(b"/bin/bash\0/app/bin/steam\0-silent\0"));
        // Not: a game, Steam's helpers, its reaper, Hearth's Steam engine, "steam" with no folder, a later argument.
        assert!(!hhd_would_stop(b"/home/me/.local/share/Steam/ubuntu12_32/reaper\0SteamLaunch\0AppId=620\0--\0"));
        assert!(!hhd_would_stop(b"/home/me/.local/share/Steam/ubuntu12_64/steamwebhelper\0-lang=en\0"));
        assert!(!hhd_would_stop(b"/usr/lib/hearth-steam/hearth-steam\0"));
        assert!(!hhd_would_stop(b"steam\0"));
        assert!(!hhd_would_stop(b"/usr/bin/python3\0/usr/bin/tool\0/x/steam\0"));
        assert!(!hhd_would_stop(b""));
    }

    #[test]
    fn stopped_state_from_stat() {
        assert_eq!(state("1234 (steam) T 1200 1234"), Some('T'));
        assert!(is_stopped("1234 (steam) T 1200 1234"));
        // The name can hold spaces and ')'.
        assert!(is_stopped("77 (a) b (c)) T 1 2 3"));
        assert!(!is_stopped("77 (a) T (c)) S 1 2 3"));
        assert!(!is_stopped("1234 (steam) S 1200 1234"));
        assert!(!is_stopped("1234 (steam) t 1200 1234"));   // a debugger's stop
        assert!(!is_stopped("garbage"));
        assert_eq!(state(""), None);
    }

    #[test]
    fn only_stopped_steam_is_found() {
        let root = fake_proc("find", &[
            (100, b"/home/me/.local/share/Steam/ubuntu12_32/steam\0-silent\0", "100 (steam) T 90 100"),
            (101, b"/bin/bash\0/app/bin/steam\0", "101 (steam) S 1 101"),
            (102, b"/home/me/.local/share/Steam/ubuntu12_32/reaper\0SteamLaunch\0AppId=620\0", "102 (reaper) T 100 102"),
            (103, b"/games/Wrath.exe\0", "103 (Wrath.exe) R 1 103"),
        ]);
        let me = uid();
        let all = steam_processes(&root, me);
        assert_eq!(all, vec![100, 101]);
        // The game Hearth put aside (102, Steam's reaper) is never Steam: only 100 is let go.
        assert_eq!(stopped(&root, &all), vec![100]);
        // Someone else's processes aren't looked at.
        assert!(steam_processes(&root, me.wrapping_add(1)).is_empty());
        // Gone meanwhile: nothing, no error.
        assert!(stopped(&root, &[999]).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The real thing: a process called .../steam stopped the way HHD stops Steam (SIGSTOP), found in /proc and let go.
    #[test]
    fn a_stopped_steam_runs_again() {
        let dir = std::env::temp_dir().join(format!("hearth-steam-thaw-real-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("steam");
        let Ok(_) = std::fs::copy("/bin/sleep", &fake) else { return };   // no /bin/sleep: nothing to test with
        let mut child = std::process::Command::new(&fake).arg("30").spawn().unwrap();
        let pid = child.id();
        let root = Path::new("/proc");
        let stat = |p: u32| std::fs::read_to_string(format!("/proc/{p}/stat")).unwrap_or_default();
        let wait_for = |want: bool| (0..100).any(|_| { if is_stopped(&stat(pid)) == want { true } else { std::thread::sleep(std::time::Duration::from_millis(20)); false } });
        assert!(steam_processes(root, uid()).contains(&pid), "HHD's kind of Steam process not found: {pid}");
        assert!(std::process::Command::new("kill").args(["-STOP", &pid.to_string()]).status().unwrap().success());
        assert!(wait_for(true), "didn't stop: {}", stat(pid));
        assert_eq!(stopped(root, &[pid]), vec![pid]);
        assert_eq!(let_go(&stopped(root, &[pid])), vec![pid]);
        assert!(wait_for(false), "still stopped after SIGCONT: {}", stat(pid));
        assert!(stopped(root, &[pid]).is_empty());
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_to_let_go() {
        assert!(let_go(&[]).is_empty());
        // pid 0 / 1 are never signalled (kill(0) would mean the whole process group).
        assert!(let_go(&[0, 1]).is_empty());
    }
}
