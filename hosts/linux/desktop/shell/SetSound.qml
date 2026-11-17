// Hearth Settings › Sound (PipeWire): volume, which speakers or headphones, which microphone; 1.14: each app's volume
// right here, each sound card's profile (hearth-sound, pactl: PipeWire remembers the choice itself) and a microphone
// test (a few seconds recorded and played back, kept in memory only).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Pipewire

SetPage {
    id: page
    title: "Sound"
    readonly property var sink: Pipewire.defaultAudioSink
    readonly property var source: Pipewire.defaultAudioSource
    readonly property var nodes: Pipewire.nodes ? Pipewire.nodes.values : []
    readonly property var outputs: nodes.filter(n => n.audio && n.isSink && !n.isStream)
    readonly property var inputs: nodes.filter(n => n.audio && !n.isSink && !n.isStream)
    PwObjectTracker { objects: [page.sink, page.source].concat(page.outputs).concat(page.inputs).filter(Boolean) }
    function label(n) { return n ? (n.description || n.nickname || n.name || "Device") : "None"; }
    property var cards: null           // hearth-sound cards
    property var test: null            // the last microphone test
    property string message: ""
    property bool messageBad: false
    property bool askOveramp: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-sound" }
    Component.onCompleted: job.run(["cards"], r => page.cards = r.cards || [], () => page.cards = [])
    function kindIcon(k) { return k === "bluetooth" ? "headphones" : k === "hdmi" ? "tv" : "vol"; }

    SetCardGroup {
        title: "Volume"
        SetSlider {
            icon: page.sink && page.sink.audio && page.sink.audio.muted ? "vol-mute" : "vol"
            title: "Volume"
            value: page.sink && page.sink.audio ? page.sink.audio.volume / Buttons.maxVolume : 0
            valueText: Math.round(value * Buttons.maxVolume * 100) + "%"
            onMoved: v => { if (page.sink && page.sink.audio) { page.sink.audio.muted = false; page.sink.audio.volume = v * Buttons.maxVolume; } }
        }
        SetSwitch {
            icon: "vol-mute"; title: "Mute"; checked: !!(page.sink && page.sink.audio && page.sink.audio.muted)
            onToggled: v => { if (page.sink && page.sink.audio) page.sink.audio.muted = v; }
        }
        // 1.14: over-amplification asks first (KDE's warning): turning it on shows what it can do, and Turn on.
        SetSwitch {
            icon: "vol"; title: "Over-amplification"
            sub: "Lets the volume go up to 150%. Sound can distort, and loud sound can damage speakers or your hearing."
            checked: Buttons.overamp || page.askOveramp
            onToggled: v => { if (v) page.askOveramp = true; else { page.askOveramp = false; Buttons.set("overamp", false); } }
        }
        SetBanner {
            visible: page.askOveramp && !Buttons.overamp
            Layout.fillWidth: true
            kind: "warn"
            title: "Louder than 100% can hurt"
            sub: "Above 100% sound is made louder by the PC, not the speakers: it can crackle, damage small speakers, and loud sound in headphones can harm your hearing. Turn the volume down before going over 100%."
        }
        RowLayout {
            visible: page.askOveramp && !Buttons.overamp
            Layout.fillWidth: true; Layout.leftMargin: 4
            spacing: 8
            SetButton { text: "Turn on"; primary: true; onActivated: { Buttons.set("overamp", true); page.askOveramp = false; } }
            SetButton { text: "Cancel"; ghost: true; onActivated: page.askOveramp = false }
        }
        Text {
            visible: Buttons.overamp && !!page.sink && !!page.sink.audio && page.sink.audio.volume > 1.001
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "The volume is over 100%: sound may distort, and loud sound in headphones can harm your hearing."
            color: SetStyle.warn; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        // Hearth 1.10: each app's own level (Mixer.qml), also from the sound icon's right-click menu.
        SetRow {
            visible: SettingsNav.mixer !== null
            icon: "vol"; title: "Volume mixer"; sub: "Each app's own volume, and which speakers or headphones it plays on"
            onActivated: if (SettingsNav.mixer) SettingsNav.mixer.open()
        }
    }
    // 1.14: more of KDE's sound settings on their own pages (settings-pages.js: sound-speakers, sound-apps, push-to-talk).
    SetCardGroup {
        title: "More sound settings"
        SetRow { icon: "vol"; title: "Speakers & balance"; sub: "Left and right balance, and a test tone from each speaker"; chevron: true; onActivated: SettingsNav.openPage("sound-speakers") }
        SetRow { icon: "headphones"; title: "Apps' sound"; sub: "Which speakers or headphones each app plays on, and scrolling on the dock"; chevron: true; onActivated: SettingsNav.openPage("sound-apps") }
        SetRow { icon: "mic"; title: "Push to talk"; sub: Settings.pttOn ? "On: hold your key to talk" : "Keep the microphone muted unless you hold a key"; chevron: true; onActivated: SettingsNav.openPage("push-to-talk") }
    }
    // Each app's own level (1.14): the mixer's apps (Mixer.qml), so a level set here is remembered the same way.
    readonly property var mixer: SettingsNav.mixer
    SetCardGroup {
        title: "Apps playing sound"
        visible: !!page.mixer && page.mixer.appKeys.length > 0
        Repeater {
            model: page.mixer ? page.mixer.appKeys : []
            delegate: SetSlider {
                required property string modelData
                icon: page.mixer.mutedOf(modelData) ? "vol-mute" : "vol"
                title: modelData
                value: page.mixer.volOf(modelData)
                onMoved: v => page.mixer.setVol(modelData, v, true)
            }
        }
    }
    SetCardGroup {
        title: "Play sound through"
        Repeater {
            model: page.outputs
            delegate: SetRow {
                required property var modelData
                icon: "vol"; title: page.label(modelData); chevron: false
                value: page.sink === modelData ? "In use" : ""
                onActivated: Pipewire.preferredDefaultAudioSink = modelData
            }
        }
    }
    SetCardGroup {
        title: "Microphone"
        SetSlider {
            visible: !!page.source
            icon: "mic"; title: "Input volume"
            value: page.source && page.source.audio ? page.source.audio.volume : 0
            onMoved: v => { if (page.source && page.source.audio) page.source.audio.volume = v; }
        }
        Repeater {
            model: page.inputs
            delegate: SetRow {
                required property var modelData
                icon: "mic"; title: page.label(modelData); chevron: false
                value: page.source === modelData ? "In use" : ""
                onActivated: Pipewire.preferredDefaultAudioSource = modelData
            }
        }
    }
    SetCardGroup {
        title: "Test your microphone"
        visible: !!page.source
        SetRow {
            icon: "mic"; chevron: false
            title: job.what === "test" ? "Recording… say something" : page.test ? "Record again" : "Record a few seconds"
            sub: !page.test ? "Then play it back to hear how you sound to others"
                 : page.test.level === "silent" ? "Nothing was heard. Check the microphone is picked above and not muted"
                 : page.test.level === "quiet" ? "Very quiet (" + Math.round(page.test.peak * 100) + "% at the loudest). Turn the input volume up"
                 : page.test.level === "loud" ? "Too loud: it clipped. Turn the input volume down"
                 : "Sounds good (" + Math.round(page.test.peak * 100) + "% at the loudest)"
            onActivated: { page.say(""); job.run(["mic-test", "4"], r => page.test = r, e => page.say(e, true), "", "test"); }
        }
        // How loud the test was, as a bar.
        Rectangle {
            visible: !!page.test
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.rightMargin: 4
            implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle {
                width: parent.width * Math.min(1, page.test ? page.test.peak : 0); height: parent.height; radius: 4
                color: page.test && (page.test.level === "loud" || page.test.level === "silent") ? Theme.danger : Theme.accent
            }
        }
        SetRow {
            visible: !!page.test
            icon: "play"; chevron: false; title: "Play it back"
            onActivated: job.run(["mic-play"], () => {}, e => page.say(e, true))
        }
    }
    SetCardGroup {
        title: "Sound cards"
        visible: !!page.cards && page.cards.length > 0
        Repeater {
            model: page.cards || []
            delegate: SetDropdown {
                required property var modelData
                icon: page.kindIcon(modelData.kind)
                title: modelData.description
                sub: "Which outputs and inputs it offers: stereo, surround, HDMI, a headset's microphone"
                options: modelData.profiles.map(p => ({ value: p.name, label: p.description + (p.available ? "" : " (not plugged in)") }))
                current: modelData.active
                boxWidth: 320
                onChosen: v => job.run(["profile", modelData.name, v], () => { page.say("Saved."); job.run(["cards"], r => page.cards = r.cards || []); }, e => page.say(e, true), "", "profile")
            }
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
