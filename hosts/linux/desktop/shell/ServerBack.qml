// Server mode: "Back in Server mode", shown once after Game Mode (hearth-server-mode back): how long you played and what's
// coming back, from Light mode's choices and the services' state right now. B hides it. Draw-only.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: back
    property real k: 1
    property var info: null
    property var d: null
    readonly property var p: d && d.pauses ? d.pauses : (info && info.pauses ? info.pauses : {})
    function tile(id) { const ts = d && d.tiles ? d.tiles : []; for (const t of ts) if (t.id === id) return t; return null; }
    function played(s) { const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60); return h ? h + " h " + m + " min" : m + " min"; }
    readonly property var items: {
        const o = [];
        if (p.aiOn) { const t = tile("ai"); o.push({ busy: !(t && t.loaded && t.loaded.length), t: "AI chat", s: t && t.loaded && t.loaded.length ? "Ready · " + t.loaded.join(", ") : (p.ai === "small" ? p.big + " loads on the next question" : "Ready") }); }
        if (p.picturesOn) { const t = tile("pictures"); o.push({ busy: !(t && t.state === "running"), t: "Pictures", s: t && t.state === "running" ? "Ready · " + t.big + " waiting in the queue" : "Starting · the queue was kept" }); }
        if (p.lightMode) { const t = tile("backups"); o.push({ busy: !!(t && t.chip === "Running"), t: "Backups", s: t && t.chip === "Running" ? "A backup that waited runs now" : "Nothing waited" }); }
        if (p.voicesOn) { const t = tile("voices"); o.push({ busy: false, t: "Voices", s: "Ready" }); }
        o.push({ busy: false, t: "Shared folders, media and remote desktop", s: "Kept working the whole time" });
        return o;
    }
    implicitHeight: col.implicitHeight + 44 * k
    radius: 18 * k
    color: Theme.panel
    border.color: Theme.line
    ColumnLayout {
        id: col
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 * back.k }
        spacing: 12 * back.k
        RowLayout { spacing: 12 * back.k
            Rectangle { width: 34 * back.k; height: width; radius: width / 2; color: Qt.rgba(SetStyle.ok.r, SetStyle.ok.g, SetStyle.ok.b, 0.2)
                Icon { anchors.centerIn: parent; name: "server"; size: 20 * back.k; color: SetStyle.ok } }
            Text { text: "Back in Server mode"; color: Theme.text; font.family: Theme.font; font.pixelSize: 21 * back.k; font.weight: Font.DemiBold } }
        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: back.info ? "You played for " + back.played(back.info.played) + ". Everything that paused is coming back." : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * back.k }
        Repeater {
            model: back.items
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true; spacing: 12 * back.k
                Rectangle { width: 22 * back.k; height: width; radius: width / 2
                    color: modelData.busy ? "transparent" : Qt.rgba(SetStyle.ok.r, SetStyle.ok.g, SetStyle.ok.b, 0.2)
                    border.width: modelData.busy ? 3 * back.k : 0; border.color: Theme.accent
                    Icon { visible: !modelData.busy; anchors.centerIn: parent; name: "check"; size: 14 * back.k; color: SetStyle.ok }
                    RotationAnimation on rotation { running: modelData.busy; from: 0; to: 360; duration: 1200; loops: Animation.Infinite } }
                ColumnLayout { Layout.fillWidth: true; spacing: 0
                    Text { text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 15.5 * back.k; font.weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                    Text { text: modelData.s; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 * back.k; Layout.fillWidth: true; elide: Text.ElideRight } }
            }
        }
        Text { Layout.alignment: Qt.AlignRight; text: "B  Hide"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13.5 * back.k }
    }
}
