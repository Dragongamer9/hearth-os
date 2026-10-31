//! Game Mode › Discover (docs/HYDRA-FEATURES.md): suggestions from what you play.
//!  - Not played yet: games in your library you haven't started (installed ones first).
//!  - Pick up again: games you put real time into but haven't played in a while.
//!  - Because you play <game>: Steam's best sellers in the genres of the games you play most (Steam's public search
//!    page, by Steam tag), leaving out games you have.
//!  - Popular on Steam that you don't have: Steam's top sellers (the Store page's own shelf), minus your games.
//! Nothing is sent anywhere about you: Hearth asks Steam's public pages for a genre, never for "you".

use serde_json::{json, Value};
use std::collections::HashSet;

const DAY_MS: u64 = 86_400_000;

/// A Steam genre's name → Steam's tag of the same name (the store's search filters by tag).
pub fn genre_tag(genre: &str) -> Option<u32> {
    Some(match genre.to_ascii_lowercase().as_str() {
        "action" => 19, "adventure" => 21, "rpg" => 122, "strategy" => 9, "simulation" => 599, "indie" => 492, "casual" => 597,
        "racing" => 699, "sports" => 701, "massively multiplayer" => 128, "free to play" => 113, "early access" => 493,
        _ => return None,
    })
}

fn mins(g: &Value) -> u64 { g["minutes"].as_u64().unwrap_or(0) }
fn last(g: &Value) -> u64 { g["lastPlayed"].as_u64().unwrap_or(0) }
fn visible(g: &Value) -> bool { g["hidden"] != true && g["cloud"] != true }

/// Library games you haven't started: installed first, then the newest additions.
pub fn unplayed(games: &[Value]) -> Vec<Value> {
    let mut l: Vec<&Value> = games.iter().filter(|g| visible(g) && mins(g) == 0 && last(g) == 0).collect();
    l.sort_by(|a, b| (b["installed"] == true).cmp(&(a["installed"] == true)).then(b["added"].as_u64().unwrap_or(0).cmp(&a["added"].as_u64().unwrap_or(0))));
    l.into_iter().take(24).map(|g| g["id"].clone()).collect()
}

/// Games you played for at least an hour but not in the last 30 days: most played first.
pub fn revisit(games: &[Value], now: u64) -> Vec<Value> {
    let mut l: Vec<&Value> = games.iter().filter(|g| visible(g) && mins(g) >= 60 && last(g) > 0 && now.saturating_sub(last(g)) > 30 * DAY_MS).collect();
    l.sort_by_key(|g| std::cmp::Reverse(mins(g)));
    l.into_iter().take(16).map(|g| g["id"].clone()).collect()
}

/// Your most played games (any store), up to `n`.
pub fn most_played(games: &[Value], n: usize) -> Vec<Value> {
    let mut l: Vec<&Value> = games.iter().filter(|g| mins(g) > 0).collect();
    l.sort_by_key(|g| std::cmp::Reverse(mins(g)));
    l.into_iter().take(n).cloned().collect()
}

/// A random game you haven't played and can play now (installed), for Surprise me. `seed`: any changing number.
pub fn surprise(games: &[Value], seed: u64) -> Option<Value> {
    let l: Vec<&Value> = games.iter().filter(|g| visible(g) && g["installed"] == true && mins(g) == 0).collect();
    if l.is_empty() { return None; }
    Some(l[(seed % l.len() as u64) as usize]["id"].clone())
}

/// What you own, to leave out of suggestions: Steam app ids and tidied names.
pub fn owned(games: &[Value]) -> (HashSet<String>, HashSet<String>) {
    let ids = games.iter().filter_map(|g| g["id"].as_str()?.strip_prefix("steam:").map(str::to_string)).collect();
    let names = games.iter().filter_map(|g| g["title"].as_str()).map(crate::gameinfo::norm).collect();
    (ids, names)
}
pub fn not_owned(items: &[Value], owned: &(HashSet<String>, HashSet<String>), max: usize) -> Vec<Value> {
    items.iter().filter(|i| {
        let id = i["id"].as_str().unwrap_or("");
        !owned.0.contains(id) && !owned.1.contains(&crate::gameinfo::norm(i["title"].as_str().unwrap_or("")))
    }).take(max).cloned().collect()
}

/// The genre to suggest for: the first of the game's genres that Steam has a tag for, skipping ones already used
/// (so three favourites don't all give "Action").
pub fn pick_genre<'a>(genres: &'a [String], used: &HashSet<String>) -> Option<&'a String> {
    genres.iter().find(|g| genre_tag(g).is_some() && !used.contains(*g) && !["indie", "free to play", "early access"].contains(&g.to_ascii_lowercase().as_str()))
        .or_else(|| genres.iter().find(|g| genre_tag(g).is_some() && !used.contains(*g)))
}

