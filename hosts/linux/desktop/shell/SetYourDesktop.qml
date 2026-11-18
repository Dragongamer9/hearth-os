// Hearth Settings › System › Your desktop: Desktop Mode is Mantel (Hearth Classic) on labwc. 1.14 moved it to KWin; 1.15
// archived KWin (the owner, Nov 17; docs/archive/KWIN.md): a saved KWin choice opens labwc, told once. 1.13's KDE Plasma and GNOME choices
// were retired in 1.14 (the owner, Nov 16; archive/desktops-kde-gnome, docs/archive/DESKTOPS-KDE-GNOME.md): someone who
// had one picked gets Mantel and a notice saying so (DesktopNotice.qml once, the banner here while it's the last one).
// Everything through hearth-your-desktop. Opening the page only reads; nothing here changes by itself.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Your desktop"
    property var st: null
    property string msg: ""
    property bool bad: false
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    function run(args, done) { act.run(args, r => { page.st = r; if (done) page.say(done); }, e => { page.say(e, true); page.refresh(); }); }
    readonly property string nowName: !st || !st.wmNow ? "" : st.wmNow === "firehold" ? "Firehold" : st.wmNow === "kwin" ? "KWin" : "labwc"
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-your-desktop" }
    SetJob { id: act; helper: "hearth-your-desktop" }

    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 8
        text: "Desktop Mode is Mantel, Hearth's own desktop."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetBanner {
        visible: !!page.st && !!page.st.notice && (page.st.notice.title || "") !== ""
        kind: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        icon: page.st && page.st.notice && page.st.notice.kind === "retired" ? "info" : "warn"
        title: page.st && page.st.notice ? page.st.notice.title : ""
        sub: page.st && page.st.notice ? page.st.notice.sub : ""
    }

    SetBase {
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
    SetRow {
        title: "Window manager"
        sub: "labwc draws, moves and snaps Mantel's windows, as in Hearth 1.13. KWin, which 1.14 tried, is archived."
        chip: "labwc"; chipKind: "ok"
        interactive: false
    }
    // 2.0 preview: Hearth's own compositor (docs/FIREHOLD.md). Off unless you turn it on; if it doesn't start,
    // Mantel opens on labwc and says why. Turning it off goes back to labwc.
    SetDropdown {
        // Hidden until it's unlocked (SettingsNav.tapVersion); locked, nothing here mentions it. @noindex
        visible: !!page.st && page.st.fireholdUnlocked
        icon: "desktops"; title: "Firehold (preview)"
        sub: !page.st ? "" : !page.st.fireholdHere && page.st.fireholdMode === "off"
            ? "Firehold, Hearth 2.0's own desktop, still being built. It isn't on this device yet."
            : page.st.fireholdPaused ? "It couldn't start twice in a row, so Always is off for now and the normal desktop (labwc) starts. Pick Always to turn it back on."
            : "Hearth 2.0's own desktop, a preview: some things don't work yet. Try once is the next Desktop Mode start only; Always is every start. If it can't start, the normal desktop (labwc) starts and Hearth tells you why. " + (page.st.leaveKey || "Ctrl+Alt+Backspace") + " always goes back to the normal desktop."
        options: [{ value: "off", label: "Off" }, { value: "once", label: "Try once" }, { value: "always", label: "Always" }]
        current: !page.st ? "off" : page.st.fireholdPaused ? "off" : page.st.fireholdMode
        boxWidth: 200
        onChosen: v => {
            if (!page.st) return;
            if (v !== "off" && !page.st.fireholdHere) { page.say("Firehold isn't on this device yet", true); return; }
            if (v === "once") page.run(["wm", "firehold-once"], "Firehold (preview) for the next Desktop Mode start only, then the normal desktop again ✓");
            else if (v === "always") page.run(["wm", "firehold"], "Firehold (preview) every time Desktop Mode starts ✓");
            else page.run(["wm", "labwc"], "Firehold is off: Mantel uses labwc from the next time Desktop Mode starts ✓");
        }
    }
    SetRow {
        visible: !!page.st && page.st.fireholdUnlocked   // @noindex
        icon: "close"; title: "Hide Firehold again"
        sub: "Turns Firehold off (the normal desktop from the next start) and hides it here"
        chevron: true
        onActivated: page.run(["relock"], "Firehold is off and hidden again ✓")
    }
    SetRow {
        title: "Running now"
        sub: !page.st ? "" : page.nowName === "" ? "Desktop Mode isn't open"
            : page.st.wm === "firehold" && page.st.wmNow !== "firehold" ? (page.st.safeGraphics ? "Safe graphics this time, so labwc" : !page.st.fireholdHere ? "Firehold isn't installed, so " + page.nowName : "Firehold didn't start this time, so " + page.nowName)
            : "Mantel on " + page.nowName
        chip: page.nowName; chipKind: page.st && page.st.wm === "firehold" && page.st.wmNow !== "firehold" ? "warn" : "ok"
        interactive: false
    }
    SetRow {
        title: "Safe start"
        sub: "If the Firehold preview doesn't start, Mantel opens on labwc for that sign-in and tells you why. Twice in a row, it stays on labwc until you pick it again. Your choice stays saved."
        chip: "always on"; chipKind: "ok"
        interactive: false
    }
    SetRow {
        title: "KDE Plasma and GNOME"
        sub: "Retired in Hearth 1.14: Mantel is the desktop. A KDE Plasma or GNOME pick from before opens Mantel, and Hearth tells you once."
        chip: "retired"; chipKind: "warn"
        interactive: false
    }
    Text { visible: page.msg !== ""; Layout.fillWidth: true; text: page.msg; color: page.bad ? SetStyle.warn : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
}
