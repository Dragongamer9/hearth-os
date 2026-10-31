//! The accounts Hearth Mods signs in to besides Nexus Mods (nexus.rs): LoversLab, MEGA and Google (Drive), so the
//! Wabbajack engine (or Hearth itself) downloads their files without clicks. Each sign-in is kept in its own file in
//! ~/.config/nimbus (loverslab.json, mega.json, google.json), created readable by you only (mode 600), and goes to
//! the engine in its environment (never on a command line, never in a log).
//!
//! - **LoversLab**: jackify-engine has Wabbajack's LoversLab downloader (api.loverslab.com, OAuth 2, scope
//!   "downloads"). It reads the sign-in from its environment variable LOVERS_LAB (EncryptedJsonTokenProvider
//!   "lovers-lab"). LoversLab's OAuth app for that is Wabbajack's own; Hearth doesn't borrow it. Hearth signs in with
//!   its own LoversLab OAuth client (authorization code + PKCE, your browser, a loopback address on this PC), which
//!   needs Hearth registered with LoversLab: until a client ID is set (oauth_client), the section says so and the
//!   guided browser flow (guided.rs) fetches LoversLab files.
//! - **MEGA**: the engine logs in with MegaApiClient's AuthInfos (email, password hash and password key), read from
//!   MEGA_LOGIN; `{"login":null}` logs in anonymously. Hearth works the AuthInfos out from the email and password you
//!   type (MEGA's v2 accounts: PBKDF2-HMAC-SHA512 with the salt MEGA gives for the email), checks them with MEGA and
//!   keeps only those, never the password. Not signed in, Hearth gives the engine the anonymous login, so MEGA files
//!   download anyway within MEGA's free transfer quota; signed in, your account's quota applies.
//! - **Google**: the engine downloads public Google Drive files without an account (and Wabbajack's CDN mirrors);
//!   it has no Google sign-in. When Google says "quota exceeded", Hearth downloads the file itself through the Drive
//!   API with your Google account (OAuth, PKCE, loopback), which needs Hearth's own Google OAuth client (oauth_client).

use crate::util;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const PROVIDERS: [&str; 3] = ["loverslab", "mega", "google"];

fn file_of(p: &str) -> PathBuf { util::config_dir().join(format!("{p}.json")) }

