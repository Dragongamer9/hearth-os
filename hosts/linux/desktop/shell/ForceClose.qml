// Force close a window that has stopped answering (Mantel on KWin, 1.14). Ctrl+Alt+Esc (shortcuts.json
// "kill-window", in Settings › Keyboard › Keyboard shortcuts) or "Force close (not responding)…" in a window's title bar
// menu. With the key, a strip at the top says to click the frozen window: the pointer is KWin's own cross (its
// queryWindowInfo, through hearth-kwin-bridge), Esc or a right-click cancels. Then Hearth asks before anything is
// lost: the app's icon and title, "Force close" or "Wait". Force close asks the window's process to end and, if it
// still hasn't after 3 s, makes it (hearth-kwin-bridge's kill; never KWin, the shell or a system process).
// labwc can't say which program a window belongs to, so on labwc this isn't there (the key does nothing there).
// Keys and the controller: Enter / A force closes, Esc / B waits. Loaded by shell.qml by file name: a problem here
// only loses this.
//   quickshell ipc -p <shell folder> call forceclose start | ask <title> | state | cancel | confirm
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: fc
    property string phase: ""           // "" | "picking" | "confirm"
    property var target: null           // {id, title, appId, pid}
    property bool busy: false
    readonly property var screen0: {
        const a = WinList.activeToplevel;
        try { if (a && a.screens && a.screens.length) return a.screens[0]; } catch (e) {}
        return Quickshell.screens.length ? Quickshell.screens[0] : null;
    }
    onPhaseChanged: SettingsNav.forceCloseShown = phase === "confirm"

    function start() {
        if (!WinList.kwin) return;
        if (!WinList.pickWindow()) { SettingsNav.toast("Force close needs the window list (KWin isn't answering)"); return; }
        target = null;
        phase = "picking";
    }
    function ask(info) {
        if (!info || !info.id) { phase = ""; return; }
        target = info;
        busy = false;
        phase = "confirm";
    }
    function cancel() { phase = ""; target = null; }
    function confirm() {
        if (!target || busy) return;
        busy = true;
        WinList.forceClose(target.id);
    }

    Connections {
        target: WinList
        function onPicked(info) {
            if (fc.phase !== "picking") return;
            if (info.cancel || !info.id) {
                fc.phase = "";
                if (info.error) SettingsNav.toast("Couldn't pick a window: " + info.error);
                return;
            }
            fc.ask(info);
        }
        function onForceCloseAsked(info) { fc.ask(info); }
        function onKilled(info) {
            if (!fc.target || info.id !== fc.target.id) return;
            const name = fc.target.title || "The window";
            fc.phase = "";
            fc.busy = false;
            SettingsNav.toast(info.ok ? name + " was closed" : "Couldn't close " + name + (info.why ? ": " + info.why : ""));
            fc.target = null;
        }
    }

    IpcHandler {
        target: "forceclose"
        function start(): void { fc.start(); }
        // The prompt for the window with this title, as the title bar menu opens it (checks).
        function ask(title: string): bool {
            const w = (WinList.toplevels ? WinList.toplevels.values : []).find(t => t.title === title);
            if (!w || !WinList.kwin) return false;
            fc.ask({ id: w.wid, title: w.title, appId: w.appId, pid: w.pid });
            return true;
        }
        function cancel(): void { fc.cancel(); }
        function confirm(): void { fc.confirm(); }      // the Force close button (checks)
        function state(): string { return JSON.stringify({ phase: fc.phase, title: fc.target ? fc.target.title : "", kwin: WinList.kwin }); }
    }

    // ---------------------------------------------------------------- "click the window" (KWin's cross is the pointer)
    PanelWindow {
        visible: fc.phase === "picking"
        screen: fc.screen0
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-forceclose-hint"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true }
        implicitWidth: hint.width + 24
        implicitHeight: hint.height + 28
        color: "transparent"
        mask: Region {}                // never in the way of the click
        Rectangle {
            id: hint
            x: 12; y: 16
            width: hintRow.implicitWidth + 36
            height: 54
            radius: 27
            color: Theme.panel
            border.color: Theme.line
            RowLayout {
                id: hintRow
                anchors.centerIn: parent
                spacing: 12
                Rectangle { implicitWidth: 30; implicitHeight: 30; radius: 15; color: "#e5484d"
                    Text { anchors.centerIn: parent; text: "✕"; color: "white"; font.pixelSize: 14; font.bold: true } }
                Text { text: "Click the window that isn't responding"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Text { text: "Esc to cancel"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
            }
        }
    }

    // ---------------------------------------------------------------- "Force close it?"
    PanelWindow {
        id: askWin
        visible: fc.phase === "confirm"
        screen: fc.screen0
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-forceclose"
        WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "#a6060809"
        onVisibleChanged: if (visible) keys.forceActiveFocus()
        MouseArea { anchors.fill: parent; onClicked: fc.cancel() }

        Item {
            id: keys
            focus: true
            Keys.onPressed: e => {
                if (e.key === Qt.Key_Escape) fc.cancel();
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) fc.confirm();
                else return;
                e.accepted = true;
            }
        }

        Rectangle {
            id: card
            anchors.centerIn: parent
            width: Math.min(parent.width - 32, 540)
            height: col.implicitHeight + 56
            radius: Math.max(10, Theme.r(22))
            color: Theme.panel
            border.color: Theme.line
            MouseArea { anchors.fill: parent }      // clicks on the card don't cancel
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 24; rightMargin: 24 } height: 3; radius: 2; color: "#e5484d" }

            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
                spacing: 16
                RowLayout {
                    spacing: 16
                    AppIcon {
                        size: 52
                        appId: fc.target ? (fc.target.appId || "") : ""
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Text {
                            Layout.fillWidth: true
                            text: "Force close " + (fc.target && fc.target.title ? "“" + fc.target.title + "”" : "this window") + "?"
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 21; font.weight: Font.DemiBold
                            wrapMode: Text.Wrap; maximumLineCount: 2; elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: "If it isn't responding, Hearth can end it for you. Anything you haven't saved in it will be lost."
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.Wrap
                        }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Item { Layout.fillWidth: true }
                    Rectangle {
                        Layout.preferredWidth: 130; Layout.preferredHeight: 44; radius: 22
                        color: waitArea.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
                        Text { anchors.centerIn: parent; text: "Wait"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 }
                        MouseArea { id: waitArea; anchors.fill: parent; hoverEnabled: true; onClicked: fc.cancel() }
                    }
                    Rectangle {
                        Layout.preferredWidth: 170; Layout.preferredHeight: 44; radius: 22
                        color: fc.busy ? Theme.panel3 : killArea.containsMouse ? Qt.lighter("#e5484d", 1.1) : "#e5484d"
                        Text { anchors.centerIn: parent; text: fc.busy ? "Closing…" : "Force close"; color: "white"; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                        MouseArea { id: killArea; anchors.fill: parent; hoverEnabled: true; onClicked: fc.confirm() }
                    }
                }
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignRight
                    text: "Enter or A: force close  ·  Esc or B: wait"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
            }
        }
    }
}
