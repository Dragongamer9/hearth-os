// Hearth Settings › Printers › Ink & toner (1.14, KDE's ink levels and ink-low notices, in Hearth's look): each
// printer's cartridges as bars in their own colours, the low ones marked, and the switch for the notice when one runs
// low (InkWatch.qml). Read from CUPS by hearth-ink; nothing is changed.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Ink & toner"
    property var st: null
    property string message: ""
    function load() { job.run(["levels"], r => page.st = r, e => { page.st = { printers: [] }; page.message = e; }); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-ink" }

    SetCardGroup {
        title: "Notices"
        SetSwitch {
            icon: "bell"; title: "Tell me when ink or toner runs low"
            sub: "One notice per cartridge, again only after it's been filled"
            checked: Settings.inkNotify
            onToggled: v => Settings.inkNotify = v
        }
    }
    SetRow { visible: !page.st; interactive: false; icon: "printer"; title: "Asking the printers…" }
    SetRow {
        visible: !!page.st && page.st.printers.length === 0
        interactive: false; icon: "printer"; title: "No printers"; sub: page.message || "Add one in Printers"
    }
    Repeater {
        model: page.st ? page.st.printers : []
        delegate: SetPanel {
            id: pp
            required property var modelData
            readonly property var known: modelData.markers.filter(m => m.level >= 0)
            implicitHeight: pcol.implicitHeight + 32
            ColumnLayout {
                id: pcol
                x: 20; y: 16; width: parent.width - 40
                spacing: 10
                RowLayout {
                    spacing: 10
                    Icon { name: "printer"; size: 20; color: Theme.muted }
                    Text { text: pp.modelData.name.replace(/_/g, " "); color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                }
                Text {
                    visible: pp.modelData.markers.length === 0 || pp.known.length === 0
                    Layout.fillWidth: true
                    text: pp.modelData.markers.length === 0 ? "This printer doesn't say how full it is" : "Some left (the printer doesn't give numbers)"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }
                Repeater {
                    model: pp.known
                    delegate: RowLayout {
                        required property var modelData
                        readonly property bool low: modelData.level <= Math.max(1, modelData.low || 15)
                        Layout.fillWidth: true
                        spacing: 12
                        Rectangle {
                            implicitWidth: 14; implicitHeight: 14; radius: 7
                            color: modelData.color ? "#" + modelData.color : Theme.muted
                            border.width: 1; border.color: Theme.panel3
                        }
                        Text { Layout.preferredWidth: 170; text: modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 12; radius: 6; color: Theme.panel3
                            Rectangle {
                                height: parent.height; radius: 6
                                width: parent.width * Math.max(0.02, Math.min(1, modelData.level / 100))
                                color: modelData.color && modelData.color !== "000000" && modelData.color !== "ffffff" ? "#" + modelData.color : Theme.accent
                            }
                        }
                        Text { Layout.preferredWidth: 46; horizontalAlignment: Text.AlignRight; text: modelData.level + " %"; color: parent.low ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                        Rectangle {
                            opacity: parent.low ? 1 : 0
                            implicitWidth: lowT.implicitWidth + 16; implicitHeight: 22; radius: 11; color: Qt.rgba(Theme.danger.r, Theme.danger.g, Theme.danger.b, 0.18)
                            Text { id: lowT; anchors.centerIn: parent; text: "Low"; color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        }
                    }
                }
            }
        }
    }
    SetRow { visible: !!page.st; icon: "sync"; chevron: false; title: "Ask again"; onActivated: if (!job.busy) page.load() }
}
