pragma Singleton

// Quickshell's few calls Mantel's files make (plasma-hearth): env, execDetached, shellDir, iconPath.
import QtQuick

Item {
    readonly property string shellDir: "/usr/share/nimbus/desktop/shell"
    function env(k) { return Hearth.env(k); }
    function execDetached(cmd) { Hearth.run(cmd); }
    // A theme icon's name for IconImage (shim): with a fallback name, "name|fallback"; Plasma has no image://icon/.
    function iconPath(name, check) {
        if (!name) return typeof check === "string" ? check : "";
        return typeof check === "string" && check ? name + "|" + check : name;
    }
}
