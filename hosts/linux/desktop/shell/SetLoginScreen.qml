// Hearth Settings › Accounts › Login screen: the picture behind Hearth's login screen (SDDM's Hearth theme) and a way
// to Sign in automatically (its one home is Sign-in options). The picture is copied for the login screen through
// hearth-admin login-background when you pick one; Hearth's fireside is the default and the fallback, so the login
// screen always comes up even if the picture can't be read.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Login screen"
    property var st: null              // hearth-loginscreen status
    property string message: ""
    property bool messageBad: false
    property int bust: 0               // reloads the preview after a change

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => { page.st = r; page.bust++; }, e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-loginscreen" }
    function use(path) {
        say("");
        job.run(["set", path], r => { page.st = r; page.bust++; page.say(path ? "The login screen shows your picture now." : "Back to Hearth's fireside."); }, e => page.say(e, true), "", "set");
    }
    Process {
        id: picker
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Login screen picture' --getopenfilename \"$HOME/Pictures\" 'image/jpeg image/png'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Login screen picture' --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.jpg *.jpeg *.png'; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f) page.use(f); } }
        onExited: code => { if (code === 3) page.say("No file picker here: put the picture in Pictures and pick it below.", true); }
    }

    SetCardGroup {
        title: "Picture"
        // The preview: your picture darkened as the login screen shows it, or the fireside.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(320, width * 9 / 16)
            radius: 12; clip: true
            gradient: Gradient {
                GradientStop { position: 0.0; color: "#17110e" }
                GradientStop { position: 0.7; color: "#0f0b09" }
                GradientStop { position: 1.0; color: "#2a140a" }
            }
            Image {
                anchors.fill: parent
                visible: !!page.st && page.st.background !== ""
                source: page.st && page.st.background ? "file://" + page.st.background + "?" + page.bust : ""
                fillMode: Image.PreserveAspectCrop; asynchronous: true; cache: false
            }
            Rectangle { anchors.fill: parent; visible: !!page.st && page.st.background !== ""; color: "#000000"; opacity: 0.45 }
            Column {
                anchors.centerIn: parent
                spacing: 4
                Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatTime(new Date(), Settings.clock24 ? "HH:mm" : "h:mm"); color: "#f4ece3"; font.family: Theme.font; font.pixelSize: 44; font.weight: Font.DemiBold }
                Text { anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatDate(new Date(), "dddd, MMMM d"); color: "#a69484"; font.family: Theme.font; font.pixelSize: 14 }
            }
        }
        Flow {
            Layout.fillWidth: true
            spacing: 10
            visible: !!page.st && page.st.pictures.length > 0
            Repeater {
                model: page.st ? page.st.pictures : []
                delegate: Rectangle {
                    id: th
                    required property string modelData
                    width: 150; height: 86; radius: 8; clip: true
                    color: Theme.panel2 || "#261d18"
                    border.width: activeFocus || hov.containsMouse ? 2 : 0
                    border.color: Theme.accent
                    activeFocusOnTab: true
                    Keys.onReturnPressed: page.use(modelData)
                    Keys.onSpacePressed: page.use(modelData)
                    Image { anchors.fill: parent; anchors.margins: 2; source: "file://" + th.modelData; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 300 }
                    MouseArea { id: hov; anchors.fill: parent; hoverEnabled: true; onClicked: { th.forceActiveFocus(); page.use(th.modelData); } }
                }
            }
        }
        SetRow { icon: "image"; title: job.what === "set" ? "Saving…" : "Choose a picture…"; sub: "Any PNG or JPEG of yours"; onActivated: picker.running = true }
        SetRow {
            visible: !!page.st && page.st.background !== ""
            icon: "undo"; chevron: false; title: "Use Hearth's fireside"; sub: "The login screen's own look"
            onActivated: page.use("")
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: "Signing in"
        SetRow {
            icon: "lock"; chevron: true; title: "Sign in automatically"
            sub: page.st ? (page.st.autologin ? "On: the login screen is skipped when this PC starts" : "Off: the login screen shows when this PC starts") + " · in Sign-in options" : ""
            onActivated: SettingsNav.openPage("signin")
        }
    }
}
