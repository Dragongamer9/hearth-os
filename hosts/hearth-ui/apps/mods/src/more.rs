//! The rest of what the Mod Manager works out itself: the script extender's version and what each mod needs of it
//! (board 1's Health), what an install would change (board 2), the crash's details (board 5: minutes into play, Proton's
//! log, what ran, what wasn't the cause, others who had it), speaking to Describe it (whisper.cpp, as Hearth Listen
//! uses it), the nightly update check (a user timer, only while "Every night" is picked) and how long snapshots stay.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ the script extender (SKSE, F4SE)

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// A Windows program's own version (its VS_FIXEDFILEINFO): a.b.c.d.
pub fn pe_version(b: &[u8]) -> Option<[u16; 4]> {
    let sig = [0xBD, 0x04, 0xEF, 0xFE];
    let i = b.windows(4).position(|w| w == sig)?;
    let ms = u32_at(b, i + 8)?;
    let ls = u32_at(b, i + 12)?;
    Some([(ms >> 16) as u16, ms as u16, (ls >> 16) as u16, ls as u16])
}

/// SKSE's loader says 0.2.2.6 for SKSE 2.2.6.
pub fn extender_words(v: [u16; 4]) -> String {
    if v[0] == 0 {
        format!("{}.{}.{}", v[1], v[2], v[3])
    } else {
        format!("{}.{}.{}", v[0], v[1], v[2])
    }
}

/// A file offset for a place in a PE image (its RVA).
fn rva_off(b: &[u8], sections: &[(u32, u32, u32, u32)], rva: u32) -> Option<usize> {
    sections.iter().find(|s| rva >= s.0 && rva < s.0 + s.1.max(s.2)).map(|s| (s.3 + rva - s.0) as usize).filter(|o| *o < b.len())
}

/// An exported symbol's file offset in a PE image (a 64-bit DLL).
pub fn export_off(b: &[u8], want: &str) -> Option<usize> {
    let pe = u32_at(b, 0x3C)? as usize;
    if b.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let n_sec = u16_at(b, pe + 6)? as usize;
    let opt_size = u16_at(b, pe + 20)? as usize;
    let opt = pe + 24;
    let magic = u16_at(b, opt)?;
    let dd = opt + if magic == 0x20B { 112 } else { 96 };
    let exp_rva = u32_at(b, dd)?;
    let sec0 = opt + opt_size;
    let sections: Vec<(u32, u32, u32, u32)> = (0..n_sec).filter_map(|i| {
        let s = sec0 + i * 40;
        Some((u32_at(b, s + 12)?, u32_at(b, s + 8)?, u32_at(b, s + 16)?, u32_at(b, s + 20)?))
    }).collect();
    let e = rva_off(b, &sections, exp_rva)?;
    let names = u32_at(b, e + 24)? as usize;
    let funcs = rva_off(b, &sections, u32_at(b, e + 28)?)?;
    let name_ptrs = rva_off(b, &sections, u32_at(b, e + 32)?)?;
    let ords = rva_off(b, &sections, u32_at(b, e + 36)?)?;
    for i in 0..names.min(20_000) {
        let np = rva_off(b, &sections, u32_at(b, name_ptrs + i * 4)?)?;
        let end = b[np..].iter().position(|c| *c == 0)? + np;
        if &b[np..end] == want.as_bytes() {
            let ord = u16_at(b, ords + i * 2)? as usize;
            return rva_off(b, &sections, u32_at(b, funcs + ord * 4)?);
        }
    }
    None
}

/// The SKSE version a plugin DLL asks for (its SKSEPlugin_Version data's seVersionRequired), e.g. 2.2.6.
pub fn skse_needed(dll: &[u8]) -> Option<String> {
    let at = export_off(dll, "SKSEPlugin_Version").or_else(|| export_off(dll, "F4SEPlugin_Version"))?;
    let v = u32_at(dll, at + 844)?;
    if v == 0 {
        return None;
    }
    Some(format!("{}.{}.{}", v >> 24, (v >> 16) & 0xFF, (v >> 4) & 0xFFF))
}

