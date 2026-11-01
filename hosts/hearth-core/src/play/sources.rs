//! Where Hearth's download manager gets games from, besides an address you paste (docs/HYDRA-FEATURES.md):
//!  - Free and open-source games: a short list Hearth knows, each downloaded from its own project's latest GitHub
//!    release (the Linux build), never from anywhere else.
//!  - itch.io: your itch.io library (games you bought or claimed) and free games, with your own itch.io API key
//!    (Settings › Games › Integrations). itch.io gives each download a short-lived signed address; that's what goes in
//!    the queue, so the key itself never does.
//! There are no community "download sources", repack feeds or file hosters here, and there won't be.

use serde_json::{json, Value};

/// A free and open-source game: its GitHub project and which release file is the Linux build.
pub struct Foss { pub id: &'static str, pub title: &'static str, pub about: &'static str, pub repo: &'static str, pub asset: &'static str, pub license: &'static str, pub site: &'static str }

pub const FOSS: &[Foss] = &[
    Foss { id: "supertuxkart", title: "SuperTuxKart", about: "Kart racing with Tux and friends: story mode, online races and battles.", repo: "supertuxkart/stk-code", asset: r"(?i)linux.*(x86_64|x64|amd64).*\.tar\.(xz|gz)$", license: "GPL-3.0", site: "https://supertuxkart.net" },
    Foss { id: "openra-ra", title: "OpenRA: Red Alert", about: "The classic Command & Conquer: Red Alert, rebuilt for today (uses the free original game files).", repo: "OpenRA/OpenRA", asset: r"(?i)Red-Alert.*x86_64\.AppImage$", license: "GPL-3.0", site: "https://www.openra.net" },
    Foss { id: "openra-td", title: "OpenRA: Tiberian Dawn", about: "Command & Conquer: Tiberian Dawn, rebuilt for today (uses the free original game files).", repo: "OpenRA/OpenRA", asset: r"(?i)Tiberian-Dawn.*x86_64\.AppImage$", license: "GPL-3.0", site: "https://www.openra.net" },
    Foss { id: "endless-sky", title: "Endless Sky", about: "Explore the galaxy as a trader, a mercenary or a pirate in this space sim.", repo: "endless-sky/endless-sky", asset: r"(?i)x86_64\.AppImage$", license: "GPL-3.0", site: "https://endless-sky.github.io" },
    Foss { id: "shattered-pd", title: "Shattered Pixel Dungeon", about: "A roguelike dungeon crawler with pixel art and lots of replay.", repo: "00-Evan/shattered-pixel-dungeon", asset: r"(?i)linux.*\.zip$", license: "GPL-3.0", site: "https://shatteredpixel.com" },
    Foss { id: "unciv", title: "Unciv", about: "A turn-based empire builder in the style of Civilization V.", repo: "yairm210/Unciv", asset: r"(?i)linux.*64.*\.zip$", license: "MPL-2.0", site: "https://yairm210.github.io/Unciv" },
    Foss { id: "cdda", title: "Cataclysm: Dark Days Ahead", about: "A deep survival roguelike set after the end of the world.", repo: "CleverRaven/Cataclysm-DDA", asset: r"(?i)linux.*(tiles|graphics).*x64.*\.tar\.gz$", license: "CC-BY-SA-3.0", site: "https://cataclysmdda.org" },
];

pub fn foss_list() -> Value {
    json!(FOSS.iter().map(|f| json!({"id": f.id, "title": f.title, "about": f.about, "license": f.license, "site": f.site, "repo": f.repo})).collect::<Vec<_>>())
}

/// The Linux build in a GitHub release (releases/latest's answer): (name, address, size).
pub fn pick_asset(release: &Value, pattern: &str) -> Option<(String, String, u64)> {
    let re = regex::Regex::new(pattern).ok()?;
    release["assets"].as_array()?.iter().find_map(|a| {
        let name = a["name"].as_str()?;
        let url = a["browser_download_url"].as_str()?;
        if !re.is_match(name) || !url.starts_with("https://github.com/") { return None; }
        Some((name.to_string(), url.to_string(), a["size"].as_u64().unwrap_or(0)))
    })
}

/// Finds the latest Linux build of one of the games above: {name, url, size, version}.
pub fn foss_resolve(id: &str) -> Result<Value, String> {
    let f = FOSS.iter().find(|f| f.id == id).ok_or("Unknown game")?;
    let rel = crate::storefront::cached(&format!("play-gh-{}", f.repo.replace('/', "-")), 6 * 3600 * 1000, || {
        crate::http::get_json(&format!("https://api.github.com/repos/{}/releases/latest", f.repo), &[("Accept", "application/vnd.github+json")])
    })?;
    let (name, url, size) = pick_asset(&rel, f.asset).ok_or(format!("{}'s latest release has no Linux download Hearth knows. Its site has others: {}", f.title, f.site))?;
    Ok(json!({"name": name, "url": url, "size": size, "version": rel["tag_name"], "title": f.title}))
}

// ------------------------------------------------------------------ itch.io

const ITCH: &str = "https://api.itch.io";

fn itch_get(path: &str) -> Result<Value, String> {
    let k = super::keys::get("itch").ok_or("Connect itch.io in Settings › Games › Integrations (with your own itch.io API key).")?;
    let auth = format!("Bearer {}", k["key"].as_str().unwrap_or(""));
    let r = crate::http::request("GET", &format!("{ITCH}{path}"), &[("Authorization", &auth), ("Accept", "application/json")], None, 20)?;
    if r.status == 401 || r.status == 403 { return Err("itch.io didn't accept your API key. Check it in Settings › Games › Integrations.".into()); }
    if !r.ok() { return Err(format!("itch.io didn't answer (HTTP {})", r.status)); }
    let v = r.json();
    if let Some(e) = v["errors"].as_array().and_then(|e| e.first()).and_then(|e| e.as_str()) { return Err(format!("itch.io: {e}")); }
    Ok(v)
}

/// A game in itch.io's answers, as the page shows it.
pub fn itch_game(g: &Value, key_id: Option<u64>) -> Option<Value> {
    let id = g["id"].as_u64()?;
    Some(json!({"id": id, "title": g["title"].as_str()?, "cover": g["cover_url"], "about": g["short_text"], "url": g["url"],
                "free": g["min_price"].as_u64().unwrap_or(0) == 0, "keyId": key_id, "by": g["user"]["display_name"].as_str().or(g["user"]["username"].as_str())}))
}
/// profile/owned-keys → your games.
pub fn parse_owned(v: &Value) -> Vec<Value> {
    v["owned_keys"].as_array().into_iter().flatten().filter_map(|k| itch_game(&k["game"], k["id"].as_u64())).collect()
}
/// search/games → the free ones (games only; paid games are bought on itch.io itself).
pub fn parse_search(v: &Value) -> Vec<Value> {
    v["games"].as_array().into_iter().flatten().filter(|g| g["min_price"].as_u64().unwrap_or(0) == 0 && g["classification"].as_str().map(|c| c == "game").unwrap_or(true))
        .filter_map(|g| itch_game(g, None)).collect()
}
/// A game's files, Linux and Windows builds first; files that aren't downloads (external links) are left out.
pub fn parse_uploads(v: &Value) -> Vec<Value> {
    let mut l: Vec<Value> = v["uploads"].as_array().into_iter().flatten().filter(|u| u["storage"].as_str().unwrap_or("hosted") == "hosted").filter_map(|u| {
        Some(json!({"id": u["id"].as_u64()?, "name": u["filename"].as_str()?, "label": u["display_name"].as_str().or(u["filename"].as_str()), "size": u["size"],
                    "linux": u["p_linux"] == true, "windows": u["p_windows"] == true, "demo": u["demo"] == true}))
    }).collect();
    l.sort_by_key(|u| (u["linux"] != true, u["windows"] != true, u["demo"] == true));
    l
}

pub fn itch_library() -> Result<Value, String> {
    let mut all = vec![];
    for page in 1..=5 {
        let v = itch_get(&format!("/profile/owned-keys?page={page}"))?;
        let got = parse_owned(&v);
        let n = got.len();
        all.extend(got);
        if n < v["per_page"].as_u64().unwrap_or(50) as usize || n == 0 { break; }
    }
    Ok(json!(all))
}
pub fn itch_search(q: &str) -> Result<Value, String> {
    let q = q.trim();
    if q.is_empty() || q.chars().count() > 80 { return Ok(json!([])); }
    Ok(json!(parse_search(&itch_get(&format!("/search/games?query={}", crate::http::urlencode(q)))?)))
}
pub fn itch_uploads(game: u64, key_id: Option<u64>) -> Result<Value, String> {
    let k = key_id.map(|k| format!("?download_key_id={k}")).unwrap_or_default();
    Ok(json!(parse_uploads(&itch_get(&format!("/games/{game}/uploads{k}"))?)))
}

/// The short-lived address itch.io gives for one file (its redirect), so the queue never holds the API key.
pub fn itch_download_url(upload: u64, key_id: Option<u64>) -> Result<String, String> {
    let k = super::keys::get("itch").ok_or("Connect itch.io in Settings › Games › Integrations.")?;
    let q = key_id.map(|k| format!("?download_key_id={k}")).unwrap_or_default();
    let cfg = crate::http::curl_config(&format!("{ITCH}/uploads/{upload}/download{q}"), &[("Authorization", &format!("Bearer {}", k["key"].as_str().unwrap_or("")))], None);
    let out = crate::system::run_input("curl", &["-sS", "-m", "20", "-o", "/dev/null", "-w", "%{http_code} %{redirect_url}", "-K", "-"], Some(&cfg), std::time::Duration::from_secs(25))?;
    let (code, url) = out.trim().split_once(' ').unwrap_or((out.trim(), ""));
    if !url.starts_with("https://") { return Err(format!("itch.io didn't give a download for that file (HTTP {code}).")); }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foss_release_files() {
        let rel: Value = serde_json::from_str(r#"{"tag_name":"1.4","assets":[
            {"name":"SuperTuxKart-1.4-src.tar.xz","browser_download_url":"https://github.com/supertuxkart/stk-code/releases/download/1.4/SuperTuxKart-1.4-src.tar.xz","size":1},
            {"name":"SuperTuxKart-1.4-linux-x86_64.tar.xz","browser_download_url":"https://github.com/supertuxkart/stk-code/releases/download/1.4/SuperTuxKart-1.4-linux-x86_64.tar.xz","size":700000000},
            {"name":"SuperTuxKart-1.4-win.exe","browser_download_url":"https://github.com/x/y","size":2}]}"#).unwrap();
        let stk = FOSS.iter().find(|f| f.id == "supertuxkart").unwrap();
        let (name, url, size) = pick_asset(&rel, stk.asset).unwrap();
        assert_eq!(name, "SuperTuxKart-1.4-linux-x86_64.tar.xz");
        assert!(url.starts_with("https://github.com/supertuxkart/"));
        assert_eq!(size, 700_000_000);
        let evil: Value = serde_json::from_str(r#"{"assets":[{"name":"SuperTuxKart-linux-x86_64.tar.xz","browser_download_url":"https://evil.example/x.tar.xz"}]}"#).unwrap();
        assert!(pick_asset(&evil, stk.asset).is_none(), "only GitHub's own download addresses");
        let ra: Value = serde_json::from_str(r#"{"assets":[{"name":"OpenRA-Dune-2000-x86_64.AppImage","browser_download_url":"https://github.com/a"},{"name":"OpenRA-Red-Alert-x86_64.AppImage","browser_download_url":"https://github.com/b"}]}"#).unwrap();
        assert_eq!(pick_asset(&ra, FOSS[1].asset).unwrap().1, "https://github.com/b");
        for f in FOSS { assert!(regex::Regex::new(f.asset).is_ok(), "{}", f.id); }
        assert_eq!(foss_list().as_array().unwrap().len(), FOSS.len());
    }

    #[test]
    fn itch_answers() {
        let owned: Value = serde_json::from_str(r#"{"page":1,"per_page":50,"owned_keys":[{"id":99,"game_id":5,"game":{"id":5,"title":"Celeste Classic","min_price":0,"cover_url":"https://img.itch.zone/c.png","short_text":"Climb","user":{"username":"maddy"}}}]}"#).unwrap();
        let l = parse_owned(&owned);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0]["keyId"], 99);
        assert_eq!(l[0]["by"], "maddy");
        let s: Value = serde_json::from_str(r#"{"games":[{"id":1,"title":"Free","min_price":0,"classification":"game"},{"id":2,"title":"Paid","min_price":500,"classification":"game"},{"id":3,"title":"Tool","min_price":0,"classification":"tool"}]}"#).unwrap();
        let f = parse_search(&s);
        assert_eq!(f.len(), 1, "free games only");
        assert_eq!(f[0]["title"], "Free");
        let u: Value = serde_json::from_str(r#"{"uploads":[{"id":1,"filename":"game-win.zip","p_windows":true,"storage":"hosted","size":10},
            {"id":2,"filename":"game-linux.tar.gz","p_linux":true,"storage":"hosted"},{"id":3,"filename":"link","storage":"external"},{"id":4,"filename":"demo-linux.zip","p_linux":true,"demo":true}]}"#).unwrap();
        let up = parse_uploads(&u);
        assert_eq!(up.iter().map(|x| x["id"].as_u64().unwrap()).collect::<Vec<_>>(), vec![2, 4, 1], "Linux first, demos after, external links left out");
    }
}
