// Hearth's own password prompt (1.14): Mantel is Desktop Mode's polkit agent, in place of KDE's, GNOME's or MATE's.
// When an app asks for administrator rights (pkexec, Bazaar's system installs, Disks, Hearth Text's Edit as
// administrator), polkit asks this agent, and this draws the prompt in Hearth's style: who's asking and what for, the
// person whose password it needs (their picture), the password field, Allow / Cancel. Controller friendly: the D-pad
// moves, A presses, B cancels, Y opens the on-screen keyboard (Pad.qml types keys while it's open).
//
// The password goes from the field straight to polkit (AuthFlow.submit) and the field is emptied: it's never logged,
// stored or sent anywhere else.
//
// RED FLAG: password prompts must always work. shell.qml loads this file by name, so if Quickshell has no Polkit
// module, or this file has a problem, only this is lost. While this agent is registered it says so in
// $XDG_RUNTIME_DIR/hearth-polkit-mantel (quickshell's PID); hearth-polkit-fallback (started by Desktop Mode's
// autostart) waits a few seconds for that, and starts the classic agent (polkit-gnome / -kde / -mate, lxpolkit) when
// it isn't there, or when the shell stops (a crash, a restart). HEARTH_POLKIT=classic in the session leaves this off.
//   quickshell ipc -p <shell folder> call polkit state | demo | close     (demo: the prompt with made-up details, for
//   the screenshots; nothing is asked of polkit)
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Polkit
import "polkit-actions.js" as Act

