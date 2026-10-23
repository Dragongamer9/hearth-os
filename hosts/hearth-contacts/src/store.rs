// Where contacts live: one .vcf file per contact, in a folder per address book.
//   ~/.local/share/hearth-contacts/books/local/      "On this computer"
//   ~/.local/share/hearth-contacts/books/<account>/  a copy of a CardDAV address book (carddav.rs keeps it in step)
//   ~/.local/share/hearth-contacts/accounts.json     the CardDAV accounts (never their passwords: those are in the
//                                                     system keyring, secrets.rs)
//   ~/.local/share/hearth-contacts/sync/<account>.json  what each server had at the last sync
// HEARTH_CONTACTS_DIR moves all of it (the tests use that).
use crate::vcard::{self, Contact};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const LOCAL: &str = "local";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    /// What the person called it ("Nextcloud", "iCloud"...).
    pub name: String,
    /// nextcloud, icloud, fastmail or other: only for the icon and the help text.
    pub provider: String,
    pub server: String,
    pub username: String,
    pub book_url: String,
    pub book_name: String,
    pub last_sync: String,
    pub last_error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub book: String,
    pub contact: Contact,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
}

pub struct Store {
    pub root: PathBuf,
}

/// FNV-1a: a quick fingerprint of a card's text, to see whether it changed since the last sync.
pub fn fingerprint(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}

/// A new UID (random, in the usual UUID form).
pub fn new_uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// The time now as a vCard REV (20261022T101010Z).
pub fn now_rev() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's civil_from_days).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// A UID as a file name: anything unusual becomes "_" (and a fingerprint keeps different UIDs apart).
pub fn file_name(uid: &str) -> String {
    let clean: String = uid.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.@".contains(c) { c } else { '_' }).collect();
    let clean = clean.trim_start_matches('.').to_string();
    if clean == uid && !clean.is_empty() && clean.len() <= 120 { format!("{clean}.vcf") }
    else { format!("{}-{}.vcf", clean.chars().take(80).collect::<String>(), &fingerprint(uid)[..8]) }
}

