// Receiving files over Bluetooth (1.14, KDE's Bluetooth file receiving, in Hearth's way). Only while
// Settings.btReceive is on (off by default), hearth-bt-receive's OBEX agent runs; each file a phone or computer sends
// is asked about with a notice (Accept / Decline; no answer in 60 s is a no), or taken without asking from a trusted
// device when Settings.btReceiveTrusted is on. Files go to Settings.btReceiveFolder ("" = Downloads). What came lately
// is kept in ~/.local/state/nimbus/bt-received.json (state, not a setting). Loaded by shell.qml by file name: a problem
// here only loses receiving; if the agent stops, it's started again after 30 s.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: br
    property var settings: null
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-bt-receive").toString().replace(/^file:\/\//, ""))
    readonly property string statePath: Quickshell.env("HOME") + "/.local/state/nimbus/bt-received.json"
    readonly property string folder: Settings.btReceiveFolder || (Quickshell.env("HOME") + "/Downloads")
    readonly property bool on: Settings.loaded && Settings.btReceive
    property var items: []
    property var asks: []
    property bool stopping: false
    SettingsWriter { id: writer; path: br.statePath }
    FileView {
        path: br.statePath
        onLoaded: { try { const o = JSON.parse(text()); br.items = o && Array.isArray(o.items) ? o.items : []; } catch (e) { br.items = []; } }
    }
    onOnChanged: { if (on) agent.running = true; else { br.stopping = true; agent.running = false; } }
    onFolderChanged: if (agent.running) { agent.running = false; again.start(); }
    Component.onCompleted: if (on) agent.running = true
    Timer { id: again; interval: 30000; onTriggered: if (br.on && !agent.running) agent.running = true }
    Process {
        id: agent
        command: ["python3", br.helper, "agent", br.folder]
        stdinEnabled: true
        stdout: SplitParser {
            onRead: l => {
                let o = null;
                try { o = JSON.parse(l); } catch (e) { return; }
                if (o.ask) br.asked(o);
                else if (o.received) br.got(o);
                else if (o.failed) br.tell("Couldn't receive " + o.failed, "From " + (o.from || "a Bluetooth device") + ". It stopped before the end", "");
            }
        }
        onExited: code => { if (br.stopping) { br.stopping = false; return; } if (br.on) again.start(); }
    }
    function answer(id, yes) { if (agent.running) agent.write((yes ? "yes " : "no ") + id + "\n"); }
    function size(n) { return n >= 1048576 ? (n / 1048576).toFixed(1) + " MB" : Math.max(1, Math.round(n / 1024)) + " KB"; }
    function asked(o) {
        if (o.trusted && Settings.btReceiveTrusted) { answer(o.ask, true); return; }
        asks = asks.concat([o]);
        if (!ask.running) nextAsk();
    }
    function nextAsk() {
        if (!asks.length) return;
        const o = asks[0];
        ask.id = o.ask;
        ask.command = ["notify-send", "-a", "Bluetooth", "-i", "bluetooth", "-u", "critical", "-A", "accept=Accept", "-A", "decline=Decline", "-w",
                       (o.from || "A Bluetooth device") + " wants to send you a file", o.name + (o.size ? " · " + size(o.size) : "")];
        ask.answered = false;
        ask.running = true;
    }
    Process {
        id: ask
        property string id: ""
        property bool answered: false
        stdout: SplitParser { onRead: l => { const a = l.trim(); if (a === "accept" || a === "decline") { ask.answered = true; br.answer(ask.id, a === "accept"); } } }
        onExited: { if (!ask.answered) br.answer(ask.id, false); br.asks = br.asks.slice(1); br.nextAsk(); }
    }
    function got(o) {
        const it = { path: o.received, name: o.name, from: o.from || "", at: Date.now() };
        items = [it].concat(items).slice(0, 30);
        writer.merge({ items: items });
        tell("Received " + o.name, "From " + (it.from || "a Bluetooth device") + " · in " + o.received.replace(/\/[^\/]*$/, "").replace(Quickshell.env("HOME"), "~"), o.received);
    }
    function tell(title, body, path) {
        done.path = path;
        done.command = ["notify-send", "-a", "Bluetooth", "-i", "bluetooth"].concat(path ? ["-A", "open=Open", "-A", "folder=Show in folder", "-w"] : []).concat([title, body]);
        if (!done.running) done.running = true;
    }
    Process {
        id: done
        property string path: ""
        stdout: SplitParser {
            onRead: l => {
                const a = l.trim();
                if (a === "open" && done.path) Quickshell.execDetached(["xdg-open", done.path]);
                if (a === "folder" && done.path) Quickshell.execDetached(["xdg-open", done.path.replace(/\/[^\/]*$/, "")]);
            }
        }
    }
    IpcHandler {
        target: "btreceive"
        function test(): void { br.asks = br.asks.concat([{ ask: "0", name: "Holiday photo.jpg", size: 2483200, from: "Pixel 8" }]); if (!ask.running) br.nextAsk(); }
    }
}
