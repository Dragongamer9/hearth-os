// The widgets you put in the bar (Add widgets › Add to bar): clock with a city, weather, system monitor, battery,
// timer, each the small form of its desktop widget (Widget*.qml with compact on). Right-click one to take it out of
// the bar. Islands.qml loads this by file name; it is empty (and takes no room) until you add one.
import QtQuick
import QtQuick.Layouts
import "widget-layout.js" as L

RowLayout {
    id: bw
    property bool vertical: false
    readonly property var list: { void WidgetStore.rev; return L.forBar(WidgetStore.widgets, Activities.current); }
    readonly property bool wanted: list.length > 0
    spacing: 14
    Repeater {
        model: bw.list
        delegate: Item {
            id: slot
            required property var modelData
            implicitWidth: ld.item ? ld.item.implicitWidth : 0
            implicitHeight: 28
            Loader {
                id: ld
                anchors.verticalCenter: parent.verticalCenter
                source: "Widget" + slot.modelData.type.charAt(0).toUpperCase() + slot.modelData.type.slice(1) + ".qml"
                onLoaded: {
                    item.compact = true;
                    item.wid = Qt.binding(() => slot.modelData.id);
                    item.cfg = Qt.binding(() => slot.modelData.cfg);
                    item.height = 28;
                }
            }
            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.RightButton
                onClicked: { WidgetStore.remove(slot.modelData.id); SettingsNav.toast("Taken out of the bar"); }
            }
        }
    }
}
