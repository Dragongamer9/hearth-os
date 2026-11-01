//! Steam Game Mode takeover (Experimental, docs/STEAM-GAME-MODE.md): what the Hearth Decky plugin's pages show inside
//! Steam's gamepad screens (Hearth's home in place of Steam's, Hearth's Store, Library tabs per store, Downloads,
//! Storage, a game's Hearth facts, the power mode in Quick Access).
//!
//! `hearth --steam-mode-data <what> [arg]` prints one line of JSON and exits. No window, no socket, no port: the
//! plugin's backend (hosts/decky-hearth/main.py, running as you) starts it and reads the answer, so nothing new
//! listens on the device. It only reads, except for the two things Hearth's own pages also do: `power-mode` (set
//! through hearth-power, as Quick Access does) and the library's usual bookkeeping when Downloads is asked.
//!
//!   home                    Hearth's home: hero, Continue playing, Favorites, a row per store, Recently added
//!   library                 every game in Hearth's library, in a tab per store
//!   store <steam|gog|epic|xbox|free|recomps|battlenet>   a store's front page (the same answers Hearth's Store uses)
//!   downloads               Hearth's download queue (with Hearth's own Steam client's, when it runs one)
//!   storage                 the drives and how full they are (Settings › Storage, read only)
//!   info <id>               a game's ProtonDB rating and your Steam achievements (what Hearth's game page shows)
//!   power                   the power mode and what's available
//!   power-mode <efficiency|balanced|performance>

use crate::library::{Game, Library};
use serde_json::{json, Value};

/// The library's tabs (and the home's rows), in Hearth's order: (tab, title, Hearth's sources in it).
pub const TABS: [(&str, &str, &[&str]); 9] = [
    ("steam", "Steam", &["steam"]),
    ("epic", "Epic", &["epic"]),
    ("gog", "GOG", &["gog"]),
    ("battlenet", "Battle.net", &["battlenet"]),
    ("xbox", "Xbox", &["xbox", "xodus"]),
    ("beacon", "Beacon", &["beacon"]),
    ("recomps", "Recomps", &["recomps"]),
    ("emu", "Emulators", &["emu"]),
    ("manual", "Added", &["manual"]),
];

/// The store pages the plugin can show (Hearth's Store's tabs, plus free games).
pub const STORE_PAGES: [&str; 7] = ["steam", "gog", "epic", "xbox", "free", "recomps", "battlenet"];

/// The tab a source belongs to ("xodus" is in Xbox; one Hearth doesn't list goes in Added).
pub fn tab_of(source: &str) -> &'static str {
    TABS.iter().find(|(_, _, s)| s.contains(&source)).map(|(t, _, _)| *t).unwrap_or("manual")
}

/// A store's name as the plugin shows it.
pub fn store_label(source: &str) -> &'static str {
    match source {
        "steam" => "Steam", "epic" => "Epic", "gog" => "GOG", "battlenet" => "Battle.net", "xbox" => "Xbox", "xodus" => "Xbox PC",
        "beacon" => "Beacon", "recomps" => "Recomps", "emu" => "Emulators", "android" => "Android", _ => "Added",
    }
}

/// One game as the plugin shows it: pictures as files on this device or web addresses (never Hearth's own
/// nimbus-art:// addresses, which only Hearth's window can load), your pictures over the store's.
pub fn entry(lib: &Library, g: &Game) -> Value {
    let d = &lib.store.data;
    let stats = &d["stats"][&g.id];
    let own = d["customArt"][&g.id].as_object();
    let art = |kinds: &[&str]| -> Value {
        for k in kinds {
            if let Some(p) = own.and_then(|o| o.get(*k)).and_then(|v| v.as_str()) { return json!(crate::desktop_games::art_for_desktop(p)); }
            if let Some((_, v)) = g.art.iter().find(|(kind, _)| kind == k) { return json!(crate::desktop_games::art_for_desktop(v)); }
        }
        Value::Null
    };
    json!({
        "id": g.id, "title": g.title, "source": g.source, "store": store_label(g.source), "tab": tab_of(g.source),
        "installed": g.installed && g.state == "installed", "state": g.state, "cloud": g.cloud,
        "fav": lib.store.list_has("favorites", &g.id), "hidden": lib.store.list_has("hidden", &g.id),
        "minutes": g.store_minutes.max(stats["minutes"].as_u64().unwrap_or(0)),
        "lastPlayed": g.store_last_played.max(stats["lastPlayed"].as_u64().unwrap_or(0)),
        "added": g.added, "sizeBytes": g.size_bytes,
        "cover": art(&["cover", "portrait"]), "wide": art(&["wide", "header", "hero"]), "hero": art(&["hero", "wide"]),
        "logo": art(&["logo"]), "icon": art(&["icon"]),
    })
}

