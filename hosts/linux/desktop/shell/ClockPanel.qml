// The clock's panel (ClockPanel): the notification centre (with Clear all and Do Not Disturb) and, Mantel 2026, a
// dashboard: a month calendar with today marked, what's playing, the weather and the system. Opens from the taskbar clock or bell, or `quickshell ipc ... call calendar toggle`.
// With Hearth Calendar installed (hearth-time), days with events get a dot and the next events show under the month,
// with a row that opens Calendar.
// Keyboard / controller: Up/Down move, Enter (A) chooses, Left/Right (or Page Up/Down) change the
// month on the calendar row, Esc (B) closes, Delete removes the notification you're on.
// 1.14: copies and downloads in progress (KDE's job progress, NotifyHub.qml) with Pause and Cancel; each notification's
// own buttons kept in the list (DeskState.invoke) and its progress; why it's quiet now (automatic Do Not Disturb);
// world clocks, another calendar's dates and holidays under the month (Settings › Date & time; BarLayout.qml).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import "calendars.js" as Cal

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
        weatherRead.running = true;
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
    // Hearth Weather's summary for the dashboard ({} without a place or the app).
    property var weather: ({})
    Process {
        id: weatherRead
        command: ["hearth-time", "--weather-summary"]
        stdout: StdioCollector { onStreamFinished: { try { root.weather = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { root.weather = {}; } } }
    }
    // ---- end of Hearth Calendar
    // ---- 1.14: another calendar, holidays and world clocks (Settings › Date & time; BarLayout.qml keeps them)
    readonly property string alt: BarLayout.clockAlt
    readonly property var holMap: Cal.holidayMap(BarLayout.clockHolidays, [month.getFullYear() - 1, month.getFullYear(), month.getFullYear() + 1])
    readonly property var nextHoliday: {
        const k = ymd(new Date());
        const ks = Object.keys(holMap).filter(x => x >= k).sort();
        return ks.length ? { date: ks[0], name: holMap[ks[0]] } : null;
    }
    function altOf(d, f) { const a = alt ? Cal.convert(d, alt) : null; return a ? String(a[f]) : ""; }
    property var zoneOffsets: ({})
    Process {
        id: zoneRead
        command: ["python3", decodeURIComponent(Qt.resolvedUrl("hearth-widgets").toString().replace(/^file:\/\//, "")), "zones"].concat(BarLayout.clockZones)
        stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); const m = {}; for (const z of o.zones || []) if (z.offset !== null) m[z.tz] = z.offset; root.zoneOffsets = m; } catch (e) {} } }
    }
    function zoneTime(tz) {
        const off = zoneOffsets[tz];
        if (off === undefined) return "…";
        const d = new Date(clock.date.getTime() + clock.date.getTimezoneOffset() * 60000 + off * 60000);
        return Qt.formatTime(d, Settings.clock24 ? "HH:mm" : "h:mm AP");
    }
    function zoneDay(tz) {
        const off = zoneOffsets[tz];
        if (off === undefined) return "";
        const d = new Date(clock.date.getTime() + clock.date.getTimezoneOffset() * 60000 + off * 60000);
        const diff = Math.round((new Date(d.getFullYear(), d.getMonth(), d.getDate()) - new Date(clock.date.getFullYear(), clock.date.getMonth(), clock.date.getDate())) / 86400000);
        return diff > 0 ? "Tomorrow" : diff < 0 ? "Yesterday" : "Today";
    }
    function activate(i) {
        if (i === 0) DeskState.setDnd(!DeskState.dnd);
        else if (i === 1) { DeskState.clearHistory(); nav = 0; }
        else if (i === calRow) { const t = new Date(); month = new Date(t.getFullYear(), t.getMonth(), 1); readEvents(); }
        else if (i === calOpenRow && hasCalendar) openCalendar();   // Hearth Calendar
        else if (i >= 2) {   // a notification: its main button (or open its app); one without either goes
            const h = history[i - 2];
            if (!DeskState.invoke(h.key, "default")) DeskState.forget(h.key);
            nav = Math.min(nav, calRow - 1);
        }
    }
    function bytes(n) {
        const u = ["B", "KB", "MB", "GB", "TB"];
        let i = 0, v = Number(n) || 0;
        while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
        return (i ? v.toFixed(v < 10 ? 1 : 0) : Math.round(v)) + " " + u[i];
    }
    function left(sec) { const s = Math.round(sec); return s < 60 ? s + " s left" : s < 3600 ? Math.round(s / 60) + " min left" : Math.floor(s / 3600) + " h " + Math.round((s % 3600) / 60) + " min left"; }
    function when(d) {
        const s = Math.floor((Date.now() - d.getTime()) / 1000);
        if (s < 60) return "Just now";
        if (s < 3600) return Math.floor(s / 60) + " min ago";
        return Qt.formatTime(d, Settings.clock24 ? "HH:mm" : "h:mm AP");
    }

    visible: shown || closing.running
    Timer { id: closing; interval: Theme.calm + 60 }
    onShownChanged: { if (!shown) closing.restart(); else if (BarLayout.clockZones.length && !zoneRead.running) zoneRead.running = true; }
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
            case Qt.Key_Delete: case Qt.Key_Backspace: if (root.nav >= 2 && root.nav < root.calRow) { DeskState.forget(root.history[root.nav - 2].key); root.nav = Math.min(root.nav, root.calRow - 1); } break;
            default: return;
            }
            event.accepted = true;
        }
    }

    MouseArea { anchors.fill: parent; onClicked: root.close() }

    // Mantel 2026: the notification centre and dashboard, a wide panel under the clock pill: notifications on the left;
    // the month, the next events, what's playing, the weather and the system on the right. One column on a narrow
    // screen; above the classic taskbar's clock, as before, when that's the bar in use.
    Item {
        id: panel
        readonly property bool wide: root.width >= 1000 && !root.portraitBar
        // Under the top bar's clock; over a bottom bar's clock (bottom right); beside a left or right bar; above the
        // classic taskbar's clock, as before, when that's the bar in use.
        readonly property bool fromTop: DeskState.topInset > 0 || (!DeskState.classicBar && DeskState.barEdge !== "bottom")
        readonly property bool side: !DeskState.classicBar && (DeskState.barEdge === "left" || DeskState.barEdge === "right")
        width: wide ? Math.min(1180, root.width - 24 - DeskState.leftInset - DeskState.rightInset) : Math.min(400, root.width - 24)
        readonly property int room: root.height - 24 - (fromTop || !DeskState.classicBar ? DeskState.topInset + DeskState.bottomInset : (root.portraitBar ? 0 : root.barSize))
        height: Math.min(room, content.implicitHeight + 40)
        x: side ? (DeskState.barEdge === "left" ? DeskState.leftInset + 6 : root.width - width - DeskState.rightInset - 6)
            : fromTop ? (root.width - width) / 2 : root.portraitBar ? root.barSize + 6 : root.width - width - 12
        y: (fromTop ? Math.max(12, DeskState.topInset + 2) : root.height - height - (root.portraitBar ? 12 : (DeskState.classicBar ? root.barSize : DeskState.bottomInset) + 6)) + (root.shown ? 0 : (fromTop ? -16 : 24))
        opacity: root.shown ? 1 : 0
        Behavior on y { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        MouseArea { anchors.fill: parent }   // clicks inside don't close
        Rectangle { anchors.fill: parent; radius: Theme.radius + 8; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, Math.max(0.94, Theme.glassAlpha)); border.color: Theme.line; visible: frame.status !== Loader.Ready }
        Loader { id: frame; anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.radius + 8); } }

        GridLayout {
            id: content
            anchors { fill: parent; margins: 20 }
            columns: panel.wide ? 2 : 1
            columnSpacing: 20
            rowSpacing: 14

            // ---------------------------------------------------------------- notifications
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: panel.wide ? 480 : -1
                Layout.maximumWidth: panel.wide ? 520 : 100000
                Layout.alignment: Qt.AlignTop
                spacing: 12
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text { text: "Notifications"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                    Rectangle {
                        visible: root.history.length > 0
                        implicitWidth: cnt.implicitWidth + 14; implicitHeight: 22; radius: 11
                        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                        Text { id: cnt; anchors.centerIn: parent; text: root.history.length; color: Theme.accent; font.family: Theme.font; font.pixelSize: 12; font.bold: true }
                    }
                    Item { Layout.fillWidth: true }
                    Chip {
                        navIndex: 0
                        icon: NotifyHub.quiet ? "bell-off" : "bell"
                        label: DeskState.dnd ? "Do not disturb: on" : NotifyHub.autoReason !== "" ? "Quiet " + (NotifyHub.autoReason === "schedule" ? "until " + NotifyHub.pref("scheduleTo") : NotifyHub.autoReason === "sharing" ? "while sharing" : "in full screen") : "Do not disturb"
                        lit: DeskState.dnd
                    }
                    Chip {
                        navIndex: 1
                        visible: root.history.length > 0
                        label: "Clear all"
                    }
                }
                // ---- copies and downloads in progress (and for a few seconds once done)
                Repeater {
                    model: NotifyHub.jobs
                    delegate: Rectangle {
                        id: job
                        required property var modelData
                        readonly property var j: modelData
                        Layout.fillWidth: true
                        implicitHeight: jcol.implicitHeight + 20
                        radius: Theme.r(12)
                        color: Theme.panel2
                        border.width: 1
                        border.color: j.state === "failed" ? Theme.danger : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.5)
                        ColumnLayout {
                            id: jcol
                            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                            spacing: 4
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                AppIcon { size: 28; genericFallback: true; icon: job.j.icon || ""; appId: job.j.entry || "" }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 0
                                    Text { Layout.fillWidth: true; text: job.j.title; color: Theme.text; font.bold: true; font.pixelSize: 13; font.family: Theme.font; elide: Text.ElideRight }
                                    Text {
                                        Layout.fillWidth: true
                                        text: job.j.state === "done" ? "Finished" : job.j.state === "failed" ? (job.j.error || "It stopped") : job.j.state === "cancelled" ? "Cancelled"
                                            : [job.j.suspended ? "Paused" : "", job.j.info, job.j.detail].filter(x => x).join(" · ")
                                        color: Theme.muted; font.pixelSize: 11; font.family: Theme.font; elide: Text.ElideRight
                                    }
                                }
                                Text { visible: job.j.state === "running" && job.j.percent >= 0; text: job.j.percent + "%"; color: Theme.accent; font.bold: true; font.pixelSize: 13; font.family: Theme.font }
                                RoundButton { visible: job.j.state === "running" && job.j.suspendable; icon: job.j.suspended ? "play" : "pause"; tip: job.j.suspended ? "Resume" : "Pause"; onPicked: NotifyHub.jobAction(job.j.id, job.j.suspended ? "resume" : "suspend") }
                                RoundButton { visible: job.j.state === "running" && job.j.killable; icon: "close"; tip: "Cancel"; onPicked: NotifyHub.jobAction(job.j.id, "cancel") }
                                RoundButton { visible: job.j.state !== "running" && !!job.j.dest; icon: "folder"; tip: "Open the folder"; onPicked: Quickshell.execDetached(["xdg-open", job.j.dest]) }
                            }
                            Item {
                                Layout.fillWidth: true
                                visible: job.j.state === "running"
                                implicitHeight: 6
                                Rectangle { anchors.fill: parent; radius: 3; color: Theme.panel3 }
                                Rectangle { height: parent.height; radius: 3; width: parent.width * Math.max(0.02, Math.max(0, job.j.percent) / 100); color: job.j.suspended ? Theme.muted : Theme.accent }
                            }
                            Text {
                                visible: job.j.state === "running" && (job.j.speed > 0 || job.j.total > 0)
                                text: [job.j.unit === "bytes" && job.j.total ? root.bytes(job.j.processed) + " of " + root.bytes(job.j.total) : job.j.total ? job.j.processed + " of " + job.j.total + " files" : "",
                                       job.j.speed > 0 ? root.bytes(job.j.speed) + "/s" : "",
                                       job.j.speed > 0 && job.j.unit === "bytes" && job.j.total > job.j.processed ? root.left((job.j.total - job.j.processed) / job.j.speed) : ""].filter(x => x).join(" · ")
                                color: Theme.muted; font.pixelSize: 11; font.family: Theme.font
                            }
                        }
                    }
                }
                ListView {
                    id: list
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.min(contentHeight, Math.max(120, panel.room - 140))
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
                        readonly property var live: { void DeskState.liveRev; return DeskState.liveOf(modelData.key); }
                        readonly property var acts: live ? Array.from(live.actions) : []
                        readonly property bool opensApp: !live && !!modelData.entry && DesktopEntries.byId(modelData.entry) !== null
                        width: list.width
                        implicitHeight: crow.implicitHeight + 20
                        radius: Theme.r(12)
                        color: hover.containsMouse ? Theme.panel3 : Theme.panel2
                        border.width: 2
                        border.color: lit ? Theme.accent : "transparent"
                        MouseArea { id: hover; anchors.fill: parent; hoverEnabled: true; onClicked: DeskState.invoke(card.modelData.key, "default") }
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
                                // its progress while its app still updates it (a download, a file being sent)
                                Item {
                                    Layout.fillWidth: true
                                    Layout.topMargin: 3
                                    readonly property int v: card.live ? DeskState.hintValue(card.live) : -1
                                    visible: v >= 0
                                    implicitHeight: 5
                                    Rectangle { anchors.fill: parent; radius: 3; color: Theme.panel3 }
                                    Rectangle { height: parent.height; radius: 3; width: parent.width * parent.v / 100; color: Theme.accent }
                                }
                                // its own buttons, kept (1.14): while its app can still answer them; after a restart only Open <app>
                                Flow {
                                    Layout.fillWidth: true
                                    Layout.topMargin: 2
                                    spacing: 6
                                    visible: card.acts.length > 0 || card.opensApp
                                    Repeater {
                                        model: card.acts.filter(a => a.identifier !== "default")
                                        delegate: Rectangle {
                                            required property var modelData
                                            implicitWidth: at.implicitWidth + 18; implicitHeight: 26
                                            radius: Theme.r(8)
                                            color: aa.containsMouse ? Theme.panel3 : Theme.panel
                                            border.color: Theme.line
                                            Text { id: at; anchors.centerIn: parent; text: parent.modelData.text; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
                                            MouseArea { id: aa; anchors.fill: parent; hoverEnabled: true; onClicked: DeskState.invoke(card.modelData.key, parent.modelData.identifier) }
                                        }
                                    }
                                    Rectangle {
                                        visible: card.opensApp
                                        implicitWidth: ot.implicitWidth + 18; implicitHeight: 26
                                        radius: Theme.r(8)
                                        color: oa.containsMouse ? Theme.panel3 : Theme.panel
                                        border.color: Theme.line
                                        Text { id: ot; anchors.centerIn: parent; text: "Open " + (card.modelData.app || "the app"); color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
                                        MouseArea { id: oa; anchors.fill: parent; hoverEnabled: true; onClicked: DeskState.invoke(card.modelData.key, "default") }
                                    }
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
                    text: DeskState.dnd ? "No notifications. Do not disturb is on: new ones wait here quietly." : "No notifications"
                    wrapMode: Text.WordWrap
                    color: Theme.muted
                    font.family: Theme.font
                    font.pixelSize: 13
                }
                Item { Layout.fillHeight: true }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Muted apps and history: Settings › Notifications"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { root.close(); if (SettingsNav.settingsApp) SettingsNav.settingsApp.open("notifications"); } }
                }
            }

            // ---------------------------------------------------------------- dashboard
            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                spacing: 14
                GridLayout {
                    Layout.fillWidth: true
                    columns: panel.wide ? 2 : 1
                    columnSpacing: 14
                    rowSpacing: 10
                    Rectangle {
                        id: calCard
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignTop
                        implicitHeight: cal.implicitHeight + 20
                        radius: Theme.r(12)
                        color: Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.7)
                        border.width: 2
                        border.color: root.nav === root.calRow ? Theme.accent : "transparent"
                        MouseArea { anchors.fill: parent; onWheel: w => { if (w.angleDelta.y !== 0) root.shiftMonth(w.angleDelta.y > 0 ? -1 : 1); } }

                        ColumnLayout {
                            id: cal
                            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                            spacing: 10

                            SystemClock { id: clock; precision: SystemClock.Minutes }
                            Text {
                                text: Qt.formatDate(clock.date, "dddd, MMMM d") + (root.altOf(clock.date, "text") ? " · " + root.altOf(clock.date, "text") : "")
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
                                        readonly property string hol: root.holMap[root.ymd(day.modelData)] || ""
                                        Text {
                                            anchors.centerIn: parent
                                            anchors.verticalCenterOffset: root.alt ? -5 : 0
                                            text: day.modelData.getDate()
                                            color: day.today ? Theme.accentInk : day.hol !== "" && day.inMonth ? "#e0a23a" : day.inMonth ? Theme.text : Theme.muted
                                            opacity: day.inMonth || day.today ? 1 : 0.55
                                            font.family: Theme.font
                                            font.pixelSize: 13
                                            font.bold: day.today || day.hol !== ""
                                        }
                                        Text {   // the other calendar's day (1.14)
                                            visible: root.alt !== ""
                                            anchors { horizontalCenter: parent.horizontalCenter; verticalCenter: parent.verticalCenter; verticalCenterOffset: 9 }
                                            text: root.altOf(day.modelData, "short")
                                            color: day.today ? Theme.accentInk : Theme.muted
                                            opacity: day.inMonth ? 0.9 : 0.4
                                            font.family: Theme.font; font.pixelSize: 8
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ---- 1.14: the next holiday and your world clocks
                    ColumnLayout {
                        Layout.fillWidth: !panel.wide
                        Layout.preferredWidth: panel.wide ? 230 : -1
                        Layout.alignment: Qt.AlignTop
                        visible: root.nextHoliday !== null || BarLayout.clockZones.length > 0
                        spacing: 6
                        RowLayout {
                            visible: root.nextHoliday !== null
                            Layout.fillWidth: true
                            spacing: 10
                            Rectangle { implicitWidth: 4; implicitHeight: 30; radius: 2; color: "#e0a23a" }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Text { Layout.fillWidth: true; text: root.nextHoliday ? root.nextHoliday.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                Text { text: root.nextHoliday ? Qt.formatDate(new Date(root.nextHoliday.date + "T12:00:00"), "dddd d MMMM") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                            }
                        }
                        Repeater {
                            model: BarLayout.clockZones
                            delegate: RowLayout {
                                required property string modelData
                                Layout.fillWidth: true
                                spacing: 8
                                Icon { name: "globe"; size: 14; color: Theme.muted }
                                Text { Layout.fillWidth: true; text: modelData.split("/").pop().replace(/_/g, " "); color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                                Text { text: root.zoneDay(modelData); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                                Text { text: root.zoneTime(modelData); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                            }
                        }
                    }
                    // ---- Hearth Calendar: the next events (a week ahead) and a row that opens Calendar.
                    ColumnLayout {
                        Layout.fillWidth: !panel.wide
                        Layout.preferredWidth: panel.wide ? 230 : -1
                        Layout.alignment: Qt.AlignTop
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
                // What's playing (MPRIS; MediaCard.qml, loaded on its own).
                Loader {
                    Layout.fillWidth: true
                    Layout.preferredHeight: item && item.has ? item.implicitHeight : 0
                    active: root.shown
                    source: "MediaCard.qml"
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 14
                    // The weather (Hearth Weather's summary for your place; opens Weather).
                    Item {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 150
                        visible: !!root.weather.line
                        Rectangle { anchors.fill: parent; radius: Theme.r(16); color: Theme.panel2 }
                        ColumnLayout {
                            anchors { fill: parent; margins: 18 }
                            spacing: 6
                            Text { text: ("Weather" + (root.weather.place ? " · " + root.weather.place : "")).toUpperCase(); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 1.2 }
                            RowLayout {
                                spacing: 12
                                Icon { name: root.weather.day === false ? "moon" : "sun"; size: 40; color: Theme.flame }
                                Text { Layout.fillWidth: true; text: root.weather.line || ""; wrapMode: Text.WordWrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                            }
                            Item { Layout.fillHeight: true }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { root.close(); Quickshell.execDetached(["hearth-weather"]); } }
                    }
                    // CPU, GPU, memory, temperature (StatsCard.qml: read only while this is open).
                    Loader {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 150
                        active: root.shown
                        source: "StatsCard.qml"
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
        radius: Theme.r(10)
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
