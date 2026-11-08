// Hearth Contacts: the address book of Hearth OS. Contacts are .vcf files (store.rs, vcard.rs); an account on a
// CardDAV server (Nextcloud, iCloud, Fastmail...) is kept in step with a copy of its address book (carddav.rs),
// its password in the system keyring (secrets.rs). The window is main.rs with the page in ui/.
/// Safe settings files shared by Hearth's apps (atomic, .bak, damaged files put back, never defaults over them).
#[path = "../../hearth-apptheme/appfile.rs"]
pub mod appfile;
pub mod carddav;
pub mod secrets;
pub mod store;
pub mod vcard;
