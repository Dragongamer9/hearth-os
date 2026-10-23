// The clock's panel (ClockPanel): notifications you missed (with Clear all and Do Not Disturb) above a month calendar with
// today marked. Opens from the taskbar clock or bell, or `quickshell ipc ... call calendar toggle`.
// With Hearth Calendar installed (hearth-time), days with events get a dot and the next events show under the month,
// with a row that opens Calendar.
// Keyboard / controller: Up/Down move, Enter (A) chooses, Left/Right (or Page Up/Down) change the
// month on the calendar row, Esc (B) closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
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
        readEvents();   // Hearth Calendar
        DeskState.unseen = 0;
        keys.forceActiveFocus();
    }
    function close() { shown = false; }

    readonly property var history: DeskState.history
    readonly property int calRow: 2 + history.length
    function shiftMonth(d) { month = new Date(month.getFullYear(), month.getMonth() + d, 1); readEvents(); }

    // ---- Hearth Calendar (hosts/hearth-time): events from `hearth-time --events FROM TO` for the month on show and
    // the week ahead. hasCalendar: the program answered (else none of this shows).
    property var calEvents: []
    property bool hasCalendar: false
    readonly property int calOpenRow: calRow + 1
    readonly property int lastRow: hasCalendar ? calOpenRow : calRow
    function ymd(d) { return Qt.formatDate(d, "yyyy-MM-dd"); }
    function readEvents() {
        const first = new Date(month.getFullYear(), month.getMonth(), 1);
        const lead = (first.getDay() - Qt.locale().firstDayOfWeek % 7 + 7) % 7;
        const from = new Date(first.getFullYear(), first.getMonth(), 1 - lead);
        const t = new Date();
        const today = new Date(t.getFullYear(), t.getMonth(), t.getDate());
        const start = from < today ? from : today;
        const gridEnd = new Date(from.getFullYear(), from.getMonth(), from.getDate() + 42);
        const weekEnd = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 8);
        calRead.running = false;
        calRead.command = ["hearth-time", "--events", ymd(start), ymd(gridEnd > weekEnd ? gridEnd : weekEnd)];
        calRead.running = true;
    }
    Process {
        id: calRead
        stdout: StdioCollector {
            onStreamFinished: {
                try { const r = JSON.parse(text.trim().split("\n").pop()); root.calEvents = r.events || []; root.hasCalendar = true; }
                catch (e) { root.calEvents = []; }
            }
        }
    }
    readonly property var eventDays: {
        const m = {};
        for (const e of calEvents) {
            const a = new Date(e.date + "T00:00:00"), b = new Date((e.lastDate || e.date) + "T00:00:00");
            for (let d = a, n = 0; d <= b && n < 62; d = new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1), n++) m[ymd(d)] = true;
        }
        return m;
    }
    readonly property var upcoming: {
        const t = new Date(), k = ymd(t), end = ymd(new Date(t.getFullYear(), t.getMonth(), t.getDate() + 7));
        return calEvents.filter(e => (e.lastDate || e.date) >= k && e.date <= end && (e.allDay || e.end > Date.now())).slice(0, 4);
    }
    function whenText(e) {
        const t = new Date();
        const k = e.date < ymd(t) ? ymd(t) : e.date;
        const tm = new Date(t.getFullYear(), t.getMonth(), t.getDate() + 1);
        const day = k === ymd(t) ? "Today" : k === ymd(tm) ? "Tomorrow" : Qt.formatDate(new Date(k + "T00:00:00"), "ddd d MMM");
        return e.allDay ? day + " · All day" : day + " · " + Qt.formatTime(new Date(e.start), Settings.clock24 ? "HH:mm" : "h:mm AP");
    }
    function openCalendar() { close(); Quickshell.execDetached(["hearth-calendar"]); }
    // ---- end of Hearth Calendar
    function activate(i) {
        if (i === 0) DeskState.setDnd(!DeskState.dnd);
        else if (i === 1) { DeskState.clearHistory(); nav = 0; }
        else if (i === calRow) { const t = new Date(); month = new Date(t.getFullYear(), t.getMonth(), 1); readEvents(); }
        else if (i === calOpenRow && hasCalendar) openCalendar();   // Hearth Calendar
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
            case Qt.Key_Up: root.nav = root.nav < 0 ? root.lastRow : Math.max(0, root.nav - 1); if (root.history.length === 0 && root.nav === 1) root.nav = 0; break;
            case Qt.Key_Down: case Qt.Key_Tab: root.nav = root.nav < 0 ? 0 : Math.min(root.lastRow, root.nav + 1); if (root.history.length === 0 && root.nav === 1) root.nav = root.calRow; break;
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
                        IconImage {
                            Layout.alignment: Qt.AlignTop
                            implicitSize: 32
                            visible: source != ""
                            source: card.modelData.image || (card.modelData.icon ? Quickshell.iconPath(card.modelData.icon, true) : "")
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
                                // Hearth Calendar: a dot on days with events.
                                Rectangle {
                                    visible: !!root.eventDays[root.ymd(day.modelData)]
                                    width: 4; height: 4; radius: 2
                                    anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 2 }
                                    color: day.today ? Theme.accentInk : Theme.accent
                                    z: 1
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

            // ---- Hearth Calendar: the next events (a week ahead) and a row that opens Calendar.
            ColumnLayout {
                Layout.fillWidth: true
                visible: root.hasCalendar
                spacing: 6
                Repeater {
                    model: root.upcoming
                    delegate: Item {
                        id: up
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: urow.implicitHeight + 4
                        RowLayout {
                            id: urow
                            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter }
                            spacing: 10
                            Rectangle { implicitWidth: 4; Layout.fillHeight: true; radius: 2; color: up.modelData.color || Theme.accent }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Text { Layout.fillWidth: true; text: up.modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                Text { Layout.fillWidth: true; text: root.whenText(up.modelData) + (up.modelData.location ? " · " + up.modelData.location : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
                            }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.openCalendar() }
                    }
                }
                Text {
                    visible: root.upcoming.length === 0
                    text: "Nothing planned this week"
                    color: Theme.muted
                    font.family: Theme.font
                    font.pixelSize: 12
                }
                Chip {
                    navIndex: root.calOpenRow
                    Layout.fillWidth: true
                    icon: "calendar"
                    label: "Open Calendar"
                }
            }
            // ---- end of Hearth Calendar
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
