// Hearth Settings › Web browser. Your browser (1.12): Hearth Browser (Firefox) or Google Chrome (downloaded from Google's
// signed package site when picked, into your home folder: hearth_chrome.py), Chrome marked "Must Have For Xodus Nexus".
// Hearth Browser: Mozilla's Firefox with Hearth's settings, part of Hearth OS (/usr/bin/hearth-browser, which answers
// --hearth-status, --hearth-default and --hearth-layout here). Aloha Browser stays an optional extra: downloaded from
// alohabrowser.com by the hearth-browser helper next to this file (it can't ship inside Hearth's image), installed
// in your home folder.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Web browser"
    property var st: null
    property var hb: null
    property var ch: null        // hearth-browser choice: {browser, picked, asked, chrome: {installed, version, note}}
    property string busy: ""
    property real progress: -1
    property string line: ""
    property string error: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-browser").toString().replace(/^file:\/\//, ""))
    readonly property string browser: "/usr/bin/hearth-browser"

    function run(cmd) {
        if (proc.running) return;
        const args = Array.isArray(cmd) ? cmd : [cmd];
        busy = args[0] === "status" ? "" : args.join(" "); error = ""; progress = -1; line = "";
        proc.command = ["python3", helper].concat(args);
        proc.running = true;
    }
    function choice() { if (!chProc.running) { chProc.command = ["python3", helper, "choice"]; chProc.running = true; } }
    function hearth(args) {
        if (hbProc.running) return;
        hbProc.command = [browser].concat(args);
        hbProc.running = true;
    }
    Component.onCompleted: { run("status"); hearth(["--hearth-status"]); choice(); }
    Process {
        id: chProc
        stdout: SplitParser { onRead: l => { if (l.startsWith("{")) { try { page.ch = JSON.parse(l); } catch (e) {} } } }
    }
    Process {
        id: proc
        stdout: SplitParser {
            onRead: l => {
                const m = l.match(/^progress: (\d+) ?(.*)$/);
                if (m) { page.progress = parseInt(m[1]) / 100; page.line = m[2]; }
                else if (l.startsWith("error: ")) page.error = l.slice(7);
                else if (l.startsWith("{")) { try { page.st = JSON.parse(l); } catch (e) {} }
            }
        }
        onExited: { page.busy = ""; page.progress = -1; page.choice(); page.hearth(["--hearth-status"]); }
    }
    Process {
        id: hbProc
        stdout: SplitParser {
            onRead: l => {
                if (l.startsWith("error: ")) page.error = l.slice(7);
                else if (l.startsWith("{")) { try { page.hb = JSON.parse(l); } catch (e) {} }
            }
        }
    }
    // ---- Your browser (1.12): pick one; links from other apps open in it
    SetHeader { text: "Your browser" }
    SetRow {
        icon: "globe"; title: "Hearth Browser (Firefox)"; chevron: false
        sub: "Comes with Hearth OS. Google sign-in, Netflix and add-ons work"
        chip: page.ch && page.ch.browser === "firefox" ? "Your browser" : ""; chipKind: "ok"
        onActivated: if (page.busy === "") page.run(["pick", "firefox"])
    }
    SetRow {
        icon: "globe"; title: "Google Chrome"; chevron: false
        sub: !page.ch ? "Checking…" : page.ch.chrome.installed ? "Installed" + (page.ch.chrome.version ? " · version " + page.ch.chrome.version : "") + " · Hearth looks for updates once a week"
            : "Downloaded from Google when you pick it (about 120 MB), checked against Google's signature, kept in your home folder"
        chip: page.ch && page.ch.browser === "chrome" ? "Your browser" : ""; chipKind: "ok"
        onActivated: if (page.busy === "") page.run(["pick", "chrome"])
    }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 56
        text: "Must Have For Xodus Nexus"; color: Theme.danger
        font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
    }
    SetRow {
        visible: !!page.ch && page.ch.chrome.installed && page.busy === ""
        icon: "restart"; title: "Look for a Chrome update"; sub: "Hearth also checks once a week"; chevron: false
        onActivated: page.run(["chrome", "update"])
    }
    SetRow {
        visible: !!page.ch && page.ch.chrome.installed && page.busy === ""
        icon: "close"; danger: true; title: "Remove Google Chrome"; sub: "Your browser goes back to Hearth Browser"; chevron: false
        onActivated: page.run(["chrome", "remove"])
    }

    SetHeader { text: "Hearth Browser" }
    SetRow {
        icon: "globe"; title: "Open Hearth Browser"
        sub: !page.hb ? "Checking…" : !page.hb.installed ? "Not part of this version of Hearth OS yet"
            : "Built on Firefox" + (page.hb.version ? " " + page.hb.version : "") + ": Google sign-in, Netflix and add-ons work" + (page.hb.default ? " · your default browser" : "")
        onActivated: if (page.hb && page.hb.installed) Quickshell.execDetached([page.browser])
    }
    SetRow {
        visible: !!page.hb && page.hb.installed && !page.hb.default
        icon: "check"; title: "Make Hearth Browser the default"; sub: "Links from other apps open in it"; chevron: false
        onActivated: page.hearth(["--hearth-default"])
    }
    SetChoice {
        visible: !!page.hb && page.hb.installed
        title: "Bigger buttons and text"; sub: "For a controller and a small screen. Applies the next time it opens."
        options: [{ value: "auto", label: "In Game Mode" }, { value: "big", label: "Always" }, { value: "normal", label: "Never" }]
        current: page.hb ? page.hb.layout : "auto"
        onChosen: v => page.hearth(["--hearth-layout", v])
    }

    SetHeader { text: "Aloha Browser (optional)" }
    SetRow {
        interactive: false; chevron: false
        icon: "info"; title: "Aloha Browser"
        sub: !page.st ? "Checking…" : !page.st.installed ? "Not installed" : (page.st.default ? "Your default browser" : "Installed, but another browser opens links") + (page.st.version ? " · version " + page.st.version : "")
    }
    ColumnLayout {
        visible: page.busy !== ""
        Layout.fillWidth: true
        spacing: 6
        Text { text: (page.line || "Working…") + (page.progress >= 0 ? "  " + Math.round(page.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, page.progress); height: parent.height; radius: 4; color: Theme.accent; visible: page.progress >= 0 }
        }
    }
    SetRow {
        visible: !!page.st && !page.st.installed && page.busy === ""
        icon: "update"; title: "Install Aloha Browser"; sub: "Downloads the official package from alohabrowser.com (about 150 MB) and makes it the default browser"
        onActivated: page.run("install")
    }
    SetRow {
        visible: !!page.st && page.st.installed && !page.st.default && page.busy === ""
        icon: "check"; title: "Make Aloha the default browser"; chevron: false
        onActivated: page.run("default")
    }
    SetRow {
        visible: !!page.st && page.st.installed && page.busy === ""
        icon: "restart"; title: "Look for an update"; sub: "Hearth also checks once a week"; chevron: false
        onActivated: page.run("update")
    }
    SetRow {
        visible: !!page.st && page.st.installed && page.busy === ""
        icon: "close"; danger: true; title: "Remove Aloha Browser"; sub: "Hearth won't install it again by itself"; chevron: false
        onActivated: page.run("remove")
    }
    Text {
        visible: page.error !== ""
        Layout.fillWidth: true
        text: page.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
