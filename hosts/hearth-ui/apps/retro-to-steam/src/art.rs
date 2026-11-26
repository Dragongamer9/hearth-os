//! The art for each game, from community art sites, only images made for that game (as Steam ROM Manager finds it):
//!   libretro's thumbnails (thumbnails.libretro.com, no account or key): box art (Grid), title screens (Wide) and
//!     screenshots (Hero), matched to the game by its name, every region and revision of it a choice;
//!   SteamGridDB, only with your own API key typed into Art sources (you get it from steamgriddb.com; it stays on this
//!     PC): grids, wide grids, heroes, logos and icons.
//! Each site's list for a console is read once and kept a week (~/.cache/hearth/retro-to-steam); each picture is
//! downloaded when it's first shown or added, and kept there too. curl does the fetching, off the window's thread.

use hearth_retro::library::{clean_title, Game};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Grid,
    Wide,
    Hero,
    Logo,
    Icon,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Grid, Kind::Wide, Kind::Hero, Kind::Logo, Kind::Icon];
    pub fn label(self) -> &'static str {
        match self {
            Kind::Grid => "Grid art",
            Kind::Wide => "Wide art",
            Kind::Hero => "Hero",
            Kind::Logo => "Logo",
            Kind::Icon => "Icon",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Kind::Grid => "grid",
            Kind::Wide => "wide",
            Kind::Hero => "hero",
            Kind::Logo => "logo",
            Kind::Icon => "icon",
        }
    }
    /// libretro's folder for this kind (none for logos and icons)
    fn libretro(self) -> Option<&'static str> {
        match self {
            Kind::Grid => Some("Named_Boxarts"),
            Kind::Wide => Some("Named_Titles"),
            Kind::Hero => Some("Named_Snaps"),
            _ => None,
        }
    }
    /// The file Steam reads it from in config/grid (the shortcut's id first).
    pub fn steam_file(self, id: u32) -> String {
        match self {
            Kind::Grid => format!("{id}p.png"),
            Kind::Wide => format!("{id}.png"),
            Kind::Hero => format!("{id}_hero.png"),
            Kind::Logo => format!("{id}_logo.png"),
            Kind::Icon => format!("{id}_icon.png"),
        }
    }
}

/// libretro's name for each console's thumbnails.
pub fn libretro_system(sys: &str) -> Option<&'static str> {
    Some(match sys {
        "snes" => "Nintendo - Super Nintendo Entertainment System",
        "psx" => "Sony - PlayStation",
        "n64" => "Nintendo - Nintendo 64",
        "gba" => "Nintendo - Game Boy Advance",
        "gc" => "Nintendo - GameCube",
        "ps2" => "Sony - PlayStation 2",
        "dreamcast" => "Sega - Dreamcast",
        "arcade" => "MAME",
        "nes" => "Nintendo - Nintendo Entertainment System",
        "genesis" => "Sega - Mega Drive - Genesis",
        "psp" => "Sony - PlayStation Portable",
        "nds" => "Nintendo - Nintendo DS",
        "saturn" => "Sega - Saturn",
        "wii" => "Nintendo - Wii",
        "gb" => "Nintendo - Game Boy",
        "gbc" => "Nintendo - Game Boy Color",
        "scummvm" => "ScummVM",
        _ => return None,
    })
}

/// One picture to choose: where it comes from and where it's kept once downloaded.
#[derive(Debug, Clone, PartialEq)]
pub struct Cand {
    pub url: String,
    pub site: &'static str,
}

/// Every console's lists, by the game's name (lowercase, cleaned) → the names of its pictures.
#[derive(Debug, Default)]
pub struct Index {
    pub lists: HashMap<(String, Kind), HashMap<String, Vec<String>>>,
    /// how many of each kind the fixture (CI) says a game has, by its title
    pub fixed: Option<serde_json::Value>,
}

pub fn cache() -> PathBuf {
    hearth_ui::paths::home().join(".cache/hearth/retro-to-steam")
}

pub fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~()!',".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn norm(name: &str) -> String {
    clean_title(name).to_lowercase().replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A console's list for a kind, from the week-old copy or read again (one request; its page lists every picture).
fn list(sys: &str, kind: Kind) -> Vec<String> {
    let (Some(system), Some(folder)) = (libretro_system(sys), kind.libretro()) else { return vec![] };
    let file = cache().join("lists").join(format!("{sys}-{}.txt", kind.id()));
    let fresh = std::fs::metadata(&file).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|e| e.as_secs() < 7 * 86400).unwrap_or(false);
    if !fresh {
        let url = format!("https://thumbnails.libretro.com/{}/{folder}/", enc(system));
        if let Ok(o) = std::process::Command::new("curl").args(["-fsSL", "--max-time", "40", &url]).output() {
            if o.status.success() {
                let html = String::from_utf8_lossy(&o.stdout);
                let mut names = Vec::new();
                for part in html.split("href=\"").skip(1) {
                    let href = part.split('"').next().unwrap_or("");
                    if let Some(n) = href.strip_suffix(".png") {
                        names.push(dec(n));
                    }
                }
                if !names.is_empty() {
                    if let Some(d) = file.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    let _ = std::fs::write(&file, names.join("\n"));
                }
            }
        }
    }
    std::fs::read_to_string(&file).map(|t| t.lines().map(String::from).collect()).unwrap_or_default()
}

