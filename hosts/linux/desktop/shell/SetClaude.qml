// Hearth Settings › Claude: Anthropic's Claude Desktop and Claude Code, installed for you (in your home folder) by
// the hearth-claude helper next to this file. Claude Desktop comes from Anthropic's own repository, checked
// against Anthropic's signing key, and Hearth checks for a newer one every day (hearth-claude-update.timer).
// Claude Code is Anthropic's installer, and updates itself. Sign-in for both is with a claude.ai account, in the
// browser. Also opened by the app menu's "Get Claude" (hearth-settings claude).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Claude"
    property var st: null
    property string error: ""
    property string busy: ""                 // what's running: install, update, remove, install-code, remove-code
    property bool openAfter: false
    readonly property bool unofficial: !!st && !st.official
    readonly property var labels: ({ install: "Getting Claude", update: "Updating Claude", remove: "Removing Claude",
        "install-code": "Getting Claude Code", "remove-code": "Removing Claude Code", check: "Checking for a newer Claude" })

    SetJob { id: job; helper: "hearth-claude" }

    function refresh(check) {
        job.run(check ? ["status", "--check"] : ["status"], r => page.st = r, e => { if (check) page.error = e; });
    }
    function act(cmd, thenOpen) {
        if (busy !== "") return;
        busy = cmd; error = ""; openAfter = !!thenOpen;
        job.run([cmd], r => {
            page.st = r; page.busy = "";
            if (page.openAfter && r.installed) job.run(["open"]);
        }, e => { page.error = e; page.busy = ""; page.refresh(false); });
    }
    Component.onCompleted: refresh(false)

    SetHeader { text: "Claude Desktop" }
    SetRow {
        interactive: false; chevron: false
        icon: "info"; title: "Claude Desktop"
        sub: !page.st ? "Checking…"
            : !page.st.supported ? "Anthropic makes Claude Desktop for x86-64 and ARM64 computers only"
            : page.st.apt ? "Installed through apt" + (page.st.aptVersion && page.st.aptVersion !== "?" ? " · version " + page.st.aptVersion : "") + ". apt keeps it up to date."
            : !page.st.ours ? "Anthropic's app for chatting with Claude, Cowork and Claude Code. Downloaded from Anthropic (about 170 MB)."
            : "Version " + page.st.version
              + (page.st.pending ? " · " + page.st.pending + " is ready: close Claude to finish"
                 : page.st.updateAvailable ? " · " + page.st.latest + " is available" : " · up to date")
              + " · Hearth checks for updates every day"
    }
    SetRow {
        visible: !!page.st && page.unofficial && page.st.supported
        interactive: false; chevron: false
        icon: "info"; title: "Unofficial on this edition"
        sub: "Anthropic supports its Linux app on Debian and Ubuntu. Here it's Anthropic's Debian build, run by Hearth from your home folder: it should work, but Anthropic doesn't support it yet."
    }
    SetRow {
        visible: !!page.st && page.st.supported && !page.st.sandbox
        interactive: false; chevron: false
        icon: "info"; title: "This system blocks the app's sandbox"
        sub: "Claude Desktop needs user namespaces, which every Hearth OS edition allows. It won't start until they're allowed again."
    }
    ColumnLayout {
        visible: page.busy !== ""
        Layout.fillWidth: true
        spacing: 6
        Text { text: (job.line || (page.labels[page.busy] || "Working") + "…") + (job.progress >= 0 ? "  " + Math.round(job.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, job.progress); height: parent.height; radius: 4; color: Theme.accent; visible: job.progress >= 0 }
        }
    }
    SetRow {
        visible: !!page.st && page.st.supported && !page.st.installed && page.busy === ""
        icon: "update"; title: "Get Claude"; sub: "Installs Claude Desktop for you, then opens it. Sign in with your Claude account in the browser."
        onActivated: page.act("install", true)
    }
    SetRow {
        visible: !!page.st && page.st.installed && page.busy === ""
        icon: "forward"; title: "Open Claude"; chevron: false
        onActivated: job.run(["open"], null, e => page.error = e)
    }
    SetRow {
        visible: !!page.st && page.st.ours && page.busy === ""
        icon: "restart"; chevron: false
        title: page.st && (page.st.updateAvailable || page.st.pending) ? "Update now" : "Check for an update"
        sub: page.st && page.st.running ? "Close Claude first" : ""
        onActivated: (page.st.updateAvailable || page.st.pending) ? page.act("update") : page.refresh(true)
    }
    SetRow {
        visible: !!page.st && page.st.ours && page.busy === ""
        icon: "close"; danger: true; chevron: false
        title: "Remove Claude Desktop"; sub: "Your chats are kept in your Claude account; settings stay in your home folder"
        onActivated: page.act("remove")
    }

    SetHeader { text: "Claude Code" }
    SetRow {
        interactive: false; chevron: false
        icon: "info"; title: "Claude Code"
        sub: !page.st ? "Checking…"
            : page.st.code.installed ? "Installed" + (page.st.code.version ? " · version " + page.st.code.version : "") + " · type claude in the Terminal. It keeps itself up to date."
            : "Claude in the Terminal, for coding. Installed in your home folder with Anthropic's installer."
    }
    SetRow {
        visible: !!page.st && !page.st.code.installed && page.busy === ""
        icon: "update"; title: "Get Claude Code"; sub: "Runs Anthropic's installer (claude.ai/install.sh) for you"
        onActivated: page.act("install-code")
    }
    SetRow {
        visible: !!page.st && page.st.code.installed && page.busy === ""
        icon: "forward"; title: "Open in the Terminal"; chevron: false
        onActivated: Quickshell.execDetached(["sh", "-c", "c=\"$HOME/.local/bin/claude\"; command -v hearth-terminal >/dev/null && exec hearth-terminal -e \"$c\"; exec xdg-terminal-exec \"$c\""])
    }
    SetRow {
        visible: !!page.st && page.st.code.installed && page.busy === ""
        icon: "close"; danger: true; chevron: false
        title: "Remove Claude Code"; sub: "Your settings in ~/.claude stay"
        onActivated: page.act("remove-code")
    }
    Text {
        visible: page.error !== ""
        Layout.fillWidth: true
        text: page.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
