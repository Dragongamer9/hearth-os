// Firehold on labwc's Couch on one screen (1.16): the approved Couch (docs/mockups/firehold-plasma 01-12, checklist
// docs/mockups/checklists/firehold-labwc.md), the very pages Firehold on Plasma draws (fhcouch/ is
// desktop/firehold-plasma/couch/contents/ui), with labwc's side given by FhCouchHost.qml. Mantel's Firehold.qml still
// decides the posture, the Desk, the Guide panel over a game and the controller; this is what's on a Couch screen.
// If the shared Couch can't load, Firehold.qml goes back to its own Couch (FireholdCouch.qml) and the log says why.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: pw
    property var fh
    property string scrName: ""
    // The Couch the controller drives: the TV's when a TV is Couch, else the first screen's (Firehold.qml).
    readonly property bool primary: fh.padScreen === scrName
    onPrimaryChanged: if (primary && shown) fh.activeCouch = pw
    Component.onCompleted: { fh.couches = fh.couches.concat([pw]); if (shown && (primary || !fh.activeCouch)) fh.activeCouch = pw; }
    readonly property bool shown: fh.unlocked && fh.postureOfName(scrName) === "couch" && fh.view === "home"
    visible: shown
    color: "#0f0b0a"
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-firehold"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    implicitWidth: screen ? screen.width : 1920
    implicitHeight: screen ? screen.height : 1080
    property double shownAt: 0

    FhCouchHost {
        id: host
        fh: pw.fh
        front: pw.shown && !pw.fh.guideShown && !pw.fh.settingsShown
    }
    readonly property var couch: loader.item
    Loader {
        id: loader
        anchors.fill: parent
        focus: true
        Component.onCompleted: setSource("fhcouch/Couch.qml", { hostIn: host, screenFor: pw.scrName })
        onStatusChanged: if (status === Loader.Error) {
            console.warn("firehold: the shared Couch didn't load; Firehold's own Couch instead");
            // (why, for Send logs: each page's own error)
            for (const f of ["Couch", "Home", "TopBar", "Library", "AppsPage", "Switcher", "CouchSettings", "Away", "GameOptions", "GameTile", "Seg", "Toggle"]) {
                const c = Qt.createComponent("fhcouch/" + f + ".qml");
                if (c.status === Component.Error) console.warn("firehold: fhcouch/" + f + ".qml: " + c.errorString().replace(/\n/g, " | "));
            }
            pw.fh.sharedCouchFailed = true;
        }
    }

    // Mantel's Firehold and this Couch agree on the tab and Everything open (the Desk's dock, the Guide twice).
    Connections {
        target: pw.fh
        function onTabChanged() { if (pw.couch && pw.couch.tabs.indexOf(pw.fh.tab) >= 0) pw.couch.tab = pw.fh.tab; }
        function onPageChanged() { if (!pw.couch) return; if (pw.fh.page === "switcher") pw.couch.page = "switcher"; else if (pw.couch.page === "switcher") pw.couch.page = "tabs"; }
    }
    Connections {
        target: pw.couch
        ignoreUnknownSignals: true
        function onPageChanged() { if (pw.couch.page !== "switcher" && pw.fh.page === "switcher") pw.fh.page = "tabs"; }
    }

    // (The live modes, Couch, Desk or Away for each screen: hearth-firehold-plasma tells the Couch when they change,
    // through Firehold's IPC "ask modes", as it's told under Plasma.)

    onShownChanged: {
        if (shown) { shownAt = Date.now(); if (primary || !fh.activeCouch) fh.activeCouch = pw; if (couch) couch.forceActiveFocus(); }
        else if (fh.activeCouch === pw) fh.activeCouch = null;
    }
    Component.onDestruction: { fh.couches = fh.couches.filter(c => c !== pw); if (fh.activeCouch === pw) fh.activeCouch = null; }

    // The controller's buttons from Mantel's pad helper (Firehold.qml padButton): the same keys the Couch takes from
    // hearth-firehold-plasma's controller under Plasma.
    function press(b) {
        console.info("firehold: Couch on " + scrName + " takes " + b + (couch ? " (" + couch.page + ", " + couch.tab + ")" : " (not loaded)"));
        if (!couch) return;
        if (b === "view") { couch.page = couch.page === "switcher" ? "tabs" : "switcher"; return; }
        if (b === "menu") { fh.toggleQuick(); return; }      // ☰ Quick Settings (mockup 08)
        couch.handle("key:" + b);
    }
}
