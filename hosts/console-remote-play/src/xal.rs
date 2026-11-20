//! Signing in, the way Greenlight's xal-node does it (MIT; docs/CONSOLE-REMOTE-PLAY.md, "Sign-in"):
//!
//! 1. a P-256 key and a device token (every Xbox Live request signed with the key);
//! 2. SISU authenticate: the address of Microsoft's own sign-in page (PKCE S256, a random state);
//! 3. the page, in the sign-in window (helper/hearth-console-remote-play-signin), ends at
//!    `ms-xal-000000004c20a908://auth?code=…&state=…`, which the window catches and hands back;
//! 4. the code for the user token (refresh token), then SISU authorize (title, user and device tokens, the gamertag);
//! 5. XSTS for streaming and for the console list, then the streaming tokens (xhome, and xgpuweb for Cloud).
//!
//! Only the refresh token and the key are kept (in the keyring, keyring.rs); everything else lives in memory.
//! Unofficial: these are the Xbox app's own services, and this is the Xbox app's sign-in id (as in xal-node).

use crate::http::{self, Fail, Req};
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const APP_ID: &str = "000000004c20a908";
pub const TITLE_ID: &str = "328178078";
pub const REDIRECT: &str = "ms-xal-000000004c20a908://auth";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

// ------------------------------------------------------------------ the key and the signature
pub struct Keys {
    sk: SigningKey,
}

impl Keys {
    pub fn new() -> Keys {
        loop {
            if let Ok(sk) = SigningKey::from_slice(&crate::random(32)) {
                return Keys { sk };
            }
        }
    }
    pub fn from_b64(s: &str) -> Option<Keys> {
        SigningKey::from_slice(&crate::unb64(s)?).ok().map(|sk| Keys { sk })
    }
    pub fn to_b64(&self) -> String {
        crate::b64(&self.sk.to_bytes())
    }
    fn xy(&self) -> (String, String) {
        use p256::elliptic_curve::sec1::ToEncodedPoint;
        let p = p256::PublicKey::from(self.sk.verifying_key()).to_encoded_point(false);
        (crate::b64url(p.x().map(|x| x.as_slice()).unwrap_or(&[])), crate::b64url(p.y().map(|y| y.as_slice()).unwrap_or(&[])))
    }
    pub fn proof_key(&self) -> Value {
        let (x, y) = self.xy();
        json!({"use": "sig", "alg": "ES256", "kty": "EC", "crv": "P-256", "x": x, "y": y})
    }
    /// The `Signature` header for a POST to `url` (xal-node's sign(), from prismarine-auth).
    pub fn signature(&self, url: &str, auth: &str, body: &str) -> String {
        self.signature_at(url, auth, body, windows_time(crate::now()))
    }
    pub fn signature_at(&self, url: &str, auth: &str, body: &str, ts: u64) -> String {
        let buf = signed_bytes(&path_of(url), auth, body, ts);
        let sig: Signature = self.sk.sign(&buf);
        let mut h = Vec::with_capacity(76);
        h.extend_from_slice(&1i32.to_be_bytes());
        h.extend_from_slice(&ts.to_be_bytes());
        h.extend_from_slice(&sig.to_bytes());
        crate::b64(&h)
    }
    #[cfg(test)]
    pub fn verifying(&self) -> p256::ecdsa::VerifyingKey {
        self.sk.verifying_key().clone()
    }
}

impl Default for Keys {
    fn default() -> Self {
        Keys::new()
    }
}

/// Windows file time: 100 ns steps since 1601.
pub fn windows_time(unix_secs: u64) -> u64 {
    (unix_secs + 11_644_473_600) * 10_000_000
}

pub fn path_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let p = rest.find('/').map(|i| &rest[i..]).unwrap_or("/");
    p.split('?').next().unwrap_or(p).to_string()
}

/// What is signed: version 1, a 0, the time, a 0, then "POST", the path, the auth header and the body, each ending in 0.
pub fn signed_bytes(path: &str, auth: &str, body: &str, ts: u64) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&1i32.to_be_bytes());
    b.push(0);
    b.extend_from_slice(&ts.to_be_bytes());
    b.push(0);
    for part in ["POST", path, auth, body] {
        b.extend_from_slice(part.as_bytes());
        b.push(0);
    }
    b
}

// ------------------------------------------------------------------ PKCE
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

pub fn pkce() -> Pkce {
    let verifier = crate::b64url(&crate::random(32));
    Pkce { challenge: challenge_of(&verifier), verifier }
}

