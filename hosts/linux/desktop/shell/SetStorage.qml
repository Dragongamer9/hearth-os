// Hearth Settings › Storage: your drives and what fills them, your games by size, cleaning up what's safe to delete
// (shader caches, old Proton prefixes, leftover downloads) and the low space warning. Moved from the archived Game
// Mode's Settings › Games › Storage, with the same calls (hearth-core diskspace.rs through HearthCall; docs/STORAGE.md).
// Moving a game to another drive and adding up every game's size again run for a long time, so they stay in the Hearth
// app's window for now (docs/SETTINGS-MOVE.md, "Interim").
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Storage"
    property var st: null          // diskspace.status: {drives, games, warn: {on, gb}, low, job}
    property string err: ""
    property string msg: ""
    property var clean: null       // diskspace.cleanList's items, once looked for
    property var picked: ({})      // key → true: what's turned on to delete
    property bool finding: false
    property bool cleaning: false
    property bool confirm: false
    readonly property var steps: [5, 10, 15, 20, 30, 50, 100]
    HearthCall { id: core }
    function load() { core.call("diskspace.status", {}, v => { page.st = v; page.err = ""; }, e => page.err = e); }
    Component.onCompleted: load()
    function gb(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : b > 0 && b < 1e9 ? Math.max(1, Math.round(b / 1e6)) + " MB" : Math.round((b || 0) / 1e9) + " GB"; }
    readonly property var bySize: st ? (st.games || []).filter(g => g.size > 0).sort((a, b) => b.size - a.size).slice(0, 12) : []
    readonly property var chosen: clean ? clean.filter(c => page.picked[c.key]) : []
    readonly property real chosenBytes: chosen.reduce((a, c) => a + (c.bytes || 0), 0)
    function find() {
        page.finding = true; page.msg = ""; page.confirm = false;
        core.call("diskspace.cleanList", {}, v => {
            page.finding = false; page.clean = v.items || [];
            const p = {}; for (const c of page.clean) if (c.selectable && c.on) p[c.key] = true; page.picked = p;
        }, e => { page.finding = false; page.msg = e; });
    }
    function toggle(c) {
        if (!c.selectable) { page.msg = c.why || "This one is kept"; return; }
        const p = Object.assign({}, page.picked); if (p[c.key]) delete p[c.key]; else p[c.key] = true; page.picked = p; page.confirm = false;
    }
    function runClean() {
        const keys = page.chosen.map(c => c.key), confirmed = page.chosen.filter(c => c.confirm).map(c => c.key);
        if (!keys.length) { page.msg = "Turn on what to delete first"; return; }
        page.cleaning = true; page.confirm = false;
        core.call("diskspace.clean", { keys: keys, confirmed: confirmed }, r => {
            page.cleaning = false; page.clean = null;
            page.msg = "Freed " + page.gb(r.freed || 0) + ((r.failed || []).length ? " · " + r.failed.length + " kept: " + r.failed[0].why : "");
            page.load();
        }, e => { page.cleaning = false; page.msg = e; });
    }
    function setWarn(on, gbs) { core.call("diskspace.setWarn", { on: on, gb: gbs }, v => { if (page.st) page.st = Object.assign({}, page.st, { warn: v }); }, e => page.msg = e); }

    Text {
        visible: !page.st && page.err === ""
        text: "Looking at your drives…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Text {
        visible: page.err !== ""
        Layout.fillWidth: true
        text: page.err; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.st ? (page.st.low || []) : []
        delegate: Text {
            required property var modelData
            Layout.fillWidth: true
            text: "Low on space: " + (modelData.name || modelData.drive || "a drive") + (modelData.free !== undefined ? " has " + page.gb(modelData.free) + " left" : "")
            color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }

    SetHeader { text: "Drives"; visible: !!page.st }
    Repeater {
        model: page.st ? (page.st.drives || []) : []
        delegate: SetRow {
            id: drow
            required property var modelData
            interactive: false; chevron: false
            icon: "folder"
            title: modelData.name || modelData.mount
            sub: page.gb(modelData.free) + " free of " + page.gb(modelData.size) + (modelData.games ? " · games go here" : "") + (modelData.readOnly ? " · read only" : "")
            value: modelData.size ? Math.round(100 * modelData.used / modelData.size) + "% used" : ""
            Rectangle {
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 68; rightMargin: 16; bottomMargin: 8 }
                height: 4; radius: 2; color: Theme.panel3
                Rectangle { height: parent.height; radius: 2; color: Theme.accent; width: drow.modelData.size ? parent.width * Math.min(1, drow.modelData.used / drow.modelData.size) : 0 }
            }
        }
    }

    SetHeader { text: "Games by size"; visible: !!page.st }
    Repeater {
        model: page.bySize
        delegate: SetRow {
            required property var modelData
            interactive: false; chevron: false
            icon: "pad"; title: modelData.title; sub: [modelData.store, modelData.drive].filter(Boolean).join(" · ")
            value: page.gb(modelData.size)
        }
    }
    Text {
        visible: !!page.st && page.bySize.length === 0
        Layout.fillWidth: true
        text: "Game sizes haven't been added up yet."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetRow {
        visible: !!page.st
        icon: "open"; title: "Move games and add up sizes"
        sub: "Moves a game to another drive, or adds up every game's size again. Opens in the Hearth app's window for now."
        onActivated: core.openPage("diskspace", "Opening Storage in the Hearth app…")
    }

    SetHeader { text: "Clean up"; visible: !!page.st }
    SetRow {
        visible: !!page.st
        icon: "search"; title: page.finding ? "Looking…" : page.clean ? "Look again" : "Find things to clean up"
        sub: "Shader caches, Proton prefixes of games that are gone, and leftover downloads. Nothing is deleted until you say so."
        onActivated: if (!page.finding && !page.cleaning) page.find()
    }
    Repeater {
        model: page.clean || []
        delegate: SetSwitch {
            required property var modelData
            title: modelData.title
            sub: [modelData.where, page.gb(modelData.bytes), (modelData.saves || []).length ? "has saves: " + modelData.saves.join(", ") + (modelData.cloud ? " (Steam Cloud has a copy)" : "") : "", modelData.selectable ? "" : (modelData.why || "kept")].filter(Boolean).join(" · ")
            checked: !!page.picked[modelData.key]
            opacity: modelData.selectable ? 1 : 0.55
            onToggled: page.toggle(modelData)
        }
    }
    Text {
        visible: !!page.clean && page.clean.length === 0
        Layout.fillWidth: true
        text: "Nothing to clean up."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
    }
    SetRow {
        visible: !!page.clean && page.clean.length > 0
        icon: "trash"; danger: true
        title: page.cleaning ? "Deleting…" : "Delete " + page.chosen.length + " item" + (page.chosen.length === 1 ? "" : "s")
        sub: "Frees about " + page.gb(page.chosenBytes) + ". This can't be undone."
        onActivated: if (!page.cleaning) page.confirm = !page.confirm
    }
    SetRow {
        visible: page.confirm
        icon: "check"; danger: true; title: "Yes, delete them"
        sub: page.chosen.some(c => c.confirm) ? "Some of these have saves on this device: they go too." : "Select to delete."
        onActivated: page.runClean()
    }

    SetHeader { text: "Low space warning"; visible: !!page.st }
    SetSwitch {
        visible: !!page.st
        title: "Warn when a games drive is low on space"
        sub: "Before an install and in notifications"
        checked: !!page.st && !!page.st.warn && page.st.warn.on
        onToggled: on => page.setWarn(on, page.st && page.st.warn ? page.st.warn.gb : 15)
    }
    SetChoice {
        visible: !!page.st && !!page.st.warn && page.st.warn.on
        title: "Low means less than"
        options: page.steps.map(g => ({ value: g, label: g + " GB" }))
        current: page.st && page.st.warn ? page.st.warn.gb : 15
        onChosen: v => page.setWarn(true, v)
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
