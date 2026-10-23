//! Passwords & Keys: what apps saved in your keyring (the Secret Service: GNOME Keyring or KWallet, over D-Bus)
//! and your saved Wi-Fi passwords (NetworkManager). Listing shows names only. Showing a password first checks
//! your account password (hearth-admin confirm-password; Wi-Fi passwords come from hearth-admin wifi-psk, which
//! checks it itself). Nothing here is sent anywhere or written to a log: secrets only go back to the page that
//! asked, and the page forgets them when you hide them.

use crate::util;
use serde_json::{json, Value};

/// A NetworkManager connection id (UUID): only hex digits and dashes reach nmcli or hearth-admin.
pub fn valid_uuid(u: &str) -> bool {
    u.len() == 36 && u.chars().enumerate().all(|(i, c)| if [8, 13, 18, 23].contains(&i) { c == '-' } else { c.is_ascii_hexdigit() })
}

/// The password (stdin JSON for hearth-admin) from the page.
fn password_json(args: &Value) -> Result<Vec<u8>, String> {
    let p = util::arg(args, "password");
    if p.is_empty() { return Err("Type your password first.".into()); }
    Ok(json!({"password": p}).to_string().into_bytes())
}

/// Saved Wi-Fi networks (names only): [{name, uuid, lastUsed}].
pub fn wifi_list() -> Result<Value, String> {
    let o = util::run("nmcli", &["-t", "-e", "yes", "-f", "NAME,UUID,TYPE,TIMESTAMP", "connection", "show"], 15)?;
    if !o.ok() { return Err(format!("Couldn't ask NetworkManager ({})", o.why())); }
    let list: Vec<Value> = o.out.lines().map(crate::network::split_nmcli).filter(|f| f.len() >= 4 && (f[2] == "802-11-wireless" || f[2] == "wifi"))
        .map(|f| json!({"name": f[0], "uuid": f[1], "lastUsed": f[3].parse::<u64>().ok().filter(|t| *t > 0)})).collect();
    Ok(json!({"networks": list}))
}

/// A saved Wi-Fi network's password, after your password (hearth-admin wifi-psk checks it, then asks
/// NetworkManager as root).
pub fn wifi_reveal(args: &Value) -> Result<Value, String> {
    let uuid = util::arg(args, "uuid");
    if !valid_uuid(uuid) { return Err("Unknown network.".into()); }
    let input = password_json(args)?;
    let v = util::admin(&["wifi-psk", uuid], Some(&input), 60)?;
    Ok(json!({"secret": v["psk"], "kind": v["kind"]}))
}

/// Forgets a saved Wi-Fi network (NetworkManager asks for permission itself if it needs to).
pub fn wifi_forget(args: &Value) -> Result<Value, String> {
    let uuid = util::arg(args, "uuid");
    if !valid_uuid(uuid) { return Err("Unknown network.".into()); }
    let o = util::run("nmcli", &["connection", "delete", "uuid", uuid], 60)?;
    if o.ok() { Ok(json!({"ok": true})) } else { Err(format!("Couldn't forget it ({})", o.why())) }
}

/// Checks your account password (hearth-admin confirm-password), before a keyring password is shown.
fn confirm(args: &Value) -> Result<(), String> {
    let input = password_json(args)?;
    util::admin(&["confirm-password"], Some(&input), 60).map(|_| ())
}

