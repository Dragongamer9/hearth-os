// KWin's hearthwindows script (desktop/kwin/windows/hearth-windows.js, as hearth-kwin-bridge renders it) run against a
// pretend KWin: the window list it sends, what Mantel's commands do, Super+arrows, desktops, the Task Manager rule
// and the title bar menu. Run by test_kwin_windows.py:  node kwin-windows-mock.js RENDERED_SCRIPT
"use strict";
const fs = require("fs");
const vm = require("vm");
const assert = require("assert");

const src = fs.readFileSync(process.argv[2], "utf8");

function signal() {
    const fns = [];
    const s = function () { for (const f of fns.slice()) f.apply(null, arguments); };
    s.connect = f => fns.push(f);
    return s;
}

const AREA = { x: 0, y: 0, width: 1920, height: 1040 };   // the screen without the taskbar
let nextId = 1;
function makeWindow(o) {
    const w = {
        internalId: "{" + (nextId++) + "}", caption: o.caption || "", resourceClass: o.cls || "", desktopFileName: o.app || "",
        normalWindow: o.normal !== false, dialog: !!o.dialog, specialWindow: !!o.special, skipTaskbar: !!o.skipTaskbar,
        moveable: o.moveable !== false, resizeable: o.resizeable !== false, minimizable: o.minimizable !== false,
        minimized: false, fullScreen: false, keepAbove: false, onAllDesktops: false, pid: 100, output: { name: "HDMI-A-1" },
        frameGeometry: { x: 100, y: 100, width: 800, height: 600 }, desktops: [], mode: 0,
        captionChanged: signal(), minimizedChanged: signal(), maximizedChanged: signal(), maximizedAboutToChange: signal(),
        fullScreenChanged: signal(), desktopsChanged: signal(), frameGeometryChanged: signal(),
        setMaximize(v, h) {
            const m = (v ? 1 : 0) | (h ? 2 : 0);
            this.maximizedAboutToChange(m);
            if (m === 3) { this.restore = this.frameGeometry; this.frameGeometry = Object.assign({}, AREA); }
            else if (this.mode === 3 && this.restore) this.frameGeometry = this.restore;
            this.mode = m;
            this.maximizedChanged();
        },
        closeWindow() { this.closed = true; ws.windowRemoved(this); },
    };
    return w;
}

const desks = [{ id: "d1", name: "Desktop 1" }, { id: "d2", name: "Desktop 2" }, { id: "d3", name: "Desktop 3" }];
const ws = {
    stackingOrder: [], desktops: desks, currentDesktop: desks[0], activeWindow: null,
    windowAdded: signal(), windowRemoved: signal(), windowActivated: signal(), currentDesktopChanged: signal(), desktopsChanged: signal(),
    cursorPos: { x: 0, y: 0 }, cursorPosChanged: signal(),
    clientArea() { return Object.assign({}, AREA); },
    slotWindowOperations() { this.menuShown = true; },
};
const sent = [];          // Update(json) calls
const shortcutsCalled = [];
const otherCalls = [];    // the script's other D-Bus calls: [service, method, args...]
let nextCallback = null;  // the script's waiting Next()
const shortcuts = {};
const menuFns = [];
let menuFn = null;
const ctx = {
    workspace: ws, KWin: { MaximizeArea: 2 }, print: m => { throw new Error("script printed: " + m); },
    JSON, Math, parseInt, String,
    callDBus(bus, path, iface, method) {
        const args = Array.prototype.slice.call(arguments, 4);
        if (method === "Update") sent.push(JSON.parse(args[0]));
        else if (method === "Next") nextCallback = args[0];
        else if (method === "Shortcut") shortcutsCalled.push(args[0]);
        else otherCalls.push([bus, method].concat(args));
    },
    registerShortcut(name, text, keys, fn) { assert(!shortcuts[name], "two shortcuts named " + name); shortcuts[name] = { keys, fn }; },
    registerUserActionsMenu(fn) { menuFns.push(fn); if (!menuFn) menuFn = fn; },
};

// Before the script: an app window, a layer-shell surface Mantel draws (the bar), and a window that skips the taskbar.
const term = makeWindow({ caption: "Terminal", app: "foot" });
const bar = makeWindow({ caption: "", cls: "quickshell", moveable: false, resizeable: false, minimizable: false });
const hidden = makeWindow({ caption: "Hidden", app: "x", skipTaskbar: true });
ws.stackingOrder = [term, bar, hidden];
ws.activeWindow = term;

vm.runInNewContext(src, ctx, { filename: "hearth-windows.js" });

const last = () => sent[sent.length - 1];
function send(cmds) {                 // the bridge answering Next()
    const cb = nextCallback;
    assert(cb, "the script isn't waiting on Next()");
    nextCallback = null;
    cb(JSON.stringify(cmds));
    assert(nextCallback, "the script didn't ask Next() again");
}
function key(id) { shortcuts["Hearth " + id].fn(); }
// The script's objects come from another realm (vm): compare them as JSON.
function same(a, b, msg) { assert.strictEqual(JSON.stringify(a), JSON.stringify(b), msg); }

