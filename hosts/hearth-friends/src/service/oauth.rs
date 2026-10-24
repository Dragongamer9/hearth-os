//! Signing in and up on the server's own page, for servers with Matrix's newer sign-in (OAuth 2.0: MSC3861 and its
//! parts MSC2964-MSC2967; matrix.org runs the Matrix Authentication Service). matrix-sdk's OAuth API does the
//! discovery, the dynamic client registration, the authorization code flow with PKCE, restoring sessions, refreshing
//! tokens and revoking them. The device authorization grant (RFC 8628, "sign in on your phone") is done here over
//! HTTP: matrix-sdk keeps its own copy of it for logging in from another Matrix device (MSC4108).
//!
//!  * Phone (QR code): Hearth registers itself with the server, asks for a device code and shows the QR code, the
//!    short address and the code; the person signs in, or makes an account, on the server's page on their phone
//!    (captcha, email and terms included) and says yes; Hearth polls the token endpoint and is signed in.
//!  * This device: Hearth listens on http://127.0.0.1:<port>/callback (a loopback address, RFC 8252), the server's
//!    page opens in the browser (with prompt=create to make an account, when the server offers that) and Hearth
//!    finishes when the browser comes back. No typing the password into Hearth afterwards.
//!
//! The session (client id, user, device, access and refresh tokens) goes to the system keyring like password
//! sessions do, and refreshed tokens replace it there. The log gets the steps only: never a token, a code or an
//! address with a code in it.

use super::*;
use matrix_sdk::authentication::oauth::{registration::ClientMetadata, ClientId, ClientRegistrationData, OAuthSession, UserSession};
use matrix_sdk::encryption::recovery::RecoveryState;
use matrix_sdk::ruma::{api::client::discovery::get_authorization_server_metadata::v1::Prompt, serde::Raw, DeviceId, OwnedDeviceId};
use matrix_sdk::utils::UrlOrQuery;
use matrix_sdk::{SessionMeta, SessionTokens};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A sign-in that's waiting: for the phone (mode "qr") or for the browser to come back (mode "browser").
pub(super) struct Pending {
    pub(super) mode: &'static str,
    pub(super) homeserver: String,
    task: tokio::task::AbortHandle,
    /// The server's page, for "Open it again" (browser mode only; it's never logged).
    url: Option<String>,
}

/// How long a sign-in on the server's page may take (the device code usually expires sooner).
const PAGE_TIMEOUT: u64 = 900;

fn homeserver_arg(a: &Value) -> String {
    let mut hs = a["homeserver"].as_str().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(hf::DEFAULT_HOMESERVER).to_string();
    while hs.ends_with('/') { hs.pop(); }
    hs
}

fn metadata(redirect: Option<&str>) -> Result<Raw<ClientMetadata>, String> {
    Raw::from_json_string(hf::client_metadata(redirect).to_string()).map_err(e2s)
}

/// A client without a store, to look at a server (its address from .well-known, its sign-in options).
async fn probe(hs: &str) -> Result<Client, String> {
    Client::builder().server_name_or_homeserver_url(hs).build().await.map_err(|e| format!("Couldn't reach {hs}: {}", e2s(e)))
}

async fn post_form(url: &str, pairs: &[(&str, &str)]) -> Result<(u16, Value), String> {
    let r = matrix_sdk::reqwest::Client::new().post(url)
        .header("content-type", "application/x-www-form-urlencoded").header("accept", "application/json")
        .body(hf::form_body(pairs)).send().await.map_err(e2s)?;
    let code = r.status().as_u16();
    let v = serde_json::from_slice(&r.bytes().await.map_err(e2s)?).unwrap_or(json!({}));
    Ok((code, v))
}

fn oauth_error(v: &Value) -> String { hf::redact(v["error_description"].as_str().or(v["error"].as_str()).unwrap_or("no reason given")) }

