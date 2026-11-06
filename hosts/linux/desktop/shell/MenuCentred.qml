// The "Centred" app menu (Settings › Appearance › Bar and dock › App menu): a centred panel over the dock or the
// bottom bar. Search at the top; Pinned (the apps pinned to the dock or bar first, then yours) in pages of 18, with
// All apps (A–Z); Recommended (what you opened lately and recent files); you, Settings and Power along the bottom.
// Keys / controller: arrows (D-pad) move, Enter (A) opens, Esc (B) goes back or closes, the Menu key (X) for options,
// typing (Y brings the keyboard) searches, Tab / Shift+Tab (RB / LB) turn the page or change the search filter.
// Drawn over Launcher.qml (`menu`), which has the lists, search and actions; loaded by file name.
import QtQuick
import QtQuick.Layouts
import Quickshell
import "menu-nav.js" as Nav

Item {
    id: st
    property var menu: null
    readonly property color scrim: Qt.rgba(0.02, 0.02, 0.03, 0.35)
    readonly property bool wantsLive: false
    property string view: "home"            // home | all
    property int page: 0
    property int cur: 0                     // the item with the focus ring (home: pinned then recommended; all: the list)
    readonly property bool searching: field.text.trim() !== ""
    readonly property int perPage: 18
    readonly property var pinned: menu ? menu.pinnedFirst : []
    readonly property int pages: Math.max(1, Math.ceil(pinned.length / perPage))
    readonly property var pageApps: pinned.slice(page * perPage, page * perPage + perPage)
    function opened() { view = "home"; page = 0; cur = 0; field.text = ""; field.forceActiveFocus(); }

    MouseArea { anchors.fill: parent; onClicked: st.menu.close() }

    Item {
        id: panel
        width: Math.min(st.width - 32, 860)
        height: Math.min(st.height - (DeskState.topInset + DeskState.bottomInset) - 24, 820)
        readonly property point pos: st.menu ? st.menu.panelPos(width, height, "centre") : Qt.point(0, 0)
        x: pos.x; y: pos.y
        MouseArea { anchors.fill: parent }
        Rectangle { anchors.fill: parent; radius: Theme.r(18); color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.line; visible: frame.status !== Loader.Ready }
        Loader { id: frame; anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.r(18)); } }

        ColumnLayout {
            anchors { fill: parent; margins: 26; bottomMargin: 0 }
            spacing: 14

            // ---- search
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 48
                radius: Theme.r(24)
                color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.85)
                border.width: 2
                border.color: field.activeFocus && (st.searching || st.cur < 0) ? Theme.accent : Theme.line
                Icon { x: 18; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 18; color: Theme.muted }
                TextInput {
                    id: field
                    anchors { fill: parent; leftMargin: 48; rightMargin: 52 }
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.text; selectionColor: Theme.accent
                    font.family: Theme.font; font.pixelSize: 16
                    focus: true
                    onTextChanged: { results.query = text; st.cur = 0; }
                    Text { anchors.verticalCenter: parent.verticalCenter; visible: !field.text; text: "Search apps, games, Settings and files"; color: Theme.muted; font: field.font }
                    Keys.onPressed: e => st.key(e)
                }
                MenuHints { anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter } model: [{ b: st.searching ? "B" : "Y", t: "" }] }
            }

            // ---- search results
            MenuSearch {
                id: results
                visible: st.searching
                menu: st.menu
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            // ---- Pinned
            RowLayout {
                visible: !st.searching && st.view === "home"
                Layout.fillWidth: true
                Text { text: "Pinned"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
                SmallButton { label: "All apps ›"; onPicked: { st.view = "all"; st.cur = 0; } }
            }
            GridLayout {
                id: grid
                visible: !st.searching && st.view === "home"
                Layout.fillWidth: true
                columns: 6
                rowSpacing: 4
                columnSpacing: 4
                Repeater {
                    id: pinRep
                    model: st.view === "home" ? st.pageApps : []
                    delegate: Cell {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        entry: modelData
                        lit: st.cur === index
                        onPicked: st.menu.launch(modelData)
                        onOptions: { st.cur = index; st.options(); }
                    }
                }
            }
            Row {   // page dots
                visible: !st.searching && st.view === "home" && st.pages > 1
                Layout.alignment: Qt.AlignHCenter
                spacing: 6
                Repeater {
                    model: st.pages
                    delegate: Rectangle {
                        required property int index
                        width: index === st.page ? 18 : 7; height: 7; radius: 3.5
                        color: index === st.page ? Theme.accent : Theme.muted
                        MouseArea { anchors.fill: parent; anchors.margins: -4; onClicked: { st.page = index; st.cur = 0; } }
                    }
                }
            }

            // ---- Recommended
            RowLayout {
                visible: !st.searching && st.view === "home" && recRep.count > 0
                Layout.fillWidth: true
                Layout.topMargin: 4
                Text { text: "Recommended"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
            }
            GridLayout {
                visible: !st.searching && st.view === "home"
                Layout.fillWidth: true
                columns: 2
                rowSpacing: 2
                columnSpacing: 12
                Repeater {
                    id: recRep
                    model: st.view === "home" && st.menu ? st.menu.recommended : []
                    delegate: RecRow {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        row: modelData
                        lit: st.cur === pinRep.count + index
                        onPicked: st.menu.openRow(modelData)
                        onOptions: { st.cur = pinRep.count + index; st.options(); }
                    }
                }
            }
            Item { visible: !st.searching && st.view === "home"; Layout.fillHeight: true }

            // ---- All apps (A–Z)
            RowLayout {
                visible: !st.searching && st.view === "all"
                Layout.fillWidth: true
                Text { text: "All apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
                SmallButton { label: "‹ Back"; onPicked: { st.view = "home"; st.cur = 0; } }
            }
            ListView {
                id: allList
                visible: !st.searching && st.view === "all"
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: st.view === "all" && st.menu ? st.menu.allApps : []
                currentIndex: st.cur
                boundsBehavior: Flickable.StopAtBounds
                delegate: Item {
                    id: ar
                    required property var modelData
                    required property int index
                    readonly property bool head: index === 0 || st.menu.letterOf(st.menu.allApps[index - 1]) !== st.menu.letterOf(modelData)
                    width: ListView.view.width
                    height: (head ? 30 : 0) + 46
                    Text { visible: ar.head; text: st.menu.letterOf(ar.modelData); x: 10; y: 6; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Rectangle {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: 46; radius: Theme.r(8)
                        color: st.cur === ar.index ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : aa.containsMouse ? Theme.panel2 : "transparent"
                        border.width: st.cur === ar.index ? 2 : 0; border.color: Theme.accent
                        RowLayout {
                            anchors { fill: parent; leftMargin: 12 }
                            spacing: 12
                            AppIcon { size: 28; icon: ar.modelData.icon || ""; appId: ar.modelData.id || "" }
                            Text { Layout.fillWidth: true; text: ar.modelData.name; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                        }
                        MouseArea {
                            id: aa; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => { st.cur = ar.index; if (mouse.button === Qt.RightButton) st.options(); else st.menu.launch(ar.modelData); }
                        }
                    }
                }
            }

            // ---- you, hints, Settings and Power
            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: -26; Layout.rightMargin: -26
                implicitHeight: 66
                color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.45)
                radius: Theme.r(18)
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
                RowLayout {
                    anchors { fill: parent; leftMargin: 26; rightMargin: 20 }
                    spacing: 12
                    Rectangle {
                        implicitWidth: 36; implicitHeight: 36; radius: 18; color: Theme.accent
                        Text { anchors.centerIn: parent; text: st.menu ? st.menu.userName.charAt(0).toUpperCase() : ""; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 16; font.bold: true }
                    }
                    Text { text: st.menu ? st.menu.userName : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                    Item { Layout.fillWidth: true }
                    MenuHints { visible: st.width > 1100; model: [{ b: "A", t: "Open" }, { b: "B", t: "Close" }, { b: "X", t: "Options" }, { b: "Y", t: "Search" }] }
                    RoundButton { icon: "gear"; tip: "Settings"; onPicked: st.menu.power("settings") }
                    RoundButton { id: powerBtn; icon: "power"; tip: "Power"; onPicked: st.menu.powerMenu(powerBtn, true) }
                }
            }
        }
    }

    // ------------------------------------------------------------------ keys (the controller types these)
    function navItems() {
        const out = [];
        for (let i = 0; i < pinRep.count; i++) out.push(pinRep.itemAt(i));
        for (let i = 0; i < recRep.count; i++) out.push(recRep.itemAt(i));
        return out;
    }
    function options() {
        if (view === "all") { const e = menu.allApps[cur]; const it = allList.itemAtIndex(cur); if (e && it) menu.rowMenu({ kind: "app", item: e }, it); return; }
        const items = navItems(), it = items[cur];
        if (!it) return;
        if (cur < pinRep.count) menu.rowMenu({ kind: "app", item: pageApps[cur] }, it);
        else menu.rowMenu(menu.recommended[cur - pinRep.count], it);
    }
    function activate() {
        if (view === "all") { const e = menu.allApps[cur]; if (e) menu.launch(e); return; }
        if (cur < pinRep.count) { const e = pageApps[cur]; if (e) menu.launch(e); }
        else { const r = menu.recommended[cur - pinRep.count]; if (r) menu.openRow(r); }
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
            const n = menu.allApps.length;
            switch (e.key) {
            case Qt.Key_Down: cur = Math.min(n - 1, cur + 1); allList.positionViewAtIndex(cur, ListView.Contain); break;
            case Qt.Key_Up: cur = Math.max(0, cur - 1); allList.positionViewAtIndex(cur, ListView.Contain); break;
            case Qt.Key_Return: case Qt.Key_Enter: activate(); break;
            case Qt.Key_Menu: options(); break;
            case Qt.Key_Escape: case Qt.Key_Left: view = "home"; cur = 0; break;
            default: return;
            }
            e.accepted = true;
            return;
        }
        const items = navItems();
        switch (e.key) {
        case Qt.Key_Left: case Qt.Key_Right: case Qt.Key_Up: case Qt.Key_Down: {
            const dx = e.key === Qt.Key_Left ? -1 : e.key === Qt.Key_Right ? 1 : 0, dy = e.key === Qt.Key_Up ? -1 : e.key === Qt.Key_Down ? 1 : 0;
            const n = Nav.nearest(items, cur, dx, dy);
            if (n >= 0) cur = n;
            else if (dx !== 0 && cur < pinRep.count && pages > 1) { page = (page + dx + pages) % pages; cur = 0; }
            break;
        }
        case Qt.Key_Tab: page = (page + 1) % pages; cur = 0; break;
        case Qt.Key_Backtab: page = (page - 1 + pages) % pages; cur = 0; break;
        case Qt.Key_Return: case Qt.Key_Enter: activate(); break;
        case Qt.Key_Menu: options(); break;
        case Qt.Key_Escape: menu.close(); break;
        default: return;
        }
        e.accepted = true;
    }

    // ------------------------------------------------------------------ building blocks
    component Cell: Rectangle {
        id: c
        property var entry: null
        property bool lit: false
        signal picked()
        signal options()
        implicitHeight: 96
        radius: Theme.r(10)
        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : ca.containsMouse ? Theme.panel2 : "transparent"
        border.width: lit ? 2 : 0
        border.color: Theme.accent
        Column {
            anchors.centerIn: parent
            spacing: 8
            AppIcon { anchors.horizontalCenter: parent.horizontalCenter; size: 40; icon: c.entry ? (c.entry.icon || "") : ""; appId: c.entry ? (c.entry.id || "") : "" }
            Text { width: c.width - 10; horizontalAlignment: Text.AlignHCenter; text: c.entry ? c.entry.name : ""; elide: Text.ElideRight; maximumLineCount: 2; wrapMode: Text.Wrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
        }
        MouseArea { id: ca; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton | Qt.RightButton; onClicked: mouse => mouse.button === Qt.RightButton ? c.options() : c.picked() }
    }
    component RecRow: Rectangle {
        id: rr
        property var row: null
        property bool lit: false
        signal picked()
        signal options()
        implicitHeight: 58
        radius: Theme.r(10)
        color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : ra.containsMouse ? Theme.panel2 : "transparent"
        border.width: lit ? 2 : 0
        border.color: Theme.accent
        RowLayout {
            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
            spacing: 12
            Item {
                implicitWidth: 40; implicitHeight: 40
                AppIcon { anchors.centerIn: parent; visible: !!rr.row && rr.row.kind === "app"; size: 36; icon: rr.row && rr.row.kind === "app" ? (rr.row.item.icon || "") : ""; appId: rr.row && rr.row.kind === "app" ? (rr.row.item.id || "") : "" }
                Rectangle {
                    anchors.fill: parent; visible: !!rr.row && rr.row.kind !== "app"; radius: Theme.r(8); color: Theme.panel3; clip: true
                    Image { anchors.fill: parent; visible: status === Image.Ready; fillMode: Image.PreserveAspectCrop; asynchronous: true; source: rr.row && rr.row.kind === "game" ? (rr.row.item.icon || rr.row.item.cover || "") : "" }
                    Icon { anchors.centerIn: parent; size: 18; visible: !rr.row || rr.row.kind !== "game"; name: !rr.row ? "file" : rr.row.kind === "page" ? (rr.row.item.icon || "gear") : rr.row.item.dir ? "folder" : "file" }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text { Layout.fillWidth: true; text: st.menu ? st.menu.rowTitle(rr.row) : ""; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                Text { Layout.fillWidth: true; text: st.menu ? st.menu.rowSub(rr.row) : ""; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
        MouseArea { id: ra; anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton | Qt.RightButton; onClicked: mouse => mouse.button === Qt.RightButton ? rr.options() : rr.picked() }
    }
    component SmallButton: Rectangle {
        id: sb
        property string label
        signal picked()
        implicitWidth: sbt.implicitWidth + 24; implicitHeight: 30
        radius: Theme.r(8)
        color: sba.containsMouse ? Theme.panel3 : Theme.panel2
        Text { id: sbt; anchors.centerIn: parent; text: sb.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
        MouseArea { id: sba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: sb.picked() }
    }
    component RoundButton: Rectangle {
        id: rb
        property string icon
        property string tip
        signal picked()
        implicitWidth: 42; implicitHeight: 42
        radius: Theme.r(12)
        color: rba.containsMouse ? Theme.panel3 : Theme.panel2
        Icon { anchors.centerIn: parent; name: rb.icon; size: 20 }
        MouseArea { id: rba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: rb.picked() }
    }
}
