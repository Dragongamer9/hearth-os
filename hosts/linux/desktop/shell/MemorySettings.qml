// Memory › Swap & settings (1.11, the owner's approved mockup docs/mockups/next-update/task-manager-swap-settings.png):
// compressed memory (zram), Keep memory free, the drive swap file and When memory runs out (the memory guard). Shown in
// Task Manager › Performance › Memory and Settings › System › Memory. Reads `hearth-memory status`; changes are kept
// here until you press Save (or Try until restart), then go to `pkexec hearth-memory set` (polkit:
// 49-hearth-memory.rules). Nothing here changes by itself: opening it writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item {
    id: ms
    property string helper: "/usr/libexec/nimbus/hearth-memory"
    property var st: null                 // the last status
    property var draft: ({})              // what you changed and haven't saved: {zram:{}, keep:{}, swappiness, swapfile:{}, guard:{}, swapGb}
    property string note: ""
    property bool noteBad: false
    property bool busy: false
    property string pendingSwap: ""       // a swap-file size to make after the settings are saved
    implicitHeight: col.implicitHeight
    readonly property real gib: 1073741824
    readonly property var cur: st ? st.settings : null
    function val(group, key, dflt) {
        const d = draft[group];
        if (d && d[key] !== undefined) return d[key];
        return cur && cur[group] && cur[group][key] !== undefined ? cur[group][key] : dflt;
    }
    function put(group, key, value) {
        const d = JSON.parse(JSON.stringify(draft));
        d[group] = Object.assign({}, d[group] || {});
        d[group][key] = value;
        draft = d;
    }
    function putKeep(mode) { const d = JSON.parse(JSON.stringify(draft)); d.keep = { mode: mode }; draft = d; }
    readonly property bool dirty: Object.keys(draft).length > 0
    readonly property string keepMode: draft.keep ? draft.keep.mode : (cur && cur.keep ? cur.keep.mode : "2gb")
    readonly property int swappiness: draft.swappiness !== undefined ? draft.swappiness : (cur ? cur.swappiness : 200)
    readonly property int swapGb: draft.swapGb !== undefined ? draft.swapGb : (st ? st.swapfile.gb : 0)
    function gb(b) { return (b / gib).toFixed(b >= 10 * gib ? 0 : 1); }

    function refresh() { if (!statusProc.running) statusProc.running = true; }
    Component.onCompleted: refresh()
    Timer { interval: 3000; repeat: true; running: ms.visible && !ms.busy; onTriggered: ms.refresh() }
    Process {
        id: statusProc
        command: ["python3", ms.helper, "status"]
        stdout: StdioCollector { onStreamFinished: { try { ms.st = JSON.parse(text.trim().split("\n").pop()); } catch (e) {} } }
    }
    Process {
        id: act
        property var then: null
        stdout: StdioCollector {
            onStreamFinished: {
                const last = text.trim().split("\n").pop() || "";
                ms.busy = false;
                if (last.indexOf("error: ") === 0) { ms.say(last.slice(7), true); return; }
                let r = {};
                try { r = JSON.parse(last); } catch (e) {}
                const notes = (r.notes || []).join(" · ");
                if (act.then) { const t = act.then; act.then = null; t(r); return; }
                ms.say((r.tried ? "Tried until the next restart" : "Saved ✓") + (notes ? " · " + notes : ""), false);
                ms.refresh();
            }
        }
    }
    function run(args, then) {
        busy = true;
        act.then = then || null;
        act.command = ["pkexec", helper].concat(args);
        act.running = true;
    }
    function say(t, bad) { note = t; noteBad = !!bad; }
    function changes() {
        const c = {};
        for (const k of ["zram", "keep", "swapfile", "guard"]) if (draft[k]) c[k] = draft[k];
        if (draft.swappiness !== undefined) c.swappiness = draft.swappiness;
        return c;
    }
    function save(tryOnly) {
        const c = changes();
        const sw = draft.swapGb !== undefined && st && draft.swapGb !== st.swapfile.gb ? String(draft.swapGb) : "";
        draft = ({});
        const doSet = () => {
            if (Object.keys(c).length) run(["set"].concat(tryOnly ? ["--try"] : []).concat([JSON.stringify(c)]));
            else { say("Saved ✓", false); refresh(); }
        };
        if (sw !== "" && !tryOnly) run(["swapfile", sw], r => { if (r.ok) doSet(); else { say("The swap file didn't change", true); } });
        else doSet();
    }

    // ---- small parts
    component Card: Rectangle {
        id: card
        default property alias body: inner.data
        property string title
        property string sub
        property string badge: ""
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignTop
        implicitHeight: head.implicitHeight + inner.implicitHeight + 46
        radius: Theme.r(10)
        color: Theme.panel
        border.color: Theme.line
        ColumnLayout {
            id: head
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 18 }
            spacing: 4
            RowLayout {
                Layout.fillWidth: true
                Text { text: card.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Rectangle {
                    visible: card.badge !== ""
                    width: bt.implicitWidth + 10; height: 18; radius: 4
                    color: "transparent"; border.color: Theme.flame
                    Text { id: bt; anchors.centerIn: parent; text: card.badge; color: Theme.flame; font.family: Theme.font; font.pixelSize: 10 }
                }
                Item { Layout.fillWidth: true }
            }
            Text { Layout.fillWidth: true; text: card.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        }
        ColumnLayout {
            id: inner
            anchors { left: parent.left; right: parent.right; top: head.bottom; leftMargin: 18; rightMargin: 18; topMargin: 12 }
            spacing: 12
        }
    }
    component Line: RowLayout {
        id: line
        property string title
        property string sub
        default property alias tailItems: tail.data   // not "right": Item.right is FINAL
        Layout.fillWidth: true
        spacing: 12
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text { text: line.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { Layout.fillWidth: true; visible: line.sub !== ""; text: line.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap }
        }
        RowLayout { id: tail; spacing: 8 }
    }
    component Chips: Row {
        id: chips
        property var options: []           // [[value, label]]
        property string value
        signal chose(string v)
        spacing: 0
        Repeater {
            model: chips.options
            delegate: Rectangle {
                required property var modelData
                readonly property bool on: chips.value === modelData[0]
                width: ct.implicitWidth + 22; height: 30
                radius: Theme.r(6)
                color: on ? Theme.accent : cm.containsMouse ? Theme.panel3 : Theme.panel2
                border.color: Theme.line
                Text { id: ct; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                MouseArea { id: cm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: chips.chose(parent.modelData[0]) }
            }
        }
    }
    component Toggle: Rectangle {
        property bool checked
        signal flipped()
        width: 42; height: 22; radius: 11
        color: checked ? Theme.accent : "transparent"
        border.width: checked ? 0 : 1; border.color: Theme.muted
        Rectangle { width: 14; height: 14; radius: 7; y: 4; x: parent.checked ? parent.width - 18 : 4; color: parent.checked ? Theme.accentInk : Theme.muted; Behavior on x { NumberAnimation { duration: Theme.quick } } }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: parent.flipped() }
    }
    component Slider: Item {
        property real from: 0
        property real to: 100
        property real value: 0
        property real mark: -1
        signal moved(real v)
        implicitWidth: 260; implicitHeight: 24
        Rectangle { id: track; anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 4; radius: 2; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0, Math.min(1, (parent.parent.value - parent.parent.from) / Math.max(1, parent.parent.to - parent.parent.from))); height: 4; radius: 2; color: Theme.accent } }
        Rectangle { visible: parent.mark >= parent.from; width: 2; height: 16; anchors.verticalCenter: parent.verticalCenter; color: "#5fd08a"
            x: parent.width * (parent.mark - parent.from) / Math.max(1, parent.to - parent.from) - 1 }
        Rectangle { width: 16; height: 16; radius: 8; anchors.verticalCenter: parent.verticalCenter; color: Theme.text; border.color: Theme.accent; border.width: 3
            x: parent.width * Math.max(0, Math.min(1, (parent.value - parent.from) / Math.max(1, parent.to - parent.from))) - 8 }
        MouseArea {
            anchors.fill: parent; cursorShape: Qt.PointingHandCursor
            function at(x) { return Math.round(parent.from + Math.max(0, Math.min(1, x / width)) * (parent.to - parent.from)); }
            onPressed: e => parent.moved(at(e.x))
            onPositionChanged: e => { if (pressed) parent.moved(at(e.x)); }
        }
    }
    component Pill: Rectangle {
        property string text
        property color tone: Theme.line
        width: pt.implicitWidth + 22; height: 28; radius: 14
        color: "transparent"; border.color: tone
        Text { id: pt; anchors.centerIn: parent; text: parent.text; color: parent.tone === Theme.line ? Theme.text : parent.tone; font.family: Theme.font; font.pixelSize: 12 }
    }
    component Button: Rectangle {
        property string text
        property bool primary: false
        property bool good: false
        signal clicked()
        width: bl.implicitWidth + 28; height: 34; radius: Theme.r(8)
        color: primary ? Theme.accent : bm.containsMouse ? Theme.panel3 : Theme.panel2
        border.color: good ? "#5fd08a" : Theme.line
        opacity: ms.busy ? 0.6 : 1
        Text { id: bl; anchors.centerIn: parent; text: parent.text; color: parent.primary ? Theme.accentInk : parent.good ? "#5fd08a" : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
        MouseArea { id: bm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: !ms.busy; onClicked: parent.clicked() }
    }

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 14

        Text { visible: !ms.st; text: "Reading memory settings…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }

        // the state, at a glance
        Flow {
            visible: !!ms.st
            Layout.fillWidth: true
            spacing: 8
            Pill { text: "Pressure: " + (ms.st ? ms.st.pressure.word : ""); tone: ms.st && ms.st.pressure.word !== "none" && ms.st.pressure.word !== "light" ? Theme.ember : "#5fd08a" }
            Pill { visible: !!(ms.st && ms.st.zram && ms.st.zram.active); text: ms.st && ms.st.zram ? "Zram " + ms.gb(ms.st.zram.used) + " / " + ms.gb(ms.st.zram.disksize) + " GB" + (ms.st.zram.ratio ? " · " + ms.st.zram.ratio + ":1" : "") : "" }
            Pill { visible: !!(ms.st && ms.st.swapfile.exists); text: { if (!ms.st) return ""; const s = ms.st.swaps.find(x => x.path === ms.st.swapfile.path); return "Drive swap " + ms.gb(s ? s.used : 0) + " / " + ms.st.swapfile.gb + " GB"; } }
        }

        // your test files: handed over only when you press it
        Rectangle {
            visible: !!(ms.st && Object.keys(ms.st.testFiles || {}).length)
            Layout.fillWidth: true
            implicitHeight: tf.implicitHeight + 24
            radius: Theme.r(8); color: Theme.panel2; border.color: Theme.flame
            RowLayout {
                id: tf
                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 14 }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 12
                    text: "Your test settings are in use (" + (ms.st ? Object.values(ms.st.testFiles).join(", ") : "") + "). Hearth shows their values here and leaves them as they are. Use Hearth's settings to hand them over: the same values, kept by Hearth, with the files kept as .bak."
                }
                Button { text: "Use Hearth's settings"; onClicked: ms.run(["adopt"]) }
            }
        }

        GridLayout {
            visible: !!ms.st
            Layout.fillWidth: true
            columns: ms.width > 900 ? 2 : 1
            columnSpacing: 14; rowSpacing: 14

            Card {
                title: "Compressed memory (zram)"
                sub: "Squeezes idle memory inside RAM instead of writing it to the drive. Fast, and it uses nothing until it's needed."
                Line {
                    title: "On"
                    Toggle { checked: ms.val("zram", "on", true); onFlipped: ms.put("zram", "on", !checked) }
                }
                Line {
                    title: "Size"; sub: "how much overflow it can hold (up to twice your memory; the green mark is recommended)"
                    Slider {
                        Layout.preferredWidth: 220
                        from: 1; to: ms.st ? ms.st.limits.zramMaxGb : 16; mark: ms.st ? ms.st.limits.zramMarkGb : -1
                        value: ms.val("zram", "gb", 8); onMoved: v => ms.put("zram", "gb", v)
                    }
                    Text { text: ms.val("zram", "gb", 8) + " GB"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; Layout.preferredWidth: 56; horizontalAlignment: Text.AlignRight }
                }
                Line {
                    title: "Compression"; sub: "zstd squeezes most · lz4 is quickest (only what this kernel offers)"
                    Chips { options: ms.st ? ms.st.algos.slice(0, 4).map(a => [a, a]) : []; value: ms.val("zram", "algo", "zstd"); onChose: v => ms.put("zram", "algo", v) }
                }
                Rectangle {
                    Layout.fillWidth: true; implicitHeight: zt.implicitHeight + 18; radius: Theme.r(6); color: Theme.panel2
                    Text {
                        id: zt; anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 }
                        wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; textFormat: Text.StyledText
                        readonly property real ratio: ms.st && ms.st.zram && ms.st.zram.ratio ? ms.st.zram.ratio : 3
                        text: "When full, holds <b>" + ms.val("zram", "gb", 8) + " GB</b> in about <b>" + Math.max(1, Math.round(ms.val("zram", "gb", 8) / ratio)) + " GB</b> of real memory (" + (ms.st && ms.st.zram && ms.st.zram.ratio ? "your usual " : "about ") + ratio + ":1)."
                            + (ms.st && ms.val("zram", "gb", 8) === ms.st.recommended.zram.gb ? "  <font color='#5fd08a'>recommended for " + Math.round(ms.st.memory.total / ms.gib) + " GB</font>" : "")
                    }
                }
            }

            Card {
                title: "Keep memory free"
                sub: "How much the system keeps empty, ready for the next heavy moment. It moves idle memory into zram to keep it."
                Line {
                    title: "Target"
                    Chips {
                        options: [["off", "Off"], ["2gb", "2 GB"], ["25%", "25%"], ["50%", "50%"], ["custom", "Custom"]]; value: ms.keepMode
                        onChose: v => {
                            if (v !== "custom") { ms.putKeep(v); return; }
                            const d = JSON.parse(JSON.stringify(ms.draft));
                            d.keep = { mode: "custom", factor: ms.cur && ms.cur.keep && ms.cur.keep.factor ? ms.cur.keep.factor : 1000 };
                            ms.draft = d;
                        }
                    }
                }
                // Custom (1.12, mockup task-manager-swap-settings): how much to keep free, as a share of memory
                Line {
                    visible: ms.keepMode === "custom"
                    title: "Keep free"; sub: "a share of your memory"
                    readonly property int factor: ms.draft.keep && ms.draft.keep.factor ? ms.draft.keep.factor : ms.cur && ms.cur.keep && ms.cur.keep.factor ? ms.cur.keep.factor : 1000
                    Slider { Layout.preferredWidth: 200; from: 1; to: 60; mark: -1; value: Math.round(parent.factor / 50); onMoved: v => { const d = JSON.parse(JSON.stringify(ms.draft)); d.keep = { mode: "custom", factor: Math.max(10, Math.min(3000, v * 50)) }; ms.draft = d; } }
                    Text { text: Math.round(parent.factor / 50) + "%"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; Layout.preferredWidth: 40; horizontalAlignment: Text.AlignRight }
                }
                Line {
                    visible: !!(ms.st && ms.st.watermarks && ms.st.watermarks.low)
                    title: "Starts freeing below"; sub: "then frees up to the target"
                    Text {
                        text: ms.st && ms.st.watermarks ? Math.max(1, Math.round(100 * ms.st.watermarks.low / Math.max(1, ms.st.memory.total))) + "% free" : ""
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold
                    }
                }
                Line {
                    title: "Swappiness"; sub: "how readily idle memory is compressed instead of dropping cached files"
                    Slider { Layout.preferredWidth: 200; from: 0; to: 200; mark: 200; value: ms.swappiness; onMoved: v => { const d = JSON.parse(JSON.stringify(ms.draft)); d.swappiness = v; ms.draft = d; } }
                    Text { text: String(ms.swappiness); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; Layout.preferredWidth: 40; horizontalAlignment: Text.AlignRight }
                }
                Text {
                    Layout.fillWidth: true; visible: !!(ms.st && Object.keys(ms.st.setBy || {}).length); wrapMode: Text.WordWrap
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                    text: ms.st ? "Set by your own files (yours win): " + Object.keys(ms.st.setBy).map(k => "vm." + k + " in " + ms.st.setBy[k]).join(", ") : ""
                }
                Rectangle {
                    visible: ms.keepMode === "50%" || ms.keepMode === "25%"
                    Layout.fillWidth: true; implicitHeight: kw.implicitHeight + 18; radius: Theme.r(6)
                    color: "transparent"; border.color: Theme.ember
                    Text { id: kw; anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 }
                        wrapMode: Text.WordWrap; color: Theme.ember; font.family: Theme.font; font.pixelSize: 12
                        text: ms.keepMode + " works your memory harder: more compressing, a little more battery. Fine for browsing and AI work; if a game stutters, try 2 GB." }
                }
            }

            Card {
                title: "Drive swap file"
                sub: "The slow overflow on your drive (C:, /var/swap). Used only when zram is full, and for hibernation."
                Line {
                    title: "Use for everyday overflow"; sub: "off = only for hibernation"
                    Toggle { checked: ms.val("swapfile", "everyday", true); onFlipped: ms.put("swapfile", "everyday", !checked) }
                }
                Line {
                    title: "Size"; sub: "on C: (/var/swap) · hibernation needs at least your memory size · C: always keeps 20 GB free · at most " + (ms.st ? ms.st.limits.swapfileMaxGb : 0) + " GB here"
                    Chips {
                        options: {
                            if (!ms.st) return [];
                            const max = ms.st.limits.swapfileMaxGb, mem = Math.round(ms.st.memory.total / ms.gib);
                            const sizes = [0, 4, 8, 16, 32, mem, 64, ms.st.swapfile.gb].filter((v, i, a) => v <= Math.max(max, ms.st.swapfile.gb) && a.indexOf(v) === i).sort((a, b) => a - b);
                            return sizes.map(v => [String(v), v === 0 ? "Off" : v + " GB"]);
                        }
                        value: String(ms.swapGb)
                        onChose: v => { const d = JSON.parse(JSON.stringify(ms.draft)); d.swapGb = parseInt(v); ms.draft = d; }
                    }
                }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                    text: ms.st && ms.st.swapfile.hibernate ? "Hibernation has its own space (/var/swap/hearth-hibernate), left as it is." : "Changing the size makes the file again when you press Save (it can take a minute)."
                }
                Line {
                    title: "Hibernation"
                    sub: !ms.st ? "" : ms.st.hibernateChecked ? "works on this laptop · last checked " + new Date(ms.st.hibernateChecked * 1000).toLocaleDateString(Qt.locale(), "MMM d")
                        : ms.st.swapfile.hibernate ? "set up, not tested on this PC yet" : "not set up"
                    Text { visible: !!(ms.st && ms.st.hibernateChecked); text: "ready"; color: "#5fd08a"; font.family: Theme.font; font.pixelSize: 12 }
                }
            }

            Card {
                title: "When memory runs out"; badge: "NEW"
                sub: "Instead of the whole PC freezing, Hearth closes the one program eating memory. It tells you which, and you can open it again."
                Line {
                    title: "Protection"
                    Chips { options: [["off", "Off"], ["gentle", "Gentle"], ["strong", "Strong"]]; value: ms.val("guard", "level", "gentle"); onChose: v => ms.put("guard", "level", v) }
                }
                Line {
                    title: "Never close"; sub: "always kept running (press one to take it off the list)"
                    Flow {
                        Layout.preferredWidth: 300
                        spacing: 6
                        Repeater {
                            model: ms.st ? Object.keys(ms.st.neverLabels) : []
                            delegate: Rectangle {
                                required property string modelData
                                readonly property bool on: ms.val("guard", "never", []).indexOf(modelData) >= 0
                                width: nt.implicitWidth + 20; height: 28; radius: Theme.r(6)
                                color: "transparent"; border.color: on ? "#5fd08a" : Theme.line
                                Text { id: nt; anchors.centerIn: parent; text: ms.st.neverLabels[modelData]; color: parent.on ? "#5fd08a" : Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                MouseArea {
                                    anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                                    onClicked: { const l = ms.val("guard", "never", []).slice(); const i = l.indexOf(modelData); if (i >= 0) l.splice(i, 1); else l.push(modelData); ms.put("guard", "never", l); }
                                }
                            }
                        }
                        Repeater {
                            model: ms.val("guard", "extra", [])
                            delegate: Rectangle {
                                required property string modelData
                                width: xt.implicitWidth + 20; height: 28; radius: Theme.r(6)
                                color: "transparent"; border.color: "#5fd08a"
                                Text { id: xt; anchors.centerIn: parent; text: parent.modelData + "  ✕"; color: "#5fd08a"; font.family: Theme.font; font.pixelSize: 12 }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ms.put("guard", "extra", ms.val("guard", "extra", []).filter(x => x !== parent.modelData)) }
                            }
                        }
                        // + Add (1.12, mockup task-manager-swap-settings): any program, by its name
                        Rectangle {
                            id: addNever
                            property bool typing: false
                            width: addNever.typing ? 170 : at.implicitWidth + 20; height: 28; radius: Theme.r(6)
                            color: "transparent"; border.color: Theme.line; border.width: 1
                            Text { id: at; visible: !addNever.typing; anchors.centerIn: parent; text: "+ Add"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                            MouseArea { anchors.fill: parent; enabled: !addNever.typing; cursorShape: Qt.PointingHandCursor; onClicked: { addNever.typing = true; neverIn.forceActiveFocus(); } }
                            TextInput {
                                id: neverIn
                                visible: addNever.typing
                                anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                                verticalAlignment: TextInput.AlignVCenter
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 12
                                Text { anchors.verticalCenter: parent.verticalCenter; visible: parent.text === ""; text: "program name, Enter"; color: Theme.muted; font: parent.font }
                                onAccepted: {
                                    const n = text.trim();
                                    if (/^[\w.+-]{1,64}$/.test(n) && ms.val("guard", "extra", []).indexOf(n) < 0) ms.put("guard", "extra", ms.val("guard", "extra", []).concat([n]));
                                    text = ""; addNever.typing = false;
                                }
                                Keys.onEscapePressed: { text = ""; addNever.typing = false; }
                            }
                        }
                    }
                }
                // Keep this running (checkpoint), 1.12: what Task Manager's right-click put on the list; press one to take it off
                Line {
                    visible: ms.val("guard", "checkpoint", []).length > 0 || !!(ms.cur && ms.cur.guard && (ms.cur.guard.checkpoint || []).length)
                    title: "Pause instead of closing"; sub: "frozen, its memory moved to the drive; Resume in Task Manager"
                    Flow {
                        Layout.preferredWidth: 300
                        spacing: 6
                        Repeater {
                            model: ms.val("guard", "checkpoint", [])
                            delegate: Rectangle {
                                required property string modelData
                                width: kt.implicitWidth + 20; height: 28; radius: Theme.r(6)
                                color: "transparent"; border.color: Theme.accent
                                Text { id: kt; anchors.centerIn: parent; text: parent.modelData + "  ✕"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 12 }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ms.put("guard", "checkpoint", ms.val("guard", "checkpoint", []).filter(x => x !== parent.modelData)) }
                            }
                        }
                    }
                }
                Line {
                    title: "Close first"; sub: "the program using the most memory and swap"
                    Text { text: "automatic"; color: "#5fd08a"; font.family: Theme.font; font.pixelSize: 12 }
                }
                Text {
                    Layout.fillWidth: true; visible: !!(ms.st && ms.st.lastClosed.length); wrapMode: Text.WordWrap
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                    text: ms.st && ms.st.lastClosed.length ? "Last closed: " + ms.st.lastClosed.slice(-3).reverse().map(c => c.name + " (" + (c.paused ? "paused, " : "") + (c.mb / 1024).toFixed(1) + " GB, " + new Date(c.at * 1000).toLocaleString(Qt.locale(), "MMM d h:mm") + ")").join(", ") : ""
                }
            }
        }

        // save
        Rectangle {
            visible: !!ms.st
            Layout.fillWidth: true
            implicitHeight: 60
            radius: Theme.r(10); color: Theme.panel; border.color: Theme.line
            RowLayout {
                anchors { fill: parent; leftMargin: 18; rightMargin: 12 }
                spacing: 10
                Text {
                    Layout.fillWidth: true; elide: Text.ElideRight
                    text: ms.note !== "" ? ms.note : ms.dirty ? "Not saved yet." : "Changes are saved and kept until you change them. Nothing here changes by itself."
                    color: ms.noteBad ? Theme.danger : ms.dirty ? Theme.flame : Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
                Button { text: "Back to recommended"; onClicked: { ms.draft = ({}); ms.run(["recommended"]); } }
                Button { text: "Try until restart"; good: true; visible: ms.dirty; onClicked: ms.save(true) }
                Button { text: "Save"; primary: true; onClicked: ms.save(false) }
            }
        }
    }
}
