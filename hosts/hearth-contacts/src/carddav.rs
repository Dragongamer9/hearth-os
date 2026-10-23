// CardDAV (RFC 6352): finds a person's address books on their server and keeps a local copy of one in step, both
// ways. Works with Nextcloud, iCloud, Fastmail, Radicale, Baïkal and other CardDAV servers. The password is passed
// in by the caller (the app reads it from the system keyring) and only ever sent to the server, over HTTPS.
use crate::store::{fingerprint, new_uid, now_rev, Account, Store};
use crate::vcard;
use base64::Engine;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
use url::Url;

pub const USER_AGENT: &str = concat!("HearthContacts/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Dragongamer9/hearth-os)");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub server: &'static str,
    pub user_hint: &'static str,
    pub help: &'static str,
}

/// Where each provider's CardDAV server is, and what the person needs to sign in.
pub fn presets() -> Vec<Preset> {
    vec![
        Preset { id: "nextcloud", name: "Nextcloud", server: "https://", user_hint: "Your Nextcloud user name",
            help: "Type your Nextcloud's address (for example https://cloud.example.com). If you use two-step sign-in, make an app password in Nextcloud: Settings › Security › Devices & sessions." },
        Preset { id: "icloud", name: "iCloud", server: "https://contacts.icloud.com", user_hint: "Your Apple Account email",
            help: "iCloud needs an app-specific password, not your Apple Account password: make one at account.apple.com › Sign-In and Security › App-Specific Passwords." },
        Preset { id: "fastmail", name: "Fastmail", server: "https://carddav.fastmail.com", user_hint: "Your Fastmail email address",
            help: "Fastmail needs an app password: Settings › Privacy & Security › Manage app passwords › New app password, with access to Contacts (CardDAV)." },
        Preset { id: "other", name: "Other CardDAV server", server: "https://", user_hint: "User name",
            help: "Type the server's address. Hearth Contacts finds the address books on it." },
    ]
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookInfo {
    pub url: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub downloaded: usize,
    pub uploaded: usize,
    pub deleted_here: usize,
    pub deleted_there: usize,
    pub conflicts: usize,
    pub problems: Vec<String>,
}

// ------------------------------------------------------------------ a little XML tree

#[derive(Debug, Default, Clone)]
pub struct El {
    pub name: String,
    pub text: String,
    pub children: Vec<El>,
}

impl El {
    fn child(&self, name: &str) -> Option<&El> { self.children.iter().find(|c| c.name == name) }
    fn find(&self, name: &str) -> Option<&El> {
        for c in &self.children {
            if c.name == name { return Some(c); }
            if let Some(f) = c.find(name) { return Some(f); }
        }
        None
    }
    fn all<'a>(&'a self, name: &str, out: &mut Vec<&'a El>) {
        for c in &self.children { if c.name == name { out.push(c); } else { c.all(name, out); } }
    }
}

/// Element names without their namespace prefix (DAV: and CardDAV names don't clash for what's read here).
pub fn parse_xml(s: &str) -> Result<El, String> {
    let mut r = Reader::from_str(s);
    let mut stack: Vec<El> = vec![El::default()];
    let local = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    loop {
        match r.read_event() {
            Ok(Event::Start(e)) => stack.push(El { name: local(e.local_name().as_ref()), ..Default::default() }),
            Ok(Event::Empty(e)) => { let el = El { name: local(e.local_name().as_ref()), ..Default::default() }; stack.last_mut().unwrap().children.push(el); }
            Ok(Event::End(_)) => {
                if stack.len() < 2 { return Err("The server's answer isn't valid XML.".into()); }
                let el = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(el);
            }
            Ok(Event::Text(t)) => { let t = t.unescape().map_err(|e| e.to_string())?; stack.last_mut().unwrap().text.push_str(&t); }
            Ok(Event::CData(t)) => { let t = String::from_utf8_lossy(&t.into_inner()).into_owned(); stack.last_mut().unwrap().text.push_str(&t); }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("The server's answer isn't valid XML: {e}")),
            _ => {}
        }
    }
    Ok(stack.swap_remove(0))
}

/// One <response> of a multistatus: its href and the properties that came back with 200.
#[derive(Debug, Default)]
pub struct DavItem {
    pub href: String,
    pub props: Vec<El>,
}

