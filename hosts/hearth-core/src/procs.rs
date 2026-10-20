//! Reading /proc to tell whether a game is running, and to pause or stop it. Port of library/proc.js.
//!
//! Which processes are a game's, most certain first:
//!   1. its number (appid.rs): processes under Steam's reaper with AppId=<id> (Steam games), or carrying
//!      HEARTH_GAME_ID=<id> in their environment (everything Hearth starts; children and Wine's processes inherit
//!      it, even the ones that move away from their parent, like wineserver).
//!   2. only when no process carries the number (a game started outside Hearth, or by an older Hearth): the old
//!      guesses: the process Hearth started, Steam's AppId= argument, the game's folders, its Wine prefix.
//! A process that carries another game's number is never counted, and Steam's own (steam, steamwebhelper) never are,
//! so pausing, resuming or closing one game can't touch another game or Steam.

use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub cmdline: Vec<String>,
    pub cwd: Option<String>,
    pub prefix: Option<String>,
    /// Its name, as /proc/<pid>/stat has it ("reaper", "steam", "wine64-preloader").
    pub comm: String,
    /// HEARTH_GAME_ID in its environment.
    pub tag: Option<u32>,
    /// Steam's reaper for a game ("reaper SteamLaunch AppId=620 -- ..."): that app id.
    pub reaper_app: Option<u32>,
}

/// What belongs to a game: its number first (`app_id`), then the guesses: Steam's wrapper with AppId=<id>, a process
/// Hearth started, or anything running from the game's folder. Children count too, and anything in the same Wine
/// prefix (launchers).
#[derive(Debug, Clone, Default)]
pub struct Track { pub steam_app_id: Option<String>, pub pid: Option<u32>, pub dirs: Vec<String>, pub app_id: Option<u32> }

/// Steam's own processes: never part of a game, never paused or closed with one.
pub fn is_steam_client(comm: &str) -> bool { matches!(comm, "steam" | "steamwebhelper" | "steam.sh") }

/// One variable from a /proc/<pid>/environ blob (NUL-separated KEY=value).
pub fn env_value(environ: &[u8], key: &str) -> Option<String> {
    let k = key.as_bytes();
    environ.split(|c| *c == 0).find_map(|e| {
        if e.len() > k.len() && e.starts_with(k) && e[k.len()] == b'=' { Some(String::from_utf8_lossy(&e[k.len() + 1..]).into_owned()) } else { None }
    })
}

/// Steam's reaper for a game: "reaper SteamLaunch AppId=620 -- ...", read the way gamescope reads it.
pub fn reaper_app(comm: &str, cmdline: &[String]) -> Option<u32> {
    if comm != "reaper" { return None; }
    let mut launch = false;
    for a in cmdline.iter().skip(1) {
        if a == "--" { break; }
        if a == "SteamLaunch" { launch = true; continue; }
        if let Some(n) = a.strip_prefix("AppId=").and_then(|n| n.parse::<u32>().ok()) { if launch && n != 0 { return Some(n); } }
    }
    None
}

/// One process from /proc (or a pretend one in tests).
pub fn read_proc(root: &Path, pid: u32) -> Option<Proc> {
    let dir = root.join(pid.to_string());
    let stat = std::fs::read_to_string(dir.join("stat")).ok()?;
    // "pid (comm with spaces) S ppid ...": the name can contain ')', so split at the last one.
    let close = stat.rfind(')')?;
    let comm = stat.find('(').filter(|i| *i < close).map(|i| stat[i + 1..close].to_string()).unwrap_or_default();
    let ppid = stat.get(close + 2..).and_then(|r| r.split(' ').nth(1)).and_then(|s| s.parse().ok()).unwrap_or(0);
    let cmdline: Vec<String> = std::fs::read(dir.join("cmdline")).map(|b| b.split(|c| *c == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8_lossy(s).into_owned()).collect()).unwrap_or_default();
    let cwd = std::fs::read_link(dir.join("cwd")).ok().map(|p| p.to_string_lossy().into_owned());
    // Other users' processes can't be read: no environment, which is fine (they're nobody's game here).
    let environ = std::fs::read(dir.join("environ")).unwrap_or_default();
    let tag = env_value(&environ, crate::appid::ENV).and_then(|v| v.trim().parse::<u32>().ok()).filter(|n| *n != 0);
    let wine = cmdline.iter().any(|a| a.to_lowercase().ends_with(".exe") || ["wine", "wine64", "wine64-preloader", "proton"].iter().any(|w| a == w || a.ends_with(&format!("/{w}"))));
    let prefix = if wine {
        let v = |k: &str| env_value(&environ, k).filter(|s| !s.is_empty());
        v("WINEPREFIX").or_else(|| v("STEAM_COMPAT_DATA_PATH").map(|p| format!("{}/pfx", p.trim_end_matches('/'))))
    } else { None };
    let reaper_app = reaper_app(&comm, &cmdline);
    Some(Proc { pid, ppid, cmdline, cwd, prefix, comm, tag, reaper_app })
}

