// The right-click menu's other looks (Settings › Personalization › Bar and dock › Right-click menu; Theme.menuStyle):
//   "windows"  Windows 11 style: a row of quick icons on top (the items marked quick: Cut, Copy, Rename, Share, Trash),
//              then 30 px rows with icons and shortcuts on the right; a soft tint with a small accent bar on the left
//              shows where you are; a submenu opens as a flyout beside the menu (to the left near the screen's edge).
//   "compact"  Compact list: 212 px, 27 px rows, small icons, a soft accent tint with an outline, red danger items.
//   "tiles"    Tiles and groups: the quick items as a 4-tile grid (icon + name), then titled groups (item.group;
//              ungrouped items go under "More") of 29 px rows with the accent filling the row.
// Today's menu (268 px, 40 px rows) is drawn by PopMenu.qml / Hearth Files themselves: they load this file by name
// only for another look, and keep today's menu if it can't load, so a problem here never loses the menu.
// Items are PopMenu's: {label, icon, run, sub, separator, checked, danger, enabled, shortcut / hint} plus
//   quick: true      in the quick row (Windows 11 style) or the tiles (Tiles and groups); a normal row in the others
//   shortcut: "Ctrl+C"
//   group: "Open"    Tiles and groups' headings
// Hearth Files' items ({id, label, key, sep, danger}) work too. The owner (PopMenu, Files) runs what's chosen.
// Keyboard and controller: Up / Down, Left / Right in the quick row or the tiles, Enter (A) chooses, Right opens a
// submenu, Left / Esc (B) goes back, then closes.
import QtQuick
import QtQuick.Layouts

