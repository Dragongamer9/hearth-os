// Hearth's Alt+Tab for Mantel on KWin (a KWin window switcher package, picked by Hearth's KWin defaults:
// hosts/linux/desktop/kwin/windows/xdg/kwinrc [TabBox] LayoutName=org.hearth.switcher; Settings › Window effects ›
// Alt+Tab lists it as "Hearth"). A card in the middle of the screen with a live picture of each window in a row, the
// app's icon and title under it, the one you'll switch to lit in the accent colour; the controller's LB / RB (Alt+Tab
// through hearth-pad) walk it the same way. A click picks a window, the X on a picture closes it.
// The colours are Hearth's KDE colour scheme (hearth-theme writes it to kdeglobals for your look and accent), which is
// what Kirigami.Theme reads inside KWin; the shape (corner radius, sizes, spacing) is Mantel's. Installed to
// /usr/share/kwin/tabbox (and kwin-wayland/tabbox, newer KWin's folder) from os/system_files.
import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kwin as KWin

KWin.TabBoxSwitcher {
    id: tabBox
    currentIndex: list.currentIndex

    readonly property color bg: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.96)
    readonly property color cell: Qt.lighter(Kirigami.Theme.backgroundColor, 1.35)
    readonly property color accent: Kirigami.Theme.highlightColor
    readonly property color ink: Kirigami.Theme.textColor
    readonly property color muted: Kirigami.Theme.disabledTextColor
    readonly property int thumbH: Math.round(Math.min(180, tabBox.screenGeometry.height * 0.16))

    Window {
        id: dialog
        visible: tabBox.visible
        flags: Qt.BypassWindowManagerHint | Qt.FramelessWindowHint
        color: "transparent"
        width: card.width
        height: card.height
        x: tabBox.screenGeometry.x + Math.round((tabBox.screenGeometry.width - width) / 2)
        y: tabBox.screenGeometry.y + Math.round((tabBox.screenGeometry.height - height) / 2)

        Rectangle {
            id: card
            width: Math.min(tabBox.screenGeometry.width - 80, list.contentWidth + 36)
            height: tabBox.thumbH + 96
            radius: 18
            color: tabBox.bg
            border.width: 1
            border.color: Qt.rgba(tabBox.ink.r, tabBox.ink.g, tabBox.ink.b, 0.12)

            ListView {
                id: list
                anchors { fill: parent; margins: 18 }
                orientation: ListView.Horizontal
                spacing: 12
                clip: true
                model: tabBox.model
                highlightMoveDuration: 0
                keyNavigationWraps: true
                boundsBehavior: Flickable.StopAtBounds
                delegate: Item {
                    id: d
                    required property int index
                    required property var model
                    readonly property bool current: ListView.isCurrentItem
                    width: tabBox.thumbH * 1.6 + 16
                    height: list.height
                    Rectangle {
                        anchors.fill: parent
                        radius: 12
                        color: d.current ? Qt.rgba(tabBox.accent.r, tabBox.accent.g, tabBox.accent.b, 0.18) : hover.hovered ? tabBox.cell : "transparent"
                        border.width: d.current ? 2 : 0
                        border.color: tabBox.accent
                    }
                    KWin.WindowThumbnail {
                        id: thumb
                        x: 8; y: 8
                        width: parent.width - 16
                        height: tabBox.thumbH
                        wId: d.model.windowId
                    }
                    RowLayout {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 10 }
                        spacing: 8
                        Kirigami.Icon {
                            Layout.preferredWidth: 24; Layout.preferredHeight: 24
                            source: d.model.icon
                        }
                        Text {
                            Layout.fillWidth: true
                            text: d.model.caption
                            elide: Text.ElideRight
                            color: d.model.minimized ? tabBox.muted : tabBox.ink
                            font.pixelSize: 14
                            font.weight: d.current ? Font.DemiBold : Font.Normal
                        }
                    }
                    HoverHandler { id: hover }
                    TapHandler {
                        onSingleTapped: {
                            if (d.index === list.currentIndex) { tabBox.model.activate(d.index); return; }
                            list.currentIndex = d.index;
                        }
                        onDoubleTapped: tabBox.model.activate(d.index)
                    }
                    Rectangle {   // close the window from here
                        visible: d.model.closeable && hover.hovered
                        x: parent.width - width - 12; y: 12
                        width: 26; height: 26; radius: 13
                        color: closeHover.hovered ? "#e5484d" : tabBox.cell
                        Text { anchors.centerIn: parent; text: "✕"; color: tabBox.ink; font.pixelSize: 12 }
                        HoverHandler { id: closeHover }
                        TapHandler { onTapped: tabBox.model.close(d.index) }
                    }
                }
                Connections {
                    target: tabBox
                    function onCurrentIndexChanged() { if (list.currentIndex !== tabBox.currentIndex) list.currentIndex = tabBox.currentIndex; }
                }
            }
            Text {
                anchors.centerIn: parent
                visible: list.count === 0
                text: "No windows"
                color: tabBox.muted
                font.pixelSize: 16
            }
        }
    }
}
