// Hearth Settings › Personalization › Screen edges (Mantel on KWin, docs/KWIN-SETTINGS.md): what happens when the pointer
// is pushed into each corner and edge (hot corners), and what a swipe in from each edge of a touchscreen does. KWin keeps
// the plain actions in kwinrc [ElectricBorders] / [TouchEdges] and the effects' ones in their own groups
// ([Effect-overview] BorderActivate, GridBorderActivate, [Effect-windowview] BorderActivateAll...); a corner has one
// action, so choosing one takes the corner out of the others. Also: the delay, switching desktops at the edge, and the
// sticky edges between screens.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Screen edges"
    KwinSettings { id: kw }
    // KWin's ElectricBorder numbers (effect/globals.h) and its [ElectricBorders] key names.
    readonly property var edges: [
        { n: 7, key: "TopLeft", label: "Top-left corner" }, { n: 0, key: "Top", label: "Top edge" },
        { n: 1, key: "TopRight", label: "Top-right corner" }, { n: 6, key: "Left", label: "Left edge" },
        { n: 2, key: "Right", label: "Right edge" }, { n: 5, key: "BottomLeft", label: "Bottom-left corner" },
        { n: 4, key: "Bottom", label: "Bottom edge" }, { n: 3, key: "BottomRight", label: "Bottom-right corner" }]
    // Actions: a plain one ([ElectricBorders] value) or an effect's list of edges.
    readonly property var actions: [
        { value: "none", label: "Nothing" },
        { value: "overview", label: "Overview", group: "Effect-overview", list: "BorderActivate", touch: "TouchBorderActivate", effect: "overview", dflt: [7] },
        { value: "grid", label: "Desktop grid", group: "Effect-overview", list: "GridBorderActivate", touch: "GridTouchBorderActivate", effect: "overview", dflt: [] },
        { value: "windows", label: "All windows", group: "Effect-windowview", list: "BorderActivateAll", touch: "TouchBorderActivateAll", effect: "windowview", dflt: [] },
        { value: "appwindows", label: "This app's windows", group: "Effect-windowview", list: "BorderActivateClass", touch: "TouchBorderActivateClass", effect: "windowview", dflt: [] },
        { value: "ShowDesktop", label: "Show the desktop" },
        { value: "LockScreen", label: "Lock the screen" }]
    function listOf(a, touch) { return kw.list("kwinrc", a.group, touch ? a.touch : a.list, touch ? [] : a.dflt); }
    function actionAt(e, touch) {
        for (const a of actions) if (a.group && listOf(a, touch).indexOf(e.n) >= 0) return a.value;
        const plain = kw.v("kwinrc", touch ? "TouchEdges" : "ElectricBorders", e.key, "None");
        return plain === "ShowDesktop" || plain === "LockScreen" ? plain : "none";
    }
    function choose(e, value, touch) {
        const ch = {};
        for (const a of actions) {
            if (!a.group) continue;
            const l = listOf(a, touch);
            const has = l.indexOf(e.n) >= 0, want = a.value === value;
            if (has === want) continue;
            // 9 is KWin's "no edge" (what KDE's own settings write for an empty list).
            const nl = (want ? l.concat([e.n]) : l.filter(x => x !== e.n)).filter(x => x !== 9);
            ch[a.group] = Object.assign(ch[a.group] || {}, { [touch ? a.touch : a.list]: nl.length ? nl.join(",") : "9" });
        }
        const g = touch ? "TouchEdges" : "ElectricBorders";
        const plainNow = kw.v("kwinrc", g, e.key, "None");
        const plainWant = value === "ShowDesktop" || value === "LockScreen" ? value : "None";
        if (plainNow !== plainWant) ch[g] = { [e.key]: plainWant };
        if (Object.keys(ch).length) kw.setMany("kwinrc", ch);
    }
    function label(v) { return (actions.find(a => a.value === v) || actions[0]).label; }
    function opts() { return actions.map(a => ({ value: a.value, label: a.label + (a.effect && !kw.effectOn(a.effect) ? " (its effect is off)" : "") })); }

    KwinGate {
        kw: kw
        what: "Screen edges and hot corners"

        SetHeader { text: "Hot corners and edges" + (kw.job.what === "save" ? " · saving…" : "") }
        // The screen, with what each corner and edge does.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(260, width * 9 / 16)
            radius: SetStyle.radius
            color: SetStyle.sunk
            border.width: 1; border.color: Theme.line
            Repeater {
                model: page.edges
                delegate: Text {
                    required property var modelData
                    readonly property string act: page.actionAt(modelData, false)
                    readonly property real fx: ({ 7: 0, 0: 0.5, 1: 1, 6: 0, 2: 1, 5: 0, 4: 0.5, 3: 1 })[modelData.n]
                    readonly property real fy: ({ 7: 0, 0: 0, 1: 0, 6: 0.5, 2: 0.5, 5: 1, 4: 1, 3: 1 })[modelData.n]
                    x: 14 + (parent.width - 28 - width) * fx
                    y: 12 + (parent.height - 24 - height) * fy
                    text: page.label(act)
                    color: act === "none" ? Theme.muted : Theme.accent
                    font.family: Theme.font; font.pixelSize: 13; font.weight: act === "none" ? Font.Normal : Font.DemiBold
                }
            }
        }
        Repeater {
            model: page.edges
            delegate: SetDropdown {
                required property var modelData
                icon: "display"; title: modelData.label
                options: page.opts()
                current: page.actionAt(modelData, false)
                onChosen: v => page.choose(modelData, v, false)
            }
        }
        SetSlider {
            icon: "clock"; title: "How long to push before it happens"
            value: kw.n("kwinrc", "Windows", "ElectricBorderDelay", 75) / 1000
            step: 0.025
            valueText: Math.round(value * 1000) + " ms"
            onMoved: v => kw.set("kwinrc", "Windows", "ElectricBorderDelay", Math.round(v * 1000))
        }
        SetSlider {
            icon: "clock"; title: "Wait before it can happen again"
            value: kw.n("kwinrc", "Windows", "ElectricBorderCooldown", 350) / 2000
            step: 0.025
            valueText: Math.round(value * 2000) + " ms"
            onMoved: v => kw.set("kwinrc", "Windows", "ElectricBorderCooldown", Math.round(v * 2000))
        }
        SetSwitch {
            icon: "display"; title: "Also over full-screen windows"
            sub: "Off: a game or video in full screen never sets off a corner"
            checked: kw.b("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", false)
            onToggled: v => kw.set("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", v)
        }
        SetChoice {
            icon: "desktops"; title: "Switch desktops at the left and right edges"
            options: [{ value: 0, label: "Never" }, { value: 1, label: "When moving a window" }, { value: 2, label: "Always" }]
            current: kw.n("kwinrc", "Windows", "ElectricBorders", 0)
            onChosen: v => kw.set("kwinrc", "Windows", "ElectricBorders", v)
        }

        SetHeader { text: "Between screens" }
        SetSlider {
            icon: "display"; title: "Sticky edges between screens"
            sub: "How far you push the pointer before it crosses to the next screen; 0 is off"
            value: kw.n("kwinrc", "EdgeBarrier", "EdgeBarrier", 100) / 500
            step: 0.02
            valueText: Math.round(value * 500) + " px"
            onMoved: v => kw.set("kwinrc", "EdgeBarrier", "EdgeBarrier", Math.round(v * 500))
        }
        SetSwitch {
            icon: "display"; title: "Corners between screens are sticky too"
            checked: kw.b("kwinrc", "EdgeBarrier", "CornerBarrier", true)
            onToggled: v => kw.set("kwinrc", "EdgeBarrier", "CornerBarrier", v)
        }

        SetHeader { text: "Touchscreen: swipe in from an edge" }
        Repeater {
            model: page.edges.filter(e => [0, 2, 4, 6].indexOf(e.n) >= 0)
            delegate: SetDropdown {
                required property var modelData
                icon: "pad"; title: "Swipe in from the " + modelData.label.replace(" edge", "")
                options: page.opts()
                current: page.actionAt(modelData, true)
                onChosen: v => page.choose(modelData, v, true)
            }
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "KWin's own touch gestures stay: three fingers up for the overview, down for the grid, sideways to switch desktops."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
}
