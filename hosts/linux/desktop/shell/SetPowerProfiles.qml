// Hearth Settings › Power › Plugged in, battery & lid (1.14, KDE's AC / battery / lid profiles, in Hearth's look): side
// by side, what happens plugged in and on battery (screen off, sleep, power mode), and what closing the lid does in
// each case. The times are desktop.json keys (screenOffMinutes / sleepMinutes plugged in, screenOffBattery /
// sleepBattery on battery: -1 = "as plugged in"); the power mode is hardware.json's power.plugged / power.unplugged
// (hearth-hwset, the same as Task Manager's "When plugged in"); the lid is logind's (hearth-admin lid). Each is saved
// only when you pick it; opening the page writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.UPower

SetPage {
    id: page
    title: "Plugged in, battery & lid"
    property var st: null              // hearth-powerset status
    property var hw: null              // hearth-hwset status (saved power.plugged / power.unplugged)
    property string message: ""
    property bool messageBad: false
    readonly property bool hasBattery: !!(UPower.displayDevice && UPower.displayDevice.isLaptopBattery) || !!(st && st.batteries && st.batteries.length)
    readonly property bool canHib: !!(st && st.hibernate && st.hibernate.can)
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        job.run(["status"], r => page.st = r, e => page.say(e, true));
        hwStatus.run(["status"], r => page.hw = r, () => page.hw = null);
    }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-powerset" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    SetJob { id: hwStatus; helper: "/usr/libexec/nimbus/hearth-hwset" }
    SetJob { id: hwJob; helper: "/usr/libexec/nimbus/hearth-hwset"; admin: true }
    function saved(k, d) { return page.hw && page.hw.saved && page.hw.saved[k] !== undefined ? page.hw.saved[k] : d; }
    readonly property var offChoices: [{ value: 0, label: "Never" }, { value: 2, label: "2 min" }, { value: 5, label: "5 min" }, { value: 10, label: "10 min" }, { value: 15, label: "15 min" }, { value: 30, label: "30 min" }]
    readonly property var sleepChoices: [{ value: 0, label: "Never" }, { value: 10, label: "10 min" }, { value: 15, label: "15 min" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }, { value: 120, label: "2 hours" }]
    readonly property var modeChoices: [{ value: "off", label: "Don't change" }, { value: "performance", label: "Performance" }, { value: "balanced", label: "Balanced" }, { value: "power-saver", label: "Battery saver" }]
    readonly property var lidChoices: [{ value: "suspend", label: "Sleep" }].concat(canHib ? [{ value: "hibernate", label: "Hibernate" }] : [])
        .concat([{ value: "lock", label: "Lock the screen" }, { value: "poweroff", label: "Shut down" }, { value: "ignore", label: "Nothing" }])
    function setLid(k, v) {
        const l = Object.assign({ battery: "suspend", ac: "suspend", docked: "ignore" }, page.st ? page.st.lid : {});
        l[k] = v;
        say("");
        admin.run(["lid", l.battery, l.ac, l.docked], r => { page.say(r.afterRestart ? "Saved. It's used from the next start." : "Saved."); page.load(); }, e => page.say(e, true), "", "lid");
    }

    // The two columns: plugged in, on battery.
    component Column2: Rectangle {
        id: col2
        property string icon
        property string head
        property string note
        default property alias body: inner.data
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignTop
        implicitHeight: inner.implicitHeight + 70
        radius: SetStyle.bigRadius
        color: SetStyle.card
        border.color: SetStyle.tint(Theme.text, 0.06)
        RowLayout {
            x: 18; y: 16; width: parent.width - 36
            spacing: 10
            Rectangle { implicitWidth: 34; implicitHeight: 34; radius: 17; color: SetStyle.tint(Theme.accent, 0.16); Icon { anchors.centerIn: parent; name: col2.icon; size: 18; color: Theme.accent } }
            ColumnLayout {
                spacing: 0
                Text { text: col2.head; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Text { visible: col2.note !== ""; text: col2.note; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
        ColumnLayout { id: inner; x: 6; y: 60; width: parent.width - 12; spacing: 3 }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 12
        Column2 {
            icon: "power"; head: "Plugged in"; note: UPower.onBattery ? "" : "In use now"
            SetChoice { title: "Screen off after"; options: page.offChoices; current: Settings.screenOffMinutes; onChosen: v => Settings.screenOffMinutes = v }
            SetChoice { title: "Sleep after"; options: page.sleepChoices; current: Settings.sleepMinutes; onChosen: v => Settings.sleepMinutes = v }
            SetDropdown {
                visible: page.hasBattery && !!page.hw
                title: "Power mode"; options: page.modeChoices; current: page.saved("power.plugged", "off"); boxWidth: 200
                onChosen: v => hwJob.run(["set", "power.plugged", v], () => page.load(), e => page.say(e, true))
            }
        }
        Column2 {
            visible: page.hasBattery
            icon: "batt"; head: "On battery"; note: UPower.onBattery ? "In use now" : ""
            SetChoice {
                title: "Screen off after"
                options: [{ value: -1, label: "As plugged in" }].concat(page.offChoices)
                current: Settings.screenOffBattery; onChosen: v => Settings.screenOffBattery = v
            }
            SetChoice {
                title: "Sleep after"
                options: [{ value: -1, label: "As plugged in" }].concat(page.sleepChoices)
                current: Settings.sleepBattery; onChosen: v => Settings.sleepBattery = v
            }
            SetDropdown {
                visible: !!page.hw
                title: "Power mode"; options: page.modeChoices; current: page.saved("power.unplugged", "off"); boxWidth: 200
                onChosen: v => hwJob.run(["set", "power.unplugged", v], () => page.load(), e => page.say(e, true))
            }
        }
    }
    SetRow {
        visible: !page.hasBattery
        interactive: false; icon: "info"; title: "This PC has no battery"; sub: "The times above are the ones it uses"
    }

    SetCardGroup {
        title: "When the lid is closed"
        visible: !!page.st && (page.st.lid.present || page.hasBattery)
        SetDropdown {
            icon: "batt"; title: "On battery"; options: page.lidChoices; current: page.st ? page.st.lid.battery : "suspend"; boxWidth: 220
            onChosen: v => page.setLid("battery", v)
        }
        SetDropdown {
            icon: "power"; title: "Plugged in"; options: page.lidChoices; current: page.st ? page.st.lid.ac : "suspend"; boxWidth: 220
            onChosen: v => page.setLid("ac", v)
        }
        SetDropdown {
            icon: "display"; title: "With another screen attached"; sub: "Docked, or an external monitor plugged in"
            options: page.lidChoices; current: page.st ? page.st.lid.docked : "ignore"; boxWidth: 220
            onChosen: v => page.setLid("docked", v)
        }
        Text {
            visible: !!page.st && page.st.lid.setBy && page.st.lid.setBy.length > 0
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.st ? "Also set in " + page.st.lid.setBy.join(", ") + " (Hearth's choice is read after it)" : ""
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
