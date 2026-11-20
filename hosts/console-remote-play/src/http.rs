//! HTTPS to Microsoft's services (ureq with rustls): a time limit on every request, tokens only in headers (never on a
//! command line, never in a log), nothing kept on disk. Each call blocks its own short-lived thread (the window's
//! thread never waits on the network).
//!
//! CI's stand-in (HEARTH_CRP_FIXTURE=DIR): the answers come from DIR/http.json instead of the network, keyed
//! "METHOD host/path" (no query). A value is one answer {"status", "headers", "body"} or a list of them, given in turn
//! (the last one repeats): `{"status": 204}` then `{"status": 200, "body": …}` is "ask again", then the answer.
//! `{"offline": true}` is no connection.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Fail {
    /// No connection (nothing answered in time).
    Offline,
    /// It answered with this status (and the start of what it said, which never holds a token of ours).
    Status(u16, String),
    Other(String),
}

impl Fail {
    pub fn words(&self) -> String {
        match self {
            Fail::Offline => "This device couldn't reach Microsoft (no internet, or it timed out).".into(),
            Fail::Status(401, _) | Fail::Status(403, _) => "Microsoft said no to this sign-in. Sign in again.".into(),
            Fail::Status(c, _) => format!("Microsoft's service answered with an error ({c})."),
            Fail::Other(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Req {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// Seconds (default 25).
    pub timeout: u64,
}

impl Req {
    pub fn new(method: &str, url: &str) -> Req {
        Req { method: method.into(), url: url.into(), headers: Vec::new(), body: None, timeout: 25 }
    }
    pub fn header(mut self, k: &str, v: &str) -> Req {
        self.headers.push((k.into(), v.into()));
        self
    }
    pub fn json(mut self, v: &Value) -> Req {
        self.body = Some(v.to_string());
        self.headers.push(("Content-Type".into(), "application/json".into()));
        self
    }
    pub fn form(mut self, pairs: &[(&str, &str)]) -> Req {
        let s: Vec<String> = pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect();
        self.body = Some(s.join("&"));
        self.headers.push(("Content-Type".into(), "application/x-www-form-urlencoded".into()));
        self
    }
    /// "METHOD host/path", the stand-in's key.
    pub fn key(&self) -> String {
        let rest = self.url.split("://").nth(1).unwrap_or(&self.url);
        let rest = rest.split('?').next().unwrap_or(rest);
        format!("{} {}", self.method.to_uppercase(), rest)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Resp {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone())
    }
}

/// Percent-encoding for forms and queries (RFC 3986's unreserved characters stay).
pub fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// Percent-decoding (a redirect's query).
pub fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                o.push(v);
                i += 3;
                continue;
            }
        }
        o.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&o).to_string()
}

/// One query parameter of a URL.
pub fn query(url: &str, name: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    let q = q.split('#').next().unwrap_or(q);
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (dec(k) == name).then(|| dec(v))
    })
}

static TURNS: Mutex<Option<HashMap<String, usize>>> = Mutex::new(None);
static TEST_ANSWERS: Mutex<Option<Value>> = Mutex::new(None);

/// Tests: answer from this map instead of the network (the same shape as http.json).
pub fn set_test_answers(v: Option<Value>) {
    *TEST_ANSWERS.lock().unwrap() = v;
    *TURNS.lock().unwrap() = None;
}

fn stand_in() -> Option<Value> {
    if let Some(v) = TEST_ANSWERS.lock().unwrap().clone() {
        return Some(v);
    }
    let dir = crate::fixture()?;
    let b = std::fs::read(dir.join("http.json")).ok()?;
    serde_json::from_slice(&b).ok()
}

