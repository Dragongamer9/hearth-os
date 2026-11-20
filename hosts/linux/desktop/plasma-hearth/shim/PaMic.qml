// The microphone for NotifyHub.qml (plasma-hearth): plasma-pa's default input (mic.muted can be set, as Mantel's
// Pipewire source) and the apps recording from a microphone (plasma-pa's recording streams, as Mantel's NotifyHub
// lists them: not Plasma's own level meters, not monitors). Told by the sound service when it changes; nothing polls.
import QtQuick
import org.kde.plasma.private.volume

Item {
    id: pm
    readonly property var mic: PreferredDevice.source
    property var apps: []
    SourceOutputModel { id: outs }
    function prop(o, k) { try { return o && o[k] !== undefined ? String(o[k]) : ""; } catch (e) { return ""; } }
    function rebuild() {
        const seen = [];
        for (let i = 0; i < ins.count; i++) {
            const d = ins.objectAt(i);
            if (!d || d.virtual) continue;
            const a = d.app;
            if (/plasmashell|quickshell|pavucontrol|wireplumber|pipewire|kmix/i.test(a + " " + d.bin) || d.monitor) continue;
            if (a && seen.indexOf(a) < 0) seen.push(a);
        }
        if (JSON.stringify(seen) !== JSON.stringify(apps)) apps = seen;
    }
    Instantiator {
        id: ins
        model: outs
        delegate: QtObject {
            required property var model
            readonly property var props: model.Properties || ({})
            readonly property bool virtual: model.VirtualStream === true
            readonly property bool monitor: pm.prop(props, "stream.monitor") === "true"
            readonly property string bin: pm.prop(props, "application.process.binary")
            readonly property string app: pm.prop(props, "application.name") || (model.Client ? String(model.Client.name || "") : "") || bin || String(model.Name || "")
            onAppChanged: Qt.callLater(pm.rebuild)
        }
        onObjectAdded: Qt.callLater(pm.rebuild)
        onObjectRemoved: Qt.callLater(pm.rebuild)
    }
}
