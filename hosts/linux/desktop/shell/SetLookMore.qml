// Hearth Settings › Personalization › Window frames & sign-in splash (1.14, KDE's frame colour intensity and splash
// screen, in Hearth's look): how much of the accent colour the window frames take (frameTint, 0 = the look's own, as
// before; hearth-theme makes labwc's and KWin's frames from it), and the splash shown for a moment after you sign in
// (signinSplash: none, Hearth's flame, the logo, or a plain welcome). Saved in desktop.json only when you change them.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Window frames & sign-in splash"
    property int tint: Settings.frameTint
    // `settings page look-more:apps` opens it at "Apps follow Hearth" (the Desktop Mode test's picture); only scrolls.
    // The anchor stays until the timer: opening Settings can make the page twice in a row, and the second must see it.
    function reveal() { if (SettingsNav.pageAnchor === "apps") revealTimer.start(); }
    Timer { id: revealTimer; interval: 600; onTriggered: { if (SettingsNav.pageAnchor === "apps") SettingsNav.pageAnchor = ""; page.contentY = Math.max(0, Math.min(appsCard.mapToItem(page.contentItem, 0, 0).y - 16, page.contentHeight - page.height)); } }
    Component.onCompleted: reveal()
    function mix(a, b, t) { return Qt.rgba(a.r * (1 - t) + b.r * t, a.g * (1 - t) + b.g * t, a.b * (1 - t) + b.b * t, 1); }

    SetCardGroup {
        title: "Window frames"
        // Two windows, the one in front and one behind, in the colours the frames will have.
        SetPanel {
            implicitHeight: 210
            Repeater {
                model: [{ x: 0.08, y: 26, active: false, t: "Files" }, { x: 0.32, y: 52, active: true, t: "Text editor" }]
                delegate: Rectangle {
                    id: win
                    required property var modelData
                    readonly property color base: modelData.active ? Theme.panel3 : Theme.panel2
                    x: parent.width * modelData.x; y: modelData.y; width: parent.width * 0.5; height: 140; radius: 9
                    color: Theme.bg
                    border.width: 1
                    border.color: page.mix(Theme.line, Theme.accent, (modelData.active ? 0.7 : 0.35) * page.tint / 100)
                    Rectangle {
                        width: parent.width; height: 32; radius: 9
                        color: page.mix(parent.base, Theme.accent, (modelData.active ? 0.55 : 0.27) * page.tint / 100)
                        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 9; color: parent.color }
                        Text { x: 14; anchors.verticalCenter: parent.verticalCenter; text: modelData.t; color: modelData.active ? Theme.text : Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                        Row {
                            anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                            spacing: 8
                            Repeater { model: 3; delegate: Rectangle { required property int index; width: 11; height: 11; radius: 6; color: index === 2 && win.modelData.active ? Theme.danger : Theme.muted; opacity: 0.8 } }
                        }
                    }
                }
            }
        }
        SetSlider {
            icon: "palette"; title: "Accent colour in the frames"
            sub: "0% keeps the look's own frames. More tints the title bars and edges with your accent"
            value: page.tint / 100; step: 0.05
            valueText: page.tint === 0 ? "Off" : page.tint + "%"
            onMoved: v => { page.tint = Math.round(v * 20) * 5; tintApply.restart(); }
            Timer { id: tintApply; interval: 400; onTriggered: Settings.frameTint = page.tint }
        }
    }
    // KDE's "Apply colours to non-Qt applications" and Plasma's colour scheme, as one switch: Qt, KDE and GTK apps take
    // Hearth's colours (on, as before) or keep their own (theme/apply.sh writes Hearth's files without its palette).
    SetCardGroup {
        id: appsCard
        title: "Apps"
        SetSwitch {
            icon: "palette"; title: "Apps follow Hearth"
            sub: Settings.appsFollow ? "Qt, KDE and GTK apps use the look's colours and your accent" : "Qt, KDE and GTK apps keep their own colours (Breeze and Adwaita)"
            checked: Settings.appsFollow
            onToggled: v => Settings.appsFollow = v
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Apps already open change the next time they're opened. Hearth's own apps always follow the look."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: "After you sign in"
        GridLayout {
            Layout.fillWidth: true
            columns: 4; columnSpacing: 10; rowSpacing: 10
            Repeater {
                model: [{ v: "", t: "None", s: "Straight to the desktop" }, { v: "flame", t: "Hearth's flame", s: "The logo over a glowing fire" },
                        { v: "logo", t: "Logo", s: "Hearth's logo on dark" }, { v: "plain", t: "Plain welcome", s: "Your name in the look's colours" }]
                delegate: Rectangle {
                    id: card
                    required property var modelData
                    readonly property bool lit: Settings.signinSplash === modelData.v
                    Layout.fillWidth: true
                    implicitHeight: 190; radius: SetStyle.bigRadius
                    color: SetStyle.card
                    border.width: lit ? 2 : 1
                    border.color: lit ? Theme.accent : SetStyle.tint(Theme.text, 0.07)
                    activeFocusOnTab: true
                    Keys.onReturnPressed: Settings.signinSplash = modelData.v
                    Rectangle {
                        x: 10; y: 10; width: parent.width - 20; height: 110; radius: 8; clip: true
                        color: card.modelData.v === "plain" ? Theme.panel : card.modelData.v === "" ? Theme.bg : "#0d0a08"
                        Image { visible: card.modelData.v === "flame"; anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: -30 } width: parent.width; height: parent.height * 1.1; source: "images/ember-glow.png"; fillMode: Image.PreserveAspectFit }
                        Image { visible: card.modelData.v === "flame" || card.modelData.v === "logo"; anchors.centerIn: parent; width: 40; height: 40; source: "images/hearth-logo.png"; fillMode: Image.PreserveAspectFit }
                        Text { visible: card.modelData.v === "plain"; anchors.centerIn: parent; text: "Welcome, " + (Quickshell.env("USER") || "you"); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                        Icon { visible: card.modelData.v === ""; anchors.centerIn: parent; name: "desktops"; size: 30; color: Theme.muted }
                    }
                    Text { x: 14; y: 130; text: card.modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Text { x: 14; y: 150; width: parent.width - 28; text: card.modelData.s; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Settings.signinSplash = card.modelData.v }
                }
            }
        }
        SetRow {
            visible: Settings.signinSplash !== ""
            icon: "play"; chevron: false; title: "Show it now"; sub: "A preview: clicks go straight through it"
            onActivated: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "splash", "preview", Settings.signinSplash])
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "It shows for two seconds while the desktop comes up and never holds anything up. The boot splash (before signing in) is in System › Boot splash."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
}
