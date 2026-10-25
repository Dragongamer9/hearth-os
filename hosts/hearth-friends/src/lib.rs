//! Hearth Friends' rules, without Matrix or a network: who counts as a friend's address, the links and QR codes
//! that add a friend, the game invite and Play together cards inside messages, which invites are friend requests,
//! the settings file, and keeping access tokens out of anything written down. The binary (main.rs) puts these on
//! top of matrix-sdk; docs/FRIENDS.md has the whole picture.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub mod proto;

/// The homeserver a new account goes to unless another is picked.
pub const DEFAULT_HOMESERVER: &str = "matrix.org";
/// The key game invites and Play together messages carry their details under (other Matrix apps show the body).
pub const GAME_KEY: &str = "io.hearthos.game";
pub const PLAY_KEY: &str = "io.hearthos.play";

// ------------------------------------------------------------------------------------------------ addresses

/// Is this a Matrix user ID: `@name:server` (server with an optional :port)? Kept strict, so a typo is caught here
/// and not by the server.
pub fn valid_user_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('@') else { return false };
    let Some((local, server)) = rest.split_once(':') else { return false };
    if local.is_empty() || s.len() > 255 { return false; }
    if !local.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._=-/+".contains(c)) { return false; }
    valid_server(server)
}

/// `example.org`, `matrix.example.org:8448`, `localhost:8008`, `[::1]:8448`, `127.0.0.1:8008`.
pub fn valid_server(s: &str) -> bool {
    if s.is_empty() || s.len() > 230 { return false; }
    let (host, port) = if let Some(r) = s.strip_prefix('[') {
        let Some((h, after)) = r.split_once(']') else { return false };
        if h.is_empty() || !h.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.') { return false; }
        if !after.is_empty() && !after.starts_with(':') { return false; }
        return valid_port(after.strip_prefix(':'));
    } else {
        match s.rsplit_once(':') { Some((h, p)) => (h, Some(p)), None => (s, None) }
    };
    valid_port(port) && !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

fn valid_port(port: Option<&str>) -> bool {
    match port { None => true, Some(p) => !p.is_empty() && p.len() <= 5 && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u32>().map_or(false, |n| n > 0 && n <= 65535) }
}

/// What someone typed or scanned to add a friend, as a user ID: `@alice:example.org`, `alice:example.org`,
/// `alice` (on `default_server`), `https://matrix.to/#/@alice:example.org`, `matrix:u/alice:example.org` and
/// Hearth's own `hearth://friends/add/@alice:example.org`. None when it isn't one.
pub fn parse_friend(input: &str, default_server: &str) -> Option<String> {
    let s = input.trim();
    let s = s.strip_prefix("hearth://friends/add/").unwrap_or(s);
    let s = s.strip_prefix("https://matrix.to/#/").or_else(|| s.strip_prefix("http://matrix.to/#/")).unwrap_or(s);
    let s = s.split(['?', '#']).next().unwrap_or("");
    let s = percent_decode(s);
    let id = if let Some(r) = s.strip_prefix("matrix:u/") { format!("@{r}") }
        else if s.starts_with('@') { s.to_string() }
        else if s.contains(':') { format!("@{s}") }
        else if !s.is_empty() { format!("@{s}:{default_server}") }
        else { return None };
    let id = id.to_lowercase_local();
    valid_user_id(&id).then_some(id)
}

trait LowerLocal { fn to_lowercase_local(&self) -> String; }
impl LowerLocal for String {
    /// User IDs' names are lower case; the server part is left as it is.
    fn to_lowercase_local(&self) -> String {
        match self.split_once(':') { Some((l, s)) => format!("{}:{}", l.to_lowercase(), s), None => self.to_lowercase() }
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(n) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) { out.push(n); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The links to share for adding you: matrix.to (opens in any Matrix app or a browser) and Hearth's own.
pub fn friend_links(user_id: &str) -> Value {
    json!({"userId": user_id, "matrixTo": format!("https://matrix.to/#/{user_id}"), "hearth": format!("hearth://friends/add/{user_id}")})
}

/// The homeserver part of a user ID ("example.org" for "@a:example.org").
pub fn server_of(user_id: &str) -> &str { user_id.split_once(':').map(|(_, s)| s).unwrap_or("") }

/// Where an account lives, in words for the screen: "matrix.org" or "your own server, chat.example.org".
pub fn where_it_lives(homeserver: &str) -> String {
    let h = homeserver.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
    if h == DEFAULT_HOMESERVER || h == "matrix-client.matrix.org" { "matrix.org (run by the Matrix.org Foundation)".into() } else { format!("{h} (a Matrix server you picked)") }
}

// ------------------------------------------------------------------------------------------------ game cards

/// A game to invite a friend to: Hearth's id for it, the store and the store's own id (so the other side finds the
/// same game in its library), and optionally a server to join.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameRef {
    pub title: String,
    #[serde(default)] pub store: String,
    #[serde(default)] pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub app_id: Option<String>,
    /// "Join my server": an address (host:port) or a game's own connect link the host filled in.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub server: Option<String>,
}

impl GameRef {
    /// Cleans what came from a message: short strings, a Steam app id that's only digits, a server that's an
    /// address or a steam://connect link (nothing that could run something else).
    pub fn cleaned(mut self) -> Option<GameRef> {
        self.title = clip(self.title.trim(), 120);
        if self.title.is_empty() { return None; }
        self.store = clip(&self.store, 24).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        self.id = clip(&self.id, 200);
        self.app_id = self.app_id.filter(|a| !a.is_empty() && a.len() <= 12 && a.chars().all(|c| c.is_ascii_digit()));
        self.server = self.server.and_then(|s| clean_server(&s));
        Some(self)
    }
}

/// A server to join: `host:port` (name, IPv4 or [IPv6]) or `steam://connect/host:port[/password]`.
pub fn clean_server(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.len() > 200 { return None; }
    let addr = s.strip_prefix("steam://connect/").unwrap_or(s);
    let host_port = addr.split('/').next().unwrap_or("");
    if !host_port.contains(':') || !valid_server(host_port) { return None; }
    if addr.chars().any(|c| c.is_whitespace() || c == '"' || c == '\'' || c == '`' || c == ';' || c == '&' || c == '|' || c == '$') { return None; }
    Some(if s.starts_with("steam://connect/") { s.to_string() } else { host_port.to_string() })
}

fn clip(s: &str, n: usize) -> String { s.chars().take(n).collect() }

/// The message a game invite is: text for any Matrix app, the details for Hearth.
pub fn invite_content(g: &GameRef) -> Value {
    let mut body = format!("🎮 Let's play {}!", g.title);
    if let Some(s) = &g.server { body.push_str(&format!(" Join my server: {s}")); }
    body.push_str(" (Hearth game invite)");
    json!({"msgtype": "m.text", "body": body, GAME_KEY: {"v": 1, "game": g}})
}

/// Play together (Remote Play Together, Hearth's way): the host streams the running game and a friend's controller
/// becomes another player. The steps are messages in the encrypted chat.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Play {
    /// offer (host → friend), request (friend → host, with the friend's pairing PIN), accept, decline, kick, end
    pub action: String,
    /// One id per Play together session, so old messages don't act on a new one.
    pub session: String,
    #[serde(default)] pub game: String,
    /// The host's addresses to stream from (LAN first; over the internet through Hearth Link when it's there).
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub port: Option<u16>,
    /// The 4-digit PIN the friend's Moonlight pairs with; only in a request, inside the encrypted chat.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub pin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub device: Option<String>,
    /// How the friend reaches the host: "lan" or "link" (Hearth Link's peer-to-peer/relay).
    #[serde(default, skip_serializing_if = "Option::is_none")] pub transport: Option<String>,
}

pub const PLAY_ACTIONS: &[&str] = &["offer", "request", "accept", "decline", "kick", "end"];

impl Play {
    pub fn cleaned(mut self) -> Option<Play> {
        if !PLAY_ACTIONS.contains(&self.action.as_str()) { return None; }
        if self.session.is_empty() || self.session.len() > 64 || !self.session.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { return None; }
        self.game = clip(&self.game, 120);
        self.addresses = self.addresses.into_iter().filter(|a| valid_server(a)).take(8).collect();
        self.pin = self.pin.filter(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit()));
        self.device = self.device.map(|d| clip(&d, 60).chars().filter(|c| !c.is_control()).collect());
        self.transport = self.transport.filter(|t| t == "lan" || t == "link");
        Some(self)
    }
    pub fn body(&self) -> String {
        match self.action.as_str() {
            "offer" => format!("🕹 Play {} together with me (Hearth Play together)", if self.game.is_empty() { "my game" } else { &self.game }),
            "request" => "🕹 I'd like to join your game (Hearth Play together)".into(),
            "accept" => "🕹 Joining you now (Hearth Play together)".into(),
            "decline" => "🕹 Not this time (Hearth Play together)".into(),
            "kick" => "🕹 Play together ended by the host".into(),
            _ => "🕹 Play together ended".into(),
        }
    }
}

