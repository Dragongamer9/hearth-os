// Address books from Online accounts (online.rs), with a stand-in `hearth-accounts` (a script answering as the real
// one does: hearth_accounts.py has its own tests against fake servers) and fake CardDAV servers on this computer:
//   * a tiny one here that checks an OAuth token arrives as "Bearer" (Google accounts sign in with a token);
//   * with HEARTH_CARDDAV_URL/USER/PASS (CI's Radicale, a throwaway account), a real two-way sync of an Online
//     accounts address book signed in with what `hearth-accounts credentials` gives.
use hearth_contacts::carddav::{self, Client, Cred};
use hearth_contacts::online;
use hearth_contacts::store::{Account, Store};
use hearth_contacts::vcard::{self, Contact, Typed};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;

fn tmp_store() -> Store {
    Store::open(std::env::temp_dir().join(format!("hearth-contacts-online-{}", hearth_contacts::store::new_uid())))
}

#[test]
fn a_token_is_sent_as_bearer() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let (mut auth, mut len) = (String::new(), 0usize);
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).unwrap_or(0) == 0 { break; }
            let t = line.trim_end();
            if t.is_empty() { break; }
            if let Some((k, v)) = t.split_once(':') {
                if k.eq_ignore_ascii_case("authorization") { auth = v.trim().to_string(); }
                if k.eq_ignore_ascii_case("content-length") { len = v.trim().parse().unwrap_or(0); }
            }
        }
        let mut body = vec![0u8; len];
        let _ = std::io::Read::read_exact(&mut r, &mut body);
        let ms = String::from(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav"><d:response><d:href>/book/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/><card:addressbook/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#);
        let _ = write!(s, "HTTP/1.1 207 Multi-Status\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{ms}", ms.len());
        auth
    });
    let s = tmp_store();
    let a = Account { id: "oa-bearer".into(), name: "Google".into(), provider: "online".into(), book_url: format!("http://127.0.0.1:{port}/book/"),
        online: "google-1".into(), ..Default::default() };
    s.put_account(a.clone()).unwrap();
    let r = carddav::sync_with(&s, &a, &Cred::Token { token: "test-token-not-real".into() }).expect("sync");
    assert_eq!((r.downloaded, r.uploaded), (0, 0));
    assert_eq!(seen.join().unwrap(), "Bearer test-token-not-real");
    let _ = std::fs::remove_dir_all(&s.root);
}

/// A stand-in hearth-accounts: answers from JSON files in DIR (addressbooks.json, list.json, credentials.json).
fn fake_accounts(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("hearth-accounts");
    std::fs::write(&p, format!("#!/bin/sh\nd='{}'\ncase \"$1\" in\n  addressbooks) cat \"$d/addressbooks.json\" ;;\n  list) cat \"$d/list.json\" ;;\n  credentials) cat \"$d/credentials.json\" ;;\n  *) echo 'error: unknown command' ;;\nesac\n", dir.display())).unwrap();
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap(); }
    p
}

fn person(given: &str, family: &str) -> Contact {
    let mut c = Contact::default();
    c.name.given = given.into();
    c.name.family = family.into();
    c.phones.push(Typed { kind: "cell".into(), value: "+1 555 0199".into(), pref: false, group: String::new() });
    c
}

