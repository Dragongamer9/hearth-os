pragma Singleton

// Desktop Mode settings that are kept between sessions (~/.local/share/nimbus/desktop.json).
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property bool nightLight: false     // warm the screen from sunset to sunrise
    property bool loaded: false

    onNightLightChanged: save()
    function save() {
        if (loaded) file.setText(JSON.stringify({ nightLight: nightLight }, null, 2) + "\n");
    }

    FileView {
        id: file
        path: Quickshell.env("HOME") + "/.local/share/nimbus/desktop.json"
        onLoaded: {
            try { root.nightLight = JSON.parse(text()).nightLight === true; } catch (e) {}
            root.loaded = true;
        }
        onLoadFailed: root.loaded = true
    }
}