Item {
    id: mv
    property string look: "windows"
    property bool padHint: false            // a controller is in use: the A Choose · B Back row
    property real spaceRight: 100000        // room to the right of the menu (the flyout flips left without it)
    property real maxHeight: 100000         // the tallest the menu may be (the list scrolls past that)
    property real maxBottom: 100000         // the flyout ends above this (in this item's coordinates)
    property real minTop: -100000           // ...and starts below this
    signal chosen(var item)                 // an item without a submenu was picked: the owner runs it
    signal closed()                         // B / Esc at the top, or the Menu key

    property var stack: []                  // [{title, items, from}]: from = the item that opened it, in the level before
    property int sel: -1
    readonly property bool win: look === "windows"
    readonly property bool tiles: look === "tiles"
    readonly property bool compact: look === "compact"
    readonly property real cardWidth: win ? 240 : compact ? 212 : 250
    readonly property real pad: win ? 5 : compact ? 4 : 6
    readonly property real rowH: win ? 30 : compact ? 27 : 29
    readonly property real iconSize: compact ? 14 : 16
    readonly property real fontSize: compact ? 13 : 13.5
    // Windows 11 style: a submenu is a flyout beside the first level; the others replace the list.
    readonly property bool flying: win && stack.length > 1
    readonly property var level: stack.length ? stack[stack.length - 1] : ({ title: "", items: [] })
    readonly property var items: level.items || []
    readonly property var cardLevel: flying ? stack[0] : level
    readonly property var cardItems: cardLevel.items || []
    readonly property int cardSel: flying ? stack[1].from : sel
    readonly property var quickIdx: (win || tiles) ? cardItems.map((it, i) => it && it.quick === true && !isSep(it) ? i : -1).filter(i => i >= 0) : []
    readonly property var groups: makeGroups(cardItems, quickIdx, tiles)
    readonly property var listOrder: orderOf(groups)
    readonly property var flyGroups: flying ? makeGroups(items, [], false) : []
    readonly property var flyOrder: orderOf(flyGroups)
    // What the keys move through: the flyout's rows, or the quick items and then the rows.
    readonly property var navQuick: flying ? [] : quickIdx
    readonly property var navList: flying ? flyOrder : listOrder
    readonly property bool inQuick: sel >= 0 && navQuick.indexOf(sel) >= 0
    property real flyY: 0

    implicitWidth: cardWidth
    implicitHeight: card.height

    function isSep(it) { return !!it && (it.separator === true || it.sep === true); }
    function usable(list, i) { const it = list[i]; return !!it && !isSep(it) && it.enabled !== false && it.off !== true; }
    function keyOf(it) { return it ? (it.shortcut || it.hint || it.key || "") : ""; }
    // [{title, boxed, rows: [{kind: "item" | "sep", i}]}]
    function makeGroups(list, quick, grouped) {
        const rest = [];
        for (let i = 0; i < list.length; i++) if (quick.indexOf(i) < 0) rest.push(i);
        if (grouped && rest.some(i => list[i] && !isSep(list[i]) && list[i].group)) {
            const names = [], by = {};
            for (const i of rest) {
                const it = list[i];
                if (!it || isSep(it)) continue;
                const g = it.group || "";
                if (!(g in by)) { by[g] = []; names.push(g); }
                by[g].push({ kind: "item", i: i });
            }
            const ordered = names.filter(g => g !== "").concat(names.indexOf("") >= 0 ? [""] : []);
            return ordered.map(g => ({ title: g || "More", boxed: true, rows: by[g] }));
        }
        const rows = [];
        for (const i of rest) {
            if (isSep(list[i])) { if (rows.length && rows[rows.length - 1].kind !== "sep") rows.push({ kind: "sep", i: i }); }
            else rows.push({ kind: "item", i: i });
        }
        while (rows.length && rows[rows.length - 1].kind === "sep") rows.pop();
        return rows.length ? [{ title: "", boxed: grouped, rows: rows }] : [];
    }
    function orderOf(gs) {
        const out = [];
        for (const g of gs) for (const r of g.rows) if (r.kind === "item") out.push(r.i);
        return out;
    }

    function open(title, list) {
        stack = [{ title: title || "", items: list || [], from: -1 }];
        sel = -1;
        flyTimer.stop();
    }
    function firstUsable(order, from, d) {
        for (let n = 0, k = from; n < order.length; n++, k += d) {
            const j = ((k % order.length) + order.length) % order.length;
            if (usable(items, order[j])) return order[j];
        }
        return -1;
    }
    // Up / Down.
    function nav(d) {
        const q = navQuick, l = navList;
        if (!q.length && !l.length) return;
        const firstList = firstUsable(l, 0, 1), lastList = firstUsable(l, l.length - 1, -1);
        const firstQuick = firstUsable(q, 0, 1);
        if (sel < 0) { sel = d > 0 ? (firstQuick >= 0 ? firstQuick : firstList) : (lastList >= 0 ? lastList : firstQuick); return; }
        if (inQuick) {
            const at = q.indexOf(sel), cols = tiles ? 4 : q.length;
            if (d > 0) { if (tiles && at + cols < q.length) sel = q[at + cols]; else if (firstList >= 0) sel = firstList; }
            else { if (tiles && at - cols >= 0) sel = q[at - cols]; else if (lastList >= 0) sel = lastList; }
            return;
        }
        const at = l.indexOf(sel);
        if (d > 0 && sel === lastList && q.length) { sel = q[0]; return; }
        if (d < 0 && sel === firstList && q.length) { sel = tiles ? q[Math.floor((q.length - 1) / 4) * 4] : q[0]; return; }
        const next = firstUsable(l, at + d, d);
        if (next >= 0) sel = next;
    }
    // Left / Right in the quick row or the tiles.
    function side(d) {
        const q = navQuick, at = q.indexOf(sel);
        for (let k = at + d; k >= 0 && k < q.length; k += d) if (usable(items, q[k])) { sel = q[k]; return; }
    }
    function pick(i) {
        if (!usable(items, i)) return;
        const it = items[i];
        flyTimer.stop();
        if (it.sub) {
            stack = stack.concat([{ title: it.label || "", items: it.sub, from: i }]);
            sel = -1;
            nav(1);
            return;
        }
        chosen(it);
    }
    function back() {
        flyTimer.stop();
        if (stack.length > 1) { const from = level.from; stack = stack.slice(0, -1); sel = from; }
        else closed();
    }
    // A row of the first card under the mouse: in Windows 11 style it closes another row's flyout and opens its own
    // after a moment.
    function hoverCard(i) {
        if (flying && stack[1].from !== i) { stack = [stack[0]]; }
        if (!flying) sel = i;
        flyTimer.stop();
        if (win && !flying && usable(items, i) && items[i].sub) { flyTimer.want = i; flyTimer.restart(); }
    }
    function clickCard(i) {
        if (flying) { if (stack[1].from === i) return; stack = [stack[0]]; }
        sel = i;
        pick(i);
    }
    Timer { id: flyTimer; interval: 280; property int want: -1; onTriggered: if (!mv.flying && mv.sel === want) mv.pick(want) }

    // Keys from the owner (PopMenu / Hearth Files): true when used.
    function key(event) {
        switch (event.key) {
        case Qt.Key_Up: case Qt.Key_Backtab: nav(-1); break;
        case Qt.Key_Down: case Qt.Key_Tab: nav(1); break;
        case Qt.Key_Left: if (inQuick) side(-1); else back(); break;
        case Qt.Key_Right:
            if (inQuick) side(1);
            else if (sel >= 0 && items[sel] && items[sel].sub) pick(sel);
            break;
        case Qt.Key_Backspace: case Qt.Key_Escape: back(); break;
        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
            if (sel < 0) nav(1); else pick(sel);
            break;
        case Qt.Key_Menu: closed(); break;
        default: return false;
        }
        return true;
    }

    // ------------------------------------------------ the menu
    Rectangle {
        id: card
        width: mv.cardWidth
        height: col.implicitHeight + mv.pad * 2
        radius: Theme.r(12)
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.RightButton }   // clicks inside don't close

        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: mv.pad }
            spacing: 0

            // The title (the app or file name), with a way back from a submenu that replaced the list.
            RowLayout {
                id: titleRow
                Layout.fillWidth: true
                Layout.leftMargin: 8; Layout.rightMargin: 8; Layout.topMargin: 2; Layout.bottomMargin: 3
                visible: mv.cardLevel.title !== ""
                spacing: 6
                Icon {
                    visible: mv.stack.length > 1 && !mv.win
                    name: "back"; size: 12; color: Theme.muted
                    MouseArea { anchors.fill: parent; anchors.margins: -8; onClicked: mv.back() }
                }
                Text {
                    Layout.fillWidth: true
                    text: mv.cardLevel.title
                    color: Theme.muted
                    font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }

            // Windows 11 style: the quick icons.
            Item {
                id: quickRow
                visible: mv.win && mv.quickIdx.length > 0
                Layout.fillWidth: true
                implicitHeight: visible ? 45 : 0
                RowLayout {
                    anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: 2; leftMargin: 2; rightMargin: 2 }
                    spacing: 0
                    Repeater {
                        model: quickRow.visible ? mv.quickIdx : []
                        delegate: Item {
                            id: qb
                            required property var modelData
                            readonly property int i: modelData
                            readonly property var it: mv.cardItems[i] || ({})
                            readonly property bool lit: mv.cardSel === i
                            readonly property bool on: mv.usable(mv.cardItems, i)
                            Layout.fillWidth: true
                            implicitHeight: 36
                            Rectangle {
                                anchors.centerIn: parent
                                width: 38; height: 34
                                radius: Theme.r(8)
                                color: qb.lit ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.09) : "transparent"
                                border.width: qb.lit && !qbMouse.containsMouse ? 1.5 : 0
                                border.color: Theme.accent
                                opacity: qb.on ? 1 : 0.4
                                Icon {
                                    anchors.centerIn: parent
                                    visible: !!qb.it.icon
                                    name: qb.it.icon || ""; size: 16
                                    color: qb.it.danger ? Theme.danger : Theme.text
                                }
                                Text {
                                    anchors.centerIn: parent
                                    visible: !qb.it.icon
                                    width: parent.width - 4
                                    horizontalAlignment: Text.AlignHCenter
                                    text: qb.it.label || ""
                                    elide: Text.ElideRight
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 9
                                }
                            }
                            Accessible.role: Accessible.MenuItem
                            Accessible.name: qb.it.label || ""
                            MouseArea {
                                id: qbMouse
                                anchors.fill: parent
                                enabled: qb.on
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                cursorShape: Qt.PointingHandCursor
                                onEntered: mv.hoverCard(qb.i)
                                onClicked: mv.clickCard(qb.i)
                            }
                        }
                    }
                }
                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom; bottomMargin: 4 } height: 1; color: Theme.line }
            }

            // Tiles and groups: the quick items as tiles.
            Rectangle {
                id: tileBox
                visible: mv.tiles && mv.quickIdx.length > 0
                Layout.fillWidth: true
                Layout.bottomMargin: 5
                implicitHeight: visible ? tileGrid.implicitHeight + 6 : 0
                radius: Theme.r(9)
                color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.03)
                Grid {
                    id: tileGrid
                    x: 3; y: 3
                    columns: 4
                    spacing: 3
                    readonly property real tileW: (tileBox.width - 6 - 9) / 4
                    Repeater {
                        model: tileBox.visible ? mv.quickIdx : []
                        delegate: Rectangle {
                            id: tile
                            required property var modelData
                            readonly property int i: modelData
                            readonly property var it: mv.cardItems[i] || ({})
                            readonly property bool lit: mv.cardSel === i
                            readonly property bool on: mv.usable(mv.cardItems, i)
                            width: tileGrid.tileW; height: 46
                            radius: Theme.r(7)
                            color: tile.lit ? Theme.accent : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.05)
                            opacity: tile.on ? 1 : 0.4
                            Column {
                                anchors.centerIn: parent
                                spacing: 4
                                Icon {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    name: tile.it.icon || "dot"; size: 16
                                    color: tile.lit ? Theme.accentInk : tile.it.danger ? Theme.danger : Theme.text
                                }
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    width: tile.width - 6
                                    horizontalAlignment: Text.AlignHCenter
                                    text: (tile.it.label || "").replace(/[….]+$/, "")
                                    elide: Text.ElideRight
                                    color: tile.lit ? Theme.accentInk : Theme.text
                                    font.family: Theme.font; font.pixelSize: 10.5; font.weight: Font.Medium
                                }
                            }
                            Accessible.role: Accessible.MenuItem
                            Accessible.name: tile.it.label || ""
                            MouseArea {
                                anchors.fill: parent
                                enabled: tile.on
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                cursorShape: Qt.PointingHandCursor
                                onEntered: mv.hoverCard(tile.i)
                                onClicked: mv.clickCard(tile.i)
                            }
                        }
                    }
                }
            }

            // The rows (scrolling when there are more than fit).
            ListBody {
                id: mainList
                Layout.fillWidth: true
                list: mv.cardItems
                gs: mv.groups
                selIdx: mv.cardSel
                isFly: false
                maxH: Math.max(120, mv.maxHeight - mv.pad * 2 - (titleRow.visible ? titleRow.implicitHeight + 5 : 0)
                    - quickRow.implicitHeight - (tileBox.visible ? tileBox.implicitHeight + 5 : 0) - hints.implicitHeight)
            }

            // A controller is in use: what A and B do.
            Item {
                id: hints
                visible: mv.padHint
                Layout.fillWidth: true
                implicitHeight: visible ? 28 : 0
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: 3 } height: 1; color: Theme.line }
                MenuHints {
                    anchors { horizontalCenter: parent.horizontalCenter; verticalCenter: parent.verticalCenter; verticalCenterOffset: 2 }
                    k: 0.68
                    model: [{ b: "A", t: "Choose" }, { b: "B", t: "Back" }]
                }
            }
        }
    }

    // ------------------------------------------------ Windows 11 style: the submenu beside the menu
    Rectangle {
        id: fly
        visible: mv.flying
        readonly property bool flip: mv.spaceRight < width + 8
        width: 200
        height: flyList.implicitHeight + 10
        x: flip ? -width + 4 : card.width - 4
        y: Math.max(mv.minTop, Math.min(mv.flyY - 5, mv.maxBottom - height))
        radius: Theme.r(10)
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.RightButton }
        ListBody {
            id: flyList
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 5 }
            list: mv.flying ? mv.items : []
            gs: mv.flyGroups
            selIdx: mv.sel
            isFly: true
            maxH: Math.max(120, mv.maxBottom - Math.max(mv.minTop, -100000) - 10)
        }
    }

    // ------------------------------------------------ the rows, in groups
    component ListBody: Flickable {
        id: lb
        property var list: []
        property var gs: []
        property int selIdx: -1
        property bool isFly: false
        property real maxH: 100000
        implicitHeight: Math.min(groupCol.implicitHeight, maxH)
        height: implicitHeight
        contentHeight: groupCol.implicitHeight
        contentWidth: width
        clip: contentHeight > height
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds
        function ensure(y, h) {
            if (y < contentY) contentY = y;
            else if (y + h > contentY + height) contentY = y + h - height;
        }
        Column {
            id: groupCol
            width: lb.width
            spacing: mv.tiles ? 5 : 0
            Repeater {
                model: lb.gs
                delegate: Rectangle {
                    id: grp
                    required property var modelData
                    width: groupCol.width
                    height: grpCol.implicitHeight + (grp.modelData.boxed ? 6 : 0)
                    radius: Theme.r(9)
                    color: grp.modelData.boxed ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.03) : "transparent"
                    Column {
                        id: grpCol
                        x: grp.modelData.boxed ? 3 : 0
                        y: grp.modelData.boxed ? 3 : 0
                        width: grp.width - 2 * x
                        Text {
                            visible: grp.modelData.title !== ""
                            width: parent.width
                            height: visible ? 22 : 0
                            leftPadding: 9; topPadding: 5
                            text: grp.modelData.title.toUpperCase()
                            color: Theme.muted
                            font.family: Theme.font; font.pixelSize: 10.5; font.weight: Font.DemiBold; font.letterSpacing: 1.2
                            elide: Text.ElideRight
                        }
                        Repeater {
                            model: grp.modelData.rows
                            delegate: Item {
                                id: row
                                required property var modelData
                                readonly property int i: modelData.i
                                readonly property bool sep: modelData.kind === "sep"
                                readonly property var it: lb.list[i] || ({})
                                readonly property bool on: !sep && mv.usable(lb.list, i)
                                readonly property bool lit: !sep && lb.selIdx === i
                                readonly property bool opener: !lb.isFly && mv.flying && mv.stack[1].from === i
                                readonly property bool danger: !!it.danger
                                // Text and icon colours: the accent fills the row in Tiles and groups.
                                readonly property bool filled: lit && mv.tiles
                                width: grpCol.width
                                height: sep ? (mv.compact ? 7 : 11) : mv.rowH
                                onLitChanged: if (lit) lb.ensure(mapToItem(lb.contentItem, 0, 0).y, height)
                                onOpenerChanged: if (opener) mv.flyY = mapToItem(mv, 0, 0).y
                                Rectangle {
                                    visible: row.sep
                                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 8; rightMargin: 8 }
                                    height: 1
                                    color: Theme.line
                                }
                                Rectangle {
                                    visible: !row.sep
                                    anchors.fill: parent
                                    radius: Theme.r(6)
                                    color: !(row.lit || row.opener) ? "transparent"
                                        : mv.tiles ? Theme.accent
                                        : mv.compact ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, Theme.dark ? 0.18 : 0.16)
                                        : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, Theme.dark ? 0.075 : 0.07)
                                    border.width: mv.compact && row.lit ? 1 : 0
                                    border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.4)
                                    // Windows 11 style: the small accent bar on the left.
                                    Rectangle {
                                        visible: mv.win && (row.lit || row.opener)
                                        x: 3; anchors.verticalCenter: parent.verticalCenter
                                        width: 3; height: 16; radius: 1.5
                                        color: Theme.accent
                                    }
                                }
                                RowLayout {
                                    visible: !row.sep
                                    anchors { fill: parent; leftMargin: mv.tiles ? 9 : 10; rightMargin: mv.tiles ? 9 : 10 }
                                    spacing: mv.compact ? 10 : 12
                                    opacity: row.on ? 1 : 0.45
                                    Item {
                                        implicitWidth: mv.iconSize; implicitHeight: mv.iconSize
                                        Icon {
                                            anchors.fill: parent
                                            visible: row.it.checked === undefined && !!row.it.icon
                                            name: row.it.icon || ""
                                            size: mv.iconSize
                                            color: row.filled ? Theme.accentInk
                                                : row.danger ? Theme.danger
                                                : mv.tiles ? Theme.accent
                                                : mv.compact ? (row.lit ? Theme.accent : Theme.muted)
                                                : Theme.text
                                            opacity: mv.win && !row.lit ? 0.82 : 1
                                        }
                                        Icon {
                                            anchors.fill: parent
                                            visible: row.it.checked === true
                                            name: "check"; size: mv.iconSize
                                            color: row.filled ? Theme.accentInk : Theme.accent
                                        }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: row.it.label || ""
                                        color: row.filled ? Theme.accentInk : row.danger ? Theme.danger : Theme.text
                                        font.family: Theme.font; font.pixelSize: mv.fontSize; font.weight: Font.Medium
                                        elide: Text.ElideRight
                                    }
                                    Text {                       // the keyboard shortcut (Ctrl+C...)
                                        visible: text !== "" && !row.it.sub
                                        text: mv.keyOf(row.it)
                                        color: row.filled ? Theme.accentInk : Theme.muted
                                        opacity: mv.compact && !row.lit ? 0.8 : 1
                                        font.family: Theme.font; font.pixelSize: mv.compact ? 11 : 12; font.weight: Font.Medium
                                    }
                                    Icon {
                                        visible: !!row.it.sub
                                        name: "forward"; size: 12
                                        color: row.filled ? Theme.accentInk : Theme.muted
                                    }
                                }
                                Accessible.role: Accessible.MenuItem
                                Accessible.name: row.it.label || ""
                                MouseArea {
                                    anchors.fill: parent
                                    enabled: row.on
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                                    cursorShape: Qt.PointingHandCursor
                                    onEntered: if (lb.isFly) mv.sel = row.i; else mv.hoverCard(row.i)
                                    onClicked: if (lb.isFly) mv.pick(row.i); else mv.clickCard(row.i)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
