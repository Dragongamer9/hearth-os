// Windows 11-style glyphs for Hearth Files' This PC and Installed apps (FilesThisPc.qml, FilesApps.qml): the yellow
// folder (with a small mark for Documents, Downloads, Pictures, Music, Videos, Desktop) and the drive glyph (grey
// drive with the accent light; Hearth's orange square on the system drive, a lighter one for USB and SD, blue for
// network locations). Drawn with rectangles so it follows the look's accent and needs no image files.
import QtQuick

Item {
    id: g
    property string kind: "folder"     // folder | desktop | documents | downloads | pictures | music | videos | system | drive | games | usb | sd | network
    property real size: 44
    readonly property bool drive: ["system", "drive", "games", "usb", "sd", "network"].indexOf(kind) >= 0
    readonly property string mark: ({ documents: "≡", downloads: "↓", pictures: "▲", music: "♪", videos: "▶", desktop: "▭", games: "◆" })[kind] || ""
    implicitWidth: size
    implicitHeight: size

    // ---- folder
    Item {
        visible: !g.drive
        anchors.centerIn: parent
        width: g.size; height: g.size * 0.78
        Rectangle {   // the tab
            x: 0; y: 0
            width: parent.width * 0.42; height: parent.height * 0.26
            radius: Math.max(1, g.size * 0.06)
            color: "#e8b23a"
        }
        Rectangle {   // the body
            x: 0; y: parent.height * 0.14
            width: parent.width; height: parent.height * 0.86
            radius: Math.max(1.5, g.size * 0.08)
            gradient: Gradient {
                GradientStop { position: 0; color: "#ffd76a" }
                GradientStop { position: 1; color: "#e8b23a" }
            }
            Text {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: g.size * 0.03
                visible: g.mark !== "" && g.size >= 28
                text: g.mark
                color: Qt.rgba(0.45, 0.28, 0.02, 0.75)
                font.pixelSize: g.size * 0.34
                font.weight: Font.Bold
            }
        }
    }

    // ---- drive
    Rectangle {
        visible: g.drive
        anchors.centerIn: parent
        width: g.size; height: g.size * 0.74
        radius: Math.max(1.5, g.size * 0.09)
        gradient: Gradient {
            GradientStop { position: 0; color: g.kind === "network" ? "#bfe3f7" : g.kind === "usb" || g.kind === "sd" ? "#cfcfcf" : "#d9d9d9" }
            GradientStop { position: 1; color: g.kind === "network" ? "#6aa8cc" : g.kind === "usb" || g.kind === "sd" ? "#7a7a7a" : "#9a9a9a" }
        }
        Rectangle {   // the light
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: parent.width * 0.18; rightMargin: parent.width * 0.18; bottomMargin: parent.height * 0.17 }
            height: Math.max(2, g.size * 0.07)
            radius: height / 2
            color: Theme.accent
        }
        Rectangle {   // Hearth's square on the system drive
            visible: g.kind === "system"
            anchors { right: parent.right; top: parent.top; rightMargin: parent.width * 0.12; topMargin: parent.height * 0.14 }
            width: parent.width * 0.28; height: width
            radius: Math.max(1, width * 0.18)
            gradient: Gradient {
                GradientStop { position: 0; color: "#ff9a4d" }
                GradientStop { position: 1; color: "#e05a2b" }
            }
        }
        Text {
            visible: (g.kind === "games" || g.kind === "sd") && g.size >= 28
            anchors { right: parent.right; top: parent.top; rightMargin: parent.width * 0.12; topMargin: parent.height * 0.06 }
            text: g.kind === "sd" ? "SD" : "◆"
            color: "#3a3a3a"
            font.pixelSize: g.size * 0.22
            font.weight: Font.Bold
        }
    }
}