pub fn play_content(p: &Play) -> Value { json!({"msgtype": "m.notice", "body": p.body(), PLAY_KEY: p}) }

/// A message's content as Hearth shows it: kind text / image / game / play / notice / file / unknown, the text,
/// and the game or Play together details when a Hearth sent them (checked: a stranger's malformed card shows as
/// plain text).
pub fn describe_content(content: &Value) -> Value {
    let body = content["body"].as_str().unwrap_or("").chars().take(4000).collect::<String>();
    if let Some(g) = content.get(GAME_KEY).and_then(|g| serde_json::from_value::<GameRef>(g["game"].clone()).ok()).and_then(GameRef::cleaned) {
        return json!({"kind": "game", "body": body, "game": g});
    }
    if let Some(p) = content.get(PLAY_KEY).and_then(|p| serde_json::from_value::<Play>(p.clone()).ok()).and_then(Play::cleaned) {
        return json!({"kind": "play", "body": body, "play": p});
    }
    match content["msgtype"].as_str().unwrap_or("") {
        "m.image" => json!({"kind": "image", "body": body, "image": {"w": content["info"]["w"], "h": content["info"]["h"], "size": content["info"]["size"], "mime": content["info"]["mimetype"]}}),
        "m.notice" => json!({"kind": "notice", "body": body}),
        "m.emote" => json!({"kind": "text", "body": format!("* {body}")}),
        "m.file" | "m.video" | "m.audio" => json!({"kind": "file", "body": body}),
        "m.text" => json!({"kind": "text", "body": body}),
        _ if !body.is_empty() => json!({"kind": "text", "body": body}),
        _ => json!({"kind": "unknown", "body": ""}),
    }
}

