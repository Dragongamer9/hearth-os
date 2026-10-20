// Apps: Desktop Mode's app store for Flathub. Browse what's popular or a category, search, open an app's page
// (icon, screenshots, description), install, update and remove apps; "Updates" updates everything at once.
// A normal window like Hearth Settings, and used the same way with mouse, touch, keyboard or controller: Up/Down
// pick, Right/A go in, Left/B come back, B on the list closes. The work is done by hearth-apps (flatpak; apps
// are installed for you, the way Hearth installs its other Flatpaks).
//   quickshell ipc -p <shell folder> call apps open | page <installed|updates|search|Game|...> | app <id> | close
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Scope {
    id: store
    property bool shown: false
    property string view: "featured"       // a nav page id, or "app" for an app's page
    property string backTo: "featured"     // where an app's page goes back to
    property var lists: ({})               // view -> [apps]
    property var detail: null              // the app's page (hearth-apps app)
    property var status: null
    property var updatesList: []
    property string busyId: ""             // the app being installed, removed or updated ("--all" for everything)
    property string busyWhat: ""
    property string message: ""
    property bool messageBad: false
    property string confirmRemove: ""
    property int seq: 0                    // answers for a page you already left are dropped
    readonly property var current: view === "app" ? [] : (lists[view] || [])
    readonly property var nav: [
        { id: "featured", icon: "store", label: "Discover" },
        { id: "search", icon: "search", label: "Search" },
        { id: "installed", icon: "grid", label: "Installed" },
        { id: "updates", icon: "update", label: "Updates" + (updatesList.length ? " (" + updatesList.length + ")" : "") },
        { id: "Game", icon: "pad", label: "Games" },
        { id: "AudioVideo", icon: "vol", label: "Music & video" },
        { id: "Graphics", icon: "palette", label: "Graphics & photos" },
        { id: "Network", icon: "globe", label: "Internet" },
        { id: "Office", icon: "kbd", label: "Office" },
        { id: "Development", icon: "gear", label: "Programming" },
        { id: "Education", icon: "info", label: "Learning" },
        { id: "Science", icon: "sun", label: "Science" },
        { id: "System", icon: "display", label: "System" },
        { id: "Utility", icon: "grid", label: "Tools" }
    ]
    readonly property string navView: view === "app" ? backTo : view
    readonly property var titles: ({ featured: "Popular apps", search: "Search", installed: "Installed apps", updates: "Updates" })

    function open(what) {
        shown = true;
        if (!status) reader.run(["status"], r => store.status = r);
        if (what && /^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+){2,}$/.test(what)) showApp(what);
        else go(what && nav.some(n => n.id === what) ? what : view === "app" ? backTo : view);
        refreshUpdates();
    }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(""); }
    onShownChanged: SettingsNav.appsShown = shown
    Component.onCompleted: SettingsNav.apps = store

    function say(t, bad) { message = t; messageBad = !!bad; }
    function setList(v, rows) { const l = Object.assign({}, lists); l[v] = rows; lists = l; }
    function go(v) {
        view = v;
        confirmRemove = "";
        load(v);
    }
    function load(v) {
        const s = ++seq;
        if (v === "installed") reader.run(["installed"], r => { store.setList("installed", r.apps || []); }, e => store.say(e, true));
        else if (v === "updates") refreshUpdates();
        else if (v === "search") { if (searchText !== "") reader.run(["search", searchText], r => { if (s === store.seq) store.setList("search", r.apps || []); }, e => store.say(e, true)); }
        else reader.run(["browse", v], r => store.setList(v, r.apps || []), e => store.say(e, true), "", "list");
    }
    function refreshUpdates() { reader.run(["updates"], r => { store.updatesList = r.apps || []; store.setList("updates", store.updatesList); }); }
    property string searchText: ""
    Timer { id: searchLater; interval: 450; onTriggered: store.load("search") }

    function showApp(id) {
        if (view !== "app") backTo = view;
        detail = null;
        confirmRemove = "";
        view = "app";
        const s = ++seq;
        reader.run(["app", id], r => { if (s === store.seq) store.detail = r; }, e => store.say(e, true));
    }
    function afterChange(id) {
        if (detail && detail.id === id) reader.run(["app", id], r => { if (store.detail && store.detail.id === r.id) store.detail = r; });
        reader.run(["installed"], r => store.setList("installed", r.apps || []));
        refreshUpdates();
        if (view !== "app" && view !== "installed" && view !== "updates") load(view);
    }
    function work(args, id, what, okText) {
        if (worker.busy) return say("Wait for " + busyWhat.toLowerCase() + " to finish.", true);
        busyId = id; busyWhat = what; say("");
        worker.run(args, () => { store.busyId = ""; store.say(okText); SettingsNav.toast(okText); store.afterChange(id); },
            e => { store.busyId = ""; store.say(e, true); store.afterChange(id); });
    }
    function install(a) { work(["install", a.id], a.id, "Installing " + a.name, a.name + " is installed."); }
    function remove(a, data) { confirmRemove = ""; work(["remove", a.id].concat(data ? ["--delete-data"] : []), a.id, "Removing " + a.name, a.name + " was removed."); }
    function update(a) { work(["update", a.id], a.id, "Updating " + a.name, a.name + " is up to date."); }
    function updateAll() { work(["update", "--all"], "--all", "Updating everything", "Your apps are up to date."); }
    function hasUpdate(id) { return updatesList.some(u => u.id === id); }
    function iconUrl(a) {
        if (!a) return "";
        if (a.icon && a.icon.startsWith("/")) return "file://" + a.icon;
        if (a.icon && a.icon.startsWith("https://")) return a.icon;
        return Quickshell.iconPath(a.id, "application-x-executable");
    }

    SetJob { id: reader; helper: "hearth-apps" }
    SetJob { id: worker; helper: "hearth-apps" }

    IpcHandler {
        target: "apps"
        function open(): void { store.open(""); }
        function page(name: string): void { store.open(name); }
        function app(id: string): void { store.open(id); }
        function close(): void { store.close(); }
        function toggle(): void { store.toggle(); }
    }

    // An app in a list: icon, name, what it is, and whether it's installed.
    component AppRow: SetBase {
        id: ar
        required property var modelData
        readonly property var app: modelData
        Layout.fillWidth: true
        implicitHeight: 72
        RowLayout {
            anchors { fill: parent; leftMargin: 14; rightMargin: 16 }
            spacing: 14
            Image {
                Layout.preferredWidth: 44; Layout.preferredHeight: 44
                source: store.iconUrl(ar.app)
                sourceSize: Qt.size(88, 88)
                fillMode: Image.PreserveAspectFit
                asynchronous: true
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Text { Layout.fillWidth: true; text: ar.app.name || ar.app.id; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; visible: text !== ""; text: ar.app.summary || ar.app.version || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
            }
            Text {
                text: store.busyId === ar.app.id ? Math.max(0, Math.round(worker.progress * 100)) + "%" : store.hasUpdate(ar.app.id) ? "Update" : ar.app.installed ? "Installed" : ""
                color: store.hasUpdate(ar.app.id) ? Theme.flame : Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
            Text { text: "›"; color: Theme.muted; font.pixelSize: 20 }
        }
        onActivated: store.showApp(ar.app.id)
    }

    // A progress bar for the install, removal or update going on.
    component WorkProgress: ColumnLayout {
        Layout.fillWidth: true
        spacing: 6
        Text { Layout.fillWidth: true; text: (worker.line || store.busyWhat || "Working…") + (worker.progress >= 0 ? "  " + Math.round(worker.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle {
                width: parent.width * Math.max(0.03, worker.progress); height: parent.height; radius: 4
                gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: Theme.ember } GradientStop { position: 1; color: Theme.flame } }
            }
        }
    }

    LazyLoader {
        active: store.shown
        FloatingWindow {
            id: win
            title: "Apps"
            implicitWidth: 1100
            implicitHeight: 720
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) store.shown = false
            Component.onCompleted: { SettingsNav.appsNav = navList; navList.forceActiveFocus(); }
            Component.onDestruction: if (SettingsNav.appsNav === navList) SettingsNav.appsNav = null

            RowLayout {
                anchors.fill: parent
                spacing: 0

                // ------------------------------------------------ nav
                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: win.width < 900 ? 220 : 260
                    color: Theme.panel
                    ColumnLayout {
                        anchors { fill: parent; margins: 14 }
                        spacing: 6
                        RowLayout {
                            Layout.bottomMargin: 10; Layout.leftMargin: 6; Layout.topMargin: 6
                            spacing: 10
                            Icon { name: "store"; size: 26; color: Theme.accent }
                            Text { text: "Apps"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                        }
                        ListView {
                            id: navList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: store.nav
                            spacing: 4
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            currentIndex: Math.max(0, store.nav.findIndex(n => n.id === store.navView))
                            keyNavigationEnabled: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            function enter() {
                                if (store.view === "search") Qt.callLater(() => searchField.focusInput());
                                else content.focusFirst();
                            }
                            function pick(i) { const n = store.nav[i]; if (n && (n.id !== store.view)) store.go(n.id); }
                            Keys.onPressed: e => {
                                switch (e.key) {
                                case Qt.Key_Down: case Qt.Key_Tab: pick(Math.min(store.nav.length - 1, currentIndex + 1)); break;
                                case Qt.Key_Up: case Qt.Key_Backtab: pick(Math.max(0, currentIndex - 1)); break;
                                case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                                    if (store.view === "app" && e.key !== Qt.Key_Right) store.go(store.backTo);
                                    enter(); break;
                                case Qt.Key_Escape: store.close(); break;
                                default: return;
                                }
                                e.accepted = true;
                            }
                            delegate: Rectangle {
                                id: item
                                required property var modelData
                                required property int index
                                readonly property bool lit: store.navView === modelData.id
                                width: ListView.view.width
                                height: 44
                                radius: 10
                                color: lit ? (navList.activeFocus ? Theme.accent : Theme.panel3) : mouse.containsMouse ? Theme.panel2 : "transparent"
                                Behavior on color { ColorAnimation { duration: Theme.quick } }
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                                    spacing: 12
                                    Icon { name: item.modelData.icon; size: 18; color: item.lit && navList.activeFocus ? Theme.accentInk : item.lit ? Theme.accent : Theme.text }
                                    Text {
                                        Layout.fillWidth: true
                                        text: item.modelData.label
                                        color: item.lit && navList.activeFocus ? Theme.accentInk : Theme.text
                                        font.family: Theme.font; font.pixelSize: 14; font.weight: item.lit ? Font.DemiBold : Font.Normal
                                    }
                                }
                                MouseArea {
                                    id: mouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: { store.go(item.modelData.id); navList.forceActiveFocus(); }
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.leftMargin: 6
                            text: "Ⓐ open  Ⓑ back"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }
                    }
                }
                Rectangle { Layout.fillHeight: true; Layout.preferredWidth: 1; color: Theme.line }

                // ------------------------------------------------ page
                SetPage {
                    id: content
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    title: store.view === "app" ? "" : store.titles[store.view] || (store.nav.find(n => n.id === store.view) || { label: "" }).label
                    onTitleChanged: contentY = 0

                    // ---- lists
                    SetRow {
                        visible: !!store.status && !store.status.flatpak
                        interactive: false; chevron: false
                        icon: "info"; title: "Flatpak isn't installed"; sub: "Apps uses Flatpak to install apps from Flathub."
                    }
                    SetField {
                        id: searchField
                        visible: store.view === "search"
                        placeholder: "Search Flathub: a name, or what it does"
                        onTextChanged: { store.searchText = text.trim(); searchLater.restart(); }
                        onSubmitted: { searchLater.stop(); store.load("search"); }
                    }
                    WorkProgress { visible: store.busyId !== "" && (store.view !== "app" || !store.detail || store.busyId !== store.detail.id) }
                    SetRow {
                        visible: store.view === "updates" && store.updatesList.length > 0 && store.busyId === ""
                        icon: "update"; chevron: false; title: "Update all"
                        sub: store.updatesList.length + (store.updatesList.length === 1 ? " app has" : " apps have") + " an update"
                        onActivated: store.updateAll()
                    }
                    Text {
                        visible: store.message !== "" && store.view !== "app"
                        Layout.fillWidth: true; Layout.leftMargin: 4
                        text: store.message; color: store.messageBad ? Theme.danger : Theme.flame
                        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                    }
                    Repeater {
                        model: store.current
                        delegate: AppRow {}
                    }
                    Text {
                        visible: store.view !== "app" && store.current.length === 0
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                        text: reader.what === "list" && !store.lists[store.view] ? "Getting the list of apps from Flathub… The first time takes a minute."
                            : reader.busy && !store.lists[store.view] ? "Loading…"
                            : store.view === "search" ? (store.searchText === "" ? "Type to search." : "Nothing on Flathub matches \"" + store.searchText + "\".")
                            : store.view === "updates" ? "Everything is up to date."
                            : store.view === "installed" ? "No apps from Flathub are installed yet."
                            : "Nothing here yet."
                    }

                    // ---- an app's page
                    SetRow {
                        visible: store.view === "app"
                        icon: "back"; chevron: false; title: "Back"; sub: store.titles[store.backTo] || (store.nav.find(n => n.id === store.backTo) || { label: "" }).label
                        onActivated: { store.go(store.backTo); Qt.callLater(() => content.focusFirst()); }
                    }
                    RowLayout {
                        visible: store.view === "app" && !!store.detail
                        Layout.fillWidth: true
                        Layout.topMargin: 8
                        spacing: 18
                        Image {
                            Layout.preferredWidth: 96; Layout.preferredHeight: 96
                            source: store.iconUrl(store.detail)
                            sourceSize: Qt.size(192, 192)
                            fillMode: Image.PreserveAspectFit
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            Text { Layout.fillWidth: true; text: store.detail ? store.detail.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
                            Text { Layout.fillWidth: true; visible: text !== ""; text: store.detail ? (store.detail.developer || "") + (store.detail.verified ? (store.detail.developer ? " · " : "") + "Verified" : "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                            Text { Layout.fillWidth: true; text: store.detail ? store.detail.summary || "" : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; wrapMode: Text.WordWrap }
                        }
                    }
                    Text { visible: store.view === "app" && !store.detail; text: "Loading…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                    WorkProgress { visible: store.view === "app" && !!store.detail && store.busyId === store.detail.id }
                    Text {
                        visible: store.message !== "" && store.view === "app"
                        Layout.fillWidth: true; Layout.leftMargin: 4
                        text: store.message; color: store.messageBad ? Theme.danger : Theme.flame
                        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                    }
                    SetRow {
                        visible: store.view === "app" && !!store.detail && !store.detail.installed && store.busyId !== store.detail.id
                        icon: "update"; chevron: false; title: "Install"; sub: "From Flathub, for you"
                        onActivated: store.install(store.detail)
                    }
                    SetRow {
                        visible: store.view === "app" && !!store.detail && store.detail.installed && store.busyId !== store.detail.id
                        icon: "check"; chevron: false; title: "Open"
                        sub: store.detail ? "Version " + (store.detail.installedVersion || store.detail.version || "?") + (store.detail.size ? " · " + store.detail.size : "") : ""
                        onActivated: { Quickshell.execDetached(["flatpak", "run", store.detail.id]); SettingsNav.toast("Opening " + store.detail.name); }
                    }
                    SetRow {
                        visible: store.view === "app" && !!store.detail && store.detail.installed && store.hasUpdate(store.detail.id) && store.busyId !== store.detail.id
                        icon: "update"; chevron: false; title: "Update"
                        sub: { const u = store.detail ? store.updatesList.find(x => x.id === store.detail.id) : null; return u && u.version ? "To version " + u.version : "A newer version is ready"; }
                        onActivated: store.update(store.detail)
                    }
                    SetRow {
                        visible: store.view === "app" && !!store.detail && store.detail.installed && store.busyId !== store.detail.id && store.confirmRemove === ""
                        icon: "trash"; danger: true; chevron: false; title: "Remove…"
                        onActivated: store.confirmRemove = store.detail.id
                    }
                    ColumnLayout {
                        visible: store.view === "app" && !!store.detail && store.confirmRemove === store.detail.id
                        Layout.fillWidth: true
                        spacing: 8
                        SetRow { danger: true; chevron: false; title: "Remove, keep its settings and files"; sub: "If you install it again, it's as you left it"; onActivated: store.remove(store.detail, false) }
                        SetRow { danger: true; chevron: false; title: "Remove and delete its settings and files"; sub: "Everything the app kept in your home folder (~/.var/app) is deleted"; onActivated: store.remove(store.detail, true) }
                        SetRow { chevron: false; title: "Cancel"; onActivated: store.confirmRemove = "" }
                    }

                    // Screenshots: Left/Right (or the arrows, or a click on the sides) go through them.
                    SetBase {
                        id: shots
                        property int index: 0
                        readonly property var list: store.detail && store.view === "app" ? (store.detail.screenshots || []) : []
                        visible: list.length > 0
                        onListChanged: index = 0
                        Layout.fillWidth: true
                        implicitHeight: Math.round(width * 9 / 16) + 36
                        usesLeftRight: true
                        onAdjusted: d => index = (index + d + list.length) % list.length
                        onActivated: index = (index + 1) % list.length
                        Image {
                            id: shot
                            anchors { left: parent.left; right: parent.right; top: parent.top; bottom: dots.top; margins: 10 }
                            source: shots.list[shots.index] || ""
                            fillMode: Image.PreserveAspectFit
                            asynchronous: true
                            Text { anchors.centerIn: parent; visible: shot.status === Image.Loading; text: "Loading the screenshot…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                            Text { anchors.centerIn: parent; visible: shot.status === Image.Error; text: "The screenshot couldn't be loaded"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                        }
                        Row {
                            id: dots
                            anchors { bottom: parent.bottom; horizontalCenter: parent.horizontalCenter; bottomMargin: 12 }
                            spacing: 6
                            Repeater {
                                model: shots.list.length
                                delegate: Rectangle { required property int index; width: 8; height: 8; radius: 4; color: index === shots.index ? Theme.accent : Theme.panel3 }
                            }
                        }
                        MouseArea { anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; width: parent.width * 0.3; cursorShape: Qt.PointingHandCursor; onClicked: { shots.forceActiveFocus(); shots.adjusted(-1); } }
                        MouseArea { anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom; width: parent.width * 0.3; cursorShape: Qt.PointingHandCursor; onClicked: { shots.forceActiveFocus(); shots.adjusted(1); } }
                    }
                    SetHeader { visible: store.view === "app" && !!store.detail && (store.detail.description || "") !== ""; text: "About" }
                    // Each paragraph can take focus, so the controller scrolls through long descriptions too.
                    Repeater {
                        model: store.view === "app" && store.detail ? (store.detail.description || "").split("\n\n").filter(p => p.trim() !== "") : []
                        delegate: SetBase {
                            required property var modelData
                            Layout.fillWidth: true
                            implicitHeight: para.implicitHeight + 24
                            color: activeFocus ? Theme.panel2 : "transparent"
                            Text {
                                id: para
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 14 }
                                text: modelData
                                textFormat: Text.PlainText
                                wrapMode: Text.WordWrap
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 14; lineHeight: 1.2
                            }
                        }
                    }
                    SetRow {
                        visible: store.view === "app" && !!store.detail
                        interactive: true; chevron: false
                        icon: "info"; title: "Details"
                        sub: store.detail ? [store.detail.version ? "Version " + store.detail.version : "", store.detail.license ? "License: " + store.detail.license : "", store.detail.id].filter(x => x).join(" · ") : ""
                    }
                    SetRow {
                        visible: store.view === "app" && !!store.detail && (store.detail.homepage || "") !== ""
                        icon: "globe"; title: "Website"; sub: store.detail ? store.detail.homepage : ""
                        onActivated: Quickshell.execDetached(["xdg-open", store.detail.homepage])
                    }
                }
            }
        }
    }
}