/// Reads (or makes) every list for these consoles. Off the window's thread: the first time it's a few requests.
pub fn index(systems: Vec<String>) -> Index {
    let fx = crate::fixture();
    if let Some(a) = fx.get("art") {
        return Index { lists: HashMap::new(), fixed: Some(a.clone()) };
    }
    let mut ix = Index::default();
    for sys in systems {
        for k in [Kind::Grid, Kind::Wide, Kind::Hero] {
            let mut m: HashMap<String, Vec<String>> = HashMap::new();
            for n in list(&sys, k) {
                m.entry(norm(&n)).or_default().push(n);
            }
            ix.lists.insert((sys.clone(), k), m);
        }
    }
    ix
}

/// SteamGridDB's pictures for one game, found by its name with your own key: (kind, urls).
pub fn sgdb(title: &str, key: &str) -> Vec<(Kind, Vec<String>)> {
    if key.is_empty() {
        return vec![];
    }
    // the key goes to curl in a header file only you can read (never on its command line)
    let hdr = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-r2s-sgdb.h");
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let Ok(mut f) = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&hdr) else { return vec![] };
        if writeln!(f, "Authorization: Bearer {key}").is_err() {
            return vec![];
        }
    }
    let get = |path: &str| -> serde_json::Value {
        let url = format!("https://www.steamgriddb.com/api/v2/{path}");
        std::process::Command::new("curl")
            .args(["-fsS", "--max-time", "20", "-H", &format!("@{}", hdr.display()), &url])
            .output()
            .ok()
            .and_then(|o| serde_json::from_slice(&o.stdout).ok())
            .unwrap_or(serde_json::Value::Null)
    };
    let found = get(&format!("search/autocomplete/{}", enc(title)));
    let Some(id) = found.get("data").and_then(|d| d.get(0)).and_then(|g| g.get("id")).and_then(|i| i.as_u64()) else {
        let _ = std::fs::remove_file(&hdr);
        return vec![];
    };
    let urls = |v: serde_json::Value| -> Vec<String> { v.get("data").and_then(|d| d.as_array()).map(|a| a.iter().filter_map(|x| x.get("url").and_then(|u| u.as_str()).map(String::from)).collect()).unwrap_or_default() };
    let out = vec![
        (Kind::Grid, urls(get(&format!("grids/game/{id}?dimensions=600x900")))),
        (Kind::Wide, urls(get(&format!("grids/game/{id}?dimensions=920x430,460x215")))),
        (Kind::Hero, urls(get(&format!("heroes/game/{id}")))),
        (Kind::Logo, urls(get(&format!("logos/game/{id}")))),
        (Kind::Icon, urls(get(&format!("icons/game/{id}")))),
    ];
    let _ = std::fs::remove_file(&hdr);
    out
}

impl Index {
    /// A game's choices for a kind: libretro's (its own region first), then SteamGridDB's when it was asked.
    pub fn cands(&self, g: &Game, kind: Kind, extra: Option<&Vec<(Kind, Vec<String>)>>) -> Vec<Cand> {
        let mut v = Vec::new();
        if let Some(fx) = &self.fixed {
            let n = fx.get(&g.title).or_else(|| fx.get("*")).and_then(|x| x.get(kind.id())).and_then(|x| x.as_u64()).unwrap_or(0);
            for i in 0..n {
                v.push(Cand { url: format!("fixture:{}:{}:{i}", g.title, kind.id()), site: "fixture" });
            }
            return v;
        }
        if let (Some(system), Some(folder), Some(m)) = (libretro_system(g.system), kind.libretro(), self.lists.get(&(g.system.to_string(), kind))) {
            if let Some(names) = m.get(&norm(&g.stem)) {
                let mut names = names.clone();
                names.sort_by_key(|n| (n != &g.stem, !n.contains("(USA")));
                for n in names {
                    v.push(Cand { url: format!("https://thumbnails.libretro.com/{}/{folder}/{}.png", enc(system), enc(&n)), site: "libretro" });
                }
            }
        }
        if let Some(x) = extra {
            for (k, urls) in x {
                if *k == kind {
                    v.extend(urls.iter().map(|u| Cand { url: u.clone(), site: "SteamGridDB" }));
                }
            }
        }
        v
    }
}

/// Where a choice is kept once downloaded.
pub fn file_of(c: &Cand) -> PathBuf {
    let h = crate::vdf::crc32(c.url.as_bytes());
    let ext = if c.url.ends_with(".jpg") || c.url.ends_with(".jpeg") { "jpg" } else if c.url.ends_with(".webp") { "webp" } else { "png" };
    cache().join("art").join(format!("{h:08x}.{ext}"))
}

/// Downloads a choice (nothing for one that's here already, or the fixture's). Blocking.
pub fn fetch(c: &Cand) -> Result<PathBuf, String> {
    let p = file_of(c);
    if p.is_file() || c.site == "fixture" {
        return Ok(p);
    }
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let part = p.with_extension("part");
    let o = std::process::Command::new("curl").args(["-fsSL", "--max-time", "60", "-o"]).arg(&part).arg(&c.url).output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(String::from_utf8_lossy(&o.stderr).trim().to_string());
    }
    std::fs::rename(&part, &p).map_err(|e| e.to_string())?;
    Ok(p)
}
