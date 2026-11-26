// Right-click any part › Customize, in one of Mantel's apps (drag and drop board 3, with board 2's side panel:
// docs/mockups/drag-drop/drag-drop-2.png). For the part or block picked: its name and where it is ("· in the command
// bar"), Reset, and the tabs Look, Shows, Does, Place, When, Motion. Every setting changes the draft at once, so it's
// seen on the part as you go (AppBlocks.qml draws it: size, spacing, fill, see-through, blur, corners, edge, shadow,
// icon, words, font, show, hover and pressed looks; AppBlock.qml for a block), and nothing is written until Keep this.
// A run of changes to one setting (a slider dragged) is one Undo. Loaded by AppBlocksEditor.qml only while it's open.
import QtQuick
import "app-blocks.js" as L

Rectangle {
    id: cp
    property var ab: null
    readonly property string pid: ab ? ab.customizeFor : ""
    readonly property var block: ab ? (ab.draft.added.find(b => b.id === pid) || null) : null
    readonly property var part: ab ? (ab.draft.parts.find(p => p.id === pid) || null) : null
    readonly property bool isBlock: !!block && block.kind !== "pane"
    readonly property bool inBar: isBlock && !!block.in
    readonly property var look: ab ? (ab.draft.looks[pid] || ({})) : ({})
    readonly property var eff: { if (!ab) return ({}); void ab.draft; return ab.lookOf(pid); }
    readonly property string tab: ab ? ab.customizeTab : "look"
    // where it's drawn now (px)
    readonly property var rect: {
        if (!ab) return { x: 0, y: 0, w: 0, h: 0 };
        if (part) return part;
        if (block && !block.in) return L.fromFrac(block, ab.width, ab.height);
        const it = block ? ab.blockItems[block.id] : null;
        return it ? { x: 0, y: 0, w: Math.round(it.width), h: Math.round(it.height) } : { x: 0, y: 0, w: 0, h: 0 };
    }
    function set(patch) { if (ab && pid) ab.setLook(pid, patch); }
    function setPart(patch) { if (!ab) return; const id = pid; ab.withDraft(d => { const p = d.parts.find(x => x.id === id); if (p) Object.assign(p, patch); else { const b = d.added.find(x => x.id === id); if (b && !b.in) Object.assign(b, L.toFrac(Object.assign(L.fromFrac(b, ab.width, ab.height), patch), ab.width, ab.height)); } }); }
    function setBlock(patch) { if (!ab) return; const id = pid; ab.withDraft(d => { const b = d.added.find(x => x.id === id); if (b) Object.assign(b, patch); }); }
    function does(how) { return look.does && look.does[how] ? look.does[how] : { k: "", a: "" }; }
    function setDoes(how, v) { const d = Object.assign({}, look.does || {}); if (!v.k) delete d[how]; else d[how] = { k: v.k, a: v.a || "" }; set({ does: Object.keys(d).length ? d : "" }); }

    color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.99)
    radius: 16
    border.color: Theme.line
    Rectangle { z: -1; anchors.fill: parent; anchors.topMargin: 8; anchors.margins: -2; radius: 18; color: Qt.rgba(0, 0, 0, 0.35) }
    MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onClicked: cp.pick = null }

    // ---- the head: its name, where it is, Reset, close
    Item {
        id: head
        width: parent.width; height: 56
        Row {
            anchors { left: parent.left; leftMargin: 18; verticalCenter: parent.verticalCenter }
            spacing: 10
            Rectangle { width: 16; height: 16; radius: 4; color: cp.eff.fill || Theme.accent; border.color: Theme.line; anchors.verticalCenter: parent.verticalCenter }
            Text { text: cp.ab ? cp.ab.labelOf(cp.pid) : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; width: Math.min(implicitWidth, 220) }
            Text { text: "· " + (cp.ab ? L.whereIs(cp.isBlock ? cp.block : null, cp.ab.draft.parts, cp.ab.appName) : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
        }
        Row {
            anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
            spacing: 8
            Btn { label: "Reset"; onClicked: cp.ab.resetLook(cp.pid) }
            Btn { label: "✕"; onClicked: cp.ab.customizeFor = "" }
        }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
    }
    // ---- the tabs
    Row {
        id: tabs
        anchors { top: head.bottom; topMargin: 14; left: parent.left; leftMargin: 18 }
        spacing: 6
        Repeater {
            model: [["look", "Look"], ["shows", "Shows"], ["does", "Does"], ["place", "Place"], ["when", "When"], ["motion", "Motion"]]
            delegate: Rectangle {
                required property var modelData
                readonly property bool on: cp.tab === modelData[0]
                width: tt.implicitWidth + 24; height: 30; radius: 8
                color: on ? Theme.accent : tm.containsMouse ? Theme.panel3 : "transparent"; border.color: on ? Theme.accent : Theme.line
                Text { id: tt; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                MouseArea { id: tm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: cp.ab.customizeTab = parent.modelData[0] }
            }
        }
    }

    Flickable {
        id: fl
        anchors { top: tabs.bottom; topMargin: 12; left: parent.left; right: parent.right; bottom: foot.top; bottomMargin: 8; leftMargin: 18; rightMargin: 18 }
        contentHeight: body.implicitHeight + 8
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Column {
            id: body
            width: fl.width
            spacing: 10

            // ================================================================ Look
            Column {
                visible: cp.tab === "look"
                width: parent.width; spacing: 10
                Head { text: "SIZE AND PLACE" }
                Line {
                    label: "Width"
                    Seg { options: [["fit", "Fit"], ["fixed", "Fixed"], ["fill", "Fill"]]; value: cp.look.wMode || "fixed"; onPicked: v => cp.set({ wMode: v, wPx: v === "fixed" ? (cp.look.wPx || cp.rect.w) : "" }) }
                    Num { value: cp.look.wMode === "fixed" && cp.look.wPx ? cp.look.wPx : cp.rect.w; suffix: " px"; onCommit: v => { if (cp.isBlock || cp.look.wMode) cp.set({ wMode: "fixed", wPx: v }); else cp.setPart({ w: v }); } }
                }
                Line {
                    label: "Height"
                    Seg { options: [["fit", "Fit"], ["fixed", "Fixed"], ["fill", "Fill"]]; value: cp.look.hMode || "fixed"; onPicked: v => cp.set({ hMode: v, hPx: v === "fixed" ? (cp.look.hPx || cp.rect.h) : "" }) }
                    Num { value: cp.look.hMode === "fixed" && cp.look.hPx ? cp.look.hPx : cp.rect.h; suffix: " px"; onCommit: v => { if (cp.isBlock || cp.look.hMode) cp.set({ hMode: "fixed", hPx: v }); else cp.setPart({ h: v }); } }
                }
                Line {
                    label: "Space around it"
                    Repeater {
                        model: [["↑", 0], ["→", 1], ["↓", 2], ["←", 3]]
                        delegate: Num {
                            required property var modelData
                            prefix: modelData[0] + " "; width: 70
                            value: (cp.look.m || [0, 0, 0, 0])[modelData[1]]
                            onCommit: v => { const m = (cp.look.m || [0, 0, 0, 0]).slice(); m[modelData[1]] = Math.max(0, Math.min(200, v)); cp.set({ m: m }); }
                        }
                    }
                }
                Line {
                    label: "Space inside"
                    Slide { from: 0; to: 48; value: cp.look.pad !== undefined ? cp.look.pad : 0; onMoved: v => cp.set({ pad: Math.round(v) }) }
                    Val { text: (cp.look.pad !== undefined ? cp.look.pad : 0) + " px" }
                }
                Head { text: "BACKGROUND AND EDGE" }
                Line {
                    label: "Fill"
                    Repeater {
                        model: ["", "#2b2b30", Theme.accent.toString(), "#3d5afe", "#f06292"]
                        delegate: Rectangle {
                            required property string modelData
                            readonly property bool on: (cp.look.fill || "") === (modelData === "" ? "" : String(modelData).toLowerCase().slice(0, 7))
                            width: 24; height: 24; radius: 6
                            color: modelData === "" ? Theme.bg : modelData
                            border.width: on ? 2 : 1; border.color: on ? "#ffffff" : Theme.line
                            Text { visible: parent.modelData === ""; anchors.centerIn: parent; text: "∅"; color: Theme.muted; font.pixelSize: 12 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: cp.set({ fill: parent.modelData === "" ? "" : String(parent.modelData).toLowerCase().slice(0, 7) }) }
                        }
                    }
                    Words { width: 96; hint: "#rrggbb"; value: cp.look.fill || ""; onCommit: v => { const t = v.trim(); if (t === "" || /^#[0-9a-fA-F]{6}$/.test(t)) cp.set({ fill: t.toLowerCase() }); } }
                }
                Line {
                    label: "See-through"
                    Slide { from: 0; to: 90; value: cp.eff.see || 0; onMoved: v => cp.set({ see: Math.round(v) }) }
                    Val { text: (cp.eff.see || 0) + " %" }
                }
                Line {
                    label: "Blur behind it"
                    Toggle { on: !!cp.look.blur; onFlipped: cp.set({ blur: !cp.look.blur ? true : "" }) }
                }
                Line {
                    label: "Corners"
                    Slide { from: 0; to: 48; value: cp.eff.corners || 0; onMoved: v => cp.set({ corners: Math.round(v) }) }
                    Val { text: (cp.eff.corners || 0) === 0 ? "Square" : (cp.eff.corners || 0) >= 24 ? "Round" : cp.eff.corners + " px" }
                }
                Line {
                    label: "Edge"
                    Seg { options: [["none", "None"], ["line", "Line"], ["glow", "Glow"]]; value: cp.eff.edge || "none"; onPicked: v => cp.set({ edge: v === "none" ? "" : v }) }
                    Colour { value: cp.eff.edgeColor || Theme.accent.toString().slice(0, 7); onPicked: v => cp.set({ edgeColor: v }) }
                    Num { value: cp.eff.edgeW || 1; suffix: " px"; width: 70; onCommit: v => cp.set({ edgeW: Math.max(1, Math.min(12, v)) }) }
                }
                Line {
                    label: "Shadow"
                    Seg { options: [["none", "None"], ["soft", "Soft"], ["deep", "Deep"]]; value: cp.look.shadow || "none"; onPicked: v => cp.set({ shadow: v === "none" ? "" : v }) }
                }
                Head { text: "ICON AND WORDS" }
                Line {
                    label: "Icon"
                    Pick {
                        width: 150
                        icon: cp.look.icon || ""
                        label: cp.look.icon ? cp.look.icon : "None"
                        options: [["", "None"]].concat(["sparkle", "share", "archive", "folder", "file", "image", "film", "doc", "trash", "copy", "cut", "edit", "search", "link", "info", "grid", "list", "star", "rocket", "gear", "clock", "calendar", "cpu", "drive", "display", "cam", "play", "pause", "sync", "undo", "lock", "eye", "pin", "home", "user", "bell", "bolt", "globe", "palette", "store", "window", "plus", "check"].map(n => [n, n]))
                        onPicked: v => cp.set({ icon: v })
                    }
                    Colour { value: cp.look.iconColor || "#f2e8df"; onPicked: v => cp.set({ iconColor: v }) }
                    Num { value: cp.look.iconSize || 16; suffix: " px"; width: 70; onCommit: v => cp.set({ iconSize: Math.max(8, Math.min(96, v)) }) }
                }
                Line {
                    label: "Words"
                    Words { width: 260; value: cp.look.words || ""; hint: cp.isBlock ? "Downloads · {count}" : "A heading · {count}"; onCommit: v => cp.set({ words: v, show: v && !cp.look.show ? (cp.look.icon ? "both" : "words") : cp.look.show || "" }) }
                }
                Line {
                    label: "Font"
                    Pick {
                        width: 130
                        label: (cp.look.font || "Inter") + " ▾"
                        options: { const f = [["", "Hearth's own"], ["Inter", "Inter"], ["Inter Display", "Inter Display"], ["Noto Sans", "Noto Sans"], ["Noto Serif", "Noto Serif"], ["Noto Sans Mono", "Noto Sans Mono"]]; const more = Qt.fontFamilies().filter(n => !f.some(x => x[0] === n)).slice(0, 30).map(n => [n, n]); return f.concat(more); }
                        onPicked: v => cp.set({ font: v })
                    }
                    Num { value: cp.look.fontSize || 14; width: 64; onCommit: v => cp.set({ fontSize: Math.max(8, Math.min(72, v)) }) }
                    Seg { options: [["light", "Light"], ["medium", "Medium"], ["bold", "Bold"]]; value: cp.look.weight || "medium"; onPicked: v => cp.set({ weight: v }) }
                }
                Line {
                    label: "Show"
                    Seg { options: [["icon", "Icon"], ["words", "Words"], ["both", "Both"]]; value: cp.look.show || (cp.isBlock ? "both" : ""); onPicked: v => cp.set({ show: v }) }
                }
                Head { text: "WHEN IT'S POINTED AT · PRESSED" }
                Line {
                    label: "Hover look"
                    Seg { options: [["same", "Same"], ["lighter", "Lighter"], ["accent", "Accent"], ["own", "Own…"]]; value: cp.look.hover || (cp.isBlock ? "lighter" : "same"); onPicked: v => cp.set({ hover: v, hoverColor: v === "own" ? (cp.look.hoverColor || "#5b8cff") : cp.look.hoverColor || "" }) }
                    Colour { visible: cp.look.hover === "own"; value: cp.look.hoverColor || "#5b8cff"; onPicked: v => cp.set({ hoverColor: v }) }
                }
                Line {
                    label: "Pressed look"
                    Seg { options: [["same", "Same"], ["darker", "Darker"], ["own", "Own…"]]; value: cp.look.pressed || "same"; onPicked: v => cp.set({ pressed: v, pressedColor: v === "own" ? (cp.look.pressedColor || "#2f9e5a") : cp.look.pressedColor || "" }) }
                    Colour { visible: cp.look.pressed === "own"; value: cp.look.pressedColor || "#2f9e5a"; onPicked: v => cp.set({ pressedColor: v }) }
                }
                Head { text: "THE OTHER TABS" }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "Shows: what's in it (a count, words, the clock…) · Does: press, right-click, middle-click, scroll, hover — open, run, show a panel, an action, nothing · Place: where it is on the window, which bar, above or under the others · When: always, only on hover, only when there's something to show, only on some pages · Motion: how it appears, bounces when something new comes"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; lineHeight: 1.3
                }
            }

            // ================================================================ Shows
            Column {
                visible: cp.tab === "shows"
                width: parent.width; spacing: 10
                Head { text: "WHAT'S IN IT" }
                Line {
                    visible: cp.isBlock
                    label: "It shows"
                    Seg {
                        options: (cp.block && (cp.block.kind === "action" || cp.block.kind === "button") ? [["own", cp.block.kind === "action" ? "Its button" : "Its words"]] : []).concat([["count", "A count"], ["words", "Words"], ["clock", "The clock"], ["space", "Space"]])
                        value: cp.look.shows || (cp.block ? (cp.block.kind === "count" ? "count" : cp.block.kind === "words" ? "words" : cp.block.kind === "spacer" ? "space" : "own") : "own")
                        onPicked: v => cp.set({ shows: v })
                    }
                }
                Line {
                    visible: !cp.isBlock
                    label: "It shows"
                    Text { text: cp.ab ? "What " + cp.ab.appName + " puts in " + L.shortName(cp.ab.titleOf(cp.pid)).toLowerCase() + ", and a heading if you give it one" : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; width: 300; wrapMode: Text.WordWrap }
                }
                Line {
                    visible: !cp.isBlock
                    label: "Its heading"
                    Seg { options: [["", "None"], ["icon", "Icon"], ["words", "Words"], ["both", "Both"]]; value: cp.look.show || ""; onPicked: v => cp.set({ show: v, words: v && v !== "icon" && !cp.look.words ? cp.ab.labelOf(cp.pid).toUpperCase() : cp.look.words || "" }) }
                }
                Line {
                    visible: cp.ab && cp.ab.counts.length > 0
                    label: "The count"
                    Pick {
                        width: 220
                        label: { const c = cp.ab ? cp.ab.countOf(cp.look.showsRef || (cp.block ? cp.block.ref : "") || (cp.ab.counts[0] || {}).id) : null; return (c ? c.label : "Pick one") + " ▾"; }
                        options: cp.ab ? cp.ab.counts.map(c => [c.id, c.label + " · " + cp.ab.countValue(c.id)]) : []
                        onPicked: v => cp.set({ showsRef: v })
                    }
                }
                Line {
                    label: "Words"
                    Words { width: 260; value: cp.look.words || (cp.block && cp.block.kind === "words" ? cp.block.label || "" : ""); hint: "Downloads · {count}"; onCommit: v => { if (cp.block && cp.block.kind === "words" && !cp.look.words) cp.setBlock({ label: v }); else cp.set({ words: v }); } }
                }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "Words can show values in {braces}: " + (cp.ab ? Object.keys(cp.ab.wordValues).map(k => "{" + k + "}").join(", ") : "")
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
            }

            // ================================================================ Does
            Column {
                visible: cp.tab === "does"
                width: parent.width; spacing: 10
                Head { text: "WHAT IT DOES" }
                Line {
                    visible: cp.block && cp.block.kind === "button"
                    label: "Runs"
                    Words { width: 300; value: cp.block ? cp.block.run || "" : ""; hint: "a command, as you'd type it"; onCommit: v => cp.setBlock({ run: v }) }
                }
                Repeater {
                    model: [["press", "Press"], ["right", "Right-click"], ["middle", "Middle-click"], ["scroll", "Scroll"], ["hover", "Pointed at"]]
                    delegate: Column {
                        id: dr
                        required property var modelData
                        readonly property var d: cp.does(modelData[0])
                        width: body.width; spacing: 6
                        Line {
                            label: dr.modelData[1]
                            Seg {
                                options: [["", "Its own"], ["nothing", "Nothing"], ["open", "Open"], ["run", "Run"], ["panel", "Show a panel"], ["action", "An action"]]
                                value: dr.d.k || ""
                                onPicked: v => cp.setDoes(dr.modelData[0], { k: v, a: v === "panel" ? (cp.ab.draft.parts[0] || {}).id || "" : v === "action" ? (cp.ab.actions[0] || {}).id || "" : "" })
                            }
                        }
                        Line {
                            visible: dr.d.k === "open" || dr.d.k === "run"
                            label: ""
                            Words { width: 300; value: dr.d.a || ""; hint: dr.d.k === "open" ? "a file, a folder or a web address" : "a command"; onCommit: v => cp.setDoes(dr.modelData[0], { k: dr.d.k, a: v }) }
                        }
                        Line {
                            visible: dr.d.k === "panel" || dr.d.k === "action"
                            label: ""
                            Pick {
                                width: 260
                                label: (dr.d.k === "panel" ? (cp.ab ? cp.ab.labelOf(dr.d.a) : "") : ((cp.ab && cp.ab.actionOf(dr.d.a)) || { label: "Pick one" }).label) + " ▾"
                                options: dr.d.k === "panel" ? (cp.ab ? cp.ab.draft.parts.map(p => [p.id, cp.ab.labelOf(p.id)]) : []) : (cp.ab ? cp.ab.actions.map(a => [a.id, a.label]) : [])
                                onPicked: v => cp.setDoes(dr.modelData[0], { k: dr.d.k, a: v })
                            }
                        }
                    }
                }
            }

            // ================================================================ Place
            Column {
                visible: cp.tab === "place"
                width: parent.width; spacing: 10
                Head { text: "WHERE IT IS" }
                Line {
                    visible: !cp.inBar
                    label: "On the window"
                    Num { prefix: "x "; value: cp.rect.x; width: 84; onCommit: v => cp.setPart({ x: Math.max(0, v) }) }
                    Num { prefix: "y "; value: cp.rect.y; width: 84; onCommit: v => cp.setPart({ y: Math.max(0, v) }) }
                    Num { prefix: "w "; value: cp.rect.w; width: 84; onCommit: v => cp.setPart({ w: Math.max(24, v) }) }
                    Num { prefix: "h "; value: cp.rect.h; width: 84; onCommit: v => cp.setPart({ h: Math.max(12, v) }) }
                }
                Line {
                    visible: cp.isBlock
                    label: "In"
                    Pick {
                        width: 260
                        label: (cp.inBar ? cp.ab.labelOf(cp.block.in) : "The window, anywhere") + " ▾"
                        options: cp.ab ? cp.ab.barIds.map(b => [b, cp.ab.labelOf(b)]).concat([["", "The window, anywhere"]]) : []
                        onPicked: v => {
                            if (v === "") cp.setBlock({ in: "", at: undefined, x: 0.4, y: 0.45, w: 0.09, h: 0.04 });
                            else cp.ab.withDraft(d => { const b = d.added.find(x => x.id === cp.pid); if (b) { b.in = v; b.at = 400; delete b.x; delete b.y; delete b.w; delete b.h; } });
                        }
                    }
                }
                Line {
                    label: "Above or under"
                    Btn { label: "To the front"; onClicked: cp.ab.raise(cp.pid, true) }
                    Btn { label: "To the back"; onClicked: cp.ab.raise(cp.pid, false) }
                }
                Line {
                    visible: !!cp.part
                    label: "Lined up"
                    Btn { label: "Left edge"; onClicked: cp.setPart({ x: 0 }) }
                    Btn { label: "Right edge"; onClicked: cp.setPart({ x: Math.max(0, cp.ab.width - cp.rect.w) }) }
                    Btn { label: "Full width"; onClicked: cp.setPart({ x: 0, w: cp.ab.width }) }
                }
                Line {
                    visible: !!cp.part
                    label: ""
                    Btn { label: "Top edge"; onClicked: cp.setPart({ y: 0 }) }
                    Btn { label: "Bottom edge"; onClicked: cp.setPart({ y: Math.max(0, cp.ab.height - cp.rect.h) }) }
                    Btn { label: "Full height"; onClicked: cp.setPart({ y: 0, h: cp.ab.height }) }
                }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap
                    text: "Or drag it: parts go anywhere on the window, not only to its sides; Snap to grid and Guides line it up."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
            }

            // ================================================================ When
            Column {
                visible: cp.tab === "when"
                width: parent.width; spacing: 10
                Head { text: "WHEN IT SHOWS" }
                Seg {
                    options: [["always", "Always"], ["hover", "Only when pointed at"], ["something", "When there's something to show"], ["pages", "Only on some pages"]]
                    value: cp.look.when || "always"
                    onPicked: v => cp.set({ when: v === "always" ? "" : v, pages: v === "pages" ? (cp.look.pages && cp.look.pages.length ? cp.look.pages : [cp.ab.page]) : "" })
                }
                Flow {
                    visible: cp.look.when === "pages"
                    width: parent.width; spacing: 6
                    Repeater {
                        model: cp.ab ? cp.ab.pages : []
                        delegate: Rectangle {
                            required property var modelData
                            readonly property bool on: (cp.look.pages || []).indexOf(modelData.id) >= 0
                            width: pt.implicitWidth + 22; height: 28; radius: 8
                            color: on ? Theme.accent : "transparent"; border.color: on ? Theme.accent : Theme.line
                            Text { id: pt; anchors.centerIn: parent; text: (parent.on ? "✓ " : "") + parent.modelData.title; color: parent.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                                onClicked: { const p = (cp.look.pages || []).slice(); const i = p.indexOf(parent.modelData.id); if (i >= 0) p.splice(i, 1); else p.push(parent.modelData.id); cp.set({ pages: p }); } }
                        }
                    }
                }
                Text {
                    visible: cp.ab && cp.ab.pages.length < 2 && cp.look.when === "pages"
                    width: parent.width; wrapMode: Text.WordWrap
                    text: cp.ab ? cp.ab.appName + " has one page here, so this part shows on it." : ""
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
            }

            // ================================================================ Motion
            Column {
                visible: cp.tab === "motion"
                width: parent.width; spacing: 10
                Head { text: "HOW IT MOVES" }
                Line {
                    label: "How it appears"
                    Seg { options: [["none", "At once"], ["fade", "Fade"], ["slide", "Slide up"], ["pop", "Pop"]]; value: cp.look.appear || "none"; onPicked: v => cp.set({ appear: v === "none" ? "" : v }) }
                }
                Line {
                    label: "Bounces when something new comes"
                    Toggle { on: !!cp.look.bounce; onFlipped: cp.set({ bounce: !cp.look.bounce ? true : "" }) }
                }
                Text {
                    width: parent.width; wrapMode: Text.WordWrap
                    text: cp.isBlock ? "A count bounces when it goes up." : "A part bounces when it has something to show again (the details pane opened)."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
            }
        }
    }

    // ---- the foot: Save as my own block…, Copy its look, Apply to every bar item
    Row {
        id: foot
        anchors { left: parent.left; leftMargin: 18; bottom: parent.bottom; bottomMargin: 16 }
        spacing: 8
        Btn {
            label: "Save as my own block…"
            onClicked: {
                const id = cp.pid, src = cp.block;
                cp.ab.withDraft(d => {
                    const n = src ? Object.assign(L.copy(src), { id: cp.ab.newId() }) : { id: cp.ab.newId(), kind: "words", label: cp.look.words || cp.ab.labelOf(id) };
                    n.kind = n.kind === "pane" || n.kind === "count" || n.kind === "spacer" ? n.kind : n.kind === "action" ? "action" : n.kind;
                    delete n.in; delete n.at; delete n.x; delete n.y; delete n.w; delete n.h;
                    n.label = n.label || cp.ab.blockName(src) || cp.ab.labelOf(id);
                    d.mine = (d.mine || []).concat([n]).slice(-40);
                    if (d.looks[id]) d.looks[n.id] = L.copy(d.looks[id]);
                });
                cp.ab.toast("Saved: it's in Blocks › Your own, to drag in again anywhere.", false);
            }
        }
        Btn { label: "Copy its look"; onClicked: { cp.ab.lookClip = L.lookPart(cp.look); cp.ab.toast("Its look is copied: right-click another part › Paste a look.", false); } }
        Btn {
            label: cp.isBlock ? "Apply to every bar item" : "Apply to every part"
            onClicked: {
                const lk = L.lookPart(cp.look), id = cp.pid, blk = cp.isBlock;
                cp.ab.withDraft(d => {
                    const ids = blk ? d.added.filter(b => b.kind !== "pane" && !!b.in).map(b => b.id) : d.parts.map(p => p.id);
                    for (const x of ids) if (x !== id) d.looks[x] = L.pasteLook(d.looks[x], lk);
                });
            }
        }
    }

    // ---- a list to pick from (an icon, a font, a count, a panel, an action)
    property var pick: null                // {options, x, y, done}
    Rectangle {
        visible: !!cp.pick
        z: 50
        x: cp.pick ? Math.max(10, Math.min(cp.width - width - 10, cp.pick.x)) : 0
        y: cp.pick ? Math.min(cp.height - height - 10, cp.pick.y) : 0
        width: 240; height: Math.min(320, pkCol.implicitHeight + 12)
        radius: 10; color: Theme.panel2; border.color: Theme.line
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
        Flickable {
            anchors { fill: parent; margins: 6 }
            contentHeight: pkCol.implicitHeight; clip: true; boundsBehavior: Flickable.StopAtBounds
            Column {
                id: pkCol
                width: parent.width
                Repeater {
                    model: cp.pick ? cp.pick.options : []
                    delegate: Rectangle {
                        required property var modelData
                        width: pkCol.width; height: 30; radius: 6
                        color: pkM.containsMouse ? Theme.panel3 : "transparent"
                        Row { x: 10; anchors.verticalCenter: parent.verticalCenter; spacing: 8
                            Icon { visible: cp.pick && cp.pick.icons && parent.parent.modelData[0] !== ""; name: parent.parent.modelData[0] || "dot"; size: 14; color: Theme.text; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: parent.parent.modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter } }
                        MouseArea { id: pkM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { const v = parent.modelData[0], p = cp.pick; cp.pick = null; p.done(v); } }
                    }
                }
            }
        }
    }

    // ================================================================== small parts
    component Head: Text { color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2; topPadding: 6 }
    component Line: Item {
        id: ln
        property string label: ""
        default property alias kids: lnRow.data
        width: body.width; height: Math.max(30, lnRow.implicitHeight)
        Text { anchors.verticalCenter: parent.verticalCenter; text: ln.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; width: Math.min(implicitWidth, 170); elide: Text.ElideRight }
        Row { id: lnRow; anchors { right: parent.right; verticalCenter: parent.verticalCenter } spacing: 6 }
    }
    component Seg: Row {
        id: sg
        property var options: []
        property string value: ""
        signal picked(string v)
        spacing: 0
        Repeater {
            model: sg.options
            delegate: Rectangle {
                required property var modelData
                required property int index
                readonly property bool on: sg.value === modelData[0]
                width: st.implicitWidth + 18; height: 28
                radius: 6
                color: on ? Theme.accent : sm.containsMouse ? Theme.panel3 : Theme.bg2
                border.color: on ? Theme.accent : Theme.line
                Text { id: st; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                MouseArea { id: sm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: sg.picked(parent.modelData[0]) }
            }
        }
    }
    component Num: Rectangle {
        id: nm
        property int value: 0
        property string prefix: ""
        property string suffix: ""
        signal commit(int v)
        width: 92; height: 30; radius: 6
        color: Theme.bg; border.color: ni.activeFocus ? Theme.accent : Theme.line
        TextInput {
            id: ni
            anchors { fill: parent; leftMargin: 10; rightMargin: 8 }
            verticalAlignment: TextInput.AlignVCenter
            text: nm.prefix + Math.round(nm.value) + nm.suffix
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
            selectByMouse: true
            onAccepted: { const n = parseInt(text.replace(/[^0-9-]/g, ""), 10); if (!isNaN(n)) nm.commit(n); focus = false; }
            onActiveFocusChanged: if (!activeFocus) text = Qt.binding(() => nm.prefix + Math.round(nm.value) + nm.suffix)
            Keys.onEscapePressed: { focus = false; }
        }
    }
    component Words: Rectangle {
        id: wd
        property string value: ""
        property string hint: ""
        signal commit(string v)
        width: 240; height: 30; radius: 6
        color: Theme.bg; border.color: wi.activeFocus ? Theme.accent : Theme.line
        TextInput {
            id: wi
            anchors { fill: parent; leftMargin: 10; rightMargin: 8 }
            verticalAlignment: TextInput.AlignVCenter
            clip: true
            text: wd.value
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
            selectByMouse: true
            onAccepted: { wd.commit(text); focus = false; }
            onActiveFocusChanged: if (!activeFocus && text !== wd.value) wd.commit(text)
        }
        Text { visible: wi.text === "" && !wi.activeFocus; anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: wd.hint; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    }
    component Slide: Item {
        id: sl
        property real from: 0
        property real to: 100
        property real value: 0
        signal moved(real v)
        width: 160; height: 24
        readonly property real t: Math.max(0, Math.min(1, (value - from) / (to - from)))
        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 4; radius: 2; color: Theme.line }
        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width * sl.t; height: 4; radius: 2; color: Theme.accent }
        Rectangle { x: (parent.width - 16) * sl.t; anchors.verticalCenter: parent.verticalCenter; width: 16; height: 16; radius: 8; color: "#ffffff"; border.color: Theme.accent; border.width: 2 }
        MouseArea {
            anchors.fill: parent
            preventStealing: true
            cursorShape: Qt.PointingHandCursor
            function at(x) { return sl.from + Math.max(0, Math.min(1, x / sl.width)) * (sl.to - sl.from); }
            onPressed: m => sl.moved(at(m.x))
            onPositionChanged: m => { if (pressed) sl.moved(at(m.x)); }
        }
    }
    component Val: Text { color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; width: 60; horizontalAlignment: Text.AlignRight; anchors.verticalCenter: parent ? parent.verticalCenter : undefined }
    component Toggle: Rectangle {
        id: tg
        property bool on: false
        signal flipped()
        width: 40; height: 22; radius: 11
        color: on ? Theme.accent : Theme.bg2; border.color: on ? Theme.accent : Theme.line
        Rectangle { x: tg.on ? parent.width - width - 3 : 3; anchors.verticalCenter: parent.verticalCenter; width: 16; height: 16; radius: 8; color: "#ffffff" }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: tg.flipped() }
    }
    component Colour: Rectangle {
        id: co
        property string value: ""
        signal picked(string v)
        width: 26; height: 26; radius: 6
        color: value || Theme.accent
        border.color: "#ffffff"; border.width: 1
        MouseArea {
            anchors.fill: parent; cursorShape: Qt.PointingHandCursor
            onClicked: {
                const q = co.mapToItem(cp, 0, co.height + 4);
                cp.pick = { x: q.x - 200, y: q.y, options: [["#ff7a2f", "Orange"], ["#5b8cff", "Blue"], ["#2f9e5a", "Green"], ["#b07cff", "Purple"], ["#f2c14e", "Yellow"], ["#e05a6a", "Red"], ["#f06292", "Pink"], ["#f2e8df", "Light"], ["#2b2b30", "Dark"], ["#000000", "Black"], ["#ffffff", "White"]], done: v => co.picked(v) };
            }
        }
    }
    component Pick: Rectangle {
        id: pk
        property string label: ""
        property string icon: ""
        property var options: []
        signal picked(string v)
        width: 180; height: 30; radius: 6
        color: pkm.containsMouse ? Theme.panel3 : Theme.bg2; border.color: Theme.line
        Row { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } spacing: 6
            Rectangle { visible: pk.icon !== ""; width: 16; height: 16; radius: 4; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter
                Icon { anchors.centerIn: parent; name: pk.icon || "dot"; size: 11; color: Theme.accentInk } }
            Text { text: pk.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; width: Math.min(implicitWidth, pk.width - 40); elide: Text.ElideRight; anchors.verticalCenter: parent.verticalCenter } }
        MouseArea {
            id: pkm
            anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
            onClicked: { const q = pk.mapToItem(cp, 0, pk.height + 4); cp.pick = { x: q.x, y: q.y, options: pk.options, icons: pk.icon !== undefined && pk.options.length > 20, done: v => pk.picked(v) }; }
        }
    }
    component Btn: Rectangle {
        id: b
        property string label: ""
        signal clicked()
        width: bt.implicitWidth + 24; height: 34; radius: 8
        color: ba.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
        Text { id: bt; anchors.centerIn: parent; text: b.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
        MouseArea { id: ba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.clicked() }
    }
}