// ------------------------------------------------------------------------------------------------ privacy

/// Where an invite to a room goes: shown with the chats (from a friend), into Requests (from someone else), or
/// nowhere (blocked: the server already hides blocked people's invites; this is the second fence).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Bucket { Chats, Requests, Hidden }

pub fn invite_bucket(inviter: &str, friends: &std::collections::HashSet<String>, blocked: &std::collections::HashSet<String>, friends_only: bool) -> Bucket {
    if blocked.contains(inviter) { Bucket::Hidden }
    else if friends.contains(inviter) || !friends_only { Bucket::Chats }
    else { Bucket::Requests }
}

/// "Playing Portal 2" (plus " · Joinable" when the host offers a way in), or nothing when the player keeps it to
/// themselves. Matrix shows presence to people who share a chat with you.
pub fn playing_status(game: Option<&str>, joinable: bool, share: bool) -> Option<String> {
    if !share { return None; }
    let g = game.map(str::trim).filter(|g| !g.is_empty())?;
    Some(format!("Playing {}{}", clip(g, 100), if joinable { " · Joinable" } else { "" }))
}

/// The other way: a friend's status message → (game, joinable).
pub fn parse_status(msg: &str) -> (Option<String>, bool) {
    let m = msg.trim();
    let Some(rest) = m.strip_prefix("Playing ") else { return (None, false) };
    let (g, j) = match rest.strip_suffix(" · Joinable") { Some(g) => (g, true), None => (rest, false) };
    (Some(g.trim().to_string()).filter(|g| !g.is_empty()), j)
}

// ------------------------------------------------------------------------------------------------ settings

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Messages and invites from people who aren't friends go to Requests, without a pop-up.
    pub friends_only: bool,
    /// Show "Playing <game>" automatically to people you share a chat with.
    pub share_playing: bool,
    /// online, unavailable (Away) or offline (appear offline).
    pub presence: String,
    /// Message text in notifications (else "New message from Sarah").
    pub previews: bool,
    /// Chats with notifications turned off.
    pub muted: Vec<String>,
    /// The last homeserver used, for the sign-in screen.
    pub homeserver: String,
    /// The "keep your encrypted chats: a recovery key" question was answered (or skipped) for this sign-in.
    pub recovery_asked: bool,
}

impl Default for Settings {
    fn default() -> Self { Settings { friends_only: true, share_playing: true, presence: "online".into(), previews: true, muted: vec![], homeserver: DEFAULT_HOMESERVER.into(), recovery_asked: false } }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str::<Settings>(&s).ok()).map(Settings::cleaned).unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() { std::fs::create_dir_all(d)?; }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default())?;
        std::fs::rename(tmp, path)
    }
    pub fn cleaned(mut self) -> Settings {
        if !["online", "unavailable", "offline"].contains(&self.presence.as_str()) { self.presence = "online".into(); }
        self.muted.retain(|r| r.starts_with('!') && r.len() < 256);
        self.muted.dedup();
        if self.homeserver.trim().is_empty() { self.homeserver = DEFAULT_HOMESERVER.into(); }
        self
    }
    /// Applies {friendsOnly, sharePlaying, presence, previews} from the page; other keys are ignored.
    pub fn patch(&mut self, p: &Value) {
        if let Some(b) = p["friendsOnly"].as_bool() { self.friends_only = b; }
        if let Some(b) = p["sharePlaying"].as_bool() { self.share_playing = b; }
        if let Some(b) = p["previews"].as_bool() { self.previews = b; }
        if let Some(s) = p["presence"].as_str() { if ["online", "unavailable", "offline"].contains(&s) { self.presence = s.into(); } }
    }
    pub fn set_muted(&mut self, room: &str, on: bool) {
        self.muted.retain(|r| r != room);
        if on && room.starts_with('!') { self.muted.push(room.to_string()); }
    }
}