fn not_offered(hs: &str, e: impl std::fmt::Display) -> String {
    let s = e2s(e);
    if s.contains("not supported") || s.contains("NotSupported") { format!("{hs} doesn't have sign-in on its own page. Use a user name and password.") } else { format!("Couldn't ask {hs} how to sign in: {s}") }
}

pub(super) fn recovery_name(c: &Client) -> &'static str {
    match c.encryption().recovery().state() {
        RecoveryState::Enabled => "enabled",
        RecoveryState::Disabled => "disabled",
        RecoveryState::Incomplete => "incomplete",
        RecoveryState::Unknown => "unknown",
    }
}

/// Keeps the OAuth session in the keyring: {"kind": "oauth", homeserver, clientId, session: user, device, tokens}.
fn save_session(paths: &Paths, c: &Client, hs: &str) -> bool {
    let Some(s) = c.oauth().full_session() else { return false };
    let v = json!({"kind": "oauth", "homeserver": hs, "clientId": s.client_id.as_str(), "session": s.user});
    secrets::set(paths, "session", &v.to_string()).is_ok()
}

impl Service {
    /// What a server offers: sign-in on its page (and whether by phone / QR code, and making an account there),
    /// user name and password, and sign-up from Hearth ("open", "token", "web" or "none"; OAuth servers: "oauth").
    pub async fn auth_methods(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        let hs = homeserver_arg(a);
        log(&format!("auth: looking at how {hs} signs people in"));
        let c = probe(&hs).await?;
        let meta = match c.oauth().server_metadata().await {
            Ok(m) => Some(m),
            Err(e) => { if !e.is_not_supported() { log(&format!("auth: {hs}: no OAuth 2.0 sign-in ({})", e2s(&e))); } None }
        };
        let http = matrix_sdk::reqwest::Client::new();
        let mut password = false;
        if let Ok(url) = c.homeserver().join("_matrix/client/v3/login") {
            if let Ok(r) = http.get(url).send().await {
                let v: Value = serde_json::from_slice(&r.bytes().await.unwrap_or_default()).unwrap_or(json!({}));
                password = v["flows"].as_array().map_or(false, |f| f.iter().any(|x| x["type"] == "m.login.password"));
            }
        }
        let register = if meta.is_some() { "oauth".to_string() } else {
            match c.homeserver().join("_matrix/client/v3/register") {
                Ok(url) => match http.post(url).header("content-type", "application/json").body("{}").send().await {
                    Ok(r) => { let code = r.status().as_u16(); let v: Value = serde_json::from_slice(&r.bytes().await.unwrap_or_default()).unwrap_or(json!({})); hf::register_kind(code, &v).to_string() }
                    Err(_) => "none".into(),
                },
                Err(_) => "none".into(),
            }
        };
        let (qr, create, account) = match &meta {
            Some(m) => (m.device_authorization_endpoint.is_some() && m.registration_endpoint.is_some(),
                        m.prompt_values_supported.iter().any(|p| p.as_str() == "create"),
                        m.account_management_uri.as_ref().map(|u| u.to_string())),
            None => (false, false, None),
        };
        let browser = meta.as_ref().map_or(false, |m| m.registration_endpoint.is_some());
        log(&format!("auth: {hs}: page sign-in {}, phone (QR) {}, make an account on the page {}, sign-up from Hearth {register}, passwords {password}",
                     meta.is_some(), qr, create));
        Ok(json!({"homeserver": hs, "oauth": meta.is_some(), "qr": qr, "browser": browser, "create": create, "password": password,
                  "register": register, "accountUrl": account, "issuer": meta.as_ref().map(|m| hf::short_url(m.issuer.as_str()))}))
    }

    /// Starts signing in (or up, `create`) on the server's page: `mode` "qr" (a phone) or "browser" (this device).
    pub async fn auth_start(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        if self.client.read().await.is_some() { return Err("You're signed in already. Sign out first.".into()); }
        let hs = homeserver_arg(a);
        let create = a["create"] == true;
        self.auth_cancel_quiet();
        self.sign_out_local().await;   // a fresh store for the new session
        match a["mode"].as_str().unwrap_or("qr") {
            "browser" => self.start_browser(&hs, create).await,
            _ => self.start_device(&hs, create).await,
        }
    }

