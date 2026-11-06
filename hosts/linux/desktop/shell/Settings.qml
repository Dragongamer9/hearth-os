pragma Singleton

// Desktop Mode settings that are kept between sessions (~/.local/share/nimbus/desktop.json).
// Changed in Hearth Settings (SettingsApp.qml) and Quick Access.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property bool nightLight: false      // warm the screen from sunset to sunrise
    property string wallpaper: ""        // a picture file; "" = the look's own (Hearth: the animated fireside, Steam-style: Slate Tide); "builtin:<name>" = one of images/wallpapers; "anim:lava" / "anim:aurora" = the other moving ones
    property bool reduceMotion: false    // still wallpaper, fewer animations
    property real pointerSpeed: 1.0      // controller pointer speed, 0.4 – 2
    property bool lockOnWake: false      // lock the screen when the device goes to sleep (Desktop Mode)
    property bool clock24: false         // taskbar clock: 24-hour ("14:05") instead of 12-hour ("2:05 PM"); Settings › Date & time
    property bool clipboardHistory: true // keep what you copy for Super+V (cliphist); Settings › Clipboard
    property int screenOffMinutes: 10    // turn the screen off after this long without use (0 = never); Settings › Power
    property int sleepMinutes: 0         // go to sleep after this long without use (0 = never); Settings › Power
    property string look: "steam"        // Mantel's look: "hearth" (warm) or "steam" (Steam-style blue-slate); Settings › Appearance
    property string mode: "dark"         // "dark" or "light"
    property string accent: ""           // "#rrggbb", or "" for the look's own (Hearth: your profile's accent)
    property bool countMe: true          // Hearth OS's anonymous player count (hearth-counter reads it); Settings › About › Privacy
    // ---- Mantel 2026 (Settings › Appearance). New keys: a missing key is the default and is never written by itself;
    // "" always means "the look's own" (theme/palettes.json), so choosing a look never changes these.
    property string barStyle: ""         // "" (the look's own) | "floating" (islands) | "bar" (one full-width bar)
    property string dockMode: ""         // "" (the look's own) | "shown" | "autohide"
    property string deskClock: ""        // the desktop clock and weather: "" (the look's own) | "on" | "off"
    property string panelGlass: ""       // panel transparency: "" (the look's own) | "solid" | "frosted" | "clear"
    property bool sunset: false          // Mode "Sunset": light by day, dark by night on your times. OFF unless you turn it on
    property string sunsetLight: "07:00" // Sunset: light from
    property string sunsetDark: "19:30"  // Sunset: dark from
    property string accentFrom: ""       // "wallpaper": the accent was worked out from your wallpaper (once, when you picked it)
    property var wallSwatches: []        // the colours "From wallpaper" found (shown as swatches)
    property bool loaded: false
    // Which defaults this file was made with (its "defaults" key, read only, never written by Hearth): 2 = made for a new
    // install (/etc/skel), whose Steam-style look gets the Sky accent by default (Theme.qml); files without it keep the
    // defaults they always had.
    property int defaultsGen: 1

    readonly property var keys: ["nightLight", "wallpaper", "reduceMotion", "pointerSpeed", "lockOnWake", "clock24", "clipboardHistory",
        "screenOffMinutes", "sleepMinutes", "look", "mode", "accent", "countMe"]
        .concat(["barStyle", "dockMode", "deskClock", "panelGlass", "sunset", "sunsetLight", "sunsetDark", "accentFrom", "wallSwatches"])
    onNightLightChanged: save("nightLight")
    onWallpaperChanged: save("wallpaper")
    onReduceMotionChanged: save("reduceMotion")
    onPointerSpeedChanged: save("pointerSpeed")
    onLockOnWakeChanged: save("lockOnWake")
    onClock24Changed: save("clock24")
    onClipboardHistoryChanged: save("clipboardHistory")
    onScreenOffMinutesChanged: save("screenOffMinutes")
    onSleepMinutesChanged: save("sleepMinutes")
    onLookChanged: save("look")
    onModeChanged: save("mode")
    onAccentChanged: save("accent")
    onCountMeChanged: save("countMe")
    onBarStyleChanged: save("barStyle")
    onDockModeChanged: save("dockMode")
    onDeskClockChanged: save("deskClock")
    onPanelGlassChanged: save("panelGlass")
    onSunsetChanged: save("sunset")
    onSunsetLightChanged: save("sunsetLight")
    onSunsetDarkChanged: save("sunsetDark")
    onAccentFromChanged: save("accentFrom")
    onWallSwatchesChanged: save("wallSwatches")
    // Settings must always be remembered and never change by themselves (docs/SETTINGS-PERSISTENCE.md): only the
    // key you changed is merged into the latest file (hearth-wallpaper and Hearth's Game Mode write it too), and
    // values read from the file are never written back. A file that can't be read is put back from its .bak; until
    // then nothing here writes the defaults over it.
    property bool applying: false
    function save(k) {
        if (!loaded || applying) return;
        const o = {};
        o[k] = root[k];
        writer.merge(o);
    }
    SettingsWriter { id: writer; path: file.path }

    FileView {
        id: file
        path: Quickshell.env("HOME") + "/.local/share/nimbus/desktop.json"
        // hearth-wallpaper (the file manager's "Set as Hearth wallpaper") changes this file too
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object") {
                // Damaged (a power cut on an older version): put back from desktop.json.bak; the watcher then
                // loads it. Saves stay merges of the one key changed, so the defaults never replace the file.
                console.warn("settings: desktop.json couldn't be read; restoring the last good copy");
                writer.repair();
                root.loaded = true;
                return;
            }
            root.applying = true;
            root.defaultsGen = typeof o.defaults === "number" ? o.defaults : 1;
            for (const k of root.keys) if (o[k] !== undefined && typeof o[k] === typeof root[k] && Array.isArray(o[k]) === Array.isArray(root[k]) && !writer.owns(k)) root[k] = o[k];
            root.applying = false;
            root.loaded = true;
        }
        onLoadFailed: root.loaded = true
    }
}
