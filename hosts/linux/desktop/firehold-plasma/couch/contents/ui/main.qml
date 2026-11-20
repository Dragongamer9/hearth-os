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
    readonly property string request: Plasmoid.configuration.request || ""
    onRequestChanged: if (couch.started && request !== "") couch.handle(request.split("|")[0])
}
