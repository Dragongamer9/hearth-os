// Server mode › AI team, opened (A on its tile; 1.12, mockup next-update/server-ai-team-detail.png): the big live mesh
// (AiMesh.qml) beside three panels: Today (requests, answered here, sent on to another device, the busiest), Engines (what
// each is doing) and While you play (Light mode's own choices). A pauses or resumes the team (AI chat, Pictures and Voices
// together), X restarts it, Y opens Settings › Home server › AI devices, B goes back. Draw-only: ServerDash runs them.
import QtQuick
import QtQuick.Layouts

Item {
    id: det
    property var team: null
    property bool paused: false
    property real k: 1
    signal action(string a)
    signal close()
    readonly property color cText: "#fff4df"
    readonly property color cMuted: "#c9a372"
    readonly property color cLine: "#4a3212"
    readonly property color cCard: "#140c03"
    readonly property color cOk: "#6fe39a"
    readonly property color cAcc: "#ffb347"
    readonly property var today: team && team.today ? team.today : ({})

    component Panel: Rectangle {
        default property alias body: inner.data
        property string title: ""
        Layout.fillWidth: true
        implicitHeight: inner.implicitHeight + 40 * det.k
        radius: 14 * det.k; color: det.cCard; border.color: det.cLine
        ColumnLayout {
            id: inner
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 * det.k }
            spacing: 10 * det.k
            Text { text: parent.parent.title; color: det.cText; font.family: Theme.font; font.pixelSize: 18 * det.k; font.weight: Font.DemiBold }
        }
    }
    component Line: RowLayout {
        property string a: ""
        property string b: ""
        property color bc: det.cMuted
        Layout.fillWidth: true
        spacing: 12 * det.k
        Text { text: parent.a; color: det.cText; font.family: Theme.font; font.pixelSize: 15 * det.k; Layout.fillWidth: true; elide: Text.ElideRight }
        Text { text: parent.b; color: parent.bc; font.family: Theme.font; font.pixelSize: 15 * det.k; horizontalAlignment: Text.AlignRight; wrapMode: Text.WordWrap; Layout.maximumWidth: 200 * det.k }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32 * det.k
        spacing: 18 * det.k
        RowLayout {
            spacing: 18 * det.k
            Rectangle {
                width: 54 * det.k; height: width; implicitWidth: 54 * det.k; implicitHeight: 54 * det.k; radius: 12 * det.k
                color: Qt.rgba(1, 0.7, 0.28, 0.16)
                Icon { anchors.centerIn: parent; name: "sparkle"; size: 28 * det.k; color: Theme.accent }
            }
            ColumnLayout {
                spacing: 2
                Text { text: "AI team"; color: det.cText; font.family: Theme.font; font.pixelSize: 34 * det.k; font.weight: Font.DemiBold }
                Text { text: "This PC's AI, for all your devices · One AI address: " + (det.team ? det.team.address : ""); color: det.cMuted; font.family: Theme.font; font.pixelSize: 15 * det.k }
            }
            Rectangle {
                Layout.alignment: Qt.AlignTop
                implicitWidth: chipT.implicitWidth + 20 * det.k; height: 30 * det.k; radius: 8 * det.k
                readonly property color c: det.paused ? "#ff8a4c" : det.team && det.team.state === "off" ? det.cMuted : det.cOk
                color: Qt.rgba(c.r, c.g, c.b, 0.16)
                Text { id: chipT; anchors.centerIn: parent; text: det.paused ? "Paused" : det.team ? det.team.chip : ""; color: parent.c; font.family: Theme.font; font.pixelSize: 15 * det.k; font.weight: Font.DemiBold }
            }
            Item { Layout.fillWidth: true }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 22 * det.k
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true
                radius: 14 * det.k; color: "#0c0702"; border.color: det.cLine; clip: true
                AiMesh { anchors.fill: parent; anchors.margins: 10 * det.k; team: det.team; k: det.k }
            }
            ColumnLayout {
                Layout.preferredWidth: 300 * det.k; Layout.maximumWidth: 300 * det.k; Layout.fillHeight: true
                spacing: 22 * det.k
                Panel {
                    title: "Today"
                    Line { a: "Requests"; b: String(det.today.requests || 0) }
                    Line { a: "Answered here"; b: String(det.today.here || 0) }
                    Line {
                        a: "Sent on to another device"
                        b: det.today.sent && det.today.sent.length ? det.today.sent.map(r => r[1] + " · " + r[0] + " (" + r[2] + ")").join("\n") : "none"
                    }
                    Line { a: "Busiest"; b: det.today.busiest ? det.today.busiest[0] + " · " + det.today.busiest[1] : "—" }
                }
                Panel {
                    title: "Engines"
                    Repeater {
                        model: det.team ? det.team.engines : []
                        delegate: Line {
                            required property var modelData
                            a: modelData.name + " · " + (modelData.id === "chat" ? (modelData.model || "no model") : (modelData.engine || ""))
                            b: modelData.state === "off" ? "off" : modelData.id === "chat" ? (modelData.model ? "loaded" : "ready") : modelData.id === "voices" && modelData.state === "working" ? "speaking" : modelData.id === "pictures" ? (modelData.status || "") : "ready"
                            bc: modelData.state === "off" ? det.cMuted : (b === "loaded" || b === "speaking") ? det.cOk : det.cMuted
                        }
                    }
                }
                Panel {
                    title: "While you play"
                    Repeater {
                        model: det.team ? det.team.play : []
                        delegate: Line { required property var modelData; a: modelData[0]; b: modelData[1] }
                    }
                }
                Item { Layout.fillHeight: true }
            }
        }
        RowLayout {
            spacing: 12 * det.k
            ServerButton { k: det.k; label: det.paused ? "Resume the team" : "Pause the team"; keyName: "A"; primary: true; onClicked: det.action(det.paused ? "resume" : "pause") }
            ServerButton { k: det.k; label: "Restart"; keyName: "X"; onClicked: det.action("restart") }
            ServerButton { k: det.k; label: "Settings"; keyName: "Y"; onClicked: det.action("settings") }
            Item { Layout.fillWidth: true }
            Text { text: "B  Back to the dashboard"; color: det.cMuted; font.family: Theme.font; font.pixelSize: 15 * det.k
                MouseArea { anchors.fill: parent; onClicked: det.close() } }
        }
    }
}
