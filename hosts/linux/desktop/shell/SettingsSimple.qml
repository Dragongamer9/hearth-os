// Hearth Settings' fallback: a plain list of every page on the left and the page on the right, used only when the
// redesign's frame (SettingsFrame.qml) can't load, so Settings always opens. Up/Down pick a page, Right / A go into
// it, Left / B come back, B on the list closes. Kept simple on purpose: nothing here but a list and a Loader.
import QtQuick
import QtQuick.Layouts
import "settings-pages.js" as Reg

Item {
    id: simple
    required property var app
    readonly property var list: Reg.PAGES.filter(p => (!p.live || app.live) && p.file !== "SetCategory.qml" && p.file !== "SetHome.qml" && p.file !== "SetGaming.qml")
    Component.onCompleted: { SettingsNav.navTarget = nav; nav.forceActiveFocus(); }
    function current() { const i = list.findIndex(p => p.id === app.page); return i < 0 ? 0 : i; }
    function isActive() { return false; }

    RowLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 270
            color: Theme.panel
            ListView {
                id: nav
                anchors { fill: parent; margins: 12 }
                model: simple.list
                clip: true
                spacing: 2
                keyNavigationEnabled: false
                currentIndex: simple.current()
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                Keys.onPressed: e => {
                    switch (e.key) {
                    case Qt.Key_Down: simple.app.page = simple.list[Math.min(simple.list.length - 1, currentIndex + 1)].id; break;
                    case Qt.Key_Up: simple.app.page = simple.list[Math.max(0, currentIndex - 1)].id; break;
                    case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: if (loader.item && loader.item.focusFirst) loader.item.focusFirst(); break;
                    case Qt.Key_Escape: simple.app.close(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
                delegate: Rectangle {
                    id: row
                    required property var modelData
                    readonly property bool lit: simple.app.page === modelData.id
                    width: ListView.view.width
                    height: 40
                    radius: 8
                    color: lit ? (nav.activeFocus ? Theme.accent : Theme.panel3) : "transparent"
                    Text {
                        anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                        text: Reg.pathOf(row.modelData.id)
                        color: row.lit && nav.activeFocus ? Theme.accentInk : Theme.text
                        font.family: Theme.font; font.pixelSize: 14
                    }
                    MouseArea { anchors.fill: parent; onClicked: { simple.app.page = row.modelData.id; nav.forceActiveFocus(); } }
                }
            }
        }
        Loader {
            id: loader
            Layout.fillWidth: true
            Layout.fillHeight: true
            readonly property var entry: simple.list[simple.current()] || simple.list[0]
            source: Qt.resolvedUrl(entry.file)
            onLoaded: { if (item && entry.props && entry.props.part !== undefined) item.part = entry.props.part; }
            onStatusChanged: if (status === Loader.Error) console.warn("Hearth Settings: couldn't load", source)
        }
    }
}