pub fn snapshot(root: &Path) -> Vec<Proc> {
    let Ok(rd) = std::fs::read_dir(root) else { return vec![] };
    rd.flatten().filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok()).filter_map(|pid| read_proc(root, pid)).collect()
}

/// Which game (by number) a process belongs to: the Steam game whose reaper it runs under, else the HEARTH_GAME_ID
/// nearest up its parents (itself first). Nothing from Hearth (`me`) up counts, and Steam's own processes belong to
/// no game (a Steam started from inside a game has that game's environment).
fn owner_by(by: &HashMap<u32, &Proc>, pid: u32, me: u32) -> Option<u32> {
    let first = by.get(&pid)?;
    if is_steam_client(&first.comm) { return None; }
    let mut tag = None;
    let mut cur = pid;
    for _ in 0..64 {
        if cur == me || cur <= 1 { break; }
        let Some(p) = by.get(&cur) else { break };
        if let Some(a) = p.reaper_app { return Some(a); }
        if tag.is_none() { tag = p.tag; }
        cur = p.ppid;
    }
    tag
}

/// Every process that belongs to a game: pid → the game's number.
pub fn owners(procs: &[Proc], me: u32) -> HashMap<u32, u32> {
    let by: HashMap<u32, &Proc> = procs.iter().map(|p| (p.pid, p)).collect();
    procs.iter().filter_map(|p| owner_by(&by, p.pid, me).map(|o| (p.pid, o))).collect()
}

/// The game (by number) one process belongs to, reading only it and its parents (for a window's process).
pub fn owner_of(root: &Path, pid: u32, me: u32) -> Option<u32> {
    let mut chain: HashMap<u32, Proc> = HashMap::new();
    let mut cur = pid;
    for _ in 0..64 {
        if cur == me || cur <= 1 || chain.contains_key(&cur) { break; }
        let Some(p) = read_proc(root, cur) else { break };
        let next = p.ppid;
        chain.insert(cur, p);
        cur = next;
    }
    let by: HashMap<u32, &Proc> = chain.iter().map(|(k, v)| (*k, v)).collect();
    owner_by(&by, pid, me)
}

/// Is `pid` a child, grandchild... of `ancestor`?
pub fn descends_from(root: &Path, pid: u32, ancestor: u32) -> bool {
    let mut cur = pid;
    for _ in 0..64 {
        let Some(p) = read_proc(root, cur) else { return false };
        if p.ppid == ancestor { return true; }
        if p.ppid <= 1 || p.ppid == cur { return false; }
        cur = p.ppid;
    }
    false
}

