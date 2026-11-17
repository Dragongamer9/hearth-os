// What the 1.14 search plugins (search/runners.js: bookmarks and history, desktops and activities, kill, world clock,
// spelling, media, App store, contacts, calendar, notes) need from the PC, and doing what their rows ask.
//   src            the plugins' data (Launcher.qml passes it as ctx.src): read when the menu opens (at most every
//                  minute or two), and looked up as you type for "kill …", "spell …" / "define …", "install …"
//   handle(answer) does a row's action if it's one of theirs (true), asking first when it has confirm: {title, message, ok}
// Everything is read by hearth-search-sources, which only reads (browsers' databases from a copy; nothing of yours is
// written) except `kill`, which only closes your own programs and never one Hearth needs. Any failure just leaves that
// plugin with nothing to show.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

Item {
    id: root
    property var launcher: null
    property var dialog: null                  // DeskDialog (shell.qml); without it nothing that asks first is done
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-search-sources").toString().replace(/^file:\/\//, ""))
    readonly property string storeHelper: decodeURIComponent(Qt.resolvedUrl("hearth-store").toString().replace(/^file:\/\//, ""))

    property var browser: null
    property var pim: null
    property var zones: null
    property var procs: null
    property var spell: ({})
    property var store: null
    property real browserAt: 0
    property real pimAt: 0
    property real zonesAt: 0

    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values.find(p => p.canControl) || null
    readonly property var media: player ? { player: player.identity || "Player", title: player.trackTitle || "", artist: player.trackArtist || "", playing: player.isPlaying === true } : null
    readonly property var desktops: ({
        desktops: (WinList.desktops || []).map((d, i) => ({ n: i + 1, name: d.name || "", active: d.active === true })),
        activities: (Activities.list || []).map(a => ({ id: a.id, name: a.name, current: a.id === Activities.current }))
    })
    readonly property var src: ({ browser: browser, pim: pim, zones: zones, procs: procs, spell: spell, store: store, media: media, desktops: desktops })

    function parse(text) { try { const t = String(text || "").trim().split("\n").pop(); return t && t[0] === "{" ? JSON.parse(t) : null; } catch (e) { return null; } }

    // The menu opened: what's read once in a while.
    function refresh() {
        const now = Date.now();
        if (now - browserAt > 120000 && !browserProc.running) { browserAt = now; browserProc.running = true; }
        if (now - pimAt > 60000 && !pimProc.running) { pimAt = now; pimProc.running = true; }
        if (now - zonesAt > 600000 && !zonesProc.running) { zonesAt = now; zonesProc.running = true; }
    }
    // As you type: only for the plugins that look something up.
    // (called while the menus work out their results, so what's typed is kept where no binding watches it)
    readonly property var typed: ({ q: "" })
    function lookup(raw) {
        const q = String(raw || "");
        if (!/^\s*(kill|spell|define|meaning of|dict|install|store|get app)\s+\S/i.test(q)) return;
        if (q === typed.q) return;
        typed.q = q;
        lookupTimer.restart();
    }
    Connections { target: root.launcher; function onShownChanged() { if (root.launcher.shown) root.refresh(); } }
    Timer {
        id: lookupTimer
        interval: 300
        onTriggered: {
            const q = root.typed.q.trim();
            let m;
            if ((m = /^kill\s+(.+)$/i.exec(q))) { procProc.running = false; procProc.command = ["python3", root.helper, "procs", m[1]]; procProc.running = true; }
            else if ((m = /^(?:spell|define|meaning of|dict)\s+(.+)$/i.exec(q))) {
                if (root.spell[m[1]] === undefined) { spellProc.running = false; spellProc.word = m[1]; spellProc.command = ["python3", root.helper, "spell", m[1]]; spellProc.running = true; }
            }
            else if ((m = /^(?:install|store|get app)\s+(.+)$/i.exec(q)) && m[1].length >= 2 && !(root.store && root.store.query.toLowerCase() === m[1].toLowerCase())) {
                storeProc.running = false; storeProc.words = m[1]; storeProc.command = ["python3", root.storeHelper, "search", m[1]]; storeProc.running = true;
            }
        }
    }

    Process { id: browserProc; command: ["python3", root.helper, "browser"]
        stdout: StdioCollector { onStreamFinished: { const r = root.parse(text); if (r) root.browser = r; } } }
    Process { id: pimProc; command: ["python3", root.helper, "pim"]
        stdout: StdioCollector { onStreamFinished: { const r = root.parse(text); if (r) root.pim = r; } } }
    Process { id: zonesProc; command: ["python3", root.helper, "zones"]
        stdout: StdioCollector { onStreamFinished: { const r = root.parse(text); if (r) root.zones = r; } } }
    Process { id: procProc
        stdout: StdioCollector { onStreamFinished: { const r = root.parse(text); if (r) root.procs = r; } } }
    Process { id: spellProc; property string word: ""
        stdout: StdioCollector { onStreamFinished: { const r = root.parse(text); if (r) { const s = Object.assign({}, root.spell); s[spellProc.word] = r; root.spell = s; } } } }
    Process { id: storeProc; property string words: ""
        stdout: StdioCollector { onStreamFinished: {
            const r = root.parse(text);
            root.store = { query: storeProc.words, apps: r && r.apps ? r.apps.map(a => ({ id: a.id, name: a.name, summary: a.summary })) : [],
                           error: r ? "" : (String(text || "").trim().split("\n").pop() || "").replace(/^error:\s*/, "") || "No answer" };
        } } }

    // A row's action, if it's one of these plugins' (Launcher.qml's runAnswer calls this first).
    function handle(a) {
        const x = a && a.action;
        if (!x) return false;
        const mine = ["desktop", "activity", "kill", "power", "media", "page", "exec"].indexOf(x.type) >= 0 || !!x.confirm;
        if (!mine) return false;
        const scr = launcher ? launcher.screen : null;
        if (launcher) launcher.shown = false;
        if (x.confirm) {
            if (dialog) dialog.confirm(scr, x.confirm.title, x.confirm.message || "", x.confirm.ok || "OK", () => root.act(x));
            else console.warn("SearchSources: no confirm dialog, so nothing was done:", x.type);
            return true;
        }
        act(x);
        return true;
    }
    function act(x) {
        if (x.type === "desktop") WinList.goDesktop(x.value);
        else if (x.type === "activity") Activities.switchTo(x.value);
        else if (x.type === "kill") Quickshell.execDetached(["python3", helper, "kill"].concat((x.value || []).map(String)));
        else if (x.type === "power") { if (launcher) launcher.power(x.value); }
        else if (x.type === "media") Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "media", x.value]);
        else if (x.type === "page") { if (launcher) launcher.pageRequested(x.value); }
        else if (x.type === "exec" && x.value && x.value.length) Quickshell.execDetached(x.value);
        else if (x.type === "copy") Quickshell.execDetached(["wl-copy", "--", String(x.value)]);
        else if (x.type === "url") Quickshell.execDetached(["xdg-open", x.value]);
    }
}
