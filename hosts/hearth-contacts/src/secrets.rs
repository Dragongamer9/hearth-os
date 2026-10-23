// CardDAV passwords live in the system keyring (the Secret Service on Linux: GNOME Keyring, which Hearth's
// session starts; Windows Credential Manager on Windows), under the service "hearth-contacts" and the account's id.
// They're never written to Hearth Contacts' own files.
const SERVICE: &str = "hearth-contacts";

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(|e| format!("The system keyring isn't available ({e})."))
}

pub fn set(account: &str, password: &str) -> Result<(), String> {
    entry(account)?.set_password(password).map_err(|e| format!("Couldn't keep the password in the system keyring ({e})."))
}

pub fn get(account: &str) -> Result<String, String> {
    entry(account)?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => "The password for this account isn't in the keyring any more: sign in again.".to_string(),
        e => format!("Couldn't read the password from the system keyring ({e})."),
    })
}

pub fn remove(account: &str) {
    if let Ok(e) = entry(account) { let _ = e.delete_credential(); }
}
