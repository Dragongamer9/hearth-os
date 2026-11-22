// A desktop layout drawn small (Settings › Desktop layout's pictures, docs/mockups/desktop-layouts board 6): the main
// bar on its edge with its blocks as dots (the flame and Game Mode in the accent), the extra panels, and the floating
// dock. Drawn from the layout's own data, so a layout you made gets its picture too.
import QtQuick

Rectangle {
    id: m
    property var lay: null
    readonly property var d: lay && lay.desktop ? lay.desktop : ({})
    readonly property bool one: d.barLayout === "one"
    readonly property string mainEdge: one ? (d.barEdge || "bottom") : "top"
    readonly property bool dock: !one || d.dockFloat !== "off"
    readonly property bool barStyle: d.barStyle === "bar"
    readonly property var bar: lay && lay.bar ? lay.bar : ({ start: ["menu"], end: ["network", "volume", "battery"] })
    readonly property real t: Math.max(5, height * 0.065)
    color: Theme.bg2
    radius: Theme.r(8)
    clip: true

    component Strip: Rectangle {
        property string edge: "top"
        property var dots: []
        property bool floats: false
        readonly property bool vert: edge === "left" || edge === "right"
        x: edge === "right" ? m.width - width - (floats ? 3 : 0) : floats ? 3 : 0
        y: edge === "bottom" ? m.height - height - (floats ? 3 : 0) : floats ? 3 : 0
        width: vert ? m.t : m.width - (floats ? 6 : 0)
        height: vert ? m.height - (floats ? 6 : 0) : m.t
        radius: floats ? m.t / 2 : 0
        color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.13)
        Flow {
            anchors { fill: parent; margins: 2 }
            flow: parent.vert ? Flow.TopToBottom : Flow.LeftToRight
            spacing: 2
            Repeater {
                model: parent.parent.dots
                Rectangle {
                    required property var modelData
                    width: modelData === "spacer" ? m.t * 2 : m.t - 4; height: m.t - 4; radius: 1.5
                    color: modelData === "menu" || modelData === "gamemode" ? Theme.accent : modelData === "spacer" ? "transparent" : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.35)
                }
            }
        }
    }
    Strip {
        edge: m.mainEdge
        floats: !m.barStyle && m.d.barStyle !== undefined && m.d.barStyle === "floating"
        dots: (m.bar.start || []).concat(["spacer"], m.bar.centre || [], ["spacer"], m.bar.end || [], ["gamemode"]).slice(0, 18)
    }
    Repeater {
        model: m.lay && m.lay.panels ? m.lay.panels : []
        Strip { required property var modelData; edge: modelData.edge; floats: !!modelData.style; dots: (modelData.items || []).slice(0, 14) }
    }
    Rectangle {   // the floating dock
        visible: m.dock
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 4 }
        width: m.width * 0.3; height: m.t * 1.2; radius: height / 2
        color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.18)
    }
}
