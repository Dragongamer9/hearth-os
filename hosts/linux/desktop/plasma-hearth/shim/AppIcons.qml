pragma Singleton

// Finding an app's picture (plasma-hearth): Mantel's AppIcons.qml matching (desktop id, window class, Flatpak id,
// program) on Plasma's list, with the icon theme through Kirigami (AppIcon.qml) instead of Quickshell's lookup.
import QtQuick

Item {
    id: root
    readonly property string generic: "file:///usr/share/nimbus/desktop/shell/images/app-generic.svg"
    function themed(name) { return name ? String(name) : ""; }
    function fromIcon(icon) {
        const i = String(icon || "").trim();
        if (!i) return "";
        if (/^(file|image|qrc|data|https?):/.test(i)) return i;
        if (i.startsWith("/")) return "file://" + encodeURI(i);
        return "image://icon/" + i;
    }
    function lower(s) { return String(s || "").toLowerCase().replace(/\.desktop$/, ""); }
    function words(e) {
        if (e.command && e.command.length) return Array.from(e.command).map(String);
        return String(e.execString || "").split(/\s+/).filter(w => w !== "");
    }
    readonly property var index: {
        const byId = {}, byClass = {}, byFlatpak = {}, byExec = {}, byLast = {};
        const put = (map, k, e) => { if (k && !map[k]) map[k] = e; };
        for (const e of DesktopEntries.applications.values) {
            const id = lower(e.id);
            if (!id) continue;
            put(byId, id, e);
            put(byClass, lower(e.startupClass), e);
            put(byLast, id.split(".").pop(), e);
            const w = words(e).filter(x => !/^[A-Za-z_][A-Za-z0-9_]*=/.test(x) && x !== "env");
            const run = w.indexOf("run");
            if (w.length && /(^|\/)flatpak$/.test(w[0]) && run > 0) {
                const fid = w.slice(run + 1).find(x => !x.startsWith("-") && !x.startsWith("@@"));
                put(byFlatpak, lower(fid), e);
            } else if (w.length) put(byExec, lower(w[0].split("/").pop()), e);
        }
        return { byId: byId, byClass: byClass, byFlatpak: byFlatpak, byExec: byExec, byLast: byLast };
    }
    function entryFor(appId) {
        const raw = String(appId || "").replace(/\.desktop$/, "");
        if (!raw) return null;
        const a = raw.toLowerCase(), x = index;
        return x.byId[a] || x.byClass[a] || x.byFlatpak[a] || x.byExec[a] || x.byLast[a.split(".").pop()] || null;
    }
    // A themed icon name (or a picture URL) for an icon and/or an app id; AppIcon.qml draws it.
    function find(icon, appId) {
        if (icon) return String(icon);
        const e = entryFor(appId);
        if (e && e.icon) return e.icon;
        return appId ? String(appId).replace(/\.desktop$/, "") : "";
    }
    // An app's picture or Hearth's generic one (the desktop's launchers, Open with lists).
    function source(icon, appId) { return find(icon, appId) || generic; }
    // Hearth's own windows (Mantel's Files, Settings...): their app's picture.
    function own(title) { return /Files/.test(title || "") ? "system-file-manager" : /Settings/.test(title || "") ? "preferences-system" : ""; }
}
