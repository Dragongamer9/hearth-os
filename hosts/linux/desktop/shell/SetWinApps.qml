// Hearth Settings › Apps › Windows apps (docs/WINDOWS-APPS.md): the Windows programs you opened with Hearth, each in its
// own space (~/Games/Windows apps/<name>), started with the GE-Proton built into Hearth OS unless the Proton detector
// or you pick another. Open, pick a Proton, DXVK / VKD3D, the Windows version, add to Steam, remove. The work is in
// hearth-winapps; each app's settings are its hearth-app.json, written the safe way (hearth_settings_io).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Windows apps"
    property var st: null
    property string message: ""
    property bool messageBad: false
    property string confirmRemove: ""
    property var pendingDownload: null      // {id, name}: a Proton to download, waiting for your yes
    readonly property var appList: page.st && page.st.apps ? page.st.apps : []
    function say(t, bad) { message = t; messageBad = !!bad; }
    function reload() { job.run(["list"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: reload()
    SetJob { id: job; helper: "hearth-winapps" }
    Timer { id: later; interval: 20000; onTriggered: page.reload() }

    function launch(a, proton) {
        Quickshell.execDetached(["python3", job.path, "run", a.id].concat(proton ? ["--proton", proton] : []));
        say("Starting " + a.name + (proton ? " with " + proton : "") + "…");
        later.restart();
    }
    function change(a, key, value, done) {
        job.run(["set", a.id, key, value], () => { page.reload(); if (done) done(); }, e => page.say(e, true));
    }
    function protonOptions(a) {
        const out = [{ value: "", label: "Automatic" }];
        const c = a.candidates || [];
        for (let i = 0; i < c.length && out.length < 5; i++)
            out.push({ value: c[i].name, label: c[i].name + (c[i].download ? " ↓" : "") });
        if (a.proton && !out.some(o => o.value === a.proton)) out.push({ value: a.proton, label: a.proton });
        return out;
    }
    function needsDownload(a, name) { return (a.candidates || []).some(c => c.name === name && c.download); }
    function pickProton(a, name) {
        if (name && needsDownload(a, name)) { page.pendingDownload = { id: a.id, name: name, app: a.name, then: "set" }; return; }
        change(a, "proton", name, () => page.say(name ? a.name + " starts with " + name + " now." : "Hearth picks the Proton for " + a.name + " again."));
    }
    function tryNext(a, next) {
        if (next.download) { page.pendingDownload = { id: a.id, name: next.name, app: a.name, then: "run" }; return; }
        launch(a, next.name);
    }
    function download() {
        const d = page.pendingDownload;
        page.pendingDownload = null;
        job.run(["fetch", d.name], () => {
            page.say(d.name + " is downloaded.");
            const a = page.appList.find(x => x.id === d.id);
            if (!a) return page.reload();
            if (d.then === "run") page.launch(a, d.name);
            else page.change(a, "proton", d.name);
        }, e => page.say(e, true), "", "Downloading " + d.name);
    }
    function summary(a) {
        const l = a.last || {};
        if (l.crashed) return "Closed right away with " + l.proton + (l.next ? ". Try " + l.next.name + " below." : ".");
        if (l.proton) return "Last started with " + l.proton + (a.proton ? " (your pick)" : " (picked by Hearth)");
        return a.proton ? "Starts with " + a.proton + " (your pick)" : "Hearth picks the Proton when it starts";
    }
    function took(sec) { return sec < 120 ? Number(sec || 0).toFixed(1) + " s" : Math.round(sec / 60) + " min"; }
    // The last start's result, from its log (hearth-winapps keeps the last 5 starts in the app's logs folder).
    function lastResult(a) {
        const l = a.last || {};
        if (!l.at) return "Not started yet";
        const when = new Date(l.at * 1000).toLocaleString(Qt.locale(), Locale.ShortFormat);
        return (l.what && l.what !== "started" ? l.what.charAt(0).toUpperCase() + l.what.slice(1) : "Started") + " " + when + " with " + l.proton
            + ", closed after " + took(l.seconds) + (l.code === 0 ? " (no error)" : " with code " + l.code);
    }
    function showLog(path) {
        if (!path) return say("There's no log yet: it's written the first time the app starts.");
        Quickshell.execDetached(["xdg-open", path]);
    }

    SetRow {
        interactive: false; icon: "window"
        title: "Proton built into Hearth OS"
        sub: !page.st ? "Checking…" : page.st.builtin
             ? page.st.builtin + ": Windows programs start without the internet." + (page.st.valve ? " Valve's " + page.st.valve + " from Steam is used when it works better." : "")
             : "This Hearth OS has none built in. Older GE-Protons can be downloaded when an app needs one."
        chip: !page.st ? "" : page.st.builtin ? "Built in" : "Missing"
        chipKind: page.st && page.st.builtin ? "ok" : "warn"
    }
    SetRow {
        icon: "folder"; chevron: true
        title: "Open a Windows program"
        sub: "Open a .exe or .msi in Files (or right-click it › Open with Hearth). An installer gets its own space and its app goes in your app menu."
        onActivated: Quickshell.execDetached(["hearth-filemanager", (Quickshell.env("HOME") || "") + "/Downloads"])
    }

    SetHeader { text: "Your Windows apps"; visible: page.appList.length > 0 }
    Text {
        visible: !!page.st && page.appList.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 8
        text: "No Windows apps yet. They're kept in “Windows apps” in your Games folder."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.appList
        delegate: SetExpander {
            id: xp
            required property var modelData
            readonly property var a: modelData
            readonly property var last: a.last || {}
            icon: "window"
            title: a.name
            sub: page.summary(a)
            chip: xp.last.crashed ? "Closed right away" : a.steam ? "In Steam" : ""
            chipKind: xp.last.crashed ? "warn" : "ok"

            SetRow { icon: "play"; title: "Open"; sub: a.exe + (a.args && a.args.length ? " " + a.args.join(" ") : ""); onActivated: page.launch(xp.a, "") }
            SetRow {
                icon: "info"; title: "Show log"; chevron: true
                sub: page.lastResult(xp.a) + ". " + (xp.a.log ? "Everything Proton and the program said: " + xp.a.log : "The log is written at its first start.")
                onActivated: page.showLog(xp.a.log)
            }
            SetSwitch {
                title: "Detailed Proton log for the next start"
                sub: xp.a.detailedNext ? "On for the next start only (PROTON_LOG=1), then off again. It's written to " + xp.a.dir + "/logs/steam-hearth-winapp-" + xp.a.id + ".log and Send logs includes it."
                     : xp.a.detailedLog ? "Off. The last detailed log: " + xp.a.detailedLog
                     : "For one start, Proton writes everything it does (big, slower) to the app's logs folder, and Send logs includes it."
                checked: xp.a.detailedNext
                onToggled: v => job.run(["detailed", xp.a.id, v ? "on" : "off"], () => page.reload(), e => page.say(e, true))
            }
            SetRow {
                visible: !!xp.a.detailedLog
                icon: "info"; title: "Show the detailed Proton log"; chevron: true; sub: xp.a.detailedLog
                onActivated: page.showLog(xp.a.detailedLog)
            }
            SetRow {
                visible: !!xp.last.crashed && !!xp.last.next
                icon: "restart"; title: "Try with " + (xp.last.next ? xp.last.next.name : "")
                sub: xp.last.next && xp.last.next.download ? "Downloads it first (about 500 MB), after you say yes" : "Hearth remembers it if it works"
                onActivated: page.tryNext(xp.a, xp.last.next)
            }
            SetChoice {
                title: "Proton"
                options: page.protonOptions(xp.a)
                current: xp.a.proton
                onChosen: v => page.pickProton(xp.a, v)
            }
            SetSwitch {
                title: "DXVK"; sub: "DirectX 9, 10 and 11 through Vulkan. Off: Wine's own, slower, for programs that draw wrongly."
                checked: xp.a.dxvk
                onToggled: v => page.change(xp.a, "dxvk", v ? "on" : "off")
            }
            SetSwitch {
                title: "VKD3D"; sub: "DirectX 12 through Vulkan. Off: the program falls back to DirectX 11 if it can."
                checked: xp.a.vkd3d
                onToggled: v => page.change(xp.a, "vkd3d", v ? "on" : "off")
            }
            SetChoice {
                title: "Windows version"
                options: [{ value: "", label: "Automatic" }, { value: "win11", label: "11" }, { value: "win10", label: "10" }, { value: "win7", label: "7" }, { value: "winxp", label: "XP" }]
                current: xp.a.winver
                onChosen: v => page.change(xp.a, "winver", v)
            }
            SetSwitch {
                title: "In Steam"; sub: "Hearth Sync adds it to Steam's library (Game Mode too) the next time it syncs"
                checked: xp.a.steam
                onToggled: v => page.change(xp.a, "steam", v ? "on" : "off", () => page.say(v ? xp.a.name + " goes into Steam at Hearth Sync's next sync." : xp.a.name + " comes out of Steam at Hearth Sync's next sync."))
            }
            SetRow {
                visible: page.confirmRemove !== xp.a.id
                icon: "trash"; danger: true; title: "Remove…"; sub: "Deletes the app, its menu entry and everything installed in its space"
                onActivated: page.confirmRemove = xp.a.id
            }
            SetRow {
                visible: page.confirmRemove === xp.a.id
                icon: "trash"; danger: true; title: "Yes, remove " + xp.a.name; sub: xp.a.dir
                onActivated: job.run(["remove", xp.a.id], () => { page.confirmRemove = ""; page.say(xp.a.name + " is removed."); page.reload(); }, e => page.say(e, true))
            }
            SetRow {
                visible: page.confirmRemove === xp.a.id
                title: "Cancel"; onActivated: page.confirmRemove = ""
            }
        }
    }

    SetRow {
        visible: !!page.pendingDownload
        icon: "update"
        title: page.pendingDownload ? "Download " + page.pendingDownload.name + "?" : ""
        sub: "About 500 MB from GE-Proton's releases on GitHub, checked against Hearth's checksum, for " + (page.pendingDownload ? page.pendingDownload.app : "") + ". It's kept for other apps too."
        onActivated: page.download()
    }
    SetRow {
        visible: !!page.pendingDownload
        title: "Not now"; onActivated: page.pendingDownload = null
    }
    ColumnLayout {
        visible: job.busy && job.progress >= 0
        Layout.fillWidth: true
        spacing: 4
        Text { text: job.line; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 6; radius: 3; color: SetStyle.card2
            Rectangle { width: parent.width * Math.max(0, Math.min(1, job.progress)); height: parent.height; radius: 3; color: Theme.accent }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