#[cfg(target_os = "linux")]
mod ss {
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use zbus::blocking::Connection;
    use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value as V};

    const DEST: &str = "org.freedesktop.secrets";
    const ROOT: &str = "/org/freedesktop/secrets";
    const SERVICE: &str = "org.freedesktop.Secret.Service";
    const ITEM: &str = "org.freedesktop.Secret.Item";
    const COLLECTION: &str = "org.freedesktop.Secret.Collection";

    fn plain(e: zbus::Error) -> String {
        let t = e.to_string();
        if t.contains("ServiceUnknown") || t.contains("was not provided") { return "No keyring is running in this session (GNOME Keyring or KWallet).".into(); }
        if t.contains("IsLocked") { return "The keyring is locked.".into(); }
        format!("The keyring said no: {t}")
    }

    fn conn() -> Result<Connection, String> { Connection::session().map_err(|e| format!("Couldn't reach your session's services: {e}")) }

    fn call<B, R>(c: &Connection, path: &str, iface: &str, method: &str, body: &B) -> Result<R, String>
    where B: serde::Serialize + zbus::zvariant::DynamicType, R: serde::de::DeserializeOwned + zbus::zvariant::Type {
        let m = c.call_method(Some(DEST), path, Some(iface), method, body).map_err(plain)?;
        m.body().deserialize::<R>().map_err(|e| format!("The keyring answered strangely: {e}"))
    }

    fn props(c: &Connection, path: &str, iface: &str) -> Result<HashMap<String, OwnedValue>, String> {
        call(c, path, "org.freedesktop.DBus.Properties", "GetAll", &(iface,))
    }

    fn s(p: &HashMap<String, OwnedValue>, k: &str) -> String { p.get(k).and_then(|v| String::try_from(v.try_clone().ok()?).ok()).unwrap_or_default() }
    fn u(p: &HashMap<String, OwnedValue>, k: &str) -> u64 { p.get(k).and_then(|v| u64::try_from(v.try_clone().ok()?).ok()).unwrap_or(0) }
    fn attrs(p: &HashMap<String, OwnedValue>) -> HashMap<String, String> {
        p.get("Attributes").and_then(|v| HashMap::<String, String>::try_from(v.try_clone().ok()?).ok()).unwrap_or_default()
    }

    /// Every item in every keyring (names and details, never the secrets). SearchItems with no attributes
    /// matches everything, unlocked and locked.
    pub fn list() -> Result<Value, String> {
        let c = conn()?;
        let empty: HashMap<&str, &str> = HashMap::new();
        let (unlocked, locked): (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) = call(&c, ROOT, SERVICE, "SearchItems", &(empty,))?;
        let mut out = vec![];
        let mut keyrings: HashMap<String, Value> = HashMap::new();
        for (it, is_locked) in unlocked.iter().map(|p| (p, false)).chain(locked.iter().map(|p| (p, true))) {
            let col = it.as_str().rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
            if !keyrings.contains_key(&col) {
                let label = props(&c, &col, COLLECTION).map(|cp| s(&cp, "Label")).unwrap_or_default();
                keyrings.insert(col.clone(), json!({"path": col, "label": label, "locked": is_locked}));
            }
            let label = keyrings[&col]["label"].as_str().unwrap_or("").to_string();
            let Ok(ip) = props(&c, it.as_str(), ITEM) else { continue };
            let a = attrs(&ip);
            // Who saved it: an app attribute or the schema (org.gnome.Epiphany..., chrome_libsecret...).
            let app = a.get("application").or(a.get("xdg:schema")).cloned().unwrap_or_default();
            let mut shown: Vec<(String, String)> = a.iter().filter(|(k, _)| !k.starts_with("xdg:")).map(|(k, v)| (k.clone(), v.clone())).collect();
            shown.sort();
            out.push(json!({"path": it.as_str(), "keyring": label, "label": s(&ip, "Label"), "app": app, "attributes": shown,
                "created": u(&ip, "Created"), "modified": u(&ip, "Modified"), "locked": is_locked}));
        }
        Ok(json!({"items": out, "keyrings": keyrings.into_values().collect::<Vec<_>>()}))
    }

    /// Waits for a keyring prompt (unlock or confirm a delete) to finish: false when it was dismissed.
    fn prompt(c: &Connection, path: &OwnedObjectPath) -> Result<bool, String> {
        if path.as_str() == "/" { return Ok(true); }
        let proxy = zbus::blocking::Proxy::new(c, DEST, path.as_str(), "org.freedesktop.Secret.Prompt").map_err(plain)?;
        let mut sig = proxy.receive_signal("Completed").map_err(plain)?;
        proxy.call_method("Prompt", &("",)).map_err(plain)?;
        let msg = sig.next().ok_or("The keyring's prompt went away.")?;
        let (dismissed, _): (bool, OwnedValue) = msg.body().deserialize().map_err(|e| e.to_string())?;
        Ok(!dismissed)
    }

    fn check_item(c: &Connection, item: &str) -> Result<(), String> {
        // Only items the keyring lists (no other object paths reach these calls).
        if !item.starts_with("/org/freedesktop/secrets/collection/") && !item.starts_with("/org/freedesktop/secrets/aliases/") { return Err("Unknown item.".into()); }
        props(c, item, ITEM).map(|_| ())
    }

    /// One item's secret (the page asked, after your password was checked).
    pub fn reveal(item: &str) -> Result<String, String> {
        let c = conn()?;
        check_item(&c, item)?;
        let path = ObjectPath::try_from(item).map_err(|_| "Unknown item.")?;
        let (_unlocked, p): (Vec<OwnedObjectPath>, OwnedObjectPath) = call(&c, ROOT, SERVICE, "Unlock", &(vec![path.clone()],))?;
        if !prompt(&c, &p)? { return Err("The keyring stayed locked.".into()); }
        let (_out, session): (OwnedValue, OwnedObjectPath) = call(&c, ROOT, SERVICE, "OpenSession", &("plain", V::from("")))?;
        // The reply is one struct (oayays): session, parameters, value, content type.
        let r: Result<(OwnedObjectPath, Vec<u8>, Vec<u8>, String), String> = call(&c, item, ITEM, "GetSecret", &(session.clone(),));
        let _ = c.call_method(Some(DEST), session.as_str(), Some("org.freedesktop.Secret.Session"), "Close", &());
        let (_, _, value, _) = r?;
        Ok(match String::from_utf8(value) { Ok(t) => t, Err(e) => format!("(not text: {} bytes)", e.into_bytes().len()) })
    }

    pub fn delete(item: &str) -> Result<(), String> {
        let c = conn()?;
        check_item(&c, item)?;
        let p: OwnedObjectPath = call(&c, item, ITEM, "Delete", &())?;
        if prompt(&c, &p)? { Ok(()) } else { Err("Cancelled.".into()) }
    }
}

