// A dropdown setting card (Settings › Personalisation: Look, Sub theme). The card shows the choice on the right (a
// small picture, its name and colour dots); A / Enter (or a click) opens the list under it, Left / Right pick the
// previous / next one without opening it. In the list: Up / Down move, A picks and closes, B closes. Options can
// carry a group heading ("Colours", "Black"), a picture and colour dots.
//   options: [{ value, label, desc, thumb, dots: ["#rrggbb"…], head }]
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: dd
    property string icon
    property string title
    property string sub
    property var options: []
    property var current
    property bool open: false
    property int boxWidth: 300
    signal chosen(var value)
    readonly property alias head: head
    readonly property int idx: Math.max(0, options.findIndex(o => o.value === current))
    readonly property var cur: options[idx] || ({})
    Layout.fillWidth: true
    spacing: 0

    function pick(v) {
        open = false;
        head.forceActiveFocus();
        if (v !== current) chosen(v);
    }
    function close() { open = false; head.forceActiveFocus(); }
    function openList() {
        if (options.length < 2) return;
        open = true;
        Qt.callLater(() => { const it = rep.itemAt(dd.idx); if (it) it.opt.forceActiveFocus(); });
    }

    SetBase {
        id: head
        Layout.fillWidth: true
        usesLeftRight: true
        ownsEscape: dd.open                    // B on the card while the list is open closes the list
        onEscaped: dd.close()
        readonly property int textLeft: SetStyle.pad + (dd.icon !== "" ? 38 : 0)
        readonly property bool inline: width - textLeft - SetStyle.pad * 2 >= dd.boxWidth + 200
        implicitHeight: inline ? Math.max(SetStyle.cardHeight, txt.implicitHeight + 24) : txt.implicitHeight + box.height + 38
        onActivated: dd.open ? dd.close() : dd.openList()
        onAdjusted: d => { const i = dd.idx + d; if (i >= 0 && i < dd.options.length && i !== dd.idx) dd.chosen(dd.options[i].value); }
        Accessible.role: Accessible.ComboBox
        Accessible.name: dd.title
        Accessible.description: (dd.cur.label || "") + (dd.open ? ". Open" : "")

        Icon {
            visible: dd.icon !== ""
            x: SetStyle.pad
            y: head.inline ? (head.height - height) / 2 : 16
            size: 22; name: dd.icon; color: Theme.text
        }
        ColumnLayout {
            id: txt
            x: head.textLeft
            y: head.inline ? (head.height - height) / 2 : 14
            width: head.inline ? head.width - head.textLeft - box.width - SetStyle.pad * 2 : head.width - head.textLeft - SetStyle.pad
            spacing: 2
            Text { Layout.fillWidth: true; text: dd.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium; elide: Text.ElideRight }
            Text { visible: dd.sub !== ""; Layout.fillWidth: true; text: dd.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
        }
        // The choice, as a dropdown button.
        Rectangle {
            id: box
            x: head.inline ? head.width - width - SetStyle.pad : head.textLeft
            y: head.inline ? (head.height - height) / 2 : txt.y + txt.height + 10
            width: Math.min(dd.boxWidth, head.width - head.textLeft - SetStyle.pad)
            height: 42
            radius: Math.max(4, Theme.r(8))
            color: SetStyle.sunk
            border.width: 1
            border.color: dd.open ? Theme.accent : Theme.line
            RowLayout {
                anchors { fill: parent; leftMargin: dd.cur.thumb ? 5 : 12; rightMargin: 10 }
                spacing: 10
                Rectangle {
                    visible: !!dd.cur.thumb
                    Layout.preferredWidth: 54; Layout.preferredHeight: 30
                    radius: Math.max(3, Theme.r(4)); clip: true; color: Theme.bg2
                    Image { anchors.fill: parent; source: dd.cur.thumb || ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 160 }
                }
                Dots { dots: dd.cur.dots || []; visible: !dd.cur.thumb }
                Text { Layout.fillWidth: true; text: dd.cur.label || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Dots { dots: dd.cur.dots || []; visible: !!dd.cur.thumb }
                Icon { name: dd.open ? "chevu" : "chevd"; size: 16; color: Theme.muted; visible: dd.options.length > 1 }
            }
        }
    }

    // The list: under the card, so it pushes the page down and the controller can scroll to every option.
    Rectangle {
        visible: dd.open
        Layout.fillWidth: true
        Layout.topMargin: 6
        implicitHeight: col.implicitHeight + 12
        color: SetStyle.sunk
        radius: SetStyle.radius
        border.width: 1
        border.color: SetStyle.tint(Theme.text, 0.05)
        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 6 }
            spacing: 2
            Repeater {
                id: rep
                model: dd.open ? dd.options : []
                delegate: ColumnLayout {
                    id: entry
                    required property var modelData
                    required property int index
                    readonly property alias opt: opt
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        visible: !!entry.modelData.head
                        Layout.leftMargin: 12; Layout.topMargin: entry.index === 0 ? 4 : 10; Layout.bottomMargin: 2
                        text: (entry.modelData.head || "").toUpperCase()
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.Bold; font.letterSpacing: 0.8
                    }
                    SetBase {
                        id: opt
                        readonly property bool lit: entry.modelData.value === dd.current
                        Layout.fillWidth: true
                        implicitHeight: 58
                        flat: true
                        ownsEscape: true
                        usesLeftRight: true            // Left / Right do nothing in the list (B closes it)
                        color: lit ? SetStyle.tint(Theme.accent, 0.16) : (optMouse.containsMouse ? SetStyle.tint(Theme.text, 0.05) : "transparent")
                        onActivated: dd.pick(entry.modelData.value)
                        onEscaped: dd.close()
                        Accessible.role: Accessible.ListItem
                        Accessible.name: entry.modelData.label || ""
                        Accessible.description: (lit ? "Selected. " : "") + (entry.modelData.desc || "")
                        RowLayout {
                            anchors { fill: parent; leftMargin: 6; rightMargin: 12 }
                            spacing: 12
                            Rectangle {
                                visible: !!entry.modelData.thumb
                                Layout.preferredWidth: 80; Layout.preferredHeight: 45
                                radius: Math.max(3, Theme.r(4)); clip: true; color: Theme.bg2
                                Image { anchors.fill: parent; source: entry.modelData.thumb || ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 200 }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: entry.modelData.thumb ? 0 : 6
                                spacing: 1
                                Text { Layout.fillWidth: true; text: entry.modelData.label || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                Text { visible: !!entry.modelData.desc; Layout.fillWidth: true; text: entry.modelData.desc || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                            }
                            Dots { dots: entry.modelData.dots || [] }
                            Text { visible: opt.lit; text: "✓"; color: Theme.accent; font.pixelSize: 15; font.weight: Font.Bold }
                        }
                        MouseArea { id: optMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: dd.pick(entry.modelData.value) }
                    }
                }
            }
        }
    }

    // A few colour dots (a sub theme's accent, highlight and third colour).
    component Dots: Row {
        property var dots: []
        spacing: 3
        Repeater {
            model: parent.dots
            delegate: Rectangle {
                required property string modelData
                width: 12; height: 12; radius: 6
                color: modelData
                border.width: 1; border.color: SetStyle.tint(Theme.text, 0.18)
            }
        }
    }
}
