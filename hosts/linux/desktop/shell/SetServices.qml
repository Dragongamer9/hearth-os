// Hearth Settings › Apps › Background services: the programs that run in the background for you (systemd --user): which
// are running, and a switch for whether each starts when you sign in (on starts it now too, off stops it). Hearth's own
// and the ones the desktop stands on are shown locked, with why. Nothing is switched unless you press (hearth-services).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Background services"
    property var list: null
    property string message: ""
    property bool messageBad: false
    property bool showLocked: false
    readonly property var mine: list ? list.filter(s => !s.locked) : []
    readonly property var locked: list ? list.filter(s => s.locked) : []

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["list"], r => page.replaceList("list", r.services || []), e => page.say(e, true)); }
    Component.onCompleted: load()
    Timer { interval: 15000; repeat: true; running: page.visible && !job.busy; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-services" }

    SetCardGroup {
        title: "Your background services"
        Repeater {
            model: page.mine
            delegate: SetSwitch {
                required property var modelData
                icon: "gear"
                title: modelData.name
                sub: (modelData.running ? "Running" : "Not running") + " · " + modelData.id.replace(/\.service$/, "")
                checked: modelData.enabled
                onToggled: v => job.run(["set", modelData.id, v ? "on" : "off"], () => { page.say(v ? modelData.name + " starts when you sign in." : modelData.name + " is off."); page.load(); },
                                        e => { page.say(e, true); page.load(); }, "", "set")
            }
        }
        Text {
            visible: !!page.list && page.mine.length === 0
            Layout.fillWidth: true
            text: "Nothing of yours runs in the background."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: "Hearth and the desktop"
        visible: page.locked.length > 0
        SetExpander {
            icon: "lock"; title: page.locked.length + " services Hearth needs"; sub: "Locked: switching them off would stop sound, apps or Hearth itself"
            Repeater {
                model: page.locked
                delegate: SetRow {
                    required property var modelData
                    interactive: false; chevron: false
                    icon: "lock"; title: modelData.name
                    sub: modelData.why + " · " + (modelData.running ? "running" : "not running")
                }
            }
        }
    }
}
