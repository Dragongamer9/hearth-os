pragma Singleton

// The notification-side state Mantel's bar shows (plasma-hearth). Quiet = Do not disturb (desk.json, as Mantel).
// Copies and downloads, background apps, camera / microphone in use, printing, disks and the keyboard layout come
// from Mantel's own services there; on Plasma they're empty for now, so those buttons stay hidden, as on a PC
// with nothing of the kind going on.
import QtQuick

Item {
    readonly property bool quiet: DeskState.dnd
    property string autoReason: ""
    property string autoLabel: ""
    property int missed: 0
    Loader { id: ph; source: "PlasmaHub.qml"; onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: keyboard layout and job progress aren't there (PlasmaHub.qml)") }
    readonly property var jobs: ph.item ? ph.item.jobs : []
    readonly property var runningJobs: jobs.filter(j => j.state === "running")
    readonly property int jobsPercent: { const r = runningJobs.filter(j => j.percent >= 0); return r.length ? Math.round(r.reduce((a, j) => a + j.percent, 0) / r.length) : -1; }
    property var backgroundApps: []
    property var cameraApps: []
    property var micApps: []
    property bool micMuted: false
    property var mic: null
    property var printJobs: []
    property var disks: []
    readonly property var layout: ph.item ? ph.item.layout : ({ layouts: [], index: 0, canSwitch: false })
    readonly property string layoutShort: { const l = layout.layouts || []; const c = l[layout.index || 0]; return c ? String(c.short || c.name || "").toUpperCase() : ""; }
    readonly property string layoutName: { const l = layout.layouts || []; const c = l[layout.index || 0]; return c ? String(c.name || c.short || "") : ""; }
    property string lastLayout: ""
    onLayoutNameChanged: { if (lastLayout !== "" && layoutName !== lastLayout) layoutSwitched(layoutName, layoutShort); lastLayout = layoutName; }
    property var badges: ({})
    property bool jobServer: false
    property var keys: []
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
    function badgeFor(id) { return null; }
    function send(o) {}
    function jobAction(id, what) { if (ph.item) ph.item.jobAction(id, what); }
    function disk(what, path) {}
    function setLayout(i) { if (ph.item) ph.item.setLayout(i); }
    function nextLayout() { const n = (layout.layouts || []).length; if (n > 1) setLayout(((layout.index || 0) + 1) % n); }
}
