// The Welcome screen (1.14, KDE's Welcome Center, in Hearth's look): once after each new version of Hearth OS, a minute
// and a half after Desktop Mode starts, what's new in it (welcome.json, by version) with a card per change that opens its
// Settings page. Closing it marks this version as seen (~/.local/state/nimbus/welcome.json, state, not a setting);
// "Show after updates" off (Settings.welcomeAfterUpdates) keeps it away. Never in the way of signing in: it waits, it
// closes with Esc, B or a click outside, and shell.qml loads it by file name, so a problem here only loses it.
//   quickshell ipc -p <shell folder> call welcome open|close|show VERSION
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: wl
    property var settings: null
    property bool shown: false
    property string version: ""
    property var pages: ({})
    property string seen: ""
    property bool seenRead: false
    property bool firstEver: false            // no record at all: a new install (first-time setup welcomed you)
    property string forceVer: ""              // `welcome show VERSION`
    readonly property string statePath: Quickshell.env("HOME") + "/.local/state/nimbus/welcome.json"
    readonly property string shortVer: { const v = forceVer || version; const m = /^(\d+)\.(\d+)/.exec(v); return m ? m[1] + "." + m[2] : v; }
    readonly property var pg: pages[shortVer] || pages["default"] || { title: "Welcome", sub: "", items: [] }
    SettingsWriter { id: writer; path: wl.statePath }
    FileView {
        path: wl.statePath
        onLoaded: { try { wl.seen = String(JSON.parse(text()).seen || ""); } catch (e) { wl.seen = ""; } wl.seenRead = true; }
        onLoadFailed: { wl.firstEver = true; wl.seenRead = true; }
    }
    FileView {
        path: Quickshell.shellDir + "/welcome.json"
        onLoaded: { try { wl.pages = JSON.parse(text()); } catch (e) { wl.pages = {}; } }
    }
    Process {
        running: true
        command: ["sh", "-c", ". /etc/os-release 2>/dev/null; echo \"${IMAGE_VERSION:-$VERSION_ID}\""]
        stdout: StdioCollector { onStreamFinished: wl.version = text.trim() }
    }
    Timer { interval: 90000; running: true; onTriggered: wl.maybe() }
    function maybe() {
        if (!seenRead || version === "" || shortVer === seen || Quickshell.env("HEARTH_SEAT")) return;
        // Once per version after an update; a new install (nothing recorded yet) is only recorded, not shown.
        if (firstEver || !Settings.welcomeAfterUpdates) { seen = shortVer; writer.merge({ seen: shortVer }); return; }
        shown = true;
    }
    function close() {
        shown = false;
        if (forceVer !== "") { forceVer = ""; return; }      // a preview: nothing recorded
        if (version !== "" && seen !== shortVer) { seen = shortVer; writer.merge({ seen: shortVer }); }
    }
    function go(page) { close(); if (settings) settings.open(page); }
    IpcHandler {
        target: "welcome"
        function open(): void { wl.shown = true; }
        // A version's page shown as it would be (Desktop Mode test's picture); doesn't mark anything as seen until closed.
        function show(v: string): void { wl.forceVer = v; wl.shown = true; }
        function close(): void { wl.close(); }
    }

    PanelWindow {
        visible: wl.shown
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-welcome"
        WlrLayershell.keyboardFocus: wl.shown ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "#a0080b10"
        MouseArea { anchors.fill: parent; onClicked: wl.close() }
        Rectangle {
            id: card
            anchors.centerIn: parent
            width: Math.min(parent.width - 80, 980); height: col.implicitHeight + 64
            radius: Theme.r(18)
            color: Theme.panel
            border.color: Theme.line
            focus: wl.shown
            Keys.onEscapePressed: wl.close()
            MouseArea { anchors.fill: parent }       // a click on the card stays on it
            ColumnLayout {
                id: col
                x: 32; y: 32; width: parent.width - 64
                spacing: 18
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 18
                    Image { source: "images/hearth-logo.png"; Layout.preferredWidth: 64; Layout.preferredHeight: 64; fillMode: Image.PreserveAspectFit; smooth: true }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Text { text: "Hearth OS " + wl.shortVer; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                        Text { Layout.fillWidth: true; text: wl.pg.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 28; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
                        Text { Layout.fillWidth: true; text: wl.pg.sub || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
                    }
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 3; columnSpacing: 12; rowSpacing: 12
                    Repeater {
                        model: wl.pg.items || []
                        delegate: Rectangle {
                            id: it
                            required property var modelData
                            Layout.fillWidth: true
                            implicitHeight: 128; radius: Theme.r(12)
                            color: im.containsMouse ? Theme.panel3 : Theme.panel2
                            border.color: Theme.line
                            Rectangle {
                                x: 16; y: 16; width: 38; height: 38; radius: 19; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
                                Icon { anchors.centerIn: parent; name: it.modelData.icon || "sparkle"; size: 20; color: Theme.accent }
                            }
                            Text { x: 16; y: 64; width: parent.width - 32; text: it.modelData.title; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                            Text { x: 16; y: 86; width: parent.width - 32; text: it.modelData.body; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                            MouseArea { id: im; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: wl.go(it.modelData.page) }
                        }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Rectangle {
                        implicitWidth: sw.implicitWidth + 50; implicitHeight: 36; radius: 18; color: "transparent"; border.color: Theme.line
                        Row {
                            anchors.centerIn: parent; spacing: 8
                            Rectangle { width: 18; height: 18; radius: 5; anchors.verticalCenter: parent.verticalCenter; color: Settings.welcomeAfterUpdates ? Theme.accent : "transparent"; border.color: Theme.line
                                Icon { anchors.centerIn: parent; visible: Settings.welcomeAfterUpdates; name: "check"; size: 12; color: Theme.accentInk } }
                            Text { id: sw; anchors.verticalCenter: parent.verticalCenter; text: "Show this after updates"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Settings.welcomeAfterUpdates = !Settings.welcomeAfterUpdates }
                    }
                    Item { Layout.fillWidth: true }
                    Rectangle {
                        implicitWidth: lb.implicitWidth + 44; implicitHeight: 40; radius: 20; color: Theme.accent
                        Text { id: lb; anchors.centerIn: parent; text: "Let's go"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: wl.close() }
                    }
                }
            }
        }
    }
}