pub fn challenge_of(verifier: &str) -> String {
    crate::b64url(&Sha256::digest(verifier.as_bytes()))
}

// ------------------------------------------------------------------ the requests
fn xbl_post(keys: &Keys, url: &str, body: &Value) -> Result<Value, Fail> {
    let text = body.to_string();
    let r = Req::new("POST", url)
        .header("x-xbl-contract-version", "1")
        .header("Cache-Control", "no-store, must-revalidate, no-cache")
        .header("Signature", &keys.signature(url, "", &text));
    let mut r = r;
    r.body = Some(text);
    r.headers.push(("Content-Type".into(), "application/json".into()));
    Ok(http::send(&r)?.json())
}

fn device_id() -> String {
    format!("{{{}}}", crate::uuid())
}

/// A device token (ProofOfPossession, as an Android phone, like xal-node).
pub fn device_token(keys: &Keys) -> Result<String, Fail> {
    let body = json!({
        "Properties": {"AuthMethod": "ProofOfPossession", "Id": device_id(), "DeviceType": "Android", "SerialNumber": device_id(), "Version": "15.0", "ProofKey": keys.proof_key()},
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });
    let v = xbl_post(keys, "https://device.auth.xboxlive.com/device/authenticate", &body)?;
    v["Token"].as_str().map(String::from).ok_or_else(|| Fail::Other("Microsoft's answer had no device token".into()))
}

/// A sign-in that's begun: the page to open, and what's needed to finish it (kept in memory only).
pub struct Begun {
    pub keys: Keys,
    pub page: String,
    pub state: String,
    pub pkce: Pkce,
    pub device: String,
    pub session: String,
}

/// Steps 1 and 2: the address of Microsoft's sign-in page.
pub fn begin() -> Result<Begun, Fail> {
    let keys = Keys::new();
    let device = device_token(&keys)?;
    let pk = pkce();
    let state = crate::b64url(&crate::random(64));
    let body = json!({
        "AppId": APP_ID, "TitleId": TITLE_ID, "RedirectUri": REDIRECT, "DeviceToken": device, "Sandbox": "RETAIL",
        "TokenType": "code", "Offers": [SCOPE],
        "Query": {"display": "android_phone", "code_challenge": pk.challenge, "code_challenge_method": "S256", "state": state}
    });
    let url = "https://sisu.xboxlive.com/authenticate";
    let text = body.to_string();
    let mut r = Req::new("POST", url)
        .header("x-xbl-contract-version", "1")
        .header("Cache-Control", "no-store, must-revalidate, no-cache")
        .header("Signature", &keys.signature(url, "", &text));
    r.body = Some(text);
    r.headers.push(("Content-Type".into(), "application/json".into()));
    let resp = http::send(&r)?;
    let v = resp.json();
    let page = v["MsaOauthRedirect"].as_str().unwrap_or("").to_string();
    if !page.starts_with("https://") {
        return Err(Fail::Other("Microsoft didn't give a sign-in page".into()));
    }
    Ok(Begun { keys, page, state, pkce: pk, device, session: resp.header("x-sessionid").unwrap_or_default() })
}

/// What the sign-in window's last address says: the code, or why not.
pub fn code_from(redirect: &str, state: &str) -> Result<String, String> {
    if !redirect.starts_with(REDIRECT) {
        return Err("The sign-in ended somewhere unexpected.".into());
    }
    if let Some(e) = http::query(redirect, "error") {
        return Err(if e == "access_denied" { "The sign-in was cancelled on Microsoft's page.".into() } else { format!("Microsoft's page said: {e}") });
    }
    if http::query(redirect, "state").as_deref() != Some(state) {
        return Err("The sign-in didn't match the one this app started (try again).".into());
    }
    http::query(redirect, "code").filter(|c| !c.is_empty()).ok_or_else(|| "Microsoft's page gave no sign-in code.".into())
}

/// The user token: access and refresh.
#[derive(Clone, Debug, Default)]
pub struct User {
    pub access: String,
    pub refresh: String,
}

fn user_from(v: &Value) -> Result<User, Fail> {
    let access = v["access_token"].as_str().unwrap_or("").to_string();
    let refresh = v["refresh_token"].as_str().unwrap_or("").to_string();
    if access.is_empty() || refresh.is_empty() {
        return Err(Fail::Other("Microsoft's answer had no sign-in token".into()));
    }
    Ok(User { access, refresh })
}

