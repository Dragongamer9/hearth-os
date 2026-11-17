// Hearth Settings › Privacy & security › Recent files (1.14, KDE's Activities › Privacy): whether the files you open are
// remembered (Files' Home page, the launcher's Recommended, apps' jump lists), for how long, forgetting the last hour,
// today or all of them, and folders and apps left out. Your choice is ~/.config/nimbus/recent.json (hearth_settings_io:
// atomic, .bak, merged); the list itself is the shared recently-used.xbel that GTK, KDE and Hearth's apps keep (written
// atomically with a .bak when something is forgotten). Opening this page changes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Recent files"
    property var st: null
    property string msg: ""
    property bool bad: false
    property string confirmAll: ""
    readonly property var rp: st && st.prefs ? st.prefs : ({ remember: true, keepDays: 0, leaveOut: [] })
    readonly property var leftOut: rp.leaveOut || []
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["recent-prefs"], r => page.st = r, e => page.say(e, true)); }
    function set(change, ok) {
        act.run(["recent-prefs-set", JSON.stringify(change)], r => { page.say(ok + (r.forgot ? " · forgot " + r.forgot + (r.forgot === 1 ? " file" : " files") : "")); page.refresh(); }, e => page.say(e, true));
    }
    function forget(what, ok) { act.run(["recent-forget", what], r => { page.say(ok + " (" + (r.forgot || 0) + ")"); page.refresh(); }, e => page.say(e, true)); }
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-files" }
    SetJob { id: act; helper: "hearth-files" }

    Text {
        Layout.fillWidth: true
        text: "Files you open show on Files' Home page, in the launcher's Recommended and in apps' jump lists. Kept on this PC only."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetSwitch {
        icon: "clock"; title: "Remember the files I open"
        sub: page.rp.remember ? "Recent files show in Files, the launcher and jump lists" : "Off: nothing is kept, and what was kept is forgotten"
        checked: page.rp.remember !== false
        onToggled: v => page.set({ remember: v }, v ? "Recent files are remembered" : "Recent files are off")
    }
    SetChoice {
        visible: page.rp.remember !== false
        icon: "calendar"; title: "Keep them for"; sub: "Older ones are forgotten"
        options: [{ value: 0, label: "Always" }, { value: 1, label: "A day" }, { value: 7, label: "A week" }, { value: 30, label: "A month" }, { value: 90, label: "3 months" }, { value: 365, label: "A year" }]
        current: page.rp.keepDays || 0
        onChosen: v => page.set({ keepDays: v }, "Kept " + ({ 0: "always", 1: "for a day", 7: "for a week", 30: "for a month", 90: "for 3 months", 365: "for a year" })[v])
    }
    Text { visible: page.msg !== ""; text: page.msg; color: page.bad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true; Layout.leftMargin: 4 }

    SetHeader { text: "Forget" }
    SetRow { icon: "undo"; chevron: false; title: "Forget the last hour"; onActivated: page.forget("hour", "Forgot the last hour") }
    SetRow { icon: "undo"; chevron: false; title: "Forget today"; sub: "The last 24 hours"; onActivated: page.forget("day", "Forgot the last 24 hours") }
    SetRow {
        icon: "trash"; chevron: false; danger: true
        title: page.confirmAll === "all" ? "Select again to forget every recent file" : "Forget all of them…"
        sub: "Only the list: your files stay where they are"
        onActivated: { if (page.confirmAll === "all") { page.confirmAll = ""; page.forget("all", "Forgot every recent file"); } else page.confirmAll = "all"; }
    }

    SetHeader { text: "Left out" }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Files in these folders, or opened with these apps, are never remembered."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.leftOut
        delegate: SetRow {
            required property string modelData
            icon: modelData.indexOf("/") === 0 ? "folder" : "grid"; title: modelData; sub: "Select to remember it again"; chevron: false
            onActivated: page.set({ leaveOut: page.leftOut.filter(x => x !== modelData) }, "“" + modelData + "” is remembered again")
        }
    }
    Repeater {
        model: page.st && page.st.apps ? page.st.apps.filter(a => page.leftOut.indexOf(a) < 0).slice(0, 12) : []
        delegate: SetSwitch {
            required property string modelData
            icon: "grid"; title: modelData; sub: "Remember what it opens"
            checked: true
            onToggled: v => { if (!v) page.set({ leaveOut: page.leftOut.concat([modelData]) }, "Files opened with " + modelData + " aren't remembered"); }
        }
    }
    SetField {
        id: folderField
        placeholder: "Leave out a folder: type its full path (/home/you/Private)"
        onSubmitted: t => {
            const f = t.trim().replace(/\/+$/, "");
            if (f.indexOf("/") !== 0) return page.say("Type the folder's full path, starting with /", true);
            page.set({ leaveOut: page.leftOut.concat([f]) }, "Files in “" + f + "” aren't remembered");
            folderField.text = "";
        }
    }
}
