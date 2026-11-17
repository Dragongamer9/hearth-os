// Hearth's own login screen (1.14, a test behind Settings › Accounts › Sign-in options; hearth-login-screen). The
// fireside of Hearth's SDDM theme, with: who's signing in (their pictures), the password with show / hide and a Caps
// Lock warning, which session to start (Hearth, Game Mode, Desktop Mode, and the others Hearth ships), Sleep / Restart /
// Shut down, the clock, battery and Wi-Fi, the keyboard layout, Bigger text, High contrast and the screen reader when
// there is one, wrong-password and account-locked messages, the on-screen keyboard (Mantel's), and the controller.
// greetd does the signing in (PAM): the password goes straight to it and is never written anywhere or logged.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Greetd

Item {
    id: root
    focus: true
    readonly property bool testMode: Quickshell.env("HEARTH_GREETER_TEST") === "1"
    readonly property bool preview: Quickshell.env("HEARTH_GREETER_PREVIEW") === "1" || !Greetd.available
    readonly property string here: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, ""))
    readonly property string assets: Quickshell.env("HEARTH_GREETER_ASSETS") || "/usr/share/sddm/themes/hearth"
    readonly property string runDir: Quickshell.env("XDG_RUNTIME_DIR") || "/tmp"
    readonly property string statePath: Quickshell.env("HEARTH_GREETER_STATE") || "/var/lib/hearth-greeter/state.json"
    readonly property string sessionStart: "/usr/libexec/nimbus/hearth-session-start"

    // ---------------------------------------------------------------- what's on screen
    property var users: []
    property var sessions: []
    property var layouts: ["us"]
    property bool screenReaderThere: false
    property var status: ({ battery: { present: false }, wifi: { present: false } })
    property int userIndex: 0
    property string view: "users"          // users | password
    property string menu: ""               // "" | session | power | layout | access
    property bool oskOpen: false
    property string password: ""
    property bool showPassword: false
    property bool caps: false
    property bool busy: false
    property string message: ""
    property string messageKind: ""        // wrong | locked | info | error
    property string prompt: ""             // a second question from PAM (a code, a new password)
    property bool answering: false
    property var pendingSecret: null
    property string sessionId: "hearth"
    property bool screenReader: false
    property var saved: ({})
    property string lastPower: ""

    readonly property var user: users.length ? users[Math.min(userIndex, users.length - 1)] : null
    readonly property var session: sessions.find(s => s.id === sessionId) || sessions[0] || null

    function say(text, kind) { message = text; messageKind = kind || "info"; }

    // ---------------------------------------------------------------- read: accounts, sessions, layouts, battery, Wi-Fi
    Process {
        id: info
        command: ["python3", root.here + "greeter-info", "info"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const d = JSON.parse(text);
                    root.users = d.users || [];
                    root.sessions = d.sessions || [];
                    root.layouts = (d.layouts && d.layouts.length) ? d.layouts : ["us"];
                    root.screenReaderThere = !!d.screenReader;
                } catch (e) { console.warn("greeter: couldn't read the accounts: " + e); }
                root.restore();
            }
        }
    }
    Process {
        id: statusProc
        command: ["python3", root.here + "greeter-info", "status"]
        stdout: StdioCollector { onStreamFinished: { try { root.status = JSON.parse(text); } catch (e) {} } }
    }
    Timer { interval: 30000; repeat: true; running: true; triggeredOnStart: true; onTriggered: statusProc.running = true }
    Process {
        id: capsProc
        command: ["python3", root.here + "greeter-info", "caps"]
        stdout: StdioCollector { onStreamFinished: { try { root.caps = !!JSON.parse(text).caps; } catch (e) {} } }
    }
    Timer { id: capsLater; interval: 120; onTriggered: if (!root.testMode) capsProc.running = true }

    // The last account and session (and Bigger text): the greeter's own state, never a setting of yours.
    FileView {
        id: stateFile
        path: root.statePath
        blockLoading: true
        printErrors: false
        onLoaded: { try { root.saved = JSON.parse(text()) || {}; } catch (e) { root.saved = {}; } root.restore(); }
    }
    property bool restored: false
    function restore() {
        if (restored || !users.length || !sessions.length) return;
        restored = true;
        const i = users.findIndex(u => u.name === saved.lastUser);
        userIndex = i >= 0 ? i : 0;
        Theme.scale = saved.bigText ? 1.3 : 1.0;
        Theme.contrast = !!saved.contrast;
        pickSessionFor(user);
        if (users.length === 1) view = "password";
        Qt.callLater(() => Nav.ensure(root));
    }
    function pickSessionFor(u) {
        const per = (saved.sessions || {})[u ? u.name : ""];
        sessionId = sessions.find(s => s.id === per) ? per : (sessions[0] ? sessions[0].id : "hearth");
    }
    function remember() {
        const s = Object.assign({}, saved);
        s.lastUser = user ? user.name : "";
        s.sessions = Object.assign({}, s.sessions || {});
        if (user) s.sessions[user.name] = sessionId;
        s.bigText = Theme.scale > 1.01;
        s.contrast = Theme.contrast;
        saved = s;
        stateFile.setText(JSON.stringify(s));
    }

    // ---------------------------------------------------------------- signing in (greetd)
    function choose(i) {
        if (busy) return;
        if (Greetd.state !== GreetdState.Inactive) Greetd.cancelSession();
        userIndex = i; password = ""; prompt = ""; answering = false; say("");
        pickSessionFor(user);
        view = "password";
        Nav.current = null; Qt.callLater(() => Nav.ensure(root));
    }
    function backToUsers() {
        if (users.length < 2 || busy) return;
        if (Greetd.state !== GreetdState.Inactive) Greetd.cancelSession();
        password = ""; prompt = ""; answering = false; say("");
        view = "users"; closeOsk();
        Nav.current = null; Qt.callLater(() => Nav.ensure(root));
    }
    function signIn() {
        if (busy || !user) return;
        if (preview) { say("This is a preview: signing in is off here.", "info"); return; }
        if (answering) {            // PAM asked something more (a code): this answers it
            answering = false; busy = true;
            Greetd.respond(password); password = "";
            return;
        }
        busy = true; say("");
        pendingSecret = password;
        password = "";
        Greetd.createSession(user.name);
    }
    function friendly(msg) {
        if (/lock|faillock|too many|denied/i.test(msg)) {
            const m = /(\d+)\s*minute/i.exec(msg);
            return { kind: "locked", text: "This account is locked for now after too many wrong passwords." + (m ? " Try again in " + m[1] + " minute" + (m[1] === "1" ? "" : "s") + "." : " Try again in a few minutes.") };
        }
        if (/expired/i.test(msg)) return { kind: "error", text: "The password has expired: an administrator needs to set a new one." };
        return null;
    }
    Connections {
        target: Greetd
        function onAuthMessage(message, error, responseRequired, echoResponse) {
            if (responseRequired) {
                if (!echoResponse && root.pendingSecret !== null) {
                    const s = root.pendingSecret; root.pendingSecret = null;
                    Greetd.respond(s);
                } else {
                    root.busy = false; root.answering = true; root.prompt = message; root.showPassword = echoResponse;
                }
            } else if (error) {
                const f = root.friendly(message);
                root.say(f ? f.text : message, f ? f.kind : "error");
            } else if (message) {
                root.say(message, "info");
            }
        }
        function onAuthFailure(message) {
            root.busy = false; root.pendingSecret = null; root.password = ""; root.answering = false; root.prompt = "";
            const f = root.friendly(message);
            if (root.messageKind === "locked") return;        // keep PAM's lock message
            if (f) root.say(f.text, f.kind);
            else root.say("That password isn't right. Try again.", "wrong");
        }
        function onError(error) {
            root.busy = false; root.pendingSecret = null; root.password = "";
            root.say("Couldn't sign in: " + error, "error");
            Greetd.cancelSession();
        }
        function onReadyToLaunch() {
            root.remember();
            const s = root.session;
            const file = s ? s.file : "nimbus.desktop";
            const id = file.replace(/\.desktop$/, "");
            const env = ["XDG_SESSION_TYPE=wayland", "XDG_SESSION_DESKTOP=" + id, "DESKTOP_SESSION=" + id];
            if (s && s.names) env.push("XDG_CURRENT_DESKTOP=" + s.names);
            if (s && s.mode) env.push("HEARTH_LOGIN_MODE=" + s.mode);
            root.say("Signing in…", "info");
            Greetd.launch([root.sessionStart, file], env);
        }
    }

    // ---------------------------------------------------------------- power
    Process { id: powerProc }
    Process { id: okMark; command: ["sh", "-c", ": > \"$0/hearth-greeter-ok\"", root.runDir] }
    function power(what) {
        menu = ""; Nav.setLayer("main", root);
        if (preview && !testMode) { say("This is a preview: " + what + " is off here.", "info"); return; }
        lastPower = what;
        okMark.running = true;      // the watch counts this start as working
        if (testMode) { say("(test) " + what, "info"); return; }
        powerProc.command = ["systemctl", what === "sleep" ? "suspend" : what === "restart" ? "reboot" : "poweroff"];
        powerProc.running = true;
    }

    // ---------------------------------------------------------------- the screen reader (Orca), when it's installed
    Process { id: orca; command: ["orca", "--replace"] }
    function setScreenReader(on) {
        screenReader = on;
        if (on && !orca.running) orca.running = true;
        if (!on && orca.running) orca.signal(15);
    }

    // ---------------------------------------------------------------- the keyboard and the controller
    function openOsk() { if (!osk.item) return; oskOpen = true; osk.item.reset(); Nav.setLayer("osk", root); }
    function closeOsk() { if (!oskOpen) return; oskOpen = false; Nav.setLayer(menu !== "" ? "menu" : "main", root); }
    function openMenu(m) { menu = m; Nav.setLayer("menu", root); }
    function closeMenu() { menu = ""; Nav.setLayer(oskOpen ? "osk" : "main", root); }
    function typeText(t) {
        if (view !== "password") { choose(userIndex); }
        if (busy) return;
        password += t;
        if (messageKind === "wrong") say("");
        if (t.length === 1 && t.toLowerCase() !== t.toUpperCase()) capsLater.restart();
    }
    function deleteBack(n) { password = password.slice(0, Math.max(0, password.length - (n || 1))); }
    function activate() {
        const c = Nav.ensure(root);
        if (c) c.activated(); else if (view === "password") signIn();
    }
    function key(name) {          // one key, from the keyboard, the controller (as keys) or the tests
        if (name === "osk") { if (oskOpen) closeOsk(); else openOsk(); return; }
        if (oskOpen && Nav.layer === "osk") {
            if (name === "up" || name === "down" || name === "left" || name === "right") { Nav.ring = true; osk.item.press(name); }
            else if (name === "enter") osk.item.press("a");
            else if (name === "back") osk.item.press("x");
            else if (name === "clear") closeOsk();
            return;
        }
        if (name === "up" || name === "down" || name === "left" || name === "right") {
            if (!Nav.move(name, root) && name === "down" && oskOpen && osk.item) { Nav.setLayer("osk", root); if (osk.item.engine) osk.item.engine.setActive(true); }
            return;
        }
        if (name === "enter") {
            // A real keyboard's Enter signs in from the password (or picks the account); the ring's item otherwise.
            if (menu === "" && !(Nav.ring && Nav.current)) { if (view === "password") signIn(); else choose(userIndex); return; }
            Nav.ring = true; activate(); return;
        }
        if (name === "back") {
            if (menu !== "") closeMenu();
            else if (view === "password" && password.length) deleteBack(1);
            else if (view === "password") backToUsers();
            return;
        }
        if (name === "clear") {
            if (menu !== "") closeMenu();
            else { password = ""; say(""); }
        }
    }
    Keys.onPressed: e => {
        const k = e.key;
        let name = "";
        if (k === Qt.Key_Up) name = "up"; else if (k === Qt.Key_Down) name = "down";
        else if (k === Qt.Key_Left) name = "left"; else if (k === Qt.Key_Right) name = "right";
        else if (k === Qt.Key_Return || k === Qt.Key_Enter) name = "enter";
        else if (k === Qt.Key_Backspace) name = (e.modifiers & Qt.ControlModifier) ? "clear" : "back";
        else if (k === Qt.Key_Escape) name = "clear";
        else if (k === Qt.Key_Tab || k === Qt.Key_Backtab) name = "osk";
        else if (k === Qt.Key_CapsLock) { capsLater.restart(); e.accepted = true; return; }
        if (name !== "") { key(name); e.accepted = true; return; }
        if (e.text && e.text.length && e.text >= " " && menu === "" && !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier))) {
            // Caps Lock: a letter in capitals without Shift (or small with it) says so straight away
            const t = e.text;
            if (t.toLowerCase() !== t.toUpperCase()) caps = (t === t.toUpperCase()) !== !!(e.modifiers & Qt.ShiftModifier);
            typeText(t); e.accepted = true;
        }
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
        anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom
        width: parent.width * 1.2; height: parent.height * 0.6
        source: "file://" + root.assets + "/ember-glow.png"
        opacity: Theme.contrast ? 0 : 0.5
        SequentialAnimation on opacity {
            running: !Theme.contrast && !root.testMode
            loops: Animation.Infinite
            NumberAnimation { to: 0.6; duration: 3600; easing.type: Easing.InOutSine }
            NumberAnimation { to: 0.45; duration: 4200; easing.type: Easing.InOutSine }
        }
    }

    // ---------------------------------------------------------------- along the top: Hearth, layout, Wi-Fi, battery, access
    RowLayout {
        id: top
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.px(24) }
        spacing: Theme.px(10)
        Image { source: "file://" + root.assets + "/hearth-logo.png"; Layout.preferredWidth: Theme.px(34); Layout.preferredHeight: Theme.px(34); fillMode: Image.PreserveAspectFit }
        Text { text: "Hearth"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.px(20); font.weight: Font.DemiBold }
        Rectangle {
            visible: root.preview
            radius: Theme.px(10); color: Theme.panel2; border.color: Theme.warn
            implicitWidth: pv.implicitWidth + Theme.px(24); implicitHeight: Theme.px(32)
            Text { id: pv; anchors.centerIn: parent; text: "Preview: signing in is off"; color: Theme.warn; font.family: Theme.font; font.pixelSize: Theme.px(13) }
        }
        Item { Layout.fillWidth: true }
        Text {
            visible: root.status.wifi && root.status.wifi.present
            text: !root.status.wifi.on ? "Wi-Fi off" : root.status.wifi.connected ? "Wi-Fi · " + root.status.wifi.name : root.status.wifi.wired ? "Wired" : "Wi-Fi · not connected"
            color: root.status.wifi && root.status.wifi.connected ? Theme.text : Theme.muted
            font.family: Theme.font; font.pixelSize: Theme.px(15)
            Accessible.role: Accessible.StaticText; Accessible.name: text
        }
        Text {
            visible: root.status.battery && root.status.battery.present
            text: "Battery " + (root.status.battery.percent || 0) + "%" + (root.status.battery.charging ? " ⚡" : "")
            color: (root.status.battery.percent || 100) <= 10 && !root.status.battery.charging ? Theme.danger : Theme.text
            font.family: Theme.font; font.pixelSize: Theme.px(15)
            Accessible.role: Accessible.StaticText; Accessible.name: text
        }
        Chip {
            name: "Keyboard layout"
            text: "⌨ " + (osk.item && osk.item.layoutShort ? osk.item.layoutShort : (root.layouts[0] || "us").toUpperCase())
            onActivated: root.openMenu("layout")
        }
        Chip { name: "Accessibility"; text: "Accessibility"; onActivated: root.openMenu("access") }
    }

    // ---------------------------------------------------------------- the clock, then who's signing in
    ColumnLayout {
        id: middle
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: root.oskOpen ? -(oskBox.height / 2) - Theme.px(10) : -Theme.px(20)
        width: Math.min(parent.width - Theme.px(80), Theme.px(1100))
        spacing: Theme.px(10)
        Behavior on anchors.verticalCenterOffset { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

        SystemClock { id: clock; precision: SystemClock.Minutes }
        Text {
            Layout.alignment: Qt.AlignHCenter
            visible: !root.oskOpen || root.height > 900
            text: Qt.formatTime(clock.date, "h:mm")
            color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.px(root.oskOpen ? 64 : 96); font.weight: Font.DemiBold
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            visible: !root.oskOpen
            text: Qt.formatDate(clock.date, "dddd, MMMM d")
            color: Theme.muted; font.family: Theme.font; font.pixelSize: Theme.px(22)
        }
        Item { Layout.preferredHeight: Theme.px(root.oskOpen ? 4 : 28) }

        // ---- who's signing in
        Row {
            Layout.alignment: Qt.AlignHCenter
            visible: root.view === "users"
            spacing: Theme.px(22)
            Repeater {
                model: root.users
                delegate: Rectangle {
                    id: uc
                    required property var modelData
                    required property int index
                    width: Theme.px(190); height: Theme.px(230); radius: Theme.px(18)
                    color: ucChip.focused || ucChip.hovered ? Theme.panel2 : "transparent"
                    border.width: ucChip.focused ? 3 : 0; border.color: Theme.accent
                    Column {
                        anchors.centerIn: parent
                        spacing: Theme.px(14)
                        Avatar {
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: Theme.px(128); height: Theme.px(128)
                            source: uc.modelData.picture || ""; label: uc.modelData.real || uc.modelData.name
                            lit: root.userIndex === uc.index
                        }
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: uc.modelData.real || uc.modelData.name
                            color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.px(20); font.weight: Font.DemiBold
                        }
                    }
                    // the whole card is the button (the ring and Enter, a click or a tap); the card draws it
                    Chip {
                        id: ucChip
                        anchors.fill: parent
                        color: "transparent"; border.width: 0
                        name: "User " + uc.modelData.name
                        preferred: root.userIndex === uc.index
                        onActivated: root.choose(uc.index)
                    }
                }
            }
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            visible: root.view === "users" && root.users.length === 0 && root.restored
            text: "No accounts to sign in to."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: Theme.px(18)
        }

        // ---- the password
        ColumnLayout {
            Layout.alignment: Qt.AlignHCenter
            visible: root.view === "password" && root.user !== null
            spacing: Theme.px(12)
            Avatar {
                Layout.alignment: Qt.AlignHCenter
                visible: !root.oskOpen
                Layout.preferredWidth: Theme.px(112); Layout.preferredHeight: Theme.px(112)
                source: root.user ? (root.user.picture || "") : ""; label: root.user ? (root.user.real || root.user.name) : "?"
            }
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: root.user ? (root.user.real || root.user.name) : ""
                color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.px(26); font.weight: Font.DemiBold
            }
            Text {
                Layout.alignment: Qt.AlignHCenter
                visible: root.answering && root.prompt !== ""
                text: root.prompt
                color: Theme.muted; font.family: Theme.font; font.pixelSize: Theme.px(15)
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: Theme.px(8)
                Chip {
                    id: field
                    name: "Password"
                    preferred: true
                    Layout.preferredWidth: Theme.px(440); Layout.preferredHeight: Theme.px(58)
                    color: Theme.panel2
                    border.width: focused ? 3 : 2
                    border.color: focused ? Theme.accent : root.messageKind === "wrong" || root.messageKind === "locked" ? Theme.danger : Theme.line
                    Accessible.role: Accessible.EditableText
                    Accessible.passwordEdit: !root.showPassword
                    onActivated: root.signIn()
                    Text {
                        anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: Theme.px(18); rightMargin: Theme.px(18) }
                        text: root.busy ? "Signing in…" : root.password.length ? (root.showPassword ? root.password : "•".repeat(root.password.length)) : (root.answering ? "Type the answer" : "Password")
                        color: root.password.length && !root.busy ? Theme.text : Theme.muted
                        font.family: Theme.font; font.pixelSize: Theme.px(22)
                        elide: Text.ElideLeft
                    }
                    Rectangle {      // the caret
                        visible: !root.busy && (root.password.length > 0 || field.focused)
                        anchors.verticalCenter: parent.verticalCenter
                        x: Theme.px(18) + Math.min(parent.width - Theme.px(40), (root.showPassword ? root.password.length * Theme.px(12) : root.password.length * Theme.px(13)))
                        width: 2; height: Theme.px(26); color: Theme.accent
                        SequentialAnimation on opacity { loops: Animation.Infinite; running: !root.testMode; NumberAnimation { to: 0; duration: 520 } NumberAnimation { to: 1; duration: 520 } }
                    }
                }
                Chip {
                    name: root.showPassword ? "Hide password" : "Show password"
                    text: root.showPassword ? "Hide" : "Show"
                    Layout.preferredHeight: Theme.px(58)
                    onActivated: root.showPassword = !root.showPassword
                }
                Chip {
                    name: "Sign in"; text: "Sign in →"; primary: true
                    enabled: !root.busy
                    Layout.preferredHeight: Theme.px(58)
                    onActivated: root.signIn()
                }
            }
            Rectangle {     // Caps Lock
                Layout.alignment: Qt.AlignHCenter
                visible: root.caps && root.view === "password"
                radius: Theme.px(10); color: Qt.rgba(1, 0.82, 0.4, 0.14); border.color: Theme.warn
                implicitWidth: capsText.implicitWidth + Theme.px(28); implicitHeight: Theme.px(34)
                Text { id: capsText; anchors.centerIn: parent; text: "⇪ Caps Lock is on"; color: Theme.warn; font.family: Theme.font; font.pixelSize: Theme.px(15); font.weight: Font.Medium }
                Accessible.role: Accessible.AlertMessage; Accessible.name: capsText.text
            }
            Rectangle {     // what happened
                Layout.alignment: Qt.AlignHCenter
                visible: root.message !== ""
                radius: Theme.px(12)
                color: root.messageKind === "locked" || root.messageKind === "wrong" || root.messageKind === "error" ? Qt.rgba(1, 0.36, 0.36, 0.14) : Theme.panel2
                border.color: root.messageKind === "locked" || root.messageKind === "wrong" || root.messageKind === "error" ? Theme.danger : Theme.line
                implicitWidth: Math.min(Theme.px(620), msg.implicitWidth + Theme.px(32)); implicitHeight: msg.implicitHeight + Theme.px(18)
                Text {
                    id: msg
                    anchors.centerIn: parent
                    width: Math.min(Theme.px(580), implicitWidth)
                    text: (root.messageKind === "locked" ? "🔒 " : "") + root.message
                    color: root.messageKind === "info" ? Theme.text : Theme.danger
                    font.family: Theme.font; font.pixelSize: Theme.px(16); wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter
                }
                Accessible.role: Accessible.AlertMessage; Accessible.name: root.message
            }
            Chip {
                Layout.alignment: Qt.AlignHCenter
                visible: root.users.length > 1
                name: "Other account"; text: "‹ Someone else"
                onActivated: root.backToUsers()
            }
        }
    }

    // ---------------------------------------------------------------- along the bottom: session, keyboard, power
    RowLayout {
        id: bottom
        anchors { left: parent.left; right: parent.right; bottom: oskBox.top; margins: Theme.px(24) }
        spacing: Theme.px(10)
        Chip {
            name: "Session"
            visible: root.sessions.length > 0
            text: "Start: " + (root.session ? root.session.name : "Hearth") + "  ▾"
            onActivated: root.openMenu("session")
        }
        Item { Layout.fillWidth: true }
        Text {
            visible: Nav.ring && !root.oskOpen
            text: "Ⓐ Select   Ⓑ Back   Ⓧ Keyboard   Ⓨ Clear"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: Theme.px(14)
        }
        Item { Layout.fillWidth: true }
        Chip { name: "On-screen keyboard"; text: root.oskOpen ? "Hide keyboard" : "Keyboard"; visible: osk.status === Loader.Ready; onActivated: root.oskOpen ? root.closeOsk() : root.openOsk() }
        Chip { name: "Power"; text: "⏻ Power"; onActivated: root.openMenu("power") }
    }

    // ---------------------------------------------------------------- the on-screen keyboard
    Item {
        id: oskBox
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: root.oskOpen ? Math.round(Math.min(root.height * 0.42, Math.max(290, root.width * 0.22))) : 0
        clip: true
        Behavior on height { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }
        Rectangle { anchors.fill: parent; color: Qt.rgba(0.07, 0.05, 0.04, 0.92); visible: root.oskOpen }
        Loader {
            id: osk
            anchors { fill: parent; leftMargin: 8; rightMargin: 8; topMargin: 6 }
            visible: root.oskOpen
            source: "GreeterOsk.qml"
            onLoaded: { item.layouts = root.layouts; }
            Connections {
                target: osk.item
                ignoreUnknownSignals: true
                function onTyped(text) { root.typeText(text); }
                function onBack(n) { root.deleteBack(n); }
                function onEnter() { root.signIn(); }
                function onHide() { root.closeOsk(); }
                function onLeave() { Nav.setLayer("main", root); }
            }
        }
    }
    onLayoutsChanged: if (osk.item) osk.item.layouts = layouts

    // ---------------------------------------------------------------- menus
    Rectangle {
        anchors.fill: parent
        visible: root.menu !== ""
        color: Qt.rgba(0, 0, 0, 0.55)
        MouseArea { anchors.fill: parent; onClicked: root.closeMenu() }
    }
    Rectangle {
        id: menuBox
        visible: root.menu !== ""
        anchors.centerIn: parent
        width: menuCol.implicitWidth + Theme.px(36); height: menuCol.implicitHeight + Theme.px(36)
        radius: Theme.px(20); color: Theme.panel; border.color: Theme.line
        MouseArea { anchors.fill: parent }
        ColumnLayout {
            id: menuCol
            anchors.centerIn: parent
            spacing: Theme.px(8)
            Text {
                text: root.menu === "session" ? "Start" : root.menu === "power" ? "Power" : root.menu === "layout" ? "Keyboard layout" : "Accessibility"
                color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.px(22); font.weight: Font.DemiBold
                Layout.bottomMargin: Theme.px(4)
            }
            Repeater {
                model: root.menu === "session" ? root.sessions : []
                delegate: Chip {
                    required property var modelData
                    layer: "menu"; wide: true
                    name: "Session " + modelData.id
                    text: modelData.name; sub: modelData.sub || ""
                    checked: root.sessionId === modelData.id
                    preferred: checked
                    onActivated: { root.sessionId = modelData.id; root.closeMenu(); }
                }
            }
            Repeater {
                model: root.menu === "power" ? [{ id: "sleep", label: "Sleep" }, { id: "restart", label: "Restart" }, { id: "shutdown", label: "Shut down" }] : []
                delegate: Chip {
                    required property var modelData
                    layer: "menu"; wide: true
                    name: "Power " + modelData.id
                    text: modelData.label
                    preferred: modelData.id === "sleep"
                    onActivated: root.power(modelData.id)
                }
            }
            Repeater {
                model: root.menu === "layout" ? root.layouts : []
                delegate: Chip {
                    required property var modelData
                    required property int index
                    layer: "menu"; wide: true
                    name: "Layout " + modelData
                    text: modelData.toUpperCase()
                    sub: index === 0 ? "The on-screen keyboard; your keyboard follows the system's layout" : "On-screen keyboard"
                    preferred: index === 0
                    onActivated: { if (osk.item) osk.item.pickLayout(modelData); root.closeMenu(); }
                }
            }
            Chip {
                visible: root.menu === "access"
                layer: "menu"; wide: true; preferred: true
                name: "Bigger text"; text: "Bigger text"; checked: Theme.scale > 1.01
                onActivated: { Theme.scale = Theme.scale > 1.01 ? 1.0 : 1.3; root.remember(); }
            }
            Chip {
                visible: root.menu === "access"
                layer: "menu"; wide: true
                name: "High contrast"; text: "High contrast"; checked: Theme.contrast
                onActivated: { Theme.contrast = !Theme.contrast; root.remember(); }
            }
            Chip {
                visible: root.menu === "access" && root.screenReaderThere
                layer: "menu"; wide: true
                name: "Screen reader"; text: "Screen reader"; sub: "Orca reads the screen aloud"; checked: root.screenReader
                onActivated: root.setScreenReader(!root.screenReader)
            }
            Chip {
                visible: root.menu === "access"
                layer: "menu"; wide: true
                name: "On-screen keyboard (access)"; text: "On-screen keyboard"; checked: root.oskOpen
                onActivated: { root.closeMenu(); root.oskOpen ? root.closeOsk() : root.openOsk(); }
            }
            Chip {
                layer: "menu"; wide: true
                name: "Close menu"; text: "Close"
                onActivated: root.closeMenu()
            }
        }
    }

    // ---------------------------------------------------------------- on screen: tell the watch (hearth-login-screen)
    Process { id: readyMark; command: ["sh", "-c", ": > \"$0/hearth-greeter-ready\"", root.runDir] }
    Timer { interval: 900; running: true; onTriggered: readyMark.running = true }

    // ---------------------------------------------------------------- the tests (HEARTH_GREETER_TEST=1 only)
    IpcHandler {
        target: "greeter"
        enabled: root.testMode
        function pick(name: string): string { const i = root.users.findIndex(u => u.name === name); if (i >= 0) root.choose(i); return root.stateJson(); }
        function type(text: string): string { root.typeText(text); return "ok"; }
        function key(name: string): string { root.key(name); return root.stateJson(); }
        function signIn(): string { root.signIn(); return "ok"; }
        function menu(name: string): string { if (name === "") root.closeMenu(); else root.openMenu(name); return root.stateJson(); }
        function osk(on: bool): string { if (on) root.openOsk(); else root.closeOsk(); return root.stateJson(); }
        function caps(on: bool): string { root.caps = on; return "ok"; }
        function show(on: bool): string { root.showPassword = on; return "ok"; }
        function big(on: bool): string { Theme.scale = on ? 1.3 : 1.0; return "ok"; }
        function session(id: string): string { root.sessionId = id; return root.stateJson(); }
        function state(): string { return root.stateJson(); }
    }
    function stateJson() {
        return JSON.stringify({
            view: view, user: user ? user.name : "", users: users.length, sessions: sessions.map(s => s.id), session: sessionId,
            menu: menu, osk: oskOpen, oskReady: osk.status === Loader.Ready, message: message, kind: messageKind, busy: busy,
            typed: password.length, focus: Nav.current ? Nav.current.name : "", layer: Nav.layer, preview: preview,
            greetd: Greetd.available, power: lastPower, caps: caps
        });
    }
    Component.onCompleted: forceActiveFocus()
}