fn answer_from(all: &Value, r: &Req) -> Result<Resp, Fail> {
    let key = r.key();
    let Some(v) = all.get(&key) else { return Err(Fail::Status(404, format!("the stand-in has no answer for {key}"))) };
    let one = match v {
        Value::Array(list) if !list.is_empty() => {
            let mut g = TURNS.lock().unwrap();
            let turns = g.get_or_insert_with(HashMap::new);
            let n = turns.entry(key.clone()).or_insert(0);
            let pick = list[(*n).min(list.len() - 1)].clone();
            *n += 1;
            pick
        }
        other => other.clone(),
    };
    if one.get("offline").and_then(Value::as_bool).unwrap_or(false) {
        return Err(Fail::Offline);
    }
    let status = one.get("status").and_then(Value::as_u64).unwrap_or(200) as u16;
    let headers = one.get("headers").and_then(Value::as_object).map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default();
    let body = match one.get("body") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    };
    let resp = Resp { status, headers, body };
    if status >= 400 {
        return Err(Fail::Status(status, resp.body.chars().take(200).collect()));
    }
    Ok(resp)
}

/// Sends it and waits (on the caller's thread: call it from a worker).
pub fn send(r: &Req) -> Result<Resp, Fail> {
    if let Some(all) = stand_in() {
        return answer_from(&all, r);
    }
    if !r.url.starts_with("https://") {
        return Err(Fail::Other("only https is used".into()));
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(r.timeout.max(5)))
        .user_agent("Hearth Console Remote Play")
        .build();
    let mut q = agent.request(&r.method, &r.url);
    for (k, v) in &r.headers {
        q = q.set(k, v);
    }
    let res = match &r.body {
        Some(b) => q.send_string(b),
        None => q.call(),
    };
    let read = |x: ureq::Response| -> Resp {
        let status = x.status();
        let headers: Vec<(String, String)> = x.headers_names().into_iter().filter_map(|n| x.header(&n).map(|v| (n.clone(), v.to_string()))).collect();
        let body = x.into_string().unwrap_or_default();
        Resp { status, headers, body }
    };
    match res {
        Ok(x) => Ok(read(x)),
        Err(ureq::Error::Status(code, x)) => {
            let body: String = read(x).body.chars().take(200).collect();
            Err(Fail::Status(code, if crate::looks_secret(&body) { String::new() } else { body }))
        }
        Err(ureq::Error::Transport(_)) => Err(Fail::Offline),
    }
}

/// `send` on its own thread; the future is ready when it answered.
pub fn spawn<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("crp-net".into()).spawn(move || {
        let _ = tx.send(work());
    });
    async move { rx.await.ok() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn forms_and_queries() {
        assert_eq!(enc("a b&c=d/é"), "a%20b%26c%3Dd%2F%C3%A9");
        assert_eq!(query("ms-xal-000000004c20a908://auth?code=M.R3_BAY&state=a%2Bb", "code").as_deref(), Some("M.R3_BAY"));
        assert_eq!(query("ms-xal-000000004c20a908://auth?code=M.R3_BAY&state=a%2Bb", "state").as_deref(), Some("a+b"));
        assert_eq!(query("x://y?error=access_denied", "code"), None);
        assert_eq!(Req::new("get", "https://h.test/v6/servers/home?x=1").key(), "GET h.test/v6/servers/home");
    }

    #[test]
    fn the_stand_in_answers_in_turn() {
        set_test_answers(Some(json!({
            "GET h.test/state": [{"status": 200, "body": {"state": "Provisioning"}}, {"status": 200, "body": {"state": "Provisioned"}}],
            "GET h.test/off": {"offline": true}
        })));
        let r = Req::new("GET", "https://h.test/state");
        assert_eq!(send(&r).unwrap().json()["state"], "Provisioning");
        assert_eq!(send(&r).unwrap().json()["state"], "Provisioned");
        assert_eq!(send(&r).unwrap().json()["state"], "Provisioned");
        assert_eq!(send(&Req::new("GET", "https://h.test/off")).unwrap_err(), Fail::Offline);
        set_test_answers(None);
    }
}
