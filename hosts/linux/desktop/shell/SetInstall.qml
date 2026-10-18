// Hearth Settings › Install Hearth OS (live USB only): Hearth's own installer with a mouse and keyboard.
// The same engine as the Hearth app's install screens (/usr/libexec/nimbus/hearth-install): next to Windows
// (shrinks Windows with hearth-live-prep first) or on a whole drive, your account, then the install itself,
// which downloads Hearth OS straight onto the drive.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Install Hearth OS"
    readonly property string engine: "/usr/libexec/nimbus/hearth-install"
    readonly property string prep: "/usr/libexec/nimbus/hearth-live-prep"
    readonly property real gib: 1024 * 1024 * 1024
    property string phase: "setup"     // setup | confirm | running | done | failed
    property string mode: ""           // beside | whole
    property var drives: null          // hearth-install disks
    property string disk: ""
    property var win: null             // hearth-live-prep check (beside)
    property bool checking: false
    property real give: 0
    property string tz: "UTC"
    property string error: ""
    property string stageText: ""
    property real pct: 0
    property int left: -1
    property string stage: ""
    readonly property var drive: drives ? drives.find(d => d.dev === disk) || null : null
    readonly property var blockers: win && win.problems ? win.problems.filter(p => !p.soft) : []
    readonly property var warnings: win && win.problems ? win.problems.filter(p => p.soft) : []

    function gb(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : Math.round(b / 1e9) + " GB"; }
    function autoName(full) { return full.toLowerCase().replace(/[^a-z0-9]+/g, "").replace(/^[0-9]+/, "").slice(0, 20); }
    function problem() {
        if (!mode) return "Choose how to install";
        if (!disk) return "Choose a drive";
        if (mode === "beside") {
            if (checking || !win) return "Checking Windows…";
            if (blockers.length) return blockers[0].text;
            if (give < 64 * gib) return "Hearth needs at least 64 GB";
        }
        if (!fullName.text.trim()) return "Type your name";
        if (!/^[a-z_][a-z0-9_-]{0,31}$/.test(userName.text)) return "The user name can only have small letters, numbers, - and _";
        if (!noPass.checked && !pass1.text) return "Choose a password, or turn on No password";
        if (!noPass.checked && pass1.text !== pass2.text) return "The passwords don't match";
        return "";
    }
    function sizes() {
        if (!win || !win.windows) return [];
        const max = Math.floor(win.windows.canGive / gib);
        const out = [];
        const steps = [64, 128, 256, 512, 1024];
        for (let i = 0; i < steps.length; i++) if (steps[i] < max) out.push({ value: steps[i] * gib, label: steps[i] + " GB" });
        out.push({ value: max * gib, label: "All it can (" + max + " GB)" });
        return out;
    }
    function check() {
        if (checkProc.running) return;
        win = null; checking = true;
        checkProc.running = true;
    }
    function start() {
        const plan = { mode: mode, disk: disk, user: { full: fullName.text.trim(), name: userName.text, password: noPass.checked ? "" : pass1.text, nopassword: noPass.checked },
                       hostname: (hostName.text || (userName.text + "-hearth")), timezone: tz, signin: signinChoice.current };
        if (mode === "beside") plan.give = Math.round(give);
        error = ""; pct = 0; stageText = "Starting"; stage = "start"; left = -1;
        phase = "running";
        runProc.plan = JSON.stringify(plan);
        runProc.running = true;
    }

    Component.onCompleted: { drivesProc.running = true; tzProc.running = true; }

    Process {
        id: drivesProc
        command: ["pkexec", page.engine, "disks"]
        stdout: SplitParser { onRead: l => { l = l.trim(); if (l.startsWith("{")) { try { page.drives = JSON.parse(l).disks; } catch (e) {} } else if (l.startsWith("error: ")) page.error = l.slice(7); } }
    }
    Process {
        id: tzProc
        command: ["timedatectl", "show", "-p", "Timezone", "--value"]
        stdout: SplitParser { onRead: l => { if (l.trim()) page.tz = l.trim(); } }
    }
    Process {
        id: checkProc
        command: ["pkexec", page.prep, "check"]
        stdout: SplitParser { onRead: l => { l = l.trim(); if (l.startsWith("{")) { try { page.win = JSON.parse(l); page.give = page.win.suggested || 0; } catch (e) {} } else if (l.startsWith("error: ")) page.win = { problems: [{ id: "error", text: l.slice(7) }] }; } }
        onExited: page.checking = false
    }
    Process {
        id: runProc
        property string plan: ""
        command: ["sh", "-c", 'printf %s "$1" | pkexec "$2" run', "sh", plan, page.engine]
        stdout: SplitParser {
            onRead: l => {
                l = l.trim();
                if (l.startsWith("stage: ")) {
                    try { const s = JSON.parse(l.slice(7)); page.pct = s.pct; page.stageText = s.text; page.stage = s.stage; page.left = s.left != null ? s.left : -1; } catch (e) {}
                } else if (l.startsWith("result: ")) page.phase = "done";
                else if (l.startsWith("error: ")) page.error = l.slice(7);
            }
        }
        onExited: code => { if (page.phase === "running") { page.phase = code === 0 && !page.error ? "done" : "failed"; if (page.phase === "failed" && !page.error) page.error = "The install stopped."; } }
    }

    // ================================================================ choosing
    Text {
        visible: page.phase === "setup" || page.phase === "confirm"
        Layout.fillWidth: true; wrapMode: Text.WordWrap
        text: "Hearth OS is downloaded while it installs (about 5 GB), so stay connected to the internet and plugged in. It takes about 10 to 20 minutes."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    SetChoice {
        visible: page.phase === "setup"
        title: "How"
        sub: page.mode === "whole" ? "Hearth gets the whole drive. Everything on it is erased." : page.mode === "beside" ? "Keep Windows and its files. Hearth gets space Windows isn't using." : "Next to Windows keeps it; a whole drive erases it."
        options: [{ value: "beside", label: "Next to Windows" }, { value: "whole", label: "Erase a drive" }]
        current: page.mode
        onChosen: v => { page.mode = v; page.disk = ""; page.win = null; }
    }

    SetHeader { visible: page.phase === "setup" && page.mode !== ""; text: page.mode === "beside" ? "The drive with Windows" : "The drive for Hearth" }
    SetRow {
        visible: page.phase === "setup" && page.mode !== "" && page.drives === null
        interactive: false; chevron: false; icon: "update"; title: "Looking for drives…"
    }
    Repeater {
        model: page.phase === "setup" && page.mode !== "" && page.drives ? page.drives.filter(d => page.mode === "whole" || d.windows) : []
        delegate: SetRow {
            required property var modelData
            readonly property string why: modelData.live ? "The USB stick Hearth is running from" : modelData.tooSmall ? "Too small (Hearth needs 64 GB)" : ""
            interactive: why === ""
            chevron: false
            icon: modelData.dev === page.disk ? "check" : "display"
            title: modelData.model + " · " + page.gb(modelData.size)
            sub: why || [modelData.tran.toUpperCase(), modelData.windows ? "Has Windows" : "", modelData.parts.length ? modelData.parts.length + " partitions" : "Empty"].filter(x => x).join(" · ")
            value: modelData.dev === page.disk ? "Chosen" : ""
            onActivated: { page.disk = modelData.dev; if (page.mode === "beside" && !page.win) page.check(); }
        }
    }

    // next to Windows: what was found, and how much room
    SetRow {
        visible: page.phase === "setup" && page.mode === "beside" && page.checking
        interactive: false; chevron: false; icon: "update"; title: "Checking Windows…"; sub: "Free space, BitLocker, Fast Startup and your games (up to a minute)"
    }
    Repeater {
        model: page.phase === "setup" && page.mode === "beside" ? page.blockers : []
        delegate: SetRow { required property var modelData; interactive: false; chevron: false; danger: true; icon: "lock"; title: "Can't continue yet"; sub: modelData.text }
    }
    Repeater {
        model: page.phase === "setup" && page.mode === "beside" ? page.warnings : []
        delegate: SetRow { required property var modelData; interactive: false; chevron: false; icon: "info"; title: modelData.id === "power" ? "Plug in the charger" : "No internet"; sub: modelData.text }
    }
    SetRow {
        visible: page.phase === "setup" && page.mode === "beside" && !page.checking && !!page.win && page.blockers.length > 0
        icon: "restart"; title: "Check again"; onActivated: page.check()
    }
    SetChoice {
        visible: page.phase === "setup" && page.mode === "beside" && !!page.win && !!page.win.windows && page.blockers.length === 0
        title: "Room for Hearth"
        sub: page.win && page.win.windows ? "Windows keeps " + page.gb(page.win.windows.size - page.give) + (page.win.games && page.win.games.count ? ". " + page.win.games.count + " games found in Windows (" + page.gb(page.win.games.size) + "): leave room to bring them over." : ".") : ""
        options: page.sizes()
        current: page.give
        onChosen: v => page.give = v
    }

    // your account
    SetHeader { visible: page.phase === "setup"; text: "Your account" }
    Text { visible: page.phase === "setup"; text: "Your name"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    SetField {
        id: fullName; visible: page.phase === "setup"; placeholder: "e.g. Sam Rivera"
        onTextChanged: if (!userName.edited) userName.text = page.autoName(text)
    }
    Text { visible: page.phase === "setup"; text: "User name (small letters and numbers)"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    SetField { id: userName; property bool edited: false; visible: page.phase === "setup"; placeholder: "e.g. sam"; onSubmitted: edited = true; onTextChanged: if (activeFocus) edited = true }
    SetSwitch {
        id: noPass; visible: page.phase === "setup"; icon: "lock"; title: "No password"
        sub: checked ? "Anyone who picks up this device can use it and make system changes. You can add a password later in Settings › Sign-in." : "Skip the password: signing in and system changes just need Enter"
        onToggled: v => checked = v
    }
    Text { visible: page.phase === "setup" && !noPass.checked; text: "Password"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    SetField { id: pass1; visible: page.phase === "setup" && !noPass.checked; secret: true; placeholder: "Password" }
    SetField { id: pass2; visible: page.phase === "setup" && !noPass.checked; secret: true; placeholder: "The same password again" }
    SetChoice {
        id: signinChoice
        visible: page.phase === "setup" && !noPass.checked
        title: "Ask for my password"
        sub: current === "auto" ? "Starts and wakes straight into Hearth, like a console. Change it any time in Settings › Sign-in." : current === "start" ? "The login screen when it turns on." : "The login screen when it turns on, and the lock screen after sleep."
        options: [{ value: "auto", label: "Never" }, { value: "start", label: "When it turns on" }, { value: "always", label: "On and after sleep" }]
        current: "auto"
        onChosen: v => current = v
    }
    Text { visible: page.phase === "setup"; text: "Device name and time zone"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    SetField { id: hostName; visible: page.phase === "setup"; placeholder: (userName.text || "my") + "-hearth" }
    SetField { id: tzField; visible: page.phase === "setup"; text: page.tz; placeholder: "Time zone, e.g. America/Chicago"; onTextChanged: page.tz = text }

    SetRow {
        visible: page.phase === "setup"
        icon: "update"
        title: page.problem() === "" ? "Install Hearth OS" : "Install Hearth OS"
        sub: page.problem() || (page.mode === "whole" ? "Next, a last check before anything is erased" : "Next, a last check before Windows is shrunk")
        onActivated: if (page.problem() === "") page.phase = "confirm"
    }

    // ================================================================ last check
    Rectangle {
        visible: page.phase === "confirm"
        Layout.fillWidth: true
        implicitHeight: confirmText.implicitHeight + 32; radius: 12
        color: Qt.rgba(0.9, 0.28, 0.23, 0.16); border.color: "#e5483b"
        Text {
            id: confirmText
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 16 }
            wrapMode: Text.WordWrap
            text: page.mode === "whole"
                ? "Everything on " + (page.drive ? page.drive.model + " (" + page.gb(page.drive.size) + ")" : "the drive") + " is erased for good" + (page.drive && page.drive.windows ? ", including Windows" : "") + ", and Hearth OS goes on it."
                : "Windows gets " + page.gb(page.give) + " smaller and Hearth OS goes in that space. Windows and its files stay, but back up anything important first: resizing a drive is never completely risk-free."
            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
        }
    }
    SetRow { visible: page.phase === "confirm"; icon: page.mode === "whole" ? "close" : "update"; danger: page.mode === "whole"; title: page.mode === "whole" ? "Erase it and install Hearth OS" : "Install Hearth OS next to Windows"; onActivated: page.start() }
    SetRow { visible: page.phase === "confirm"; icon: "back"; title: "Go back"; onActivated: page.phase = "setup" }

    // ================================================================ installing
    ColumnLayout {
        visible: page.phase === "running"
        Layout.fillWidth: true; Layout.topMargin: 6
        spacing: 10
        Text { text: page.stageText || "Starting…"; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 10; radius: 5; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.02, page.pct / 100); height: parent.height; radius: 5; color: Theme.accent; Behavior on width { NumberAnimation { duration: 500 } } }
        }
        Text {
            text: Math.round(page.pct) + "%" + (page.left > 0 ? " · about " + (page.left < 90 ? "a minute" : Math.round(page.left / 60) + " minutes") + " left" : "")
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }
        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Keep the device plugged in and don't take out the USB stick until this is done."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    }

    // ================================================================ done / failed
    Rectangle {
        visible: page.phase === "done"
        Layout.fillWidth: true
        implicitHeight: doneText.implicitHeight + 32; radius: 12
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18); border.color: Theme.accent
        Text {
            id: doneText
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 16 }
            wrapMode: Text.WordWrap
            text: "Hearth OS is installed. Restart, and take out the USB stick when the screen goes dark. Hearth starts with a short setup" + (page.mode === "beside" ? ", including bringing your games over from Windows." : ".")
            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
        }
    }
    SetRow { visible: page.phase === "done"; icon: "restart"; title: "Restart now"; onActivated: Quickshell.execDetached(["systemctl", "reboot"]) }
    SetRow { visible: page.phase === "failed"; interactive: false; chevron: false; danger: true; icon: "info"; title: "The install didn't finish"; sub: page.error }
    SetRow {
        visible: page.phase === "failed"
        icon: "restart"; title: "Try again"
        sub: page.mode === "whole" ? "Installing again starts over." : "Windows is still there. If it got smaller, the space is still free: installing again uses it."
        onActivated: page.phase = "setup"
    }
}
