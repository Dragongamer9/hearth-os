// Address books from Online accounts (Settings › Accounts › Online accounts, docs/ONLINE-ACCOUNTS.md). Sign in once
// there, and the address books of every account with Contacts on show here beside Contacts' own CardDAV accounts,
// kept in step both ways by carddav.rs like any other.
//
// Everything is asked of `hearth-accounts` (/usr/bin/hearth-accounts; HEARTH_ACCOUNTS_BIN for the tests):
//   addressbooks      which address books there are (url, name, account)
//   list              which accounts there are, and whether Contacts is on for each
//   credentials ID    the password or a fresh OAuth token, asked for at each sync and only held in memory for it:
//                     never written to Contacts' files, never logged
//
// An address book that's both one of Contacts' own accounts and in Online accounts isn't shown twice: Contacts
// offers to join them (the copy here and its sync state are kept; Online accounts signs in from then on). Nothing is
// joined without the person saying so, and "keep them apart" is remembered.
use crate::carddav::Cred;
use crate::store::{fingerprint, Account, Store};
use serde::Serialize;
use serde_json::Value;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineBook {
    pub account: String,
    pub account_name: String,
    pub kind: String,
    pub user: String,
    pub url: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Duplicate {
    /// Contacts' own account.
    pub own: String,
    pub own_name: String,
    /// The Online accounts account with the same address book.
    pub account: String,
    pub account_name: String,
    pub book_name: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refresh {
    /// hearth-accounts is on this computer.
    pub available: bool,
    /// Address books newly shown.
    pub added: Vec<String>,
    pub duplicates: Vec<Duplicate>,
    /// Accounts that couldn't be reached just now ("Name: why").
    pub errors: Vec<String>,
}

pub fn program() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("HEARTH_ACCOUNTS_BIN").filter(|p| !p.is_empty()) { return Some(p.into()); }
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("hearth-accounts")).find(|f| f.is_file()))
}

/// Runs hearth-accounts: its answer (the last JSON line), or what its "error: ..." line says.
fn run(args: &[&str]) -> Result<Value, String> {
    let prog = program().ok_or("Online accounts aren't on this computer.")?;
    let out = Command::new(prog).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).output()
        .map_err(|e| format!("Couldn't ask Online accounts ({e})."))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if let Some(e) = text.lines().find_map(|l| l.strip_prefix("error: ")) { return Err(e.trim().to_string()); }
    text.lines().rev().find(|l| l.starts_with('{')).and_then(|l| serde_json::from_str(l).ok())
        .ok_or_else(|| "Online accounts didn't answer.".to_string())
}

/// How to sign in to an Online accounts account's address book, fresh from hearth-accounts.
pub fn credentials(account: &str) -> Result<Cred, String> {
    let v = run(&["credentials", account])?;
    if let Some(t) = v["token"].as_str().filter(|t| !t.is_empty()) { return Ok(Cred::Token { token: t.to_string() }); }
    match v["password"].as_str().filter(|p| !p.is_empty()) {
        Some(p) => Ok(Cred::Password { user: v["user"].as_str().unwrap_or("").to_string(), pass: p.to_string() }),
        None => Err("Online accounts has no password for this account: sign in to it again in Settings › Accounts › Online accounts.".into()),
    }
}

/// The same address book however the server's address was written (case of the host, a trailing /, %-escapes).
pub fn same_book(a: &str, b: &str) -> bool {
    let norm = |s: &str| -> Option<(String, String, u16, String)> {
        let u = url::Url::parse(s.trim()).ok()?;
        let path: String = percent_decode(u.path()).trim_end_matches('/').to_string();
        Some((u.scheme().to_string(), u.host_str()?.to_ascii_lowercase(), u.port_or_known_default().unwrap_or(0), path))
    };
    match (norm(a), norm(b)) { (Some(x), Some(y)) => x == y, _ => false }
}