fn num(v: &Value, k: &str) -> u64 { v[k].as_u64().unwrap_or(0) }
fn title_key(v: &Value) -> String { v["title"].as_str().unwrap_or("").to_lowercase() }
/// Downloading, updating, queued...: shown on the home like an installed game (Hearth's home does too).
fn active(v: &Value) -> bool { !["installed", "notInstalled", "cloud", ""].contains(&v["state"].as_str().unwrap_or("")) }

/// Hearth's home (ui/index.html viewHome): the games it shows are installed ones, ones downloading, and cloud games
/// you've played or marked; not hidden ones. The hero is the last played (or newest), then Continue playing,
/// Favorites, a row per store (installed, A to Z) and Recently added.
pub fn home_layout(games: &[Value]) -> Value {
    let v: Vec<&Value> = games.iter().filter(|g| g["hidden"] != true)
        .filter(|g| g["installed"] == true || active(g) || (g["cloud"] == true && (num(g, "lastPlayed") > 0 || g["fav"] == true))).collect();
    let mut recent: Vec<&Value> = v.iter().copied().filter(|g| num(g, "lastPlayed") > 0).collect();
    recent.sort_by(|a, b| num(b, "lastPlayed").cmp(&num(a, "lastPlayed")).then(title_key(a).cmp(&title_key(b))));
    recent.truncate(10);
    let mut favs: Vec<&Value> = v.iter().copied().filter(|g| g["fav"] == true).collect();
    favs.sort_by_key(|g| title_key(g));
    let mut added: Vec<&Value> = v.iter().copied().filter(|g| num(g, "added") > 0).collect();
    added.sort_by(|a, b| num(b, "added").cmp(&num(a, "added")).then(title_key(a).cmp(&title_key(b))));
    added.truncate(12);
    let hero = recent.first().or(added.first()).or(v.first()).map(|g| (*g).clone()).unwrap_or(Value::Null);
    let rows: Vec<Value> = TABS.iter().filter_map(|(tab, title, _)| {
        let mut items: Vec<&Value> = v.iter().copied().filter(|g| g["tab"] == *tab && g["installed"] == true).collect();
        if items.is_empty() { return None; }
        items.sort_by_key(|g| title_key(g));
        let total = items.len();
        items.truncate(30);
        Some(json!({"id": tab, "title": title, "total": total, "items": items}))
    }).collect();
    json!({"hero": hero, "continue": recent, "favorites": favs, "rows": rows, "added": added, "count": v.len()})
}

/// The library in a tab per store (only tabs with games), installed games first, then A to Z. Hidden games are left
/// out, like Hearth's library does outside its Hidden tab.
pub fn library_tabs(games: &[Value]) -> Value {
    let tabs: Vec<Value> = TABS.iter().filter_map(|(tab, title, _)| {
        let mut items: Vec<&Value> = games.iter().filter(|g| g["hidden"] != true && g["tab"] == *tab).collect();
        if items.is_empty() { return None; }
        items.sort_by(|a, b| (b["installed"] == true).cmp(&(a["installed"] == true)).then(title_key(a).cmp(&title_key(b))));
        let installed = items.iter().filter(|g| g["installed"] == true).count();
        Some(json!({"id": tab, "title": title, "count": items.len(), "installed": installed, "items": items}))
    }).collect();
    json!({"tabs": tabs})
}