// ------------------------------------------------------------------------------------------------ places

/// Where Hearth Friends keeps things. `data`: the encrypted Matrix store (~/.local/share/hearth/friends);
/// `config`: settings (~/.config/hearth/friends.json); `socket`: the local socket ($XDG_RUNTIME_DIR, the user's only).
#[derive(Clone, Debug)]
pub struct Paths { pub data: PathBuf, pub config: PathBuf, pub socket: PathBuf }

impl Paths {
    pub fn from_env() -> Paths {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
        let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")).join("hearth/friends");
        let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config")).join("hearth/friends.json");
        Paths { data, config, socket: socket_path() }
    }
}

pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEARTH_FRIENDS_SOCKET") { return PathBuf::from(p); }
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir());
    dir.join("hearth-friends.sock")
}

// ------------------------------------------------------------------------------------------------ logs

/// Text that may be written to a log or shown: access tokens, refresh tokens, passwords, PINs, sign-in codes and the
/// query part of web addresses (where OAuth codes and states travel) taken out.
/// Matrix servers' tokens start syt_ (Synapse), mct_/mat_/mar_ (Matrix Authentication Service) or are opaque after
/// "access_token"; every such word is replaced.
pub fn redact(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut hide_next = false;
    let mut in_query = false;
    for word in s.split_inclusive(|c: char| c.is_whitespace() || c == '&' || c == ',' || c == '"' || c == '\'' || c == '=') {
        let w = word.trim_end_matches(|c: char| c.is_whitespace() || c == '&' || c == ',' || c == '"' || c == '\'' || c == '=');
        let tail = &word[w.len()..];
        let lw = w.to_ascii_lowercase();
        let secretish = ["syt_", "mct_", "mat_", "mar_", "syr_", "mxr_"].iter().any(|p| lw.starts_with(p)) || lw.starts_with("bearer") && w.len() > 6;
        let url_ends = |t: &str| t.is_empty() || t.chars().any(|c| c.is_whitespace() || c == '"' || c == '\'' || c == ',');
        // A web address's query (codes, states, user codes): left out up to the end of the address.
        if in_query { if url_ends(tail) { in_query = false; out.push_str(tail); } continue; }
        if (lw.starts_with("http://") || lw.starts_with("https://")) && w.contains('?') {
            out.push_str(&w[..w.find('?').unwrap_or(w.len())]);
            out.push_str("?[hidden]");
            if url_ends(tail) { out.push_str(tail); } else { in_query = true; }
            continue;
        }
        if hide_next && !w.is_empty() && !lw.ends_with(':') { out.push_str("[hidden]"); out.push_str(tail); hide_next = false; continue; }
        if secretish { out.push_str("[hidden]"); out.push_str(tail); continue; }
        out.push_str(word);
        if ["access_token", "refresh_token", "password", "pin", "authorization", "token", "device_code", "user_code", "code_verifier", "client_secret", "recovery_key"].iter().any(|k| lw.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_') == *k) { hide_next = true; }
    }
    out
}

/// The QR code for a link, as an SVG picture (dark on light, a quiet zone round it).
pub fn qr_svg(text: &str) -> Option<String> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    Some(code.render::<qrcode::render::svg::Color>().min_dimensions(220, 220).quiet_zone(true).build())
}

/// The same QR code as rows of "1" (dark) and "0" (light), without the quiet zone: for Mantel's Friends panel, which
/// draws it itself.
pub fn qr_rows(text: &str) -> Option<Vec<String>> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let w = code.width();
    let colors = code.to_colors();
    Some(colors.chunks(w).map(|row| row.iter().map(|c| if *c == qrcode::Color::Dark { '1' } else { '0' }).collect()).collect())
}

// ------------------------------------------------------------------------------------------------ the server's own sign-in page

