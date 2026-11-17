// The top of a KWin settings page and the box its controls sit in. Under labwc (or before KWin answered) the controls
// are shown greyed, never hidden, with why: these are KWin's settings and Desktop Mode runs Mantel on labwc. The banner
// takes focus (A opens Settings › Your desktop) so a controller can still read the page.
//   KwinGate { kw: kw; what: "Window effects"; SetSwitch { ... } ... }
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: gate
    property var kw
    property string what: "These settings"
    default property alias content: body.data
    readonly property bool on: !!kw && kw.live
    Layout.fillWidth: true
    spacing: 3

    SetBanner {
        visible: !!gate.kw && gate.kw.ready && !gate.on
        kind: "info"; icon: "window"
        title: "Needs Mantel on KWin"
        sub: gate.what + " are KWin's. Desktop Mode is running Mantel on labwc now, which doesn't have them: they work when " +
             "Mantel runs on KWin. Nothing here is changed until then."
        action: "Your desktop"
        onActivated: SettingsNav.openPage("your-desktop")
    }
    SetBanner {
        visible: !!gate.kw && gate.kw.preview
        kind: "info"; icon: "info"
        title: "Preview"
        sub: "KWin isn't running: these show what's saved, and KWin reads them when it starts."
    }
    SetBanner {
        visible: !!gate.kw && Object.keys(gate.kw.bad || {}).length > 0
        kind: "bad"
        title: "A KWin settings file couldn't be read"
        sub: !gate.kw ? "" : Object.keys(gate.kw.bad).map(k => gate.kw.bad[k]).join(" · ") + ". Hearth won't write over it; nothing in it was lost."
    }
    Text {
        visible: !!gate.kw && !gate.kw.ready
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Asking KWin…"
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    ColumnLayout {
        id: body
        Layout.fillWidth: true
        spacing: 3
        enabled: gate.on
        opacity: gate.on ? 1 : 0.45
    }
    Text {
        visible: !!gate.kw && gate.kw.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 6
        text: gate.kw ? gate.kw.message : ""; color: gate.kw && gate.kw.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
