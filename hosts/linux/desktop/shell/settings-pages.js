.pragma library
// Hearth Settings' page registry (docs/MANTEL.md, "Settings"): the 12 categories plus Home, and every page with its one
// home. Adding a page is one entry in PAGES:
//   { id: "hs-thing", file: "SetHomeThing.qml", cat: "hs", label: "Thing", icon: "grid", desc: "One line for its card",
//     words: "other words people search for", old: "Its old name" }
// Optional: parent (a page it sits under: breadcrumbs and Back), props (set on the page once loaded, e.g. a part of a page
// shown on its own), hidden (not listed on its category's page: opened from another page), live (only on the live USB),
// status (a live value on its card: SetCategory.qml knows these), link (a card that opens another page's one real home).
// Old page ids keep working: every page id from before the redesign is still an id here (deep links, notifications, the
// launcher's search, `hearth-settings <page>`). tools/build-settings-index.py reads this file and every page's cards to
// make settings-index.js (the search); hosts/linux/test/test_settings_redesign.py checks both.

var CATEGORIES = [
    { id: "home", icon: "home", label: "Home", landing: "home" },
    { id: "system", icon: "display", label: "System", landing: "system" },
    { id: "net", icon: "wifi", label: "Network & internet", landing: "net" },
    { id: "bt", icon: "bt", label: "Bluetooth & devices", landing: "bt" },
    { id: "link", icon: "desktops", label: "Hearth Link", landing: "link" },
    { id: "pers", icon: "palette", label: "Personalization", landing: "appearance" },
    { id: "apps", icon: "store", label: "Apps", landing: "apps-home" },
    { id: "gaming", icon: "pad", label: "Gaming", landing: "gaming" },
    { id: "accounts", icon: "user", label: "Accounts", landing: "accounts" },
    { id: "hs", icon: "server", label: "Home server", landing: "hs-overview" },
    { id: "privacy", icon: "shield", label: "Privacy & security", landing: "privacy" },
    { id: "a11y", icon: "access", label: "Accessibility", landing: "accessibility" },
    { id: "updates", icon: "update", label: "Updates & recovery", landing: "updates" }
];

