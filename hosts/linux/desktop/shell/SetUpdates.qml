// Hearth Settings › Updates: the running version, look for an update, download it (with progress). The
// update is put in place for the next restart. Runs hearth-admin (pkexec).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Updates"
    readonly property string helper: "/usr/libexec/nimbus/hearth-admin"
    property bool canAdmin: false
    property var status: null        // {booted, staged, available}
    property string phase: "idle"    // idle | checking | updating
    property real progress: -1
    property string line: ""
    property string error: ""

    function ver(i) { return i && i.version ? i.version : (i && i.timestamp ? i.timestamp.slice(0, 10) : ""); }
    function start(cmd) {
        if (admin.running) return;
        error = ""; line = cmd === "update" ? "Starting…" : cmd === "update-check" ? "Looking for updates…" : "";
        phase = cmd === "update" ? "updating" : cmd === "update-check" ? "checking" : "idle";
        progress = cmd === "update" ? 0 : -1;
        admin.command = ["pkexec", helper, cmd];
        admin.running = true;
    }
    Process {
        running: true
        command: ["test", "-x", page.helper]
        onExited: code => { page.canAdmin = code === 0; if (page.canAdmin) page.start("update-status"); }
    }
    Process {
        id: admin
        stdout: SplitParser {
            onRead: l => {
                l = l.trim();
                if (l.startsWith("log: ")) page.line = l.slice(5);
                else if (l.startsWith("error: ")) page.error = l.slice(7);
                else if (l.startsWith("progress: ")) {
                    const m = l.match(/^progress: ([\d.]+) ?(.*)$/);
                    if (m) { page.progress = parseFloat(m[1]) / 100; if (m[2]) page.line = m[2]; }
                } else if (l.startsWith("{")) { try { page.status = JSON.parse(l); } catch (e) {} }
            }
        }
        onExited: code => {
            if (code === 126 || code === 127) page.error = "Not allowed (Hearth's polkit rule is missing).";
            page.phase = "idle"; page.progress = -1; page.line = "";
        }
    }

    SetRow {
        visible: page.canAdmin
        interactive: false; chevron: false
        icon: "info"; title: "Hearth " + page.ver(page.status && page.status.booted)
        sub: page.status && page.status.booted && page.status.booted.image ? page.status.booted.image : "Running now"
    }
    Rectangle {
        visible: !!(page.status && page.status.staged)
        Layout.fillWidth: true
        implicitHeight: 64; radius: 12
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18); border.color: Theme.accent
        Text {
            anchors { fill: parent; margins: 16 } verticalAlignment: Text.AlignVCenter; wrapMode: Text.WordWrap
            text: "Hearth " + page.ver(page.status ? page.status.staged : null) + " is ready. Restart to finish updating."
            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
        }
    }
    SetRow {
        visible: !!(page.status && page.status.staged) && page.phase === "idle"
        icon: "restart"; title: "Restart now"; onActivated: Quickshell.execDetached(["systemctl", "reboot"])
    }

    // progress
    ColumnLayout {
        visible: page.phase !== "idle"
        Layout.fillWidth: true; Layout.topMargin: 6
        spacing: 8
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: page.line; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
            Text { visible: page.progress >= 0; text: Math.floor(page.progress * 100) + "%"; color: Theme.flame; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        }
        Item {
            Layout.fillWidth: true; implicitHeight: 8
            Rectangle { anchors.fill: parent; radius: 4; color: Theme.panel3 }
            Rectangle {
                id: bar
                height: parent.height; radius: 4
                width: page.progress >= 0 ? Math.max(8, parent.width * page.progress) : parent.width * 0.3
                x: 0
                gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: Theme.ember } GradientStop { position: 1; color: Theme.flame } }
                Behavior on width { NumberAnimation { duration: 300 } }
                SequentialAnimation on x {
                    running: page.phase !== "idle" && page.progress < 0; loops: Animation.Infinite
                    NumberAnimation { from: 0; to: bar.parent.width * 0.7; duration: 900; easing.type: Easing.InOutQuad }
                    NumberAnimation { from: bar.parent.width * 0.7; to: 0; duration: 900; easing.type: Easing.InOutQuad }
                }
            }
        }
    }

    SetRow {
        visible: page.canAdmin && page.phase === "idle" && !!(page.status && page.status.available) && !(page.status && page.status.staged)
        icon: "update"; title: "Download Hearth " + page.ver(page.status ? page.status.available : null)
        sub: "Keeps working while it downloads; finishes when you restart"
        onActivated: page.start("update")
    }
    SetRow {
        visible: page.canAdmin && page.phase === "idle"
        icon: "restart"; title: "Look for updates"
        sub: page.status && !page.status.available && !page.status.staged ? "Hearth is up to date" : ""
        onActivated: page.start("update-check")
    }
    SetRow {
        visible: page.canAdmin && page.phase === "idle" && !(page.status && page.status.available) && !(page.status && page.status.staged)
        icon: "update"; title: "Update anyway"; sub: "Downloads anything new for Hearth and its apps"
        onActivated: page.start("update")
    }
    Text {
        visible: page.error !== ""
        Layout.fillWidth: true
        text: page.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !page.canAdmin
        Layout.fillWidth: true
        text: "System updates are made here on Hearth, the OS."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
