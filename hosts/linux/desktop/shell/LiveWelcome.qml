// The welcome card on the Hearth live USB (/etc/hearth-live): install Hearth, connect to Wi-Fi first (the
// installer downloads Hearth), or try Game Mode. Controller: Up/Down and A; B closes it (the Hearth menu and
// the launcher's "Install Hearth" bring the installer back).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    required property var settingsApp
    property bool live: false
    property bool shown: false
    property int sel: 0
    property bool choosing: false   // "Install Hearth OS" picked: with Hearth (controller) or Mantel (mouse/keyboard)?
    readonly property var mainItems: [
        { icon: "update", label: "Install Hearth OS", sub: "Next to Windows, or on a whole drive" },
        { icon: "wifi", label: "Connect to Wi-Fi", sub: "Optional: Hearth OS installs from this USB stick. Online, it can get the latest update afterwards" },
        { icon: "pad", label: "Try Game Mode", sub: "Come back here from the Power menu › Switch to desktop" },
        { icon: "close", label: "Keep exploring", sub: "Install Hearth OS is also in the app launcher" }
    ]
    readonly property var chooseItems: [
        { icon: "pad", label: "Install with Hearth", sub: "Recommended with a controller: fullscreen, big and simple" },
        { icon: "display", label: "Install with Mantel", sub: "Best with a mouse and keyboard: a window here on the desktop" },
        { icon: "close", label: "Back", sub: "" }
    ]
    readonly property var items: choosing ? chooseItems : mainItems
    /** Opens the welcome card straight on the "how do you want to install" choice (the launcher's Install). */
    function choose() { choosing = true; sel = 0; shown = true; }
    function pick(i) {
        if (!choosing) {
            if (i === 0) { choosing = true; sel = 0; return; }
            shown = false;
            if (i === 1) settingsApp.open("wifi");
            else if (i === 2) Quickshell.execDetached(["nimbus-mode", "game"]);
            return;
        }
        if (i === 2) { choosing = false; sel = 0; return; }
        shown = false; choosing = false;
        if (i === 0) Quickshell.execDetached(["sh", "-c", 'echo hearth > "${XDG_RUNTIME_DIR:-/tmp}/hearth-open-installer"; exec nimbus-mode game']);
        else settingsApp.open("install");
    }
    // Asked for from the Hearth app ("Install with Mantel"): open the installer once Mantel is up.
    Process {
        running: true
        command: ["sh", "-c", 'f="${XDG_RUNTIME_DIR:-/tmp}/hearth-open-installer"; [ "$(cat "$f" 2>/dev/null)" = mantel ] && rm -f "$f" && echo yes']
        stdout: SplitParser { onRead: l => { if (l.trim() === "yes") { root.shown = false; root.handoff = true; root.settingsApp.open("install"); } } }
    }
    property bool handoff: false
    Process {
        running: true
        command: ["test", "-e", "/etc/hearth-live"]
        onExited: code => { root.live = code === 0; root.shown = root.live && !root.handoff; }
    }

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-live"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, 0.5) }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 620)
        height: col.implicitHeight + 56
        radius: 24
        color: Theme.glass
        border.color: Theme.line
        focus: root.shown
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Down || e.key === Qt.Key_Tab) root.sel = (root.sel + 1) % root.items.length;
            else if (e.key === Qt.Key_Up || e.key === Qt.Key_Backtab) root.sel = (root.sel + root.items.length - 1) % root.items.length;
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) root.pick(root.sel);
            else if (e.key === Qt.Key_Escape) { if (root.choosing) { root.choosing = false; root.sel = 0; } else root.shown = false; }
            else return;
            e.accepted = true;
        }
        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
            spacing: 10
            RowLayout {
                spacing: 16
                Layout.bottomMargin: 4
                Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 64; Layout.preferredHeight: 64; fillMode: Image.PreserveAspectFit; smooth: true }
                ColumnLayout {
                    spacing: 2
                    Text { text: root.choosing ? "How do you want to install?" : "Welcome to Hearth"; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                    Text { Layout.fillWidth: true; text: root.choosing ? "Both do the same install. Pick the one that suits how you're using this PC." : "You're running Hearth from the USB stick. Look around, then install it when you're ready. Games won't run well until it's installed."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
                }
            }
            Repeater {
                model: root.items
                delegate: Rectangle {
                    id: it
                    required property var modelData
                    required property int index
                    readonly property bool lit: root.sel === index
                    Layout.fillWidth: true
                    implicitHeight: it.modelData.sub ? 64 : 48
                    radius: 14
                    color: lit ? Theme.accent : area.containsMouse ? Theme.panel3 : Theme.panel2
                    RowLayout {
                        anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                        spacing: 14
                        Icon { name: it.modelData.icon; size: 22; color: it.lit ? Theme.accentInk : Theme.text }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text { text: it.modelData.label; color: it.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                            Text { visible: it.modelData.sub !== ""; Layout.fillWidth: true; text: it.modelData.sub; color: it.lit ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                        }
                    }
                    MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onEntered: root.sel = it.index; onClicked: root.pick(it.index) }
                }
            }
        }
    }
}
