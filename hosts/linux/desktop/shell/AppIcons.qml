pragma Singleton

// Finding an app's picture, for every part of Mantel that shows apps: the launcher, the taskbar, notifications,
// desktop shortcuts, Task Manager, Open with... (AppIcon.qml draws one.) Also matches windows to their apps.
//
// An icon is looked for, in order:
//   1. the desktop entry's Icon= as an icon name in the icon theme, with hicolor behind it, through
//      $XDG_DATA_DIRS (Flatpak's /var/lib/flatpak/exports/share and ~/.local/share/flatpak/exports/share are put
//      there by nimbus-session) and ~/.local/share/icons (AppImages, Distrobox exports), and /usr/share/pixmaps;
//   2. Icon= as a file (an absolute path, or file://), or a name with its extension left on ("foo.png");
//   3. the window's app id (on Wayland a Flatpak's app id is its Flatpak id), matched to a desktop entry by
//      id, StartupWMClass, `flatpak run <id>`, the program's name or the last part of a reverse-DNS id;
//   4. Hearth's generic app icon.
import QtQuick
import Quickshell

Singleton {
    id: root

    // Hearth's generic app picture (installed as hearth-generic-app; a copy ships with the shell).
    readonly property string generic: themed("hearth-generic-app") || Qt.resolvedUrl("images/app-generic.svg")

    function themed(name) {
        if (!name) return "";
        try { return Quickshell.iconPath(name, true) || ""; } catch (e) { return ""; }
    }

    // An Icon= value as an image source, or "" when it can't be found.
    function fromIcon(icon) {
        const i = String(icon || "").trim();
        if (!i) return "";
        if (/^(file|image|qrc|data|https?):/.test(i)) return i;
        if (i.startsWith("/")) return "file://" + encodeURI(i);
        const t = themed(i);
        if (t) return t;
        const bare = i.replace(/\.(png|svg|svgz|xpm|ico|jpe?g)$/i, "");
        if (bare !== i) { const b = themed(bare); if (b) return b; }
        if (bare.toLowerCase() !== bare) { const l = themed(bare.toLowerCase()); if (l) return l; }
        // "foo.png" that only lives in /usr/share/pixmaps (Qt's own search there wants the bare name).
        if (bare !== i) return "file:///usr/share/pixmaps/" + encodeURI(i);
        return "";
    }

    // ------------------------------------------------------------------ windows and notifications → desktop entries
    function lower(s) { return String(s || "").toLowerCase().replace(/\.desktop$/, ""); }
    function words(e) {
        if (e.command && e.command.length !== undefined) return Array.from(e.command).map(String);
        return String(e.execString || "").split(/\s+/).filter(w => w !== "");
    }
    readonly property var index: {
        const byId = {}, byClass = {}, byFlatpak = {}, byExec = {}, byLast = {};
        const put = (map, k, e) => { if (k && !map[k]) map[k] = e; };
        for (const e of Array.from(DesktopEntries.applications.values)) {
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
            } else if (w.length) {
                put(byExec, lower(w[0].split("/").pop()), e);
            }
        }
        return { byId: byId, byClass: byClass, byFlatpak: byFlatpak, byExec: byExec, byLast: byLast };
    }

    // The desktop entry for a window's app id (or a notification's desktop-entry hint), or null.
    function entryFor(appId) {
        const raw = String(appId || "").replace(/\.desktop$/, "");
        if (!raw) return null;
        const a = raw.toLowerCase(), x = index;
        const hit = x.byId[a] || x.byClass[a] || x.byFlatpak[a] || x.byExec[a];
        if (hit) return hit;
        try { const h = DesktopEntries.heuristicLookup(raw); if (h) return h; } catch (e) {}
        return x.byLast[a.split(".").pop()] || null;
    }

    // An image source for an app: its Icon= (or a window's app id) as described above, or "" when nothing fits.
    function find(icon, appId) {
        const s = fromIcon(icon);
        if (s) return s;
        if (!appId) return "";
        const e = entryFor(appId);
        if (e && e.icon && e.icon !== icon) {
            const s2 = fromIcon(e.icon);
            if (s2) return s2;
        }
        const raw = String(appId).replace(/\.desktop$/, "");
        for (const n of [raw, raw.toLowerCase(), raw.split(".").pop().toLowerCase()]) {
            const t = themed(n);
            if (t) return t;
        }
        return "";
    }
    // The same, with Hearth's generic app picture when nothing fits.
    function source(icon, appId) { return find(icon, appId) || generic; }

    // Hearth's own windows (part of this shell, so their app id is Quickshell's): their icons by window title.
    readonly property var ownIcons: ({
        "Hearth Settings": "hearth-settings", "Task Manager": "hearth-tasks", "Files": "hearth-files", "Apps": "hearth-apps"
    })
    function own(title) {
        const t = String(title || "");
        const name = ownIcons[t] || (/ – Files$/.test(t) ? "hearth-files" : "");
        return themed(name) || themed("hearth") || Qt.resolvedUrl("images/hearth-logo.png");
    }
}