impl DavItem {
    fn prop(&self, name: &str) -> Option<&El> { self.props.iter().find(|p| p.name == name) }
    fn text(&self, name: &str) -> String { self.prop(name).map(|p| p.text.trim().to_string()).unwrap_or_default() }
    fn is(&self, kind: &str) -> bool { self.prop("resourcetype").is_some_and(|r| r.child(kind).is_some()) }
    fn href_in(&self, name: &str) -> Option<String> { self.prop(name).and_then(|p| p.find("href")).map(|h| h.text.trim().to_string()) }
}

pub fn multistatus(xml: &str) -> Result<Vec<DavItem>, String> {
    let root = parse_xml(xml)?;
    let mut rs = Vec::new();
    root.all("response", &mut rs);
    Ok(rs.into_iter().map(|r| {
        let mut item = DavItem { href: r.child("href").map(|h| h.text.trim().to_string()).unwrap_or_default(), props: Vec::new() };
        let mut stats = Vec::new();
        r.all("propstat", &mut stats);
        for ps in stats {
            let ok = ps.child("status").map(|s| s.text.contains(" 200")).unwrap_or(true);
            if !ok { continue; }
            if let Some(p) = ps.child("prop") { item.props.extend(p.children.iter().cloned()); }
        }
        item
    }).collect())
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The same resource however the server spelled its path.
fn same_path(a: &str, b: &str) -> bool { pct_decode(a).trim_end_matches('/') == pct_decode(b).trim_end_matches('/') }

// ------------------------------------------------------------------ talking to the server

pub struct Client {
    agent: ureq::Agent,
    auth: String,
}

pub struct Resp {
    pub status: u16,
    pub etag: String,
    pub location: String,
    pub body: String,
    pub url: Url,
}

/// Only HTTPS, except for a server on this computer (for testing).
fn allowed(u: &Url) -> Result<(), String> {
    match (u.scheme(), u.host_str()) {
        ("https", Some(_)) => Ok(()),
        ("http", Some("localhost" | "127.0.0.1" | "[::1]")) => Ok(()),
        _ => Err("The server's address has to start with https://".into()),
    }
}

pub fn normalize_server(s: &str) -> Result<Url, String> {
    let s = s.trim();
    let s = if s.contains("://") { s.to_string() } else { format!("https://{s}") };
    let u = Url::parse(&s).map_err(|_| "That isn't a web address.".to_string())?;
    allowed(&u)?;
    Ok(u)
}

impl Client {
    pub fn new(user: &str, pass: &str) -> Client {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(60))
            .user_agent(USER_AGENT)
            .build();
        let auth = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}")));
        Client { agent, auth }
    }

    /// One request, following redirects (to the same kind of address) by hand so PROPFIND stays PROPFIND.
    pub fn send(&self, method: &str, url: &Url, headers: &[(&str, &str)], body: Option<&str>) -> Result<Resp, String> {
        let mut url = url.clone();
        for _ in 0..6 {
            allowed(&url)?;
            let mut req = self.agent.request_url(method, &url).set("Authorization", &self.auth);
            for (k, v) in headers { req = req.set(k, v); }
            let res = match body { Some(b) => req.send_string(b), None => req.call() };
            let resp = match res {
                Ok(r) => r,
                Err(ureq::Error::Status(_, r)) => r,
                Err(ureq::Error::Transport(t)) => return Err(format!("Couldn't reach the server ({}).", t.kind())),
            };
            let status = resp.status();
            let location = resp.header("Location").unwrap_or("").to_string();
            if (300..400).contains(&status) && status != 304 && !location.is_empty() {
                url = url.join(&location).map_err(|e| e.to_string())?;
                continue;
            }
            let etag = resp.header("ETag").unwrap_or("").to_string();
            let mut body = String::new();
            let _ = resp.into_reader().take(64 << 20).read_to_string(&mut body);
            if status == 401 { return Err("The server didn't take that user name and password. (iCloud and Fastmail need an app password.)".into()); }
            return Ok(Resp { status, etag, location, body, url });
        }
        Err("The server sent the request round in circles (too many redirects).".into())
    }

    pub fn propfind(&self, url: &Url, depth: &str, props: &str) -> Result<(Url, Vec<DavItem>), String> {
        let body = format!(r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:cs="http://calendarserver.org/ns/"><d:prop>{props}</d:prop></d:propfind>"#);
        let r = self.send("PROPFIND", url, &[("Depth", depth), ("Content-Type", "application/xml; charset=utf-8")], Some(&body))?;
        if r.status != 207 { return Err(format!("The server answered {} to a CardDAV question at {}.", r.status, url)); }
        Ok((r.url.clone(), multistatus(&r.body)?))
    }
}