fn ver_num(s: &str) -> Vec<u64> {
    s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse().unwrap_or(0)).collect()
}

/// The script extender you have (its loader in the game folder or in a mod), and each mod on that needs a newer one:
/// (have, [(mod, needs)]).
pub fn extender_check(inst: &Value, mods: &[Value], mods_dir: &Path) -> (String, Vec<(String, String)>) {
    let kind = inst["game"].as_str().unwrap_or("");
    let loader = match kind {
        "skyrimse" | "enderalse" => "skse64_loader.exe",
        "skyrimvr" => "sksevr_loader.exe",
        "fallout4" => "f4se_loader.exe",
        _ => return (String::new(), Vec::new()),
    };
    let mut have = String::new();
    let mut places: Vec<PathBuf> = inst["gameDir"].as_str().map(|g| vec![PathBuf::from(g)]).unwrap_or_default();
    for m in mods.iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true) {
        places.push(mods_dir.join(m["id"].as_str().unwrap_or("")));
    }
    for d in places.iter().rev() {
        if let Some(p) = hearth_gaming::organizer::vfs::ci_find(d, loader) {
            if let Some(v) = std::fs::read(&p).ok().and_then(|b| pe_version(&b)) {
                have = extender_words(v);
                break;
            }
        }
    }
    let mut need = Vec::new();
    if have.is_empty() {
        return (have, need);
    }
    for m in mods.iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true) {
        let dir = mods_dir.join(m["id"].as_str().unwrap_or(""));
        for sub in ["SKSE/Plugins", "F4SE/Plugins"] {
            let Some(pd) = hearth_gaming::organizer::vfs::ci_resolve(&dir, sub) else { continue };
            for e in std::fs::read_dir(&pd).into_iter().flatten().flatten() {
                if !e.file_name().to_string_lossy().to_lowercase().ends_with(".dll") {
                    continue;
                }
                if let Some(n) = std::fs::read(e.path()).ok().and_then(|b| skse_needed(&b)) {
                    if ver_num(&n) > ver_num(&have) && !need.iter().any(|x: &(String, String)| x.0 == m["name"].as_str().unwrap_or("")) {
                        need.push((m["name"].as_str().unwrap_or("").to_string(), n));
                    }
                }
            }
        }
    }
    (have, need)
}

/// The script extender's own page on Nexus Mods (where Update SKSE takes it from).
pub fn extender_page(kind: &str) -> Option<(&'static str, u64)> {
    match kind {
        "skyrimse" => Some(("skyrimspecialedition", 30379)),
        "fallout4" => Some(("fallout4", 42147)),
        _ => None,
    }
}

/// Crash Logger's page for each game (Get Crash Logger for games that need it).
pub fn crash_logger_page(kind: &str) -> Option<(&'static str, u64, &'static str)> {
    match kind {
        "skyrimse" => Some(("skyrimspecialedition", 59818, "Crash Logger SSE AE VR")),
        "fallout4" => Some(("fallout4", 47359, "Buffout 4")),
        _ => None,
    }
}

pub fn has_crash_logger(mods_dir: &Path, mods: &[Value]) -> bool {
    mods.iter().filter(|m| m["kind"] == "mod").any(|m| {
        let d = mods_dir.join(m["id"].as_str().unwrap_or(""));
        ["SKSE/Plugins/CrashLogger.dll", "F4SE/Plugins/Buffout4.dll", "SKSE/Plugins/Trainwreck.dll"].iter().any(|f| hearth_gaming::organizer::vfs::ci_resolve(&d, f).is_some())
    })
}

/// The newest main file of a Nexus mod: its file id.
pub fn newest_main(domain: &str, mod_id: u64) -> Result<u64, String> {
    let acct = hearth_gaming::nexus::account().ok_or("Sign in to Nexus Mods (Accounts) first.")?;
    let f = hearth_gaming::nexus::files(&acct.key, domain, mod_id)?;
    f["files"].as_array().and_then(|a| a.iter().filter(|x| x["category_name"] == "MAIN").max_by_key(|x| x["uploaded_timestamp"].as_u64().unwrap_or(0))).and_then(|x| x["file_id"].as_u64()).ok_or("That page has no main file.".into())
}

