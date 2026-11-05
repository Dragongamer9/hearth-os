// Hearth Settings › Home server › Files & links (docs/HOME-SERVER.md): search every share, smart folders (recent
// screenshots, big files, duplicates, not changed in a year) with a clean up that only moves files to their share's
// recycle bin (Put back undoes it), what changed lately, space per share, share links (a file or folder inside a share,
// for a while, with an optional password and most downloads) and a drop-off folder. Links and the drop-off are served
// on Tailscale only, or on your home network if you chose that (Services › Files on the web); never the internet.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Files & links"
    property var fv: null
    property var found: null
    property string openSmart: ""          // the smart folder that's open
    property string linkPath: ""      // a new link's file or folder
    property int linkDays: 7
    property string linkFor: ""
    property bool linkPw: false
    property string msg: ""
    property bool bad: false
    readonly property var scan: fv ? fv.scan : null
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["files"], r => page.fv = r, e => page.fv = { error: e, links: [], shares: [], urls: [], dropoff: { on: false, urls: [] }, recycled: [], web: {}, scan: {} }); }
    function size(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : b >= 1e9 ? (b / 1e9).toFixed(1) + " GB" : b >= 1e6 ? Math.round(b / 1e6) + " MB" : Math.round((b || 0) / 1e3) + " KB"; }
    function show(p) { Quickshell.execDetached(["python3", decodeURIComponent(Qt.resolvedUrl("hearth-portal").toString().replace(/^file:\/\//, "")), "show", p]); }
    function copy(t) { Quickshell.execDetached(["wl-copy", t]); page.say("Copied ✓"); }
    function makeLink(pw) {
        const req = { path: page.linkPath, days: page.linkDays, for: page.linkFor, max: 0 };
        if (pw) req.password = pw;
        act.run(["link-add"], r => { page.linkPath = ""; page.linkPw = false; page.say("Link made ✓ Copy it below"); page.refresh(); }, e => page.say(e, true), JSON.stringify(req));
    }
    readonly property var smart: scan && scan.at ? [
        { id: "screenshots", t: "Recent screenshots", d: scan.screenshots },
        { id: "big", t: "Big files", d: scan.big },
        { id: "duplicates", t: "Duplicates", d: scan.duplicates },
        { id: "old", t: "Not changed in a year", d: scan.old }] : []
    Component.onCompleted: refresh()
    Timer { interval: 5000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Find anything in your shares, free up space, and share files safely: links work on Tailscale or your home network only."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- search
    SetField {
        placeholder: "Search every share, then Enter"
        onSubmitted: t => { if (t.trim()) act.run(["files-search", t.trim()], r => page.found = r, e => page.say(e, true)); }
    }
    Repeater {
        model: page.found ? page.found.results.slice(0, 30) : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.dir ? "folder" : "file"; title: modelData.name
            sub: modelData.share + " · " + modelData.path + (modelData.dir ? "" : " · " + page.size(modelData.size))
            value: page.linkPath === modelData.path ? "Picked" : "Share a link"
            onActivated: page.linkPath = page.linkPath === modelData.path ? "" : modelData.path
        }
    }
    Text {
        visible: !!page.found
        Layout.fillWidth: true
        text: page.found ? (page.found.results.length ? page.found.results.length + " found" + (page.found.partial ? " (stopped looking after a few seconds: type more of the name)" : "") + ". Select one to share a link to it" : "Nothing found") : ""
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- new link
    SetHeader { text: "Share a link to " + page.linkPath.split("/").pop(); visible: page.linkPath !== "" }
    SetChoice {
        visible: page.linkPath !== ""
        title: "Works for"
        options: [{ value: 1, label: "1 day" }, { value: 7, label: "7 days" }, { value: 30, label: "30 days" }, { value: 0, label: "Until I stop it" }]
        current: page.linkDays
        onChosen: v => page.linkDays = v
    }
    SetField {
        visible: page.linkPath !== ""
        placeholder: "Who it's for (only a note for you), then Enter"
        onSubmitted: t => page.linkFor = t
    }
    SetSwitch {
        visible: page.linkPath !== ""
        icon: "lock"; title: "With a password"; sub: "They type it when they open the link"
        checked: page.linkPw
        onToggled: v => page.linkPw = v
    }
    SetField {
        visible: page.linkPath !== "" && page.linkPw
        secret: true
        placeholder: "The link's password (at least 6 characters), then Enter to make the link"
        onSubmitted: t => { if (t.length < 6) { page.say("Use at least 6 characters", true); return; } page.makeLink(t); text = ""; }
    }
    SetRow {
        visible: page.linkPath !== "" && !page.linkPw
        icon: "globe"; title: "Make the link"
        sub: page.fv && page.fv.urls.length ? "Reachable at " + page.fv.urls[0] + " (Tailscale" + (page.fv.web.reach === "home" ? " and your home network" : " only") + ")" : "Turn on Tailscale first (or pick Home network in Services › Files on the web)"
        onActivated: page.makeLink("")
    }

    // ---------------------------------------------------------------- links
    SetHeader { text: "Shared links" }
    Repeater {
        model: page.fv ? page.fv.links : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.expired || modelData.used ? "clock" : "globe"
            title: modelData.label + (modelData.for ? " · for " + modelData.for : "")
            sub: (modelData.expired ? "Expired" : modelData.used ? "Used up" : modelData.expires ? "Until " + new Date(modelData.expires).toLocaleDateString(Qt.locale(), "d MMM") : "Until you stop it")
                 + " · " + (modelData.pw ? "password" : "no password") + " · " + modelData.count + (modelData.max ? " of " + modelData.max : "") + " downloads"
                 + (modelData.urls.length ? " · " + modelData.urls[0] : "")
            value: modelData.urls.length ? "Copy" : ""
            onActivated: if (modelData.urls.length) page.copy(modelData.urls[0])
        }
    }
    Repeater {
        model: page.fv ? page.fv.links : []
        delegate: SetRow {
            required property var modelData
            icon: "close"; title: "Stop " + modelData.label; sub: "The link stops working. The file stays"
            onActivated: act.run(["link-remove", modelData.id], () => { page.say("Stopped"); page.refresh(); }, e => page.say(e, true))
        }
    }
    SetRow {
        visible: !!page.fv && page.fv.links.length === 0
        icon: "globe"; chevron: false; interactive: false; title: "No links yet"
        sub: "Search for a file or folder above and select it to share a link"
    }

    // ---------------------------------------------------------------- drop-off
    SetHeader { text: "Drop-off folder" }
    SetSwitch {
        visible: !!page.fv
        icon: "folder"; title: "Drop-off folder"
        sub: page.fv && page.fv.dropoff.on ? "Anyone with its link can add files to " + page.fv.dropoff.path + ". They can't see or delete anything" : "Friends add files with a link, but can't see or delete anything"
        checked: !!page.fv && page.fv.dropoff.on
        onToggled: v => act.run(["dropoff", v ? "on" : "off"], () => page.refresh(), e => page.say(e, true))
    }
    SetRow {
        visible: !!page.fv && page.fv.dropoff.on && page.fv.dropoff.urls.length > 0
        icon: "globe"; title: "Drop-off link"; sub: page.fv ? page.fv.dropoff.urls[0] || "" : ""
        value: "Copy"
        onActivated: page.copy(page.fv.dropoff.urls[0])
    }

    // ---------------------------------------------------------------- smart folders
    SetHeader { text: "Smart folders" }
    SetRow {
        icon: "search"; title: page.fv && page.fv.scanning ? "Looking through your shares…" : page.scan && page.scan.at ? "Look again" : "Look through your shares"
        sub: (page.scan && page.scan.at ? "Last looked " + new Date(page.scan.at * 1000).toLocaleString(Qt.locale(), "d MMM h:mm AP") + ". " : "")
             + "Finds recent screenshots, big files, duplicates and files not changed in a year, at low priority"
        onActivated: if (!(page.fv && page.fv.scanning)) act.run(["files-scan"], () => page.refresh(), e => page.say(e, true))
    }
    Repeater {
        model: page.smart
        delegate: SetRow {
            required property var modelData
            icon: "folder"; title: modelData.t
            sub: modelData.d.count + (modelData.d.count === 1 ? " file · " : " files · ") + page.size(modelData.d.bytes) + (modelData.id === "duplicates" ? " you could free" : "")
            value: page.openSmart === modelData.id ? "Close" : modelData.d.count ? "Review" : ""
            onActivated: page.openSmart = page.openSmart === modelData.id ? "" : modelData.id
        }
    }
    Repeater {
        model: page.openSmart && page.scan && page.scan[page.openSmart] ? page.scan[page.openSmart].items.slice(0, 40) : []
        delegate: SetRow {
            required property var modelData
            icon: "file"; title: modelData.path.split("/").pop()
            sub: modelData.share + " · " + page.size(modelData.size) + " · " + new Date(modelData.mtime * 1000).toLocaleDateString(Qt.locale(), "d MMM yyyy")
            value: "Show"
            onActivated: page.show(modelData.path)
        }
    }
    SetRow {
        visible: page.openSmart !== "" && !!page.scan && !!page.scan[page.openSmart] && page.scan[page.openSmart].count > 0
        icon: "trash"; title: page.openSmart === "duplicates" ? "Move the extra copies to the recycle bin" : "Move these to the recycle bin"
        sub: "Each goes to its share's recycle bin for 30 days (never deleted now). Put back undoes it"
        onActivated: act.run(["files-recycle"], r => { page.say(r.moved + " moved to the recycle bin. Put back undoes it"); page.openSmart = ""; act.run(["files-scan"], () => page.refresh()); }, e => page.say(e, true),
                             JSON.stringify(page.scan[page.openSmart].items.slice(0, 40).map(i => i.path)))
    }
    SetRow {
        visible: !!page.fv && page.fv.recycled.length > 0
        icon: "restart"; title: "Put back"
        sub: page.fv ? page.fv.recycled.length + " files moved to the recycle bin last time go back where they were" : ""
        onActivated: act.run(["files-putback"], r => { page.say(r.putBack + " put back"); page.refresh(); }, e => page.say(e, true))
    }

    // ---------------------------------------------------------------- recent and space
    SetHeader { text: "Changed this week"; visible: !!page.scan && !!page.scan.recent && page.scan.recent.count > 0 }
    Repeater {
        model: page.scan && page.scan.recent ? page.scan.recent.items.slice(0, 12) : []
        delegate: SetRow {
            required property var modelData
            icon: "file"; title: modelData.path.split("/").pop()
            sub: modelData.share + " · " + new Date(modelData.mtime * 1000).toLocaleString(Qt.locale(), "ddd h:mm AP") + " · " + page.size(modelData.size)
            onActivated: page.show(modelData.path)
        }
    }
    SetHeader { text: "Space per share"; visible: !!page.scan && !!page.scan.sizes }
    Repeater {
        model: page.scan && page.scan.sizes && page.fv ? page.fv.shares : []
        delegate: SetRow {
            required property var modelData
            icon: "folder"; chevron: false; interactive: false; title: modelData.name
            sub: page.size(page.scan.sizes[modelData.id] || 0) + " · " + modelData.path
        }
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