fn percent_decode(s: &str) -> String {
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

/// The address books Online accounts offers, and the accounts that couldn't be reached (their ids and messages).
pub fn books() -> Result<(Vec<OnlineBook>, Vec<(String, String)>), String> {
    let v = run(&["addressbooks"])?;
    let s = |x: &Value, k: &str| x[k].as_str().unwrap_or("").to_string();
    let list = v["addressbooks"].as_array().cloned().unwrap_or_default().iter().filter(|b| !s(b, "url").is_empty() && !s(b, "account").is_empty())
        .map(|b| OnlineBook { account: s(b, "account"), account_name: s(b, "accountName"), kind: s(b, "kind"), user: s(b, "user"), url: s(b, "url"), name: s(b, "name") })
        .collect();
    let errors = v["errors"].as_array().cloned().unwrap_or_default().iter()
        .map(|e| (s(e, "account"), format!("{}: {}", s(e, "name"), s(e, "error")))).collect();
    Ok((list, errors))
}

/// The address book's id here: the same one every time for the same account and address book.
pub fn book_id(account: &str, url: &str) -> String {
    format!("oa-{}", &fingerprint(&format!("{account}\n{}", url.trim().trim_end_matches('/')))[..12])
}

fn host_of(u: &str) -> String { url::Url::parse(u).ok().and_then(|u| u.host_str().map(|h| h.to_string())).unwrap_or_default() }

/// Brings the address books here in line with Online accounts: new ones are added (their contacts come with the
/// next sync), ones whose account went or has Contacts off are paused (the copy stays), and an address book that's
/// also one of Contacts' own accounts is offered to be joined (never joined here).
pub fn refresh(store: &Store) -> Result<Refresh, String> {
    let mut r = Refresh::default();
    if program().is_none() { return Ok(r); }
    r.available = true;
    let (offered, errors) = books()?;
    let unreachable: Vec<String> = errors.iter().map(|(a, _)| a.clone()).collect();
    r.errors = errors.into_iter().map(|(_, m)| m).collect();
    for b in &offered {
        let accounts = store.accounts();
        if let Some(a) = accounts.iter().find(|a| same_book(&a.book_url, &b.url)) {
            if a.online.is_empty() && a.merge_declined != b.account {
                r.duplicates.push(Duplicate { own: a.id.clone(), own_name: a.name.clone(), account: b.account.clone(),
                    account_name: b.account_name.clone(), book_name: if b.name.is_empty() { a.book_name.clone() } else { b.name.clone() } });
            }
            continue;
        }
        let name = if b.account_name.is_empty() { b.kind.clone() } else { b.account_name.clone() };
        store.put_account(Account { id: book_id(&b.account, &b.url), name: name.clone(), provider: "online".into(), server: host_of(&b.url),
            username: b.user.clone(), book_url: b.url.clone(), book_name: b.name.clone(), online: b.account.clone(), ..Default::default() })?;
        r.added.push(if b.name.is_empty() { name } else { format!("{name} · {}", b.name) });
    }
    for mut a in store.accounts().into_iter().filter(|a| !a.online.is_empty()) {
        let there = offered.iter().any(|b| b.account == a.online && same_book(&b.url, &a.book_url));
        // An account that just couldn't be reached stays as it is.
        if unreachable.contains(&a.online) { continue; }
        if there == a.paused {
            a.paused = !there;
            a.last_error = if there { String::new() } else { "Not in Online accounts any more (or Contacts is off for it there): the copy is kept, nothing is synced.".into() };
            store.put_account(a)?;
        }
    }
    Ok(r)
}

/// Joins Contacts' own account OWN with the Online accounts account ACCOUNT that has the same address book: the
/// copy here and what was last synced are kept, and from now on it signs in through Online accounts (so its own
/// password leaves Contacts' keyring item). Only when the person chose to.
pub fn merge(store: &Store, own: &str, account: &str) -> Result<(), String> {
    let mut a = store.accounts().into_iter().find(|a| a.id == own).ok_or("That account isn't here any more.")?;
    if !a.online.is_empty() { return Err("That address book already comes from Online accounts.".into()); }
    let (offered, _) = books()?;
    let b = offered.iter().find(|b| b.account == account && same_book(&b.url, &a.book_url))
        .ok_or("That address book isn't in Online accounts any more.")?;
    a.online = b.account.clone();
    a.merge_declined.clear();
    a.paused = false;
    a.last_error.clear();
    if !b.account_name.is_empty() { a.name = b.account_name.clone(); }
    store.put_account(a)?;
    crate::secrets::remove(own);
    Ok(())
}

/// "Keep them apart": not offered again for this Online accounts account.
pub fn decline(store: &Store, own: &str, account: &str) -> Result<(), String> {
    let mut a = store.accounts().into_iter().find(|a| a.id == own).ok_or("That account isn't here any more.")?;
    a.merge_declined = account.to_string();
    store.put_account(a)
}

#[cfg(test)]
mod tests {
    #[test]
    fn same_book_however_written() {
        use super::same_book;
        assert!(same_book("https://Cloud.example.com/remote.php/dav/addressbooks/users/ann/contacts/", "https://cloud.example.com:443/remote.php/dav/addressbooks/users/ann/contacts"));
        assert!(same_book("https://x.org/a%20b/", "https://x.org/a b"));
        assert!(!same_book("https://x.org/a/", "https://x.org/b/"));
        assert!(!same_book("https://x.org/a/", "http://x.org/a/"));
        assert!(!same_book("", ""));
        assert_eq!(super::book_id("nextcloud-1", "https://x.org/a/"), super::book_id("nextcloud-1", "https://x.org/a"));
    }
}
