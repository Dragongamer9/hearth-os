pragma Singleton

// Apps' tray icons (StatusNotifierItem) for Mantel's bar (plasma-hearth): Plasma's statusnotifieritem source.
// Its sources are connected as they come (the singleton exists only where a bar shows them); nothing polls (the
// source tells when an icon comes or changes).
import QtQuick
import org.kde.plasma.plasma5support as P5Support

Item {
    id: st
    property var items: []
    P5Support.DataSource {
        id: sni
        engine: "statusnotifieritem"
        connectedSources: sources
        onSourceAdded: st.rebuild()
        onSourceRemoved: st.rebuild()
        onNewData: st.rebuild()
    }
    Component { id: itemComp; SystemTrayItem {} }
    function rebuild() {
        const out = [];
        for (const s of sni.sources) {
            const d = sni.data[s] || {};
            if (d.Status === "Passive") continue;
            out.push(itemComp.createObject(st, { title: d.Title || "", tooltipTitle: d.ToolTipTitle || "", tooltipDescription: d.ToolTipSubTitle || "",
                icon: d.IconName || d.Icon || "", hasMenu: true, onlyMenu: d.ItemIsMenu === true, tray: st, source: s }));
        }
        for (const o of st.items) o.destroy();
        st.items = out;
    }
    function op(source, name, args) {
        const svc = sni.serviceForSource(source);
        if (!svc) return;
        const o = svc.operationDescription(name);
        if (args) for (const k in args) o[k] = args[k];
        svc.startOperationCall(o);
    }
}
