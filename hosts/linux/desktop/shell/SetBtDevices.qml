// Hearth Settings › Bluetooth › Your devices (1.14, KDE's Bluetooth device page, in Hearth's look): each paired device
// with its battery, Trust (connects by itself without asking), Block (never connects, even if it tries), and Send a
// file for phones and computers (OBEX through BlueZ: hearth-bt). BlueZ remembers trust and blocks itself; Hearth keeps
// no file. Reads every few seconds while open; changes only what you press.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Your Bluetooth devices"
    property var list: null
    property string sending: ""
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["devices"], r => page.replaceList("list", r.devices || []), e => { page.list = []; page.say(e, true); }); }
    Component.onCompleted: load()
    Timer { interval: 8000; repeat: true; running: page.visible && !sendJob.busy; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-bt" }
    SetJob { id: sendJob; helper: "hearth-bt" }
    Process {
        id: picker
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Send a file' --getopenfilename \"$HOME\"; elif command -v zenity >/dev/null; then zenity --file-selection --title='Send a file' --filename=\"$HOME/\"; else exit 3; fi"]
        stdout: StdioCollector {
            onStreamFinished: {
                const f = text.trim(); if (!f) return;
                const mac = page.sending;
                page.say("");
                sendJob.run(["send", mac, f], () => { page.say(f.replace(/^.*\//, "") + " sent."); page.sending = ""; }, e => { page.say(e, true); page.sending = ""; }, "", "send");
            }
        }
        onExited: code => { if (code === 3) { SettingsNav.toast("No file picker installed"); page.sending = ""; } }
    }
    function iconFor(i) { return /phone/.test(i) ? "phone" : /audio|head/.test(i) ? "headphones" : /input-gaming/.test(i) ? "pad" : /input-mouse/.test(i) ? "mouse" : /input-keyboard/.test(i) ? "kbd" : /computer/.test(i) ? "desktops" : "bt"; }

    SetCardGroup {
        title: page.list && page.list.length ? "Paired" : "No paired devices"
        SetRow {
            visible: !!page.list && page.list.length === 0
            icon: "bt"; title: "Pair a device first"; sub: "Bluetooth › Add a device"; chevron: true
            onActivated: SettingsNav.openPage("bluetooth")
        }
        Repeater {
            model: page.list || []
            delegate: SetExpander {
                id: dv
                required property var modelData
                Layout.fillWidth: true
                icon: page.iconFor(modelData.icon)
                title: modelData.name
                sub: (modelData.connected ? "Connected" : "Not connected") + (modelData.trusted ? " · trusted" : "") + (modelData.blocked ? " · blocked" : "")
                value: modelData.battery !== null && modelData.battery !== undefined ? modelData.battery + "%" : ""
                chip: modelData.blocked ? "Blocked" : ""; chipKind: "bad"
                SetSwitch {
                    icon: "check"; title: "Trust"; sub: "Connects by itself when it's near, without asking"
                    checked: dv.modelData.trusted
                    onToggled: v => job.run([v ? "trust" : "untrust", dv.modelData.mac], () => page.load(), e => page.say(e, true))
                }
                SetSwitch {
                    icon: "shield"; title: "Block"; sub: "Never connects, even when it tries"
                    checked: dv.modelData.blocked
                    onToggled: v => job.run([v ? "block" : "unblock", dv.modelData.mac], () => page.load(), e => page.say(e, true))
                }
                SetRow {
                    visible: dv.modelData.canSend
                    icon: "share"; chevron: false
                    title: page.sending === dv.modelData.mac && sendJob.busy ? "Sending… " + (sendJob.progress >= 0 ? Math.round(sendJob.progress * 100) + "%" : "") : "Send a file…"
                    sub: "The other device asks whether to take it"
                    onActivated: if (!sendJob.busy && !picker.running) { page.sending = dv.modelData.mac; picker.running = true; }
                }
                Rectangle {
                    visible: page.sending === dv.modelData.mac && sendJob.busy
                    Layout.fillWidth: true; Layout.leftMargin: 4; Layout.rightMargin: 4
                    implicitHeight: 6; radius: 3; color: Theme.panel3
                    Rectangle { width: parent.width * Math.max(0.03, sendJob.progress); height: parent.height; radius: 3; color: Theme.accent }
                }
            }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
