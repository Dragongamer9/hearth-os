//! hearth_server_parts: the Server side of Hearth OS (docs/HOME-SERVER-PARTS.md; boards docs/mockups/server-expansion,
//! approved Nov 26, 2026: "thoes look good").
//!
//! * [`parts`]: the server's parts, one line each (the Home server helper's sidebar and Settings › Home server's tiles
//!   are both made from it, so a new part is one line);
//! * [`engine`]: running a part's engine (Python, in the Home server helper's Store system part, so every part arrives
//!   and updates through the Hearth Store: "All this has to be updated thru the store");
//! * [`store`]: each part's state in the Hearth Store (up to date, an update ready, a test version);
//! * [`draw`]: the boards' small graphs and pieces (a spark line, bars, a ring, the KPI box, status dots);
//! * [`home`]: the Server home board (board 11): the overall state, a tile per part, Updates for the server, your
//!   devices, and what else is here.

pub mod draw;
pub mod engine;
pub mod home;
pub mod parts;
pub mod store;

use serde_json::Value;

/// A value by a dotted path ("machine.gpu.pct").
pub fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    let mut cur = v;
    for k in path.split('.') {
        cur = match cur {
            Value::Object(m) => m.get(k).unwrap_or(&Value::Null),
            Value::Array(a) => k.parse::<usize>().ok().and_then(|i| a.get(i)).unwrap_or(&Value::Null),
            _ => &Value::Null,
        };
    }
    cur
}
/// Its words (a number as it was written, true/false), or "".
pub fn s(v: &Value, path: &str) -> String {
    match at(v, path) {
        Value::String(x) => x.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}
pub fn f(v: &Value, path: &str) -> f64 {
    match at(v, path) {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(x) => x.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}
pub fn b(v: &Value, path: &str) -> bool {
    match at(v, path) {
        Value::Bool(b) => *b,
        Value::String(x) => x == "on" || x == "true",
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        _ => false,
    }
}
pub fn arr(v: &Value, path: &str) -> Vec<Value> {
    at(v, path).as_array().cloned().unwrap_or_default()
}
/// Numbers 0..1 (a graph's points).
pub fn nums(v: &Value, path: &str) -> Vec<f32> {
    arr(v, path).iter().filter_map(Value::as_f64).map(|x| x as f32).collect()
}
/// A list of words.
pub fn strs(v: &Value, path: &str) -> Vec<String> {
    arr(v, path).iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
}
