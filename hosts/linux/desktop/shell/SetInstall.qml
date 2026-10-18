// Hearth Settings › Install next to Windows (live USB only): checks the PC (Windows found, BitLocker, Fast
// Startup, charger, internet, games on it), lets you pick how much space Hearth gets, makes that room by
// shrinking Windows (hearth-live-prep), then opens the installer with "Use free space". After installing,
// Hearth offers to bring the games over on its first start (Settings › Import games).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Install next to Windows"
    readonly property string helper: "/usr/libexec/nimbus/hearth-live-prep"
    property var info: null          // hearth-live-prep check
    property string phase: "idle"    // idle | checking | shrinking | ready
    property real progress: -1
    property string line: ""
    property string error: ""
    property real give: 0
    readonly property real gib: 1024 * 1024 * 1024
    readonly property var blockers: info ? info.problems.filter(p => !p.soft) : []
    readonly property var warnings: info ? info.problems.filter(p => p.soft) : []

    function gb(b) { return Math.round(b / gib) + " GB"; }
    function run(args) {
        if (proc.running) return;
        error = ""; line = ""; progress = -1;
        proc.command = ["pkexec", helper].concat(args);
        proc.running = true;
    }
    function check() { phase = "checking"; info = null; run(["check"]); }
    function sizes() {
        if (!info || !info.windows) return [];
        const max = Math.floor(info.windows.canGive / gib);
        const out = [];
        const steps = [64, 128, 256, 512, 1024];
        for (let i = 0; i < steps.length; i++) if (steps[i] < max) out.push({ value: steps[i] * gib, label: steps[i] + " GB" });
        out.push({ value: max * gib, label: "All it can (" + max + " GB)" });
        return out;
    }
    Component.onCompleted: check()

    Process {
        id: proc
        stdout: SplitParser {
            onRead: l => {
                l = l.trim();
                if (l.startsWith("error: ")) page.error = l.slice(7);
                else if (l.startsWith("progress: ")) {
                    const m = l.match(/^progress: ([\d.]+) ?(.*)$/);
                    if (m) { page.progress = parseFloat(m[1]) / 100; if (m[2]) page.line = m[2]; }
                } else if (l.startsWith("{")) {
                    try {
                        const j = JSON.parse(l);
                        if (page.phase === "checking") { page.info = j; page.give = j.suggested || 0; }
                        else if (j.ok) page.phase = "ready";
                    } catch (e) {}
                }
            }
        }
        onExited: code => {
            if (page.phase === "checking" || (page.phase === "shrinking" && page.error)) page.phase = "idle";
            page.progress = -1;
        }
    }

    // ---------------------------------------------------------------- explanation
    Text {
        Layout.fillWidth: true; wrapMode: Text.WordWrap
        text: "Keep Windows and your games, and add Hearth next to it. Hearth gets part of Windows' drive, and after installing it brings your Steam, Epic and GOG games over (Settings › Import games). Later you can remove Windows and give Hearth the whole drive."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }

    SetRow {
        visible: page.phase === "checking"
        interactive: false; chevron: false
        icon: "update"; title: "Checking this PC…"; sub: "Looking for Windows, how much room it has, and your games"
    }

    // ---------------------------------------------------------------- what was found
    SetRow {
        visible: !!(page.info && page.info.windows) && page.phase !== "checking"
        interactive: false; chevron: false
        icon: "display"
        title: page.info && page.info.windows ? "Windows · " + page.gb(page.info.windows.size) : ""
        sub: page.info && page.info.windows ? page.gb(page.info.windows.free) + " free · can give Hearth up to " + page.gb(page.info.windows.canGive) + " (keeps 20 GB free for Windows)" : ""
    }
    SetRow {
        visible: !!(page.info && page.info.games && page.info.games.count > 0)
        interactive: false; chevron: false
        icon: "pad"
        title: page.info && page.info.games ? page.info.games.count + " games to bring over · " + page.gb(page.info.games.size || 0) : ""
        sub: page.info && page.info.games && page.info.games.names ? page.info.games.names.join(", ") : ""
    }
    Repeater {
        model: page.blockers
        delegate: SetRow {
            required property var modelData
            interactive: false; chevron: false; danger: true
            icon: "lock"; title: "Can't continue yet"; sub: modelData.text
        }
    }
    Repeater {
        model: page.warnings
        delegate: SetRow {
            required property var modelData
            interactive: false; chevron: false
            icon: "info"; title: modelData.id === "power" ? "Plug in the charger" : "No internet"; sub: modelData.text
        }
    }
    SetRow {
        visible: page.phase === "idle" && !!page.info && page.blockers.length > 0
        icon: "restart"; title: "Check again"; sub: "After fixing the above (or restarting into Windows and back)"
        onActivated: page.check()
    }

    // ---------------------------------------------------------------- choose and go
    SetChoice {
        visible: page.phase === "idle" && !!page.info && page.blockers.length === 0
        title: "Space for Hearth"
        sub: "Games you bring over need room too. Windows keeps the rest."
        options: page.sizes()
        current: page.give
        onChosen: v => page.give = v
    }
    SetRow {
        visible: page.phase === "idle" && !!page.info && page.blockers.length === 0
        icon: "update"; title: "Make room for Hearth (" + page.gb(page.give) + ")"
        sub: "Shrinks Windows' drive. Your files stay, but back up anything important first: resizing a drive is never completely risk-free. Takes a few minutes, longer on a full drive."
        onActivated: { page.phase = "shrinking"; page.run(["shrink", String(Math.round(page.give))]); }
    }

    SetRow {
        visible: page.phase === "idle" && !!page.info
        icon: "update"; title: "Already made room in Windows?"
        sub: "If Ember's Switch to Hearth OS already shrank Windows, go straight to the installer and choose \"Use free space\"."
        onActivated: Quickshell.execDetached(["sh", "-c", "command -v liveinst >/dev/null && exec liveinst || exec anaconda-webui"])
    }

    // ---------------------------------------------------------------- progress
    ColumnLayout {
        visible: page.phase === "shrinking"
        Layout.fillWidth: true; Layout.topMargin: 6
        spacing: 8
        Text { text: page.line || "Starting…"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0, page.progress); height: parent.height; radius: 4; color: Theme.accent }
        }
        Text { text: "Don't turn off the device or unplug it until this is done."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    }
    SetRow {
        visible: page.error !== ""
        interactive: false; chevron: false; danger: true
        icon: "info"; title: "That didn't work"; sub: page.error
    }

    // ---------------------------------------------------------------- ready
    Rectangle {
        visible: page.phase === "ready"
        Layout.fillWidth: true
        implicitHeight: readyText.implicitHeight + 32; radius: 12
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18); border.color: Theme.accent
        Text {
            id: readyText
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 16 }
            wrapMode: Text.WordWrap
            text: "There's room for Hearth now. In the installer, pick the drive and choose \"Use free space\" so Windows stays. After the restart, Hearth offers to bring your games over."
            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
        }
    }
    SetRow {
        visible: page.phase === "ready"
        icon: "update"; title: "Open the installer"
        onActivated: Quickshell.execDetached(["sh", "-c", "command -v liveinst >/dev/null && exec liveinst || exec anaconda-webui"])
    }
}
