// Hearth Settings › Display: resolution, refresh rate, scale, rotation and brightness; with two or more screens which
// one you're setting, where it is next to the others, and turning one off; under KWin (Mantel on KWin, 1.14) HDR on
// the desktop with its SDR brightness, wide colour, VRR / FreeSync, colour profiles and allowing tearing in games
// (under labwc those rows say they need Mantel on KWin). Night light (on/off, how warm, when) is Settings' own.
// Everything goes through hearth-screens (docs/SCREENS.md: wlr-randr under labwc, kscreen-doctor / KWin's D-Bus under
// KWin). A change is kept only if you confirm it within 15 seconds, so a mode the screen can't show fixes itself, and
// the page says it went back. Kept settings go to ~/.config/nimbus/screens.json (and display.sh, which labwc's start
// runs; same lines as before), written by hearth-screens keep with hearth_settings_io.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

SetPage {
    id: page
    title: "Display"
    property var out: null        // the screen being set (hearth-screens get's shape: wlr-randr's, with more)
    property var outs: []         // every screen (on or off)
    property var sinfo: null      // hearth-screens info: {wm, nightLight, tearing, mirror, iccProfiles}
    readonly property bool kwin: !!sinfo && sinfo.wm === "kwin"
    readonly property string needsKwin: "Needs Mantel on KWin"
    property string picked: ""    // the name of the screen being set ("" = the first one that's on)
    property var cur: null        // {width, height, refresh}
    property var prev: null       // {outputs: {name: entry}}: what to go back to if not confirmed
    property var trying: []       // the screens whose change waits for Keep
    property int countdown: 0
    property var want: null       // the mode just asked for, checked when hearth-screens answers
    property string problem: ""   // the screen didn't take it
    property string notice: ""    // what went back by itself
    property int brightness: -1

    readonly property var resolutions: {
        if (!out || !out.enabled) return [];
        const seen = {}, r = [];
        for (const m of out.modes) { const k = m.width + "x" + m.height; if (!seen[k]) { seen[k] = true; r.push({ w: m.width, h: m.height }); } }
        return r.sort((a, b) => b.w * b.h - a.w * a.h);
    }
    readonly property var rates: out && cur ? out.modes.filter(m => m.width === cur.width && m.height === cur.height).map(m => Math.round(m.refresh)).filter((v, i, a) => a.indexOf(v) === i).sort((a, b) => b - a) : []

    function load() { screensJob.run(["info"], r => page.took(r), e => { page.out = null; page.problem = e; }); bright.running = true; gm.run(["get"], r => { page.gmode = r; }, () => {}); }
    // Game Mode's HDR (gamescope's --hdr-enabled, the same switch as Gaming › Performance): hearth-gamemode-display.
    property var gmode: null
    SetJob { id: gm; helper: "hearth-gamemode-display" }
    SetJob { id: screensJob; helper: "hearth-screens" }
    readonly property bool hdrOk: !!gmode && !!gmode.screen && gmode.screen.hdr && gmode.has.hdr
    Component.onCompleted: load()
    // "display:night", "display:hdr", "display:games", "display:layout": Settings opened at that part of the page
    // (SettingsNav.pageAnchor), scrolled there once the screens are read (its rows may be greyed out, so it scrolls
    // rather than focusing).
    function goToAnchor() {
        const to = ({ night: nightSwitch, hdr: hdrHead, games: gamesHead, layout: layoutHead })[SettingsNav.pageAnchor];
        if (!to) return;
        SettingsNav.pageAnchor = "";
        anchorGo.to = to;
        anchorGo.restart();     // after the rows that just appeared are laid out
    }
    Timer {
        id: anchorGo
        property var to: null
        interval: 400
        onTriggered: {
            const y = to.mapToItem(page.contentItem, 0, 0).y;
            page.contentY = Math.max(0, Math.min(page.contentHeight - page.height, y - 16));
            if (to === nightSwitch) nightSwitch.forceActiveFocus();
        }
    }
    // hearth-screens answered (info, or apply's info): the screens as they are now.
    function took(r) {
        const first = page.sinfo === null;
        page.sinfo = r;
        if (first) goToAnchor();
        const all = r.outputs || [];
        page.outs = all;
        const outs = all.filter(o => o.enabled);
        page.out = all.find(o => o.name === page.picked) || outs[0] || null;
        if (page.out && !page.picked) page.picked = page.out.name;
        const m = page.out ? page.out.modes.find(x => x.current) : null;
        page.cur = m ? { width: m.width, height: m.height, refresh: m.refresh } : null;
        // Did the screen really switch? (A compositor can refuse quietly.)
        if (r.problem) { page.problem = r.problem; page.countdown = 0; page.want = null; }
        else if (page.want && m) {
            const ok = m.width === page.want.w && m.height === page.want.h && Math.round(m.refresh) === Math.round(page.want.r);
            if (!ok) { page.problem = `The screen stayed at ${m.width} × ${m.height}, ${Math.round(m.refresh)} Hz: it didn't accept ${page.want.w} × ${page.want.h} at ${Math.round(page.want.r)} Hz.`; page.countdown = 0; }
            else page.problem = "";
            page.want = null;
        }
    }
    Process {
        id: bright
        command: ["brightnessctl", "-m", "-c", "backlight"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim().split(","); page.brightness = f.length >= 4 ? parseInt(f[3]) : -1; } }
    }
    // What a screen shows now, in hearth-screens' words (to go back to it).
    function entryOf(o) {
        const m = o.modes.find(x => x.current);
        const e = { enabled: o.enabled, scale: o.scale, pos: o.position || { x: 0, y: 0 }, rotation: o.rotation || "normal" };
        if (m) e.mode = { w: m.width, h: m.height, r: m.refresh };
        if (o.hdr && o.hdr.capable) e.hdr = o.hdr.on;
        if (o.vrr && o.vrr.capable && kwin) e.vrr = o.vrr.policy;
        if (o.wcg && o.wcg.capable) e.wcg = o.wcg.on;
        return e;
    }
    // Change screens now and wait for Keep (CHANGES: {name: entry}). A second change while one waits goes back to what
    // was there before both, not to the one waiting (it may be the mode the screen can't show).
    function tryChange(changes) {
        if (!out) return;
        if (countdown === 0) {
            const p = {};
            for (const n in changes) { const o = outs.find(x => x.name === n); if (o) p[n] = entryOf(o); }
            prev = { outputs: p };
            trying = Object.keys(changes);
        } else {
            const p = prev ? prev.outputs : {};
            for (const n in changes) if (!p[n]) { const o = outs.find(x => x.name === n); if (o) p[n] = entryOf(o); }
            prev = { outputs: p };
            trying = Object.keys(p);
        }
        problem = ""; notice = "";
        countdown = 15;
        screensJob.run(["apply", JSON.stringify({ outputs: changes })], r => page.took(r), e => { page.problem = e; page.countdown = 0; page.want = null; page.load(); });
    }
    function modeArg(w, h, r) { return { w: w, h: h, r: r }; }
    function apply(w, h, r, scale) {
        if (!out || !cur) return;
        const e = {};
        if (w) {
            // The screen's exact rate (e.g. 143.981), not the rounded one shown on the button.
            const exact = out.modes.find(x => x.width === w && x.height === h && Math.round(x.refresh) === Math.round(r));
            const rate = exact ? exact.refresh : r;
            e.mode = modeArg(w, h, rate);
            want = { w: w, h: h, r: rate };
        }
        if (scale) e.scale = scale;
        const c = {}; c[out.name] = e;
        tryChange(c);
    }
    // Changes that need no Keep (they can't leave the screen dark): applied and kept at once.
    function setNow(e, label) {
        if (!out) return;
        const c = {}; c[out.name] = e;
        screensJob.run(["set", JSON.stringify({ outputs: c })], r => { page.took(r); if (!r.problem && label) SettingsNav.toast(label); }, err => { page.problem = err; page.load(); });
    }
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
        // Positions from 0 (wlroots wants that): move the other screen over instead of going below 0.
        const c = {};
        c[out.name] = { pos: { x: Math.max(0, pos[0]), y: Math.max(0, pos[1]) } };
        if (pos[0] < 0 || pos[1] < 0) c[o.name] = { pos: { x: Math.max(0, -pos[0]), y: Math.max(0, -pos[1]) } };
        tryChange(c);
    }
    function setOn(on) {
        if (!out) return;
        if (!on && !others.length) { problem = "That's the only screen that's on, so it stays on."; return; }
        const c = {}; c[out.name] = { enabled: on };
        tryChange(c);
    }
    // Keep: what the waiting screens show now goes into screens.json (and display.sh), through hearth-screens keep.
    function keep() {
        countdown = 0;
        const names = trying.length ? trying : (out ? [out.name] : []);
        trying = [];
        if (names.length) screensJob.run(["keep", JSON.stringify(names)], r => { page.took(r); SettingsNav.toast("Display settings kept"); }, e => { page.problem = "Couldn't save the display settings: " + e; });
    }
    // Go back (the button, or by itself after 15 s: then the page says so, so nothing changes without you knowing).
    function revert(byItself) {
        countdown = 0;
        trying = [];
        if (prev && prev.outputs && Object.keys(prev.outputs).length) {
            screensJob.run(["apply", JSON.stringify(prev)], r => page.took(r), e => { page.problem = e; page.load(); });
            if (byItself) {
                page.notice = "Went back to the display settings you had: the new ones weren't kept within 15 seconds.";
                SettingsNav.toast("Display: went back to the settings you had");
            }
        } else Qt.callLater(load);
        prev = null;
    }
    // X on the controller (the frame's hint, mockup settings-A2): Keep these settings while a change waits.
    readonly property string xLabel: countdown > 0 ? "Keep these settings" : ""
    function xAction() { if (countdown > 0) keep(); }
    Timer { interval: 1000; repeat: true; running: page.countdown > 0; onTriggered: { page.countdown--; if (page.countdown === 0) page.revert(true); } }

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
                text: page.cur ? [page.cur.width + " × " + page.cur.height, Math.round(page.cur.refresh) + " Hz", page.gmode && page.gmode.hdr ? "HDR on in Game Mode" : "HDR off", page.gpuName ? "connected to the " + page.gpuName : ""].filter(x => x).join(" · ") : (page.out && !page.out.enabled ? "Off" : "")
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
    SetSwitch { id: nightSwitch; icon: "moon"; title: "Night light"; sub: "Warmer colours from sunset to sunrise, to help you sleep"; checked: Settings.nightLight; onToggled: v => Settings.nightLight = v }
    SetSwitch {
        icon: "sparkle"; title: "HDR"
        sub: !page.gmode ? "" : !page.gmode.screen.hdr ? "This screen doesn't have HDR" : !page.gmode.has.hdr ? "This PC's gamescope can't do HDR"
            : "Brighter, more vivid picture in games and videos that have it" + (page.gmode.hdr ? " · in Game Mode" : "")
        enabled: page.hdrOk
        checked: !!page.gmode && page.gmode.hdr
        onToggled: v => gm.run(["set", JSON.stringify({ hdr: v })], r => { page.gmode = r; SettingsNav.toast((v ? "HDR on" : "HDR off") + " · from the next time Game Mode starts"); }, e => SettingsNav.toast(e))
    }
    // Night light's details (Settings: nightTemp, nightMode, nightFrom, nightTo; wlsunset under labwc, KWin's Night
    // Light under KWin, both from the same settings).
    SetSlider {
        visible: Settings.nightLight
        icon: "moon"; title: "How warm"; sub: "Night light's colour: further right is warmer"
        value: (6500 - Settings.nightTemp) / 4500
        valueText: Settings.nightTemp + " K"
        onMoved: v => Settings.nightTemp = Math.round((6500 - v * 4500) / 100) * 100
    }
    SetChoice {
        visible: Settings.nightLight
        icon: "clock"; title: "When"; sub: Settings.nightMode === "always" ? "Warmer all the time" : "From " + Settings.nightFrom + " to " + Settings.nightTo + ", fading in and out"
        options: [{ value: "times", label: "Evenings" }, { value: "always", label: "All the time" }]
        current: Settings.nightMode
        onChosen: v => Settings.nightMode = v
    }
    SetChoice {
        visible: Settings.nightLight && Settings.nightMode !== "always"
        icon: "moon"; title: "Starts at"; sub: "Night light comes on"
        options: ["18:00", "19:00", "19:30", "20:00", "21:00", "22:00"].map(v => ({ value: v, label: v }))
        current: Settings.nightFrom
        onChosen: v => Settings.nightFrom = v
    }
    SetChoice {
        visible: Settings.nightLight && Settings.nightMode !== "always"
        icon: "sun"; title: "Ends at"; sub: "Back to normal colours"
        options: ["05:00", "06:00", "07:00", "08:00", "09:00"].map(v => ({ value: v, label: v }))
        current: Settings.nightTo
        onChosen: v => Settings.nightTo = v
    }

    // ---- HDR and colour on the desktop (KWin; under labwc these say they need Mantel on KWin)
    readonly property bool deskHdr: !!out && !!out.hdr && out.hdr.capable
    SetHeader { id: hdrHead; text: "HDR & colour on the desktop"; visible: !!page.out }
    SetSwitch {
        visible: !!page.out
        icon: "sparkle"; title: "HDR on the desktop"
        sub: !page.kwin ? page.needsKwin + " (Game Mode's HDR is the switch above)" : !page.deskHdr ? "This screen doesn't have HDR"
            : "Brighter highlights in HDR videos, games and pictures; the desktop itself keeps its colours"
        enabled: page.kwin && page.deskHdr
        opacity: enabled ? 1 : 0.5
        checked: page.deskHdr && page.out.hdr.on
        onToggled: v => { const c = {}; c[page.out.name] = { hdr: v }; page.tryChange(c); }
    }
    // SDR (ordinary) content's brightness while HDR is on, in nits: KWin's sdr-brightness. Saved a moment after you
    // stop moving it (no Keep needed: it can't leave the screen dark).
    SetSlider {
        visible: page.deskHdr && page.out.hdr.on
        icon: "sun"; title: "Brightness of everything else"; sub: "Apps, the desktop and videos that aren't HDR, while HDR is on"
        value: ((page.out && page.out.sdrBrightness ? page.out.sdrBrightness : 200) - 80) / 420
        valueText: Math.round(80 + value * 420) + " nits"
        onMoved: v => { page.sdrWant = Math.round(80 + v * 420); sdrSave.restart(); }
    }
    property int sdrWant: 0
    readonly property bool wcgOk: kwin && !!out && !!out.wcg && out.wcg.capable
    Timer { id: sdrSave; interval: 700; onTriggered: page.setNow({ sdrBrightness: page.sdrWant }, "") }
    SetSwitch {
        visible: !!page.out
        icon: "palette"; title: "Wide colour"
        sub: !page.kwin ? page.needsKwin : !page.wcgOk ? "This screen doesn't have wide colour" : "Richer colours on screens that show more than sRGB"
        enabled: page.wcgOk
        opacity: enabled ? 1 : 0.5
        checked: page.wcgOk && page.out.wcg.on
        onToggled: v => { const c = {}; c[page.out.name] = { wcg: v }; page.tryChange(c); }
    }
    // A colour profile: none (sRGB), the screen's own (from its EDID), or an ICC file (~/.local/share/icc,
    // /usr/share/color/icc).
    readonly property bool iccOk: kwin && !!out && !!out.icc && out.icc.capable
    SetChoice {
        visible: !!page.out
        icon: "palette"; title: "Colour profile"
        sub: !page.kwin ? page.needsKwin : !page.iccOk ? "KWin can't use a colour profile for this screen"
            : "For a calibrated screen: an ICC file in ~/.local/share/icc, or the screen's own"
        enabled: page.iccOk
        opacity: enabled ? 1 : 0.5
        options: [{ value: "", label: "None" }, { value: "EDID", label: "The screen's own" }].concat((page.sinfo && page.sinfo.iccProfiles || []).slice(0, 4).map(f => ({ value: f.path, label: f.label })))
        current: !page.iccOk ? "" : page.out.icc.source === "EDID" ? "EDID" : page.out.icc.source === "ICC" ? page.out.icc.path : ""
        onChosen: v => page.setNow({ icc: v }, "Colour profile: " + (v === "" ? "none" : v === "EDID" ? "the screen's own" : v.replace(/^.*\//, "")))
    }

    // ---- smoothness in games
    SetHeader { id: gamesHead; text: "Games and smoothness"; visible: !!page.out }
    readonly property bool vrrOk: kwin && !!out && !!out.vrr && out.vrr.capable
    SetChoice {
        visible: !!page.out
        icon: "bolt"; title: "Variable refresh rate (VRR / FreeSync)"
        sub: !page.kwin ? page.needsKwin : !page.vrrOk ? "This screen can't follow the game's frame rate"
            : "The screen follows the game's frame rate, so there's no tearing or stutter. Automatic: in full-screen games"
        enabled: page.vrrOk
        opacity: enabled ? 1 : 0.5
        options: [{ value: "never", label: "Never" }, { value: "automatic", label: "Automatic" }, { value: "always", label: "Always" }]
        current: page.vrrOk ? page.out.vrr.policy : "never"
        onChosen: v => { const c = {}; c[page.out.name] = { vrr: v }; page.tryChange(c); }
    }
    SetSwitch {
        visible: !!page.out
        icon: "pad"; title: "Allow tearing in full-screen games"
        sub: !page.kwin ? page.needsKwin : "Less delay in games that ask for it, with a line where frames meet; the desktop never tears"
        enabled: page.kwin && !!page.sinfo && page.sinfo.tearing.capable
        opacity: enabled ? 1 : 0.5
        checked: !!page.sinfo && page.sinfo.tearing.on
        onToggled: v => screensJob.run(["set", JSON.stringify({ tearing: v })], r => { page.took(r); SettingsNav.toast(v ? "Tearing allowed in full-screen games" : "No tearing in games"); }, e => { page.problem = e; page.load(); })
    }

    SetHeader { id: layoutHead; text: "Scale & layout"; visible: !!page.out }
    SetChoice {
        visible: !!page.out && page.out.enabled
        icon: "display"; title: "Scale"; sub: "Makes text, apps and the bar bigger or smaller"
        // Fractional steps; KWin draws in-between scales sharply, so it gets more of them.
        options: (page.kwin ? [1, 1.1, 1.25, 1.4, 1.5, 1.75, 2, 2.25, 2.5, 3] : [1, 1.25, 1.5, 1.75, 2])
            .concat(page.out && [1, 1.1, 1.25, 1.4, 1.5, 1.75, 2, 2.25, 2.5, 3].indexOf(Math.round(page.out.scale * 100) / 100) < 0 ? [Math.round(page.out.scale * 100) / 100] : [])
            .sort((a, b) => a - b).map(v => ({ value: v, label: Math.round(v * 100) + "%" }))
        current: page.out ? Math.round(page.out.scale * 100) / 100 : 1
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
    // Rotation: for a screen on its side (both compositors: wlr-randr's --transform, KWin's rotation).
    SetChoice {
        visible: !!page.out && page.out.enabled
        icon: "restart"; title: "Rotation"; sub: "Turn the picture for a screen that's on its side or upside down"
        options: [{ value: "normal", label: "Normal" }, { value: "left", label: "Left" }, { value: "right", label: "Right" }, { value: "inverted", label: "Upside down" }]
        current: page.out ? (page.out.rotation || "normal") : "normal"
        onChosen: v => { const c = {}; c[page.out.name] = { rotation: v }; page.tryChange(c); }
    }
    // Mirroring (the same picture on two screens): neither labwc nor KWin 6.3 can, so it says so instead of hiding.
    SetRow {
        visible: page.outs.length > 1 && !!page.sinfo
        icon: "desktops"; title: "Show the same thing on both screens (mirror)"
        sub: page.sinfo ? page.sinfo.mirror.why : ""
        value: "Not available"
        enabled: false
        opacity: 0.5
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
    Text { visible: !page.out; text: "Display settings work in Desktop Mode."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
    Text { visible: page.notice !== ""; text: page.notice; color: SetStyle.warn; wrapMode: Text.Wrap; Layout.fillWidth: true; font.family: Theme.font; font.pixelSize: 14 }
    Text { visible: page.problem !== ""; text: page.problem; color: "#ff9b8f"; wrapMode: Text.Wrap; Layout.fillWidth: true; font.family: Theme.font; font.pixelSize: 14 }

    // Nvidia: how the Hearth app draws (first-time setup, its windows; the same choice as its Settings › Display), so a
    // black screen there can be fixed from Desktop Mode.
    property string nvDrawing: "auto"
    readonly property string nvFile: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/nvidia-drawing"
    FileView { id: nvRead; path: page.nvFile; onLoaded: page.nvDrawing = (text().trim() || "auto"); onLoadFailed: page.nvDrawing = "auto" }
    Process { id: nvProbe; command: ["test", "-d", "/sys/module/nvidia"]; running: true; onExited: code => page.hasNvidia = code === 0 }
    property bool hasNvidia: false
    SetHeader { text: "Related" }
    SetRow {
        icon: "pad"; chevron: true
        title: "Game Mode's resolution and frame limit"; sub: "Gaming › Performance"
        onActivated: SettingsNav.openPage("gameperf")
    }
    SetHeader { text: "Advanced"; visible: page.hasNvidia }
    SetChoice {
        visible: page.hasNvidia
        icon: "cpu"; title: "Graphics card (Nvidia): how the Hearth app draws"; sub: "Fast is smooth on newer drivers. Compatible is slower but works everywhere: pick it if first-time setup or the Hearth app's window shows a black screen. Steam Game Mode isn't affected."
        options: [{ value: "auto", label: "Automatic" }, { value: "fast", label: "Fast" }, { value: "compatible", label: "Compatible" }]
        current: page.nvDrawing
        onChosen: v => { page.nvDrawing = v; Quickshell.execDetached(["sh", "-c", v === "auto" ? "rm -f \"$1\"" : "mkdir -p \"$(dirname \"$1\")\" && printf %s \"$2\" > \"$1\"", "sh", page.nvFile, v]); }
    }
}