/// Finds the person's address books, starting from the server's address (any of: the server, its /.well-known/carddav,
/// the principal, the address book home, or an address book itself).
pub fn discover(server: &str, user: &str, pass: &str) -> Result<Vec<BookInfo>, String> {
    let start = normalize_server(server)?;
    let c = Client::new(user, pass);
    let mut tries = vec![start.clone()];
    if let Ok(w) = start.join("/.well-known/carddav") { tries.push(w); }
    let mut principal: Option<Url> = None;
    let mut last_err = String::from("No CardDAV server answered there.");
    for t in &tries {
        match c.propfind(t, "0", "<d:current-user-principal/><d:resourcetype/><d:displayname/>") {
            Ok((base, items)) => {
                if let Some(it) = items.first() {
                    if it.is("addressbook") {
                        let u = base.join(&it.href).unwrap_or(base.clone());
                        let name = it.text("displayname");
                        return Ok(vec![BookInfo { url: u.to_string(), name: if name.is_empty() { "Contacts".into() } else { name } }]);
                    }
                    if let Some(h) = it.href_in("current-user-principal") { principal = base.join(&h).ok(); break; }
                }
            }
            Err(e) => { if e.contains("user name and password") { return Err(e); } last_err = e; }
        }
    }
    let principal = principal.ok_or(last_err)?;
    let (base, items) = c.propfind(&principal, "0", "<card:addressbook-home-set/>")?;
    let home = items.first().and_then(|i| i.href_in("addressbook-home-set")).and_then(|h| base.join(&h).ok()).unwrap_or(principal.clone());
    let (base, items) = c.propfind(&home, "1", "<d:resourcetype/><d:displayname/>")?;
    let mut books: Vec<BookInfo> = items.iter().filter(|i| i.is("addressbook")).filter_map(|i| {
        let u = base.join(&i.href).ok()?;
        let name = i.text("displayname");
        let fallback = u.path_segments().and_then(|s| s.filter(|x| !x.is_empty()).last().map(pct_decode)).unwrap_or_else(|| "Contacts".into());
        Some(BookInfo { url: u.to_string(), name: if name.is_empty() { fallback } else { name } })
    }).collect();
    books.dedup_by(|a, b| a.url == b.url);
    if books.is_empty() { return Err("Signed in, but there are no address books on that account.".into()); }
    Ok(books)
}

// ------------------------------------------------------------------ keeping a copy in step

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Item {
    pub href: String,
    pub etag: String,
    /// The fingerprint of the local file when it last matched the server.
    pub hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub items: HashMap<String, Item>,
}

