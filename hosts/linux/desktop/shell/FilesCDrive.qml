// Hearth Files' Hearth (C:) with Windows folder names (docs/HEARTH-FILES.md "Hearth (C:) the Windows way", the
// owner's approved mockup "Hearth (C:) drive layout"): C:\ (Program Files, Program Files (x86), ProgramData, Users,
// Windows, and the real Linux folders under their own heading when "Show the Linux folders too" is on), Program Files
// (every installed app, one folder each), Program Files (x86) (Windows apps and Steam) and Users\<you>\AppData.
// A view only: nothing on disk moves or is made. The rows are FileManager's `entries` (each with `cv`: type, size,
// On Linux, the C:\ name, where it really is), so the keyboard and the controller move through them like a list, and
// opening one goes to its real folder. Loaded with a Loader: a problem here never stops Files or Desktop Mode.
import QtQuick
import QtQuick.Layouts

Item {
    id: page
    property var fm: null
    property var keysItem: null
    readonly property var ents: fm && fm.onCView ? fm.entries : []
    readonly property string pg: fm ? fm.cPageNow : ""
    readonly property var tint: ["#2f9e44", "#e8590c", "#d6336c", "#1971c2", "#7048e8", "#0c8599", "#f08c00", "#5c7cfa"]
    readonly property string banner: ({
        root: "This is Hearth’s drive with Windows folder names. Nothing has been moved: each folder shows where its files really are.",
        pf: "Every installed app in one place. These are names only: Flatpaks, Hearth’s apps and games stay where they are.",
        x86: "Windows apps and Steam, named the way Windows names them. Each Windows app still keeps its own space.",
        appdata: "Where apps keep their settings and data, like on Windows: Roaming is ~/.config, Local is ~/.local/share."
    })[pg] || ""
    // Name, Type, Size, On Linux (the mockup's columns: 2.2 / 1.5 / 0.7 / 2.4).
    readonly property real wName: (col.width - 20) * 2.2 / 6.8
    readonly property real wType: (col.width - 20) * 1.5 / 6.8
    readonly property real wSize: (col.width - 20) * 0.7 / 6.8

    function colorFor(name) {
        let h = 0;
        for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) & 0xffff;
        return tint[h % tint.length];
    }
    function ensure(item) {
        const p = item.mapToItem(col, 0, 0);
        if (p.y < flick.contentY + 4) flick.contentY = Math.max(0, p.y - 60);
        else if (p.y + item.height > flick.contentY + flick.height - 4) flick.contentY = Math.min(Math.max(0, flick.contentHeight - flick.height), p.y + item.height - flick.height + 12);
    }
    function folderMenu(area, x, y) {
        if (keysItem) keysItem.forceActiveFocus();
        const p = area.mapToItem(keysItem, x, y);
        fm.viaPad = false;
        fm.openMenu(true, p.x, p.y);
    }
    function menuAt(i, area, x, y) {
        if (keysItem) keysItem.forceActiveFocus();
        fm.zone = "main";
        fm.cur = i;
        const p = area.mapToItem(keysItem, x, y);
        fm.viaPad = false;
        fm.openMenu(false, p.x, p.y);
    }

    // Dropped here: refused, saying why (Program Files fills itself when you install an app).
    DropArea {
        anchors.fill: parent
        keys: ["text/uri-list"]
        onDropped: drop => { if (page.fm) page.fm.dropOn(drop, page.fm.path); }
    }
    Flickable {
        id: flick
        anchors.fill: parent
        contentHeight: col.implicitHeight + 24
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        // Empty space: the folder's own menu (right-click or press and hold): the switch, Copy path, Properties.
        MouseArea {
            id: bgMouse
            width: flick.width
            height: Math.max(flick.height, flick.contentHeight)
            acceptedButtons: Qt.RightButton
            onClicked: mouse => page.folderMenu(bgMouse, mouse.x, mouse.y)
        }
        Column {
            id: col
            x: 18
            width: page.width - 36
            spacing: 0
            topPadding: 14

            // The note at the top (the mockup's blue banner), and on C:\ the "Show the Linux folders too" switch.
            RowLayout {
                width: col.width
                spacing: 12
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: bannerText.implicitHeight + 20
                    radius: Theme.r(8)
                    color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.10)
                    border.width: 1
                    border.color: Theme.line
                    RowLayout {
                        anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                        spacing: 12
                        Icon { name: "info"; size: 18; color: Theme.accent }
                        Text { id: bannerText; Layout.fillWidth: true; text: page.banner; color: Theme.text; wrapMode: Text.WordWrap; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
                Rectangle {
                    visible: page.pg === "root"
                    Layout.preferredWidth: togRow.implicitWidth + 20
                    Layout.preferredHeight: 36
                    radius: Theme.r(6)
                    color: togMouse.containsMouse ? Theme.panel3 : "transparent"
                    Row {
                        id: togRow
                        anchors.centerIn: parent
                        spacing: 8
                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            width: 34; height: 18; radius: 9
                            color: page.fm && page.fm.cLinux ? Theme.accent : Theme.panel3
                            Rectangle { y: 3; x: page.fm && page.fm.cLinux ? 19 : 3; width: 12; height: 12; radius: 6; color: "#ffffff" }
                        }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: "Show the Linux folders too"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    }
                    MouseArea {
                        id: togMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: { if (page.keysItem) page.keysItem.forceActiveFocus(); page.fm.setFilesPref("linuxFolders", !page.fm.cLinux); }
                    }
                    Accessible.role: Accessible.CheckBox
                    Accessible.name: "Show the Linux folders too"
                    Accessible.checked: !!page.fm && page.fm.cLinux
                }
            }
            Item { width: 1; height: 10 }

            // The column heads.
            Item {
                width: col.width
                height: 30
                Row {
                    anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                    Text { width: page.wName; anchors.verticalCenter: parent.verticalCenter; text: "Name ↑"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Text { width: page.wType; anchors.verticalCenter: parent.verticalCenter; text: "Type"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Text { width: page.wSize - 14; anchors.verticalCenter: parent.verticalCenter; horizontalAlignment: Text.AlignRight; text: "Size"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Item { width: 14; height: 1 }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "On Linux"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.line }
            }
            Item { width: 1; height: 4 }

            Repeater {
                model: page.ents
                delegate: Column {
                    id: rowBox
                    required property var modelData
                    required property int index
                    readonly property var cv: modelData.cv || ({})
                    width: col.width
                    // "THE LINUX FOLDERS" over the first real top-level folder.
                    Text {
                        visible: !!rowBox.cv.group
                        topPadding: 14; bottomPadding: 4; leftPadding: 10
                        text: rowBox.cv.group || ""
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.5
                    }
                    Rectangle {
                        id: row
                        readonly property bool isCur: !!page.fm && page.fm.cur === rowBox.index
                        readonly property bool lit: isCur && page.fm.zone === "main"
                        width: col.width
                        height: 36
                        radius: Theme.r(6)
                        color: isCur ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : rowMouse.containsMouse ? Theme.panel2 : "transparent"
                        border.width: lit ? 1.5 : 0
                        border.color: Theme.accent
                        opacity: rowBox.cv.dim ? 0.75 : 1
                        onIsCurChanged: if (isCur) Qt.callLater(page.ensure, row)
                        Accessible.role: Accessible.ListItem
                        Accessible.name: rowBox.modelData.name + ", " + (rowBox.cv.type || "") + ", on Linux " + (rowBox.cv.linux || "")
                        MouseArea {
                            id: rowMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                            onClicked: mouse => {
                                if (page.keysItem) page.keysItem.forceActiveFocus();
                                page.fm.zone = "main";
                                page.fm.cur = rowBox.index;
                                if (mouse.button === Qt.RightButton) page.menuAt(rowBox.index, rowMouse, mouse.x, mouse.y);
                                else if (mouse.button === Qt.MiddleButton) page.fm.newTab(rowBox.modelData.path);
                            }
                            onDoubleClicked: page.fm.activate(rowBox.modelData)
                            onPressAndHold: mouse => page.menuAt(rowBox.index, rowMouse, mouse.x, mouse.y)
                        }
                        DropArea {
                            anchors.fill: parent
                            keys: ["text/uri-list"]
                            // Onto a row: into that real folder (an app's folder, Users…) as anywhere else, or refused
                            // with why for the view's own folders and Windows / ProgramData.
                            onDropped: drop => page.fm.dropOn(drop, rowBox.modelData.path)
                        }
                        Row {
                            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                            Item {
                                width: page.wName
                                height: parent.height
                                Item {
                                    id: glyph
                                    width: 20; height: 20
                                    anchors.verticalCenter: parent.verticalCenter
                                    readonly property var a: rowBox.cv.app || null
                                    WinGlyph { anchors.centerIn: parent; visible: !glyph.a; kind: "folder"; size: 20 }
                                    Rectangle {
                                        anchors.fill: parent
                                        visible: !!glyph.a && appIcon.status !== Image.Ready
                                        radius: 4
                                        color: page.colorFor(rowBox.modelData.name)
                                        Text { anchors.centerIn: parent; text: rowBox.modelData.name.charAt(0).toUpperCase(); color: "#ffffff"; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold }
                                    }
                                    Image {
                                        id: appIcon
                                        anchors.fill: parent
                                        visible: !!glyph.a
                                        source: glyph.a && glyph.a.icon ? "file://" + glyph.a.icon : ""
                                        sourceSize: Qt.size(40, 40)
                                        asynchronous: true
                                        fillMode: Image.PreserveAspectFit
                                    }
                                    // Look only (Windows, ProgramData, Hearth's own apps): a small lock.
                                    Rectangle {
                                        visible: !!rowBox.cv.lock
                                        x: 12; y: 11
                                        width: 13; height: 13; radius: 6.5
                                        color: Theme.panel
                                        Icon { anchors.centerIn: parent; name: "lock"; size: 10; color: Theme.text }
                                    }
                                }
                                Text {
                                    anchors { left: glyph.right; leftMargin: 10; right: parent.right; rightMargin: 8; verticalCenter: parent.verticalCenter }
                                    text: rowBox.modelData.name
                                    color: Theme.text; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 13
                                    font.weight: row.isCur ? Font.DemiBold : Font.Normal
                                }
                            }
                            Text { width: page.wType; anchors.verticalCenter: parent.verticalCenter; text: rowBox.cv.type || ""; color: Theme.muted; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 13 }
                            Text { width: page.wSize - 14; anchors.verticalCenter: parent.verticalCenter; horizontalAlignment: Text.AlignRight; text: rowBox.cv.size ? page.fm.fmtSize(rowBox.cv.size) : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                            Item { width: 14; height: 1 }
                            Text {
                                width: parent.width - page.wName - page.wType - page.wSize
                                anchors.verticalCenter: parent.verticalCenter
                                text: rowBox.cv.linux || ""
                                color: Theme.muted; elide: Text.ElideRight; font.family: "monospace"; font.pixelSize: 12
                            }
                        }
                    }
                }
            }
            Text {
                visible: !!page.fm && page.ents.length === 0
                topPadding: 30
                width: col.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: !page.fm ? "" : page.fm.query ? "Nothing here matches “" + page.fm.query + "”"
                    : page.fm.appsLoading ? "Looking for your apps…" : page.pg === "x86" ? "No Windows apps or Steam yet" : "Nothing here yet"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}
