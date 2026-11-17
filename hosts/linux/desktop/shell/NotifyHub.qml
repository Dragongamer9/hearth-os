pragma Singleton

// What Mantel's notifications, tray and on-screen messages know about the rest of the PC (1.14), the same under KWin
// and labwc. hearth-notifyd (started here, restarted if it stops, never needed for the shell to start) serves KDE's job
// progress (org.kde.JobViewServer), hears Unity launcher badges, the keyboard's Caps Lock / Num Lock and layout,
// removable disks, print jobs, the camera and background apps. This adds what Quickshell sees itself: the microphone
// in use, the screen being shared, a full-screen window in front.
//
// Automatic Do Not Disturb (Settings › Notifications; ~/.config/nimbus/notify.json, only what you changed is written,
// through SettingsWriter): while a full-screen window is in front, while the screen is shared or recorded, and on a
// schedule. It never touches your own Do Not Disturb switch (DeskState.dnd): it only keeps pop-ups quiet while the
// reason lasts, shows why in the bar and the notification centre, and says how many came in once it's over.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

Singleton {
    id: hub
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-notifyd").toString().replace(/^file:\/\//, ""))
    readonly property string run: Quickshell.env("XDG_RUNTIME_DIR") || "/tmp"

    // ------------------------------------------------------------------ from hearth-notifyd
    property var jobs: []                  // job views (hearth-notifyd job_view), running first
    property var badges: ({})              // desktop id → {count, countVisible, progress, progressVisible, urgent}
    property var keys: ({ caps: false, num: false, known: false })
    property var layout: ({ index: 0, layouts: [], canSwitch: false })
    property var disks: []
    property var printJobs: []
    property var cameraApps: []
    property var backgroundRaw: []
    property bool jobServer: false
    property string problem: ""
    property bool started: false           // the first lines are in (the OSDs stay quiet until then)
    signal capsToggled(bool on)
    signal numToggled(bool on)
    signal layoutSwitched(string name, string shortName)
    signal jobFinished(var job)
    signal replied(var r)

    readonly property var runningJobs: jobs.filter(j => j.state === "running")
    readonly property int jobsPercent: {
        const r = runningJobs.filter(j => j.percent >= 0);
        return r.length ? Math.round(r.reduce((s, j) => s + j.percent, 0) / r.length) : -1;
    }
    readonly property string layoutShort: { const l = layout.layouts || []; const c = l[layout.index || 0]; return c ? String(c.short || c.name || "").toUpperCase() : ""; }
    readonly property string layoutName: { const l = layout.layouts || []; const c = l[layout.index || 0]; return c ? String(c.name || c.short || "") : ""; }

    // Background apps: the ones running without a window of their own.
    readonly property var backgroundApps: {
        const open = (WinList.toplevels ? WinList.toplevels.values : []).map(t => String(t.appId || "").toLowerCase());
        return backgroundRaw.filter(a => a.app && open.indexOf(String(a.app).toLowerCase()) < 0);
    }

    // A dock icon's badge: the app's own (Unity launcher API), or the progress of its running job.
    function badgeFor(id) {
        if (!id) return null;
        const k = String(id).replace(/\.desktop$/, "");
        const b = badges[k] || badges[k.toLowerCase()] || null;
        const j = runningJobs.find(x => x.entry && (x.entry === k || x.entry.replace(/\.desktop$/, "") === k));
        if (!b && !j) return null;
        return {
            count: b && b.countVisible ? b.count : 0,
            progress: b && b.progressVisible ? b.progress : j && j.percent >= 0 ? j.percent / 100 : -1,
            urgent: !!(b && b.urgent)
        };
    }

    function send(cmd) {
        if (!proc.running) { SettingsNav.toast("That isn't available right now"); return false; }
        proc.write(JSON.stringify(cmd) + "\n");
        return true;
    }
    function jobAction(id, op) {
        if (op === "forget") { jobs = jobs.filter(j => j.id !== id); return; }
        send({ op: op, id: id });
    }
    function setLayout(i) { send({ op: "layout", index: i }); }
    function nextLayout() { const n = (layout.layouts || []).length; if (n > 1) setLayout(((layout.index || 0) + 1) % n); }
    function disk(op, path) { send({ op: op, path: path }); }

    function take(m) {
        switch (m.t) {
        case "job": {
            const rest = jobs.filter(j => j.id !== m.id);
            if (m.state !== "running") {
                jobFinished(m);
                // A finished job stays a few seconds with its result, then it's a notification in the history.
                jobs = rest.concat([m]);
                forgetLater.createObject(hub, { jid: m.id });
            } else jobs = [m].concat(rest).sort((a, b) => (a.state === "running" ? 0 : 1) - (b.state === "running" ? 0 : 1) || a.id - b.id);
            break;
        }
        case "badge": { const b = Object.assign({}, badges); b[m.app] = m; badges = b; break; }
        case "keys":
            if (started && keys.known && m.caps !== keys.caps) capsToggled(m.caps);
            if (started && keys.known && m.num !== keys.num) numToggled(m.num);
            keys = m;
            break;
        case "layout": {
            const before = layoutName;
            layout = m;
            if (started && before !== "" && layoutName !== before) layoutSwitched(layoutName, layoutShort);
            break;
        }
        case "disks": disks = m.disks || []; break;
        case "printers": printJobs = m.jobs || []; break;
        case "camera": cameraApps = m.apps || []; break;
        case "background": backgroundRaw = m.apps || []; break;
        case "status":
            jobServer = m.jobServer === true; problem = m.problem || "";
            if (problem) console.info("notifyd: " + problem);
            break;
        case "done": replied(m); if (!m.ok) SettingsNav.toast("Couldn't do that: " + (m.error || "it didn't work")); break;
        }
    }
    Component {
        id: forgetLater
        Timer {
            property int jid: 0
            interval: 8000; running: true
            onTriggered: { hub.jobs = hub.jobs.filter(j => j.id !== jid || j.state === "running"); destroy(); }
        }
    }

    property int failures: 0
    Process {
        id: proc
        command: ["python3", hub.helper, "serve", "--stdio"]
        running: true
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => {
                let m;
                try { m = JSON.parse(line); } catch (e) { return; }
                try { hub.take(m); } catch (e) { console.warn("notifyd: " + e); }
            }
        }
        onRunningChanged: if (running) startedTimer.restart()
        // Stopped: tried again a little later, a few times (the shell never waits for it).
        onExited: code => {
            hub.jobServer = false;
            if (hub.failures++ < 8) again.interval = 2000 * hub.failures, again.restart();
            else console.warn("notifyd: hearth-notifyd keeps stopping (code " + code + "); job progress and the tray's extras are off until the shell restarts");
        }
    }
    Timer { id: again; onTriggered: proc.running = true }
    Timer { id: startedTimer; interval: 2500; onTriggered: { hub.started = true; hub.failures = 0; } }

    // ------------------------------------------------------------------ what Quickshell sees itself
    PwObjectTracker { objects: [Pipewire.defaultAudioSource] }
    readonly property var mic: Pipewire.defaultAudioSource ? Pipewire.defaultAudioSource.audio : null
    readonly property bool micMuted: !!mic && mic.muted
    function prop(n, k) { try { return n.properties ? (n.properties[k] || "") : ""; } catch (e) { return ""; } }
    function appOf(n) { return prop(n, "application.name") || prop(n, "application.process.binary") || n.description || n.name || "An app"; }
    // Apps recording from a microphone (not Hearth's own level meters).
    readonly property var micApps: {
        const all = Pipewire.nodes ? Pipewire.nodes.values : [];
        const seen = [];
        for (const n of all) {
            if (prop(n, "media.class") !== "Stream/Input/Audio") continue;
            const a = appOf(n), bin = prop(n, "application.process.binary");
            if (/quickshell|pavucontrol|wireplumber|pipewire/i.test(a + " " + bin) || prop(n, "stream.monitor") === "true") continue;
            if (seen.indexOf(a) < 0) seen.push(a);
        }
        return seen;
    }
    // The screen going out: a screencast (the portal's video source, KWin's or wlroots') or Hearth's own recorder and
    // Remote desktop (their runtime files, as CaptureIndicator.qml reads them).
    readonly property var castNodes: {
        const all = Pipewire.nodes ? Pipewire.nodes.values : [];
        return all.filter(n => prop(n, "media.class") === "Video/Source" && prop(n, "device.api") === "" && !/v4l2|libcamera/.test(prop(n, "node.name") + prop(n, "factory.name")));
    }
    property bool recording: false
    property bool remoteViewers: false
    FileView {
        path: hub.run + "/hearth-record/state.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { const s = JSON.parse(text()); hub.recording = s.state === "recording" || s.state === "stopping"; } catch (e) { hub.recording = false; } }
        onLoadFailed: hub.recording = false
    }
    FileView {
        path: hub.run + "/hearth-vnc/clients.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { const s = JSON.parse(text()); hub.remoteViewers = (s.clients || []).length > 0; } catch (e) { hub.remoteViewers = false; } }
        onLoadFailed: hub.remoteViewers = false
    }
    property bool testSharing: false       // `ipc call notify sharing on`: CI's stand-in for a screencast
    readonly property bool sharing: testSharing || castNodes.length > 0 || recording || remoteViewers

    // ------------------------------------------------------------------ automatic Do Not Disturb
    readonly property string prefsPath: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/notify.json"
    property var prefs: ({})
    readonly property var defaults: ({ autoFullscreen: true, autoSharing: true, schedule: false, scheduleFrom: "22:00", scheduleTo: "07:00", keepHistory: true })
    function pref(k) { return prefs[k] !== undefined && typeof prefs[k] === typeof defaults[k] ? prefs[k] : defaults[k]; }
    function setPref(k, v) {
        if (pref(k) === v) return;
        const p = Object.assign({}, prefs); p[k] = v; prefs = p;
        const o = {}; o[k] = v; writer.merge(o);
    }
    SettingsWriter { id: writer; path: hub.prefsPath; history: true }
    FileView {
        id: prefsFile
        path: hub.prefsPath
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") { console.warn("notify: notify.json couldn't be read; putting back the last good copy"); writer.repair(); }
                return;
            }
            const p = Object.assign({}, o);
            for (const k in hub.prefs) if (writer.owns(k)) p[k] = hub.prefs[k];
            hub.prefs = p;
        }
    }

    SystemClock { id: clock; precision: SystemClock.Minutes }
    function minutes(s, d) { const m = /^(\d{1,2}):(\d{2})$/.exec(String(s || "")); return m ? Math.min(23, +m[1]) * 60 + Math.min(59, +m[2]) : d; }
    function inSchedule(now) {
        const m = now.getHours() * 60 + now.getMinutes();
        const a = minutes(pref("scheduleFrom"), 1320), b = minutes(pref("scheduleTo"), 420);
        return a === b ? false : a < b ? (m >= a && m < b) : (m >= a || m < b);
    }
    readonly property var front: WinList.activeToplevel
    readonly property bool fullscreenFront: !!front && front.fullscreen === true && !/quickshell/i.test(front.appId || "")
    property bool testFullscreen: false
    // Why pop-ups are quiet by themselves right now ("" = they aren't).
    readonly property string autoReason: pref("autoSharing") && sharing ? "sharing"
        : pref("autoFullscreen") && (fullscreenFront || testFullscreen) ? "fullscreen"
        : pref("schedule") && inSchedule(clock.date) ? "schedule" : ""
    readonly property string autoLabel: autoReason === "sharing" ? "while you share your screen"
        : autoReason === "fullscreen" ? "while a full-screen app is in front"
        : autoReason === "schedule" ? "until " + pref("scheduleTo") : ""
    readonly property bool quiet: DeskState.dnd || autoReason !== ""
    property int missed: 0                 // came in while it was quiet by itself
    property string missedReason: ""
    function noteMissed() { if (!DeskState.dnd && autoReason !== "") { missed++; missedReason = autoLabel; } }
    signal quietEnded(int count, string why)
    onAutoReasonChanged: {
        if (autoReason === "" && missed > 0) { quietEnded(missed, missedReason); missed = 0; }
    }
}
