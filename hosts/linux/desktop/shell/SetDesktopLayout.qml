// Hearth Settings › Personalization › Desktop layout (docs/mockups/desktop-layouts/layouts-6-layout-editor.png;
// checklist docs/mockups/checklists/desktop-layouts.md, part 6): where the bars, the dock, the app menu and the window
// buttons go. Your look's colours stay as they are.
//   the ready-made layouts with their pictures (Hearth, Steam-, Plasma-, GNOME-, Windows 11-, macOS-style, yours,
//   Start from empty), the one in use marked; the screen with its main bar and panels (Add a panel on a free edge; click
//   one to pick it, it's lit); blocks dragged from Blocks into a panel or along it (a lit line where it lands; right-click
//   a block to take it out); This panel (edge, style, height, hide until the pointer touches the edge); The whole desktop
//   (app menu, dock, window buttons); Share…, Open a layout file…, Back to (the layout), Save as my own….
// Every change is put in place at once through hearth-layout (hosts/hearth-layout: checked first, Mantel's own
// settings files merged, atomic, .bak; red flag 9), with Undo; Mantel follows its files live. Nothing is written by
// opening this page.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Desktop layout"

    property var all: []            // hearth-layout list --json
    property var draft: null        // the layout being changed (a copy of the one in use)
    property string baseId: ""      // the ready-made (or your) layout it's a copy of
    property string sel: "main"     // the selected bar: "main" or "p<n>"
    property string msg: ""
    property bool msgBad: false
    property bool naming: false
    property int dragFrom: -1       // a block dragged along its bar: its place there
    property string dragBar: ""

    // ---- running hearth-layout (its answer, its complaint, how it ended)
    Component {
        id: runComp
        Process {
            id: rp
            property var cb: null
            property int left: 3
            property string out: ""
            property string err: ""
            property int code: 0
            function step() { if (--left === 0) { const f = cb; cb = null; if (f) f(out, err, code); Qt.callLater(() => rp.destroy()); } }
            stdout: StdioCollector { onStreamFinished: { rp.out = text; rp.step(); } }
            stderr: StdioCollector { onStreamFinished: { rp.err = text; rp.step(); } }
            onExited: c => { rp.code = c; rp.step(); }
        }
    }
    function run(argv, cb) { const p = runComp.createObject(page, { command: argv, cb: cb || null }); if (p) p.running = true; }

    function copy(o) { return JSON.parse(JSON.stringify(o)); }
    function load(keepDraft) {
        run(["hearth-layout", "list", "--json"], (out, err, code) => {
            let l = [];
            try { l = JSON.parse(out); } catch (e) { l = []; }
            page.all = l;
            if (keepDraft && page.draft) return;
            const inUse = l.find(x => x.inUse) || l.find(x => page.draft && x.id === page.draft.id);
            if (inUse) { page.baseId = inUse.id.replace(/-mine$/, ""); page.draft = copy(inUse.layout); }
            else run(["hearth-layout", "now"], o => { try { page.draft = JSON.parse(o); page.baseId = ""; } catch (e) {} });
        });
    }
    Component.onCompleted: load(false)

    // ---- changes: the copy is put in place a moment after the last one (a few quick ones go together)
    Timer { id: commitLater; interval: 350; onTriggered: page.commit() }
    function change(fn) {
        const d = copy(draft);
        fn(d);
        if (!/-mine$/.test(d.id) && d.id !== "mine") { d.id = (d.id || "layout") + "-mine"; d.name = (baseName() || d.name) + " (your copy)"; }
        draft = d;
        commitLater.restart();
    }
    function commit() {
        run(["hearth-layout", "apply-json", JSON.stringify(draft)], (out, err, code) => {
            if (code === 0) { page.msg = ""; page.msgBad = false; load(true); }
            else { page.msg = (err || out).trim() || "That change couldn't be made"; page.msgBad = true; }
        });
    }
    function use(id) {
        run(["hearth-layout", "apply", id], (out, err, code) => {
            if (code === 0) { page.msg = ""; page.draft = null; load(false); }
            else { page.msg = (err || out).trim(); page.msgBad = true; }
        });
    }
    function startEmpty() {
        draft = { format: 1, id: "empty-mine", name: "My own layout", desktop: { barLayout: "one", barEdge: "bottom", dockFloat: "off", barStyle: "bar" },
                  bar: { start: ["menu"], centre: [], end: [], hidden: hiddenAllBut(["menu"]) }, panels: [] };
        baseId = "";
        commit();
    }
    function hiddenAllBut(keep) { return blockIds.filter(b => barOk(b) && keep.indexOf(b) < 0 && b !== "menu"); }
    function baseName() { const b = all.find(x => x.id === baseId); return b ? b.name : ""; }

    // ---- the blocks (hosts/hearth-layout blocks.rs), with the editor's short names
    readonly property var names: ({ menu: "App menu", apps: "Apps (icons)", windows: "Window list", pager: "Desktops", spacer: "Space", clock: "Clock",
        tray: "Tray", status: "Status", notifications: "Notifications", layout: "Keyboard layout", gamemode: "Game Mode", search: "Search",
        desktops: "Desktops", activity: "Activity", window: "Window title", jobs: "Copies", background: "Background apps", privacy: "Camera & mic",
        printers: "Printing", disks: "Disks", discord: "Discord", keyboard: "Keyboard", network: "Wi-Fi", bluetooth: "Bluetooth", volume: "Volume",
        battery: "Battery", overview: "Desktops pill", taskview: "Task view", appname: "App name", appmenus: "App menus (File, Edit…)",
        quickaccess: "Quick Access", showdesktop: "Show desktop", "widget:weather": "Weather", "widget:monitor": "System monitor",
        "widget:battery": "Battery widget", "widget:timer": "Timer" })
    readonly property var icons: ({ menu: "menu", apps: "grid", windows: "window", pager: "desktops", clock: "clock", tray: "chevu", status: "wifi",
        notifications: "bell", layout: "kbd", gamemode: "pad", search: "search", desktops: "desktops", window: "window", network: "wifi", volume: "vol",
        battery: "batt", bluetooth: "bt", keyboard: "kbd", overview: "desktops", taskview: "desktops", appname: "window", appmenus: "menu",
        quickaccess: "grid", showdesktop: "desktops", "widget:weather": "cloud", "widget:monitor": "cpu", "widget:battery": "batt", "widget:timer": "clock" })
    readonly property var blockIds: ["menu", "search", "desktops", "activity", "window", "jobs", "tray", "background", "privacy", "printers", "disks", "discord",
        "keyboard", "layout", "network", "bluetooth", "volume", "battery", "notifications", "apps", "windows", "pager", "spacer", "clock", "status",
        "gamemode", "widget:weather", "widget:monitor", "widget:battery", "widget:timer", "overview", "taskview", "appname", "appmenus", "quickaccess", "showdesktop"]
    readonly property var barOnly: ["search", "desktops", "activity", "window", "jobs", "background", "privacy", "printers", "disks", "discord", "keyboard", "network", "bluetooth", "volume", "battery"]
    readonly property var panelOnly: ["apps", "windows", "pager", "spacer", "clock", "status", "gamemode", "widget:weather", "widget:monitor", "widget:battery", "widget:timer"]
    function barOk(b) { return panelOnly.indexOf(b) < 0; }
    function panelOk(b) { return barOnly.indexOf(b) < 0; }

    // ---- the bars as the editor shows them: the main bar's blocks with its middle and end marked, a panel's in order
    readonly property var d: draft && draft.desktop ? draft.desktop : ({})
    readonly property string mainEdge: d.barLayout === "one" ? (d.barEdge || "bottom") : "top"
    readonly property bool dockOn: d.dockFloat !== "off" && d.dockFloat !== "none"
    readonly property var panels: draft && draft.panels ? draft.panels : []
    function mainTokens() {
        const b = draft && draft.bar ? draft.bar : { start: ["menu"], centre: [], end: [] };
        return (b.start || []).concat(["|centre"], b.centre || [], ["|end"], b.end || []);
    }
    function setMainTokens(t, bar) {
        const i = t.indexOf("|centre"), j = t.indexOf("|end");
        bar.start = t.slice(0, i); bar.centre = t.slice(i + 1, j); bar.end = t.slice(j + 1);
        const placed = bar.start.concat(bar.centre, bar.end);
        bar.hidden = blockIds.filter(x => barOk(x) && placed.indexOf(x) < 0 && x !== "menu");
        if (placed.indexOf("menu") < 0) bar.start.unshift("menu");   // the flame always stays
    }
    function tokensOf(key) { return key === "main" ? mainTokens() : ((panels[parseInt(key.slice(1))] || {}).items || []); }
    function putTokens(key, t) {
        change(dd => {
            if (key === "main") { dd.bar = dd.bar || {}; setMainTokens(t, dd.bar); }
            else dd.panels[parseInt(key.slice(1))].items = t;
        });
    }
    function insertBlock(key, id, at) {
        const t = tokensOf(key).slice();
        if (key === "main" && t.indexOf(id) >= 0) t.splice(t.indexOf(id), 1);
        t.splice(Math.max(0, Math.min(at, t.length)), 0, id);
        putTokens(key, t);
    }
    function moveBlock(key, from, to) {
        const t = tokensOf(key).slice();
        const [x] = t.splice(from, 1);
        t.splice(to > from ? to - 1 : to, 0, x);
        putTokens(key, t);
    }
    function removeBlock(key, at) {
        const t = tokensOf(key).slice();
        if (t[at] === "menu" && key === "main") { page.msg = "The app menu always stays in the main bar."; page.msgBad = true; return; }
        t.splice(at, 1);
        if (key !== "main" && t.length === 0) { page.msg = "A panel needs at least one block: remove the panel instead."; page.msgBad = true; return; }
        putTokens(key, t);
    }
    function addPanel(edge) {
        change(dd => { dd.panels = dd.panels || []; dd.panels.push({ edge: edge, items: ["menu", "windows", "spacer", "clock"], size: 44, autoHide: false }); });
        sel = "p" + (panels.length);
    }
    function removePanel(i) { change(dd => dd.panels.splice(i, 1)); sel = "main"; }
    readonly property var freeEdges: ["top", "left", "right", "bottom"].filter(e => e !== mainEdge && !(e === "bottom" && dockOn) && !panels.some(p => p.edge === e))

    // ---- the page
    Item {
        Layout.fillWidth: true
        implicitHeight: body.implicitHeight
        ColumnLayout {
            id: body
            width: parent.width
            spacing: 14

            // heading, line and the four buttons
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Text {
                    Layout.fillWidth: true
                    text: "Where the bars, the dock, the app menu and the window buttons go. Your look's colours stay as they are."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                }
                // drag and drop on the real desktop, everything anywhere (DesktopEditor.qml)
                SetButton { icon: "edit"; text: "Edit on the desktop"; primary: true; onActivated: Quickshell.execDetached(["hearth-mantel-ipc", "call", "deskedit", "open", ""]) }
                SetButton { icon: "share"; text: "Share…"; onActivated: page.share() }
                SetButton { text: "Open a layout file…"; onActivated: openFile.running = true }
                SetButton { visible: page.baseId !== "" && !!page.draft && /-mine$/.test(page.draft.id); text: "Back to " + page.baseName(); onActivated: page.use(page.baseId) }
                SetButton { icon: "sparkle"; text: "Describe it…"; onActivated: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "describe", "open", "bar"]) }
                SetButton { primary: true; text: "Save as my own…"; onActivated: { page.naming = true; nameField.forceActiveFocus(); } }
            }
            RowLayout {
                visible: page.naming
                Layout.fillWidth: true
                SetField { id: nameField; Layout.fillWidth: true; placeholder: "A name for this layout"; onSubmitted: t => page.saveAs(t) }
                SetButton { primary: true; text: "Save"; onActivated: page.saveAs(nameField.text) }
                SetButton { ghost: true; text: "Cancel"; onActivated: page.naming = false }
            }
            Text {
                visible: page.msg !== ""
                Layout.fillWidth: true
                text: page.msg
                color: page.msgBad ? Theme.red || "#e5534b" : Theme.text; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
            }

            // the ready-made layouts and yours, with their pictures
            Flow {
                Layout.fillWidth: true
                spacing: 14
                Repeater {
                    model: page.all
                    Column {
                        id: lc
                        required property var modelData
                        readonly property bool on: modelData.inUse || (page.draft && page.draft.id === modelData.id + "-mine")
                        spacing: 8
                        Rectangle {
                            width: 176; height: 102; radius: Theme.r(10)
                            color: "transparent"
                            border.width: lc.on ? 2 : 1
                            border.color: lc.on ? Theme.accent : lcArea.containsMouse ? Theme.muted : Theme.line
                            LayoutMini { anchors { fill: parent; margins: 3 } lay: lc.modelData.layout }
                            MouseArea { id: lcArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: page.use(lc.modelData.id) }
                        }
                        Text {
                            text: lc.modelData.name + (lc.on ? (page.draft && /-mine$/.test(page.draft.id) && !lc.modelData.inUse ? " · your copy" : " · in use") : "") + (lc.modelData.mine ? " (yours)" : "")
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: lc.on ? Font.DemiBold : Font.Normal
                            width: 176; elide: Text.ElideRight
                        }
                    }
                }
                Column {
                    spacing: 8
                    Rectangle {
                        width: 176; height: 102; radius: Theme.r(10); color: "transparent"
                        border.color: emptyArea.containsMouse ? Theme.accent : Theme.line
                        Text { anchors.centerIn: parent; text: "+"; color: Theme.muted; font.pixelSize: 30 }
                        MouseArea { id: emptyArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: page.startEmpty() }
                        Canvas {   // dashed edge
                            anchors.fill: parent
                            onPaint: { const c = getContext("2d"); c.reset(); c.setLineDash([5, 4]); c.strokeStyle = Theme.line; c.lineWidth = 1.5; c.strokeRect(1, 1, width - 2, height - 2); }
                        }
                    }
                    Text { text: "Start from empty"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                }
            }

            // the screen with its bars, and the right-hand column
            RowLayout {
                Layout.fillWidth: true
                spacing: 18
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignTop
                    spacing: 8
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: page.draft ? page.draft.name.replace(" (your copy)", "") + (/-mine$/.test(page.draft.id) ? " · your copy" : "") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
                        Item { Layout.fillWidth: true }
                        Text { text: "Drag blocks into a panel, or along it · right-click a panel for its settings"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                    Rectangle {
                        id: stageBox
                        Layout.fillWidth: true
                        Layout.preferredHeight: width * 0.62
                        radius: Theme.r(12)
                        color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.9)
                        border.color: Theme.line
                        clip: true
                        readonly property real t: 54

                        // free edges: Add a panel
                        Repeater {
                            model: page.freeEdges
                            Item {
                                required property string modelData
                                readonly property bool vert: modelData === "left" || modelData === "right"
                                x: modelData === "left" ? 24 : modelData === "right" ? stageBox.width - width - 24 : 24
                                y: modelData === "top" ? 22 : modelData === "bottom" ? stageBox.height - height - 22 : 110
                                width: vert ? 220 : stageBox.width - 48
                                height: vert ? 200 : 44
                                Canvas {
                                    anchors.fill: parent
                                    onPaint: { const c = getContext("2d"); c.reset(); c.setLineDash([6, 5]); c.strokeStyle = addArea.containsMouse ? Theme.accent : Theme.line; c.lineWidth = 1.5; c.strokeRect(1, 1, width - 2, height - 2); }
                                    Connections { target: addArea; function onContainsMouseChanged() { parent.requestPaint(); } }
                                }
                                Text { anchors.centerIn: parent; horizontalAlignment: Text.AlignHCenter; text: parent.modelData === "top" ? "+ Add a panel at the top" : parent.modelData === "bottom" ? "+ Add a panel at the bottom" : "+ Add a panel\non the " + parent.modelData; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                                MouseArea { id: addArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: page.addPanel(parent.modelData) }
                            }
                        }
                        // the floating dock
                        Rectangle {
                            visible: page.dockOn
                            anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 16 }
                            width: 260; height: 50; radius: Theme.r(16)
                            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.8); border.color: Theme.line
                            Text { anchors.centerIn: parent; text: "Dock: the flame and your apps"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        }
                        // the main bar and the panels
                        BarEdit { key: "main"; edge: page.mainEdge; floats: page.d.barStyle === "floating"; centred: false }
                        Repeater {
                            model: page.panels
                            BarEdit { required property var modelData; required property int index; key: "p" + index; edge: modelData.edge; floats: modelData.style === "floating"; centred: modelData.style === "centred" }
                        }
                    }
                }

                // ---- the right-hand column
                ColumnLayout {
                    Layout.preferredWidth: 430
                    Layout.maximumWidth: 430
                    Layout.alignment: Qt.AlignTop
                    spacing: 14
                    Card {
                        title: "Blocks"
                        Flow {
                            Layout.fillWidth: true
                            spacing: 8
                            Repeater {
                                model: page.blockIds.filter(b => (page.sel === "main" ? page.barOk(b) && page.mainTokens().indexOf(b) < 0 : page.panelOk(b)) && b !== "gamemode")
                                Chip { required property string modelData; blockId: modelData; fromBar: ""; fromAt: -1 }
                            }
                        }
                    }
                    Card {
                        id: thisPanel
                        title: page.sel === "main" ? "The main bar" : "This panel"
                        readonly property var p: page.sel === "main" ? null : page.panels[parseInt(page.sel.slice(1))]
                        SetChoice {
                            inset: true; title: "Edge"
                            options: ["top", "bottom", "left", "right"].map(e => ({ value: e, label: e.charAt(0).toUpperCase() + e.slice(1) }))
                            current: thisPanel.p ? thisPanel.p.edge : page.mainEdge
                            onChosen: v => {
                                if (page.sel === "main") page.change(dd => { dd.desktop = dd.desktop || {}; dd.desktop.barLayout = "one"; dd.desktop.barEdge = v; });
                                else { const i = parseInt(page.sel.slice(1)); page.change(dd => dd.panels[i].edge = v); }
                            }
                        }
                        SetChoice {
                            inset: true; title: "Style"
                            options: [{ value: "", label: "Full width" }, { value: "floating", label: "Floating" }, { value: "centred", label: "Centred", disabled: page.sel === "main" }]
                            current: thisPanel.p ? (thisPanel.p.style || "") : (page.d.barStyle === "floating" ? "floating" : "")
                            onChosen: v => {
                                if (page.sel === "main") page.change(dd => { dd.desktop = dd.desktop || {}; dd.desktop.barStyle = v === "" ? "bar" : v; });
                                else { const i = parseInt(page.sel.slice(1)); page.change(dd => dd.panels[i].style = v); }
                            }
                        }
                        SetSlider {
                            inset: true; title: "Height"
                            visible: page.sel !== "main"
                            value: thisPanel.p ? (thisPanel.p.size - 28) / 68 : 0.3
                            valueText: (thisPanel.p ? thisPanel.p.size : 44) + " px"
                            step: 1 / 68
                            onMoved: v => { const i = parseInt(page.sel.slice(1)); const s = Math.round(28 + v * 68); page.change(dd => dd.panels[i].size = s); }
                        }
                        Text { visible: page.sel === "main"; Layout.leftMargin: 16; text: "The main bar's height comes with your look."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        SetSwitch {
                            inset: true; title: "Hide until the pointer touches the edge"
                            checked: thisPanel.p ? !!thisPanel.p.autoHide : page.d.dockMode === "autohide"
                            onToggled: on => {
                                if (page.sel === "main") page.change(dd => { dd.desktop = dd.desktop || {}; dd.desktop.dockMode = on ? "autohide" : ""; });
                                else { const i = parseInt(page.sel.slice(1)); page.change(dd => dd.panels[i].autoHide = on); }
                            }
                        }
                        SetButton { visible: page.sel !== "main"; ghost: true; icon: "trash"; text: "Remove this panel"; onActivated: page.removePanel(parseInt(page.sel.slice(1))) }
                    }
                    Card {
                        title: "The whole desktop"
                        SetDropdown {
                            inset: true; title: "App menu"; boxWidth: 170
                            options: [{ value: "", label: "Hearth" }, { value: "start", label: "Start menu" }, { value: "centred", label: "Centred" }, { value: "tiles", label: "Tiles" }, { value: "classic", label: "Classic" }]
                            current: page.d.appMenu || ""
                            onChosen: v => page.change(dd => { dd.desktop = dd.desktop || {}; dd.desktop.appMenu = v; })
                        }
                        SetChoice {
                            inset: true; title: "Dock"
                            options: [{ value: "none", label: "None" }, { value: "floating", label: "Floating" }, { value: "hide", label: "Hide" }]
                            current: !page.dockOn ? "none" : page.d.dockMode === "autohide" ? "hide" : "floating"
                            onChosen: v => page.change(dd => {
                                dd.desktop = dd.desktop || {};
                                if (v === "none") { dd.desktop.barLayout = "one"; dd.desktop.barEdge = dd.desktop.barEdge || "bottom"; dd.desktop.dockFloat = "off"; }
                                else { dd.desktop.dockFloat = "on"; dd.desktop.dockMode = v === "hide" ? "autohide" : ""; }
                            })
                        }
                        SetChoice {
                            inset: true; title: "Window buttons"
                            readonly property var wb: page.draft && page.draft.windowButtons ? page.draft.windowButtons : { side: "right", buttons: ["iconify", "max", "close"] }
                            options: [{ value: "left", label: "Left" }, { value: "right", label: "Right" }]
                            current: wb.side
                            onChosen: v => page.change(dd => { const w = dd.windowButtons || { side: "right", buttons: ["iconify", "max", "close"] }; w.side = v; w.buttons = v === "left" && w.buttons.length === 3 ? ["close", "iconify", "max"] : w.buttons.length === 3 ? ["iconify", "max", "close"] : ["close"]; dd.windowButtons = w; })
                        }
                        SetChoice {
                            inset: true; title: " "
                            readonly property var wb: page.draft && page.draft.windowButtons ? page.draft.windowButtons : { side: "right", buttons: ["iconify", "max", "close"] }
                            options: [{ value: "close", label: "Close" }, { value: "all", label: "All three" }]
                            current: wb.buttons.length === 1 ? "close" : "all"
                            onChosen: v => page.change(dd => { const w = dd.windowButtons || { side: "right", buttons: [] }; w.buttons = v === "close" ? ["close"] : w.side === "left" ? ["close", "iconify", "max"] : ["iconify", "max", "close"]; dd.windowButtons = w; })
                        }
                        Text { Layout.leftMargin: 16; Layout.bottomMargin: 6; text: "Game Mode always stays in a panel, in every layout."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }
                }
            }
        }
    }

    // ---- a bar on the screen: its blocks in order, the selected one lit; drop blocks into it or along it
    component BarEdit: Rectangle {
        id: be
        property string key: "main"
        property string edge: "top"
        property bool floats: false
        property bool centred: false
        readonly property bool vert: edge === "left" || edge === "right"
        readonly property bool picked: page.sel === key
        readonly property var tokens: page.draft ? page.tokensOf(key) : []
        readonly property real gap: floats || centred ? 12 : 0
        readonly property real sw: parent ? parent.width : 800
        readonly property real sh: parent ? parent.height : 500
        readonly property real len: (vert ? sh : sw) - gap * 2
        readonly property real want: (vert ? list.implicitHeight : list.implicitWidth) + 24
        x: vert ? (edge === "left" ? gap : sw - width - gap) : (sw - width) / 2
        y: vert ? (sh - height) / 2 : (edge === "top" ? gap : sh - height - gap)
        width: vert ? (vert && key === "main" ? 150 : 140) : (centred ? Math.min(len, want) : len)
        height: vert ? (centred ? Math.min(len, want) : len) : 62
        radius: floats || centred ? Theme.r(14) : 0
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.92)
        border.width: picked ? 2 : 0
        border.color: Theme.accent
        Rectangle {   // the lit line toward the windows (the selected bar)
            visible: be.picked && !be.floats && !be.centred
            color: Theme.accent
            width: be.vert ? 2 : parent.width; height: be.vert ? parent.height : 2
            x: be.edge === "left" ? parent.width - 2 : 0
            y: be.edge === "top" ? parent.height - 2 : 0
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: page.sel = be.key
        }
        property int dropAt: -1
        DropArea {
            anchors.fill: parent
            keys: ["hearth-block"]
            onPositionChanged: d => be.dropAt = be.indexAt(d.x, d.y)
            onExited: be.dropAt = -1
            onDropped: d => {
                const id = d.source.blockId, from = d.source.fromBar, at = be.indexAt(d.x, d.y);
                be.dropAt = -1;
                page.sel = be.key;
                if (from === be.key && d.source.fromAt >= 0) page.moveBlock(be.key, d.source.fromAt, at);
                else if (be.key === "main" ? page.barOk(id) : page.panelOk(id)) page.insertBlock(be.key, id, at);
                else { page.msg = page.names[id] + " can't go " + (be.key === "main" ? "in the main bar" : "in a panel") + "."; page.msgBad = true; }
            }
        }
        function indexAt(px, py) {
            const p = vert ? py : px;
            for (let i = 0; i < chips.count; i++) {
                const c = chips.itemAt(i);
                if (!c) continue;
                const q = c.mapToItem(be, c.width / 2, c.height / 2);
                if (p < (vert ? q.y : q.x)) return i;
            }
            return tokens.length;
        }
        Grid {
            id: list
            anchors.centerIn: parent
            flow: be.vert ? Grid.TopToBottom : Grid.LeftToRight
            columns: be.vert ? 1 : 0
            rows: be.vert ? 0 : 1
            spacing: 8
            Repeater {
                id: chips
                model: be.tokens
                Item {
                    id: slot
                    required property string modelData
                    required property int index
                    implicitWidth: ch.visible ? ch.width : mark.width
                    implicitHeight: ch.visible ? ch.height : mark.height
                    width: implicitWidth; height: implicitHeight
                    Rectangle {   // where a dragged block lands
                        visible: be.dropAt === slot.index
                        color: Theme.accent
                        width: be.vert ? slot.width : 3; height: be.vert ? 3 : slot.height
                        x: be.vert ? 0 : -6; y: be.vert ? -6 : 0
                    }
                    // the main bar's middle and end: a Space you can drop into
                    Rectangle {
                        id: mark
                        visible: slot.modelData.charAt(0) === "|"
                        width: be.vert ? 110 : (be.key === "main" ? 150 : 90); height: be.vert ? 30 : 40
                        radius: Theme.r(8); color: "transparent"; border.color: Theme.line
                        Text { anchors.centerIn: parent; text: "space"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }
                    Chip { id: ch; visible: !mark.visible; blockId: slot.modelData; fromBar: be.key; fromAt: slot.index }
                }
            }
            Rectangle {   // Game Mode always stays (in the main bar)
                visible: be.key === "main"
                width: gmt.implicitWidth + 50; height: 40; radius: Theme.r(8); color: Theme.accent
                Row { anchors.centerIn: parent; spacing: 8; Icon { name: "pad"; size: 16; color: Theme.accentInk } Text { id: gmt; text: "Game Mode"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold } }
            }
        }
        Rectangle {   // a dropped block at the very end
            visible: be.dropAt === be.tokens.length && be.tokens.length > 0
            color: Theme.accent
            width: be.vert ? list.width : 3; height: be.vert ? 3 : 40
            x: be.vert ? list.x : list.x + list.width + 4
            y: be.vert ? list.y + list.height + 4 : (be.height - 40) / 2
        }
    }

    // ---- a block: drag it into a bar or along one; right-click takes it out of its bar
    component Chip: Rectangle {
        id: c
        property string blockId: ""
        property string fromBar: ""
        property int fromAt: -1
        width: cr.implicitWidth + 24; height: fromBar === "" ? 32 : 40
        radius: Theme.r(8)
        color: ca.containsMouse || ca.drag.active ? Theme.panel3 : Theme.bg2
        border.color: Theme.line
        Drag.active: ca.drag.active
        Drag.keys: ["hearth-block"]
        Drag.source: c
        Drag.hotSpot.x: width / 2; Drag.hotSpot.y: height / 2
        Row {
            id: cr
            anchors.centerIn: parent
            spacing: 7
            Text { visible: c.fromBar === ""; text: "⠿"; color: Theme.muted; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
            Image { visible: c.blockId === "menu"; width: 18; height: 18; source: Qt.resolvedUrl("images/hearth-logo.png"); anchors.verticalCenter: parent.verticalCenter; smooth: true; mipmap: true }
            Icon { visible: c.fromBar !== "" && c.blockId !== "menu" && !!page.icons[c.blockId]; name: page.icons[c.blockId] || "dot"; size: 16; anchors.verticalCenter: parent.verticalCenter }
            Text { text: page.names[c.blockId] || c.blockId; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
        }
        property point home
        MouseArea {
            id: ca
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: drag.active ? Qt.ClosedHandCursor : Qt.OpenHandCursor
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            drag.target: c
            onPressed: m => { c.home = Qt.point(c.x, c.y); c.z = 100; }
            onReleased: { if (c.Drag.active) c.Drag.drop(); c.x = c.home.x; c.y = c.home.y; c.z = 0; }
            onClicked: m => { if (m.button === Qt.RightButton && c.fromBar !== "") page.removeBlock(c.fromBar, c.fromAt); else if (c.fromBar !== "") page.sel = c.fromBar; }
        }
    }

    component Card: Rectangle {
        property string title
        default property alias content: cc.data
        Layout.fillWidth: true
        implicitHeight: cc.implicitHeight + 24
        radius: Theme.r(12)
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.7)
        border.color: Theme.line
        ColumnLayout {
            id: cc
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
            spacing: 8
            Text { text: parent.parent.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; Layout.leftMargin: 4 }
        }
    }

    // ---- Share…, Open a layout file…, Save as my own…
    function share() {
        const f = (Quickshell.env("HOME") || "") + "/Documents/" + (draft ? draft.id : "layout") + ".hearth-layout.json";
        run(["sh", "-c", 'mkdir -p "$(dirname "$1")" && printf %s "$2" > "$1.tmp" && mv "$1.tmp" "$1" && echo "$1"', "sh", f, JSON.stringify(draft, null, 2)], (out, err, code) => {
            page.msg = code === 0 ? "Saved to share: " + out.trim() + " (anyone with Hearth OS can open it here)" : "Couldn't save it: " + err.trim();
            page.msgBad = code !== 0;
            if (code === 0) Quickshell.execDetached(["xdg-open", (Quickshell.env("HOME") || "") + "/Documents"]);
        });
    }
    function saveAs(name) {
        if (String(name).trim() === "") return;
        run(["hearth-layout", "save", String(name)], (out, err, code) => {
            page.naming = false;
            page.msg = code === 0 ? "Saved with your layouts" : (err || out).trim();
            page.msgBad = code !== 0;
            if (code === 0) load(true);
        });
    }
    Process {
        id: openFile
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Open a layout file' --getopenfilename \"$HOME\" '*.json'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Open a layout file' --filename=\"$HOME/\" --file-filter='Layouts | *.json'; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f !== "") page.use(f); } }
    }
}
