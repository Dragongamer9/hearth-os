//! Web requests through curl (on every Hearth OS): JSON in and out, a timeout, no extra TLS code in Hearth.

use serde_json::Value;

pub struct Reply { pub status: u16, pub text: String }
impl Reply {
    pub fn ok(&self) -> bool { (200..300).contains(&self.status) }
    pub fn json(&self) -> Value { serde_json::from_str(&self.text).unwrap_or(Value::Null) }
}

pub fn request(method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>, timeout_s: u32) -> Result<Reply, String> {
    let t = timeout_s.to_string();
    let mut args: Vec<String> = vec!["-sS".into(), "-L".into(), "-m".into(), t, "-X".into(), method.into(), "-A".into(), "Hearth".into(), "-w".into(), "\n%{http_code}".into()];
    for (k, v) in headers { args.push("-H".into()); args.push(format!("{k}: {v}")); }
    if let Some(b) = body { args.push("--data-binary".into()); args.push(b.into()); }
    args.push(url.into());
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = crate::system::run_timeout("curl", &refs, std::time::Duration::from_secs(timeout_s as u64 + 5)).map_err(|e| format!("No connection ({e})"))?;
    let (text, code) = out.rsplit_once('\n').unwrap_or((&out, "0"));
    Ok(Reply { status: code.trim().parse().unwrap_or(0), text: text.to_string() })
}
pub fn get(url: &str, headers: &[(&str, &str)]) -> Result<Reply, String> { request("GET", url, headers, None, 30) }
pub fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<Value, String> {
    let r = get(url, headers)?;
    if !r.ok() { return Err(format!("HTTP {}", r.status)); }
    Ok(r.json())
}
pub fn post(url: &str, body: &str, headers: &[(&str, &str)]) -> Result<Reply, String> { request("POST", url, headers, Some(body), 30) }

pub fn urlencode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() { if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { o.push(b as char); } else { o.push_str(&format!("%{b:02X}")); } }
    o
}
/// The value of a query parameter in a URL.
pub fn query_param(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?')?.1.split('#').next()?;
    q.split('&').find_map(|kv| { let (k, v) = kv.split_once('=').unwrap_or((kv, "")); if k == key { Some(urldecode(v)) } else { None } })
}
/// Two hex digits ("2F") as their byte.
pub fn hex_byte(hi: u8, lo: u8) -> Option<u8> {
    let d = |c: u8| (c as char).to_digit(16);
    Some((d(hi)? * 16 + d(lo)?) as u8)
}
pub fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => { out.push(b' '); i += 1; }
            // Bytes, not &s[..]: a '%' before a non-ASCII letter ("%aé") cut the letter in half and panicked.
            b'%' if i + 2 < b.len() => { match hex_byte(b[i + 1], b[i + 2]) { Some(v) => { out.push(v); i += 3; } None => { out.push(b'%'); i += 1; } } }
            c => { out.push(c); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls() {
        assert_eq!(query_param("https://embed.gog.com/on_login_success?origin=client&code=ab%2Fc+d", "code").as_deref(), Some("ab/c d"));
        assert_eq!(query_param("https://x/?a=1", "b"), None);
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
        assert_eq!(urldecode("%aé%zz%+1"), "%aé%zz% 1");
        assert_eq!(urldecode("%C3%A9"), "é");
    }
}
