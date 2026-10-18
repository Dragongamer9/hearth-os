//! Reading /proc to tell whether a game is running, and to pause or stop it. Port of library/proc.js.

use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Proc { pub pid: u32, pub ppid: u32, pub cmdline: Vec<String>, pub cwd: Option<String>, pub prefix: Option<String> }

/// What belongs to a game: Steam's wrapper with AppId=<id>, a process Hearth started, or anything running
/// from the game's folder. Children count too, and anything in the same Wine prefix (launchers).
#[derive(Debug, Clone, Default)]
pub struct Track { pub steam_app_id: Option<String>, pub pid: Option<u32>, pub dirs: Vec<String> }

pub fn snapshot(root: &Path) -> Vec<Proc> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(pid) = name.parse::<u32>() else { continue };
        let dir = e.path();
        let Ok(stat) = std::fs::read_to_string(dir.join("stat")) else { continue };
        // "pid (comm with spaces) S ppid ...": the name can contain ')', so split at the last one.
        let ppid = stat.rfind(')').and_then(|i| stat[i + 2..].split(' ').nth(1)).and_then(|s| s.parse().ok()).unwrap_or(0);
        let cmdline: Vec<String> = std::fs::read(dir.join("cmdline")).map(|b| b.split(|c| *c == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8_lossy(s).into_owned()).collect()).unwrap_or_default();
        let cwd = std::fs::read_link(dir.join("cwd")).ok().map(|p| p.to_string_lossy().into_owned());
        let wine = cmdline.iter().any(|a| a.to_lowercase().ends_with(".exe") || ["wine", "wine64", "wine64-preloader", "proton"].iter().any(|w| a == w || a.ends_with(&format!("/{w}"))));
        let prefix = if wine {
            std::fs::read(dir.join("environ")).ok().and_then(|b| {
                let env: Vec<String> = b.split(|c| *c == 0).map(|s| String::from_utf8_lossy(s).into_owned()).collect();
                let v = |k: &str| env.iter().find_map(|e| e.strip_prefix(&format!("{k}="))).map(str::to_string).filter(|s| !s.is_empty());
                v("WINEPREFIX").or_else(|| v("STEAM_COMPAT_DATA_PATH").map(|p| format!("{}/pfx", p.trim_end_matches('/'))))
            })
        } else { None };
        out.push(Proc { pid, ppid, cmdline, cwd, prefix });
    }
    out
}

fn under(p: &str, dir: &str) -> bool {
    let d = dir.trim_end_matches('/');
    !p.is_empty() && !d.is_empty() && (p.trim_end_matches('/') == d || p.starts_with(&format!("{d}/")))
}

pub fn game_processes(track: &Track, procs: &[Proc], me: u32) -> Vec<u32> {
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
    all.remove(&me);
    let mut v: Vec<u32> = all.into_iter().collect();
    v.sort();
    v
}

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

#[cfg(test)]
mod tests {
    use super::*;
    fn p(pid: u32, ppid: u32, cmd: &[&str], cwd: Option<&str>, prefix: Option<&str>) -> Proc {
        Proc { pid, ppid, cmdline: cmd.iter().map(|s| s.to_string()).collect(), cwd: cwd.map(str::to_string), prefix: prefix.map(str::to_string) }
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
}
