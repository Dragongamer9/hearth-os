//! Your gaming profile (docs/HYDRA-FEATURES.md): what you're playing now, what you played lately, your most played
//! games and your total play time, from Hearth's own play-time tracking (every game in the library, any store).
//! Friends › Activity shows yours and, when you turn on "Share my activity", sends it to your Hearth friends over
//! Hearth Friends (an encrypted message in each friend's chat, hosts/hearth-friends). Off until you turn it on.

use serde_json::{json, Value};

const DAY_MS: u64 = 86_400_000;

/// The profile from the library (library.rs dto list) and what's running.
pub fn mine(games: &[Value], running: Option<&str>, now: u64) -> Value {
    let played: Vec<&Value> = games.iter().filter(|g| g["minutes"].as_u64().unwrap_or(0) > 0 || g["lastPlayed"].as_u64().unwrap_or(0) > 0).collect();
    let card = |g: &Value| json!({"id": g["id"], "title": g["title"], "source": g["source"], "minutes": g["minutes"].as_u64().unwrap_or(0), "lastPlayed": g["lastPlayed"].as_u64().unwrap_or(0)});
    let mut recent: Vec<&Value> = played.iter().copied().filter(|g| g["lastPlayed"].as_u64().unwrap_or(0) > now.saturating_sub(30 * DAY_MS)).collect();
    recent.sort_by_key(|g| std::cmp::Reverse(g["lastPlayed"].as_u64().unwrap_or(0)));
    let mut top = played.clone();
    top.sort_by_key(|g| std::cmp::Reverse(g["minutes"].as_u64().unwrap_or(0)));
    let total: u64 = played.iter().map(|g| g["minutes"].as_u64().unwrap_or(0)).sum();
    let week: u64 = recent.iter().filter(|g| g["lastPlayed"].as_u64().unwrap_or(0) > now.saturating_sub(7 * DAY_MS)).count() as u64;
    let playing = running.and_then(|id| games.iter().find(|g| g["id"] == id)).map(|g| card(g));
    json!({
        "playing": playing,
        "recent": recent.iter().take(8).map(|g| card(*g)).collect::<Vec<_>>(),
        "top": top.iter().take(5).map(|g| card(*g)).collect::<Vec<_>>(),
        "totalMinutes": total, "gamesPlayed": played.len(), "playedThisWeek": week, "librarySize": games.len(),
    })
}

/// What goes to friends: titles and times only (no store ids, folders or anything else), at most 5 recent games.
pub fn shared(m: &Value) -> Value {
    let slim = |g: &Value| json!({"title": g["title"].as_str().unwrap_or("").chars().take(120).collect::<String>(), "minutes": g["minutes"], "lastPlayed": g["lastPlayed"]});
    json!({
        "playing": m["playing"].as_object().map(|_| slim(&m["playing"])),
        "recent": m["recent"].as_array().into_iter().flatten().take(5).map(slim).collect::<Vec<_>>(),
        "top": m["top"].as_array().into_iter().flatten().take(3).map(slim).collect::<Vec<_>>(),
        "totalMinutes": m["totalMinutes"], "gamesPlayed": m["gamesPlayed"],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_from_play_time() {
        let now = 100 * DAY_MS;
        let games = vec![
            json!({"id": "steam:1", "title": "Portal 2", "source": "steam", "minutes": 600, "lastPlayed": now - DAY_MS, "path": "/secret/folder"}),
            json!({"id": "gog:2", "title": "Witcher 3", "source": "gog", "minutes": 3000, "lastPlayed": now - 60 * DAY_MS}),
            json!({"id": "manual:3", "title": "My Game", "source": "manual", "minutes": 30, "lastPlayed": now - 3 * DAY_MS}),
            json!({"id": "epic:4", "title": "Never", "source": "epic", "minutes": 0, "lastPlayed": 0}),
        ];
        let m = mine(&games, Some("manual:3"), now);
        assert_eq!(m["totalMinutes"], 3630);
        assert_eq!(m["gamesPlayed"], 3);
        assert_eq!(m["playedThisWeek"], 2);
        assert_eq!(m["playing"]["title"], "My Game");
        assert_eq!(m["recent"].as_array().unwrap().iter().map(|g| g["title"].as_str().unwrap()).collect::<Vec<_>>(), vec!["Portal 2", "My Game"], "last 30 days, newest first");
        assert_eq!(m["top"][0]["title"], "Witcher 3");
        let s = shared(&m);
        assert!(!s.to_string().contains("steam:1") && !s.to_string().contains("/secret"), "only titles and times go to friends: {s}");
        assert_eq!(s["playing"]["title"], "My Game");
        assert_eq!(s["top"].as_array().unwrap().len(), 3);
        let idle = shared(&mine(&games, None, now));
        assert!(idle["playing"].is_null());
    }
}