/// A process's name ("" when it's gone).
pub fn comm(root: &Path, pid: u32) -> String {
    std::fs::read_to_string(root.join(pid.to_string()).join("comm")).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn under(p: &str, dir: &str) -> bool {
    let d = dir.trim_end_matches('/');
    !p.is_empty() && !d.is_empty() && (p.trim_end_matches('/') == d || p.starts_with(&format!("{d}/")))
}

/// The old guesses (for games whose processes carry no number).
fn guess(track: &Track, procs: &[Proc]) -> HashSet<u32> {
    let mut roots: HashSet<u32> = HashSet::new();
    for p in procs {
        if track.pid == Some(p.pid) { roots.insert(p.pid); }
        if let Some(id) = &track.steam_app_id { if p.cmdline.iter().any(|a| a == &format!("AppId={id}")) { roots.insert(p.pid); } }
        if !track.dirs.is_empty() {
            // The program can be a later argument under Wine, written as Z:\home\...: back to a Linux path.
            let paths: Vec<String> = p.cmdline.iter().take(4).map(|a| {
                let b = a.as_bytes();
                if b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\' { a[2..].replace('\\', "/") } else { a.clone() }
            }).collect();
            if track.dirs.iter().any(|d| p.cwd.as_deref().map(|c| under(c, d)).unwrap_or(false) || paths.iter().any(|a| under(a, d))) { roots.insert(p.pid); }
        }
    }
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in procs { kids.entry(p.ppid).or_default().push(p.pid); }
    let mut all: HashSet<u32> = roots.clone();
    let mut queue: Vec<u32> = roots.into_iter().collect();
    while let Some(q) = queue.pop() { for k in kids.get(&q).cloned().unwrap_or_default() { if all.insert(k) { queue.push(k); } } }
    // Launchers started by Wine's own services run in the game's prefix: they belong to it too.
    let prefixes: HashSet<String> = procs.iter().filter(|p| all.contains(&p.pid)).filter_map(|p| p.prefix.clone()).filter(|x| !x.trim_end_matches('/').ends_with("/.wine")).collect();
    if !prefixes.is_empty() { for p in procs { if p.prefix.as_ref().map(|x| prefixes.contains(x)).unwrap_or(false) { all.insert(p.pid); } } }
    all
}

/// The numbers a track answers to: its own, and its Steam app id.
fn track_ids(track: &Track) -> Vec<u32> {
    track.app_id.into_iter().chain(track.steam_app_id.as_deref().and_then(|s| s.parse::<u32>().ok())).filter(|n| *n != 0).collect()
}

/// A game's processes, sorted. `how` says which way they were found ("id" or "guess"), for the log.
pub fn game_processes_how(track: &Track, procs: &[Proc], me: u32) -> (Vec<u32>, &'static str) {
    let own = owners(procs, me);
    let ids = track_ids(track);
    let mut exact: Vec<u32> = own.iter().filter(|(pid, o)| ids.contains(*o) && **pid != me).map(|(pid, _)| *pid).collect();
    if !exact.is_empty() { exact.sort(); return (exact, "id"); }
    let client: HashSet<u32> = procs.iter().filter(|p| is_steam_client(&p.comm)).map(|p| p.pid).collect();
    let mut v: Vec<u32> = guess(track, procs).into_iter()
        .filter(|p| *p != me && !client.contains(p) && own.get(p).map(|o| ids.contains(o)).unwrap_or(true))
        .collect();
    v.sort();
    (v, "guess")
}
pub fn game_processes(track: &Track, procs: &[Proc], me: u32) -> Vec<u32> { game_processes_how(track, procs, me).0 }

pub fn signal(pids: &[u32], sig: &str) -> usize {
    pids.iter().filter(|p| std::process::Command::new("kill").args([&format!("-{sig}"), &p.to_string()]).status().map(|s| s.success()).unwrap_or(false)).count()
}
pub fn alive(pid: u32) -> bool { Path::new(&format!("/proc/{pid}")).exists() }

/// SIGTERM (and SIGCONT, so a paused game acts on it), then SIGKILL what's left after `grace`.
pub fn stop(pids: &[u32], grace: std::time::Duration) -> bool {
    signal(pids, "TERM"); signal(pids, "CONT");
    let until = std::time::Instant::now() + grace;
    while std::time::Instant::now() < until && pids.iter().any(|p| alive(*p)) { std::thread::sleep(std::time::Duration::from_millis(250)); }
    let left: Vec<u32> = pids.iter().copied().filter(|p| alive(*p)).collect();
    signal(&left, "KILL");
    std::thread::sleep(std::time::Duration::from_millis(300));
    !pids.iter().any(|p| alive(*p))
}

/// A short "12 processes (101, 102, 103 …)" for the log.
pub fn describe(pids: &[u32]) -> String {
    let shown: Vec<String> = pids.iter().take(12).map(|p| p.to_string()).collect();
    format!("{} process{} ({}{})", pids.len(), if pids.len() == 1 { "" } else { "es" }, shown.join(", "), if pids.len() > 12 { " …" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(pid: u32, ppid: u32, cmd: &[&str], cwd: Option<&str>, prefix: Option<&str>) -> Proc {
        Proc { pid, ppid, cmdline: cmd.iter().map(|s| s.to_string()).collect(), cwd: cwd.map(str::to_string), prefix: prefix.map(str::to_string), ..Default::default() }
    }
    #[test]
    fn finds_a_games_processes() {
        let procs = vec![
            p(10, 1, &["reaper", "SteamLaunch", "AppId=620", "--"], None, None),
            p(11, 10, &["portal2_linux"], None, None),
            p(20, 1, &["wine64-preloader", "Z:\\home\\sam\\Games\\NWN\\nwmain.exe"], None, Some("/p/gog-1")),
            p(21, 1, &["services.exe"], None, Some("/p/gog-1")),            // the game's launcher, via Wine's services
            p(22, 1, &["explorer.exe"], None, Some("/home/sam/.wine")),     // another program in the shared prefix
            p(30, 1, &["bash"], Some("/home/sam/Games/Thing"), None),
            p(99, 1, &["hearth"], None, None),
        ];
        assert_eq!(game_processes(&Track { steam_app_id: Some("620".into()), ..Default::default() }, &procs, 99), vec![10, 11]);
        assert_eq!(game_processes(&Track { dirs: vec!["/home/sam/Games/NWN".into()], ..Default::default() }, &procs, 99), vec![20, 21]);
        assert_eq!(game_processes(&Track { dirs: vec!["/home/sam/Games/Thing/".into()], ..Default::default() }, &procs, 99), vec![30]);
        assert_eq!(game_processes(&Track { pid: Some(99), ..Default::default() }, &procs, 99), Vec::<u32>::new());
    }
    #[test]
    fn reads_this_system() {
        let me = std::process::id();
        assert!(snapshot(Path::new("/proc")).iter().any(|p| p.pid == me));
    }
    #[test]
    fn reads_environment_variables() {
        let env = b"PATH=/usr/bin\0HEARTH_GAME_IDX=7\0HEARTH_GAME_ID=2147487744\0SteamGameId=0\0";
        assert_eq!(env_value(env, "HEARTH_GAME_ID").as_deref(), Some("2147487744"));
        assert_eq!(env_value(env, "SteamGameId").as_deref(), Some("0"));
        assert_eq!(env_value(env, "HEARTH_GAME"), None);
        assert_eq!(env_value(b"", "PATH"), None);
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(reaper_app("reaper", &args(&["/s/reaper", "SteamLaunch", "AppId=620", "--", "AppId=9"])), Some(620));
        assert_eq!(reaper_app("reaper", &args(&["/s/reaper", "AppId=620"])), None);          // not a game launch
        assert_eq!(reaper_app("bash", &args(&["bash", "SteamLaunch", "AppId=620"])), None);  // only Steam's reaper
    }

    /// A pretend /proc: (pid, ppid, name, command line, environment, working folder).
    fn fake_proc(root: &Path, list: &[(u32, u32, &str, &[&str], &[&str], Option<&str>)]) {
        for (pid, ppid, comm, cmd, env, cwd) in list {
            let d = root.join(pid.to_string());
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("stat"), format!("{pid} ({comm}) S {ppid} {pid} {pid} 0 -1 4194560 0 0 0 0 0 0 0 0 20 0 1 0 100 0 0")).unwrap();
            std::fs::write(d.join("comm"), format!("{comm}\n")).unwrap();
            std::fs::write(d.join("cmdline"), cmd.iter().map(|a| format!("{a}\0")).collect::<String>()).unwrap();
            std::fs::write(d.join("environ"), env.iter().map(|a| format!("{a}\0")).collect::<String>()).unwrap();
            if let Some(c) = cwd { std::os::unix::fs::symlink(c, d.join("cwd")).unwrap(); }
        }
    }

    #[test]
    fn groups_two_games_by_number_with_wine_and_a_shared_steam() {
        const A: u32 = 0x8000_1000;   // an Epic game with Proton (umu)
        const B: u32 = 0x8000_2000;   // an emulator
        const C: u32 = 0x8000_3000;   // a game from an older Hearth (no number on its processes)
        let (a, b) = (format!("HEARTH_GAME_ID={A}"), format!("HEARTH_GAME_ID={B}"));
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fake_proc(root, &[
            (50, 1, "hearth", &["/usr/bin/hearth"], &["HOME=/home/sam"], None),
            // Game A: umu → pressure-vessel → Proton → Wine; umu sets SteamGameId=0, GAMEID stays umu's.
            (100, 50, "umu-run", &["python3", "/usr/bin/umu-run", "/games/A/A.exe"], &[a.as_str(), "GAMEID=0"], None),
            (101, 100, "pv-adverb", &["pv-adverb", "--", "proton"], &[a.as_str(), "SteamGameId=0", "SteamAppId=0"], None),
            (102, 101, "A.exe", &["wine64-preloader", "Z:\\games\\A\\A.exe"], &[a.as_str(), "SteamGameId=0", "WINEPREFIX=/p/a"], None),
            // Wine's own processes move away from their parent but keep the environment.
            (103, 1, "wineserver", &["wineserver"], &[a.as_str(), "WINEPREFIX=/p/a"], None),
            (104, 1, "services.exe", &["C:\\windows\\system32\\services.exe"], &[a.as_str(), "WINEPREFIX=/p/a"], None),
            // Game B: an emulator, and a helper that started with an empty environment (its parent says whose it is).
            (200, 50, "retroarch", &["retroarch", "-f", "/roms/snes/x.sfc"], &[b.as_str()], None),
            (201, 200, "helper", &["helper"], &[], None),
            // Steam, started from inside game A: Steam's own processes have A's environment but are nobody's game;
            // the Steam game it runs belongs to Steam's app id, not to A.
            (300, 1, "steam", &["/home/sam/.steam/steam/ubuntu12_32/steam", "-silent"], &[a.as_str()], None),
            (301, 300, "steamwebhelper", &["steamwebhelper"], &[a.as_str()], None),
            (302, 300, "reaper", &["/s/reaper", "SteamLaunch", "AppId=620", "--", "/s/portal2.sh"], &[a.as_str(), "SteamGameId=620"], None),
            (303, 302, "portal2_linux", &["portal2_linux"], &[a.as_str(), "SteamGameId=620", "SteamAppId=620"], None),
            // Something in game C's folder that carries no number (C was started by an older Hearth).
            (400, 1, "C", &["/games/C/C.x86_64"], &[], Some("/games/C")),
        ]);
        let procs = snapshot(root);
        assert_eq!(procs.len(), 13);
        let get = |pid: u32| procs.iter().find(|p| p.pid == pid).unwrap().clone();
        assert_eq!(get(102).tag, Some(A));
        assert_eq!(get(102).prefix.as_deref(), Some("/p/a"));
        assert_eq!(get(201).tag, None);
        assert_eq!(get(302).reaper_app, Some(620));
        assert_eq!(get(303).comm, "portal2_linux");

        let track = |id: u32| Track { app_id: Some(id), ..Default::default() };
        assert_eq!(game_processes_how(&track(A), &procs, 50), (vec![100, 101, 102, 103, 104], "id"));
        assert_eq!(game_processes(&track(B), &procs, 50), vec![200, 201]);
        // Steam's own processes never; the reaper and the game, yes.
        assert_eq!(game_processes(&Track { app_id: Some(620), steam_app_id: Some("620".into()), ..Default::default() }, &procs, 50), vec![302, 303]);
        // With its number found, the guesses aren't used (C's folder holds a process that isn't A's).
        assert_eq!(game_processes(&Track { app_id: Some(A), dirs: vec!["/games/C".into()], ..Default::default() }, &procs, 50), vec![100, 101, 102, 103, 104]);
        // Nothing carries C's number: the guesses, but never another game's processes (A's Wine processes match the
        // folder and prefix guesses here) and never Steam's.
        let c = Track { app_id: Some(C), dirs: vec!["/games/C".into(), "/games/A".into()], pid: Some(300), ..Default::default() };
        assert_eq!(game_processes_how(&c, &procs, 50), (vec![400], "guess"));
        // One process at a time (a window's): the same answers.
        assert_eq!(owner_of(root, 104, 50), Some(A));
        assert_eq!(owner_of(root, 201, 50), Some(B));
        assert_eq!(owner_of(root, 303, 50), Some(620));
        assert_eq!(owner_of(root, 301, 50), None);
        assert_eq!(owner_of(root, 400, 50), None);
        assert_eq!(owner_of(root, 50, 50), None);
        assert!(descends_from(root, 201, 50));
        assert!(!descends_from(root, 104, 50));
        assert_eq!(comm(root, 302), "reaper");
        assert_eq!(describe(&[1, 2]), "2 processes (1, 2)");
    }
}
