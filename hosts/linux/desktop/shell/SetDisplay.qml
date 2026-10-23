// Hearth Settings › Display: resolution, refresh rate, scale (wlr-randr), and brightness. With two or more screens:
// which one you're setting, where it is next to the others, and turning one off. A change is kept only if you
// confirm it within 15 seconds, so a mode the screen can't show fixes itself. Kept settings go to
// ~/.config/nimbus/display.sh, which Desktop Mode runs at start (same file as Hearth's Game Mode settings use for
// the desktop).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Display"
    property var out: null        // wlr-randr's output (the one being set)
    property var outs: []         // every screen wlr-randr knows (on or off)
    property string picked: ""    // the name of the screen being set ("" = the first one that's on)
    property var cur: null        // {width, height, refresh}
    property var prev: null       // what to go back to if not confirmed
    property int countdown: 0
    property var want: null       // the mode just asked for, checked when wlr-randr answers
    property string problem: ""   // the screen didn't take it
    property int brightness: -1

    readonly property var resolutions: {
        if (!out || !out.enabled) return [];
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
                    const all = JSON.parse(text);
                    page.outs = all;
                    const outs = all.filter(o => o.enabled);
                    page.out = all.find(o => o.name === page.picked) || outs[0] || null;
                    if (page.out && !page.picked) page.picked = page.out.name;
                    const m = page.out ? page.out.modes.find(x => x.current) : null;
                    page.cur = m ? { width: m.width, height: m.height, refresh: m.refresh } : null;
                    // Did the screen really switch? (wlr-randr can refuse quietly.)
                    if (page.want && m) {
                        const ok = m.width === page.want.w && m.height === page.want.h && Math.round(m.refresh) === Math.round(page.want.r);
                        if (!ok) { page.problem = `The screen stayed at ${m.width} × ${m.height}, ${Math.round(m.refresh)} Hz: it didn't accept ${page.want.w} × ${page.want.h} at ${Math.round(page.want.r)} Hz.`; page.countdown = 0; }
                        else page.problem = "";
                        page.want = null;
                    }
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
        if (!out || !cur) return;
        // A second change while one is waiting for Keep goes back to what was there before both, not to the
        // one waiting (it may be the mode the screen can't show).
        if (countdown === 0) prev = { name: out.name, mode: modeArg(cur.width, cur.height, cur.refresh), scale: out.scale, pos: posArg(out), on: true };
        const args = ["wlr-randr", "--output", out.name];
        if (w) {
            // The screen's exact rate (e.g. 143.981), not the rounded one shown on the button: wlr-randr only
            // switches to a mode the screen lists.
            const exact = out.modes.find(x => x.width === w && x.height === h && Math.round(x.refresh) === Math.round(r));
            const rate = exact ? exact.refresh : r;
            args.push("--mode", modeArg(w, h, rate));
            want = { w: w, h: h, r: rate };
        }
        if (scale) args.push("--scale", String(scale));
        problem = "";
        applyProc.command = args;
        applyProc.running = true;
        countdown = 15;
    }
    // wlr-randr runs, then the screen is read back to see what it's really showing.
    Process { id: applyProc; onExited: (code) => { if (code !== 0 && page.want) { page.problem = "The screen didn't accept that mode."; page.countdown = 0; page.want = null; } Qt.callLater(page.load); } }
    // ---- several screens: where each one is, and turning one off
    function posArg(o) { const p = o && o.position ? o.position : { x: 0, y: 0 }; return p.x + "," + p.y; }
    // A screen's size on the desktop (its mode divided by its scale, turned for a rotated screen).
    function logical(o) {
        const m = o.modes.find(x => x.current) || o.modes[0] || { width: 1920, height: 1080 };
        const turned = /90|270/.test(o.transform || "");
        const sc = o.scale || 1;
        return { w: Math.round((turned ? m.height : m.width) / sc), h: Math.round((turned ? m.width : m.height) / sc) };
    }
    readonly property var others: outs.filter(o => o.enabled && out && o.name !== out.name)
    readonly property string side: {
        if (!out || !others.length || !out.position) return "";
        const o = others[0], p = out.position, q = o.position || { x: 0, y: 0 };
        const a = logical(o);
        if (p.x >= q.x + a.w) return "right";
        if (p.x + logical(out).w <= q.x) return "left";
        if (p.y + logical(out).h <= q.y) return "above";
        if (p.y >= q.y + a.h) return "below";
        return "";
    }
    function placeAt(where) {
        if (!out || !others.length) return;
        const o = others[0], q = o.position || { x: 0, y: 0 }, a = logical(o), me = logical(out);
        const pos = where === "right" ? [q.x + a.w, q.y] : where === "left" ? [q.x - me.w, q.y]
            : where === "above" ? [q.x, q.y - me.h] : [q.x, q.y + a.h];
        // wlroots wants positions from 0: move the other screen over instead of going below 0.
        const args = ["wlr-randr", "--output", out.name, "--pos", Math.max(0, pos[0]) + "," + Math.max(0, pos[1])];
        if (pos[0] < 0 || pos[1] < 0) args.push("--output", o.name, "--pos", Math.max(0, -pos[0]) + "," + Math.max(0, -pos[1]));
        if (countdown === 0) prev = { name: out.name, mode: modeArg(cur.width, cur.height, cur.refresh), scale: out.scale, pos: posArg(out), on: true, other: o.name, otherPos: posArg(o) };
        problem = "";
        applyProc.command = args;
        applyProc.running = true;
        countdown = 15;
    }
    function setOn(on) {
        if (!out) return;
        if (!on && !others.length) { problem = "That's the only screen that's on, so it stays on."; return; }
        if (countdown === 0) prev = { name: out.name, mode: cur ? modeArg(cur.width, cur.height, cur.refresh) : "", scale: out.scale, pos: posArg(out), on: out.enabled };
        problem = "";
        applyProc.command = ["wlr-randr", "--output", out.name, on ? "--on" : "--off"];
        applyProc.running = true;
        countdown = 15;
    }
    function lineFor(o) {
        if (!o.enabled) return `wlr-randr --output '${o.name}' --off`;   // turned off: remembered as off
        const m = o.modes.find(x => x.current) || o.modes[0];
        return `wlr-randr --output '${o.name}' --on --mode '${modeArg(m.width, m.height, m.refresh)}' --scale ${o.scale}` + (outs.length > 1 ? ` --pos ${posArg(o)}` : "");
    }
    function keep() {
        countdown = 0;
        const lines = [[lineFor(out), out.name]];
        // A screen moved over to make room keeps its new place too.
        if (prev && prev.other) { const o = outs.find(x => x.name === prev.other); if (o && o.enabled) lines.push([lineFor(o), o.name]); }
        saveLines(lines);
    }
    // [[line, screen name]...] into display.sh, replacing those screens' lines and keeping the others'.
    function saveLines(lines) {
        Quickshell.execDetached(["sh", "-c", 'f="$HOME/.config/nimbus/display.sh"; mkdir -p "$HOME/.config/nimbus" || exit 1; keep=$(grep "^wlr-randr " "$f" 2>/dev/null); new=""; while [ $# -ge 2 ]; do keep=$(printf "%s\n" "$keep" | grep -vF -- "--output \'$2\' "); new="$new$1\n"; shift 2; done; { printf "#!/bin/sh\n# Written by Hearth Settings › Display\n"; [ -n "$keep" ] && printf "%s\n" "$keep"; printf "$new"; } > "$f.tmp" && chmod +x "$f.tmp" && mv -f "$f.tmp" "$f"', "sh"].concat([].concat.apply([], lines)));
    }
    function revert() {
        countdown = 0;
        if (prev) {
            const args = ["wlr-randr", "--output", prev.name || out.name];
            if (prev.on === false) args.push("--off");
            else {
                args.push("--on");
                if (prev.mode) args.push("--mode", prev.mode);
                args.push("--scale", String(prev.scale));
                if (outs.length > 1 && prev.pos) args.push("--pos", prev.pos);
            }
            if (prev.other) args.push("--output", prev.other, "--pos", prev.otherPos);
            Quickshell.execDetached(args);
        }
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

    // Two or more screens: which one these settings are for, where it is, and on or off.
    SetChoice {
        visible: page.outs.length > 1
        title: "Screen"; sub: "The settings below are for this one"
        options: page.outs.slice(0, 4).map(o => ({ value: o.name, label: (o.model || o.description || o.name).split(" ").slice(0, 3).join(" ") + (o.enabled ? "" : " (off)") }))
        current: page.picked
        onChosen: v => { page.picked = v; page.countdown = 0; page.load(); }
    }
    SetSwitch {
        visible: page.outs.length > 1 && !!page.out
        icon: "display"; title: "Use this screen"
        sub: page.out && page.out.enabled ? "On" : "Off: windows and the taskbar move to the other screens"
        checked: !!page.out && page.out.enabled
        onToggled: v => page.setOn(v)
    }
    SetChoice {
        visible: page.others.length > 0 && !!page.out && page.out.enabled
        title: "Where it is"; sub: "Next to " + (page.others.length ? (page.others[0].model || page.others[0].name) : "the other screen") + ", so the pointer goes across where they meet"
        options: [{ value: "left", label: "Left" }, { value: "right", label: "Right" }, { value: "above", label: "Above" }, { value: "below", label: "Below" }]
        current: page.side
        onChosen: v => page.placeAt(v)
    }
    SetSlider {
        visible: page.brightness >= 0
        icon: "sun"; title: "Brightness"; value: page.brightness / 100
        onMoved: v => { page.brightness = Math.max(1, Math.round(v * 100)); Quickshell.execDetached(["brightnessctl", "-q", "-c", "backlight", "set", page.brightness + "%"]); }
    }
    SetChoice {
        visible: !!page.out && page.out.enabled
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
    Text { visible: page.problem !== ""; text: page.problem; color: "#ff9b8f"; wrapMode: Text.Wrap; Layout.fillWidth: true; font.family: Theme.font; font.pixelSize: 14 }

    // Nvidia: how Game Mode's Hearth app draws (the same choice as its Settings › Display). Here too, so a
    // black Game Mode screen can be fixed from Desktop Mode.
    property string nvDrawing: "auto"
    readonly property string nvFile: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/nvidia-drawing"
    FileView { id: nvRead; path: page.nvFile; onLoaded: page.nvDrawing = (text().trim() || "auto"); onLoadFailed: page.nvDrawing = "auto" }
    Process { id: nvProbe; command: ["test", "-d", "/sys/module/nvidia"]; running: true; onExited: code => page.hasNvidia = code === 0 }
    property bool hasNvidia: false
    SetHeader { text: "Nvidia"; visible: page.hasNvidia }
    SetChoice {
        visible: page.hasNvidia
        title: "How Game Mode draws"; sub: "Fast is smooth on newer drivers. Compatible is slower but works everywhere: pick it if Game Mode shows a black screen. Applies the next time Game Mode starts."
        options: [{ value: "auto", label: "Automatic" }, { value: "fast", label: "Fast" }, { value: "compatible", label: "Compatible" }]
        current: page.nvDrawing
        onChosen: v => { page.nvDrawing = v; Quickshell.execDetached(["sh", "-c", v === "auto" ? "rm -f \"$1\"" : "mkdir -p \"$(dirname \"$1\")\" && printf %s \"$2\" > \"$1\"", "sh", page.nvFile, v]); }
    }
}
