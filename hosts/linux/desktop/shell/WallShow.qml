pragma Singleton

// The wallpaper slideshow and a picture per screen (Settings › Personalization › Wallpaper per screen & slideshow,
// 1.14, KDE's in Hearth's look). Settings.wallShow {on, folder, minutes, shuffle} and Settings.wallScreens {screen name:
// picture}: only read here, never written. Wallpaper.qml asks forScreen(name): that screen's own picture, else the
// slideshow's picture while it's on, else the wallpaper as before (Theme.wall). The folder is read when the slideshow
// starts and again every hour; a folder with no pictures (or gone) shows the wallpaper as before.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: ws
    readonly property var cfg: Settings.wallShow || ({})
    readonly property bool on: !!cfg.on && typeof cfg.folder === "string" && cfg.folder !== ""
    readonly property int minutes: Math.max(1, Math.min(1440, parseInt(cfg.minutes) || 30))
    property var pictures: []
    property int index: 0
    readonly property string current: on && pictures.length ? pictures[index % pictures.length] : ""
    function forScreen(name) {
        const own = Settings.wallScreens && name ? Settings.wallScreens[name] : "";
        return typeof own === "string" && own !== "" ? own : current !== "" ? current : Theme.wall;
    }
    function next() {
        if (pictures.length < 2) return;
        if (cfg.shuffle) { let n = index; while (n === index) n = Math.floor(Math.random() * pictures.length); index = n; }
        else index = (index + 1) % pictures.length;
    }
    onOnChanged: if (on) look()
    onCfgChanged: if (on) look()
    function look() { if (!lister.running) lister.running = true; }
    Process {
        id: lister
        command: ["find", ws.cfg.folder || "/nonexistent", "-maxdepth", "1", "-type", "f", "(", "-iname", "*.jpg", "-o", "-iname", "*.jpeg", "-o", "-iname", "*.png", "-o", "-iname", "*.webp", ")"]
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.split("\n").filter(x => x.trim() !== "").sort();
                ws.pictures = l.slice(0, 2000);
                if (ws.index >= ws.pictures.length) ws.index = 0;
            }
        }
    }
    Timer { interval: ws.minutes * 60000; repeat: true; running: ws.on && ws.pictures.length > 1; onTriggered: ws.next() }
    Timer { interval: 3600000; repeat: true; running: ws.on; onTriggered: ws.look() }
    Component.onCompleted: if (on) look()
}