// ------------------------------------------------------------------ what an install would change (board 2)

/// Every file the game sees now (lower-case path → the mod it comes from, or "" for the game's own), with each mod's
/// place in the list.
pub fn owners(inst: &Value, mods: &[Value], mods_dir: &Path) -> (BTreeMap<String, String>, BTreeMap<String, i64>) {
    let mut own: BTreeMap<String, String> = BTreeMap::new();
    let mut prio: BTreeMap<String, i64> = BTreeMap::new();
    if let Some(d) = inst["dataDir"].as_str() {
        let mut f = Vec::new();
        hearth_gaming::util::files_under(Path::new(d), &mut f);
        for x in f.iter().take(300_000) {
            if let Ok(r) = x.strip_prefix(d) {
                own.insert(r.to_string_lossy().to_lowercase(), String::new());
            }
        }
    }
    let mut on: Vec<&Value> = mods.iter().filter(|m| m["kind"] == "mod" && m["enabled"] == true).collect();
    on.sort_by_key(|m| m["priority"].as_i64().unwrap_or(0));
    for m in on {
        let id = m["id"].as_str().unwrap_or("");
        let name = m["name"].as_str().unwrap_or(id).to_string();
        prio.insert(name.clone(), m["priority"].as_i64().unwrap_or(0));
        let base = mods_dir.join(id);
        let mut f = Vec::new();
        hearth_gaming::util::files_under(&base, &mut f);
        for x in f {
            if let Ok(r) = x.strip_prefix(&base) {
                let r = r.to_string_lossy().to_lowercase();
                if r != "meta.ini" {
                    own.insert(r, name.clone());
                }
            }
        }
    }
    (own, prio)
}

/// The files an installer would put in (its evaluation's files, folders opened up), from the unpacked mod.
pub fn fomod_files(staged: &Path, ev: &Value) -> Vec<(String, PathBuf)> {
    // the installer's own folder: where fomod/ is
    let mut root = staged.to_path_buf();
    if hearth_gaming::organizer::vfs::ci_find(staged, "fomod").is_none() {
        if let Ok(rd) = std::fs::read_dir(staged) {
            for e in rd.flatten() {
                if hearth_gaming::organizer::vfs::ci_find(&e.path(), "fomod").is_some() {
                    root = e.path();
                }
            }
        }
    }
    let mut out = Vec::new();
    for f in ev["files"].as_array().cloned().unwrap_or_default() {
        let src = f["source"].as_str().unwrap_or("").replace('\\', "/");
        let dst = f["destination"].as_str().unwrap_or("").replace('\\', "/").trim_matches('/').to_string();
        let Some(p) = hearth_gaming::organizer::vfs::ci_resolve(&root, &src) else { continue };
        if p.is_dir() {
            let mut fs = Vec::new();
            hearth_gaming::util::files_under(&p, &mut fs);
            for x in fs {
                if let Ok(r) = x.strip_prefix(&p) {
                    let rel = if dst.is_empty() { r.to_string_lossy().to_string() } else { format!("{dst}/{}", r.to_string_lossy()) };
                    out.push((rel.to_lowercase(), x.clone()));
                }
            }
        } else {
            let rel = if dst.is_empty() { p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() } else { dst.clone() };
            out.push((rel.to_lowercase(), p));
        }
    }
    out
}

/// Before it goes in: (files, new, overwrites, it wins over [(mod, files)], it loses to [(mod, files)], the plugins it
/// adds with their masters).
#[derive(Debug, Default, Clone)]
pub struct Before {
    pub files: usize,
    pub new: usize,
    pub over: usize,
    pub wins: Vec<(String, usize)>,
    pub loses: Vec<(String, usize)>,
    pub needs: Vec<(String, bool)>,
    pub size: u64,
}

