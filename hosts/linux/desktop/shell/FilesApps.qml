// Hearth Files' Installed apps page (docs/HEARTH-FILES.md "Installed apps"), like Windows Settings › Apps › Installed
// apps: every app with its icon, a kind tag (Hearth, Flatpak, Windows app, Steam, Recomp, Hearth Games), publisher,
// install date and size, sorted by name, size or date and filtered by kind, with a "···" menu (Open or Play, Modify,
// Move to another drive, Uninstall: FileManager.openAppMenu). The rows are FileManager's `entries` (app: …), so the
// keyboard and the controller move through them like a list. Loaded with a Loader: a problem here never stops Files.
import QtQuick
import QtQuick.Layouts

Item {
    id: page
    property var fm: null
    property var keysItem: null
    readonly property var ents: fm && fm.onApps ? fm.entries : []
    readonly property real total: ents.reduce((n, e) => n + (e.app.size || 0), 0)
    readonly property var tint: ["#2f9e44", "#e8590c", "#d6336c", "#1971c2", "#7048e8", "#0c8599", "#f08c00", "#5c7cfa"]

    function colorFor(name) {
        let h = 0;
        for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) & 0xffff;
        return tint[h % tint.length];
    }
    function subLine(a) {
        const out = [];
        if (a.part) out.push("Part of Hearth");
        else if (a.publisher) out.push(a.publisher);
        const dl = a.drive && a.drive !== "/" ? fm.driveLabelFor(a.drive) : "";
        if (dl) out.push("on " + dl);
        if (a.date) out.push(Qt.formatDate(new Date(a.date * 1000), "d MMM yyyy"));
        return out.join(" · ");
    }
    function ensure(item) {
        const p = item.mapToItem(col, 0, 0);
        if (p.y < flick.contentY + 4) flick.contentY = Math.max(0, p.y - 120);
        else if (p.y + item.height > flick.contentY + flick.height - 4) flick.contentY = Math.min(Math.max(0, flick.contentHeight - flick.height), p.y + item.height - flick.height + 12);
    }
    function menuAt(i, area, x, y) {
        if (keysItem) keysItem.forceActiveFocus();
        fm.zone = "main";
        fm.cur = i;
        const p = area.mapToItem(keysItem, x, y);
        fm.viaPad = false;
        fm.openMenu(false, p.x, p.y);
    }

    Flickable {
        id: flick
        anchors.fill: parent
        contentHeight: col.implicitHeight + 24
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Column {
            id: col
            x: 22
            width: page.width - 44
            spacing: 4
            topPadding: 12
            Text { text: "Installed apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 28; font.weight: Font.DemiBold }
            Text {
                width: parent.width
                bottomPadding: 10
                text: !page.fm ? "" : "Everything on this PC: Hearth, Flatpak, Windows apps, Steam games, Hearth Games and Recomps. "
                    + (page.fm.appsLoading && !page.ents.length ? "Looking for your apps…"
                       : page.fm.plural(page.ents.length, "app") + " found" + (page.total ? " · " + page.fm.fmtSize(page.total) : ""))
                color: Theme.muted; wrapMode: Text.WordWrap; font.family: Theme.font; font.pixelSize: 13
            }
            Row {
                spacing: 8
                bottomPadding: 8
                Repeater {
                    model: [
                        { id: "sort", label: "Sort by: " + ({ name: "Name", size: "Size", date: "Install date" })[page.fm ? page.fm.appsSort : "name"] },
                        { id: "filter", label: "Filter by: " + (page.fm ? page.fm.appKindName(page.fm.appsFilter) : "All") }
                    ]
                    delegate: Rectangle {
                        id: pill
                        required property var modelData
                        width: pillText.implicitWidth + 40
                        height: 34
                        radius: Theme.r(6)
                        color: pillMouse.containsMouse ? Theme.panel3 : Theme.panel2
                        border.width: 1
                        border.color: Theme.line
                        Text { id: pillText; anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter } text: pill.modelData.label + "  ⌄"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                        MouseArea {
                            id: pillMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (page.keysItem) page.keysItem.forceActiveFocus();
                                const p = pill.mapToItem(page.keysItem, 0, pill.height + 4);
                                page.fm.viaPad = false;
                                if (pill.modelData.id === "sort") page.fm.appsSortMenu(p.x, p.y);
                                else page.fm.appsFilterMenu(p.x, p.y);
                            }
                        }
                    }
                }
            }
            Repeater {
                model: page.ents
                delegate: Rectangle {
                    id: row
                    required property var modelData
                    required property int index
                    readonly property var a: modelData.app
                    readonly property bool isCur: !!page.fm && page.fm.cur === index
                    readonly property bool lit: isCur && page.fm.zone === "main"
                    width: col.width
                    height: 64
                    radius: Theme.r(6)
                    color: isCur ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : rowMouse.containsMouse ? Theme.panel3 : Theme.panel2
                    border.width: lit ? 2 : 0
                    border.color: Theme.accent
                    onIsCurChanged: if (isCur) Qt.callLater(page.ensure, row)
                    MouseArea {
                        id: rowMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: mouse => {
                            if (page.keysItem) page.keysItem.forceActiveFocus();
                            page.fm.zone = "main";
                            page.fm.cur = row.index;
                            if (mouse.button === Qt.RightButton) page.menuAt(row.index, rowMouse, mouse.x, mouse.y);
                        }
                        onDoubleClicked: page.fm.appOpen(row.a)
                        onPressAndHold: mouse => page.menuAt(row.index, rowMouse, mouse.x, mouse.y)
                    }
                    RowLayout {
                        anchors { fill: parent; leftMargin: 16; rightMargin: 10 }
                        spacing: 14
                        Item {
                            Layout.preferredWidth: 32
                            Layout.preferredHeight: 32
                            Rectangle {
                                anchors.fill: parent
                                visible: appIcon.status !== Image.Ready
                                radius: 7
                                color: page.colorFor(row.a.name || "?")
                                Text { anchors.centerIn: parent; text: (row.a.name || "?").charAt(0).toUpperCase(); color: "#ffffff"; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Bold }
                            }
                            Image {
                                id: appIcon
                                anchors.fill: parent
                                source: row.a.icon ? "file://" + row.a.icon : ""
                                sourceSize: Qt.size(64, 64)
                                asynchronous: true
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Row {
                                Layout.fillWidth: true
                                spacing: 8
                                Text { id: appName; text: row.a.name; color: Theme.text; elide: Text.ElideRight; width: Math.min(implicitWidth, row.width * 0.5); font.family: Theme.font; font.pixelSize: 14 }
                                Rectangle {
                                    anchors.verticalCenter: appName.verticalCenter
                                    width: tagText.implicitWidth + 14
                                    height: 18
                                    radius: 4
                                    color: row.a.kind === "winapp" ? "#2a3f52" : Theme.panel3
                                    Text { id: tagText; anchors.centerIn: parent; text: page.fm.appKindName(row.a.kind); color: row.a.kind === "winapp" ? "#9fd4ff" : Theme.text; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold }
                                }
                            }
                            Text { Layout.fillWidth: true; text: page.subLine(row.a); color: Theme.muted; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 12 }
                        }
                        Text {
                            text: row.a.size ? page.fm.fmtSize(row.a.size) : ""
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        }
                        Rectangle {
                            id: dots
                            Layout.preferredWidth: 36
                            Layout.preferredHeight: 36
                            radius: Theme.r(6)
                            color: dotsMouse.containsMouse ? Theme.panel3 : "transparent"
                            Text { anchors.centerIn: parent; text: "···"; color: Theme.text; font.pixelSize: 16; font.weight: Font.Bold }
                            MouseArea {
                                id: dotsMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: page.menuAt(row.index, dotsMouse, 0, dots.height + 2)
                            }
                        }
                    }
                }
            }
            Text {
                visible: !!page.fm && !page.fm.appsLoading && page.ents.length === 0
                topPadding: 30
                width: col.width
                horizontalAlignment: Text.AlignHCenter
                text: page.fm && page.fm.query ? "No app matches “" + page.fm.query + "”" : "No apps of this kind"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}
