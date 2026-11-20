pragma Singleton

// Quickshell's few calls Mantel's files make (plasma-hearth): env, execDetached, shellDir, iconPath.
import QtQuick

Item {
    readonly property string shellDir: "/usr/share/nimbus/desktop/shell"
    function env(k) { return Hearth.env(k); }
    function execDetached(cmd) { Hearth.run(cmd); }
    function iconPath(name, check) { return name ? "image://icon/" + name : ""; }
}
