//! The Xbox side (docs/CONSOLE-REMOTE-PLAY.md, "My consoles and Wake", "The stream"): the console list, Wake, the
//! Cloud library (Game Pass Ultimate), and a stream's session (play, its state, the SDP offer and answer, ICE,
//! keepalive, the end), as xbox-xcloud-player and xbox-webapi (MIT) do it. Every call is asked for once, by a page
//! opening, a button, or the stream's own steps: nothing here repeats on a timer except the stream's own waits.

use crate::http::{self, Fail, Req};
use crate::xal::{Stream, Xsts};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    On,
    Sleeping,
    Off,
}

impl Power {
    pub fn of(s: &str) -> Power {
        match s {
            "On" => Power::On,
            "ConnectedStandby" | "Standby" | "Sleeping" => Power::Sleeping,
            _ => Power::Off,
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Power::On => "On",
            Power::Sleeping => "Sleeping",
            Power::Off => "Offline",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Console {
    pub id: String,
    pub name: String,
    pub model: String,
    pub power: Power,
    /// Remote features (xccs' remoteManagementEnabled); true when xccs didn't say.
    pub remote_on: bool,
    /// When this app got its status (seconds since 1970).
    pub seen: u64,
}

/// "XboxSeriesX" → "Xbox Series X".
pub fn model_words(t: &str) -> String {
    match t {
        "XboxSeriesX" => "Xbox Series X".into(),
        "XboxSeriesS" => "Xbox Series S".into(),
        "XboxOne" => "Xbox One".into(),
        "XboxOneS" => "Xbox One S".into(),
        "XboxOneX" => "Xbox One X".into(),
        "" => "Xbox".into(),
        other => {
            let mut o = String::new();
            for (i, ch) in other.chars().enumerate() {
                if i > 0 && ch.is_uppercase() {
                    o.push(' ');
                }
                o.push(ch);
            }
            o
        }
    }
}

fn bearer(r: Req, s: &Stream) -> Req {
    r.header("Authorization", &format!("Bearer {}", s.token)).header("x-gssv-client", "XboxComBrowser").header("Accept", "application/json")
}

fn xbl(r: Req, x: &Xsts, contract: &str) -> Req {
    r.header("Authorization", &format!("XBL3.0 x={};{}", x.uhs, x.token)).header("x-xbl-contract-version", contract).header("skillplatform", "RemoteManagement").header("Accept-Language", "en-US")
}

/// The consoles that can stream, from the streaming service, with xccs' names and power states laid over them.
pub fn consoles_from(home: &Value, xccs: &Value) -> Vec<Console> {
    let devs = xccs["result"].as_array().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for r in home["results"].as_array().cloned().unwrap_or_default() {
        let id = r["serverId"].as_str().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let dev = devs.iter().find(|d| d["id"].as_str() == Some(id.as_str()));
        let name = dev.and_then(|d| d["name"].as_str()).or_else(|| r["deviceName"].as_str()).unwrap_or("Your Xbox").to_string();
        let power = dev.and_then(|d| d["powerState"].as_str()).or_else(|| r["powerState"].as_str()).map(Power::of).unwrap_or(Power::Off);
        let model = model_words(r["consoleType"].as_str().or_else(|| dev.and_then(|d| d["consoleType"].as_str())).unwrap_or(""));
        let remote_on = dev.and_then(|d| d["remoteManagementEnabled"].as_bool()).unwrap_or(true);
        out.push(Console { id, name, model, power, remote_on, seen: crate::now() });
    }
    out
}

pub fn consoles(home: &Stream, web: Option<&Xsts>) -> Result<Vec<Console>, Fail> {
    let h = http::send(&bearer(Req::new("GET", &format!("{}/v6/servers/home?mr=50", home.base)), home))?.json();
    let x = match web {
        Some(w) => http::send(&xbl(Req::new("GET", "https://xccs.xboxlive.com/lists/devices?queryCurrentDevice=false&includeStorageDevices=true"), w, "4")).map(|r| r.json()).unwrap_or(Value::Null),
        None => Value::Null,
    };
    Ok(consoles_from(&h, &x))
}

/// Wake: Power › WakeUp through xccs (the Xbox app's own way).
pub fn wake(web: &Xsts, console: &str) -> Result<(), Fail> {
    let body = json!({"destination": "Xbox", "type": "Power", "command": "WakeUp", "sessionId": crate::uuid(), "sourceId": "com.microsoft.smartglass", "parameters": [], "linkedXboxId": console});
    http::send(&xbl(Req::new("POST", "https://xccs.xboxlive.com/commands"), web, "4").json(&body)).map(|_| ())
}

/// The status of one console now (after Wake).
pub fn power(web: &Xsts, console: &str) -> Result<Power, Fail> {
    let v = http::send(&xbl(Req::new("GET", &format!("https://xccs.xboxlive.com/consoles/{console}")), web, "4"))?.json();
    Ok(Power::of(v["powerState"].as_str().unwrap_or("")))
}

// ------------------------------------------------------------------ Cloud (optional)
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub title: String,
    pub product: String,
    pub name: String,
    pub kind: String,
    pub picture: String,
}

pub fn games_from(titles: &Value, catalog: &Value) -> Vec<Game> {
    let mut out = Vec::new();
    for t in titles["results"].as_array().cloned().unwrap_or_default() {
        let title = t["titleId"].as_str().unwrap_or("").to_string();
        let product = t.pointer("/details/productId").and_then(Value::as_str).unwrap_or("").to_string();
        if title.is_empty() {
            continue;
        }
        let p = &catalog["Products"][product.as_str()];
        let name = p["ProductTitle"].as_str().unwrap_or(&title).to_string();
        let kind = p["Categories"].as_array().and_then(|c| c.first()).and_then(Value::as_str).unwrap_or("").to_string();
        let mut picture = p.pointer("/Image_Tile/URL").or_else(|| p.pointer("/Image_Poster/URL")).and_then(Value::as_str).unwrap_or("").to_string();
        if picture.starts_with("//") {
            picture = format!("https:{picture}");
        }
        out.push(Game { title, product, name, kind, picture });
    }
    out
}

/// Cloud's games: recently played (mru) or all.
pub fn games(cloud: &Stream, recent: bool) -> Result<Vec<Game>, Fail> {
    let path = if recent { "/v2/titles/mru?mr=25" } else { "/v2/titles?mr=500" };
    let titles = http::send(&bearer(Req::new("GET", &format!("{}{}", cloud.base, path)), cloud))?.json();
    let ids: Vec<Value> = titles["results"].as_array().cloned().unwrap_or_default().iter().filter_map(|t| t.pointer("/details/productId").cloned()).collect();
    let catalog = if ids.is_empty() {
        Value::Null
    } else {
        let r = Req::new("POST", "https://catalog.gamepass.com/v3/products?market=US&language=en-US&hydration=RemoteLowJade0")
            .header("ms-cv", "0")
            .header("calling-app-name", "Xbox Cloud Gaming Web")
            .header("calling-app-version", "21.0.0")
            .json(&json!({"Products": ids}));
        http::send(&r).map(|r| r.json()).unwrap_or(Value::Null)
    };
    Ok(games_from(&titles, &catalog))
}

// ------------------------------------------------------------------ a stream's session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Home,
    Cloud,
}

impl Kind {
    fn path(self) -> &'static str {
        match self {
            Kind::Home => "home",
            Kind::Cloud => "cloud",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub kind: Kind,
    pub base: String,
    pub token: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Provisioning,
    /// Cloud: a place in the queue.
    Waiting(u64),
    /// Cloud: it wants the transfer token (connect).
    ReadyToConnect,
    Provisioned,
    Failed(String),
}

/// The device the stream says it's on (xbox-xcloud-player's X-MS-Device-Info), at the picture size asked for.
pub fn device_info(w: u32, h: u32) -> String {
    json!({
        "appInfo": {"env": {"clientAppId": "Microsoft.GamingApp", "clientAppType": "native", "clientAppVersion": "2203.1001.4.0", "clientSdkVersion": "8.5.2", "httpEnvironment": "prod", "sdkInstallId": ""}},
        "dev": {"hw": {"make": "Microsoft", "model": "Surface Pro", "sdktype": "native"}, "os": {"name": "Windows 11", "ver": "22631.2715", "platform": "desktop"},
                "displayInfo": {"dimensions": {"widthInPixels": w, "heightInPixels": h}, "pixelDensity": {"dpiX": 1, "dpiY": 1}}}
    })
    .to_string()
}

/// The play request's body. 720p asks as a phone (android), 1080p as Windows: how the service picks the picture size.
pub fn play_body(kind: Kind, target: &str, hd: bool, offset_minutes: i64) -> Value {
    json!({
        "clientSessionId": "",
        "titleId": if kind == Kind::Cloud { target } else { "" },
        "systemUpdateGroup": "",
        "settings": {"nanoVersion": "V3;WebrtcTransport.dll", "enableTextToSpeech": false, "highContrast": 0, "locale": "en-US", "useIceConnection": false,
                     "timezoneOffsetMinutes": offset_minutes, "sdkType": "web", "osName": if hd { "windows" } else { "android" }},
        "serverId": if kind == Kind::Home { target } else { "" },
        "fallbackRegionNames": []
    })
}

pub fn play(s: &Stream, kind: Kind, target: &str, hd: bool) -> Result<Session, Fail> {
    let (w, h) = if hd { (1920, 1080) } else { (1280, 720) };
    let r = bearer(Req::new("POST", &format!("{}/v5/sessions/{}/play", s.base, kind.path())), s).header("X-MS-Device-Info", &device_info(w, h)).json(&play_body(kind, target, hd, 0));
    let v = http::send(&r)?.json();
    let id = v["sessionId"].as_str().map(String::from).or_else(|| v["sessionPath"].as_str().and_then(|p| p.rsplit('/').next()).map(String::from)).unwrap_or_default();
    if id.is_empty() {
        return Err(Fail::Other("the stream didn't start (no session)".into()));
    }
    Ok(Session { kind, base: s.base.clone(), token: s.token.clone(), id })
}

impl Session {
    fn url(&self, what: &str) -> String {
        format!("{}/v5/sessions/{}/{}{}", self.base, self.kind.path(), self.id, what)
    }
    fn req(&self, method: &str, what: &str) -> Req {
        bearer(Req::new(method, &self.url(what)), &Stream { token: self.token.clone(), base: self.base.clone(), until: 0 })
    }