/// Writes a sign-in file readable by you only: the temporary file is made with mode 600 before anything is written
/// to it, then renamed over the old one.
pub fn write_secret(path: &Path, v: &Value) -> Result<(), String> {
    let dir = path.parent().ok_or("no folder")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = path.with_file_name(format!(".{}.{}.tmp", path.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
    let r = (|| -> std::io::Result<()> {
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; o.mode(0o600); }
        let mut f = o.open(&tmp)?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; f.set_permissions(std::fs::Permissions::from_mode(0o600))?; }
        f.write_all(serde_json::to_vec_pretty(v).unwrap_or_default().as_slice())?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() { let _ = std::fs::remove_file(&tmp); }
    r.map_err(|e| format!("Couldn't keep the sign-in ({e})"))
}
fn load(p: &str) -> Option<Value> { let v = util::read_json(&file_of(p)); if v.is_object() { Some(v) } else { None } }
fn save(p: &str, v: &Value) -> Result<(), String> { write_secret(&file_of(p), v) }
pub fn sign_out(p: &str) { if PROVIDERS.contains(&p) { let _ = std::fs::remove_file(file_of(p)); } }

// ------------------------------------------------------------------ OAuth clients

/// Hearth's own OAuth client for a site: (client id, client secret). None until Hearth is registered with the site.
/// From the build (HEARTH_LOVERSLAB_CLIENT_ID, HEARTH_GOOGLE_CLIENT_ID / _SECRET), or oauth-clients.json in
/// ~/.config/nimbus or /usr/share/hearth (so it can be tried on a device before it's built in):
/// {"loverslab": {"clientId": "..."}, "google": {"clientId": "...", "clientSecret": "..."}}
pub fn oauth_client(p: &str) -> Option<(String, String)> {
    let built = match p {
        "loverslab" => option_env!("HEARTH_LOVERSLAB_CLIENT_ID").map(|i| (i.to_string(), String::new())),
        "google" => option_env!("HEARTH_GOOGLE_CLIENT_ID").map(|i| (i.to_string(), option_env!("HEARTH_GOOGLE_CLIENT_SECRET").unwrap_or("").to_string())),
        _ => None,
    };
    if let Some(c) = built.filter(|c| !c.0.is_empty()) { return Some(c); }
    for f in [util::config_dir().join("oauth-clients.json"), PathBuf::from("/usr/share/hearth/oauth-clients.json")] {
        let v = util::read_json(&f);
        if let Some(id) = v[p]["clientId"].as_str().filter(|s| !s.is_empty()) { return Some((id.to_string(), v[p]["clientSecret"].as_str().unwrap_or("").to_string())); }
    }
    None
}

struct Site { auth: &'static str, token: &'static str, scope: &'static str, port: u16 }
fn site(p: &str) -> Option<Site> {
    match p {
        // The endpoints jackify-engine's LoversLabLoginState uses. A fixed port: IPS4 wants the exact redirect address.
        "loverslab" => Some(Site { auth: "https://api.loverslab.com/oauth/authorize/", token: "https://api.loverslab.com/oauth/token/", scope: "downloads", port: 47165 }),
        // Google's installed-app flow takes any loopback port.
        "google" => Some(Site { auth: "https://accounts.google.com/o/oauth2/v2/auth", token: "https://oauth2.googleapis.com/token", scope: "https://www.googleapis.com/auth/drive.readonly", port: 0 }),
        _ => None,
    }
}
pub fn redirect_uri(p: &str, port: u16) -> String { format!("http://127.0.0.1:{port}/hearth-{p}") }

fn random_bytes(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let t = util::now_ms() ^ (std::process::id() as u64) << 32;
        for (i, x) in b.iter_mut().enumerate() { *x = (t >> ((i % 8) * 8)) as u8 ^ i as u8; }
    }
    b
}
fn b64url(b: &[u8]) -> String { use base64::Engine; base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b) }
/// PKCE (RFC 7636): the verifier and its S256 challenge.
pub fn pkce() -> (String, String) {
    use sha2::Digest;
    let verifier = b64url(&random_bytes(32));
    let challenge = b64url(&sha2::Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}
pub fn urlencode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}
fn query_value(q: &str, k: &str) -> Option<String> {
    q.split('&').find_map(|kv| kv.split_once('=').filter(|(a, _)| *a == k).map(|(_, v)| crate::nexus::urldecode(&v.replace('+', " "))))
}

/// Sign-ins waiting for the browser: provider → "waiting" / an error / "done".
static PENDING: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

/// Starts signing in to LoversLab or Google: opens the site's sign-in page in your browser and waits (5 minutes) on
/// a loopback address for the site to send you back with a code, which is traded for the token.
pub fn oauth_begin(p: &str) -> Result<Value, String> {
    let s = site(p).ok_or("Unknown account.")?;
    let (client, secret) = oauth_client(p).ok_or_else(|| format!("Hearth isn't registered with {} yet, so it can't sign in there. Files from there are fetched in Hearth Browser instead.", label(p)))?;
    let listener = std::net::TcpListener::bind(("127.0.0.1", s.port)).map_err(|e| format!("Couldn't wait for the sign-in on this PC ({e})"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = redirect_uri(p, port);
    let (verifier, challenge) = pkce();
    let state = b64url(&random_bytes(16));
    let mut url = format!("{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        s.auth, urlencode(&client), urlencode(&redirect), urlencode(s.scope), state, challenge);
    if p == "google" { url.push_str("&access_type=offline&prompt=consent"); }
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).insert(p.into(), "waiting".into());
    let p2 = p.to_string();
    std::thread::spawn(move || {
        let r = wait_for_code(&listener, &state, &p2).and_then(|code| {
            let mut form = format!("grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}", urlencode(&code), urlencode(&redirect), urlencode(&client), urlencode(&verifier));
            if !secret.is_empty() { form.push_str(&format!("&client_secret={}", urlencode(&secret))); }
            token_call(&p2, &form, None)
        });
        PENDING.lock().unwrap_or_else(|e| e.into_inner()).insert(p2, match r { Ok(_) => "done".into(), Err(e) => e });
    });
    crate::guided::open_in_browser(&url)?;
    Ok(status(p))
}

fn wait_for_code(l: &std::net::TcpListener, state: &str, p: &str) -> Result<String, String> {
    l.set_nonblocking(true).map_err(|e| e.to_string())?;
    let until = util::now_ms() + 5 * 60_000;
    while util::now_ms() < until {
        match l.accept() {
            Ok((mut s, _)) => {
                let _ = s.set_nonblocking(false);
                let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                let mut buf = [0u8; 8192];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
                let Some(q) = path.strip_prefix(&format!("/hearth-{p}?")) else { let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"); continue };
                let ok = query_value(q, "state").as_deref() == Some(state);
                let code = query_value(q, "code").filter(|_| ok);
                let msg = if code.is_some() { "Signed in. You can close this tab and go back to Hearth Mods." } else { "The sign-in didn't finish. Go back to Hearth Mods and try again." };
                let page = format!("<!doctype html><meta charset=utf-8><title>Hearth Mods</title><body style=\"font:20px sans-serif;background:#0f0b09;color:#eee;padding:3em\">{msg}</body>");
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}", page.len()).as_bytes());
                return code.ok_or_else(|| query_value(q, "error_description").or_else(|| query_value(q, "error")).map(|e| format!("{} said: {e}", label(p))).unwrap_or_else(|| "The sign-in didn't finish.".into()));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(std::time::Duration::from_millis(200)),
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("The sign-in took too long (5 minutes). Try again.".into())
}

/// A POST through curl with everything (address, headers, body) on curl's standard input. (status, body)
pub fn curl_post(url: &str, headers: &[(&str, &str)], body: &str, timeout_s: u32) -> Result<(u16, String), String> {
    let q = crate::nexus::curl_quote;
    let mut cfg = format!("url = {}\ndata-binary = {}\n", q(url), q(body));
    for (k, v) in headers { cfg.push_str(&format!("header = {}\n", q(&format!("{k}: {v}")))); }
    let mut child = std::process::Command::new("curl")
        .args(["-sS", "--proto", "=https", "-m", &timeout_s.to_string(), "-A", concat!("Hearth/", env!("CARGO_PKG_VERSION")), "-w", "\n%{http_code}", "-K", "-"])
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e| format!("No connection (curl: {e})"))?;
    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(cfg.as_bytes()); }
    let out = child.wait_with_output().map_err(|e| format!("No connection ({e})"))?;
    let all = String::from_utf8_lossy(&out.stdout).to_string();
    let (body, code) = all.rsplit_once('\n').unwrap_or((&all, "0"));
    let status: u16 = code.trim().parse().unwrap_or(0);
    if status == 0 { return Err("No connection. Is this PC online?".into()); }
    Ok((status, body.to_string()))
}

/// Trades a code (or a refresh token) for tokens and keeps them.
fn token_call(p: &str, form: &str, keep_refresh: Option<&str>) -> Result<Value, String> {
    let s = site(p).ok_or("Unknown account.")?;
    let (st, body) = curl_post(s.token, &[("Content-Type", "application/x-www-form-urlencoded"), ("Accept", "application/json")], form, 30)?;
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let access = v["access_token"].as_str().unwrap_or("");
    if !(200..300).contains(&st) || access.is_empty() {
        return Err(format!("{} didn't accept the sign-in ({st}{}).", label(p), v["error"].as_str().map(|e| format!(": {e}")).unwrap_or_default()));
    }
    let refresh = v["refresh_token"].as_str().or(keep_refresh).unwrap_or("");
    let rec = token_record(access, refresh, v["expires_in"].as_u64().unwrap_or(3600), v["scope"].as_str().unwrap_or(s.scope), util::now_ms());
    save(p, &rec)?;
    Ok(rec)
}
pub fn token_record(access: &str, refresh: &str, expires_in: u64, scope: &str, now: u64) -> Value {
    json!({"accessToken": access, "refreshToken": refresh, "expiresAt": now + expires_in * 1000, "scope": scope, "signedIn": now})
}

/// The token, renewed first when it runs out within 10 minutes (and the site gave a refresh token).
pub fn token(p: &str) -> Option<Value> {
    let rec = load(p)?;
    rec["accessToken"].as_str().filter(|s| !s.is_empty())?;
    let left = rec["expiresAt"].as_u64().unwrap_or(0).saturating_sub(util::now_ms());
    let refresh = rec["refreshToken"].as_str().unwrap_or("").to_string();
    if left > 10 * 60_000 || refresh.is_empty() { return Some(rec); }
    let (client, secret) = oauth_client(p)?;
    let mut form = format!("grant_type=refresh_token&refresh_token={}&client_id={}", urlencode(&refresh), urlencode(&client));
    if !secret.is_empty() { form.push_str(&format!("&client_secret={}", urlencode(&secret))); }
    token_call(p, &form, Some(&refresh)).ok().or(if left > 0 { Some(rec) } else { None })
}

// ------------------------------------------------------------------ MEGA

const MEGA_API: &str = "https://g.api.mega.co.nz/cs";

/// MegaApiClient's AuthInfos for a v2 MEGA account: PBKDF2-HMAC-SHA512(password, salt, 100000) → the first 16 bytes
/// are the password key, the last 16 (base64url) the hash MEGA checks.
pub fn mega_auth_v2(password: &str, salt: &[u8]) -> (Vec<u8>, String) {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha512>(password.as_bytes(), salt, 100_000, &mut out);
    (out[..16].to_vec(), b64url(&out[16..]))
}
fn mega_call(body: &Value) -> Result<Value, String> {
    let (st, text) = curl_post(&format!("{MEGA_API}?id={}", util::now_ms() % 1_000_000), &[("Content-Type", "application/json")], &body.to_string(), 30)?;
    if !(200..300).contains(&st) { return Err(format!("MEGA had a problem ({st}).")); }
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    Ok(v.as_array().and_then(|a| a.first()).cloned().unwrap_or(v))
}

/// Signs in to MEGA: works out the login from your email and password, checks it with MEGA, keeps only the login
/// (not the password).
pub fn mega_sign_in(email: &str, password: &str) -> Result<Value, String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.contains(char::is_whitespace) { return Err("That doesn't look like an email address.".into()); }
    if password.is_empty() { return Err("Type your MEGA password.".into()); }
    let pre = mega_call(&json!([{"a": "us0", "user": email}]))?;
    if pre.as_i64().is_some() { return Err("MEGA doesn't know that email address.".into()); }
    if pre["v"].as_i64() != Some(2) { return Err("This is an older MEGA account (made before 2018), which Hearth can't sign in to yet. Changing its password on mega.nz upgrades it.".into()); }
    use base64::Engine;
    let salt = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(pre["s"].as_str().unwrap_or("").trim_end_matches('=')).map_err(|_| "MEGA's reply was odd. Try again.")?;
    let (key, hash) = mega_auth_v2(password, &salt);
    let r = mega_call(&json!([{"a": "us", "user": email, "uh": hash}]))?;
    match r.as_i64() {
        Some(-26) => return Err("This MEGA account uses two-step sign-in, which the Wabbajack engine can't use. MEGA files still download without an account.".into()),
        Some(-9) | Some(-2) => return Err("MEGA didn't accept that email and password.".into()),
        Some(n) if n < 0 => return Err(format!("MEGA said no ({n}).")),
        _ => {}
    }
    let rec = json!({"email": email, "hash": hash, "passwordAesKey": base64::engine::general_purpose::STANDARD.encode(&key), "signedIn": util::now_ms()});
    save("mega", &rec)?;
    Ok(status("mega"))
}

/// What the engine reads from MEGA_LOGIN: MegaToken {"login": AuthInfos}, or anonymous.
pub fn mega_engine_value(rec: Option<&Value>) -> String {
    match rec {
        Some(r) if r["hash"].is_string() => json!({"login": {"Email": r["email"], "Hash": r["hash"], "PasswordAesKey": r["passwordAesKey"], "MFAKey": null}}).to_string(),
        _ => json!({"login": null}).to_string(),
    }
}
/// What the engine reads from LOVERS_LAB: LoversLabLoginState.
pub fn loverslab_engine_value(rec: &Value, now: u64) -> String {
    json!({"result_state": {"access_token": rec["accessToken"], "token_type": "Bearer", "expires_in": rec["expiresAt"].as_u64().unwrap_or(0).saturating_sub(now) / 1000,
           "refresh_token": rec["refreshToken"], "scope": rec["scope"]}, "cookies": []}).to_string()
}

/// The environment the engine gets for these accounts.
pub fn engine_env() -> Vec<(&'static str, String)> {
    let mut env = vec![("MEGA_LOGIN", mega_engine_value(load("mega").as_ref()))];
    if let Some(t) = token("loverslab") { env.push(("LOVERS_LAB", loverslab_engine_value(&t, util::now_ms()))); }
    env
}
pub fn signed_in(p: &str) -> bool { load(p).is_some() }

// ------------------------------------------------------------------ Google Drive, by Hearth

/// Downloads a Google Drive file with your Google account (Drive API) to `dest`. For files the engine couldn't get
/// (Google's "quota exceeded" for files downloaded without an account).
pub fn google_download(file_id: &str, dest: &Path) -> Result<(), String> {
    if file_id.is_empty() || !file_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') { return Err("That isn't a Google Drive file.".into()); }
    let t = token("google").ok_or("Sign in to Google first.")?;
    let access = t["accessToken"].as_str().unwrap_or("");
    let q = crate::nexus::curl_quote;
    let cfg = format!("url = {}\nheader = {}\n", q(&format!("https://www.googleapis.com/drive/v3/files/{file_id}?alt=media&acknowledgeAbuse=true&supportsAllDrives=true")), q(&format!("Authorization: Bearer {access}")));
    let part = dest.with_file_name(format!("{}.hearth-part", dest.file_name().unwrap_or_default().to_string_lossy()));
    if let Some(d) = dest.parent() { let _ = std::fs::create_dir_all(d); }
    let mut child = std::process::Command::new("curl").args(["-fsSL", "--proto", "=https", "--retry", "2", "-C", "-", "-o"]).arg(&part).args(["-K", "-"])
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| e.to_string())?;
    if let Some(mut i) = child.stdin.take() { let _ = i.write_all(cfg.as_bytes()); }
    let st = child.wait().map_err(|e| e.to_string())?;
    if !st.success() { return Err("Google Drive didn't give the file with your account either.".into()); }
    std::fs::rename(&part, dest).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ for the page

pub fn label(p: &str) -> &'static str { match p { "loverslab" => "LoversLab", "mega" => "MEGA", "google" => "Google", _ => "that site" } }

/// One account for the page: never the token.
pub fn status(p: &str) -> Value {
    let rec = load(p);
    let pending = PENDING.lock().unwrap_or_else(|e| e.into_inner()).get(p).cloned().unwrap_or_default();
    let registered = match p { "mega" => true, _ => oauth_client(p).is_some() };
    json!({"id": p, "name": label(p), "signedIn": rec.is_some(), "registered": registered,
           "who": rec.as_ref().and_then(|r| r["email"].as_str()).unwrap_or(""), "since": rec.as_ref().and_then(|r| r["signedIn"].as_u64()),
           "pending": if pending == "done" { "" } else { pending.as_str() }})
}
pub fn all() -> Value { json!(PROVIDERS.iter().map(|p| status(p)).collect::<Vec<_>>()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_ins_are_yours_only() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("nimbus").join("loverslab.json");
        write_secret(&f, &token_record("a-token", "r", 3600, "downloads", 1000)).unwrap();
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o600);
            // Written again over a file someone opened up: still 600.
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
            write_secret(&f, &json!({"x": 1})).unwrap();
            assert_eq!(std::fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert!(std::fs::read_dir(f.parent().unwrap()).unwrap().count() == 1, "no temporary files left");
    }

    #[test]
    fn what_the_engine_gets() {
        let rec = token_record("acc", "ref", 3600, "downloads", 1_000_000);
        let v: Value = serde_json::from_str(&loverslab_engine_value(&rec, 1_000_000)).unwrap();
        assert_eq!(v["result_state"]["access_token"], "acc");
        assert_eq!(v["result_state"]["expires_in"], 3600);
        assert!(v["cookies"].as_array().unwrap().is_empty());
        let anon: Value = serde_json::from_str(&mega_engine_value(None)).unwrap();
        assert!(anon["login"].is_null());
        let m: Value = serde_json::from_str(&mega_engine_value(Some(&json!({"email": "a@b.c", "hash": "h", "passwordAesKey": "k"})))).unwrap();
        assert_eq!(m["login"]["Email"], "a@b.c");
        assert_eq!(m["login"]["PasswordAesKey"], "k");
    }

    #[test]
    fn mega_key_derivation() {
        // PBKDF2-HMAC-SHA512("password", "salt", 1) starts 867f70cf1ade02cf f3752599a3a53dc4 (published test vector);
        // the 100000-round MEGA form uses the same function.
        let mut out = [0u8; 16];
        pbkdf2::pbkdf2_hmac::<sha2::Sha512>(b"password", b"salt", 1, &mut out);
        assert_eq!(out.iter().map(|b| format!("{b:02x}")).collect::<String>(), "867f70cf1ade02cff3752599a3a53dc4");
        let (k, h) = mega_auth_v2("pw", b"0123456789abcdef0123456789abcdef");
        assert_eq!(k.len(), 16);
        assert_eq!(h.len(), 22);
    }

    #[test]
    fn oauth_pieces() {
        let (v, c) = pkce();
        assert!(v.len() >= 43 && c.len() == 43 && v != c);
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
        assert_eq!(query_value("code=x%2Fy&state=s", "code").as_deref(), Some("x/y"));
        assert_eq!(redirect_uri("loverslab", 47165), "http://127.0.0.1:47165/hearth-loverslab");
    }
}
