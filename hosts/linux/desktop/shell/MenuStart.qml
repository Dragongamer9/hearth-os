// The "Start screen" app menu (Settings › Appearance › Bar and dock › App menu): the whole screen, big live tiles
// in groups (Play, Today, Home server, Apps) that scroll sideways, All apps below (A–Z in columns, sideways too) and
// a full-screen search. Big targets that work well from the couch and on handhelds: the default app menu on new
// handheld installs only (Theme.appMenu). Right-click / X on a tile: Small, Medium, Wide, Large or Take off the menu.
// Keys / controller: arrows (D-pad) move between tiles, Down past the last row (or the ↓ button) opens All apps,
// Enter (A) opens, Esc (B) goes back to the desktop, the Menu key (X) for options, typing (Y) searches,
// Tab / Shift+Tab (RB / LB) scroll a group at a time.
// Drawn over Launcher.qml (`menu`); loaded by file name.
import QtQuick
import QtQuick.Layouts
import Quickshell
import "menu-nav.js" as Nav

Item {
    id: st
    property var menu: null
    readonly property color scrim: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.975)
    readonly property bool wantsLive: true
    property string view: "tiles"           // tiles | all
    property string tileId: ""
    property int allCur: 0
    readonly property bool searching: field.text.trim() !== ""
    readonly property real k: Math.max(0.8, Math.min(1.25, height / 1080))
    readonly property real unit: Math.max(50, Math.min(80, (height - 300) / 8.8))
    readonly property real gap: 8
    readonly property real side: Math.max(32, width * 0.05)
    function opened() { view = "tiles"; tileId = ""; allCur = 0; field.text = ""; field.forceActiveFocus(); tilesView.contentX = 0; }

    Image {   // the wallpaper's glow behind (the scrim above darkens the desktop)
        anchors.fill: parent
        source: Qt.resolvedUrl("images/ember-glow.png")
        opacity: Theme.dark ? 0.16 : 0.08
    }
    MouseArea { anchors.fill: parent }

    // ---- the header: Start, you, search and power
    RowLayout {
        id: header
        anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: st.side; rightMargin: st.side; topMargin: 40 * st.k }
        spacing: 16
        Text { text: st.searching ? "Search" : st.view === "all" ? "Apps" : "Start"; color: Theme.text; font.family: Theme.font; font.pixelSize: 54 * st.k; font.weight: Font.Light }
        Item { Layout.fillWidth: true }
        ColumnLayout {
            spacing: 0
            Text { Layout.alignment: Qt.AlignRight; text: st.menu ? st.menu.userName : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 20 * st.k }
        }
        Rectangle {
            implicitWidth: 52 * st.k; implicitHeight: implicitWidth; radius: width / 2; color: Theme.accent
            Text { anchors.centerIn: parent; text: st.menu ? st.menu.userName.charAt(0).toUpperCase() : ""; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 22 * st.k; font.bold: true }
        }
        Rectangle {
            implicitWidth: 46 * st.k; implicitHeight: implicitWidth; radius: width / 2
            color: sa.containsMouse ? Theme.panel3 : Theme.panel2
            Icon { anchors.centerIn: parent; name: "search"; size: 20 * st.k }
            MouseArea { id: sa; anchors.fill: parent; hoverEnabled: true; onClicked: field.forceActiveFocus() }
        }
        Rectangle {
            id: powerBtn
            implicitWidth: 46 * st.k; implicitHeight: implicitWidth; radius: width / 2
            color: pa.containsMouse ? Theme.panel3 : Theme.panel2
            Icon { anchors.centerIn: parent; name: "power"; size: 20 * st.k }
            MouseArea { id: pa; anchors.fill: parent; hoverEnabled: true; onClicked: st.menu.powerMenu(powerBtn, false) }
        }
    }

    // ---- the search field (shown once you type; typing anywhere lands here)
    Rectangle {
        id: searchBox
        anchors { left: parent.left; top: header.bottom; leftMargin: st.side; topMargin: 18 * st.k }
        width: Math.min(parent.width - 2 * st.side, 720)
        height: st.searching ? 50 * st.k : 0
        opacity: st.searching ? 1 : 0       // kept (not hidden) so it holds the keyboard: typing anywhere lands here
        clip: true
        radius: Theme.r(6)
        color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.9)
        border.width: 2; border.color: Theme.accent
        Icon { x: 16; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 18 * st.k; color: Theme.muted }
        TextInput {
            id: field
            anchors { fill: parent; leftMargin: 46 * st.k; rightMargin: 12 }
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.text; selectionColor: Theme.accent
            font.family: Theme.font; font.pixelSize: 18 * st.k
            focus: true
            onTextChanged: results.query = text
            Keys.onPressed: e => st.key(e)
        }
    }
    MenuSearch {
        id: results
        visible: st.searching
        menu: st.menu
        k: st.k
        anchors { left: parent.left; right: parent.right; top: searchBox.bottom; bottom: hints.top; leftMargin: st.side; rightMargin: st.side; topMargin: 18 * st.k; bottomMargin: 12 }
    }

    // ---- the tiles, sideways
    Flickable {
        id: tilesView
        visible: !st.searching && st.view === "tiles"
        anchors { left: parent.left; right: parent.right; top: header.bottom; bottom: allBtn.top; topMargin: 40 * st.k; bottomMargin: 16 }
        contentWidth: groupsRow.implicitWidth + 2 * st.side
        clip: true
        flickableDirection: Flickable.HorizontalFlick
        boundsBehavior: Flickable.StopAtBounds
        Behavior on contentX { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Row {
            id: groupsRow
            x: st.side
            spacing: 58 * st.k
            Repeater {
                model: st.menu ? st.menu.tileGroups : []
                delegate: Column {
                    id: grp
                    required property string modelData
                    spacing: 12 * st.k
                    Text { text: grp.modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 20 * st.k }
                    Flow {
                        width: 6 * st.unit + 5 * st.gap
                        spacing: st.gap
                        Repeater {
                            model: st.menu ? st.menu.tiles.filter(t => t.group === grp.modelData) : []
                            delegate: MenuTile {
                                required property var modelData
                                menu: st.menu
                                tile: modelData
                                unit: st.unit
                                gap: st.gap
                                focused: st.tileId === modelData.id
                                onPicked: st.tileId = modelData.id
                            }
                        }
                    }
                }
            }
        }
    }
    // ---- All apps: A–Z in columns, sideways
    GridView {
        id: allGrid
        visible: !st.searching && st.view === "all"
        anchors { left: parent.left; right: parent.right; top: header.bottom; bottom: allBtn.top; leftMargin: st.side; rightMargin: st.side; topMargin: 40 * st.k; bottomMargin: 16 }
        flow: GridView.FlowTopToBottom
        clip: true
        cellWidth: 300 * st.k
        cellHeight: 52 * st.k
        boundsBehavior: Flickable.StopAtBounds
        model: st.menu && st.view === "all" ? st.menu.allApps : []
        currentIndex: st.allCur
        delegate: Rectangle {
            id: ai
            required property var modelData
            required property int index
            readonly property bool lit: st.allCur === index
            width: allGrid.cellWidth - 10; height: allGrid.cellHeight - 6
            radius: Theme.r(4)
            color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2) : aia.containsMouse ? Theme.panel2 : "transparent"
            border.width: lit ? 2 : 0; border.color: Theme.text
            RowLayout {
                anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                spacing: 12
                Rectangle {
                    implicitWidth: 34 * st.k; implicitHeight: implicitWidth; radius: Theme.r(4)
                    color: (ai.index === 0 || st.menu.letterOf(st.menu.allApps[ai.index - 1]) !== st.menu.letterOf(ai.modelData)) ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2) : "transparent"
                    AppIcon { anchors.centerIn: parent; size: 28 * st.k; icon: ai.modelData.icon || ""; appId: ai.modelData.id || "" }
                }
                Text { Layout.fillWidth: true; text: ai.modelData.name; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * st.k }
                Text { visible: ai.index === 0 || st.menu.letterOf(st.menu.allApps[ai.index - 1]) !== st.menu.letterOf(ai.modelData); text: st.menu.letterOf(ai.modelData); color: Theme.accent; font.family: Theme.font; font.pixelSize: 15 * st.k; font.bold: true }
            }
            MouseArea {
                id: aia; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton | Qt.RightButton
                onClicked: mouse => { st.allCur = ai.index; if (mouse.button === Qt.RightButton) st.options(); else st.menu.launch(ai.modelData); }
            }
        }
    }

    // ---- All apps / back, and how far along you are
    RowLayout {
        id: allBtn
        visible: !st.searching
        anchors { left: parent.left; bottom: hints.top; leftMargin: st.side; bottomMargin: 12 * st.k }
        spacing: 12
        Rectangle {
            implicitWidth: 44 * st.k; implicitHeight: implicitWidth; radius: width / 2
            color: "transparent"; border.width: 2; border.color: Theme.text
            Text { anchors.centerIn: parent; text: st.view === "all" ? "↑" : "↓"; color: Theme.text; font.pixelSize: 20 * st.k }
            MouseArea { anchors.fill: parent; onClicked: st.view = st.view === "all" ? "tiles" : "all" }
        }
        Text { text: st.view === "all" ? "Start" : "All apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 * st.k }
    }
    Rectangle {   // the sideways scroll position
        visible: !st.searching && st.view === "tiles" && tilesView.contentWidth > tilesView.width
        anchors { left: parent.left; right: parent.right; bottom: hints.top; leftMargin: st.side; rightMargin: st.side; bottomMargin: 4 }
        height: 4; radius: 2; color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.15)
        Rectangle {
            height: parent.height; radius: 2; color: Theme.text; opacity: 0.6
            width: parent.width * Math.min(1, tilesView.width / Math.max(1, tilesView.contentWidth))
            x: (parent.width - width) * (tilesView.contentX / Math.max(1, tilesView.contentWidth - tilesView.width))
        }
    }

    // ---- controller hints
    Rectangle {
        id: hints
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: 56 * st.k
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.6)
        Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
        MenuHints {
            anchors { left: parent.left; leftMargin: st.side; verticalCenter: parent.verticalCenter }
            k: st.k
            model: [{ b: "A", t: "Open" }, { b: "B", t: st.searching || st.view === "all" ? "Back" : "Back to the desktop" }, { b: "X", t: st.view === "tiles" ? "Resize or move" : "Options" }, { b: "Y", t: "Search" }]
        }
        MenuHints {
            anchors { right: parent.right; rightMargin: st.side; verticalCenter: parent.verticalCenter }
            k: st.k
            model: [{ b: "LB", t: "" }, { b: "RB", t: "Scroll" }, { b: "↓", t: st.view === "all" ? "Start" : "All apps" }]
        }
    }

    // ------------------------------------------------------------------ keys
    function tileItems() {
        const out = [];
        const walk = it => { for (const c of it.children) { if (c.tile !== undefined && c.unit !== undefined) out.push(c); else walk(c); } };
        walk(groupsRow);
        return out;
    }
    function curTile() { return tileItems().find(t => t.tile.id === tileId) || null; }
    function showTile(t) {
        if (!t) return;
        const p = t.mapToItem(groupsRow, 0, 0);
        const left = p.x + st.side - 40, right = p.x + st.side + t.width + 40;
        if (left < tilesView.contentX) tilesView.contentX = Math.max(0, left);
        else if (right > tilesView.contentX + tilesView.width) tilesView.contentX = Math.min(tilesView.contentWidth - tilesView.width, right - tilesView.width);
    }
    function options() {
        if (view === "all") { const e = menu.allApps[allCur], it = allGrid.itemAtIndex(allCur); if (e && it) menu.rowMenu({ kind: "app", item: e }, it, [{ label: "Add as a tile", icon: "grid", run: () => st.menu.setTileHidden("app:" + e.id, false) }]); return; }
        const t = curTile();
        if (t) menu.tileMenu(t.tile, t);
    }
    function key(e) {
        if (searching) {
            switch (e.key) {
            case Qt.Key_Down: results.move(1); break;
            case Qt.Key_Up: results.move(-1); break;
            case Qt.Key_Return: case Qt.Key_Enter: results.activate(); break;
            case Qt.Key_Menu: results.options(); break;
            case Qt.Key_Tab: results.nextFilter(1); break;
            case Qt.Key_Backtab: results.nextFilter(-1); break;
            case Qt.Key_Escape: field.text = ""; break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        if (view === "all") {
            const n = menu.allApps.length, rowsPer = Math.max(1, Math.floor(allGrid.height / allGrid.cellHeight));
            switch (e.key) {
            case Qt.Key_Down: allCur = Math.min(n - 1, allCur + 1); break;
            case Qt.Key_Up: if (allCur % rowsPer === 0) view = "tiles"; else allCur = Math.max(0, allCur - 1); break;
            case Qt.Key_Right: case Qt.Key_Tab: allCur = Math.min(n - 1, allCur + rowsPer); break;
            case Qt.Key_Left: case Qt.Key_Backtab: allCur = Math.max(0, allCur - rowsPer); break;
            case Qt.Key_Return: case Qt.Key_Enter: { const a = menu.allApps[allCur]; if (a) menu.launch(a); break; }
            case Qt.Key_Menu: options(); break;
            case Qt.Key_Escape: view = "tiles"; break;
            default: return;
            }
            allGrid.positionViewAtIndex(allCur, GridView.Contain);
            e.accepted = true;
            return;
        }
        const items = tileItems();
        switch (e.key) {
        case Qt.Key_Left: case Qt.Key_Right: case Qt.Key_Up: case Qt.Key_Down: {
            if (!curTile()) { if (items.length) { tileId = items[0].tile.id; showTile(items[0]); } break; }
            const dx = e.key === Qt.Key_Left ? -1 : e.key === Qt.Key_Right ? 1 : 0, dy = e.key === Qt.Key_Up ? -1 : e.key === Qt.Key_Down ? 1 : 0;
            const n = Nav.nearest(items, items.findIndex(t => t.tile.id === tileId), dx, dy);
            if (n >= 0) { tileId = items[n].tile.id; showTile(items[n]); }
            else if (dy > 0) { view = "all"; allCur = 0; }
            break;
        }
        case Qt.Key_Tab: tilesView.contentX = Math.min(Math.max(0, tilesView.contentWidth - tilesView.width), tilesView.contentX + tilesView.width * 0.6); break;
        case Qt.Key_Backtab: tilesView.contentX = Math.max(0, tilesView.contentX - tilesView.width * 0.6); break;
        case Qt.Key_Return: case Qt.Key_Enter: { const t = curTile(); if (t && t.tile.open) t.tile.open(); break; }
        case Qt.Key_Menu: options(); break;
        case Qt.Key_Escape: menu.close(); break;
        default: return;
        }
        e.accepted = true;
    }
}
