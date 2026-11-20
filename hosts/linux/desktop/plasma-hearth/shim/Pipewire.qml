pragma Singleton

// Sound for Mantel's bar (plasma-hearth): Plasma's own volume service (plasma-pa), loaded on its own (PaSink.qml) so
// a Plasma without it only loses the volume button. defaultAudioSink.audio.volume (0..1.5) and .muted can be set.
import QtQuick

Item {
    id: pw
    Loader { id: pa; source: "PaSink.qml" }
    readonly property var sink: pa.status === Loader.Ready && pa.item ? pa.item.sink : null
    readonly property var nodes: ({ values: pa.status === Loader.Ready && pa.item ? pa.item.sinks : [] })
    property var preferredDefaultAudioSink: null
    onPreferredDefaultAudioSinkChanged: if (preferredDefaultAudioSink && pa.item) pa.item.makeDefault(preferredDefaultAudioSink)
    readonly property var defaultAudioSink: sink ? ({
        description: sink.description || "", nickname: "", name: sink.name || "", audio: audioObj, isSink: true, isStream: false
    }) : null
    QtObject {
        id: audioObj
        readonly property real normal: pa.item ? pa.item.normal : 65536
        property real volume: pw.sink ? pw.sink.volume / normal : 0
        property bool muted: pw.sink ? pw.sink.muted : false
        onVolumeChanged: if (pw.sink && Math.abs(pw.sink.volume / normal - volume) > 0.004) pw.sink.volume = Math.round(volume * normal)
        onMutedChanged: if (pw.sink && pw.sink.muted !== muted) pw.sink.muted = muted
    }
}