// Servers with Matrix's newer sign-in (OAuth 2.0, MSC3861 and its parts MSC2964-MSC2967; matrix.org runs the Matrix
// Authentication Service) sign people in and up on their own page. Hearth registers itself as an app there on each
// device, at each sign-in (dynamic client registration, MSC2966): a public client, no secret anywhere.
pub const CLIENT_NAME: &str = "Hearth";
pub const CLIENT_URI: &str = "https://hearthos.fun/";
pub const LOGO_URI: &str = "https://hearthos.fun/img/logo.svg";
/// RFC 8628's grant type: sign in on another device (a phone) with a code or a QR code.
pub const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Hearth's client metadata for the server: for a phone, the device authorization grant; with a redirect address
/// (the browser on this device comes back to Hearth's loopback address, RFC 8252), the authorization code grant.
/// Each registration asks only for the grant it uses (a server may have the device grant turned off).
pub fn client_metadata(redirect_uri: Option<&str>) -> Value {
    let mut m = json!({"application_type": "native", "client_name": CLIENT_NAME, "client_uri": CLIENT_URI, "logo_uri": LOGO_URI,
                       "token_endpoint_auth_method": "none"});
    match redirect_uri {
        Some(r) => {
            m["grant_types"] = json!(["authorization_code", "refresh_token"]);
            m["redirect_uris"] = json!([r]);
            m["response_types"] = json!(["code"]);
        }
        None => { m["grant_types"] = json!([DEVICE_GRANT, "refresh_token"]); }
    }
    m
}

/// The scopes for a session: the whole client-server API, and this device's id (MSC2967).
pub fn oauth_scope(device_id: &str) -> String {
    format!("urn:matrix:org.matrix.msc2967.client:api:* urn:matrix:org.matrix.msc2967.client:device:{device_id}")
}