pub fn exchange(code: &str, verifier: &str) -> Result<User, Fail> {
    let r = Req::new("POST", "https://login.live.com/oauth20_token.srf")
        .header("Cache-Control", "no-store, must-revalidate, no-cache")
        .form(&[("client_id", APP_ID), ("code", code), ("code_verifier", verifier), ("grant_type", "authorization_code"), ("redirect_uri", REDIRECT), ("scope", SCOPE)]);
    user_from(&http::send(&r)?.json())
}

pub fn refresh(refresh_token: &str) -> Result<User, Fail> {
    let r = Req::new("POST", "https://login.live.com/oauth20_token.srf")
        .header("Cache-Control", "no-store, must-revalidate, no-cache")
        .form(&[("client_id", APP_ID), ("grant_type", "refresh_token"), ("refresh_token", refresh_token), ("scope", SCOPE)]);
    user_from(&http::send(&r)?.json())
}

/// The token Cloud's "connect" step asks for (xal-node's exchangeRefreshTokenForXcloudTransferToken).
pub fn transfer_token(refresh_token: &str) -> Result<String, Fail> {
    let r = Req::new("POST", "https://login.live.com/oauth20_token.srf")
        .header("Cache-Control", "no-store, must-revalidate, no-cache")
        .form(&[("client_id", APP_ID), ("grant_type", "refresh_token"), ("scope", "service::http://Passport.NET/purpose::PURPOSE_XBOX_CLOUD_CONSOLE_TRANSFER_TOKEN"), ("refresh_token", refresh_token), ("code", ""), ("code_verifier", ""), ("redirect_uri", "")]);
    let v = http::send(&r)?.json();
    v["lpt"].as_str().or_else(|| v["access_token"].as_str()).map(String::from).ok_or_else(|| Fail::Other("no Cloud token".into()))
}

/// SISU authorize's answer: the tokens XSTS needs, and the gamertag.
#[derive(Clone, Debug, Default)]
pub struct Sisu {
    pub device: String,
    pub title: String,
    pub user: String,
    pub gamertag: String,
}

pub fn sisu_from(v: &Value) -> Result<Sisu, Fail> {
    let s = |p: &str| v.pointer(p).and_then(Value::as_str).unwrap_or("").to_string();
    let xui = v.pointer("/AuthorizationToken/DisplayClaims/xui/0").cloned().unwrap_or(Value::Null);
    let gamertag = ["mgt", "gtg"].iter().find_map(|k| xui[*k].as_str().filter(|x| !x.is_empty())).unwrap_or("").to_string();
    let out = Sisu { device: s("/DeviceToken"), title: s("/TitleToken/Token"), user: s("/UserToken/Token"), gamertag };
    if out.user.is_empty() || out.title.is_empty() {
        return Err(Fail::Other("Microsoft's answer had no Xbox tokens".into()));
    }
    Ok(out)
}

pub fn sisu_authorize(keys: &Keys, user: &User, device: &str, session: &str) -> Result<Sisu, Fail> {
    let mut body = json!({
        "AccessToken": format!("t={}", user.access), "AppId": APP_ID, "DeviceToken": device, "Sandbox": "RETAIL",
        "SiteName": "user.auth.xboxlive.com", "UseModernGamertag": true, "ProofKey": keys.proof_key()
    });
    if !session.is_empty() {
        body["SessionId"] = Value::String(session.into());
    }
    sisu_from(&xbl_post(keys, "https://sisu.xboxlive.com/authorize", &body)?)
}

/// An XSTS token and its user hash (Authorization: XBL3.0 x=uhs;token).
#[derive(Clone, Debug, Default)]
pub struct Xsts {
    pub token: String,
    pub uhs: String,
}

pub fn xsts(keys: &Keys, sisu: &Sisu, party: &str) -> Result<Xsts, Fail> {
    let body = json!({
        "Properties": {"SandboxId": "RETAIL", "DeviceToken": sisu.device, "TitleToken": sisu.title, "UserTokens": [sisu.user]},
        "RelyingParty": party, "TokenType": "JWT"
    });
    let v = xbl_post(keys, "https://xsts.auth.xboxlive.com/xsts/authorize", &body)?;
    let token = v["Token"].as_str().unwrap_or("").to_string();
    let uhs = v.pointer("/DisplayClaims/xui/0/uhs").and_then(Value::as_str).unwrap_or("").to_string();
    if token.is_empty() {
        return Err(Fail::Other("Microsoft's answer had no XSTS token".into()));
    }
    Ok(Xsts { token, uhs })
}

