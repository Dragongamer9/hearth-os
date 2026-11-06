// Server mode: a tile opened on the dashboard (A on a tile). AI chat shows its models, what's on the graphics card and
// who can ask; every other service its rows. X restarts, Y pauses or resumes (AI chat: A unloads its model), B goes back.
// Draw-only: the dashboard runs the actions through hearth-server-mode.
import QtQuick
import QtQuick.Layouts

Item {
    id: det
    property var t: null
    property var d: null
    property real k: 1
    signal action(string a)
    signal close()
    readonly property bool ai: !!t && t.id === "ai"
    readonly property var vram: d && d.vram ? d.vram : []
    readonly property var names: ({ ai: "AI chat", pictures: "Pictures", voices: "Voices", media: "Media", other: "Other apps" })
    readonly property var colors: ({ ai: Theme.accent, pictures: "#8b9cf7", voices: "#e58fd0", media: "#5bd18b", other: Theme.muted })

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32 * det.k
        spacing: 18 * det.k
        RowLayout {
            spacing: 18 * det.k
            Rectangle { width: 52 * det.k; height: width; implicitWidth: 52 * det.k; implicitHeight: 52 * det.k; radius: 12 * det.k; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
                Icon { anchors.centerIn: parent; name: det.t ? det.t.icon : "dot"; size: 28 * det.k; color: Theme.accent } }
            ColumnLayout { spacing: 2; Layout.fillWidth: true
                Text { text: det.t ? det.t.title : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 30 * det.k; font.weight: Font.DemiBold }
                Text { text: det.t ? det.t.engine + (det.ai ? " · answers your devices over Tailscale" : "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * det.k } }
            Text { text: det.t ? det.t.chip : ""; color: SetStyle.ok; font.family: Theme.font; font.pixelSize: 15 * det.k; font.weight: Font.DemiBold }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 16 * det.k
            // Left: models (AI chat) or what it's doing.
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1
                radius: 14 * det.k; color: SetStyle.card
                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 20 * det.k; spacing: 10 * det.k
                    Text { text: det.ai ? "Models" : "Now"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 * det.k; font.weight: Font.DemiBold }
                    Repeater {
                        model: det.ai && det.t ? (det.t.models || []) : []
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true; spacing: 12 * det.k
                            readonly property bool isOn: (det.t.loaded || []).indexOf(modelData) >= 0
                            Icon { name: parent.isOn ? "check" : "drive"; size: 20 * det.k; color: parent.isOn ? SetStyle.ok : Theme.muted }
                            Text { Layout.fillWidth: true; text: modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 * det.k; elide: Text.ElideRight }
                            Text { text: parent.isOn ? "Loaded" : "Downloaded"; color: parent.isOn ? SetStyle.ok : Theme.muted; font.family: Theme.font; font.pixelSize: 14 * det.k }
                        }
                    }
                    Text { visible: det.ai && det.t && !(det.t.models || []).length; text: "No models downloaded: Settings › Home server › AI chat"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * det.k }
                    Text { visible: !det.ai; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: det.t ? ((det.t.big || "") + " " + (det.t.unit || "") + (det.t.sub ? "\n" + det.t.sub : "")) : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 20 * det.k }
                    Repeater {
                        model: det.t && !det.ai ? (det.t.rows || []).concat((det.t.boxes || []).map(b => [b[0], b[1] + (b[2] ? " · " + b[2] : "")])) : []
                        delegate: RowLayout { required property var modelData; Layout.fillWidth: true
                            Text { text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * det.k; Layout.fillWidth: true; elide: Text.ElideMiddle }
                            Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * det.k; font.weight: Font.DemiBold } }
                    }
                    Item { Layout.fillHeight: true }
                    Text { visible: det.ai; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 * det.k
                        text: d && d.pauses ? (d.pauses.ai === "small" ? "While you play: " + d.pauses.small + " answers on the processor; " + d.pauses.big + " loads again when you're back. Models are never deleted." : d.pauses.ai === "unload" ? "While you play: the model is unloaded from the graphics card." : d.pauses.ai === "stop" ? "While you play: AI chat stops." : "While you play: it keeps its model loaded.") : "" }
                }
            }
            // Middle: the graphics card's memory, by service.
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1
                radius: 14 * det.k; color: SetStyle.card
                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 20 * det.k; spacing: 12 * det.k
                    Text { text: "Graphics memory"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 * det.k; font.weight: Font.DemiBold }
                    Text { text: det.d && det.d.vramTotal ? det.d.vramUsed + " of " + Math.round(det.d.vramTotal) + " GB used" : "Not known for this graphics card"; color: Theme.text; font.family: Theme.font; font.pixelSize: 28 * det.k; font.weight: Font.DemiBold }
                    Rectangle {
                        Layout.fillWidth: true; height: 14 * det.k; radius: 7 * det.k; color: SetStyle.sunk; clip: true
                        Row { anchors.fill: parent
                            Repeater { model: det.vram
                                delegate: Rectangle { required property var modelData; height: parent.height; color: det.colors[modelData.who] || Theme.muted
                                    width: det.d && det.d.vramTotal ? parent.width * modelData.gb / det.d.vramTotal : 0 } } }
                    }
                    Repeater { model: det.vram
                        delegate: RowLayout { required property var modelData; Layout.fillWidth: true; spacing: 10 * det.k
                            Rectangle { width: 12 * det.k; height: width; implicitWidth: 12 * det.k; implicitHeight: 12 * det.k; radius: 3 * det.k; color: det.colors[modelData.who] || Theme.muted }
                            Text { text: det.names[modelData.who] || modelData.who; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * det.k; Layout.fillWidth: true }
                            Text { text: modelData.gb + " GB"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * det.k } } }
                    Item { Layout.fillHeight: true }
                }
            }
            // Right: who can reach it.
            Rectangle {
                Layout.fillWidth: true; Layout.fillHeight: true; Layout.preferredWidth: 1
                radius: 14 * det.k; color: SetStyle.card
                ColumnLayout {
                    anchors.fill: parent; anchors.margins: 20 * det.k; spacing: 12 * det.k
                    Text { text: "Who can reach it"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 * det.k; font.weight: Font.DemiBold }
                    Repeater {
                        model: [["tailscale", "Your devices on Tailscale", "Only devices signed in to your Tailscale", true],
                                ["server", "This PC", "Apps on this PC itself", true],
                                ["globe", "The internet", "Never: Hearth's firewall only lets Tailscale in", false]]
                        delegate: RowLayout { required property var modelData; Layout.fillWidth: true; spacing: 12 * det.k
                            Icon { name: modelData[0]; size: 20 * det.k; color: Theme.muted }
                            ColumnLayout { spacing: 0; Layout.fillWidth: true
                                Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * det.k }
                                Text { text: modelData[2]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 * det.k; Layout.fillWidth: true; elide: Text.ElideRight } }
                            Text { text: modelData[3] ? "On" : "Off"; color: modelData[3] ? SetStyle.ok : Theme.muted; font.family: Theme.font; font.pixelSize: 14 * det.k; font.weight: Font.DemiBold } }
                    }
                    Text { text: "Connected now"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * det.k; font.weight: Font.DemiBold; Layout.topMargin: 10 * det.k }
                    Repeater { model: det.d && det.t ? (det.d.who || []).filter(w => (w.what || []).indexOf(det.t.title) >= 0) : []
                        delegate: Text { required property var modelData; text: "● " + modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * det.k } }
                    Item { Layout.fillHeight: true }
                }
            }
        }
        RowLayout {
            spacing: 12 * det.k
            ServerButton { visible: det.ai; k: det.k; label: "Unload model"; keyName: "A"; primary: true; onClicked: det.action("unload") }
            ServerButton { visible: !!det.t && (det.t.actions || []).indexOf("restart") >= 0; k: det.k; label: "Restart"; keyName: "X"; onClicked: det.action("restart") }
            ServerButton { visible: !!det.t && (det.t.actions || []).indexOf("pause") >= 0; k: det.k; label: det.t && det.t.state === "paused-you" ? "Resume" : "Pause"; keyName: "Y"; onClicked: det.action("pause") }
            Item { Layout.fillWidth: true }
            Text { text: "B  Back to the dashboard"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 * det.k
                MouseArea { anchors.fill: parent; onClicked: det.close() } }
        }
    }
}