/// application/x-www-form-urlencoded, for the device authorization and token endpoints.
pub fn form_body(pairs: &[(&str, &str)]) -> String {
    let enc = |s: &str| s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

/// The server's answer to "sign in on another device" (RFC 8628 §3.2).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceGrant { pub device_code: String, pub user_code: String, pub uri: String, pub complete: Option<String>, pub expires_in: u64, pub interval: u64 }

impl DeviceGrant {
    pub fn parse(v: &Value) -> Option<DeviceGrant> {
        let web = |s: &str| (s.starts_with("https://") || s.starts_with("http://")) && s.len() < 2048 && !s.chars().any(|c| c.is_whitespace() || c.is_control());
        let s = |k: &str| v[k].as_str().map(str::to_string).filter(|x| !x.is_empty());
        let uri = s("verification_uri").filter(|u| web(u))?;
        Some(DeviceGrant {
            device_code: s("device_code")?, user_code: s("user_code")?, uri,
            complete: s("verification_uri_complete").filter(|u| web(u)),
            expires_in: v["expires_in"].as_u64().unwrap_or(600).clamp(30, 1800),
            interval: v["interval"].as_u64().unwrap_or(5).clamp(1, 60),
        })
    }
    /// What the QR code opens: the page with the code filled in, when the server gives one.
    pub fn link(&self) -> &str { self.complete.as_deref().unwrap_or(&self.uri) }
}

/// What to do after a token request during the device grant (RFC 8628 §3.5).
#[derive(Debug, PartialEq)]
pub enum Poll { Wait, SlowDown, Expired, Denied, Failed }

pub fn device_poll(error: &str) -> Poll {
    match error { "authorization_pending" => Poll::Wait, "slow_down" => Poll::SlowDown, "expired_token" => Poll::Expired, "access_denied" => Poll::Denied, _ => Poll::Failed }
}

/// A web address as people type it: no https://, no trailing slash.
pub fn short_url(u: &str) -> String {
    u.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string()
}

/// The browser coming back to Hearth's loopback address: the query of `GET /callback?...` (else None: a favicon,
/// anything else).
pub fn callback_query(head: &str) -> Option<String> {
    let line = head.lines().next()?;
    let mut parts = line.split(' ');
    if parts.next()? != "GET" { return None; }
    let target = parts.next()?;
    let q = target.strip_prefix("/callback?")?;
    (!q.is_empty() && q.len() < 8192).then(|| q.to_string())
}

/// The page the browser shows after coming back to Hearth.
pub fn callback_page(ok: bool) -> String {
    let (title, text) = if ok { ("You're signed in", "Hearth Friends is signed in. You can close this page and go back to Hearth.") }
        else { ("That didn't work", "Hearth couldn't finish signing in. Go back to Hearth and try again.") };
    format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Hearth: {title}</title>\
<style>body{{font-family:system-ui,sans-serif;background:#12161c;color:#e8edf2;display:grid;place-items:center;min-height:100vh;margin:0}}main{{max-width:30em;padding:2em;text-align:center}}h1{{font-size:1.6em}}</style></head>\
<body><main><h1>{title}</h1><p>{text}</p></main></body></html>")
}

/// What a server says to an empty sign-up request: "open" (sign-up from apps), "token" (needs a sign-up token),
/// "web" (its own page: captcha, email, terms) or "none" (no sign-up).
pub fn register_kind(status: u16, body: &Value) -> &'static str {
    if status != 401 { return "none"; }
    let flows: Vec<Vec<String>> = body["flows"].as_array().cloned().unwrap_or_default().iter()
        .map(|f| f["stages"].as_array().cloned().unwrap_or_default().iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).collect();
    if flows.iter().any(|st| st.iter().all(|s| s == "m.login.dummy")) { "open" }
    else if flows.iter().any(|st| st.iter().all(|s| s == "m.login.dummy" || s == "m.login.registration_token")) { "token" }
    else if flows.is_empty() { "none" } else { "web" }
}

/// A random id or PIN: `n` characters from `alphabet`.
pub fn random(n: usize, alphabet: &[u8]) -> String {
    let mut b = vec![0u8; n];
    let _ = getrandom::getrandom(&mut b);
    b.iter().map(|x| alphabet[*x as usize % alphabet.len()] as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn user_ids() {
        assert!(valid_user_id("@alice:example.org"));
        assert!(valid_user_id("@a.b_c=d-e/f+g:matrix.example.org:8448"));
        assert!(valid_user_id("@bob:localhost:8008"));
        assert!(valid_user_id("@bob:[::1]:8448"));
        assert!(!valid_user_id("alice:example.org"));
        assert!(!valid_user_id("@Alice:example.org"));
        assert!(!valid_user_id("@alice"));
        assert!(!valid_user_id("@:example.org"));
        assert!(!valid_user_id("@alice:exa mple.org"));
        assert!(!valid_user_id("@alice:example.org:99999"));
        assert!(!valid_user_id("@alice:example.org:"));
    }

    #[test]
    fn friend_links_and_codes() {
        let d = "matrix.org";
        assert_eq!(parse_friend("@alice:example.org", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("  alice:example.org ", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("Alice", d).as_deref(), Some("@alice:matrix.org"));
        assert_eq!(parse_friend("https://matrix.to/#/@alice:example.org", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("https://matrix.to/#/%40alice%3Aexample.org", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("https://matrix.to/#/@alice:example.org?via=example.org", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("matrix:u/alice:example.org", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("hearth://friends/add/@alice:example.org", d).as_deref(), Some("@alice:example.org"));
        assert_eq!(parse_friend("", d), None);
        assert_eq!(parse_friend("not a user", d), None);
        assert_eq!(parse_friend("https://matrix.to/#/!room:example.org", d), None);
        let l = friend_links("@alice:example.org");
        assert_eq!(l["matrixTo"], "https://matrix.to/#/@alice:example.org");
        assert_eq!(parse_friend(l["hearth"].as_str().unwrap(), d).as_deref(), Some("@alice:example.org"));
        let svg = qr_svg(l["matrixTo"].as_str().unwrap()).unwrap();
        assert!(svg.contains("<svg") && svg.len() > 500);
        assert_eq!(server_of("@a:example.org:8448"), "example.org:8448");
        assert!(where_it_lives("https://matrix.org").starts_with("matrix.org"));
        assert!(where_it_lives("chat.example.org").contains("you picked"));
    }

    #[test]
    fn game_invites_round_trip_and_are_checked() {
        let g = GameRef { title: "Portal 2".into(), store: "steam".into(), id: "steam:620".into(), app_id: Some("620".into()), server: Some("192.168.1.20:27015".into()) };
        let c = invite_content(&g);
        assert!(c["body"].as_str().unwrap().contains("Portal 2"));
        let d = describe_content(&c);
        assert_eq!(d["kind"], "game");
        assert_eq!(serde_json::from_value::<GameRef>(d["game"].clone()).unwrap(), g);
        // A card from someone else's app with things that don't belong.
        let bad = json!({"msgtype": "m.text", "body": "hi", GAME_KEY: {"game": {"title": "X", "appId": "62;rm", "server": "1.2.3.4:27015; rm -rf /", "store": "st eam"}}});
        let d = describe_content(&bad);
        assert_eq!(d["kind"], "game");
        assert!(d["game"]["appId"].is_null());
        assert!(d["game"]["server"].is_null());
        assert_eq!(d["game"]["store"], "steam");
        // No title: plain text.
        assert_eq!(describe_content(&json!({"msgtype": "m.text", "body": "hey", GAME_KEY: {"game": {"title": " "}}}))["kind"], "text");
        assert_eq!(clean_server("steam://connect/10.0.0.2:27015/pw").as_deref(), Some("steam://connect/10.0.0.2:27015/pw"));
        assert_eq!(clean_server("[2001:db8::1]:7777").as_deref(), Some("[2001:db8::1]:7777"));
        assert_eq!(clean_server("example.org"), None);
        assert_eq!(clean_server("steam://run/620"), None);
    }

    #[test]
    fn play_together_cards() {
        let p = Play { action: "request".into(), session: "abc-123".into(), game: "Hades".into(), addresses: vec!["192.168.1.5".into(), "bad host".into()], port: Some(47989), pin: Some("1234".into()), device: Some("Ally\u{7}".into()), transport: Some("lan".into()) };
        let d = describe_content(&play_content(&p));
        assert_eq!(d["kind"], "play");
        let back: Play = serde_json::from_value(d["play"].clone()).unwrap();
        assert_eq!(back.addresses, vec!["192.168.1.5".to_string()]);
        assert_eq!(back.device.as_deref(), Some("Ally"));
        assert_eq!(back.pin.as_deref(), Some("1234"));
        let mut bad = p.clone(); bad.action = "run".into();
        assert!(bad.cleaned().is_none());
        let mut bad = p.clone(); bad.pin = Some("12a4".into());
        assert!(bad.cleaned().unwrap().pin.is_none());
        let mut bad = p; bad.session = "x y".into();
        assert!(bad.cleaned().is_none());
    }

    #[test]
    fn content_kinds() {
        assert_eq!(describe_content(&json!({"msgtype": "m.image", "body": "shot.png", "info": {"w": 1280, "h": 800}}))["kind"], "image");
        assert_eq!(describe_content(&json!({"msgtype": "m.emote", "body": "waves"}))["body"], "* waves");
        assert_eq!(describe_content(&json!({"msgtype": "m.text", "body": "x".repeat(5000)}))["body"].as_str().unwrap().len(), 4000);
        assert_eq!(describe_content(&json!({}))["kind"], "unknown");
    }

    #[test]
    fn requests_from_strangers() {
        let friends: HashSet<String> = ["@sarah:matrix.org".to_string()].into();
        let blocked: HashSet<String> = ["@spam:evil.example".to_string()].into();
        assert_eq!(invite_bucket("@sarah:matrix.org", &friends, &blocked, true), Bucket::Chats);
        assert_eq!(invite_bucket("@new:matrix.org", &friends, &blocked, true), Bucket::Requests);
        assert_eq!(invite_bucket("@new:matrix.org", &friends, &blocked, false), Bucket::Chats);
        assert_eq!(invite_bucket("@spam:evil.example", &friends, &blocked, false), Bucket::Hidden);
    }

    #[test]
    fn playing_status_both_ways() {
        assert_eq!(playing_status(Some("Portal 2"), false, true).as_deref(), Some("Playing Portal 2"));
        assert_eq!(playing_status(Some("Portal 2"), true, true).as_deref(), Some("Playing Portal 2 · Joinable"));
        assert_eq!(playing_status(Some("Portal 2"), true, false), None);
        assert_eq!(playing_status(None, false, true), None);
        assert_eq!(parse_status("Playing Portal 2 · Joinable"), (Some("Portal 2".into()), true));
        assert_eq!(parse_status("Playing Hades"), (Some("Hades".into()), false));
        assert_eq!(parse_status("brb"), (None, false));
    }

    #[test]
    fn settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hearth/friends.json");
        let mut s = Settings::load(&p);
        assert_eq!(s, Settings::default());
        assert!(s.friends_only && s.share_playing);
        s.patch(&json!({"friendsOnly": false, "presence": "unavailable", "evil": 1}));
        s.set_muted("!room:example.org", true);
        s.set_muted("not-a-room", true);
        s.save(&p).unwrap();
        let t = Settings::load(&p);
        assert!(!t.friends_only);
        assert_eq!(t.presence, "unavailable");
        assert_eq!(t.muted, vec!["!room:example.org".to_string()]);
        std::fs::write(&p, r#"{"presence":"hacked","muted":["x"]}"#).unwrap();
        let u = Settings::load(&p);
        assert_eq!(u.presence, "online");
        assert!(u.muted.is_empty());
        assert!(u.friends_only, "missing keys keep the safe defaults");
    }

    #[test]
    fn tokens_never_reach_logs() {
        let r = redact("request failed: access_token=syt_YWxpY2U_abcdef_123 & refresh_token=mar_zzz Authorization: Bearer mct_secret");
        assert!(!r.contains("syt_") && !r.contains("mar_zzz") && !r.contains("mct_secret"), "{r}");
        assert!(r.contains("request failed"));
        let r = redact(r#"{"password":"hunter2","user":"alice"}"#);
        assert!(!r.contains("hunter2"), "{r}");
        assert!(r.contains("alice"));
        let r = redact("pin 4821 for pairing");
        assert!(!r.contains("4821"), "{r}");
        assert_eq!(redact("Couldn't reach matrix.org"), "Couldn't reach matrix.org");
    }

    #[test]
    fn sign_in_codes_never_reach_logs() {
        let r = redact("open https://account.example.org/link?code=ABCD-EFGH&x=1 now");
        assert_eq!(r, "open https://account.example.org/link?[hidden] now");
        let r = redact("GET http://127.0.0.1:4711/callback?code=s3cr3t&state=xyz");
        assert!(!r.contains("s3cr3t") && !r.contains("xyz"), "{r}");
        let r = redact(r#"{"device_code":"dc123","user_code":"UC42"}"#);
        assert!(!r.contains("dc123") && !r.contains("UC42"), "{r}");
        assert_eq!(redact("see https://hearthos.fun/ for more"), "see https://hearthos.fun/ for more");
    }

    #[test]
    fn oauth_client_and_device_grant() {
        let m = client_metadata(None);
        assert_eq!(m["application_type"], "native");
        assert_eq!(m["client_name"], "Hearth");
        assert_eq!(m["token_endpoint_auth_method"], "none");
        assert!(m["grant_types"].as_array().unwrap().iter().any(|g| g == DEVICE_GRANT));
        assert!(m.get("redirect_uris").is_none());
        let m = client_metadata(Some("http://127.0.0.1:4711/callback"));
        assert_eq!(m["redirect_uris"][0], "http://127.0.0.1:4711/callback");
        assert_eq!(m["grant_types"][0], "authorization_code");
        assert!(!m["grant_types"].as_array().unwrap().iter().any(|g| g == DEVICE_GRANT));
        assert_eq!(m["response_types"][0], "code");
        assert!(oauth_scope("ABCDEF").ends_with("client:device:ABCDEF"));
        assert_eq!(form_body(&[("scope", "a:* b"), ("x", "é&=")]), "scope=a%3A%2A%20b&x=%C3%A9%26%3D");
        let g = DeviceGrant::parse(&json!({"device_code": "d", "user_code": "WDJB-MJHT", "verification_uri": "https://account.example.org/link",
                                           "verification_uri_complete": "https://account.example.org/link?code=WDJB-MJHT", "expires_in": 900, "interval": 0})).unwrap();
        assert_eq!(g.link(), "https://account.example.org/link?code=WDJB-MJHT");
        assert_eq!(g.interval, 1);
        assert_eq!(short_url(&g.uri), "account.example.org/link");
        assert!(DeviceGrant::parse(&json!({"device_code": "d", "user_code": "u", "verification_uri": "javascript:alert(1)"})).is_none());
        let g = DeviceGrant::parse(&json!({"device_code": "d", "user_code": "u", "verification_uri": "https://a.example/link", "verification_uri_complete": "bad uri"})).unwrap();
        assert_eq!(g.link(), "https://a.example/link");
        assert_eq!(device_poll("authorization_pending"), Poll::Wait);
        assert_eq!(device_poll("slow_down"), Poll::SlowDown);
        assert_eq!(device_poll("expired_token"), Poll::Expired);
        assert_eq!(device_poll("access_denied"), Poll::Denied);
        assert_eq!(device_poll("invalid_grant"), Poll::Failed);
    }

    #[test]
    fn loopback_and_sign_up_kinds() {
        assert_eq!(callback_query("GET /callback?code=abc&state=def HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_deref(), Some("code=abc&state=def"));
        assert_eq!(callback_query("GET /favicon.ico HTTP/1.1\r\n\r\n"), None);
        assert_eq!(callback_query("POST /callback?code=abc HTTP/1.1\r\n\r\n"), None);
        assert!(callback_page(true).contains("signed in"));
        assert!(!callback_page(false).contains("<script"));
        assert_eq!(register_kind(401, &json!({"flows": [{"stages": ["m.login.dummy"]}]})), "open");
        assert_eq!(register_kind(401, &json!({"flows": [{"stages": ["m.login.registration_token"]}]})), "token");
        assert_eq!(register_kind(401, &json!({"flows": [{"stages": ["m.login.recaptcha", "m.login.terms", "m.login.email.identity"]}]})), "web");
        assert_eq!(register_kind(403, &json!({"errcode": "M_FORBIDDEN"})), "none");
        assert_eq!(register_kind(404, &json!({})), "none");
        let rows = qr_rows("https://account.example.org/link?code=WDJB-MJHT").unwrap();
        assert!(rows.len() >= 21 && rows.iter().all(|r| r.len() == rows.len() && r.chars().all(|c| c == '0' || c == '1')));
    }

    #[test]
    fn randoms() {
        let a = random(24, b"abcdefghijklmnopqrstuvwxyz0123456789");
        assert_eq!(a.len(), 24);
        assert_ne!(a, random(24, b"abcdefghijklmnopqrstuvwxyz0123456789"));
        assert!(random(4, b"0123456789").chars().all(|c| c.is_ascii_digit()));
    }
}
