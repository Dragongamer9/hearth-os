// 1.14 (the owner, Nov 16: KDE and GNOME look exactly like Mantel, and follow it): Mantel's look as it's shown right
// now, written for hearth-desktop-look, which puts it into KDE Plasma and GNOME when one of them starts. This is a
// record of what's on screen (~/.local/state/hearth/mantel-look.json), not a setting: nothing reads it back into
// Mantel, and it's written only when the look changes. The wallpaper as shown is saved by Wallpaper.qml beside it.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: ex
    readonly property string dir: (Quickshell.env("XDG_STATE_HOME") || (Quickshell.env("HOME") + "/.local/state")) + "/hearth"
    function hex(c) { const h = n => ("0" + Math.round(n * 255).toString(16)).slice(-2); return "#" + h(c.r) + h(c.g) + h(c.b); }
    readonly property string json: JSON.stringify({
        look: Theme.look, label: Theme.lookLabel, mode: Theme.mode, accent: hex(Theme.accent),
        colors: { bg: hex(Theme.bg), panel: hex(Theme.panel), line: hex(Theme.line), text: hex(Theme.text), muted: hex(Theme.muted), ember: hex(Theme.ember), accent: hex(Theme.accent) },
        font: Theme.font, radius: Theme.radius, corner: Theme.r(14),
        wall: Theme.wall, wallpaper: dir + "/mantel-wallpaper.png",
        oneBar: Theme.oneBar, barEdge: Theme.barEdge, dockFloats: Theme.dockFloats, dockAutoHide: Theme.dockAutoHide, barAutoHide: Theme.barAutoHide,
        appMenu: Theme.appMenu
    })
    onJsonChanged: writeLater.restart()
    Timer { id: writeLater; interval: 2000; onTriggered: { Quickshell.execDetached(["mkdir", "-p", ex.dir]); out.setText(ex.json + "\n"); } }
    FileView { id: out; path: ex.dir + "/mantel-look.json"; blockLoading: false; printErrors: false }
    Component.onCompleted: writeLater.restart()
}
