// Power (the web page's renderPower): sleep, restart, shut down, sign out, switch to Mantel (Desktop Mode).
import QtQuick

Item {
    id: pm
    property bool open: App.layers.indexOf("power") >= 0
    property string scopeName: "power"
    property var app: App                 // whatever does the power action: App, or the overlay window
    property var cancel: null             // what Cancel does (default: close the layer)
    function doCancel() { if (cancel) cancel(); else App.closeLayer("power"); }
    visible: open
    Rectangle { anchors.fill: parent; color: "#000000"; opacity: 0.6
        MouseArea { anchors.fill: parent; onClicked: pm.doCancel() } }
    Rectangle {
        anchors.centerIn: parent
        width: Theme.u(380)
        height: col.implicitHeight + Theme.u(36)
        radius: Theme.u(16)
        color: Theme.panel
        border.color: Theme.line
        Column {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(18) }
            spacing: Theme.u(4)
            Text { text: "Power"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(18); font.weight: Font.Bold; bottomPadding: Theme.u(8) }
            Repeater {
                model: [["sleep", "Sleep", "moon"], ["restart", "Restart", "restart"], ["shutdown", "Shut down", "power"],
                        ["signout", "Sign out", "user"], ["desktop", "Switch to desktop", "desk"]]
                delegate: Btn {
                    required property var modelData
                    width: parent.width
                    kind: "menu"
                    navScope: pm.scopeName
                    navKey: "p-" + modelData[0]
                    navFirst: modelData[0] === "sleep"
                    icon: modelData[2]
                    text: modelData[1]
                    onActivated: pm.app.power(modelData[0])
                }
            }
            Btn { width: parent.width; kind: "menu"; navScope: pm.scopeName; navKey: "p-cancel"; icon: "x"; text: "Cancel"; onActivated: pm.doCancel() }
        }
    }
}
