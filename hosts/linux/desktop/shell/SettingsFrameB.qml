// Hearth Settings' controller look (1.14, the new Settings, mockups settings-B1 and settings-B2): shown by itself while
// a controller is what you use (SettingsNav.input), the keyboard-and-mouse look (SettingsFrame.qml) otherwise. The same
// pages in both: this frame only draws them bigger (SetStyle.big) and lays Settings out for a controller.
//   Top: "Settings" and the LB / RB tabs (Quick, System, Network, Devices, Gaming, Look, Accounts, Server).
//   Left: the tab's pages. Middle: the page, big rows (sliders with their value, ‹ value › choices, switches, › for
//   more). Right: help for the focused setting: what it does, how to change it with the controller, and when it was
//   last changed and by whom (the change history). Bottom: what the buttons do; X is the page's own action (Pin to
//   Quick on Quick, Keep these settings on Display while a change waits) or Default (that one setting back, asked first).
import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import Quickshell
import "settings-pages.js" as Reg
import "settings-changes.js" as Ch
import "settings-index.js" as Index

Item {
    id: frame
    // Runs enterPage on the next turn; owned by the frame so it can't fire after the frame is gone
    // (Qt.callLater on a closed frame gave "evaluate a function in an invalid context").
    Timer { id: enterLater; interval: 0; onTriggered: frame.enterPage() }
    required property var app
    readonly property var entry: Reg.page(app.page) || Reg.page("quick")
    readonly property string tab: Reg.tabOf(entry.id)
    readonly property var tabPages: Reg.tabPages(tab, app.live)
    property string pendingFocus: ""
    function isActive() { return frame.Window.active; }

    Component.onCompleted: {
        SetStyle.big = true;
        SettingsNav.frame = frame;
        SettingsNav.navTarget = list;
        load();
        list.forceActiveFocus();
    }
    Component.onDestruction: { SetStyle.big = false; if (SettingsNav.frame === frame) SettingsNav.frame = null; }
    Connections { target: frame.app; function onPageChanged() { frame.load(); } }

    function load() {
        loader.failed = false;
        loader.source = "";
        const o = Object.assign({ framed: true }, entry.props || {});
        if (entry.file === "SetCategory.qml") o.category = entry.cat;
        loader.setSource(Qt.resolvedUrl(entry.file), o);
    }
    function enterPage() {
        if (!loader.item) return;
        if (pendingFocus !== "" && loader.item.focusTitle && loader.item.focusTitle(pendingFocus)) { pendingFocus = ""; return; }
        if (loader.item.focusFirst) loader.item.focusFirst();
    }
    // The setting you were on stays picked when Settings changes look (SettingsApp.qml passes its title).
    function restoreFocus(t) { if (t) { pendingFocus = t; enterLater.restart(); } }
    function back() {
        const up = Reg.up(entry.id);
        if (up === "" || entry.id === "quick") { list.forceActiveFocus(); return; }
        pendingFocus = entry.label;
        frame.app.open(up);
        enterLater.restart();
    }
    function stepTab(d) {
        const i = Math.max(0, Reg.TABS.findIndex(t => t.id === frame.tab));
        const n = Reg.TABS[(i + d + Reg.TABS.length) % Reg.TABS.length];
        const first = n.id === "quick" ? "quick" : (Reg.tabPages(n.id, frame.app.live)[0] || {}).id || "quick";
        frame.app.open(first);
        list.forceActiveFocus();
    }

    // ---------------------------------------------------------------- the focused setting (for the help panel and X)
    readonly property var focusRow: {
        for (let it = frame.Window.activeFocusItem; it; it = it.parent) {
            if (it === loader) return null;
            if (it.title !== undefined && typeof it.title === "string" && it.title !== "" && it.activated !== undefined) return it;
        }
        return null;
    }
    readonly property string rowKey: focusRow ? (Object.keys(Ch.NAMES).find(k => Ch.NAMES[k] === focusRow.title) || "") : ""
    readonly property var lastChange: rowKey ? changes.last(rowKey) : null
    SetChanges { id: changes; count: 200; running: frame.visible }
    readonly property string howTo: !focusRow ? "" : focusRow.usesLeftRight ? "Pick with ‹ › (left and right on the stick or D-pad)."
        : focusRow.checked !== undefined ? "Press A to turn it on or off." : focusRow.chevron ? "Press A to open it; B comes back." : "Press A to choose it."

    // X: the page's own action, or Default for the focused setting.
    readonly property string xLabel: loader.item && loader.item.xLabel ? loader.item.xLabel : "Default"
    function xAction() {
        if (loader.item && loader.item.xAction && loader.item.xLabel) { loader.item.xAction(); return; }
        if (!rowKey || Ch.DEFAULTS[rowKey] === undefined) { SettingsNav.toast(focusRow ? focusRow.title + " has no default to go back to" : "Pick a setting first"); return; }
        confirm.key = rowKey;
        confirm.visible = true;
        confirm.forceActiveFocus();
    }

    Shortcut { sequences: ["Ctrl+PgDown"]; onActivated: frame.stepTab(1) }
    Shortcut { sequences: ["Ctrl+PgUp"]; onActivated: frame.stepTab(-1) }
    Shortcut { sequences: ["Ctrl+W"]; onActivated: frame.app.close() }
    Shortcut { sequences: ["Ctrl+D"]; onActivated: frame.xAction() }
    Shortcut { sequences: ["Ctrl+F"]; onActivated: frame.focusSearch() }
    function focusSearch() { finder.visible = true; findInput.text = ""; findInput.forceActiveFocus(); }
    // Search (Y): single settings and pages, as in the other look (settings-index.js and the registry).
    property string query: ""
    readonly property var found: {
        const ws = query.toLowerCase().split(/\s+/).filter(w => w);
        if (!ws.length) return [];
        const hit = t => { const x = t.toLowerCase(); return ws.every(w => x.indexOf(w) >= 0); };
        const ok = id => { const p = Reg.page(id); return !!p && (!p.live || frame.app.live); };
        const pages = Reg.PAGES.filter(p => ok(p.id) && hit(p.label + " " + (p.old || "") + " " + (p.words || "") + " " + Reg.pathOf(p.id)))
            .map(p => ({ title: p.label, icon: p.icon, page: p.id, path: Reg.pathOf(p.id), isPage: true }));
        const sets = Index.ENTRIES.filter(e => ok(e.p) && hit(e.t + " " + (e.d || "") + " " + (e.k || "") + " " + Reg.pathOf(e.p)))
            .map(e => ({ title: e.t, icon: e.i || (Reg.page(e.p) || {}).icon || "gear", page: e.p, path: Reg.pathOf(e.p) }));
        return pages.concat(sets).slice(0, 8);
    }
    function openFound(r) {
        if (!r) return;
        finder.visible = false;
        pendingFocus = r.isPage ? "" : r.title;
        frame.app.open(r.page);
        enterLater.restart();
    }

    Rectangle { anchors.fill: parent; color: Theme.bg }
    // A warm glow in the top right, as in the mockups.
    Rectangle {
        anchors { right: parent.right; top: parent.top }
        width: parent.width * 0.5; height: 260
        gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: SetStyle.tint(Theme.accent, 0.08) } }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // ---------------------------------------------------------------- header and tabs
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 50; Layout.rightMargin: 50; Layout.topMargin: 22; Layout.bottomMargin: 18
            spacing: 14
            Icon { name: "gear"; size: 30; color: Theme.text }
            Text { text: "Settings"; color: Theme.text; font.family: Theme.font; font.pixelSize: 34; font.weight: Font.Bold }
            Item { Layout.fillWidth: true }
            Rectangle {
                implicitWidth: tabRow.implicitWidth + 16; implicitHeight: 60
                radius: SetStyle.bigRadius; color: SetStyle.card; border.width: 1; border.color: SetStyle.tint(Theme.text, 0.06)
                Row {
                    id: tabRow
                    anchors.centerIn: parent
                    spacing: 4
                    KeyCap { text: "LB"; anchors.verticalCenter: parent.verticalCenter }
                    Repeater {
                        model: Reg.TABS
                        delegate: Rectangle {
                            id: tabItem
                            required property var modelData
                            readonly property bool lit: modelData.id === frame.tab
                            width: tl.implicitWidth + 44; height: 46; radius: Math.max(6, Theme.r(10))
                            color: lit ? Theme.accent : "transparent"
                            Text { id: tl; anchors.centerIn: parent; text: tabItem.modelData.label; color: tabItem.lit ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 19; font.weight: tabItem.lit ? Font.DemiBold : Font.Normal }
                            MouseArea {
                                anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                                onClicked: { const i = Reg.TABS.findIndex(t => t.id === frame.tab); frame.stepTab(Reg.TABS.indexOf(tabItem.modelData) - i); }
                            }
                        }
                    }
                    KeyCap { text: "RB"; anchors.verticalCenter: parent.verticalCenter }
                }
            }
        }

        // ---------------------------------------------------------------- pages, the page, help
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 50; Layout.rightMargin: 50
            spacing: 32
            ListView {
                id: list
                visible: frame.tab !== "quick"
                Layout.preferredWidth: 340
                Layout.fillHeight: true
                model: frame.tabPages
                spacing: 8
                clip: true
                currentIndex: Math.max(0, frame.tabPages.findIndex(p => p.id === frame.entry.id || p.id === Reg.up(frame.entry.id)))
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                function go(i) { if (i >= 0 && i < frame.tabPages.length) frame.app.open(frame.tabPages[i].id); }
                Keys.onPressed: e => {
                    switch (e.key) {
                    case Qt.Key_Down: go(currentIndex + 1); break;
                    case Qt.Key_Up: go(currentIndex - 1); break;
                    case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: frame.enterPage(); break;
                    case Qt.Key_Escape: frame.app.close(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
                delegate: Rectangle {
                    id: li
                    required property var modelData
                    required property int index
                    readonly property bool lit: index === ListView.view.currentIndex
                    width: ListView.view.width; height: 62
                    radius: SetStyle.bigRadius
                    color: lit ? SetStyle.card : "transparent"
                    Rectangle { visible: li.lit; width: 4; height: parent.height - 16; anchors.verticalCenter: parent.verticalCenter; radius: 2; color: Theme.accent }
                    Rectangle { visible: li.lit && list.activeFocus; anchors { fill: parent; margins: -3 } radius: li.radius + 3; color: "transparent"; border.width: 3; border.color: Theme.accent }
                    Row {
                        anchors { left: parent.left; leftMargin: 24; verticalCenter: parent.verticalCenter }
                        spacing: 12
                        Icon { name: li.modelData.icon || "gear"; size: 22; color: li.lit ? Theme.accent : Theme.muted; anchors.verticalCenter: parent.verticalCenter }
                        Text { text: li.modelData.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: li.lit ? Font.DemiBold : Font.Normal; anchors.verticalCenter: parent.verticalCenter }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { list.go(li.index); list.forceActiveFocus(); } }
                }
            }
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Loader {
                    id: loader
                    property bool failed: false
                    anchors.fill: parent
                    onStatusChanged: {
                        if (status === Loader.Error) { failed = true; console.warn("Hearth Settings: couldn't load", source); }
                        if (status === Loader.Ready && frame.pendingFocus !== "") enterLater.restart();
                        if (status === Loader.Ready && frame.tab === "quick") enterLater.restart();
                    }
                }
                SetBanner { visible: loader.failed; width: parent.width; kind: "bad"; title: "This page couldn't open"; sub: "The rest of Settings works." }
            }
            // The help panel (B2)
            Rectangle {
                visible: frame.tab !== "quick"
                Layout.preferredWidth: 410
                Layout.fillHeight: true
                Layout.bottomMargin: 24
                radius: SetStyle.bigRadius; color: SetStyle.card; border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
                ColumnLayout {
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
                    spacing: 14
                    Text { Layout.fillWidth: true; text: frame.focusRow ? frame.focusRow.title : frame.entry.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
                    Text {
                        Layout.fillWidth: true
                        text: frame.focusRow ? (frame.focusRow.sub || frame.focusRow.Accessible.description || "") : (frame.entry.desc || "")
                        visible: text !== ""
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 16; wrapMode: Text.WordWrap; lineHeight: 1.15
                    }
                    Text { Layout.fillWidth: true; visible: frame.howTo !== ""; text: frame.howTo; color: Theme.muted; font.family: Theme.font; font.pixelSize: 16; wrapMode: Text.WordWrap; lineHeight: 1.15 }
                    SetChip {
                        visible: !!frame.lastChange
                        text: frame.lastChange ? "Changed by " + (frame.lastChange.who || "you") + " · " + Ch.ago(frame.lastChange.at) : ""
                        kind: "off"
                    }
                }
            }
        }

        // ---------------------------------------------------------------- what the buttons do
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 56
            color: SetStyle.nav
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
            RowLayout {
                anchors { fill: parent; leftMargin: 30; rightMargin: 30 }
                spacing: 26
                Hint { keys: ["A"]; label: "Select" }
                Hint { keys: ["B"]; label: "Back" }
                Hint { keys: ["Y"]; label: "Search" }
                Hint { keys: ["X"]; label: frame.xLabel; clickable: true; onClicked: frame.xAction() }
                Item { Layout.fillWidth: true }
                Hint { keys: ["LB", "RB"]; square: true; label: "Previous / next category" }
                Hint { keys: ["☰"]; square: true; label: "Close" }
            }
        }
    }

    // ---------------------------------------------------------------- search (Y)
    Rectangle {
        id: finder
        visible: false
        anchors.fill: parent
        color: SetStyle.tint(Theme.bg, 0.75)
        MouseArea { anchors.fill: parent; onClicked: finder.visible = false }
        Rectangle {
            x: (parent.width - width) / 2; y: 110
            width: Math.min(900, parent.width - 100); height: fcol.implicitHeight + 32
            radius: SetStyle.bigRadius; color: SetStyle.card; border.width: 2; border.color: Theme.accent
            ColumnLayout {
                id: fcol
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
                spacing: 6
                RowLayout {
                    Layout.fillWidth: true; Layout.margins: 8
                    spacing: 12
                    Icon { name: "search"; size: 26; color: Theme.muted }
                    TextInput {
                        id: findInput
                        Layout.fillWidth: true
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 22
                        onTextChanged: { frame.query = text; foundList.currentIndex = 0; }
                        Keys.onDownPressed: if (frame.found.length) { foundList.currentIndex = 0; foundList.forceActiveFocus(); }
                        Keys.onReturnPressed: frame.openFound(frame.found[0])
                        Keys.onEnterPressed: frame.openFound(frame.found[0])
                        Keys.onEscapePressed: { finder.visible = false; list.forceActiveFocus(); }
                        Text { visible: !parent.text; text: "Search settings"; color: Theme.muted; font: parent.font }
                    }
                }
                ListView {
                    id: foundList
                    Layout.fillWidth: true
                    implicitHeight: contentHeight
                    interactive: false
                    model: frame.found
                    Keys.onPressed: e => {
                        if (e.key === Qt.Key_Down) currentIndex = Math.min(count - 1, currentIndex + 1);
                        else if (e.key === Qt.Key_Up) { if (currentIndex === 0) findInput.forceActiveFocus(); else currentIndex--; }
                        else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) frame.openFound(frame.found[currentIndex]);
                        else if (e.key === Qt.Key_Escape) { finder.visible = false; list.forceActiveFocus(); }
                        else return;
                        e.accepted = true;
                    }
                    delegate: Rectangle {
                        id: fr
                        required property var modelData
                        required property int index
                        width: ListView.view.width; height: 60; radius: SetStyle.radius
                        color: index === foundList.currentIndex && foundList.activeFocus ? SetStyle.mix(SetStyle.card, Theme.accent, 0.15) : "transparent"
                        Row {
                            anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                            spacing: 14
                            Icon { name: fr.modelData.icon; size: 24; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                            Column {
                                Text { text: fr.modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 18 }
                                Text { text: fr.modelData.path; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                            }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: frame.openFound(fr.modelData) }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- X Default: asked first
    Rectangle {
        id: confirm
        property string key: ""
        visible: false
        anchors.fill: parent
        color: SetStyle.tint(Theme.bg, 0.7)
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) yes();
            else if (e.key === Qt.Key_Escape) no();
            e.accepted = true;
        }
        function yes() {
            const k = confirm.key;
            Settings[k] = Ch.DEFAULTS[k];
            SettingsNav.toast(Ch.name(k) + " is back to " + Ch.value(k, Ch.DEFAULTS[k], Theme.palettes.accents || []));
            no();
        }
        function no() { confirm.visible = false; enterLater.restart(); }
        MouseArea { anchors.fill: parent; onClicked: confirm.no() }
        Rectangle {
            anchors.centerIn: parent
            width: 560; height: col.implicitHeight + 56
            radius: SetStyle.bigRadius; color: SetStyle.card; border.width: 2; border.color: Theme.accent
            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
                spacing: 14
                Text { Layout.fillWidth: true; text: "Put " + Ch.name(confirm.key) + " back to its default?"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
                Text { Layout.fillWidth: true; text: "It goes back to " + Ch.value(confirm.key, Ch.DEFAULTS[confirm.key], Theme.palettes.accents || []) + ". Only this one setting changes, and Undo on Home puts it back."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 16; wrapMode: Text.WordWrap }
                Row { spacing: 26; Hint { keys: ["A"]; label: "Put it back" } Hint { keys: ["B"]; label: "Keep it" } }
            }
        }
    }

    component KeyCap: Rectangle {
        property alias text: kt.text
        width: Math.max(34, kt.implicitWidth + 14); height: 26; radius: 7
        color: "transparent"; border.width: 1.5; border.color: Theme.muted
        Text { id: kt; anchors.centerIn: parent; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.Bold }
    }
    component Hint: Item {
        id: hint
        property var keys: []
        property string label
        property bool square: false
        property bool clickable: false
        signal clicked()
        implicitWidth: hintRow.implicitWidth
        implicitHeight: 28
        MouseArea { anchors.fill: parent; enabled: hint.clickable; cursorShape: Qt.PointingHandCursor; onClicked: hint.clicked() }
        Row {
        id: hintRow
        spacing: 8
        Repeater {
            model: hint.keys
            delegate: Rectangle {
                required property string modelData
                width: Math.max(28, k.implicitWidth + 12); height: 28
                radius: hint.square ? 7 : 14
                color: "transparent"; border.width: 1.5; border.color: Theme.muted
                Text { id: k; anchors.centerIn: parent; text: modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.Bold }
            }
        }
        Text { height: 28; verticalAlignment: Text.AlignVCenter; text: hint.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 17 }
        }
    }
}
