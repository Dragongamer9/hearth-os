// The menu (☰): Hearth's pages. The ones the Qt renderer doesn't have yet say so on their page.
import QtQuick

Item {
    id: menu
    readonly property bool open: App.layers.indexOf("menu") >= 0
    visible: panel.x > -panel.width + 1
    Rectangle { anchors.fill: parent; color: "#000000"; opacity: menu.open ? 0.5 : 0; Behavior on opacity { NumberAnimation { duration: Theme.calm } }
        MouseArea { anchors.fill: parent; enabled: menu.open; onClicked: App.closeLayer("menu") } }
    Rectangle {
        id: panel
        width: Theme.u(320)
        height: parent.height
        x: menu.open ? 0 : -width
        color: Theme.bg2
        Behavior on x { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Rectangle { anchors { right: parent.right; top: parent.top; bottom: parent.bottom } width: 1; color: Theme.line }
        Column {
            anchors { fill: parent; margins: Theme.u(16); topMargin: Theme.u(22) }
            spacing: Theme.u(4)
            Text { text: "Hearth"; color: Theme.text; font.family: Theme.display; font.pixelSize: Theme.u(28); font.weight: Font.Black; bottomPadding: Theme.u(14); leftPadding: Theme.u(8) }
            Repeater {
                model: [
                    ["home", "Home", "home", ""], ["library", "Library", "lib", ""], ["downloads", "Downloads", "dl", ""],
                    ["store", "Store", "globe", "Store"], ["friends", "Friends", "user", "Friends"], ["shots", "Screenshots", "display", "Screenshots"],
                    ["settings", "Settings", "gear", ""], ["power", "Power", "power", ""]
                ]
                delegate: Btn {
                    required property var modelData
                    width: parent.width
                    kind: "menu"
                    navScope: "menu"
                    navKey: "mn-" + modelData[0]
                    navFirst: App.view === modelData[0]
                    icon: modelData[2]
                    text: modelData[1]
                    value: modelData[3] ? "web renderer" : ""
                    on: App.view === modelData[0]
                    onActivated: {
                        if (modelData[0] === "power") { App.closeLayer("menu"); App.openLayer("power"); return; }
                        if (modelData[3]) App.navigate("missing", { missing: modelData[3] });
                        else App.navigate(modelData[0]);
                    }
                }
            }
        }
    }
}