fn genres_of(appid: u64) -> Vec<String> {
    crate::storefront::details("steam", &appid.to_string())
        .map(|d| d["genres"].as_array().into_iter().flatten().filter_map(|g| g.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn best_in_tag(tag: u32) -> Result<Vec<Value>, String> {
    let (market, _) = crate::storefront::locale();
    let v = crate::storefront::cached(&format!("play-tag-{market}-{tag}"), DAY_MS, || {
        let url = format!("https://store.steampowered.com/search/results/?query&start=0&count=30&tags={tag}&filter=topsellers&json=1&cc={market}&l=english");
        let r = crate::storefront::fetch("steam", "search", "GET", &url, None, &[])?;
        let s = crate::storefront::steam::parse_popular_new(&r);
        Ok(s.into_iter().next().map(|x| x["items"].clone()).unwrap_or(json!([])))
    })?;
    Ok(v.as_array().cloned().unwrap_or_default())
}

/// The Discover page: {shelves: [{id, title, kind: "library"|"store", items}], surprise}.
/// `games`: the library as the page sees it (library.rs dto).
pub fn build(games: &[Value], now: u64) -> Value {
    let mut shelves = vec![];
    let un = unplayed(games);
    if !un.is_empty() { shelves.push(json!({"id": "unplayed", "title": "Not played yet", "sub": "In your library, waiting for a first go", "kind": "library", "items": un})); }
    let re = revisit(games, now);
    if !re.is_empty() { shelves.push(json!({"id": "revisit", "title": "Pick up again", "sub": "Games you put time into, not played in a while", "kind": "library", "items": re})); }
    let own = owned(games);
    let mut used = HashSet::new();
    for g in most_played(games, 6) {
        if used.len() >= 3 { break; }
        let appid = super::details::steam_id(g["id"].as_str().unwrap_or(""), g["title"].as_str().unwrap_or(""), g["source"].as_str().unwrap_or(""));
        if appid == 0 { continue; }
        let genres = genres_of(appid);
        let Some(genre) = pick_genre(&genres, &used).cloned() else { continue };
        let Some(tag) = genre_tag(&genre) else { continue };
        used.insert(genre.clone());
        if let Ok(items) = best_in_tag(tag) {
            let items = not_owned(&items, &own, 16);
            if !items.is_empty() {
                shelves.push(json!({"id": format!("like-{tag}"), "title": format!("Because you play {}", g["title"].as_str().unwrap_or("")), "sub": format!("Best-selling {genre} games on Steam you don't have"), "kind": "store", "items": items}));
            }
        }
    }
    if let Ok(h) = crate::storefront::home("steam") {
        let top: Vec<Value> = h["sections"].as_array().into_iter().flatten().filter(|s| s["id"] == "top" || s["id"] == "featured").flat_map(|s| s["items"].as_array().cloned().unwrap_or_default()).collect();
        let items = not_owned(&top, &own, 20);
        if !items.is_empty() { shelves.push(json!({"id": "popular", "title": "Popular games you haven't played", "sub": "Steam's top sellers, minus what you have", "kind": "store", "items": items})); }
    }
    json!({"shelves": shelves, "surprise": surprise(games, now / 1000)})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn g(id: &str, title: &str, minutes: u64, last: u64, installed: bool, added: u64) -> Value {
        json!({"id": id, "title": title, "minutes": minutes, "lastPlayed": last, "installed": installed, "added": added, "source": id.split(':').next().unwrap()})
    }

    #[test]
    fn library_shelves() {
        let now = 1_000 * DAY_MS;
        let games = vec![
            g("steam:1", "Portal 2", 900, now - 40 * DAY_MS, true, 1),
            g("steam:2", "Hades", 0, 0, false, 9),
            g("gog:3", "Witcher 3", 0, 0, true, 2),
            g("manual:4", "My Game", 30, now - 90 * DAY_MS, true, 3),
            g("epic:5", "Recent", 500, now - DAY_MS, true, 4),
            json!({"id": "xbox:6", "title": "Cloud", "minutes": 0, "lastPlayed": 0, "cloud": true}),
            json!({"id": "steam:7", "title": "Hidden", "minutes": 0, "lastPlayed": 0, "hidden": true, "installed": true}),
        ];
        assert_eq!(unplayed(&games), vec![json!("gog:3"), json!("steam:2")], "installed first; cloud and hidden games left out");
        assert_eq!(revisit(&games, now), vec![json!("steam:1")], "an hour or more, not in 30 days");
        assert_eq!(most_played(&games, 2).iter().map(|x| x["id"].clone()).collect::<Vec<_>>(), vec![json!("steam:1"), json!("epic:5")]);
        assert_eq!(surprise(&games, 0), Some(json!("gog:3")), "only installed, unplayed games");
        assert_eq!(surprise(&[], 3), None);
    }

    #[test]
    fn store_suggestions_leave_out_your_games() {
        let games = vec![g("steam:620", "Portal 2", 10, 1, true, 1), g("gog:9", "Hades", 0, 0, true, 1)];
        let own = owned(&games);
        let items = vec![json!({"id": "620", "title": "Portal 2"}), json!({"id": "1145360", "title": "Hades"}), json!({"id": "1", "title": "Celeste"})];
        let left = not_owned(&items, &own, 10);
        assert_eq!(left, vec![json!({"id": "1", "title": "Celeste"})], "by Steam id and by name (bought on GOG)");
        assert_eq!(genre_tag("RPG"), Some(122));
        assert_eq!(genre_tag("Nudity"), None);
        let gs: Vec<String> = vec!["Indie".into(), "Action".into(), "Adventure".into()];
        let mut used = HashSet::new();
        assert_eq!(pick_genre(&gs, &used).map(String::as_str), Some("Action"), "Indie is too broad when there's another");
        used.insert("Action".to_string());
        assert_eq!(pick_genre(&gs, &used).map(String::as_str), Some("Adventure"));
        used.insert("Adventure".to_string());
        assert_eq!(pick_genre(&gs, &used).map(String::as_str), Some("Indie"));
        assert_eq!(pick_genre(&["Gore".to_string()], &HashSet::new()), None);
    }
}
