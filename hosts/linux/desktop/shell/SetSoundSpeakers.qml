// Hearth Settings › Sound › Speakers & balance (1.14, KDE's balance and speaker test, in Hearth's look): for the output
// you pick, its left/right balance and a picture of its speakers around you; press one to hear a short tone from it.
// Through hearth-sound (pactl, pw-play, argument lists); PipeWire remembers the balance itself, Hearth keeps no file.
// Opening the page reads only.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Speakers & balance"
    property var outs: null            // hearth-sound outputs
    property string pick: ""           // the output shown ("" = the one in use)
    property string playing: ""
    property string message: ""
    property bool messageBad: false
    readonly property var cur: outs ? (outs.find(o => o.name === pick) || outs.find(o => o.default) || outs[0] || null) : null
    readonly property bool stereo: !!cur && cur.channels.some(c => /left/.test(c.id)) && cur.channels.some(c => /right/.test(c.id))
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["outputs"], r => page.outs = r.outputs || [], e => { page.outs = []; page.say(e, true); }); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-sound" }
    SetJob { id: setter; helper: "hearth-sound" }
    // Where each speaker stands around you (x, y as parts of the picture).
    readonly property var spots: ({
        "mono": [0.5, 0.16], "front-left": [0.2, 0.16], "front-right": [0.8, 0.16], "front-center": [0.5, 0.1],
        "lfe": [0.66, 0.06], "side-left": [0.08, 0.52], "side-right": [0.92, 0.52], "rear-left": [0.22, 0.88],
        "rear-right": [0.78, 0.88], "rear-center": [0.5, 0.92], "front-left-of-center": [0.35, 0.12], "front-right-of-center": [0.65, 0.12]
    })

    SetCardGroup {
        title: "Output"
        SetDropdown {
            visible: !!page.outs && page.outs.length > 1
            icon: "vol"; title: "Speakers or headphones"; sub: "Which one these settings are for"
            options: (page.outs || []).map(o => ({ value: o.name, label: o.description + (o.default ? " (in use)" : "") }))
            current: page.cur ? page.cur.name : ""
            boxWidth: 340
            onChosen: v => page.pick = v
        }
        SetRow {
            visible: !!page.outs && page.outs.length === 0
            interactive: false; icon: "info"; title: "No speakers or headphones found"
            sub: "Plug some in, or check Bluetooth"
        }
    }
    SetCardGroup {
        title: "Balance"
        visible: page.stereo
        SetSlider {
            id: bal
            icon: "vol"; title: "Left and right"
            sub: "Moves the sound towards the left or the right speaker"
            value: page.cur ? (page.cur.balance + 100) / 200 : 0.5
            step: 0.025
            valueText: { const b = Math.round(value * 200 - 100); return b === 0 ? "Centre" : b < 0 ? (-b) + "% left" : b + "% right"; }
            onMoved: v => { value = v; balApply.restart(); }
            Timer { id: balApply; interval: 250; onTriggered: if (page.cur) setter.run(["balance", page.cur.name, String(Math.round(bal.value * 200 - 100))], () => {}, e => page.say(e, true)) }
        }
        SetRow {
            icon: "undo"; title: "Centre the balance"; chevron: false
            visible: !!page.cur && page.cur.balance !== 0
            onActivated: setter.run(["balance", page.cur.name, "0"], () => page.load(), e => page.say(e, true))
        }
    }
    SetCardGroup {
        title: "Test your speakers"
        visible: !!page.cur && page.cur.channels.length > 0
        // The room: you in the middle, each speaker where it stands. Press one to hear it.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 300
            radius: SetStyle.radius
            color: SetStyle.tint(Theme.text, 0.035)
            border.color: SetStyle.tint(Theme.text, 0.07)
            Rectangle {
                anchors.centerIn: parent
                width: 74; height: 74; radius: 37
                color: SetStyle.tint(Theme.accent, 0.14)
                border.color: SetStyle.tint(Theme.accent, 0.5)
                Icon { anchors.centerIn: parent; name: "user"; size: 30; color: Theme.accent }
            }
            Text { anchors.horizontalCenter: parent.horizontalCenter; y: parent.height / 2 + 44; text: "You"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            Repeater {
                model: page.cur ? page.cur.channels : []
                delegate: Rectangle {
                    id: spk
                    required property var modelData
                    readonly property var at: page.spots[modelData.id] || [0.5, 0.5]
                    readonly property bool on: page.playing === modelData.id
                    width: 132; height: 64; radius: 14
                    x: Math.max(8, Math.min(parent.width - width - 8, parent.width * at[0] - width / 2))
                    y: Math.max(8, Math.min(parent.height - height - 8, parent.height * at[1] - height / 2))
                    color: on ? Theme.accent : spkMouse.containsMouse || spk.activeFocus ? SetStyle.tint(Theme.text, 0.1) : Theme.panel2
                    border.color: spk.activeFocus ? Theme.accent : SetStyle.tint(Theme.text, 0.08)
                    border.width: spk.activeFocus ? 2 : 1
                    activeFocusOnTab: true
                    Keys.onReturnPressed: spk.play()
                    Keys.onSpacePressed: spk.play()
                    function play() {
                        page.playing = modelData.id; stopGlow.restart();
                        setter.run(["speaker-test", page.cur.name, modelData.id], () => {}, e => page.say(e, true));
                    }
                    Column {
                        anchors.centerIn: parent
                        spacing: 3
                        Icon { anchors.horizontalCenter: parent.horizontalCenter; name: spk.on ? "vol" : "play"; size: 18; color: spk.on ? Theme.accentInk : Theme.text }
                        Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.label; color: spk.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                    }
                    MouseArea { id: spkMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: spk.play() }
                }
            }
            Timer { id: stopGlow; interval: 1300; onTriggered: page.playing = "" }
        }
        SetRow {
            icon: "play"; chevron: false; title: "Play each speaker in turn"
            sub: "A short tone from every speaker, one after another"
            onActivated: { page.turn = 0; turnTimer.restart(); }
        }
    }
    property int turn: -1
    Timer {
        id: turnTimer
        interval: 1500
        onTriggered: {
            if (!page.cur || page.turn < 0 || page.turn >= page.cur.channels.length) { page.turn = -1; return; }
            const c = page.cur.channels[page.turn];
            page.playing = c.id; stopGlow.restart();
            setter.run(["speaker-test", page.cur.name, c.id], () => {}, e => page.say(e, true));
            page.turn++;
            turnTimer.restart();
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
