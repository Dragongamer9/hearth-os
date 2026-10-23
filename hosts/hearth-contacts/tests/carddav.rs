// Sync against a real CardDAV server. CI starts Radicale on this computer with a test account and an empty
// address book, and sets HEARTH_CARDDAV_URL, HEARTH_CARDDAV_USER and HEARTH_CARDDAV_PASS; without them this is skipped.
use hearth_contacts::carddav::{self, Client};
use hearth_contacts::store::{Account, Store, LOCAL};
use hearth_contacts::vcard::{self, Contact, Typed};
use url::Url;

fn env() -> Option<(String, String, String)> {
    Some((std::env::var("HEARTH_CARDDAV_URL").ok()?, std::env::var("HEARTH_CARDDAV_USER").ok()?, std::env::var("HEARTH_CARDDAV_PASS").ok()?))
}

fn person(given: &str, family: &str, phone: &str) -> Contact {
    let mut c = Contact::default();
    c.name.given = given.into();
    c.name.family = family.into();
    c.phones.push(Typed { kind: "cell".into(), value: phone.into(), pref: false, group: String::new() });
    c.groups.push("Test".into());
    c
}

fn names(s: &Store, book: &str) -> Vec<String> {
    let mut v: Vec<String> = s.list_book(book).into_iter().map(|e| e.contact.shown_name()).collect();
    v.sort();
    v
}

/// What the server has, read straight from it (not through sync).
fn server_cards(c: &Client, book: &Url) -> Vec<(String, Contact)> {
    let (_, items) = c.propfind(book, "1", "<d:getetag/><d:resourcetype/>").unwrap();
    let mut out = Vec::new();
    for it in items {
        if it.href.ends_with('/') { continue; }
        let u = book.join(&it.href).unwrap();
        let r = c.send("GET", &u, &[], None).unwrap();
        if let Some(card) = vcard::parse(&r.body).into_iter().next() { out.push((u.to_string(), card)); }
    }
    out
}

