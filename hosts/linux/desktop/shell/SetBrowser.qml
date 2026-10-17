// Hearth Settings › Web browser: Aloha Browser, downloaded from alohabrowser.com by hearth-browser (it can't
// ship inside Hearth's image), installed in your home folder and made the default browser.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Web browser"
    property var st: null
    property string busy: ""
    property real progress: -1
    property string line: ""
    property string error: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-browser").toString().replace(/^file:\/\//, ""))

    function run(cmd) {
        if (proc.running) return;
        busy = cmd === "status" ? "" : cmd; error = ""; progress = -1; line = "";
        proc.command = ["python3", helper, cmd];
        proc.running = true;
    }
    Component.onCompleted: run("status")
    Process {
        id: proc
        stdout: SplitParser {
            onRead: l => {
                const m = l.match(/^progress: (\d+) ?(.*)$/);
                if (m) { page.progress = parseInt(m[1]) / 100; page.line = m[2]; }
                else if (l.startsWith("error: ")) page.error = l.slice(7);
                else if (l.startsWith("{")) { try { page.st = JSON.parse(l); } catch (e) {} }
            }
        }
        onExited: { page.busy = ""; page.progress = -1; }
    }

    SetRow {
        interactive: false; chevron: false
        icon: "info"; title: "Aloha Browser"
        sub: !page.st ? "Checking…" : !page.st.installed ? "Not installed" : (page.st.default ? "Your default browser" : "Installed, but another browser opens links") + (page.st.version ? " · version " + page.st.version : "")
    }
    ColumnLayout {
        visible: page.busy !== ""
        Layout.fillWidth: true
        spacing: 6
        Text { text: (page.line || "Working…") + (page.progress >= 0 ? "  " + Math.round(page.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, page.progress); height: parent.height; radius: 4; color: Theme.accent; visible: page.progress >= 0 }
        }
    }
    SetRow {
        visible: !!page.st && !page.st.installed && page.busy === ""
        icon: "update"; title: "Install Aloha Browser"; sub: "Downloads the official package from alohabrowser.com (about 150 MB) and makes it the default browser"
        onActivated: page.run("install")
    }
    SetRow {
        visible: !!page.st && page.st.installed && !page.st.default && page.busy === ""
        icon: "check"; title: "Make Aloha the default browser"; chevron: false
        onActivated: page.run("default")
    }
    SetRow {
        visible: !!page.st && page.st.installed && page.busy === ""
        icon: "restart"; title: "Look for an update"; sub: "Hearth also checks once a week"; chevron: false
        onActivated: page.run("update")
    }
    SetRow {
        visible: !!page.st && page.st.installed && page.busy === ""
        icon: "close"; danger: true; title: "Remove Aloha Browser"; sub: "Hearth won't install it again by itself"; chevron: false
        onActivated: page.run("remove")
    }
    Text {
        visible: page.error !== ""
        Layout.fillWidth: true
        text: page.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
