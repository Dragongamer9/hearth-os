// Hearth's own login screen (Quickshell). greetd starts it in cage as the greeter's account (hearth-greeter); cage
// shows the one window full screen. `hearth-login-screen preview` shows it in a window in your session instead, with
// signing in off. Files: Login.qml (the screen), Chip.qml, Avatar.qml, Nav.qml (arrows and the controller), Theme.qml,
// GreeterOsk.qml (Mantel's on-screen keyboard, copied in with OskBoard.qml, OskKey.qml and osk/ when the image is
// built), greeter-info (accounts, sessions, battery, Wi-Fi; read only).
import QtQuick
import Quickshell

ShellRoot {
    FloatingWindow {
        id: win
        title: "Hearth login"
        color: Theme.bg
        implicitWidth: 1280
        implicitHeight: 800
        Login { anchors.fill: parent }
    }
}
