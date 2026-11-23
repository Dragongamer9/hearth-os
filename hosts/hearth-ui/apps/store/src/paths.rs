//! Where the Store keeps things. Everything it writes is in your home folder (the XDG folders): the apps, their menu
//! entries and icons, its cache and its settings. The system (/usr, /etc, boot files, sessions, services) is only read.

use std::path::PathBuf;

fn xdg(var: &str, rest: &str) -> PathBuf {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| hearth_ui::paths::home().join(rest))
}

pub fn data() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share")
}
pub fn config() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
}
pub fn cache_home() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache")
}

/// Where the person moved the Store's apps (Settings › Where apps go › Change…), read from the Store's settings by
/// prefs::load; None: the usual place.
static APPS_AT: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

pub fn set_apps(p: Option<PathBuf>) {
    if let Ok(mut g) = APPS_AT.write() {
        *g = p;
    }
}

/// ~/.local/share/hearth/store/apps: where apps go unless the person picked another folder.
pub fn default_apps() -> PathBuf {
    data().join("hearth/store/apps")
}

/// One folder per app (its AppImage and store.json, what the Store put there): the folder picked in Settings, else
/// ~/.local/share/hearth/store/apps.
pub fn apps() -> PathBuf {
    APPS_AT.read().ok().and_then(|g| g.clone()).unwrap_or_else(default_apps)
}
pub fn app_dir(id: &str) -> PathBuf {
    apps().join(id)
}
/// The app's menu entry (~/.local/share/applications/hearth-store-<id>.desktop).
pub fn desktop_file(id: &str) -> PathBuf {
    data().join("applications").join(format!("hearth-store-{id}.desktop"))
}
/// Its icon (drawn by the Store from the catalog: the coloured square with its letters).
pub fn icon_file(id: &str) -> PathBuf {
    data().join("icons/hicolor/scalable/apps").join(format!("hearth-store-{id}.svg"))
}
/// ~/.cache/hearth/hearth-store: the last list of apps and releases seen (offline, the Store shows these), pictures.
/// (~/.cache/hearth/store is Settings › Apps › App store's, Flathub's answers: kept apart.)
pub fn cache() -> PathBuf {
    cache_home().join("hearth/hearth-store")
}
pub fn pictures() -> PathBuf {
    cache().join("pictures")
}
/// The Store's own settings (prefs.rs).
pub fn prefs() -> PathBuf {
    config().join("nimbus/hearth-store.json")
}
/// The owner's private list (only on his own devices; never published): docs/HEARTH-STORE.md.
pub fn private_list() -> PathBuf {
    std::env::var_os("HEARTH_STORE_PRIVATE").map(PathBuf::from).unwrap_or_else(|| config().join("nimbus/store-private.json"))
}
/// The list of apps built into this Hearth OS (updated with each release of Hearth OS).
pub fn system_catalog() -> PathBuf {
    std::env::var_os("HEARTH_STORE_CATALOG").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/hearth/store/catalog.json"))
}
/// The public half of the owner's signing key, built into Hearth OS (64 hex characters). Without it, no list from
/// hearthos.fun is ever used: the one built in is.
pub fn signing_key() -> PathBuf {
    std::env::var_os("HEARTH_STORE_KEY").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/hearth/store/catalog-key.pub"))
}
/// The Trash (freedesktop.org's: what Files and Dolphin show).
pub fn trash() -> PathBuf {
    data().join("Trash")
}
/// CI's pretend world (fake releases, offline): HEARTH_STORE_FIXTURE=<folder>. Never set on a PC.
pub fn fixture() -> Option<PathBuf> {
    std::env::var_os("HEARTH_STORE_FIXTURE").filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// "~/.local/share/hearth/store/apps" for showing.
pub fn tilde(p: &std::path::Path) -> String {
    let h = hearth_ui::paths::home();
    match p.strip_prefix(&h) {
        Ok(r) => format!("~/{}", r.display()),
        Err(_) => p.display().to_string(),
    }
}

/// "~/.config/Hearth AI" → the home folder's path.
pub fn expand(s: &str) -> PathBuf {
    if let Some(r) = s.strip_prefix("~/") {
        hearth_ui::paths::home().join(r)
    } else {
        PathBuf::from(s)
    }
}
