// Hearth Settings › Display: resolution, refresh rate, scale (wlr-randr), and brightness. A new mode is
// kept only if you confirm it within 15 seconds, so a mode the screen can't show fixes itself. Kept modes
// go to ~/.config/nimbus/display.sh, which Desktop Mode runs at start (same file as Hearth's Game Mode
// settings use for the desktop).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Display"
    property var out: null        // wlr-randr's output
    property var cur: null        // {width, height, refresh}
    property var prev: null       // what to go back to if not confirmed
    property int countdown: 0
    property int brightness: -1

    readonly property var resolutions: {
        if (!out) return [];
        const seen = {}, r = [];
        for (const m of out.modes) { const k = m.width + "x" + m.height; if (!seen[k]) { seen[k] = true; r.push({ w: m.width, h: m.height }); } }
        return r.sort((a, b) => b.w * b.h - a.w * a.h);
    }
    readonly property var rates: out && cur ? out.modes.filter(m => m.width === cur.width && m.height === cur.height).map(m => Math.round(m.refresh)).filter((v, i, a) => a.indexOf(v) === i).sort((a, b) => b - a) : []

    function load() { randr.running = true; bright.running = true; }
    Component.onCompleted: load()
    Process {
        id: randr
        command: ["wlr-randr", "--json"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const outs = JSON.parse(text).filter(o => o.enabled);
                    page.out = outs[0] || null;
                    const m = page.out ? page.out.modes.find(x => x.current) : null;
                    page.cur = m ? { width: m.width, height: m.height, refresh: m.refresh } : null;
                } catch (e) { page.out = null; }
            }
        }
    }
    Process {
        id: bright
        command: ["brightnessctl", "-m", "-c", "backlight"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim().split(","); page.brightness = f.length >= 4 ? parseInt(f[3]) : -1; } }
    }
    function modeArg(w, h, r) { return `${w}x${h}@${r.toFixed ? r.toFixed(3) : r}Hz`; }
    function apply(w, h, r, scale) {
        if (!out) return;
        prev = { mode: modeArg(cur.width, cur.height, cur.refresh), scale: out.scale };
        const args = ["wlr-randr", "--output", out.name];
        if (w) args.push("--mode", modeArg(w, h, r));
        if (scale) args.push("--scale", String(scale));
        Quickshell.execDetached(args);
        countdown = 15;
        Qt.callLater(load);
    }
    function keep() {
        countdown = 0;
        const m = out.modes.find(x => x.current) || { width: cur.width, height: cur.height, refresh: cur.refresh };
        const line = `wlr-randr --output '${out.name}' --mode '${modeArg(m.width, m.height, m.refresh)}' --scale ${out.scale}`;
        Quickshell.execDetached(["sh", "-c", 'mkdir -p "$HOME/.config/nimbus" && printf \'#!/bin/sh\\n# Written by Hearth Settings › Display\\n%s\\n\' "$1" > "$HOME/.config/nimbus/display.sh" && chmod +x "$HOME/.config/nimbus/display.sh"', "sh", line]);
    }
    function revert() {
        countdown = 0;
        if (prev) Quickshell.execDetached(["wlr-randr", "--output", out.name, "--mode", prev.mode, "--scale", String(prev.scale)]);
        Qt.callLater(load);
    }
    Timer { interval: 1000; repeat: true; running: page.countdown > 0; onTriggered: { page.countdown--; if (page.countdown === 0) page.revert(); } }

    Rectangle {
        visible: page.countdown > 0
        Layout.fillWidth: true
        implicitHeight: keepCol.implicitHeight + 28
        radius: 12
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
        border.color: Theme.accent
        ColumnLayout {
            id: keepCol
            anchors { fill: parent; margins: 14 }
            Text { text: "Keep these display settings? Going back in " + page.countdown + " s"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
            RowLayout {
                spacing: 8
                SetRow { id: keepBtn; Layout.preferredWidth: 160; Layout.fillWidth: false; title: "Keep"; chevron: false; onActivated: page.keep() }
                SetRow { Layout.preferredWidth: 160; Layout.fillWidth: false; title: "Go back"; chevron: false; onActivated: page.revert() }
            }
        }
    }
    onCountdownChanged: if (countdown === 15) keepBtn.forceActiveFocus()

    SetSlider {
        visible: page.brightness >= 0
        icon: "sun"; title: "Brightness"; value: page.brightness / 100
        onMoved: v => { page.brightness = Math.max(1, Math.round(v * 100)); Quickshell.execDetached(["brightnessctl", "-q", "-c", "backlight", "set", page.brightness + "%"]); }
    }
    SetChoice {
        visible: !!page.out
        title: "Scale"; sub: "Makes text and everything else bigger or smaller"
        options: [1, 1.25, 1.5, 1.75, 2].map(v => ({ value: v, label: Math.round(v * 100) + "%" }))
        current: page.out ? page.out.scale : 1
        onChosen: v => page.apply(0, 0, 0, v)
    }
    SetChoice {
        visible: page.rates.length > 1
        title: "Refresh rate"; sub: "Higher is smoother; lower saves battery"
        options: page.rates.map(r => ({ value: r, label: r + " Hz" }))
        current: page.cur ? Math.round(page.cur.refresh) : 0
        onChosen: r => { const m = page.out.modes.find(x => x.width === page.cur.width && x.height === page.cur.height && Math.round(x.refresh) === r); if (m) page.apply(m.width, m.height, m.refresh, 0); }
    }
    SetHeader { text: "Resolution"; visible: page.resolutions.length > 0 }
    Repeater {
        model: page.resolutions.slice(0, 10)
        delegate: SetRow {
            required property var modelData
            required property int index
            icon: "display"; chevron: false
            title: modelData.w + " × " + modelData.h + (index === 0 ? "  (native)" : "")
            value: page.cur && page.cur.width === modelData.w && page.cur.height === modelData.h ? "In use" : ""
            onActivated: { const m = page.out.modes.filter(x => x.width === modelData.w && x.height === modelData.h).sort((a, b) => b.refresh - a.refresh)[0]; page.apply(m.width, m.height, m.refresh, 0); }
        }
    }
    Text { visible: !page.out; text: "Display settings work in Desktop Mode (wlr-randr)."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
}