    pub fn state(&self) -> Result<State, Fail> {
        let v = http::send(&self.req("GET", "/state"))?.json();
        Ok(state_from(&v))
    }

    pub fn connect(&self, transfer: &str) -> Result<(), Fail> {
        http::send(&self.req("POST", "/connect").json(&json!({"userToken": transfer}))).map(|_| ())
    }

    pub fn configuration(&self) -> Result<Value, Fail> {
        Ok(http::send(&self.req("GET", "/configuration"))?.json())
    }

    pub fn send_offer(&self, sdp: &str) -> Result<(), Fail> {
        http::send(&self.req("POST", "/sdp").json(&offer_body(sdp))).map(|_| ())
    }

    /// The Xbox's answer: None means "not yet, ask again" (204).
    pub fn answer(&self) -> Result<Option<String>, Fail> {
        let r = http::send(&self.req("GET", "/sdp"))?;
        Ok(if r.status == 204 { None } else { answer_sdp(&r.json()) })
    }

    pub fn send_ice(&self, candidates: &[Value]) -> Result<(), Fail> {
        http::send(&self.req("POST", "/ice").json(&json!({"iceCandidates": candidates}))).map(|_| ())
    }

    pub fn remote_ice(&self) -> Result<Option<Vec<Value>>, Fail> {
        let r = http::send(&self.req("GET", "/ice"))?;
        Ok(if r.status == 204 { None } else { Some(remote_candidates(&r.json())) })
    }

