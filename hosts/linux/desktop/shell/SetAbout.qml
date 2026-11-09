// Hearth Settings › About: what this device is, help and logs, and backing up Hearth (the last two moved from the
// archived Game Mode's Settings › System: docs/SETTINGS-MOVE.md). In the redesign its parts have their own homes, one
// file still (page.part): "about" in System › About, "privacy" (the player count) in Privacy & security, "backup" (Help
// and logs, Backup) in Updates & recovery › Backup and help. Opened on its own (part ""), it shows them all.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "About"
    property var info: ({})
    Process {
        running: true
        command: ["sh", "-c", [
            ". /etc/os-release 2>/dev/null; echo \"os=${PRETTY_NAME:-Linux}\"; echo \"ver=${IMAGE_VERSION:-$VERSION_ID}\"",
            // Hearth OS's own version and its release date (the image writes both; older images have no date).
            "echo \"hver=$(cat /usr/share/nimbus/version 2>/dev/null)\"; echo \"hdate=$(cat /usr/share/nimbus/version-date 2>/dev/null)\"",
            "echo \"host=$(cat /etc/hostname 2>/dev/null || hostname)\"",
            "echo \"model=$(cat /sys/devices/virtual/dmi/id/product_name 2>/dev/null)\"",
            "echo \"vendor=$(cat /sys/devices/virtual/dmi/id/sys_vendor 2>/dev/null)\"",
            "echo \"cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)\"",
            "echo \"cores=$(nproc 2>/dev/null)\"",
            "echo \"mem=$(awk '/MemTotal/ {printf \"%.0f GB\", $2/1048576}' /proc/meminfo)\"",
            "echo \"gpu=$(lspci 2>/dev/null | sed -n 's/.*\\(VGA\\|3D\\|Display\\)[^:]*: //p' | head -1)\"",
            "echo \"disk=$(df -h --output=avail,size /var/home 2>/dev/null | tail -1 | awk '{print $1\" free of \"$2}')\"",
            "echo \"kernel=$(uname -r)\"",
            "echo \"shell=$(quickshell --version 2>/dev/null | head -1)\"",
            "echo \"user=$(id -un)\""
        ].join("; ")]
        stdout: StdioCollector {
            onStreamFinished: {
                const o = {};
                for (const l of text.split("\n")) { const i = l.indexOf("="); if (i > 0) o[l.slice(0, i)] = l.slice(i + 1).trim(); }
                page.info = o;
            }
        }
    }
    // Hearth OS version numbers as people see them (docs/UPDATES.md, "Version numbers"): from 1.0 on MAJOR.MINOR, so
    // 1.0.0 shows as "1.0" (0.x stays whole), with the release date next to it: "1.0 · November 2, 2026".
    function shownVersion(v) {
        const m = /^(\d+)\.(\d+)\.0$/.exec(v || "");
        return m && Number(m[1]) >= 1 ? m[1] + "." + m[2] : (v || "");
    }
    function shownDate(d) {
        const m = /^(\d{4})-(\d\d)-(\d\d)$/.exec(d || "");
        return m ? new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])).toLocaleDateString(Qt.locale(), "MMMM d, yyyy") : "";
    }
    readonly property string hearthVersion: {
        if (!info.hver || info.hver === "0.0.0") return "";
        const d = shownDate(info.hdate);
        return shownVersion(info.hver) + (d ? " · " + d : "");
    }
    readonly property var facts: [
        ["Device", [info.vendor, info.model].filter(Boolean).join(" ")],
        ["Name", info.host], ["Signed in as", info.user],
        ["Processor", info.cpu ? info.cpu + (info.cores ? " (" + info.cores + " threads)" : "") : ""],
        ["Graphics", info.gpu], ["Memory", info.mem], ["Storage", info.disk],
        ["System", info.os], ["Version", hearthVersion || info.ver], ["Kernel", info.kernel], ["Desktop", info.shell ? "Mantel · " + info.shell : ""]
    ].filter(r => r[1])

    // @part about
    RowLayout {
        visible: page.inPart("about")
        Layout.fillWidth: true; Layout.bottomMargin: 10
        spacing: 16
        Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 64; Layout.preferredHeight: 64; fillMode: Image.PreserveAspectFit; smooth: true }
        ColumnLayout {
            spacing: 2
            Text { text: "Hearth"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
            Text { text: page.hearthVersion ? "Hearth OS " + page.hearthVersion : page.info.ver ? "Version " + page.info.ver : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        }
    }
    Repeater {
        model: page.facts
        delegate: SetRow {
            required property var modelData
            visible: page.inPart("about")
            interactive: false; chevron: false
            implicitHeight: 52
            title: modelData[0]; value: modelData[1]
        }
    }

    // ------------------------------------------------ Privacy: the player count (hearth-counter, sent by a systemd user
    // timer; the Hearth app sent it before). On unless turned off, as before.
    SetRow {
        visible: page.inPart("about") && !!page.info.host
        chevron: true
        icon: "edit"; title: "Rename this PC"; sub: "Its name on your network and your other devices · on Home"
        onActivated: SettingsNav.openPage("home")
    }

    // @part privacy
    SetHeader { text: "Privacy"; visible: page.inPart("privacy") }
    SetSwitch {
        visible: page.inPart("privacy")
        icon: "user"; title: "Count me in the player count"
        sub: "Hearth OS tells its counter it's running about every 10 minutes: a number made from this device, the edition and the version. No names, games or accounts."
        checked: Settings.countMe
        onToggled: on => { Settings.countMe = on; SettingsNav.toast(on ? "You're in the player count" : "You're no longer counted"); }
    }

    // ------------------------------------------------ Help and logs
    property string logsMsg: ""
    property bool sending: false
    Process {
        id: sendLogs
        command: ["nimbus", "--send-logs", "Sent from Settings › About"]
        stdout: StdioCollector { id: sendOut }
        stderr: StdioCollector { id: sendErr }
        onExited: code => {
            page.sending = false;
            const m = /code: (\S+)/.exec(sendOut.text || "");
            page.logsMsg = code === 0 && m ? "Sent. Share this code with the developer: " + m[1] : "Couldn't send them: " + ((sendErr.text || "").trim().split("\n").pop() || "the Hearth app isn't on this system");
        }
    }
    // @part backup
    SetHeader { text: "Help and logs"; visible: page.inPart("backup") }
    SetRow {
        visible: page.inPart("backup")
        icon: "file"; title: "Logs"
        sub: "Read the system's and Hearth's logs, search them, and save or send them"
        onActivated: { Quickshell.execDetached(["hearth-tools", "--app", "logs"]); SettingsNav.toast("Opening Logs…"); }
    }
    SetRow {
        visible: page.inPart("backup")
        icon: "update"; title: page.sending ? "Sending…" : "Send logs to the developer"
        sub: "Hearth's and the system's logs, personal details taken out, sent to Hearth OS's bug-report service. You get a short code to share; they're kept 14 days."
        onActivated: if (!page.sending) { page.sending = true; page.logsMsg = ""; sendLogs.running = true; }
    }
    Text {
        visible: page.logsMsg !== "" && page.inPart("backup")
        Layout.fillWidth: true
        text: page.logsMsg; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ------------------------------------------------ Backup (hearth-core backup.rs through HearthCall)
    property var backup: null      // backup.status: {destinations: [{path, label, removable}], backups}
    property string backupMsg: ""
    property bool backingUp: false
    HearthCall { id: core }
    Component.onCompleted: core.call("backup.status", {}, v => page.backup = v, e => page.backupMsg = e)
    SetHeader { text: "Backup"; visible: page.inPart("backup") }
    Repeater {
        model: page.backup ? (page.backup.destinations || []).slice(0, 4) : []
        delegate: SetRow {
            required property var modelData
            visible: page.inPart("backup")
            icon: "folder"; title: "Back up Hearth to " + modelData.label
            sub: "Settings, profiles, favorites, collections, play time and your own games. Store sign-ins stay out."
            onActivated: {
                if (page.backingUp) return;
                page.backingUp = true; page.backupMsg = "Backing up…";
                core.call("backup.create", { dest: String(modelData.path), signins: false },
                    () => { page.backingUp = false; page.backupMsg = "Backed up to " + modelData.label; core.call("backup.status", {}, v => page.backup = v); },
                    e => { page.backingUp = false; page.backupMsg = e; });
            }
        }
    }
    SetRow { chevron: true;
        icon: "restart"; title: "Restore from a backup"
        sub: (page.backup && (page.backup.backups || []).length ? page.backup.backups.length + " found. " : "") + "On a new device, or after a reinstall. Opens in the Hearth app's window for now."
        onActivated: core.openPage("system", "Opening Restore in the Hearth app…")
    }
    Text {
        visible: page.backupMsg !== "" && page.inPart("backup")
        Layout.fillWidth: true
        text: page.backupMsg; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
