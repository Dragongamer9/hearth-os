// Hearth Settings › Gaming › Saved games (1.11): every game's saves, found by Ludusavi (hearth-saves), backed up into
// your Saved Games folder and put back on this PC or another. Opening it only reads; Back up and Restore are buttons.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Saved games"
    SetJob { id: job; helper: "hearth-saves" }
    property var st: null
    property string msg: ""
    property bool bad: false
    function refresh() { job.run(["status"], r => page.st = r, e => { page.msg = e; page.bad = true; }, "", "Looking for saves…"); }
    function say(t, b) { msg = t; bad = !!b; }
    function size(b) { return b >= 1073741824 ? (b / 1073741824).toFixed(1) + " GB" : b >= 1048576 ? Math.round(b / 1048576) + " MB" : Math.max(1, Math.round(b / 1024)) + " KB"; }
    Component.onCompleted: refresh()

    SetBanner {
        visible: !!page.st && !page.st.ludusavi
        title: "Ludusavi isn't on this Hearth"; sub: "Saves can't be found here without it."
    }
    SetRow {
        icon: "sync"
        title: job.busy ? (job.what || "Working…") : "Back up all saves"
        sub: page.st ? (page.st.games.filter(g => g.here !== false).length + " games with saves on this PC · into " + page.st.folder + (page.st.last ? " · last " + page.st.last : "")) : "Looking for saves…"
        onActivated: if (!job.busy) job.run(["backup"], r => { page.say("Backed up " + r.games + " games (" + page.size(r.bytes) + ") ✓"); page.refresh(); }, e => page.say(e, true), "", "Backing up…")
    }
    SetRow {
        icon: "restart"
        title: "Restore all saves"
        sub: "Puts every backed-up save back where its game looks for it (after a reinstall, or on another PC). What it replaces is kept by Ludusavi."
        onActivated: if (!job.busy) job.run(["restore"], r => { page.say("Restored " + r.games + " games ✓"); page.refresh(); }, e => page.say(e, true), "", "Restoring…")
    }
    Text {
        Layout.fillWidth: true; visible: page.msg !== ""; text: page.msg; wrapMode: Text.WordWrap
        color: page.bad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    SetHeader { text: "Games" }
    Repeater {
        model: page.st ? page.st.games : []
        delegate: SetRow {
            required property var modelData
            icon: "pad"
            title: modelData.name
            sub: page.size(modelData.bytes) + " · " + modelData.files + " files" + (modelData.here === false ? " · only in the backup" : "")
            chip: modelData.backedUp ? "backed up" : "not backed up"
            chipKind: modelData.backedUp ? "ok" : "warn"
            onActivated: if (!job.busy) job.run(modelData.here === false ? ["restore", modelData.name] : ["backup", modelData.name],
                r => { page.say((modelData.here === false ? "Restored " : "Backed up ") + modelData.name + " ✓"); page.refresh(); }, e => page.say(e, true), "", modelData.name + "…")
        }
    }
}