    pub fn keepalive(&self) -> Result<(), Fail> {
        let mut r = self.req("POST", "/keepalive");
        r.body = Some(String::new());
        http::send(&r).map(|_| ())
    }

    /// The end (Disconnect). The Xbox stays on and sleeps by itself later.
    pub fn stop(&self) {
        let _ = http::send(&self.req("DELETE", ""));
    }
}

pub fn state_from(v: &Value) -> State {
    match v["state"].as_str().unwrap_or("") {
        "Provisioned" => State::Provisioned,
        "Provisioning" | "" => State::Provisioning,
        "ReadyToConnect" => State::ReadyToConnect,
        "WaitingForResources" => State::Waiting(v.pointer("/waitingTimes/estimatedTotalWaitTimeInSeconds").and_then(Value::as_u64).unwrap_or(0)),
        "Failed" => {
            let m = v.pointer("/errorDetails/message").and_then(Value::as_str).or_else(|| v.pointer("/errorDetails/code").and_then(Value::as_str)).unwrap_or("it failed");
            State::Failed(m.to_string())
        }
        other => State::Failed(other.to_string()),
    }
}

/// The offer as xbox-xcloud-player sends it, with the channel versions it speaks.
pub fn offer_body(sdp: &str) -> Value {
    json!({
        "messageType": "offer",
        "sdp": sdp,
        "configuration": {
            "chatConfiguration": {"bytesPerSample": 2, "expectedClipDurationMs": 20, "format": {"codec": "opus", "container": "webm"}, "numChannels": 1, "sampleFrequencyHz": 24000},
            "chat": {"minVersion": 1, "maxVersion": 1},
            "control": {"minVersion": 1, "maxVersion": 3},
            "input": {"minVersion": 1, "maxVersion": 8},
            "message": {"minVersion": 1, "maxVersion": 1}
        }
    })
}

/// The answer's SDP (it's JSON inside a string: exchangeResponse).
pub fn answer_sdp(v: &Value) -> Option<String> {
    let inner: Value = serde_json::from_str(v["exchangeResponse"].as_str()?).ok()?;
    if inner["status"].as_str() == Some("failed") {
        return None;
    }
    inner["sdp"].as_str().map(String::from)
}

/// The Xbox's ICE candidates; "a=end-of-candidates" becomes the empty candidate.
pub fn remote_candidates(v: &Value) -> Vec<Value> {
    let inner: Value = v["exchangeResponse"].as_str().and_then(|s| serde_json::from_str(s).ok()).unwrap_or(Value::Null);
    let mut out = Vec::new();
    for c in inner.as_array().cloned().unwrap_or_default() {
        let mut c = c;
        if c["candidate"].as_str() == Some("a=end-of-candidates") {
            c["candidate"] = Value::String(String::new());
        }
        out.push(c);
    }
    out
}

/// Our offer, adjusted as xbox-xcloud-player does: stereo sound, and a bitrate cap (kbps) for Data saver.
pub fn munge_offer(sdp: &str, video_kbps: u32) -> String {
    let mut s = sdp.replace("useinbandfec=1", "useinbandfec=1; stereo=1");
    if video_kbps > 0 {
        let mut lines: Vec<String> = s.split("\r\n").map(String::from).collect();
        if let Some(i) = lines.iter().position(|l| l.starts_with("m=video")) {
            let mut j = i + 1;
            while j < lines.len() && (lines[j].starts_with("i=") || lines[j].starts_with("c=")) {
                j += 1;
            }
            if j < lines.len() && lines[j].starts_with("b=") {
                lines[j] = format!("b=AS:{video_kbps}");
            } else {
                lines.insert(j, format!("b=AS:{video_kbps}"));
            }
        }
        s = lines.join("\r\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stand_in() {
        http::set_test_answers(Some(serde_json::from_str(include_str!("../tests/fixtures/http.json")).unwrap()));
    }

    #[test]
    fn the_console_list() {
        stand_in();
        let s = Stream { token: "T".into(), base: "https://home.stand-in.test".into(), until: 0 };
        let w = Xsts { token: "W".into(), uhs: "1".into() };
        let c = consoles(&s, Some(&w)).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "Living room");
        assert_eq!(c[0].model, "Xbox Series X");
        assert_eq!(c[0].power, Power::Sleeping);
        assert!(c[0].remote_on);
        wake(&w, &c[0].id).unwrap();
        assert_eq!(power(&w, &c[0].id).unwrap(), Power::On);
        http::set_test_answers(None);
    }

    #[test]
    fn a_session_from_play_to_answer() {
        stand_in();
        let s = Stream { token: "T".into(), base: "https://home.stand-in.test".into(), until: 0 };
        let ses = play(&s, Kind::Home, "F4000000STANDIN", true).unwrap();
        assert_eq!(ses.id, "STANDIN-SESSION");
        assert_eq!(ses.state().unwrap(), State::Provisioning);
        assert_eq!(ses.state().unwrap(), State::Provisioned);
        ses.send_offer("v=0\r\n").unwrap();
        assert_eq!(ses.answer().unwrap(), None);
        assert!(ses.answer().unwrap().unwrap().starts_with("v=0"));
        ses.send_ice(&[json!({"candidate": "a=candidate:1 1 UDP 1 192.0.2.1 9002 typ host", "sdpMid": "0", "sdpMLineIndex": 0})]).unwrap();
        let ice = ses.remote_ice().unwrap().unwrap();
        assert_eq!(ice.len(), 2);
        assert_eq!(ice[1]["candidate"], "");
        ses.keepalive().unwrap();
        http::set_test_answers(None);
    }

    #[test]
    fn cloud_games() {
        stand_in();
        let s = Stream { token: "T".into(), base: "https://cloud.stand-in.test".into(), until: 0 };
        let g = games(&s, true).unwrap();
        assert_eq!(g.len(), 4);
        assert_eq!(g[0].name, "Lanternfall");
        assert_eq!(g[0].kind, "Adventure");
        http::set_test_answers(None);
    }

    #[test]
    fn states_and_the_offer() {
        assert_eq!(state_from(&json!({"state": "WaitingForResources", "waitingTimes": {"estimatedTotalWaitTimeInSeconds": 40}})), State::Waiting(40));
        assert_eq!(state_from(&json!({"state": "Failed", "errorDetails": {"code": "x", "message": "No console"}})), State::Failed("No console".into()));
        let o = offer_body("SDP");
        assert_eq!(o["configuration"]["input"]["maxVersion"], 8);
        let m = munge_offer("v=0\r\nm=audio 9 UDP\r\na=fmtp:111 minptime=10;useinbandfec=1\r\nm=video 9 UDP\r\nc=IN IP4 0.0.0.0\r\na=x", 6000);
        assert!(m.contains("useinbandfec=1; stereo=1"));
        assert!(m.contains("c=IN IP4 0.0.0.0\r\nb=AS:6000\r\na=x"));
        assert_eq!(model_words("XboxSeriesS"), "Xbox Series S");
        assert!(device_info(1280, 720).contains("\"widthInPixels\":1280"));
        assert_eq!(play_body(Kind::Home, "C1", false, 0)["settings"]["osName"], "android");
    }
}