pub fn before(files: &[(String, PathBuf)], own: &BTreeMap<String, String>, prio: &BTreeMap<String, i64>, at: Option<i64>, have_plugins: &[String], extender: &str) -> Before {
    let mut b = Before { files: files.len(), ..Default::default() };
    let mut w: BTreeMap<String, usize> = BTreeMap::new();
    let mut l: BTreeMap<String, usize> = BTreeMap::new();
    let mut needs: Vec<(String, bool)> = Vec::new();
    let adding: Vec<String> = files.iter().filter(|f| hearth_gaming::organizer::plugins::is_plugin(&f.0)).map(|f| f.0.rsplit('/').next().unwrap_or("").to_string()).collect();
    for (rel, p) in files {
        b.size += std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        match own.get(rel) {
            None => b.new += 1,
            Some(m) => {
                b.over += 1;
                let theirs = prio.get(m).copied().unwrap_or(-1);
                let label = if m.is_empty() { "the game's own files".to_string() } else { m.clone() };
                if at.map(|a| theirs > a).unwrap_or(false) {
                    *l.entry(label).or_default() += 1;
                } else {
                    *w.entry(label).or_default() += 1;
                }
            }
        }
        if hearth_gaming::organizer::plugins::is_plugin(rel) {
            if let Ok(bytes) = std::fs::read(p) {
                for m in masters_of(&bytes) {
                    let ok = have_plugins.iter().any(|h| h.eq_ignore_ascii_case(&m)) || adding.iter().any(|a| a.eq_ignore_ascii_case(&m));
                    if !needs.iter().any(|n| n.0.eq_ignore_ascii_case(&m)) {
                        needs.push((m, ok));
                    }
                }
            }
        }
        let lr = rel.to_lowercase();
        if (lr.starts_with("skse/plugins/") || lr.starts_with("f4se/plugins/")) && lr.ends_with(".dll") && !extender.is_empty() {
            if let Some(n) = std::fs::read(p).ok().and_then(|x| skse_needed(&x)) {
                let ok = ver_num(&n) <= ver_num(extender);
                let label = format!("SKSE {n}");
                if !needs.iter().any(|x| x.0 == label) {
                    needs.push((label, ok));
                }
            }
        }
    }
    let mut wv: Vec<(String, usize)> = w.into_iter().collect();
    wv.sort_by(|a, b| b.1.cmp(&a.1));
    let mut lv: Vec<(String, usize)> = l.into_iter().collect();
    lv.sort_by(|a, b| b.1.cmp(&a.1));
    b.wins = wv;
    b.loses = lv;
    b.needs = needs;
    b
}

/// A plugin's masters (its TES4 header's MAST entries).
pub fn masters_of(b: &[u8]) -> Vec<String> {
    if b.get(0..4) != Some(b"TES4") {
        return Vec::new();
    }
    let size = u32_at(b, 4).unwrap_or(0) as usize;
    let h = b.get(24..24 + size).unwrap_or(&[]);
    let mut v = Vec::new();
    let mut i = 0;
    while i + 6 <= h.len() {
        let len = u16_at(h, i + 4).unwrap_or(0) as usize;
        if &h[i..i + 4] == b"MAST" {
            v.push(String::from_utf8_lossy(h.get(i + 6..i + 6 + len).unwrap_or(&[])).trim_end_matches('\0').to_string());
        }
        i += 6 + len;
    }
    v
}

/// An unpacked mod's read me (readme.txt, a .md or a .pdf in it).
pub fn readme(staged: &Path) -> Option<PathBuf> {
    let mut f = Vec::new();
    hearth_gaming::util::files_under(staged, &mut f);
    f.sort_by_key(|p| p.components().count());
    f.into_iter().find(|p| {
        let n = p.file_name().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        (n.contains("readme") || n.contains("read me") || n.contains("liesmich")) && [".txt", ".md", ".pdf", ".html", ".htm", ".rtf"].iter().any(|e| n.ends_with(e))
    })
}

// ------------------------------------------------------------------ crashes (board 5)

