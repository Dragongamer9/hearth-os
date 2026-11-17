//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Hearth's "Allow?" question for apps (xdg-desktop-portal's Access, on KWin: hearth_screencast.py starts it with
// quickshell -p <this file>), e.g. "Allow Firefox to take screenshots?". Mantel's look; Enter / A allows, Esc / B
// doesn't, arrows / D-pad move between the two buttons. Nothing is allowed unless you choose Allow.
//   HEARTH_ACCESS_REQUEST  {app, title, subtitle, body, grant, deny, icon}
//   HEARTH_ACCESS_RESULT   {grant: true} | {grant: false}
// For checks: quickshell ipc -p <this file> call access grant | deny | state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
    id: root
    readonly property string reqPath: Quickshell.env("HEARTH_ACCESS_REQUEST") || ""
    readonly property string resPath: Quickshell.env("HEARTH_ACCESS_RESULT") || ""
    property var req: ({ title: "", subtitle: "", body: "", grant: "", deny: "" })
    property bool onAllow: false      // the button lit for the keyboard / D-pad: Don't allow first
    property bool ending: false

    FileView {
        path: root.reqPath
        onLoaded: { try { root.req = JSON.parse(text()); } catch (e) { root.finish(false); } }
        onLoadFailed: if (root.reqPath) root.finish(false)
    }
    Process { id: writer; onExited: Qt.quit() }
    function finish(grant) {
        if (ending) return;
        ending = true;
        if (!resPath) return Qt.quit();
        writer.command = ["sh", "-c", 'printf "%s" "$1" > "$2.part" && mv -f "$2.part" "$2"', "sh", JSON.stringify({ grant: grant === true }), resPath];
        writer.running = true;
    }

    IpcHandler {
        target: "access"
        function grant(): void { root.finish(true); }
        function deny(): void { root.finish(false); }
        function state(): string { return JSON.stringify({ title: root.req.title || "", onAllow: root.onAllow }); }
    }

    PanelWindow {
        anchors { top: true; bottom: true; left: true; right: true }
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-access-prompt"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        color: Qt.rgba(0, 0, 0, 0.55)

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 64, 620)
            height: col.implicitHeight + 56
            radius: Theme.radius + 4
            color: Theme.panel
            border.color: Theme.line
            focus: true
            Keys.onPressed: e => {
                if (e.key === Qt.Key_Escape) root.finish(false);
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) root.finish(root.onAllow);
                else if (e.key === Qt.Key_Left || e.key === Qt.Key_Right || e.key === Qt.Key_Tab) root.onAllow = !root.onAllow;
                else return;
                e.accepted = true;
            }
            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
                spacing: 12
                RowLayout {
                    spacing: 14
                    Rectangle {
                        Layout.preferredWidth: 46; Layout.preferredHeight: 46; radius: 23
                        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
                        Text { anchors.centerIn: parent; text: "?"; color: Theme.accent; font.pixelSize: 24; font.weight: Font.DemiBold; font.family: Theme.font }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.req.title || "Allow this?"
                        wrapMode: Text.WordWrap
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold
                    }
                }
                Text { Layout.fillWidth: true; visible: !!root.req.subtitle; text: root.req.subtitle || ""; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 }
                Text { Layout.fillWidth: true; visible: !!root.req.body; text: root.req.body || ""; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                RowLayout {
                    Layout.topMargin: 10
                    Layout.alignment: Qt.AlignRight
                    spacing: 12
                    Rectangle {
                        Layout.preferredWidth: 150; Layout.preferredHeight: 44; radius: 22
                        color: Theme.panel2; border.width: 2; border.color: !root.onAllow ? Theme.accent : Theme.line
                        Text { anchors.centerIn: parent; text: root.req.deny || "Don't allow"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 }
                        MouseArea { anchors.fill: parent; onClicked: root.finish(false) }
                    }
                    Rectangle {
                        Layout.preferredWidth: 150; Layout.preferredHeight: 44; radius: 22
                        color: Theme.accent; border.width: 2; border.color: root.onAllow ? Theme.text : Theme.accent
                        Text { anchors.centerIn: parent; text: root.req.grant || "Allow"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                        MouseArea { anchors.fill: parent; onClicked: root.finish(true) }
                    }
                }
            }
        }
    }
}
