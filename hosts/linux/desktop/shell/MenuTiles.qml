// The "Tiles" app menu (Settings › Appearance › Bar and dock › App menu): from the corner with the Hearth flame.
// A left rail (you, Documents, Pictures, Files, Settings, Power), the app list (Most used, then A–Z with letter
// headers: pick a letter to jump), and live tiles in two columns (Play: games, Game Mode, updates; Today: what's
// playing, the weather, your calendar; Home server; Apps). Right-click / X on a tile: Small, Medium, Wide, Large or
// Take off the menu (saved per user); on an app: its jump list (recent files, its own actions).
// Keys / controller: arrows (D-pad) move, Left / Right between the list and the tiles, Enter (A) opens, Esc (B)
// closes, the Menu key (X) for options, typing (Y brings the keyboard) searches, Tab (LB / RB) switches side.
// Drawn over Launcher.qml (`menu`); loaded by file name.
import QtQuick
import QtQuick.Layouts
import Quickshell
import "menu-nav.js" as Nav

Item {
    id: st
    property var menu: null
    readonly property color scrim: Qt.rgba(0.02, 0.02, 0.03, 0.28)
    readonly property bool wantsLive: true
    property string zone: "list"            // list | tiles | letters
    property int cur: 0                     // list: a row; tiles: a tile
    property string tileId: ""              // the tile with the focus ring
    property int letterCur: 0
    readonly property bool searching: field.text.trim() !== ""
    readonly property real unit: Math.max(56, Math.min(72, (panel.height - 120) / 9.5))
    readonly property real gap: 6
    // The list: Most used, then every app A–Z under its letter.
    readonly property var rows: {
        if (!menu) return [];
        const out = [{ kind: "head", text: "Most used" }];
        for (const e of menu.frequent.slice(0, 4)) out.push({ kind: "app", item: e });
        let last = "";
        for (const e of menu.allApps) {
            const l = menu.letterOf(e);
            if (l !== last) { out.push({ kind: "letter", text: l }); last = l; }
            out.push({ kind: "app", item: e });
        }
        return out;
    }
    function opened() { zone = "list"; cur = 1; tileId = ""; field.text = ""; field.forceActiveFocus(); list.positionViewAtBeginning(); }

    MouseArea { anchors.fill: parent; onClicked: st.menu.close() }

    Item {
        id: panel
        // The rail, the list and two columns of tiles (no empty room on the right).
        width: Math.min(st.width - DeskState.leftInset - DeskState.rightInset - 20, 56 + 300 + 2 * (4 * st.unit + 3 * st.gap) + 26 + 34)
        height: Math.min(st.height - DeskState.topInset - DeskState.bottomInset - 20, 780)
        readonly property point pos: st.menu ? st.menu.panelPos(width, height, "corner") : Qt.point(0, 0)
        x: pos.x; y: pos.y
        MouseArea { anchors.fill: parent }
        Rectangle { anchors.fill: parent; radius: Theme.r(8); color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.line; visible: frame.status !== Loader.Ready }
        Loader { id: frame; anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.r(8)); } }

        RowLayout {
            anchors { fill: parent; bottomMargin: 44 }
            spacing: 0

            // ---- the rail
            ColumnLayout {
                Layout.fillHeight: true
                Layout.preferredWidth: 56
                Layout.topMargin: 12; Layout.bottomMargin: 8
                spacing: 6
                RailButton { icon: "menu"; tip: "Hearth menu"; onPicked: {} }
                Item { Layout.fillHeight: true }
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: 32; implicitHeight: 32; radius: 16; color: Theme.accent
                    Text { anchors.centerIn: parent; text: st.menu ? st.menu.userName.charAt(0).toUpperCase() : ""; color: Theme.accentInk; font.family: Theme.font; font.bold: true }
                }
                RailButton { icon: "file"; tip: "Documents"; onPicked: st.menu.openFolder(Quickshell.env("HOME") + "/Documents") }
                RailButton { icon: "cam"; tip: "Pictures"; onPicked: st.menu.openFolder(Quickshell.env("HOME") + "/Pictures") }
                RailButton { icon: "folder"; tip: "Files"; onPicked: st.menu.openFolder(Quickshell.env("HOME")) }
                RailButton { icon: "gear"; tip: "Settings"; onPicked: st.menu.power("settings") }
                RailButton { id: railPower; icon: "power"; tip: "Power"; onPicked: st.menu.powerMenu(railPower, true) }
            }

            // ---- search field, then the list (or the search results across the panel)
            ColumnLayout {
                Layout.fillHeight: true
                Layout.fillWidth: st.searching
                Layout.preferredWidth: st.searching ? panel.width - 56 - 20 : 300
                Layout.maximumWidth: st.searching ? 100000 : 300
                Layout.topMargin: 14
                spacing: 10
                Rectangle {
                    Layout.fillWidth: true
                    Layout.rightMargin: 12
                    implicitHeight: 40
                    radius: Theme.r(6)
                    color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.85)
                    border.width: 2
                    border.color: st.searching ? Theme.accent : Theme.line
                    Icon { x: 12; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 16; color: Theme.muted }
                    TextInput {
                        id: field
                        anchors { fill: parent; leftMargin: 38; rightMargin: 10 }
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.text; selectionColor: Theme.accent
                        font.family: Theme.font; font.pixelSize: 14
                        focus: true
                        onTextChanged: results.query = text
                        Text { anchors.verticalCenter: parent.verticalCenter; visible: !field.text; text: "Type to search"; color: Theme.muted; font: field.font }
                        Keys.onPressed: e => st.key(e)
                    }
                }
                MenuSearch {
                    id: results
                    visible: st.searching
                    menu: st.menu
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.rightMargin: 12
                }
                ListView {
                    id: list
                    visible: !st.searching
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: st.rows
                    currentIndex: st.cur
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: Rectangle {
                        id: lr
                        required property var modelData
                        required property int index
                        readonly property bool lit: st.zone === "list" && st.cur === index
                        width: ListView.view.width - 10
                        height: modelData.kind === "head" ? 30 : modelData.kind === "letter" ? 36 : 44
                        radius: Theme.r(4)
                        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : la.containsMouse ? Theme.panel2 : "transparent"
                        border.width: lit ? 2 : 0; border.color: Theme.accent
                        Text { visible: lr.modelData.kind === "head"; anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter } text: lr.modelData.text || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Rectangle {
                            visible: lr.modelData.kind === "letter"
                            x: 8; anchors.verticalCenter: parent.verticalCenter
                            implicitWidth: 28; implicitHeight: 28; radius: Theme.r(4)
                            color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
                            Text { anchors.centerIn: parent; text: lr.modelData.text || ""; color: Theme.accent; font.family: Theme.font; font.pixelSize: 13; font.bold: true }
                        }
                        RowLayout {
                            visible: lr.modelData.kind === "app"
                            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                            spacing: 12
                            AppIcon { size: 28; icon: lr.modelData.kind === "app" ? (lr.modelData.item.icon || "") : ""; appId: lr.modelData.kind === "app" ? (lr.modelData.item.id || "") : "" }
                            Text { Layout.fillWidth: true; text: lr.modelData.kind === "app" ? lr.modelData.item.name : ""; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                        }
                        MouseArea {
                            id: la; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => { st.zone = "list"; st.cur = lr.index; if (mouse.button === Qt.RightButton) st.options(); else st.activate(); }
                        }
                    }
                }
            }

            // ---- the live tiles
            Flickable {
                id: tilesView
                visible: !st.searching
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.leftMargin: 10; Layout.topMargin: 14; Layout.rightMargin: 14
                clip: true
                contentHeight: tileCols.implicitHeight + 10
                boundsBehavior: Flickable.StopAtBounds
                Row {
                    id: tileCols
                    spacing: 26
                    Repeater {
                        model: [["Play", "Apps"], ["Today", "Home server"]]
                        delegate: Column {
                            id: col
                            required property var modelData
                            spacing: 14
                            Repeater {
                                model: col.modelData.filter(g => st.menu && st.menu.tileGroups.indexOf(g) >= 0)
                                delegate: Column {
                                    id: grp
                                    required property string modelData
                                    spacing: 8
                                    Text { text: grp.modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                                    Flow {
                                        width: 4 * st.unit + 3 * st.gap
                                        spacing: st.gap
                                        Repeater {
                                            model: st.menu ? st.menu.tiles.filter(t => t.group === grp.modelData) : []
                                            delegate: MenuTile {
                                                required property var modelData
                                                menu: st.menu
                                                tile: modelData
                                                unit: st.unit
                                                gap: st.gap
                                                focused: st.zone === "tiles" && st.tileId === modelData.id
                                                onPicked: { st.zone = "tiles"; st.tileId = modelData.id; }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ---- the letters (pick one to jump there)
        Rectangle {
            id: letters
            visible: st.zone === "letters"
            anchors { left: parent.left; leftMargin: 56; top: parent.top; bottom: parent.bottom; bottomMargin: 44 }
            width: 300
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98)
            Grid {
                anchors.centerIn: parent
                columns: 4
                spacing: 8
                Repeater {
                    model: st.menu ? st.menu.letters : []
                    delegate: Rectangle {
                        required property string modelData
                        required property int index
                        width: 56; height: 56; radius: Theme.r(4)
                        color: st.letterCur === index ? Theme.accent : Theme.panel2
                        Text { anchors.centerIn: parent; text: modelData; color: st.letterCur === parent.index ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 20; font.bold: true }
                        MouseArea { anchors.fill: parent; onClicked: { st.letterCur = parent.index; st.jumpTo(parent.modelData); } }
                    }
                }
            }
        }

        // ---- hints along the bottom
        RowLayout {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 16; rightMargin: 16 }
            height: 44
            MenuHints { model: st.zone === "tiles" ? [{ b: "A", t: "Open" }, { b: "B", t: "Close" }, { b: "X", t: "Resize" }, { b: "Y", t: "Search" }]
                : [{ b: "A", t: st.rowIsLetter() ? "Jump to letter" : "Open" }, { b: "B", t: "Close" }, { b: "X", t: "Jump list" }, { b: "Y", t: "Search" }] }
            Item { Layout.fillWidth: true }
            MenuHints { model: [{ b: "LB", t: "" }, { b: "RB", t: st.zone === "tiles" ? "App list" : "Tiles" }] }
        }
    }

    // ------------------------------------------------------------------ keys
    function tileItems() {
        const out = [];
        const walk = it => { for (const c of it.children) { if (c.tile !== undefined && c.unit !== undefined) out.push(c); else walk(c); } };
        walk(tileCols);
        return out;
    }
    function curTile() { return tileItems().find(t => t.tile.id === tileId) || null; }
    function rowIsLetter() { const r = rows[cur]; return !!r && r.kind === "letter"; }
    function jumpTo(l) {
        const i = rows.findIndex(r => r.kind === "letter" && r.text === l);
        zone = "list";
        if (i >= 0) { cur = Math.min(rows.length - 1, i + 1); list.positionViewAtIndex(i, ListView.Beginning); }
    }
    function stepList(d) {
        let i = cur + d;
        while (i >= 0 && i < rows.length && rows[i].kind === "head") i += d;
        if (i >= 0 && i < rows.length) { cur = i; list.positionViewAtIndex(cur, ListView.Contain); }
    }
    function activate() {
        if (zone === "tiles") { const t = curTile(); if (t && t.tile.open) t.tile.open(); return; }
        const r = rows[cur];
        if (!r) return;
        if (r.kind === "letter") { zone = "letters"; letterCur = Math.max(0, menu.letters.indexOf(r.text)); }
        else if (r.kind === "app") menu.launch(r.item);
    }
    function options() {
        if (zone === "tiles") { const t = curTile(); if (t) menu.tileMenu(t.tile, t); return; }
        const r = rows[cur], it = list.itemAtIndex(cur);
        if (!r || r.kind !== "app" || !it) return;
        const isTile = menu.tiles.some(t => t.id === "app:" + r.item.id);
        menu.rowMenu({ kind: "app", item: r.item }, it, [
            isTile ? { label: "Take off the tiles", icon: "unpin", run: () => st.menu.setTileHidden("app:" + r.item.id, true) }
                   : { label: "Add as a tile", icon: "grid", run: () => st.menu.setTileHidden("app:" + r.item.id, false) },
            { label: DeskState.isPinned(r.item.id) ? "Unpin from the bar" : "Pin to the bar", icon: DeskState.isPinned(r.item.id) ? "unpin" : "pin", run: () => DeskState.togglePin(r.item.id) }
        ]);
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
        if (zone === "letters") {
            const n = menu.letters.length;
            switch (e.key) {
            case Qt.Key_Left: letterCur = Math.max(0, letterCur - 1); break;
            case Qt.Key_Right: letterCur = Math.min(n - 1, letterCur + 1); break;
            case Qt.Key_Up: letterCur = Math.max(0, letterCur - 4); break;
            case Qt.Key_Down: letterCur = Math.min(n - 1, letterCur + 4); break;
            case Qt.Key_Return: case Qt.Key_Enter: jumpTo(menu.letters[letterCur]); break;
            case Qt.Key_Escape: zone = "list"; break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        if (zone === "tiles") {
            const items = tileItems();
            switch (e.key) {
            case Qt.Key_Left: case Qt.Key_Right: case Qt.Key_Up: case Qt.Key_Down: {
                const dx = e.key === Qt.Key_Left ? -1 : e.key === Qt.Key_Right ? 1 : 0, dy = e.key === Qt.Key_Up ? -1 : e.key === Qt.Key_Down ? 1 : 0;
                const n = Nav.nearest(items, Math.max(0, items.findIndex(t => t.tile.id === tileId)), dx, dy);
                if (n >= 0) { tileId = items[n].tile.id; const t = items[n], p = t.mapToItem(tileCols, 0, 0); if (p.y < tilesView.contentY) tilesView.contentY = p.y; else if (p.y + t.height > tilesView.contentY + tilesView.height) tilesView.contentY = p.y + t.height - tilesView.height; }
                else if (dx < 0) zone = "list";
                break;
            }
            case Qt.Key_Tab: case Qt.Key_Backtab: zone = "list"; break;
            case Qt.Key_Return: case Qt.Key_Enter: activate(); break;
            case Qt.Key_Menu: options(); break;
            case Qt.Key_Escape: menu.close(); break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        switch (e.key) {
        case Qt.Key_Down: stepList(1); break;
        case Qt.Key_Up: stepList(-1); break;
        case Qt.Key_Right: case Qt.Key_Tab: case Qt.Key_Backtab: if (tileItems().length) { zone = "tiles"; if (!curTile()) tileId = tileItems()[0].tile.id; } break;
        case Qt.Key_PageDown: stepList(8); break;
        case Qt.Key_PageUp: stepList(-8); break;
        case Qt.Key_Return: case Qt.Key_Enter: activate(); break;
        case Qt.Key_Menu: options(); break;
        case Qt.Key_Escape: menu.close(); break;
        default: return;
        }
        e.accepted = true;
    }

    component RailButton: Rectangle {
        id: rb
        property string icon
        property string tip
        signal picked()
        Layout.alignment: Qt.AlignHCenter
        implicitWidth: 42; implicitHeight: 42
        radius: Theme.r(6)
        color: rba.containsMouse ? Theme.panel3 : "transparent"
        Icon { anchors.centerIn: parent; name: rb.icon; size: 20 }
        MouseArea { id: rba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: rb.picked() }
        PopupWindow {
            visible: rba.containsMouse && rb.tip !== ""
            anchor.item: rb
            anchor.rect.x: rb.width + 8
            anchor.rect.y: Math.round(rb.height / 2 - implicitHeight / 2)
            implicitWidth: rbt.implicitWidth + 20; implicitHeight: rbt.implicitHeight + 12
            color: "transparent"
            Rectangle { anchors.fill: parent; radius: Theme.r(6); color: Theme.panel2; border.color: Theme.line
                Text { id: rbt; anchors.centerIn: parent; text: rb.tip; color: Theme.text; font.pixelSize: 12; font.family: Theme.font } }
        }
    }
}
