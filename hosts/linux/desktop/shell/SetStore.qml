// Hearth Settings › App store (like KDE Discover), Hearth's own and controller-friendly: Flathub's popular, trending
// and new apps, its categories and search; an app's page with screenshots, description, rating and reviews (ODRS),
// size, license and Install / Open / Remove; Updates (your Flatpaks and the system's, with links to Hearth OS updates
// and Firmware); Sources (Flatpak remotes: add a .flatpakrepo, remove yours). All through hearth-store. Up/Down move,
// A opens or installs, B goes back (from an app to the list it came from).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "App store"
    property string view: "discover"      // discover | category | search | app | updates | sources
    property string back: "discover"
    property var home: null
    property var apps: []                 // the category's or the search's
    property string catLabel: ""
    property var app: null
    property string appId: ""
    property var ups: null
    property var srcs: []
    property bool loading: false
    property bool addingSource: false
    property string confirm: ""
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function fail(e) { loading = false; say(e, true); }
    function go(v) { say(""); confirm = ""; view = v; contentY = 0; }
    function loadHome() { loading = true; reader.run(["home"], r => { page.home = r; page.loading = false; if (r.cached) page.say("Offline: showing what Flathub showed last time."); }, page.fail); }
    function openCategory(c) { catLabel = c.label; apps = []; go("category"); loading = true; reader.run(["category", c.id], r => { page.apps = r.apps; page.loading = false; }, page.fail); }
    function search(t) { if (t.trim().length < 2) return; catLabel = "“" + t.trim() + "”"; apps = []; go("search"); loading = true; reader.run(["search", t.trim()], r => { page.apps = r.apps; page.loading = false; }, page.fail); }
    function openApp(id) { back = view === "app" ? back : view; appId = id; app = null; go("app"); loading = true; reader.run(["app", id], r => { page.app = r; page.loading = false; }, page.fail); }
    function loadUpdates() { go("updates"); loading = true; reader.run(["updates"], r => { page.ups = r; page.loading = false; }, page.fail); }
    function loadSources() { go("sources"); loading = true; reader.run(["sources"], r => { page.srcs = r.sources; page.loading = false; }, page.fail); }
    function work(args, okText, then) {
        if (worker.busy) return;
        confirm = ""; say("");
        worker.run(args, () => { page.say(okText); SettingsNav.toast(okText); if (then) then(); }, e => page.say(e, true));
    }
    function stars(n) { const f = Math.round(n || 0); return "★★★★★".slice(0, f) + "☆☆☆☆☆".slice(0, 5 - f); }
    function mb(b) { return !b ? "" : b > 1e9 ? (b / 1e9).toFixed(1) + " GB" : Math.round(b / 1e6) + " MB"; }
    // Deep links (Settings' "page:part"): store:updates, store:sources, store:app-org_kde_krita (an app's page; "_" for ".").
    Component.onCompleted: {
        const a = SettingsNav.pageAnchor || "";
        if (a === "updates" || a === "sources" || a.startsWith("app-")) SettingsNav.pageAnchor = "";
        if (a === "updates") loadUpdates();
        else if (a === "sources") loadSources();
        else if (a.startsWith("app-")) { loadHome(); openApp(a.slice(4).replace(/_/g, ".")); }
        else loadHome();
    }
    SetJob { id: reader; helper: "hearth-store" }
    SetJob { id: worker; helper: "hearth-store" }
    // B on an app's page goes back to the list it came from.
    Keys.onEscapePressed: event => { if (view === "app") { go(back); event.accepted = true; } else if (view !== "discover") { go("discover"); event.accepted = true; } else event.accepted = false; }

    component AppLine: SetBase {
        id: line
        property var a
        Layout.fillWidth: true
        implicitHeight: 72
        onActivated: page.openApp(a.id)
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            spacing: 14
            Rectangle {
                implicitWidth: 48; implicitHeight: 48; radius: Theme.r(12); color: Theme.panel3; clip: true
                Image { anchors.fill: parent; source: line.a && line.a.icon ? line.a.icon : ""; asynchronous: true; fillMode: Image.PreserveAspectFit; sourceSize.width: 96; visible: status === Image.Ready }
                Icon { anchors.centerIn: parent; name: "store"; size: 22; visible: !line.a || !line.a.icon }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text { Layout.fillWidth: true; text: line.a ? line.a.name + (line.a.verified ? "  ✓" : "") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: line.a ? line.a.summary + (line.a.developer ? " · " + line.a.developer : "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
            }
            Icon { name: "chev"; size: 16; color: Theme.muted }
        }
    }

    // ---------------------------------------------------------------- the top: search and the three places
    SetField {
        Layout.fillWidth: true
        placeholder: "Search Flathub (Enter)"
        onSubmitted: t => page.search(t)
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        SetButton { text: "Discover"; icon: "store"; primary: page.view === "discover" || page.view === "category" || page.view === "search"; onActivated: { page.go("discover"); if (!page.home) page.loadHome(); } }
        SetButton { text: "Updates" + (page.ups && page.ups.flatpaks.length ? " (" + page.ups.flatpaks.length + ")" : ""); icon: "update"; primary: page.view === "updates"; onActivated: page.loadUpdates() }
        SetButton { text: "Sources"; icon: "link"; primary: page.view === "sources"; onActivated: page.loadSources() }
        SetButton { text: "Installed apps"; icon: "grid"; ghost: true; onActivated: SettingsNav.openPage("apps") }
        Item { Layout.fillWidth: true }
    }
    ColumnLayout {
        visible: worker.busy
        Layout.fillWidth: true
        spacing: 6
        Text { Layout.fillWidth: true; text: (worker.line || "Working…") + (worker.progress >= 0 ? "  " + Math.round(worker.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
        Rectangle { Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, worker.progress); height: parent.height; radius: 4; color: Theme.flame } }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.loading
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Asking Flathub…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }

    // ---------------------------------------------------------------- Discover
    SetCardGroup {
        title: "Categories"
        visible: page.view === "discover" && !!page.home
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: page.home ? page.home.categories : []
                delegate: SetButton { required property var modelData; text: modelData.label; icon: modelData.icon; onActivated: page.openCategory(modelData) }
            }
        }
    }
    Repeater {
        model: page.view === "discover" && page.home ? [["popular", "Popular"], ["trending", "Trending"], ["new", "New and updated"]] : []
        delegate: SetCardGroup {
            id: grp
            required property var modelData
            title: modelData[1]
            visible: (page.home[modelData[0]] || []).length > 0
            Repeater {
                model: (page.home[grp.modelData[0]] || []).slice(0, 8)
                delegate: AppLine { required property var modelData; a: modelData }
            }
        }
    }

    // ---------------------------------------------------------------- a category or the search
    SetCardGroup {
        title: page.catLabel
        visible: page.view === "category" || page.view === "search"
        SetRow { visible: !page.loading && page.apps.length === 0; interactive: false; icon: "search"; chevron: false; title: "Nothing found" }
        Repeater {
            model: page.view === "category" || page.view === "search" ? page.apps : []
            delegate: AppLine { required property var modelData; a: modelData }
        }
    }

    // ---------------------------------------------------------------- an app
    ColumnLayout {
        visible: page.view === "app" && !!page.app
        Layout.fillWidth: true
        spacing: 14
        RowLayout {
            Layout.fillWidth: true
            spacing: 18
            Rectangle {
                implicitWidth: 96; implicitHeight: 96; radius: Theme.r(20); color: Theme.panel3; clip: true
                Image { anchors.fill: parent; source: page.app && page.app.icon ? page.app.icon : ""; asynchronous: true; fillMode: Image.PreserveAspectFit; sourceSize.width: 192 }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4
                Text { Layout.fillWidth: true; text: page.app ? page.app.name + (page.app.verified ? "  ✓" : "") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: page.app ? page.app.summary : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; wrapMode: Text.WordWrap }
                Text {
                    Layout.fillWidth: true
                    text: !page.app ? "" : [page.app.developer, page.app.ratings ? page.stars(page.app.ratings.average) + " " + page.app.ratings.average + " (" + page.app.ratings.total + " ratings)" : "No ratings yet",
                                            page.mb(page.app.size) ? page.mb(page.app.size) + " download" : ""].filter(x => x).join("  ·  ")
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight
                }
            }
        }
        RowLayout {
            spacing: 10
            SetButton {
                visible: !!page.app && page.app.installed.length === 0
                primary: true; text: "Install"; icon: "plus"
                onActivated: page.work(["install", page.appId], page.app.name + " is installed.", () => page.openApp(page.appId))
            }
            SetButton {
                visible: !!page.app && page.app.installed.length > 0
                primary: true; text: "Open"; icon: "open"
                onActivated: Quickshell.execDetached(["flatpak", "run", page.appId])
            }
            SetButton {
                visible: !!page.app && page.app.installed.length > 0
                text: page.confirm === "remove" ? "Yes, remove it" : "Remove…"; icon: "trash"
                onActivated: { if (page.confirm !== "remove") page.confirm = "remove"; else page.work(["remove", page.appId], page.app.name + " was removed.", () => page.openApp(page.appId)); }
            }
            SetButton { text: "Back"; icon: "back"; ghost: true; onActivated: page.go(page.back) }
        }
        // Screenshots: Left/Right move through them.
        SetBase {
            id: shotsCard
            visible: !!page.app && page.app.screenshots.length > 0
            Layout.fillWidth: true
            implicitHeight: 380
            usesLeftRight: true
            property int at: 0
            onAdjusted: step => at = Math.max(0, Math.min((page.app ? page.app.screenshots.length : 1) - 1, at + step))
            onActivated: at = (at + 1) % Math.max(1, page.app ? page.app.screenshots.length : 1)
            ListView {
                id: shotList
                anchors { fill: parent; margins: 12 }
                orientation: ListView.Horizontal
                spacing: 12
                clip: true
                currentIndex: shotsCard.at
                highlightMoveDuration: Theme.quick
                model: page.app ? page.app.screenshots : []
                delegate: Rectangle {
                    required property var modelData
                    width: Math.min(shotList.width, shotList.height * 16 / 9); height: shotList.height
                    radius: Theme.r(10); color: Theme.panel3; clip: true
                    Image { anchors.fill: parent; source: modelData.url; asynchronous: true; fillMode: Image.PreserveAspectFit; sourceSize.width: 1280 }
                }
            }
        }
        Text {
            Layout.fillWidth: true
            text: page.app ? page.app.description : ""
            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap; lineHeight: 1.2
        }
        SetCardGroup {
            title: "Reviews"
            visible: !!page.app && page.app.reviews.length > 0
            Repeater {
                model: page.app ? page.app.reviews : []
                delegate: SetRow {
                    required property var modelData
                    icon: "user"; chevron: false; interactive: false
                    title: page.stars(modelData.stars) + "  " + modelData.summary
                    sub: modelData.who + " · " + modelData.text
                }
            }
        }
        SetCardGroup {
            title: "About"
            SetRow { interactive: false; icon: "info"; chevron: false; title: "License"; value: page.app ? page.app.license || "Not given" : "" }
            SetRow { interactive: false; icon: "box"; chevron: false; title: "App id"; value: page.appId }
            SetRow {
                visible: !!page.app && page.app.homepage !== ""
                icon: "globe"; title: "Website"; sub: page.app ? page.app.homepage : ""
                onActivated: Quickshell.execDetached(["xdg-open", page.app.homepage])
            }
        }
    }

    // ---------------------------------------------------------------- Updates
    SetCardGroup {
        title: "Updates"
        visible: page.view === "updates"
        SetRow {
            visible: !!page.ups && page.ups.flatpaks.length > 0
            icon: "update"; chevron: false; title: "Update all apps"
            sub: page.ups ? page.ups.flatpaks.length + " Flatpak " + (page.ups.flatpaks.length === 1 ? "update" : "updates") : ""
            onActivated: page.work(["update"], "Your apps are up to date.", () => page.loadUpdates())
        }
        SetRow { visible: !!page.ups && page.ups.flatpaks.length === 0; interactive: false; icon: "check"; chevron: false; title: "Your apps are up to date" }
        Repeater {
            model: page.ups ? page.ups.flatpaks : []
            delegate: SetRow {
                required property var modelData
                icon: "store"; chevron: false
                title: modelData.name || modelData.id
                sub: [modelData.version, modelData.size, modelData.where === "system" ? "for everyone" : "yours", modelData.origin].filter(x => x).join(" · ")
                value: "Update"
                onActivated: page.work(["update", modelData.id], (modelData.name || modelData.id) + " is up to date.", () => page.loadUpdates())
            }
        }
        SetRow { icon: "update"; title: "Hearth OS updates"; sub: "The system itself: Settings › Updates"; onActivated: SettingsNav.openPage("updates") }
        SetRow { icon: "cpu"; title: "Firmware"; sub: "Your PC's and devices' firmware: Settings › Firmware"; onActivated: SettingsNav.openPage("firmware") }
    }

    // ---------------------------------------------------------------- Sources
    SetCardGroup {
        title: "Sources"
        visible: page.view === "sources"
        Repeater {
            model: page.srcs
            delegate: SetRow {
                required property var modelData
                icon: "link"; chevron: false
                title: (modelData.title || modelData.name) + (modelData.enabled ? "" : " (off)")
                sub: modelData.url + " · " + (modelData.where === "system" ? "for everyone" : "yours")
                value: modelData.where === "yours" ? (page.confirm === "src:" + modelData.name ? "Remove it" : "Remove…") : ""
                danger: page.confirm === "src:" + modelData.name
                onActivated: {
                    if (modelData.where !== "yours") return;
                    if (page.confirm !== "src:" + modelData.name) page.confirm = "src:" + modelData.name;
                    else page.work(["source-remove", modelData.name], modelData.name + " was removed.", () => page.loadSources());
                }
            }
        }
        SetRow { visible: !page.addingSource; icon: "plus"; chevron: false; title: "Add a source…"; sub: "A .flatpakrepo file's https:// address (it carries the key that checks its apps)"; onActivated: page.addingSource = true }
        SetField {
            id: srcUrl
            visible: page.addingSource
            Layout.fillWidth: true
            placeholder: "https://…/name.flatpakrepo (Enter adds it)"
            onSubmitted: t => {
                const u = t.trim(), name = (u.split("/").pop() || "").replace(/\.flatpakrepo$/, "").replace(/[^A-Za-z0-9._-]/g, "-");
                page.addingSource = false; text = "";
                page.work(["source-add", name, u], name + " was added.", () => page.loadSources());
            }
        }
    }
}
