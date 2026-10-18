//! Remote play (Settings › Remote play): Sunshine streams this device to Moonlight; Hearth gives it a login
//! only it knows and pairs devices with Moonlight's 4-digit PIN. Moonlight to play another PC here.
//! Port of remote-play.js; talks to Sunshine's local web API with curl (its certificate is its own).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const UNITS: [&str; 2] = ["sunshine.service", "app-dev.lizardbyte.app.Sunshine.service"];
const MOONLIGHT: &str = "com.moonlight_stream.Moonlight";

fn creds_file() -> PathBuf { crate::paths::data_dir().join("remote-play.json") }
fn creds() -> Option<(String, String)> {
    let v: Value = std::fs::read(creds_file()).ok().and_then(|b| serde_json::from_slice(&b).ok())?;
    Some((v["user"].as_str()?.to_string(), v["pass"].as_str()?.to_string()))
}

/// One call to Sunshine's API. Err("AUTH") when it wants a username and password.
pub fn api(method: &str, path: &str, body: Option<&Value>, login: Option<(String, String)>) -> Result<Value, String> {
    let url = format!("https://127.0.0.1:47990{path}");
    let mut args: Vec<String> = vec!["-sk".into(), "-m".into(), "8".into(), "-X".into(), method.into(), "-H".into(), "Content-Type: application/json".into(), "-w".into(), "\n%{http_code}".into()];
    if let Some((u, p)) = &login { args.push("-u".into()); args.push(format!("{u}:{p}")); }
    if let Some(b) = body { args.push("-d".into()); args.push(b.to_string()); }
    args.push(url);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = crate::system::run("curl", &refs).map_err(|_| "Sunshine isn't running".to_string())?;
    let (text, code) = out.rsplit_once('\n').unwrap_or(("", "0"));
    match code.trim() {
        "401" => Err("AUTH".into()),
        c if c.starts_with('2') => Ok(serde_json::from_str(text).unwrap_or(json!({}))),
        "000" => Err("Sunshine isn't running".into()),
        c => Err(format!("Sunshine said {c}")),
    }
}

fn unit() -> Option<&'static str> {
    UNITS.iter().copied().find(|u| crate::system::run("systemctl", &["--user", "list-unit-files", u, "--no-legend"]).map(|o| !o.trim().is_empty()).unwrap_or(false))
}
fn running() -> bool { unit().map(|u| crate::system::run("systemctl", &["--user", "is-active", u]).map(|o| o.trim() == "active").unwrap_or(false)).unwrap_or(false) }

pub fn moonlight_installed() -> bool { crate::system::has("flatpak") && crate::system::run("flatpak", &["info", MOONLIGHT]).is_ok() }

pub fn addresses() -> Vec<String> {
    crate::system::run("ip", &["-4", "-o", "addr", "show", "scope", "global"]).unwrap_or_default().lines()
        .filter_map(|l| { let mut w = l.split_whitespace(); let iface = w.nth(1)?; let addr = w.nth(1)?; if ["docker", "veth", "virbr", "br-"].iter().any(|p| iface.starts_with(p)) { None } else { addr.split('/').next().map(str::to_string) } })
        .collect()
}

pub fn status() -> Value {
    let on = running();
    let (mut clients, mut needs_login) = (vec![], false);
    if on {
        match api("GET", "/api/clients/list", None, creds()) {
            Ok(v) => clients = v["named_certs"].as_array().cloned().unwrap_or_default().into_iter().map(|c| json!({"name": c["name"], "uuid": c["uuid"]})).collect(),
            Err(e) if e == "AUTH" => needs_login = true,
            Err(_) => {}
        }
    }
    json!({"installed": unit().is_some() || crate::system::has("sunshine"), "on": on, "clients": clients, "needsLogin": needs_login,
           "addresses": addresses(), "moonlight": moonlight_installed()})
}

pub fn ensure_login() -> bool {
    if creds().is_some() { return true; }
    let mut b = [0u8; 18];
    if getrandom::getrandom(&mut b).is_err() { return false; }
    let pass: String = b.iter().map(|x| format!("{x:02x}")).collect();
    match api("POST", "/api/password", Some(&json!({"newUsername": "hearth", "newPassword": pass, "confirmNewPassword": pass})), None) {
        Ok(v) if v["status"] != false => save_creds("hearth", &pass).is_ok(),
        _ => false,
    }
}
fn save_creds(user: &str, pass: &str) -> Result<(), String> {
    let f = creds_file();
    crate::paths::write_atomic(&f, json!({"user": user, "pass": pass}).to_string().as_bytes()).map_err(|e| e.to_string())?;
    let _ = crate::system::run("chmod", &["600", &f.to_string_lossy()]);
    Ok(())
}

pub fn set_on(on: bool) -> Result<Value, String> {
    let u = unit().ok_or("Sunshine isn't on this system. It comes with Hearth OS.")?;
    crate::system::run_timeout("systemctl", &["--user", if on { "enable" } else { "disable" }, "--now", u], std::time::Duration::from_secs(30))?;
    if on {
        for _ in 0..30 { if matches!(api("GET", "/api/clients/list", None, None), Ok(_)) || matches!(api("GET", "/api/clients/list", None, None), Err(ref e) if e == "AUTH") { break; } std::thread::sleep(std::time::Duration::from_millis(500)); }
        ensure_login();
    }
    Ok(status())
}
pub fn login(user: &str, pass: &str) -> Result<bool, String> {
    if user.trim().is_empty() || pass.is_empty() { return Err("Type the username and password".into()); }
    api("GET", "/api/clients/list", None, Some((user.trim().into(), pass.into()))).map_err(|e| if e == "AUTH" { "Sunshine wants its username and password".to_string() } else { e })?;
    save_creds(user.trim(), pass)?;
    Ok(true)
}
pub fn pair(pin: &str, name: &str) -> Result<bool, String> {
    if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) { return Err("The PIN is the 4 numbers Moonlight shows".into()); }
    ensure_login();
    let r = api("POST", "/api/pin", Some(&json!({"pin": pin, "name": name.chars().take(40).collect::<String>()})), creds())?;
    if r["status"] == false { return Err("That PIN didn't work. In Moonlight, pick this device again for a new one.".into()); }
    Ok(true)
}
pub fn unpair(uuid: &str) -> Result<bool, String> { api("POST", "/api/clients/unpair", Some(&json!({"uuid": uuid})), creds()).map(|_| true) }
pub fn open_moonlight() -> bool { std::process::Command::new("flatpak").args(["run", MOONLIGHT]).spawn().is_ok() }
pub fn install_moonlight() -> Result<bool, String> {
    if !crate::system::has("flatpak") { return Err("Flatpak is needed to install Moonlight".into()); }
    crate::system::run_timeout("flatpak", &["install", "--user", "-y", "--noninteractive", "flathub", MOONLIGHT], std::time::Duration::from_secs(900)).map(|_| true)
}
#[allow(dead_code)]
fn _unused(_: &Path) {}
