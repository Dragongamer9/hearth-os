//! Mantel's Settings app asking Hearth's core: `nimbus --settings-call <method> [json args]` makes one of Hearth's own
//! calls (the same ones the archived Game Mode's Settings pages made) and prints one line of JSON:
//! {"ok": true, "value": ...} or {"ok": false, "error": "..."}. No window is opened, nothing listens.
//!
//! The settings that only lived in Hearth's old Game Mode moved to Mantel's Settings (docs/SETTINGS-MOVE.md):
//! Remote play (Sunshine), Steam Game Mode (Decky Loader and Hearth Sync), Storage (drives, clean up, the low space
//! warning) and backups. Only the calls in CALLS are made, each with only its own arguments (strings, booleans, small
//! numbers, lists of strings); anything else is refused. Calls whose work outlives one answer (moving a game, a full
//! size scan) aren't here: they stay with the Hearth app in a window (`--windowed --page <page>`).
//! The Bridge is core_service's (made the first time, answered when its work is done, up to WAIT).

use serde_json::{json, Value};

/// How long a call may take (Decky Loader's install asks for a password and downloads; cleaning up deletes gigabytes).
const WAIT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Every call Mantel's Settings may make, with the arguments it may carry.
pub const CALLS: &[(&str, &[&str])] = &[
    // Settings › Remote play (SetRemote.qml)
    ("remote.status", &[]), ("remote.setOn", &["on"]), ("remote.pair", &["pin", "name"]), ("remote.unpair", &["uuid"]),
    ("remote.login", &["user", "pass"]), ("moonlight.install", &[]),
    // Settings › Steam Game Mode (SetSteam.qml)
    ("steamMode.status", &[]), ("steamMode.decky", &["action"]),
    // Settings › Storage (SetStorage.qml)
    ("diskspace.status", &[]), ("diskspace.cleanList", &[]), ("diskspace.clean", &["keys", "confirmed"]), ("diskspace.setWarn", &["on", "gb"]),
    // Settings › About › Backup (SetAbout.qml)
    ("backup.status", &[]), ("backup.create", &["dest", "signins"]),
];

/// The call and its arguments, cleaned: only the listed keys, only plain values. Err: why it's refused.
pub fn check(method: &str, args: &Value) -> Result<(String, Value), String> {
    let (m, keys) = CALLS.iter().find(|(n, _)| *n == method).ok_or_else(|| format!("{method:.40} isn't something Settings asks Hearth"))?;
    if !(args.is_object() || args.is_null()) { return Err("The arguments have to be a JSON object".into()); }
    fn ok_str(s: &str) -> bool { s.len() <= 400 && !s.chars().any(|c| c.is_control()) }
    let mut out = serde_json::Map::new();
    for k in keys.iter() {
        let v = &args[*k];
        let ok = match v {
            Value::Null => continue,
            Value::Bool(_) => true,
            Value::Number(n) => n.as_u64().map(|x| x <= 100_000).unwrap_or(false),
            Value::String(s) => ok_str(s),
            Value::Array(l) => l.len() <= 500 && l.iter().all(|x| x.as_str().map(ok_str).unwrap_or(false)),
            _ => false,
        };
        if !ok { return Err(format!("That isn't something Settings sends ({k})")); }
        out.insert(k.to_string(), v.clone());
    }
    if *m == "steamMode.decky" && !crate::steam_mode::decky_action_ok(out.get("action").and_then(Value::as_str).unwrap_or("")) {
        return Err("Unknown choice".into());
    }
    if *m == "remote.pair" && !out.get("pin").and_then(Value::as_str).map(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit())).unwrap_or(false) {
        return Err("The PIN is the 4 digits Moonlight shows".into());
    }
    Ok((m.to_string(), Value::Object(out)))
}

/// `nimbus --settings-call <method> [json]`: prints the answer and returns the exit code (0 answered, 1 refused or failed).
pub fn run(args: &[String]) -> i32 {
    let at = args.iter().position(|a| a == "--settings-call").map(|i| i + 1).unwrap_or(args.len());
    let method = args.get(at).map(String::as_str).unwrap_or("");
    let raw = args.get(at + 1).map(String::as_str).unwrap_or("{}");
    let out = serde_json::from_str::<Value>(raw).map_err(|_| "The arguments aren't JSON".to_string())
        .and_then(|a| check(method, &a))
        .and_then(|(m, a)| crate::core_service::bridge_call_within(&m, &a, WAIT));
    match out {
        Ok(v) => { println!("{}", json!({"ok": true, "value": v})); 0 }
        Err(e) => { println!("{}", json!({"ok": false, "error": e})); 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_listed_calls_and_their_own_arguments() {
        assert!(check("library.downloads", &json!({})).is_err());
        assert!(check("game.uninstall", &json!({"id": "steam:1"})).is_err());
        assert!(check("diskspace.move", &json!({"id": "steam:1", "to": "/"})).is_err());
        let (m, a) = check("remote.setOn", &json!({"on": true, "extra": "x"})).unwrap();
        assert_eq!(m, "remote.setOn");
        assert_eq!(a, json!({"on": true}));
        assert_eq!(check("remote.status", &Value::Null).unwrap().1, json!({}));
        assert_eq!(check("diskspace.setWarn", &json!({"on": true, "gb": 15})).unwrap().1, json!({"on": true, "gb": 15}));
        assert_eq!(check("diskspace.clean", &json!({"keys": ["a", "b"], "confirmed": []})).unwrap().1, json!({"keys": ["a", "b"], "confirmed": []}));
    }

    #[test]
    fn odd_arguments_are_refused() {
        assert!(check("remote.setOn", &json!([1])).is_err());
        assert!(check("diskspace.setWarn", &json!({"gb": -1})).is_err());
        assert!(check("diskspace.setWarn", &json!({"gb": 1.5})).is_err());
        assert!(check("diskspace.clean", &json!({"keys": [1]})).is_err());
        assert!(check("remote.unpair", &json!({"uuid": "a\nb"})).is_err());
        assert!(check("remote.unpair", &json!({"uuid": {"x": 1}})).is_err());
        assert!(check("steamMode.decky", &json!({"action": "rm -rf"})).is_err());
        assert!(check("steamMode.decky", &json!({"action": "install"})).is_ok());
        assert!(check("remote.pair", &json!({"pin": "12a4"})).is_err());
        assert!(check("remote.pair", &json!({"pin": "1234", "name": "Phone"})).is_ok());
    }

    #[test]
    fn the_command_line_refuses_without_making_a_bridge() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(run(&a(&["nimbus", "--settings-call", "game.uninstall", "{\"id\":\"steam:1\"}"])), 1);
        assert_eq!(run(&a(&["nimbus", "--settings-call", "remote.status", "not json"])), 1);
        assert_eq!(run(&a(&["nimbus", "--settings-call"])), 1);
    }
}
