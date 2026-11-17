// Hearth Settings › Clipboard: keep a history of what you copy (text and pictures) to paste again with Super+V, or
// don't. Off until you turn it on (the same switch is at the top of the Super+V popup). Kept on this device only, in
// ~/.local/share/hearth/clipboard (hearth-clipboard); Clipboard.qml shows it.
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
        icon: "clipboard"; title: "Clipboard history"
        sub: !page.st ? "Checking…" : !page.st.available ? "Needs wl-clipboard, which isn't installed" : "Keep the last 25 things you copy, and the ones you pin, to paste again with Super+V. Kept on this device only."
        checked: Settings.clipboardHistory
        onToggled: v => { Settings.clipboardHistory = v; page.say(v ? "Clipboard history is on." : "Clipboard history is off. What's already kept stays until you clear it."); }
    }
    // The selection (1.14): what you highlight, which middle-click pastes. Both off until you turn them on; kept in
    // ~/.config/nimbus/clipboard.json by Clipboard.qml (only the key you change).
    readonly property var cbx: SettingsNav.clipboard
    SetSwitch {
        visible: !!page.st && page.st.available
        enabled: !!page.cbx && Settings.clipboardHistory
        icon: "clipboard"; title: "Selection history"
        sub: !Settings.clipboardHistory ? "Turn on clipboard history first" : "Also keep what you highlight (middle-click pastes it), marked Selection"
        checked: !!page.cbx && page.cbx.selectionOn
        onToggled: v => { if (page.cbx) page.cbx.setPref("selection", v); page.say(v ? "What you highlight is kept too." : "Highlighted text isn't kept any more."); }
    }
    SetSwitch {
        visible: !!page.st && page.st.available
        enabled: !!page.cbx && Settings.clipboardHistory
        icon: "sync"; title: "Keep the clipboard and selection in step"
        sub: !Settings.clipboardHistory ? "Turn on clipboard history first" : "Text you copy can be middle-clicked, and text you highlight can be pasted with Ctrl+V"
        checked: !!page.cbx && page.cbx.syncOn
        onToggled: v => { if (page.cbx) page.cbx.setPref("sync", v); page.say(v ? "The clipboard and the selection are kept in step." : "The clipboard and the selection are separate again."); }
    }
    SetRow {
        visible: !!page.st && page.st.available
        icon: "clipboard"; title: "Open clipboard history"; sub: "Super+V"
        onActivated: if (SettingsNav.clipboard) SettingsNav.clipboard.open()
    }
    SetRow {
        visible: !!page.st && page.st.available && !page.confirm
        icon: "trash"; chevron: false; title: "Clear clipboard history…"; sub: "Pinned items stay"
        onActivated: page.confirm = true
    }
    RowLayout {
        visible: page.confirm
        Layout.fillWidth: true
        spacing: 8
        SetRow {
            title: "Yes, clear it"; danger: true; chevron: false
            onActivated: job.run(["clear"], r => { page.confirm = false; page.say(r && r.kept ? "Cleared. Your " + r.kept + " pinned item" + (r.kept === 1 ? " stays." : "s stay.") : "Clipboard history cleared."); }, e => page.say(e, true))
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
        text: "Passwords copied from password managers that mark them as secret are never kept or put on the selection. In Super+V, ⋯ on a text item opens it in a browser, shows it as a QR code, or edits it. Super+. opens the emoji picker."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
}
