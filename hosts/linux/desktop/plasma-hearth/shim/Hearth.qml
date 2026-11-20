pragma Singleton

// Hearth on Plasma's bridge (plasma-hearth): what Quickshell gave Mantel's files, done with Plasma's own pieces.
//   call(argv, cb(stdout, exitCode))   a program, through Plasma's executable source (no shell words: each argument
//                                      is quoted); never a loop, only when something asks
//   run(argv)                          the same, detached (hearth-plasma-panel run: it starts it and returns)
//   envs                               the session's environment (HOME, USER, XDG_*...), read once from the helper
//   stamp                              bumped when a Hearth file changed (hearth-plasma-panel sync, from a systemd
//                                      .path unit): FileViews with watchChanges read their file again then
import QtQuick
import org.kde.plasma.plasma5support as P5Support

Item {
    id: h
    readonly property string helper: "/usr/share/nimbus/desktop/plasma-hearth/hearth-plasma-panel"
    property var envs: ({})
    property bool envReady: false
    property string stamp: ""
    property var screen: ({ width: 1920, height: 1080, name: "" })

    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    P5Support.DataSource {
        id: sh
        engine: "executable"
        property var cbs: ({})
        property int n: 0
        onNewData: function (source, data) {
            const cb = sh.cbs[source];
            delete sh.cbs[source];
            sh.disconnectSource(source);
            if (cb) { try { cb(String(data["stdout"] || ""), Number(data["exit code"] || 0)); } catch (e) { console.warn("plasma-hearth: " + e); } }
        }
    }
    function call(argv, cb) {
        if (!argv || !argv.length) { if (cb) cb("", 1); return; }
        sh.n += 1;
        const cmd = argv.map(q).join(" ") + " #" + sh.n;
        sh.cbs[cmd] = cb || null;
        sh.connectSource(cmd);
    }
    function run(argv) { if (argv && argv.length) call(["python3", helper, "run"].concat(argv.map(String)), null); }
    // Mantel's Qt.resolvedUrl("...") for its helpers and pictures: they stay in Mantel's folder (one copy).
    function shellUrl(p) { return "file:///usr/share/nimbus/desktop/shell/" + p; }
    function env(k) { return envs[k] !== undefined ? String(envs[k]) : ""; }

    Component.onCompleted: call(["python3", helper, "env"], (out) => {
        try { const o = JSON.parse(out.trim().split("\n").pop() || "{}"); if (o && typeof o === "object") h.envs = o; } catch (e) {}
        h.envReady = true;
    })
}
