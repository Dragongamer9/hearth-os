pragma Singleton
// The power and volume buttons' helper (plasma-hearth): the bits Mantel's bar and menus use.
import QtQuick
Item {
    id: b
    property bool overamp: false
    readonly property real maxVolume: overamp ? 1.5 : 1.0
    readonly property string helper: "/usr/share/nimbus/desktop/shell/hearth-buttons"
    signal powerMenuRequested()
    function sleep(who) { Hearth.run(["systemctl", "suspend"]); }
}