fn valid_book_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Store {
    pub fn open_default() -> Store {
        let root = std::env::var_os("HEARTH_CONTACTS_DIR").map(PathBuf::from)
            .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("hearth-contacts"));
        Store::open(root)
    }

    pub fn open(root: impl Into<PathBuf>) -> Store {
        let s = Store { root: root.into() };
        let _ = fs::create_dir_all(s.book_dir(LOCAL));
        s
    }

    pub fn book_dir(&self, book: &str) -> PathBuf { self.root.join("books").join(book) }
    fn accounts_file(&self) -> PathBuf { self.root.join("accounts.json") }
    pub fn sync_file(&self, account: &str) -> PathBuf { self.root.join("sync").join(format!("{account}.json")) }

    // ---------------------------------------------------------------- accounts

    pub fn accounts(&self) -> Vec<Account> {
        fs::read_to_string(self.accounts_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save_accounts(&self, list: &[Account]) -> Result<(), String> {
        write_atomic(&self.accounts_file(), serde_json::to_string_pretty(list).map_err(|e| e.to_string())?.as_bytes())
    }

    pub fn put_account(&self, a: Account) -> Result<(), String> {
        if !valid_book_id(&a.id) || a.id == LOCAL { return Err("That account id can't be used.".into()); }
        let mut list = self.accounts();
        match list.iter_mut().find(|x| x.id == a.id) { Some(x) => *x = a.clone(), None => list.push(a.clone()) }
        fs::create_dir_all(self.book_dir(&a.id)).map_err(|e| e.to_string())?;
        self.save_accounts(&list)
    }

    /// Takes the account away, and its copy of the address book unless `keep_copy` (then its contacts move to
    /// "On this computer").
    pub fn remove_account(&self, id: &str, keep_copy: bool) -> Result<(), String> {
        if !valid_book_id(id) || id == LOCAL { return Err("Not an account.".into()); }
        if keep_copy {
            for e in self.list_book(id) {
                let mut c = e.contact;
                if self.get(LOCAL, &c.uid).is_some() { c.uid = new_uid(); }
                self.write_contact(LOCAL, &c)?;
            }
        }
        let _ = fs::remove_dir_all(self.book_dir(id));
        let _ = fs::remove_file(self.sync_file(id));
        let list: Vec<Account> = self.accounts().into_iter().filter(|a| a.id != id).collect();
        self.save_accounts(&list)
    }

    // ---------------------------------------------------------------- contacts

    pub fn books(&self) -> Vec<Book> {
        let mut out = vec![Book { id: LOCAL.into(), name: "On this computer".into(), kind: "local".into(), count: self.count(LOCAL) }];
        for a in self.accounts() {
            out.push(Book { id: a.id.clone(), name: if a.book_name.is_empty() { a.name.clone() } else { format!("{} · {}", a.name, a.book_name) }, kind: "carddav".into(), count: self.count(&a.id) });
        }
        out
    }

    fn count(&self, book: &str) -> usize {
        fs::read_dir(self.book_dir(book)).map(|r| r.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "vcf")).count()).unwrap_or(0)
    }

    /// Every contact in one address book, with the text of its file (for sync).
    pub fn read_book(&self, book: &str) -> Vec<(PathBuf, String, Contact)> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(self.book_dir(book)) else { return out };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "vcf") { continue; }
            let Ok(text) = fs::read_to_string(&p) else { continue };
            if let Some(mut c) = vcard::parse(&text).into_iter().next() {
                // A card without a UID (copied in by hand) gets its file's name as one.
                if c.uid.is_empty() { c.uid = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(new_uid); }
                out.push((p, text, c));
            }
        }
        out
    }

    pub fn list_book(&self, book: &str) -> Vec<Entry> {
        self.read_book(book).into_iter().map(|(_, _, c)| Entry { book: book.to_string(), contact: c }).collect()
    }

    /// All contacts in every address book, sorted by name.
    pub fn list(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = self.books().iter().flat_map(|b| self.list_book(&b.id)).collect();
        out.sort_by_cached_key(|e| e.contact.shown_name().to_lowercase());
        out
    }

    pub fn get(&self, book: &str, uid: &str) -> Option<Contact> {
        let p = self.book_dir(book).join(file_name(uid));
        if let Some(c) = fs::read_to_string(&p).ok().and_then(|t| vcard::parse(&t).into_iter().next()) { if c.uid == uid || c.uid.is_empty() { return Some(c); } }
        self.read_book(book).into_iter().map(|(_, _, c)| c).find(|c| c.uid == uid)
    }

    fn path_of(&self, book: &str, uid: &str) -> PathBuf {
        let p = self.book_dir(book).join(file_name(uid));
        if p.exists() { return p; }
        self.read_book(book).into_iter().find(|(_, _, c)| c.uid == uid).map(|(p, _, _)| p).unwrap_or(p)
    }

    /// Writes a contact exactly as given (sync uses this; it doesn't touch REV). Returns the file's text.
    pub fn write_contact(&self, book: &str, c: &Contact) -> Result<String, String> {
        if !valid_book_id(book) { return Err("No such address book.".into()); }
        if c.uid.is_empty() { return Err("A contact needs a UID.".into()); }
        let text = vcard::write(c);
        let p = self.path_of(book, &c.uid);
        fs::create_dir_all(self.book_dir(book)).map_err(|e| e.to_string())?;
        write_atomic(&p, text.as_bytes())?;
        Ok(text)
    }

    /// Saves a contact the person changed or made: a new one gets a UID, and REV is now.
    pub fn save(&self, book: &str, mut c: Contact) -> Result<Contact, String> {
        if book != LOCAL && !self.accounts().iter().any(|a| a.id == book) { return Err("No such address book.".into()); }
        if c.uid.trim().is_empty() { c.uid = new_uid(); }
        if c.shown_name().is_empty() { return Err("Give the contact a name, a phone number or an email address.".into()); }
        c.rev = now_rev();
        self.write_contact(book, &c)?;
        Ok(c)
    }

    pub fn delete(&self, book: &str, uid: &str) -> Result<(), String> {
        let p = self.path_of(book, uid);
        if !p.exists() { return Err("That contact is already gone.".into()); }
        fs::remove_file(p).map_err(|e| e.to_string())
    }

    /// Moves a contact to another address book (same UID, so a CardDAV book that already had it isn't doubled).
    pub fn move_to(&self, from: &str, uid: &str, to: &str) -> Result<(), String> {
        let c = self.get(from, uid).ok_or("That contact is gone.")?;
        self.save(to, c)?;
        if from != to { self.delete(from, uid)?; }
        Ok(())
    }

    /// Takes in the contacts of a .vcf file: one with a UID already in the book replaces it, the others are added.
    pub fn import(&self, book: &str, text: &str) -> Result<ImportReport, String> {
        let cards = vcard::parse(text);
        if cards.is_empty() { return Err("There are no contacts in that file.".into()); }
        let mut r = ImportReport::default();
        let mut seen = std::collections::HashSet::new();
        for mut c in cards {
            if c.uid.trim().is_empty() || !seen.insert(c.uid.clone()) { c.uid = new_uid(); seen.insert(c.uid.clone()); }
            if c.shown_name().is_empty() { r.skipped += 1; continue; }
            let exists = self.get(book, &c.uid).is_some();
            self.write_contact(book, &c)?;
            if exists { r.updated += 1 } else { r.added += 1 }
        }
        Ok(r)
    }

    /// A .vcf file of the given contacts (book and UID pairs), or of everything when the list is empty.
    pub fn export(&self, which: &[(String, String)]) -> String {
        if which.is_empty() { return vcard::write_all(self.list().iter().map(|e| &e.contact)); }
        let cs: Vec<Contact> = which.iter().filter_map(|(b, u)| self.get(b, u)).collect();
        vcard::write_all(cs.iter())
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(d) = path.parent() { fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    let tmp = path.with_extension("tmp-hearth");
    fs::write(&tmp, data).map_err(|e| format!("Couldn't save {}: {e}", path.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("Couldn't save {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> Store {
        let d = std::env::temp_dir().join(format!("hearth-contacts-test-{}", new_uid()));
        Store::open(d)
    }

    #[test]
    fn save_list_import_export() {
        let s = tmp();
        let mut c = Contact::default();
        c.name.given = "Ann".into();
        c.groups = vec!["Family".into()];
        let saved = s.save(LOCAL, c).unwrap();
        assert!(!saved.uid.is_empty());
        assert_eq!(saved.rev.len(), 16);
        assert_eq!(s.list().len(), 1);
        let text = s.export(&[]);
        assert!(text.contains("FN:Ann"));
        // Importing the same file again replaces, not doubles.
        let r = s.import(LOCAL, &text).unwrap();
        assert_eq!((r.added, r.updated), (0, 1));
        let r = s.import(LOCAL, "BEGIN:VCARD\nVERSION:3.0\nFN:Bob\nEND:VCARD\nBEGIN:VCARD\nVERSION:3.0\nEND:VCARD\n").unwrap();
        assert_eq!((r.added, r.skipped), (1, 1));
        assert_eq!(s.list().iter().map(|e| e.contact.shown_name()).collect::<Vec<_>>(), vec!["Ann", "Bob"]);
        s.delete(LOCAL, &saved.uid).unwrap();
        assert_eq!(s.list().len(), 1);
        let _ = fs::remove_dir_all(&s.root);
    }

    #[test]
    fn odd_uids_make_safe_file_names() {
        assert_eq!(file_name("abc-1"), "abc-1.vcf");
        let f = file_name("../../etc/passwd");
        assert!(!f.contains('/') && f.ends_with(".vcf") && !f.starts_with('.'));
        assert_ne!(file_name("a/b"), file_name("a?b"));
    }

    #[test]
    fn rev_looks_right() {
        let r = now_rev();
        assert!(r.starts_with("20") && r.ends_with('Z') && r.chars().nth(8) == Some('T'), "{r}");
    }
}