/// When each Play modded started (kept by the Mod Manager): what ran, on which device, with which profile.
pub fn note_play(game: &str, profile: &str, exe: &str, mods_on: usize) {
    let mut p = crate::back::read_own("plays.json");
    let mut list = p["plays"].as_array().cloned().unwrap_or_default();
    list.push(json!({"at": crate::now(), "game": game, "profile": profile, "exe": exe, "modsOn": mods_on, "device": crate::sync::here()}));
    let n = list.len();
    if n > 200 {
        list.drain(0..n - 200);
    }
    p["plays"] = json!(list);
    let _ = crate::back::write_own("plays.json", &p);
}

/// The start a crash happened in (the last Play modded before it, within six hours).
pub fn play_before(at: i64) -> Option<Value> {
    let p = crate::back::read_own("plays.json");
    p["plays"].as_array()?.iter().filter(|x| x["at"].as_i64().map(|t| t <= at && at - t < 6 * 3600).unwrap_or(false)).max_by_key(|x| x["at"].as_i64().unwrap_or(0)).cloned()
}

/// "Not the cause": what the log shows was fine (memory, the graphics card, Proton and the driver out of the call stack).
pub fn not_cause(c: &crate::logic::Crash) -> Vec<String> {
    let mut v = Vec::new();
    let num = |s: &str| s.trim().trim_end_matches("GB").trim().replace(',', ".").parse::<f64>().ok();
    let pair = |l: &str| -> Option<(f64, f64)> {
        let r = l.split_once(':')?.1;
        let (a, b) = r.split_once('/')?;
        Some((num(a)?, num(b.split_whitespace().next().unwrap_or(""))?))
    };
    let mut mem = None;
    let mut gpu = None;
    for l in &c.lines {
        let u = l.to_uppercase();
        if u.contains("PHYSICAL MEMORY") {
            mem = pair(l);
        } else if u.contains("GPU MEMORY") {
            gpu = pair(l);
        }
    }
    let driver = c.lines.iter().any(|l| {
        let x = l.to_lowercase();
        (x.contains("nvwgf") || x.contains("amdxx") || x.contains("dxvk") || x.contains("wined3d") || x.contains("vkd3d")) && x.contains('+')
    });
    let mut fine = Vec::new();
    if !driver {
        fine.push("Proton and the graphics driver weren't in the crash's call stack".to_string());
    }
    if let Some((a, b)) = gpu.filter(|(a, b)| a / b < 0.92) {
        fine.push(format!("{a:.1} of {b:.0} GB on the GPU"));
    }
    if let Some((a, b)) = mem.filter(|(a, b)| a / b < 0.92) {
        fine.push(format!("{a:.1} of {b:.0} GB of memory"));
    }
    if !fine.is_empty() {
        v.push(format!("{}.", fine.join(" · ")));
    }
    if c.plugins.is_some() && !c.lines.iter().any(|l| l.to_lowercase().contains("missing master")) {
        v.push("No plugin was missing a master.".into());
    }
    v
}

/// Proton's own log for the game (PROTON_LOG's steam-<app>.log in your home folder) or Hearth Mods' launch log.
pub fn proton_log(steam_app: u64) -> (String, String) {
    let home = hearth_oss_kit_a::home();
    for p in [home.join(format!("steam-{steam_app}.log")), hearth_gaming::launch::log_file()] {
        if let Ok(t) = std::fs::read_to_string(&p) {
            let lines: Vec<&str> = t.lines().collect();
            let tail = lines[lines.len().saturating_sub(120)..].join("\n");
            return (p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), tail);
        }
    }
    (String::new(), String::new())
}