#[cfg(target_os = "linux")]
pub fn keyring_list() -> Result<Value, String> { ss::list() }
#[cfg(not(target_os = "linux"))]
pub fn keyring_list() -> Result<Value, String> { Err("Not on this system yet.".into()) }

pub fn keyring_reveal(args: &Value) -> Result<Value, String> {
    confirm(args)?;
    #[cfg(target_os = "linux")]
    { Ok(json!({"secret": ss::reveal(util::arg(args, "item"))?})) }
    #[cfg(not(target_os = "linux"))]
    { Err("Not on this system yet.".into()) }
}

pub fn keyring_delete(args: &Value) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    { ss::delete(util::arg(args, "item")).map(|_| json!({"ok": true})) }
    #[cfg(not(target_os = "linux"))]
    { let _ = args; Err("Not on this system yet.".into()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn uuids() {
        assert!(super::valid_uuid("0f6a1c2e-3b4d-4e5f-8a9b-0c1d2e3f4a5b"));
        assert!(!super::valid_uuid("0f6a1c2e-3b4d-4e5f-8a9b-0c1d2e3f4a5"));
        assert!(!super::valid_uuid("--help-3b4d-4e5f-8a9b-0c1d2e3f4a5b00"));
        assert!(!super::valid_uuid("0f6a1c2e;3b4d-4e5f-8a9b-0c1d2e3f4a5b"));
    }
}
