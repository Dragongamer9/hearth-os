pragma Singleton

// Desktop widgets' places and settings (~/.config/nimbus/widgets.json; the rules in widget-layout.js).
// Settings are always remembered and never change by themselves (docs/SETTINGS-PERSISTENCE.md):
//   * each widget is its own top-level key ("w:<id>"), and only the widget you changed is merged into the latest
//     file (SettingsWriter → hearth_settings_io.py: under the lock, atomic, FILE.bak kept);
//   * nothing is written when the shell starts, a screen is unplugged or a widget is drawn pulled inside a smaller
//     screen: only moving, resizing, adding, removing or changing a widget writes;
//   * a file that can't be read is put back from its .bak (repair) and the widgets already on screen stay; the
//     defaults are never saved over it; an entry the shell doesn't understand is left in the file as it is.
import QtQuick
import Quickshell
import Quickshell.Io
import "widget-layout.js" as L

Singleton {
    id: st
    readonly property string path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/widgets.json"
    property var raw: ({})                 // the file as last read, with this session's own changes on top
    property var widgets: []               // parsed (widget-layout.js normalize)
    property bool loaded: false
    property bool damaged: false
    property int rev: 0
    // Widgets.qml (the layers, the gallery, Arrange) registers itself here, so the bar, Quick Access and the desktop's
    // menu can open them without knowing whether Widgets.qml loaded.
    property var ui: null
    property string arranging: ""          // the screen being arranged ("" = none)
    property string configuring: ""        // the widget whose settings are open ("" = none)
    // Arrange's moves while you make them: drawn at once, saved when you're done with that widget (never per step).
    property var preview: ({})
    property int previewRev: 0
    function setPreview(id, rect) { const p = Object.assign({}, preview); if (rect) p[id] = rect; else delete p[id]; preview = p; previewRev++; }

    function openGallery(screenName) { if (ui) ui.openGallery(screenName || ""); }
    function arrange(screenName) { if (ui) ui.startArrange(screenName || ""); }

    function reparse() {
        widgets = L.parse(raw).widgets;
        rev++;
    }
    function get(id) { return widgets.find(w => w.id === id) || null; }

    // ---- the only writers (each one is something you did)
    function put(id, changes) {
        const e = L.entryFor(raw, id, changes);
        const r = Object.assign({}, raw);
        r["w:" + id] = e;
        raw = r;
        reparse();
        const o = {};
        o["w:" + id] = e;
        writer.merge(o);
    }
    function add(type, screenName, activity, place, at, cfg) {
        const t = L.info(type);
        if (!t) return "";
        const id = L.newId(Date.now(), Math.random());
        const maxOrder = widgets.reduce((m, w) => Math.max(m, w.order), 0);
        put(id, { type: type, place: place === "bar" ? "bar" : "desk", screen: screenName || "", x: at ? at.x : 0, y: at ? at.y : 0,
                  w: at && at.w ? at.w : t.w, h: at && at.h ? at.h : t.h, activity: activity || "default", order: maxOrder + 1, cfg: cfg || {} });
        return id;
    }
    function place(id, x, y, w, h, screenName) {
        const cur = get(id);
        if (!cur) return;
        const c = { x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h) };
        if (screenName) c.screen = screenName;
        if (c.x === cur.x && c.y === cur.y && c.w === cur.w && c.h === cur.h && (!screenName || screenName === cur.screen)) return;
        put(id, c);
    }
    function setCfg(id, key, value) {
        const cur = get(id);
        if (!cur) return;
        if (JSON.stringify(cur.cfg[key]) === JSON.stringify(value)) return;
        const cfg = Object.assign({}, cur.cfg);
        cfg[key] = value;
        put(id, { cfg: cfg });
    }
    function setActivity(id, act) { const cur = get(id); if (cur && cur.activity !== act) put(id, { activity: act }); }
    function remove(id) {
        const r = Object.assign({}, raw);
        delete r["w:" + id];
        raw = r;
        reparse();
        const o = {};
        o["w:" + id] = null;
        writer.merge(o);
    }

    SettingsWriter { id: writer; path: st.path }

    FileView {
        id: file
        path: st.path
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") {
                    console.warn("widgets: widgets.json couldn't be read; putting back the last good copy (your widgets stay as they are)");
                    st.damaged = true;
                    writer.repair();
                }
                st.loaded = true;
                return;
            }
            // Keys still on their way to the file keep this session's value (a reload of an earlier write mustn't
            // put an old place back).
            const r = Object.assign({}, o);
            for (const k in st.raw) if (writer.owns(k)) r[k] = st.raw[k];
            for (const k in r) if (writer.owns(k) && st.raw[k] === undefined) delete r[k];
            st.raw = r;
            st.damaged = false;
            st.reparse();
            st.loaded = true;
        }
        onLoadFailed: st.loaded = true      // no file yet: no widgets (nothing is written until you add one)
    }
}