var PAGES = [
    // ---- Home
    { id: "home", file: "SetHome.qml", cat: "home", label: "Home", icon: "home", words: "device name rename status recommended start", landing: true },

    // ---- System
    { id: "system", file: "SetCategory.qml", cat: "system", label: "System", icon: "display", landing: true },
    { id: "install", file: "SetInstall.qml", cat: "system", label: "Install Hearth", icon: "update", desc: "Put Hearth OS on this PC, next to Windows or on its own", words: "install live usb windows dual boot", old: "Install Hearth", live: true },
    { id: "display", file: "SetDisplay.qml", cat: "system", label: "Display", icon: "display", desc: "Brightness, night light, scale, resolution and refresh rate", words: "screen resolution refresh scale monitor brightness night light", old: "Display", status: "display" },
    { id: "sound", file: "SetSound.qml", cat: "system", label: "Sound", icon: "vol", desc: "Volume, speakers and headphones, microphone", words: "volume audio speakers output microphone", old: "Sound" },
    { id: "notifications", file: "SetNotifications.qml", cat: "system", label: "Notifications", icon: "bell", desc: "Do not disturb, apps you muted", words: "do not disturb mute alerts", old: "Notifications", status: "dnd" },
    { id: "power", file: "SetPower.qml", cat: "system", label: "Power & battery", icon: "power", desc: "Screen off and sleep times, the power button, battery", words: "battery sleep screen off suspend power button", old: "Power" },
    { id: "storage", file: "SetStorage.qml", cat: "system", label: "Storage", icon: "drive", desc: "Space on your drives, games by size, clean up", words: "disk space clean games by size move games low space", old: "Storage" },
    { id: "boot", file: "SetBoot.qml", cat: "system", label: "Startup", icon: "power", desc: "The boot splash Hearth OS shows while it starts", words: "boot splash startup plymouth", old: "Boot & startup", props: { part: "splash" } },
    { id: "datetime", file: "SetDateTime.qml", cat: "system", label: "Date & time", icon: "clock", desc: "Time zone, 24-hour clock", words: "clock time zone 24-hour", old: "Date & time" },
    { id: "clipboard", file: "SetClipboard.qml", cat: "system", label: "Clipboard", icon: "clipboard", desc: "Clipboard history (Super+V)", words: "copy paste history", old: "Clipboard" },
    { id: "about", file: "SetAbout.qml", cat: "system", label: "About", icon: "info", desc: "Version, this PC's parts and name", words: "system version hardware name", old: "About", props: { part: "about" } },

    // ---- Network & internet
    { id: "net", file: "SetCategory.qml", cat: "net", label: "Network & internet", icon: "wifi", landing: true },
    { id: "wifi", file: "SetWifi.qml", cat: "net", label: "Wi-Fi", icon: "wifi", desc: "Connect to wireless networks", words: "wireless network internet hotspot", old: "Wi-Fi", status: "wifi" },
    { id: "hs-tailscale", file: "SetHomeTailscale.qml", cat: "net", label: "Tailscale", icon: "tailscale", desc: "Reach your devices from anywhere, privately", words: "vpn remote away taildrop exit node wake", old: "Home server › Tailscale", status: "tailscale" },

    // ---- Bluetooth & devices
    { id: "bt", file: "SetCategory.qml", cat: "bt", label: "Bluetooth & devices", icon: "bt", landing: true },
    { id: "bluetooth", file: "SetBluetooth.qml", cat: "bt", label: "Bluetooth", icon: "bt", desc: "Headphones, controllers and other wireless devices", words: "headphones headset speaker pair devices", old: "Bluetooth", status: "bt" },
    { id: "keyboard", file: "SetKeyboard.qml", cat: "bt", label: "Keyboard", icon: "kbd", desc: "Layouts, typing, the on-screen keyboard, shortcuts", words: "layout on-screen osk typing shortcuts", old: "Keyboard" },
    { id: "mouse", file: "SetMouse.qml", cat: "bt", label: "Mouse & touchpad", icon: "mouse", desc: "Pointer speed, scrolling, tapping", words: "touchpad pointer cursor speed", old: "Mouse & touchpad" },
    { id: "printers", file: "SetPrinters.qml", cat: "bt", label: "Printers", icon: "printer", desc: "Printers and scanners", words: "print scanner", old: "Printers" },

    // ---- Hearth Link (its own category; the page is the landing)
    { id: "link", file: "SetLink.qml", cat: "link", label: "Hearth Link", icon: "desktops", words: "phone devices sync pair storage backups saves", old: "Hearth Link", landing: true },

    // ---- Personalization
    { id: "appearance", file: "SetAppearance.qml", cat: "pers", label: "Personalization", icon: "palette", words: "look theme accent colour color wallpaper dark light mode sunset transparency clock", old: "Appearance", landing: true },
    { id: "bar-dock", file: "SetBarDock.qml", cat: "pers", label: "Bar and dock", icon: "window", desc: "Floating islands or a full-width bar; dock always shown or auto-hide", words: "taskbar panel dock autohide islands", status: "bar" },

    // ---- Apps
    { id: "apps-home", file: "SetCategory.qml", cat: "apps", label: "Apps", icon: "store", landing: true },
    { id: "apps", file: "SetApps.qml", cat: "apps", label: "Installed apps", icon: "store", desc: "Apps from Debian packages, Flathub and AppImages; uninstall", words: "install uninstall packages deb flatpak appimage", old: "Apps" },
    { id: "defaults", file: "SetDefaults.qml", cat: "apps", label: "Default apps", icon: "grid", desc: "Which app opens web links, pictures, music and more", words: "default apps browser terminal", old: "Default apps" },
    { id: "browser", file: "SetBrowser.qml", cat: "apps", label: "Web browser", icon: "globe", desc: "Hearth's browser and its settings", words: "web firefox", old: "Web browser" },
    { id: "claude", file: "SetClaude.qml", cat: "apps", label: "Claude", icon: "chat", desc: "Claude on this PC", words: "ai assistant", old: "Claude" },

    // ---- Gaming
    { id: "gaming", file: "SetGaming.qml", cat: "gaming", label: "Gaming", icon: "pad", words: "game mode start in steam", landing: true, ownList: true },
    { id: "steam", file: "SetSteam.qml", cat: "gaming", label: "Steam Game Mode and Decky", icon: "steam", desc: "Steam's big-screen mode, Decky Loader and Hearth Sync", words: "game mode decky plugins steam", old: "Steam Game Mode" },
    { id: "startup-animation", file: "SetBoot.qml", cat: "gaming", label: "Startup animation", icon: "film", desc: "What Steam Game Mode plays while it starts", words: "boot startup movie animation", old: "Boot & startup", props: { part: "animation" } },
    { id: "games", file: "SetGames.qml", cat: "gaming", label: "Games & stores", icon: "store", desc: "Epic Games, GOG and Xbox accounts, library sources, mods and saves", words: "steam stores library epic gog xbox", old: "Games & stores" },
    { id: "remote", file: "SetRemote.qml", cat: "gaming", label: "Remote play", icon: "play", desc: "Stream this PC's games to your phone or TV, or play another PC here", words: "moonlight sunshine stream", old: "Remote play" },
    { id: "controller", file: "SetController.qml", cat: "gaming", label: "Controller", icon: "pad", desc: "Controller mouse, button layout, what games see, remapping", words: "gamepad buttons pad", old: "Controller" },
    { id: "handheld", file: "SetHandheld.qml", cat: "gaming", label: "Handheld", icon: "bolt", desc: "Power limit, fans, lights and back buttons (Handheld Daemon)", words: "hhd tdp rgb lights fans", old: "Handheld" },

    // ---- Accounts
    { id: "accounts", file: "SetCategory.qml", cat: "accounts", label: "Accounts", icon: "user", landing: true },
    { id: "users", file: "SetUsers.qml", cat: "accounts", label: "Your account and other people", icon: "users", desc: "Your picture and name, other people on this PC", words: "accounts users people family", old: "Users" },
    { id: "signin", file: "SetSignin.qml", cat: "accounts", label: "Sign-in options", icon: "lock", desc: "Sign in automatically, password, lock after sleep", words: "password login autologin lock", old: "Sign-in" },

    // ---- Home server (always its own category; Overview is the landing)
    { id: "hs-overview", file: "SetHomeServer.qml", cat: "hs", label: "Home server", icon: "server", words: "nas home server light mode overview", old: "Home server › Overview", landing: true, ownList: true },
    { id: "hs-setup", file: "SetHomeSetup.qml", cat: "hs", label: "Set up your home server", icon: "check", desc: "A few steps: name, Tailscale, a shared folder, a backup", words: "nas setup steps", old: "Home server › Setup", hidden: true },
    { id: "hs-shares", file: "SetHomeShares.qml", cat: "hs", label: "Shared folders", icon: "folder", desc: "What your devices can open, and how", words: "nas smb samba share network windows folder", old: "Home server › Shared folders" },
    { id: "hs-files", file: "SetHomeFiles.qml", cat: "hs", label: "Files & links", icon: "link", desc: "Search every share, share links, drop-off folder", words: "nas search find links share link drop-off upload duplicates big files screenshots clean up", old: "Home server › Files & links" },
    { id: "hs-people", file: "SetHomePeople.qml", cat: "hs", label: "People", icon: "users", desc: "Family and friends with their own password", words: "nas family users accounts guest quota", old: "Home server › People" },
    { id: "hs-family", file: "SetHomeFamily.qml", cat: "hs", label: "Family & home", icon: "family", desc: "A Family folder, calendar and cameras", words: "nas family cameras onvif folder", old: "Home server › Family & home" },
    { id: "hs-services", file: "SetHomeServices.qml", cat: "hs", label: "Services", icon: "window", desc: "Media server, files on the web, AI chat, pictures and voices", words: "nas jellyfin media server photos web files apps comfyui kokoro", old: "Home server › Services" },
    { id: "hs-ai", file: "SetHomeAI.qml", cat: "hs", parent: "hs-services", label: "AI chat", icon: "chat", desc: "Ollama for the Hearth AI Command Center: models, address, Light mode", words: "nas ollama ai llm models command center gpu qwen", old: "Home server › AI" },
    { id: "hs-gaming", file: "SetHomeGaming.qml", cat: "hs", label: "Gaming", icon: "pad", desc: "A Games folder for your devices, Steam downloads at home", words: "nas games steam lan install remote play light mode", old: "Home server › Gaming" },
    { id: "hs-drives", file: "SetHomeDrives.qml", cat: "hs", label: "Drives & undo", icon: "drive", desc: "Health of every drive, snapshots to bring files back", words: "nas disk smart health snapshot undo raid mirror format", old: "Home server › Drives & undo" },
    { id: "hs-backups", file: "SetHomeBackups.qml", cat: "hs", label: "Backups & sync", icon: "cloud", desc: "To a drive, another Hearth device or the cloud", words: "nas backup restore restic cloud b2 s3 sync syncthing photos phone", old: "Home server › Backups & sync" },
    { id: "hs-safety", file: "SetHomeSafety.qml", cat: "hs", label: "Safety", icon: "shield", desc: "Who can reach what, open ports, ransomware guard", words: "nas security ransomware guard ports firewall", old: "Home server › Safety" },
    { id: "hs-alerts", file: "SetHomeAlerts.qml", cat: "hs", label: "Alerts & power", icon: "bell", desc: "Failing or full drives, sleep and wake, UPS", words: "nas alerts smart notifications sleep wake on lan spin down ups power schedule", old: "Home server › Alerts & power" },

    // ---- Privacy & security
    { id: "privacy", file: "SetCategory.qml", cat: "privacy", label: "Privacy & security", icon: "shield", landing: true },
    { id: "player-count", file: "SetAbout.qml", cat: "privacy", label: "Player count", icon: "users", desc: "Hearth OS's anonymous count of running devices", words: "privacy counter telemetry anonymous", old: "About › Privacy", props: { part: "privacy" }, status: "countMe" },

    // ---- Accessibility (the page is the landing)
    { id: "accessibility", file: "SetAccessibility.qml", cat: "a11y", label: "Accessibility", icon: "access", words: "magnifier contrast text size screen reader colour filters reduce motion", old: "Accessibility", landing: true },

    // ---- Updates & recovery (the page is the landing)
    { id: "updates", file: "SetUpdates.qml", cat: "updates", label: "Updates & recovery", icon: "update", words: "update upgrade version go back rollback decky plugins writable", old: "Updates", landing: true },
    { id: "backup", file: "SetAbout.qml", cat: "updates", label: "Backup and help", icon: "drive", desc: "Back up or restore Hearth, logs, send logs to Dragongamer9", words: "backup restore logs help send", old: "About › Backup", props: { part: "backup" } }
];

