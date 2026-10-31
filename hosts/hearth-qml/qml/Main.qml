// Hearth's window, drawn by Qt Quick (Experimental): the same Game Mode as the web page (ui/index.html), as far as the
// Qt renderer has it (docs/RENDERERS.md). Title "Hearth", as the other renderers', so Game Mode finds it.
import QtQuick

Window {
    id: win
    title: "Hearth"
    color: Theme.bg
    width: 1280
    height: 800
    visible: true
    visibility: hostApp.windowed || hostApp.selfTest ? Window.Windowed : Window.FullScreen
    property alias searchLayer: search

    function showFromCore() { if (hostApp.windowed || hostApp.selfTest) win.show(); else win.showFullScreen(); win.raise(); win.requestActivate(); }
    function makeOverlay() {
        if (App.overlayWin) return;
        const c = Qt.createComponent("OverlayWindow.qml");
        if (c.status === Component.Ready) c.createObject(null);
        else App.log("overlay window: " + c.errorString());
    }
    function settingsPage() { return viewLoader.item; }
    function gamePage() { return viewLoader.item; }
    function hostPad(b) { input.press(b, true, "core"); input.press(b, false, "core"); }

    onWidthChanged: Theme.base = Math.min(height / 800, width / 1280)
    onHeightChanged: Theme.base = Math.min(height / 800, width / 1280)
    onVisibleChanged: App.reportWin("main", win)
    onActiveChanged: App.reportWin("main", win)
    // As the session there's nothing behind Hearth, so Alt+F4 must not close it by accident.
    onClosing: close => { if (hostApp.inSession && !hostApp.windowed) close.accepted = false; }
    Component.onCompleted: {
        App.mainWin = win;
        Theme.base = Math.min(height / 800, width / 1280);
        App.log("window up (" + width + "×" + height + ")");
        if (hostApp.selfTest) selfTest.start();
    }
    Connections {
        target: coreLink
        function onLine(l) { App.fromCore(l); }
        function onRunningChanged() { if (coreLink.running && App.connection === "starting") { App.boot(); win.reportLater.start(); } }
    }
    property Timer reportLater: Timer { interval: 300; onTriggered: App.reportWin("main", win) }

    readonly property var titles: ({ home: "Home", library: "Library", game: "", downloads: "Downloads", settings: "Settings", missing: "Not here yet" })

    Item {
        id: frame
        anchors.fill: parent
        Rectangle { anchors.fill: parent; color: Theme.bg }
        TopBar {
            id: top
            anchors { left: parent.left; right: parent.right; top: parent.top }
            title: App.view === "game" ? ((App.g(App.gameId) || {}).title || "") : (win.titles[App.view] || "")
            z: 3
        }
        Loader {
            id: viewLoader
            anchors { left: parent.left; right: parent.right; top: top.bottom; bottom: foot.top }
            source: ({ home: "HomeView.qml", library: "LibraryView.qml", game: "GameView.qml", downloads: "DownloadsView.qml", settings: "SettingsView.qml", missing: "MissingView.qml" })[App.view] || "HomeView.qml"
            onLoaded: App.later(40, () => { if (!App.layers.length && (!Nav.focused || Nav.scope !== "view" || !Nav.usable(Nav.focused))) Nav.reset("view"); })
        }
        Footer {
            id: foot
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            hints: App.layerTop() === "search" ? [["A", "Type"], ["X", "Delete"], ["Y", "Space"], ["☰", "Search"], ["B", "Hide"]]
                 : App.layerTop() ? [["A", "Select"], ["B", "Back"]]
                 : [["A", "Select"], ["B", "Back"], ["Y", "Search"], ["☰", "Menu"], ["⊙", "Quick Access"]]
            z: 3
        }
    }
    MainMenu { anchors.fill: parent; z: 10 }
    QuickAccess { id: qam; anchors.fill: parent; z: 11; onCloseRequested: App.back() }
    LaunchScreen { anchors.fill: parent; z: 9 }
    SearchLayer { id: search; anchors.fill: parent; z: 12 }
    PowerMenu { anchors.fill: parent; z: 13 }
    Sheet { anchors.fill: parent; z: 14 }
    Toast { id: toast; z: 20; text: App.toastText; seq: App.toastSeq }
    // Volume and brightness (the hardware buttons): a small notice at the top.
    Rectangle {
        id: osd
        z: 21
        property bool shown: false
        anchors.horizontalCenter: parent.horizontalCenter
        y: shown ? Theme.u(20) : -height - 10
        width: Theme.u(260); height: Theme.u(54); radius: Theme.u(14); color: Theme.panel3
        Behavior on y { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        Row {
            anchors.centerIn: parent; spacing: Theme.u(12)
            Icon { name: App.osd && App.osd.kind === "bright" ? "sun" : (App.osd && App.osd.muted ? "mute" : "vol"); size: Theme.u(20); anchors.verticalCenter: parent.verticalCenter }
            Bar { width: Theme.u(160); anchors.verticalCenter: parent.verticalCenter; value: App.osd ? ((App.osd.volume !== undefined ? App.osd.volume : App.osd.brightness || 0) / (App.osd.max || 100)) : 0 }
        }
        Timer { id: osdHide; interval: 1800; onTriggered: osd.shown = false }
        Connections { target: App; function onOsdSeqChanged() { osd.shown = true; osdHide.restart(); } }
    }

    Input { id: input; anchors.fill: parent; searchLayer: search; window: win }
    SelfTest { id: selfTest; mainWindow: win }
}
