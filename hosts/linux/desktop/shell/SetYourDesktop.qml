// Hearth Settings › System › Your desktop: Desktop Mode is Mantel (Hearth Classic), on KWin (1.14) or labwc, the window
// manager it used before (docs/DESKTOPS.md "Mantel on KWin"; nimbus-session's safe start falls back to labwc and says
// why; twice in a row it stays on labwc until "Use KWin again" here). 1.13's KDE Plasma and GNOME choices
// were retired in 1.14 (the owner, Nov 16; archive/desktops-kde-gnome, docs/archive/DESKTOPS-KDE-GNOME.md): someone who
// had one picked gets Mantel and a notice saying so (DesktopNotice.qml once, the banner here while it's the last one).
// Everything through hearth-your-desktop. Opening the page only reads; nothing here changes by itself.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Your desktop"
    property var st: null
    property string msg: ""
    property bool bad: false
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    function run(args, done) { act.run(args, r => { page.st = r; if (done) page.say(done); }, e => { page.say(e, true); page.refresh(); }); }
    readonly property string wmName: "labwc"
    readonly property string nowName: !st || !st.wmNow ? "" : st.wmNow === "labwc" ? "labwc" : st.wmNow === "firehold" ? "Firehold" : "KWin"
    Component.onCompleted: { if (fedora) fRefresh(); else refresh(); blocksRefresh(); }
    SetJob { id: job; helper: "hearth-your-desktop" }
    SetJob { id: act; helper: "hearth-your-desktop" }

    Text {
        visible: !page.fedora
        Layout.fillWidth: true; Layout.bottomMargin: 8
        text: "Desktop Mode is Mantel, Hearth's own desktop."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetBanner {
        visible: !page.fedora && !!page.st && !!page.st.notice && (page.st.notice.title || "") !== ""
        kind: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        icon: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        title: page.st && page.st.notice ? page.st.notice.title : ""
        sub: page.st && page.st.notice ? page.st.notice.sub : ""
    }

    // 1.18 (the owner, Nov 23: "only 2 versions i want are Mantel and the Rust version of it, add a switcher in the
    // Setting"): Mantel or Mantel in Rust, both on labwc. Picking one saves HEARTH_MANTEL (hearth-your-desktop mantel)
    // for the next start of Desktop Mode; Mantel in Rust only once it's installed from the Hearth Store.
    Repeater {
        model: page.fedora ? [] : [
            { id: "mantel", name: "Mantel", letter: "M", sub: "Hearth's own desktop with all of Hearth's apps: the bar, the dock, Quick Access, notifications and the controller. Built into Hearth OS." },
            { id: "rust", name: "Mantel in Rust", letter: "R", sub: "The same desktop, written in Rust: lighter and quicker. From the Hearth Store." }
        ]
        delegate: SetBase {
            id: card
            required property var modelData
            readonly property bool picked: !!page.st && (page.st.mantel || "mantel") === modelData.id
            readonly property bool here: modelData.id === "mantel" || (!!page.st && page.st.mantelRustHere)
            Layout.fillWidth: true
            implicitHeight: Math.max(96, ccol.implicitHeight + 36)
            border.width: picked ? 2 : 1
            border.color: picked ? Theme.accent : SetStyle.tint(Theme.text, 0.05)
            interactive: !picked && here
            onActivated: page.run(["mantel", modelData.id], modelData.name + " opens from the next time Desktop Mode starts ✓")
            RowLayout {
                anchors { fill: parent; leftMargin: 22; rightMargin: 22 }
                spacing: 18
                Rectangle {   // the radio
                    Layout.alignment: Qt.AlignVCenter
                    width: 22; height: 22; radius: 11; color: "transparent"
                    border.width: 2; border.color: card.picked ? Theme.accent : Theme.muted
                    Rectangle { visible: card.picked; anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: Theme.accent }
                }
                Rectangle {   // its letter
                    Layout.alignment: Qt.AlignVCenter
                    width: 54; height: 54; radius: 12; color: card.modelData.id === "rust" ? "#3a2a24" : "#4a3a24"
                    Text { anchors.centerIn: parent; text: card.modelData.letter; color: card.modelData.id === "rust" ? "#e8743a" : "#f0a840"; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.Bold }
                }
                ColumnLayout {
                    id: ccol
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    spacing: 6
                    RowLayout {
                        spacing: 10
                        Text { text: card.modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                        SetChip { visible: card.modelData.id === "rust"; text: "Rust"; kind: "new" }
                        SetChip { visible: card.picked; text: "picked"; kind: "ok" }
                        SetChip { visible: !card.here; text: "not installed"; kind: "warn" }
                    }
                    Text { Layout.fillWidth: true; text: card.modelData.sub + (card.here ? "" : " Get it in the Hearth Store first, then pick it here."); color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
                }
            }
        }
    }
    SetRow {
        visible: !page.fedora
        title: "Running now"
        sub: !page.st ? "" : page.nowName === "" ? "Desktop Mode isn't open" : "Mantel on " + page.nowName
        chip: page.nowName; chipKind: "ok"
        interactive: false
    }
    SetRow {
        visible: !page.fedora
        title: "Safe start"
        sub: "If Mantel in Rust closes right after it starts, Mantel opens instead for that sign-in. Your choice stays saved."
        chip: "always on"; chipKind: "ok"
        interactive: false
    }
    SetRow {
        visible: !page.fedora
        title: "KWin, Firehold, KDE Plasma and GNOME"
        sub: "Not used since Hearth 1.18: Desktop Mode is Mantel or Mantel in Rust, on labwc. A pick from before opens Mantel, and Hearth tells you once."
        chip: "retired"; chipKind: "warn"
        interactive: false
    }
    // >>> labwc on the Fedora edition (1.16, docs/LABWC-FEDORA.md): Mantel running on the Fedora edition's labwc
    // (HEARTH_FEDORA_LABWC=1, nimbus-session run_fedora_labwc): the four desktops of that edition, as Hearth Settings
    // (Rust) shows them. Picks go through hearth-labwc-desktop and hearth-firehold-plasma (one session.conf line).
    readonly property bool fedora: Quickshell.env("HEARTH_FEDORA_LABWC") === "1"
    property var fst: null               // hearth-labwc-desktop status
    property string fpick: Quickshell.env("HEARTH_FIREHOLD_SESSION") === "1" ? "firehold-labwc" : "classic"
    function fRefresh() { fjob.run(["status"], r => { page.fst = r; page.fpick = r.choice === "firehold" ? "firehold-labwc" : r.choice === "classic" ? "classic" : page.fpick; }, e => page.say(e, true)); }
    function fUse(id) {
        const done = () => { page.fpick = id; page.say("Saved. It opens the next time you switch to Desktop Mode ✓"); page.fRefresh(); };
        const fail = e => page.say(e, true);
        if (id === "classic") fjob.run(["choose", "classic"], done, fail);
        else if (id === "firehold-labwc") fjob.run(["choose", "firehold"], done, fail);
        else fjob.run(["choose", "plasma"], () => fpjob.run(["choose", id === "firehold-plasma" ? "firehold" : "plasma"], done, fail), fail);
    }
    // (an absolute path: Quickshell doesn't let a relative one leave the shell's folder)
    SetJob { id: fjob; helper: Quickshell.shellDir.replace(/\/shell\/?$/, "") + "/labwc-fedora/hearth-labwc-desktop" }
    SetJob { id: fpjob; helper: "/usr/bin/hearth-firehold-plasma" }
    Text {
        visible: page.fedora
        Layout.fillWidth: true; Layout.bottomMargin: 8
        text: "What Desktop Mode opens. Nothing changes until you switch to Desktop Mode again."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    Repeater {
        // 1.17 (the owner, Nov 21): Mantel is the default and the one choice shown; KDE Plasma and Firehold are hidden,
        // not removed, and a device that picked one keeps it (the note below says so).
        model: page.fedora ? [
            { id: "classic", name: "Mantel (Hearth Classic)", sub: "Hearth's own desktop: the bar, the dock, Quick Access and all of Hearth's apps (the default)" }] : []
        delegate: SetRow {
            required property var modelData
            icon: "desktops"; title: modelData.name; sub: modelData.sub
            chip: page.fpick === modelData.id ? "Picked" : ""
            chevron: page.fpick !== modelData.id
            interactive: page.fpick !== modelData.id && !fjob.busy && !fpjob.busy
            onActivated: page.fUse(modelData.id)
        }
    }
    SetRow {
        // A device that picked a desktop that's hidden now keeps it (red flag 9) and is told here.
        visible: page.fedora && page.fpick === "firehold-labwc"
        icon: "info"; title: "Right now"
        sub: "This PC opens Firehold on labwc, your earlier pick. Use Mantel to switch to Hearth Classic."
        interactive: false
    }
    SetRow {
        visible: page.fedora
        title: "Safe start"
        sub: "If Mantel doesn't start, KDE Plasma opens for that sign-in and Hearth tells you why. Twice in a row, KDE Plasma opens until you pick it again. Your choice stays saved."
        chip: "always on"; chipKind: "ok"
        interactive: false
    }
    // <<< labwc on the Fedora edition

    // >>> Build anything (build-anything boards 1-3, 7): how your desktop looks. Hearth OS Mantel, or a whole-screen
    // desktop of blocks (ScreenLayout.qml), changed at once (no restart) and back to Mantel any time.
    Text {
        Layout.fillWidth: true; Layout.topMargin: 18
        text: "How your desktop looks"
        color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
    }
    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 4
        text: "Hearth OS Mantel, or a whole desktop of your own: start from one of these and change anything. Either way it's Hearth underneath, and your windows, Files and Alt+Tab work the same."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "desktops"; title: "Hearth OS Mantel"
        sub: "The bar, the dock, Quick Access and Game Mode one press away"
        chip: !ScreenLayout.on ? "Using now" : ""
        chevron: ScreenLayout.on
        interactive: ScreenLayout.on
        onActivated: { const e = ScreenLayout.useMantel(); page.say(e || "Back to Hearth OS Mantel ✓", !!e); }
    }
    Repeater {
        model: ScreenLayout.readyMade
        delegate: SetRow {
            required property var modelData
            icon: "palette"; title: modelData.name
            sub: modelData.id === "console" ? "Your profile and the time in the corners, your last game big, rows of games and apps, a Guide on Home"
               : modelData.id === "iconrow" ? "Tabs across the top, your games as icons, the picked one's art filling the screen, Play"
               : modelData.id === "cockpit" ? "A big clock in the middle, your last game and your apps on glowing panels"
               : modelData.id === "cabin" ? "Warm wood colours, the time, what's playing and your games"
               : modelData.id === "notebook" ? "Cream paper and dark ink, your apps and windows as notes"
               : "A teal screen, square buttons, the clock in the corner"
            chip: ScreenLayout.on && ScreenLayout.from === modelData.id ? "Using now" : ""
            chevron: !(ScreenLayout.on && ScreenLayout.from === modelData.id)
            interactive: chevron
            onActivated: { const e = ScreenLayout.use(modelData.id); page.say(e || modelData.name + " is your desktop now. Home opens its Guide; Settings › Your desktop goes back ✓", !!e); }
        }
    }
    SetRow {
        icon: "edit"; title: ScreenLayout.on ? "Change this desktop" : "Build your own"
        sub: ScreenLayout.on ? "The block editor: move, size and restyle its blocks, add new ones, or say what you want" : "Start from nothing in the block editor, or from one of these"
        chevron: true
        onActivated: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "screen", ScreenLayout.on ? "edit" : "editNothing"])
    }
    SetRow {
        visible: ScreenLayout.failed
        icon: "warn"; title: "Your desktop couldn't be drawn"
        sub: "Mantel's bar and dock are back so nothing's lost. Pick it again to try once more."
        interactive: false
    }
    SetRow {
        icon: "info"; title: "Show the first-start choice"
        sub: "The full-screen \"How do you want your desktop?\" a new install opens with"
        chevron: true
        onActivated: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "screen", "firstStart"])
    }
    // >>> The lock screen made of blocks (lock-screen-blocks.md): plain, matching your desktop, or your own (the editor).
    // Only what you press here changes it (hearth-lockscreen: merged, atomic, a .bak kept).
    property var lockSt: null
    FileView {
        id: lockFileV
        path: Quickshell.env("HOME") + "/.local/share/nimbus/lock-screen.json"
        watchChanges: true; onFileChanged: reload()
        onLoaded: { try { const t = text(); page.lockSt = t && t.trim() ? JSON.parse(t) : null; } catch (e) { page.lockSt = null; } }
        onLoadFailed: page.lockSt = null
    }
    // what the lock screen is now: no file follows a ready-made desktop, else plain (as LockBlocks.qml decides)
    readonly property string lockNow: page.lockSt ? (page.lockSt.on !== true ? "plain" : page.lockSt.match === true ? "match" : "own") : (ScreenLayout.on ? "match" : "plain")
    Process { id: lockSet; onExited: lockFileV.reload() }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 18
        text: "Your lock screen"
        color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
    }
    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 4
        text: "Made of the same blocks as your desktop. Nothing on it can open an app, a file or a setting; only your password (or fingerprint) unlocks, whatever it looks like."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "lock"; title: "Plain lock screen"
        sub: "The time, the date and the password box"
        chip: page.lockNow === "plain" ? "Using now" : ""
        chevron: page.lockNow !== "plain"; interactive: chevron
        onActivated: { lockSet.command = ["python3", Qt.resolvedUrl("hearth-lockscreen").toString().replace(/^file:\/\//, ""), "plain"]; lockSet.running = true; page.say("The plain lock screen from the next lock ✓"); }
    }
    SetRow {
        icon: "palette"; title: "Match your desktop"
        sub: ScreenLayout.on ? "The lock screen made for " + ((ScreenLayout.readyMade.find(r => r.id === ScreenLayout.from) || { name: "your desktop" }).name) : "Hearth's own lock screen of blocks: the clock, the weather, what's playing, while you were away, downloads and the battery"
        chip: page.lockNow === "match" ? "Using now" : ""
        chevron: page.lockNow !== "match"; interactive: chevron
        onActivated: { lockSet.command = ["python3", Qt.resolvedUrl("hearth-lockscreen").toString().replace(/^file:\/\//, ""), "match"]; lockSet.running = true; page.say("Your lock screen matches your desktop from the next lock ✓"); }
    }
    SetRow {
        icon: "edit"; title: page.lockNow === "own" ? "Change your lock screen" : "Make your own lock screen"
        sub: "The block editor for the lock screen: move, size and restyle its blocks, or say what you want"
        chip: page.lockNow === "own" ? "Using now" : ""
        chevron: true
        onActivated: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "screen", "editLock"])
    }
    // <<< the lock screen
    // Your blocks (build-anything board 5): the blocks your AI made, each on or off, and what you said to its ask.
    property var myBlocks: []
    function blocksRefresh() { blocksP.command = ScreenLayout.layoutCmd(["blocks-mine"]); blocksP.running = true; }
    Process { id: blocksP; stdout: StdioCollector { onStreamFinished: { try { const v = JSON.parse(text.trim() || "[]"); if (Array.isArray(v)) page.myBlocks = v; } catch (e) {} } } }
    Process { id: blockSet; onExited: page.blocksRefresh() }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 18
        text: "Your blocks"
        color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
    }
    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 4
        text: page.myBlocks.length ? "Made by your AI for your desktop. Each runs on its own and asks before it reaches anything. Press one to turn it off or on." : "None yet: in the block editor, describe a block Hearth hasn't got and your AI makes it."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.myBlocks
        delegate: SetRow {
            required property var modelData
            icon: "sparkle"; title: modelData.name
            sub: (modelData.reach && modelData.reach.kind ? (modelData.allowed === "always" ? "Allowed to reach " : modelData.allowed === "no" ? "You said No to " : "Asks before it reaches ") + modelData.reach.value + " · " : "") + "\"" + modelData.asked + "\""
            chip: modelData.on ? "On" : "Off"; chipKind: modelData.on ? "ok" : "warn"
            chevron: true
            onActivated: { blockSet.command = ScreenLayout.layoutCmd(["block-on", modelData.id, modelData.on ? "false" : "true"]); blockSet.running = true; }
        }
    }
    SetRow {
        visible: page.myBlocks.some(b => b.allowed !== "")
        icon: "shield"; title: "Ask again before they reach anything"
        sub: "Forgets every Allow and No you gave your blocks"
        chevron: true
        onActivated: { for (const b of page.myBlocks) Quickshell.execDetached(ScreenLayout.layoutCmd(["block-allow", b.id, "ask"])); refreshT.start(); }
        Timer { id: refreshT; interval: 800; onTriggered: page.blocksRefresh() }
    }
    // <<< Build anything
    Text { visible: page.msg !== ""; Layout.fillWidth: true; text: page.msg; color: page.bad ? SetStyle.warn : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
}
