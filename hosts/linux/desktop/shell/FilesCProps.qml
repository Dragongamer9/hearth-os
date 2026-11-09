// Properties of one of Hearth (C:)'s own folders (docs/HEARTH-FILES.md "Hearth (C:) the Windows way", the owner's
// mockup "Properties of Program Files"): a Hearth view folder, how big, how many apps (or folders), which real places
// they come from, and that files can't be put straight into it. FileManager.showCProps fills fm.cProps; real folders
// get the usual Properties window. Loaded with a Loader inside the Files window: a problem here never stops Files.
import QtQuick
import QtQuick.Layouts

Item {
    id: root
    property var fm: null
    // A click outside the card closes it (as on Files' other dialogs).
    MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.RightButton; onClicked: if (root.fm) root.fm.closeOverlay() }

Rectangle {
    id: card
    readonly property var fm: root.fm
    readonly property var p: fm && fm.cProps ? fm.cProps : ({ name: "", places: [] })
    anchors.centerIn: parent
    width: Math.min(560, root.width - 48)
    height: Math.min(root.height - 48, body.implicitHeight + 156)   // title bar, OK row and margins
    radius: Theme.radius + 2
    color: Theme.panel
    border.color: Theme.line
    clip: true
    MouseArea { anchors.fill: parent }    // a click on the card doesn't close it
    Accessible.role: Accessible.Dialog
    Accessible.name: (p.name || "") + " Properties"

    // The title bar.
    Rectangle {
        id: head
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 44
        color: Theme.bg2
        radius: card.radius
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: card.radius; color: Theme.bg2 }
        Text { anchors { left: parent.left; leftMargin: 18; verticalCenter: parent.verticalCenter } text: (card.p.name || "") + " Properties"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
    }

    Flickable {
        anchors { left: parent.left; right: parent.right; top: head.bottom; bottom: foot.top; margins: 20 }
        contentHeight: body.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ColumnLayout {
            id: body
            width: parent.width
            spacing: 14
            RowLayout {
                spacing: 14
                WinGlyph { kind: card.p.page === "root" ? "system" : "folder"; size: 40 }
                ColumnLayout {
                    spacing: 2
                    Text { text: card.p.name || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                    Text { text: "A Hearth view folder"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
            }
            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }
            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 14
                rowSpacing: 9
                Text { text: "Location"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; Layout.preferredWidth: 100 }
                Text { Layout.fillWidth: true; text: card.p.location || ""; color: Theme.text; font.family: "monospace"; font.pixelSize: 12 }
                Text { visible: !!card.p.size; text: "Size"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Text { visible: !!card.p.size; Layout.fillWidth: true; color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                       text: card.p.size && card.fm ? card.fm.fmtSize(card.p.size) + " (" + Number(card.p.size).toLocaleString(Qt.locale(), "f", 0) + " bytes)" : "" }
                Text { text: "Contains"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Text { Layout.fillWidth: true; text: card.p.contains || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                Text { text: "Created"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Text { Layout.fillWidth: true; text: "Made by Hearth when Files opens it"; color: Theme.text; wrapMode: Text.WordWrap; font.family: Theme.font; font.pixelSize: 13 }
            }
            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }
            Text {
                visible: (card.p.places || []).length > 0
                text: card.p.page === "pf" || card.p.page === "x86"
                      ? "THE APPS IN IT COME FROM " + (card.p.places || []).length + ((card.p.places || []).length === 1 ? " PLACE" : " PLACES")
                      : "WHERE ITS FOLDERS REALLY ARE"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.5
            }
            Repeater {
                model: card.p.places || []
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.preferredHeight: placeCol.implicitHeight + 16
                    radius: Theme.r(6)
                    color: Theme.panel2
                    RowLayout {
                        anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                        spacing: 10
                        Rectangle { Layout.preferredWidth: 10; Layout.preferredHeight: 10; radius: 3; color: modelData.color || Theme.accent }
                        ColumnLayout {
                            id: placeCol
                            Layout.fillWidth: true
                            spacing: 1
                            Text { Layout.fillWidth: true; text: modelData.name || ""; color: Theme.text; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 13 }
                            Text { Layout.fillWidth: true; text: modelData.path || ""; color: Theme.muted; elide: Text.ElideMiddle; font.family: "monospace"; font.pixelSize: 11 }
                        }
                        Text { text: modelData.size && card.fm ? card.fm.fmtSize(modelData.size) : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: noteText.implicitHeight + 20
                radius: Theme.r(8)
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.10)
                border.width: 1
                border.color: Theme.line
                RowLayout {
                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                    spacing: 10
                    Icon { name: "info"; size: 18; color: Theme.accent }
                    Text { id: noteText; Layout.fillWidth: true; text: card.p.note || ""; color: Theme.text; wrapMode: Text.WordWrap; lineHeight: 1.1; font.family: Theme.font; font.pixelSize: 13 }
                }
            }
        }
    }

    // OK (Enter, A or B close it too: FileManager's overlay keys).
    Rectangle {
        id: foot
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: 56
        color: "transparent"
        Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
        Rectangle {
            anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter }
            width: 90; height: 34
            radius: Theme.r(6)
            color: Theme.accent
            Text { anchors.centerIn: parent; text: "OK"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: if (card.fm) card.fm.closeOverlay() }
        }
    }
}
}