Scope {
    id: pk
    property bool demo: false
    readonly property var flow: agent.flow
    readonly property bool active: demo || (agent.isActive && flow !== null)
    readonly property string message: demo ? "Authentication is needed to run `/usr/bin/hearth-admin update-status' as the super user" : flow ? flow.message : ""
    readonly property string actionId: demo ? "org.freedesktop.policykit.exec" : flow ? flow.actionId : ""
    readonly property string iconName: demo ? "" : flow ? flow.iconName : ""
    readonly property var identities: demo ? [{ id: 1000, string: "unix-user:" + (Quickshell.env("USER") || "dragon"), displayName: "", isGroup: false }] : flow ? flow.identities : []
    readonly property var selected: demo ? identities[0] : flow ? flow.selectedIdentity : null
    readonly property string inputPrompt: demo ? "Password:" : flow ? flow.inputPrompt : ""
    readonly property bool visibleResponse: demo ? false : flow ? flow.responseVisible : false
    readonly property bool needsAnswer: demo ? true : flow ? flow.isResponseRequired : false
    readonly property string extra: demo ? "" : flow ? flow.supplementaryMessage : ""
    readonly property bool extraIsError: demo ? false : flow ? flow.supplementaryIsError : false
    readonly property var about: Act.describe(actionId)
    readonly property string prog: Act.program(message)
    property bool busy: false
    property string wrong: ""
    property int shakes: 0

    onActiveChanged: {
        busy = false; wrong = "";
        if (!active && SettingsNav.pad) SettingsNav.pad.closeKeyboard();
    }
    Binding { target: SettingsNav; property: "polkitShown"; value: pk.active }

    PolkitAgent { id: agent }

    // Registered: the fallback watcher (hearth-polkit-fallback) sees quickshell's PID here and leaves the classic agent
    // off; gone (or a PID that isn't running) and it starts the classic one.
    readonly property string marker: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-polkit-mantel"
    Process { id: mark }
    function writeMarker() {
        mark.running = false;
        mark.command = agent.isRegistered
            ? ["sh", "-c", "umask 077; echo \"$PPID\" > \"$0.tmp\" && mv -f \"$0.tmp\" \"$0\"", pk.marker]
            : ["rm", "-f", pk.marker];
        mark.running = true;
    }
    Connections {
        target: agent
        function onIsRegisteredChanged() { pk.writeMarker(); }
    }
    Component.onCompleted: { if (agent.isRegistered) writeMarker(); noRegister.start(); }
    Timer {
        id: noRegister; interval: 6000
        // Not registered after a while: a marker left by an earlier agent in this same shell is taken away.
        onTriggered: if (!agent.isRegistered) { pk.writeMarker(); console.info("Mantel's password prompt isn't registered (another agent has this session, or polkit isn't reachable); the classic agent asks"); }
    }
    Connections {
        target: pk.flow
        ignoreUnknownSignals: true
        function onIsResponseRequiredChanged() { if (pk.flow && pk.flow.isResponseRequired) { pk.busy = false; pk.clearField(); } }
        function onAuthenticationFailed() { pk.busy = false; pk.wrong = "That password isn't right. Try again."; pk.shakes++; pk.clearField(); }
    }

    // The field lives in the prompt's window: it's emptied (and focused) from there when this asks.
    signal clearField()
    // V: what was typed, handed straight to polkit; the window empties the field as it calls this.
    function submit(v) {
        clearField();
        if (!needsAnswer || busy) return;
        wrong = "";
        if (demo) { wrong = "That password isn't right. Try again."; shakes++; return; }
        busy = true;
        flow.submit(v);
    }
    function cancel() {
        clearField();
        if (demo) { demo = false; return; }
        if (flow) flow.cancelAuthenticationRequest();
    }
    function choose(i) {
        if (demo || !flow || i < 0 || i >= identities.length) return;
        flow.selectedIdentity = identities[i];
    }

    IpcHandler {
        target: "polkit"
        function state(): string { return JSON.stringify({ registered: agent.isRegistered, active: pk.active, demo: pk.demo, action: pk.actionId, busy: pk.busy }); }
        function demo(): void { pk.demo = true; }
        function demoWrong(): void { pk.demo = true; pk.wrong = "That password isn't right. Try again."; }
        function close(): void { pk.cancel(); }
    }

    LazyLoader {
        active: pk.active
        PanelWindow {
            id: win
            screen: Quickshell.screens[0]
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-password"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            color: "#b3060809"
            Component.onCompleted: focusLater.start()
            Timer { id: focusLater; interval: 50; onTriggered: field.forceActiveFocus() }
            Connections { target: pk; function onClearField() { field.text = ""; field.forceActiveFocus(); } }
            MouseArea { anchors.fill: parent }

            Rectangle {
                id: card
                anchors.horizontalCenter: parent.horizontalCenter
                y: Math.max(24, parent.height * 0.14)
                width: Math.min(parent.width - 32, 620)
                height: col.implicitHeight + 56
                radius: Math.max(10, Theme.r(22))
                color: Theme.panel
                border.color: Theme.line
                property real shake: 0
                transform: Translate { x: card.shake }
                SequentialAnimation {
                    id: shakeAnim
                    NumberAnimation { target: card; property: "shake"; to: -14; duration: 50 }
                    NumberAnimation { target: card; property: "shake"; to: 12; duration: 70 }
                    NumberAnimation { target: card; property: "shake"; to: -7; duration: 60 }
                    NumberAnimation { target: card; property: "shake"; to: 0; duration: 50 }
                }
                Connections { target: pk; function onShakesChanged() { shakeAnim.restart(); } }
                // A thin accent line along the top: Hearth's "this is the system asking".
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 24; rightMargin: 24 } height: 3; radius: 2; color: Theme.accent }

                ColumnLayout {
                    id: col
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
                    spacing: 16

                    RowLayout {
                        spacing: 14
                        Rectangle {
                            implicitWidth: 52; implicitHeight: 52; radius: 14
                            color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
                            Image {
                                id: appIcon
                                anchors.centerIn: parent; width: 30; height: 30
                                source: pk.iconName !== "" ? Quickshell.iconPath(pk.iconName, true) : ""
                                visible: status === Image.Ready
                                sourceSize: Qt.size(60, 60)
                            }
                            Icon { anchors.centerIn: parent; visible: !appIcon.visible; name: "shield"; size: 28; color: Theme.accent }
                        }
                        ColumnLayout {
                            spacing: 3
                            Text { text: pk.about.who + (pk.prog !== "" && pk.about.who === "An app" ? " (" + pk.prog + ")" : "") + " is asking"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.Medium }
                            Text { text: pk.about.what; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                        }
                    }
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: pk.message
                        color: Theme.text; opacity: 0.85; font.family: Theme.font; font.pixelSize: 14
                    }

                    // Whose password: their picture and name (pick another when polkit offers more than one).
                    Flow {
                        Layout.fillWidth: true
                        spacing: 10
                        Repeater {
                            model: pk.identities
                            Rectangle {
                                id: who
                                required property var modelData
                                required property int index
                                readonly property bool chosen: pk.selected === modelData || (pk.demo && index === 0)
                                readonly property string uname: Act.userName(modelData)
                                readonly property string label: Act.personLabel(modelData)
                                width: whoRow.implicitWidth + 26; height: 64; radius: 32
                                color: chosen ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.14) : Theme.panel2
                                border.width: chosen ? 2 : 1; border.color: chosen ? Theme.accent : Theme.line
                                Row {
                                    id: whoRow
                                    anchors { left: parent.left; leftMargin: 8; verticalCenter: parent.verticalCenter }
                                    spacing: 12
                                    Rectangle {
                                        width: 48; height: 48; radius: 24; clip: true
                                        color: Theme.accent
                                        Text { anchors.centerIn: parent; visible: face.status !== Image.Ready; text: Act.initial(who.label); color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                                        Image {
                                            id: face
                                            anchors.fill: parent
                                            fillMode: Image.PreserveAspectCrop
                                            sourceSize: Qt.size(96, 96)
                                            property int tries: 0
                                            // The account's picture: AccountsService's (Settings › Sign-in), then ~/.face.
                                            source: who.uname === "" ? "" : tries === 0 ? "file:///var/lib/AccountsService/icons/" + who.uname
                                                : (tries === 1 && who.uname === Quickshell.env("USER")) ? "file://" + Quickshell.env("HOME") + "/.face" : ""
                                            onStatusChanged: if (status === Image.Error && tries < 2) tries++
                                        }
                                    }
                                    Column {
                                        anchors.verticalCenter: parent.verticalCenter
                                        Text { text: who.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                                        Text { text: who.modelData.isGroup ? "Any administrator" : "Administrator"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                    }
                                }
                                MouseArea { anchors.fill: parent; onClicked: pk.choose(who.index) }
                            }
                        }
                    }

                    // The password.
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Text { text: (pk.inputPrompt || "Password").replace(/:\s*$/, ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 50; radius: 10
                            color: Theme.bg2
                            border.width: field.activeFocus ? 2 : 1; border.color: pk.wrong !== "" ? Theme.danger : field.activeFocus ? Theme.accent : Theme.line
                            TextInput {
                                id: field
                                anchors { left: parent.left; right: showBtn.left; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 8 }
                                echoMode: pk.visibleResponse || showBtn.reveal ? TextInput.Normal : TextInput.Password
                                passwordCharacter: "•"
                                enabled: !pk.busy
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 18
                                clip: true
                                inputMethodHints: Qt.ImhHiddenText | Qt.ImhSensitiveData | Qt.ImhNoPredictiveText
                                onAccepted: pk.submit(field.text)
                                Keys.onEscapePressed: pk.cancel()
                                Keys.onDownPressed: allowBtn.forceActiveFocus()
                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    visible: field.text === "" && !pk.busy
                                    text: "Type your password  ·  Y opens the keyboard"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                                }
                            }
                            CaptureBtn {
                                id: showBtn
                                property bool reveal: false
                                anchors { right: parent.right; rightMargin: 6; verticalCenter: parent.verticalCenter }
                                visible: !pk.visibleResponse
                                icon: "eye"; tip: reveal ? "Hide the password" : "Show the password"; selected: reveal
                                arrowsMove: false
                                onActivated: { reveal = !reveal; field.forceActiveFocus(); }
                            }
                        }
                        Text {
                            visible: pk.wrong !== "" || pk.extra !== "" || pk.busy
                            Layout.fillWidth: true; wrapMode: Text.WordWrap
                            text: pk.busy ? "Checking…" : pk.wrong !== "" ? pk.wrong : pk.extra
                            color: pk.busy ? Theme.muted : (pk.wrong !== "" || pk.extraIsError) ? Theme.danger : Theme.muted
                            font.family: Theme.font; font.pixelSize: 13
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10
                        CaptureBtn { icon: "kbd"; text: "Keyboard"; onActivated: { field.forceActiveFocus(); if (SettingsNav.pad) SettingsNav.pad.openKeyboard(); } }
                        Item { Layout.fillWidth: true }
                        CaptureBtn { id: cancelBtn; text: "Cancel"; onActivated: pk.cancel(); }
                        CaptureBtn { id: allowBtn; primary: true; icon: "lock"; text: "Allow"; enabledLook: !pk.busy; onActivated: pk.submit(field.text); }
                    }
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: "A allow  ·  B cancel  ·  Y keyboard" + (pk.actionId !== "" ? "      " + pk.actionId : "")
                        color: Theme.muted; opacity: 0.8; font.family: Theme.font; font.pixelSize: 11
                    }
                }
                Keys.onEscapePressed: pk.cancel()
            }
        }
    }
}