/// Hearth Recomps' catalogue as a store page: Featured, then every project, in the store pages' item shape.
pub fn recomps_page(cat: &Value) -> Value {
    let consoles = &cat["consoles"];
    let item = |e: &Value| {
        let console = e["console"].as_str().unwrap_or("");
        json!({"store": "recomps", "id": e["id"], "title": e["title"], "wide": Value::Null, "tall": Value::Null, "icon": e["icon"],
               "sub": consoles[console].as_str().unwrap_or(console), "needs": e["needs"],
               "price": {"final": "Free", "base": Value::Null, "discount": 0, "free": true}, "url": e["repo"].as_str().map(|r| format!("https://github.com/{r}"))})
    };
    let all: Vec<&Value> = cat["recomps"].as_array().map(|a| a.iter().filter(|e| e["id"].is_string() && e["title"].is_string()).collect()).unwrap_or_default();
    let featured: Vec<Value> = all.iter().filter(|e| e["featured"] == true).map(|e| item(*e)).collect();
    let mut sections = vec![];
    if !featured.is_empty() { sections.push(json!({"id": "featured", "title": "Featured", "items": featured})); }
    sections.push(json!({"id": "all", "title": "All recomps", "items": all.iter().map(|e| item(*e)).collect::<Vec<_>>()}));
    json!({"store": "recomps", "sections": sections})
}

/// Free games: the free sections of Epic's front page (Free now, Free next), as their own page.
pub fn free_page(epic_home: &Value) -> Value {
    let sections: Vec<Value> = epic_home["sections"].as_array().map(|a| a.iter()
        .filter(|s| s["id"].as_str().map(|i| i.starts_with("free")).unwrap_or(false)).cloned().collect()).unwrap_or_default();
    json!({"store": "free", "sections": sections, "stale": epic_home["stale"]})
}

/// A store's front page for the plugin.
pub fn store_page(store: &str) -> Result<Value, String> {
    match store {
        "steam" | "gog" | "epic" | "xbox" => crate::storefront::home(store),
        "free" => crate::storefront::home("epic").map(|h| free_page(&h)),
        "recomps" => Ok(recomps_page(&crate::recomps::catalogue())),
        // Battle.net's shop is Blizzard's own app (Hearth's Store opens it, signed in there): nothing to list here.
        "battlenet" => Ok(json!({"store": "battlenet", "inApp": true, "sections": []})),
        _ => Err("Unknown store".into()),
    }
}

/// The downloads, short: what's downloading, how far, how fast.
pub fn downloads_page(d: &Value) -> Value {
    let active: Vec<Value> = d["active"].as_array().map(|a| a.iter().map(|g| json!({
        "id": g["id"], "title": g["title"], "source": g["source"], "store": store_label(g["source"].as_str().unwrap_or("")),
        "state": g["state"], "stage": g["stage"], "progress": g["progress"], "bytesDone": g["bytesDone"], "bytesTotal": g["bytesTotal"],
        "bytesPerSec": g["bytesPerSec"], "engine": g["engine"],
    })).collect()).unwrap_or_default();
    json!({"active": active, "recent": d["recent"]})
}

/// The drives, read only (moving and cleaning up stay in Hearth's Settings › Storage).
pub fn storage_page(drives: &[crate::diskspace::Drive]) -> Value {
    let list: Vec<Value> = drives.iter().map(|d| json!({"name": d.name, "kind": d.kind, "mount": d.mount.to_string_lossy(), "size": d.size,
        "used": d.used, "free": d.free, "readOnly": d.read_only, "steamLibs": d.steam_libs.len()})).collect();
    json!({"drives": list})
}

fn library_entries(lib: &Library) -> Vec<Value> { lib.games.values().map(|g| entry(lib, g)).collect() }

