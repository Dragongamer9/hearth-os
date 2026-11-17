// The login screen's on-screen keyboard: Mantel's own (OskBoard.qml, OskKey.qml and its engine osk/hearth-osk.js,
// copied next to this file when the image is built: docs/OSK.md), in its password mode: no suggestions, nothing
// learned or saved, no capitals by itself. Loaded by Login.qml with a Loader, so the login screen still works if it
// can't load. The D-pad / arrows pick keys, A / Enter types, B / Backspace deletes, Y / Escape hides it.
import QtQuick
import "osk/hearth-osk.js" as Osk

Item {
    id: k
    property var engine: null
    property var vm: null
    property var layouts: []
    signal typed(string text)
    signal back(int n)
    signal enter()
    signal hide()
    signal leave()
    readonly property string layoutShort: vm ? (vm.layoutShort || "") : ""
    readonly property string layoutName: vm ? (vm.layoutName || "") : ""

    function make() {
        Osk.HearthOSK.configure({
            settings: { theme: "ember", learn: false, suggestions: false, autocap: false, doubleSpace: false, fast: "off", haptics: "off" },
            systemLayouts: k.layouts,
            save: d => {}
        });
        engine = Osk.HearthOSK.createEngine({
            host: "mantel", mode: "password", enterLabel: "Sign in",
            adapter: {
                insert: (text, mods) => k.typed(text),
                deleteBack: n => k.back(n || 1),
                key: (name, mods) => { if (name === "enter") k.enter(); }
            },
            onChange: () => { k.vm = k.engine.view(); },
            onHide: () => k.hide(),
            onLeave: dir => { if (dir === "up") k.leave(); }
        });
        vm = engine.view();
    }
    function reset() { if (engine) { engine.reset("password", "Sign in"); engine.setHints(true); } }
    // arrows / Enter / Backspace from the login screen (the controller arrives as those keys)
    function press(btn) { if (engine) { engine.setHints(true); engine.handle(btn, "press", Date.now()); } }
    function pickLayout(xkb) {
        const id = Osk.HearthOSK.layoutForXkb ? Osk.HearthOSK.layoutForXkb(xkb) : null;
        Osk.HearthOSK.setSetting("layouts", id ? [id] : ["auto"]);
    }
    onLayoutsChanged: Osk.HearthOSK.configure({ settings: { theme: "ember", learn: false, suggestions: false, autocap: false, doubleSpace: false, fast: "off", haptics: "off" }, systemLayouts: k.layouts })

    OskBoard {
        id: board
        anchors.fill: parent
        vm: k.vm
        onTapped: (r, c) => { if (k.engine) { k.engine.setHints(false); k.engine.tap(r, c); } }
        onLongPressed: (r, c) => { if (k.engine) k.engine.longPress(r, c); }
        onAltPicked: i => { if (k.engine) k.engine.pickAlt(i); }
    }
    Timer { interval: 16; repeat: true; running: k.visible && k.engine !== null; onTriggered: k.engine.tick(Date.now()) }
    Component.onCompleted: make()
}
