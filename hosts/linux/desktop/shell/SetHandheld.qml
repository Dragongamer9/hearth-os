// Hearth Settings › Handheld: Hearth's own app for Handheld Daemon (HHD), in place of HHD's hhd-ui, which Hearth OS
// doesn't ship. Every setting HHD offers for this device (controller emulation, gyro, back buttons, power limit and
// fan, lights, HHD's own options...), drawn from HHD's settings tree, so a newer HHD's options appear by themselves.
// hearth-hhd page / apply talk to HHD's local API (its socket /run/hhd/api, or 127.0.0.1:5335 with its token) and
// check each change against the tree first. Advanced settings (HHD's "advanced" / "expert") behind a switch.
// Opened from Settings, Quick Access (when HHD runs) and Settings › Controller. Without HHD: says so, nothing else.
// Steam Game Mode has its own power and controller settings: each section says where Steam takes over there.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Handheld"
    property var hh: null              // hearth-hhd page
    property string err: ""
    property bool advanced: false
    property string picking: ""        // the key of a choice whose list is open
    property string armed: ""          // a button (HHD action) pressed once: press again to run it
    property var pending: ({})         // values just changed here, shown until HHD answers
    readonly property var pickItem: {
        if (!hh || !hh.sections || picking === "") return null;
        for (const s of hh.sections) for (const i of s.items) if (i.key === picking) return i;
        return null;
    }
    readonly property int advancedCount: {
        let n = 0;
        if (hh && hh.sections) for (const s of hh.sections) for (const i of s.items) if (i.advanced && i.kind !== "group") n++;
        return n;
    }

    SetJob { id: job; helper: "hearth-hhd" }
    function load() {
        job.run(["page"], r => { page.replaceList("hh", r); page.err = ""; }, e => page.err = e, "", "load");
    }
    function apply(key, value) {
        const p = Object.assign({}, pending); p[key] = value; pending = p;
        job.run(["apply", key, JSON.stringify(value)], r => { page.pending = ({}); page.replaceList("hh", r); page.err = ""; },
            e => { page.pending = ({}); page.err = e; page.load(); }, "", "apply");
    }
    function val(item) { return pending[item.key] !== undefined ? pending[item.key] : item.value; }
    function label(item) {
        const v = String(val(item));
        const o = (item.options || []).find(x => x.id === v);
        return o ? o.label : v;
    }
    function fewChoices(item) {
        const o = item.options || [];
        return o.length >= 2 && o.length <= 4 && o.reduce((a, x) => a + x.label.length, 0) <= 36;
    }
    // An icon for a Handheld Daemon setting, from its key (power, fan, lights, controller, screen...).
    function iconFor(d) {
        const k = d && d.key ? String(d.key).toLowerCase() : "";
        return /tdp|power|boost|cpu|gpu|charge/.test(k) ? "power" : /fan/.test(k) ? "fan" : /rgb|led|light/.test(k) ? "bulb"
            : /batt/.test(k) ? "batt" : /display|screen|bright/.test(k) ? "display" : /touch|mouse/.test(k) ? "mouse"
            : /keyboard|osk/.test(k) ? "kbd" : /controller|gamepad|button|stick|trigger|gyro|rumble|vibr|swap|layout/.test(k) ? "pad" : "gear";
    }
    function pick(key) { picking = key; Qt.callLater(page.focusFirst); }
    function unpick() { const k = picking; picking = ""; Qt.callLater(page.focusFirst); }
    Component.onCompleted: load()
    // HHD changes things by itself too (Steam's power settings, its own shortcuts): read again now and then.
    Timer { interval: 6000; repeat: true; running: page.visible && page.picking === "" && !job.busy; onTriggered: page.load() }
    // Sliders move in small steps: send once they stop.
    Timer { id: later; interval: 700; property string key: ""; property var value; onTriggered: page.apply(key, value) }

    // ------------------------------------------------------------ the daemon
    Rectangle {
        visible: page.picking === ""
        Layout.fillWidth: true
        implicitHeight: statusCol.implicitHeight + 28
        radius: Theme.r(12)
        color: Theme.panel2
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            spacing: 14
            Rectangle {
                Layout.preferredWidth: 44; Layout.preferredHeight: 44; radius: Theme.r(12)
                color: page.hh && page.hh.running ? Theme.accent : Theme.panel3
                Icon { anchors.centerIn: parent; name: "pad"; size: 24; color: page.hh && page.hh.running ? Theme.accentInk : Theme.muted }
            }
            ColumnLayout {
                id: statusCol
                Layout.fillWidth: true
                spacing: 3
                Text {
                    Layout.fillWidth: true
                    text: !page.hh ? (page.err ? "Couldn't read Handheld Daemon" : "Looking for Handheld Daemon…")
                        : page.hh.running ? "Handheld Daemon" + (page.hh.version ? "  ·  " + String(page.hh.version).slice(0, 16) : "")
                        : "Handheld Daemon isn't running"
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight
                }
                Text {
                    Layout.fillWidth: true
                    text: page.hh && page.hh.running
                        ? ((page.hh.controllers && page.hh.controllers.length ? page.hh.controllers.join(", ") + ". " : "")
                           + "Its settings for this device. Changes apply right away, in games too.")
                        : page.hh ? "Handheld Daemon makes a handheld's built-in controls, power limit, fan and lights work (ROG Ally, Legion Go, Steam Deck, MSI Claw and more). It isn't running here, so there's nothing to set: on a desktop or laptop that's expected."
                        : page.err
                    color: page.err && !page.hh ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                }
            }
        }
    }
    SetRow {
        visible: page.picking === "" && (!page.hh || !page.hh.running)
        icon: "restart"; title: "Check again"; chevron: false
        onActivated: { page.hh = null; page.load(); }
    }
    Text {
        visible: page.picking === "" && !!page.hh && page.hh.running
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "In Steam Game Mode, Steam has its own Performance and controller settings; the sections below say where Steam takes over."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.picking === "" && !!page.hh && page.err !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.err; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetSwitch {
        visible: page.picking === "" && !!page.hh && page.hh.running && page.advancedCount > 0
        icon: "gear"; title: "Show advanced settings"
        sub: page.advancedCount + " more, for tinkering (Handheld Daemon's advanced and expert options)"
        checked: page.advanced
        onToggled: on => page.advanced = on
    }

    // ------------------------------------------------------------ every section HHD has
    Repeater {
        model: page.picking === "" && page.hh && page.hh.running ? page.hh.sections : []
        // Each section is a titled group of cards (SetCardGroup), as in the rest of Settings.
        delegate: SetCardGroup {
            id: sec
            required property var modelData
            readonly property var shown: modelData.items.filter(i => page.advanced || !i.advanced)
            visible: shown.some(i => i.kind !== "group")
            title: sec.modelData.title
            Rectangle {
                visible: sec.modelData.steamNote !== ""
                Layout.fillWidth: true
                implicitHeight: noteTxt.implicitHeight + 18
                radius: Theme.r(10)
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.12)
                border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.35)
                RowLayout {
                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                    spacing: 10
                    Text { text: "Steam"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 0.8 }
                    Text { id: noteTxt; Layout.fillWidth: true; text: sec.modelData.steamNote; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
                }
            }
            Repeater {
                model: sec.shown
                delegate: Loader {
                    id: row
                    required property var modelData
                    readonly property var d: modelData
                    Layout.fillWidth: true
                    Layout.leftMargin: Math.min(3, d.depth) * 22
                    sourceComponent: d.kind === "bool" ? boolC : d.kind === "choice" ? (page.fewChoices(d) ? segC : listC)
                        : d.kind === "number" ? numC : d.kind === "action" ? actC : d.kind === "group" ? groupC
                        : d.kind === "color" ? colorC : textC
                }
            }
        }
    }

    // ------------------------------------------------------------ a long list of choices, picked one by one
    SetRow { visible: page.picking !== ""; icon: "back"; title: "Back"; chevron: false; onActivated: page.unpick() }
    SetHeader { visible: page.picking !== ""; text: page.pickItem ? page.pickItem.title : "" }
    Text {
        visible: page.picking !== "" && !!page.pickItem && page.pickItem.hint !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.pickItem ? page.pickItem.hint : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.pickItem ? page.pickItem.options : []
        delegate: SetRow {
            required property var modelData
            icon: String(page.val(page.pickItem)) === modelData.id ? "check" : "dot"
            title: modelData.label; chevron: false
            value: String(page.val(page.pickItem)) === modelData.id ? "In use" : ""
            onActivated: { const it = page.pickItem; page.apply(it.key, modelData.raw !== undefined ? modelData.raw : modelData.id); page.unpick(); }
        }
    }

    // ------------------------------------------------------------ the kinds of rows
    Component {
        id: boolC
        SetSwitch {
            readonly property var d: parent ? parent.d : null
            icon: page.iconFor(d)
            title: d ? d.title : ""; sub: d ? d.hint : ""
            checked: d ? !!page.val(d) : false
            onToggled: on => page.apply(d.key, on)
        }
    }
    Component {
        id: segC
        SetChoice {
            readonly property var d: parent ? parent.d : null
            icon: page.iconFor(d)
            title: d ? d.title : ""; sub: d ? d.hint : ""
            options: d ? d.options.map(o => ({ value: o.id, label: o.label })) : []
            current: d ? String(page.val(d)) : ""
            onChosen: v => { const o = d.options.find(x => x.id === v); page.apply(d.key, o && o.raw !== undefined ? o.raw : v); }
        }
    }
    Component {
        id: listC
        SetRow {
            readonly property var d: parent ? parent.d : null
            icon: page.iconFor(d)
            title: d ? d.title : ""; sub: d ? d.hint : ""
            value: d ? page.label(d) : ""
            onActivated: page.pick(d.key)
        }
    }
    Component {
        id: numC
        SetSlider {
            readonly property var d: parent ? parent.d : null
            readonly property real span: d ? d.max - d.min : 1
            readonly property real cur: d ? Number(page.val(d)) : 0
            icon: d && d.key.indexOf("tdp") >= 0 ? "power" : "gear"
            title: d ? d.title : ""
            value: d ? (cur - d.min) / span : 0
            step: d ? Math.max(0.01, d.step / span) : 0.05
            valueText: d ? (d.int ? Math.round(cur) : Math.round(cur * 100) / 100) + (d.unit ? " " + d.unit : "") : ""
            onMoved: v => {
                let n = d.min + Math.round(v * span / d.step) * d.step;
                n = Math.max(d.min, Math.min(d.max, d.int ? Math.round(n) : Math.round(n * 1000) / 1000));
                const p = Object.assign({}, page.pending); p[d.key] = n; page.pending = p;
                later.key = d.key; later.value = n; later.restart();
            }
        }
    }
    Component {
        id: actC
        SetRow {
            readonly property var d: parent ? parent.d : null
            icon: "restart"; chevron: false
            title: d ? d.title : ""; sub: d ? d.hint : ""
            value: d && page.armed === d.key ? "Press again to confirm" : ""
            onActivated: { if (page.armed === d.key) { page.armed = ""; page.apply(d.key, true); } else page.armed = d.key; }
            onActiveFocusChanged: if (!activeFocus && d && page.armed === d.key) page.armed = ""
        }
    }
    Component {
        id: groupC
        Text {
            readonly property var d: parent ? parent.d : null
            topPadding: 6
            text: d ? d.title : ""
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
        }
    }
    Component {
        id: textC
        SetRow {
            readonly property var d: parent ? parent.d : null
            icon: page.iconFor(d)
            interactive: false; chevron: false
            title: d ? d.title : ""
            readonly property string v: d ? String(d.value).trim() : ""
            value: v.length <= 28 && v.indexOf("\n") < 0 ? v : ""
            sub: v.length <= 28 && v.indexOf("\n") < 0 ? (d ? d.hint : "") : v.replace(/\n+/g, " ")
        }
    }
    Component {
        id: colorC
        SetRow {
            readonly property var d: parent ? parent.d : null
            icon: page.iconFor(d)
            interactive: false; chevron: false
            title: d ? d.title : ""; sub: d ? d.hint : ""
            value: d ? String(d.value) : ""
            Rectangle { anchors { right: parent.right; rightMargin: 90; verticalCenter: parent.verticalCenter } width: 22; height: 22; radius: 11; color: parent.d ? parent.d.value : "transparent"; border.color: Theme.line }
        }
    }
}
