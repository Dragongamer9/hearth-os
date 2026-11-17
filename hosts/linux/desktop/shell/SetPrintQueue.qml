// Hearth Settings › Printers › Print queue (1.14, KDE's print queue window, in Hearth's look): each printer and what's
// waiting on it, the one printing now with a moving bar, Hold / Go on / Cancel for your own jobs, Cancel all mine, and
// what printed lately. CUPS through hearth-printq; refreshed every few seconds while open. Opens on its own too:
// `hearth-settings print-queue`.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Print queue"
    property var st: null
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["jobs"], r => page.st = r, e => { page.st = { printers: [], jobs: [], done: [] }; page.say(e, true); }); }
    Component.onCompleted: load()
    Timer { interval: 3000; repeat: true; running: page.visible; onTriggered: if (!job.busy) page.load() }
    SetJob { id: job; helper: "hearth-printq" }
    SetJob { id: act; helper: "hearth-printq" }
    function doit(args, ok) { act.run(args, () => { page.say(ok); page.load(); }, e => page.say(e, true)); }
    function kb(n) { return n >= 1048576 ? (n / 1048576).toFixed(1) + " MB" : Math.max(1, Math.round(n / 1024)) + " KB"; }
    readonly property var mine: st ? st.jobs.filter(j => j.mine) : []

    Repeater {
        model: page.st ? page.st.printers : []
        delegate: SetCardGroup {
            id: pg
            required property var modelData
            readonly property var jobs: page.st.jobs.filter(j => j.printer === modelData.name)
            title: modelData.name.replace(/_/g, " ") + " · " + (modelData.state === "printing" ? "printing" : modelData.state === "disabled" ? "paused" : "ready") + (jobs.length ? " · " + jobs.length + " waiting" : "")
            SetRow {
                visible: pg.jobs.length === 0
                interactive: false; chevron: false; icon: "printer"; title: "Nothing waiting"; sub: pg.modelData.reason || ""
            }
            Repeater {
                model: pg.jobs
                delegate: ColumnLayout {
                    id: jr
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 3
                    SetRow {
                        Layout.fillWidth: true
                        interactive: false; chevron: false
                        icon: jr.modelData.state === "printing" ? "printer" : jr.modelData.state === "held" ? "pause" : "doc"
                        title: jr.modelData.title || ("Job " + jr.modelData.num)
                        sub: [jr.modelData.user, page.kb(jr.modelData.size), jr.modelData.when].filter(x => x).join(" · ")
                        chip: jr.modelData.state === "printing" ? "Printing" : jr.modelData.state === "held" ? "Held" : "Waiting"
                        chipKind: jr.modelData.state === "held" ? "warn" : "ok"
                    }
                    Rectangle {
                        visible: jr.modelData.state === "printing"
                        Layout.fillWidth: true; Layout.leftMargin: 4; Layout.rightMargin: 4
                        implicitHeight: 5; radius: 3; color: Theme.panel3; clip: true
                        Rectangle {
                            width: parent.width * 0.3; height: parent.height; radius: 3; color: Theme.accent
                            SequentialAnimation on x { loops: Animation.Infinite; NumberAnimation { from: 0; to: 400; duration: 1200 } NumberAnimation { from: 400; to: 0; duration: 1200 } }
                        }
                    }
                    RowLayout {
                        visible: jr.modelData.mine
                        Layout.leftMargin: 4; spacing: 8
                        SetButton { visible: jr.modelData.state !== "held"; text: "Hold"; icon: "pause"; ghost: true; onActivated: page.doit(["hold", jr.modelData.id], "Held.") }
                        SetButton { visible: jr.modelData.state === "held"; text: "Go on"; icon: "play"; ghost: true; onActivated: page.doit(["resume", jr.modelData.id], "Going on.") }
                        SetButton { text: "Cancel"; icon: "close"; ghost: true; onActivated: page.doit(["cancel", jr.modelData.id], "Cancelled.") }
                    }
                }
            }
        }
    }
    SetRow {
        visible: !!page.st && page.st.printers.length === 0
        interactive: false; icon: "printer"; title: "No printers"; sub: "Add one in Printers"
    }
    SetRow {
        visible: page.mine.length > 1
        icon: "close"; danger: true; chevron: false; title: "Cancel all mine"; sub: page.mine.length + " of your jobs"
        onActivated: page.doit(["cancel-all"], "Cancelled.")
    }
    SetCardGroup {
        title: "Printed lately"
        visible: !!page.st && page.st.done.length > 0
        Repeater {
            model: page.st ? page.st.done.slice(0, 8) : []
            delegate: SetRow {
                required property var modelData
                interactive: false; chevron: false; icon: "check"
                title: modelData.printer.replace(/_/g, " ") + " · job " + modelData.num
                sub: [modelData.user, page.kb(modelData.size), modelData.when].filter(x => x).join(" · ")
            }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
