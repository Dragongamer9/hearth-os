// Hearth Settings › Privacy & security › Firmware & device security › Update history (1.14, KDE Discover's firmware
// history, in Hearth's look): every firmware update fwupd has put in on this PC, newest first: the part, from which
// version to which, when, and whether it worked (with fwupd's reason when it didn't). Read only (hearth-firmware
// history).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Firmware update history"
    property var st: null
    property string error: ""
    SetJob { id: job; helper: "hearth-firmware" }
    Component.onCompleted: job.run(["history"], r => page.st = r, e => { page.st = { history: [] }; page.error = e; })
    function when(s) { if (!s) return ""; const d = new Date(s * 1000); return d.toLocaleDateString(Qt.locale(), "d MMM yyyy") + " " + d.toLocaleTimeString(Qt.locale(), Settings.clock24 ? "HH:mm" : "h:mm AP"); }

    SetCardGroup {
        title: !page.st ? "Reading…" : page.st.history.length ? page.st.history.length + (page.st.history.length === 1 ? " update" : " updates") : "No firmware updates yet"
        SetRow {
            visible: !!page.st && page.st.history.length === 0
            interactive: false; icon: "info"
            title: page.error ? "Couldn't read the history" : "Nothing has been updated on this PC"
            sub: page.error || "Updates you put in from Firmware & device security show here"
        }
        Repeater {
            model: page.st ? page.st.history : []
            delegate: SetRow {
                required property var modelData
                interactive: false; chevron: false
                icon: modelData.state === "failed" ? "warn" : modelData.state === "done" ? "check" : modelData.state === "restart" ? "restart" : "update"
                title: modelData.name + (modelData.to ? " · " + (modelData.from && modelData.from !== modelData.to ? modelData.from + " → " : "") + modelData.to : "")
                sub: [modelData.vendor, page.when(modelData.when), modelData.state === "failed" && modelData.error ? modelData.error : modelData.summary].filter(x => x).join(" · ")
                chip: modelData.state === "done" ? "Done" : modelData.state === "failed" ? "Didn't work" : modelData.state === "restart" ? "At restart" : modelData.state === "pending" ? "Waiting" : ""
                chipKind: modelData.state === "done" ? "ok" : modelData.state === "failed" ? "bad" : "warn"
            }
        }
    }
}