// Cards that open a page whose one real home is in another category (settings that show up in two places on purpose).
var LINKS = {
    net: [],
    hs: [{ page: "hs-tailscale", desc: "Lives in Network & internet now" }],
    gaming: [{ page: "storage", label: "Game storage", desc: "Games by size, moving games between drives, low space warning" }],
    privacy: [{ page: "clipboard", label: "Clipboard history", desc: "What you copy is kept for Super+V. Turn it off or clear it" },
              { page: "signin", label: "Lock after sleep", desc: "Ask for your password when this PC wakes" },
              { page: "hs-safety", label: "Home server safety", desc: "Who can reach what, open ports, ransomware guard" }],
    accounts: [{ page: "home", label: "Device name", desc: "This PC's name on your network: rename it on Home" }]
};

// Ids that aren't pages: old forms that open a page.
var ALIASES = { "appearance:bar": "bar-dock", "personalization": "appearance", "hs": "hs-overview", "a11y": "accessibility", "hearth-link": "link" };

function page(id) { for (var i = 0; i < PAGES.length; i++) if (PAGES[i].id === id) return PAGES[i]; return null; }
function category(id) { for (var i = 0; i < CATEGORIES.length; i++) if (CATEGORIES[i].id === id) return CATEGORIES[i]; return null; }
function landingOf(catId) { var c = category(catId); return c ? c.landing : "home"; }
// The pages a category lists (not its landing, not hidden ones, not live-only ones off the live USB).
function pagesIn(catId, live) {
    return PAGES.filter(function (p) { return p.cat === catId && !p.landing && !p.hidden && !p.parent && (!p.live || live); });
}
function resolve(id) {
    if (!id) return "";
    if (page(id)) return id;
    if (ALIASES[id]) return ALIASES[id];
    var c = category(id);
    return c ? c.landing : "";
}
// Breadcrumbs: [{id, label}] from the category to the page.
function crumbs(id) {
    var p = page(id);
    if (!p) return [];
    var c = category(p.cat), out = [];
    if (p.landing) return [{ id: id, label: c ? c.label : p.label }];
    out.push({ id: c ? c.landing : "home", label: c ? c.label : "" });
    if (p.parent) { var q = page(p.parent); if (q) out.push({ id: q.id, label: q.label }); }
    out.push({ id: id, label: p.label });
    return out;
}
// Back from a page: its parent, else its category's landing ("" on a landing).
function up(id) {
    var p = page(id);
    if (!p || p.landing) return "";
    return p.parent || landingOf(p.cat);
}
function pathOf(id) { return crumbs(id).map(function (c) { return c.label; }).join(" › "); }