// ---- the list: only the app window, with its details
assert(sent.length >= 1, "no window list sent at start");
assert.deepStrictEqual(last().windows.map(w => w.title), ["Terminal"]);
const t0 = last().windows[0];
assert.strictEqual(t0.appId, "foot");
assert.strictEqual(t0.activated, true);
assert.strictEqual(last().active, term.internalId);
assert.deepStrictEqual(last().desktops.map(d => d.active), [true, false, false]);

// ---- a new window comes in the order it opened
const files = makeWindow({ caption: "Files", app: "org.hearth.files" });
ws.windowAdded(files);
assert.deepStrictEqual(last().windows.map(w => w.title), ["Terminal", "Files"]);

// ---- commands from Mantel
send([{ op: "minimize", id: term.internalId, on: true }]);
assert.strictEqual(term.minimized, true);
assert.strictEqual(last().windows[0].minimized, true);
send([{ op: "activate", id: term.internalId }]);
assert.strictEqual(term.minimized, false);
assert.strictEqual(ws.activeWindow, term);
send([{ op: "maximize", id: term.internalId, on: true }]);
assert.strictEqual(last().windows[0].maximized, true);
send([{ op: "maximize", id: term.internalId, on: false }]);
assert.strictEqual(last().windows[0].maximized, false);

// ---- snapping: the zones with labwc's gap (6 px at the edges, 6 px between)
send([{ op: "snap", id: term.internalId, zone: "left-half" }]);
same(term.frameGeometry, { x: 6, y: 6, width: 951, height: 1028 });
assert.strictEqual(last().windows[0].zone, "left-half");
send([{ op: "snap", id: files.internalId, zone: "right-half" }]);
same(files.frameGeometry, { x: 963, y: 6, width: 951, height: 1028 });
assert.strictEqual(files.frameGeometry.x - (term.frameGeometry.x + term.frameGeometry.width), 6, "the gap between two halves");
send([{ op: "snap", id: term.internalId, zone: "center" }]);
assert.strictEqual(last().windows[0].zone, "center");

// ---- Super+arrows (rc.xml's steps): one press, one change
ws.activeWindow = term;
key("snap-left");
assert.strictEqual(last().windows[0].zone, "left-half");
key("snap-up");
assert.strictEqual(last().windows[0].zone, "top-left");
key("snap-down");
assert.strictEqual(last().windows[0].zone, "left-half");
key("snap-down");
assert.strictEqual(last().windows[0].zone, "bottom-left");
key("snap-up");
assert.strictEqual(last().windows[0].zone, "left-half");
// KWin's own tiling (dragged to the edge) leaves no gap: still a half.
term.frameGeometry = { x: 960, y: 0, width: 960, height: 1040 };
key("snap-up");
assert.strictEqual(last().windows[0].zone, "top-right");
term.frameGeometry = { x: 300, y: 200, width: 700, height: 500 };
key("snap-up");
assert.strictEqual(term.mode, 3, "Super+Up on a free window maximizes");
key("snap-down");
assert.strictEqual(term.mode, 0, "Super+Down on a maximized window restores");
assert.strictEqual(term.minimized, false);
key("snap-down");
assert.strictEqual(term.minimized, true, "Super+Down on a restored window minimizes");
send([{ op: "activate", id: term.internalId }]);

// ---- desktops
key("window-to-desktop-2");
same(term.desktops, [desks[1]]);
assert.deepStrictEqual(last().windows[0].desktops, [2]);
assert.strictEqual(ws.currentDesktop, desks[0], "sending a window doesn't follow it");
send([{ op: "goDesktop", n: 3 }]);
assert.strictEqual(ws.currentDesktop, desks[2]);
key("desktop-1");
assert.strictEqual(ws.currentDesktop, desks[0]);
send([{ op: "desktop", id: files.internalId, n: 3 }]);
same(files.desktops, [desks[2]]);

// ---- shortcuts that run Mantel's commands go to the bridge by id
key("launcher");
key("snap-layouts");
assert.deepStrictEqual(shortcutsCalled.slice(-2), ["launcher", "snap-layouts"]);
assert.strictEqual(shortcuts["Hearth launcher"].keys, "Meta+Space");
assert.strictEqual(shortcuts["Hearth snap-left"].keys, "Meta+Left");

// ---- Task Manager's "always on top" title (labwc's window rule), and off again
const tm = makeWindow({ caption: "Task Manager", app: "org.quickshell" });
ws.windowAdded(tm);
assert.strictEqual(tm.keepAbove, false);
tm.caption = "Task Manager (always on top)"; tm.captionChanged();
assert.strictEqual(tm.keepAbove, true);
tm.caption = "Task Manager"; tm.captionChanged();
assert.strictEqual(tm.keepAbove, false);

