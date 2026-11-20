pragma Singleton

// Apps' tray icons (StatusNotifierItem) for Mantel's bar (plasma-hearth): Plasma's statusnotifieritem source.
// Connected only while there's a bar to show them; nothing polls (the source tells when an icon comes or changes).
import QtQuick
import org.kde.plasma.plasma5support as P5Support

Item {
    id: st
    property var items: []
    property int users: 0
    P5Support.DataSource {
        id: sni
        engine: "statusnotifieritem"
        connectedSources: st.users > 0 ? sources : []
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
            out.push(itemComp.createObject(st, { id: d.Id || s, title: d.Title || "", tooltipTitle: d.ToolTipTitle || "", tooltipDescription: d.ToolTipSubTitle || "",
                icon: d.Icon || d.IconName || "", hasMenu: true, onlyMenu: d.ItemIsMenu === true, tray: st, source: s }));
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
