//! Web requests through curl (on every Hearth OS): JSON in and out, a timeout, no extra TLS code in Hearth.

use serde_json::Value;

pub struct Reply { pub status: u16, pub text: String }
impl Reply {
    pub fn ok(&self) -> bool { (200..300).contains(&self.status) }
    pub fn json(&self) -> Value { serde_json::from_str(&self.text).unwrap_or(Value::Null) }
}

pub fn request(method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>, timeout_s: u32) -> Result<Reply, String> {
    let t = timeout_s.to_string();
    let args = ["-sS", "-L", "-m", &t, "-X", method, "-A", "Hearth", "-w", "\n%{http_code}", "-K", "-"];
    // The address, headers and body go to curl on its standard input (a config file), not on its command line:
    // every program on the PC, any user's, can read a command line, and these carry sign-ins (Epic, GOG and Xbox
    // tokens in headers, GOG's refresh token in the address, Microsoft's in the body).
    let cfg = curl_config(url, headers, body);
    let out = crate::system::run_input("curl", &args, Some(&cfg), std::time::Duration::from_secs(timeout_s as u64 + 5)).map_err(|e| format!("No connection ({e})"))?;
    let (text, code) = out.rsplit_once('\n').unwrap_or((&out, "0"));
    Ok(Reply { status: code.trim().parse().unwrap_or(0), text: text.to_string() })
}
/// A value for a curl config file: in double quotes, with \\ \" and line breaks escaped (curl's own rules).
pub fn curl_quote(v: &str) -> String {
    let mut o = String::with_capacity(v.len() + 2);
    o.push('"');
    for c in v.chars() {
        match c { '\\' => o.push_str("\\\\"), '"' => o.push_str("\\\""), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"), c => o.push(c) }
    }
    o.push('"');
    o
}
/// curl's config for one request: its address, headers and body.
pub fn curl_config(url: &str, headers: &[(&str, &str)], body: Option<&str>) -> String {
    let mut c = format!("url = {}\n", curl_quote(url));
    for (k, v) in headers { c.push_str(&format!("header = {}\n", curl_quote(&format!("{k}: {v}")))); }
    if let Some(b) = body { c.push_str(&format!("data-binary = {}\n", curl_quote(b))); }
    c
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
    #[test]
    fn secrets_go_to_curl_on_its_input() {
        assert_eq!(curl_quote(r#"a "b" \c"#), r#""a \"b\" \\c""#);
        assert_eq!(curl_quote("x\ny"), r#""x\ny""#);
        let c = curl_config("https://x/?t=1&r=2", &[("Authorization", "Bearer tok")], Some("a=1&b=\"2\""));
        assert_eq!(c, "url = \"https://x/?t=1&r=2\"\nheader = \"Authorization: Bearer tok\"\ndata-binary = \"a=1&b=\\\"2\\\"\"\n");
        // curl reads it: a local file through the same path, with a header that needs quoting.
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("page.txt");
        std::fs::write(&f, "hello").unwrap();
        let r = request("GET", &format!("file://{}", f.display()), &[("Authorization", "Bearer \"x\" y")], None, 5).unwrap();
        assert_eq!(r.text, "hello");
    }
}
