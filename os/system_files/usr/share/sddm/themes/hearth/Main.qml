// Hearth OS's login screen (an SDDM theme), made to match Hearth's lock screen (Lock.qml): the fireside
// glow, a big clock, who's signing in, the password, and an on-screen keyboard you can use with the D-pad
// and A (hearth-greeter-pad turns the controller into keys while this screen is up). Sleep, Restart and
// Shut down along the bottom, and which session to start (Game Mode or the desktop).
// Shown only when "Sign in automatically" is off (Hearth Settings › Sign-in).
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root
    width: 1280; height: 800
    color: "#0f0b09"

    // Hearth's palette
    readonly property color bg2: "#17110e"
    readonly property color panel2: "#261d18"
    readonly property color panel3: "#33271f"
    readonly property color line: "#3b2e26"
    readonly property color text: "#f4ece3"
    readonly property color muted: "#a69484"
    readonly property color danger: "#ff5d5d"
    readonly property color accent: config.accent || "#ff8a3d"
    readonly property color accentInk: "#2a1206"
    readonly property string font: "sans-serif"

    property int userIndex: Math.max(0, userModel.lastIndex)
    property int sessionIndex: Math.max(0, sessionModel.lastIndex)
    property string typed: ""
    property string message: ""
    property bool busy: false
    readonly property string userName: userModel.data(userModel.index(userIndex, 0), Qt.UserRole + 1) || ""
    readonly property string realName: userModel.data(userModel.index(userIndex, 0), Qt.UserRole + 2) || ""
    // Focus: 0 user, 1 keyboard, 2 power row
    property int zone: 1

    function login() {
        if (busy) return;
        busy = true; message = "";
        sddm.login(userName, typed, sessionIndex);
    }
    Connections {
        target: sddm
        function onLoginFailed() { root.busy = false; root.typed = ""; root.message = "That password isn't right"; }
        function onLoginSucceeded() { root.busy = false; }
    }

    // ---------------------------------------------------------------- the fireside
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#17110e" }
            GradientStop { position: 0.7; color: "#0f0b09" }
            GradientStop { position: 1.0; color: "#2a140a" }
        }
    }
    Image {
        id: glow
        anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom
        width: parent.width * 1.2; height: parent.height * 0.6
        source: "ember-glow.png"; opacity: 0.5
        SequentialAnimation on opacity {
            loops: Animation.Infinite
            NumberAnimation { to: 0.6; duration: 3600; easing.type: Easing.InOutSine }
            NumberAnimation { to: 0.45; duration: 4200; easing.type: Easing.InOutSine }
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 760)
        spacing: 14

        Text {
            id: clock
            Layout.alignment: Qt.AlignHCenter
            color: root.text; font.family: root.font; font.pixelSize: 84; font.weight: Font.DemiBold
            Timer { interval: 5000; running: true; repeat: true; triggeredOnStart: true
                onTriggered: { clock.text = Qt.formatTime(new Date(), "h:mm"); date.text = Qt.formatDate(new Date(), "dddd, MMMM d"); } }
        }
        Text { id: date; Layout.alignment: Qt.AlignHCenter; color: root.muted; font.family: root.font; font.pixelSize: 20 }

        // Who's signing in (more than one account: left/right, or click the arrows)
        Rectangle {
            Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 18
            Layout.preferredWidth: 420; Layout.preferredHeight: 56; radius: 14
            color: root.zone === 0 ? root.panel3 : "transparent"
            border.width: 2; border.color: root.zone === 0 ? root.accent : "transparent"
            RowLayout {
                anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                Text { visible: userModel.count > 1; text: "‹"; color: root.muted; font.pixelSize: 26
                    MouseArea { anchors.fill: parent; anchors.margins: -10; onClicked: root.userIndex = (root.userIndex + userModel.count - 1) % userModel.count } }
                Item { Layout.fillWidth: true }
                Rectangle {
                    visible: (root.realName || root.userName) !== ""
                    width: 36; height: 36; radius: 18; color: root.accent
                    Text { anchors.centerIn: parent; text: (root.realName || root.userName).charAt(0).toUpperCase(); color: root.accentInk; font.family: root.font; font.pixelSize: 17; font.weight: Font.Bold }
                }
                Text { text: root.realName || root.userName || "Hearth"; color: root.text; font.family: root.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
                Text { visible: userModel.count > 1; text: "›"; color: root.muted; font.pixelSize: 26
                    MouseArea { anchors.fill: parent; anchors.margins: -10; onClicked: root.userIndex = (root.userIndex + 1) % userModel.count } }
            }
        }

        // Password
        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 420; Layout.preferredHeight: 56; radius: 14
            color: root.panel2; border.color: root.message ? root.danger : root.line; border.width: 2
            Text {
                anchors.centerIn: parent
                text: root.busy ? "Signing in…" : root.typed.length ? "•".repeat(root.typed.length) : "Password"
                color: root.typed.length ? root.text : root.muted; font.family: root.font; font.pixelSize: 22
            }
        }
        Text { Layout.alignment: Qt.AlignHCenter; visible: root.message !== ""; text: root.message; color: root.danger; font.family: root.font; font.pixelSize: 14 }

        // The keyboard: tap, or arrows / D-pad and Enter (A)
        Item {
            id: kb
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(root.width - 80, 760)
            Layout.preferredHeight: 4 * 54
            focus: true
            property bool upper: false
            property bool sym: false
            property int r: 1
            property int c: 0
            property bool navigating: false
            readonly property var letters: [
                ["1","2","3","4","5","6","7","8","9","0"],
                ["q","w","e","r","t","y","u","i","o","p"],
                ["a","s","d","f","g","h","j","k","l","⌫"],
                ["⇧","z","x","c","v","b","n","m","#+=","Sign in"]]
            readonly property var symbols: [
                ["!","@","#","$","%","^","&","*","(",")"],
                ["-","_","=","+","[","]","{","}","\\","|"],
                [";",":","'","\"",",",".","<",">","/","⌫"],
                ["⇧","?","~","`"," ","€","£","¥","abc","Sign in"]]
            readonly property var rows: sym ? symbols : letters
            function label(k) { return (upper && k.length === 1 && /[a-z]/.test(k)) ? k.toUpperCase() : (k === " " ? "Space" : k); }
            function press(k) {
                if (k === "⌫") root.typed = root.typed.slice(0, -1);
                else if (k === "⇧") upper = !upper;
                else if (k === "#+=") sym = true;
                else if (k === "abc") sym = false;
                else if (k === "Sign in") root.login();
                else { root.typed += label(k) === "Space" ? " " : label(k); if (upper) upper = false; root.message = ""; }
            }
            Keys.onPressed: e => {
                const arrow = e.key === Qt.Key_Left || e.key === Qt.Key_Right || e.key === Qt.Key_Up || e.key === Qt.Key_Down;
                if (root.zone === 0) {   // the account row
                    if (e.key === Qt.Key_Left && userModel.count > 1) root.userIndex = (root.userIndex + userModel.count - 1) % userModel.count;
                    else if (e.key === Qt.Key_Right && userModel.count > 1) root.userIndex = (root.userIndex + 1) % userModel.count;
                    else if (e.key === Qt.Key_Down || e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { root.zone = 1; navigating = true; r = 0; }
                    e.accepted = true; return;
                }
                if (root.zone === 2) {   // power row
                    if (e.key === Qt.Key_Left) power.sel = Math.max(0, power.sel - 1);
                    else if (e.key === Qt.Key_Right) power.sel = Math.min(power.count - 1, power.sel + 1);
                    else if (e.key === Qt.Key_Up) { root.zone = 1; r = rows.length - 1; }
                    else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) power.activate(power.sel);
                    e.accepted = true; return;
                }
                if (e.key === Qt.Key_Left) c = Math.max(0, c - 1);
                else if (e.key === Qt.Key_Right) c = Math.min(rows[r].length - 1, c + 1);
                else if (e.key === Qt.Key_Up) { if (navigating && r === 0) root.zone = userModel.count > 1 ? 0 : 1; else r = Math.max(0, r - 1); }
                else if (e.key === Qt.Key_Down) { if (navigating && r === rows.length - 1) { root.zone = 2; power.sel = 0; } else r = Math.min(rows.length - 1, r + 1); }
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { if (navigating) press(rows[r][c]); else root.login(); }
                else if (e.key === Qt.Key_Backspace) root.typed = root.typed.slice(0, -1);
                else if (e.key === Qt.Key_Escape) root.typed = "";
                else if (e.text && e.text.length === 1 && e.text >= " ") { root.typed += e.text; root.message = ""; }   // a real keyboard
                else return;
                if (arrow) navigating = true;
                else if (e.key !== Qt.Key_Return && e.key !== Qt.Key_Enter) navigating = false;
                e.accepted = true;
            }
            Column {
                anchors.fill: parent
                spacing: 6
                Repeater {
                    model: kb.rows
                    delegate: Row {
                        id: kline
                        required property var modelData
                        required property int index
                        spacing: 6
                        Repeater {
                            model: kline.modelData
                            delegate: Rectangle {
                                id: key
                                required property string modelData
                                required property int index
                                readonly property bool lit: root.zone === 1 && kb.navigating && kb.r === kline.index && kb.c === index
                                width: (kb.width - 9 * 6) / 10; height: 48; radius: 10
                                color: modelData === "Sign in" ? root.accent : (modelData === "⇧" && kb.upper) ? root.panel3 : tap.pressed ? root.panel3 : root.panel2
                                border.width: 2; border.color: lit ? (modelData === "Sign in" ? root.text : root.accent) : "transparent"
                                Text {
                                    anchors.centerIn: parent; text: kb.label(key.modelData)
                                    color: key.modelData === "Sign in" ? root.accentInk : root.text
                                    font.family: root.font; font.pixelSize: key.modelData.length > 1 ? 15 : 20; font.weight: Font.Medium
                                }
                                MouseArea { id: tap; anchors.fill: parent; onClicked: { root.zone = 1; kb.r = kline.index; kb.c = key.index; kb.press(key.modelData); kb.forceActiveFocus(); } }
                            }
                        }
                    }
                }
            }
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "Type your password, or pick keys with the D-pad and A"
            color: root.muted; font.family: root.font; font.pixelSize: 13
        }
    }

    // ---------------------------------------------------------------- session and power, along the bottom
    RowLayout {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 24 }
        spacing: 10
        // Which session (Hearth's Game Mode, the desktop, …): click to switch
        Rectangle {
            visible: sessionModel.rowCount() > 1
            implicitWidth: sess.implicitWidth + 32; implicitHeight: 44; radius: 12
            color: sessArea.containsMouse ? root.panel3 : root.panel2; border.color: root.line
            Text { id: sess; anchors.centerIn: parent; color: root.text; font.family: root.font; font.pixelSize: 14
                text: "Start: " + (sessionModel.data(sessionModel.index(root.sessionIndex, 0), Qt.UserRole + 4) || "Hearth") }
            MouseArea { id: sessArea; anchors.fill: parent; hoverEnabled: true; onClicked: root.sessionIndex = (root.sessionIndex + 1) % sessionModel.rowCount() }
        }
        // The desktops: Hearth Classic, Hearth Plasma and Hearth GNOME are sessions above (the one you pick is saved).
        Item { Layout.fillWidth: true }
        Repeater {
            id: power
            property int sel: 0
            function activate(i) { if (i === 0) sddm.suspend(); else if (i === 1) sddm.reboot(); else sddm.powerOff(); }
            model: [{ label: "Sleep", ok: sddm.canSuspend }, { label: "Restart", ok: sddm.canReboot }, { label: "Shut down", ok: sddm.canPowerOff }]
            delegate: Rectangle {
                required property var modelData
                required property int index
                readonly property bool lit: root.zone === 2 && power.sel === index
                visible: modelData.ok
                implicitWidth: pl.implicitWidth + 32; implicitHeight: 44; radius: 12
                color: lit ? root.accent : pArea.containsMouse ? root.panel3 : root.panel2
                border.color: root.line
                Text { id: pl; anchors.centerIn: parent; text: parent.modelData.label; color: parent.lit ? root.accentInk : root.text; font.family: root.font; font.pixelSize: 14 }
                MouseArea { id: pArea; anchors.fill: parent; hoverEnabled: true; onClicked: power.activate(parent.index) }
            }
        }
    }

    Component.onCompleted: kb.forceActiveFocus()
}
