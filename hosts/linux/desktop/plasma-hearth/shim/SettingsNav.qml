pragma Singleton

// What Mantel's shell-wide pieces share (plasma-hearth): Settings opens Hearth's Settings app (hearth-settings-app),
// toasts are notifications. Mantel's other windows (Discord's small window, the mixer, the Super+X menu) aren't on
// Plasma: null, so the bar leaves them out as Mantel does when they can't load.
import QtQuick
import "settings-pages.js" as Reg

Item {
    id: nav
    // Hearth Settings' own page list (Mantel's settings-pages.js, which the Rust Settings app is made from too): the app
    // menu's search finds every page, as on Mantel.
    readonly property var settingsApp: ({
        pages: Reg.PAGES.filter(p => !p.hidden && !p.live).map(p => ({ id: p.id, icon: p.icon, label: p.label })),
        keywords: (function () { const o = {}; for (const p of Reg.PAGES) o[p.id] = [p.words || "", p.old || "", (Reg.category(p.cat) || {}).label || ""].join(" ").trim(); return o; })(),
        open: function (p) { nav.openPage(p); }
    })
    property bool fireholdDesk: false
    property bool fireholdCouch: false
    property var mixer: null
    property var winTools: null
    property var discord: null
    property var activityShows: null
    property var activityMoveItems: null
    property var pad: null
    property var widgets: null
    property bool activitiesMany: false
    property string activityName: ""
    property string menuEdit: ""
    property bool overlay: false
    function openPage(p) { Hearth.run(["hearth-settings-app"].concat(p ? ["--page", String(p)] : [])); }
    function toast(msg) { Hearth.run(["notify-send", "-a", "Hearth", "-i", "hearth", String(msg)]); }
}
