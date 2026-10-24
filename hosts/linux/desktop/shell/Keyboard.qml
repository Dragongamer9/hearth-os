// Desktop Mode's on-screen keyboard: Hearth's shared keyboard (osk/hearth-osk.js, a copy of ui/osk; docs/OSK.md),
// typing into whichever app has the keyboard focus. Y on the controller (or the keyboard button on the taskbar, or a
// swipe in from the left edge) opens it. It takes its own space at the bottom (or top) of the screen, so windows shrink
// and the field you're typing in stays visible.
//   Controller (hearth-pad hands it the buttons and the left stick): D-pad / left stick pick keys, A types (hold for
//   accents), X ⌫, Y space, LT shift (twice: caps), LB / RB move the caret, L3 emoji, View accents, ☰ Enter, B hides
//   it; the right stick still moves the pointer and RT clicks (into another field). Fast typing (Settings › Keyboard):
//   the Ember Wheel on the left stick, or split sticks.
//   Touch and the mouse: tap the keys; a long touch on a letter shows its accents.
// Typing goes through wtype (Wayland's virtual keyboard: any character, whatever the system layout is); without wtype
// through hearth-pad's uinput keyboard (US layout, plain characters). Settings and learned words: ~/.config/nimbus/osk.json
// and ~/.local/share/nimbus/osk-words.json, shared with Game Mode's keyboard.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "osk/hearth-osk.js" as Osk

