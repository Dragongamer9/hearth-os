// Device Manager's page (loaded by WinTools.qml). Windows' Device Manager in Hearth's look: the PC's devices in groups
// (Audio, Bluetooth, Cameras, Controllers, Disk drives, Display adapters, Keyboards and mice, Monitors, Network adapters,
// Processors, USB controllers, and Other devices when something has no group), a yellow warning on anything with a
// problem (no driver, missing firmware, an error the kernel logged), and the details on the right: driver, firmware,
// maker, where it's connected, with what can be done (Send logs, Disable / Enable for radios, the device's Settings
// page, Nvidia's driver settings).
// Read by `hearth-devmgr list` (next to this file) when it opens and on "Scan for hardware changes" (F5); nothing runs
// in between and nothing is written. Which groups are open is only for this window.
// Keyboard and controller: Up / Down move, Right / A open a group or go to the device's actions, Left / B come back,
// X (Menu) the actions, B in the list closes; F5 scans again.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

FocusScope {
    id: v
    focus: true
    readonly property string helper: wt.shellDir + "/hearth-devmgr"
    property var devData: null
    property string loadError: ""
    property var expanded: ({})
    property int sel: 0
    property string zone: "tree"         // tree | actions
    property int act: 0
    property string confirming: ""       // a device whose Disable was pressed once
    property string message: ""
    property string messageFor: ""
    property bool sending: false

    function start() { scan(); tree.forceActiveFocus(); }
    function scan() {
        if (lister.running) return;
        lister.running = true;
    }
    Process {
        id: lister
        command: ["python3", v.helper, "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                const line = text.trim().split("\n").pop() || "";
                if (line.startsWith("error: ")) { v.loadError = line.slice(7); return; }
                let d = null;
                try { d = JSON.parse(line); } catch (e) { d = null; }
                if (!d) { v.loadError = "Device Manager couldn't read this PC's devices."; return; }
                const keep = v.selected ? v.selected.id : "";
                const first = v.devData === null;
                v.loadError = "";
                v.devData = d;
                if (first) {
                    // Groups with a problem start open, so the warning is the first thing you see.
                    const e = {};
                    for (const g of d.groups) if (g.problems > 0) e[g.id] = true;
                    v.expanded = e;
                }
                v.reselect(keep, first);
            }
        }
    }

    // ------------------------------------------------ the tree's rows
    readonly property var rows: {
        const out = [];
        if (!devData) return out;
        for (const g of devData.groups) {
            if (!g.devices.length && g.id === "other") continue;
            out.push({ kind: "group", id: g.id, label: g.label, count: g.devices.length, problems: g.problems, open: expanded[g.id] === true });
            if (expanded[g.id] === true) for (const d of g.devices) out.push({ kind: "device", id: d.id, group: g.id, dev: d });
        }
        return out;
    }
    readonly property var row: rows[sel] || null
    readonly property var selected: row && row.kind === "device" ? row.dev : null
    function reselect(id, first) {
        let i = id ? rows.findIndex(r => r.id === id) : -1;
        if (i < 0 && first) i = rows.findIndex(r => r.kind === "device" && r.dev.problem);
        sel = Math.max(0, Math.min(rows.length - 1, i < 0 ? sel : i));
        tree.positionViewAtIndex(sel, ListView.Contain);
    }
    function toggle(gid, open) {
        const e = Object.assign({}, expanded);
        e[gid] = open === undefined ? !e[gid] : open;
        const id = row ? row.id : "";
        expanded = e;
        const i = rows.findIndex(r => r.id === id);
        sel = i >= 0 ? i : rows.findIndex(r => r.id === gid);
    }
    function move(d) {
        if (!rows.length) return;
        sel = Math.max(0, Math.min(rows.length - 1, sel + d));
        tree.positionViewAtIndex(sel, ListView.Contain);
        act = 0;
        confirming = "";
    }
    function selectByName(name) {
        const q = String(name || "").toLowerCase();
        for (const g of (devData ? devData.groups : [])) {
            const d = g.devices.find(x => x.name.toLowerCase().indexOf(q) >= 0);
            if (d) {
                if (expanded[g.id] !== true) toggle(g.id, true);
                sel = Math.max(0, rows.findIndex(r => r.id === d.id));
                tree.positionViewAtIndex(sel, ListView.Contain);
                return;
            }
        }
    }
    function stateJson() {
        return JSON.stringify({ loaded: devData !== null, error: loadError, count: devData ? devData.count : 0, problems: devData ? devData.problems : 0,
                                selected: selected ? selected.name : (row ? row.label : ""), zone: zone,
                                actions: actions.map(a => a.label), groups: devData ? devData.groups.map(g => g.label + " (" + g.devices.length + ")") : [] });
    }

    // ------------------------------------------------ what can be done for the selected device
    readonly property var settingsFor: ({
        audio: ["sound", "Sound settings"], bluetooth: ["bluetooth", "Bluetooth settings"], net: ["wifi", "Network settings"],
        monitors: ["display", "Display settings"], display: ["display", "Display settings"], controllers: ["controller", "Controller settings"]
    })
    readonly property var actions: {
        const d = selected;
        if (!d) return [];
        const out = [];
        if (d.problem) out.push({ id: "logs", label: v.sending && v.messageFor === d.id ? "Sending…" : "Send logs", primary: true });
        if (d.nvidia) out.push({ id: "nvidia", label: "Nvidia driver settings", primary: !d.problem });
        if (d.canDisable) out.push({ id: d.disabled ? "enable" : "disable", label: d.disabled ? "Enable device" : (confirming === d.id ? "Press again to turn it off" : "Disable device"), primary: d.disabled });
        const s = d.group === "input" ? (d.inputKind === "keyboard" ? ["keyboard", "Keyboard settings"] : ["mouse", "Mouse & touchpad settings"]) : settingsFor[d.group];
        if (s && !d.nvidia) out.push({ id: "settings", page: s[0], label: s[1] });
        if (d.group === "disks" && wt.installed("hearth-disks")) out.push({ id: "disks", label: "Open Disk Management" });
        if (d.group === "cpu") out.push({ id: "perf", label: "Performance in Task Manager" });
        if (!d.problem) out.push({ id: "logs", label: v.sending && v.messageFor === d.id ? "Sending…" : "Send logs" });
        return out;
    }
    function run(a) {
        const d = selected;
        if (!a || !d) return;
        if (a.id !== "disable") confirming = "";
        switch (a.id) {
        case "logs": sendLogs(d); break;
        case "nvidia": if (wt.settings) wt.settings.open("display"); break;
        case "settings": if (wt.settings) wt.settings.open(a.page); break;
        case "disks": Quickshell.execDetached(["hearth-disks"]); break;
        case "perf": if (wt.tasks) wt.tasks.open("performance:cpu"); break;
        case "disable":
            if (confirming !== d.id) { confirming = d.id; confirmTimer.restart(); break; }
            confirming = "";
            radio(d, "disable");
            break;
        case "enable": radio(d, "enable"); break;
        }
    }
    Timer { id: confirmTimer; interval: 5000; onTriggered: v.confirming = "" }

    Process {
        id: radioProc
        property string name: ""
        property bool off: false
        stdout: StdioCollector {
            onStreamFinished: {
                const line = text.trim().split("\n").pop() || "";
                if (line.startsWith("error: ")) SettingsNav.toast("Couldn't change " + radioProc.name + ": " + line.slice(7));
                else SettingsNav.toast(radioProc.name + (radioProc.off ? " is turned off. Enable device turns it back on." : " is on again"));
                v.scan();
            }
        }
    }
    function radio(d, what) {
        if (radioProc.running) return;
        radioProc.name = d.name;
        radioProc.off = what === "disable";
        radioProc.command = ["python3", helper, what, d.id];
        radioProc.running = true;
    }

    // Send logs: the same as Settings › Updates & recovery › Send logs to the developer, with this device in the note.
    Process {
        id: logsProc
        stdout: StdioCollector { id: logsOut }
        stderr: StdioCollector { id: logsErr }
        onExited: code => {
            v.sending = false;
            const m = /code: (\S+)/.exec(logsOut.text || "");
            v.message = code === 0 && m ? "Sent. Share this code with the developer: " + m[1]
                : "Couldn't send them: " + ((logsErr.text || "").trim().split("\n").pop() || "the Hearth app isn't on this system");
        }
    }
    function sendLogs(d) {
        if (logsProc.running) return;
        const bits = [d.name, d.ids, d.problem ? d.problem.title : "", (d.firmwareFiles || []).join(" ")].filter(x => x);
        sending = true;
        messageFor = d.id;
        message = "";
        logsProc.command = ["sh", "-c", 'if command -v nimbus >/dev/null 2>&1; then exec nimbus --send-logs "$1"; else exec hearth-send-logs "$1"; fi',
                            "sh", "Sent from Device Manager: " + bits.join(" · ")];
        logsProc.running = true;
    }

    // ------------------------------------------------ looks
    readonly property color warn: "#f2c14e"
    readonly property color warnBg: Theme.dark ? "#3a3326" : "#fbf0d2"
    readonly property color warnLine: Theme.dark ? "#6b5a33" : "#e2c77a"
    readonly property color warnText: Theme.dark ? "#ecdcb6" : "#5c4815"
    readonly property var icons: ({ audio: "vol", bluetooth: "bt", cameras: "cam", controllers: "pad", disks: "drive", display: "desktops",
                                     input: "kbd", monitors: "display", net: "wifi", cpu: "cpu", usb: "link", other: "help" })
    function iconFor(r) {
        if (r.kind === "device") {
            if (r.group === "net" && !r.dev.wifi) return "ethernet";
            if (r.group === "input" && r.dev.inputKind === "mouse") return "mouse";
        }
        return icons[r.kind === "device" ? r.group : r.id] || "dot";
    }

    Rectangle { anchors.fill: parent; color: Theme.bg }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ---- the toolbar
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 40
            color: Theme.panel
            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.line }
            RowLayout {
                anchors { fill: parent; leftMargin: 8; rightMargin: 12 }
                spacing: 6
                Rectangle {
                    implicitWidth: scanRow.implicitWidth + 20; implicitHeight: 30; radius: Theme.r(6)
                    color: scanMa.containsMouse ? Theme.panel3 : "transparent"
                    Row {
                        id: scanRow
                        anchors.centerIn: parent
                        spacing: 7
                        Icon { anchors.verticalCenter: parent.verticalCenter; name: "restart"; size: 15; color: Theme.text }
                        Text { text: lister.running ? "Scanning…" : "Scan for hardware changes"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    }
                    MouseArea { id: scanMa; anchors.fill: parent; hoverEnabled: true; onClicked: v.scan() }
                }
                Rectangle {
                    implicitWidth: expRow.implicitWidth + 20; implicitHeight: 30; radius: Theme.r(6)
                    color: expMa.containsMouse ? Theme.panel3 : "transparent"
                    readonly property bool allOpen: !!v.devData && v.devData.groups.every(g => !g.devices.length || v.expanded[g.id] === true)
                    Row {
                        id: expRow
                        anchors.centerIn: parent
                        Text { text: parent.parent.allOpen ? "Collapse all" : "Expand all"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    }
                    MouseArea {
                        id: expMa; anchors.fill: parent; hoverEnabled: true
                        onClicked: {
                            const e = {};
                            if (!parent.allOpen && v.devData) for (const g of v.devData.groups) e[g.id] = true;
                            v.expanded = e;
                            v.sel = Math.min(v.sel, v.rows.length - 1);
                        }
                    }
                }
                Item { Layout.fillWidth: true }
                Rectangle {
                    visible: !!v.devData && v.devData.problems > 0
                    implicitWidth: pill.implicitWidth + 22; implicitHeight: 24; radius: 12
                    color: v.warnBg
                    Row {
                        id: pill
                        anchors.centerIn: parent
                        spacing: 6
                        Icon { anchors.verticalCenter: parent.verticalCenter; name: "warn"; size: 13; color: v.warn }
                        Text {
                            text: v.devData ? (v.devData.problems === 1 ? "1 device needs attention" : v.devData.problems + " devices need attention") : ""
                            color: v.warn; font.family: Theme.font; font.pixelSize: 12
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // ---- the tree
            ListView {
                id: tree
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 10
                clip: true
                focus: true
                model: v.rows
                currentIndex: v.sel
                highlightFollowsCurrentItem: false
                keyNavigationEnabled: false
                boundsBehavior: Flickable.StopAtBounds
                header: Item {
                    width: tree.width; height: 30
                    Row {
                        anchors { left: parent.left; leftMargin: 6; verticalCenter: parent.verticalCenter }
                        spacing: 8
                        Icon { anchors.verticalCenter: parent.verticalCenter; name: "display"; size: 16; color: Theme.accent }
                        Text { text: v.devData ? v.devData.host : "This PC"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                    }
                }
                delegate: Rectangle {
                    id: rowItem
                    required property var modelData
                    required property int index
                    readonly property bool isGroup: modelData.kind === "group"
                    readonly property bool bad: isGroup ? modelData.problems > 0 : !!modelData.dev.problem
                    readonly property bool lit: v.sel === index
                    width: tree.width
                    height: 28
                    radius: Theme.r(5)
                    color: lit ? Theme.panel3 : rma.containsMouse ? Qt.rgba(Theme.panel3.r, Theme.panel3.g, Theme.panel3.b, 0.5) : "transparent"
                    border.width: lit && tree.activeFocus ? 1 : 0
                    border.color: Theme.accent
                    Row {
                        anchors { left: parent.left; leftMargin: rowItem.isGroup ? 12 : 36; verticalCenter: parent.verticalCenter }
                        spacing: 8
                        Text {
                            width: 10
                            anchors.verticalCenter: parent.verticalCenter
                            text: rowItem.isGroup ? (rowItem.modelData.count === 0 ? "" : rowItem.modelData.open ? "▾" : "›") : ""
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                        }
                        Icon {
                            anchors.verticalCenter: parent.verticalCenter
                            name: v.iconFor(rowItem.modelData)
                            size: 15
                            color: rowItem.bad && !rowItem.isGroup ? v.warn : rowItem.isGroup ? Theme.accent : Theme.muted
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: rowItem.isGroup ? rowItem.modelData.label : rowItem.modelData.dev.name
                            color: rowItem.isGroup && rowItem.modelData.count === 0 ? Theme.muted : Theme.text
                            opacity: !rowItem.isGroup && rowItem.modelData.dev.disabled ? 0.55 : 1
                            font.family: Theme.font; font.pixelSize: 13; font.weight: rowItem.isGroup ? Font.Medium : Font.Normal
                        }
                    }
                    Row {
                        anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                        spacing: 8
                        Text {
                            visible: !rowItem.isGroup && rowItem.modelData.dev.disabled === true
                            text: "Turned off"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                        }
                        Icon { visible: rowItem.bad && !rowItem.isGroup; name: "warn"; size: 14; color: v.warn }
                    }
                    MouseArea {
                        id: rma
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            tree.forceActiveFocus();
                            v.zone = "tree";
                            v.sel = rowItem.index;
                            v.act = 0;
                            if (rowItem.isGroup) v.toggle(rowItem.modelData.id);
                        }
                        onDoubleClicked: if (!rowItem.isGroup && v.actions.length) { v.zone = "actions"; v.act = 0; pane.forceActiveFocus(); }
                    }
                }
                Keys.onPressed: event => {
                    const r = v.row;
                    switch (event.key) {
                    case Qt.Key_Up: v.move(-1); break;
                    case Qt.Key_Down: v.move(1); break;
                    case Qt.Key_PageUp: v.move(-10); break;
                    case Qt.Key_PageDown: v.move(10); break;
                    case Qt.Key_Home: v.move(-v.rows.length); break;
                    case Qt.Key_End: v.move(v.rows.length); break;
                    case Qt.Key_Right:
                        if (!r) break;
                        if (r.kind === "group") { if (!r.open && r.count) v.toggle(r.id, true); else v.move(1); }
                        else if (v.actions.length) { v.zone = "actions"; v.act = 0; pane.forceActiveFocus(); }
                        break;
                    case Qt.Key_Left:
                        if (!r) break;
                        if (r.kind === "device") { v.sel = Math.max(0, v.rows.findIndex(x => x.kind === "group" && x.id === r.group)); }
                        else if (r.open) v.toggle(r.id, false);
                        break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: case Qt.Key_Menu:
                        if (!r) break;
                        if (r.kind === "group" && event.key !== Qt.Key_Menu) v.toggle(r.id);
                        else if (r.kind === "device" && v.actions.length) { v.zone = "actions"; v.act = 0; pane.forceActiveFocus(); }
                        break;
                    case Qt.Key_F5: v.scan(); break;
                    case Qt.Key_Escape: wt.closeDevices(); break;
                    default:
                        if (event.key === Qt.Key_R && (event.modifiers & Qt.ControlModifier)) { v.scan(); break; }
                        return;
                    }
                    event.accepted = true;
                }
                onActiveFocusChanged: if (activeFocus) v.zone = "tree"
            }

            // ---- the details
            Rectangle {
                id: pane
                Layout.preferredWidth: 380
                Layout.fillHeight: true
                color: Theme.panel
                Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: 1; color: Theme.line }
                activeFocusOnTab: false
                Keys.onPressed: event => {
                    const n = v.actions.length;
                    switch (event.key) {
                    case Qt.Key_Up: case Qt.Key_Left:
                        if (v.act > 0) v.act--; else if (event.key === Qt.Key_Left) tree.forceActiveFocus();
                        break;
                    case Qt.Key_Down: case Qt.Key_Right: if (v.act < n - 1) v.act++; break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: v.run(v.actions[v.act]); break;
                    case Qt.Key_Escape: case Qt.Key_Menu: case Qt.Key_Backspace: v.confirming = ""; tree.forceActiveFocus(); break;
                    case Qt.Key_F5: v.scan(); break;
                    default: return;
                    }
                    event.accepted = true;
                }
                onActiveFocusChanged: if (activeFocus) v.zone = "actions"

                ColumnLayout {
                    anchors { fill: parent; margins: 18 }
                    spacing: 14

                    // Loading / couldn't read
                    Text {
                        Layout.fillWidth: true
                        visible: v.devData === null
                        text: v.loadError !== "" ? v.loadError : "Looking for devices…"
                        wrapMode: Text.WordWrap
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                    }

                    // A group: how many, and how many need attention
                    ColumnLayout {
                        visible: !!v.row && v.row.kind === "group"
                        Layout.fillWidth: true
                        spacing: 6
                        Text { text: v.row && v.row.kind === "group" ? v.row.label : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                        Text {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            text: !v.row || v.row.kind !== "group" ? "" : v.row.count === 0 ? "Nothing of this kind is connected."
                                : (v.row.count === 1 ? "1 device" : v.row.count + " devices") + (v.row.problems ? " · " + v.row.problems + " need" + (v.row.problems === 1 ? "s" : "") + " attention" : "")
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        }
                    }

                    // A device
                    RowLayout {
                        visible: !!v.selected
                        Layout.fillWidth: true
                        spacing: 12
                        Rectangle {
                            implicitWidth: 44; implicitHeight: 44; radius: Theme.r(10)
                            color: v.selected && v.selected.problem ? v.warnBg : Theme.panel2
                            Icon { anchors.centerIn: parent; name: v.row ? v.iconFor(v.row) : "dot"; size: 24; color: v.selected && v.selected.problem ? v.warn : Theme.accent }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text { Layout.fillWidth: true; text: v.selected ? v.selected.name : ""; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                            Text {
                                Layout.fillWidth: true
                                text: v.selected ? [(v.devData.groups.find(g => g.id === v.selected.group) || {}).label, v.selected.ids].filter(x => x).join(" · ") : ""
                                elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                            }
                        }
                    }
                    Rectangle {
                        visible: !!v.selected && !!v.selected.problem
                        Layout.fillWidth: true
                        implicitHeight: probCol.implicitHeight + 24
                        radius: Theme.r(8)
                        color: v.warnBg
                        border.color: v.warnLine
                        RowLayout {
                            id: probCol
                            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
                            spacing: 10
                            Icon { Layout.alignment: Qt.AlignTop; name: "warn"; size: 18; color: v.warn }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4
                                Text { text: v.selected && v.selected.problem ? v.selected.problem.title : ""; color: v.warn; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                                Text { Layout.fillWidth: true; text: v.selected && v.selected.problem ? v.selected.problem.text : ""; wrapMode: Text.WordWrap; lineHeight: 1.15; color: v.warnText; font.family: Theme.font; font.pixelSize: 12 }
                            }
                        }
                    }
                    GridLayout {
                        visible: !!v.selected
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 12
                        rowSpacing: 7
                        Repeater {
                            model: {
                                const d = v.selected;
                                if (!d) return [];
                                const out = [];
                                out.push(["Driver", d.driver ? d.driver + (d.driverNote ? " (" + d.driverNote + ")" : "") : d.driverNote || "None"]);
                                if (d.driverVersion) out.push(["Version", d.driverVersion]);
                                out.push(["Firmware", d.firmware || (v.devData.kernelLog ? "Nothing missing" : "Can't check: the kernel log isn't readable")]);
                                if (d.maker) out.push(["Maker", d.maker]);
                                out.push(["Connected to", d.where || "–"]);
                                if (d.threads) out.push(["Threads", String(d.threads)]);
                                if (d.note) out.push(["Note", d.note]);
                                const flat = [];
                                for (const x of out) { flat.push({ t: x[0], k: true }); flat.push({ t: x[1], k: false }); }
                                return flat;
                            }
                            delegate: Text {
                                required property var modelData
                                Layout.preferredWidth: modelData.k ? 100 : -1
                                Layout.fillWidth: !modelData.k
                                Layout.alignment: Qt.AlignTop
                                text: modelData.t
                                wrapMode: Text.WordWrap
                                color: modelData.k ? Theme.muted : Theme.text
                                font.family: Theme.font; font.pixelSize: 12
                            }
                        }
                    }
                    Text {
                        visible: v.message !== "" && !!v.selected && v.messageFor === v.selected.id
                        Layout.fillWidth: true
                        text: v.message
                        wrapMode: Text.WordWrap
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                    }

                    Item { Layout.fillHeight: true }

                    // The actions
                    Flow {
                        Layout.fillWidth: true
                        spacing: 8
                        Repeater {
                            model: v.actions
                            delegate: Rectangle {
                                id: ab
                                required property var modelData
                                required property int index
                                readonly property bool lit: pane.activeFocus && v.act === index
                                width: modelData.primary ? parent.width : (parent.width - 8) / 2
                                height: 36
                                radius: Theme.r(6)
                                color: modelData.primary ? Theme.accent : abMa.containsMouse ? Theme.panel3 : "transparent"
                                border.width: lit ? 2 : modelData.primary ? 0 : 1
                                border.color: lit ? (modelData.primary ? Theme.text : Theme.accent) : Theme.line
                                Text {
                                    anchors.centerIn: parent
                                    width: parent.width - 16
                                    horizontalAlignment: Text.AlignHCenter
                                    elide: Text.ElideRight
                                    text: ab.modelData.label
                                    color: ab.modelData.primary ? Theme.accentInk : Theme.text
                                    font.family: Theme.font; font.pixelSize: 13; font.weight: ab.modelData.primary ? Font.DemiBold : Font.Normal
                                }
                                MouseArea {
                                    id: abMa
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: { v.act = ab.index; v.run(ab.modelData); }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ---- the status line
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 28
            color: Theme.panel
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
            Text {
                anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                text: !v.devData ? "" : (v.devData.count === 1 ? "1 device" : v.devData.count + " devices") + (v.devData.problems ? " · " + v.devData.problems + " need" + (v.devData.problems === 1 ? "s" : "") + " attention" : "")
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
            Text {
                anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                text: v.zone === "actions" ? "A choose · B back" : "A open · X actions · B close · F5 scan"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
        }
    }
}
