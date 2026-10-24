//! The Matrix side of Hearth Friends: the account, syncing, friends (direct chats), requests, chats, presence with
//! "Playing <game>", messages with pictures, game invites and Play together messages, block/report/mute.
//! Built on matrix-sdk with end-to-end encryption: new chats are encrypted, and the keys stay in the local store,
//! itself encrypted with a key kept in the system keyring.
//!
//! Message text is only held in memory (the last few hundred per chat) for the screens; nothing readable is written
//! to disk by Hearth Friends itself.

use crate::{notify, secrets};
use hearth_friends::{self as hf, Paths, Settings};
use matrix_sdk::{
    attachment::AttachmentConfig,
    authentication::matrix::MatrixSession,
    config::SyncSettings,
    encryption::EncryptionSettings,
    media::{MediaFormat, MediaRequestParameters},
    room::MessagesOptions,
    ruma::{
        api::client::{receipt::create_receipt::v3::ReceiptType, reporting::report_user, room::create_room},
        events::{
            ignored_user_list::IgnoredUserListEventContent,
            receipt::ReceiptThread,
            room::{encryption::RoomEncryptionEventContent, EncryptedFile, MediaSource},
            InitialStateEvent,
        },
        presence::PresenceState,
        EventId, OwnedMxcUri, OwnedUserId, RoomId, UserId,
    },
    store::RoomLoadSettings,
    Client, Room,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, Notify, RwLock};

const KEEP: usize = 300;           // messages kept in memory per chat
const MAX_IMAGE: usize = 20 << 20;  // biggest picture sent or shown

#[derive(Default)]
struct State {
    presence: HashMap<String, Value>,
    unread: HashMap<String, u64>,
    typing: HashMap<String, Vec<String>>,
    msgs: HashMap<String, VecDeque<Value>>,
    /// The token to load older messages from, per chat (None: the start was reached).
    older: HashMap<String, Option<String>>,
    /// Pictures' sources (with their decryption keys), by event id: only in memory.
    media: HashMap<String, Value>,
    names: HashMap<String, String>,
    blocked: HashSet<String>,
    seen_invites: HashSet<String>,
    playing: Option<hf::GameRef>,
    joinable: bool,
    error: Option<String>,
    synced: bool,
    keyring_ok: bool,
    homeserver: String,
}

pub struct Service {
    pub paths: Paths,
    client: RwLock<Option<Client>>,
    generation: AtomicU64,
    st: Mutex<State>,
    settings: Mutex<Settings>,
    pub events: broadcast::Sender<(String, Value)>,
    pub gamemode_watchers: AtomicUsize,
    wake: Notify,
    /// When the session was last looked for in the keyring (it may not be unlocked yet when the service starts).
    last_restore: AtomicU64,
    /// Signing in, up or out right now: no looking for the old session meanwhile.
    busy: std::sync::atomic::AtomicBool,
}

fn e2s<E: std::fmt::Display>(e: E) -> String { hf::redact(&e.to_string()) }
fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
fn arg<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> { a[k].as_str().filter(|s| !s.is_empty()).ok_or(format!("Missing '{k}'")) }

fn device_name() -> String {
    let host = std::fs::read_to_string("/etc/hostname").ok().map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).unwrap_or_else(|| "this device".into());
    format!("Hearth on {host}")
}

fn presence_state(s: &str) -> PresenceState {
    match s { "unavailable" => PresenceState::Unavailable, "offline" => PresenceState::Offline, _ => PresenceState::Online }
}

impl Service {
    pub fn new(paths: Paths) -> Arc<Service> {
        let settings = Settings::load(&paths.config);
        let (tx, _) = broadcast::channel(512);
        Arc::new(Service { paths, client: RwLock::new(None), generation: AtomicU64::new(0), st: Mutex::new(State::default()), settings: Mutex::new(settings),
                           events: tx, gamemode_watchers: AtomicUsize::new(0), wake: Notify::new(), last_restore: AtomicU64::new(0), busy: std::sync::atomic::AtomicBool::new(false) })
    }

    fn emit(&self, name: &str, data: Value) { let _ = self.events.send((name.to_string(), data)); }
    fn settings(&self) -> Settings { self.settings.lock().unwrap().clone() }
    fn save_settings(&self, s: Settings) -> Result<(), String> {
        s.save(&self.paths.config).map_err(e2s)?;
        *self.settings.lock().unwrap() = s;
        Ok(())
    }

    async fn client(&self) -> Result<Client, String> {
        self.client.read().await.clone().ok_or_else(|| "Sign in to Hearth Friends first.".to_string())
    }

    fn store_dir(&self) -> std::path::PathBuf { self.paths.data.join("store") }

