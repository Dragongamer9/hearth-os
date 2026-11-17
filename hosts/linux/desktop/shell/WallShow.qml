pragma Singleton

// The wallpaper slideshow and a picture per screen (Settings › Personalization › Wallpaper per screen & slideshow,
// 1.14, KDE's in Hearth's look). Settings.wallShow {on, folder, minutes, shuffle} and Settings.wallScreens {screen name:
// picture}: only read here, never written. Wallpaper.qml asks forScreen(name): that screen's own picture, else the
// slideshow's picture while it's on, else the wallpaper as before (Theme.wall). The folder is read when the slideshow
// starts and again every hour; a folder with no pictures (or gone) shows the wallpaper as before.
// Also (1.14) the picture of the day (Settings.wallPotd {on, source}: hearth-potd downloads it once a day and keeps it,
// off unless you turn it on) and a light/dark pair (Settings.wallPair {on, light, dark}: the one for the look's mode).
// Order: the screen's own picture, the slideshow, the picture of the day, the pair, then Theme.wall.
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
    // ---- picture of the day
    readonly property var potdCfg: Settings.wallPotd || ({})
    readonly property string potdSource: ["bing", "apod", "wikimedia"].indexOf(potdCfg.source) >= 0 ? potdCfg.source : "bing"
    readonly property bool potdOn: !!potdCfg.on
    property var potd: null            // hearth-potd's answer: {path, title, credit, source, date, cached, note}
    property string potdError: ""
    readonly property string potdPath: potdOn && potd && potd.source === potdSource && typeof potd.path === "string" ? potd.path : ""
    // ---- light and dark pair
    readonly property var pair: Settings.wallPair || ({})
    readonly property string pairWall: {
        if (!pair.on) return "";
        const f = Theme.dark ? pair.dark : pair.light;
        return typeof f === "string" ? f : "";
    }
    readonly property string base: current !== "" ? current : potdPath !== "" ? potdPath : pairWall !== "" ? pairWall : Theme.wall
    function forScreen(name) {
        const own = Settings.wallScreens && name ? Settings.wallScreens[name] : "";
        return typeof own === "string" && own !== "" ? own : base;
    }
    onPotdOnChanged: if (potdOn) fetchPotd()
    onPotdSourceChanged: if (potdOn) fetchPotd()
    function fetchPotd() {
        if (!potdOn) return;
        if (potdProc.running) { potdProc.again = true; return; }
        potdProc.command = ["python3", Quickshell.shellDir + "/hearth-potd", "fetch", potdSource];
        potdProc.running = true;
    }
    // A failure (offline, nothing kept yet) only keeps the wallpaper as it was and says why on the settings page.
    Process {
        id: potdProc
        property bool again: false
        stdout: StdioCollector {
            onStreamFinished: {
                const t = text.trim();
                if (t.startsWith("error: ")) { ws.potdError = t.slice(7); return; }
                try { ws.potd = JSON.parse(t); ws.potdError = ""; } catch (e) { ws.potdError = "hearth-potd: " + e; }
            }
        }
        onExited: if (again) { again = false; Qt.callLater(ws.fetchPotd); }
    }
    // Every three hours (a new day's picture shows the same morning).
    Timer { interval: 3 * 3600000; repeat: true; running: ws.potdOn; onTriggered: ws.fetchPotd() }
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
    Component.onCompleted: { if (on) look(); if (potdOn) fetchPotd(); }
}
