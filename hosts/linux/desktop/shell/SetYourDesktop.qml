// Hearth Settings › System › Your desktop: Desktop Mode is Mantel (Hearth Classic). 1.13's KDE Plasma and GNOME choices
// were retired in 1.14 (the owner, Nov 16; archive/desktops-kde-gnome, docs/archive/DESKTOPS-KDE-GNOME.md): someone who
// had one picked gets Mantel and a notice saying so (DesktopNotice.qml once, the banner here while it's the last one).
// Everything through hearth-your-desktop. Opening the page only reads; nothing here changes by itself.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Your desktop"
    property var st: null
    property string msg: ""
    property bool bad: false
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-your-desktop" }

    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 8
        text: "Desktop Mode is Mantel, Hearth's own desktop."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetBanner {
        visible: !!page.st && !!page.st.notice && (page.st.notice.title || "") !== ""
        kind: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        icon: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        title: page.st && page.st.notice ? page.st.notice.title : ""
        sub: page.st && page.st.notice ? page.st.notice.sub : ""
    }

    SetBase {
        Layout.fillWidth: true
        implicitHeight: Math.max(96, col.implicitHeight + 36)
        border.width: 2
        border.color: Theme.accent
        interactive: false
        RowLayout {
            anchors { fill: parent; leftMargin: 22; rightMargin: 22 }
            spacing: 18
            Rectangle {   // the radio
                Layout.alignment: Qt.AlignVCenter
                width: 22; height: 22; radius: 11; color: "transparent"
                border.width: 2; border.color: Theme.accent
                Rectangle { anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: Theme.accent }
            }
            Rectangle {   // its letter
                Layout.alignment: Qt.AlignVCenter
                width: 54; height: 54; radius: 12; color: "#4a3a24"
                Text { anchors.centerIn: parent; text: "M"; color: "#f0a840"; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.Bold }
            }
            ColumnLayout {
                id: col
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 6
                RowLayout {
                    spacing: 10
                    Text { text: "Mantel (Hearth Classic)"; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                    SetChip { text: "built in · using now"; kind: "ok" }
                }
                Text { Layout.fillWidth: true; text: "Hearth's own desktop with all of Hearth's apps: the bar, the dock, Quick Access, notifications and the controller."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
            }
        }
    }
    SetRow {
        title: "KDE Plasma and GNOME"
        sub: "Retired in Hearth 1.14: Mantel is the desktop. A KDE Plasma or GNOME pick from before opens Mantel, and Hearth tells you once."
        chip: "retired"; chipKind: "warn"
        interactive: false
    }
    Text { visible: page.msg !== ""; Layout.fillWidth: true; text: page.msg; color: page.bad ? SetStyle.warn : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
}
