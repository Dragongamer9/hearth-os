// Task Manager: what's running in Desktop Mode and how hard the device is working. A normal window like
// Hearth Settings, with the same controls: Up/Down pick a tab, Right/A go into it, Left/B come back, B on
// the tab list closes. In Apps, A on an app offers End and Go to window.
// The numbers come from hearth-tasks (next to this file), only while the window is open.
// Opened from the app launcher, or:
//   quickshell ipc -p <shell folder> call tasks open [tab]      (tabs: apps, perf, startup)
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets

Scope {
    id: tm
    property bool shown: false
    property string page: "apps"
    readonly property var pages: [
        { id: "apps", icon: "grid", label: "Apps" },
        { id: "perf", icon: "motion", label: "Performance" },
        { id: "startup", icon: "restart", label: "Startup and background" }
    ]
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-tasks").toString().replace(/^file:\/\//, ""))

    function open(p) {
        if (p && pages.some(x => x.id === p)) page = p;
        shown = true;
    }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(""); }

    // ------------------------------------------------ friendly numbers
    readonly property var units: ["B", "KB", "MB", "GB", "TB"]
    function unitOf(b) {
        let i = 0;
        let v = b || 0;
        while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
        return i;
    }
    function inUnit(b, i) {
        const v = (b || 0) / Math.pow(1024, i);
        return i === 0 || v >= 100 ? String(Math.round(v)) : v.toFixed(1);
    }
    function size(b) {
        if (b === undefined || b === null) return "–";
        const i = unitOf(b);
        return inUnit(b, i) + " " + units[i];
    }
    function sizeOf(used, total) {
        if (!total) return "–";
        const i = unitOf(total);
        return inUnit(used, i) + " of " + inUnit(total, i) + " " + units[i];
    }
    function pct(v) { return v === undefined || v === null ? "–" : Math.round(v) + "%"; }
    function temp(c) { return c === undefined || c === null ? "–" : Math.round(c) + " °C"; }
    function hm(hours) {
        if (!hours || hours <= 0) return "";
        const h = Math.floor(hours);
        const m = Math.round((hours - h) * 60);
        return h ? `${h} h ${m} min` : `${m} min`;
    }
    function pushed(list, v) {
        const a = list.slice(-59);
        a.push(v === undefined || v === null ? 0 : v);
        return a;
    }
    // Updates a ListModel in place (by "key"), so rows keep their place and focus while numbers change.
    function sync(model, items) {
        for (let i = 0; i < items.length; i++) {
            let j = -1;
            for (let k = i; k < model.count; k++) {
                if (model.get(k).key === items[i].key) { j = k; break; }
            }
            if (j < 0) {
                model.insert(i, items[i]);
            } else {
                if (j !== i) model.move(j, i, 1);
                model.set(i, items[i]);
            }
        }
        if (model.count > items.length) model.remove(items.length, model.count - items.length);
    }

    LazyLoader {
        active: tm.shown
        FloatingWindow {
            id: win
            title: "Task Manager"
            implicitWidth: 1100
            implicitHeight: 720
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) tm.shown = false
            Component.onCompleted: nav.forceActiveFocus()

            // ------------------------------------------------ what hearth-tasks says
            property var groups: ({})          // key → app group from `hearth-tasks list`
            property var totals: ({ cpu: 0, mem: { total: 0, used: 0 }, apps: 0 })
            property var perf: null            // the latest `hearth-tasks perf`
            property var cpuHist: []
            property var memHist: []
            property var gpuHist: []           // one list per graphics card
            property string listError: ""
            property string startupError: ""
            property bool startupLoaded: false

            // The A-button sheet in Apps: menu → ending → stuck (offer Force end), or protected / error.
            property string sheetKey: ""
            property string sheetMode: ""
            property string sheetTitle: ""
            property string sheetText: ""
            property var sheetButtons: []
            property int sheetIndex: 0
            property var endPids: []
            property bool checkEnd: false

            function run(proc, args) {
                if (proc.running) return false;
                proc.command = ["python3", tm.helper].concat(args);
                proc.running = true;
                return true;
            }
            function lastLine(text) { return text.trim().split("\n").pop() || ""; }

            function applyList(r) {
                const map = {};
                const rank = { app: 0, game: 0, background: 1, system: 2 };
                const label = { app: "Apps", game: "Apps", background: "Background", system: "Part of Hearth (can't be ended)" };
                const list = (r.apps || []).slice().sort((a, b) => (rank[a.kind] - rank[b.kind]) || (b.mem - a.mem));
                const items = list.map(g => {
                    map[g.id] = g;
                    let sub = "";
                    if (g.protected) sub = g.why || "Keeps Hearth running";
                    else if (g.kind === "game") sub = g.count > 1 ? "Game · " + g.count + " processes" : "Game";
                    else if (g.count > 1) sub = g.count + " processes";
                    return {
                        key: g.id, pid: g.pid, name: g.name || String(g.pid), icon: g.icon || "", sub: sub,
                        cpu: g.cpu || 0, mem: g.mem || 0, section: label[g.kind] || "Background", locked: !!g.protected
                    };
                });
                groups = map;
                totals = { cpu: r.cpu || 0, mem: r.mem || { total: 0, used: 0 }, apps: list.filter(g => g.kind === "app" || g.kind === "game").length };
                tm.sync(appModel, items);
                if (checkEnd) {
                    checkEnd = false;
                    const alive = list.some(g => g.pids.some(p => endPids.indexOf(p) >= 0));
                    if (sheetMode === "ending") {
                        if (alive) showSheet("stuck", sheetTitle, sheetTitle + " hasn't closed yet. You can wait a little longer, or force it to close (anything not saved will be lost).",
                            [{ label: "Force end", action: "kill", danger: true }, { label: "Wait", action: "wait" }]);
                        else closeSheet();
                    }
                }
            }
            function applyPerf(r) {
                perf = r;
                cpuHist = tm.pushed(cpuHist, r.cpu ? r.cpu.total : 0);
                memHist = tm.pushed(memHist, r.mem && r.mem.total ? 100 * r.mem.used / r.mem.total : 0);
                const g = [];
                (r.gpus || []).forEach((x, i) => g.push(tm.pushed(gpuHist[i] || [], x.busy)));
                gpuHist = g;
            }
            function applyStartup(r) {
                const rows = k => (r.items || []).filter(i => i.kind === k).map(i => ({ key: i.id, name: i.name, sub: i.sub || "", active: !!i.on, canChange: !!i.canChange }));
                tm.sync(hearthModel, rows("hearth"));
                tm.sync(autoModel, rows("autostart"));
                tm.sync(serviceModel, rows("service"));
                startupLoaded = true;
            }
            function setStartup(model, index, on) {
                const row = model.get(index);
                startupError = "";
                if (!run(setProc, ["startup-set", row.key, on ? "on" : "off"])) return;
                model.setProperty(index, "active", on);
            }

            // ------------------------------------------------ the A-button sheet
            function showSheet(mode, t, text, buttons) {
                sheetMode = mode;
                sheetTitle = t;
                sheetText = text;
                sheetButtons = buttons;
                sheetIndex = 0;
            }
            function closeSheet() {
                sheetMode = "";
                sheetKey = "";
                endTimer.stop();
                checkEnd = false;
            }
            function windowFor(g) {
                if (!g) return null;
                const ids = g.appIds || [];
                const desk = (g.desktop || "").toLowerCase();
                const all = ToplevelManager.toplevels.values;
                for (let i = 0; i < all.length; i++) {
                    const t = all[i];
                    const a = (t.appId || "").toLowerCase();
                    if (!a || t.title === "Task Manager" || t.title === "Hearth Settings") continue;
                    if (ids.indexOf(a) >= 0 || (desk && a === desk)) return t;
                    const e = DesktopEntries.heuristicLookup(t.appId);
                    if (e && desk && (e.id || "").toLowerCase() === desk) return t;
                }
                return null;
            }
            function openActions(key) {
                const g = groups[key];
                if (!g) return;
                sheetKey = key;
                if (g.protected) {
                    showSheet("protected", g.name, (g.why || "This") + " keeps Hearth running, so it can't be ended here.", [{ label: "OK", action: "close" }]);
                    return;
                }
                const b = [{ label: "End", action: "end", danger: true }];
                if (windowFor(g)) b.push({ label: "Go to window", action: "window" });
                b.push({ label: "Cancel", action: "close" });
                const parts = [tm.pct(g.cpu) + " processor", tm.size(g.mem) + " memory"];
                if (g.count > 1) parts.push(g.count + " processes");
                showSheet("menu", g.name, parts.join(" · "), b);
            }
            function sheetAction(action) {
                const g = groups[sheetKey];
                switch (action) {
                case "close":
                    closeSheet();
                    break;
                case "window": {
                    const t = windowFor(g);
                    closeSheet();
                    if (t) { t.minimized = false; t.activate(); }
                    break;
                }
                case "end":
                    if (g && run(actProc, ["end", String(g.pid)])) {
                        endPids = g.pids;
                        showSheet("ending", g.name, "Asking " + g.name + " to close…", [{ label: "Hide", action: "close" }]);
                    }
                    break;
                case "wait":
                    showSheet("ending", sheetTitle, "Waiting for " + sheetTitle + " to close…", [{ label: "Force end", action: "kill", danger: true }, { label: "Hide", action: "close" }]);
                    endTimer.restart();
                    break;
                case "kill":
                    if (g && run(actProc, ["kill", String(g.pid)])) closeSheet();
                    else if (!g) closeSheet();
                    break;
                }
            }

            Item {
                visible: false
                ListModel { id: appModel }
                ListModel { id: hearthModel }
                ListModel { id: autoModel }
                ListModel { id: serviceModel }

                Process {
                    id: listProc
                    stdout: StdioCollector {
                        onStreamFinished: {
                            const line = win.lastLine(text);
                            if (line.startsWith("error: ")) { win.listError = line.slice(7); return; }
                            try { win.applyList(JSON.parse(line)); win.listError = ""; } catch (e) {}
                        }
                    }
                }
                Process {
                    id: perfProc
                    stdout: StdioCollector {
                        onStreamFinished: {
                            const line = win.lastLine(text);
                            try { if (!line.startsWith("error: ")) win.applyPerf(JSON.parse(line)); } catch (e) {}
                        }
                    }
                }
                Process {
                    id: startProc
                    stdout: StdioCollector {
                        onStreamFinished: {
                            const line = win.lastLine(text);
                            if (line.startsWith("error: ")) { win.startupError = line.slice(7); return; }
                            try { win.applyStartup(JSON.parse(line)); } catch (e) {}
                        }
                    }
                }
                Process {
                    id: setProc
                    stdout: StdioCollector {
                        onStreamFinished: {
                            const line = win.lastLine(text);
                            if (line.startsWith("error: ")) win.startupError = line.slice(7);
                            startReload.restart();
                        }
                    }
                }
                Process {
                    id: actProc
                    stdout: StdioCollector {
                        onStreamFinished: {
                            const line = win.lastLine(text);
                            if (line.startsWith("error: ")) {
                                win.showSheet("error", win.sheetTitle || "Task Manager", line.slice(7).replace(/^./, c => c.toUpperCase()) + ".", [{ label: "OK", action: "close" }]);
                                return;
                            }
                            if (win.sheetMode === "ending") endTimer.restart();
                            listTimer.restart();
                        }
                    }
                }
                // Apps every 2 s while that tab is shown; performance every second (the graphs show the last minute).
                Timer {
                    id: listTimer
                    interval: 2000
                    repeat: true
                    triggeredOnStart: true
                    running: tm.page === "apps"
                    onTriggered: win.run(listProc, ["list"])
                }
                Timer {
                    interval: 1000
                    repeat: true
                    triggeredOnStart: true
                    running: true
                    onTriggered: win.run(perfProc, ["perf"])
                }
                Timer {
                    id: endTimer
                    interval: 3000
                    onTriggered: {
                        win.checkEnd = true;
                        if (!win.run(listProc, ["list"])) retry.start();
                    }
                }
                Timer { id: retry; interval: 300; onTriggered: if (!win.run(listProc, ["list"])) retry.start() }
                Timer { id: startReload; interval: 400; onTriggered: win.run(startProc, ["startup"]) }
                Connections {
                    target: tm
                    function onPageChanged() {
                        if (tm.page !== "apps") win.closeSheet();
                        if (tm.page === "startup") win.run(startProc, ["startup"]);
                    }
                }
                Component.onCompleted: if (tm.page === "startup") win.run(startProc, ["startup"])
            }

            RowLayout {
                anchors.fill: parent
                spacing: 0

                // ------------------------------------------------ tab list
                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: win.width < 900 ? 220 : 270
                    color: Theme.panel
                    ColumnLayout {
                        anchors { fill: parent; margins: 14 }
                        spacing: 6
                        RowLayout {
                            Layout.bottomMargin: 10; Layout.leftMargin: 6; Layout.topMargin: 6
                            spacing: 10
                            Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 30; Layout.preferredHeight: 30; fillMode: Image.PreserveAspectFit; smooth: true }
                            Text { text: "Task Manager"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                        }
                        ListView {
                            id: nav
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: tm.pages
                            spacing: 4
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            currentIndex: Math.max(0, tm.pages.findIndex(p => p.id === tm.page))
                            highlightMoveDuration: Theme.quick
                            keyNavigationEnabled: false
                            function enter() { if (loader.item && loader.item.focusFirst) loader.item.focusFirst(); }
                            Keys.onPressed: e => {
                                switch (e.key) {
                                case Qt.Key_Down: case Qt.Key_Tab: tm.page = tm.pages[Math.min(tm.pages.length - 1, currentIndex + 1)].id; break;
                                case Qt.Key_Up: case Qt.Key_Backtab: tm.page = tm.pages[Math.max(0, currentIndex - 1)].id; break;
                                case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: enter(); break;
                                case Qt.Key_Escape: tm.close(); break;
                                default: return;
                                }
                                e.accepted = true;
                            }
                            delegate: Rectangle {
                                id: tab
                                required property var modelData
                                required property int index
                                readonly property bool lit: tm.page === modelData.id
                                width: ListView.view.width
                                height: 48
                                radius: 10
                                color: lit ? (nav.activeFocus ? Theme.accent : Theme.panel3) : tabMouse.containsMouse ? Theme.panel2 : "transparent"
                                Behavior on color { ColorAnimation { duration: Theme.quick } }
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                                    spacing: 12
                                    Icon { name: tab.modelData.icon; size: 20; color: tab.lit && nav.activeFocus ? Theme.accentInk : tab.lit ? Theme.accent : Theme.text }
                                    Text {
                                        Layout.fillWidth: true
                                        text: tab.modelData.label
                                        elide: Text.ElideRight
                                        color: tab.lit && nav.activeFocus ? Theme.accentInk : Theme.text
                                        font.family: Theme.font; font.pixelSize: 15; font.weight: tab.lit ? Font.DemiBold : Font.Normal
                                    }
                                }
                                MouseArea {
                                    id: tabMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: { tm.page = tab.modelData.id; nav.forceActiveFocus(); }
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.leftMargin: 6
                            text: "Ⓐ open  Ⓑ back"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }
                    }
                }
                Rectangle { Layout.fillHeight: true; Layout.preferredWidth: 1; color: Theme.line }

                // ------------------------------------------------ tab
                Loader {
                    id: loader
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    function toNav() { nav.forceActiveFocus(); }
                    sourceComponent: tm.page === "perf" ? perfPage : tm.page === "startup" ? startupPage : appsPage
                }
            }

            // The tabs (loaded one at a time by the Loader above).
            Item {
                visible: false

                // ================================================ Apps
                Component {
                    id: appsPage
                    Item {
                        id: apps
                        function focusFirst() { list.forceActiveFocus(); }
                        readonly property bool sheetOpen: win.sheetMode !== ""
                        onSheetOpenChanged: {
                            if (sheetOpen) sheetCard.forceActiveFocus();
                            else if (list.visible) list.forceActiveFocus();
                        }

                        ColumnLayout {
                            anchors { fill: parent; leftMargin: 24; rightMargin: 24; topMargin: 20; bottomMargin: 12 }
                            spacing: 8
                            Text { text: "Apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                            Text {
                                Layout.fillWidth: true
                                text: win.listError !== "" ? win.listError
                                    : appModel.count === 0 ? "Looking at what's running…"
                                    : (win.totals.apps === 1 ? "1 app" : win.totals.apps + " apps") + " running · Processor " + tm.pct(win.totals.cpu)
                                      + " · Memory " + tm.sizeOf(win.totals.mem.used, win.totals.mem.total)
                                color: win.listError !== "" ? Theme.danger : Theme.muted
                                font.family: Theme.font; font.pixelSize: 13
                                wrapMode: Text.WordWrap
                                Layout.bottomMargin: 6
                            }
                            // Column titles
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: 16; Layout.rightMargin: 16
                                spacing: 14
                                Text { Layout.fillWidth: true; text: "Name"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                                Text { Layout.preferredWidth: 70; horizontalAlignment: Text.AlignRight; text: "Processor"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                                Text { Layout.preferredWidth: 90; horizontalAlignment: Text.AlignRight; text: "Memory ▾"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                            }
                            ListView {
                                id: list
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                model: appModel
                                spacing: 6
                                clip: true
                                boundsBehavior: Flickable.StopAtBounds
                                keyNavigationEnabled: false
                                section.property: "section"
                                section.delegate: Text {
                                    required property string section
                                    width: list.width
                                    topPadding: 14; bottomPadding: 6
                                    text: section
                                    color: Theme.muted
                                    font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                                    font.letterSpacing: 1.2; font.capitalization: Font.AllUppercase
                                }
                                function step(d) {
                                    if (count === 0) return;
                                    currentIndex = Math.max(0, Math.min(count - 1, currentIndex + d));
                                    positionViewAtIndex(currentIndex, ListView.Contain);
                                }
                                Keys.onPressed: e => {
                                    switch (e.key) {
                                    case Qt.Key_Down: case Qt.Key_Tab: step(1); break;
                                    case Qt.Key_Up: case Qt.Key_Backtab: step(-1); break;
                                    case Qt.Key_PageDown: step(8); break;
                                    case Qt.Key_PageUp: step(-8); break;
                                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                                        if (currentIndex >= 0 && currentIndex < count) win.openActions(appModel.get(currentIndex).key);
                                        break;
                                    case Qt.Key_Left: case Qt.Key_Escape: loader.toNav(); break;
                                    default: return;
                                    }
                                    e.accepted = true;
                                }
                                delegate: Rectangle {
                                    id: row
                                    required property int index
                                    required property string key
                                    required property string name
                                    required property string icon
                                    required property string sub
                                    required property real cpu
                                    required property real mem
                                    required property bool locked
                                    readonly property bool current: ListView.isCurrentItem
                                    width: ListView.view.width
                                    height: 58
                                    radius: 12
                                    color: rowMouse.containsMouse ? Theme.panel3 : Theme.panel2
                                    border.width: 2
                                    border.color: current && list.activeFocus ? Theme.accent : "transparent"
                                    Behavior on border.color { ColorAnimation { duration: Theme.quick } }
                                    RowLayout {
                                        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                                        spacing: 14
                                        Rectangle {
                                            Layout.preferredWidth: 38; Layout.preferredHeight: 38
                                            radius: 10
                                            color: Theme.panel3
                                            IconImage {
                                                anchors.centerIn: parent
                                                visible: row.icon !== ""
                                                implicitSize: 26
                                                source: row.icon === "" ? "" : row.icon.startsWith("/") ? "file://" + row.icon : Quickshell.iconPath(row.icon, "application-x-executable")
                                            }
                                            Icon {
                                                anchors.centerIn: parent
                                                visible: row.icon === ""
                                                name: row.locked ? "lock" : "gear"
                                                size: 20
                                                color: Theme.muted
                                            }
                                        }
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 3
                                            Text { Layout.fillWidth: true; text: row.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                            Text { Layout.fillWidth: true; visible: row.sub !== ""; text: row.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                                        }
                                        Text {
                                            Layout.preferredWidth: 70
                                            horizontalAlignment: Text.AlignRight
                                            text: row.cpu >= 10 ? Math.round(row.cpu) + "%" : row.cpu.toFixed(1) + "%"
                                            color: row.cpu >= 50 ? Theme.flame : Theme.text
                                            font.family: Theme.font; font.pixelSize: 14
                                        }
                                        Text {
                                            Layout.preferredWidth: 90
                                            horizontalAlignment: Text.AlignRight
                                            text: tm.size(row.mem)
                                            color: Theme.text
                                            font.family: Theme.font; font.pixelSize: 14
                                        }
                                    }
                                    MouseArea {
                                        id: rowMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: { list.currentIndex = row.index; list.forceActiveFocus(); win.openActions(row.key); }
                                    }
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: "Ⓐ options  Ⓑ back"
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                            }
                        }

                        // The sheet: dims the tab, a card with what you can do.
                        Rectangle {
                            anchors.fill: parent
                            visible: apps.sheetOpen
                            color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.72)
                            MouseArea { anchors.fill: parent; onClicked: win.closeSheet() }
                            Rectangle {
                                id: sheetCard
                                anchors.centerIn: parent
                                width: Math.min(parent.width - 48, 480)
                                height: sheetCol.implicitHeight + 44
                                radius: Theme.radius + 4
                                color: Theme.panel
                                border.color: Theme.line
                                MouseArea { anchors.fill: parent }
                                Keys.onPressed: e => {
                                    const n = win.sheetButtons.length;
                                    switch (e.key) {
                                    case Qt.Key_Left: case Qt.Key_Up: case Qt.Key_Backtab: win.sheetIndex = Math.max(0, win.sheetIndex - 1); break;
                                    case Qt.Key_Right: case Qt.Key_Down: case Qt.Key_Tab: win.sheetIndex = Math.min(n - 1, win.sheetIndex + 1); break;
                                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (n) win.sheetAction(win.sheetButtons[win.sheetIndex].action); break;
                                    case Qt.Key_Escape: win.closeSheet(); break;
                                    default: return;
                                    }
                                    e.accepted = true;
                                }
                                ColumnLayout {
                                    id: sheetCol
                                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 }
                                    spacing: 10
                                    Text { Layout.fillWidth: true; text: win.sheetTitle; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                    Text { Layout.fillWidth: true; text: win.sheetText; color: win.sheetMode === "error" ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
                                    RowLayout {
                                        Layout.fillWidth: true
                                        Layout.topMargin: 8
                                        spacing: 8
                                        Item { Layout.fillWidth: true }
                                        Repeater {
                                            model: win.sheetButtons
                                            delegate: Rectangle {
                                                id: btn
                                                required property var modelData
                                                required property int index
                                                readonly property bool lit: win.sheetIndex === index
                                                implicitWidth: btnText.implicitWidth + 32
                                                implicitHeight: 40
                                                radius: 10
                                                color: lit ? (modelData.danger ? Theme.danger : Theme.accent) : btnMouse.containsMouse ? Theme.panel3 : Theme.panel2
                                                Behavior on color { ColorAnimation { duration: Theme.quick } }
                                                Text {
                                                    id: btnText
                                                    anchors.centerIn: parent
                                                    text: btn.modelData.label
                                                    color: btn.lit ? (btn.modelData.danger ? "#ffffff" : Theme.accentInk) : btn.modelData.danger ? Theme.danger : Theme.text
                                                    font.family: Theme.font; font.pixelSize: 13; font.weight: btn.lit ? Font.DemiBold : Font.Normal
                                                }
                                                MouseArea {
                                                    id: btnMouse
                                                    anchors.fill: parent
                                                    hoverEnabled: true
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: { win.sheetIndex = btn.index; win.sheetAction(btn.modelData.action); }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ================================================ Performance
                Component {
                    id: perfPage
                    Flickable {
                        id: pf
                        readonly property var p: win.perf
                        readonly property bool wide: width > 820
                        function focusFirst() { pf.forceActiveFocus(); }
                        contentHeight: pcol.implicitHeight + 48
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        Behavior on contentY { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
                        function scroll(d) { contentY = Math.max(0, Math.min(Math.max(0, contentHeight - height), contentY + d)); }
                        Keys.onPressed: e => {
                            switch (e.key) {
                            case Qt.Key_Down: scroll(90); break;
                            case Qt.Key_Up: scroll(-90); break;
                            case Qt.Key_PageDown: scroll(height - 60); break;
                            case Qt.Key_PageUp: scroll(-(height - 60)); break;
                            case Qt.Key_Left: case Qt.Key_Escape: loader.toNav(); break;
                            default: return;
                            }
                            e.accepted = true;
                        }

                        ColumnLayout {
                            id: pcol
                            width: Math.min(pf.width - 48, 1000)
                            x: 24
                            y: 20
                            spacing: 12
                            Text { text: "Performance"; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                            Text {
                                Layout.fillWidth: true
                                visible: !pf.p
                                text: "Measuring…"
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                            }

                            GridLayout {
                                Layout.fillWidth: true
                                visible: !!pf.p
                                columns: pf.wide ? 2 : 1
                                columnSpacing: 12
                                rowSpacing: 12

                                // Processor
                                Card {
                                    Layout.fillWidth: true
                                    Layout.alignment: Qt.AlignTop
                                    title: "Processor"
                                    value: tm.pct(pf.p ? pf.p.cpu.total : null)
                                    sub: pf.p ? [pf.p.cpu.name, pf.p.cpu.mhz ? (pf.p.cpu.mhz / 1000).toFixed(2) + " GHz" : "", pf.p.cpu.temp !== null && pf.p.cpu.temp !== undefined ? tm.temp(pf.p.cpu.temp) : ""].filter(s => s).join(" · ") : ""
                                    Graph { Layout.fillWidth: true; Layout.preferredHeight: 110; values: win.cpuHist }
                                    Text {
                                        visible: !!pf.p && pf.p.cpu.cores.length > 1
                                        text: "Each core"
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                    }
                                    Flow {
                                        Layout.fillWidth: true
                                        spacing: 4
                                        Repeater {
                                            model: pf.p && pf.p.cpu.cores.length > 1 ? pf.p.cpu.cores : []
                                            delegate: Rectangle {
                                                required property real modelData
                                                width: 14; height: 34; radius: 3
                                                color: Theme.panel3
                                                Rectangle {
                                                    anchors.bottom: parent.bottom
                                                    width: parent.width
                                                    height: Math.max(2, parent.height * Math.min(100, parent.modelData) / 100)
                                                    radius: 3
                                                    color: parent.modelData >= 85 ? Theme.flame : Theme.accent
                                                    Behavior on height { NumberAnimation { duration: Theme.calm } }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Memory
                                Card {
                                    Layout.fillWidth: true
                                    Layout.alignment: Qt.AlignTop
                                    title: "Memory"
                                    value: pf.p ? tm.sizeOf(pf.p.mem.used, pf.p.mem.total) : "–"
                                    sub: pf.p && pf.p.mem.total ? Math.round(100 * pf.p.mem.used / pf.p.mem.total) + "% in use" : ""
                                    Graph { Layout.fillWidth: true; Layout.preferredHeight: 110; values: win.memHist }
                                    RowLayout {
                                        Layout.fillWidth: true
                                        visible: !!pf.p && pf.p.swap.total > 0
                                        spacing: 10
                                        Text { text: "Swap"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                        Meter { Layout.fillWidth: true; value: pf.p && pf.p.swap.total ? pf.p.swap.used / pf.p.swap.total : 0 }
                                        Text { text: pf.p ? tm.sizeOf(pf.p.swap.used, pf.p.swap.total) : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                    }
                                }

                                // Graphics (one card per GPU)
                                Repeater {
                                    model: pf.p ? pf.p.gpus : []
                                    delegate: Card {
                                        id: gcard
                                        required property var modelData
                                        required property int index
                                        readonly property var g: modelData
                                        readonly property real vramUsed: g.vramUsed || 0
                                        readonly property real vramTotal: g.vramTotal || 0
                                        Layout.fillWidth: true
                                        Layout.alignment: Qt.AlignTop
                                        title: "Graphics"
                                        value: g.busy === null || g.busy === undefined ? "–" : tm.pct(g.busy)
                                        sub: [g.name, g.mhz ? g.mhz + " MHz" : "", g.temp !== null && g.temp !== undefined ? tm.temp(g.temp) : ""].filter(s => s).join(" · ")
                                        Graph { Layout.fillWidth: true; Layout.preferredHeight: 110; visible: gcard.g.busy !== null && gcard.g.busy !== undefined; values: win.gpuHist[gcard.index] || [] }
                                        Text {
                                            Layout.fillWidth: true
                                            visible: gcard.g.busy === null || gcard.g.busy === undefined
                                            text: "This graphics chip doesn't say how busy it is."
                                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                                        }
                                        RowLayout {
                                            Layout.fillWidth: true
                                            visible: gcard.vramTotal > 0
                                            spacing: 10
                                            Text { text: "Video memory"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                            Meter { Layout.fillWidth: true; value: gcard.vramTotal ? gcard.vramUsed / gcard.vramTotal : 0 }
                                            Text { text: tm.sizeOf(gcard.vramUsed, gcard.vramTotal); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                        }
                                        RowLayout {
                                            Layout.fillWidth: true
                                            visible: (gcard.g.gttTotal || 0) > 0
                                            spacing: 10
                                            Text { text: "Shared memory"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                            Meter { Layout.fillWidth: true; value: gcard.g.gttTotal ? (gcard.g.gttUsed || 0) / gcard.g.gttTotal : 0 }
                                            Text { text: tm.sizeOf(gcard.g.gttUsed || 0, gcard.g.gttTotal || 0); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                        }
                                    }
                                }
                                Card {
                                    Layout.fillWidth: true
                                    Layout.alignment: Qt.AlignTop
                                    visible: !!pf.p && pf.p.gpus.length === 0
                                    title: "Graphics"
                                    value: "–"
                                    sub: "No graphics details are available on this device."
                                }

                                // Temperatures
                                Card {
                                    Layout.fillWidth: true
                                    Layout.alignment: Qt.AlignTop
                                    title: "Temperatures"
                                    value: ""
                                    sub: pf.p && pf.p.temps.list.length === 0 ? "No temperature sensors found." : ""
                                    RowLayout {
                                        Layout.fillWidth: true
                                        visible: !!pf.p && pf.p.temps.list.length > 0
                                        spacing: 8
                                        Repeater {
                                            model: pf.p ? [
                                                { label: "Processor", c: pf.p.temps.cpu },
                                                { label: "Graphics", c: pf.p.temps.gpu },
                                                { label: "Storage", c: pf.p.temps.nvme }
                                            ].filter(t => t.c !== null && t.c !== undefined) : []
                                            delegate: Tile {
                                                required property var modelData
                                                Layout.fillWidth: true
                                                label: modelData.label
                                                value: tm.temp(modelData.c)
                                                hot: modelData.c >= 85
                                            }
                                        }
                                    }
                                    Flow {
                                        Layout.fillWidth: true
                                        spacing: 6
                                        Repeater {
                                            model: pf.p ? pf.p.temps.list : []
                                            delegate: Text {
                                                required property var modelData
                                                text: (modelData.label === modelData.sensor ? modelData.sensor : modelData.sensor + " " + modelData.label) + "  " + tm.temp(modelData.c) + "   "
                                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                                            }
                                        }
                                    }
                                }

                                // Fans (not every device has one it can read)
                                Card {
                                    Layout.fillWidth: true
                                    Layout.alignment: Qt.AlignTop
                                    visible: !!pf.p && pf.p.fans.length > 0
                                    title: pf.p && pf.p.fans.length === 1 ? "Fan" : "Fans"
                                    value: ""
                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 8
                                        Repeater {
                                            model: pf.p ? pf.p.fans : []
                                            delegate: Tile {
                                                required property var modelData
                                                Layout.fillWidth: true
                                                label: modelData.label
                                                value: modelData.rpm > 0 ? modelData.rpm + " rpm" : "Stopped"
                                            }
                                        }
                                    }
                                }

                                // Battery (handhelds and laptops)
                                Card {
                                    id: batt
                                    readonly property var b: pf.p ? pf.p.battery : null
                                    Layout.fillWidth: true
                                    Layout.alignment: Qt.AlignTop
                                    visible: !!b
                                    title: "Battery"
                                    value: b ? b.percent + "%" : ""
                                    sub: !b ? "" : b.status === "Charging" ? "Charging" + (b.hours ? " · full in about " + tm.hm(b.hours) : "")
                                        : b.status === "Full" ? "Full"
                                        : b.status === "Discharging" ? "On battery" + (b.hours ? " · about " + tm.hm(b.hours) + " left" : "")
                                        : b.plugged ? "Plugged in" : "On battery"
                                    Meter { Layout.fillWidth: true; value: batt.b ? batt.b.percent / 100 : 0 }
                                    Tile {
                                        Layout.fillWidth: true
                                        visible: !!(batt.b && batt.b.watts)
                                        label: batt.b && batt.b.status === "Charging" ? "Charging at" : "Power draw"
                                        value: batt.b && batt.b.watts ? batt.b.watts.toFixed(1) + " W" : "–"
                                    }
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                visible: !!pf.p
                                text: pf.p ? "On for " + (tm.hm(pf.p.uptime / 3600) || "less than a minute") : ""
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                            }
                        }
                    }
                }

                // ================================================ Startup and background
                Component {
                    id: startupPage
                    SetPage {
                        id: sp
                        title: "Startup and background"
                        Text {
                            Layout.fillWidth: true
                            visible: win.startupError !== ""
                            text: win.startupError
                            color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: !win.startupLoaded && win.startupError === ""
                            text: "Looking for things that start on their own…"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        }
                        SetHeader { text: "Hearth"; visible: hearthModel.count > 0 }
                        Repeater {
                            model: hearthModel
                            delegate: TmSwitch {
                                required property int index
                                required property string name
                                required property string sub
                                required property bool active
                                required property bool canChange
                                icon: "pad"
                                title: name
                                subText: sub
                                checked: active
                                interactive: canChange
                                onToggled: v => win.setStartup(hearthModel, index, v)
                            }
                        }
                        SetHeader { text: "Apps that open when you sign in"; visible: autoModel.count > 0 }
                        Repeater {
                            model: autoModel
                            delegate: TmSwitch {
                                required property int index
                                required property string name
                                required property string sub
                                required property bool active
                                required property bool canChange
                                title: name
                                subText: sub
                                checked: active
                                interactive: canChange
                                onToggled: v => win.setStartup(autoModel, index, v)
                            }
                        }
                        SetHeader { text: "Background services"; visible: serviceModel.count > 0 }
                        Repeater {
                            model: serviceModel
                            delegate: TmSwitch {
                                required property int index
                                required property string name
                                required property string sub
                                required property bool active
                                required property bool canChange
                                title: name
                                subText: sub
                                checked: active
                                interactive: canChange
                                onToggled: v => win.setStartup(serviceModel, index, v)
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.topMargin: 10
                            visible: win.startupLoaded
                            text: "Services that keep sound, sign-in, the desktop and Hearth working aren't listed, so they can't be turned off by mistake. Turning something off here stops it now and the next time you sign in."
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------ building blocks
    // A focusable row like Hearth Settings' (SetBase), but Left/Esc go back to Task Manager's own tab list.
    component TmSwitch: Rectangle {
        id: sw
        property string icon
        property string title
        property string subText
        property bool checked: false
        property bool interactive: true
        signal toggled(bool on)

        function back() {
            let p = sw.parent;
            while (p && typeof p.toNav !== "function") p = p.parent;
            if (p) p.toNav();
        }
        function scrollIntoView() {
            let p = sw.parent;
            while (p && !(p.contentItem !== undefined && p.contentHeight !== undefined && p.contentY !== undefined)) p = p.parent;
            if (!p) return;
            const y = sw.mapToItem(p.contentItem, 0, 0).y;
            if (y < p.contentY + 8) p.contentY = Math.max(0, y - 8);
            else if (y + sw.height > p.contentY + p.height - 8) p.contentY = Math.min(p.contentHeight - p.height, y + sw.height - p.height + 8);
        }
        onActiveFocusChanged: if (activeFocus) scrollIntoView()

        Layout.fillWidth: true
        implicitHeight: Math.max(64, swCol.implicitHeight + 24)
        activeFocusOnTab: interactive
        radius: 12
        color: swMouse.containsMouse && interactive ? Theme.panel3 : Theme.panel2
        border.width: 2
        border.color: activeFocus ? Theme.accent : "transparent"
        Behavior on border.color { ColorAnimation { duration: Theme.quick } }
        opacity: interactive ? 1 : 0.75

        Keys.onPressed: e => {
            switch (e.key) {
            case Qt.Key_Down: case Qt.Key_Tab: sw.nextItemInFocusChain(true).forceActiveFocus(); break;
            case Qt.Key_Up: case Qt.Key_Backtab: sw.nextItemInFocusChain(false).forceActiveFocus(); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (sw.interactive) sw.toggled(!sw.checked); break;
            case Qt.Key_Left: case Qt.Key_Escape: sw.back(); break;
            default: return;
            }
            e.accepted = true;
        }
        MouseArea {
            id: swMouse
            anchors.fill: parent
            hoverEnabled: true
            enabled: sw.interactive
            cursorShape: Qt.PointingHandCursor
            onClicked: { sw.forceActiveFocus(); sw.toggled(!sw.checked); }
        }
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            spacing: 14
            Rectangle {
                visible: sw.icon !== ""
                Layout.preferredWidth: 38; Layout.preferredHeight: 38
                radius: 10
                color: Theme.panel3
                Icon { anchors.centerIn: parent; name: sw.icon; size: 20; color: Theme.text }
            }
            ColumnLayout {
                id: swCol
                Layout.fillWidth: true
                spacing: 3
                Text { Layout.fillWidth: true; text: sw.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; visible: sw.subText !== ""; text: sw.subText; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
            }
            Rectangle {
                Layout.preferredWidth: 46; Layout.preferredHeight: 26
                radius: 13
                color: sw.checked ? Theme.accent : Theme.panel3
                Behavior on color { ColorAnimation { duration: Theme.quick } }
                Rectangle {
                    width: 20; height: 20; radius: 10
                    y: 3; x: sw.checked ? parent.width - width - 3 : 3
                    color: sw.checked ? Theme.accentInk : Theme.text
                    Behavior on x { NumberAnimation { duration: Theme.quick } }
                }
            }
        }
    }

    // A performance card: title and big number, a line of detail, then whatever goes inside.
    component Card: Rectangle {
        id: card
        property string title
        property string value
        property string sub
        default property alias content: cardBody.data
        radius: Theme.radius
        color: Theme.panel2
        implicitHeight: cardCol.implicitHeight + 32
        ColumnLayout {
            id: cardCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
            spacing: 8
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Text { Layout.fillWidth: true; text: card.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { visible: card.value !== ""; text: card.value; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
            }
            Text { Layout.fillWidth: true; visible: card.sub !== ""; text: card.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
            ColumnLayout {
                id: cardBody
                Layout.fillWidth: true
                spacing: 8
            }
        }
    }

    // The last minute as a line with a soft fill, 0–100 %.
    component Graph: Canvas {
        id: graph
        property var values: []
        property real maxValue: 100
        property color lineColor: Theme.accent
        onValuesChanged: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        function css(c, a) { return "rgba(" + Math.round(c.r * 255) + "," + Math.round(c.g * 255) + "," + Math.round(c.b * 255) + "," + (a === undefined ? c.a : a) + ")"; }
        onPaint: {
            const ctx = getContext("2d");
            const w = width;
            const h = height;
            ctx.reset();
            ctx.fillStyle = css(Theme.bg2);
            ctx.fillRect(0, 0, w, h);
            ctx.strokeStyle = css(Theme.line);
            ctx.lineWidth = 1;
            for (let i = 1; i < 4; i++) {
                const y = Math.round(h * i / 4) + 0.5;
                ctx.beginPath();
                ctx.moveTo(0, y);
                ctx.lineTo(w, y);
                ctx.stroke();
            }
            const v = values || [];
            if (v.length < 2) return;
            const step = w / 59;
            const x0 = w - (v.length - 1) * step;
            const yOf = n => h - 2 - (h - 4) * Math.max(0, Math.min(maxValue, n)) / maxValue;
            ctx.beginPath();
            ctx.moveTo(x0, h);
            for (let i = 0; i < v.length; i++) ctx.lineTo(x0 + i * step, yOf(v[i]));
            ctx.lineTo(w, h);
            ctx.closePath();
            ctx.fillStyle = css(lineColor, 0.18);
            ctx.fill();
            ctx.beginPath();
            for (let i = 0; i < v.length; i++) {
                if (i === 0) ctx.moveTo(x0, yOf(v[0]));
                else ctx.lineTo(x0 + i * step, yOf(v[i]));
            }
            ctx.strokeStyle = css(lineColor);
            ctx.lineWidth = 2;
            ctx.lineJoin = "round";
            ctx.stroke();
        }
    }

    // A thin bar: how full something is (0–1).
    component Meter: Rectangle {
        id: meter
        property real value: 0
        implicitHeight: 8
        radius: 4
        color: Theme.panel3
        Rectangle {
            width: parent.width * Math.max(0, Math.min(1, meter.value))
            height: parent.height
            radius: 4
            color: meter.value >= 0.9 ? Theme.flame : Theme.accent
            Behavior on width { NumberAnimation { duration: Theme.calm } }
        }
    }

    // A small labelled number (temperatures, fans, power draw).
    component Tile: Rectangle {
        id: tile
        property string label
        property string value
        property bool hot: false
        implicitHeight: 60
        radius: 10
        color: Theme.panel3
        ColumnLayout {
            anchors { fill: parent; margins: 10 }
            spacing: 2
            Text { Layout.fillWidth: true; text: tile.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; text: tile.value; color: tile.hot ? Theme.flame : Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold; elide: Text.ElideRight }
        }
    }
}