/// A streaming token: where to stream (the default region's base address) and the bearer token.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    pub token: String,
    pub base: String,
    pub until: u64,
}

pub fn stream_from(v: &Value) -> Result<Stream, Fail> {
    let token = v["gsToken"].as_str().unwrap_or("").to_string();
    let regions = v.pointer("/offeringSettings/regions").and_then(Value::as_array).cloned().unwrap_or_default();
    let base = regions.iter().find(|r| r["isDefault"].as_bool().unwrap_or(false)).or_else(|| regions.first()).and_then(|r| r["baseUri"].as_str()).unwrap_or("").trim_end_matches('/').to_string();
    if token.is_empty() || base.is_empty() {
        return Err(Fail::Other("Microsoft's answer had no streaming token".into()));
    }
    Ok(Stream { token, base, until: crate::now() + v["durationInSeconds"].as_u64().unwrap_or(3600) })
}

pub fn stream_token(x: &Xsts, offering: &str) -> Result<Stream, Fail> {
    let r = Req::new("POST", &format!("https://{offering}.gssv-play-prod.xboxlive.com/v2/login/user"))
        .header("Cache-Control", "no-store, must-revalidate, no-cache")
        .header("x-gssv-client", "XboxComBrowser")
        .json(&json!({"token": x.token, "offeringId": offering}));
    stream_from(&http::send(&r)?.json())
}

// ------------------------------------------------------------------ the account, in memory
/// A signed-in account: what's kept (keys, refresh token, gamertag) and what's asked for again each time.
pub struct Account {
    pub keys: Keys,
    pub refresh: String,
    pub gamertag: String,
    pub sisu: Option<Sisu>,
    pub web: Option<(Xsts, u64)>,
    pub home: Option<Stream>,
    /// Cloud (xgpuweb: Game Pass Ultimate); None when the account hasn't it (or it isn't asked yet).
    pub cloud: Option<Stream>,
    pub cloud_checked: bool,
}

impl Account {
    /// Step 4: from the page's code to a signed-in account.
    pub fn finish(b: Begun, code: &str) -> Result<Account, Fail> {
        let user = exchange(code, &b.pkce.verifier)?;
        let sisu = sisu_authorize(&b.keys, &user, &b.device, &b.session)?;
        Ok(Account { gamertag: sisu.gamertag.clone(), keys: b.keys, refresh: user.refresh, sisu: Some(sisu), web: None, home: None, cloud: None, cloud_checked: false })
    }

    /// The kept part, for the keyring (one JSON item; it's never written anywhere else).
    pub fn kept(&self) -> String {
        json!({"v": 1, "key": self.keys.to_b64(), "refresh": self.refresh, "gamertag": self.gamertag}).to_string()
    }

    pub fn from_kept(s: &str) -> Option<Account> {
        let v: Value = serde_json::from_str(s).ok()?;
        let keys = Keys::from_b64(v["key"].as_str()?)?;
        let refresh = v["refresh"].as_str()?.to_string();
        let gamertag = v["gamertag"].as_str().unwrap_or("").to_string();
        Some(Account { keys, refresh, gamertag, sisu: None, web: None, home: None, cloud: None, cloud_checked: false })
    }

    /// The tokens again from the refresh token (when the app opens, or one has run out). Microsoft gives a new refresh
    /// token each time: the caller keeps it (keyring::save) when `kept()` changed.
    pub fn renew(&mut self) -> Result<(), Fail> {
        let user = refresh(&self.refresh)?;
        let device = device_token(&self.keys)?;
        let sisu = sisu_authorize(&self.keys, &user, &device, "")?;
        if !sisu.gamertag.is_empty() {
            self.gamertag = sisu.gamertag.clone();
        }
        self.refresh = user.refresh;
        self.sisu = Some(sisu);
        self.web = None;
        self.home = None;
        self.cloud = None;
        self.cloud_checked = false;
        Ok(())
    }

    fn sisu(&mut self) -> Result<Sisu, Fail> {
        if self.sisu.is_none() {
            self.renew()?;
        }
        self.sisu.clone().ok_or_else(|| Fail::Other("not signed in".into()))
    }

    /// For the console list and Wake (xboxlive.com).
    pub fn web(&mut self) -> Result<Xsts, Fail> {
        if let Some((x, until)) = &self.web {
            if *until > crate::now() + 60 {
                return Ok(x.clone());
            }
        }
        let s = self.sisu()?;
        let x = xsts(&self.keys, &s, "http://xboxlive.com")?;
        self.web = Some((x.clone(), crate::now() + 3600));
        Ok(x)
    }