// ---- the title bar menu: Hearth's Snap with Snap layouts at the top; nothing for Mantel's own surfaces
const m = menuFn(files);
assert.strictEqual(m.text, "Snap");
assert.strictEqual(m.items[0].text, "Snap layouts…");
assert(m.items.some(i => i.text === "Left half"), "Left half in the Snap menu");
assert(m.items.some(i => i.text === "Right half" && i.checked === true), "the zone it's in is ticked");
m.items[0].triggered();
assert.strictEqual(shortcutsCalled[shortcutsCalled.length - 1], "snap-layouts-open");
m.items.find(i => i.text === "Left half").triggered();
assert.strictEqual(last().windows.find(w => w.title === "Files").zone, "left-half");
assert.strictEqual(menuFn(bar), undefined);

// ---- Hearth's other title bar menu entries: hide from screen sharing (checkable), force close
assert.strictEqual(menuFns.length, 3, "Snap, Hide from screen sharing, Force close");
const share = menuFns[1](files);
assert.strictEqual(share.text, "Hide from screen sharing");
assert.strictEqual(share.checked, false);
share.triggered();
assert.strictEqual(last().windows.find(w => w.title === "Files").noShare, true);
assert.strictEqual(menuFns[1](files).checked, true, "ticked once hidden");
send([{ op: "noShare", id: files.internalId, on: false }]);
assert.strictEqual(last().windows.find(w => w.title === "Files").noShare, false);
assert.strictEqual(last().captureHide, false, "this pretend KWin can't paint windows out of a share");
const fcItem = menuFns[2](files);
assert(/Force close/.test(fcItem.text));
fcItem.triggered();
assert.deepStrictEqual(otherCalls[otherCalls.length - 1].slice(1), ["ForceClose", files.internalId]);
assert.strictEqual(menuFns[1](bar), undefined);
assert.strictEqual(menuFns[2](bar), undefined);

// ---- KWin's views: through kglobalaccel's invokeShortcut, from a command or Hearth's keys
send([{ op: "effect", name: "overview" }]);
assert.deepStrictEqual(otherCalls[otherCalls.length - 1], ["org.kde.kglobalaccel", "invokeShortcut", "Overview"]);
key("present-windows");
assert.deepStrictEqual(otherCalls[otherCalls.length - 1], ["org.kde.kglobalaccel", "invokeShortcut", "ExposeAll"]);
key("desktop-grid");
assert.deepStrictEqual(otherCalls[otherCalls.length - 1], ["org.kde.kglobalaccel", "invokeShortcut", "Grid View"]);
key("overview");
assert.deepStrictEqual(otherCalls[otherCalls.length - 1], ["org.kde.kglobalaccel", "invokeShortcut", "Overview"]);

// ---- resting the pointer on Maximize (Hearth's Aurorae frame: 34 px buttons, close 5 px from the right edge)
term.minimized = false;
term.frameGeometry = { x: 100, y: 100, width: 800, height: 600 };
term.clientGeometry = { x: 101, y: 130, width: 798, height: 569 };
ws.stackingOrder = [files, bar, hidden, term];
const hovers = () => otherCalls.filter(c => c[1] === "HoverMax").map(c => JSON.parse(c[2]));
ws.cursorPos = { x: 100 + 800 - 5 - 34 - 2 - 17, y: 112 };     // the middle of Maximize
ws.cursorPosChanged();
let h = hovers();
assert.strictEqual(h[h.length - 1].id, term.internalId, "on Maximize");
same([h[h.length - 1].x, h[h.length - 1].w], [100 + 800 - 5 - 34 - 2 - 34, 34]);
const nh = h.length;
ws.cursorPos = { x: ws.cursorPos.x + 3, y: 113 }; ws.cursorPosChanged();
assert.strictEqual(hovers().length, nh, "moving on the same button sends nothing more");
ws.cursorPos = { x: 100 + 800 - 20, y: 112 }; ws.cursorPosChanged();     // onto Close
h = hovers();
assert.strictEqual(h[h.length - 1].id, "", "off Maximize");
ws.cursorPos = { x: 300, y: 300 }; ws.cursorPosChanged();
assert.strictEqual(hovers().length, h.length, "still off: nothing more");

// ---- closing, and windows that go
send([{ op: "close", id: files.internalId }]);
assert(files.closed);
assert(!last().windows.some(w => w.title === "Files"));
send([{ op: "refresh" }, { op: "nonsense" }]);

// ---- moving a window about: no list for every step, one when its zone changes
const before = sent.length;
term.frameGeometryChanged(); term.frameGeometryChanged();
assert(sent.length <= before + 1, "a list for every geometry step");
const n0 = sent.length;
term.frameGeometryChanged();
assert.strictEqual(sent.length, n0, "the same place sent again");

console.log("kwin-windows-mock: all passed (" + sent.length + " lists sent)");
