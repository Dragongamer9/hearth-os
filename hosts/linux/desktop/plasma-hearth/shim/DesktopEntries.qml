pragma Singleton

// The installed apps for Mantel's menus and dock (plasma-hearth): every .desktop file (hearth-plasma-panel apps reads
// them as KDE's menu does), read when the bar starts and again when apps change (Hearth.stamp, from the .path unit
// that watches the applications folders). entry.execute() starts it the way Plasma does (KIO).
import QtQuick

Item {
    id: de
    property var list: []
    readonly property var applications: ({ values: list })
    property var byIdMap: ({})
    function byId(id) { const k = String(id || "").replace(/\.desktop$/, ""); return byIdMap[k] || byIdMap[k.toLowerCase()] || null; }
    function heuristicLookup(id) { return byId(id); }
    function make(e) {
        const o = Object.assign({}, e);
        o.keywords = e.keywords || [];
        o.command = e.command || [];
        o.execute = function () { Hearth.run(["python3", Hearth.helper, "launch", e.path || e.id]); };
        o.actions = (e.actions || []).map(a => ({ name: a.name, id: a.id, icon: a.icon || "", execute: function () { Hearth.run(["python3", Hearth.helper, "launch", e.path || e.id, a.id]); } }));
        return o;
    }
    function load() {
        Hearth.call(["python3", Hearth.helper, "apps"], (out) => {
            let l = null;
            try { l = JSON.parse(out.trim().split("\n").pop() || "[]"); } catch (e) { l = null; }
            if (!Array.isArray(l)) return;
            const made = l.map(make), m = {};
            for (const e of made) { m[e.id] = e; if (!m[e.id.toLowerCase()]) m[e.id.toLowerCase()] = e; }
            de.byIdMap = m;
            de.list = made;
        });
    }
    Component.onCompleted: load()
    Connections { target: Hearth; function onStampChanged() { de.load(); } }
}
