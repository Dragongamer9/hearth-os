pragma Singleton

// The main bar's own items and the extra panels (1.14; Arrange the bar: BarArrange.qml). ~/.config/nimbus/bar.json:
//   "items": {start: [...], end: [...], hidden: [...]}   only once you move or hide something (widget-tools.js
//            barLayout fills in what's missing, so a newer Hearth's items land in their own place)
//   "p:<id>": {screen, edge, items, size, autoHide}     one key per extra panel, so changing one merges only it
//   "clockZones", "clockAlt", "clockHolidays"          the clock's panel: world clocks, another calendar, holidays
//                                                      (Settings › Date & time)
//   "s:<block id>"                                     a block's look and what it does (drag and drop board 2,
//                                                      block-look.js; one key per block), drawn by BlockLook.qml
//   "bar"                                              the main bar's own settings (board 2, widget-tools.js barLook)
// Written only when you change something (SettingsWriter: merged, atomic, .bak; a file that can't be read is put back
// from its .bak and nothing is saved over it). Game Mode is always in the main bar, last; the flame is never hidden.
import QtQuick
import Quickshell
import Quickshell.Io
import "widget-tools.js" as T

Singleton {
    id: bl
    readonly property string path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/bar.json"
    property var raw: ({})
    property bool loaded: false
    // false while bar.json can't be read (its last good copy is being put back): the desktop editor saves nothing then
    property bool readOk: true
    // the main bar's own settings, and its look key (a change builds the bar again: shell.qml)
    readonly property var mainBar: T.barLook(raw.bar)
    readonly property string mainKey: mainBar.edge + "/" + mainBar.layer + "/" + mainBar.orient + "/" + T.barSide(mainBar, "")
    function mainSide(fallback) { return T.barSide(mainBar, fallback); }
    // a block's look (sparse; BlockLook.qml normalizes it)
    function lookOf(id) { const v = raw["s:" + id]; return v && typeof v === "object" && !Array.isArray(v) ? v : null; }
    readonly property var layout: T.barLayout(raw.items)
    readonly property var panels: {
        const out = [];
        for (const k in raw) if (k.startsWith("p:")) { const p = T.panelOf(k.slice(2), raw[k]); if (p) out.push(p); }
        return out.sort((a, b) => a.id < b.id ? -1 : 1);
    }
    property bool arranging: false         // Arrange the bar (BarArrange.qml) is open
    property string arrangeTab: "items"    // items | panels
    readonly property var clockZones: Array.isArray(raw.clockZones) ? raw.clockZones.filter(z => typeof z === "string") : []
    readonly property string clockAlt: typeof raw.clockAlt === "string" ? raw.clockAlt : ""
    readonly property string clockHolidays: typeof raw.clockHolidays === "string" ? raw.clockHolidays : ""
    function setClock(k, v) { if (JSON.stringify(raw[k]) !== JSON.stringify(v)) { const o = {}; o[k] = v; put(o); } }
    function shown(id) { return layout.hidden.indexOf(id) < 0; }
    function zoneOf(id) { return layout.start.indexOf(id) >= 0 ? "start" : layout.end.indexOf(id) >= 0 ? "end" : "hidden"; }
    function indexIn(id) { const z = zoneOf(id); return z === "hidden" ? 999 : layout[z].indexOf(id); }

    function move(id, dir, zone) { setItems(T.barMove(raw.items, id, dir, zone)); }
    function setItems(l) {
        if (JSON.stringify(T.barLayout(l)) === JSON.stringify(layout)) return;
        put({ items: T.barLayout(l) });
    }
    function resetItems() { if (raw.items !== undefined) put({ items: null }); }

    function addPanel(screen, edge, items) {
        const id = Date.now().toString(36);
        const o = {}; o["p:" + id] = { screen: screen || "", edge: edge || "bottom", items: items || ["clock"], size: 44 };
        put(o);
        return id;
    }
    function setPanel(id, changes) {
        const cur = raw["p:" + id];
        if (!cur) return;
        const o = {}; o["p:" + id] = Object.assign({}, cur, changes);
        put(o);
    }
    function removePanel(id) { const o = {}; o["p:" + id] = null; put(o); }
    function put(o) {
        const r = Object.assign({}, raw);
        for (const k in o) { if (o[k] === null) delete r[k]; else r[k] = o[k]; }
        raw = r;
        writer.merge(o);
    }
    SettingsWriter { id: writer; path: bl.path; history: true }
    FileView {
        path: bl.path
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") { console.warn("bar: bar.json couldn't be read; putting back the last good copy (the bar stays as it is)"); bl.readOk = false; writer.repair(); }
                bl.loaded = true;
                return;
            }
            const r = Object.assign({}, o);
            for (const k in bl.raw) if (writer.owns(k)) r[k] = bl.raw[k];
            for (const k in r) if (writer.owns(k) && bl.raw[k] === undefined) delete r[k];
            bl.raw = r;
            bl.readOk = true;
            bl.loaded = true;
        }
        onLoadFailed: bl.loaded = true
    }
}
