// Build anything board 4 (and 5): the block editor. Any block anywhere on a grid, any size, any colour, corners, font,
// see-through, sound and animation; what it shows and what a press does; above or under your windows. Blocks on the
// left (drag one onto the screen, or press it), the screen in the middle (drag a block to move it, its corners to size
// it), the picked block's Shows, Look, Place and When pressed on the right, and "Or say it" under the screen: your AI
// changes the screen (hearth-layout describe-screen) and, when Hearth hasn't got a block for it, makes one (board 5:
// MADE FOR YOU BY YOUR AI, what you asked, how it's boxed in, its ask, Keep this / Change it / Try again / Undo / Share).
// Undo, Try it full screen, Share it, Keep this. Nothing changes your desktop until Keep this (ScreenLayout.keep).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: ed
    property bool shown: false
    property var launcher: null
    property var quick: null
    property var settingsApp: null
    property var filesApp: null
    property var powerMenu: null
    visible: shown
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-screen-editor"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "#1b1715"

    property var draft: ({ grid: { cols: 24, rows: 14 }, background: { kind: "gradient", from: "#10243a", to: "#1a0d07" }, look: {}, blocks: [] })
    property var undoStack: []
    property string name: ""
    property int pick: -1
    property string said: ""
    property string error: ""
    property bool busy: false
    property var made: null              // board 5: what your AI made ({block, asks, asked})
    property var previewWin: null
    readonly property var cur: pick >= 0 && pick < (draft.blocks || []).length ? draft.blocks[pick] : null

    // ---- what each block is called, and what it may show (the editor's Shows chip goes round them)
    readonly property var catalog: [
        { b: "hero", t: "Hero tile", g: "window", shows: [["recent", "Your last game, with its art"]] },
        { b: "tilerow", t: "Tile row", g: "grid", shows: [["recent", "Recent games"], ["games", "All your games"], ["", "Your apps and things"], ["words", "Your apps as words"], ["sticky", "A sticky note of them"]] },
        { b: "iconrow", t: "Icon row", g: "dot", shows: [["games", "Your games as icons"]] },
        { b: "tabs", t: "Tabs", g: "list", shows: [["", "Games, Media, Desktop"]] },
        { b: "guide", t: "Guide menu", g: "menu", shows: [["", "Home, Quick settings, Friends… Power"]] },
        { b: "clock", t: "Clock", g: "clock", shows: [["", "The time"], ["date", "The date"], ["ring", "The time in a ring"]] },
        { b: "profile", t: "Profile", g: "user", shows: [["", "Your picture and name"]] },
        { b: "battery", t: "Battery", g: "batt", shows: [["", "The battery"]] },
        { b: "bar", t: "Bar", g: "desktops", shows: [["", "A bar: Start, your windows, the clock"]] },
        { b: "dock", t: "Dock", g: "home", shows: [["", "Your dock's apps"]] },
        { b: "media", t: "Media", g: "headphones", shows: [["", "What's playing"]] },
        { b: "friends", t: "Friends", g: "family", shows: [["", "Family and friends"]] },
        { b: "notes", t: "Notes", g: "doc", shows: [["", "Your words"]] },
        { b: "weather", t: "Weather", g: "sun", shows: [["", "The weather where you are"]] },
        { b: "windows", t: "Windows", g: "window", shows: [["", "Your open windows"]] },
        { b: "own", t: "Your own…", g: "sparkle", shows: [] }
    ]
    readonly property var more: [
        { b: "status", t: "Status corner", shows: [["", "Search, settings, battery, time"]] },
        { b: "play", t: "Play", shows: [["", "The picked game, Play and ⋯"]] },
        { b: "activity", t: "Activity", shows: [["", "What you did"]] },
        { b: "widget", t: "Readout", shows: [["system", "CPU, GPU, memory, battery"]] },
        { b: "list", t: "List", shows: [["games", "Your games, as words"], ["recent", "Recent games, as words"]] },
        { b: "icons", t: "Desktop icons", shows: [["", "My Hearth, Bin, Files, Games"]] },
        { b: "text", t: "Words", shows: [["", "Your own words"]] },
        { b: "picture", t: "Picture", shows: [["", "A picture of yours"], ["fire", "The fireside"]] },
        { b: "menu", t: "App menu", shows: [["", "The app menu"]] },
        { b: "gamemode", t: "Game Mode", shows: [["", "Game Mode, one press"]] },
        { b: "search", t: "Search", shows: [["", "Search"]] },
        { b: "downloads", t: "Downloads", shows: [["", "Downloads and updates"]] }
    ]
    function info(b) { return catalog.concat(more).find(c => c.b === b) || { b: b, t: String(b).startsWith("own:") ? "Made by your AI" : b, shows: [] }; }
    readonly property var colours: ["#ffffff", "#2f9e5a", "#ff7a2f", "#5b8cff", "#b07cff", "#f2c14e", "#e05a6a", "#0b3a44", "#1f3a2e", "#00000000"]
    readonly property var fonts: ["", "Inter", "Inter Display", "Noto Serif", "Noto Sans Mono", "Pixel", "Rounded", "Condensed"]
    readonly property var focuses: [["grow", "Grows, white outline"], ["outline", "White outline"], ["glow", "Glows"], ["lift", "Lifts"], ["none", "Nothing"]]
    readonly property var sounds: [["", "The look's own"], ["soft", "Soft click (Hearth's)"], ["click", "Click"], ["pop", "Pop"], ["chime", "Chime"], ["wood", "Wood"], ["retro", "Retro"], ["none", "No sound"]]
    readonly property var anims: [["", "The look's own"], ["slide-left", "Slides in from the left"], ["slide-right", "Slides in from the right"], ["slide-up", "Slides up"], ["slide-down", "Slides down"], ["fade", "Fades in"], ["zoom", "Zooms in"], ["none", "Just there"]]
    readonly property var presses: [["", "What it does anyway"], ["play", "Play it"], ["menu", "Open the app menu"], ["game-mode", "Game Mode"], ["quickaccess", "Quick Access"], ["overview", "All your windows"], ["screenshot", "Take a screenshot"], ["lock", "Lock the screen"], ["none", "Nothing"]]

    function copy(o) { return JSON.parse(JSON.stringify(o)); }
    function open(screenObj, title) {
        draft = copy(screenObj || ScreenLayout.screen || { grid: { cols: 24, rows: 14 }, background: { kind: "gradient", from: "#10243a", to: "#1a0d07" }, look: {}, blocks: [] });
        if (!draft.grid) draft.grid = { cols: 24, rows: 14 };
        if (!draft.blocks) draft.blocks = [];
        name = title || (ScreenLayout.readyMade.find(r => r.id === ScreenLayout.from) || { name: draft.blocks.length ? "Your desktop" : "Start from nothing" }).name;
        said = draft.said || "";
        undoStack = []; pick = draft.blocks.length ? 0 : -1; error = ""; made = null; sayField.text = "";
        shown = true;
        refreshMine();
    }
    function openReady(id) {
        const r = ScreenLayout.readyMade.find(x => x.id === id);
        readyFile.path = ""; readyFile.title = r ? r.name : id; readyFile.path = ScreenLayout.readyFile(id);
    }
    FileView { id: readyFile; property string title: ""; path: ""; onLoaded: { try { ed.open(JSON.parse(text()), title); } catch (e) {} } }
    function change(f) { undoStack = undoStack.concat([JSON.stringify(draft)]).slice(-60); const d = copy(draft); f(d); draft = d; }
    function undo() { if (!undoStack.length) return; draft = JSON.parse(undoStack[undoStack.length - 1]); undoStack = undoStack.slice(0, -1); if (pick >= draft.blocks.length) pick = draft.blocks.length - 1; }
    function setStyle(k, v) { if (!cur) return; change(d => { const b = d.blocks[pick]; b.style = b.style || {}; if (v === "" || v === undefined) delete b.style[k]; else b.style[k] = v; }); }
    function setField(k, v) { if (!cur) return; change(d => { const b = d.blocks[pick]; if (v === "" || v === undefined) delete b[k]; else b[k] = v; }); }
    function nextOf(list, v) { const i = list.findIndex(x => (Array.isArray(x) ? x[0] : x) === (v || "")); const n = list[(i + 1) % list.length]; return Array.isArray(n) ? n[0] : n; }
    function labelOf(list, v) { const x = list.find(x => x[0] === (v || "")); return x ? x[1] : String(v || ""); }
    function freeSpot(w, h) {
        const c = draft.grid.cols, r = draft.grid.rows;
        for (let y = 0; y + h <= r; y++) for (let x = 0; x + w <= c; x++) {
            if (!(draft.blocks || []).some(b => x < b.x + b.w && b.x < x + w && y < b.y + b.h && b.y < y + h)) return [x, y];
        }
        return [0, 0];
    }
    function add(b, gx, gy) {
        if (b === "own") { sayField.text = "A block that "; sayField.forceActiveFocus(); return; }
        const size = ({ hero: [8, 5], tilerow: [16, 4], iconrow: [14, 3], tabs: [10, 1], guide: [6, 14], clock: [5, 1], profile: [6, 1], battery: [3, 1], bar: [24, 1],
                        dock: [10, 2], media: [8, 2], friends: [6, 3], notes: [8, 6], weather: [8, 1], windows: [8, 6], status: [7, 1], play: [14, 3], activity: [15, 3],
                        widget: [6, 4], list: [6, 5], icons: [6, 6], text: [8, 1], picture: [8, 6], menu: [2, 2], gamemode: [4, 1], search: [6, 1], downloads: [4, 3] })[b] || [4, 2];
        const w = Math.min(size[0], draft.grid.cols), h = Math.min(size[1], draft.grid.rows);
        const spot = gx === undefined ? freeSpot(w, h) : [Math.max(0, Math.min(draft.grid.cols - w, gx)), Math.max(0, Math.min(draft.grid.rows - h, gy))];
        let id = b, n = 2;
        while ((draft.blocks || []).some(x => x.id === id)) id = b + "-" + n++;
        const nb = { id: id, block: b, x: spot[0], y: spot[1], w: w, h: h };
        if (b === "tilerow") { nb.items = ["games", "store", "files", "browser", "music"]; }
        if (b === "tabs") nb.items = ["games", "music", "windows"];
        if (b === "bar") nb.items = ["menu", "windows"];
        if (b === "icons") nb.items = ["home", "bin", "files", "games"];
        if (b === "notes") { nb.title = "Notes"; nb.value = "☐ "; }
        if (b === "text") nb.value = "Your words";
        if (b === "widget") nb.shows = "system";
        if (b === "iconrow" || b === "list") nb.shows = "games";
        if (b === "hero") nb.shows = "recent";
        change(d => d.blocks.push(nb));
        pick = draft.blocks.length - 1;
    }
    function remove() { if (!cur) return; const i = pick; change(d => d.blocks.splice(i, 1)); pick = Math.min(i, draft.blocks.length - 1); }

    // ---- your AI: the screen changed (and blocks made) from your words
    function say(t) { sayField.text = t; sayIt(); }
    function sayIt() {
        const t = sayField.text.trim();
        if (!t || busy) return;
        busy = true; error = "";
        describeP.command = ScreenLayout.layoutCmd(["describe-screen", t, JSON.stringify(draft)]);
        describeP.asked = t;
        describeP.running = true;
    }
    Process {
        id: describeP
        property string asked: ""
        environment: ({ HEARTH_DESCRIBE_APP: "Mantel " + (Quickshell.env("HEARTH_MANTEL_VERSION") || "") })
        stdout: StdioCollector {
            onStreamFinished: {
                ed.busy = false;
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
                if (!r || r.ok !== true) { ed.error = (r && r.error) || "Your AI didn't answer."; return; }
                ed.change(d => { const s = r.screen; for (const k in d) delete d[k]; for (const k in s) d[k] = s[k]; });
                ed.said = describeP.asked;
                ed.pick = ed.draft.blocks.length ? 0 : -1;
                if (r.made && r.made.length) {
                    const m = r.made[0];
                    ed.pick = ed.draft.blocks.findIndex(b => b.block === "own:" + m.id);
                    // kept first (so it can run and ask), then shown
                    keepBlockP.next = { block: m, asks: (r.asks || [])[0] || null, asked: describeP.asked, all: r.made };
                    keepBlockP.command = ScreenLayout.layoutCmd(["keep-block", JSON.stringify(m)]);
                    keepBlockP.running = true;
                    for (const b of r.made.slice(1)) Quickshell.execDetached(ScreenLayout.layoutCmd(["keep-block", JSON.stringify(b)]));
                }
            }
        }
    }
    Process { id: keepBlockP; property var next: null; onExited: { ed.made = next; ed.refreshMine(); } }
    // your blocks (made by your AI, kept) and what people shared with Hearth
    property var mine: []
    function refreshMine() { mineP.command = ScreenLayout.layoutCmd(["blocks-mine"]); mineP.running = true; }
    Process { id: mineP; stdout: StdioCollector { onStreamFinished: { try { const v = JSON.parse(text.trim() || "[]"); if (Array.isArray(v)) ed.mine = v; } catch (e) {} } } }

    function tryFull() {
        if (previewWin) { previewWin.destroy(); previewWin = null; }
        const c = Qt.createComponent("Screen.qml");
        if (c.status !== Component.Ready) { error = "It couldn't be shown: " + c.errorString(); return; }
        previewWin = c.createObject(ed, { screen: ed.screen, preview: copy(draft), launcher: launcher, quick: quick, settingsApp: settingsApp, filesApp: filesApp, powerMenu: powerMenu });
        if (previewWin) previewWin.previewChanged.connect(() => { if (previewWin && !previewWin.preview) { previewWin.destroy(); previewWin = null; } });
    }
    function keep() {
        checkP.command = ScreenLayout.layoutCmd(["check-screen", JSON.stringify(Object.assign(copy(draft), { said: said || draft.said || "" }))]);
        checkP.running = true;
    }
    Process {
        id: checkP
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
                // (with no hearth-layout here at all, the screen is kept as it is: Screen.qml draws only what it knows)
                if (r && r.ok === false) { ed.error = (r.problems || []).join(" "); return; }
                const e = ScreenLayout.keep(ed.copy(ed.draft), ed.said || ed.draft.said || "");
                if (e) { ed.error = e; return; }
                ed.shown = false;
            }
        }
    }
    function share() {
        const idea = { format: 1, part: "screen", asked: (said || name || "My desktop").slice(0, 1000), needs: ["A desktop shared with Hearth: " + (name || "my own")], sketch: false, hearth: "", app: "Mantel", kept: true };
        Quickshell.execDetached(ScreenLayout.layoutCmd(["send-idea", JSON.stringify(idea)]));
        error = "Sent to Hearth's makers: if they share it, it shows under Made by people ✓";
    }

    Item {
        anchors.fill: parent
        focus: ed.shown
        Keys.onPressed: e => {
        if (e.key === Qt.Key_Escape) { if (ed.made) ed.made = null; else ed.shown = false; e.accepted = true; }
        else if ((e.key === Qt.Key_Delete || e.key === Qt.Key_Backspace) && !sayField.activeFocus && ed.cur) { ed.remove(); e.accepted = true; }
        else if (e.key === Qt.Key_Z && (e.modifiers & Qt.ControlModifier)) { ed.undo(); e.accepted = true; }
        }
    }

    // ---- the right panel's rows
    component Head: Text { color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.2; topPadding: 10 }
    component Pill: Rectangle {
        property string text: ""
        signal clicked()
        width: Math.min(parent ? parent.width : 336, pt.implicitWidth + 24); height: 30; radius: 15; color: "transparent"; border.color: Qt.rgba(1, 1, 1, 0.25)
        Text { id: pt; anchors.centerIn: parent; width: Math.min(implicitWidth, parent.width - 20); elide: Text.ElideRight; text: parent.text; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 13 }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: parent.clicked() }
    }
    component Line: RowLayout {
        property string label: ""
        property string value: ""
        signal clicked()
        width: parent ? parent.width : 336
        Text { text: parent.label; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 14; Layout.preferredWidth: 120 }
        Rectangle {
            Layout.fillWidth: true; height: 30; radius: 8; color: "transparent"; border.color: Qt.rgba(1, 1, 1, 0.2)
            Text { anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter } text: parent.parent.value; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 13 }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: parent.parent.clicked() }
        }
    }
    component Slide: RowLayout {
        property string label: ""
        property real value: 0
        property real max: 64
        signal moved(real v)
        width: parent ? parent.width : 336
        Text { text: parent.label; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 14; Layout.preferredWidth: 120 }
        Item {
            Layout.fillWidth: true; height: 20
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 3; radius: 2; color: Qt.rgba(1, 1, 1, 0.15) }
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width * Math.min(1, parent.parent.value / parent.parent.max); height: 3; radius: 2; color: "#ff7a2f" }
            MouseArea { anchors.fill: parent; onPressed: m => parent.parent.moved(Math.round(Math.max(0, Math.min(1, m.x / width)) * parent.parent.max)); onPositionChanged: m => parent.parent.moved(Math.round(Math.max(0, Math.min(1, m.x / width)) * parent.parent.max)) }
        }
    }

    // ================================================================ the top bar
    Rectangle {
        id: top
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 56; color: "#211c19"
        Row {
            anchors { left: parent.left; leftMargin: 24; verticalCenter: parent.verticalCenter }
            spacing: 14
            Text { text: ed.made ? "Editing your desktop" : "Your desktop"; color: "white"; font.family: Theme.font; font.pixelSize: 17; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
            Rectangle {
                visible: !ed.made
                height: 28; width: nameT.implicitWidth + 26; radius: 14; color: "transparent"; border.color: Qt.rgba(1, 1, 1, 0.25)
                anchors.verticalCenter: parent.verticalCenter
                Text { id: nameT; anchors.centerIn: parent; text: "\"" + ed.name + "\""; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 14 }
            }
            Text { visible: !!ed.made; text: "· a block that isn't here yet"; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 15; anchors.verticalCenter: parent.verticalCenter }
        }
        Row {
            anchors { right: parent.right; rightMargin: 24; verticalCenter: parent.verticalCenter }
            spacing: 12
            Repeater {
                model: ed.made ? [{ t: "Done", a: "done" }] : [{ t: "Undo", a: "undo" }, { t: "Try it full screen", a: "full" }, { t: "Share it", a: "share" }, { t: "Keep this", a: "keep" }]
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool primary: modelData.a === "keep"
                    width: bt.implicitWidth + 36; height: 36; radius: 6
                    color: primary ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.07); border.width: primary ? 0 : 1; border.color: Qt.rgba(1, 1, 1, 0.18)
                    opacity: modelData.a === "undo" && !ed.undoStack.length ? 0.45 : 1
                    Text { id: bt; anchors.centerIn: parent; text: modelData.t; color: parent.primary ? "#1b1006" : "white"; font.family: Theme.font; font.pixelSize: 15; font.bold: true }
                    MouseArea {
                        anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                        onClicked: modelData.a === "undo" ? ed.undo() : modelData.a === "full" ? ed.tryFull() : modelData.a === "share" ? ed.share() : modelData.a === "keep" ? ed.keep() : (ed.made = null)
                    }
                }
            }
        }
    }

    // ================================================================ the blocks (left)
    Rectangle {
        id: left
        visible: !ed.made
        anchors { left: parent.left; top: top.bottom; bottom: parent.bottom }
        width: 300; color: "#1f1a17"
        Flickable {
            anchors { fill: parent; margins: 20 }
            contentHeight: lc.implicitHeight; clip: true
            Column {
                id: lc
                width: parent.width
                spacing: 10
                Text { text: "BLOCKS · DRAG ONTO THE SCREEN"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4 }
                Grid {
                    columns: 2; spacing: 10
                    Repeater {
                        model: ed.catalog
                        delegate: Rectangle {
                            id: chip
                            required property var modelData
                            width: (lc.width - 10) / 2; height: 40; radius: 6; color: "#2a2420"
                            Row {
                                anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                                spacing: 8
                                Icon { name: chip.modelData.g; size: 15; color: "#e8ddd4"; anchors.verticalCenter: parent.verticalCenter }
                                Text { text: chip.modelData.t; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter }
                            }
                            MouseArea {
                                id: dragArea
                                anchors.fill: parent; cursorShape: Qt.OpenHandCursor
                                property bool moved: false
                                onPressed: { moved = false; ghost.label = chip.modelData.t; ghost.kind = chip.modelData.b; }
                                onPositionChanged: mouse => {
                                    const p = mapToItem(null, mouse.x, mouse.y);
                                    if (!moved && (Math.abs(mouse.x - width / 2) > 6 || Math.abs(mouse.y - height / 2) > 6)) moved = true;
                                    if (moved) { ghost.x = p.x - ghost.width / 2; ghost.y = p.y - ghost.height / 2; ghost.visible = true; }
                                }
                                onReleased: mouse => {
                                    ghost.visible = false;
                                    if (!moved) { ed.add(chip.modelData.b); return; }
                                    const p = mapToItem(canvas, mouse.x, mouse.y);
                                    if (p.x >= 0 && p.y >= 0 && p.x <= canvas.width && p.y <= canvas.height) ed.add(chip.modelData.b, Math.floor(p.x / canvas.cw), Math.floor(p.y / canvas.ch));
                                }
                            }
                        }
                    }
                }
                Text { text: "MORE"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4; topPadding: 6 }
                Flow {
                    width: lc.width; spacing: 6
                    Repeater {
                        model: ed.more
                        delegate: Rectangle {
                            required property var modelData
                            width: mt.implicitWidth + 18; height: 28; radius: 14; color: "#2a2420"
                            Text { id: mt; anchors.centerIn: parent; text: parent.modelData.t; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 12 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.add(parent.modelData.b) }
                        }
                    }
                }
                Rectangle {
                    width: lc.width; height: 62; radius: 8; color: "transparent"
                    border.width: 1; border.color: "#ff7a2f"
                    Text { anchors { fill: parent; margins: 14 } wrapMode: Text.WordWrap; text: "+ Describe a block that isn't here, and your AI makes it"; color: "#ff9a52"; font.family: Theme.font; font.pixelSize: 14 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.add("own") }
                }
                Text { text: "YOUR BLOCKS"; visible: ed.mine.length > 0; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4; topPadding: 8 }
                Repeater {
                    model: ed.mine
                    delegate: Text {
                        required property var modelData
                        text: modelData.name + (modelData.on ? "" : " (off)"); color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 14
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.add("own:" + parent.modelData.id) }
                    }
                }
                Text { text: "MADE BY PEOPLE"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4; topPadding: 8 }
                Text { width: lc.width; wrapMode: Text.WordWrap; text: "Blocks people share with Hearth show here once Hearth's makers have checked them."; color: "#8a8079"; font.family: Theme.font; font.pixelSize: 13 }
            }
        }
    }
    Rectangle {
        id: ghost
        property string label: ""
        property string kind: ""
        visible: false; z: 50
        width: 160; height: 44; radius: 6; color: Qt.rgba(1, 0.48, 0.18, 0.25); border.color: "#ff7a2f"; border.width: 2
        Text { anchors.centerIn: parent; text: ghost.label; color: "white"; font.family: Theme.font; font.pixelSize: 14 }
    }

    // ================================================================ the screen (middle)
    Item {
        id: middle
        visible: !ed.made
        anchors { left: left.right; right: right.left; top: top.bottom; bottom: parent.bottom; margins: 30 }
        Rectangle {
            id: canvasBox
            readonly property real ratio: 16 / 9
            width: Math.min(parent.width, (parent.height - 90) * ratio); height: width / ratio
            anchors.horizontalCenter: parent.horizontalCenter
            radius: 10; clip: true
            gradient: Gradient {
                GradientStop { position: 0; color: (ed.draft.background && ed.draft.background.from) || "#10243a" }
                GradientStop { position: 1; color: (ed.draft.background && (ed.draft.background.kind === "colour" ? ed.draft.background.from : ed.draft.background.to)) || "#1a0d07" }
            }
            Item {
                id: canvas
                anchors.fill: parent
                readonly property real cw: width / ed.draft.grid.cols
                readonly property real ch: height / ed.draft.grid.rows
                // the grid
                Repeater { model: ed.draft.grid.cols - 1; delegate: Rectangle { required property int index; x: (index + 1) * canvas.cw; width: 1; height: canvas.height; color: Qt.rgba(1, 1, 1, 0.05) } }
                Repeater { model: ed.draft.grid.rows - 1; delegate: Rectangle { required property int index; y: (index + 1) * canvas.ch; height: 1; width: canvas.width; color: Qt.rgba(1, 1, 1, 0.05) } }
                MouseArea { anchors.fill: parent; onClicked: ed.pick = -1 }
                Repeater {
                    model: ed.draft.blocks || []
                    delegate: Rectangle {
                        id: blkBox
                        required property var modelData
                        required property int index
                        readonly property bool on: index === ed.pick
                        x: modelData.x * canvas.cw + 4; y: modelData.y * canvas.ch + 4
                        width: modelData.w * canvas.cw - 8; height: modelData.h * canvas.ch - 8
                        radius: Math.min(10, (modelData.style && modelData.style.corners) || 6)
                        color: modelData.style && modelData.style.colour && modelData.style.colour !== "#00000000" ? Qt.rgba(Qt.color(modelData.style.colour).r, Qt.color(modelData.style.colour).g, Qt.color(modelData.style.colour).b, 0.45) : Qt.rgba(0, 0, 0, 0.25)
                        border.width: on ? 2 : 1
                        border.color: on ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.35)
                        Text {
                            anchors { left: parent.left; bottom: parent.bottom; margins: 12 }
                            visible: blkBox.modelData.block === "hero"
                            text: "Hero tile · your last game"; color: "white"; font.family: Theme.font; font.pixelSize: 16; font.bold: true
                        }
                        Text {
                            anchors.centerIn: parent; width: parent.width - 16; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
                            visible: blkBox.modelData.block !== "hero"
                            text: ed.info(blkBox.modelData.block).t + (blkBox.modelData.title ? " · " + blkBox.modelData.title : blkBox.modelData.items ? " · " + blkBox.modelData.items.length + " things" : "")
                            color: "white"; font.family: Theme.font; font.pixelSize: 14
                        }
                        // move it
                        MouseArea {
                            anchors.fill: parent; cursorShape: Qt.SizeAllCursor
                            property real sx: 0; property real sy: 0; property int ox: 0; property int oy: 0; property bool dragged: false
                            onPressed: mouse => { ed.pick = blkBox.index; sx = mouse.x; sy = mouse.y; ox = blkBox.modelData.x; oy = blkBox.modelData.y; dragged = false; }
                            onPositionChanged: mouse => {
                                const dx = Math.round((mouse.x - sx) / canvas.cw), dy = Math.round((mouse.y - sy) / canvas.ch);
                                const nx = Math.max(0, Math.min(ed.draft.grid.cols - blkBox.modelData.w, ox + dx)), ny = Math.max(0, Math.min(ed.draft.grid.rows - blkBox.modelData.h, oy + dy));
                                if (nx !== blkBox.modelData.x || ny !== blkBox.modelData.y) {
                                    if (!dragged) { ed.undoStack = ed.undoStack.concat([JSON.stringify(ed.draft)]); dragged = true; }
                                    const d = ed.copy(ed.draft); d.blocks[blkBox.index].x = nx; d.blocks[blkBox.index].y = ny; ed.draft = d;
                                }
                            }
                        }
                        // its handles: top-left and bottom-right (size it)
                        Repeater {
                            model: blkBox.on ? ["tl", "br"] : []
                            delegate: Rectangle {
                                required property var modelData
                                width: 12; height: 12; color: "#ff7a2f"; radius: 2
                                x: modelData === "tl" ? -6 : blkBox.width - 6; y: modelData === "tl" ? -6 : blkBox.height - 6
                                MouseArea {
                                    anchors.fill: parent; anchors.margins: -6
                                    cursorShape: parent.modelData === "tl" ? Qt.SizeFDiagCursor : Qt.SizeFDiagCursor
                                    property real sx: 0; property real sy: 0; property var o: null; property bool dragged: false
                                    onPressed: mouse => { const p = mapToItem(canvas, mouse.x, mouse.y); sx = p.x; sy = p.y; o = ed.copy(blkBox.modelData); dragged = false; }
                                    onPositionChanged: mouse => {
                                        const p = mapToItem(canvas, mouse.x, mouse.y);
                                        const dx = Math.round((p.x - sx) / canvas.cw), dy = Math.round((p.y - sy) / canvas.ch);
                                        const g = ed.draft.grid; const b = ed.copy(o);
                                        if (parent.modelData === "br") { b.w = Math.max(1, Math.min(g.cols - b.x, o.w + dx)); b.h = Math.max(1, Math.min(g.rows - b.y, o.h + dy)); }
                                        else { const nx = Math.max(0, Math.min(o.x + o.w - 1, o.x + dx)), ny = Math.max(0, Math.min(o.y + o.h - 1, o.y + dy)); b.w = o.w + (o.x - nx); b.h = o.h + (o.y - ny); b.x = nx; b.y = ny; }
                                        if (b.x !== blkBox.modelData.x || b.y !== blkBox.modelData.y || b.w !== blkBox.modelData.w || b.h !== blkBox.modelData.h) {
                                            if (!dragged) { ed.undoStack = ed.undoStack.concat([JSON.stringify(ed.draft)]); dragged = true; }
                                            const d = ed.copy(ed.draft); d.blocks[blkBox.index] = b; ed.draft = d;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // what went wrong (or what was done)
        Text {
            anchors { top: canvasBox.bottom; topMargin: 12; left: canvasBox.left; right: canvasBox.right }
            visible: ed.error !== "" || ed.busy
            text: ed.busy ? "Your AI is making it…" : ed.error; wrapMode: Text.WordWrap
            color: ed.busy ? "#c9bfb6" : "#ff9a52"; font.family: Theme.font; font.pixelSize: 14
        }
        // Or say it
        Rectangle {
            anchors { bottom: parent.bottom; left: canvasBox.left; right: canvasBox.right }
            height: 52; radius: 10; color: "#211c19"; border.width: 2; border.color: "#ff7a2f"
            TextInput {
                id: sayField
                anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                verticalAlignment: TextInput.AlignVCenter
                color: "white"; font.family: Theme.font; font.pixelSize: 16; clip: true
                onAccepted: ed.sayIt()
                Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter; visible: !sayField.text && !sayField.activeFocus; text: "Or say it: \"make the hero tile bigger and add my printer's camera in the corner\"…"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 16 }
            }
        }
    }

    // ================================================================ the picked block (right)
    Rectangle {
        id: right
        visible: !ed.made
        anchors { right: parent.right; top: top.bottom; bottom: parent.bottom }
        width: 380; color: "#1f1a17"
        Text { visible: !ed.cur; anchors.centerIn: parent; width: parent.width - 60; wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter; text: "Pick a block on the screen, or add one from the left."; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 14 }
        Flickable {
            visible: !!ed.cur
            anchors { fill: parent; margins: 22 }
            contentHeight: rc.implicitHeight; clip: true
            Column {
                id: rc
                width: parent.width
                spacing: 10
                Text { text: ed.cur ? ed.info(ed.cur.block).t.toUpperCase() : ""; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 13; font.letterSpacing: 1.4 }
                Head { text: "SHOWS" }
                Pill {
                    readonly property var opts: ed.cur ? ed.info(ed.cur.block).shows : []
                    visible: opts.length > 0
                    text: ed.cur ? ed.labelOf(opts, ed.cur.shows || "") || "—" : ""
                    onClicked: ed.setField("shows", ed.nextOf(opts, ed.cur.shows || ""))
                }
                // words: a title, the text (notes and words)
                Rectangle {
                    visible: !!ed.cur && ["notes", "text", "tilerow", "hero", "windows", "activity"].indexOf(ed.cur.block) >= 0
                    width: rc.width; height: 34; radius: 6; color: "#2a2420"
                    TextInput {
                        anchors { fill: parent; leftMargin: 10; rightMargin: 10 } verticalAlignment: TextInput.AlignVCenter; clip: true
                        text: ed.cur ? (ed.cur.title || "") : ""; color: "white"; font.family: Theme.font; font.pixelSize: 13
                        onEditingFinished: if (ed.cur && text !== (ed.cur.title || "")) ed.setField("title", text)
                        Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter; visible: !parent.text; text: "Its title (\"Jump back in\")"; color: "#8a8079"; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
                Rectangle {
                    visible: !!ed.cur && ["notes", "text"].indexOf(ed.cur.block) >= 0
                    width: rc.width; height: 90; radius: 6; color: "#2a2420"
                    TextEdit {
                        anchors { fill: parent; margins: 10 } clip: true; wrapMode: TextEdit.WordWrap
                        text: ed.cur ? (ed.cur.value || "") : ""; color: "white"; font.family: Theme.font; font.pixelSize: 13
                        onActiveFocusChanged: if (!activeFocus && ed.cur && text !== (ed.cur.value || "")) ed.setField("value", text)
                    }
                }
                Head { text: "LOOK" }
                Row {
                    spacing: 8
                    Repeater {
                        model: ed.colours.slice(0, 4).concat(["more"])
                        delegate: Rectangle {
                            required property var modelData
                            readonly property string c: modelData === "more" ? "" : modelData
                            readonly property bool on: !!(ed.cur && ed.cur.style && ed.cur.style.colour === c)
                            width: 28; height: 28; radius: 14
                            color: modelData === "more" ? "transparent" : modelData
                            border.width: on ? 3 : (modelData === "more" ? 0 : 1); border.color: on ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.3)
                            gradient: modelData === "more" ? rainbow : null
                            Gradient { id: rainbow; orientation: Gradient.Horizontal; GradientStop { position: 0; color: "#ff4d4d" } GradientStop { position: 0.33; color: "#ffd24d" } GradientStop { position: 0.66; color: "#4dff88" } GradientStop { position: 1; color: "#4d7aff" } }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.setStyle("colour", parent.modelData === "more" ? ed.nextOf(ed.colours, (ed.cur.style || {}).colour || "") : parent.c) }
                        }
                    }
                }
                Slide { label: "Corners"; max: 64; value: ed.cur && ed.cur.style ? (ed.cur.style.corners || 0) : 0; onMoved: v => ed.setStyle("corners", v) }
                Slide { label: "See-through"; max: 100; value: ed.cur && ed.cur.style ? (ed.cur.style.seeThrough || 0) : 0; onMoved: v => ed.setStyle("seeThrough", v) }
                Line { label: "Font"; value: ed.cur && ed.cur.style && ed.cur.style.font ? ed.cur.style.font : "The look's own"; onClicked: ed.setStyle("font", ed.nextOf(ed.fonts, (ed.cur.style || {}).font || "")) }
                Line { label: "On focus"; value: ed.labelOf(ed.focuses, ed.cur && ed.cur.style ? ed.cur.style.focus || "grow" : "grow"); onClicked: ed.setStyle("focus", ed.nextOf(ed.focuses, (ed.cur.style || {}).focus || "grow")) }
                Line {
                    label: "Sound"; value: ed.labelOf(ed.sounds, ed.cur && ed.cur.style ? ed.cur.style.sound || "" : "")
                    onClicked: { const n = ed.nextOf(ed.sounds, (ed.cur.style || {}).sound || ""); ed.setStyle("sound", n); if (n && n !== "none") Sounds.preview("block-" + n, "hearth"); }
                }
                Line { label: "Animation"; value: ed.labelOf(ed.anims, ed.cur && ed.cur.style ? ed.cur.style.animation || "" : ""); onClicked: ed.setStyle("animation", ed.nextOf(ed.anims, (ed.cur.style || {}).animation || "")) }
                Head { text: "PLACE" }
                Text { text: ed.cur ? "Column " + (ed.cur.x + 1) + ", row " + (ed.cur.y + 1) + " · " + ed.cur.w + " × " + ed.cur.h + " squares" : ""; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 14 }
                Text {
                    text: "Above other windows: " + (ed.cur && ed.cur.style && ed.cur.style.aboveWindows ? "yes (over them)" : "no (under them)")
                    color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 14
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.setStyle("aboveWindows", !(ed.cur.style && ed.cur.style.aboveWindows) ? true : "") }
                }
                Head { text: "WHEN PRESSED" }
                Pill { text: ed.cur ? ed.labelOf(ed.presses, ed.cur.press ? ed.cur.press.action : "") : ""; onClicked: { const n = ed.nextOf(ed.presses, ed.cur.press ? ed.cur.press.action : ""); ed.setField("press", n ? { action: n } : ""); } }
                Item { width: 1; height: 14 }
                Pill { text: "Remove this block"; onClicked: ed.remove() }
            }
        }
    }

    // ================================================================ board 5: a block your AI made
    Item {
        visible: !!ed.made
        anchors { fill: parent; topMargin: 56 + 50; leftMargin: 100; rightMargin: 100 }
        Column {
            id: madeLeft
            width: parent.width * 0.47
            spacing: 14
            Text { text: "MADE FOR YOU BY YOUR AI · NEW"; color: "#ff7a2f"; font.family: Theme.font; font.pixelSize: 14; font.bold: true; font.letterSpacing: 1.2 }
            Loader {
                width: parent.width; height: width * 0.52
                active: !!ed.made
                sourceComponent: Component {
                    OwnBlock { blk: ({ block: "own:" + ed.made.block.id }); st: ({ corners: 12 }); ink: "white" }
                }
            }
            Text {
                width: parent.width; wrapMode: Text.WordWrap; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 15
                text: ed.made ? ed.made.block.name + " as a block on your desktop. Made by " + (ed.made.block.by || "your AI") + "; it's yours, no one else's desktop changes." : ""
            }
        }
        Column {
            anchors { left: madeLeft.right; leftMargin: 60; right: parent.right }
            spacing: 14
            Text { text: "WHAT YOU ASKED"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 13; font.letterSpacing: 1.2 }
            Rectangle {
                width: parent.width; height: askedT.implicitHeight + 40; radius: 8; color: "#221d1a"; border.color: Qt.rgba(1, 1, 1, 0.08)
                Text { id: askedT; anchors { fill: parent; margins: 20 } wrapMode: Text.WordWrap; text: ed.made ? "\"" + ed.made.asked + "\"" : ""; color: "white"; font.family: Theme.font; font.pixelSize: 19 }
            }
            Text { text: "IT RUNS ON ITS OWN, SO NOTHING ELSE CAN BREAK"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 13; font.letterSpacing: 1.2; topPadding: 8 }
            Rectangle {
                width: parent.width; height: 130; radius: 8; color: "#221d1a"; border.color: Qt.rgba(1, 1, 1, 0.08)
                Column {
                    anchors { fill: parent; margins: 20 }
                    spacing: 12
                    Repeater {
                        model: ["Its own small program: stopped by itself if it uses too much processor or memory", "Can't touch starting up, Hearth's system or your settings", "Turn it off any time in Settings › Your desktop › Your blocks"]
                        delegate: Text { required property var modelData; text: "✓  " + modelData; color: "white"; font.family: Theme.font; font.pixelSize: 15 }
                    }
                }
            }
            Rectangle {
                visible: !!ed.made && !!ed.made.asks
                width: parent.width; height: 132; radius: 8; color: "#2c2418"; border.color: Qt.rgba(1, 0.75, 0.3, 0.5)
                Column {
                    anchors { fill: parent; margins: 20 }
                    spacing: 8
                    Text { text: ed.made && ed.made.asks ? ed.made.asks.title : ""; color: "white"; font.family: Theme.font; font.pixelSize: 17; font.bold: true }
                    Text { text: ed.made && ed.made.asks ? ed.made.asks.sub : ""; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 14 }
                    Row {
                        spacing: 10; topPadding: 6
                        Repeater {
                            model: [{ t: "Allow", a: "always" }, { t: "Just this once", a: "once" }, { t: "No", a: "no" }]
                            delegate: Rectangle {
                                required property var modelData
                                required property int index
                                width: at.implicitWidth + 36; height: 36; radius: 6
                                color: index === 0 ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.07); border.width: index === 0 ? 0 : 1; border.color: Qt.rgba(1, 1, 1, 0.18)
                                Text { id: at; anchors.centerIn: parent; text: modelData.t; color: index === 0 ? "#1b1006" : "white"; font.family: Theme.font; font.pixelSize: 15; font.bold: true }
                                MouseArea {
                                    anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                                    onClicked: if (modelData.a !== "once") Quickshell.execDetached(ScreenLayout.layoutCmd(["block-allow", ed.made.block.id, modelData.a]))
                                }
                            }
                        }
                    }
                }
            }
            Row {
                width: parent.width; spacing: 10
                Repeater {
                    model: [{ t: "Keep this", a: "keep" }, { t: "Change it…", a: "change" }, { t: "Try again", a: "again" }, { t: "Undo", a: "undo" }]
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        width: kt.implicitWidth + 36; height: 38; radius: 6
                        color: index === 0 ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.07); border.width: index === 0 ? 0 : 1; border.color: Qt.rgba(1, 1, 1, 0.18)
                        Text { id: kt; anchors.centerIn: parent; text: modelData.t; color: index === 0 ? "#1b1006" : "white"; font.family: Theme.font; font.pixelSize: 15; font.bold: true }
                        MouseArea {
                            anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (modelData.a === "keep") { ed.made = null; ed.refreshMine(); }
                                else if (modelData.a === "change") { sayField.text = ed.made.asked + ", and "; ed.made = null; sayField.forceActiveFocus(); }
                                else if (modelData.a === "again") { sayField.text = ed.made.asked; ed.made = null; ed.sayIt(); }
                                else { Quickshell.execDetached(ScreenLayout.layoutCmd(["block-on", ed.made.block.id, "false"])); ed.made = null; ed.undo(); ed.refreshMine(); }
                            }
                        }
                    }
                }
                Item { width: 1; height: 1 }
            }
            Rectangle {
                width: st.implicitWidth + 36; height: 38; radius: 6; color: Qt.rgba(1, 1, 1, 0.07); border.color: Qt.rgba(1, 1, 1, 0.18)
                Text { id: st; anchors.centerIn: parent; text: "Share it with Hearth…"; color: "white"; font.family: Theme.font; font.pixelSize: 15; font.bold: true }
                MouseArea {
                    anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        const idea = { format: 1, part: "screen", asked: ed.made.asked, needs: ["A block made by your AI: " + ed.made.block.name], sketch: false, hearth: "", app: "Mantel", kept: true };
                        Quickshell.execDetached(ScreenLayout.layoutCmd(["send-idea", JSON.stringify(idea)]));
                        ed.error = "Sent to Hearth's makers ✓";
                    }
                }
            }
        }
    }
}