    /// For streaming from the Xbox at home.
    pub fn home(&mut self) -> Result<Stream, Fail> {
        if let Some(h) = &self.home {
            if h.until > crate::now() + 60 {
                return Ok(h.clone());
            }
        }
        let s = self.sisu()?;
        let x = xsts(&self.keys, &s, "http://gssv.xboxlive.com/")?;
        let h = stream_token(&x, "xhome")?;
        if !self.cloud_checked {
            self.cloud_checked = true;
            self.cloud = stream_token(&x, "xgpuweb").ok();
        }
        self.home = Some(h.clone());
        Ok(h)
    }

    /// Cloud's token: only with Game Pass Ultimate (xgpuweb). None: the tab isn't shown.
    pub fn cloud(&mut self) -> Option<Stream> {
        let _ = self.home();
        self.cloud.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::signature::Verifier;

    #[test]
    fn the_signature_is_xal_nodes() {
        let k = Keys::new();
        let ts = windows_time(1_700_000_000);
        assert_eq!(ts, 133_444_736_000_000_000);
        let url = "https://sisu.xboxlive.com/authorize?x=1";
        assert_eq!(path_of(url), "/authorize");
        let body = "{\"a\":1}";
        let bytes = signed_bytes("/authorize", "", body, ts);
        // 4 + 1 + 8 + 1, then "POST\0", "/authorize\0", "\0", body and "\0" (xal-node's allocSize).
        assert_eq!(bytes.len(), 5 + 9 + 5 + "/authorize".len() + 1 + 0 + 1 + body.len() + 1);
        assert_eq!(&bytes[..5], &[0, 0, 0, 1, 0]);
        assert_eq!(&bytes[5..13], &ts.to_be_bytes());
        assert_eq!(&bytes[13..19], b"\0POST\0");
        let h = crate::unb64(&k.signature_at(url, "", body, ts)).unwrap();
        assert_eq!(h.len(), 76);
        assert_eq!(&h[..4], &[0, 0, 0, 1]);
        assert_eq!(&h[4..12], &ts.to_be_bytes());
        let sig = Signature::from_slice(&h[12..]).unwrap();
        assert!(k.verifying().verify(&bytes, &sig).is_ok());
        let pk = k.proof_key();
        assert_eq!(pk["crv"], "P-256");
        assert_eq!(pk["x"].as_str().unwrap().len(), 43);
        // The key comes back from what's kept.
        let again = Keys::from_b64(&k.to_b64()).unwrap();
        assert_eq!(again.proof_key(), pk);
    }

    #[test]
    fn pkce_is_s256() {
        // RFC 7636's own example.
        assert_eq!(challenge_of("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let p = pkce();
        assert_eq!(p.verifier.len(), 43);
    }

    #[test]
    fn the_redirect_gives_the_code_only_for_our_state() {
        let ok = "ms-xal-000000004c20a908://auth?code=M.C5_abc&state=S1";
        assert_eq!(code_from(ok, "S1").unwrap(), "M.C5_abc");
        assert!(code_from(ok, "S2").is_err());
        assert!(code_from("ms-xal-000000004c20a908://auth?error=access_denied&state=S1", "S1").unwrap_err().contains("cancelled"));
        assert!(code_from("https://evil.test/?code=x&state=S1", "S1").is_err());
    }

    #[test]
    fn the_whole_sign_in_against_the_stand_in() {
        http::set_test_answers(Some(serde_json::from_str(include_str!("../tests/fixtures/http.json")).unwrap()));
        let b = begin().unwrap();
        assert!(b.page.starts_with("https://login.live.com/"));
        assert_eq!(b.session, "stand-in-session");
        let code = code_from(&format!("{REDIRECT}?code=STANDIN&state={}", http::enc(&b.state)), &b.state).unwrap();
        let mut a = Account::finish(b, &code).unwrap();
        assert_eq!(a.gamertag, "EmberRunner42");
        let kept = a.kept();
        assert!(kept.contains("refresh"));
        let mut again = Account::from_kept(&kept).unwrap();
        assert_eq!(again.gamertag, "EmberRunner42");
        again.renew().unwrap();
        let h = again.home().unwrap();
        assert_eq!(h.base, "https://home.stand-in.test");
        assert!(again.cloud().is_some());
        assert_eq!(a.web().unwrap().uhs, "1234567890");
        http::set_test_answers(None);
    }
}
