// Hearth Settings' frame (the redesign, docs/MANTEL.md "Settings"): on the left your account, the search and the 12
// categories plus Home; on the right the page under a breadcrumb title ("Home server › Services"); at the bottom what
// the controller's buttons do. SettingsApp.qml loads it; if it can't, Settings falls back to the plain list of every
// page (SettingsSimple.qml), so Settings always opens.
// Controller / keyboard: Up/Down pick a category (its page shows), Right / A go into the page, B goes back one level
// (a subpage to its category, the page to the list, the list closes Settings), Y searches (Ctrl+F), LB / RB go to the
// previous / next category (Ctrl+PgUp / Ctrl+PgDn), ☰ closes (Ctrl+W). Search finds single settings (with their path),
// pages, and the pages' old names.
import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import Quickshell
import Quickshell.Io
import "settings-pages.js" as Reg
import "settings-index.js" as Index

Item {
    id: frame
    required property var app
    readonly property bool compact: width < 1300 || height < 760
    onCompactChanged: SetStyle.compact = compact
    readonly property var entry: Reg.page(app.page) || Reg.page("home")
    readonly property var cat: Reg.category(entry.cat) || Reg.CATEGORIES[0]
    readonly property var crumbs: Reg.crumbs(entry.id)
    property string pendingFocus: ""      // a card to put the focus on once the page has loaded (search, Back)
    function isActive() { return frame.Window.active; }

    Component.onCompleted: {
        SetStyle.compact = compact;
        SettingsNav.frame = frame;
        SettingsNav.navTarget = nav;
        nav.forceActiveFocus();
        load();
    }
    Component.onDestruction: { if (SettingsNav.frame === frame) SettingsNav.frame = null; }
    Connections { target: frame.app; function onPageChanged() { frame.load(); } }

    // ---------------------------------------------------------------- the page
    function propsFor(e) {
        const o = Object.assign({ framed: true }, e.props || {});
        if (e.file === "SetCategory.qml") o.category = e.cat;
        else if (e.landing && !e.ownList) {
            const subs = Reg.pagesIn(e.cat, frame.app.live).map(p => p.id).concat((Reg.LINKS[e.cat] || []).map(l => l.page));
            if (subs.length) o.footerLinks = subs;
            if (Reg.MORE[e.cat]) o.footerTitle = Reg.MORE[e.cat];
        }
        return o;
    }
    function load() {
        const e = frame.entry;
        loader.failed = false;
        loader.source = "";
        loader.setSource(Qt.resolvedUrl(e.file), propsFor(e));
    }
    function enterPage() {
        if (!loader.item) return;
        if (pendingFocus !== "" && loader.item.focusTitle && loader.item.focusTitle(pendingFocus)) { pendingFocus = ""; return; }
        if (loader.item.focusFirst) loader.item.focusFirst();
    }
    // B / Esc in a page: a subpage goes up to its parent (the card that opened it keeps the focus); a page to the list.
    function back() {
        const up = Reg.up(entry.id);
        if (up === "") { nav.forceActiveFocus(); return; }
        pendingFocus = entry.label;
        frame.app.open(up);
        Qt.callLater(frame.enterPage);
    }
    function stepCategory(d) {
        const cats = Reg.CATEGORIES;
        const i = Math.max(0, cats.findIndex(c => c.id === frame.cat.id));
        const n = cats[(i + d + cats.length) % cats.length];
        const hadNav = nav.activeFocus;
        frame.app.open(n.landing);
        if (!hadNav) Qt.callLater(frame.enterPage);
    }

    Shortcut { sequences: ["Ctrl+PgDown"]; onActivated: frame.stepCategory(1) }
    Shortcut { sequences: ["Ctrl+PgUp"]; onActivated: frame.stepCategory(-1) }
    Shortcut { sequences: ["Ctrl+F"]; onActivated: search.forceActiveFocus() }
    Shortcut { sequences: ["Ctrl+W"]; onActivated: frame.app.close() }

    // ---------------------------------------------------------------- who you are (the card on top)
    property string userName: ""
    property string userSub: ""
    Process {
        running: true
        command: ["sh", "-c", "u=$(id -un); n=$(getent passwd \"$u\" | cut -d: -f5 | cut -d, -f1); echo \"name=${n:-$u}\"; "
            + "case \" $(id -Gn) \" in *\" wheel \"*|*\" sudo \"*) echo admin=1;; esac; echo \"host=$(cat /etc/hostname 2>/dev/null || hostname)\""]
        stdout: StdioCollector {
            onStreamFinished: {
                const n = /^name=(.*)$/m.exec(text), h = /^host=(.*)$/m.exec(text);
                frame.userName = n ? n[1].trim() : "";
                frame.userSub = [/^admin=1$/m.test(text) ? "Administrator" : "Standard account", h ? h[1].trim() : ""].filter(x => x).join(" · ");
            }
        }
    }

    // ---------------------------------------------------------------- search
    property string query: ""
    property bool showAll: false
    function words(q) { return q.toLowerCase().split(/\s+/).filter(w => w); }
    function hit(text, ws) { const t = text.toLowerCase(); return ws.every(w => t.indexOf(w) >= 0); }
    readonly property var results: {
        const ws = words(query);
        if (!ws.length) return { settings: [], pages: [] };
        const live = frame.app.live;
        const ok = id => { const p = Reg.page(id); return !!p && (!p.live || live); };
        const settings = Index.ENTRIES.filter(e => ok(e.p) && hit(e.t + " " + (e.d || "") + " " + (e.k || "") + " " + Reg.pathOf(e.p), ws))
            .map(e => {
                const t = e.t.toLowerCase(), first = ws[0];
                return { title: e.t, icon: e.i || (Reg.page(e.p) || {}).icon || "gear", page: e.p, path: Reg.pathOf(e.p),
                         rank: t.startsWith(first) ? 0 : t.indexOf(first) >= 0 ? 1 : 2 };
            })
            .sort((a, b) => a.rank - b.rank || a.title.length - b.title.length);
        const pages = Reg.PAGES.filter(p => ok(p.id) && hit(p.label + " " + (p.old || "") + " " + (p.words || "") + " " + Reg.pathOf(p.id), ws))
            .map(p => ({ title: p.label, icon: p.icon, page: p.id, path: Reg.pathOf(p.id), old: p.old && p.old !== p.label && hit(p.old, ws) ? p.old : "", isPage: true }));
        return { settings: settings, pages: pages };
    }
    readonly property var shown: {
        const s = results.settings, p = results.pages;
        // Pages first when the words are a page's name (or its old name); then single settings.
        const list = p.filter(x => hit(x.title + " " + x.old, words(query))).concat(s).concat(p.filter(x => !hit(x.title + " " + x.old, words(query))));
        return showAll ? list : list.slice(0, 8);
    }
    function openResult(r) {
        if (!r) return;
        frame.query = ""; search.text = ""; frame.showAll = false;
        pendingFocus = r.isPage ? "" : r.title;
        frame.app.open(r.page);
        Qt.callLater(frame.enterPage);
    }
    function highlight(t) {
        const ws = words(query);
        let out = t.replace(/&/g, "&amp;").replace(/</g, "&lt;");
        for (const w of ws) {
            const i = out.toLowerCase().indexOf(w);
            if (i >= 0) out = out.slice(0, i) + "<font color=\"" + Theme.accent + "\"><b>" + out.slice(i, i + w.length) + "</b></font>" + out.slice(i + w.length);
        }
        return out;
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // ------------------------------------------------------------ the list
            Rectangle {
                id: side
                Layout.fillHeight: true
                Layout.preferredWidth: frame.compact ? 236 : 300
                color: SetStyle.nav
                Rectangle { anchors { right: parent.right; top: parent.top; bottom: parent.bottom } width: 1; color: SetStyle.tint(Theme.text, 0.05) }
                ColumnLayout {
                    anchors { fill: parent; leftMargin: frame.compact ? 10 : 14; rightMargin: frame.compact ? 10 : 14; topMargin: frame.compact ? 12 : 18; bottomMargin: 10 }
                    spacing: 4
                    RowLayout {
                        Layout.leftMargin: 8; Layout.bottomMargin: 10
                        spacing: 10
                        Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 28; Layout.preferredHeight: 28; fillMode: Image.PreserveAspectFit; smooth: true }
                        Text { text: "Settings"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                    }
                    // Your account (opens Accounts)
                    Rectangle {
                        visible: !frame.compact
                        Layout.fillWidth: true
                        implicitHeight: 66
                        radius: SetStyle.radius
                        color: acctMouse.containsMouse ? SetStyle.tint(Theme.text, 0.04) : "transparent"
                        RowLayout {
                            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                            spacing: 12
                            Rectangle {
                                Layout.preferredWidth: 46; Layout.preferredHeight: 46; radius: 23
                                color: Theme.accent
                                Text { anchors.centerIn: parent; text: (frame.userName || "?").charAt(0).toUpperCase(); color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.Bold }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Text { Layout.fillWidth: true; text: frame.userName; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                Text { Layout.fillWidth: true; text: frame.userSub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                            }
                        }
                        MouseArea { id: acctMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: frame.app.open("users") }
                    }
                    // Search (Y on the controller, Ctrl+F, or start typing on the list)
                    Rectangle {
                        id: searchBox
                        Layout.fillWidth: true
                        Layout.leftMargin: 2; Layout.rightMargin: 2; Layout.bottomMargin: 10; Layout.topMargin: frame.compact ? 0 : 2
                        implicitHeight: 40
                        radius: 20
                        color: search.activeFocus ? SetStyle.card2 : SetStyle.card
                        border.width: search.activeFocus ? 2 : 1
                        border.color: search.activeFocus ? Theme.accent : Theme.line
                        Icon { x: 14; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 18; color: Theme.muted }
                        TextInput {
                            id: search
                            anchors { fill: parent; leftMargin: 42; rightMargin: 40 }
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.text; selectionColor: Theme.accent; selectedTextColor: Theme.accentInk
                            font.family: Theme.font; font.pixelSize: 14
                            clip: true
                            onTextChanged: { frame.query = text; frame.showAll = false; }
                            Keys.onDownPressed: if (frame.shown.length) { resultList.currentIndex = 0; resultList.forceActiveFocus(); } else nav.forceActiveFocus()
                            Keys.onReturnPressed: frame.openResult(frame.shown[0])
                            Keys.onEnterPressed: frame.openResult(frame.shown[0])
                            Keys.onEscapePressed: { if (text) text = ""; else nav.forceActiveFocus(); }
                            Text { anchors.verticalCenter: parent.verticalCenter; visible: !search.text; text: "Search settings"; color: Theme.muted; font: search.font }
                        }
                        Rectangle {
                            anchors { right: parent.right; rightMargin: 9; verticalCenter: parent.verticalCenter }
                            width: 22; height: 22; radius: 11
                            color: "transparent"; border.width: 1.5; border.color: Theme.muted
                            Text { anchors.centerIn: parent; text: "Y"; color: Theme.text; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.IBeamCursor; onClicked: search.forceActiveFocus() }
                    }
                    ListView {
                        id: nav
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        model: Reg.CATEGORIES
                        spacing: 4
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        keyNavigationEnabled: false
                        currentIndex: Math.max(0, Reg.CATEGORIES.findIndex(c => c.id === frame.cat.id))
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        function go(i) { if (i >= 0 && i < Reg.CATEGORIES.length) frame.app.open(Reg.CATEGORIES[i].landing); }
                        Keys.onPressed: e => {
                            switch (e.key) {
                            case Qt.Key_Down: case Qt.Key_Tab: go(Math.min(Reg.CATEGORIES.length - 1, currentIndex + 1)); break;
                            case Qt.Key_Up: case Qt.Key_Backtab: if (currentIndex === 0) search.forceActiveFocus(); else go(currentIndex - 1); break;
                            case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: frame.enterPage(); break;
                            case Qt.Key_Escape: frame.app.close(); break;
                            case Qt.Key_Slash: search.forceActiveFocus(); break;
                            default:
                                // Typing a letter on the list starts a search.
                                if (e.text && e.text.length === 1 && !(e.modifiers & Qt.ControlModifier) && (/[0-9]/.test(e.text) || e.text.toLowerCase() !== e.text.toUpperCase())) { search.text += e.text; search.forceActiveFocus(); break; }
                                return;
                            }
                            e.accepted = true;
                        }
                        delegate: Item {
                            id: navItem
                            required property var modelData
                            required property int index
                            readonly property bool lit: frame.cat.id === modelData.id
                            width: ListView.view.width
                            height: frame.compact ? 38 : 42
                            Rectangle {
                                anchors.fill: parent
                                radius: SetStyle.radius
                                color: navItem.lit ? SetStyle.card2 : navMouse.containsMouse ? SetStyle.tint(Theme.text, 0.04) : "transparent"
                                Rectangle { visible: navItem.lit; x: 0; y: 11; width: 3; height: parent.height - 22; radius: 2; color: Theme.accent }
                                // The controller's ring on the list.
                                Rectangle {
                                    visible: navItem.lit && nav.activeFocus
                                    anchors { fill: parent; margins: -3 }
                                    radius: parent.radius + 3
                                    color: "transparent"; border.width: 3; border.color: Theme.accent
                                }
                                RowLayout {
                                    anchors { fill: parent; leftMargin: frame.compact ? 12 : 14; rightMargin: 14 }
                                    spacing: frame.compact ? 12 : 14
                                    Icon { name: navItem.modelData.icon; size: 20; color: navItem.lit ? Theme.accent : Theme.muted }
                                    Text {
                                        Layout.fillWidth: true
                                        text: navItem.modelData.label
                                        color: Theme.text
                                        font.family: Theme.font; font.pixelSize: frame.compact ? 14 : 15; font.weight: navItem.lit ? Font.DemiBold : Font.Normal
                                        elide: Text.ElideRight
                                    }
                                    Rectangle {
                                        visible: navItem.modelData.id === "hs" && homeServer.needsLook && !frame.compact
                                        width: 8; height: 8; radius: 4; color: SetStyle.warn
                                    }
                                }
                                MouseArea { id: navMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { frame.app.open(navItem.modelData.landing); nav.forceActiveFocus(); } }
                            }
                        }
                    }
                }
            }

            // ------------------------------------------------------------ the page, under its breadcrumb title
            Item {
                id: main
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                RowLayout {
                    id: title
                    x: SetStyle.pageMargin
                    y: frame.compact ? 16 : 26
                    width: Math.min(main.width - SetStyle.pageMargin * 2, SetStyle.contentMax)
                    spacing: 14
                    Repeater {
                        model: frame.crumbs
                        delegate: Row {
                            id: crumb
                            required property var modelData
                            required property int index
                            readonly property bool last: index === frame.crumbs.length - 1
                            spacing: 14
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: crumb.modelData.label
                                color: crumb.last ? Theme.text : Theme.muted
                                font.family: Theme.font; font.pixelSize: frame.compact ? 25 : 30; font.weight: crumb.last ? Font.DemiBold : Font.Medium
                                MouseArea { anchors.fill: parent; enabled: !crumb.last; cursorShape: Qt.PointingHandCursor; onClicked: frame.app.open(crumb.modelData.id) }
                            }
                            Icon { visible: !crumb.last; anchors.verticalCenter: parent.verticalCenter; name: "chev"; size: frame.compact ? 20 : 24; color: Theme.muted; opacity: 0.7 }
                        }
                    }
                    Item { Layout.fillWidth: true }
                }
                Loader {
                    id: loader
                    property bool failed: false
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom; top: title.bottom; topMargin: frame.compact ? 12 : 18 }
                    onStatusChanged: {
                        if (status === Loader.Error) { failed = true; console.warn("Hearth Settings: couldn't load", source); }
                        if (status === Loader.Ready && frame.pendingFocus !== "" && !nav.activeFocus) Qt.callLater(frame.enterPage);
                    }
                }
                // A page that couldn't load: say so (the rest of Settings keeps working).
                ColumnLayout {
                    visible: loader.failed
                    x: SetStyle.pageMargin
                    y: title.y + title.height + 24
                    width: Math.min(main.width - SetStyle.pageMargin * 2, 700)
                    SetBanner { kind: "bad"; title: "This page couldn't open"; sub: "The rest of Settings works. Send logs from Updates & recovery › Backup and help so it can be fixed." }
                }
                // The page fades out under the hint strip.
                Rectangle {
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                    height: 36
                    gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: Theme.bg } }
                }
            }
        }

        // ---------------------------------------------------------------- what the buttons do
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 44
            color: SetStyle.nav
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
            RowLayout {
                anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
                spacing: 22
                Hint { keys: ["A"]; label: "Select" }
                Hint { keys: ["B"]; label: "Back" }
                Hint { keys: ["Y"]; label: "Search" }
                Item { Layout.fillWidth: true }
                Hint { keys: ["LB", "RB"]; square: true; label: "Previous / next category" }
                Hint { keys: ["☰"]; square: true; label: "Close" }
            }
        }
    }

    // ---------------------------------------------------------------- search results, over the page
    Rectangle {
        anchors.fill: parent
        visible: dropdown.visible
        color: "#73000000"
        MouseArea { anchors.fill: parent; onClicked: { search.text = ""; nav.forceActiveFocus(); } }
    }
    Rectangle {
        id: dropdown
        visible: frame.query.trim() !== "" && (search.activeFocus || resultList.activeFocus)
        x: side.x + (frame.compact ? 10 : 16)
        y: searchBox.mapToItem(frame, 0, 0).y + searchBox.height + 10
        width: Math.min(600, frame.width - x - 20)
        height: Math.min(frame.height - y - 56, ddCol.implicitHeight + 16)
        radius: Math.max(10, Theme.r(14))
        color: SetStyle.card2
        border.width: 1; border.color: Theme.line
        ColumnLayout {
            id: ddCol
            anchors { fill: parent; margins: 8 }
            spacing: 2
            RowLayout {
                Layout.fillWidth: true; Layout.leftMargin: 12; Layout.rightMargin: 12; Layout.topMargin: 6; Layout.bottomMargin: 4
                Text {
                    Layout.fillWidth: true
                    text: frame.results.settings.length + frame.results.pages.length === 0 ? "Nothing matches “" + frame.query.trim() + "”"
                        : frame.results.settings.length + (frame.results.settings.length === 1 ? " setting matches “" : " settings match “") + frame.query.trim() + "”"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight
                }
                Text { text: "A open · B close"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            }
            ListView {
                id: resultList
                Layout.fillWidth: true
                Layout.fillHeight: true
                implicitHeight: contentHeight
                clip: true
                model: frame.shown
                spacing: 2
                boundsBehavior: Flickable.StopAtBounds
                keyNavigationEnabled: false
                Keys.onPressed: e => {
                    switch (e.key) {
                    case Qt.Key_Down: case Qt.Key_Tab: if (currentIndex < count - 1) currentIndex++; break;
                    case Qt.Key_Up: case Qt.Key_Backtab: if (currentIndex <= 0) search.forceActiveFocus(); else currentIndex--; break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: frame.openResult(frame.shown[currentIndex]); break;
                    case Qt.Key_Escape: search.forceActiveFocus(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
                delegate: Rectangle {
                    id: res
                    required property var modelData
                    required property int index
                    readonly property bool lit: resultList.activeFocus && resultList.currentIndex === index
                    width: ListView.view.width
                    height: 58
                    radius: Math.max(8, Theme.r(10))
                    color: lit || resMouse.containsMouse ? SetStyle.card : "transparent"
                    border.width: lit ? 3 : 0
                    border.color: Theme.accent
                    RowLayout {
                        anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                        spacing: 14
                        Icon { name: res.modelData.icon || "gear"; size: 22; color: Theme.text }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text { Layout.fillWidth: true; textFormat: Text.StyledText; text: frame.highlight(res.modelData.title); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium; elide: Text.ElideRight }
                            Text { Layout.fillWidth: true; text: (res.modelData.isPage ? "Page · " : "") + res.modelData.path + (res.modelData.old ? " · was “" + res.modelData.old + "”" : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                        }
                        Icon { name: "chev"; size: 18; color: Theme.muted }
                    }
                    MouseArea { id: resMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: frame.openResult(res.modelData) }
                }
            }
            RowLayout {
                visible: frame.results.pages.length > 0 || (!frame.showAll && frame.results.settings.length + frame.results.pages.length > 8)
                Layout.fillWidth: true; Layout.leftMargin: 12; Layout.rightMargin: 12; Layout.topMargin: 4; Layout.bottomMargin: 4
                Text { Layout.fillWidth: true; text: frame.results.pages.length ? "Pages: " + frame.results.pages.slice(0, 4).map(p => p.title).join(" · ") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                Text {
                    visible: !frame.showAll && frame.results.settings.length + frame.results.pages.length > 8
                    text: "Show all results"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 13
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: frame.showAll = true }
                }
            }
        }
    }

    // The Home server's dot on the list: something there needs a look (read only, every few minutes).
    QtObject {
        id: homeServer
        property bool needsLook: false
    }
    Process {
        id: hsProbe
        running: true
        command: ["sh", "-c", "f=\"$0\"; [ -f \"$f\" ] || exit 0; exec python3 \"$f\" quick", decodeURIComponent(Qt.resolvedUrl("hearth-home-server").toString().replace(/^file:\/\//, ""))]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const o = JSON.parse(text.split("\n").find(l => l.startsWith("{")) || "{}");
                    homeServer.needsLook = (o.alerts || []).length > 0 || ((o.backups || {}).failed || []).length > 0;
                } catch (e) { homeServer.needsLook = false; }
            }
        }
    }

    component Hint: Row {
        id: hint
        property var keys: []
        property string label
        property bool square: false
        spacing: 7
        Repeater {
            model: hint.keys
            delegate: Rectangle {
                required property string modelData
                width: Math.max(22, k.implicitWidth + 10); height: 22
                radius: hint.square ? 6 : 11
                color: "transparent"; border.width: 1.5; border.color: Theme.muted
                Text { id: k; anchors.centerIn: parent; text: modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold }
            }
        }
        Text { height: 22; verticalAlignment: Text.AlignVCenter; text: hint.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
    }
}
