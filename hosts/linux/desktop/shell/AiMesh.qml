// Server mode › AI team (1.12, mockups next-update/server-ai-team-dash.png and server-ai-team-detail.png): a live mesh.
// This PC's AI engines sit in a capsule in the middle (AI chat, Pictures, Voices, Speech to text, each with what it's doing);
// every device or Command Center AI that sent them work today sits around them with today's count and what it last did.
// A sender that just sent something is lit (white for chat, pink for pictures and voices) and a pulse runs down its line
// to the engines. Draw-only: everything comes from `hearth-server-mode dash` (team). The tile draws it small (compact).
import QtQuick

Item {
    id: mesh
    property var team: null
    property real k: 1
    property bool compact: false
    readonly property var engines: team && team.engines ? team.engines : []
    readonly property var senders: team && team.senders ? team.senders : []
    readonly property real s: compact ? 0.42 : 1
    readonly property color cText: "#fff4df"
    readonly property color cMuted: "#c9a372"
    readonly property color cDim: "#6b5a44"
    readonly property color cLine: "#4a3212"
    readonly property color cChat: "#ffffff"
    readonly property color cPink: "#ff5fae"
    readonly property color cTeal: "#4fe3c1"
    function colorOf(x) { return !x.active ? cDim : (x.kind === "pictures" || x.kind === "voice") ? cPink : cChat; }
    // The places around the capsule, clockwise from the top: like the mockup (laptop on top, the Ally top right…).
    readonly property var slots: [[0, -1], [0.62, -0.66], [0.86, 0], [0.62, 0.66], [0, 1], [-0.62, 0.66], [-0.86, 0], [-0.62, -0.66], [0.32, -0.95], [-0.32, 0.95]]
    function posOf(i) {
        const sl = slots[i % slots.length];
        return Qt.point(width / 2 + sl[0] * width * 0.42, height / 2 + sl[1] * height * 0.40);
    }

    // A faint grid behind it all, as in the mockup.
    Canvas {
        id: grid
        anchors.fill: parent
        visible: !mesh.compact
        onPaint: {
            const c = getContext("2d");
            c.reset();
            c.strokeStyle = "rgba(201,163,114,0.06)";
            c.lineWidth = 1;
            const g = 40 * mesh.k;
            for (let x = 0; x < width; x += g) { c.beginPath(); c.moveTo(x, 0); c.lineTo(x, height); c.stroke(); }
            for (let y = 0; y < height; y += g) { c.beginPath(); c.moveTo(0, y); c.lineTo(width, y); c.stroke(); }
        }
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
    }
    // A warm glow under the engines.
    Rectangle {
        anchors.centerIn: parent
        width: capsule.width * 1.6; height: capsule.height * 3.2; radius: height / 2
        gradient: Gradient {
            GradientStop { position: 0; color: "transparent" }
            GradientStop { position: 0.5; color: Qt.rgba(1, 0.55, 0.2, mesh.compact ? 0.10 : 0.07) }
            GradientStop { position: 1; color: "transparent" }
        }
    }
    // The lines from each sender to the capsule.
    Canvas {
        id: lines
        anchors.fill: parent
        property var snap: mesh.senders
        onSnapChanged: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const c = getContext("2d");
            c.reset();
            for (let i = 0; i < mesh.senders.length; i++) {
                const x = mesh.senders[i], p = mesh.posOf(i);
                const col = mesh.colorOf(x);
                c.strokeStyle = x.active ? Qt.rgba(col.r, col.g, col.b, 0.85) : "rgba(201,163,114,0.18)";
                c.lineWidth = (x.active ? 2 : 1) * Math.max(0.6, mesh.k * mesh.s * 1.4);
                c.beginPath(); c.moveTo(p.x, p.y); c.lineTo(width / 2, height / 2); c.stroke();
            }
        }
    }
    // Pulses: a dot running down each lit sender's line to the engines.
    Repeater {
        model: mesh.senders
        delegate: Rectangle {
            required property var modelData
            required property int index
            readonly property point from: mesh.posOf(index)
            property real t: 0
            visible: modelData.active
            width: 8 * mesh.k * Math.max(0.6, mesh.s * 1.4); height: width; radius: width / 2
            color: mesh.colorOf(modelData)
            x: from.x + (mesh.width / 2 - from.x) * t - width / 2
            y: from.y + (mesh.height / 2 - from.y) * t - height / 2
            NumberAnimation on t { running: modelData.active; from: 0; to: 0.82; duration: 1400; loops: Animation.Infinite; easing.type: Easing.InOutQuad }
        }
    }
    // The engines' capsule.
    Rectangle {
        id: capsule
        anchors.centerIn: parent
        width: Math.min(parent.width * 0.42, 460 * mesh.k * mesh.s * (mesh.compact ? 2.0 : 1))
        height: 136 * mesh.k * mesh.s * (mesh.compact ? 1.6 : 1)
        radius: height / 2
        color: "#1a1008"; border.color: mesh.cLine; border.width: 1
        Row {
            anchors.centerIn: parent
            spacing: 22 * mesh.k * mesh.s
            Repeater {
                model: mesh.engines
                delegate: Column {
                    required property var modelData
                    spacing: 4 * mesh.k * mesh.s
                    readonly property color ec: modelData.state === "off" ? mesh.cDim : modelData.id === "stt" ? mesh.cTeal : modelData.id === "chat" ? mesh.cChat : mesh.cPink
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 54 * mesh.k * mesh.s * (mesh.compact ? 1.4 : 1); height: width; radius: width / 2
                        color: Qt.rgba(parent.ec.r, parent.ec.g, parent.ec.b, modelData.state === "working" ? 0.22 : 0.10)
                        border.color: parent.ec; border.width: modelData.state === "off" ? 1 : 2
                        Icon { anchors.centerIn: parent; name: modelData.icon || "dot"; size: parent.width * 0.48; color: Theme.text; opacity: modelData.state === "off" ? 0.4 : 1 }
                        SequentialAnimation on scale {
                            running: modelData.state === "working"; loops: Animation.Infinite
                            NumberAnimation { to: 1.08; duration: 700; easing.type: Easing.InOutQuad }
                            NumberAnimation { to: 1.0; duration: 700; easing.type: Easing.InOutQuad }
                        }
                    }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.name; color: mesh.cText; font.family: Theme.font; font.pixelSize: Math.max(6, 13 * mesh.k * mesh.s * (mesh.compact ? 1.25 : 1)); font.weight: Font.DemiBold }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.status || ""; color: mesh.cMuted; font.family: Theme.font; font.pixelSize: Math.max(5, 11.5 * mesh.k * mesh.s * (mesh.compact ? 1.25 : 1)) }
                }
            }
        }
    }
    // The senders.
    Repeater {
        model: mesh.senders
        delegate: Rectangle {
            required property var modelData
            required property int index
            readonly property point p: mesh.posOf(index)
            readonly property color c: mesh.colorOf(modelData)
            x: p.x - width / 2; y: p.y - height / 2
            width: Math.max(nm.implicitWidth + cnt.implicitWidth + 30 * mesh.k * mesh.s, lst.implicitWidth + 22 * mesh.k * mesh.s)
            height: (modelData.last ? 46 : 30) * mesh.k * mesh.s * (mesh.compact ? 1.3 : 1)
            radius: 8 * mesh.k * mesh.s
            color: modelData.active ? "#1e130a" : "#140c05"
            border.color: modelData.active ? c : Qt.rgba(201 / 255, 163 / 255, 114 / 255, 0.22)
            border.width: modelData.active ? 2 : 1
            opacity: modelData.active || modelData.n > 0 ? 1 : 0.55
            Text { id: nm; x: 11 * mesh.k * mesh.s; y: 7 * mesh.k * mesh.s; text: modelData.name; color: mesh.cText; font.family: Theme.font; font.pixelSize: Math.max(5, 13 * mesh.k * mesh.s * (mesh.compact ? 1.3 : 1)); font.weight: Font.DemiBold }
            Text { id: cnt; anchors { right: parent.right; rightMargin: 11 * mesh.k * mesh.s; verticalCenter: nm.verticalCenter }
                text: String(modelData.n); color: modelData.active ? parent.c : mesh.cMuted; font.family: Theme.font; font.pixelSize: Math.max(5, 12 * mesh.k * mesh.s * (mesh.compact ? 1.3 : 1)); font.weight: Font.DemiBold }
            Text { id: lst; visible: !!modelData.last; x: nm.x; anchors.top: nm.bottom; anchors.topMargin: 2 * mesh.k * mesh.s
                text: modelData.last || ""; color: mesh.cMuted; font.family: Theme.font; font.pixelSize: Math.max(4.5, 11 * mesh.k * mesh.s * (mesh.compact ? 1.3 : 1)) }
        }
    }
    Text {
        visible: !mesh.compact && mesh.senders.length === 0
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 18 * mesh.k }
        text: "Nobody has sent the AI team work today. Your devices reach it through One AI address."
        color: mesh.cMuted; font.family: Theme.font; font.pixelSize: 14 * mesh.k
    }
}