/// What ran in that start, in lines.
pub fn what_ran(play: &Option<Value>, c: &crate::logic::Crash) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(p) = play {
        v.push(format!("Started {} · {}", hearth_ui::time::when(p["at"].as_i64().unwrap_or(0)), p["device"].as_str().unwrap_or("")));
        v.push(format!("Program: {}", p["exe"].as_str().filter(|x| !x.is_empty()).unwrap_or("the script extender")));
        v.push(format!("Profile: {} · {} mods on", p["profile"].as_str().unwrap_or(""), p["modsOn"]));
    }
    let mods: Vec<&String> = c.lines.iter().skip_while(|l| !l.trim().to_uppercase().starts_with("MODULES")).skip(1).take_while(|l| !l.trim().is_empty() && !l.trim().ends_with(':')).collect();
    if !mods.is_empty() {
        v.push(format!("Script extender plugins loaded: {}", mods.len()));
        for m in mods.into_iter().take(30) {
            v.push(format!("  {}", m.trim()));
        }
    }
    if let Some((n, _)) = c.plugins {
        v.push(format!("Plugins: {n}"));
    }
    v
}

/// Hearth's list of known crashes (kept with the app): a module or object in the log → what's known about it.
pub const KNOWN: &[(&str, &str)] = &[
    ("skyui_se.dll", "Crashes inside SkyUI's own code are a known kind: they're usually fixed by SkyUI's newer versions or its patches, so an update is the first thing to try."),
    ("enginefixes.dll", "SSE Engine Fixes in the call stack usually means a crash it caught, not one it caused: look at the line above it."),
    ("tbbmalloc.dll", "tbbmalloc (Engine Fixes' memory part) in the stack: crashes there are known when its two parts' versions don't match."),
    ("xaudio2_7.dll", "Audio crashes in XAudio2 under Proton are known: the game's Proton prefix needs its audio parts (Proton's own fix, or a newer Proton)."),
    ("bstrishape", "A mesh (BSTriShape) is named: crashes like this are known to come from a broken or wrongly made mesh in a mod."),
    ("exception_stack_overflow", "A stack overflow is known to come from a script or a record that refers to itself without end."),
];

/// Others had it too: the known list, and your other devices' crashes with the same signature.
pub fn others_had(c: &crate::logic::Crash, devices: &[Value]) -> Vec<String> {
    let mut v = Vec::new();
    let text = format!("{} {} {}", c.suspects.join(" "), c.objects.join(" "), c.exception).to_lowercase();
    for (k, w) in KNOWN {
        if text.contains(k) {
            v.push(format!("{w} (Hearth's known crashes list)"));
        }
    }
    let sig = c.suspects.first().cloned().unwrap_or_default().to_lowercase();
    for d in devices {
        for x in d["crashes"].as_array().cloned().unwrap_or_default() {
            if !sig.is_empty() && x["module"].as_str().map(|m| m.to_lowercase() == sig).unwrap_or(false) {
                v.push(format!("{} had the same crash {} ({}){}.", d["name"].as_str().unwrap_or("Another device"), hearth_ui::time::when(x["at"].as_i64().unwrap_or(0)), x["game"].as_str().unwrap_or(""), if x["fixed"] == true { " and it's fixed there" } else { "" }));
            }
        }
    }
    v
}

/// The fixes tried and the test starts that passed (crash file → state).
pub fn fixes() -> Value {
    crate::back::read_own("fixes.json")
}

pub fn mark_fix(crash: &Path, state: &str, how: &str) {
    let mut f = fixes();
    f[crash.to_string_lossy().as_ref()] = json!({"state": state, "how": how, "at": crate::now()});
    let _ = crate::back::write_own("fixes.json", &f);
}

/// The test start after a fix: the game is started with the mods, and after 60 seconds it's still running with no
/// newer crash log → it's fixed. (game running?, a newer crash log?)
pub fn test_start_result(inst_id: &str, since: i64, roots: &[PathBuf]) -> Result<String, String> {
    std::thread::sleep(std::time::Duration::from_secs(62));
    let newer = crate::logic::crash_logs(roots).into_iter().any(|(_, t)| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64 > since).unwrap_or(false));
    if newer {
        return Err("It crashed again in the test start: the fix wasn't it. Try the next one.".into());
    }
    let st = hearth_gaming::mods_app::route("organizer", &json!({"args": [inst_id, "status"]})).unwrap_or(Value::Null);
    let running = st["status"]["running"].as_bool().or(st["deployed"]["running"].as_bool()).unwrap_or(false);
    if running {
        Ok("Fixed: the test start ran 60 seconds without a crash.".into())
    } else {
        Err("The game wasn't running after 60 seconds (it closed, or didn't start): start it once more to be sure.".into())
    }
}