    /// Drops a waiting sign-in (its task stops; a half-made store is removed at the next start).
    pub(super) fn auth_cancel_quiet(&self) {
        if let Some(p) = self.auth.lock().unwrap().take() { p.task.abort(); }
    }

    /// Opens the server's page again in the default browser (browser mode).
    pub fn auth_open(&self) -> Result<Value, String> {
        let url = self.auth.lock().unwrap().as_ref().and_then(|p| p.url.clone()).ok_or("Start signing in first.")?;
        let child = std::process::Command::new("xdg-open").arg(&url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
            .spawn().map_err(|e| format!("Couldn't open the browser ({e})."))?;
        std::thread::spawn(move || { let mut c = child; let _ = c.wait(); });
        log("auth: opened the server's sign-in page in the browser");
        Ok(json!({"opened": true}))
    }

    fn auth_failed(&self, state: &str, why: &str) {
        self.auth.lock().unwrap().take();
        log(&format!("auth: sign-in on the server's page ended: {state} ({why})"));
        self.st.lock().unwrap().error = Some(why.to_string());
        self.emit("auth", json!({"state": state, "why": why}));
    }

    // ------------------------------------------------------------------ a phone: the device authorization grant

    async fn start_device(self: &Arc<Self>, hs: &str, create: bool) -> Result<Value, String> {
        log(&format!("auth: sign-{} with a phone (QR code) on {hs}: discovery", if create { "up" } else { "in" }));
        let probe = probe(hs).await?;
        let meta = probe.oauth().server_metadata().await.map_err(|e| not_offered(hs, e))?;
        let endpoint = meta.device_authorization_endpoint.clone()
            .ok_or_else(|| format!("{hs} doesn't offer signing in from a phone yet. Use \"On this device\" instead."))?;
        let reg = probe.oauth().register_client(&metadata(None)?).await.map_err(|e| format!("{hs} didn't take Hearth as an app: {}", e2s(e)))?;
        let client_id = reg.client_id.as_str().to_string();
        log("auth: Hearth registered with the server (dynamic client registration)");
        let device_id: OwnedDeviceId = DeviceId::new();
        let scope = hf::oauth_scope(device_id.as_str());
        let (code, v) = post_form(endpoint.as_str(), &[("client_id", &client_id), ("scope", &scope)]).await?;
        if code != 200 { return Err(format!("{hs} didn't start the sign-in: {}", oauth_error(&v))); }
        let grant = hf::DeviceGrant::parse(&v).ok_or("The server's answer had no sign-in code in it.")?;
        log(&format!("auth: device code issued (good for {} s, checking every {} s); waiting for the phone", grant.expires_in, grant.interval));
        let link = grant.link().to_string();
        let out = json!({
            "mode": "qr", "homeserver": hs, "create": create, "url": link, "shortUrl": hf::short_url(&grant.uri), "userCode": grant.user_code,
            "qr": hf::qr_svg(&link), "qrRows": hf::qr_rows(&link), "expiresIn": grant.expires_in,
        });
        let (svc, hs2, token_url) = (self.clone(), hs.to_string(), meta.token_endpoint.to_string());
        let task = tokio::spawn(async move { svc.poll_device(hs2, client_id, device_id, token_url, grant).await });
        *self.auth.lock().unwrap() = Some(Pending { mode: "qr", homeserver: hs.to_string(), task: task.abort_handle(), url: None });
        Ok(out)
    }

    async fn poll_device(self: Arc<Self>, hs: String, client_id: String, device_id: OwnedDeviceId, token_url: String, g: hf::DeviceGrant) {
        let mut every = g.interval;
        let end = now_ms() + g.expires_in.min(PAGE_TIMEOUT) * 1000;
        let mut troubles = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(every)).await;
            if now_ms() > end { return self.auth_failed("expired", "The code ran out of time. Start again for a new one."); }
            let (code, v) = match post_form(&token_url, &[("grant_type", hf::DEVICE_GRANT), ("device_code", &g.device_code), ("client_id", &client_id)]).await {
                Ok(x) => { troubles = 0; x }
                Err(e) => { troubles += 1; if troubles == 3 { log(&format!("auth: can't reach the server to check the sign-in ({e}); still trying")); } continue; }
            };
            if code == 200 {
                let Some(access) = v["access_token"].as_str().filter(|t| !t.is_empty()) else { return self.auth_failed("error", "The server said yes but sent no session.") };
                log("auth: approved on the phone; finishing");
                let tokens = SessionTokens { access_token: access.to_string(), refresh_token: v["refresh_token"].as_str().map(str::to_string) };
                self.busy.store(true, Ordering::SeqCst);
                let r = self.finish_device(&hs, &client_id, device_id, tokens).await;
                self.busy.store(false, Ordering::SeqCst);
                if let Err(e) = r { self.auth_failed("error", &format!("Couldn't finish signing in: {e}")); }
                return;
            }
            match hf::device_poll(v["error"].as_str().unwrap_or("")) {
                hf::Poll::Wait => {}
                hf::Poll::SlowDown => { every = (every + 5).min(60); }
                hf::Poll::Expired => return self.auth_failed("expired", "The code ran out of time. Start again for a new one."),
                hf::Poll::Denied => return self.auth_failed("denied", "The sign-in was turned down on the other device."),
                hf::Poll::Failed => return self.auth_failed("error", &format!("The server stopped the sign-in: {}", oauth_error(&v))),
            }
        }
    }

    async fn finish_device(self: &Arc<Self>, hs: &str, client_id: &str, device_id: OwnedDeviceId, tokens: SessionTokens) -> Result<(), String> {
        let key = hf::random(48, b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
        let keyring_ok = secrets::set(&self.paths, "store-key", &key).is_ok();
        let c = self.build_with(hs, &key, true).await?;
        // Who signed in: whoami with the new token (the device id is the one Hearth asked for in the scope).
        let url = c.homeserver().join("_matrix/client/v3/account/whoami").map_err(e2s)?;
        let r = matrix_sdk::reqwest::Client::new().get(url).bearer_auth(&tokens.access_token).send().await.map_err(e2s)?;
        let code = r.status().as_u16();
        let v: Value = serde_json::from_slice(&r.bytes().await.map_err(e2s)?).unwrap_or(json!({}));
        if code != 200 { return Err(format!("the server didn't say who signed in ({})", hf::redact(v["errcode"].as_str().unwrap_or("?")))); }
        let user_id = UserId::parse(v["user_id"].as_str().unwrap_or("")).map_err(|_| "the server sent a strange user id".to_string())?;
        let session = OAuthSession { client_id: ClientId::new(client_id.to_string()), user: UserSession { meta: SessionMeta { user_id, device_id }, tokens } };
        c.oauth().restore_session(session, RoomLoadSettings::default()).await.map_err(e2s)?;
        self.auth.lock().unwrap().take();
        self.keep_oauth(c, hs, keyring_ok, "a phone (RFC 8628 device grant)").await;
        Ok(())
    }

    // ------------------------------------------------------------------ this device: authorization code + PKCE

    async fn start_browser(self: &Arc<Self>, hs: &str, create: bool) -> Result<Value, String> {
        log(&format!("auth: sign-{} in the browser on this device, on {hs}: discovery", if create { "up" } else { "in" }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.map_err(|e| format!("Couldn't get ready for the browser ({e})."))?;
        let port = listener.local_addr().map_err(e2s)?.port();
        let redirect = format!("http://127.0.0.1:{port}/callback");
        let key = hf::random(48, b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
        let keyring_ok = secrets::set(&self.paths, "store-key", &key).is_ok();
        let c = self.build_with(hs, &key, true).await?;
        let meta = c.oauth().server_metadata().await.map_err(|e| not_offered(hs, e))?;
        let can_create = meta.prompt_values_supported.iter().any(|p| p.as_str() == "create");
        let reg: ClientRegistrationData = metadata(Some(&redirect))?.into();
        let back = matrix_sdk::reqwest::Url::parse(&redirect).map_err(e2s)?;
        let mut b = c.oauth().login(back, None, Some(reg), None);
        if create && can_create { b = b.prompt(vec![Prompt::Create]); }
        let data = b.build().await.map_err(|e| format!("{hs} didn't start the sign-in: {}", e2s(e)))?;
        log(&format!("auth: Hearth registered with the server; the browser comes back to this device on port {port}{}",
                     if create && !can_create { " (the server has no separate sign-up page: its sign-in page has it)" } else { "" }));
        let url = data.url.to_string();
        let (svc, hs2) = (self.clone(), hs.to_string());
        let task = tokio::spawn(async move { svc.wait_for_browser(listener, c, hs2, keyring_ok).await });
        *self.auth.lock().unwrap() = Some(Pending { mode: "browser", homeserver: hs.to_string(), task: task.abort_handle(), url: Some(url.clone()) });
        Ok(json!({"mode": "browser", "homeserver": hs, "create": create && can_create, "url": url}))
    }

    async fn wait_for_browser(self: Arc<Self>, listener: tokio::net::TcpListener, c: Client, hs: String, keyring_ok: bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(PAGE_TIMEOUT);
        loop {
            let (mut sock, _) = match tokio::time::timeout_at(deadline, listener.accept()).await {
                Ok(Ok(x)) => x,
                Ok(Err(_)) => continue,
                Err(_) => return self.auth_failed("expired", "The sign-in page wasn't finished in time. Start again."),
            };
            let mut buf = vec![0u8; 8192];
            let mut n = 0;
            while n < buf.len() {
                match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf[n..])).await {
                    Ok(Ok(k)) if k > 0 => { n += k; if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") { break; } }
                    _ => break,
                }
            }
            let head = String::from_utf8_lossy(&buf[..n]).to_string();
            let Some(query) = hf::callback_query(&head) else {
                let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                continue;
            };
            log("auth: the browser came back to Hearth; finishing");
            let r = c.oauth().finish_login(UrlOrQuery::Query(query)).await;
            let page = hf::callback_page(r.is_ok());
            let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}", page.len());
            let _ = sock.write_all(reply.as_bytes()).await;
            let _ = sock.shutdown().await;
            match r {
                Ok(()) => {
                    self.auth.lock().unwrap().take();
                    self.busy.store(true, Ordering::SeqCst);
                    self.keep_oauth(c, &hs, keyring_ok, "the browser on this device (auth code with PKCE)").await;
                    self.busy.store(false, Ordering::SeqCst);
                }
                Err(e) => {
                    let s = e2s(e);
                    let why = if s.contains("access_denied") { "The sign-in was turned down on the page.".to_string() } else { format!("Couldn't finish signing in: {s}") };
                    self.auth_failed(if s.contains("access_denied") { "denied" } else { "error" }, &why);
                }
            }
            return;
        }
    }

    // ------------------------------------------------------------------ the session: kept, refreshed, restored

    /// Signed in with the server's page: the session goes to the keyring, and so do refreshed tokens later.
    async fn keep_oauth(self: &Arc<Self>, c: Client, hs: &str, keyring_ok: bool, how: &str) {
        let saved = keyring_ok && save_session(&self.paths, &c, hs);
        self.watch_refresh(&c, hs);
        self.st.lock().unwrap().keyring_ok = saved;
        let mut s = self.settings(); s.homeserver = hs.to_string(); s.recovery_asked = false; let _ = self.save_settings(s);
        log(&format!("auth: signed in on the server's page, from {how}{}", if saved { "; session kept in the keyring" } else { "; the keyring isn't available" }));
        self.signed_in(c, hs, true).await;
    }

    /// matrix-sdk renews the access token with the refresh token when it runs out; the new pair replaces the old in
    /// the keyring.
    fn watch_refresh(&self, c: &Client, hs: &str) {
        let (paths, hs) = (self.paths.clone(), hs.to_string());
        let reload = Box::new(|_c: Client| -> Result<SessionTokens, Box<dyn std::error::Error + Send + Sync>> { Err("not used (one Hearth Friends per user)".into()) });
        let save = Box::new(move |c: Client| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let ok = save_session(&paths, &c, &hs);
            log(if ok { "auth: renewed the access and refresh tokens; the new ones are in the keyring" } else { "auth: renewed the access and refresh tokens (couldn't keep them in the keyring)" });
            Ok(())
        });
        if let Err(e) = c.set_session_callbacks(reload, save) { log(&format!("auth: can't follow renewals of the tokens ({})", e2s(e))); }
    }

    pub(super) async fn restore_oauth(self: &Arc<Self>, v: &Value, hs: &str, key: &str) {
        let (Ok(user), Some(cid)) = (serde_json::from_value::<UserSession>(v["session"].clone()), v["clientId"].as_str()) else { return };
        let c = match self.build_with(hs, key, true).await { Ok(c) => c, Err(e) => { self.st.lock().unwrap().error = Some(e); return; } };
        match c.oauth().restore_session(OAuthSession { client_id: ClientId::new(cid.to_string()), user }, RoomLoadSettings::default()).await {
            Ok(()) => {
                self.watch_refresh(&c, hs);
                self.st.lock().unwrap().keyring_ok = true;
                log("session restored from the keyring (signed in on the server's page)");
                self.signed_in(c, hs, true).await;
            }
            Err(e) => { self.st.lock().unwrap().error = Some(format!("Couldn't open your Hearth Friends account: {}", e2s(e))); }
        }
    }

    // ------------------------------------------------------------------ end-to-end encryption: the recovery key

    pub async fn recovery(&self) -> Result<Value, String> {
        let c = self.client().await?;
        Ok(json!({"state": recovery_name(&c)}))
    }

    /// Makes a recovery key (and turns on the encrypted key backup): shown once, on the screen only.
    pub async fn recovery_enable(&self) -> Result<Value, String> {
        let c = self.client().await?;
        log("e2ee: setting up a recovery key and the key backup");
        let key = c.encryption().recovery().enable().await.map_err(|e| {
            let s = e2s(&e);
            if s.contains("BackupExistsOnServer") || s.contains("backup already exists") { "This account already has a key backup from another device. Use that device's recovery key instead.".to_string() } else { format!("Couldn't set up the recovery key: {s}") }
        })?;
        self.recovery_done().await;
        log("e2ee: recovery key made (shown on screen once, never kept by Hearth)");
        Ok(json!({"key": key, "state": recovery_name(&c)}))
    }

    /// Unlocks the account's secrets (cross-signing, key backup) with the recovery key made on another device.
    pub async fn recovery_restore(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let key = arg(a, "key")?.trim().to_string();
        log("e2ee: unlocking with a recovery key");
        c.encryption().recovery().recover(&key).await.map_err(|_| "That recovery key didn't work. Check it and try again.".to_string())?;
        self.recovery_done().await;
        log("e2ee: unlocked: this device can read older encrypted messages");
        Ok(json!({"state": recovery_name(&c)}))
    }

    pub async fn recovery_skip(&self) -> Result<Value, String> {
        self.recovery_done().await;
        log("e2ee: recovery key skipped for now");
        Ok(self.status_value().await)
    }

    /// The question was answered: the screens stop asking (until the next sign-in).
    async fn recovery_done(&self) {
        let mut s = self.settings();
        s.recovery_asked = true;
        let _ = self.save_settings(s);
        self.emit("status", self.status_value().await);
    }
}
