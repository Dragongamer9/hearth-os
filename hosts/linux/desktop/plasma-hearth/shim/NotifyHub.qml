pragma Singleton

// The notification-side state Mantel's bar shows (plasma-hearth). Quiet = Do not disturb (desk.json, as Mantel).
//   * Copies and downloads and the keyboard layout: Plasma's own (PlasmaHub.qml).
//   * Background apps, the camera, printing, removable disks, Caps Lock / Num Lock and dock badges: Mantel's own
//     hearth-notifyd, through hearth-plasma-panel's go-between (notifyd-wait: asleep until something changes;
//     notifyd-send: the bar's asks). Nothing polls. Without it those buttons stay hidden, as on a PC with nothing
//     of the kind going on.
//   * The microphone: Plasma's own sound service (PaMic.qml: the default input and the apps recording from it).
import QtQuick

Item {
    id: hub
    readonly property bool quiet: DeskState.dnd
    property string autoReason: ""
    property string autoLabel: ""
    property int missed: 0
    Loader { id: ph; source: "PlasmaHub.qml"; onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: keyboard layout and job progress aren't there (PlasmaHub.qml)") }
    readonly property var jobs: (ph.item ? ph.item.jobs : []).concat(njobs)
    readonly property var runningJobs: jobs.filter(j => j.state === "running")
    readonly property int jobsPercent: { const r = runningJobs.filter(j => j.percent >= 0); return r.length ? Math.round(r.reduce((a, j) => a + j.percent, 0) / r.length) : -1; }
    // ---- from hearth-notifyd (Mantel's take(), the same messages)
    property var njobs: []
    property var badges: ({})
    property var keys: ({ caps: false, num: false, known: false })
    property var disks: []
    property var printJobs: []
    property var cameraApps: []
    property var backgroundRaw: []
    property bool started: false
    property bool jobServer: false
    readonly property var backgroundApps: {
        const open = (WinList.toplevels ? WinList.toplevels.values : []).map(t => String(t.appId || "").toLowerCase());
        return backgroundRaw.filter(a => a.app && open.indexOf(String(a.app).toLowerCase()) < 0);
    }
    function take(m) {
        switch (m.t) {
        case "job": {
            const rest = njobs.filter(j => j.id !== m.id);
            if (m.state !== "running") { jobFinished(m); njobs = rest; }
            else njobs = [m].concat(rest);
            break;
        }
        case "badge": { const b = Object.assign({}, badges); b[m.app] = m; badges = b; break; }
        case "keys":
            if (started && keys.known && m.caps !== keys.caps) capsToggled(m.caps);
            if (started && keys.known && m.num !== keys.num) numToggled(m.num);
            keys = m;
            break;
        case "disks": disks = m.disks || []; break;
        case "printers": printJobs = m.jobs || []; break;
        case "camera": cameraApps = m.apps || []; break;
        case "background": backgroundRaw = m.apps || []; break;
        case "status": jobServer = m.jobServer === true; if (m.problem) console.info("plasma-hearth: notifyd: " + m.problem); break;
        case "done": if (!m.ok) SettingsNav.toast("Couldn't do that: " + (m.error || "it didn't work")); break;
        }
    }
    property int seq: 0
    property int fails: 0
    function wait() {
        Hearth.call(["python3", Hearth.helper, "notifyd-wait", String(seq)], (out, code) => {
            let o = null;
            try { o = JSON.parse(out.trim().split("\n").pop() || "null"); } catch (e) { o = null; }
            if (o && o.missing) return;                       // no hearth-notifyd here: those buttons stay hidden
            if (o && Array.isArray(o.msgs)) {
                if (o.seq < seq) seq = 0;
                for (const m of o.msgs) { try { take(m); } catch (e) { console.warn("plasma-hearth: notifyd: " + e); } }
                seq = Number(o.seq) || seq;
                fails = 0;
                if (!startedT.running && !started) startedT.start();
                wait();
            } else if (fails++ < 8) again.restart();
            else console.warn("plasma-hearth: hearth-notifyd keeps stopping; the tray's extras are off until Plasma restarts");
        });
    }
    Timer { id: again; interval: 2000 * Math.max(1, fails); onTriggered: hub.wait() }
    Timer { id: startedT; interval: 2500; onTriggered: hub.started = true }
    Component.onCompleted: { wait(); startedT.start(); }
    // ---- the microphone (Plasma's sound service)
    Loader { id: pm; source: "PaMic.qml"; onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: the microphone isn't there (PaMic.qml)") }
    readonly property var mic: pm.item ? pm.item.mic : null
    readonly property bool micMuted: !!mic && mic.muted === true
    readonly property var micApps: pm.item ? pm.item.apps : []
    readonly property var layout: ph.item ? ph.item.layout : ({ layouts: [], index: 0, canSwitch: false })
    readonly property string layoutShort: { const l = layout.layouts || []; const c = l[layout.index || 0]; return c ? String(c.short || c.name || "").toUpperCase() : ""; }
    readonly property string layoutName: { const l = layout.layouts || []; const c = l[layout.index || 0]; return c ? String(c.name || c.short || "") : ""; }
    property string lastLayout: ""
    onLayoutNameChanged: { if (lastLayout !== "" && layoutName !== lastLayout) layoutSwitched(layoutName, layoutShort); lastLayout = layoutName; }
    property bool sharing: false
    property bool testFullscreen: false
    property bool testSharing: false
    signal jobFinished(var j)
    signal quietEnded(int count, string why)
    signal capsToggled(bool on)
    signal numToggled(bool on)
    signal layoutSwitched(string name, string short)
    readonly property var defaults: ({ autoFullscreen: true, autoSharing: true, schedule: false, scheduleFrom: "22:00", scheduleTo: "07:00", keepHistory: true })
    function pref(k) { return defaults[k]; }
    function noteMissed() { missed++; }
    function badgeFor(id) {
        if (!id) return null;
        const k = String(id).replace(/\.desktop$/, "");
        const b = badges[k] || badges[k.toLowerCase()] || null;
        const j = runningJobs.find(x => x.entry && (x.entry === k || x.entry.replace(/\.desktop$/, "") === k));
        if (!b && !j) return null;
        return { count: b && b.countVisible ? b.count : 0, progress: b && b.progressVisible ? b.progress : j && j.percent >= 0 ? j.percent / 100 : -1, urgent: !!(b && b.urgent) };
    }
    function send(o) { Hearth.call(["python3", Hearth.helper, "notifyd-send", JSON.stringify(o)], (out) => {
        try { const r = JSON.parse(out.trim().split("\n").pop()); if (!r.ok) SettingsNav.toast("That isn't available right now"); } catch (e) {} }); return true; }
    function jobAction(id, what) {
        if (what === "forget") { njobs = njobs.filter(j => j.id !== id); return; }
        if (njobs.some(j => j.id === id)) send({ op: what, id: id }); else if (ph.item) ph.item.jobAction(id, what);
    }
    function disk(what, path) { send({ op: what, path: path }); }
    function setLayout(i) { if (ph.item) ph.item.setLayout(i); }
    function nextLayout() { const n = (layout.layouts || []).length; if (n > 1) setLayout(((layout.index || 0) + 1) % n); }
}
