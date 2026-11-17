// Calendar widget: the month (Monday first, today lit, a dot on days with events; ‹ › for other months) and the
// agenda under it: what's coming up from Hearth Time's calendar (hearth-widgets events). Click a day to see its
// events. Settings: show the agenda or not.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "widget-layout.js" as L

WidgetBase {
    id: w
    property int year: new Date().getFullYear()
    property int month: new Date().getMonth()
    property var picked: null              // a day you clicked: { y, m, d }
    property var events: []
    readonly property bool agenda: c("agenda", true) !== false
    readonly property date today: new Date()
    implicitWidth: 340; implicitHeight: 380

    Process {
        id: ev
        command: ["python3", w.helper, "events"]
        running: w.live
        stdout: StdioCollector { onStreamFinished: { try { w.events = JSON.parse(text.trim().split("\n").pop()).events || []; } catch (e) { w.events = []; } } }
    }
    Timer { interval: 300000; running: w.live; repeat: true; onTriggered: ev.running = true }

    function key(y, m, d) { return y + "-" + ("0" + (m + 1)).slice(-2) + "-" + ("0" + d).slice(-2); }
    function eventsOn(k) { return events.filter(e => (e.date || "") <= k && (e.lastDate || e.date || "") >= k); }
    readonly property var dotted: { const o = {}; for (const e of events) o[e.date] = true; return o; }
    readonly property var shownEvents: {
        if (picked) return eventsOn(key(picked.y, picked.m, picked.d));
        const t = key(today.getFullYear(), today.getMonth(), today.getDate());
        return events.filter(e => (e.lastDate || e.date || "") >= t).slice(0, 6);
    }
    function when(e) {
        if (e.allDay) return "All day";
        return e.start ? Qt.formatTime(new Date(e.start), Settings.clock24 ? "HH:mm" : "h:mm AP") : "";
    }

    ColumnLayout {
        visible: !w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: Qt.formatDate(new Date(w.year, w.month, 1), "MMMM yyyy"); color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
            WBtn { icon: "back"; flat: true; size: 26; onClicked: { w.picked = null; if (w.month === 0) { w.month = 11; w.year--; } else w.month--; } }
            WBtn { icon: "forward"; flat: true; size: 26; onClicked: { w.picked = null; if (w.month === 11) { w.month = 0; w.year++; } else w.month++; } }
        }
        GridLayout {
            Layout.fillWidth: true
            columns: 7
            rowSpacing: 2; columnSpacing: 2
            Repeater {
                model: ["M", "T", "W", "T", "F", "S", "S"]
                delegate: Text { required property var modelData; Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: modelData; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
            Repeater {
                model: L.monthGrid(w.year, w.month)
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool isToday: modelData.y === w.today.getFullYear() && modelData.m === w.today.getMonth() && modelData.d === w.today.getDate()
                    readonly property bool isPicked: w.picked && w.picked.y === modelData.y && w.picked.m === modelData.m && w.picked.d === modelData.d
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.max(22, (w.height - (w.agenda ? 200 : 80)) / 6)
                    radius: Theme.r(8)
                    color: isToday ? Theme.accent : isPicked ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.15) : "transparent"
                    Text {
                        anchors.centerIn: parent
                        text: modelData.d
                        color: isToday ? Theme.accentInk : modelData.inMonth ? Theme.text : Qt.rgba(Theme.muted.r, Theme.muted.g, Theme.muted.b, 0.5)
                        font.family: Theme.font; font.pixelSize: 13; font.weight: isToday ? Font.Bold : Font.Normal
                    }
                    Rectangle {
                        visible: !!w.dotted[w.key(modelData.y, modelData.m, modelData.d)]
                        width: 4; height: 4; radius: 2
                        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 2 }
                        color: isToday ? Theme.accentInk : Theme.accent
                    }
                    MouseArea { anchors.fill: parent; onClicked: w.picked = isPicked ? null : { y: modelData.y, m: modelData.m, d: modelData.d } }
                }
            }
        }
        Rectangle { visible: w.agenda; Layout.fillWidth: true; height: 1; color: Theme.line }
        Text {
            visible: w.agenda
            text: w.picked ? Qt.formatDate(new Date(w.picked.y, w.picked.m, w.picked.d), "dddd d MMMM") : "Coming up"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
        }
        Repeater {
            model: w.agenda ? w.shownEvents : []
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                Rectangle { width: 4; Layout.fillHeight: true; Layout.minimumHeight: 30; radius: 2; color: modelData.color || Theme.accent }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Text { Layout.fillWidth: true; text: modelData.title || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
                    Text {
                        Layout.fillWidth: true
                        text: [modelData.date === w.key(w.today.getFullYear(), w.today.getMonth(), w.today.getDate()) ? "Today" : Qt.formatDate(new Date(modelData.date + "T12:00:00"), "ddd d MMM"), w.when(modelData), modelData.location || ""].filter(Boolean).join(" · ")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight
                    }
                }
            }
        }
        Text {
            visible: w.agenda && w.shownEvents.length === 0
            text: w.picked ? "Nothing that day" : "Nothing coming up"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }
        Item { Layout.fillHeight: true }
    }

    ColumnLayout {
        visible: w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Calendar"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        WBtn { text: w.agenda ? "Agenda: shown" : "Agenda: hidden"; on: w.agenda; onClicked: w.set("agenda", !w.agenda) }
        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Events come from Hearth Time's calendar (add your Google, Outlook or other calendars there)."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        WBtn { text: "Open Hearth Time"; icon: "calendar"; onClicked: Quickshell.execDetached(["hearth-time"]) }
        Item { Layout.fillHeight: true }
    }
}
