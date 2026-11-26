// A block's look and what it does, around every block of the main bar (Islands.qml), the extra panels
// (ExtraPanels.qml, PanelBlocks.qml), the panels inside them (NestedPanel.qml) and your own blocks (OwnBarBlock.qml),
// and around the blocks in the desktop editor (DesktopEditor.qml), so what you set there is what the desktop shows.
// Drag and drop board 2 (docs/mockups/drag-drop/drag-drop-2.png): its size and the space around and inside it, its
// fill, see-through, blur behind it, corners, edge and shadow; its icon, words and font; how it looks pointed at and
// pressed; what it shows; what a press, right-click, middle-click, scroll or hover does; which screen and desktops;
// when it shows; how it appears and its bounce when something new comes. The rules are in block-look.js; the look is
// bar.json "s:<block id>" (BarLayout.lookOf). With no look it draws exactly the block, nothing more.
//   BlockLook { bid: "jobs"; Loader { sourceComponent: … } }     the block is its first child
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire
import Quickshell.Services.UPower
import Quickshell.Services.SystemTray
import "block-look.js" as BL
import "widget-tools.js" as T

Item {
    id: bk
    property string bid: ""
    property bool vert: false
    property bool editing: false              // in the desktop editor: nothing it does runs, When doesn't hide it
    property var lookOverride: undefined      // the editor's draft look (null: none); else bar.json's
    property var ownOverride: undefined       // the editor's draft of your own block's definition
    property string screenName: ""
    property real side: 34
    property bool island: false               // its own small island (a bar or panel set to Islands)
    default property alias content: holder.data
    readonly property Item body: holder.children.length > 0 ? holder.children[0] : null

    readonly property string base: T.baseId(bid)
    readonly property var look: BL.normalize(lookOverride !== undefined ? lookOverride : BarLayout.lookOf(bid))
    readonly property bool plain: BL.isDefault(look)
    readonly property var own: {
        if (base.indexOf("own:") !== 0) return null;
        const d = ownOverride !== undefined ? ownOverride : BarLayout.raw["b:" + base.slice(4)];
        return d && typeof d === "object" ? d : null;
    }
    readonly property var meta: {
        if (own) return { icon: own.icon || "dot", name: own.label || "" };
        const m = T.barItem(bid) || T.panelItem(bid);
        return m ? { icon: m.icon || "dot", name: String(m.name || "").replace(/ \(.*\)$/, "") } : { icon: "dot", name: "" };
    }

    // ---- its values ({count} and the rest), only read when the look uses them
    readonly property bool needsValues: look.words.indexOf("{") >= 0 || look.shows !== "own" || look.when === "something" || look.bounce
    PwObjectTracker { objects: bk.needsValues && Pipewire.defaultAudioSink ? [Pipewire.defaultAudioSink] : [] }
    SystemClock { id: clk; enabled: bk.needsValues; precision: SystemClock.Minutes }
    function source() {
        try {
            const sink = Pipewire.defaultAudioSink ? Pipewire.defaultAudioSink.audio : null;
            const b = UPower.displayDevice;
            const f = WinList.activeToplevel;
            const tray = SystemTray.items && SystemTray.items.values ? SystemTray.items.values.length : 0;
            return {
                jobs: NotifyHub.runningJobs.length, jobsPct: NotifyHub.jobsPercent, unseen: NotifyHub.quiet ? 0 : DeskState.unseen,
                vol: sink ? Math.round(sink.volume * 100) : 0, muted: sink ? sink.muted : false,
                batt: b ? Math.round(b.percentage * 100) : 0, net: NetBtWatch.netName,
                title: f ? (f.title || "") : "", app: f ? (f.appId || "") : "", tray: tray,
                printers: NotifyHub.printJobs.length, disks: NotifyHub.disks.length, bg: NotifyHub.backgroundApps.length,
                windows: WinList.toplevels ? WinList.toplevels.values.length : 0, desks: DeskState.deskCount,
                privacy: NotifyHub.cameraApps.length + NotifyHub.micApps.length,
                time: Qt.formatTime(clk.date, Settings.clock24 ? "HH:mm" : "h:mm AP"), date: Qt.formatDate(clk.date, "ddd d MMM")
            };
        } catch (e) { return {}; }
    }
    readonly property var vals: {
        const v = BL.values(base, needsValues ? source() : ({}));
        if (look.shows === "clock") v.clockNow = Qt.formatTime(clk.date, look.clock !== "" ? look.clock : (Settings.clock24 ? "HH:mm" : "h:mm AP"));
        return v;
    }

    // ---- Place and When: whether it shows here now (the host sets visible from this)
    readonly property string frontApp: WinList.activeToplevel ? (WinList.activeToplevel.appId || "") : ""
    readonly property bool shownNow: BL.shownHere(look, {
        editing: editing, screen: screenName, screens: Quickshell.screens.map(s => s.name), desk: DeskState.deskNow,
        app: frontApp, something: needsValues ? BL.something(base, vals) : true
    })
    readonly property bool wantsFill: look.w === "fill"

    // ---- its size: its own, or what you set, and the space around and inside it
    readonly property bool replaced: BL.replaces(base, look)
    readonly property bool labelled: BL.labels(base, look)
    readonly property var shown: BL.content(base, look, vals, meta)
    readonly property real padIn: look.pad >= 0 ? look.pad : 0
    readonly property real bodyW: body ? body.implicitWidth : 0
    readonly property real bodyH: body ? body.implicitHeight : 0
    readonly property real contentW: replaced ? ownRow.implicitWidth : line.implicitWidth
    readonly property real contentH: replaced ? Math.max(ownRow.implicitHeight, bodyH) : line.implicitHeight
    readonly property real innerW: look.w === "fixed" && look.wpx > 0 ? look.wpx : contentW + padIn * 2
    readonly property real innerH: look.h > 0 ? look.h : contentH + padIn * 2
    implicitWidth: plain ? bodyW : innerW + look.ml + look.mr
    implicitHeight: plain ? bodyH : innerH + look.mt + look.mb

    readonly property string accentHex: String(Theme.accent)
    readonly property string edgeHex: look.edgeColor !== "" ? look.edgeColor : accentHex
    readonly property real rad: BL.radius(look, box.height, look.fill !== "" || look.edge !== "none" ? Theme.r(8) : 0)
    readonly property int fontW: look.weight === "light" ? Font.Light : look.weight === "bold" ? Font.Bold : Font.Medium

    // ---- Motion
    property real fadeK: 1
    property bool pressedNow: false
    Timer { id: pressTimer; interval: 180; onTriggered: bk.pressedNow = false }
    function flash() { pressedNow = true; pressTimer.restart(); }
    function appear() {
        if (editing || Settings.reduceMotion || !shownNow) return;
        if (look.appear === "fade") fadeIn.restart();
        else if (look.appear === "slide") slideIn.restart();
        else if (look.appear === "grow") growIn.restart();
    }
    onShownNowChanged: if (shownNow) appear()
    Component.onCompleted: appear()
    NumberAnimation { id: fadeIn; target: bk; property: "fadeK"; from: 0; to: 1; duration: 260; easing.type: Easing.OutCubic }
    NumberAnimation { id: slideIn; target: shift; property: bk.vert ? "x" : "y"; from: 10; to: 0; duration: 260; easing.type: Easing.OutCubic }
    ParallelAnimation {
        id: growIn
        NumberAnimation { target: grow; property: "xScale"; from: 0.6; to: 1; duration: 240; easing.type: Easing.OutBack }
        NumberAnimation { target: grow; property: "yScale"; from: 0.6; to: 1; duration: 240; easing.type: Easing.OutBack }
    }
    // bounce when something new comes (its count goes up)
    readonly property int countNow: parseInt(vals.count) || 0
    property int lastCount: -1
    onCountNowChanged: { if (look.bounce && lastCount >= 0 && countNow > lastCount && !Settings.reduceMotion) bounceAnim.restart(); lastCount = countNow; }
    function bounce() { bounceAnim.restart(); }
    SequentialAnimation {
        id: bounceAnim
        NumberAnimation { target: hop; property: bk.vert ? "x" : "y"; to: -6; duration: 120; easing.type: Easing.OutQuad }
        NumberAnimation { target: hop; property: bk.vert ? "x" : "y"; to: 0; duration: 380; easing.type: Easing.OutBounce }
    }

    // ---- what it does: a press, right-click, middle-click or scroll you changed; a hover
    function which(b) { return b === Qt.RightButton ? "Right" : b === Qt.MiddleButton ? "Middle" : "Press"; }
    function run(a) {
        if (!a || a.kind === "nothing" || editing) return;
        if (a.kind === "app") { const e = AppEntries.byId(a.arg); if (e) { e.execute(); return; } }
        const c = BL.command(a, Quickshell.shellDir);
        if (c) Quickshell.execDetached(c);
    }
    HoverHandler {
        id: hh
        enabled: !bk.editing
        onHoveredChanged: { if (hovered && BL.action(bk.look, "Hover")) dwell.restart(); else dwell.stop(); }
    }
    Timer { id: dwell; interval: 350; onTriggered: bk.run(BL.action(bk.look, "Hover")) }
    Timer { id: scrollGate; interval: 250 }

    Item {
        id: box
        x: bk.plain ? 0 : bk.look.ml
        y: bk.plain ? 0 : bk.look.mt
        width: Math.max(0, bk.width - (bk.plain ? 0 : bk.look.ml + bk.look.mr))
        height: Math.max(0, bk.height - (bk.plain ? 0 : bk.look.mt + bk.look.mb))
        // see-through: its fill when it has one, else the whole block; only on hover: shown while pointed at
        opacity: bk.fadeK * (bk.look.fill === "" && bk.look.see > 0 ? 1 - bk.look.see / 100 : 1)
                 * (bk.look.when === "hover" && !bk.editing && !hh.hovered ? 0 : 1)
        transform: [
            Translate { id: shift },
            Translate { id: hop },
            Scale { id: grow; origin.x: box.width / 2; origin.y: box.height / 2 }
        ]

        // its own island, in the look's style (a bar or panel set to Islands)
        Frame { anchors.fill: parent; anchors.margins: -3; visible: bk.island; rad: Math.max(bk.rad, Theme.r(10)) }
        // shadow (None, Soft, Deep)
        Repeater {
            model: bk.look.shadow === "deep" ? 3 : bk.look.shadow === "soft" ? 2 : 0
            delegate: Rectangle {
                required property int index
                readonly property int spread: index * 2 + 1
                readonly property int off: bk.look.shadow === "deep" ? 4 + index * 2 : 1 + index
                anchors { fill: parent; leftMargin: -spread; rightMargin: -spread; topMargin: off - spread; bottomMargin: -off - spread }
                radius: bk.rad + spread
                color: Qt.rgba(0, 0, 0, bk.look.shadow === "deep" ? 0.2 - index * 0.05 : 0.13 - index * 0.04)
            }
        }
        // blur behind it (the wallpaper behind it, blurred: Frost.qml, the same as Glass's islands)
        Loader {
            anchors.fill: parent
            active: bk.look.blur
            source: "Frost.qml"
            onLoaded: item.rad = Qt.binding(() => bk.rad)
            onStatusChanged: if (status === Loader.Error) console.warn("Mantel: a block's blur couldn't load (Frost.qml)")
        }
        // glow: soft rings outside the edge
        Repeater {
            model: bk.look.edge === "glow" ? 3 : 0
            delegate: Rectangle {
                required property int index
                anchors.fill: parent
                anchors.margins: -(index + 1) * 2
                radius: bk.rad + (index + 1) * 2
                color: "transparent"
                border.width: 2
                border.color: BL.argb(bk.edgeHex, 0.3 - index * 0.08)
            }
        }
        // fill and edge
        Rectangle {
            anchors.fill: parent
            visible: bk.look.fill !== "" || bk.look.edge !== "none"
            radius: bk.rad
            color: bk.look.fill !== "" ? BL.argb(bk.look.fill, 1 - bk.look.see / 100) : "transparent"
            border.width: bk.look.edge === "none" ? 0 : bk.look.edgeW
            border.color: BL.argb(bk.edgeHex, 1)
        }
        // pointed at, pressed
        Rectangle {
            anchors.fill: parent
            radius: Math.max(bk.rad, Theme.r(6))
            readonly property string hov: hh.hovered && !bk.editing ? BL.hoverWash(bk.look, bk.accentHex) : ""
            readonly property string prs: bk.pressedNow ? BL.pressWash(bk.look) : ""
            visible: hov !== "" || prs !== ""
            color: prs !== "" ? prs : hov !== "" ? hov : "transparent"
        }

        // the block itself (with your icon and words before it, for one that draws many things of its own)
        Row {
            id: line
            anchors.centerIn: parent
            spacing: bk.labelled ? 6 : 0
            Row {
                visible: bk.labelled
                spacing: 6
                anchors.verticalCenter: parent.verticalCenter
                Icon { visible: bk.look.icon !== "" && bk.look.show !== "words"; name: bk.look.icon || "dot"; size: bk.look.iconSize > 0 ? bk.look.iconSize : 16; color: bk.look.iconColor !== "" ? bk.look.iconColor : Theme.text; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    visible: bk.look.words !== "" && bk.look.show !== "icon"
                    text: BL.fillWords(bk.look.words, bk.vals)
                    color: Theme.text
                    font.family: bk.look.font !== "" ? bk.look.font : Theme.font
                    font.pixelSize: bk.look.fontSize > 0 ? bk.look.fontSize : 13
                    font.weight: bk.fontW
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
            Item {
                id: holder
                width: bk.bodyW
                height: bk.bodyH
                anchors.verticalCenter: parent.verticalCenter
                // put away for yours (still there underneath, so its own press still works)
                opacity: bk.replaced ? 0 : 1
            }
        }
        // yours in its place: an icon, words (with its values), a count, a clock, a list or a picture
        Row {
            id: ownRow
            visible: bk.replaced
            anchors.centerIn: parent
            spacing: 6
            Icon {
                visible: bk.shown.icon !== ""
                name: bk.shown.icon || "dot"
                size: bk.look.iconSize > 0 ? bk.look.iconSize : Math.max(14, Math.min(20, Math.round(bk.side * 0.5)))
                color: bk.look.iconColor !== "" ? bk.look.iconColor : Theme.text
                anchors.verticalCenter: parent.verticalCenter
            }
            Image {
                visible: bk.shown.pic !== ""
                width: Math.max(16, Math.round(bk.side * 0.8)); height: width
                source: bk.shown.pic !== "" ? (bk.shown.pic.indexOf("/") === 0 ? "file://" + bk.shown.pic : bk.shown.pic) : ""
                fillMode: Image.PreserveAspectFit; asynchronous: true; smooth: true
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                visible: bk.shown.words !== ""
                text: bk.shown.words
                color: Theme.text
                font.family: bk.look.font !== "" ? bk.look.font : Theme.font
                font.pixelSize: bk.look.fontSize > 0 ? bk.look.fontSize : 13
                font.weight: bk.fontW
                anchors.verticalCenter: parent.verticalCenter
            }
        }
        // presses you changed (the rest still go to the block)
        MouseArea {
            anchors.fill: parent
            enabled: !bk.editing && (BL.takes(bk.look) || bk.look.pressed !== "same")
            acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
            onPressed: m => { bk.flash(); if (!BL.action(bk.look, bk.which(m.button))) m.accepted = false; }
            onClicked: m => bk.run(BL.action(bk.look, bk.which(m.button)))
            onWheel: w => {
                const a = BL.action(bk.look, "Scroll");
                if (!a) { w.accepted = false; return; }
                if (!scrollGate.running) { scrollGate.start(); bk.run(a); }
            }
        }
    }
}
