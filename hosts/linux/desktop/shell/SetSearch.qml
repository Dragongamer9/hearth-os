// Hearth Settings › Search (like KDE's File Search and KRunner settings): the file index (hearth-search, like Baloo)
// on or off, which folders it covers and leaves out, whether it reads the text inside documents, on battery or not,
// rebuild; and which of the search plugins the app menus use (calculator, units, currency, web shortcuts, commands,
// emoji, recent documents, open windows), with currency rates online or not. Saved through hearth-search prefs-set
// (hearth_settings_io: only what you change, merge-on-write, .bak); nothing is written when the page opens.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Search"
    property var st: null
    property string message: ""
    property bool messageBad: false
    property string adding: ""            // "folders" | "exclude" while typing a folder
    readonly property var pr: st ? st.prefs : null
    readonly property var pluginNames: ({
        calc: ["Calculator", "Sums as you type: 2+2, =sqrt(2), 15% of 80", "plus"],
        units: ["Unit conversion", "10 km to mi, 72 f in c, 500 MB to GB", "sync"],
        currency: ["Currency conversion", "20 usd to eur (today's rates when online, the built-in ones offline)", "sync"],
        web: ["Web shortcuts", "gg:words (Google), wp: (Wikipedia), yt: (YouTube), ddg:, gh:, fh: (Flathub), a web address", "globe"],
        commands: ["Commands", "Type > and a command to run it, in the terminal or on its own", "kbd"],
        emoji: ["Emoji", "heart, thumbs up, or :fire for emoji only; Enter copies it", "sparkle"],
        recent: ["Recent documents", "Files you opened lately", "clock"],
        windows: ["Open windows", "Switch to a window by its title", "window"]
    })

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    function set(changes, okText) {
        job.run(["prefs-set"], r => { page.load(); if (okText) page.say(okText); }, e => page.say(e, true), JSON.stringify(changes), "Saving…");
    }
    function startService() { Quickshell.execDetached(["systemctl", "--user", "start", "--no-block", "hearth-search.service"]); }
    function when(ms) {
        if (!ms) return "never";
        const m = Math.round((Date.now() - ms) / 60000);
        return m < 1 ? "just now" : m < 60 ? m + " min ago" : m < 1440 ? Math.round(m / 60) + " h ago" : Math.round(m / 1440) + " days ago";
    }
    function size(b) { return b > 1e9 ? (b / 1e9).toFixed(1) + " GB" : b > 1e6 ? Math.round(b / 1e6) + " MB" : Math.round((b || 0) / 1e3) + " KB"; }
    function stateLine() {
        if (!st) return "Checking…";
        if (!st.fts5) return "This system's SQLite has no full-text search, so files are found by looking through folders instead";
        if (st.paused === "off") return "Off: the menus and Files look through your folders when you search (slower)";
        if (st.running && !st.complete) return "Making the index… " + (st.files || 0) + " found so far";
        if (st.paused === "battery") return "Paused while on battery · " + st.files + " files and folders indexed";
        return st.files + " files and folders · " + size(st.bytes) + " · last looked " + when(st.lastPass);
    }
    Component.onCompleted: load()
    Timer { interval: 4000; repeat: true; running: page.visible && !!page.st && page.st.running; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-search" }

    SetCardGroup {
        title: "File index"
        SetSwitch {
            icon: "search"; title: "Index my files"
            sub: page.stateLine()
            checked: !!page.pr && page.pr.indexing
            onToggled: v => { page.set({ indexing: v }, v ? "The index is on. It's made in the background at the lowest priority." : "The index is off. Searching still works, more slowly.");
                              if (v) page.startService(); }
        }
        SetSwitch {
            visible: !!page.pr && page.pr.indexing
            icon: "doc"; title: "Search inside documents"
            sub: "Text files, OpenDocument, Word, Excel, PowerPoint" + (page.st && page.st.pdf ? " and PDFs" : "") + ": what's in them, not only their names. Kept on this device only."
            checked: !!page.pr && page.pr.contents
            onToggled: v => { page.set({ contents: v }, v ? "Documents' text is added on the next pass." : "Documents' text is taken out of the index on the next pass."); page.startService(); }
        }
        SetSwitch {
            visible: !!page.pr && page.pr.indexing
            icon: "batt"; title: "Keep indexing on battery"
            sub: "Off: the index waits until you plug in"
            checked: !!page.pr && page.pr.onBattery
            onToggled: v => page.set({ onBattery: v })
        }
        SetRow {
            visible: !!page.pr && page.pr.indexing
            icon: "restart"; chevron: false; title: "Rebuild the index"
            sub: "Start again from nothing (it's made again in the background)"
            onActivated: job.run(["rebuild"], () => { page.say("Rebuilding the index…"); page.load(); }, e => page.say(e, true))
        }
    }

    SetCardGroup {
        title: "Folders it covers"
        visible: !!page.pr && page.pr.indexing
        Repeater {
            model: page.pr ? page.pr.folders : []
            delegate: SetRow {
                required property var modelData
                required property int index
                icon: "folder"; title: modelData === "~" ? "Home" : modelData; chevron: false
                value: page.pr.folders.length > 1 ? "Remove" : ""
                onActivated: if (page.pr.folders.length > 1) page.set({ folders: page.pr.folders.filter((x, i) => i !== index) })
            }
        }
        SetRow { visible: page.adding !== "folders"; icon: "plus"; title: "Add a folder…"; chevron: false; onActivated: page.adding = "folders" }
        SetField {
            visible: page.adding === "folders"
            Layout.fillWidth: true
            placeholder: "A folder, like ~/Projects or /run/media/you/Data (Enter adds it)"
            onSubmitted: t => { if (t.trim()) page.set({ folders: page.pr.folders.concat([t.trim()]) }, "Added. It's indexed on the next pass."); page.adding = ""; text = ""; page.startService(); }
        }
    }

    SetCardGroup {
        title: "Left out"
        visible: !!page.pr && page.pr.indexing
        SetRow {
            interactive: false; icon: "shield"; chevron: false; title: "Always left out"
            sub: "Hidden folders (your settings and keyrings), caches, game libraries and Windows prefixes, build folders"
        }
        Repeater {
            model: page.pr ? page.pr.exclude : []
            delegate: SetRow {
                required property var modelData
                required property int index
                icon: "eye"; title: modelData; value: "Remove"; chevron: false
                onActivated: page.set({ exclude: page.pr.exclude.filter((x, i) => i !== index) })
            }
        }
        SetRow { visible: page.adding !== "exclude"; icon: "plus"; title: "Leave out a folder…"; chevron: false; onActivated: page.adding = "exclude" }
        SetField {
            visible: page.adding === "exclude"
            Layout.fillWidth: true
            placeholder: "A folder to leave out, like ~/Private (Enter)"
            onSubmitted: t => { if (t.trim()) page.set({ exclude: page.pr.exclude.concat([t.trim()]) }, "Left out. What was indexed from it goes on the next pass."); page.adding = ""; text = ""; page.startService(); }
        }
    }

    SetCardGroup {
        title: "In the app menus' search"
        Repeater {
            model: page.st ? page.st.plugins : []
            delegate: SetSwitch {
                required property var modelData
                icon: (page.pluginNames[modelData] || ["", "", "search"])[2]
                title: (page.pluginNames[modelData] || [modelData])[0]
                sub: (page.pluginNames[modelData] || ["", ""])[1]
                checked: !!page.pr && !(page.pr.pluginsOff || {})[modelData]
                onToggled: v => { const o = Object.assign({}, page.pr.pluginsOff || {}); if (v) delete o[modelData]; else o[modelData] = true; page.set({ pluginsOff: o }); }
            }
        }
        SetSwitch {
            icon: "cloud"; title: "Today's currency rates"
            sub: "Once a day, from the European Central Bank, when you're online. Off: the built-in rates only (the answer says how old they are)."
            checked: !!page.pr && page.pr.currencyOnline
            onToggled: v => page.set({ currencyOnline: v })
        }
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
