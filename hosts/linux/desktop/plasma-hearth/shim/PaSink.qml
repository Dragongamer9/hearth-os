// plasma-pa's default output, for Pipewire.qml (plasma-hearth).
import QtQuick
import org.kde.plasma.private.volume

Item {
    readonly property var sink: PreferredDevice.sink
    readonly property real normal: PulseAudio.NormalVolume
    readonly property var sinks: []
    function makeDefault(n) {}
}
