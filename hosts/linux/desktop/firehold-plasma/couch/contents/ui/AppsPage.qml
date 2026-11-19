// Apps and Store (the tabs of mockups 1 and 5; no mockup of their own yet): the menu's apps as big tiles, and the
// stores. The first Apps tile is Settings › Couch and Desk.
import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: ap
    property QtObject couch
    readonly property real k: couch.k
    readonly property var storeIds: ["org.kde.discover", "steam", "com.heroicgameslauncher.hgl", "net.lutris.Lutris", "hearth-store"]
    readonly property var mediaCats: ["AudioVideo", "Audio", "Video", "Player", "TV"]
    readonly property var items: couch.tab === "store"
        ? couch.apps.filter(function (a) { return ap.storeIds.indexOf(a.id) >= 0; })
        : couch.tab === "media"
        // Media (Firehold on Plasma 1.16's tab): your video, music and streaming apps.
        ? couch.apps.filter(function (a) { return (a.categories || []).some(function (c) { return ap.mediaCats.indexOf(c) >= 0; }); })
        : [{ id: "::settings", name: "Couch and Desk", icon: "preferences-desktop", comment: "Firehold's settings" }].concat(couch.apps)
    property int current: 0
    function key(kk) {
        const n = items.length;
        if (kk === Qt.Key_Left) current = Math.max(0, current - 1);
        else if (kk === Qt.Key_Right) current = Math.min(n - 1, current + 1);
        else if (kk === Qt.Key_Up) current = Math.max(0, current - grid.cols);
        else if (kk === Qt.Key_Down) current = Math.min(n - 1, current + grid.cols);
        else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) open(items[current]);
        else return false;
        grid.positionViewAtIndex(current, GridView.Contain);
        return true;
    }
    function open(a) {
        if (!a) return;
        if (a.id === "::settings") couch.page = "settings";
        else couch.launchApp(a);
    }
    GridView {
        id: grid
        readonly property int cols: 8
        x: 72 * ap.k; y: 136 * ap.k
        width: cols * cellWidth; height: parent.height - y - 90 * ap.k
        cellWidth: 222 * ap.k; cellHeight: 222 * ap.k
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: ap.items
        highlightMoveDuration: 0
        cacheBuffer: 0
        delegate: Rectangle {
            required property var modelData
            required property int index
            width: 204 * ap.k; height: 204 * ap.k; radius: 16 * ap.k
            color: "#1d1512"
            border.width: index === ap.current ? 3 * ap.k : 1
            border.color: index === ap.current ? ap.couch.pal.accent : ap.couch.pal.line
            Kirigami.Icon {
                anchors.horizontalCenter: parent.horizontalCenter
                y: 36 * ap.k
                width: 88 * ap.k; height: width
                source: modelData.icon || "application-x-executable"
                fallback: "application-x-executable"
            }
            T {
                couchRef: ap.couch
                anchors.horizontalCenter: parent.horizontalCenter
                y: 146 * ap.k
                width: parent.width - 24 * ap.k
                horizontalAlignment: Text.AlignHCenter
                text: modelData.name
                font.pixelSize: 18 * ap.k; font.weight: Font.Medium
            }
            MouseArea { anchors.fill: parent; onClicked: { ap.current = index; ap.open(modelData); } }
        }
    }
    T {
        couchRef: ap.couch
        visible: ap.items.length === 0
        x: 72 * ap.k; y: 160 * ap.k
        text: ap.couch.tab === "store" ? "No store apps are installed" : "Reading your apps…"
        color: ap.couch.pal.dim; font.pixelSize: 22 * ap.k
    }
    Row {
        anchors.right: parent.right; anchors.rightMargin: 72 * ap.k
        y: 1032 * ap.k
        spacing: 30 * ap.k
        Hint { couch: ap.couch; key: "A"; label: "Open"; tint: ap.couch.pal.green }
        Hint { couch: ap.couch; key: "≡"; label: "Guide: Quick Access"; tint: "#a89a90" }
        Hint { couch: ap.couch; key: "LB RB"; label: "Tabs"; tint: "#a89a90" }
    }
}
