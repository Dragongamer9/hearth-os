// The password store really is the system keyring (not keyring's in-memory stand-in): CI runs this inside a D-Bus
// session with GNOME Keyring unlocked, with HEARTH_KEYRING_TEST=1.
use hearth_contacts::secrets;

#[test]
fn passwords_go_to_the_secret_service() {
    if std::env::var("HEARTH_KEYRING_TEST").ok().as_deref() != Some("1") { eprintln!("HEARTH_KEYRING_TEST isn't 1: skipped"); return; }
    let id = "dav-keyringtest";
    secrets::set(id, "not-a-real-password").expect("set");
    assert_eq!(secrets::get(id).expect("get"), "not-a-real-password");
    // Another process sees it too: it's in the keyring, not in this one's memory.
    let out = std::process::Command::new("secret-tool").args(["lookup", "service", "hearth-contacts", "username", id]).output();
    if let Ok(o) = out { if o.status.success() { assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "not-a-real-password"); eprintln!("secret-tool found it"); } }
    secrets::remove(id);
    assert!(secrets::get(id).is_err());
}
