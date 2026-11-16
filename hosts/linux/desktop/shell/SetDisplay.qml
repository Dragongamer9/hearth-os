// Hearth Settings › Display: resolution, refresh rate, scale (wlr-randr), and brightness. With two or more screens:
// which one you're setting, where it is next to the others, and turning one off. A change is kept only if you
// confirm it within 15 seconds, so a mode the screen can't show fixes itself. Kept settings go to
// ~/.config/nimbus/display.sh, which Desktop Mode runs at start (same file as Hearth's Game Mode settings use for
// the desktop).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

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
    // [[line, screen name]...] into display.sh, replacing those screens' lines and keeping the others'. Under the
    // lock Hearth's own writer uses, flushed to the drive, the old one kept as display.sh.bak (SETTINGS-PERSISTENCE.md).
    function saveLines(lines) {
        Quickshell.execDetached(["sh", "-c", 'd="$HOME/.config/nimbus"; f="$d/display.sh"; mkdir -p "$d" || exit 1; exec 9>"$d/.display.sh.lock"; flock 9 2>/dev/null; keep=$(grep "^wlr-randr " "$f" 2>/dev/null); new=""; while [ $# -ge 2 ]; do keep=$(printf "%s\n" "$keep" | grep -vF -- "--output \'$2\' "); new="$new$1\n"; shift 2; done; t=$(mktemp "$d/.display.sh.XXXXXX") || exit 1; { printf "#!/bin/sh\n# Written by Hearth Settings › Display\n"; [ -n "$keep" ] && printf "%s\n" "$keep"; printf "$new"; } > "$t" && chmod +x "$t" && sync "$t" 2>/dev/null; [ -s "$f" ] && cp -p "$f" "$f.bak"; mv -f "$t" "$f"', "sh"].concat([].concat.apply([], lines)));
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
    // X on the controller (the frame's hint, mockup settings-A2): Keep these settings while a change waits.
    readonly property string xLabel: countdown > 0 ? "Keep these settings" : ""
    function xAction() { if (countdown > 0) keep(); }
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

    // 1.14 (mockup settings-A2): your screens drawn as they're placed (1, 2…), the picked one lit; its name, mode and
    // the graphics card it's on; Identify (each screen shows its number for 3 s), Detect (reads the screens again) and
    // Arrange (to "Where this screen is").
    property string gpuName: ""
    Process {
        running: true
        command: ["sh", "-c", "g=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1); [ -n \"$g\" ] || g=$(lspci -mm 2>/dev/null | grep -E '\"(VGA|3D|Display)' | head -1 | awk -F'\" \"' '{print $2\" \"$3}'); echo \"$g\""]
        stdout: StdioCollector { onStreamFinished: {
            const t = text.trim().replace(/\[|\]/g, " ").replace(/\s+/g, " ");
            const m = /\b((RTX|GTX|RX|Arc)\s?[A-Z]?\d{3,4}\s?(Ti|SUPER|XT|XTX)?)\b/i.exec(t);
            page.gpuName = m ? m[1].trim() : /Radeon/i.test(t) ? "Radeon graphics" : /Intel/i.test(t) ? "Intel graphics" : t;
        } }
    }
    readonly property var drawn: {
        const on = outs.filter(o => o.enabled);
        if (!on.length) return [];
        const ls = on.map(o => ({ o: o, p: o.position || { x: 0, y: 0 }, s: logical(o) }));
        const minX = Math.min(...ls.map(l => l.p.x)), minY = Math.min(...ls.map(l => l.p.y));
        const maxX = Math.max(...ls.map(l => l.p.x + l.s.w)), maxY = Math.max(...ls.map(l => l.p.y + l.s.h));
        const k = Math.min(420 / Math.max(1, maxX - minX), 150 / Math.max(1, maxY - minY));
        return ls.map((l, i) => ({ name: l.o.name, n: i + 1, x: (l.p.x - minX) * k, y: (l.p.y - minY) * k, w: Math.max(40, l.s.w * k - 8), h: Math.max(28, l.s.h * k - 8) }));
    }
    property bool identifying: false
    Timer { id: identifyOff; interval: 3000; onTriggered: page.identifying = false }
    Variants {
        model: page.identifying ? Quickshell.screens : []
        delegate: PanelWindow {
            required property var modelData
            screen: modelData
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-identify"
            anchors { top: true; left: true }
            margins { top: 40; left: 40 }
            implicitWidth: 180; implicitHeight: 180
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            Rectangle {
                anchors.fill: parent; radius: 24; color: Theme.panel; border.width: 3; border.color: Theme.accent
                Text {
                    anchors.centerIn: parent
                    text: String(Math.max(1, (page.drawn.find(d => d.name === modelData.name) || { n: Quickshell.screens.indexOf(modelData) + 1 }).n))
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 110; font.weight: Font.Bold
                }
            }
        }
    }
    Rectangle {
        visible: !!page.out
        Layout.fillWidth: true
        implicitHeight: Math.max(190, scrInfo.implicitHeight + 50)
        radius: SetStyle.bigRadius; color: SetStyle.card; border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
        Item {
            id: scrPic
            x: 24; y: 22
            width: Math.max(...page.drawn.map(d => d.x + d.w).concat([120])); height: Math.max(...page.drawn.map(d => d.y + d.h).concat([80]))
            Repeater {
                model: page.drawn
                delegate: Rectangle {
                    id: scr
                    required property var modelData
                    readonly property bool lit: modelData.name === page.picked
                    x: modelData.x; y: modelData.y; width: modelData.w; height: modelData.h
                    radius: 8
                    color: lit ? SetStyle.mix(SetStyle.card2, Theme.accent, 0.12) : SetStyle.card2
                    border.width: lit ? 3 : 1; border.color: lit ? Theme.accent : Theme.line
                    Text { anchors.centerIn: parent; text: scr.modelData.n; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.min(44, scr.height * 0.4); font.weight: scr.lit ? Font.Bold : Font.Normal }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { page.picked = scr.modelData.name; page.countdown = 0; page.load(); } }
                }
            }
        }
        ColumnLayout {
            id: scrInfo
            anchors { left: scrPic.right; leftMargin: 32; right: parent.right; rightMargin: 24; verticalCenter: parent.verticalCenter }
            spacing: 6
            Text {
                Layout.fillWidth: true
                text: page.out ? (page.out.model || page.out.description || page.out.name) + (page.drawn.length > 1 && page.drawn[0] && page.drawn[0].name === page.out.name ? " · main display" : page.drawn.length > 1 ? "" : " · main display") : ""
                color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold; elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                text: page.cur ? [page.cur.width + " × " + page.cur.height, Math.round(page.cur.refresh) + " Hz", "HDR off", page.gpuName ? "connected to the " + page.gpuName : ""].filter(x => x).join(" · ") : (page.out && !page.out.enabled ? "Off" : "")
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
            }
            RowLayout {
                spacing: 8
                Layout.topMargin: 6
                SetButton { text: "Identify"; onActivated: { page.identifying = true; identifyOff.restart(); } }
                SetButton { ghost: true; text: "Detect"; onActivated: { page.load(); SettingsNav.toast("Looked for screens again: " + page.outs.length + (page.outs.length === 1 ? " screen" : " screens")); } }
                SetButton { ghost: true; text: "Arrange"; visible: page.others.length > 0; onActivated: arrangeChoice.forceActiveFocus() }
            }
        }
    }

    // Two or more screens: which one these settings are for.
    SetChoice {
        visible: page.outs.length > 1
        icon: "display"; title: "Screen"; sub: "The settings below are for this one"
        options: page.outs.slice(0, 4).map(o => ({ value: o.name, label: (o.model || o.description || o.name).split(" ").slice(0, 3).join(" ") + (o.enabled ? "" : " (off)") }))
        current: page.picked
        onChosen: v => { page.picked = v; page.countdown = 0; page.load(); }
    }

    SetHeader { text: "Brightness & colour" }
    SetSlider {
        visible: page.brightness >= 0
        icon: "sun"; title: "Brightness"; sub: "Only for screens that let Hearth change it"; value: page.brightness / 100
        onMoved: v => { page.brightness = Math.max(1, Math.round(v * 100)); Quickshell.execDetached(["brightnessctl", "-q", "-c", "backlight", "set", page.brightness + "%"]); }
    }
    // Night light lives here now (it was in Appearance): the same setting (Settings.nightLight), and Quick Access's tile.
    SetSwitch { icon: "moon"; title: "Night light"; sub: "Warmer colours from sunset to sunrise, to help you sleep"; checked: Settings.nightLight; onToggled: v => Settings.nightLight = v }

    SetHeader { text: "Scale & layout"; visible: !!page.out }
    SetChoice {
        visible: !!page.out && page.out.enabled
        icon: "display"; title: "Scale"; sub: "Makes text, apps and the bar bigger or smaller"
        options: [1, 1.25, 1.5, 1.75, 2].map(v => ({ value: v, label: Math.round(v * 100) + "%" }))
        current: page.out ? page.out.scale : 1
        onChosen: v => page.apply(0, 0, 0, v)
    }
    // Resolution as a choice (mockups A2 and B2): the screen's own and the next one down, plus Other… for the rest
    // (the list below); in the controller look every resolution, one after another with ‹ ›.
    property bool otherRes: false
    readonly property var resChoices: {
        const all = resolutions.map(r => ({ value: r.w + "x" + r.h, label: r.w + " × " + r.h }));
        if (SetStyle.big) return all;
        const top = all.slice(0, 2);
        const curK = cur ? cur.width + "x" + cur.height : "";
        if (curK && !top.some(o => o.value === curK)) top.push({ value: curK, label: cur.width + " × " + cur.height });
        return all.length > top.length ? top.concat([{ value: "other", label: "Other…" }]) : top;
    }
    SetChoice {
        visible: page.resolutions.length > 0
        icon: "window"; title: "Resolution"
        sub: page.resolutions.length ? "Recommended: " + page.resolutions[0].w + " × " + page.resolutions[0].h : ""
        options: page.resChoices
        current: page.otherRes ? "other" : page.cur ? page.cur.width + "x" + page.cur.height : ""
        onChosen: v => {
            if (v === "other") { page.otherRes = true; return; }
            page.otherRes = false;
            const [w, h] = v.split("x").map(Number);
            const m = page.out.modes.filter(x => x.width === w && x.height === h).sort((a, b) => b.refresh - a.refresh)[0];
            if (m) page.apply(m.width, m.height, m.refresh, 0);
        }
    }
    SetChoice {
        visible: page.rates.length > 1
        icon: "bolt"; title: "Refresh rate"; sub: "Higher is smoother; lower saves battery"
        options: page.rates.map(r => ({ value: r, label: r + " Hz" }))
        current: page.cur ? Math.round(page.cur.refresh) : 0
        onChosen: r => { const m = page.out.modes.find(x => x.width === page.cur.width && x.height === page.cur.height && Math.round(x.refresh) === r); if (m) page.apply(m.width, m.height, m.refresh, 0); }
    }
    SetChoice {
        id: arrangeChoice
        visible: page.others.length > 0 && !!page.out && page.out.enabled
        icon: "desktops"; title: "Where this screen is"; sub: "Next to " + (page.others.length ? (page.others[0].model || page.others[0].name) : "the other screen") + ", so the pointer goes across where they meet"
        options: [{ value: "left", label: "Left" }, { value: "right", label: "Right" }, { value: "above", label: "Above" }, { value: "below", label: "Below" }]
        current: page.side
        onChosen: v => page.placeAt(v)
    }
    SetSwitch {
        visible: page.outs.length > 1 && !!page.out
        icon: "display"; title: "Use this screen"
        sub: page.out && page.out.enabled ? "On" : "Off: windows and the taskbar move to the other screens"
        checked: !!page.out && page.out.enabled
        onToggled: v => page.setOn(v)
    }
    SetExpander {
        visible: page.resolutions.length > 0 && page.otherRes && !SetStyle.big
        icon: "window"; title: "Other resolutions"; sub: "Sharper with more pixels"
        value: page.cur ? page.cur.width + " × " + page.cur.height : ""
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
    }
    Text { visible: !page.out; text: "Display settings work in Desktop Mode (wlr-randr)."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
    Text { visible: page.problem !== ""; text: page.problem; color: "#ff9b8f"; wrapMode: Text.Wrap; Layout.fillWidth: true; font.family: Theme.font; font.pixelSize: 14 }

    // Nvidia: how the Hearth app draws (first-time setup, its windows; the same choice as its Settings › Display), so a
    // black screen there can be fixed from Desktop Mode.
    property string nvDrawing: "auto"
    readonly property string nvFile: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/nvidia-drawing"
    FileView { id: nvRead; path: page.nvFile; onLoaded: page.nvDrawing = (text().trim() || "auto"); onLoadFailed: page.nvDrawing = "auto" }
    Process { id: nvProbe; command: ["test", "-d", "/sys/module/nvidia"]; running: true; onExited: code => page.hasNvidia = code === 0 }
    property bool hasNvidia: false
    SetHeader { text: "Advanced"; visible: page.hasNvidia }
    SetChoice {
        visible: page.hasNvidia
        icon: "cpu"; title: "Graphics card (Nvidia): how the Hearth app draws"; sub: "Fast is smooth on newer drivers. Compatible is slower but works everywhere: pick it if first-time setup or the Hearth app's window shows a black screen. Steam Game Mode isn't affected."
        options: [{ value: "auto", label: "Automatic" }, { value: "fast", label: "Fast" }, { value: "compatible", label: "Compatible" }]
        current: page.nvDrawing
        onChosen: v => { page.nvDrawing = v; Quickshell.execDetached(["sh", "-c", v === "auto" ? "rm -f \"$1\"" : "mkdir -p \"$(dirname \"$1\")\" && printf %s \"$2\" > \"$1\"", "sh", page.nvFile, v]); }
    }
}
