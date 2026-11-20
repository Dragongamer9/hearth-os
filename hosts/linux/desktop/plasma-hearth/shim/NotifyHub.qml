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
    property var runningJobs: []
    property int jobsPercent: -1
    property var backgroundApps: []
    property var cameraApps: []
    property var micApps: []
    property bool micMuted: false
    property var mic: null
    property var printJobs: []
    property var disks: []
    property var layout: ({ layouts: [], index: 0, canSwitch: false })
    property string layoutName: ""
    property string layoutShort: ""
    property var badges: ({})
    property bool jobServer: false
    property var jobs: []
    property var keys: []
    property bool sharing: false
    property bool testFullscreen: false
    property bool testSharing: false
    signal jobFinished(var j)
    signal quietEnded(int count, string why)
    signal capsToggled(bool on)
    signal numToggled(bool on)
    signal layoutSwitched(string name)
    readonly property var defaults: ({ autoFullscreen: true, autoSharing: true, schedule: false, scheduleFrom: "22:00", scheduleTo: "07:00", keepHistory: true })
    function pref(k) { return defaults[k]; }
    function noteMissed() { missed++; }
    function badgeFor(id) { return null; }
    function send(o) {}
    function jobAction(id, what) {}
    function disk(what, path) {}
    function nextLayout() {}
    function setLayout(i) {}
}
