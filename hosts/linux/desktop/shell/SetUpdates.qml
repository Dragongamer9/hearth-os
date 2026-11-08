// Hearth Settings › Updates: the running version, look for an update, download it (with progress). The
// update is put in place for the next restart. Runs hearth-admin (pkexec). Go back to the version before (bootc keeps
// it; hearth-admin rollback makes it start next, and again before restarting undoes that): moved from the archived Game
// Mode's Settings › Updates (docs/SETTINGS-MOVE.md).
// On Peppermint also the writable system (hearth-unlock): the Unlocked switch (locking applies from the next restart),
// Turn my changes back on after a safe start, Put my packages back again, and the notice Hearth shows.
// Decky plugins (docs/PLUGIN-UPDATES.md): Hearth's Decky plugins update without a system update (hearth-decky plugins-*,
// as you, no password): each plugin's version and where it's from, Check now, Update, Use the built-in plugins, the
// last error, and "Update plugins automatically" (off unless you turn it on; saved with SettingsWriter). New plugins
// from a plugins release are listed under "New from Hearth" and installed only with "Also install new Hearth plugins"
// (off unless you tick it; saved with SettingsWriter, never changed by itself).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Updates"
    readonly property string helper: "/usr/libexec/nimbus/hearth-admin"
    property bool canAdmin: false
    property var status: null        // {booted, staged, available}
    property string phase: "idle"    // idle | checking | updating
    property real progress: -1
    property string line: ""
    property string error: ""

    // From 1.0 on Hearth OS is MAJOR.MINOR: 1.0.0 shows as "1.0" (0.x stays whole; docs/UPDATES.md, "Version numbers").
    function shown(v) { const m = /^(\d+)\.(\d+)\.0$/.exec(v || ""); return m && Number(m[1]) >= 1 ? m[1] + "." + m[2] : v; }
    function ver(i) { return i && i.version ? shown(i.version) : (i && i.timestamp ? i.timestamp.slice(0, 10) : ""); }
    property string lastCmd: ""
    function start(cmd) {
        if (admin.running) return;
        lastCmd = cmd;
        error = ""; line = cmd === "update" ? "Starting…" : cmd === "update-check" ? "Looking for updates…" : "";
        phase = cmd === "update" ? "updating" : cmd === "update-check" ? "checking" : "idle";
        progress = cmd === "update" ? 0 : -1;
        admin.command = ["pkexec", helper, cmd];
        admin.running = true;
    }
    // ---- the writable system (Hearth OS on Peppermint)
    property var unlock: null        // hearth-unlock --status --json; null where there's none (Anatase)
    property string unlockError: ""
    function unlockLoad() { unlockStatus.running = true; }
    function unlockSet(action) {
        if (unlockAdmin.running) return;
        unlockError = "";
        unlockAdmin.command = ["pkexec", helper, "unlock", action];
        unlockAdmin.running = true;
    }
    Process {
        id: unlockStatus
        running: true
        command: ["sh", "-c", "[ -x /usr/bin/hearth-unlock ] && exec /usr/bin/hearth-unlock --status --json"]
        stdout: SplitParser { onRead: l => { if (l.startsWith("{")) { try { page.unlock = JSON.parse(l); } catch (e) {} } } }
    }
    Process {
        id: unlockAdmin
        stdout: SplitParser {
            onRead: l => {
                if (l.startsWith("error: ")) page.unlockError = l.slice(7);
                else if (l.startsWith("{")) { try { page.unlock = JSON.parse(l); } catch (e) {} }
            }
        }
        onExited: code => { if (code === 126 || code === 127) page.unlockError = "Not allowed (Hearth's polkit rule is missing)."; page.unlockLoad(); }
    }

    Process {
        running: true
        command: ["test", "-x", page.helper]
        onExited: code => { page.canAdmin = code === 0; if (page.canAdmin) page.start("update-status"); }
    }
    Process {
        id: admin
        stdout: SplitParser {
            onRead: l => {
                l = l.trim();
                if (l.startsWith("log: ")) page.line = l.slice(5);
                else if (l.startsWith("error: ")) page.error = l.slice(7);
                else if (l.startsWith("progress: ")) {
                    const m = l.match(/^progress: ([\d.]+) ?(.*)$/);
                    if (m) { page.progress = parseFloat(m[1]) / 100; if (m[2]) page.line = m[2]; }
                } else if (l.startsWith("{") && page.lastCmd !== "rollback") { try { page.status = JSON.parse(l); } catch (e) {} }
            }
        }
        onExited: code => {
            if (code === 126 || code === 127) page.error = "Not allowed (Hearth's polkit rule is missing).";
            page.phase = "idle"; page.progress = -1; page.line = "";
            // rollback prints bootc's own status: read Hearth's summary of it again.
            if (page.lastCmd === "rollback") Qt.callLater(() => page.start("update-status"));
        }
    }

    // ---- The status card (the redesign): where Hearth stands and the one thing to do next. A (or a click) does it.
    readonly property bool staged: !!(page.status && page.status.staged)
    readonly property bool available: !!(page.status && page.status.available) && !page.staged
    readonly property string heroAction: !page.canAdmin || page.phase !== "idle" ? "" : page.staged ? "Restart now"
        : page.available ? "Download" : "Look for updates"
    function heroDo() {
        if (page.heroAction === "") return;
        if (page.staged) Quickshell.execDetached(["systemctl", "reboot"]);
        else if (page.available) page.start("update");
        else page.start("update-check");
    }
    SetBase {
        id: hero
        Layout.fillWidth: true
        Layout.bottomMargin: 4
        // Focusable whenever updates are made here (never turned off while it has the focus: Qt refuses that)
        interactive: page.canAdmin
        implicitHeight: Math.max(96, heroText.implicitHeight + 32)
        radius: SetStyle.bigRadius
        onActivated: page.heroDo()
        Accessible.name: heroTitle.text
        Accessible.description: heroSub.text
        RowLayout {
            anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
            spacing: 18
            Rectangle {
                Layout.preferredWidth: 56; Layout.preferredHeight: 56; radius: Math.max(8, Theme.r(12))
                color: SetStyle.tint(Theme.accent, 0.18)
                Icon { anchors.centerIn: parent; name: page.staged || page.available || page.phase !== "idle" ? "update" : page.canAdmin ? "check" : "info"; size: 28; color: Theme.accent }
            }
            ColumnLayout {
                id: heroText
                Layout.fillWidth: true
                spacing: 3
                Text {
                    id: heroTitle
                    Layout.fillWidth: true; elide: Text.ElideRight
                    text: !page.canAdmin ? "Updates come with Hearth OS"
                        : page.phase === "updating" ? "Downloading the update…" : page.phase === "checking" ? "Looking for updates…"
                        : page.staged ? "Hearth " + page.ver(page.status.staged) + " is ready. Restart to finish updating."
                        : page.available ? "Hearth " + page.ver(page.status.available) + " is ready to download"
                        : page.status ? "You're up to date" : "Hearth"
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold
                }
                Text {
                    id: heroSub
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: !page.canAdmin ? "System updates are made here on Hearth, the OS."
                        : page.available && !page.staged && page.phase === "idle" ? "Keeps working while it downloads; finishes when you restart"
                        : ["Hearth OS " + page.ver(page.status && page.status.booted),
                           page.status && page.status.booted && page.status.booted.image ? page.status.booted.image : "Running now"].filter(x => x).join(" · ")
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
            }
            Rectangle {
                visible: page.heroAction !== ""
                implicitWidth: heroBtn.implicitWidth + 32; implicitHeight: 38
                radius: Math.max(6, Theme.r(8))
                color: Theme.accent
                Text { id: heroBtn; anchors.centerIn: parent; text: page.heroAction; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
            }
        }
    }

    // progress
    ColumnLayout {
        visible: page.phase !== "idle"
        Layout.fillWidth: true; Layout.topMargin: 6
        spacing: 8
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: page.line; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
            Text { visible: page.progress >= 0; text: Math.floor(page.progress * 100) + "%"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        }
        Item {
            Layout.fillWidth: true; implicitHeight: 8
            Rectangle { anchors.fill: parent; radius: 4; color: Theme.panel3 }
            Rectangle {
                id: bar
                height: parent.height; radius: 4
                width: page.progress >= 0 ? Math.max(8, parent.width * page.progress) : parent.width * 0.3
                x: 0
                color: Theme.accent
                Behavior on width { NumberAnimation { duration: 300 } }
                SequentialAnimation on x {
                    running: page.phase !== "idle" && page.progress < 0; loops: Animation.Infinite
                    NumberAnimation { from: 0; to: bar.parent.width * 0.7; duration: 900; easing.type: Easing.InOutQuad }
                    NumberAnimation { from: bar.parent.width * 0.7; to: 0; duration: 900; easing.type: Easing.InOutQuad }
                }
            }
        }
    }
    Text {
        visible: page.error !== ""
        Layout.fillWidth: true
        text: page.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { visible: (page.canAdmin && page.phase === "idle" && !page.available && !page.staged) || !!page.plugins; text: "Updates" }
    SetRow {
        visible: page.canAdmin && page.phase === "idle" && !page.available && !page.staged
        icon: "update"; title: "Update anyway"; sub: "Downloads anything new for Hearth and its apps, even small fixes"
        onActivated: page.start("update")
    }
    // ---- Decky plugins (docs/PLUGIN-UPDATES.md)
    readonly property string decky: "/usr/libexec/nimbus/hearth-decky"
    property var plugins: null       // hearth-decky plugins-status: {auto, installNew, plugins: {name: {name, installed, builtin, source}}, new: {id: {name, description, version}}, lastError, lastCheck, available}
    property string pluginsLine: ""
    property string pluginsCmd: ""
    readonly property var pluginNames: ({ "decky-hearth-sync": "Hearth Sync", "decky-hearth-link": "Hearth Link" })
    function pluginsRun(cmd) {
        if (pluginsProc.running) return;
        pluginsCmd = cmd;
        pluginsLine = cmd === "plugins-check" ? "Looking for plugin updates…" : cmd === "plugins-update" ? "Updating the plugins…"
            : cmd === "plugins-builtin" ? "Going back to the built-in plugins…" : "";
        pluginsProc.command = ["python3", decky, cmd];
        pluginsProc.running = true;
    }
    function pluginsAfter(r) {
        if (!r) return;
        if (pluginsCmd === "plugins-status") { page.plugins = r; return; }
        pluginsLine = r.error ? "" : r.restarted ? "Done. Decky restarted with the new plugins."
            : r.applies ? "Ready. Put in place " + r.applies + "."
            : pluginsCmd === "plugins-check" ? (Object.keys(r.available || {}).length ? "Plugin updates are ready to install."
                : Object.keys(r["new"] || {}).length ? "The plugins are up to date. New plugins are listed below." : "The plugins are up to date.") : "";
    }
    function pluginsAvailable() { return page.plugins ? Object.keys(page.plugins.available || {}).length : 0; }
    function pluginsNew() { return page.plugins && page.plugins["new"] ? Object.keys(page.plugins["new"]) : []; }
    Process {
        id: pluginsProc
        stdout: SplitParser { onRead: l => { if (l.startsWith("{")) { try { page.pluginsAfter(JSON.parse(l)); } catch (e) {} } else if (l.startsWith("error: ")) page.pluginsLine = l.slice(7); } }
        onExited: code => { if (page.pluginsCmd !== "plugins-status") { page.pluginsCmd = ""; Qt.callLater(() => page.pluginsRun("plugins-status")); } }
    }
    Process {
        running: true
        command: ["test", "-f", page.decky]
        onExited: code => { if (code === 0) page.pluginsRun("plugins-status"); }
    }
    SettingsWriter { id: pluginsWriter; path: Quickshell.env("HOME") + "/.config/nimbus/plugin-updates.json"; onSaved: page.pluginsRun("plugins-status") }

    SetExpander {
        id: deckyX
        visible: !!page.plugins
        anchor: "decky"
        icon: "box"; title: "Decky plugins"; sub: "Hearth's plugins for Steam Game Mode, updated without a system update"
        value: page.pluginsAvailable() ? page.pluginsAvailable() + (page.pluginsAvailable() === 1 ? " update" : " updates") : page.plugins && page.plugins.lastCheck ? "Up to date" : ""
    Repeater {
        model: page.plugins ? Object.keys(page.plugins.plugins || {}) : []
        SetRow {
            required property string modelData
            readonly property var p: page.plugins.plugins[modelData] || {}
            Layout.fillWidth: true
            interactive: false; chevron: false; icon: "update"
            title: (page.pluginNames[modelData] || p.name || modelData) + " " + (p.installed || p.builtin || "")
            sub: (p.source === "downloaded" ? (p.builtin ? "Updated (built-in: " + p.builtin + ")" : "Added from Hearth") : p.source === "built-in" ? "Built in" : "Not installed yet")
                + (page.plugins.available && page.plugins.available[modelData] ? " · " + page.plugins.available[modelData] + " available" : "")
                + (p.downloaded && p.downloaded !== p.installed ? " · " + p.downloaded + " ready for the next Steam Game Mode start" : "")
        }
    }
    SetHeader { visible: page.pluginsNew().length > 0; text: "New from Hearth"; Layout.leftMargin: 56 }
    Repeater {
        model: page.pluginsNew()
        SetRow {
            required property string modelData
            readonly property var n: (page.plugins && page.plugins["new"] && page.plugins["new"][modelData]) || {}
            Layout.fillWidth: true
            interactive: false; chevron: false; icon: "update"
            title: (n.name || modelData) + " " + (n.version || "")
            sub: (n.description ? n.description + " · " : "") + (page.plugins && page.plugins.installNew ? "Installed with the next update" : "Tick \"Also install new Hearth plugins\" to get it")
        }
    }
    SetSwitch {
        visible: !!page.plugins
        icon: "update"; title: "Also install new Hearth plugins"
        checked: !!(page.plugins && page.plugins.installNew)
        sub: "Update (and automatic updates, when on) also installs new plugins from Hearth, checked the same way. Off: they're only listed"
        onToggled: on => { page.plugins = Object.assign({}, page.plugins, { installNew: on }); pluginsWriter.merge({ installNew: on }); }
    }
    SetRow {
        visible: !!page.plugins
        icon: "restart"; title: "Check now"
        sub: page.pluginsLine !== "" ? page.pluginsLine : page.plugins && page.plugins.lastCheck ? "Last checked " + new Date(page.plugins.lastCheck * 1000).toLocaleString(Qt.locale(), Locale.ShortFormat) : "Not checked yet"
        onActivated: page.pluginsRun("plugins-check")
    }
    SetRow {
        visible: !!page.plugins
        icon: "update"; title: page.pluginsAvailable() ? "Update the plugins" : "Update"
        sub: "Downloads and checks newer plugins; Decky restarts with them if Steam Game Mode is on (Steam keeps running)"
        onActivated: page.pluginsRun("plugins-update")
    }
    SetSwitch {
        visible: !!page.plugins
        icon: "update"; title: "Update plugins automatically"
        checked: !!(page.plugins && page.plugins.auto)
        sub: "Checks once a day while online; a new plugin is put in place at the next Steam Game Mode start, or right away when no game is running"
        onToggled: on => { page.plugins = Object.assign({}, page.plugins, { auto: on }); pluginsWriter.merge({ auto: on }); }
    }
    SetRow {
        visible: !!page.plugins
        icon: "restart"; title: "Use the built-in plugins"
        sub: "The plugins that came with this Hearth OS version come back (those downloads aren't fetched again by themselves)"
        onActivated: page.pluginsRun("plugins-builtin")
    }
    Text {
        visible: !!(page.plugins && page.plugins.lastError)
        Layout.fillWidth: true
        text: page.plugins && page.plugins.lastError ? page.plugins.lastError : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        Layout.leftMargin: 56; Layout.rightMargin: 16; Layout.bottomMargin: 10
    }
    }
    // ---- Go back to the version before (shown only when the system reports one, and no update waits for a restart)
    readonly property bool canGoBack: page.canAdmin && page.phase === "idle" && !!(page.status && page.status.rollback && page.ver(page.status.rollback)) && !(page.status && page.status.staged)
    property bool askBack: false
    SetHeader { visible: page.canAdmin; text: "If something broke" }
    SetRow {
        visible: page.canGoBack && !page.status.rollbackQueued
        icon: "restart"; title: "Go back to " + page.ver(page.status ? page.status.rollback : null)
        sub: "If something broke after the last update: the version before this one starts at the next restart. Your games and files stay."
        onActivated: page.askBack = !page.askBack
    }
    SetRow {
        visible: page.canGoBack && !page.status.rollbackQueued && page.askBack
        icon: "check"; title: "Yes, go back at the next restart"
        sub: "Hearth updates again by itself later. Changed your mind before restarting? Stay on " + page.ver(page.status ? page.status.booted : null) + " shows up here."
        onActivated: { page.askBack = false; page.start("rollback"); }
    }
    SetRow {
        visible: page.canGoBack && !!page.status.rollbackQueued
        icon: "restart"; title: "Restart to go back to " + page.ver(page.status ? page.status.rollback : null)
        sub: "It starts at the next restart. Your games and files stay."
        onActivated: Quickshell.execDetached(["systemctl", "reboot"])
    }
    SetRow {
        visible: page.canGoBack && !!page.status.rollbackQueued
        icon: "close"; title: "Stay on " + page.ver(page.status ? page.status.booted : null); sub: "Don't go back after all"
        onActivated: page.start("rollback")
    }
    SetRow {
        visible: page.canAdmin
        interactive: false
        icon: "shield"; title: "The previous version always stays startable"
        sub: "Every update keeps the version before it in the boot menu"
        chip: "Kept"
    }
    SetHeader { visible: !!(page.unlock && page.unlock.available); text: "Writable system" }
    SetRow {
        visible: !!(page.unlock && page.unlock.available && page.unlock.notice)
        interactive: false; chevron: false; icon: "info"
        title: page.unlock && page.unlock.notice ? page.unlock.notice.title : ""
        sub: page.unlock && page.unlock.notice ? page.unlock.notice.text : ""
    }
    SetRow {
        visible: !!(page.unlock && page.unlock.available && page.unlock.notice)
        icon: "check"; chevron: false; title: "OK, hide this"
        onActivated: page.unlockSet("dismiss")
    }
    SetSwitch {
        visible: !!(page.unlock && page.unlock.available)
        icon: "lock"; title: "Unlocked system"
        checked: !!(page.unlock && page.unlock.unlocked)
        sub: !page.unlock ? "" : page.unlock.state === "unlocked" ? "apt and .deb files install anywhere, kept across restarts. What you add comes back by itself after an update."
            : page.unlock.state === "locks-at-restart" ? "Locked from the next restart: the system as the image made it. Your packages stay saved."
            : page.unlock.state === "unlocks-at-restart" ? "Writable from the next restart."
            : page.unlock.state === "suspended" ? "This start is without your changes (safe start)."
            : page.unlock.state === "repairing" ? "This start is without your changes while an install that was cut off is finished."
            : "The system is as the image made it. Unlock to install with apt again; your saved packages come back."
        onToggled: on => page.unlockSet(on ? "on" : "off")
    }
    SetRow {
        visible: !!(page.unlock && page.unlock.available && page.unlock.suspended && page.unlock.unlocked)
        icon: "restart"; title: "Turn my changes back on"
        sub: "Uses the packages you added again, now. If Hearth then doesn't start, it starts without them again by itself."
        onActivated: page.unlockSet("resume")
    }
    SetRow {
        visible: !!(page.unlock && page.unlock.available)
        interactive: false; chevron: false; icon: "update"
        title: "Your packages"
        sub: !page.unlock ? "" : (page.unlock.packages || []).length ? (page.unlock.packages || []).join(", ") : "Nothing added yet"
    }
    SetRow {
        visible: !!(page.unlock && page.unlock.available && page.unlock.writable && ((page.unlock.packages || []).length || (page.unlock.removed || []).length))
        icon: "restart"; title: "Put my packages back again"; sub: "Reinstalls what you added on this version, in the background"
        onActivated: page.unlockSet("reapply")
    }
    Text {
        visible: !!(page.unlock && page.unlock.available)
        Layout.fillWidth: true
        text: page.unlockError !== "" ? page.unlockError : "If something you install stops Hearth from starting, the next start leaves your changes off by itself, and the previous version in the boot menu keeps its own packages. Want a system that can't be changed at all? That's Hearth OS on Anatase."
        color: page.unlockError !== "" ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
