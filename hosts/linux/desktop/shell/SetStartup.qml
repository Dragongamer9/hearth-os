// Hearth Settings › System › Startup (docs/SERVER-MODE.md): what this PC opens when it turns on. Start in one mode every
// time (Game Mode, Desktop Mode, or Server mode once it's on), or, with Server mode on, follow a schedule (weekdays and the
// weekend, each a day of changes: "Server mode from 12 AM, Game Mode from 6 PM"). No schedule unless you make one.
// Start in is session.conf's NIMBUS_START_MODE (only that line changes, through hearth_settings_io); the schedule is in
// Server mode's settings. Both through hearth-server-mode, which saves only what you change.
// The controller on a change: Left / Right move it by 30 minutes, A changes its mode, X (☰ key) removes it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Startup"
    property var st: null
    property string msg: ""
    property bool bad: false
    property var draft: null          // the schedule being edited ({weekday: [...], weekend: [...]}), saved shortly after a change
    readonly property var s: st ? st.settings : null
    readonly property bool serverOn: !!s && s.on
    readonly property bool scheduled: serverOn && !!s.schedule && s.schedule.on
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => { page.st = r; if (!saver.running && !act.busy) page.draft = JSON.parse(JSON.stringify({ weekday: r.settings.schedule.weekday, weekend: r.settings.schedule.weekend })); }, e => page.say(e, true)); }
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-server-mode" }
    SetJob { id: act; helper: "hearth-server-mode" }
    Timer { id: saver; interval: 900; onTriggered: act.run(["schedule-set"], () => page.say("Schedule saved ✓"), e => { page.say(e, true); page.refresh(); }, JSON.stringify(page.draft)) }
    function edit(day, fn) { const d = JSON.parse(JSON.stringify(draft)); fn(d[day]); d[day].sort((a, b) => a.at < b.at ? -1 : 1); draft = d; saver.restart(); }
    function mins(at) { const p = at.split(":"); return +p[0] * 60 + +p[1]; }
    function at(m) { m = ((m % 1440) + 1440) % 1440; return ("0" + Math.floor(m / 60)).slice(-2) + ":" + ("0" + (m % 60)).slice(-2); }
    function label(at) { const m = mins(at), h = Math.floor(m / 60), mm = m % 60; return ((h + 11) % 12 + 1) + (mm ? ":" + ("0" + mm).slice(-2) : "") + (h < 12 ? " AM" : " PM"); }
    readonly property var modeNames: ({ game: "Game Mode", desktop: "Desktop Mode", server: "Server mode" })
    readonly property var modeColors: ({ game: Theme.accent, desktop: Qt.rgba(0.357, 0.82, 0.545, 1), server: Qt.rgba(0.545, 0.612, 0.969, 1) })

    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 8
        text: "What this PC opens when it turns on" + (page.serverOn ? ", and when it changes mode by itself." : ".")
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetChoice {
        visible: page.serverOn
        icon: "power"; title: "When this PC turns on"
        sub: "Start in one mode every time, or follow a schedule"
        options: [{ value: "same", label: "Always the same mode" }, { value: "schedule", label: "Follow a schedule" }]
        current: page.scheduled ? "schedule" : "same"
        onChosen: v => act.run(["set", "scheduleOn", v === "schedule" ? "on" : "off"], () => { page.say(v === "schedule" ? "Following the schedule ✓" : "Starting in " + page.modeNames[page.st.startIn] + " every time"); page.refresh(); }, e => page.say(e, true))
    }

    SetHeader { text: page.scheduled ? "Modes in the schedule" : "Start in" }
    RowLayout {
        Layout.fillWidth: true
        spacing: 12
        Repeater {
            model: [{ v: "game", icon: "pad", t: "Game Mode", d: "Steam Game Mode or Hearth Game Mode (Gaming), for the TV and controllers" },
                    { v: "desktop", icon: "desktops", t: "Desktop Mode", d: "The Mantel desktop with windows, files and apps" },
                    { v: "server", icon: "server", t: "Server mode", d: "A dashboard and full power for your services; one press to play" }]
            delegate: SetBase {
                required property var modelData
                readonly property bool chosen: !page.scheduled && page.st && page.st.startIn === modelData.v
                readonly property bool usable: modelData.v !== "server" || page.serverOn
                Layout.fillWidth: true; Layout.preferredWidth: 1
                implicitHeight: 196
                interactive: !page.scheduled
                opacity: usable ? 1 : 0.7
                border.width: chosen ? 2 : 1
                border.color: chosen ? Theme.accent : SetStyle.tint(Theme.text, 0.05)
                onActivated: if (!usable) SettingsNav.openPage("hs-server"); else act.run(["start-in", modelData.v], () => { page.say("Starts in " + modelData.t + " ✓"); page.refresh(); }, e => page.say(e, true))
                Rectangle {   // a small picture of the mode
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                    height: 104; radius: 8; clip: true
                    gradient: Gradient {
                        GradientStop { position: 0; color: modelData.v === "game" ? "#1a3a56" : modelData.v === "desktop" ? "#1f5f94" : "#0c121a" }
                        GradientStop { position: 1; color: modelData.v === "game" ? "#0c1520" : modelData.v === "desktop" ? "#0d141e" : "#141c26" } }
                    Row { visible: modelData.v === "game"; anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 10 } spacing: 6
                        Repeater { model: ["#c86b3c", "#4a7fbf", "#7c5bb6", "#3f9a7a", "#b8a04a"]; delegate: Rectangle { required property var modelData; width: 34; height: 52; radius: 5; color: modelData } } }
                    Rectangle { visible: modelData.v === "desktop"; x: 30; y: 22; width: 140; height: 66; radius: 6; color: "#151b24"; border.color: "#2c3846" }
                    Grid { visible: modelData.v === "server"; anchors { fill: parent; margins: 10 } columns: 4; spacing: 5
                        Repeater { model: 8; delegate: Rectangle { required property int index; width: 52; height: 36; radius: 4; color: "#1f2834"
                            Rectangle { x: 5; y: 5; width: 7; height: 7; radius: 4; color: index === 4 ? SetStyle.warn : SetStyle.ok } } } }
                }
                Row {
                    anchors { left: parent.left; top: parent.top; leftMargin: 14; topMargin: 124 }
                    spacing: 8
                    Icon { name: modelData.icon; size: 20; color: Theme.text }
                    Text { text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                    Text { visible: parent.parent.chosen; text: "· Start in"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14 }
                    Text { visible: !parent.parent.usable; text: "· turn it on in Home server"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                }
                Text { anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 14; rightMargin: 14; topMargin: 152 }
                    text: usable ? modelData.d : "Off. Press to turn it on in Home server › Server mode"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
            }
        }
    }

    SetHeader { visible: page.scheduled; text: "Schedule" }
    Repeater {
        model: page.scheduled && page.draft ? [{ day: "weekday", t: "Monday – Friday" }, { day: "weekend", t: "Saturday – Sunday" }] : []
        delegate: ColumnLayout {
            id: dayCol
            required property var modelData
            readonly property var items: page.draft[modelData.day] || []
            Layout.fillWidth: true
            spacing: 6
            // The day as a 24-hour track, a block per mode.
            Rectangle {
                Layout.fillWidth: true; implicitHeight: 74; radius: SetStyle.radius; color: SetStyle.card
                Text { x: 16; y: 10; text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Item {
                    id: track
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 16; rightMargin: 16; bottomMargin: 10 }
                    height: 30
                    Repeater {
                        model: dayCol.items.length + 1
                        delegate: Rectangle {
                            required property int index
                            readonly property var its: dayCol.items
                            readonly property int from: index === 0 ? 0 : page.mins(its[index - 1].at)
                            readonly property int to: index < its.length ? page.mins(its[index].at) : 1440
                            readonly property string mode: index === 0 ? its[its.length - 1].mode : its[index - 1].mode
                            visible: to > from
                            x: track.width * from / 1440; width: Math.max(2, track.width * (to - from) / 1440 - 2); height: track.height; radius: 6
                            color: Qt.rgba(page.modeColors[mode].r, page.modeColors[mode].g, page.modeColors[mode].b, 0.35)
                            Text { anchors { left: parent.left; leftMargin: 8; verticalCenter: parent.verticalCenter } visible: parent.width > 150
                                text: page.modeNames[parent.mode] + " · " + page.label(page.at(parent.from)) + " – " + page.label(page.at(parent.to)); color: Theme.text; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight; width: parent.width - 12 }
                        }
                    }
                }
            }
            // Each change on its own row: Left / Right move it, A changes the mode after it, X removes it.
            Repeater {
                model: dayCol.items
                delegate: SetBase {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true; inset: false
                    usesLeftRight: true
                    implicitHeight: 54
                    onAdjusted: step => page.edit(dayCol.modelData.day, list => { list[index].at = page.at(page.mins(list[index].at) + step * 30); })
                    onActivated: page.edit(dayCol.modelData.day, list => { const order = ["server", "game", "desktop"]; list[index].mode = order[(order.indexOf(list[index].mode) + 1) % 3]; })
                    Keys.onMenuPressed: e => { if (dayCol.items.length > 1) page.edit(dayCol.modelData.day, list => list.splice(index, 1)); e.accepted = true; }
                    Row { anchors { left: parent.left; leftMargin: 18; verticalCenter: parent.verticalCenter } spacing: 12
                        Rectangle { width: 12; height: 12; radius: 6; color: page.modeColors[modelData.mode]; anchors.verticalCenter: parent.verticalCenter }
                        Text { text: "From " + page.label(modelData.at); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                        Text { text: page.modeNames[modelData.mode]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 } }
                    Text { anchors { right: parent.right; rightMargin: 18; verticalCenter: parent.verticalCenter }
                        text: "← → move · A mode · X remove"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
            }
            SetRow {
                visible: dayCol.items.length < 8
                icon: "plus"; title: "Add a change"; sub: "A new change an hour after the last one"
                onActivated: page.edit(dayCol.modelData.day, list => { const last = list.length ? page.mins(list[list.length - 1].at) : 0; let m = Math.min(1410, last + 60); while (list.some(c => c.at === page.at(m))) m = (m + 30) % 1440; list.push({ at: page.at(m), mode: "game" }); })
            }
        }
    }

    SetHeader { visible: page.scheduled; text: "When it's time to change" }
    SetRow { visible: page.scheduled; icon: "pad"; chevron: false; interactive: false; title: "If a game is running"; sub: "Server mode never closes your game: the change waits until it ends"; value: "Wait until the game ends" }
    SetRow { visible: page.scheduled; icon: "desktops"; chevron: false; interactive: false; title: "If you're in Desktop Mode"; sub: "Unsaved work stays open: the schedule never switches away from Desktop Mode"; value: "Stay" }
    SetRow { visible: page.scheduled; icon: "server"; chevron: false; interactive: false; title: "In Server mode"; sub: "The dashboard counts down 10 seconds first; B cancels"; value: "10-second countdown" }

    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
