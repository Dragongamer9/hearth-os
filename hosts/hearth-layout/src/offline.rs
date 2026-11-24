//! Describe it with no AI at all (the owner, Nov 23: "figure out a way for the describe it to work without local ai",
//! then "Let's do your suggestions"): Hearth matches the common things people say itself, offline and at once.
//!
//!   "like a cabin" / "make it look like a console" / "my first PC"   → that ready-made desktop (screen.rs READY)
//!   "make the clock bigger" / "smaller weather"                       → that block two squares bigger or smaller
//!   "add the weather" / "add a clock and my music"                    → the block in a free place on the grid
//!   "remove the friends" / "hide the battery"                         → those blocks gone
//!   "move the clock to the top right" / "games at the bottom"         → the block moved there
//!
//! A request is only answered here when every part of it was understood ("like a cabin and add my printer's camera"
//! isn't: the camera needs an AI), so nothing anyone asked is quietly left out; otherwise describe.rs asks an AI. The
//! answer is checked by screen.rs like an AI's.

use serde_json::{json, Value};

/// Words for each ready-made desktop.
const LOOKS: &[(&str, &[&str])] = &[
    ("console", &["console", "xbox", "dashboard", "tiles"]),
    ("iconrow", &["playstation", "ps5", "ps4", "icon row", "icons in a row"]),
    ("cockpit", &["cockpit", "sci-fi", "scifi", "hud", "starship", "spaceship", "futuristic"]),
    ("cabin", &["cabin", "cozy", "cosy", "wood", "fireplace", "fire", "warm"]),
    ("notebook", &["notebook", "paper", "lists", "sticky note", "notepad"]),
    ("retro", &["1995", "retro", "windows 95", "old computer", "first pc", "90s", "nineties"]),
];

/// Words for each block (screen.rs SCREEN_BLOCKS) and the size it's added at.
const BLOCKS: &[(&str, &[&str], u32, u32)] = &[
    ("clock", &["clock", "time"], 4, 3),
    ("weather", &["weather", "forecast"], 4, 3),
    ("battery", &["battery"], 3, 1),
    ("media", &["music", "media", "what's playing", "now playing"], 6, 3),
    ("notes", &["notes", "note", "to do", "todo", "checklist"], 5, 4),
    ("friends", &["friends", "who's online"], 5, 3),
    ("profile", &["profile", "my picture", "my name"], 4, 1),
    ("status", &["status", "wifi", "volume"], 4, 1),
    ("downloads", &["downloads"], 5, 3),
    ("dock", &["dock"], 10, 2),
    ("hero", &["hero", "big tile", "last game"], 8, 5),
    ("tilerow", &["games", "tile row", "tiles of my games", "my apps"], 12, 3),
    ("guide", &["guide", "side menu"], 4, 10),
    ("search", &["search"], 6, 1),
    ("activity", &["activity"], 8, 3),
];

const ADD: &[&str] = &["add", "put", "show", "give me", "i want", "with"];
const REMOVE: &[&str] = &["remove", "hide", "delete", "get rid of", "take away", "no "];
const BIGGER: &[&str] = &["bigger", "larger", "huge", "grow"];
const SMALLER: &[&str] = &["smaller", "tiny", "shrink", "less big"];
const MOVE: &[&str] = &["move", "put", "at the", "to the", "in the"];

fn has(t: &str, words: &[&str]) -> bool {
    words.iter().any(|w| t.contains(w))
}

fn block_in(t: &str) -> Option<&'static (&'static str, &'static [&'static str], u32, u32)> {
    BLOCKS.iter().find(|(_, w, _, _)| has(t, w))
}

/// Where on the grid a place word puts a w×h block.
fn place(t: &str, cols: u32, rows: u32, w: u32, h: u32) -> Option<(u32, u32)> {
    let right = t.contains("right");
    let left = t.contains("left");
    let top = t.contains("top") || t.contains("up");
    let bottom = t.contains("bottom") || t.contains("down");
    let centre = t.contains("middle") || t.contains("centre") || t.contains("center");
    if !(right || left || top || bottom || centre || t.contains("corner")) {
        return None;
    }
    let x = if right || (t.contains("corner") && !left) { cols.saturating_sub(w) } else if left { 0 } else { (cols.saturating_sub(w)) / 2 };
    let y = if bottom { rows.saturating_sub(h) } else if top || t.contains("corner") { 0 } else { (rows.saturating_sub(h)) / 2 };
    Some((x, y))
}

fn grid(s: &Value) -> (u32, u32) {
    let g = s.get("grid");
    (g.and_then(|g| g.get("cols")).and_then(Value::as_u64).unwrap_or(24) as u32, g.and_then(|g| g.get("rows")).and_then(Value::as_u64).unwrap_or(14) as u32)
}

fn free_spot(s: &Value, w: u32, h: u32) -> Option<(u32, u32)> {
    let (cols, rows) = grid(s);
    let taken: Vec<(u32, u32, u32, u32)> = s.get("blocks").and_then(Value::as_array).map(|a| a.iter().map(|b| {
        let n = |k: &str| b.get(k).and_then(Value::as_u64).unwrap_or(0) as u32;
        (n("x"), n("y"), n("w"), n("h"))
    }).collect()).unwrap_or_default();
    for y in 0..=rows.saturating_sub(h) {
        for x in (0..=cols.saturating_sub(w)).rev() {
            if !taken.iter().any(|&(bx, by, bw, bh)| x < bx + bw && bx < x + w && y < by + bh && by < y + h) {
                return Some((x, y));
            }
        }
    }
    None
}