fn load_state(store: &Store, id: &str) -> State {
    std::fs::read_to_string(store.sync_file(id)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_state(store: &Store, id: &str, s: &State) -> Result<(), String> {
    crate::store::write_atomic(&store.sync_file(id), serde_json::to_string(s).map_err(|e| e.to_string())?.as_bytes())
}

/// The cards on the server: href (as the server wrote it) → etag.
fn listing(c: &Client, book: &Url) -> Result<Vec<(String, String)>, String> {
    let (_, items) = c.propfind(book, "1", "<d:getetag/><d:resourcetype/>")?;
    Ok(items.into_iter()
        .filter(|i| !i.is("collection") && !same_path(&i.href, book.path()) && !i.href.is_empty())
        .map(|i| (i.href.clone(), i.text("getetag")))
        .collect())
}

/// The cards for these hrefs: in batches with addressbook-multiget, or one by one if the server doesn't do that.
fn fetch(c: &Client, book: &Url, hrefs: &[String]) -> Result<HashMap<String, (String, String)>, String> {
    let mut out: HashMap<String, (String, String)> = HashMap::new();
    for chunk in hrefs.chunks(100) {
        let body = format!(r#"<?xml version="1.0" encoding="utf-8"?><card:addressbook-multiget xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav"><d:prop><d:getetag/><card:address-data/></d:prop>{}</card:addressbook-multiget>"#,
            chunk.iter().map(|h| format!("<d:href>{}</d:href>", xml_escape(h))).collect::<String>());
        if let Ok(r) = c.send("REPORT", book, &[("Depth", "1"), ("Content-Type", "application/xml; charset=utf-8")], Some(&body)) {
            if r.status == 207 {
                for it in multistatus(&r.body)? {
                    let data = it.text("address-data");
                    if data.is_empty() { continue; }
                    if let Some(h) = chunk.iter().find(|h| same_path(h, &it.href)) { out.insert(h.clone(), (data, it.text("getetag"))); }
                }
            }
        }
    }
    for h in hrefs {
        if out.contains_key(h) { continue; }
        let u = book.join(h).map_err(|e| e.to_string())?;
        let r = c.send("GET", &u, &[("Accept", "text/vcard")], None)?;
        if r.status == 200 { out.insert(h.clone(), (r.body, r.etag)); }
    }
    Ok(out)
}

fn xml_escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }

/// Brings the local copy of the account's address book and the server's into step. Changes on only one side go to
/// the other; when a card changed on both, the server's is kept and the local one is saved next to it as a copy.
pub fn sync(store: &Store, a: &Account, pass: &str) -> Result<SyncReport, String> {
    let book = normalize_server(&a.book_url)?;
    let book = if book.path().ends_with('/') { book } else { Url::parse(&format!("{}/", book)).map_err(|e| e.to_string())? };
    let c = Client::new(&a.username, pass);
    let mut state = load_state(store, &a.id);
    let mut rep = SyncReport::default();

    let remote = listing(&c, &book)?;
    let local: HashMap<String, (String, vcard::Contact)> = store.read_book(&a.id).into_iter().map(|(_, text, ct)| (ct.uid.clone(), (fingerprint(&text), ct))).collect();
    let by_href: HashMap<String, String> = state.items.iter().map(|(uid, it)| (pct_decode(&it.href), uid.clone())).collect();

    // What changed on the server since last time (new or a different etag) is downloaded.
    let mut want: Vec<String> = Vec::new();
    let mut seen_uids: Vec<String> = Vec::new();
    for (href, etag) in &remote {
        match by_href.get(&pct_decode(href)) {
            Some(uid) => {
                seen_uids.push(uid.clone());
                let it = &state.items[uid];
                if it.etag.is_empty() || &it.etag != etag { want.push(href.clone()); }
            }
            None => want.push(href.clone()),
        }
    }
    let got = fetch(&c, &book, &want)?;
    for href in &want {
        let Some((text, etag)) = got.get(href) else { rep.problems.push(format!("Couldn't download {href}")); continue };
        let Some(mut card) = vcard::parse(text).into_iter().next() else { rep.problems.push(format!("{href} isn't a vCard")); continue };
        let known = by_href.get(&pct_decode(href)).cloned();
        if card.uid.is_empty() { card.uid = known.clone().unwrap_or_else(new_uid); }
        let etag = if etag.is_empty() { remote.iter().find(|(h, _)| h == href).map(|(_, e)| e.clone()).unwrap_or_default() } else { etag.clone() };
        // Changed here too since the last sync: keep this computer's version as a separate contact.
        if let Some(uid) = &known {
            if let (Some(it), Some((h, mine))) = (state.items.get(uid), local.get(uid)) {
                if &it.hash != h {
                    let mut copy = mine.clone();
                    copy.uid = new_uid();
                    copy.display = format!("{} (changed on this computer)", mine.shown_name());
                    copy.rev = now_rev();
                    store.write_contact(&a.id, &copy)?;
                    rep.conflicts += 1;
                }
            }
            if uid != &card.uid { if let Some(old) = state.items.remove(uid) { let _ = old; } let _ = store.delete(&a.id, uid); }
        }
        let written = store.write_contact(&a.id, &card)?;
        state.items.insert(card.uid.clone(), Item { href: href.clone(), etag, hash: fingerprint(&written) });
        if !seen_uids.contains(&card.uid) { seen_uids.push(card.uid.clone()); }
        rep.downloaded += 1;
    }

    // Gone from the server: gone here too, unless it was changed here (then it goes back up as a new card).
    let gone: Vec<String> = state.items.keys().filter(|u| !seen_uids.contains(u)).cloned().collect();
    for uid in gone {
        let it = state.items.remove(&uid).unwrap();
        match local.get(&uid) {
            Some((h, _)) if h != &it.hash => {}
            Some(_) => { let _ = store.delete(&a.id, &uid); rep.deleted_here += 1; }
            None => {}
        }
    }

    // Changed or deleted here (and not on the server): sent up.
    let now_local: HashMap<String, (String, vcard::Contact)> = store.read_book(&a.id).into_iter().map(|(_, text, ct)| (ct.uid.clone(), (fingerprint(&text), ct))).collect();
    let uids: Vec<String> = state.items.keys().cloned().collect();
    for uid in uids {
        let it = state.items[&uid].clone();
        let u = book.join(&it.href).map_err(|e| e.to_string())?;
        match now_local.get(&uid) {
            None => {
                let mut h = vec![];
                if !it.etag.is_empty() { h.push(("If-Match", it.etag.as_str())); }
                let r = c.send("DELETE", &u, &h, None)?;
                if (200..300).contains(&r.status) || r.status == 404 { state.items.remove(&uid); rep.deleted_there += 1; }
                else if r.status == 412 { state.items.get_mut(&uid).unwrap().etag.clear(); rep.problems.push(format!("{uid} changed on the server; it'll come back on the next sync")); }
                else { rep.problems.push(format!("The server answered {} when deleting a contact", r.status)); }
            }
            Some((h, ct)) if h != &it.hash => {
                let text = vcard::write(ct);
                let mut hd = vec![("Content-Type", "text/vcard; charset=utf-8")];
                if !it.etag.is_empty() { hd.push(("If-Match", it.etag.as_str())); }
                let r = c.send("PUT", &u, &hd, Some(&text))?;
                if (200..300).contains(&r.status) {
                    state.items.insert(uid.clone(), Item { href: it.href.clone(), etag: r.etag.clone(), hash: h.clone() });
                    rep.uploaded += 1;
                } else if r.status == 412 { state.items.get_mut(&uid).unwrap().etag.clear(); rep.problems.push(format!("{} changed on the server too; the next sync sorts it out", ct.shown_name())); }
                else { rep.problems.push(format!("The server answered {} when saving {}", r.status, ct.shown_name())); }
            }
            Some(_) => {}
        }
    }

    // New here: sent up as new cards.
    for (uid, (h, ct)) in &now_local {
        if state.items.contains_key(uid) { continue; }
        let href = format!("{}{}", book.path(), crate::store::file_name(uid));
        let u = book.join(&href).map_err(|e| e.to_string())?;
        let r = c.send("PUT", &u, &[("Content-Type", "text/vcard; charset=utf-8"), ("If-None-Match", "*")], Some(&vcard::write(ct)))?;
        if (200..300).contains(&r.status) {
            state.items.insert(uid.clone(), Item { href, etag: r.etag.clone(), hash: h.clone() });
            rep.uploaded += 1;
        } else {
            rep.problems.push(format!("The server answered {} when adding {}", r.status, ct.shown_name()));
        }
    }

    save_state(store, &a.id, &state)?;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_multistatus() {
        let x = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
 <d:response><d:href>/dav/addressbooks/users/ann/contacts/</d:href>
  <d:propstat><d:prop><d:resourcetype><d:collection/><card:addressbook/></d:resourcetype><d:displayname>Contacts &amp; friends</d:displayname></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
  <d:propstat><d:prop><d:getetag/></d:prop><d:status>HTTP/1.1 404 Not Found</d:status></d:propstat>
 </d:response>
 <d:response><d:href>/dav/addressbooks/users/ann/contacts/a%20b.vcf</d:href>
  <d:propstat><d:prop><d:getetag>"123"</d:getetag><card:address-data><![CDATA[BEGIN:VCARD
FN:X
END:VCARD]]></card:address-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
 </d:response>
</d:multistatus>"#;
        let items = multistatus(x).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items[0].is("addressbook") && items[0].is("collection"));
        assert_eq!(items[0].text("displayname"), "Contacts & friends");
        assert!(items[0].prop("getetag").is_none());
        assert_eq!(items[1].text("getetag"), "\"123\"");
        assert!(items[1].text("address-data").contains("FN:X"));
        assert!(same_path(&items[1].href, "/dav/addressbooks/users/ann/contacts/a b.vcf"));
    }

    #[test]
    fn only_https_except_on_this_computer() {
        assert!(normalize_server("cloud.example.com").unwrap().as_str().starts_with("https://cloud.example.com"));
        assert!(normalize_server("http://cloud.example.com").is_err());
        assert!(normalize_server("http://127.0.0.1:5232/").is_ok());
    }
}
