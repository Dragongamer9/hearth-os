pragma Singleton

// Shared by Hearth Settings' pages: focus back to the page list (Left / Esc / B), and the controller
// helper (Pad.qml) for the Controller page. Apps (AppStore.qml, archived: docs/ARCHIVED.md) used the same pages and controls, so its
// own list registers here too: Left / Esc go back to the list of the window you're in.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    // ---- The Firehold preview's hidden switch (the owner's alone on 1.14; not in docs, release notes or the site).
    // How to unlock and test: Settings › System › About, tap or click the "Hearth OS <version>" line (or the Version
    // row, controller A) 7 times within 4 seconds. The 7th tap runs `hearth-your-desktop unlock`
    // (HEARTH_FIREHOLD_UNLOCKED=1 in ~/.config/nimbus/session.conf, atomic with .bak), "Firehold unlocked" shows on
    // the page, and Settings › System › Your desktop shows "Firehold (preview)" (Off / Try once / Always) with "Hide
    // Firehold again" next to it (`relock`: Firehold off, the window manager from before back, hidden again). Until
    // then nimbus-session ignores any Firehold setting. Inside Firehold, Ctrl+Alt+Backspace goes back to KWin. Tests:
    // `quickshell ipc call settings tapversion` is one tap (firehold-test.sh taps 7 times).
    property int versionTaps: 0
    property string unlockNote: ""
    function tapVersion() {
        versionTaps += 1;
        tapWindow.restart();
        if (versionTaps >= 7) {
            versionTaps = 0;
            tapWindow.stop();
            unlockProc.running = true;
        }
    }
    property Timer tapWindow: Timer { interval: 4000; onTriggered: versionTaps = 0 }
    property Timer noteTimer: Timer { interval: 6000; onTriggered: unlockNote = "" }
    property Process unlockProc: Process {
        command: ["python3", decodeURIComponent(Qt.resolvedUrl("hearth-your-desktop").toString().replace(/^file:\/\//, "")), "unlock"]
        onExited: (code) => {
            unlockNote = code === 0 ? "Firehold unlocked · Settings › System › Your desktop" : "Couldn't unlock Firehold";
            noteTimer.restart();
            unlocked();
        }
    }
    signal unlocked()

    property var navTarget: null
    property string pageAnchor: ""    // a part of the page to open at ("appearance:bar": Settings › Appearance › Bar and dock)
    property var settingsApp: null    // Hearth Settings (SettingsApp.qml): a page can open another (openPage)
    function openPage(p) { if (settingsApp) settingsApp.open(p); }
    // (pageAnchor above: Settings opened at a part of a page, "keyboard:osk", "appearance:bar"; the page or its SetExpander goes there.)
    property var frame: null          // the frame, for Back (B / Esc): a subpage goes up to its category
    property var appsNav: null        // Apps' list (AppStore.qml)
    property var clipboard: null      // the clipboard history picker (Clipboard.qml), for Settings › Clipboard
    property var apps: null           // Apps (AppStore.qml): archived, always null now (Bazaar is the app store)
    property var debs: null           // the Install Package window (DebInstaller.qml), for Settings › Apps
    property var debNav: null         // its page while it's open: Left / Esc (B) there close it
    property bool debShown: false
    property var pad: null
    property var help: null
    // Open windows that want the controller to type arrow keys / Enter / Esc (read by Pad.qml).
    property bool appsShown: false    // Apps
    property bool clipShown: false    // the clipboard history picker (Super+V)
    // Hearth 1.10's volume mixer (Mixer.qml) and snap layouts (SnapLayouts.qml, Super+Z): set while open, so the
    // controller types arrow keys / Enter / Esc there; the objects themselves for the bar and Settings › Sound.
    property var mixer: null
    property bool mixerShown: false
    property var snap: null
    property bool snapShown: false
    property var discord: null        // Discord (Discord.qml): the bar's Discord button toggles its small window
    property bool discordShown: false // its "Which Discord?" chooser is open
    property var winTools: null       // Device Manager, Run and the Super+X menu (WinTools.qml): the Start button's right-click
    property bool winxShown: false    // the Super+X menu is open
    function toNav() {
        if (debNav && debNav.Window.active) { debNav.back(); return; }
        const t = appsNav && appsNav.Window.active ? appsNav : navTarget;
        if (t) t.forceActiveFocus();
    }
    // B / Esc on a page: up one level (a subpage to its category's page, a category's page to the list).
    function back() {
        if (debNav && debNav.Window.active) { debNav.back(); return; }
        if (appsNav && appsNav.Window.active) { appsNav.forceActiveFocus(); return; }
        if (frame && frame.isActive && frame.isActive()) { frame.back(); return; }
        toNav();
    }
    function toast(t) { if (help) help.toast(t); }
    // 1.14, the new Settings: "pad" while the controller was used last (its look, B), "mouse" while the mouse,
    // touch or keyboard was (A). hearth-pad says when a controller button is pressed; the pointer moving counts only
    // when no controller button was pressed in the last 1.5 s (the controller's pointer mode moves it too).
    property string input: "mouse"
    property double padAt: 0
    function padUsed() { padAt = Date.now(); if (input !== "pad") input = "pad"; }
    function pointerUsed() { if (input !== "mouse" && Date.now() - padAt > 1500) input = "mouse"; }
}
