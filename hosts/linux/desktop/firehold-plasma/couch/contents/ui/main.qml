// Firehold's Couch shell's desktop (the containment org.hearthos.firehold.couch): each screen's Couch (Couch.qml).
// Requests also come through Plasma's scripting (its "request" key: hearth-firehold-plasma ask, KWin's Meta+Ctrl+G).
import QtQuick
import org.kde.plasma.plasmoid

ContainmentItem {
    id: root
    Couch {
        id: couch
        anchors.fill: parent
        focus: true
        Component.onCompleted: couch.run(["ready", "org.hearthos.firehold"])
    }
    // Widgets in the Couch's desktop (Hearth's shared Quick Settings): kept out of sight; they show their own
    // windows over the Couch when they open (the Couch's ≡).
    Item { id: tucked; x: -64; y: -64; width: 32; height: 32 }
    function tuck(applet) {
        const it = root.itemFor(applet);
        if (!it) return;
        it.parent = tucked;
        it.anchors.fill = tucked;
    }
    Connections {
        target: Plasmoid
        function onAppletAdded(applet, geometry) { root.tuck(applet); }
    }
    Component.onCompleted: { for (const a of Plasmoid.applets) tuck(a); }
    readonly property string request: Plasmoid.configuration.request || ""
    onRequestChanged: if (couch.started && request !== "") couch.handle(request.split("|")[0])
}
