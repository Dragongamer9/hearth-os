// Search (Y): a field, the games that match, and Hearth's on-screen keyboard (the same keyboard as the web page's and
// Mantel's: hearth-osk.js and OskBoard.qml, docs/OSK.md). Controller on the keyboard: A types (hold for accents),
// X ⌫, Y space, LT shift, RT symbols, LB / RB move the caret, ☰ Enter (goes to the Library with the search), B hides
// the keyboard (again: closes search). Settings and learned words are Hearth's (osk.get / osk.learn), shared with
// Game Mode's web keyboard and Mantel's. A physical keyboard types straight into the field.
import QtQuick
import "hearth-osk.js" as Osk

Item {
    id: sl
    readonly property bool open: App.layers.indexOf("search") >= 0
    property var engine: null
    property var vm: null
    property var textModel: ({ text: "", caret: 0 })
    property int caret: 0
    property bool kbOn: true
    property bool configured: false
    readonly property var results: {
        App.rev;
        const q = App.query.trim().toLowerCase();
        if (!q) return [];
        return App.visibleGames().filter(x => x.title.toLowerCase().indexOf(q) >= 0).sort((a, b) => a.title.localeCompare(b.title)).slice(0, 8);
    }
    visible: open

    onOpenChanged: {
        if (!open) return;
        if (!engine) makeEngine();
        textModel = { text: App.query, caret: App.query.length };
        caret = textModel.caret;
        engine.reset("search", "Search");
        engine.setHints(true);
        engine.setActive(true);
        kbOn = true;
        App.log("osk: search opened (controller " + (typeof padInput !== "undefined" && padInput.available ? padInput.count + " pad(s) through SDL" : "from Hearth") + ")");
    }
    function makeEngine() {
        if (!configured) {
            configured = true;
            Osk.HearthOSK.configure({ save: d => App.call("osk.learn", { data: d }).catch(() => {}) });
            App.call("osk.get").then(r => {
                if (!r) return;
                Osk.HearthOSK.configure({ settings: r.settings && Object.keys(r.settings).length ? r.settings : undefined, systemLayouts: r.systemLayouts || [],
                                          learned: r.learned && r.learned.learned ? r.learned : undefined });
            }).catch(() => {});
        }
        engine = Osk.HearthOSK.createEngine({
            host: "qt",
            mode: "search",
            enterLabel: "Search",
            adapter: Osk.HearthOSK.modelAdapter(() => sl.textModel, () => { App.query = sl.textModel.text; sl.caret = sl.textModel.caret; }, (name) => { if (name === "enter") sl.done(); }),
            // Search: words from your games' titles instead of the dictionary's (as the web page does).
            suggest: (w) => {
                if (!w) return [];
                const f = Osk.HearthOSK.fold(w), seen = {}, out = [];
                for (const x of App.games) for (const t of String(x.title || "").split(/[\s:,.!?()\-–]+/)) {
                    const k = t.toLowerCase();
                    if (t.length > w.length && Osk.HearthOSK.fold(t).startsWith(f) && !seen[k]) { seen[k] = true; out.push(t); if (out.length >= 3) return out; }
                }
                return out;
            },
            onChange: () => { sl.vm = sl.engine.view(); },
            onHide: () => { sl.kbOn = false; App.later(20, () => Nav.reset("search")); },
            onLeave: () => { App.later(20, () => Nav.reset("search")); },
            onHaptic: (s, ms) => { if (typeof padInput !== "undefined") padInput.rumble(s, ms); }
        });
        vm = engine.view();
    }
    function done() {
        App.closeLayer("search");
        if (App.view !== "library") App.navigate("library");
    }
    // The controller while the keyboard has it (Input.qml asks first): true when the keyboard used the press.
    function padButton(b, down) {
        if (!open || !engine || !kbOn || !(vm && vm.active)) return false;
        engine.setHints(true);
        const used = engine.handle(b, down, Date.now());
        return used || b === "a" || b === "x" || b === "y";
    }
    function padAxes(lx, ly, rx, ry) { if (open && engine && kbOn) engine.axes(lx, ly, rx, ry, Date.now()); }
    function typeText(t) { const m = textModel; m.text = m.text.slice(0, m.caret) + t + m.text.slice(m.caret); m.caret += t.length; textModel = m; App.query = m.text; caret = m.caret; if (engine) engine.refresh(); }
    function deleteBack() { const m = textModel; if (m.caret <= 0) return; m.text = m.text.slice(0, m.caret - 1) + m.text.slice(m.caret); m.caret--; textModel = m; App.query = m.text; caret = m.caret; if (engine) engine.refresh(); }
    function showKeyboard() { kbOn = true; if (engine) { engine.setActive(true); engine.setHints(true); } }
    Timer { interval: 16; repeat: true; running: sl.open && sl.kbOn; onTriggered: if (sl.engine) sl.engine.tick(Date.now()) }

    Rectangle { anchors.fill: parent; color: Theme.alpha(Theme.bg, 0.94) }
    MouseArea { anchors.fill: parent }
    Column {
        id: top
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.gut; topMargin: Theme.u(24) }
        spacing: Theme.u(12)
        Focusable {
            id: field
            width: parent.width
            height: Theme.u(56)
            navScope: "search"
            navKey: "s-field"
            navFirst: true
            onActivated: sl.showKeyboard()
            Rectangle { anchors.fill: parent; radius: Theme.u(12); color: Theme.panel2; border.width: sl.kbOn ? Theme.u(2) : 1; border.color: sl.kbOn ? Theme.accent : Theme.line }
            Row {
                anchors { left: parent.left; leftMargin: Theme.u(16); verticalCenter: parent.verticalCenter }
                spacing: Theme.u(12)
                Icon { name: "search"; color: Theme.dim; size: Theme.u(22); anchors.verticalCenter: parent.verticalCenter }
                Item {
                    width: field.width - Theme.u(80)
                    height: Theme.u(30)
                    anchors.verticalCenter: parent.verticalCenter
                    Text {
                        id: qtext
                        anchors.verticalCenter: parent.verticalCenter
                        text: App.query || "Search your games"
                        color: App.query ? Theme.text : Theme.faint
                        font.family: Theme.font
                        font.pixelSize: Theme.u(20)
                    }
                    Text { id: measure; visible: false; text: App.query.slice(0, sl.caret); font: qtext.font }
                    Rectangle {
                        visible: sl.kbOn
                        x: App.query ? measure.implicitWidth + 1 : 0
                        anchors.verticalCenter: parent.verticalCenter
                        width: Theme.u(2); height: Theme.u(24); color: Theme.accent
                        SequentialAnimation on opacity { loops: Animation.Infinite; running: sl.open; NumberAnimation { to: 0; duration: 500 } NumberAnimation { to: 1; duration: 500 } }
                    }
                }
            }
        }
        Text {
            text: App.query ? (sl.results.length ? sl.results.length + (sl.results.length === 8 ? "+" : "") + " games" : "No games match") : "Type with the keyboard below, or a physical keyboard"
            color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13)
        }
        Flow {
            width: parent.width
            spacing: Theme.u(10)
            Repeater {
                model: sl.results
                delegate: Btn {
                    required property var modelData
                    navScope: "search"
                    navKey: "sr-" + modelData.id
                    icon: "play"
                    text: modelData.title
                    sub: App.sourceName(modelData) + (modelData.inst ? "" : " · Not installed")
                    onActivated: { App.closeLayer("search"); App.navigate("game", { gameId: modelData.id }); }
                }
            }
            Btn { visible: App.query !== ""; navScope: "search"; navKey: "s-lib"; kind: "ghost"; icon: "lib"; text: "Show in Library"; onActivated: sl.done() }
        }
    }
    OskBoard {
        id: board
        visible: sl.kbOn
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: Theme.gut; rightMargin: Theme.gut; bottomMargin: Theme.u(12) }
        height: Math.min(parent.height * 0.5, Math.max(Theme.u(260), parent.width * 0.26))
        vm: sl.vm
        onTapped: (r, c) => { if (sl.engine) { sl.engine.setHints(false); sl.engine.tap(r, c); } }
        onLongPressed: (r, c) => { if (sl.engine) sl.engine.longPress(r, c); }
        onAltPicked: i => { if (sl.engine) sl.engine.pickAlt(i); }
    }
}