/// The only test that sets HEARTH_ACCOUNTS_BIN (tests run side by side).
#[test]
fn online_accounts_address_books() {
    let s = tmp_store();
    let dir = s.root.join("fake-bin");
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("HEARTH_ACCOUNTS_BIN", fake_accounts(&dir));
    let radicale = std::env::var("HEARTH_CARDDAV_URL").ok().zip(std::env::var("HEARTH_CARDDAV_USER").ok()).zip(std::env::var("HEARTH_CARDDAV_PASS").ok());
    // Family: on CI's Radicale when it's there (an address book of its own), otherwise just an address.
    let family = match &radicale { Some(((u, user), _)) => format!("{}/{user}/family/", u.trim_end_matches('/')), None => "https://cloud.example.com/dav/family/".into() };
    let w = |n: &str, v: serde_json::Value| std::fs::write(dir.join(n), format!("{v}\n")).unwrap();
    w("addressbooks.json", serde_json::json!({"addressbooks": [
        {"url": "https://Dup.example.com/books/work", "name": "Work", "account": "dav-aa11", "accountName": "Work server", "kind": "dav", "user": "ann"},
        {"url": family.clone(), "name": "Family", "account": "nextcloud-bb22", "accountName": "Nextcloud", "kind": "nextcloud", "user": "ann"}],
        "errors": []}));
    match &radicale {
        Some(((_, user), pass)) => w("credentials.json", serde_json::json!({"user": user, "password": pass})),
        None => w("credentials.json", serde_json::json!({"user": "ann", "password": "not-a-real-password"})),
    }
    // Contacts' own account with the same address book as "Work server".
    s.put_account(Account { id: "dav-own".into(), name: "Work".into(), provider: "other".into(), server: "https://dup.example.com".into(),
        username: "ann".into(), book_url: "https://dup.example.com/books/work/".into(), book_name: "Work".into(), ..Default::default() }).unwrap();

    let r = online::refresh(&s).expect("refresh");
    assert!(r.available);
    assert_eq!(r.added, vec!["Nextcloud · Family".to_string()]);
    assert_eq!(r.duplicates.len(), 1, "{r:?}");
    assert_eq!((r.duplicates[0].own.as_str(), r.duplicates[0].account.as_str()), ("dav-own", "dav-aa11"));
    let ids: Vec<String> = s.accounts().into_iter().map(|a| a.id).collect();
    assert_eq!(ids.len(), 2, "the shared address book is shown once: {ids:?}");
    let fam = s.accounts().into_iter().find(|a| a.online == "nextcloud-bb22").expect("Family added");
    assert_eq!(fam.id, online::book_id("nextcloud-bb22", &family));
    assert!(s.books().iter().any(|b| b.id == fam.id && b.kind == "online"));
    // Asking again adds nothing more; still offered (not answered yet).
    let r = online::refresh(&s).unwrap();
    assert!(r.added.is_empty());
    assert_eq!(r.duplicates.len(), 1);
    // "Keep them apart" is remembered; joining works after it too, and keeps the account's copy.
    online::decline(&s, "dav-own", "dav-aa11").unwrap();
    assert!(online::refresh(&s).unwrap().duplicates.is_empty());
    s.save("dav-own", person("Wendy", "Work")).unwrap();
    online::merge(&s, "dav-own", "dav-aa11").unwrap();
    let own = s.accounts().into_iter().find(|a| a.id == "dav-own").unwrap();
    assert_eq!(own.online, "dav-aa11");
    assert_eq!(own.name, "Work server");
    assert_eq!(s.list_book("dav-own").len(), 1, "joining kept the contacts here");
    assert!(online::merge(&s, "dav-own", "dav-aa11").is_err());
    // The password comes from hearth-accounts.
    match online::credentials("nextcloud-bb22").unwrap() { Cred::Password { user, .. } => assert!(!user.is_empty()), Cred::Token { .. } => panic!("a token") }

    // Synced both ways, signed in through Online accounts (CI's Radicale).
    if let Some(((_, user), pass)) = &radicale {
        let ann = s.save(&fam.id, person("Fiona", "Family")).unwrap();
        let cred = online::credentials(&fam.online).unwrap();
        let rep = carddav::sync_with(&s, &fam, &cred).expect("sync");
        eprintln!("online sync: {rep:?}");
        assert_eq!(rep.uploaded, 1);
        let c = Client::new(user, pass);
        let book = Url::parse(&family).unwrap();
        let (_, items) = c.propfind(&book, "1", "<d:getetag/>").unwrap();
        let mut found = false;
        for it in items.iter().filter(|i| !i.href.ends_with('/')) {
            let r = c.send("GET", &book.join(&it.href).unwrap(), &[], None).unwrap();
            if vcard::parse(&r.body).iter().any(|x| x.uid == ann.uid) { found = true; }
        }
        assert!(found, "the contact reached the server");
        eprintln!("an Online accounts address book synced with Radicale");
    }

    // An account that can't be reached just now isn't taken for gone.
    w("addressbooks.json", serde_json::json!({"addressbooks": [{"url": family.clone(), "name": "Family", "account": "nextcloud-bb22", "accountName": "Nextcloud", "kind": "nextcloud", "user": "ann"}],
        "errors": [{"account": "dav-aa11", "name": "Work server", "error": "Couldn't reach it"}]}));
    let r = online::refresh(&s).unwrap();
    assert_eq!(r.errors, vec!["Work server: Couldn't reach it".to_string()]);
    assert!(!s.accounts().into_iter().find(|a| a.id == "dav-own").unwrap().paused);
    // The account leaves Online accounts (or Contacts is switched off there): the copy stays, paused.
    w("addressbooks.json", serde_json::json!({"addressbooks": [], "errors": []}));
    online::refresh(&s).unwrap();
    let f = s.accounts().into_iter().find(|a| a.id == fam.id).unwrap();
    assert!(f.paused && !f.last_error.is_empty());
    assert!(!s.list_book(&fam.id).is_empty() || radicale.is_none(), "the copy is kept");
    std::env::remove_var("HEARTH_ACCOUNTS_BIN");
    let _ = std::fs::remove_dir_all(&s.root);
}
