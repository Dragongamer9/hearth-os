// Hearth Settings › Clipboard: keep a history of what you copy (text and pictures) to paste again with Super+V,
// or don't. Kept by cliphist on this device only; Clipboard.qml shows it.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Clipboard"
    property var st: null
    property bool confirm: false
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    Component.onCompleted: job.run(["status"], r => page.st = r)
    SetJob { id: job; helper: "hearth-clipboard" }

    SetSwitch {
        icon: "clipboard"; title: "Keep clipboard history"
        sub: !page.st ? "Checking…" : !page.st.available ? "Needs cliphist and wl-clipboard, which aren't installed" : "Press Super+V to paste something you copied before. Kept on this device only."
        checked: Settings.clipboardHistory
        onToggled: v => { Settings.clipboardHistory = v; page.say(v ? "Clipboard history is on." : "Clipboard history is off. What's already kept stays until you clear it."); }
    }
    SetRow {
        visible: !!page.st && page.st.available
        icon: "clipboard"; title: "Open clipboard history"; sub: "Super+V"
        onActivated: if (SettingsNav.clipboard) SettingsNav.clipboard.open()
    }
    SetRow {
        visible: !!page.st && page.st.available && !page.confirm
        icon: "trash"; chevron: false; title: "Clear clipboard history…"
        onActivated: page.confirm = true
    }
    RowLayout {
        visible: page.confirm
        Layout.fillWidth: true
        spacing: 8
        SetRow {
            title: "Yes, clear it"; danger: true; chevron: false
            onActivated: job.run(["wipe"], () => { page.confirm = false; page.say("Clipboard history cleared."); }, e => page.say(e, true))
        }
        SetRow { title: "Cancel"; chevron: false; onActivated: page.confirm = false }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 6
        text: "Passwords copied from password managers that mark them as secret aren't kept."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
}
