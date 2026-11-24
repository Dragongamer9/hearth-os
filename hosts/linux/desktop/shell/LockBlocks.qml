// The lock screen made of blocks (docs/mockups/checklists/lock-screen-blocks.md): the same screen format as Build
// anything (grid, background, look, blocks), drawn under Hearth's own password block. Lock.qml loads this file inside
// its locked surface; if it can't be loaded, or the layout can't be read, `ready` stays false and the plain lock screen
// shows (never a lock that can't be unlocked, never an unlocked screen). This file only DRAWS: it has no keyboard focus,
// nothing in it opens an app, a file or a setting, and unlocking is only Lock.qml's (PAM). The one thing to press is
// the media block's previous / pause / next (Privacy › Media controls), which only steers what's playing.
// Which layout: the editor's Try it (once, $XDG_RUNTIME_DIR/hearth-lock-try.json), else ~/.local/share/nimbus/
// lock-screen.json (on, match: the ready-made one matching your desktop, or your own screen), else, with no file, the
// one matching a ready-made whole-screen desktop (screen.json); Hearth OS Mantel with no file keeps the plain lock
// screen. Read only: nothing here writes a setting. hearth-lockscreen made the layouts safe when they were kept.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
import Quickshell.Services.Mpris

Item {
    id: lb
    // the editor's draft (drawn at the editor's size, nothing read from files)
    property var preview: null
    property bool editing: false
    property var lockedAt: new Date()
    readonly property string shellDir: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")).replace(/\/$/, "")
    readonly property string helper: shellDir + "/hearth-lockscreen"
    readonly property string widgets: shellDir + "/hearth-widgets"

    // ---- which layout
    function readJson(v) { try { const t = v.text(); return t && t.trim() ? JSON.parse(t) : null; } catch (e) { return null; } }
    property var tried: null
    property var mine: null
    property var desk: null
    property var readyLay: null
    FileView { id: tryFile; path: lb.preview ? "" : (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-lock-try.json"; blockLoading: true; onLoaded: lb.tried = lb.readJson(tryFile) }
    FileView { id: lockFile; path: lb.preview ? "" : Quickshell.env("HOME") + "/.local/share/nimbus/lock-screen.json"; blockLoading: true; onLoaded: lb.mine = lb.readJson(lockFile) }
    FileView { id: deskFile; path: lb.preview ? "" : Quickshell.env("HOME") + "/.local/share/nimbus/screen.json"; blockLoading: true; onLoaded: lb.desk = lb.readJson(deskFile) }
    readonly property var readyIds: ["hearth", "console", "iconrow", "cockpit", "cabin", "notebook", "retro"]
    readonly property string readyId: {
        if (preview || tried) return "";
        if (mine && mine.on === true && mine.match === true) return desk && desk.on === true && readyIds.indexOf(desk.from) >= 0 ? desk.from : "hearth";
        if (!mine && desk && desk.on === true) return readyIds.indexOf(desk.from) >= 0 ? desk.from : "hearth";
        return "";
    }
    FileView { id: readyFile; path: lb.readyId ? lb.shellDir + "/lock-screens/" + lb.readyId + ".json" : ""; blockLoading: true; onLoaded: lb.readyLay = lb.readJson(readyFile) }
    readonly property var lay: {
        const l = preview ? preview : tried ? tried : readyId ? readyLay : (mine && mine.on === true ? mine.screen : null);
        return l && typeof l === "object" && Array.isArray(l.blocks) ? l : null;
    }
    readonly property bool ready: !!lay
    // The Try it layout is used once: gone when this lock screen goes.
    Component.onDestruction: if (tried) Quickshell.execDetached(["rm", "-f", tryFile.path])

    // ---- the grid (as Screen.qml)
    readonly property int cols: Math.max(4, Math.min(48, (lay && lay.grid && lay.grid.cols) || 24))
    readonly property int rows: Math.max(4, Math.min(48, (lay && lay.grid && lay.grid.rows) || 14))
    readonly property real margin: Math.round(Math.min(width, height) * 0.035)
    readonly property real cellW: (width - margin * 2) / cols
    readonly property real cellH: (height - margin * 2) / rows
    readonly property var look: (lay && lay.look) || ({})
    readonly property var privacy: (lay && lay.privacy) || ({ notifications: "names", media: true })
    readonly property color ink: look.text || "#f6ece4"
    readonly property color accent: look.accent || "#ff7a2f"
    function fontFor(n) {
        switch (n || look.font || "") {
        case "Noto Serif": return "Noto Serif";
        case "Noto Sans Mono": case "Pixel": return "Noto Sans Mono";
        case "Inter Display": return "Inter Display";
        case "Inter": return "Inter";
        default: return Theme.font;
        }
    }
    // The password block: always there, always on top (a missing one goes bottom right).
    readonly property var pwBlock: {
        const b = lay ? (lay.blocks || []).find(x => x && x.block === "password") : null;
        const w = Math.min(cols, Math.max(5, (b && b.w) || 7)), h = Math.min(rows, Math.max(4, (b && b.h) || 5));
        return { x: Math.max(0, Math.min(cols - w, b ? b.x || 0 : cols - w)), y: Math.max(0, Math.min(rows - h, b ? b.y || 0 : rows - h - 1)), w: w, h: h,
                 shows: b && b.shows ? b.shows : "", style: (b && b.style) || {} };
    }
    readonly property rect pwRect: Qt.rect(margin + pwBlock.x * cellW, margin + pwBlock.y * cellH, pwBlock.w * cellW, pwBlock.h * cellH)
    readonly property bool pwRetro: pwBlock.shows === "window"
    readonly property var names: ({ clock: "Clock", date: "Date", weather: "Weather", media: "Media", notifications: "Notifications", downloads: "Downloads",
                                    battery: "Battery", calendar: "Calendar", picture: "Picture", text: "Words", friends: "Friends online", icons: "Desktop icons", bar: "the locked bar" })
    readonly property string line: {
        if (!lay || lay.line === false) return "";
        const seen = [];
        for (const b of lay.blocks || []) { const n = String(b.block).startsWith("own:") ? "your AI's own" : names[b.block]; if (n && seen.indexOf(n) < 0) seen.push(n); }
        const list = seen.length > 1 ? seen.slice(0, -1).join(", ") + " and " + seen[seen.length - 1] : seen.join("");
        return "Lock screen made of blocks, like your desktop" + (list ? ": these are " + list : "") + ". The password block is always there and always on top.";
    }

    // ---- the background
    readonly property var bg: (lay && lay.background) || ({})
    Rectangle {
        anchors.fill: parent
        visible: lb.ready && lb.bg.kind !== "none" && lb.bg.kind !== "wallpaper"
        gradient: Gradient {
            GradientStop { position: 0; color: lb.bg.from || "#3a1a0e" }
            GradientStop { position: 1; color: lb.bg.kind === "colour" ? (lb.bg.from || "#120c0a") : (lb.bg.to || lb.bg.from || "#120c0a") }
        }
        Rectangle {
            visible: lb.bg.kind === "gradient"
            x: parent.width * 0.35; y: -parent.height * 0.25; width: parent.width * 0.6; height: width; radius: width / 2
            gradient: Gradient { GradientStop { position: 0; color: Qt.rgba(1, 0.45, 0.15, 0.16) } GradientStop { position: 1; color: "transparent" } }
        }
        Repeater {
            model: lb.bg.kind === "paper" ? Math.ceil(lb.height / 48) : 0
            delegate: Rectangle { required property int index; x: 0; y: 120 + index * 48; width: lb.width; height: 1; color: Qt.rgba(0.35, 0.45, 0.7, 0.18) }
        }
        Image {
            anchors.fill: parent
            visible: lb.bg.kind === "picture" && !!lb.bg.value
            source: lb.bg.kind === "picture" && lb.bg.value ? "file://" + String(lb.bg.value).replace(/^~/, Quickshell.env("HOME")) : ""
            fillMode: Image.PreserveAspectCrop; asynchronous: true
        }
    }

    // ---- the blocks (all but the password block, which Lock.qml draws on top)
    Repeater {
        model: lb.ready ? (lb.lay.blocks || []).filter(b => b && b.block !== "password") : []
        delegate: Item {
            id: cell
            required property var modelData
            readonly property var b: modelData
            // a bar across the whole top row reaches the screen's edges (the 1995 look's "Hearth is locked")
            readonly property bool edge: b.block === "bar" && (b.x || 0) === 0 && (b.y || 0) === 0 && (b.w || 0) >= lb.cols
            x: edge ? 0 : lb.margin + Math.max(0, b.x || 0) * lb.cellW
            y: edge ? 0 : lb.margin + Math.max(0, b.y || 0) * lb.cellH
            width: edge ? lb.width : Math.max(1, b.w || 1) * lb.cellW
            height: edge ? lb.margin + lb.cellH * 0.7 : Math.max(1, b.h || 1) * lb.cellH
            Loader {
                anchors.fill: parent
                // a little room between blocks
                anchors.margins: cell.edge ? 0 : Math.round(Math.min(lb.cellW, lb.cellH) * 0.09)
                property var blk: cell.b
                property var st: cell.b.style || ({})
                sourceComponent: String(cell.b.block).startsWith("own:") ? ownC
                    : ({ clock: clockC, date: clockC, weather: weatherC, media: mediaC, notifications: notesC, downloads: downloadsC,
                         battery: batteryC, calendar: calendarC, picture: pictureC, text: wordsC, friends: friendsC, icons: iconsC, bar: barC })[cell.b.block] || null
            }
        }
    }
    Text {
        visible: lb.line !== "" && !lb.editing
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: lb.margin * 0.8; bottomMargin: lb.margin * 0.6 }
        text: lb.line; color: Qt.rgba(lb.ink.r, lb.ink.g, lb.ink.b, 0.55); font.family: lb.fontFor(""); font.pixelSize: 15; elide: Text.ElideRight
    }

    // ================================================================ the blocks' drawings (display only)
    // A block's card: its colour, see-through and corners.
    component Card: Rectangle {
        property var st: ({})
        radius: st.corners !== undefined ? st.corners : 18
        color: st.colour ? st.colour : Qt.rgba(0.1, 0.07, 0.06, 0.7)
        opacity: st.seeThrough ? 1 - st.seeThrough / 100 * 0.6 : 1
        border.width: st.colour === "#00000000" ? 0 : 1; border.color: Qt.rgba(1, 1, 1, 0.07)
    }
    component Ink: Text { color: lb.ink; font.family: lb.fontFor(""); elide: Text.ElideRight }
    property date now: new Date()
    Timer { interval: 5000; running: lb.visible; repeat: true; triggeredOnStart: true; onTriggered: lb.now = new Date() }

    Component {
        id: clockC
        Item {
            id: ck
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            readonly property bool ring: blk.shows === "ring"
            Rectangle {
                visible: ck.ring
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height); height: width; radius: width / 2
                color: "transparent"; border.width: 3; border.color: Qt.rgba(lb.accent.r, lb.accent.g, lb.accent.b, 0.8)
                Rectangle { anchors.centerIn: parent; width: parent.width * 0.72; height: width; radius: width / 2; color: "transparent"; border.width: 1; border.color: Qt.rgba(lb.accent.r, lb.accent.g, lb.accent.b, 0.35) }
            }
            Ink {
                anchors.fill: parent
                // 12-hour without AM / PM ("7:58"), or 24-hour when that's your choice (Settings › Date & time)
                text: ck.blk.block === "date" ? Qt.formatDate(lb.now, "dddd, d MMMM") : Settings.clock24 || ck.ring ? Qt.formatTime(lb.now, "HH:mm") : Qt.formatTime(lb.now, "h:mm AP").replace(/\s*[AP]M$/i, "")
                font.pixelSize: ck.st.size || (ck.blk.block === "date" ? 28 : 120); font.weight: ck.st.weight || Font.Normal; font.family: lb.fontFor(ck.st.font)
                color: ck.st.text || lb.ink
                horizontalAlignment: ck.ring ? Text.AlignHCenter : (ck.blk.x || 0) > lb.cols / 2 ? Text.AlignRight : Text.AlignLeft
                verticalAlignment: ck.blk.block === "date" ? Text.AlignTop : Text.AlignVCenter
            }
        }
    }

    // weather: the place, the temperature, tonight's line (Tonight too), from Open-Meteo through hearth-widgets.
    function weatherWord(c) {
        if (c === undefined || c === null) return "";
        if (c === 0) return "clear"; if (c <= 3) return "cloudy"; if (c <= 48) return "fog"; if (c <= 57) return "drizzle";
        if (c <= 67) return "rain"; if (c <= 77) return "snow"; if (c <= 82) return "showers"; if (c <= 86) return "snow showers";
        return "storms";
    }
    Component {
        id: weatherC
        Card {
            id: wx
            readonly property var blk: parent.blk
            st: parent.st
            readonly property string place: blk.place || Settings.nightPlace || ""
            readonly property real lat: blk.lat !== undefined ? blk.lat : Settings.nightLat
            readonly property real lon: blk.lon !== undefined ? blk.lon : Settings.nightLon
            property var now: null
            Process {
                id: wxp
                command: ["python3", lb.widgets, "weather", String(wx.lat), String(wx.lon)]
                stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); if (!o.error) wx.now = o; } catch (e) {} } }
            }
            Timer { interval: 900000; running: wx.visible && wx.place !== "" && !lb.editing; repeat: true; triggeredOnStart: true; onTriggered: { wxp.running = false; wxp.running = true; } }
            Column {
                anchors { fill: parent; margins: 22 }
                spacing: 4
                Ink { text: "Weather" + (wx.place ? " · " + wx.place : ""); font.pixelSize: 17; opacity: 0.8; width: parent.width }
                Row {
                    spacing: 14
                    Ink { text: wx.now ? Math.round(wx.now.temp) + "°" : (wx.place ? "—" : ""); font.pixelSize: 44; font.weight: Font.Medium }
                    Ink {
                        anchors.bottom: parent.bottom; anchors.bottomMargin: 6
                        visible: wx.blk.tonight !== false
                        text: !wx.place ? "Pick its place in the lock screen editor" : wx.now && wx.now.days && wx.now.days.length ? lb.weatherWord(wx.now.days[0].code) + " tonight" : ""
                        font.pixelSize: 19; opacity: 0.8
                    }
                }
            }
        }
    }

    // media: what's playing: title, artist, position of length, and (Privacy › Media controls) previous, pause, next.
    function mmss(s) { s = Math.max(0, Math.floor(s || 0)); return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0"); }
    Component {
        id: mediaC
        Card {
            id: md
            st: parent.st
            readonly property var pl: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
            property real pos: 0
            Timer { interval: 1000; running: md.visible && !!md.pl; repeat: true; triggeredOnStart: true; onTriggered: md.pos = md.pl && md.pl.positionSupported ? md.pl.position : 0 }
            Column {
                anchors { fill: parent; margins: 22 }
                spacing: 3
                Ink { text: md.pl ? (md.pl.isPlaying ? "Now playing" : "Paused") : "Now playing"; font.pixelSize: 16; opacity: 0.8 }
                Ink { text: md.pl ? (md.pl.trackTitle || "Something") : "Nothing playing"; font.pixelSize: 21; font.weight: Font.DemiBold; width: parent.width }
                Ink {
                    visible: !!md.pl
                    text: md.pl ? (md.pl.trackArtist || "") + (md.pl.lengthSupported && md.pl.length > 0 ? (md.pl.trackArtist ? " · " : "") + lb.mmss(md.pos) + " of " + lb.mmss(md.pl.length) : "") : ""
                    font.pixelSize: 15; opacity: 0.8; width: parent.width
                }
                Row {
                    visible: !!md.pl && lb.privacy.media !== false
                    spacing: 18; topPadding: 8
                    Repeater {
                        model: [{ t: "⏮", a: "prev" }, { t: md.pl && md.pl.isPlaying ? "⏸" : "▶", a: "play" }, { t: "⏭", a: "next" }]
                        delegate: Ink {
                            required property var modelData
                            text: modelData.t; font.pixelSize: 20
                            // steers what's playing only: nothing opens, nothing unlocks
                            MouseArea {
                                anchors.fill: parent; anchors.margins: -8
                                enabled: !lb.editing
                                onClicked: {
                                    const p = md.pl; if (!p) return;
                                    if (parent.modelData.a === "prev" && p.canGoPrevious) p.previous();
                                    else if (parent.modelData.a === "next" && p.canGoNext) p.next();
                                    else if (parent.modelData.a === "play" && p.canTogglePlaying) p.togglePlaying();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // notifications: from apps you let show on the lock screen (Settings › Notifications), since it locked. Privacy:
    // names only (the title), messages too, just how many, or none.
    Component {
        id: notesC
        Card {
            id: nt
            st: parent.st
            readonly property string mode: lb.privacy.notifications || "names"
            visible: mode !== "none"
            readonly property var list: lb.editing ? [{ summary: "Mantel is ready in the Store", body: "" }, { summary: "Dad", body: "trains 2010 crashed again" }]
                : (DeskState.history || []).filter(h => h.lock && h.time >= lb.lockedAt)
            Column {
                anchors { fill: parent; margins: 22 }
                spacing: 6
                Ink { text: "While you were away"; font.pixelSize: 16; opacity: 0.8 }
                Ink { visible: nt.list.length === 0; text: "Nothing new"; font.pixelSize: 18; opacity: 0.85 }
                Repeater {
                    model: nt.mode === "count" ? [] : nt.list.slice(0, 2)
                    delegate: Ink {
                        required property var modelData
                        width: parent.width
                        text: nt.mode === "messages" && modelData.body ? modelData.summary + ": \"" + modelData.body + "\"" : modelData.summary
                        font.pixelSize: 18
                    }
                }
                Ink {
                    visible: nt.list.length > 0 && (nt.mode === "count" || nt.list.length > 2 || nt.mode === "names")
                    width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone
                    text: nt.mode === "count" ? nt.list.length + " new" + (nt.list.length === 1 ? " notification" : " notifications") + " · nothing else shown (your choice)"
                        : (nt.list.length > 2 ? (nt.list.length - 2) + " more · " : "") + (nt.mode === "names" ? "names only, no messages shown (your choice)" : "")
                    font.pixelSize: 15; opacity: 0.7
                }
            }
        }
    }

    // downloads: what Steam is downloading on this device (hearth-lockscreen downloads).
    Component {
        id: downloadsC
        Card {
            id: dl
            st: parent.st
            property var items: lb.editing ? [{ name: "Ember Trails", percent: 62 }] : []
            Process {
                id: dlp
                command: ["python3", lb.helper, "downloads"]
                stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); if (o && Array.isArray(o.items)) dl.items = o.items; } catch (e) {} } }
            }
            Timer { interval: 20000; running: dl.visible && !lb.editing; repeat: true; triggeredOnStart: true; onTriggered: { dlp.running = false; dlp.running = true; } }
            Column {
                anchors { fill: parent; margins: 22 }
                spacing: 6
                Ink { text: "Your downloads"; font.pixelSize: 16; opacity: 0.8 }
                Ink { text: dl.items.length ? dl.items[0].name + " · " + dl.items[0].percent + "%" + (dl.items.length > 1 ? "  (+" + (dl.items.length - 1) + ")" : "") : "Nothing downloading"; font.pixelSize: 19; width: parent.width }
                Rectangle {
                    visible: dl.items.length > 0
                    width: parent.width; height: 6; radius: 3; color: Qt.rgba(1, 1, 1, 0.15)
                    Rectangle { width: parent.width * Math.min(1, (dl.items.length ? dl.items[0].percent : 0) / 100); height: parent.height; radius: 3; color: lb.accent }
                }
            }
        }
    }

    // battery: this device's percent and time left, and the controllers' batteries.
    Component {
        id: batteryC
        Card {
            id: bt
            st: parent.st
            readonly property var dev: UPower.displayDevice
            readonly property bool has: !!dev && dev.isLaptopBattery
            readonly property var pads: (UPower.devices ? UPower.devices.values : []).filter(d => d && d.type === UPowerDeviceType.GamingInput && d.percentage > 0)
            function left(s) { if (!s || s <= 0) return ""; const h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60); return " · " + (h ? h + " h " : "") + m + " m"; }
            Row {
                anchors { left: parent.left; leftMargin: 22; verticalCenter: parent.verticalCenter }
                spacing: 16
                Icon { name: "batt"; size: 30; color: "#7fe0a3"; anchors.verticalCenter: parent.verticalCenter }
                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    Ink { text: lb.editing ? "71% · 3 h 20 m" : bt.has ? Math.round(bt.dev.percentage * 100) + "%" + bt.left(bt.dev.timeToEmpty) : "Plugged in"; font.pixelSize: 20 }
                    Ink {
                        visible: text !== ""
                        text: lb.editing ? "Controller 88%" : bt.pads.map(p => (p.model || "Controller") + " " + Math.round(p.percentage * 100) + "%").join(" · ")
                        font.pixelSize: 15; opacity: 0.8
                    }
                }
            }
        }
    }

    // calendar: today, and what's next today (Hearth Time and your accounts' calendars, through hearth-widgets).
    Component {
        id: calendarC
        Card {
            id: cal
            st: parent.st
            property var next: null
            Process {
                id: calp
                command: ["python3", lb.widgets, "events"]
                stdout: StdioCollector {
                    onStreamFinished: {
                        try {
                            const o = JSON.parse(text.trim().split("\n").pop());
                            const today = Qt.formatDate(new Date(), "yyyy-MM-dd"), nowS = Date.now() / 1000;
                            cal.next = (o.events || []).find(e => String(e.date || "") === today && (!e.start || e.start >= nowS - 1800)) || null;
                        } catch (e) {}
                    }
                }
            }
            Timer { interval: 600000; running: cal.visible && !lb.editing; repeat: true; triggeredOnStart: true; onTriggered: { calp.running = false; calp.running = true; } }
            Column {
                anchors { fill: parent; margins: 22 }
                spacing: 4
                Ink { text: Qt.formatDate(lb.now, "dddd"); font.pixelSize: 16; opacity: 0.8; color: cal.st.text || lb.ink; font.family: lb.fontFor(cal.st.font) }
                Ink { text: Qt.formatDate(lb.now, "d MMMM"); font.pixelSize: 30; color: cal.st.text || lb.ink; font.family: lb.fontFor(cal.st.font) }
                Ink {
                    width: parent.width
                    text: cal.next ? (cal.next.start ? Qt.formatTime(new Date(cal.next.start * 1000), Settings.clock24 ? "HH:mm" : "h:mm AP") + "  " : "") + (cal.next.title || "") : "Nothing else today"
                    font.pixelSize: 17; opacity: 0.85; color: cal.st.text || lb.ink; font.family: lb.fontFor(cal.st.font)
                }
            }
        }
    }

    Component {
        id: pictureC
        Rectangle {
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            radius: st.corners || 12; clip: true; color: "transparent"
            Image {
                anchors.fill: parent
                source: parent.blk.value ? "file://" + String(parent.blk.value).replace(/^~/, Quickshell.env("HOME")) : Qt.resolvedUrl("images/ember-glow.png")
                fillMode: Image.PreserveAspectCrop; asynchronous: true
            }
        }
    }

    Component {
        id: wordsC
        Ink {
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            text: blk.value || ""
            font.pixelSize: st.size || 22; font.weight: st.weight || Font.Normal; font.family: lb.fontFor(st.font); color: st.text || lb.ink
            wrapMode: Text.WordWrap; elide: Text.ElideNone; verticalAlignment: Text.AlignVCenter
        }
    }

    // friends online: from Steam's own list on this device (hearth-lockscreen friends).
    Component {
        id: friendsC
        Card {
            id: fr
            st: parent.st
            property var online: lb.editing ? [{ name: "Dad", game: "Trains 2010" }, { name: "Sam", game: "Quad Mech" }] : []
            Process {
                id: frp
                command: ["python3", lb.helper, "friends"]
                stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); if (o && Array.isArray(o.online)) fr.online = o.online; } catch (e) {} } }
            }
            Timer { interval: 300000; running: fr.visible && !lb.editing; repeat: true; triggeredOnStart: true; onTriggered: { frp.running = false; frp.running = true; } }
            Column {
                anchors { fill: parent; margins: 22 }
                spacing: 4
                Ink { text: "Friends online"; font.pixelSize: 16; color: lb.accent }
                Ink { visible: fr.online.length === 0; text: "No one right now"; font.pixelSize: 17; opacity: 0.85 }
                Repeater {
                    model: fr.online.slice(0, 4)
                    delegate: Ink { required property var modelData; width: parent.width; text: modelData.name + (modelData.game ? " · " + modelData.game : ""); font.pixelSize: 17 }
                }
            }
        }
    }

    // icons: the look's desktop icons, shown only (nothing opens from a lock screen).
    Component {
        id: iconsC
        Column {
            id: ic
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            spacing: 22
            readonly property var known: ({ home: ["home", "My Hearth"], bin: ["trash", "Bin"], files: ["folder", "Files"], games: ["pad", "Games"],
                                            store: ["store", "Hearth Store"], music: ["headphones", "Music"], notes: ["doc", "Notes"], settings: ["gear", "Settings"] })
            Repeater {
                model: (ic.blk.items || []).filter(i => !!ic.known[i])
                delegate: Column {
                    required property var modelData
                    width: 100; spacing: 4
                    Icon { anchors.horizontalCenter: parent.horizontalCenter; name: ic.known[parent.modelData][0]; size: 28; color: ic.st.text || lb.ink }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: ic.known[parent.modelData][1]; color: ic.st.text || lb.ink; font.family: lb.fontFor(ic.st.font); font.pixelSize: 16 }
                }
            }
        }
    }

    // bar: "Hearth is locked" (its words) and the time, across its place.
    Component {
        id: barC
        Rectangle {
            readonly property var blk: parent.blk
            readonly property var st: parent.st
            color: st.colour || "#c0c0c0"; radius: st.corners || 0
            Text { anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter } text: parent.blk.value || "Hearth is locked"; color: parent.st.text || "#000000"; font.family: lb.fontFor(parent.st.font); font.pixelSize: 18; font.bold: true }
            Text { anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter } text: Qt.formatTime(lb.now, Settings.clock24 ? "HH:mm" : "h:mm AP"); color: parent.st.text || "#000000"; font.family: lb.fontFor(parent.st.font); font.pixelSize: 18 }
        }
    }

    // A block your AI made: drawn by its own boxed-in program (OwnBlock), with nothing to press here (its asks wait for
    // the desktop).
    Component {
        id: ownC
        Item {
            enabled: false
            OwnBlock { anchors.fill: parent; blk: parent.parent.blk; st: parent.parent.st; ink: lb.ink; fontName: lb.fontFor("") }
        }
    }
}
