// Firehold on Plasma's notification list (the shared org.hearthos.notifications, style "firehold"; mockup
// docs/mockups/firehold-plasma/10-notification-history.png): opened from the clock or the bell, under the bar at the
// right. The time and the calendar; Do not disturb with its schedule and what always gets through; the list (Hearth's
// own history, the one Mantel keeps) with filter chips, "While you played", grouped Today / Yesterday, an action on
// each; right-click one to mute its app for an hour or for good. Shows the last 7 days.
import QtQuick
import QtQuick.Layouts
import "fh.js" as F

Item {
    id: fh
    property bool shown: false
    property string filter: "all"
    readonly property string fontName: F.font(Qt.fontFamilies())
    function toggle() { if (shown) close(); else open(); }
    function open() { FhState.askDnd(); DeskState.unseen = 0; shown = true; forceActiveFocus(); }
    function close() { shown = false; menu.visible = false; }
    Keys.onEscapePressed: close()
    SystemClock { id: clock; precision: SystemClock.Minutes }

    // ---- the list: the last 7 days, sorted into Hearth / Games / Apps, the same app in a row grouped
    readonly property var week: { void DeskState.history; const lim = Date.now() - 7 * 86400000; return DeskState.history.filter(h => (h.time instanceof Date ? h.time.getTime() : Number(h.time)) >= lim); }
    function kind(h) {
        if (NotifyRules.isHearth(h.app)) return "hearth";
        if (/steam|heroic|lutris|beacon|game|downloads|proton|bottles|itch|gog/i.test((h.app || "") + " " + (h.entry || "") + " " + (h.category || ""))) return "games";
        return "apps";
    }
    function count(k) { return week.filter(h => k === "all" || kind(h) === k).length; }
    readonly property var rows: {
        const out = [];
        const list = week.filter(h => filter === "all" || kind(h) === filter);
        for (const h of list) {
            const day = F.dayOf(h.time);
            const last = out.length ? out[out.length - 1] : null;
            if (last && last.day === day && last.app === h.app && h.urgency !== "critical" && last.items[0].urgency !== "critical") last.items.push(h);
            else out.push({ day: day, app: h.app, items: [h] });
        }
        return out;
    }
    function rowTitle(r) {
        const h = r.items[0];
        if (r.items.length > 1) return r.app + " · " + r.items.length;
        return h.urgency === "critical" ? h.summary : (r.app || h.summary);
    }
    function rowSub(r) {
        const h = r.items[0];
        if (r.items.length > 1) {
            const sums = r.items.map(x => x.summary).filter((x, i, a) => x && a.indexOf(x) === i);
            return sums.length > 1 ? sums.join(" · ") : (h.body || h.summary);
        }
        if (h.urgency === "critical") return r.app + (h.body ? " · " + h.body : "");
        return h.summary + (h.body ? " · " + h.body : "");
    }
    // A row's button: the notification's own first one while its app still has it, else Open <app>.
    function rowAction(r) {
        const h = r.items[0], n = DeskState.liveOf(h.key);
        const acts = n ? Array.from(n.actions || []).filter(a => a.text && a.identifier !== "default") : [];
        if (acts.length) return { id: acts[0].identifier, text: acts[0].text };
        if (n && Array.from(n.actions || []).some(a => a.identifier === "default")) return { id: "default", text: "Open" };
        if (h.entry && DesktopEntries.byId(h.entry)) return { id: "default", text: "Open" };
        return null;
    }

    MouseArea { anchors.fill: parent; onClicked: fh.close() }

    Rectangle {
        id: box
        width: Math.min(parent.width - 28, 1064)
        x: parent.width - width - 14
        y: (Hearth.panelEdge === 0 ? Hearth.panelSize : 0) + 9
        height: Math.min(parent.height - y - 14, Math.max(leftCol.implicitHeight, rightCol.implicitHeight) + 48)
        radius: 18
        color: F.panel
        border.color: F.line
        clip: true
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.RightButton; onClicked: menu.visible = false }

        // ---------------------------------------------- the time, the calendar, Do not disturb
        Rectangle { x: 372; width: 1; height: parent.height; color: F.line }
        ColumnLayout {
            id: leftCol
            x: 24; y: 26; width: 324
            spacing: 4
            Text { text: Qt.formatTime(clock.date, "h:mm AP"); color: F.text; font.pixelSize: 36; font.weight: Font.Bold; font.family: fh.fontName }
            Text { text: Qt.formatDate(clock.date, "dddd, MMMM d, yyyy"); color: F.muted; font.pixelSize: 16; font.family: fh.fontName }
            Text { Layout.topMargin: 18; text: Qt.formatDate(clock.date, "MMMM yyyy"); color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fh.fontName }
            Grid {
                id: cal
                Layout.topMargin: 6
                columns: 7
                rowSpacing: 4
                readonly property real cw: 324 / 7
                readonly property var first: new Date(clock.date.getFullYear(), clock.date.getMonth(), 1)
                readonly property int days: new Date(clock.date.getFullYear(), clock.date.getMonth() + 1, 0).getDate()
                Repeater {
                    model: ["S", "M", "T", "W", "T", "F", "S"]
                    delegate: Item { required property var modelData; width: cal.cw; height: 24
                        Text { anchors.centerIn: parent; text: parent.modelData; color: F.muted; font.pixelSize: 13; font.family: fh.fontName } }
                }
                Repeater {
                    model: cal.first.getDay() + cal.days
                    delegate: Item {
                        required property int index
                        readonly property int day: index - cal.first.getDay() + 1
                        readonly property bool today: day === clock.date.getDate()
                        width: cal.cw; height: 31
                        Rectangle { anchors.centerIn: parent; visible: parent.day > 0 && parent.today; width: 44; height: 30; radius: 7; color: F.accent }
                        Text { anchors.centerIn: parent; visible: parent.day > 0; text: parent.day; color: parent.today ? "#ffffff" : F.text; font.pixelSize: 15; font.weight: parent.today ? Font.Bold : Font.Normal; font.family: fh.fontName }
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true; Layout.topMargin: 18
                implicitHeight: dcol.implicitHeight + 32
                radius: 12; color: F.tile
                ColumnLayout {
                    id: dcol
                    x: 16; y: 16; width: parent.width - 32
                    spacing: 6
                    RowLayout {
                        Layout.fillWidth: true
                        Text { Layout.fillWidth: true; text: "Do not disturb"; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fh.fontName }
                        Rectangle {
                            width: 50; height: 28; radius: 14
                            color: FhState.dnd.on ? F.accent : "#4a413c"
                            Rectangle { x: FhState.dnd.on ? parent.width - width - 3 : 3; anchors.verticalCenter: parent.verticalCenter; width: 22; height: 22; radius: 11; color: "#ffffff"
                                Behavior on x { NumberAnimation { duration: 120 } } }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: FhState.toggleDnd() }
                        }
                    }
                    Text { Layout.topMargin: 6; visible: FhState.dnd.schedule !== false; text: "Quiet " + FhState.dnd.from + " – " + FhState.dnd.to; color: F.muted; font.pixelSize: 14; font.family: fh.fontName }
                    Text { visible: FhState.dnd.games !== false; text: (FhState.dnd.schedule !== false ? "and " : "") + "while a game is full screen"; color: F.muted; font.pixelSize: 14; font.family: fh.fontName }
                    Text { Layout.fillWidth: true; Layout.topMargin: 6; text: "Always lets through: " + (FhState.dnd.family !== false ? "Hearth Link family, " : "") + "alarms"; color: "#ddd2ca"; font.pixelSize: 14; font.family: fh.fontName; wrapMode: Text.Wrap }
                    Text { Layout.topMargin: 4; text: "Change ›"; color: F.accent; font.pixelSize: 14; font.weight: Font.DemiBold; font.family: fh.fontName
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { fh.close(); SettingsNav.openPage("notifications"); } } }
                }
            }
        }

        // ---------------------------------------------- the list
        ColumnLayout {
            id: rightCol
            x: 396; y: 26; width: box.width - 396 - 24
            spacing: 10
            RowLayout {
                Layout.fillWidth: true
                Text { Layout.fillWidth: true; text: "Notifications"; color: F.text; font.pixelSize: 24; font.weight: Font.DemiBold; font.family: fh.fontName }
                FhButton { text: "Clear all"; onPressed: { DeskState.clearHistory(); FhState.seen(); } }
                FhButton { glyph: "gear"; text: ""; pad: 10; onPressed: { fh.close(); SettingsNav.openPage("notifications"); } }
            }
            Row {
                spacing: 8
                Repeater {
                    model: [{ id: "all", t: "All" }, { id: "hearth", t: "Hearth" }, { id: "games", t: "Games" }, { id: "apps", t: "Apps" }]
                    delegate: Rectangle {
                        required property var modelData
                        readonly property bool sel: fh.filter === modelData.id
                        visible: modelData.id === "all" || fh.count(modelData.id) > 0
                        width: chipT.implicitWidth + 28; height: 34; radius: 9
                        color: sel ? F.accent : F.seg
                        Text { id: chipT; anchors.centerIn: parent; text: parent.modelData.t + " · " + fh.count(parent.modelData.id); color: "#ffffff"; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: fh.fontName }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: fh.filter = parent.modelData.id }
                    }
                }
            }
            // While you played
            Rectangle {
                Layout.fillWidth: true
                visible: !!FhState.played
                implicitHeight: playT.implicitHeight + 22
                radius: 10; color: "#3a2518"
                Text {
                    id: playT
                    anchors { left: parent.left; leftMargin: 14; right: seenT.left; rightMargin: 14; verticalCenter: parent.verticalCenter }
                    textFormat: Text.StyledText; wrapMode: Text.Wrap
                    text: FhState.played ? "<b>While you played</b> · " + FhState.played.game + ", " + FhState.played.minutes + " min · " + FhState.played.count + " held back so they didn't cover the game" : ""
                    color: "#ddd2ca"; font.pixelSize: 14; font.family: fh.fontName
                }
                Text { id: seenT; anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter } text: "Seen"; color: F.accent; font.pixelSize: 15; font.weight: Font.DemiBold; font.family: fh.fontName
                    MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: FhState.seen() } }
            }
            Text { visible: fh.rows.length === 0; Layout.topMargin: 20; text: "No notifications"; color: F.muted; font.pixelSize: 15; font.family: fh.fontName }
            Repeater {
                model: fh.rows
                delegate: ColumnLayout {
                    id: rowC
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    spacing: 8
                    readonly property var act: fh.rowAction(modelData)
                    Text { visible: rowC.index === 0 || fh.rows[rowC.index - 1].day !== rowC.modelData.day; Layout.topMargin: rowC.index === 0 ? 4 : 8
                        text: rowC.modelData.day.toUpperCase(); color: F.muted; font.pixelSize: 14; font.weight: Font.DemiBold; font.letterSpacing: 0.6; font.family: fh.fontName }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 62; radius: 10
                        color: rma.containsMouse ? "#302824" : F.tile
                        MouseArea {
                            id: rma; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => {
                                if (mouse.button === Qt.RightButton) { menu.open(rowC.modelData, mapToItem(box, mouse.x, mouse.y)); return; }
                                menu.visible = false;
                            }
                        }
                        Rectangle {
                            id: rt
                            x: 14; anchors.verticalCenter: parent.verticalCenter
                            width: 36; height: 36; radius: 9
                            color: F.tint(rowC.modelData.app)
                            AppIcon { id: rai; anchors.centerIn: parent; size: 22; icon: rowC.modelData.items[0].icon || ""; appId: rowC.modelData.items[0].entry || ""; genericFallback: false; visible: hasIcon }
                            Text { anchors.centerIn: parent; visible: !rai.visible; text: F.initial(rowC.modelData.items[0].urgency === "critical" ? rowC.modelData.items[0].summary : (rowC.modelData.app || rowC.modelData.items[0].summary)); color: "#ffffff"; font.pixelSize: 16; font.weight: Font.Bold; font.family: fh.fontName }
                        }
                        ColumnLayout {
                            anchors { left: rt.right; leftMargin: 14; right: when.left; rightMargin: 12; verticalCenter: parent.verticalCenter }
                            spacing: 2
                            Text { Layout.fillWidth: true; text: fh.rowTitle(rowC.modelData); color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: fh.fontName; elide: Text.ElideRight }
                            Text { Layout.fillWidth: true; text: fh.rowSub(rowC.modelData); color: "#ddd2ca"; font.pixelSize: 14; font.family: fh.fontName; elide: Text.ElideRight }
                        }
                        Text {
                            id: when
                            anchors { right: actB.visible ? actB.left : parent.right; rightMargin: actB.visible ? 10 : 14; top: parent.top; topMargin: 12 }
                            text: F.ago(rowC.modelData.items[0].time, true); color: F.muted; font.pixelSize: 13; font.family: fh.fontName
                        }
                        FhButton {
                            id: actB
                            visible: !!rowC.act
                            anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                            text: rowC.act ? rowC.act.text : ""
                            onPressed: { DeskState.invoke(rowC.modelData.items[0].key, rowC.act.id); }
                        }
                    }
                }
            }
            Text { Layout.fillWidth: true; Layout.topMargin: 6; text: "Right-click or press X on one: mute this app for 1 hour, or for good · Kept for 7 days"; color: F.muted; font.pixelSize: 14; font.family: fh.fontName; elide: Text.ElideRight }
        }

        // Right-click (or X): mute its app for an hour or for good, or remove it
        Rectangle {
            id: menu
            visible: false
            z: 9
            property var row: null
            function open(r, p) { row = r; x = Math.min(box.width - width - 8, p.x); y = Math.min(box.height - height - 8, p.y); visible = true; }
            width: 250; height: mcol.implicitHeight + 12; radius: 10
            color: F.tile; border.color: F.line
            Column {
                id: mcol
                x: 6; y: 6; width: parent.width - 12
                Repeater {
                    model: menu.row ? [{ t: "Mute " + menu.row.app + " for 1 hour", w: "hour" }, { t: "Mute " + menu.row.app, w: "good" }, { t: "Remove", w: "remove" }] : []
                    delegate: Rectangle {
                        required property var modelData
                        width: mcol.width; height: 34; radius: 7
                        color: mma.containsMouse ? F.seg : "transparent"
                        Text { x: 10; anchors.verticalCenter: parent.verticalCenter; text: parent.modelData.t; color: F.text; font.pixelSize: 14; font.family: fh.fontName }
                        MouseArea {
                            id: mma; anchors.fill: parent; hoverEnabled: true
                            onClicked: {
                                const r = menu.row, w = parent.modelData.w;
                                menu.visible = false;
                                if (w === "hour") FhState.muteHour(r.app);
                                else if (w === "good") DeskState.setMuted(r.app, true);
                                else for (const h of r.items) DeskState.forget(h.key, true);
                            }
                        }
                    }
                }
            }
        }
    }
}
