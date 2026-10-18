pragma Singleton

// Desktop Mode settings that are kept between sessions (~/.local/share/nimbus/desktop.json).
// Changed in Hearth Settings (SettingsApp.qml) and Quick Access.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property bool nightLight: false      // warm the screen from sunset to sunrise
    property string wallpaper: ""        // a picture file; "" = the animated fireside; "anim:lava" / "anim:aurora" = the other moving ones
    property bool reduceMotion: false    // still wallpaper, fewer animations
    property real pointerSpeed: 1.0      // controller pointer speed, 0.4 – 2
    property bool lockOnWake: false      // lock the screen when the device goes to sleep (Desktop Mode)
    property bool loaded: false

    readonly property var keys: ["nightLight", "wallpaper", "reduceMotion", "pointerSpeed", "lockOnWake"]
    onNightLightChanged: save()
    onWallpaperChanged: save()
    onReduceMotionChanged: save()
    onPointerSpeedChanged: save()
    onLockOnWakeChanged: save()
    function save() {
        if (!loaded) return;
        const o = {};
        for (const k of keys) o[k] = root[k];
        file.setText(JSON.stringify(o, null, 2) + "\n");
    }

    FileView {
        id: file
        path: Quickshell.env("HOME") + "/.local/share/nimbus/desktop.json"
        // hearth-wallpaper (the file manager's "Set as Hearth wallpaper") changes this file too
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const o = JSON.parse(text());
                for (const k of root.keys) if (o[k] !== undefined && typeof o[k] === typeof root[k]) root[k] = o[k];
            } catch (e) {}
            root.loaded = true;
        }
        onLoadFailed: root.loaded = true
    }
}