    async fn build(&self, homeserver: &str, store_key: &str) -> Result<Client, String> {
        let dir = self.store_dir();
        std::fs::create_dir_all(&dir).map_err(e2s)?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.paths.data, std::fs::Permissions::from_mode(0o700));
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
        let mut enc = EncryptionSettings::default();
        enc.auto_enable_cross_signing = true;
        Client::builder()
            .server_name_or_homeserver_url(homeserver)
            .sqlite_store(&dir, Some(store_key))
            .with_encryption_settings(enc)
            .build().await
            .map_err(|e| format!("Couldn't reach {homeserver}: {}", e2s(e)))
    }

    /// At start: the session from the keyring, if there is one.
    pub async fn restore(self: &Arc<Self>) {
        self.last_restore.store(now_ms(), Ordering::SeqCst);
        let (Some(sess), Some(key)) = (secrets::get(&self.paths, "session"), secrets::get(&self.paths, "store-key")) else { return };
        let Ok(v) = serde_json::from_str::<Value>(&sess) else { return };
        let hs = v["homeserver"].as_str().unwrap_or("").to_string();
        let Ok(session) = serde_json::from_value::<MatrixSession>(v["session"].clone()) else { return };
        match self.build(&hs, &key).await {
            Ok(c) => match c.matrix_auth().restore_session(session, RoomLoadSettings::default()).await {
                Ok(()) => { self.st.lock().unwrap().keyring_ok = true; self.signed_in(c, &hs).await; }
                Err(e) => { self.st.lock().unwrap().error = Some(format!("Couldn't open your Hearth Friends account: {}", e2s(e))); }
            },
            Err(e) => { self.st.lock().unwrap().error = Some(e); }
        }
    }

    async fn signed_in(self: &Arc<Self>, c: Client, homeserver: &str) {
        {
            let mut st = self.st.lock().unwrap();
            let keyring_ok = st.keyring_ok;
            *st = State { keyring_ok, homeserver: homeserver.to_string(), ..Default::default() };
        }
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.client.write().await = Some(c.clone());
        self.refresh_blocked(&c).await;
        let s = self.settings();
        let _ = c.set_presence(presence_state(&s.presence), None, false).await;
        self.wake.notify_one();
        self.emit("status", self.status_value().await);
    }

    /// Sign in with a user name (or @name:server) and password.
    pub async fn login(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        let user = arg(a, "user")?.trim().to_string();
        let password = arg(a, "password")?.to_string();
        let mut hs = a["homeserver"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
            .unwrap_or_else(|| if user.starts_with('@') && user.contains(':') { hf::server_of(&user).to_string() } else { hf::DEFAULT_HOMESERVER.to_string() });
        if hs.ends_with('/') { hs.pop(); }
        self.sign_out_local().await;
        let key = hf::random(48, b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
        let keyring_ok = secrets::set(&self.paths, "store-key", &key).is_ok();
        let c = self.build(&hs, &key).await?;
        c.matrix_auth().login_username(&user, &password).initial_device_display_name(&device_name()).send().await
            .map_err(|e| { let s = e2s(&e); if s.contains("M_FORBIDDEN") { "That user name and password didn't work.".to_string() } else if s.contains("M_USER_DEACTIVATED") { "That account was closed.".into() } else { format!("Couldn't sign in: {s}") } })?;
        let session = c.matrix_auth().session().ok_or("The server didn't start a session.")?;
        let saved = keyring_ok && secrets::set(&self.paths, "session", &json!({"homeserver": hs, "session": session}).to_string()).is_ok();
        self.st.lock().unwrap().keyring_ok = saved;
        let mut s = self.settings(); s.homeserver = hs.clone(); let _ = self.save_settings(s);
        self.signed_in(c, &hs).await;
        Ok(self.status_value().await)
    }

    /// A new account on servers that allow it from an app (open sign-up, or a sign-up token). Servers that want a
    /// captcha, an email or their terms accepted on their own page (matrix.org among them) get their page instead.
    pub async fn register(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        let user = arg(a, "user")?.trim().trim_start_matches('@').split(':').next().unwrap_or("").to_lowercase();
        let password = arg(a, "password")?.to_string();
        if password.chars().count() < 8 { return Err("Pick a password of at least 8 characters.".into()); }
        let hs = a["homeserver"].as_str().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(hf::DEFAULT_HOMESERVER).trim_end_matches('/').to_string();
        let token = a["token"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        let web = if hs == hf::DEFAULT_HOMESERVER { "https://app.element.io/#/register".to_string() } else { format!("https://app.element.io/#/register?hs_url={}", if hs.starts_with("http") { hs.clone() } else { format!("https://{hs}") }) };
        // Find the server's address (well-known) with a throwaway client.
        let probe = Client::builder().server_name_or_homeserver_url(&hs).build().await.map_err(|e| format!("Couldn't reach {hs}: {}", e2s(e)))?;
        let url = probe.homeserver().join("_matrix/client/v3/register").map_err(e2s)?;
        let http = matrix_sdk::reqwest::Client::new();
        let post = |body: Value| { let (http, url) = (http.clone(), url.clone()); async move {
            let r = http.post(url).header("content-type", "application/json").body(body.to_string()).send().await.map_err(e2s)?;
            let code = r.status().as_u16();
            let v: Value = serde_json::from_slice(&r.bytes().await.map_err(e2s)?).unwrap_or(json!({}));
            Ok::<(u16, Value), String>((code, v))
        } };
        let base = json!({"username": user, "password": password, "initial_device_display_name": device_name(), "inhibit_login": true});
        let (mut code, mut v) = post(base.clone()).await?;
        if code == 401 {
            let session = v["session"].as_str().unwrap_or("").to_string();
            let can = |st: &str| st == "m.login.dummy" || (st == "m.login.registration_token" && token.is_some());
            let flow = v["flows"].as_array().cloned().unwrap_or_default().into_iter()
                .map(|f| f["stages"].as_array().cloned().unwrap_or_default().iter().filter_map(|s| s.as_str().map(str::to_string)).collect::<Vec<_>>())
                .filter(|st| st.iter().all(|s| can(s))).min_by_key(|st| st.len());
            let Some(flow) = flow else {
                let needs_token = v["flows"].to_string().contains("m.login.registration_token");
                return Ok(json!({"needsWeb": !needs_token, "needsToken": needs_token, "url": web,
                    "why": if needs_token { "This server needs a sign-up token from whoever runs it." } else { "This server wants new accounts made on its own page (it asks for things like a captcha or an email). Make it there, then sign in here." }}));
            };
            for stage in flow {
                let mut auth = json!({"type": stage, "session": session});
                if stage == "m.login.registration_token" { auth["token"] = json!(token.clone().unwrap_or_default()); }
                let mut b = base.clone(); b["auth"] = auth;
                (code, v) = post(b).await?;
                if code != 401 { break; }
            }
        }
        if code != 200 {
            let err = v["errcode"].as_str().unwrap_or("");
            return Err(match err {
                "M_USER_IN_USE" => "That name is taken on this server. Try another.".into(),
                "M_INVALID_USERNAME" => "Names can have lower-case letters, numbers and . _ = - / only.".into(),
                "M_WEAK_PASSWORD" => "The server wants a stronger password.".into(),
                "M_FORBIDDEN" => format!("This server doesn't take new accounts from apps ({}).", hf::redact(v["error"].as_str().unwrap_or("forbidden"))),
                _ => format!("The server said no: {}", hf::redact(v["error"].as_str().unwrap_or(err))),
            });
        }
        self.login(&json!({"user": user, "password": password, "homeserver": hs})).await
    }

    async fn sign_out_local(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.client.write().await = None;
        secrets::remove(&self.paths, "session");
        secrets::remove(&self.paths, "store-key");
        let _ = std::fs::remove_dir_all(self.store_dir());
        let mut st = self.st.lock().unwrap();
        *st = State::default();
    }

    pub async fn logout(self: &Arc<Self>) -> Result<Value, String> {
        if let Some(c) = self.client.read().await.clone() { let _ = c.matrix_auth().logout().await; }
        self.sign_out_local().await;
        self.emit("status", self.status_value().await);
        Ok(self.status_value().await)
    }

    pub async fn status_value(&self) -> Value {
        let c = self.client.read().await.clone();
        let s = self.settings();
        let st = self.st.lock().unwrap();
        match c.as_ref().and_then(|c| c.user_id().map(|u| (c, u.to_string()))) {
            Some((c, uid)) => json!({
                "signedIn": true, "userId": uid, "deviceId": c.device_id().map(|d| d.to_string()), "homeserver": st.homeserver,
                "lives": hf::where_it_lives(&st.homeserver), "keyringOk": st.keyring_ok, "synced": st.synced, "error": st.error,
                "settings": s, "encryption": "Chats with friends are end-to-end encrypted: only you and the people in the chat can read them.",
                "playing": st.playing, "joinable": st.joinable,
            }),
            None => json!({"signedIn": false, "error": st.error, "settings": s, "defaultHomeserver": hf::DEFAULT_HOMESERVER}),
        }
    }

    async fn refresh_blocked(&self, c: &Client) {
        let list: HashSet<String> = match c.account().account_data::<IgnoredUserListEventContent>().await {
            Ok(Some(raw)) => raw.deserialize().map(|c| c.ignored_users.keys().map(|u| u.to_string()).collect()).unwrap_or_default(),
            _ => HashSet::new(),
        };
        self.st.lock().unwrap().blocked = list;
    }

    // -------------------------------------------------------------------------------------------- syncing

    pub async fn sync_loop(self: Arc<Self>) {
        let mut backoff = 1u64;
        loop {
            let c = self.client.read().await.clone();
            let Some(c) = c else { self.wake.notified().await; continue };
            let gen = self.generation.load(Ordering::SeqCst);
            let pres = presence_state(&self.settings().presence);
            let first = !self.st.lock().unwrap().synced;
            let settings = SyncSettings::default().timeout(Duration::from_secs(if first { 0 } else { 25 })).set_presence(pres);
            match c.sync_once(settings).await {
                Ok(resp) => {
                    backoff = 1;
                    if self.generation.load(Ordering::SeqCst) != gen { continue; }
                    self.process(&c, resp, first).await;
                    if first {
                        { let mut st = self.st.lock().unwrap(); st.synced = true; st.error = None; }
                        self.emit("status", self.status_value().await);
                    }
                }
                Err(e) => {
                    if self.generation.load(Ordering::SeqCst) != gen { continue; }
                    let s = e2s(&e);
                    if s.contains("M_UNKNOWN_TOKEN") {
                        self.sign_out_local().await;
                        self.st.lock().unwrap().error = Some("You were signed out of Hearth Friends (the session ended on the server). Sign in again.".into());
                        self.emit("status", self.status_value().await);
                        continue;
                    }
                    self.st.lock().unwrap().error = Some(format!("Can't reach your Matrix server right now ({s}). Trying again…"));
                    self.emit("status", self.status_value().await);
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff * 2).min(60);
                }
            }
        }
    }

    async fn process(&self, c: &Client, resp: matrix_sdk::sync::SyncResponse, first: bool) {
        let me = c.user_id().map(|u| u.to_string()).unwrap_or_default();
        let mut presence_changed = false;
        for raw in &resp.presence {
            let Ok(v) = serde_json::from_str::<Value>(raw.json().get()) else { continue };
            let Some(sender) = v["sender"].as_str() else { continue };
            let k = &v["content"];
            self.st.lock().unwrap().presence.insert(sender.to_string(), json!({
                "presence": k["presence"].as_str().unwrap_or("offline"), "statusMsg": k["status_msg"], "lastActiveAgo": k["last_active_ago"],
                "currentlyActive": k["currently_active"], "at": now_ms()}));
            presence_changed = true;
        }
        if presence_changed { self.emit("presence", json!({})); }
        for raw in &resp.account_data {
            if raw.json().get().contains("m.ignored_user_list") { self.refresh_blocked(c).await; }
        }
        let mut rooms_changed = !resp.rooms.left.is_empty() || !resp.rooms.invited.is_empty();
        for (room_id, upd) in &resp.rooms.joined {
            let rid = room_id.to_string();
            {
                let mut st = self.st.lock().unwrap();
                let old = st.unread.insert(rid.clone(), upd.unread_notifications.notification_count);
                if old != Some(upd.unread_notifications.notification_count) { rooms_changed = true; }
                if upd.timeline.limited { st.msgs.remove(&rid); st.older.insert(rid.clone(), upd.timeline.prev_batch.clone()); }
                else if !st.older.contains_key(&rid) { st.older.insert(rid.clone(), upd.timeline.prev_batch.clone()); }
            }
            let room = c.get_room(room_id);
            for ev in &upd.timeline.events {
                let Ok(v) = serde_json::from_str::<Value>(ev.raw().json().get()) else { continue };
                if v["type"] == "m.room.member" { rooms_changed = true; }
                let Some(msg) = self.message_json(&me, &v, ev.encryption_info().is_some()) else { continue };
                {
                    let mut st = self.st.lock().unwrap();
                    let q = st.msgs.entry(rid.clone()).or_default();
                    if !q.iter().any(|m| m["id"] == msg["id"]) { q.push_back(msg.clone()); while q.len() > KEEP { q.pop_front(); } }
                }
                rooms_changed = true;
                if !first {
                    let name = self.name_of(room.as_ref(), msg["sender"].as_str().unwrap_or("")).await;
                    let mut m = msg.clone(); m["senderName"] = json!(name);
                    self.emit("message", json!({"roomId": rid, "msg": m}));
                    if msg["mine"] != true { self.notify_message(room.as_ref(), &rid, &name, &m).await; }
                }
            }
            for raw in &upd.ephemeral {
                let Ok(v) = serde_json::from_str::<Value>(raw.json().get()) else { continue };
                if v["type"] == "m.typing" {
                    let users: Vec<String> = v["content"]["user_ids"].as_array().cloned().unwrap_or_default().iter().filter_map(|u| u.as_str()).filter(|u| *u != me).map(str::to_string).collect();
                    self.st.lock().unwrap().typing.insert(rid.clone(), users.clone());
                    self.emit("typing", json!({"roomId": rid, "users": users}));
                }
            }
        }
        // New invites: friend requests and chat invites, with a pop-up for the ones that aren't just requests.
        let friends = self.friend_ids(c).await;
        for room_id in resp.rooms.invited.keys() {
            let rid = room_id.to_string();
            if !self.st.lock().unwrap().seen_invites.insert(rid.clone()) || first { continue; }
            let Some(room) = c.get_room(room_id) else { continue };
            let Ok(inv) = room.invite_details().await else { continue };
            let from = inv.inviter_id.to_string();
            let (bucket, blocked) = { let st = self.st.lock().unwrap(); (hf::invite_bucket(&from, &friends, &st.blocked, self.settings().friends_only), st.blocked.contains(&from)) };
            if blocked { continue; }
            let name = inv.inviter.as_ref().and_then(|m| m.display_name().map(str::to_string)).unwrap_or_else(|| from.clone());
            let direct = room.is_direct().await.unwrap_or(false);
            let text = if direct { format!("{name} wants to be friends") } else { format!("{name} invited you to a group chat") };
            self.emit("request", json!({"roomId": rid, "from": from, "name": name, "direct": direct, "bucket": if bucket == hf::Bucket::Requests { "requests" } else { "chats" }, "text": text}));
            // Only invites that aren't requests pop up (friends, or anyone when "friends only" is off); requests wait quietly.
            if bucket == hf::Bucket::Chats { self.pop_up("Hearth Friends", &text); }
        }
        if rooms_changed { self.emit("rooms", json!({})); }
    }

    fn pop_up(&self, title: &str, body: &str) {
        if self.gamemode_watchers.load(Ordering::SeqCst) > 0 { return; }   // Game Mode shows its own toast
        let (t, b) = (title.to_string(), body.to_string());
        std::thread::spawn(move || { let _ = notify::desktop(&t, &b); });
    }

    async fn notify_message(&self, room: Option<&Room>, rid: &str, name: &str, m: &Value) {
        let s = self.settings();
        if s.muted.iter().any(|r| r == rid) { return; }
        let group = match room { Some(r) if r.direct_targets_length() != 1 => r.display_name().await.ok().map(|n| n.to_string()), _ => None };
        let title = match group { Some(g) => format!("{name} in {g}"), None => name.to_string() };
        let body = if !s.previews { "New message".to_string() } else { match m["kind"].as_str().unwrap_or("") {
            "image" => "Sent a picture".into(), "game" => format!("Invited you to play {}", m["game"]["title"].as_str().unwrap_or("a game")),
            "play" => m["body"].as_str().unwrap_or("Play together").to_string(), _ => m["body"].as_str().unwrap_or("").chars().take(200).collect() } };
        self.pop_up(&title, &body);
    }

    async fn name_of(&self, room: Option<&Room>, uid: &str) -> String {
        if let Some(n) = self.st.lock().unwrap().names.get(uid) { return n.clone(); }
        let mut name = uid.trim_start_matches('@').split(':').next().unwrap_or(uid).to_string();
        if let (Some(r), Ok(u)) = (room, UserId::parse(uid)) {
            if let Ok(Some(m)) = r.get_member_no_sync(&u).await { name = m.display_name().map(str::to_string).unwrap_or_else(|| m.name().to_string()); }
        }
        self.st.lock().unwrap().names.insert(uid.to_string(), name.clone());
        name
    }

    /// One timeline event as the screens show it, or None for events they don't.
    fn message_json(&self, me: &str, v: &Value, encrypted: bool) -> Option<Value> {
        let ty = v["type"].as_str()?;
        let id = v["event_id"].as_str()?.to_string();
        let sender = v["sender"].as_str().unwrap_or("").to_string();
        let base = json!({"id": id, "sender": sender, "mine": sender == me, "at": v["origin_server_ts"], "encrypted": encrypted});
        let mut m = match ty {
            "m.room.message" => {
                let c = &v["content"];
                if c["m.relates_to"]["rel_type"] == "m.replace" { return None; }
                if c.get("body").is_none() { return None; }   // redacted
                let d = hf::describe_content(c);
                if d["kind"] == "image" { self.st.lock().unwrap().media.insert(id.clone(), c.clone()); }
                d
            }
            "m.room.encrypted" => json!({"kind": "locked", "body": "This message can't be unlocked on this device yet. It may open once your other device shares its keys."}),
            "m.room.member" => {
                let who = v["content"]["displayname"].as_str().map(str::to_string).unwrap_or_else(|| v["state_key"].as_str().unwrap_or("").to_string());
                let what = match v["content"]["membership"].as_str().unwrap_or("") { "join" => "joined", "leave" => "left", "invite" => "was invited", "ban" => "was removed", _ => return None };
                json!({"kind": "member", "body": format!("{who} {what}")})
            }
            _ => return None,
        };
        if let (Some(o), Some(b)) = (m.as_object_mut(), base.as_object()) { for (k, x) in b { o.insert(k.clone(), x.clone()); } }
        Some(m)
    }

    // -------------------------------------------------------------------------------------------- lists

    async fn friend_ids(&self, c: &Client) -> HashSet<String> {
        let mut out = HashSet::new();
        for room in c.joined_rooms() {
            let t: Vec<String> = room.direct_targets().iter().filter_map(|t| t.as_user_id().map(|u| u.to_string())).collect();
            if t.len() == 1 { out.insert(t[0].clone()); }
        }
        out
    }

    pub async fn friends(&self) -> Result<Value, String> {
        let c = self.client().await?;
        let mut out = vec![];
        let mut seen = HashSet::new();
        for room in c.joined_rooms() {
            let targets: Vec<OwnedUserId> = room.direct_targets().iter().filter_map(|t| t.as_user_id().map(|u| u.to_owned())).collect();
            if targets.len() != 1 { continue; }
            let uid = &targets[0];
            if !seen.insert(uid.to_string()) { continue; }
            let member = room.get_member_no_sync(uid).await.ok().flatten();
            let membership = member.as_ref().map(|m| m.membership().to_string()).unwrap_or_default();
            let state = match membership.as_str() { "join" => "friend", "invite" => "pending", _ => continue };
            let name = member.as_ref().and_then(|m| m.display_name().map(str::to_string)).unwrap_or_else(|| uid.localpart().to_string());
            let rid = room.room_id().to_string();
            let st = self.st.lock().unwrap();
            let p = st.presence.get(uid.as_str()).cloned().unwrap_or(json!({}));
            let (game, joinable) = hf::parse_status(p["statusMsg"].as_str().unwrap_or(""));
            let pres = p["presence"].as_str().unwrap_or(if state == "pending" { "unknown" } else { "offline" }).to_string();
            out.push(json!({
                "userId": uid.to_string(), "name": name, "roomId": rid, "state": state, "presence": pres,
                "status": p["statusMsg"], "game": game, "joinable": joinable && pres != "offline", "lastActiveAgo": p["lastActiveAgo"],
                "unread": st.unread.get(&rid).copied().unwrap_or(0), "muted": self.settings().muted.contains(&rid),
                "blocked": st.blocked.contains(uid.as_str()),
            }));
        }
        let rank = |f: &Value| match (f["game"].is_string(), f["presence"].as_str().unwrap_or("")) { (true, _) => 0, (_, "online") => 1, (_, "unavailable") => 2, _ => 3 };
        out.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase())));
        Ok(json!(out))
    }

    pub async fn requests(&self) -> Result<Value, String> {
        let c = self.client().await?;
        let friends = self.friend_ids(&c).await;
        let s = self.settings();
        let (mut requests, mut invites) = (vec![], vec![]);
        for room in c.invited_rooms() {
            let Ok(inv) = room.invite_details().await else { continue };
            let from = inv.inviter_id.to_string();
            let bucket = { let st = self.st.lock().unwrap(); hf::invite_bucket(&from, &friends, &st.blocked, s.friends_only) };
            if bucket == hf::Bucket::Hidden { continue; }
            let direct = room.is_direct().await.unwrap_or(false);
            let name = inv.inviter.as_ref().and_then(|m| m.display_name().map(str::to_string)).unwrap_or_else(|| from.clone());
            let room_name = if direct { None } else { room.display_name().await.ok().map(|n| n.to_string()) };
            let r = json!({"roomId": room.room_id().to_string(), "from": from, "name": name, "direct": direct, "roomName": room_name});
            // A friend request (a direct chat) from anyone is a request; group invites from friends sit with the chats.
            if direct || bucket == hf::Bucket::Requests { requests.push(r) } else { invites.push(r) }
        }
        Ok(json!({"requests": requests, "invites": invites}))
    }

    pub async fn chats(&self) -> Result<Value, String> {
        let c = self.client().await?;
        let s = self.settings();
        let mut out = vec![];
        for room in c.joined_rooms() {
            let rid = room.room_id().to_string();
            let targets: Vec<String> = room.direct_targets().iter().filter_map(|t| t.as_user_id().map(|u| u.to_string())).collect();
            let name = room.display_name().await.map(|n| n.to_string()).unwrap_or_else(|_| rid.clone());
            let st = self.st.lock().unwrap();
            let last = st.msgs.get(&rid).and_then(|q| q.iter().rev().find(|m| m["kind"] != "member").cloned());
            out.push(json!({
                "roomId": rid, "name": name, "direct": targets.len() == 1, "with": targets.first(), "members": room.joined_members_count(),
                "unread": st.unread.get(&rid).copied().unwrap_or(0), "muted": s.muted.contains(&rid), "encrypted": room.encryption_state().is_encrypted(),
                "typing": st.typing.get(&rid).cloned().unwrap_or_default(), "last": last,
            }));
        }
        out.sort_by_key(|r| std::cmp::Reverse(r["last"]["at"].as_u64().unwrap_or(0)));
        Ok(json!(out))
    }

    fn room_of(c: &Client, a: &Value) -> Result<Room, String> {
        let rid = RoomId::parse(arg(a, "roomId")?).map_err(|_| "That chat isn't there.".to_string())?;
        c.get_room(&rid).ok_or_else(|| "That chat isn't there.".to_string())
    }

    pub async fn messages(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let rid = room.room_id().to_string();
        let me = c.user_id().map(|u| u.to_string()).unwrap_or_default();
        let have = self.st.lock().unwrap().msgs.get(&rid).map(|q| q.len()).unwrap_or(0);
        let more = a["more"] == true;
        if more || have < 20 {
            let from = self.st.lock().unwrap().older.get(&rid).cloned();
            if !matches!(from, Some(None)) {
                let mut o = MessagesOptions::backward();
                o.from = from.flatten();
                o.limit = 40u32.into();
                let r = room.messages(o).await.map_err(e2s)?;
                let mut older = vec![];
                for ev in &r.chunk {
                    let Ok(v) = serde_json::from_str::<Value>(ev.raw().json().get()) else { continue };
                    if let Some(m) = self.message_json(&me, &v, ev.encryption_info().is_some()) { older.push(m); }
                }
                let mut st = self.st.lock().unwrap();
                st.older.insert(rid.clone(), r.end.clone());
                let q = st.msgs.entry(rid.clone()).or_default();
                for m in older { if !q.iter().any(|x| x["id"] == m["id"]) { q.push_front(m); } }
                while q.len() > KEEP * 2 { q.pop_back(); }
            }
        }
        let list: Vec<Value> = self.st.lock().unwrap().msgs.get(&rid).map(|q| q.iter().cloned().collect()).unwrap_or_default();
        let mut out = Vec::with_capacity(list.len());
        for mut m in list { let n = self.name_of(Some(&room), m["sender"].as_str().unwrap_or("")).await; m["senderName"] = json!(n); out.push(m); }
        let at_start = matches!(self.st.lock().unwrap().older.get(&rid), Some(None));
        Ok(json!({"roomId": rid, "messages": out, "atStart": at_start, "encrypted": room.encryption_state().is_encrypted()}))
    }

    // -------------------------------------------------------------------------------------------- doing things

    pub async fn send_text(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let text = arg(a, "text")?.trim();
        if text.is_empty() { return Err("Nothing to send.".into()); }
        if text.len() > 16000 { return Err("That message is too long.".into()); }
        let r = room.send_raw("m.room.message", json!({"msgtype": "m.text", "body": text})).await.map_err(e2s)?;
        let _ = room.typing_notice(false).await;
        Ok(json!({"eventId": r.response.event_id.to_string()}))
    }

    pub async fn send_image(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let path = std::path::PathBuf::from(arg(a, "path")?);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let mime: mime::Mime = match ext.as_str() { "png" => mime::IMAGE_PNG, "jpg" | "jpeg" => mime::IMAGE_JPEG, "gif" => mime::IMAGE_GIF, "webp" => "image/webp".parse().unwrap(), _ => return Err("Only pictures (PNG, JPEG, WebP, GIF) can be sent.".into()) };
        let meta = std::fs::metadata(&path).map_err(|_| "That picture isn't there any more.".to_string())?;
        if !meta.is_file() || meta.len() as usize > MAX_IMAGE { return Err("That picture is too big to send (20 MB at most).".into()); }
        let data = std::fs::read(&path).map_err(e2s)?;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("screenshot.png").to_string();
        room.send_attachment(name, &mime, data, AttachmentConfig::new()).await.map_err(e2s)?;
        Ok(json!({"sent": true}))
    }

    /// A picture from a message, as a data: address for the page (decrypted in memory, never saved).
    pub async fn image(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let id = arg(a, "eventId")?;
        let content = self.st.lock().unwrap().media.get(id).cloned().ok_or("That picture isn't loaded. Open the chat again.")?;
        let source = if content["file"].is_object() {
            MediaSource::Encrypted(Box::new(serde_json::from_value::<EncryptedFile>(content["file"].clone()).map_err(|_| "That picture can't be opened.")?))
        } else {
            MediaSource::Plain(OwnedMxcUri::from(content["url"].as_str().ok_or("That picture can't be opened.")?))
        };
        let bytes = c.media().get_media_content(&MediaRequestParameters { source, format: MediaFormat::File }, true).await.map_err(e2s)?;
        if bytes.len() > MAX_IMAGE { return Err("That picture is too big to show.".into()); }
        let mime = content["info"]["mimetype"].as_str().filter(|m| m.starts_with("image/") && !m.contains("svg")).unwrap_or("image/png");
        use base64::Engine;
        Ok(json!({"url": format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))}))
    }

    pub async fn send_game(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let g: hf::GameRef = serde_json::from_value(a["game"].clone()).map_err(|_| "Pick a game to invite to.".to_string())?;
        let g = g.cleaned().ok_or("Pick a game to invite to.")?;
        room.send_raw("m.room.message", hf::invite_content(&g)).await.map_err(e2s)?;
        Ok(json!({"sent": true}))
    }

    pub async fn send_play(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let p: hf::Play = serde_json::from_value(a["play"].clone()).map_err(|_| "That Play together step isn't right.".to_string())?;
        let p = p.cleaned().ok_or("That Play together step isn't right.")?;
        room.send_raw("m.room.message", hf::play_content(&p)).await.map_err(e2s)?;
        Ok(json!({"sent": true}))
    }

    pub async fn typing(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        Self::room_of(&c, a)?.typing_notice(a["on"] == true).await.map_err(e2s)?;
        Ok(Value::Null)
    }

    pub async fn read(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let rid = room.room_id().to_string();
        let last = self.st.lock().unwrap().msgs.get(&rid).and_then(|q| q.back()).and_then(|m| m["id"].as_str().map(str::to_string));
        if let Some(id) = last.and_then(|i| EventId::parse(&i).ok()) {
            room.send_single_receipt(ReceiptType::Read, ReceiptThread::Unthreaded, id).await.map_err(e2s)?;
        }
        self.st.lock().unwrap().unread.insert(rid, 0);
        self.emit("rooms", json!({}));
        Ok(Value::Null)
    }

    /// A friend request: a new encrypted direct chat with them (they see it under Requests).
    pub async fn add(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let me = c.user_id().map(|u| u.to_string()).unwrap_or_default();
        let id = hf::parse_friend(arg(a, "userId")?, hf::server_of(&me)).ok_or("That isn't a Matrix address. It looks like @name:server (for example @sam:matrix.org).")?;
        if id == me { return Err("That's you.".into()); }
        let uid = UserId::parse(&id).map_err(|_| "That isn't a Matrix address.".to_string())?;
        if self.st.lock().unwrap().blocked.contains(&id) { return Err("You blocked them. Unblock them first.".into()); }
        if let Some(room) = c.get_dm_room(&uid) { return Ok(json!({"roomId": room.room_id().to_string(), "already": true})); }
        let room = c.create_dm(&uid).await.map_err(|e| { let s = e2s(&e); if s.contains("M_NOT_FOUND") || s.contains("Unknown user") { format!("There's no {id}.") } else { format!("Couldn't send the request: {s}") } })?;
        self.emit("rooms", json!({}));
        Ok(json!({"roomId": room.room_id().to_string(), "userId": id}))
    }

    pub async fn create_group(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let name = arg(a, "name")?.trim().chars().take(80).collect::<String>();
        let me = c.user_id().map(|u| u.to_string()).unwrap_or_default();
        let users: Vec<OwnedUserId> = a["users"].as_array().cloned().unwrap_or_default().iter().filter_map(|u| u.as_str()).filter_map(|u| hf::parse_friend(u, hf::server_of(&me))).filter(|u| *u != me).filter_map(|u| UserId::parse(&u).ok()).take(50).collect();
        let mut req = create_room::v3::Request::new();
        req.name = Some(name);
        req.invite = users;
        req.preset = Some(create_room::v3::RoomPreset::PrivateChat);
        req.initial_state = vec![InitialStateEvent::with_empty_state_key(RoomEncryptionEventContent::with_recommended_defaults()).to_raw_any()];
        let room = c.create_room(req).await.map_err(e2s)?;
        self.emit("rooms", json!({}));
        Ok(json!({"roomId": room.room_id().to_string()}))
    }

    pub async fn invite_to(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let room = Self::room_of(&c, a)?;
        let me = c.user_id().map(|u| u.to_string()).unwrap_or_default();
        let id = hf::parse_friend(arg(a, "userId")?, hf::server_of(&me)).ok_or("That isn't a Matrix address.")?;
        room.invite_user_by_id(&UserId::parse(&id).map_err(e2s)?).await.map_err(e2s)?;
        Ok(Value::Null)
    }

    pub async fn accept(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        Self::room_of(&c, a)?.join().await.map_err(e2s)?;
        self.emit("rooms", json!({}));
        Ok(Value::Null)
    }

    /// Decline a request or leave a chat (for a direct chat: no longer friends).
    pub async fn leave(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        Self::room_of(&c, a)?.leave().await.map_err(e2s)?;
        self.emit("rooms", json!({}));
        Ok(Value::Null)
    }

    pub async fn block(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let uid = UserId::parse(arg(a, "userId")?).map_err(|_| "That isn't a Matrix address.".to_string())?;
        if a["on"] == false {
            c.account().unignore_user(&uid).await.map_err(e2s)?;
        } else {
            c.account().ignore_user(&uid).await.map_err(e2s)?;
            // Their pending requests go too.
            for room in c.invited_rooms() {
                if let Ok(inv) = room.invite_details().await { if inv.inviter_id == uid { let _ = room.leave().await; } }
            }
        }
        self.refresh_blocked(&c).await;
        self.emit("rooms", json!({}));
        Ok(json!({"blocked": self.st.lock().unwrap().blocked.iter().cloned().collect::<Vec<_>>()}))
    }

    pub async fn blocked(&self) -> Result<Value, String> {
        let c = self.client().await?;
        self.refresh_blocked(&c).await;
        Ok(json!(self.st.lock().unwrap().blocked.iter().cloned().collect::<Vec<_>>()))
    }

    /// Report a message (roomId + eventId), a person (userId) or a chat (roomId) to the server's admins.
    pub async fn report(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let reason = a["reason"].as_str().unwrap_or("Reported from Hearth").chars().take(1000).collect::<String>();
        if let (Some(_), Some(eid)) = (a["roomId"].as_str(), a["eventId"].as_str()) {
            let room = Self::room_of(&c, a)?;
            room.report_content(EventId::parse(eid).map_err(e2s)?, Some(reason)).await.map_err(e2s)?;
            return Ok(json!({"reported": "message"}));
        }
        if let Some(u) = a["userId"].as_str() {
            let uid = UserId::parse(u).map_err(e2s)?;
            match c.send(report_user::v3::Request::new(uid.clone(), reason.clone())).await {
                Ok(_) => return Ok(json!({"reported": "user"})),
                Err(e) => {
                    // Older servers: report the chat with them instead.
                    if let Some(room) = c.get_dm_room(&uid) { room.report_room(reason).await.map_err(e2s)?; return Ok(json!({"reported": "chat"})); }
                    return Err(e2s(e));
                }
            }
        }
        Self::room_of(&c, a)?.report_room(reason).await.map_err(e2s)?;
        Ok(json!({"reported": "chat"}))
    }

    pub async fn mute(&self, a: &Value) -> Result<Value, String> {
        let rid = arg(a, "roomId")?.to_string();
        let mut s = self.settings();
        s.set_muted(&rid, a["on"] != false);
        self.save_settings(s)?;
        self.emit("rooms", json!({}));
        Ok(Value::Null)
    }

    pub async fn set_settings(&self, a: &Value) -> Result<Value, String> {
        let mut s = self.settings();
        let before = s.clone();
        s.patch(a);
        self.save_settings(s.clone())?;
        if before.presence != s.presence || before.share_playing != s.share_playing { self.push_presence().await; }
        let st = self.status_value().await;
        self.emit("status", st.clone());
        Ok(st)
    }

    pub async fn set_name(&self, a: &Value) -> Result<Value, String> {
        let c = self.client().await?;
        let n = arg(a, "name")?.trim().chars().take(60).collect::<String>();
        c.account().set_display_name(Some(&n)).await.map_err(e2s)?;
        Ok(Value::Null)
    }

    /// What Hearth is playing right now (the core tells this when a game starts and stops), shown as the status
    /// "Playing <game>" when sharing is on.
    pub async fn playing(&self, a: &Value) -> Result<Value, String> {
        let g = serde_json::from_value::<hf::GameRef>(a["game"].clone()).ok().and_then(hf::GameRef::cleaned);
        let joinable = a["joinable"] == true && g.is_some();
        { let mut st = self.st.lock().unwrap(); if st.playing == g && st.joinable == joinable { return Ok(Value::Null); } st.playing = g; st.joinable = joinable; }
        self.push_presence().await;
        Ok(Value::Null)
    }

    async fn push_presence(&self) {
        let Ok(c) = self.client().await else { return };
        let s = self.settings();
        let (title, joinable) = { let st = self.st.lock().unwrap(); (st.playing.as_ref().map(|g| g.title.clone()), st.joinable) };
        let msg = hf::playing_status(title.as_deref(), joinable, s.share_playing && s.presence != "offline").unwrap_or_default();
        let _ = c.set_presence(presence_state(&s.presence), Some(msg), true).await;
    }

    pub async fn link(&self) -> Result<Value, String> {
        let c = self.client().await?;
        let me = c.user_id().ok_or("Sign in first.")?.to_string();
        let mut l = hf::friend_links(&me);
        l["qr"] = json!(hf::qr_svg(l["matrixTo"].as_str().unwrap_or("")));
        Ok(l)
    }

    pub async fn handle(self: &Arc<Self>, method: &str, a: &Value) -> Result<Value, String> {
        match method {
            "status" => {
                // Not signed in: maybe the keyring wasn't unlocked yet when the service started. Look again now and then.
                if !self.busy.load(Ordering::SeqCst) && self.client.read().await.is_none() && now_ms().saturating_sub(self.last_restore.load(Ordering::SeqCst)) > 10_000 { self.restore().await; }
                Ok(self.status_value().await)
            }
            "login" | "register" | "logout" => {
                if self.busy.swap(true, Ordering::SeqCst) { return Err("Hold on: still signing in or out.".into()); }
                let r = match method { "login" => self.login(a).await, "register" => self.register(a).await, _ => self.logout().await };
                self.busy.store(false, Ordering::SeqCst);
                r
            }
            "friends" => self.friends().await,
            "requests" => self.requests().await,
            "chats" => self.chats().await,
            "messages" => self.messages(a).await,
            "send" => self.send_text(a).await,
            "sendImage" => self.send_image(a).await,
            "image" => self.image(a).await,
            "invite" => self.send_game(a).await,
            "play" => self.send_play(a).await,
            "typing" => self.typing(a).await,
            "read" => self.read(a).await,
            "add" => self.add(a).await,
            "createGroup" => self.create_group(a).await,
            "inviteTo" => self.invite_to(a).await,
            "accept" => self.accept(a).await,
            "decline" | "leave" => self.leave(a).await,
            "block" => self.block(a).await,
            "blocked" => self.blocked().await,
            "report" => self.report(a).await,
            "mute" => self.mute(a).await,
            "settings" => self.set_settings(a).await,
            "setName" => self.set_name(a).await,
            "playing" => self.playing(a).await,
            "link" => self.link().await,
            "parseFriend" => Ok(json!(hf::parse_friend(arg(a, "text")?, a["server"].as_str().unwrap_or(hf::DEFAULT_HOMESERVER)))),
            _ => Err(format!("Unknown request '{method}'")),
        }
    }
}