// ------------------------------------------------------------------ Speak (whisper.cpp, as Hearth Listen has it)

static REC: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

fn wav() -> PathBuf {
    crate::back::own_dir().join("speak.wav")
}

pub fn listening() -> bool {
    REC.lock().map(|r| r.is_some()).unwrap_or(false)
}

/// Starts listening (the microphone, through PipeWire's pw-record or ALSA's arecord), or stops and says the words.
pub fn speak_start() -> Result<(), String> {
    let _ = std::fs::create_dir_all(crate::back::own_dir());
    let f = wav();
    let child = std::process::Command::new("pw-record").args(["--rate", "16000", "--channels", "1", "--format", "s16"]).arg(&f).spawn().or_else(|_| std::process::Command::new("arecord").args(["-q", "-f", "S16_LE", "-r", "16000", "-c", "1"]).arg(&f).spawn()).map_err(|_| "Speak needs a microphone (PipeWire's pw-record wasn't found).".to_string())?;
    *REC.lock().map_err(|e| e.to_string())? = Some(child);
    Ok(())
}

pub fn speak_stop() -> Result<String, String> {
    let child = REC.lock().map_err(|e| e.to_string())?.take();
    if let Some(mut c) = child {
        unsafe {
            libc::kill(c.id() as i32, libc::SIGINT);
        }
        let _ = c.wait();
    }
    let listen = hearth_oss_kit_a::home().join(".local/share/hearth/listen");
    let cli = hearth_oss::which(&["whisper-cli", "whisper-cpp"], &[listen.join("bin/whisper-cli")]).ok_or("Speak turns your words into text with whisper.cpp: get it in Hearth Listen (Settings › Get whisper.cpp) first.")?;
    let mut models: Vec<PathBuf> = std::fs::read_dir(listen.join("models")).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "bin").unwrap_or(false)).collect();
    models.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0));
    let model = models.first().cloned().ok_or("Speak needs one of whisper.cpp's models: get one in Hearth Listen first.")?;
    let o = std::process::Command::new(cli).arg("-m").arg(&model).arg("-f").arg(wav()).args(["-nt", "-np"]).output().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(wav());
    let words = String::from_utf8_lossy(&o.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    if words.is_empty() {
        return Err("Nothing was heard: speak a little closer, then Send.".into());
    }
    Ok(words)
}

// ------------------------------------------------------------------ the nightly check and how long snapshots stay

fn unit_dir() -> PathBuf {
    hearth_gaming::util::xdg_config().join("systemd/user")
}

