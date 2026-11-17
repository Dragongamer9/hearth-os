// Hearth Settings › Bluetooth & devices › USB devices & drives (1.14, KDE's removable-storage actions and USB device
// prompts, in Hearth's look): what happens when a USB drive or memory card is plugged in (Ask, Open in Files, Nothing:
// Settings.removable.storage), the notice for a USB device plugged in for the first time (Settings.usbNotify), the USB
// devices plugged in now with their kind and speed, and where USBGuard runs, Allow / Block for each (hearth-usbdev).
// DeviceWatch.qml does what these say. Saved only when you pick.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "USB devices & drives"
    property var st: null
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["list"], r => page.replaceList("st", r), e => page.say(e, true)); }
    Component.onCompleted: load()
    Timer { interval: 5000; repeat: true; running: page.visible; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-usbdev" }
    readonly property string storage: (Settings.removable && Settings.removable.storage) || "ask"
    function setRemovable(k, v) { const o = Object.assign({}, Settings.removable || {}); o[k] = v; Settings.removable = o; }
    function iconFor(k) { return k === "Keyboard" ? "kbd" : k === "Mouse" ? "mouse" : k === "Storage" ? "drive" : k === "Webcam" || k === "Camera or phone" ? "cam" : k === "Sound" || k === "Sound and video" ? "headphones" : k === "Controller or input" ? "pad" : k.indexOf("Bluetooth") >= 0 ? "bt" : k === "Printer" ? "printer" : "link"; }

    SetCardGroup {
        title: "When a USB drive or memory card is plugged in"
        SetChoice {
            icon: "drive"; title: "Then"
            options: [{ value: "ask", label: "Ask me" }, { value: "open", label: "Open it in Files" }, { value: "nothing", label: "Nothing" }]
            current: page.storage
            onChosen: v => page.setRemovable("storage", v)
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Either way it shows in Files' sidebar, and Safely remove is there too."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: "New USB devices"
        SetSwitch {
            icon: "bell"; title: "Tell me about a USB device plugged in for the first time"
            sub: "Its name and what kind it is, once per device"
            checked: Settings.usbNotify
            onToggled: v => Settings.usbNotify = v
        }
        SetRow {
            visible: !!page.st && page.st.usbguard.running
            interactive: false; chevron: false; icon: "shield"; title: "USBGuard is on"
            sub: "A device it blocks asks you first: Allow once or Allow always below"
        }
    }
    SetCardGroup {
        title: page.st ? (page.st.devices.length ? "Plugged in now" : "Nothing plugged in") : "Looking…"
        Repeater {
            model: page.st ? page.st.devices : []
            delegate: ColumnLayout {
                id: ud
                required property var modelData
                Layout.fillWidth: true
                spacing: 3
                SetRow {
                    Layout.fillWidth: true
                    interactive: false; chevron: false
                    icon: page.iconFor(ud.modelData.kind)
                    title: ud.modelData.name
                    sub: [ud.modelData.kind, ud.modelData.maker, ud.modelData.speed, ud.modelData.vendor + ":" + ud.modelData.product].filter(x => x).join(" · ")
                    chip: ud.modelData.guard ? (ud.modelData.guard.target === "allow" ? "Allowed" : "Blocked") : !ud.modelData.authorized ? "Blocked" : ""
                    chipKind: ud.modelData.guard && ud.modelData.guard.target === "allow" ? "ok" : "bad"
                }
                RowLayout {
                    visible: !!ud.modelData.guard
                    Layout.leftMargin: 4; spacing: 8
                    SetButton { visible: !!ud.modelData.guard && ud.modelData.guard.target !== "allow"; text: "Allow once"; onActivated: job.run(["allow", ud.modelData.guard.id], () => page.load(), e => page.say(e, true)) }
                    SetButton { visible: !!ud.modelData.guard && ud.modelData.guard.target !== "allow"; text: "Allow always"; primary: true; onActivated: job.run(["allow", ud.modelData.guard.id, "always"], () => page.load(), e => page.say(e, true)) }
                    SetButton { visible: !!ud.modelData.guard && ud.modelData.guard.target === "allow" && ud.modelData.kind !== "Keyboard" && ud.modelData.kind !== "Mouse"; text: "Block"; ghost: true; onActivated: job.run(["block", ud.modelData.guard.id], () => page.load(), e => page.say(e, true)) }
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
