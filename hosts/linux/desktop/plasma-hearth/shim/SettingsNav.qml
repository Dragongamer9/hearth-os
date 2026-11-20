pragma Singleton

// What Mantel's shell-wide pieces share (plasma-hearth): Settings opens Hearth's Settings app (hearth-settings-app),
// toasts are notifications. Mantel's other windows (Discord's small window, the mixer, the Super+X menu) aren't on
// Plasma: null, so the bar leaves them out as Mantel does when they can't load.
import QtQuick

Item {
    id: nav
    property var settingsApp: ({ pages: [], keywords: {}, open: function (p) { nav.openPage(p); } })
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
