// Sort my Downloads' sheet and note, in Hearth Files' window (FilesSortDownloads.qml has the work; mockups
// docs/mockups/1.17-sort-downloads): the preview of what goes where over the dimmed window (a row per folder in its
// colour, what goes in it and how many; To delete tinted red; Show every file, Cancel, Sort N items), and after
// sorting the note low in the middle: "✓ Sorted N items into 5 folders." with Undo and "Ctrl+Z also puts them back".
// Keys: Enter sorts, Esc cancels, Tab / arrows walk the buttons (A presses, B cancels with a controller).
import QtQuick
import QtQuick.Layouts

Item {
    id: sheet
    property var sd: null
    anchors.fill: parent
    visible: !!sd && (sd.open || !!sd.sorted)
    property int btn: 2                        // 0 Show every file · 1 Cancel · 2 Sort
    onVisibleChanged: if (visible && sd.open) { btn = 2; focusTaker.forceActiveFocus(); }
    Connections {
        target: sheet.sd
        function onOpenChanged() { if (sheet.sd.open) { sheet.btn = 2; focusTaker.forceActiveFocus(); } }
    }
    function press(i) { if (i === 0) sd.every = !sd.every; else if (i === 1) sd.cancel(); else sd.go(); }

    // the dimmed window behind the sheet
    Rectangle {
        anchors.fill: parent
        visible: sheet.sd && sheet.sd.open
        color: "#99000000"
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.RightButton; onClicked: sheet.sd.cancel() }
    }

    Item {
        id: focusTaker
        Keys.onPressed: e => {
            if (!sheet.sd || !sheet.sd.open) return;
            switch (e.key) {
            case Qt.Key_Escape: case Qt.Key_Back: sheet.sd.cancel(); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: sheet.press(sheet.btn); break;
            case Qt.Key_Left: case Qt.Key_Backtab: sheet.btn = Math.max(0, sheet.btn - 1); break;
            case Qt.Key_Right: case Qt.Key_Tab: sheet.btn = Math.min(2, sheet.btn + 1); break;
            case Qt.Key_Up: if (sheet.sd.every) list.contentY = Math.max(0, list.contentY - 60); break;
            case Qt.Key_Down: if (sheet.sd.every) list.contentY = Math.min(Math.max(0, list.contentHeight - list.height), list.contentY + 60); break;
            default: return;
            }
            e.accepted = true;
        }
    }

    // ------------------------------------------------ the preview (mockup 1)
    Rectangle {
        id: box
        visible: sheet.sd && sheet.sd.open && !!sheet.sd.plan
        anchors.centerIn: parent
        width: Math.min(parent.width - 48, 780)
        height: Math.min(parent.height - 48, col.implicitHeight + 64)
        radius: Theme.r(16)
        color: Theme.panel
        border.color: Theme.line
        border.width: 1
        MouseArea { anchors.fill: parent }    // a click on the sheet stays on it
        ColumnLayout {
            id: col
            anchors { fill: parent; margins: 32 }
            spacing: 10
            Text {
                text: "Sort my Downloads"
                color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold
            }
            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "The " + (sheet.sd && sheet.sd.plan ? sheet.sd.plan.moves.length : 0) + " things loose in Downloads go into folders by what they are. Nothing is deleted or overwritten, and one click puts everything back."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
            Flickable {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredHeight: rows.implicitHeight
                Layout.maximumHeight: sheet.sd && sheet.sd.every ? 520 : rows.implicitHeight
                contentHeight: rows.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                interactive: contentHeight > height
                ColumnLayout {
                    id: rows
                    width: list.width
                    spacing: 10
                    Repeater {
                        model: 5
                        delegate: Rectangle {
                            id: grp
                            required property int index
                            readonly property int n: sheet.sd ? sheet.sd.count(index) : 0
                            readonly property bool danger: index === 4
                            readonly property var files: sheet.sd && sheet.sd.plan ? sheet.sd.plan.moves.filter(m => m.group === index) : []
                            visible: !(danger && n === 0)
                            Layout.fillWidth: true
                            implicitHeight: grpRow.implicitHeight + 24
                            radius: 10
                            color: danger ? Qt.rgba(Theme.danger.r, Theme.danger.g, Theme.danger.b, 0.06) : "transparent"
                            border.width: 1
                            border.color: danger ? Qt.rgba(Theme.danger.r, Theme.danger.g, Theme.danger.b, 0.45) : Theme.line
                            RowLayout {
                                id: grpRow
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 }
                                spacing: 14
                                Rectangle {
                                    Layout.alignment: Qt.AlignVCenter
                                    width: 30; height: 30; radius: 7
                                    color: sheet.sd ? sheet.sd.colours[grp.index] : "transparent"
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 3
                                    Text {
                                        text: sheet.sd ? sheet.sd.titles[grp.index] : ""
                                        color: Theme.text; font.family: Theme.font; font.pixelSize: 15
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        elide: Text.ElideRight
                                        text: {
                                            if (!sheet.sd) return "";
                                            if (!grp.danger) return sheet.sd.what[grp.index] + (grp.index === 3 && sheet.sd.asking ? " · asking your AI about the rest…" : "");
                                            const names = grp.files.map(m => sheet.sd.app.base(m.from));
                                            const first = names.length ? names[0] : "";
                                            return names.length > 1 ? "Old copies Files found (" + first + " and " + (names.length - 1) + " more) · kept for you to empty yourself"
                                                                    : "Old copies Files found (" + first + ") · kept for you to empty yourself";
                                        }
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                    }
                                    Repeater {
                                        model: sheet.sd && sheet.sd.every ? grp.files : []
                                        delegate: Text {
                                            required property var modelData
                                            Layout.fillWidth: true
                                            elide: Text.ElideMiddle
                                            readonly property string root: sheet.sd && sheet.sd.plan ? String(sheet.sd.plan.dir).replace(/\/+$/, "") + "/" : ""
                                            text: sheet.sd.app.base(modelData.from) + " → " + (modelData.to.indexOf(root) === 0 ? modelData.to.slice(root.length) : modelData.to)
                                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12.5
                                        }
                                    }
                                }
                                Text {
                                    Layout.alignment: Qt.AlignVCenter
                                    text: sheet.sd ? sheet.sd.app.plural(grp.n, "item", "items") : ""
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                }
                            }
                        }
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 12
                spacing: 12
                Item { Layout.fillWidth: true }
                Repeater {
                    model: [sheet.sd && sheet.sd.every ? "Hide the files" : "Show every file", "Cancel",
                            "Sort " + (sheet.sd && sheet.sd.plan ? sheet.sd.app.plural(sheet.sd.plan.moves.length, "item", "items") : "")]
                    delegate: Rectangle {
                        id: b
                        required property var modelData
                        required property int index
                        readonly property bool primary: index === 2
                        implicitWidth: bt.implicitWidth + 40
                        implicitHeight: 44
                        radius: Theme.r(10)
                        color: primary ? Theme.accent : bm.containsMouse ? Theme.panel3 : Theme.panel2
                        border.width: 2
                        border.color: sheet.btn === index ? (primary ? Theme.text : Theme.accent) : primary ? "transparent" : Theme.line
                        Text {
                            id: bt
                            anchors.centerIn: parent
                            text: b.modelData
                            color: b.primary ? Theme.accentInk : Theme.text
                            font.family: Theme.font; font.pixelSize: 14; font.weight: b.primary ? Font.DemiBold : Font.Normal
                        }
                        MouseArea {
                            id: bm
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: { sheet.btn = b.index; sheet.press(b.index); }
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------ the note, with Undo (mockup 2)
    Rectangle {
        visible: sheet.sd && !!sheet.sd.sorted && !sheet.sd.open
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 60 }
        width: noteRow.implicitWidth + 36
        height: noteRow.implicitHeight + 24
        radius: 12
        color: Theme.panel
        border.color: Theme.line
        border.width: 1
        RowLayout {
            id: noteRow
            anchors.centerIn: parent
            spacing: 18
            Text {
                text: sheet.sd ? sheet.sd.noteText : ""
                color: Theme.text; font.family: Theme.font; font.pixelSize: 14
            }
            Rectangle {
                implicitWidth: ut.implicitWidth + 36
                implicitHeight: 40
                radius: Theme.r(10)
                color: um.containsMouse ? Theme.panel3 : Theme.panel2
                border.width: 1
                border.color: Theme.line
                Text { id: ut; anchors.centerIn: parent; text: "Undo"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                MouseArea { id: um; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: sheet.sd.undo() }
            }
            Text {
                text: "Ctrl+Z also puts them back"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
        }
    }
}
