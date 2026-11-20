// Quickshell's FileView (plasma-hearth): reads `path` (when it's set and the session's HOME is known) and says
// loaded / loadFailed; text() is what it read. With watchChanges, fileChanged comes when Hearth's files changed
// (Hearth.stamp: hearth-plasma-panel sync, from a systemd .path unit; nothing polls).
import QtQuick

Item {
    id: f
    property string path: ""
    property bool watchChanges: false
    property bool blockLoading: false
    property bool printErrors: true
    property bool preload: true
    signal loaded()
    signal loadFailed(int error)
    signal fileChanged()
    property string content: ""
    property int gen: 0
    function text() { return content; }
    function reload() {
        if (!Hearth.envReady || path === "" || path.startsWith("/.") || path.startsWith("/undefined")) return;
        const g = ++gen;
        Hearth.call(["cat", "--", path], (out, code) => {
            if (!f || g !== f.gen) return;
            if (code === 0) { f.content = out; f.loaded(); } else f.loadFailed(2);
        });
    }
    onPathChanged: reload()
    Connections { target: Hearth; function onEnvReadyChanged() { f.reload(); } function onStampChanged() { if (f.watchChanges) f.fileChanged(); } }
    Component.onCompleted: reload()
}
