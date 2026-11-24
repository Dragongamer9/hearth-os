// Build anything board 4 (and 5): the block editor. Any block anywhere on a grid, any size, any colour, corners, font,
// see-through, sound and animation; what it shows and what a press does; above or under your windows. Blocks on the
// left (drag one onto the screen, or press it), the screen in the middle (drag a block to move it, its corners to size
// it), the picked block's Shows, Look, Place and When pressed on the right, and "Or say it" under the screen: your AI
// changes the screen (hearth-layout describe-screen) and, when Hearth hasn't got a block for it, makes one (board 5:
// MADE FOR YOU BY YOUR AI, what you asked, how it's boxed in, its ask, Keep this / Change it / Try again / Undo / Share).
// Undo, Try it full screen, Share it, Keep this. Nothing changes your desktop until Keep this (ScreenLayout.keep).
// 1.18, lock mode (openLock; docs/mockups/checklists/lock-screen-blocks.md board 3): the same editor for the lock
// screen. Only blocks that show things (Clock, Date, Weather, Media, Notifications, Downloads, Battery, Calendar,
// Picture, Words, Friends online, your AI's own); the password block always there (moved, sized and styled, never
// removed or covered); Match your desktop; Undo, Try it (locks for real), Keep this; Or say it; the picked block's
// Shows, Look and the Privacy of the whole lock screen. hearth-lockscreen makes every layout safe before it's tried or
// kept (nothing on it can open an app, a file or a setting) and keeps it (atomic, merged, a .bak kept).
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
    // ---- lock mode
    property bool lockMode: false
    property bool lockMatch: false
    property string lockFrom: ""
    readonly property string lockHelper: decodeURIComponent(Qt.resolvedUrl("hearth-lockscreen").toString().replace(/^file:\/\//, ""))
    readonly property string deskName: ScreenLayout.on ? ((ScreenLayout.readyMade.find(r => r.id === ScreenLayout.from) || { name: "your desktop" }).name) : "Hearth OS Mantel"
    readonly property var lockCatalog: [
        { b: "clock", t: "Clock" }, { b: "date", t: "Date" }, { b: "weather", t: "Weather" }, { b: "media", t: "Media" },
        { b: "notifications", t: "Notifications" }, { b: "downloads", t: "Downloads" }, { b: "battery", t: "Battery" },
        { b: "calendar", t: "Calendar" }, { b: "picture", t: "Picture" }, { b: "text", t: "Words" }, { b: "friends", t: "Friends online" },
        { b: "own", t: "Your AI's own..." }
    ]
    readonly property var lockNames: ({ date: "Date", notifications: "Notifications", calendar: "Calendar", password: "Password and fingerprint", bar: "Locked bar" })
    readonly property var privNotes: [["names", "names only"], ["messages", "messages too"], ["count", "just how many"], ["none", "hidden"]]
    readonly property var lockSizes: ({ clock: [8, 3], date: [8, 1], weather: [5, 2], media: [5, 2], notifications: [5, 3], downloads: [6, 2], battery: [6, 1],
                                        calendar: [5, 3], picture: [6, 4], text: [8, 1], friends: [5, 3] })
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
    function info(b) {
        if (lockMode && lockNames[b]) return { b: b, t: lockNames[b], shows: [] };
        if (lockMode && b === "friends") return { b: b, t: "Friends online", shows: [] };
        if (lockMode && b === "text") return { b: b, t: "Words", shows: [] };
        return catalog.concat(more).find(c => c.b === b) || { b: b, t: String(b).startsWith("own:") ? "Made by your AI" : b, shows: [] }; }
    readonly property var colours: ["#ffffff", "#2f9e5a", "#ff7a2f", "#5b8cff", "#b07cff", "#f2c14e", "#e05a6a", "#0b3a44", "#1f3a2e", "#00000000"]
    readonly property var fonts: ["", "Inter", "Inter Display", "Noto Serif", "Noto Sans Mono", "Pixel", "Rounded", "Condensed"]
    readonly property var focuses: [["grow", "Grows, white outline"], ["outline", "White outline"], ["glow", "Glows"], ["lift", "Lifts"], ["none", "Nothing"]]
    readonly property var sounds: [["", "The look's own"], ["soft", "Soft click (Hearth's)"], ["click", "Click"], ["pop", "Pop"], ["chime", "Chime"], ["wood", "Wood"], ["retro", "Retro"], ["none", "No sound"]]
    readonly property var anims: [["", "The look's own"], ["slide-left", "Slides in from the left"], ["slide-right", "Slides in from the right"], ["slide-up", "Slides up"], ["slide-down", "Slides down"], ["fade", "Fades in"], ["zoom", "Zooms in"], ["none", "Just there"]]
    readonly property var presses: [["", "What it does anyway"], ["play", "Play it"], ["menu", "Open the app menu"], ["game-mode", "Game Mode"], ["quickaccess", "Quick Access"], ["overview", "All your windows"], ["screenshot", "Take a screenshot"], ["lock", "Lock the screen"], ["none", "Nothing"]]

    function copy(o) { return JSON.parse(JSON.stringify(o)); }
    function open(screenObj, title) {
        lockMode = false;
        draft = copy(screenObj || ScreenLayout.screen || { grid: { cols: 24, rows: 14 }, background: { kind: "gradient", from: "#10243a", to: "#1a0d07" }, look: {}, blocks: [] });
        if (!draft.grid) draft.grid = { cols: 24, rows: 14 };
        if (!draft.blocks) draft.blocks = [];
        name = title || (ScreenLayout.readyMade.find(r => r.id === ScreenLayout.from) || { name: draft.blocks.length ? "Your desktop" : "Start from nothing" }).name;
        said = draft.said || "";
        undoStack = []; pick = draft.blocks.length ? 0 : -1; error = ""; made = null; sayField.text = "";
        shown = true;
        refreshMine();
    }
    // The lock screen editor: your own lock screen as kept, else the one matching your desktop.
    FileView { id: lockView; path: Quickshell.env("HOME") + "/.local/share/nimbus/lock-screen.json"; blockLoading: true; watchChanges: false }
    function openLock() {
        let o = null;
        try { lockView.reload(); const t = lockView.text(); o = t && t.trim() ? JSON.parse(t) : null; } catch (e) { o = null; }
        lockMatch = !!o && o.match === true;
        if (o && o.on === true && o.match !== true && o.screen && Array.isArray(o.screen.blocks)) { lockOpenWith(o.screen, o.screen.name || "Your own", o.from || ""); return; }
        lockReady(ScreenLayout.on && ScreenLayout.from ? ScreenLayout.from : "hearth");
    }
    function lockReady(id) { lockReadyP.wanted = id; lockReadyP.command = ["python3", lockHelper, "ready", id]; lockReadyP.running = true; }
    Process {
        id: lockReadyP
        property string wanted: ""
        stdout: StdioCollector {
            onStreamFinished: {
                let o = null;
                try { o = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { o = null; }
                if (!o || !Array.isArray(o.blocks)) { ed.error = "The lock screen couldn't be read."; return; }
                ed.lockOpenWith(o, o.name || lockReadyP.wanted, lockReadyP.wanted);
            }
        }
    }
    function lockOpenWith(o, title, from) {
        lockMode = true;
        draft = copy(o);
        if (!draft.grid) draft.grid = { cols: 24, rows: 14 };
        if (!draft.privacy) draft.privacy = { notifications: "names", media: true };
        name = title || "Your own"; lockFrom = from || "";
        said = draft.said || "";
        undoStack = []; error = ""; made = null; sayField.text = "";
        pick = Math.max(0, draft.blocks.findIndex(b => b.block === "weather"));
        shown = true;
    }
    function matchDesktop() { lockMatch = true; lockReady(ScreenLayout.on && ScreenLayout.from ? ScreenLayout.from : "hearth"); }
    function pwIndex() { return (draft.blocks || []).findIndex(b => b.block === "password"); }
    // a block moved or sized over the password block: not allowed (it's never covered)
    function coversPw(b, i) {
        if (!lockMode || !b || b.block === "password") return false;
        const p = draft.blocks[pwIndex()];
        return !!p && i !== pwIndex() && b.x < p.x + p.w && p.x < b.x + b.w && b.y < p.y + p.h && p.y < b.y + b.h;
    }
    function setPrivacy(k, v) { change(d => { d.privacy = Object.assign({ notifications: "names", media: true }, d.privacy || {}); d.privacy[k] = v; }); }
    function lockCheck(o, after) { lockCheckP.after = after; lockCheckP.command = ["python3", lockHelper, "check", JSON.stringify(o)]; lockCheckP.running = true; }
    Process {
        id: lockCheckP
        property string after: ""
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
                if (!r || r.ok !== true) { ed.error = (r && (r.error || (r.problems || []).join(" "))) || "That lock screen couldn't be checked."; return; }
                if (lockCheckP.after === "said") {
                    ed.change(d => { for (const k in d) delete d[k]; for (const k in r.screen) d[k] = r.screen[k]; });
                    ed.pick = Math.max(0, ed.draft.blocks.length - 1);
                    ed.error = (r.problems || []).join(" ");
                }
            }
        }
    }
    function lockTry() {
        lockTryP.command = ["python3", lockHelper, "try", JSON.stringify(draft)];
        lockTryP.running = true;
    }
    Process {
        id: lockTryP
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
                if (!r || r.ok !== true) { ed.error = (r && r.error) || "It couldn't be tried."; return; }
                // locks for real: Hearth's own lock (Lock.qml) with this layout, once; your password unlocks as always
                Quickshell.execDetached(["loginctl", "lock-session"]);
            }
        }
    }
    function lockKeep() {
        lockKeepP.command = ["python3", lockHelper, "keep", JSON.stringify({ screen: Object.assign(copy(draft), { name: name, said: said || draft.said || "" }), match: lockMatch, from: lockFrom })];
        lockKeepP.running = true;
    }
    Process {
        id: lockKeepP
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
                if (!r || r.ok !== true) { ed.error = (r && r.error) || "Your lock screen couldn't be kept."; return; }
                ed.shown = false;
                SettingsNav.toast("Your lock screen is kept ✓ (Super+L to see it)");
            }
        }
    }
    // the weather block's Place: looked up by name (Open-Meteo, through hearth-widgets)
    Process {
        id: placeP
        property string asked: ""
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const o = JSON.parse(text.trim().split("\n").pop() || "{}");
                    const p = (o.places || [])[0];
                    if (!p) { ed.error = "No place called \"" + placeP.asked + "\" was found."; return; }
                    ed.change(d => { const b = d.blocks[ed.pick]; b.place = p.name; b.lat = p.lat; b.lon = p.lon; });
                } catch (e) { ed.error = "The place couldn't be looked up."; }
            }
        }
    }
    function findPlace(t) {
        if (!t.trim()) return;
        placeP.asked = t.trim();
        placeP.command = ["python3", decodeURIComponent(Qt.resolvedUrl("hearth-widgets").toString().replace(/^file:\/\//, "")), "places", t.trim()];
        placeP.running = true;
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
    function nextNum(list, v) { const i = list.indexOf(v); return list[(i + 1) % list.length]; }
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
        const size = lockMode ? (lockSizes[b] || [4, 2]) : ({ hero: [8, 5], tilerow: [16, 4], iconrow: [14, 3], tabs: [10, 1], guide: [6, 14], clock: [5, 1], profile: [6, 1], battery: [3, 1], bar: [24, 1],
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
        if (lockMode && b === "weather") nb.tonight = true;
        if (lockMode && b === "text") nb.value = "Back soon.";
        if (lockMode && coversPw(nb, -1)) { error = "There's no room there: nothing can cover the password block."; return; }
        // the password block stays last: always on top
        change(d => { const i = d.blocks.findIndex(x => x.block === "password"); if (lockMode && i >= 0) d.blocks.splice(i, 0, nb); else d.blocks.push(nb); });
        if (lockMode) { pick = draft.blocks.findIndex(x => x.id === id); return; }
        pick = draft.blocks.length - 1;
    }
    function remove() {
        if (!cur) return;
        if (lockMode && cur.block === "password") { error = "The password block is always there: it can be moved, sized and styled, never removed."; return; } const i = pick; change(d => d.blocks.splice(i, 1)); pick = Math.min(i, draft.blocks.length - 1); }

    // ---- your AI: the screen changed (and blocks made) from your words
    function say(t) { sayField.text = t; sayIt(); }
    function sayIt() {
        const t = sayField.text.trim();
        if (!t || busy) return;
        busy = true; error = "";
        // Which app is asking (Describe it's notes), passed with env: no environment object on the Process
        // (test_desktop_bug_hunt_2).
        const asked = lockMode ? "This is the LOCK SCREEN, not the desktop: only blocks that show things (clock, date, weather, media, notifications, downloads, battery, calendar, picture, text, friends), nothing pressed opens anything, and the password block stays. " + t : t;
        describeP.command = ["env", "HEARTH_DESCRIBE_APP=Mantel " + (Quickshell.env("HEARTH_MANTEL_VERSION") || "")].concat(ScreenLayout.layoutCmd(["describe-screen", asked, JSON.stringify(draft)]));
        describeP.asked = t;
        describeP.running = true;
    }
    Process {
        id: describeP
        property string asked: ""
        stdout: StdioCollector {
            onStreamFinished: {
                ed.busy = false;
                let r = null;
                try { r = JSON.parse(text.trim().split("\n").pop() || "{}"); } catch (e) { r = null; }
                if (!r || r.ok !== true) { ed.error = (r && r.error) || "Your AI didn't answer."; return; }
                if (ed.lockMode) {
                    // made safe first (hearth-lockscreen check): the password block kept, nothing that opens anything
                    ed.said = describeP.asked;
                    ed.lockCheck(Object.assign({}, r.screen || {}, { privacy: ed.draft.privacy, name: ed.name }), "said");
                    return;
                }
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
        else if ((e.key === Qt.Key_Delete || e.key === Qt.Key_Backspace) && !sayField.activeFocus && !placeField.activeFocus && ed.cur) { ed.remove(); e.accepted = true; }
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

    // a row: its name on the left, its value on the right (lock mode's panel)
    component Kv: Item {
        property string label: ""
        property string value: ""
        property bool interactive: true
        signal clicked()
        width: parent ? parent.width : 336; height: 38
        Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Qt.rgba(1, 1, 1, 0.08) }
        Text { anchors { left: parent.left; verticalCenter: parent.verticalCenter } text: parent.label; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 15 }
        Text { anchors { right: parent.right; verticalCenter: parent.verticalCenter } text: parent.value; color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 15 }
        MouseArea { anchors.fill: parent; z: -1; enabled: parent.interactive; cursorShape: Qt.PointingHandCursor; onClicked: parent.clicked() }
    }

    // ================================================================ the top bar
    Rectangle {
        id: top
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 56; color: "#211c19"
        Row {
            anchors { left: parent.left; leftMargin: 24; verticalCenter: parent.verticalCenter }
            spacing: 14
            Text { text: ed.made ? "Editing your desktop" : ed.lockMode ? "Lock screen" : "Your desktop"; color: "white"; font.family: Theme.font; font.pixelSize: 17; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
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
                model: ed.made ? [{ t: "Done", a: "done" }] : ed.lockMode ? [{ t: "Undo", a: "undo" }, { t: "Try it (locks for real)", a: "locktry" }, { t: "Keep this", a: "keep" }] : [{ t: "Undo", a: "undo" }, { t: "Try it full screen", a: "full" }, { t: "Share it", a: "share" }, { t: "Keep this", a: "keep" }]
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool primary: modelData.a === "keep"
                    width: bt.implicitWidth + 36; height: 36; radius: 6
                    color: primary ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.07); border.width: primary ? 0 : 1; border.color: Qt.rgba(1, 1, 1, 0.18)
                    opacity: modelData.a === "undo" && !ed.undoStack.length ? 0.45 : 1
                    Text { id: bt; anchors.centerIn: parent; text: modelData.t; color: parent.primary ? "#1b1006" : "white"; font.family: Theme.font; font.pixelSize: 15; font.bold: true }
                    MouseArea {
                        anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                        onClicked: modelData.a === "undo" ? ed.undo() : modelData.a === "full" ? ed.tryFull() : modelData.a === "locktry" ? ed.lockTry() : modelData.a === "share" ? ed.share() : modelData.a === "keep" ? (ed.lockMode ? ed.lockKeep() : ed.keep()) : (ed.made = null)
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
        // lock mode: "Your lock screen", its blocks, Always there, Match your desktop
        Flickable {
            visible: ed.lockMode
            anchors { fill: parent; margins: 20 }
            contentHeight: lkc.implicitHeight; clip: true
            Column {
                id: lkc
                width: parent.width
                spacing: 12
                Text { text: "Your lock screen"; color: "white"; font.family: Theme.font; font.pixelSize: 19; font.bold: true }
                Text { text: "BLOCKS · DRAG ONTO IT"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4 }
                Flow {
                    width: lkc.width; spacing: 8
                    Repeater {
                        model: ed.lockCatalog
                        delegate: Rectangle {
                            id: lchip
                            required property var modelData
                            width: lct.implicitWidth + 26; height: 34; radius: 8; color: "#2a2420"; border.color: Qt.rgba(1, 1, 1, 0.08)
                            Text { id: lct; anchors.centerIn: parent; text: lchip.modelData.t; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 14 }
                            MouseArea {
                                anchors.fill: parent; cursorShape: Qt.OpenHandCursor
                                property bool moved: false
                                onPressed: { moved = false; ghost.label = lchip.modelData.t; ghost.kind = lchip.modelData.b; }
                                onPositionChanged: mouse => {
                                    const p = mapToItem(null, mouse.x, mouse.y);
                                    if (!moved && (Math.abs(mouse.x - width / 2) > 6 || Math.abs(mouse.y - height / 2) > 6)) moved = true;
                                    if (moved) { ghost.x = p.x - ghost.width / 2; ghost.y = p.y - ghost.height / 2; ghost.visible = true; }
                                }
                                onReleased: mouse => {
                                    ghost.visible = false;
                                    if (!moved) { ed.add(lchip.modelData.b); return; }
                                    const p = mapToItem(canvas, mouse.x, mouse.y);
                                    if (p.x >= 0 && p.y >= 0 && p.x <= canvas.width && p.y <= canvas.height) ed.add(lchip.modelData.b, Math.floor(p.x / canvas.cw), Math.floor(p.y / canvas.ch));
                                }
                            }
                        }
                    }
                }
                Text { text: "ALWAYS THERE"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4; topPadding: 6 }
                Rectangle {
                    width: pwt.implicitWidth + 28; height: 36; radius: 8; color: Qt.rgba(1, 0.48, 0.18, 0.1); border.color: "#ff7a2f"
                    Text { id: pwt; anchors.centerIn: parent; text: "🔒 Password and fingerprint"; color: "#ffb27a"; font.family: Theme.font; font.pixelSize: 14 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.pick = ed.pwIndex() }
                }
                Text { width: lkc.width; wrapMode: Text.WordWrap; text: "It can be moved, sized and styled, never removed or covered. Unlocking is Hearth's own, whatever the look."; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 13 }
                Text { text: "MATCH YOUR DESKTOP"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 12; font.letterSpacing: 1.4; topPadding: 6 }
                Rectangle {
                    width: Math.min(lkc.width, mdt.implicitWidth + 28); height: 36; radius: 8; color: ed.lockMatch ? Qt.rgba(1, 1, 1, 0.12) : "#2a2420"; border.color: ed.lockMatch ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.08)
                    Text { id: mdt; anchors.centerIn: parent; width: Math.min(implicitWidth, lkc.width - 20); elide: Text.ElideRight; text: "Same as " + ed.deskName; color: "#e8ddd4"; font.family: Theme.font; font.pixelSize: 14 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.matchDesktop() }
                }
            }
        }
        Flickable {
            visible: !ed.lockMode
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
            // lock mode: the lock screen as it will look (LockBlocks.qml at full size, scaled down), its password block too
            Item {
                id: lockPrev
                visible: ed.lockMode
                width: 1920; height: 1080
                scale: canvasBox.width / 1920; transformOrigin: Item.TopLeft
                Loader {
                    id: lockBlocksPrev
                    anchors.fill: parent
                    active: ed.lockMode && ed.shown
                    source: "LockBlocks.qml"
                    onLoaded: { item.editing = true; item.preview = Qt.binding(() => ed.draft); }
                }
                Loader {
                    active: lockBlocksPrev.status === Loader.Ready && !!lockBlocksPrev.item && lockBlocksPrev.item.ready
                    source: "LockPassword.qml"
                    x: active ? lockBlocksPrev.item.pwRect.x : 0; y: active ? lockBlocksPrev.item.pwRect.y : 0
                    width: active ? lockBlocksPrev.item.pwRect.width : 0; height: active ? lockBlocksPrev.item.pwRect.height : 0
                    onLoaded: { item.preview = true; item.retro = Qt.binding(() => lockBlocksPrev.item.pwRetro); item.st = Qt.binding(() => lockBlocksPrev.item.pwBlock.style); }
                }
            }
            Item {
                id: canvas
                anchors.fill: parent
                anchors.margins: ed.lockMode ? Math.round(Math.min(canvasBox.width, canvasBox.height) * 0.035) : 0
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
                        color: ed.lockMode ? "transparent" : modelData.style && modelData.style.colour && modelData.style.colour !== "#00000000" ? Qt.rgba(Qt.color(modelData.style.colour).r, Qt.color(modelData.style.colour).g, Qt.color(modelData.style.colour).b, 0.45) : Qt.rgba(0, 0, 0, 0.25)
                        border.width: on ? 2 : (ed.lockMode ? 0 : 1)
                        border.color: on ? "#ff7a2f" : Qt.rgba(1, 1, 1, 0.35)
                        Text {
                            anchors { left: parent.left; bottom: parent.bottom; margins: 12 }
                            visible: blkBox.modelData.block === "hero" && !ed.lockMode
                            text: "Hero tile · your last game"; color: "white"; font.family: Theme.font; font.pixelSize: 16; font.bold: true
                        }
                        Text {
                            anchors.centerIn: parent; width: parent.width - 16; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
                            visible: blkBox.modelData.block !== "hero" && !ed.lockMode
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
                                if ((nx !== blkBox.modelData.x || ny !== blkBox.modelData.y) && !ed.coversPw(Object.assign({}, blkBox.modelData, { x: nx, y: ny }), blkBox.index)) {
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
                                        if ((b.x !== blkBox.modelData.x || b.y !== blkBox.modelData.y || b.w !== blkBox.modelData.w || b.h !== blkBox.modelData.h) && !ed.coversPw(b, blkBox.index)) {
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
                Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter; visible: !sayField.text && !sayField.activeFocus; text: ed.lockMode ? "Or say it: \"put the weather under the clock and hide my notifications\"" : "Or say it: \"make the hero tile bigger and add my printer's camera in the corner\"…"; color: "#a59a91"; font.family: Theme.font; font.pixelSize: 16 }
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
        // lock mode: the picked block's Shows and Look, the lock screen's Privacy, and the rule
        Flickable {
            visible: !!ed.cur && ed.lockMode
            anchors { fill: parent; margins: 22 }
            contentHeight: lrc.implicitHeight; clip: true
            Column {
                id: lrc
                width: parent.width
                spacing: 2
                readonly property var st: ed.cur && ed.cur.style ? ed.cur.style : ({})
                readonly property var pv: ed.draft.privacy || ({ notifications: "names", media: true })
                Text { text: ed.cur ? ed.info(ed.cur.block).t : ""; color: "white"; font.family: Theme.font; font.pixelSize: 17; font.bold: true; bottomPadding: 6 }
                Head { text: "SHOWS" }
                // Weather: its place (looked up by name) and tonight's line
                Kv {
                    visible: !!ed.cur && ed.cur.block === "weather"
                    label: "Place"; value: ""
                    TextInput {
                        id: placeField
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter }
                        width: parent.width * 0.55; horizontalAlignment: TextInput.AlignRight; clip: true
                        text: ed.cur && ed.cur.place ? ed.cur.place + " ▾" : (Settings.nightPlace ? Settings.nightPlace + " ▾" : "Type a place ▾")
                        color: "#c9bfb6"; font.family: Theme.font; font.pixelSize: 15
                        onActiveFocusChanged: if (activeFocus) selectAll()
                        onAccepted: { ed.findPlace(text.replace(/ ▾$/, "")); focus = false; }
                    }
                }
                Kv { visible: !!ed.cur && ed.cur.block === "weather"; label: "Tonight too"; value: ed.cur && ed.cur.tonight === false ? "off" : "on"; onClicked: ed.setField("tonight", !(ed.cur.tonight !== false)) }
                Kv { visible: !!ed.cur && ed.cur.block === "clock"; label: "Shows"; value: ed.cur && ed.cur.shows === "ring" ? "the time in a ring" : "the time"; onClicked: ed.setField("shows", ed.cur.shows === "ring" ? "" : "ring") }
                Kv { visible: !!ed.cur && ed.cur.block === "password"; label: "Look"; value: ed.cur && ed.cur.shows === "window" ? "a 1995 window" : "Hearth's card"; onClicked: ed.setField("shows", ed.cur.shows === "window" ? "" : "window") }
                Kv { visible: !!ed.cur && ["date", "media", "notifications", "downloads", "battery", "calendar", "friends"].indexOf(ed.cur.block) >= 0
                     label: "Shows"; value: !ed.cur ? "" : ({ date: "today's date", media: "what's playing", notifications: "while you were away", downloads: "your downloads",
                                                              battery: "this device and controllers", calendar: "today and what's next", friends: "who's online" })[ed.cur.block] || ""; interactive: false }
                Rectangle {
                    visible: !!ed.cur && (ed.cur.block === "text" || ed.cur.block === "picture")
                    width: lrc.width; height: ed.cur && ed.cur.block === "text" ? 70 : 34; radius: 6; color: "#2a2420"
                    TextEdit {
                        anchors { fill: parent; margins: 8 } clip: true; wrapMode: TextEdit.WordWrap
                        text: ed.cur ? (ed.cur.value || "") : ""; color: "white"; font.family: Theme.font; font.pixelSize: 13
                        onActiveFocusChanged: if (!activeFocus && ed.cur && text !== (ed.cur.value || "")) ed.setField("value", text)
                        Text { anchors.fill: parent; visible: !parent.text; text: ed.cur && ed.cur.block === "picture" ? "A picture of yours (~/Pictures/…)" : "Your words"; color: "#8a8079"; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
                Head { text: "LOOK" }
                Kv { label: "Corners"; value: String(lrc.st.corners !== undefined ? lrc.st.corners : 18); onClicked: ed.setStyle("corners", ed.nextNum([0, 6, 12, 18, 24, 32], lrc.st.corners !== undefined ? lrc.st.corners : 18)) }
                Kv { label: "See-through"; value: (lrc.st.seeThrough || 0) + "%"; onClicked: ed.setStyle("seeThrough", ed.nextNum([0, 25, 40, 55, 70, 85], lrc.st.seeThrough || 0)) }
                Kv { label: "Font"; value: lrc.st.font || "The look's own"; onClicked: ed.setStyle("font", ed.nextOf(ed.fonts, lrc.st.font || "")) }
                Head { text: "PRIVACY" }
                Kv { label: "Notifications"; value: ed.labelOf(ed.privNotes, lrc.pv.notifications || "names"); onClicked: ed.setPrivacy("notifications", ed.nextOf(ed.privNotes, lrc.pv.notifications || "names")) }
                Kv { label: "Media controls"; value: lrc.pv.media === false ? "off" : "on"; onClicked: ed.setPrivacy("media", lrc.pv.media === false) }
                Text {
                    width: lrc.width; wrapMode: Text.WordWrap; topPadding: 18
                    text: "Nothing on the lock screen can open an app, a file or a setting. Blocks only show; the password block unlocks."
                    color: "#a59a91"; font.family: Theme.font; font.pixelSize: 14
                }
                Item { width: 1; height: 14 }
                Pill { visible: !!ed.cur && ed.cur.block !== "password"; text: "Remove this block"; onClicked: ed.remove() }
            }
        }
        Flickable {
            visible: !!ed.cur && !ed.lockMode
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
