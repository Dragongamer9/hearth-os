// What every desktop widget (Widget*.qml) has: which widget it is, its own settings (cfg, from widgets.json), and
// whether it's shown in the bar (compact) or turned round to its settings (configuring). A widget saves a choice
// only through set(), which merges that one setting of that one widget into widgets.json (WidgetStore.qml).
import QtQuick
import Quickshell

Item {
    id: base
    property string wid: ""
    property var cfg: ({})
    property bool compact: false           // the small one in the bar
    property bool configuring: false       // showing its settings (the gear, or A in Arrange)
    property bool paintsOwn: false         // draws its own background (the sticky note)
    property bool live: visible            // only works (polls, animates) while it can be seen
    readonly property real k: compact ? 1 : Math.max(0.75, Math.min(1.6, Math.min(width / 320, height / 230)))
    signal doneConfiguring()
    signal wantsSettings()
    function set(key, value) { if (wid) WidgetStore.setCfg(wid, key, value); }
    function c(key, dflt) { return cfg && cfg[key] !== undefined && cfg[key] !== null ? cfg[key] : dflt; }
    readonly property string fixtures: Quickshell.env("HEARTH_WIDGETS_FIXTURES") || ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-widgets").toString().replace(/^file:\/\//, ""))
    function open(path) { if (path) Quickshell.execDetached(["xdg-open", path]); }
}
