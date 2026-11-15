// Server mode › Quiet hours & rules, opened (A on its tile; 1.13, mockup docs/mockups/next-update/server-rules.png):
// Quiet hours (each with its times, days and switch; what happens while quiet; games you play always at full speed),
// Now (quiet until when, what waits, the fans), When this, do that (each rule "When … → …" with its switch, Paused on
// a paused one, + New rule) and What a rule can watch and do. Nothing here is on until you make it.
// The controller: Up / Down pick a quiet time or a rule, A turns it on or off, X edits it (its numbers: Left / Right),
// Y pauses the rule for an hour (again: carries on), ☰ makes a new rule (pick a When, then a Do), B goes back.
// Draw-only: ServerDash runs the actions (hearth-server-mode rules …).
import QtQuick
import QtQuick.Layouts

Item {
    id: det
    property var t: null
    property real k: 1
    property int sel: 0
    // The new-rule picker: step 0 off, 1 When, 2 Do; its choices and the one picked in each.
    property int step: 0
    property int wi: 0
    property int di: 0
    property var whenVals: ({ drive: 50, power: 300, from: 23 * 60, to: 7 * 60 })
    property int fanPct: 40
    property string device: ""
    signal action(var args)
    signal close()
    // The two columns' widths come from the page's width: sized by ratio they kept the layouts busy forever
    readonly property real colsW: Math.max(0, width - 84 * k)
    readonly property var st: t && t.rules ? t.rules : ({})
    readonly property var quiet: st.quiet || []
    readonly property var rules: st.rules || []
    readonly property var now: st.now || ({})
    readonly property var w: st.while || ({ fans: 40, powerW: 150, jobs: true, downloads: true })
    readonly property int rows: quiet.length + rules.length + 1
    readonly property bool onNew: sel === rows - 1
    function hhmm(m) { m = ((m % 1440) + 1440) % 1440; return ("0" + Math.floor(m / 60)).slice(-2) + ":" + ("0" + (m % 60)).slice(-2); }
    function clock(ts) { const d = new Date(ts * 1000); return Qt.formatTime(d, "h:mm AP"); }
    readonly property var whens: [
        { id: "jellyfin", text: "someone is watching Jellyfin" }, { id: "game", text: "a game runs on this PC" },
        { id: "device", text: "a device is connected" + (device ? " (" + device + ")" : "") },
        { id: "time", text: "it's " + hhmm(whenVals.from) + " – " + hhmm(whenVals.to) },
        { id: "drive", text: "the drive is over " + whenVals.drive + " °C" }, { id: "power", text: "power use is over " + whenVals.power + " W" },
        { id: "quiet", text: "Quiet hours: " + hhmm(whenVals.from) + " – " + hhmm(whenVals.to) + " · Mon–Fri" }]
    readonly property var dos: [
        { id: "pictures", text: "AI pictures wait" }, { id: "voices", text: "voices wait" }, { id: "downloads", text: "downloads at half speed" },
        { id: "backups", text: "backups wait" }, { id: "fans", text: "fans capped at " + fanPct + " %" }, { id: "phone", text: "tell my phone" }]
    function whenArg(x) {
        if (x.id === "device") return "device:" + (device || "phone");
        if (x.id === "time") return "time:" + hhmm(whenVals.from) + "-" + hhmm(whenVals.to);
        if (x.id === "drive") return "drive:" + whenVals.drive;
        if (x.id === "power") return "power:" + whenVals.power;
        return x.id;
    }
    function doArg(x) { return x.id === "fans" ? "fans:" + fanPct : x.id; }
    function pick(d) {
        if (step === 1) wi = Math.max(0, Math.min(whens.length - 1, wi + d));
        else if (step === 2) di = Math.max(0, Math.min(dos.length - 1, di + d));
        else sel = Math.max(0, Math.min(rows - 1, sel + d));
    }
    // Left / Right: the picked choice's number (times by half an hour, heat by 5 °C, power by 50 W, fans by 10 %).
    function adjust(d) {
        if (step === 1) {
            const x = whens[wi].id, v = Object.assign({}, whenVals);
            if (x === "drive") v.drive = Math.max(30, Math.min(90, v.drive + 5 * d));
            else if (x === "power") v.power = Math.max(50, Math.min(1500, v.power + 50 * d));
            else if (x === "time" || x === "quiet") v.from = (v.from + 30 * d + 1440) % 1440;
            whenVals = v;
        } else if (step === 2 && dos[di].id === "fans") fanPct = Math.max(20, Math.min(100, fanPct + 10 * d));
    }
    function enter() {
        if (step === 1) { if (whens[wi].id === "quiet") { action(["quiet-add", "Quiet hours", hhmm(whenVals.from), hhmm(whenVals.to), "mon-fri"]); step = 0; } else step = 2; return; }
        if (step === 2) { action(["rule-add", whenArg(whens[wi]), doArg(dos[di])]); step = 0; sel = rows - 1; return; }
        if (onNew) { newRule(); return; }
        if (sel < quiet.length) { const q = quiet[sel]; action(["quiet-set", q.id, q.on ? "off" : "on"]); }
        else { const r = rules[sel - quiet.length]; action(["rule-set", r.id, r.on ? "off" : "on"]); }
    }
    function newRule() { step = 1; wi = 0; di = 0; const d = (t && t.devices) || []; device = d.length ? String(d[0]) : ""; }
    function pauseRule() {
        if (step || onNew || sel < quiet.length) return;
        const r = rules[sel - quiet.length];
        action(["rule-pause", r.id, r.paused ? "0" : "3600"]);
    }
    // X: edit = remove and pick it again (its When and Do picked as they were).
    function edit() {
        if (step || onNew) return;
        if (sel < quiet.length) { action(["quiet-remove", quiet[sel].id]); newRule(); wi = whens.length - 1; return; }
        const r = rules[sel - quiet.length];
        const wk = r.when.split(":")[0], dk = r["do"].split(":")[0];
        action(["rule-remove", r.id]);
        newRule();
        wi = Math.max(0, whens.findIndex(x => x.id === wk));
        di = Math.max(0, dos.findIndex(x => x.id === dk));
    }
    function back() { if (step) { step = step - 1; return true; } return false; }
    onVisibleChanged: { sel = 0; step = 0; }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32 * det.k
        spacing: 20 * det.k
        ServerViewHead {
            k: det.k; icon: "moon"; title: "Quiet hours & rules"
            sub: "Your times and your rules for this server · nothing here is on until you make it"
            chip: det.now.quiet ? "Quiet now" : (det.quiet.some(q => q.on) || det.rules.some(r => r.on)) ? "On" : "Off"
            chipColor: det.now.quiet || det.quiet.some(q => q.on) || det.rules.some(r => r.on) ? "#6fe39a" : "#c9b79f"
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 20 * det.k
            ColumnLayout {
                Layout.fillWidth: false; Layout.preferredWidth: Math.floor(det.colsW * 1/2); Layout.minimumWidth: Math.floor(det.colsW * 1/2); Layout.maximumWidth: Math.floor(det.colsW * 1/2); Layout.alignment: Qt.AlignTop; Layout.fillHeight: false
                spacing: 20 * det.k
                ServerPanel {
                    k: det.k; title: "Quiet hours"
                    Repeater {
                        model: det.quiet
                        delegate: ServerSwitch {
                            required property var modelData
                            required property int index
                            k: det.k; richLabel: true
                            label: "<b>" + modelData.name + "</b>&nbsp;&nbsp;&nbsp;" + modelData.from + " – " + modelData.to + " · " + modelData.daysText
                            on: !!modelData.on
                            picked: det.sel === index && !det.step
                            onToggled: { det.sel = index; det.enter(); }
                        }
                    }
                    Text { visible: !det.quiet.length; text: "None yet: ☰ New rule › Quiet hours makes one."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 16 * det.k }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 8 * det.k
                        Text { text: "While quiet:"; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 18 * det.k }
                        Repeater {
                            model: ["Fans ≤ " + det.w.fans + " %", "Power ≤ " + det.w.powerW + " W"].concat(det.w.jobs ? ["Pictures and dubs wait"] : []).concat(det.w.downloads ? ["Downloads at half speed"] : [])
                            delegate: Rectangle {
                                required property string modelData
                                implicitWidth: ct.implicitWidth + 18 * det.k; implicitHeight: 28 * det.k; radius: 6 * det.k; color: Qt.rgba(0.43, 0.89, 0.6, 0.14)
                                Text { id: ct; anchors.centerIn: parent; text: modelData; color: "#6fe39a"; font.family: Theme.font; font.pixelSize: 14 * det.k }
                            }
                        }
                    }
                    Text { text: "Games you play and things you start yourself always run at full speed."; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 15 * det.k }
                }
                ServerPanel {
                    k: det.k; title: "Now"
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.StyledText
                        color: "#fff4df"; font.family: Theme.font; font.pixelSize: 20 * det.k
                        text: (det.now.quiet ? "Quiet until <b>" + det.clock(det.now.quietUntil) + "</b>" : det.now.game ? "A game is running: everything at full speed" : "Not quiet")
                              + ((det.now.wait || []).length ? " · waiting: " + det.now.wait.join(", ") : "")
                              + (det.now.fanSpeed !== undefined && det.now.fanSpeed !== null ? " · fans " + det.now.fanSpeed + " %" : "")
                    }
                }
            }
            ColumnLayout {
                Layout.fillWidth: false; Layout.preferredWidth: Math.floor(det.colsW * 1/2); Layout.minimumWidth: Math.floor(det.colsW * 1/2); Layout.maximumWidth: Math.floor(det.colsW * 1/2); Layout.alignment: Qt.AlignTop; Layout.fillHeight: false
                spacing: 20 * det.k
                ServerPanel {
                    k: det.k; title: det.step === 1 ? "New rule · pick a When" : det.step === 2 ? "New rule · then a Do" : "When this, do that"
                    Repeater {
                        model: det.step ? [] : det.rules
                        delegate: ServerSwitch {
                            required property var modelData
                            required property int index
                            k: det.k; richLabel: true
                            label: "When&nbsp;&nbsp;<b>" + modelData.whenText + "</b>&nbsp;&nbsp;→&nbsp;&nbsp;<b>" + modelData.doText + "</b>"
                            chip: modelData.paused ? "Paused" : ""
                            chipKind: "warn"
                            on: !!modelData.on
                            picked: det.sel === det.quiet.length + index
                            onToggled: { det.sel = det.quiet.length + index; det.enter(); }
                        }
                    }
                    Rectangle {   // + New rule
                        visible: !det.step
                        Layout.fillWidth: true; implicitHeight: 46 * det.k; radius: 10 * det.k; color: "transparent"
                        border.width: det.onNew ? 2.5 * det.k : 0; border.color: Theme.accent
                        Text { anchors { left: parent.left; leftMargin: det.onNew ? 12 * det.k : 0; verticalCenter: parent.verticalCenter }
                            text: "+  New rule: pick a When, then a Do"; color: "#ff8c42"; font.family: Theme.font; font.pixelSize: 18 * det.k }
                        MouseArea { anchors.fill: parent; onClicked: det.newRule() }
                    }
                    Repeater {   // the picker
                        model: det.step === 1 ? det.whens : det.step === 2 ? det.dos : []
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            readonly property bool on: (det.step === 1 ? det.wi : det.di) === index
                            Layout.fillWidth: true; implicitHeight: 44 * det.k; radius: 10 * det.k
                            color: on ? Qt.rgba(1, 0.55, 0.26, 0.14) : "transparent"; border.width: on ? 2 * det.k : 0; border.color: Theme.accent
                            Text { anchors { left: parent.left; leftMargin: 12 * det.k; verticalCenter: parent.verticalCenter }
                                text: (det.step === 1 && index < det.whens.length - 1 ? "When  " : det.step === 2 ? "→  " : "") + modelData.text
                                color: "#fff4df"; font.family: Theme.font; font.pixelSize: 18 * det.k }
                            MouseArea { anchors.fill: parent; onClicked: { if (det.step === 1) det.wi = index; else det.di = index; det.enter(); } }
                        }
                    }
                    Text { visible: det.step > 0; text: "A picks · Left / Right change its number · B goes back"; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 14 * det.k }
                }
                ServerPanel {
                    k: det.k; title: "What a rule can watch and do"
                    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 15 * det.k
                        text: "When: Jellyfin watching · a game running · a device connected · a time · a drive's heat · power use.\nDo: AI pictures wait · voices wait · downloads slower · backups wait · fans capped · tell my phone." }
                }
            }
        }
        ServerKeys {
            k: det.k; back: "Back"
            keys: [["A", "Turn on or off", "enter"], ["X", "Edit", "edit"], ["Y", "Pause this rule", "pause"], ["☰", "New rule", "new"]]
            onPressed: x => x === "enter" ? det.enter() : x === "edit" ? det.edit() : x === "pause" ? det.pauseRule() : det.newRule()
            onBackPressed: { if (!det.back()) det.close(); }
        }
    }
}
