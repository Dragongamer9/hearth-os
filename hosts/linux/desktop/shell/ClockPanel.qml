// The clock's panel (ClockPanel): notifications you missed (with Clear all and Do Not Disturb) above a month calendar with
// today marked. Opens from the taskbar clock or bell, or `quickshell ipc ... call calendar toggle`.
// Keyboard / controller: Up/Down move, Enter (A) chooses, Left/Right (or Page Up/Down) change the
// month on the calendar row, Esc (B) closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Widgets

PanelWindow {
    id: root
    property bool shown: false
    property bool portraitBar: false       // the taskbar runs down the left side of this screen
    property int barSize: 60
    property date month: new Date()        // the month on show (its first day)
    property int nav: -1                 // -1 none, 0 Do Not Disturb, 1 Clear all, 2.. notifications, then the calendar row
    function toggle(scr, portrait) { shown ? close() : open(scr, portrait); }
    function open(scr, portrait) {
        if (scr) screen = scr;
        portraitBar = !!portrait;
        const t = new Date();
        month = new Date(t.getFullYear(), t.getMonth(), 1);
        nav = -1;
        shown = true;
        DeskState.unseen = 0;
        keys.forceActiveFocus();
    }
    function close() { shown = false; }

    readonly property var history: DeskState.history
    readonly property int calRow: 2 + history.length
    function shiftMonth(d) { month = new Date(month.getFullYear(), month.getMonth() + d, 1); }
    function activate(i) {
        if (i === 0) DeskState.setDnd(!DeskState.dnd);
        else if (i === 1) { DeskState.clearHistory(); nav = 0; }
        else if (i === calRow) { const t = new Date(); month = new Date(t.getFullYear(), t.getMonth(), 1); }
        else if (i >= 2) { DeskState.forget(history[i - 2].key); nav = Math.min(nav, calRow - 1); }
    }
    function when(d) {
        const s = Math.floor((Date.now() - d.getTime()) / 1000);
        if (s < 60) return "Just now";
        if (s < 3600) return Math.floor(s / 60) + " min ago";
        return Qt.formatTime(d, Settings.clock24 ? "HH:mm" : "h:mm AP");
    }

    visible: shown || closing.running
    Timer { id: closing; interval: Theme.calm + 60 }
    onShownChanged: if (!shown) closing.restart()
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-calendar"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Escape: root.close(); break;
            case Qt.Key_Up: root.nav = root.nav < 0 ? root.calRow : Math.max(0, root.nav - 1); if (root.history.length === 0 && root.nav === 1) root.nav = 0; break;
            case Qt.Key_Down: case Qt.Key_Tab: root.nav = root.nav < 0 ? 0 : Math.min(root.calRow, root.nav + 1); if (root.history.length === 0 && root.nav === 1) root.nav = root.calRow; break;
            case Qt.Key_Left: if (root.nav === root.calRow || root.nav < 0) root.shiftMonth(-1); else if (root.nav === 1) root.nav = 0; break;
            case Qt.Key_Right: if (root.nav === root.calRow || root.nav < 0) root.shiftMonth(1); else if (root.nav === 0 && root.history.length) root.nav = 1; break;
            case Qt.Key_PageUp: root.shiftMonth(-1); break;
            case Qt.Key_PageDown: root.shiftMonth(1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (root.nav >= 0) root.activate(root.nav); break;
            default: return;
            }
            event.accepted = true;
        }
    }

    MouseArea { anchors.fill: parent; onClicked: root.close() }

    Rectangle {
        id: panel
        width: Math.min(380, root.width - 24)
        height: Math.min(root.height - 24 - (root.portraitBar ? 0 : root.barSize), content.implicitHeight + 36)
        x: root.portraitBar ? root.barSize + 6 : root.width - width - 12
        y: root.height - height - (root.portraitBar ? 12 : root.barSize + 6) + (root.shown ? 0 : 24)
        opacity: root.shown ? 1 : 0
        Behavior on y { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        radius: Theme.radius + 6
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent }   // clicks inside don't close

        ColumnLayout {
            id: content
            anchors { fill: parent; margins: 18 }
            spacing: 14

            // ---------------------------------------------------------------- notifications
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text { Layout.fillWidth: true; text: "Notifications"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
                Chip {
                    navIndex: 1
                    visible: root.history.length > 0
                    label: "Clear all"
                }
            }
            Chip {
                navIndex: 0
                Layout.fillWidth: true
                icon: DeskState.dnd ? "bell-off" : "bell"
                label: "Do not disturb"
                detail: DeskState.dnd ? "On: notifications wait here quietly" : "Off"
                lit: DeskState.dnd
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(contentHeight, Math.max(120, root.height * 0.42))
                Layout.minimumHeight: root.history.length ? 80 : 0
                visible: root.history.length > 0
                clip: true
                spacing: 8
                model: root.history
                currentIndex: root.nav >= 2 && root.nav < root.calRow ? root.nav - 2 : -1
                onCurrentIndexChanged: if (currentIndex >= 0) positionViewAtIndex(currentIndex, ListView.Contain)
                delegate: Rectangle {
                    id: card
                    required property var modelData
                    required property int index
                    readonly property bool lit: root.nav === index + 2
                    width: list.width
                    implicitHeight: crow.implicitHeight + 20
                    radius: 12
                    color: hover.containsMouse ? Theme.panel3 : Theme.panel2
                    border.width: 2
                    border.color: lit ? Theme.accent : "transparent"
                    MouseArea { id: hover; anchors.fill: parent; hoverEnabled: true }
                    RowLayout {
                        id: crow
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                        spacing: 10
                        AppIcon {
                            Layout.alignment: Qt.AlignTop
                            size: 32
                            genericFallback: false
                            visible: hasIcon
                            source: card.modelData.image ? AppIcons.fromIcon(card.modelData.image) : ""
                            icon: card.modelData.icon || ""
                            appId: card.modelData.entry || ""
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            RowLayout {
                                Layout.fillWidth: true
                                Text { Layout.fillWidth: true; text: card.modelData.app; visible: text !== ""; color: Theme.muted; font.pixelSize: 11; font.family: Theme.font; elide: Text.ElideRight }
                                Text { text: root.when(card.modelData.time); color: Theme.muted; font.pixelSize: 11; font.family: Theme.font }
                            }
                            Text { Layout.fillWidth: true; text: card.modelData.summary; visible: text !== ""; color: Theme.text; font.bold: true; font.pixelSize: 13; font.family: Theme.font; elide: Text.ElideRight }
                            Text {
                                Layout.fillWidth: true
                                visible: text !== ""
                                text: card.modelData.body
                                textFormat: Text.PlainText
                                wrapMode: Text.Wrap
                                maximumLineCount: 3
                                elide: Text.ElideRight
                                color: Theme.text
                                opacity: 0.85
                                font.pixelSize: 12
                                font.family: Theme.font
                            }
                        }
                        Text {
                            Layout.alignment: Qt.AlignTop
                            text: "×"
                            color: x1.containsMouse || card.lit ? Theme.text : Theme.muted
                            font.pixelSize: 18
                            MouseArea { id: x1; anchors.fill: parent; anchors.margins: -8; hoverEnabled: true; onClicked: DeskState.forget(card.modelData.key) }
                        }
                    }
                }
            }
            Text {
                visible: root.history.length === 0
                Layout.fillWidth: true
                text: "No notifications"
                color: Theme.muted
                font.family: Theme.font
                font.pixelSize: 13
            }

            Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.line }

            // ---------------------------------------------------------------- calendar
            Rectangle {
                id: calCard
                Layout.fillWidth: true
                implicitHeight: cal.implicitHeight + 20
                radius: 12
                color: "transparent"
                border.width: 2
                border.color: root.nav === root.calRow ? Theme.accent : "transparent"
                MouseArea { anchors.fill: parent; onWheel: w => { if (w.angleDelta.y !== 0) root.shiftMonth(w.angleDelta.y > 0 ? -1 : 1); } }

                ColumnLayout {
                    id: cal
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                    spacing: 10

                    SystemClock { id: clock; precision: SystemClock.Minutes }
                    Text {
                        text: Qt.formatDate(clock.date, "dddd, MMMM d")
                        color: Theme.accent
                        font.family: Theme.font
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            Layout.fillWidth: true
                            text: Qt.formatDate(root.month, "MMMM yyyy")
                            color: Theme.text
                            font.family: Theme.font
                            font.pixelSize: 16
                            font.weight: Font.DemiBold
                        }
                        RoundButton { icon: "back"; tip: "Previous month"; onPicked: root.shiftMonth(-1) }
                        RoundButton { icon: "forward"; tip: "Next month"; onPicked: root.shiftMonth(1) }
                    }

                    GridLayout {
                        id: grid
                        Layout.fillWidth: true
                        columns: 7
                        rowSpacing: 2
                        columnSpacing: 2
                        // Weeks start on the day your language uses (Sunday or Monday).
                        readonly property int first: Qt.locale().firstDayOfWeek % 7
                        readonly property var days: {
                            const m = root.month;
                            const start = new Date(m.getFullYear(), m.getMonth(), 1);
                            const lead = (start.getDay() - first + 7) % 7;
                            const out = [];
                            for (let i = 0; i < 42; i++) out.push(new Date(m.getFullYear(), m.getMonth(), 1 - lead + i));
                            return out;
                        }
                        Repeater {
                            model: 7
                            delegate: Text {
                                required property int index
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                                text: Qt.locale().dayName((grid.first + index) % 7, Locale.NarrowFormat)
                                color: Theme.muted
                                font.family: Theme.font
                                font.pixelSize: 12
                            }
                        }
                        Repeater {
                            model: grid.days
                            delegate: Item {
                                id: day
                                required property var modelData
                                readonly property bool inMonth: modelData.getMonth() === root.month.getMonth()
                                readonly property bool today: modelData.toDateString() === clock.date.toDateString()
                                Layout.fillWidth: true
                                implicitHeight: 36
                                Rectangle {
                                    anchors.centerIn: parent
                                    width: 34; height: 34
                                    radius: 17
                                    color: day.today ? Theme.accent : "transparent"
                                    border.color: day.today ? Qt.lighter(Theme.accent, 1.2) : "transparent"
                                }
                                Text {
                                    anchors.centerIn: parent
                                    text: day.modelData.getDate()
                                    color: day.today ? Theme.accentInk : day.inMonth ? Theme.text : Theme.muted
                                    opacity: day.inMonth || day.today ? 1 : 0.55
                                    font.family: Theme.font
                                    font.pixelSize: 13
                                    font.bold: day.today
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks
    component Chip: Rectangle {
        id: c
        property int navIndex: -1
        property string icon: ""
        property string label
        property string detail: ""
        property bool lit: false
        readonly property bool focused: root.nav === navIndex
        implicitWidth: crow2.implicitWidth + 24
        implicitHeight: detail ? 56 : 34
        radius: 10
        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22) : (ca.containsMouse ? Theme.panel3 : Theme.panel2)
        border.width: 2
        border.color: focused ? Theme.accent : "transparent"
        RowLayout {
            id: crow2
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            spacing: 10
            Icon { visible: c.icon !== ""; name: c.icon; size: 20; color: c.lit ? Theme.accent : Theme.text }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text { text: c.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: c.detail ? Font.DemiBold : Font.Normal }
                Text { visible: c.detail !== ""; Layout.fillWidth: true; text: c.detail; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
            }
            // A switch for Do Not Disturb
            Rectangle {
                visible: c.detail !== ""
                implicitWidth: 40; implicitHeight: 22; radius: 11
                color: c.lit ? Theme.accent : Theme.panel3
                Rectangle { width: 16; height: 16; radius: 8; y: 3; x: c.lit ? parent.width - width - 3 : 3; color: c.lit ? Theme.accentInk : Theme.muted; Behavior on x { NumberAnimation { duration: Theme.quick } } }
            }
        }
        MouseArea { id: ca; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { root.nav = c.navIndex; root.activate(c.navIndex); } }
    }

    component RoundButton: Rectangle {
        id: rb
        property string icon
        property string tip
        signal picked()
        implicitWidth: 34; implicitHeight: 34
        radius: 17
        color: ra.containsMouse ? Theme.panel3 : Theme.panel2
        Icon { anchors.centerIn: parent; name: rb.icon; size: 16 }
        MouseArea { id: ra; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: rb.picked() }
    }
}
