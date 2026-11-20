pragma Singleton

// Apps' tray icons (StatusNotifierItem) for Mantel's bar (plasma-hearth), in Quickshell's shape: Plasma 6's own tray
// reads them in its own code and offers them to no other widget, so Hearth's small host does (hearth-tray-host: it
// tells KDE's watcher there's a tray, and passes clicks, scrolls and menus back to the apps). Nothing polls: each ask
// sleeps until an icon comes, goes or changes. The singleton exists only where a bar shows the icons.
import QtQuick

Item {
    id: st
    property var items: []
    readonly property string host: "/usr/share/nimbus/desktop/plasma-hearth/hearth-tray-host"
    property int seq: 0
    property int fails: 0
    Component { id: itemComp; SystemTrayItem {} }
    function take(list) {
        const out = [];
        for (const d of list || []) {
            out.push(itemComp.createObject(st, { itemId: d.id, title: d.title || "", tooltipTitle: d.tooltip || d.title || "",
                tooltipDescription: d.sub || "", icon: d.icon || "", hasMenu: d.hasMenu === true, onlyMenu: d.isMenu === true, tray: st }));
        }
        const old = st.items;
        st.items = out;
        for (const o of old) o.destroy();
    }
    function wait() {
        Hearth.call(["python3", host, "wait", String(seq)], (out, code) => {
            let o = null;
            try { o = JSON.parse(String(out).trim().split("\n").pop() || "null"); } catch (e) { o = null; }
            if (o && Array.isArray(o.items)) {
                seq = Number(o.seq) || 0;
                fails = 0;
                take(o.items);
                wait();
            } else if (fails++ < 8) again.restart();
            else console.warn("plasma-hearth: the tray host keeps stopping; apps' tray icons are off until Plasma restarts");
        });
    }
    Timer { id: again; interval: 2000 * Math.max(1, st.fails); onTriggered: st.wait() }
    Component.onCompleted: wait()
    function op(id, what, x, y, done) {
        Hearth.call(["python3", host, "do", id, what, String(Math.round(x || 0)), String(Math.round(y || 0))], (out) => {
            let r = null;
            try { r = JSON.parse(String(out).trim().split("\n").pop()); } catch (e) {}
            if (done) done(r || {});
        });
    }
    // The app's own menu (com.canonical.dbusmenu) in Mantel's pop-up menu by the icon.
    function showMenu(item, win, x, y) {
        Hearth.call(["python3", host, "menu", item.itemId], (out) => {
            let r = null;
            try { r = JSON.parse(String(out).trim().split("\n").pop()); } catch (e) {}
            const list = [];
            for (const m of (r && r.items) || []) {
                if (m.sep) { if (list.length && !list[list.length - 1].separator) list.push({ separator: true }); continue; }
                if (m.sub) continue;       // a submenu's own items follow it
                list.push({ label: (m.depth > 0 ? "   " : "") + (m.check === 1 ? "✓ " : "") + m.label, icon: m.icon || "", enabled: m.enabled,
                            run: () => Hearth.call(["python3", host, "click", item.itemId, String(m.id)], null) });
            }
            if (!list.length) { op(item.itemId, "context", x, y); return; }
            if (win && win.menu) win.menu.popup(null, x, y, { title: item.title || "" }, list);
        });
    }
}
