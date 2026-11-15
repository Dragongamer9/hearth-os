// Server mode › Ithil, opened (A on her tile; 1.13, mockup next-update/server-ithil-detail.png): her live 3D / 4D brain
// (IthilBrain.qml) with Live, Last 24 hours and Her whole life and the timeline (the 4th dimension), beside Right now,
// Today, Her brain on this PC and Reach her from. A talks to her (her app), X restarts her app, Y puts her to sleep (asks
// twice), ☰ her settings (her app), B goes back; LB / RB (or ◀ ▶) move through time, ▲ ▼ change the window, View X-ray.
// Draw-only: ServerDash runs the buttons (hearth-server-mode ithil-*).
import QtQuick
import QtQuick.Layouts

Item {
    id: det
    property var it: null                 // hearth_ithil.view()
    property var brain: null              // hearth_ithil.brain(hours)
    property real k: 1
    property string win: "live"           // live | 24 | all
    property real tpos: 120
    property bool xray: false
    property bool sleepArmed: false
    signal action(string a)
    signal close()
    signal want(string win)
    readonly property color cText: "#fff4df"
    readonly property color cMuted: "#c9a372"
    readonly property color cLine: "#4a3212"
    readonly property color cCard: "#140c03"
    readonly property color cOk: "#6fe39a"
    readonly property color cAcc: "#ffb347"
    readonly property color cBad: "#f0a8a8"
    readonly property var i: it || ({})
    readonly property var today: i.today || ({})

    function step(d) { if (win === "live") { win = "24"; want("24"); tpos = 118; } tpos = Math.max(0, Math.min(120, tpos + d)); }
    function cycle(d) { const w = ["live", "24", "all"]; win = w[(w.indexOf(win) + d + 3) % 3]; tpos = 120; want(win); }

    // Not live: the window plays through slowly by itself (120 s), unless you just moved it.
    property double moved: 0
    Timer { interval: 100; repeat: true; running: det.visible && det.win !== "live"
        onTriggered: { if (Date.now() - det.moved > 4000) det.tpos = det.tpos >= 120 ? 0 : det.tpos + 0.1; } }
    onTposChanged: strip.requestPaint()
    onBrainChanged: strip.requestPaint()

    component Panel: Rectangle {
        default property alias body: inner.data
        property string title: ""
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 40 * det.k
        radius: 14 * det.k; color: det.cCard; border.color: det.cLine
        ColumnLayout {
            id: inner
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 * det.k }
            spacing: 8 * det.k
            Text { text: parent.parent.title; color: det.cMuted; font.family: Theme.font; font.pixelSize: 15 * det.k; font.weight: Font.DemiBold; font.letterSpacing: 1; font.capitalization: Font.AllUppercase }
        }
    }
    component Chip: Rectangle {
        property string t: ""
        property color c: det.cOk
        implicitWidth: ct.implicitWidth + 18 * det.k; implicitHeight: 26 * det.k; radius: 7 * det.k
        color: Qt.rgba(c.r, c.g, c.b, 0.16)
        Text { id: ct; anchors.centerIn: parent; text: parent.t; color: parent.c; font.family: Theme.font; font.pixelSize: 13 * det.k; font.weight: Font.DemiBold }
    }
    component Gauge: RowLayout {
        property string a: ""
        property real v: 0
        Layout.fillWidth: true; spacing: 12 * det.k
        Text { text: parent.a; color: det.cText; font.family: Theme.font; font.pixelSize: 15 * det.k; Layout.preferredWidth: 100 * det.k }
        Rectangle { Layout.fillWidth: true; height: 8 * det.k; radius: 4 * det.k; color: "#0c0702"
            Rectangle { width: parent.width * parent.parent.v; height: parent.height; radius: parent.radius; color: "#f472b6" } }
        Text { text: parent.v.toFixed(2).replace(/^0/, ""); color: det.cMuted; font.family: Theme.font; font.pixelSize: 15 * det.k; Layout.preferredWidth: 40 * det.k; horizontalAlignment: Text.AlignRight }
    }
    component Pill: Rectangle {
        property string t: ""
        property bool on: false
        implicitWidth: pt.implicitWidth + 26 * det.k; implicitHeight: 32 * det.k; radius: 16 * det.k
        color: on ? Qt.rgba(1, 0.7, 0.28, 0.2) : "#1c140a"; border.color: on ? det.cAcc : det.cLine
        Text { id: pt; anchors.centerIn: parent; text: parent.t; color: det.cText; font.family: Theme.font; font.pixelSize: 14 * det.k }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 30 * det.k
        spacing: 18 * det.k
        RowLayout {
            spacing: 18 * det.k
            Rectangle { implicitWidth: 56 * det.k; implicitHeight: 56 * det.k; radius: 12 * det.k; color: "#22193a"
                Icon { anchors.centerIn: parent; name: "moon"; size: 30 * det.k; color: "#c4b5fd" } }
            ColumnLayout {
                spacing: 2; Layout.fillWidth: true
                Text { text: det.i.name || "Ithil"; color: det.cText; font.family: Theme.font; font.pixelSize: 34 * det.k; font.weight: Font.DemiBold }
                Text { textFormat: Text.StyledText; color: det.cMuted; font.family: Theme.font; font.pixelSize: 16 * det.k; Layout.fillWidth: true; elide: Text.ElideRight
                    text: "Her own mind on this PC" + (det.i.version ? " · Hearth AI " + det.i.version : "") + " · reach her at <b><font color='" + det.cText + "'>" + (det.i.address || "") + "</font></b> (Tailscale)" }
            }
            Chip { t: det.i.chip || ""; c: det.i.state === "off" ? det.cMuted : det.cOk }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 18 * det.k
            // Her brain, with time.
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1.35
                radius: 14 * det.k; clip: true; border.color: det.cLine
                gradient: Gradient { GradientStop { position: 0; color: "#2a2232" } GradientStop { position: 1; color: "#120d08" } }
                IthilBrain { id: big; anchors.fill: parent; anchors.bottomMargin: 70 * det.k; brain: det.brain; live: det.win === "live"; tpos: det.tpos; xray: det.xray
                    asleep: det.i.state === "asleep" }
                Row {
                    anchors { left: parent.left; top: parent.top; margins: 16 * det.k }
                    spacing: 10 * det.k
                    Pill { t: "● Live"; on: det.win === "live" }
                    Pill { t: "Last 24 hours"; on: det.win === "24" }
                    Pill { t: "Her whole life"; on: det.win === "all" }
                    Pill { t: "X-ray"; on: det.xray; visible: det.xray }
                }
                Pill { anchors { right: parent.right; top: parent.top; margins: 16 * det.k }
                    t: (det.i.state === "asleep" || big.sleepiness(det.tpos) > 0.5 && det.win !== "live" ? "☾ Asleep" : "Awake") + (det.win === "live" ? " · live" : "") }
                Column {
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 16 * det.k }
                    spacing: 6 * det.k
                    Canvas {
                        id: strip
                        width: parent.width; height: 14 * det.k
                        onPaint: {
                            const g = getContext("2d"); g.clearRect(0, 0, width, height);
                            const p = big.prepared; if (!p) return;
                            g.fillStyle = "rgba(129,140,248,.25)";
                            for (const s of p.sleeps) g.fillRect(s.a / 120 * width, 0, Math.max(3, (s.b - s.a) / 120 * width), height);
                            for (const e of p.ev) { if (e.quiet) continue; g.fillStyle = big.parts[e.reg].hex; g.globalAlpha = e.block ? 1 : 0.75; g.fillRect(e.t / 120 * width, e.block ? 0 : height * 0.3, 3, e.block ? height : height * 0.7); }
                            g.globalAlpha = 1;
                        }
                    }
                    Rectangle { width: parent.width; height: 6 * det.k; radius: 3 * det.k; color: "#3a2a19"
                        Rectangle { width: parent.width * (det.win === "live" ? 1 : det.tpos / 120); height: parent.height; radius: parent.radius; color: det.cAcc } }
                    Text { color: det.cMuted; font.family: Theme.font; font.pixelSize: 14 * det.k
                        text: "Ⓛ Ⓡ move through time (the 4th dimension) · right stick turns her brain · Ⓥ X-ray" +
                              (det.win !== "live" && big.prepared.to ? " · " + new Date(big.prepared.from + det.tpos / 120 * (big.prepared.to - big.prepared.from)).toLocaleString(Qt.locale(), "ddd HH:mm") : "") }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1
                spacing: 18 * det.k
                Panel {
                    title: "Right now"
                    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.StyledText; color: det.cText; font.family: Theme.font; font.pixelSize: 16 * det.k
                        text: "<font color='" + det.cMuted + "'>Mood</font> " + (det.i.mood || "—") + (det.i.focus ? " &nbsp;·&nbsp; <font color='" + det.cMuted + "'>Focused on</font> " + det.i.focus : "") }
                    Gauge { a: "Energy"; v: det.i.needs ? det.i.needs.energy : 0 }
                    Gauge { a: "Social"; v: det.i.needs ? det.i.needs.social : 0 }
                    Gauge { a: "Curiosity"; v: det.i.needs ? det.i.needs.curiosity : 0 }
                    Gauge { a: "Stability"; v: det.i.needs ? det.i.needs.stability : 0 }
                }
                Panel {
                    title: "Today"
                    GridLayout {
                        columns: 2; columnSpacing: 20 * det.k; rowSpacing: 6 * det.k; Layout.fillWidth: true
                        Repeater {
                            model: [[String(det.today.talks || 0), (det.today.talks === 1 ? "talk" : "talks") + " with you"], [String(det.today.memories || 0), "memories kept"],
                                    [String(det.today.blocks || 0), "blocked by her gate"], [det.i.sleepTime || "—", "she sleeps"]]
                            delegate: Text { required property var modelData; Layout.fillWidth: true; textFormat: Text.StyledText; color: det.cText; font.family: Theme.font; font.pixelSize: 16 * det.k
                                text: "<b><font size='5'>" + modelData[0] + "</font></b> " + modelData[1] }
                        }
                    }
                }
                Panel {
                    title: "Her brain on this PC"
                    RowLayout { spacing: 10 * det.k
                        Text { text: det.i.brainModel || "—"; color: det.cText; font.family: Theme.font; font.pixelSize: 16 * det.k }
                        Chip { t: det.i.brainLoaded ? "Loaded" : "Not loaded"; c: det.i.brainLoaded ? det.cOk : det.cMuted } }
                    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; color: det.cMuted; font.family: Theme.font; font.pixelSize: 14 * det.k
                        text: "Stays loaded in Server mode. While you play, it steps back and she rests; she's back when you stop. Pausing the AI team asks first, because her brain is in it." }
                }
                Panel {
                    title: "Reach her from"
                    Repeater {
                        model: det.i.reach || []
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Text { text: modelData.name + (modelData.via && modelData.via !== "Tailscale" ? " (" + modelData.via + ")" : ""); color: det.cText; font.family: Theme.font; font.pixelSize: 16 * det.k; Layout.fillWidth: true; elide: Text.ElideRight }
                            Chip { t: modelData.chip; c: modelData.never ? det.cBad : modelData.ok ? det.cOk : det.cMuted }
                        }
                    }
                }
                Item { Layout.fillHeight: true }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 30 * det.k
            Repeater {
                model: [["A", "Talk to her"], ["X", "Restart her app"], ["Y", det.sleepArmed ? "Press Y again to put her to sleep" : "Put her to sleep (asks twice)"], ["☰", "Her settings"]]
                delegate: Text { required property var modelData; color: det.cText; font.family: Theme.font; font.pixelSize: 16 * det.k
                    text: ({ A: "Ⓐ ", X: "Ⓧ ", Y: "Ⓨ ", "☰": "☰ " })[modelData[0]] + modelData[1]
                    MouseArea { anchors.fill: parent; onClicked: det.action(modelData[0]) } }
            }
            Item { Layout.fillWidth: true }
            Text { text: "Ⓑ Back to the dashboard"; color: det.cText; font.family: Theme.font; font.pixelSize: 16 * det.k
                MouseArea { anchors.fill: parent; onClicked: det.close() } }
        }
    }
}
