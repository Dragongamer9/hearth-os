//! The Matrix session (its access token) and the key the local store is encrypted with live in the system keyring:
//! the Secret Service on Linux (GNOME Keyring, which Hearth's session starts), as Hearth Contacts does. They're
//! never written to Hearth Friends' own files or logs. Each entry is named after the data folder, so two Hearth
//! profiles (or the CI test's two users) don't share one.

const SERVICE: &str = "hearth-friends";

fn entry(paths: &hearth_friends::Paths, what: &str) -> Result<keyring::Entry, String> {
    let name = format!("{what}:{}", paths.data.display());
    keyring::Entry::new(SERVICE, &name).map_err(|e| format!("The system keyring isn't available ({e})."))
}

pub fn set(paths: &hearth_friends::Paths, what: &str, secret: &str) -> Result<(), String> {
    entry(paths, what)?.set_password(secret).map_err(|e| format!("Couldn't keep it in the system keyring ({e})."))
}

pub fn get(paths: &hearth_friends::Paths, what: &str) -> Option<String> {
    entry(paths, what).ok()?.get_password().ok()
}

pub fn remove(paths: &hearth_friends::Paths, what: &str) {
    if let Ok(e) = entry(paths, what) { let _ = e.delete_credential(); }
}
