// vCard files as other apps write them (Google Contacts, iPhone/iCloud, an old Android phone's 2.1 with
// quoted-printable, an RFC 6350 4.0 card) are read, written back, and read again without losing anything.
use hearth_contacts::store::{Store, LOCAL};
use hearth_contacts::vcard;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn round_trip(name: &str) -> Vec<vcard::Contact> {
    let first = vcard::parse(&fixture(name));
    assert!(!first.is_empty(), "{name}: nothing read");
    let text = vcard::write_all(first.iter());
    let second = vcard::parse(&text);
    assert_eq!(first, second, "{name}: changed on the way through");
    assert_eq!(vcard::write_all(second.iter()), text, "{name}: writing isn't stable");
    for l in text.split("\r\n") { assert!(l.len() <= 75, "{name}: line over 75 bytes: {l}"); }
    first
}

#[test]
fn google() {
    let cs = round_trip("google.vcf");
    assert_eq!(cs.len(), 2);
    let g = &cs[0];
    assert_eq!(g.shown_name(), "Grace Hopper");
    assert_eq!(g.name.prefix, "Rear Admiral");
    assert_eq!(g.emails[0].kind, "work");
    assert_eq!(g.phones.len(), 2);
    assert_eq!(g.addresses[0].locality, "Arlington");
    assert_eq!(g.urls[0].value, "https://example.org/grace");
    assert_eq!(g.birthday, "1906-12-09");
    assert_eq!(g.groups, vec!["myContacts", "Pioneers"]);
}

#[test]
fn icloud() {
    let c = &round_trip("icloud.vcf")[0];
    assert_eq!(c.shown_name(), "Johnny Appleseed");
    assert_eq!(c.org, "Apple Inc.");
    assert!(c.emails[0].pref);
    assert_eq!(c.emails[0].group, "item1");
    assert_eq!(c.phones[1].kind, "cell");
    assert_eq!(c.addresses[0].kind, "work");
    assert_eq!(c.birthday, "--02-24", "Apple's year-less birthday");
    assert!(c.photo.starts_with("data:image/jpeg;base64,/9j/"));
    assert!(c.extra.iter().any(|l| l.starts_with("item1.X-ABLabel")));
    assert!(c.extra.iter().any(|l| l.starts_with("X-SOCIALPROFILE")));
    let text = vcard::write(c);
    assert!(text.contains("item1.EMAIL;"), "the label stays with its email:\n{text}");
}

#[test]
fn android_2_1() {
    let c = &round_trip("android-2.1.vcf")[0];
    assert_eq!(c.shown_name(), "Ayşe Öztürk");
    assert_eq!(c.name.family, "Öztürk");
    assert_eq!(c.phones[0].kind, "cell");
    assert!(c.phones[0].pref);
    assert_eq!(c.addresses[0].locality, "İstanbul");
    assert_eq!(c.addresses[0].country, "Türkiye");
    assert_eq!(c.note, "Uzun bir not, satır sonu ile\nikinci satır ve devamı");
}

#[test]
fn rfc6350_4_0() {
    let c = &round_trip("rfc6350-4.0.vcf")[0];
    assert_eq!(c.uid, "urn:uuid:4fbe8971-0bc3-424c-9c26-36c3e1eff6b1");
    assert_eq!(c.phones[0].kind, "home");
    assert!(c.phones[0].pref);
    assert_eq!(c.phones[0].value, "+1-555-555-5555;ext=5555");
    assert_eq!(c.birthday, "--04-12");
    assert!(c.photo.starts_with("data:image/png;base64,"));
    assert!(c.extra.iter().any(|l| l == "GENDER:F"));
}

#[test]
fn import_then_export_through_the_store() {
    let dir = std::env::temp_dir().join(format!("hearth-contacts-rt-{}", hearth_contacts::store::new_uid()));
    let s = Store::open(&dir);
    let mut n = 0;
    for f in ["google.vcf", "icloud.vcf", "android-2.1.vcf", "rfc6350-4.0.vcf"] { n += s.import(LOCAL, &fixture(f)).unwrap().added; }
    assert_eq!(n, 5);
    let out = s.export(&[]);
    let back = vcard::parse(&out);
    assert_eq!(back.len(), 5);
    let mut names: Vec<String> = back.iter().map(|c| c.shown_name()).collect();
    names.sort();
    assert_eq!(names, vec!["Ayşe Öztürk", "Dr. Ruth Example", "Grace Hopper", "Johnny Appleseed", "Linus"]);
    // Every contact as its own .vcf file.
    assert_eq!(std::fs::read_dir(s.book_dir(LOCAL)).unwrap().count(), 5);
    let _ = std::fs::remove_dir_all(&dir);
}