/// What Hearth understood, as an AI's answer would have it: {"kind": "screen", "screen", "changes"}; None when any part
/// of the request needs an AI.
pub fn screen(text: &str, current: &Value) -> Option<Value> {
    let t = text.to_lowercase();
    let parts: Vec<String> = t.replace(" and then ", ",").replace(" and ", ",").replace(" then ", ",").replace(';', ",").replace('.', ",")
        .split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut s = current.clone();
    let mut changes = Vec::new();
    for p in &parts {
        // a whole look ("like a cabin")
        if let Some((id, _)) = LOOKS.iter().find(|(_, w)| has(p, w)).filter(|_| block_in(p).is_none() || has(p, &["like", "look"])) {
            let ready = crate::screen::READY.iter().find(|(r, _)| r == id)?;
            s = serde_json::from_str(ready.1).ok()?;
            changes.push(format!("Your desktop is now the {} one, Hearth's own (no AI needed)", match *id { "iconrow" => "icon row", "retro" => "1995 computer", x => x }));
            continue;
        }
        let (kind, _, dw, dh) = *block_in(p)?;
        let (cols, rows) = grid(&s);
        let blocks = s.get_mut("blocks").and_then(Value::as_array_mut)?;
        let here: Vec<usize> = blocks.iter().enumerate().filter(|(_, b)| b.get("block").and_then(Value::as_str) == Some(kind)).map(|(i, _)| i).collect();
        if has(p, REMOVE) {
            if here.is_empty() {
                return None;
            }
            blocks.retain(|b| b.get("block").and_then(Value::as_str) != Some(kind));
            changes.push(format!("The {kind} is gone"));
        } else if has(p, BIGGER) || has(p, SMALLER) {
            let i = *here.first()?;
            let b = &mut blocks[i];
            let n = |b: &Value, k: &str| b.get(k).and_then(Value::as_u64).unwrap_or(1) as i64;
            let d: i64 = if has(p, BIGGER) { 2 } else { -2 };
            let (x, y) = (n(b, "x"), n(b, "y"));
            let w = (n(b, "w") + d).clamp(1, cols as i64 - x);
            let h = (n(b, "h") + d).clamp(1, rows as i64 - y);
            b["w"] = json!(w);
            b["h"] = json!(h);
            changes.push(format!("The {kind} is {}", if d > 0 { "bigger" } else { "smaller" }));
        } else if has(p, ADD) && here.is_empty() {
            // its own size where it fits, else a little smaller (a full screen keeps its blocks as they are)
            let (dw, dh) = (dw.min(cols), dh.min(rows));
            let mut spot = place(p, cols, rows, dw, dh).map(|(x, y)| (x, y, dw, dh));
            if spot.is_none() {
                'size: for h in (1..=dh).rev() {
                    for w in ((dw / 2).max(1)..=dw).rev() {
                        if let Some((x, y)) = free_spot(&s, w, h) {
                            spot = Some((x, y, w, h));
                            break 'size;
                        }
                    }
                }
            }
            let (x, y, w, h) = spot?;
            let blocks = s.get_mut("blocks").and_then(Value::as_array_mut)?;
            blocks.push(json!({ "id": format!("{kind}-{}", blocks.len() + 1), "block": kind, "x": x, "y": y, "w": w, "h": h }));
            changes.push(format!("Added the {kind}"));
        } else if has(p, MOVE) || place(p, cols, rows, 1, 1).is_some() {
            let i = *here.first()?;
            let w = blocks[i].get("w").and_then(Value::as_u64).unwrap_or(1) as u32;
            let h = blocks[i].get("h").and_then(Value::as_u64).unwrap_or(1) as u32;
            let (x, y) = place(p, cols, rows, w, h)?;
            blocks[i]["x"] = json!(x);
            blocks[i]["y"] = json!(y);
            changes.push(format!("The {kind} moved"));
        } else {
            return None;
        }
    }
    s["said"] = json!(text.trim());
    Some(json!({ "kind": "screen", "screen": s, "changes": changes }))
}

/// What Hearth can do with no AI, for the message when no AI answers.
pub const KNOWS: &str = "Without an AI, Hearth understands: a look (\"like a cabin\", \"like a console\", \"1995 computer\"), and making a block bigger or smaller, adding, removing or moving it (\"add the weather\", \"move the clock to the top right\").";

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> Value {
        serde_json::from_str(crate::screen::READY.iter().find(|(r, _)| *r == "console").unwrap().1).unwrap()
    }

    #[test]
    fn a_look_is_a_ready_made() {
        let v = screen("make it look like a cozy cabin", &console()).unwrap();
        assert!(crate::screen::parse(&v["screen"]).is_ok());
        assert_eq!(v["screen"]["said"], "make it look like a cozy cabin");
    }

    #[test]
    fn blocks_are_added_moved_and_resized() {
        let v = screen("add the weather and move it to the top right", &console());
        // "move it": no block named in the second part, so an AI is asked
        assert!(v.is_none());
        let v = screen("add the weather, make the hero bigger", &console()).unwrap();
        let s = crate::screen::parse(&v["screen"]).expect("still a good screen");
        assert!(s.blocks.iter().any(|b| b.block == "weather"));
    }

    #[test]
    fn anything_it_doesnt_know_goes_to_an_ai() {
        assert!(screen("like a cabin and add my printer's camera", &console()).is_none());
        assert!(screen("make it feel like autumn", &console()).is_none());
    }
}
