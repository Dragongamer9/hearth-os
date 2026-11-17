// Hearth Settings › Personalization › Tiling (Mantel on KWin, docs/KWIN-SETTINGS.md): each screen's own tile layout,
// drawn and changed here (columns, each split into rows), kept where KWin reads it (kwinrc [Tiling][<screen's id>]
// tiles) and put on the screen at once (a one-off KWin script). Drag a window with Shift held to drop it in a tile.
// Also quick tiling: dragging a window to an edge or corner, the top edge to maximize, and snapping.
// Snap layouts (Mantel's own, SnapLayouts.qml) are separate and stay as they are.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Tiling"
    KwinSettings { id: kw; onLoaded: page.load() }
    property var outs: []
    property var layouts: ({})         // uuid -> {columns: [{width, rows: [h...]}]}
    property real padding: 4
    property int pick: 0
    readonly property var out: outs[pick] || null
    readonly property var lay: out ? (layouts[out.uuid] || defaultLayout()) : defaultLayout()
    function defaultLayout() { return { columns: [{ width: 0.25, rows: [] }, { width: 0.5, rows: [] }, { width: 0.25, rows: [] }] }; }   // KWin's own
    function load() {
        kw.job.run(["outputs"], r => {
            page.outs = r.outputs || [];
            page.padding = parseFloat(r.padding || 4);
            const l = {};
            for (const o of page.outs) l[o.uuid] = o.layout || null;
            page.layouts = l;
        }, e => kw.say(e, true));
    }
    function norm(list) { const t = list.reduce((a, b) => a + b, 0) || 1; return list.map(x => Math.round(x / t * 1000) / 1000); }
    function edit(f) {
        if (!out) return;
        const l = JSON.parse(JSON.stringify(lay));
        f(l);
        const w = norm(l.columns.map(c => Math.max(0.05, c.width)));
        l.columns.forEach((c, i) => { c.width = w[i]; if (c.rows.length > 1) c.rows = norm(c.rows.map(h => Math.max(0.05, h))); else c.rows = []; });
        layouts = Object.assign({}, layouts, { [out.uuid]: l });
        save.restart();
    }
    Timer {
        id: save
        interval: 600
        onTriggered: if (page.out) kw.job.run(["tiles-set", page.out.uuid, page.out.name || "-"], r => kw.say(r.live || !kw.running ? "" : "Saved. KWin uses it from the next time you sign in."),
            e => kw.say(e, true), JSON.stringify(page.layouts[page.out.uuid] || page.defaultLayout()), "save")
    }
    readonly property var presets: [
        { label: "Halves", columns: [{ width: 0.5, rows: [] }, { width: 0.5, rows: [] }] },
        { label: "Thirds", columns: [{ width: 1, rows: [] }, { width: 1, rows: [] }, { width: 1, rows: [] }] },
        { label: "Wide middle", columns: [{ width: 0.25, rows: [] }, { width: 0.5, rows: [] }, { width: 0.25, rows: [] }] },
        { label: "Big left", columns: [{ width: 0.66, rows: [] }, { width: 0.34, rows: [0.5, 0.5] }] },
        { label: "Quarters", columns: [{ width: 0.5, rows: [0.5, 0.5] }, { width: 0.5, rows: [0.5, 0.5] }] }
    ]

    KwinGate {
        kw: kw
        what: "Tile layouts"

        SetHeader { text: "Your screens' tiles" + (kw.job.what === "save" ? " · saving…" : "") }
        SetChoice {
            visible: page.outs.length > 1
            icon: "display"; title: "Screen"
            options: page.outs.map((o, i) => ({ value: i, label: o.name || o.model }))
            current: page.pick
            onChosen: v => page.pick = v
        }
        Text {
            visible: page.outs.length === 0 && kw.ready
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "No screen found to lay out."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }
        // The layout, drawn the shape of the screen.
        Rectangle {
            visible: !!page.out
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(320, width * 9 / 16)
            radius: SetStyle.radius
            color: SetStyle.sunk
            border.width: 1; border.color: Theme.line
            Row {
                id: colsRow
                anchors { fill: parent; margins: 8 }
                spacing: 6
                Repeater {
                    model: page.lay.columns
                    delegate: Column {
                        id: colItem
                        required property var modelData
                        required property int index
                        readonly property int n: Math.max(1, modelData.rows.length)
                        width: (colsRow.width - colsRow.spacing * (page.lay.columns.length - 1)) * modelData.width
                        height: colsRow.height
                        spacing: 6
                        Repeater {
                            model: modelData.rows.length > 1 ? modelData.rows : [1]
                            delegate: Rectangle {
                                required property var modelData
                                width: parent.width
                                height: (colItem.height - 6 * (colItem.n - 1)) * modelData
                                radius: Math.max(4, Theme.r(8))
                                color: SetStyle.tint(Theme.accent, 0.18)
                                border.width: 1; border.color: SetStyle.tint(Theme.accent, 0.6)
                            }
                        }
                    }
                }
            }
            Text {
                anchors { right: parent.right; bottom: parent.bottom; margins: 12 }
                visible: !!page.out && page.out.custom && !page.layouts[page.out.uuid]
                text: "Made in KWin's own editor (Super+T): changing it here starts from KWin's three columns"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
        }
        SetChoice {
            visible: !!page.out
            icon: "grid"; title: "Start from"
            options: page.presets.map((p, i) => ({ value: i, label: p.label }))
            current: -1
            onChosen: v => page.edit(l => { l.columns = JSON.parse(JSON.stringify(page.presets[v].columns)); })
        }
        Repeater {
            model: page.out ? page.lay.columns : []
            delegate: ColumnLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: 3
                SetSlider {
                    icon: "window"; title: "Column " + (index + 1) + " width"
                    value: modelData.width
                    step: 0.05
                    onMoved: v => page.edit(l => {
                        const n = l.columns.length, old = l.columns[index].width, nv = Math.max(0.1, Math.min(0.9, v));
                        const rest = 1 - old, nrest = 1 - nv;
                        l.columns.forEach((c, i) => { c.width = i === index ? nv : (rest > 0 ? c.width / rest * nrest : nrest / (n - 1)); });
                    })
                }
                SetChoice {
                    icon: "grid"; title: "Column " + (index + 1) + " rows"
                    options: [1, 2, 3].map(n => ({ value: n, label: String(n) }))
                    current: Math.max(1, modelData.rows.length)
                    onChosen: v => page.edit(l => { l.columns[index].rows = v > 1 ? Array(v).fill(1) : []; })
                }
                SetButton {
                    visible: page.lay.columns.length > 1
                    text: "Remove column " + (index + 1); icon: "close"; ghost: true
                    Layout.leftMargin: 56
                    onActivated: page.edit(l => { l.columns.splice(index, 1); })
                }
            }
        }
        RowLayout {
            visible: !!page.out
            spacing: 10
            Layout.topMargin: 4
            SetButton {
                text: "Add a column"; icon: "plus"
                enabled: page.lay.columns.length < 8
                onActivated: page.edit(l => { l.columns.push({ width: 1 / l.columns.length, rows: [] }); })
            }
            SetButton {
                text: "Open KWin's tile editor"; icon: "edit"; ghost: true
                visible: kw.running
                onActivated: kw.shortcut("tiles")
            }
        }
        SetSlider {
            icon: "grid"; title: "Gap between tiles"
            value: page.padding / 32
            step: 1 / 32
            valueText: Math.round(page.padding) + " px"
            onMoved: v => {
                page.padding = Math.round(v * 32);
                gap.restart();
            }
            Timer { id: gap; interval: 600; onTriggered: if (page.out) kw.job.run(["tiles-set", page.out.uuid, page.out.name || "-"], () => {}, e => kw.say(e, true), JSON.stringify({ padding: page.padding }), "save") }
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Hold Shift while you drag a window to drop it into a tile. Super+T opens KWin's own editor too."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }

        SetHeader { text: "Quick tiling" }
        SetSwitch {
            icon: "window"; title: "Drag a window to a side or corner to tile it"
            sub: "Half the screen at a side, a quarter in a corner"
            checked: kw.b("kwinrc", "Windows", "ElectricBorderTiling", true)
            onToggled: v => kw.set("kwinrc", "Windows", "ElectricBorderTiling", v)
        }
        SetSwitch {
            icon: "window"; title: "Drag a window to the top to maximize it"
            checked: kw.b("kwinrc", "Windows", "ElectricBorderMaximize", true)
            onToggled: v => kw.set("kwinrc", "Windows", "ElectricBorderMaximize", v)
        }
        SetSlider {
            icon: "grid"; title: "Size of the corners"
            sub: "How much of each edge counts as its corner"
            value: kw.n("kwinrc", "Windows", "ElectricBorderCornerRatio", 0.25) / 0.5
            valueText: Math.round(value * 50) + "%"
            onMoved: v => kw.set("kwinrc", "Windows", "ElectricBorderCornerRatio", Math.max(0.05, Math.round(v * 50) / 100))
        }
        SetSwitch {
            icon: "window"; title: "Maximized windows have no title bar"
            checked: kw.b("kwinrc", "Windows", "BorderlessMaximizedWindows", false)
            onToggled: v => kw.set("kwinrc", "Windows", "BorderlessMaximizedWindows", v)
        }

        SetHeader { text: "Snapping" }
        SetSlider {
            icon: "display"; title: "Snap to the screen's edges"
            value: kw.n("kwinrc", "Windows", "BorderSnapZone", 10) / 50
            valueText: value === 0 ? "Off" : Math.round(value * 50) + " px"
            onMoved: v => kw.set("kwinrc", "Windows", "BorderSnapZone", Math.round(v * 50))
        }
        SetSlider {
            icon: "window"; title: "Snap to other windows"
            value: kw.n("kwinrc", "Windows", "WindowSnapZone", 10) / 50
            valueText: value === 0 ? "Off" : Math.round(value * 50) + " px"
            onMoved: v => kw.set("kwinrc", "Windows", "WindowSnapZone", Math.round(v * 50))
        }
        SetSlider {
            icon: "grid"; title: "Snap to the middle of the screen"
            value: kw.n("kwinrc", "Windows", "CenterSnapZone", 0) / 50
            valueText: value === 0 ? "Off" : Math.round(value * 50) + " px"
            onMoved: v => kw.set("kwinrc", "Windows", "CenterSnapZone", Math.round(v * 50))
        }
        SetSwitch {
            icon: "window"; title: "Only snap when windows would overlap"
            checked: kw.b("kwinrc", "Windows", "SnapOnlyWhenOverlapping", false)
            onToggled: v => kw.set("kwinrc", "Windows", "SnapOnlyWhenOverlapping", v)
        }
    }
}
