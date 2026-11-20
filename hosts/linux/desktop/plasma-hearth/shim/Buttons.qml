pragma Singleton
// The power and volume buttons' side of Mantel (plasma-hearth). On Plasma the keys are KDE's own (plasma-pa and
// PowerDevil change the volume and brightness); Hearth's bars show when they change (osd / brightnessOsd), as
// Mantel's VolumeOsd.qml does for the keys; KDE's own on-screen display is off while Hearth's panel is in use.
import QtQuick
import org.kde.plasma.plasma5support as P5Support

Item {
    id: b
    property bool overamp: false
    readonly property real maxVolume: overamp ? 1.5 : 1.0
    readonly property string helper: "/usr/share/nimbus/desktop/shell/hearth-buttons"
    signal powerMenuRequested()
    signal osd(real v, bool m)
    signal brightnessOsd(int pct)
    function sleep(who) { Hearth.run(["systemctl", "suspend"]); }
    // Changes made in Hearth's own Quick settings don't pop the bar (as on Mantel, where only the keys do).
    property bool quiet: false
    property bool ready: false
    Timer { interval: 4000; running: true; onTriggered: b.ready = true }
    readonly property var sinkAudio: Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null
    readonly property real vol: sinkAudio ? sinkAudio.volume : -1
    readonly property bool muted: sinkAudio ? sinkAudio.muted : false
    onVolChanged: if (ready && !quiet && vol >= 0) osd(vol, muted)
    onMutedChanged: if (ready && !quiet && vol >= 0) osd(vol, muted)
    P5Support.DataSource { id: pm; engine: "powermanagement"; connectedSources: ["PowerDevil"] }
    readonly property var pd: pm.data["PowerDevil"] || ({})
    readonly property int bright: pd["Screen Brightness"] !== undefined && pd["Maximum Screen Brightness"] > 0 ? Math.round(pd["Screen Brightness"] * 100 / pd["Maximum Screen Brightness"]) : -1
    onBrightChanged: if (ready && !quiet && bright >= 0) brightnessOsd(bright)
}
