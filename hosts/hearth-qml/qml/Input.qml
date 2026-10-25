// Buttons, from three places, all as the web page's names (a b x y lb rb lt rt l3 r3 view menu guide up down left right):
// controllers (SDL2, main.cpp's padInput), the core's own pad events (App: "pad"), and the keyboard. Held directions
// repeat (340 ms, then every 85 ms). The on-screen keyboard gets them first while it's open.
import QtQuick

Item {
    id: inp
    property var searchLayer: null
    property var window: null
    focus: true
    property string held: ""
    readonly property bool padOk: App.padsAllowed && (hostApp.selfTest || hostApp.inSession || (window && window.active))

    Timer {
        id: rep
        interval: 340
        repeat: true
        onTriggered: { interval = 85; if (inp.held) inp.act(inp.held); }
    }
    function startRepeat(b) { held = b; rep.interval = 340; rep.restart(); }
    function stopRepeat(b) { if (!b || held === b) { held = ""; rep.stop(); } }

    function press(b, down, src) {
        if (searchLayer && searchLayer.padButton(b, down)) { stopRepeat(); return; }
        if (!down) { stopRepeat(b); return; }
        if (src !== "key") App.pointer = false;
        if (["up", "down", "left", "right"].indexOf(b) >= 0) { act(b); if (src !== "core") startRepeat(b); return; }
        act(b);
    }
    function act(b) {
        const top = App.layerTop();
        switch (b) {
        case "up": case "down": case "left": case "right": Nav.move(b); return;
        case "a": Nav.activate(); return;
        case "b": App.back(); return;
        case "guide": case "view":
            if (top === "qam") App.back(); else { App.closeAll(); App.openLayer("qam"); }
            return;
        case "menu":
            if (top === "menu") App.closeLayer("menu"); else if (!top) App.openLayer("menu");
            return;
        case "y":
            if (top === "search" && searchLayer) { searchLayer.showKeyboard(); return; }
            if (!top) App.openLayer("search");
            return;
        case "x":
            if (!top && App.view === "game") { const g = App.g(App.gameId); if (g) App.setFav(g); }
            return;
        case "lb": App.tabStep(-1); return;
        case "rb": App.tabStep(1); return;
        }
    }

    Connections {
        target: typeof padInput !== "undefined" ? padInput : null
        function onButton(name, down) { if (inp.padOk) inp.press(name, down, "pad"); }
        function onStick(dir, down) {
            if (!inp.padOk) return;
            // The keyboard reads the stick itself (its Ember Wheel); elsewhere it moves the selection.
            if (inp.searchLayer && inp.searchLayer.open && inp.searchLayer.kbOn) return;
            inp.press(dir, down, "stick");
        }
        function onAxes(lx, ly, rx, ry) { if (inp.padOk && inp.searchLayer) inp.searchLayer.padAxes(lx, ly, rx, ry); }
    }

    readonly property var keyMap: ({})
    function keyName(e) {
        switch (e.key) {
        case Qt.Key_Up: return "up";
        case Qt.Key_Down: return "down";
        case Qt.Key_Left: return "left";
        case Qt.Key_Right: return "right";
        case Qt.Key_Return: case Qt.Key_Enter: return App.layerTop() === "search" ? "menu" : "a";
        case Qt.Key_Escape: return "b";
        case Qt.Key_Backspace: return App.layerTop() === "search" ? "" : "b";
        case Qt.Key_PageUp: return "lb";
        case Qt.Key_PageDown: return "rb";
        case Qt.Key_Menu: return "menu";
        }
        if (App.layerTop() === "search") return "";
        if (e.text === " ") return "a";
        if (e.text === "/") return "y";
        if (e.text === "m" || e.text === "M") return "menu";
        if (e.text === "q" || e.text === "Q") return "guide";
        if (e.text === "x" || e.text === "X") return "x";
        return "";
    }
    Keys.onPressed: e => {
        if (App.layerTop() === "search" && searchLayer) {
            if (e.key === Qt.Key_Backspace) { searchLayer.deleteBack(); e.accepted = true; return; }
            if (e.text && e.text.length === 1 && e.text >= " " && e.key !== Qt.Key_Escape && e.key !== Qt.Key_Return && e.key !== Qt.Key_Enter) { searchLayer.typeText(e.text); e.accepted = true; return; }
        }
        const b = keyName(e);
        if (!b) return;
        e.accepted = true;
        if (e.isAutoRepeat) { if (["up", "down", "left", "right"].indexOf(b) >= 0) act(b); return; }
        App.pointer = false;
        if (searchLayer && searchLayer.padButton(b, true)) return;
        act(b);
    }
    Keys.onReleased: e => {
        const b = keyName(e);
        if (b && !e.isAutoRepeat && searchLayer) searchLayer.padButton(b, false);
    }
}
