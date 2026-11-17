// Hearth Settings › Bluetooth & devices › Thunderbolt: docks, drives and screens on Thunderbolt or USB4 (bolt, through
// hearth-thunderbolt): which are plugged in, which may connect, and allowing one once or always (bolt remembers it), or
// forgetting it. Nothing is allowed unless you press.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Thunderbolt"
    property var st: null
    property string picked: ""
    property string message: ""
    property bool messageBad: false
    readonly property var devs: st ? st.devices : []

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: load()
    Timer { interval: 5000; repeat: true; running: page.visible && !job.busy; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-thunderbolt" }
    function stateText(d) {
        return !d.connected ? "Not plugged in" + (d.stored ? " · remembered" : "")
             : d.authorized ? "Connected" + (d.stored ? " · always allowed" : " · allowed this time")
             : "Plugged in, waiting: allow it to use it";
    }
    function securityText(s) {
        return s === "none" || s === "dponly" ? "Every device connects by itself (set in your PC's firmware setup)"
             : s === "user" || s === "secure" ? "New devices wait until you allow them"
             : s === "usbonly" ? "Only USB on these ports (set in your PC's firmware setup)" : "";
    }

    SetCardGroup {
        title: "Thunderbolt and USB4"
        SetRow {
            visible: !!page.st && !page.st.available
            interactive: false; chevron: false; icon: "info"
            title: "No Thunderbolt here"; sub: "This PC has no Thunderbolt or USB4 ports, or bolt isn't running"
        }
        SetRow {
            visible: !!page.st && page.st.available && page.securityText(page.st.security) !== ""
            interactive: false; chevron: false; icon: "shield"
            title: "Security"; sub: page.st ? page.securityText(page.st.security) : ""
        }
        Repeater {
            model: page.devs
            delegate: ColumnLayout {
                id: dr
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                SetRow {
                    icon: "link"
                    title: (dr.modelData.vendor ? dr.modelData.vendor + " " : "") + dr.modelData.name
                    sub: page.stateText(dr.modelData) + (dr.modelData.generation ? " · " + dr.modelData.generation : "")
                    chip: dr.modelData.connected && !dr.modelData.authorized ? "Waiting" : dr.modelData.authorized ? "On" : ""
                    chipKind: dr.modelData.connected && !dr.modelData.authorized ? "warn" : "ok"
                    onActivated: page.picked = page.picked === dr.modelData.uuid ? "" : dr.modelData.uuid
                }
                ColumnLayout {
                    visible: page.picked === dr.modelData.uuid
                    Layout.fillWidth: true
                    Layout.leftMargin: 52
                    spacing: 8
                    SetRow {
                        visible: dr.modelData.connected && !dr.modelData.authorized
                        icon: "check"; chevron: false; title: "Allow this time"; sub: "Until it's unplugged"
                        onActivated: job.run(["allow", dr.modelData.uuid, "once"], r => { page.st = r; page.say("Allowed."); }, e => page.say(e, true), "", "allow")
                    }
                    SetRow {
                        visible: !dr.modelData.stored
                        icon: "lock"; chevron: false; title: "Always allow it"; sub: "It connects by itself from now on"
                        onActivated: job.run(["allow", dr.modelData.uuid, "always"], r => { page.st = r; page.say("Always allowed."); }, e => page.say(e, true), "", "allow")
                    }
                    SetRow {
                        visible: dr.modelData.stored
                        icon: "trash"; danger: true; chevron: false; title: "Forget it"; sub: "It waits for you again next time"
                        onActivated: job.run(["forget", dr.modelData.uuid], r => { page.st = r; page.picked = ""; page.say("Forgotten."); }, e => page.say(e, true), "", "forget")
                    }
                }
            }
        }
        Text {
            visible: !!page.st && page.st.available && page.devs.length === 0
            Layout.fillWidth: true
            text: "Nothing plugged in or remembered."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
