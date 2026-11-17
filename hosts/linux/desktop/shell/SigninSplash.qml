// The splash after signing in (Settings › Personalization › Window frames & sign-in splash, 1.14, KDE's splash
// screen in Hearth's look): for a moment while the desktop comes up, Hearth's flame, its logo or a plain welcome, then
// it fades away. Off unless you pick one (Settings.signinSplash). Once per sign-in (a mark in $XDG_RUNTIME_DIR, so the
// shell refreshing itself doesn't show it again). It never holds anything up: clicks and keys go straight through to
// the desktop (an empty input mask, no keyboard focus), it's gone after 2.5 s whatever happens, and a problem here only
// loses the splash (shell.qml loads it by file name).
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: sp
    property bool shown: false
    property string style: ""
    readonly property string mark: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-signin-splash-shown"
    // Within the first seconds of the shell only; the mark says whether this sign-in has had it.
    property bool early: true
    Timer { running: true; interval: 4000; onTriggered: sp.early = false }
    Connections {
        target: Settings
        function onLoadedChanged() { sp.maybe(); }
    }
    Component.onCompleted: maybe()
    function maybe() {
        if (!Settings.loaded || !early || Settings.signinSplash === "" || checker.running || shown) return;
        checker.running = true;
    }
    Process {
        id: checker
        command: ["sh", "-c", 'm="$1"; [ -e "$m" ] && exit 1; : > "$m"; exit 0', "sh", sp.mark]
        onExited: code => { if (code === 0) { sp.style = Settings.signinSplash; sp.shown = true; away.start(); } }
    }
    // Settings' preview (ipc: splash preview STYLE), without the mark.
    function preview(s) { style = s || Settings.signinSplash || "flame"; shown = true; away.restart(); }
    IpcHandler {
        target: "splash"
        function preview(s: string): void { sp.preview(s); }
    }
    Timer { id: away; interval: 2500; onTriggered: sp.shown = false }

    Variants {
        model: sp.shown ? Quickshell.screens : []
        PanelWindow {
            required property var modelData
            screen: modelData
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-signin-splash"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            color: "transparent"
            mask: Region {}
            Rectangle {
                id: face
                anchors.fill: parent
                color: sp.style === "plain" ? Theme.panel : "#0d0a08"
                opacity: 0
                Component.onCompleted: fade.start()
                SequentialAnimation {
                    id: fade
                    NumberAnimation { target: face; property: "opacity"; to: 1; duration: 180 }
                    PauseAnimation { duration: 1700 }
                    NumberAnimation { target: face; property: "opacity"; to: 0; duration: 500; easing.type: Easing.InCubic }
                }
                Image {
                    visible: sp.style === "flame"
                    anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: -parent.height * 0.25 }
                    width: parent.width * 0.9; height: parent.height * 0.9
                    source: "images/ember-glow.png"; fillMode: Image.PreserveAspectFit; opacity: 0.85
                    SequentialAnimation on scale { loops: Animation.Infinite; NumberAnimation { from: 0.96; to: 1.04; duration: 1100; easing.type: Easing.InOutSine } NumberAnimation { from: 1.04; to: 0.96; duration: 1100; easing.type: Easing.InOutSine } }
                }
                Column {
                    anchors.centerIn: parent
                    spacing: 18
                    Image {
                        visible: sp.style !== "plain"
                        anchors.horizontalCenter: parent.horizontalCenter
                        source: "images/hearth-logo.png"; width: 132; height: 132; fillMode: Image.PreserveAspectFit; smooth: true
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: sp.style === "plain" ? "Welcome, " + (Quickshell.env("USER") || "you") : "Hearth"
                        color: sp.style === "plain" ? Theme.text : "#f6e7d6"
                        font.family: Theme.font; font.pixelSize: sp.style === "plain" ? 34 : 30; font.weight: Font.DemiBold
                    }
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 120; height: 3; radius: 2; color: Qt.rgba(1, 1, 1, 0.12)
                        Rectangle { id: bar; height: parent.height; radius: 2; color: sp.style === "plain" ? Theme.accent : "#ff8a3d"; width: 0
                            NumberAnimation on width { from: 0; to: 120; duration: 2000; running: true } }
                    }
                }
            }
        }
    }
}
