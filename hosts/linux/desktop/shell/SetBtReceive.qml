// Hearth Settings › Bluetooth › Receive files & this PC's name (1.14, KDE's Bluetooth receiving and adapter name, in
// Hearth's look): the name phones and other computers see (BlueZ's own, hearth-bt-receive rename; Put back gives
// BlueZ's usual one), Receive files (off by default; each file is asked about, BtReceive.qml), where they go, taking
// files from trusted devices without asking, and what came lately. Opening the page only reads.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Receive files & this PC's name"
    property var bt: null
    property var items: []
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-bt-receive" }
    Component.onCompleted: job.run(["name"], r => page.bt = r, e => page.say(e, true))
    FileView {
        path: Quickshell.env("HOME") + "/.local/state/nimbus/bt-received.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { const o = JSON.parse(text()); page.items = o && Array.isArray(o.items) ? o.items : []; } catch (e) { page.items = []; } }
    }
    Process {
        id: picker
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Save received files in' --getexistingdirectory \"$HOME\"; elif command -v zenity >/dev/null; then zenity --file-selection --directory --title='Save received files in' --filename=\"$HOME/\"; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f) Settings.btReceiveFolder = f; } }
        onExited: code => { if (code === 3) SettingsNav.toast("No folder picker installed"); }
    }
    function when(ms) { const d = new Date(ms); return d.toLocaleDateString(Qt.locale(), "d MMM") + " " + d.toLocaleTimeString(Qt.locale(), Settings.clock24 ? "HH:mm" : "h:mm AP"); }

    SetBanner {
        visible: !!page.bt && !page.bt.present
        Layout.fillWidth: true
        kind: "info"; icon: "bt"; title: "No Bluetooth on this PC"; sub: "Plug in a Bluetooth adapter, or turn it on in the PC's firmware setup"
    }
    SetCardGroup {
        title: "This PC"
        SetRow {
            interactive: false; chevron: false; icon: "desktops"
            title: page.bt ? (page.bt.name || "No name") : "Reading…"
            sub: page.bt && page.bt.address ? "What phones and other computers see · " + page.bt.address : "What phones and other computers see"
            chip: page.bt && page.bt.present ? (page.bt.powered ? "On" : "Off") : ""
            chipKind: page.bt && page.bt.powered ? "ok" : "warn"
        }
        SetField {
            id: nameField
            placeholder: "A new name, like Living room PC"
            onSubmitted: t => {
                if (!t.trim()) return;
                page.say("");
                job.run(["rename", t.trim()], r => { page.bt = r; nameField.text = ""; page.say("Renamed. Devices see the new name the next time they look."); }, e => page.say(e, true));
            }
        }
        SetRow {
            visible: !!page.bt && !!page.bt.systemName && page.bt.name !== page.bt.systemName
            icon: "undo"; chevron: false; title: "Put back the usual name"; sub: page.bt ? page.bt.systemName : ""
            onActivated: job.run(["reset-name"], r => { page.bt = r; page.say("The usual name is back."); }, e => page.say(e, true))
        }
    }
    SetCardGroup {
        title: "Receiving files"
        SetSwitch {
            icon: "share"; title: "Receive files"
            sub: "Phones and computers you've paired can send you files. You're asked about each one"
            checked: Settings.btReceive
            onToggled: v => Settings.btReceive = v
        }
        SetRow {
            icon: "folder"; chevron: true; title: "Save them in"
            sub: (Settings.btReceiveFolder || "~/Downloads").replace(Quickshell.env("HOME"), "~")
            onActivated: if (!picker.running) picker.running = true
        }
        SetRow {
            visible: Settings.btReceiveFolder !== ""
            icon: "undo"; chevron: false; title: "Back to Downloads"
            onActivated: Settings.btReceiveFolder = ""
        }
        SetSwitch {
            icon: "check"; title: "Take files from trusted devices without asking"
            sub: "Devices you trust in Your Bluetooth devices. Others are still asked about"
            checked: Settings.btReceiveTrusted
            onToggled: v => Settings.btReceiveTrusted = v
        }
    }
    SetCardGroup {
        title: page.items.length ? "Received lately" : "Nothing received yet"
        Repeater {
            model: page.items.slice(0, 10)
            delegate: SetRow {
                required property var modelData
                icon: /\.(jpe?g|png|heic|webp|gif)$/i.test(modelData.name) ? "image" : /\.(mp4|mov|mkv|webm)$/i.test(modelData.name) ? "film" : "doc"
                title: modelData.name
                sub: [modelData.from, page.when(modelData.at)].filter(x => x).join(" · ")
                chevron: true
                onActivated: Quickshell.execDetached(["xdg-open", modelData.path])
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
