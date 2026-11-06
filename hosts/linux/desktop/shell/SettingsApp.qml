// Hearth Settings: Desktop Mode's own settings app. A normal window (labwc decorates and tiles it like any app) with
// the redesign's frame (SettingsFrame.qml: your account, search and 12 categories plus Home on the left, the page on the
// right under a breadcrumb title, the controller's buttons at the bottom). Works with mouse, touch, keyboard and the
// controller. If the frame can't load, Settings falls back to a plain list of every page (SettingsSimple.qml), so it
// always opens. Opens on Home every time.
// Opened from the taskbar's Hearth menu, Quick Access, the app launcher, or:
//   quickshell ipc -p <shell folder> call settings open [page]       (page: any id in settings-pages.js, old ids too,
//                                                                     "page:part" opens at a part, e.g. keyboard:osk)
// The pages, their categories and the search are one registry: settings-pages.js (adding a page is one entry there).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "settings-pages.js" as Reg

Scope {
    id: app
    property bool shown: false
    property string page: "home"
    // On the live USB, "Install Hearth" shows (System, and a banner on Home).
    property bool live: false
    Process { running: true; command: ["test", "-e", "/etc/hearth-live"]; onExited: code => app.live = code === 0 }

    // For the launcher's search (Launcher.qml) and older callers: every page as {id, icon, label}, the words each is
    // found by (its old name and category too), and each page's file.
    readonly property var pages: Reg.PAGES.filter(p => !p.hidden && (!p.live || live)).map(p => ({ id: p.id, icon: p.icon, label: p.label }))
    readonly property var keywords: {
        const o = {};
        for (const p of Reg.PAGES) o[p.id] = [p.words || "", p.old || "", (Reg.category(p.cat) || {}).label || ""].join(" ").trim();
        return o;
    }
    readonly property var files: {
        const o = {};
        for (const p of Reg.PAGES) o[p.id] = p.file;
        return o;
    }
    function groupOf(id) { const p = Reg.page(id); return p ? (Reg.category(p.cat) || {}).label || "" : ""; }

    Component.onCompleted: SettingsNav.settingsApp = app
    function open(p) {
        p = p || "";
        if (Reg.ALIASES[p]) p = Reg.ALIASES[p];
        // "page:part" opens the page at a part of it (SettingsNav.pageAnchor; the page or its expander goes there).
        const m = /^([\w-]+):([\w-]+)$/.exec(p);
        if (m && Reg.resolve(m[1]) !== "") { SettingsNav.pageAnchor = m[2]; p = m[1]; }
        const id = Reg.resolve(p);
        // A page only on the live USB stays closed elsewhere; anything unknown opens Home.
        const e = Reg.page(id);
        page = e && (!e.live || live) ? id : (shown ? page : "home");
        shown = true;
    }
    function close() { shown = false; SettingsNav.pageAnchor = ""; }
    function toggle() { shown ? close() : open(""); }

    LazyLoader {
        active: app.shown
        FloatingWindow {
            id: win
            title: "Hearth Settings"
            readonly property var scr: Quickshell.screens.length ? Quickshell.screens[0] : null
            implicitWidth: Math.max(900, Math.min(1600, (scr ? scr.width : 1920) - 64))
            implicitHeight: Math.max(600, Math.min(990, (scr ? scr.height : 1080) - 90))
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) app.shown = false
            // Opens on Home every time (a page asked for by name, or by a notification, opens that page instead).
            Component.onCompleted: if (app.page === "") app.page = "home"

            // The frame, loaded by file name: if it fails, the plain list instead (Settings always opens).
            Loader {
                id: frameLoader
                anchors.fill: parent
                property bool fellBack: false
                Component.onCompleted: setSource(Qt.resolvedUrl("SettingsFrame.qml"), { app: app })
                onStatusChanged: {
                    if (status === Loader.Error && !fellBack) {
                        fellBack = true;
                        console.warn("Hearth Settings: the new frame didn't load; using the plain list of pages");
                        setSource(Qt.resolvedUrl("SettingsSimple.qml"), { app: app });
                    }
                }
            }
        }
    }
}
