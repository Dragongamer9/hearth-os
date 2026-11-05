// Hearth Settings › Home server › Set up (docs/HOME-SERVER.md): the setup steps, all with the controller (Up/Down
// pick, A goes on, B back to the page list): this PC's name, Tailscale, the first shared folder, the first backup
// (Backups & sync comes with the next update; for now it points at Hearth Storage's backups). Every step can be
// skipped; nothing is shared or turned on unless you pick it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Set up your home server"
    property int step: 0
    property var st: null
    property var cands: []
    property var login: null
    property string msg: ""
    property bool bad: false
    readonly property var steps: ["Name", "Tailscale", "First share", "First backup", "Done"]
    readonly property var ts: st ? (st.tailscale || {}) : ({})
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function next() { page.msg = ""; page.step = Math.min(4, page.step + 1); Qt.callLater(page.focusFirst); }
    function back() { page.msg = ""; page.step = Math.max(0, page.step - 1); Qt.callLater(page.focusFirst); }
    Component.onCompleted: { refresh(); cj.run(["candidates"], r => page.cands = r.candidates || [], () => page.cands = []); }
    Timer { interval: 4000; running: page.visible && page.step === 1; repeat: true; onTriggered: if (!job.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: cj; helper: "hearth-home-server" }
    SetJob { id: adminJob; admin: true; helper: "/usr/libexec/nimbus/hearth-admin" }

    // The steps, with where you are
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        Repeater {
            model: page.steps
            delegate: Rectangle {
                required property var modelData
                required property int index
                implicitHeight: 28; implicitWidth: chip.implicitWidth + 22
                radius: 14
                color: index === page.step ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22) : index < page.step ? Qt.rgba(0.34, 0.76, 0.48, 0.18) : Theme.panel2
                Text { id: chip; anchors.centerIn: parent; text: (index < page.step ? "✓ " : "") + modelData; color: index === page.step ? Theme.accent : index < page.step ? "#57c27a" : Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
    }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 10
        text: ["Name this PC", "Reach your PC from anywhere", "Share your first folder", "Your first backup", "Your home server is ready"][page.step]
        color: Theme.text; font.family: Theme.font; font.pixelSize: 24; font.weight: Font.Bold; wrapMode: Text.WordWrap
    }
    Text {
        Layout.fillWidth: true
        text: ["Your other devices find this PC by its name: in Windows' File Explorer, on your phone and on Tailscale.",
               "Tailscale makes a private network just for your devices. Nothing is opened to the internet.",
               "Pick a folder your Windows PC, phone, TV and Hearth devices can open. It starts read only, on your home network.",
               "Backups & sync come with the next update. Until then, Hearth Storage keeps your Hearth devices' saves and screenshots.",
               "You can change everything in Settings › Home server."][page.step]
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }

    // 0: name
    SetRow {
        visible: page.step === 0
        icon: "display"; chevron: false; interactive: false; title: page.st ? page.st.host : ""
        sub: "This PC's name now"
    }
    SetField {
        id: nameField
        visible: page.step === 0
        placeholder: "A new name (letters, digits and -), then Enter. Y types with the controller"
        onSubmitted: t => {
            const n = t.trim();
            if (!/^[A-Za-z0-9][A-Za-z0-9-]{0,62}$/.test(n)) { page.say("Use letters, digits and - (no spaces)", true); return; }
            adminJob.run(["hostname", n], () => { page.say("Renamed to " + n); text = ""; page.refresh(); }, e => page.say(e, true));
        }
    }

    // 1: Tailscale
    SetRow {
        visible: page.step === 1 && page.ts.state === "Running"
        icon: "check"; chevron: false; interactive: false; title: "Signed in ✓"
        sub: "This PC is " + (page.ts.dns || page.ts.name || "") + " on your tailnet"
    }
    SetRow {
        visible: page.step === 1 && page.ts.installed === false
        icon: "info"; chevron: false; interactive: false; title: "Tailscale isn't on this PC yet"
        sub: "It comes with the Hearth OS update that brings Home server. Skip this step for now."
    }
    SetRow {
        visible: page.step === 1 && page.ts.installed === true && page.ts.state !== "Running"
        icon: "lock"; title: page.login ? "Waiting for you to sign in…" : "Show the sign-in code"
        sub: "A QR code for your phone, and a link for any device"
        onActivated: {
            const login = () => act.run(["ts-login"], r => { page.login = r; page.refresh(); }, e => page.say(e, true));
            if (page.ts.running === false) act.run(["ts-start"], login, e => page.say(e, true));
            else login();
        }
    }
    RowLayout {
        visible: page.step === 1 && !!page.login && !!page.login.authUrl && page.ts.state !== "Running"
        Layout.fillWidth: true
        spacing: 18
        Rectangle {
            visible: !!page.login && page.login.qr !== ""
            Layout.preferredWidth: 220; Layout.preferredHeight: 220
            radius: Theme.r(12); color: "white"
            Image {
                anchors.fill: parent
                anchors.margins: 10
                source: page.login && page.login.qr ? "file://" + page.login.qr : ""
                cache: false; smooth: false; fillMode: Image.PreserveAspectFit
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 6
            Text { Layout.fillWidth: true; text: "1. Scan with your phone, or open " + (page.login ? page.login.authUrl : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WrapAnywhere }
            Text { Layout.fillWidth: true; text: "2. Sign in with Google, Microsoft, GitHub or Apple"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { Layout.fillWidth: true; text: "3. This PC shows up as " + (page.st ? page.st.host : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
        }
    }

    // 2: first share
    Repeater {
        model: page.step === 2 ? page.cands.filter(c => !(page.st ? page.st.shares : []).some(s => s.path === c.path)).slice(0, 6) : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.drive ? "store" : "folder"; title: "Share " + modelData.name; sub: modelData.path
            onActivated: act.run(["share-add", modelData.path], r => { page.say("Shared ✓ " + r.share.name + ". Set its password in Shared folders"); page.refresh(); }, e => { page.say(e, true); page.refresh(); })
        }
    }
    SetRow {
        visible: page.step === 2 && !!page.st && page.st.shares.length > 0
        icon: "check"; chevron: false; interactive: false; title: "Shared: " + (page.st ? page.st.shares.map(s => s.name).join(", ") : "")
    }

    // 3: first backup
    SetRow {
        visible: page.step === 3
        icon: "update"; title: "Hearth Storage's backups"
        sub: "Your Hearth devices back up their saves and screenshots to this PC (Hearth Link › Hearth Storage)"
        onActivated: SettingsNav.openPage("link")
    }

    // Moving on
    SetRow {
        visible: page.step < 4
        icon: "forward"; title: page.step === 1 && page.ts.state === "Running" ? "Next" : page.step === 1 ? "I've signed in" : "Next"
        sub: page.step === 1 && page.ts.state !== "Running" ? "Or skip it: you can sign in on the Tailscale page any time" : ""
        onActivated: { if (page.step === 1 && page.ts.state !== "Running") page.refresh(); page.next(); }
    }
    SetRow {
        visible: page.step > 0 && page.step < 4
        icon: "back"; title: "Back"; chevron: false
        onActivated: page.back()
    }
    SetRow {
        visible: page.step === 4
        icon: "check"; title: "Go to Home server"
        onActivated: act.run(["set", "setupDone", "on"], () => SettingsNav.openPage("hs-overview"), () => SettingsNav.openPage("hs-overview"))
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