/// One answer. Err: what went wrong, in plain words.
pub fn answer(what: &str, arg: Option<&str>) -> Result<Value, String> {
    match what {
        "home" => Ok(home_layout(&library_entries(&crate::desktop_games::read_library()))),
        "library" => Ok(library_tabs(&library_entries(&crate::desktop_games::read_library()))),
        "store" => store_page(arg.unwrap_or("steam")),
        "downloads" => Ok(downloads_page(&crate::desktop_games::read_library().downloads())),
        "storage" => Ok(storage_page(&crate::diskspace::drives(&crate::diskspace::ctx_from(&crate::desktop_games::read_library())))),
        "info" => {
            let id = arg.filter(|i| crate::desktop_games::valid_id(i)).ok_or("That isn't a game id Hearth knows.")?;
            let lib = crate::desktop_games::read_library();
            let (title, source) = lib.games.get(id).map(|g| (g.title.clone(), g.source.to_string())).unwrap_or_default();
            let appid = id.strip_prefix("steam:").filter(|a| a.chars().all(|c| c.is_ascii_digit()));
            Ok(json!({"id": id, "title": title, "protondb": crate::gameinfo::protondb(id, &title, &source),
                      "achievements": appid.map(|a| crate::gameinfo::achievements(&lib.home, a)).unwrap_or(Value::Null)}))
        }
        "power" => Ok(crate::power::status()),
        "power-mode" => { crate::power::set_mode(arg.unwrap_or(""))?; Ok(crate::power::status()) }
        _ => Err(format!("Unknown request: {what}")),
    }
}

/// `--steam-mode-data <what> [arg]`: (what, arg) when the arguments ask for it.
pub fn parse_args(args: &[String]) -> Option<(String, Option<String>)> {
    let i = args.iter().position(|a| a == "--steam-mode-data")?;
    let what = args.get(i + 1)?.clone();
    Some((what, args.get(i + 2).cloned()))
}