/// "Every night": a user timer that runs `hearth-mods --check-updates` at night (it only reads the mods' pages and
/// writes the Mod Manager's own notes; nothing is installed). Any other choice takes the timer away again.
pub fn nightly(on: bool) -> Result<(), String> {
    let (svc, tmr) = (unit_dir().join("hearth-mods-updates.service"), unit_dir().join("hearth-mods-updates.timer"));
    let ctl = |a: &[&str]| {
        let _ = std::process::Command::new("systemctl").arg("--user").args(a).output();
    };
    if !on {
        if tmr.exists() {
            ctl(&["disable", "--now", "hearth-mods-updates.timer"]);
            let _ = std::fs::remove_file(&tmr);
            let _ = std::fs::remove_file(&svc);
            ctl(&["daemon-reload"]);
        }
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let s = format!("[Unit]\nDescription=Mod Manager: look for mod updates (nothing is installed)\n\n[Service]\nType=oneshot\nNice=15\nIOSchedulingClass=idle\nExecStart={} --check-updates\n", exe.display());
    let t = "[Unit]\nDescription=Mod Manager: look for mod updates each night\n\n[Timer]\nOnCalendar=*-*-* 03:40\nRandomizedDelaySec=40min\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n";
    let same = std::fs::read_to_string(&svc).ok().as_deref() == Some(s.as_str()) && tmr.exists();
    if same {
        return Ok(());
    }
    std::fs::create_dir_all(unit_dir()).map_err(|e| e.to_string())?;
    hearth_ui::settings_file::write_text(&svc, &s).map_err(|e| e.to_string())?;
    hearth_ui::settings_file::write_text(&tmr, t).map_err(|e| e.to_string())?;
    ctl(&["daemon-reload"]);
    ctl(&["enable", "--now", "hearth-mods-updates.timer"]);
    Ok(())
}

/// `hearth-mods --check-updates`: every game's Nexus mods checked against their pages, kept in the Mod Manager's notes;
/// tells you when something's new (when "Tell me on the Ally and the TV" is on).
pub fn check_all() -> i32 {
    // without a Nexus Mods sign-in nothing can be checked, and what was found before stays
    if hearth_gaming::nexus::account().is_none() {
        return 0;
    }
    let Ok((games, _)) = crate::back::load_world() else { return 1 };
    let mut all = crate::back::read_own("updates.json");
    let mut list: Vec<Value> = all["updates"].as_array().cloned().unwrap_or_default();
    let before = list.len();
    for g in games.iter().filter(|g| g.inst.is_some()) {
        let d = crate::back::load_detail(g);
        let mods = d.s["mods"].as_array().cloned().unwrap_or_default();
        let app = g.lib["steamApp"].as_u64();
        let Some(domain) = hearth_gaming::nexus::STEAM_DOMAINS.iter().find(|x| Some(x.0) == app).map(|x| x.1.to_string()) else { continue };
        if let Ok(v) = crate::sources::check_updates(&g.key, &g.title, &domain, &mods) {
            list.retain(|u| u["game"].as_str() != Some(g.title.as_str()));
            list.extend(v["updates"].as_array().cloned().unwrap_or_default());
        }
    }
    all["updates"] = json!(list);
    all["checked"] = json!(crate::now());
    let _ = crate::back::write_own("updates.json", &all);
    let tell = hearth_oss_kit_a::read_settings("mods")["tellDevices"].as_bool().unwrap_or(true);
    if tell && list.len() > before {
        let _ = std::process::Command::new("notify-send").args(["-a", "Mod Manager", "Mod updates", &format!("{} new on your mods' pages. Nothing is installed until you pick them.", list.len() - before)]).status();
    }
    0
}

/// Snapshots older than `days`, or past `gb` in all (oldest first), are taken away; the newest is always kept.
pub fn prune(instance: &Path, days: i64, gb: u64) -> usize {
    let list = crate::logic::list(instance);
    let now = crate::now();
    let mut total: u64 = 0;
    let mut gone = 0;
    for (i, s) in list.iter().enumerate() {
        let size = hearth_oss_kit_a::folder_size(&s.dir);
        total += size;
        if i == 0 {
            continue;
        }
        if (days > 0 && now - s.at > days * 86400) || (gb > 0 && total > gb * 1_073_741_824) {
            if std::fs::remove_dir_all(&s.dir).is_ok() {
                gone += 1;
            }
        }
    }
    gone
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_and_masters() {
        let mut b = vec![0u8; 64];
        b.extend([0xBD, 0x04, 0xEF, 0xFE, 0, 0, 1, 0]);
        b.extend(((0u32 << 16) | 2).to_le_bytes());
        b.extend(((2u32 << 16) | 6).to_le_bytes());
        assert_eq!(super::extender_words(super::pe_version(&b).unwrap()), "2.2.6");
        let mut h = Vec::new();
        for m in ["Skyrim.esm", "Lux.esp"] {
            h.extend(b"MAST");
            h.extend(((m.len() + 1) as u16).to_le_bytes());
            h.extend(m.as_bytes());
            h.push(0);
        }
        let mut p = b"TES4".to_vec();
        p.extend((h.len() as u32).to_le_bytes());
        p.extend([0u8; 16]);
        p.extend(h);
        assert_eq!(super::masters_of(&p), vec!["Skyrim.esm", "Lux.esp"]);
    }
}
