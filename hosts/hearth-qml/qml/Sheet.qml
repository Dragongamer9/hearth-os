// A short list of choices over the screen (the web page's openSheet): a title, a line about it, the choices, Cancel.
import QtQuick

Item {
    id: sh
    readonly property bool open: App.layers.indexOf("sheet") >= 0
    readonly property var s: App.sheet || ({ title: "", items: [] })
    visible: open
    Rectangle { anchors.fill: parent; color: "#000000"; opacity: 0.6
        MouseArea { anchors.fill: parent; onClicked: App.closeLayer("sheet") } }
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.u(60)
        width: Math.min(parent.width - Theme.u(40), Theme.u(560))
        height: col.implicitHeight + Theme.u(36)
        radius: Theme.u(16)
        color: Theme.panel
        border.color: Theme.line
        Column {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(18) }
            spacing: Theme.u(4)
            Text { text: sh.s.title || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(18); font.weight: Font.Bold; width: parent.width; wrapMode: Text.WordWrap }
            Text { visible: !!sh.s.sub; text: sh.s.sub || ""; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13); width: parent.width; wrapMode: Text.WordWrap; bottomPadding: Theme.u(8) }
            Repeater {
                model: sh.s.items || []
                delegate: Btn {
                    required property var modelData
                    required property int index
                    width: parent.width
                    kind: modelData.danger ? "danger" : "menu"
                    navScope: "sheet"
                    navKey: "sh-" + index
                    navFirst: modelData.on || index === 0
                    icon: modelData.icon || ""
                    text: modelData.label || ""
                    sub: modelData.sub || ""
                    on: !!modelData.on
                    onActivated: { const run = modelData.run; App.closeLayer("sheet"); if (run) run(); }
                }
            }
            Btn { visible: !sh.s.noCancel; width: parent.width; kind: "menu"; navScope: "sheet"; navKey: "sh-cancel"; icon: "x"; text: "Cancel"; onActivated: App.closeLayer("sheet") }
        }
    }
}
