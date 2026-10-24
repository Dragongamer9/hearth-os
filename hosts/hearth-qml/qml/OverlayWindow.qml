// Quick Access over a game: the see-through "Hearth Overlay" window Game Mode puts over the game (gamemode.rs). Not
// on the Handheld Daemon editions (Anatase, Peppermint), where HHD's overlay app shows the web Quick Access instead.
// The core opens and closes it (overlay.shown / overlay.hidden) and hands it the controller (pad events).
import QtQuick

Window {
    id: ow
    title: "Hearth Overlay"
    color: "transparent"
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    visible: false
    width: Screen.width
    height: Screen.height
    property bool shown: false
    property string before: "view"

    function showFromCore() { ow.show(); }
    function close_() { App.send({ type: "overlay", action: "close" }, "overlay"); }
    onVisibleChanged: App.reportWin("overlay", ow)
    onActiveChanged: App.reportWin("overlay", ow)
    Component.onCompleted: { App.overlayWin = ow; App.reportWin("overlay", ow); App.send({ type: "ready" }, "overlay"); App.log("overlay window made"); }

    Timer { interval: 1000; repeat: true; running: ow.shown; onTriggered: App.send({ type: "overlay", action: "alive" }, "overlay") }
    Connections {
        target: App
        function onOverlayEvent(name, data) {
            if (name === "overlay.shown") {
                ow.shown = true;
                qa.open = true;
                qa.tab = "now";
                qa.refreshPerf(); qa.loadInput();
                if (Nav.scope.indexOf("ov") !== 0) ow.before = Nav.scope;
                // Said once it's really drawn, so Game Mode shows it with something in it.
                App.later(50, () => App.send({ type: "overlay", action: "painted" }, "overlay"));
                App.later(30, () => Nav.reset("ovqam"));
            } else if (name === "overlay.hidden") {
                ow.shown = false; qa.open = false; pmenu.open = false;
                Nav.reset(ow.before || "view");
            } else if (name === "pad") {
                ow.pad(data.b);
            } else if (name === "power.menu") {
                pmenu.open = true; App.later(30, () => Nav.reset("ovpower"));
            } else if (name === "toast") {
                App.toast(data.text);
            } else if (name === "padHost") {
            }
        }
    }
    function pad(b) {
        if (!ow.shown) return;
        if (["up", "down", "left", "right"].indexOf(b) >= 0) Nav.move(b);
        else if (b === "a") Nav.activate();
        else if (b === "b") { if (pmenu.open) pmenu.doCancel(); else ow.close_(); }
        else if (b === "lb") App.tabStep(-1);
        else if (b === "rb") App.tabStep(1);
    }

    QuickAccess {
        id: qa
        anchors.fill: parent
        overlay: true
        scopeName: "ovqam"
        open: false
        onCloseRequested: ow.close_()
    }
    PowerMenu {
        id: pmenu
        anchors.fill: parent
        open: false
        scopeName: "ovpower"
        app: ow
        cancel: function () { pmenu.open = false; Nav.reset("ovqam"); }
    }
    // Power from Quick Access over a game: the core closes it first (gamemode.rs from_overlay "power"), then does it.
    function power(p) { App.log("power " + p + " (over a game)"); App.send({ type: "power", action: p }, "overlay"); pmenu.open = false; }
}
