// The "Classic" app menu (Settings › Appearance › Bar and dock › App menu): two columns from the corner with the
// Hearth flame, glassy in Hearth's colours. Left: your frequent apps, each with its jump list (recent files where
// the app keeps them, else its own actions: the arrow, or Right), All apps opening in place, and the search field.
// Right: your places (home, Documents, Pictures, Music, Games), Files, Settings, Devices, Help, and Shut down with an
// arrow for the rest (Game Mode, Lock, Sleep, Restart, Log out).
// Keys / controller: Up / Down (D-pad) move, Right opens a jump list or goes to the places, Left comes back, Enter
// (A) opens, Esc (B) goes back or closes, the Menu key (X) for options, typing (Y) searches, Tab (LB / RB) switches
// column. Drawn over Launcher.qml (`menu`); loaded by file name.
import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: st
    property var menu: null
    readonly property color scrim: Qt.rgba(0, 0, 0, 0.12)
    readonly property bool wantsLive: false
    property string zone: "left"            // left | right | jump
    property bool allOpen: false
    property int cur: 0
    property int rightCur: 0
    property int jumpCur: 0
    property var jumpFor: null              // the app whose jump list is showing
    readonly property bool searching: field.text.trim() !== ""
    readonly property string home: Quickshell.env("HOME") || "/"
    readonly property var leftRows: !menu ? [] : allOpen ? menu.allApps : menu.frequent
    readonly property var places: [
        { label: menu ? menu.userName : "Home", icon: "user", run: () => st.menu.openFolder(st.home), bold: true },
        { label: "Documents", icon: "file", run: () => st.menu.openFolder(st.home + "/Documents") },
        { label: "Pictures", icon: "cam", run: () => st.menu.openFolder(st.home + "/Pictures") },
        { label: "Music", icon: "vol", run: () => st.menu.openFolder(st.home + "/Music") },
        { label: "Games", icon: "pad", run: () => { st.menu.close(); st.menu.pageRequested("games"); } },
        { sep: true },
        { label: "Files", icon: "folder", run: () => st.menu.openFolder(st.home) },
        { label: "Settings", icon: "gear", run: () => st.menu.power("settings") },
        { label: "Devices", icon: "bt", run: () => { st.menu.close(); st.menu.pageRequested("bluetooth"); } },
        { sep: true },
        { label: "Help", icon: "help", run: () => { st.menu.close(); Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "help", "toggle"]); } }
    ]
    readonly property var placeIdx: places.map((p, i) => p.sep ? -1 : i).filter(i => i >= 0)
    readonly property var jump: menu && jumpFor ? menu.jumpList(jumpFor) : []
    function opened() { zone = "left"; allOpen = false; cur = 0; rightCur = 0; jumpFor = null; field.text = ""; field.forceActiveFocus(); }

    MouseArea { anchors.fill: parent; onClicked: st.menu.close() }

    Item {
        id: panel
        width: Math.min(st.width - 20, 640)
        height: Math.min(st.height - DeskState.topInset - DeskState.bottomInset - 20, 700)
        readonly property point pos: st.menu ? st.menu.panelPos(width, height, "corner") : Qt.point(0, 0)
        x: pos.x; y: pos.y
        MouseArea { anchors.fill: parent }

        // Aero-style glass in Hearth's colours: a tinted, see-through frame with a light edge and a sheen.
        Rectangle {
            anchors.fill: parent
            radius: Theme.r(10)
            gradient: Gradient {
                GradientStop { position: 0; color: Qt.rgba(Theme.accent.r * 0.5 + Theme.panel.r * 0.5, Theme.accent.g * 0.5 + Theme.panel.g * 0.5, Theme.accent.b * 0.5 + Theme.panel.b * 0.5, 0.86) }
                GradientStop { position: 1; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.9) }
            }
            border.width: 1
            border.color: Qt.rgba(1, 1, 1, 0.28)
            Rectangle {   // the sheen
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 1 }
                height: parent.height * 0.4
                radius: parent.radius
                gradient: Gradient { GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.14) } GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) } }
            }
        }

        // ---- left column: frequent apps / All apps / search results, then All apps and the search field
        Rectangle {
            id: leftCol
            anchors { left: parent.left; top: parent.top; bottom: parent.bottom; margins: 10 }
            width: parent.width * 0.56
            radius: Theme.r(6)
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.94)
            border.color: Qt.rgba(1, 1, 1, 0.12)
            ColumnLayout {
                anchors { fill: parent; margins: 8 }
                spacing: 6
                ListView {
                    id: leftList
                    visible: !st.searching
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: st.leftRows
                    currentIndex: st.cur
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: Rectangle {
                        id: lr
                        required property var modelData
                        required property int index
                        readonly property bool lit: st.zone === "left" && st.cur === index || st.jumpFor === modelData && st.zone === "jump"
                        readonly property bool jl: !st.allOpen && st.menu && st.menu.hasJumpList(modelData)
                        width: ListView.view.width
                        height: st.allOpen ? 36 : 48
                        radius: Theme.r(4)
                        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22) : lra.containsMouse ? Theme.panel2 : "transparent"
                        border.width: lit ? 1.5 : 0; border.color: Theme.accent
                        RowLayout {
                            anchors { fill: parent; leftMargin: 8; rightMargin: 4 }
                            spacing: 10
                            AppIcon { size: st.allOpen ? 22 : 32; icon: lr.modelData.icon || ""; appId: lr.modelData.id || "" }
                            Text { Layout.fillWidth: true; text: lr.modelData.name; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: st.allOpen ? 13 : 14 }
                            Rectangle {   // the jump list arrow
                                visible: lr.jl
                                implicitWidth: 28; implicitHeight: lr.height - 10; radius: Theme.r(4)
                                color: jla.containsMouse ? Theme.panel3 : "transparent"
                                Rectangle { anchors { left: parent.left; verticalCenter: parent.verticalCenter } width: 1; height: parent.height - 8; color: Theme.line; visible: lr.lit }
                                Text { anchors.centerIn: parent; text: "›"; color: Theme.muted; font.pixelSize: 18 }
                                MouseArea { id: jla; anchors.fill: parent; hoverEnabled: true; onClicked: st.openJump(lr.index) }
                            }
                        }
                        MouseArea {
                            id: lra
                            anchors { fill: parent; rightMargin: lr.jl ? 32 : 0 }
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => { st.zone = "left"; st.cur = lr.index; if (mouse.button === Qt.RightButton) st.options(); else st.menu.launch(lr.modelData); }
                        }
                    }
                    header: Item { width: 1; height: st.allOpen ? 0 : 2 }
                }
                MenuSearch {
                    id: results
                    visible: st.searching
                    menu: st.menu
                    preview: false
                    chips: false
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                }
                Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.line; visible: !st.searching }
                Rectangle {   // All apps / Back
                    visible: !st.searching
                    Layout.fillWidth: true
                    implicitHeight: 36
                    radius: Theme.r(4)
                    color: aaa.containsMouse ? Theme.panel2 : "transparent"
                    RowLayout {
                        anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                        Text { Layout.fillWidth: true; text: st.allOpen ? "‹ Back" : "All apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                        Text { visible: !st.allOpen; text: "›"; color: Theme.muted; font.pixelSize: 18 }
                    }
                    MouseArea { id: aaa; anchors.fill: parent; hoverEnabled: true; onClicked: { st.allOpen = !st.allOpen; st.cur = 0; st.zone = "left"; } }
                }
                Rectangle {   // the search field
                    Layout.fillWidth: true
                    implicitHeight: 36
                    radius: Theme.r(4)
                    color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.9)
                    border.width: 1.5; border.color: st.searching ? Theme.accent : Theme.line
                    Icon { x: 10; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 15; color: Theme.muted }
                    TextInput {
                        id: field
                        anchors { fill: parent; leftMargin: 34; rightMargin: 8 }
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.text; selectionColor: Theme.accent
                        font.family: Theme.font; font.pixelSize: 13
                        focus: true
                        onTextChanged: results.query = text
                        Text { anchors.verticalCenter: parent.verticalCenter; visible: !field.text; text: "Search apps, games, Settings and files"; color: Theme.muted; font: field.font; elide: Text.ElideRight; width: field.width }
                        Keys.onPressed: e => st.key(e)
                    }
                }
            }
        }

        // ---- right column: your picture, the places (or a jump list), Shut down
        Item {
            id: rightCol
            anchors { left: leftCol.right; right: parent.right; top: parent.top; bottom: parent.bottom; leftMargin: 10; rightMargin: 12; topMargin: 12; bottomMargin: 12 }
            Rectangle {   // your picture: the first letter on the accent, framed like the old glass
                id: pic
                anchors.horizontalCenter: parent.horizontalCenter
                y: 0
                width: 64; height: 64; radius: Theme.r(8)
                color: Theme.accent
                border.width: 3; border.color: Qt.rgba(1, 1, 1, 0.55)
                Text { anchors.centerIn: parent; text: st.menu ? st.menu.userName.charAt(0).toUpperCase() : ""; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 28; font.bold: true }
            }
            ColumnLayout {
                anchors { left: parent.left; right: parent.right; top: pic.bottom; bottom: powerRow.top; topMargin: 16; bottomMargin: 10 }
                spacing: 2
                // the places
                Repeater {
                    model: st.zone === "jump" ? [] : st.places
                    delegate: Item {
                        id: pl
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        implicitHeight: modelData.sep ? 12 : 36
                        Rectangle { visible: !!pl.modelData.sep; anchors.centerIn: parent; width: parent.width - 16; height: 1; color: Qt.rgba(1, 1, 1, 0.16) }
                        Rectangle {
                            visible: !pl.modelData.sep
                            anchors.fill: parent
                            radius: Theme.r(4)
                            readonly property bool lit: st.zone === "right" && st.placeIdx[st.rightCur] === pl.index
                            color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.3) : pla.containsMouse ? Qt.rgba(1, 1, 1, 0.1) : "transparent"
                            border.width: lit ? 1.5 : 0; border.color: Qt.rgba(1, 1, 1, 0.6)
                            RowLayout {
                                anchors { fill: parent; leftMargin: 10 }
                                spacing: 10
                                Icon { name: pl.modelData.icon || "dot"; size: 16 }
                                Text { Layout.fillWidth: true; text: pl.modelData.label || ""; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: pl.modelData.bold ? Font.DemiBold : Font.Normal }
                            }
                            MouseArea { id: pla; anchors.fill: parent; hoverEnabled: true; onClicked: pl.modelData.run() }
                        }
                    }
                }
                // a jump list: recent files, then the app's own actions
                Text { visible: st.zone === "jump"; text: st.jump.some(j => j.kind === "file") ? "Recent" : "Tasks"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; Layout.leftMargin: 8 }
                Repeater {
                    model: st.zone === "jump" ? st.jump : []
                    delegate: Rectangle {
                        id: jr
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        implicitHeight: modelData.sub ? 44 : 34
                        radius: Theme.r(4)
                        readonly property bool lit: st.jumpCur === index
                        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.3) : jra.containsMouse ? Qt.rgba(1, 1, 1, 0.1) : "transparent"
                        border.width: lit ? 1.5 : 0; border.color: Qt.rgba(1, 1, 1, 0.6)
                        RowLayout {
                            anchors { fill: parent; leftMargin: 10; rightMargin: 6 }
                            spacing: 10
                            Icon { name: jr.modelData.icon; size: 16 }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Text { Layout.fillWidth: true; text: jr.modelData.label; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                Text { Layout.fillWidth: true; visible: !!jr.modelData.sub; text: jr.modelData.sub || ""; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                            }
                        }
                        MouseArea { id: jra; anchors.fill: parent; hoverEnabled: true; onClicked: jr.modelData.run() }
                    }
                }
                Item { Layout.fillHeight: true }
            }
            // Shut down, with an arrow for the rest
            RowLayout {
                id: powerRow
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                spacing: 0
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 38
                    radius: Theme.r(4)
                    readonly property bool lit: st.zone === "right" && st.rightCur === st.placeIdx.length
                    color: sda.containsMouse || lit ? Qt.rgba(1, 1, 1, 0.16) : Qt.rgba(1, 1, 1, 0.08)
                    border.width: lit ? 1.5 : 1; border.color: Qt.rgba(1, 1, 1, lit ? 0.7 : 0.3)
                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 8
                        Icon { name: "power"; size: 16 }
                        Text { text: "Shut down"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    }
                    MouseArea { id: sda; anchors.fill: parent; hoverEnabled: true; onClicked: st.menu.power("shutdown") }
                }
                Rectangle {
                    id: moreBtn
                    implicitWidth: 38; implicitHeight: 38
                    radius: Theme.r(4)
                    readonly property bool lit: st.zone === "right" && st.rightCur === st.placeIdx.length + 1
                    color: ma.containsMouse || lit ? Qt.rgba(1, 1, 1, 0.16) : Qt.rgba(1, 1, 1, 0.08)
                    border.width: lit ? 1.5 : 1; border.color: Qt.rgba(1, 1, 1, lit ? 0.7 : 0.3)
                    Text { anchors.centerIn: parent; text: "›"; color: Theme.text; font.pixelSize: 18 }
                    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; onClicked: st.menu.powerMenu(moreBtn, true) }
                }
            }
        }
    }
    // Hints, beside the menu
    Rectangle {
        x: panel.x + panel.width + 12
        y: panel.y + panel.height - height
        visible: x + width < st.width - 10
        width: hintRow.implicitWidth + 24; height: 40
        radius: Theme.r(8)
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.9)
        border.color: Theme.line
        MenuHints { id: hintRow; anchors.centerIn: parent; model: [{ b: "A", t: "Open" }, { b: "B", t: "Close" }, { b: "→", t: "Jump list" }, { b: "Y", t: "Search" }] }
    }

    // ------------------------------------------------------------------ keys
    function openJump(i) {
        const e = leftRows[i];
        if (!e || !menu.hasJumpList(e)) return false;
        cur = i; jumpFor = e; jumpCur = 0; zone = "jump";
        return true;
    }
    function options() {
        const e = leftRows[cur], it = leftList.itemAtIndex(cur);
        if (e && it) menu.rowMenu({ kind: "app", item: e }, it);
    }
    function key(e) {
        if (searching) {
            switch (e.key) {
            case Qt.Key_Down: results.move(1); break;
            case Qt.Key_Up: results.move(-1); break;
            case Qt.Key_Return: case Qt.Key_Enter: results.activate(); break;
            case Qt.Key_Menu: results.options(); break;
            case Qt.Key_Escape: field.text = ""; break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        if (zone === "jump") {
            switch (e.key) {
            case Qt.Key_Down: jumpCur = Math.min(jump.length - 1, jumpCur + 1); break;
            case Qt.Key_Up: jumpCur = Math.max(0, jumpCur - 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: { const j = jump[jumpCur]; if (j) j.run(); break; }
            case Qt.Key_Left: case Qt.Key_Escape: zone = "left"; jumpFor = null; break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        if (zone === "right") {
            const n = placeIdx.length + 2;   // the places, Shut down, the arrow
            switch (e.key) {
            case Qt.Key_Down: rightCur = Math.min(n - 1, rightCur + 1); break;
            case Qt.Key_Up: rightCur = Math.max(0, rightCur - 1); break;
            case Qt.Key_Right: if (rightCur === n - 2) rightCur = n - 1; break;
            case Qt.Key_Left: if (rightCur === n - 1) rightCur = n - 2; else zone = "left"; break;
            case Qt.Key_Tab: case Qt.Key_Backtab: zone = "left"; break;
            case Qt.Key_Return: case Qt.Key_Enter:
                if (rightCur < placeIdx.length) places[placeIdx[rightCur]].run();
                else if (rightCur === n - 2) menu.power("shutdown");
                else menu.powerMenu(moreBtn, true);
                break;
            case Qt.Key_Escape: menu.close(); break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        const n = leftRows.length;
        switch (e.key) {
        case Qt.Key_Down: cur = Math.min(n - 1, cur + 1); leftList.positionViewAtIndex(cur, ListView.Contain); break;
        case Qt.Key_Up: cur = Math.max(0, cur - 1); leftList.positionViewAtIndex(cur, ListView.Contain); break;
        case Qt.Key_Right: if (!openJump(cur)) zone = "right"; break;
        case Qt.Key_Tab: case Qt.Key_Backtab: zone = "right"; break;
        case Qt.Key_Return: case Qt.Key_Enter: { const a = leftRows[cur]; if (a) menu.launch(a); break; }
        case Qt.Key_Menu: options(); break;
        case Qt.Key_Escape: if (allOpen) { allOpen = false; cur = 0; } else menu.close(); break;
        default: return;
        }
        e.accepted = true;
    }
}
