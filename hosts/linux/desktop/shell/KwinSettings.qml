// What a Hearth Settings page for KWin (Set*Kwin pages, docs/KWIN-SETTINGS.md) reads and changes: KWin's own settings
// through hearth-kwin-settings, which writes only the keys changed here (kwinrc, kdeglobals, kaccessrc...) and asks
// KWin to read them again. Nothing is written when a page opens.
//   KwinSettings { id: kw }   kw.b("kwinrc", "Plugins", "blurEnabled", true)   kw.set("kwinrc", "Plugins", "blurEnabled", false)
// `live`: Mantel runs on KWin (or the test preview); under labwc the pages show their controls greyed with KwinGate.
import QtQuick

Item {
    id: kw
    visible: false
    property var st: null              // hearth-kwin-settings status
    property var files: ({})           // {file: {group: {key: "value"}}}: what KWin sees (system defaults + yours)
    property var mine: ({})            // {file: {group: [keys you set]}}
    property var bad: ({})             // {file: "why it couldn't be read"} (it's never written then)
    readonly property bool ready: st !== null
    readonly property bool live: !!st && !!st.live
    readonly property bool running: !!st && !!st.running
    readonly property bool preview: !!st && !!st.preview
    property string message: ""
    property bool messageBad: false
    property alias job: job
    signal loaded()
    signal saved(string file)
    function say(t, isBad) { message = t; messageBad = !!isBad; }

    SetJob { id: job; helper: "hearth-kwin-settings" }
    Component.onCompleted: refresh()
    function refresh() {
        job.run(["status"], r => {
            kw.st = r;
            if (r.live) job.run(["get"], g => { kw.take(g); kw.loaded(); }, e => kw.say(e, true));
        }, e => { kw.st = { live: false, error: e }; kw.say(e, true); });
    }
    function take(g) { files = g.files || {}; mine = g.mine || {}; bad = g.bad || {}; }

    // ---- reading (values are KDE's text: "true", "4", "0.5")
    function v(file, group, key, dflt) {
        const f = files[file] || {}, g = f[group] || {};
        return g[key] !== undefined ? g[key] : dflt;
    }
    function b(file, group, key, dflt) {
        const x = v(file, group, key, undefined);
        if (x === undefined || x === null || x === "") return !!dflt;
        return String(x).toLowerCase() === "true" || x === "1";
    }
    function n(file, group, key, dflt) {
        const x = parseFloat(v(file, group, key, undefined));
        return isNaN(x) ? dflt : x;
    }
    function list(file, group, key, dflt) {   // IntList ("7,1")
        const x = v(file, group, key, undefined);
        if (x === undefined) return dflt || [];
        return String(x).split(",").map(s => parseInt(s)).filter(i => !isNaN(i));
    }
    function isMine(file, group, key) { return ((mine[file] || {})[group] || []).indexOf(key) >= 0; }
    // An effect is on: what you chose, else KWin's own default (status.defaults).
    function effectOn(id) {
        const d = st && st.defaults ? !!st.defaults[id] : false;
        return b("kwinrc", "Plugins", id + "Enabled", d);
    }
    // KWin has this effect (when KWin can be asked; otherwise every KWin 6 effect is offered).
    function hasEffect(id) { return !st || !st.effects || st.effects.indexOf(id) >= 0; }

    // ---- changing: shown at once, saved together half a second later (sliders move in small steps)
    property var pending: ({})          // {file: {group: {key: value or null}}}
    function set(file, group, key, value) { setMany(file, { [group]: { [key]: value } }); }
    function setMany(file, groups, now) {
        const f = Object.assign({}, files[file] || {});
        const p = Object.assign({}, pending);
        p[file] = Object.assign({}, p[file] || {});
        for (const g in groups) {
            f[g] = Object.assign({}, f[g] || {});
            p[file][g] = Object.assign({}, p[file][g] || {});
            for (const k in groups[g]) {
                const val = groups[g][k];
                if (val === null) delete f[g][k];
                else f[g][k] = typeof val === "boolean" ? (val ? "true" : "false") : Array.isArray(val) ? val.join(",") : String(val);
                p[file][g][k] = val;
            }
        }
        files = Object.assign({}, files, { [file]: f });
        pending = p;
        if (now) later.triggered(); else later.restart();
    }
    function setEffect(id, on) { set("kwinrc", "Plugins", id + "Enabled", on); }
    Timer {
        id: later
        interval: 500
        onTriggered: {
            const p = kw.pending;
            kw.pending = ({});
            for (const file in p) {
                job.run(["set", file], r => {
                    kw.take(r);
                    kw.say(r.reloaded || !kw.running ? "" : "Saved. KWin uses it from the next time you sign in.");
                    kw.saved(file);
                }, e => { kw.say(e, true); kw.refresh(); }, JSON.stringify(p[file]), "save");
            }
        }
    }
    function shortcut(name) { job.run(["shortcut", name], () => {}, e => kw.say(e, true)); }
}