#[test]
fn sync_both_ways_with_a_real_server() {
    let Some((url, user, pass)) = env() else { eprintln!("HEARTH_CARDDAV_URL isn't set: skipped"); return };
    let root = std::env::temp_dir().join(format!("hearth-contacts-dav-{}", hearth_contacts::store::new_uid()));
    let s = Store::open(&root);

    // Finding the address book from the server's bare address (well-known / principal / home set).
    let books = carddav::discover(&url, &user, &pass).expect("discover");
    eprintln!("address books: {books:?}");
    assert!(!books.is_empty());
    let wrong = carddav::discover(&url, &user, "not-the-password");
    assert!(wrong.is_err(), "a wrong password was taken");

    let a = Account { id: "dav-test".into(), name: "Radicale".into(), provider: "other".into(), server: url.clone(), username: user.clone(),
        book_url: books[0].url.clone(), book_name: books[0].name.clone(), ..Default::default() };
    s.put_account(a.clone()).unwrap();
    let book = Url::parse(&a.book_url).unwrap();
    let client = Client::new(&user, &pass);

    // 1. New here → uploaded.
    let ann = s.save(&a.id, person("Ann", "Archer", "+1 555 0101")).unwrap();
    let mut bob = person("Bob", "Baker", "+1 555 0102");
    bob.photo = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==".into();
    let bob = s.save(&a.id, bob).unwrap();
    let r = carddav::sync(&s, &a, &pass).unwrap();
    eprintln!("1: {r:?}");
    assert_eq!(r.uploaded, 2);
    let on_server = server_cards(&client, &book);
    assert_eq!(on_server.len(), 2);
    let srv_bob = on_server.iter().find(|(_, c)| c.uid == bob.uid).expect("Bob on the server");
    assert_eq!(srv_bob.1.photo, bob.photo, "the photo survived the trip");

    // 2. Nothing changed → nothing moves.
    let r = carddav::sync(&s, &a, &pass).unwrap();
    assert_eq!((r.uploaded, r.downloaded, r.deleted_here, r.deleted_there), (0, 0, 0, 0), "{r:?}");

    // 3. Changed on the server (as another device would) → downloaded.
    let (ann_url, mut srv_ann) = on_server.iter().find(|(_, c)| c.uid == ann.uid).cloned().unwrap();
    srv_ann.emails.push(Typed { kind: "work".into(), value: "ann@example.org".into(), pref: false, group: String::new() });
    let put = client.send("PUT", &Url::parse(&ann_url).unwrap(), &[("Content-Type", "text/vcard")], Some(&vcard::write(&srv_ann))).unwrap();
    assert!((200..300).contains(&put.status), "PUT {}", put.status);
    // And a card made on "another device".
    let carol = Contact { uid: "carol-from-phone".into(), display: "Carol Cook".into(), ..person("Carol", "Cook", "+1 555 0103") };
    let put = client.send("PUT", &book.join("carol-from-phone.vcf").unwrap(), &[("Content-Type", "text/vcard"), ("If-None-Match", "*")], Some(&vcard::write(&carol))).unwrap();
    assert!((200..300).contains(&put.status));
    let r = carddav::sync(&s, &a, &pass).unwrap();
    eprintln!("3: {r:?}");
    assert_eq!(r.downloaded, 2);
    assert_eq!(s.get(&a.id, &ann.uid).unwrap().emails[0].value, "ann@example.org");
    assert_eq!(names(&s, &a.id), vec!["Ann Archer", "Bob Baker", "Carol Cook"]);

    // 4. Changed and deleted here → sent up.
    let mut b = s.get(&a.id, &bob.uid).unwrap();
    b.note = "Changed on this computer".into();
    s.save(&a.id, b).unwrap();
    s.delete(&a.id, "carol-from-phone").unwrap();
    let r = carddav::sync(&s, &a, &pass).unwrap();
    eprintln!("4: {r:?}");
    assert_eq!((r.uploaded, r.deleted_there), (1, 1));
    let on_server = server_cards(&client, &book);
    assert_eq!(on_server.len(), 2);
    assert_eq!(on_server.iter().find(|(_, c)| c.uid == bob.uid).unwrap().1.note, "Changed on this computer");

    // 5. Deleted on the server → deleted here.
    let (bob_url, _) = on_server.iter().find(|(_, c)| c.uid == bob.uid).cloned().unwrap();
    let del = client.send("DELETE", &Url::parse(&bob_url).unwrap(), &[], None).unwrap();
    assert!((200..300).contains(&del.status));
    let r = carddav::sync(&s, &a, &pass).unwrap();
    assert_eq!(r.deleted_here, 1, "{r:?}");
    assert_eq!(names(&s, &a.id), vec!["Ann Archer"]);

    // 6. Changed on both sides → the server's kept, this computer's saved next to it, both end up on the server.
    let on_server = server_cards(&client, &book);
    let (ann_url, mut srv_ann) = on_server.into_iter().find(|(_, c)| c.uid == ann.uid).unwrap();
    srv_ann.title = "Changed on the phone".into();
    client.send("PUT", &Url::parse(&ann_url).unwrap(), &[("Content-Type", "text/vcard")], Some(&vcard::write(&srv_ann))).unwrap();
    let mut mine = s.get(&a.id, &ann.uid).unwrap();
    mine.title = "Changed here".into();
    s.save(&a.id, mine).unwrap();
    let r = carddav::sync(&s, &a, &pass).unwrap();
    eprintln!("6: {r:?}");
    assert_eq!(r.conflicts, 1);
    assert_eq!(s.get(&a.id, &ann.uid).unwrap().title, "Changed on the phone");
    assert_eq!(names(&s, &a.id), vec!["Ann Archer", "Ann Archer (changed on this computer)"]);
    assert_eq!(server_cards(&client, &book).len(), 2);

    // 7. Taking the account away, keeping a copy on this computer.
    s.remove_account(&a.id, true).unwrap();
    assert_eq!(names(&s, LOCAL).len(), 2);
    let _ = std::fs::remove_dir_all(&root);
}
