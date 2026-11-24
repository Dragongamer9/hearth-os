// The lock screen's password block (lock-screen-blocks boards 1-3): your picture and name, the password box with its
// arrow, "Fingerprint, or your password" and the "always on top · can't be removed" tag; or, in the 1995 look, the
// "Welcome back to Hearth" window with OK and Shut down.... It only DRAWS what Lock.qml has: the typed password comes
// from Lock.qml's own keyboard handling and unlocking is Lock.qml's PAM check (unlock() asks it to check). Whatever the
// look, the same safety underneath. Also the editor's preview of the block (preview: true, nothing to press).
import QtQuick
import Quickshell

Item {
    id: pw
    property string typed: ""
    property bool busy: false
    property string message: ""
    property string hint: "Fingerprint, or your password"
    property bool retro: false
    property bool preview: false
    property var st: ({})
    property color ink: "#f6ece4"
    property string fontName: Theme.font
    signal unlock()
    signal keys()
    signal shutDown()
    readonly property string userName: { const n = Quickshell.env("USER") || "you"; return n.charAt(0).toUpperCase() + n.slice(1); }
    readonly property string dots: preview ? "•••••••" : "•".repeat(typed.length)
    property bool askShutDown: false

    // ---- Hearth's own card
    Rectangle {
        visible: !pw.retro
        anchors.fill: parent
        radius: pw.st.corners !== undefined ? pw.st.corners : 22
        color: pw.st.colour || Qt.rgba(0.1, 0.07, 0.06, 0.82)
        border.width: 1; border.color: Qt.rgba(1, 1, 1, 0.1)
        Column {
            anchors.centerIn: parent
            width: parent.width - 48
            spacing: Math.max(6, parent.height * 0.03)
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                width: Math.min(104, pw.height * 0.28); height: width; radius: width / 2; clip: true
                gradient: Gradient { GradientStop { position: 0; color: "#5b8cff" } GradientStop { position: 1; color: "#b07cff" } }
                Text { anchors.centerIn: parent; text: pw.userName.charAt(0); color: "white"; font.family: pw.fontName; font.pixelSize: parent.width * 0.42; font.weight: Font.DemiBold }
                Image { anchors.fill: parent; source: "file://" + Quickshell.env("HOME") + "/.face"; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready }
            }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: pw.userName; color: pw.st.text || pw.ink; font.family: pw.fontName; font.pixelSize: 26; font.weight: Font.DemiBold }
            Rectangle {
                width: parent.width; height: 54; radius: 12
                color: Qt.rgba(1, 1, 1, 0.06); border.width: 1; border.color: pw.message ? "#ff6b6b" : Qt.rgba(1, 1, 1, 0.22)
                Text {
                    anchors { left: parent.left; leftMargin: 18; right: arrow.left; verticalCenter: parent.verticalCenter }
                    text: pw.busy ? "Checking…" : pw.dots.length ? pw.dots : "Password"; elide: Text.ElideRight
                    color: pw.dots.length ? (pw.st.text || pw.ink) : Qt.rgba(1, 1, 1, 0.45); font.family: pw.fontName; font.pixelSize: pw.dots.length ? 24 : 17; font.letterSpacing: pw.dots.length ? 2 : 0
                }
                MouseArea { anchors.fill: parent; enabled: !pw.preview; onClicked: pw.keys() }
                Rectangle {
                    id: arrow
                    anchors { right: parent.right; rightMargin: 8; verticalCenter: parent.verticalCenter }
                    width: 38; height: 38; radius: 10; color: am.containsMouse ? Qt.rgba(1, 1, 1, 0.12) : "transparent"
                    Text { anchors.centerIn: parent; text: "→"; color: pw.st.text || pw.ink; font.pixelSize: 20 }
                    MouseArea { id: am; anchors.fill: parent; hoverEnabled: true; enabled: !pw.preview; onClicked: pw.unlock() }
                }
            }
            Text { anchors.horizontalCenter: parent.horizontalCenter; width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
                   text: pw.message || pw.hint; color: pw.message ? "#ff8a8a" : Qt.rgba(1, 1, 1, 0.75); font.family: pw.fontName; font.pixelSize: 16 }
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                width: tagT.implicitWidth + 26; height: 26; radius: 13; color: Qt.rgba(1, 0.48, 0.18, 0.12); border.width: 1; border.color: Qt.rgba(1, 0.48, 0.18, 0.55)
                Text { id: tagT; anchors.centerIn: parent; text: "always on top · can't be removed"; color: "#ffb27a"; font.family: pw.fontName; font.pixelSize: 13 }
            }
        }
    }

    // ---- the 1995 look's window: "Welcome back to Hearth" (as tall as it needs, at the top of its place)
    Rectangle {
        visible: pw.retro
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: Math.min(parent.height, 220)
        color: "#c0c0c0"; border.width: 2; border.color: "#ffffff"
        Rectangle { anchors { right: parent.right; bottom: parent.bottom } width: parent.width; height: 2; color: "#404040" }
        Rectangle { anchors { right: parent.right; bottom: parent.bottom } width: 2; height: parent.height; color: "#404040" }
        Rectangle {
            id: title
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 3 }
            height: 34; color: "#000080"
            Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: "Welcome back to Hearth"; color: "white"; font.family: "Inter"; font.pixelSize: 17; font.bold: true }
        }
        Rectangle {
            id: face
            anchors { left: parent.left; leftMargin: 28; top: title.bottom; topMargin: 44 }
            width: 80; height: 80; color: "#808080"
            Text { anchors.centerIn: parent; text: pw.userName.charAt(0); color: "white"; font.family: "Inter"; font.pixelSize: 34 }
            Image { anchors.fill: parent; source: "file://" + Quickshell.env("HOME") + "/.face"; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready }
        }
        Column {
            anchors { left: face.right; leftMargin: 24; right: parent.right; rightMargin: 26; top: title.bottom; topMargin: 26 }
            spacing: 12
            Text { text: pw.message || "Type your password to unlock this PC."; color: pw.message ? "#a00000" : "#000000"; font.family: "Inter"; font.pixelSize: 18; width: parent.width; elide: Text.ElideRight }
            Rectangle {
                width: parent.width; height: 34; color: "white"; border.width: 2; border.color: "#404040"
                Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: pw.busy ? "Checking…" : pw.dots; color: "#000000"; font.pixelSize: 20 }
                MouseArea { anchors.fill: parent; enabled: !pw.preview; onClicked: pw.keys() }
            }
            Row {
                anchors.right: parent.right
                spacing: 12
                Repeater {
                    model: [{ t: "OK", a: "ok" }, { t: pw.askShutDown ? "Shut down now" : "Shut down...", a: "off" }]
                    delegate: Rectangle {
                        required property var modelData
                        width: Math.max(78, bt.implicitWidth + 34); height: 40; color: "#c0c0c0"; border.width: 2; border.color: "#ffffff"
                        Rectangle { anchors { right: parent.right; bottom: parent.bottom } width: parent.width; height: 2; color: "#404040" }
                        Rectangle { anchors { right: parent.right; bottom: parent.bottom } width: 2; height: parent.height; color: "#404040" }
                        Text { id: bt; anchors.centerIn: parent; text: modelData.t; color: "#000000"; font.family: "Inter"; font.pixelSize: 18 }
                        MouseArea {
                            anchors.fill: parent; enabled: !pw.preview
                            // Shut down asks once more (pressed by mistake on a lock screen, it would lose your work)
                            onClicked: { if (parent.modelData.a === "ok") pw.unlock(); else if (pw.askShutDown) pw.shutDown(); else { pw.askShutDown = true; askT.restart(); } }
                        }
                    }
                }
            }
        }
        Timer { id: askT; interval: 5000; onTriggered: pw.askShutDown = false }
    }
}
