// Hearth Settings › System › Your desktop: Desktop Mode is Mantel (Hearth Classic), on KWin (1.14) or labwc, the window
// manager it used before (docs/DESKTOPS.md "Mantel on KWin"; nimbus-session's safe start falls back to labwc and says
// why; twice in a row it stays on labwc until "Use KWin again" here). 1.13's KDE Plasma and GNOME choices
// were retired in 1.14 (the owner, Nov 16; archive/desktops-kde-gnome, docs/archive/DESKTOPS-KDE-GNOME.md): someone who
// had one picked gets Mantel and a notice saying so (DesktopNotice.qml once, the banner here while it's the last one).
// Everything through hearth-your-desktop. Opening the page only reads; nothing here changes by itself.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Your desktop"
    property var st: null
    property string msg: ""
    property bool bad: false
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    function run(args, done) { act.run(args, r => { page.st = r; if (done) page.say(done); }, e => { page.say(e, true); page.refresh(); }); }
    readonly property string wmName: !st ? "KWin" : st.wm === "labwc" ? "labwc" : "KWin"
    readonly property string nowName: !st || !st.wmNow ? "" : st.wmNow === "labwc" ? "labwc" : st.wmNow === "firehold" ? "Firehold" : "KWin"
    Component.onCompleted: if (fedora) fRefresh(); else refresh()
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
        action: page.st && page.st.kwinPaused ? "Use KWin again" : ""
        onActivated: if (page.st && page.st.kwinPaused) page.run(["wm", "kwin"], "Mantel uses KWin again from the next time Desktop Mode starts ✓")
        icon: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        title: page.st && page.st.notice ? page.st.notice.title : ""
        sub: page.st && page.st.notice ? page.st.notice.sub : ""
    }

    SetBase {
        visible: !page.fedora
        Layout.fillWidth: true
        implicitHeight: Math.max(96, col.implicitHeight + 36)
        border.width: 2
        border.color: Theme.accent
        interactive: false
        RowLayout {
            anchors { fill: parent; leftMargin: 22; rightMargin: 22 }
            spacing: 18
            Rectangle {   // the radio
                Layout.alignment: Qt.AlignVCenter
                width: 22; height: 22; radius: 11; color: "transparent"
                border.width: 2; border.color: Theme.accent
                Rectangle { anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: Theme.accent }
            }
            Rectangle {   // its letter
                Layout.alignment: Qt.AlignVCenter
                width: 54; height: 54; radius: 12; color: "#4a3a24"
                Text { anchors.centerIn: parent; text: "M"; color: "#f0a840"; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.Bold }
            }
            ColumnLayout {
                id: col
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 6
                RowLayout {
                    spacing: 10
                    Text { text: "Mantel (Hearth Classic)"; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                    SetChip { text: "built in · using now" + (page.nowName !== "" ? " on " + page.nowName : ""); kind: "ok" }
                }
                Text { Layout.fillWidth: true; text: "Hearth's own desktop with all of Hearth's apps: the bar, the dock, Quick Access, notifications and the controller."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
            }
        }
    }
    SetDropdown {
        visible: !page.fedora
        title: "Window manager"
        sub: "What draws, moves and snaps Mantel's windows, from the next time Desktop Mode starts. KWin (KDE's) is the default from Hearth 1.14; labwc is the one Mantel used before."
        options: [{ value: "kwin", label: "KWin" }, { value: "labwc", label: "labwc" }]
        current: !page.st ? "kwin" : page.st.wm === "firehold" ? page.st.wmBefore : page.st.wm
        boxWidth: 200
        onChosen: v => page.run(["wm", v], "Mantel uses " + (v === "labwc" ? "labwc" : "KWin") + " from the next time Desktop Mode starts ✓")
    }
    // 2.0 preview: Hearth's own compositor (docs/FIREHOLD.md). Off unless you turn it on; if it doesn't start,
    // Mantel opens on KWin (then labwc) and says why. Turning it off goes back to what was picked before.
    SetDropdown {
        // Hidden until it's unlocked (SettingsNav.tapVersion); locked, nothing here mentions it. @noindex
        visible: !page.fedora && !!page.st && page.st.fireholdUnlocked
        icon: "desktops"; title: "Firehold (preview)"
        sub: !page.st ? "" : !page.st.fireholdHere && page.st.fireholdMode === "off"
            ? "Firehold, Hearth 2.0's own desktop, still being built. It isn't on this device yet."
            : page.st.fireholdPaused ? "It couldn't start twice in a row, so Always is off for now and the normal desktop (KWin) starts. Pick Always to turn it back on."
            : "Hearth 2.0's own desktop, a preview: some things don't work yet. Try once is the next Desktop Mode start only; Always is every start. If it can't start, the normal desktop (KWin) starts and Hearth tells you why. " + (page.st.leaveKey || "Ctrl+Alt+Backspace") + " always goes back to the normal desktop."
        options: [{ value: "off", label: "Off" }, { value: "once", label: "Try once" }, { value: "always", label: "Always" }]
        current: !page.st ? "off" : page.st.fireholdPaused ? "off" : page.st.fireholdMode
        boxWidth: 200
        onChosen: v => {
            if (!page.st) return;
            if (v !== "off" && !page.st.fireholdHere) { page.say("Firehold isn't on this device yet", true); return; }
            if (v === "once") page.run(["wm", "firehold-once"], "Firehold (preview) for the next Desktop Mode start only, then the normal desktop again ✓");
            else if (v === "always") page.run(["wm", "firehold"], "Firehold (preview) every time Desktop Mode starts ✓");
            else page.run(["wm", page.st.wm === "firehold" ? (page.st.wmBefore || "kwin") : page.st.wm], "Firehold is off: Mantel uses " + ((page.st.wm === "firehold" ? page.st.wmBefore : page.st.wm) === "labwc" ? "labwc" : "KWin") + " from the next time Desktop Mode starts ✓");
        }
    }
    SetRow {
        visible: !page.fedora && !!page.st && page.st.fireholdUnlocked   // @noindex
        icon: "close"; title: "Hide Firehold again"
        sub: "Turns Firehold off (the normal desktop from the next start) and hides it here"
        chevron: true
        onActivated: page.run(["relock"], "Firehold is off and hidden again ✓")
    }
    SetRow {
        visible: !page.fedora
        title: "Running now"
        sub: !page.st ? "" : page.nowName === "" ? "Desktop Mode isn't open"
            : page.st.kwinPaused ? "KWin didn't start twice in a row, so Mantel is on labwc until you use KWin again"
            : page.st.wm === "firehold" && page.st.wmNow !== "firehold" ? (page.st.safeGraphics ? "Safe graphics this time, so labwc" : !page.st.fireholdHere ? "Firehold isn't installed, so " + page.nowName : "Firehold didn't start this time, so " + page.nowName)
            : page.st.wm === "kwin" && page.nowName === "labwc" ? (page.st.safeGraphics ? "Safe graphics this time, so labwc" : !page.st.kwinHere ? "KWin isn't installed, so labwc" : "KWin didn't start this time, so labwc")
            : "Mantel on " + page.nowName
        chip: page.nowName; chipKind: page.st && ((page.st.wm === "kwin" && page.nowName === "labwc") || (page.st.wm === "firehold" && page.st.wmNow !== "firehold")) ? "warn" : "ok"
        interactive: false
    }
    SetRow {
        visible: !page.fedora
        title: "Safe start"
        sub: "If KWin doesn't start, Mantel opens on labwc for that sign-in and tells you why. Twice in a row, it stays on labwc until you pick KWin again. Your choice stays saved."
        chip: "always on"; chipKind: "ok"
        interactive: false
    }
    SetRow {
        visible: !page.fedora
        title: "KDE Plasma and GNOME"
        sub: "Retired in Hearth 1.14: Mantel is the desktop. A KDE Plasma or GNOME pick from before opens Mantel, and Hearth tells you once."
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
        model: page.fedora ? [
            { id: "plasma", name: "KDE Plasma", sub: "The desktop, with Hearth's look (the default)" },
            { id: "firehold-plasma", name: "Firehold (Plasma, test)", sub: "Firehold's Couch and Desk on KDE Plasma" },
            { id: "classic", name: "Mantel (Hearth Classic, labwc)", sub: "Hearth's own desktop: the bar, the dock, Quick Access and all of Hearth's apps, on labwc" },
            { id: "firehold-labwc", name: "Firehold on labwc (test)", sub: "Mantel's Firehold: the Couch for the controller, the Desk for the mouse, on labwc" }] : []
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
        visible: page.fedora
        title: "Safe start"
        sub: "If Mantel or Firehold on labwc doesn't start, KDE Plasma opens for that sign-in and Hearth tells you why. Twice in a row, KDE Plasma opens until you pick it again. Your choice stays saved."
        chip: "always on"; chipKind: "ok"
        interactive: false
    }
    // <<< labwc on the Fedora edition
    Text { visible: page.msg !== ""; Layout.fillWidth: true; text: page.msg; color: page.bad ? SetStyle.warn : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
}
