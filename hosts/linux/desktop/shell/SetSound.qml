// Hearth Settings › Sound (PipeWire): volume, which speakers or headphones, which microphone.
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

    SetSlider {
        icon: page.sink && page.sink.audio && page.sink.audio.muted ? "vol-mute" : "vol"
        title: "Volume"
        value: page.sink && page.sink.audio ? page.sink.audio.volume : 0
        onMoved: v => { if (page.sink && page.sink.audio) { page.sink.audio.muted = false; page.sink.audio.volume = v; } }
    }
    SetSwitch {
        icon: "vol-mute"; title: "Mute"; checked: !!(page.sink && page.sink.audio && page.sink.audio.muted)
        onToggled: v => { if (page.sink && page.sink.audio) page.sink.audio.muted = v; }
    }
    SetHeader { text: "Play sound through" }
    Repeater {
        model: page.outputs
        delegate: SetRow {
            required property var modelData
            icon: "vol"; title: page.label(modelData); chevron: false
            value: page.sink === modelData ? "In use" : ""
            onActivated: Pipewire.preferredDefaultAudioSink = modelData
        }
    }
    SetHeader { text: "Microphone" }
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