PanelWindow {
    id: root
    property bool open: false
    property var pad: null               // Pad.qml (set by it): sends to hearth-pad
    property var engine: null
    property var vm: null
    property bool hasWtype: false
    property bool settingsLoaded: false
    property string typedLog: ""         // the last things typed (for the Desktop Mode test)
    signal closeRequested()

    readonly property string home: Quickshell.env("HOME") || "/"
    readonly property string cfgDir: (Quickshell.env("XDG_CONFIG_HOME") || (home + "/.config"))
    readonly property string dataDir: (Quickshell.env("XDG_DATA_HOME") || (home + "/.local/share"))
    readonly property var cfg: vm && vm.settings ? vm.settings : ({})

    visible: open
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-keyboard"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None       // typing goes to the window you're in
    anchors { bottom: cfg.position !== "top"; top: cfg.position === "top"; left: true; right: true }
    implicitHeight: Math.round(Math.min(screen.height * (cfg.size === "l" ? 0.5 : 0.44), Math.max(cfg.size === "s" ? 250 : 290, screen.width * (cfg.size === "s" ? 0.2 : cfg.size === "l" ? 0.29 : 0.24))))
    exclusiveZone: implicitHeight
    color: "transparent"

    OskBoard {
        id: board
        anchors { fill: parent; leftMargin: 8; rightMargin: 8; topMargin: cfg.position === "top" ? 0 : 6; bottomMargin: cfg.position === "top" ? 6 : 0 }
        vm: root.vm
        onTapped: (r, c) => { if (root.engine) { root.engine.setHints(false); root.engine.tap(r, c); } }
        onLongPressed: (r, c) => { if (root.engine) root.engine.longPress(r, c); }
        onAltPicked: i => { if (root.engine) root.engine.pickAlt(i); }
    }

    // ------------------------------------------------ the engine
    function makeEngine() {
        engine = Osk.HearthOSK.createEngine({
            host: "mantel",
            adapter: {
                insert: (text, mods) => root.typeText(text, mods),
                deleteBack: n => root.typeKey("bksp", n, null),
                key: (name, mods) => root.typeKey(name, 1, mods)
            },
            onChange: () => { root.vm = root.engine.view(); },
            onHide: () => root.closeRequested(),
            onHaptic: (strength, ms) => { if (root.pad) root.pad.send({ cmd: "rumble", strength: strength, ms: ms }); }
        });
        vm = engine.view();
    }
    Component.onCompleted: {
        Osk.HearthOSK.configure({ systemLayouts: systemLayouts(), save: d => { root.pendingLearned = d; saveLater.restart(); } });
        makeEngine();
        whichWtype.running = true;
    }

    function openKb() {
        if (!engine) makeEngine();
        engine.reset("text", "");
        engine.setHints(true);
        open = true;
        typedLog = "";
        if (pad) pad.send({ cmd: "osk", open: true, split: cfg.fast === "split" });
    }
    function closeKb() {
        open = false;
        if (pad) pad.send({ cmd: "osk", open: false });
    }
    // From hearth-pad: a button down / up, the sticks.
    function padButton(b, down) { if (open && engine) { engine.setHints(true); engine.handle(b, down, Date.now()); } }
    function padAxes(lx, ly, rx, ry) { if (open && engine) engine.axes(lx, ly, rx, ry, Date.now()); }
    Timer { interval: 16; repeat: true; running: root.open; onTriggered: if (root.engine) root.engine.tick(Date.now()) }
    // For checks (shell.qml's "keyboard" IPC).
    function holdButton(b, ms) { padButton(b, true); releaseLater.b = b; releaseLater.interval = Math.max(20, ms); releaseLater.restart(); }
    Timer { id: releaseLater; property string b: ""; onTriggered: root.padButton(b, false) }
    function tapKey(name, onlySelect) {
        if (!engine || !vm) return;
        for (let r = 0; r < vm.rows.length; r++) for (let c = 0; c < vm.rows[r].length; c++) {
            const k = vm.rows[r][c];
            if ((k.t === "ch" && k.v === name) || (k.t !== "ch" && k.t === name) || (k.t === "txt" && k.v === name)) {
                if (onlySelect) { engine.state.sel = { r: r, c: c }; engine.setActive(true); } else engine.tap(r, c);
                return;
            }
        }
    }
    function setSetting(key, value) { Osk.HearthOSK.setSetting(key, value === "true" ? true : value === "false" ? false : key === "layouts" ? value.split(",") : value); }
    function stateJson() {
        return JSON.stringify({ open: open, wtype: hasWtype, typed: typedLog, panel: vm ? vm.panel : "", sel: vm ? vm.sel : null, layout: vm ? vm.layout : "",
            suggestions: vm ? vm.suggestions : [], shift: vm ? vm.shift : false, popup: vm ? vm.popup : null, wheel: vm && vm.wheel ? vm.wheel.petal : -1,
            words: Object.keys(wordsLoaded), height: implicitHeight });
    }

    // ------------------------------------------------ typing (one wtype at a time, in order)
    property var queue: []
    readonly property var wkeys: ({ bksp: "BackSpace", enter: "Return", tab: "Tab", esc: "Escape", left: "Left", right: "Right", up: "Up", down: "Down", home: "Home", end: "End" })
    function modArgs(mods, down) {
        const out = [];
        if (mods && mods.ctrl) out.push(down ? "-M" : "-m", "ctrl");
        if (mods && mods.alt) out.push(down ? "-M" : "-m", "alt");
        return out;
    }
    function typeText(text, mods) {
        typedLog = (typedLog + text).slice(-200);
        if (!hasWtype) { if (pad) pad.send({ cmd: "emit", text: text, mods: mods ? Object.keys(mods).filter(k => mods[k]) : [] }); return; }
        const args = modArgs(mods, true);
        if (args.length && text.length === 1 && /[a-z0-9]/i.test(text)) push(["wtype"].concat(args, [text.toLowerCase()], modArgs(mods, false)));
        else push(["wtype", "--", text]);
    }
    function typeKey(name, n, mods) {
        if (name === "bksp") typedLog = typedLog.slice(0, Math.max(0, typedLog.length - (n || 1)));
        if (!hasWtype) { if (pad) pad.send({ cmd: "key", key: name, n: n || 1, mods: mods ? Object.keys(mods).filter(k => mods[k]) : [] }); return; }
        if (name === "paste") return push(["wtype", "-M", "ctrl", "v", "-m", "ctrl"]);
        const k = wkeys[name]; if (!k) return;
        let args = ["wtype"].concat(modArgs(mods, true));
        for (let i = 0; i < Math.max(1, Math.min(n || 1, 200)); i++) args.push("-k", k);
        push(args.concat(modArgs(mods, false)));
    }
    // wtype reads its text in the session's character set: UTF-8 even where the session has none (C locale).
    readonly property bool needUtf8: !/utf-?8/i.test(Quickshell.env("LC_ALL") || Quickshell.env("LC_CTYPE") || Quickshell.env("LANG") || "")
    function push(cmd) {
        if (needUtf8) cmd = ["env", "LC_ALL=C.UTF-8"].concat(cmd);
        queue = queue.concat([cmd]);
        if (!typer.running) next();
    }
    function next() {
        if (!queue.length) return;
        const q = queue.slice();
        typer.command = q.shift();
        queue = q;
        typer.running = true;
    }
    Process { id: typer; onExited: root.next() }
    Process {
        id: whichWtype
        command: ["sh", "-c", "command -v wtype"]
        stdout: SplitParser { onRead: line => { if (line.trim()) root.hasWtype = true; } }
    }

    // ------------------------------------------------ settings, learned words, word lists, the system's layouts
    function systemLayouts() {
        const env = Quickshell.env("XKB_DEFAULT_LAYOUT") || "";
        return env.split(",").map(x => x.trim()).filter(x => x);
    }
    FileView {
        id: settingsFile
        path: root.cfgDir + "/nimbus/osk.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { Osk.HearthOSK.configure({ settings: JSON.parse(text()) }); } catch (e) {} root.settingsLoaded = true; root.needWords(); }
        onLoadFailed: { root.settingsLoaded = true; root.needWords(); }
    }
    FileView {
        id: labwcEnv
        path: root.cfgDir + "/labwc/environment"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            const m = /^\s*XKB_DEFAULT_LAYOUT=["']?([^"'\n]*)/m.exec(text());
            if (m && m[1].trim()) { Osk.HearthOSK.configure({ systemLayouts: m[1].split(",").map(x => x.trim()).filter(x => x) }); root.needWords(); }
        }
    }
    property var pendingLearned: null
    property bool learnedLoaded: false
    FileView {
        id: learnedFile
        path: root.dataDir + "/nimbus/osk-words.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            if (saveLater.running) return;
            try { const d = JSON.parse(text()) || {}; Osk.HearthOSK.importLearned({ learned: d.learned || {}, next: d.next || {}, recentEmoji: d.recentEmoji || [] }); } catch (e) {}
            root.learnedLoaded = true;
        }
        // Gone (Game Mode's Clear learned words): forgotten here too.
        onLoadFailed: { if (root.learnedLoaded) Osk.HearthOSK.importLearned({ learned: {}, next: {}, recentEmoji: [] }); root.learnedLoaded = true; }
    }
    Timer {
        id: saveLater
        interval: 1500
        onTriggered: if (root.pendingLearned) learnedFile.setText(JSON.stringify(root.pendingLearned))
    }
    // Word lists: osk/words/<lang>.js, read as text (the words are the quoted part).
    property var wordsLoaded: ({})
    property string wordsLang: ""
    function needWords() {
        const ids = Osk.HearthOSK.activeLayouts();
        for (const id of ids) {
            const lay = Osk.HearthOSK.LAYOUTS.find(l => l.id === id);
            if (lay && !wordsLoaded[lay.lang] && wordsLang !== lay.lang) { wordsLang = lay.lang; return; }
        }
    }
    FileView {
        id: wordsFile
        path: root.wordsLang ? decodeURIComponent(Qt.resolvedUrl("osk/words/" + root.wordsLang + ".js").toString().replace(/^file:\/\//, "")) : ""
        onLoaded: {
            const m = /HearthOSK\.words\("([a-z]+)",\s*"([^"]*)"\)/.exec(text());
            if (m) {
                Osk.HearthOSK.words(m[1], m[2]);
                const w = Object.assign({}, root.wordsLoaded); w[m[1]] = true; root.wordsLoaded = w;
            }
            Qt.callLater(root.needWords);
        }
    }
    Connections {
        target: root
        function onVmChanged() { if (root.vm && root.vm.layout) root.needWords(); }
    }
}
