// "Game Mode" (docs/HEARTH-GAME-MODE.md, Hearth 1.10): which Game Mode this PC goes into: Steam Game Mode (Valve's Steam in
// gamescope, the default) or Hearth Game Mode (Hearth's own screens). Two picture cards in Settings › Gaming, moved
// through with the controller (Up/Down, Left/Right between the cards, A picks). Picking one saves only that
// (desktop/shell/hearth-game-mode set: session.conf's HEARTH_GAME_MODE_CHOICE line, atomic, .bak, under the lock) and it
// applies the next time Game Mode starts; a row then offers to switch now (nimbus-mode game). Opening the page only reads.
// When Hearth Game Mode couldn't start this sign-in (the session opened Steam Game Mode or Mantel instead) or is paused
// after failing twice, it says why here, with "Try Hearth Game Mode again" (nimbus-mode hearth). Hearth never changes
// the choice by itself.
import QtQuick
import QtQuick.Layouts
import Quickshell

ColumnLayout {
    id: gm
    Layout.fillWidth: true
    spacing: 10
    property var st: null              // hearth-game-mode get
    property string picked: ""         // just picked here: offer to switch now
    property string msg: ""
    readonly property string choice: st ? st.choice : "steam"
    signal changed(string choice)
    SetJob { id: job; helper: "hearth-game-mode" }
    function load() { job.run(["get"], r => { gm.st = r; }, e => gm.msg = e); }
    Component.onCompleted: load()
    function pick(v) {
        if (v !== "steam" && v !== "hearth") return;
        if (st && st.choice === v && st.saved) { picked = ""; msg = ""; return; }
        job.run(["set", v], r => { gm.st = r; gm.picked = v; gm.msg = ""; gm.changed(v); }, e => gm.msg = e);
    }
    readonly property var cards: [
        { v: "steam", icon: "steam", t: "Steam Game Mode", d: "Steam's own big-screen mode, like a Steam Deck, with Hearth Sync for your other stores' games." },
        { v: "hearth", icon: "pad", t: "Hearth Game Mode", d: "Hearth's own screens: every store in one library, Hearth's Store, Quick Access and Settings. Steam games still play through Steam." }
    ]

    RowLayout {
        Layout.fillWidth: true
        spacing: 12
        Repeater {
            id: rep
            model: gm.cards
            delegate: SetBase {
                id: card
                required property var modelData
                required property int index
                readonly property bool chosen: gm.choice === modelData.v
                readonly property bool usable: !gm.st || (modelData.v === "steam" ? gm.st.steam.ok : gm.st.hearth.ok)
                objectName: "gm-card-" + modelData.v
                Layout.fillWidth: true; Layout.preferredWidth: 1
                implicitHeight: 214
                usesLeftRight: true
                border.width: chosen ? 2 : 1
                border.color: chosen ? Theme.accent : SetStyle.tint(Theme.text, 0.05)
                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData.t
                Accessible.checked: chosen
                onActivated: gm.pick(modelData.v)
                onAdjusted: step => {
                    const other = rep.itemAt(index + step);
                    if (other) other.forceActiveFocus();
                    else if (step < 0) SettingsNav.toNav();
                }
                // A small picture of each Game Mode.
                Rectangle {
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                    height: 112; radius: 8; clip: true
                    gradient: Gradient {
                        GradientStop { position: 0; color: card.modelData.v === "steam" ? "#1b2b3f" : "#3a2216" }
                        GradientStop { position: 1; color: card.modelData.v === "steam" ? "#0d141d" : "#140c09" }
                    }
                    // Steam: the library's row of covers under a top bar.
                    Item {
                        visible: card.modelData.v === "steam"
                        anchors.fill: parent
                        Rectangle { x: 10; y: 10; width: parent.width - 20; height: 10; radius: 3; color: "#23364c" }
                        Rectangle { x: 10; y: 10; width: 46; height: 10; radius: 3; color: "#1a9fff" }
                        Row {
                            x: 10; y: 30; spacing: 6
                            Repeater { model: ["#3b6ea5", "#6b4fa0", "#2f8f72", "#a5793b", "#a54a3b", "#3b8aa5"]
                                delegate: Rectangle { required property var modelData; width: 34; height: 50; radius: 4; color: modelData } }
                        }
                        Rectangle { x: 10; y: 88; width: 64; height: 14; radius: 7; color: "#1a9fff" }
                    }
                    // Hearth: a big "jump back in" tile, a row of games and the warm accent.
                    Item {
                        visible: card.modelData.v === "hearth"
                        anchors.fill: parent
                        Rectangle { x: 10; y: 10; width: parent.width * 0.55; height: 52; radius: 6
                            gradient: Gradient { orientation: Gradient.Horizontal
                                GradientStop { position: 0; color: "#c8642c" } GradientStop { position: 1; color: "#7a2e1c" } } }
                        Rectangle { x: 18; y: 44; width: 40; height: 10; radius: 5; color: "#ffd9a8" }
                        Column { x: parent.width * 0.55 + 18; y: 12; spacing: 6
                            Repeater { model: 3; delegate: Rectangle { width: 70; height: 12; radius: 4; color: "#4a2c1e" } } }
                        Row {
                            x: 10; y: 70; spacing: 6
                            Repeater { model: ["#e08a3c", "#9b5a3a", "#5e7f9e", "#7d9e5e", "#b0624f", "#6a5e9e"]
                                delegate: Rectangle { required property var modelData; width: 30; height: 32; radius: 4; color: modelData } }
                        }
                    }
                }
                Row {
                    anchors { left: parent.left; top: parent.top; leftMargin: 14; topMargin: 132 }
                    spacing: 8
                    Icon { name: card.modelData.icon; size: 20; color: Theme.text }
                    Text { text: card.modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                    Text { visible: card.chosen; text: card.modelData.v === "steam" && gm.st && !gm.st.saved ? "· In use (default)" : "· In use"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14 }
                }
                Text {
                    anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 14; rightMargin: 14; topMargin: 160 }
                    text: card.usable ? card.modelData.d : "Can't start here: " + (card.modelData.v === "steam" ? gm.st.steam.why : gm.st.hearth.why) + "."
                    color: card.usable ? Theme.muted : Theme.danger
                    font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                    maximumLineCount: 3; elide: Text.ElideRight
                }
            }
        }
    }

    // Just picked: it applies the next time Game Mode starts; switch now if you like.
    SetRow {
        objectName: "gm-switch-now"
        visible: gm.picked !== "" && !!gm.st && gm.st.now !== "game" && gm.st.now !== "steam"
        icon: "pad"
        title: "Switch to " + (gm.picked === "hearth" ? "Hearth Game Mode" : "Steam Game Mode") + " now"
        sub: "Saved. Game Mode opens this way from now on, the next time it starts. Switching now closes Desktop Mode's windows."
        chevron: true
        onActivated: Quickshell.execDetached(["nimbus-mode", gm.picked === "hearth" ? "hearth" : "game"])
    }
    // "Play Steam games in Steam Game Mode" (docs/HEARTH-GAME-MODE.md, "Steam games"): with Hearth Game Mode picked, Play on
    // a Steam game opens it in Steam Game Mode and comes back when it ends. On unless turned off; saved only when flipped.
    SetSwitch {
        objectName: "gm-steam-games"
        visible: !!gm.st && gm.st.choice === "hearth"
        icon: "steam"; title: "Play Steam games in Steam Game Mode"
        sub: "In Hearth Game Mode, Play on a Steam game opens it in Steam Game Mode, then comes back when the game ends"
        checked: !gm.st || gm.st.steamGames !== false
        onToggled: v => job.run(["steam-games", v ? "on" : "off"], r => { gm.st = r; }, e => gm.msg = e)
    }
    // Hearth Game Mode didn't start this sign-in, or is paused after failing twice: why, and a way to try again.
    SetRow {
        objectName: "gm-fallback"
        visible: !!gm.st && gm.st.choice === "hearth" && (gm.st.fallback !== "" || gm.st.paused !== "")
        icon: "warn"; danger: true
        title: gm.st && gm.st.paused !== "" ? "Hearth Game Mode is paused" : "Hearth Game Mode didn't start last time"
        sub: !gm.st ? "" : (gm.st.paused !== "" ? "It failed twice (" + gm.st.paused + "), so Game Mode opens Steam Game Mode until you try again. "
                                                : "Why: " + gm.st.fallback + ". Steam Game Mode (or Desktop Mode) opened instead. ")
             + "Your choice is still Hearth Game Mode. Press to try Hearth Game Mode again."
        onActivated: Quickshell.execDetached(["nimbus-mode", "hearth"])
    }
    Text {
        visible: gm.msg !== ""
        Layout.fillWidth: true
        text: gm.msg; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