/// The command line: prints the answer (or {"error": …}) as one line, returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let Some((what, arg)) = parse_args(args) else { println!("{}", json!({"error": "usage: --steam-mode-data <what> [arg]"})); return 2 };
    match answer(&what, arg.as_deref()) {
        Ok(v) => { println!("{v}"); 0 }
        Err(e) => { println!("{}", json!({"error": e})); 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(id: &str, source: &str, installed: bool, last: u64, added: u64, fav: bool) -> Value {
        json!({"id": id, "title": id.split(':').nth(1).unwrap_or(id), "source": source, "tab": tab_of(source), "installed": installed,
               "state": if installed { "installed" } else { "notInstalled" }, "cloud": false, "fav": fav, "hidden": false,
               "lastPlayed": last, "added": added})
    }

    #[test]
    fn sources_go_in_hearths_tabs() {
        assert_eq!(tab_of("xodus"), "xbox");
        assert_eq!(tab_of("xbox"), "xbox");
        assert_eq!(tab_of("battlenet"), "battlenet");
        assert_eq!(tab_of("something-new"), "manual");
        assert_eq!(store_label("manual"), "Added");
        assert_eq!(TABS.map(|t| t.1), ["Steam", "Epic", "GOG", "Battle.net", "Xbox", "Beacon", "Recomps", "Emulators", "Added"]);
    }

    #[test]
    fn home_is_laid_out_like_hearths() {
        let mut hidden = g("gog:Hidden", "gog", true, 99, 1, false);
        hidden["hidden"] = json!(true);
        let mut downloading = g("epic:Coming", "epic", false, 0, 50, false);
        downloading["state"] = json!("downloading");
        let games = vec![
            g("steam:Dota", "steam", true, 10, 5, false), g("epic:Alan", "epic", true, 30, 7, true), g("gog:Owned", "gog", false, 40, 9, true),
            g("beacon:BFME", "beacon", true, 0, 8, false), hidden, downloading,
        ];
        let h = home_layout(&games);
        assert_eq!(h["hero"]["id"], "epic:Alan", "the last played shown game");
        let cont: Vec<&str> = h["continue"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap()).collect();
        assert_eq!(cont, ["epic:Alan", "steam:Dota"], "not installed and hidden games aren't on the home");
        assert_eq!(h["favorites"].as_array().unwrap().len(), 1);
        let rows: Vec<&str> = h["rows"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(rows, ["steam", "epic", "beacon"], "a row per store, Hearth's order, installed games only");
        assert_eq!(h["added"][0]["id"], "epic:Coming", "a download shows like Hearth's home shows it");
        assert_eq!(h["count"], 4);
        let empty = home_layout(&[]);
        assert!(empty["hero"].is_null());
        assert_eq!(empty["rows"], json!([]));
    }

    #[test]
    fn library_has_a_tab_per_store() {
        let games = vec![g("gog:b", "gog", false, 0, 0, false), g("gog:a", "gog", false, 0, 0, false), g("gog:z", "gog", true, 0, 0, false),
                         g("xodus:Halo", "xodus", true, 0, 0, false), g("steam:x", "steam", true, 0, 0, false)];
        let l = library_tabs(&games);
        let tabs: Vec<&str> = l["tabs"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
        assert_eq!(tabs, ["steam", "gog", "xbox"]);
        let gog: Vec<&str> = l["tabs"][1]["items"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap()).collect();
        assert_eq!(gog, ["gog:z", "gog:a", "gog:b"], "installed first, then A to Z");
        assert_eq!(l["tabs"][1]["installed"], 1);
    }

    #[test]
    fn recomps_and_free_games_are_store_pages() {
        let cat = json!({"consoles": {"n64": "Nintendo 64"}, "recomps": [
            {"id": "zelda64", "title": "Zelda 64: Recompiled", "console": "n64", "repo": "Zelda64Recomp/Zelda64Recomp", "featured": true},
            {"id": "x", "title": "X", "console": "ps1", "repo": "a/b"}, {"title": "no id"}]});
        let p = recomps_page(&cat);
        assert_eq!(p["sections"][0]["id"], "featured");
        assert_eq!(p["sections"][0]["items"][0]["sub"], "Nintendo 64");
        assert_eq!(p["sections"][1]["items"].as_array().unwrap().len(), 2);
        assert_eq!(p["sections"][1]["items"][1]["url"], "https://github.com/a/b");
        assert_eq!(p["sections"][1]["items"][0]["price"]["free"], true);
        // The catalogue that ships with Hearth makes a page too.
        let built: Value = serde_json::from_str(crate::recomps::BUILTIN).unwrap();
        assert!(!recomps_page(&built)["sections"][0]["items"].as_array().unwrap().is_empty());
        let epic = json!({"store": "epic", "sections": [{"id": "free", "items": [1]}, {"id": "top", "items": [2]}, {"id": "freeNext", "items": [3]}]});
        let f = free_page(&epic);
        assert_eq!(f["sections"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect::<Vec<_>>(), ["free", "freeNext"]);
        assert!(store_page("nope").is_err());
        assert_eq!(store_page("battlenet").unwrap()["inApp"], true);
        assert_eq!(STORE_PAGES.len(), 7);
    }

    #[test]
    fn downloads_are_short() {
        let d = json!({"active": [{"id": "gog:1", "title": "A", "source": "gog", "state": "downloading", "progress": 0.5, "bytesPerSec": 10, "art": {"x": 1}}], "recent": []});
        let p = downloads_page(&d);
        assert_eq!(p["active"][0]["store"], "GOG");
        assert_eq!(p["active"][0]["progress"], 0.5);
        assert!(p["active"][0].get("art").is_none());
        assert_eq!(downloads_page(&json!({}))["active"], json!([]));
    }

    #[test]
    fn arguments() {
        let a = |v: &[&str]| parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&["nimbus", "--steam-mode-data", "store", "epic"]), Some(("store".into(), Some("epic".into()))));
        assert_eq!(a(&["nimbus", "--steam-mode-data", "home"]), Some(("home".into(), None)));
        assert_eq!(a(&["nimbus", "--steam-mode-data"]), None);
        assert_eq!(a(&["nimbus"]), None);
        assert!(answer("nope", None).is_err());
        assert!(answer("info", Some("no colon")).is_err());
    }
}
