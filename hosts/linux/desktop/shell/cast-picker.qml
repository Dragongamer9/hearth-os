//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Hearth's screen-sharing picker on KWin (hearth_screencast.py starts it: quickshell -p <this file>): an app (a
// browser, Discord, OBS) asked to share your screen, or to take an interactive screenshot. Shows your screens and
// your windows, each with a picture of what it shows now; you pick one and press Share (or Take for a screenshot).
// Mantel's look (Theme.qml). Keys and the controller: arrows / D-pad move, Tab switches Screens → Windows → Area,
// Enter / A shares, Esc / B cancels. Nothing is shared until you choose.
// Area (1.14, sharing only, when KWin can stream a region): a part of a screen. Draw it on the screen's picture with
// the mouse, or pick one of the ready-made parts (halves, quarters, the middle: 1–6, or [ and ] to go through them), move it with the
// arrows / D-pad and resize it with Shift+arrows; Page Up / Down picks another screen.
//   HEARTH_CAST_REQUEST  {kind: cast|shot, app, screens: [{name, label, w, h, thumb}], windows: [{id, title, app,
//                        thumb, minimized}], desktop (a "Whole desktop" tile, screenshots), area (offer Area)}
//   HEARTH_CAST_RESULT   {kind: screen, name} | {kind: window, id} | {kind: desktop} |
//                        {kind: area, name, fx, fy, fw, fh} (parts of that screen, 0–1) | {cancel: true}
// For checks: quickshell ipc -p <this file> call cast tab screens|windows|area | select <n> | preset <n> |
//             area <fx> <fy> <fw> <fh> | share | cancel | state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
    id: root
    readonly property string reqPath: Quickshell.env("HEARTH_CAST_REQUEST") || ""
    readonly property string resPath: Quickshell.env("HEARTH_CAST_RESULT") || ""
    property var req: ({ kind: "cast", app: "", screens: [], windows: [] })
    property string tab: "screens"
    property int sel: 0
    property bool ending: false
    readonly property bool isShot: req.kind === "shot"
    readonly property var items: {
        if (tab === "windows") return (req.windows || []).map(w => ({ kind: "window", id: w.id, label: w.title, sub: w.app, thumb: w.thumb, app: w.app, minimized: !!w.minimized }));
        const s = (req.screens || []).map(o => ({ kind: "screen", name: o.name, label: o.label, sub: o.w && o.h ? (o.w + " × " + o.h) : o.name, thumb: o.thumb }));
        return req.desktop ? [{ kind: "desktop", label: "Whole desktop", sub: "Every screen", thumb: "" }].concat(s) : s;
    }
    readonly property bool hasScreens: (req.screens || []).length > 0 || !!req.desktop
    readonly property bool hasWindows: (req.windows || []).length > 0
    readonly property bool hasArea: !!req.area && (req.screens || []).length > 0
    // Area: which screen (an index into req.screens) and the part of it (0–1).
    property int areaScreen: 0
    property real fx: 0.25
    property real fy: 0.25
    property real fw: 0.5
    property real fh: 0.5
    readonly property var areaOut: (req.screens || [])[areaScreen] || null
    readonly property var presets: [
        { label: "Left half", r: [0, 0, 0.5, 1] }, { label: "Right half", r: [0.5, 0, 0.5, 1] },
        { label: "Top half", r: [0, 0, 1, 0.5] }, { label: "Bottom half", r: [0, 0.5, 1, 0.5] },
        { label: "Middle", r: [0.2, 0.2, 0.6, 0.6] }, { label: "Top-left quarter", r: [0, 0, 0.5, 0.5] }
    ]
    property int preset: 4
    function setArea(x, y, w, h) {
        w = Math.max(0.05, Math.min(1, w)); h = Math.max(0.05, Math.min(1, h));
        fx = Math.max(0, Math.min(1 - w, x)); fy = Math.max(0, Math.min(1 - h, y)); fw = w; fh = h;
    }
    function usePreset(n) { const p = presets[(n + presets.length) % presets.length]; preset = (n + presets.length) % presets.length; setArea(p.r[0], p.r[1], p.r[2], p.r[3]); }
    readonly property string appName: {
        const a = (req.app || "").replace(/\.desktop$/, "");
        if (!a) return "An app";
        const last = a.split(".").pop();
        return last.charAt(0).toUpperCase() + last.slice(1);
    }

    FileView {
        path: root.reqPath
        onLoaded: {
            try { root.req = JSON.parse(text()); } catch (e) { console.warn("cast-picker: unreadable request", e); root.finish({ cancel: true }); return; }
            root.tab = root.hasScreens ? "screens" : "windows";
            root.sel = 0;
        }
        onLoadFailed: if (root.reqPath) root.finish({ cancel: true })
    }

    Process { id: writer; onExited: Qt.quit() }
    function finish(result) {
        if (ending) return;
        ending = true;
        if (!resPath) return Qt.quit();
        writer.command = ["sh", "-c", 'printf "%s" "$1" > "$2.part" && mv -f "$2.part" "$2"', "sh", JSON.stringify(result), resPath];
        writer.running = true;
    }
    function share() {
        if (tab === "area") {
            if (!areaOut) return;
            finish({ kind: "area", name: areaOut.name, fx: fx, fy: fy, fw: fw, fh: fh });
            return;
        }
        const it = items[sel];
        if (!it) return;
        if (it.kind === "screen") finish({ kind: "screen", name: it.name });
        else if (it.kind === "window") finish({ kind: "window", id: it.id });
        else finish({ kind: "desktop" });
    }
    function setTab(t) {
        if (t === "windows" && !hasWindows) return;
        if (t === "screens" && !hasScreens) return;
        if (t === "area" && !hasArea) return;
        tab = t; sel = 0;
        if (t === "area") usePreset(preset);
    }
    function nextTab(d) {
        const tabs = ["screens", "windows", "area"].filter(t => t === "screens" ? hasScreens : t === "windows" ? hasWindows : hasArea);
        const i = tabs.indexOf(tab);
        setTab(tabs[(i + d + tabs.length) % tabs.length]);
    }

    IpcHandler {
        target: "cast"
        function tab(t: string): void { root.setTab(t); }
        function select(n: int): void { root.sel = Math.max(0, Math.min(root.items.length - 1, n)); }
        function preset(n: int): void { root.usePreset(n); }
        function area(x: real, y: real, w: real, h: real): void { root.setArea(x, y, w, h); }
        function share(): void { root.share(); }
        function cancel(): void { root.finish({ cancel: true }); }
        function state(): string { return JSON.stringify({ tab: root.tab, sel: root.sel, items: root.items.map(i => i.label), kind: root.req.kind, area: root.hasArea ? { screen: root.areaOut ? root.areaOut.name : "", fx: root.fx, fy: root.fy, fw: root.fw, fh: root.fh } : null }); }
    }

    PanelWindow {
        id: win
        anchors { top: true; bottom: true; left: true; right: true }
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-cast-picker"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        color: Qt.rgba(0, 0, 0, 0.55)

        MouseArea { anchors.fill: parent; onClicked: root.finish({ cancel: true }) }

        Rectangle {
            id: card
            anchors.centerIn: parent
            width: Math.min(parent.width - 64, 1180)
            height: Math.min(parent.height - 64, 760)
            radius: Theme.radius + 4
            color: Theme.panel
            border.color: Theme.line
            border.width: 1
            focus: true
            MouseArea { anchors.fill: parent }   // clicks on the card don't cancel

            Keys.onPressed: e => {
                const cols = grid.cols;
                const step = 0.02;
                if (root.tab === "area" && (e.key === Qt.Key_Left || e.key === Qt.Key_Right || e.key === Qt.Key_Up || e.key === Qt.Key_Down)) {
                    const dx = e.key === Qt.Key_Left ? -step : e.key === Qt.Key_Right ? step : 0;
                    const dy = e.key === Qt.Key_Up ? -step : e.key === Qt.Key_Down ? step : 0;
                    if (e.modifiers & Qt.ShiftModifier) root.setArea(root.fx, root.fy, root.fw + dx, root.fh + dy);
                    else root.setArea(root.fx + dx, root.fy + dy, root.fw, root.fh);
                    e.accepted = true;
                    return;
                }
                if (root.tab === "area" && e.key >= Qt.Key_1 && e.key <= Qt.Key_6) { root.usePreset(e.key - Qt.Key_1); e.accepted = true; return; }
                if (root.tab === "area" && (e.key === Qt.Key_BracketLeft || e.key === Qt.Key_BracketRight)) { root.usePreset(root.preset + (e.key === Qt.Key_BracketRight ? 1 : -1)); e.accepted = true; return; }
                if (root.tab === "area" && (e.key === Qt.Key_PageUp || e.key === Qt.Key_PageDown)) {
                    const n = (root.req.screens || []).length;
                    root.areaScreen = (root.areaScreen + (e.key === Qt.Key_PageDown ? 1 : n - 1)) % Math.max(1, n);
                    e.accepted = true;
                    return;
                }
                if (e.key === Qt.Key_Escape) root.finish({ cancel: true });
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) root.share();
                else if (e.key === Qt.Key_Tab) root.nextTab(1);
                else if (e.key === Qt.Key_Backtab) root.nextTab(-1);
                else if (e.key === Qt.Key_Right) root.sel = Math.min(root.items.length - 1, root.sel + 1);
                else if (e.key === Qt.Key_Left) root.sel = Math.max(0, root.sel - 1);
                else if (e.key === Qt.Key_Down) root.sel = Math.min(root.items.length - 1, root.sel + cols);
                else if (e.key === Qt.Key_Up) { if (root.sel < cols) root.setTab("screens"); else root.sel -= cols; }
                else if (e.key === Qt.Key_PageDown) root.setTab("windows");
                else if (e.key === Qt.Key_PageUp) root.setTab("screens");
                else return;
                e.accepted = true;
            }

            ColumnLayout {
                anchors { fill: parent; margins: 28 }
                spacing: 18

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 14
                    Rectangle {
                        Layout.preferredWidth: 46; Layout.preferredHeight: 46; radius: 23
                        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
                        Text { anchors.centerIn: parent; text: root.isShot ? "◳" : "⇪"; color: Theme.accent; font.pixelSize: 24; font.family: Theme.font }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            text: root.isShot ? root.appName + " wants a screenshot" : root.appName + " wants to share your screen"
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 24; font.weight: Font.DemiBold
                        }
                        Text {
                            text: root.isShot ? "Pick what to take a picture of." : "Pick a screen or a window. Nothing is shared until you press Share; stop sharing in the app."
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 15
                        }
                    }
                }

                // Screens | Windows
                Row {
                    spacing: 8
                    Repeater {
                        model: [{ id: "screens", label: "Screens", on: root.hasScreens }, { id: "windows", label: "Windows", on: root.hasWindows }, { id: "area", label: "Area", on: root.hasArea }]
                        Rectangle {
                            required property var modelData
                            visible: modelData.on
                            width: tabText.implicitWidth + 36; height: 40; radius: 20
                            color: root.tab === modelData.id ? Theme.accent : Theme.panel2
                            border.color: root.tab === modelData.id ? Theme.accent : Theme.line
                            Text {
                                id: tabText
                                anchors.centerIn: parent
                                text: modelData.id === "area" ? modelData.label : modelData.label + "  " + (modelData.id === "windows" ? (root.req.windows || []).length : (root.req.screens || []).length + (root.req.desktop ? 1 : 0))
                                color: root.tab === modelData.id ? Theme.accentInk : Theme.text
                                font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium
                            }
                            MouseArea { anchors.fill: parent; onClicked: root.setTab(parent.modelData.id) }
                        }
                    }
                }

                // Area: the screen's picture with the part you're sharing lit; drag on it to draw another part.
                ColumnLayout {
                    visible: root.tab === "area"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 12
                    Flow {
                        Layout.fillWidth: true
                        spacing: 8
                        Repeater {
                            model: root.presets
                            Rectangle {
                                required property var modelData
                                required property int index
                                width: pl.implicitWidth + 28; height: 34; radius: 17
                                color: root.preset === index ? Theme.accent : Theme.panel2
                                border.color: root.preset === index ? Theme.accent : Theme.line
                                Text { id: pl; anchors.centerIn: parent; text: (index + 1) + "  " + modelData.label; color: root.preset === parent.index ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                MouseArea { anchors.fill: parent; onClicked: root.usePreset(parent.index) }
                            }
                        }
                        Repeater {
                            model: (root.req.screens || []).length > 1 ? root.req.screens : []
                            Rectangle {
                                required property var modelData
                                required property int index
                                width: sl.implicitWidth + 28; height: 34; radius: 17
                                color: root.areaScreen === index ? Theme.panel3 : "transparent"
                                border.color: root.areaScreen === index ? Theme.accent : Theme.line
                                Text { id: sl; anchors.centerIn: parent; text: "▭ " + parent.modelData.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                MouseArea { anchors.fill: parent; onClicked: root.areaScreen = parent.index }
                            }
                        }
                    }
                    Item {
                        id: areaBox
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        readonly property real ar: root.areaOut && root.areaOut.w && root.areaOut.h ? root.areaOut.w / root.areaOut.h : 16 / 9
                        Rectangle {
                            id: areaPic
                            anchors.centerIn: parent
                            width: Math.min(parent.width, parent.height * areaBox.ar)
                            height: width / areaBox.ar
                            radius: Theme.radius - 4
                            color: Theme.bg2
                            clip: true
                            Image {
                                anchors.fill: parent
                                source: root.areaOut && root.areaOut.thumb ? "file://" + root.areaOut.thumb : ""
                                sourceSize.width: 1280
                                fillMode: Image.Stretch
                                asynchronous: true
                            }
                            // Dimmed outside the part being shared.
                            Rectangle { x: 0; y: 0; width: parent.width; height: areaSel.y; color: Qt.rgba(0, 0, 0, 0.55) }
                            Rectangle { x: 0; y: areaSel.y + areaSel.height; width: parent.width; height: parent.height - y; color: Qt.rgba(0, 0, 0, 0.55) }
                            Rectangle { x: 0; y: areaSel.y; width: areaSel.x; height: areaSel.height; color: Qt.rgba(0, 0, 0, 0.55) }
                            Rectangle { x: areaSel.x + areaSel.width; y: areaSel.y; width: parent.width - x; height: areaSel.height; color: Qt.rgba(0, 0, 0, 0.55) }
                            Rectangle {
                                id: areaSel
                                x: root.fx * areaPic.width; y: root.fy * areaPic.height
                                width: root.fw * areaPic.width; height: root.fh * areaPic.height
                                color: "transparent"
                                border.width: 3; border.color: Theme.accent
                                Rectangle {
                                    anchors { top: parent.top; left: parent.left; margins: 8 }
                                    width: sz.implicitWidth + 16; height: 26; radius: 13; color: Theme.accent
                                    Text { id: sz; anchors.centerIn: parent; text: root.areaOut ? Math.round(root.fw * root.areaOut.w) + " × " + Math.round(root.fh * root.areaOut.h) : ""; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                                }
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.CrossCursor
                                property real sx: 0
                                property real sy: 0
                                onPressed: m => { sx = m.x / width; sy = m.y / height; }
                                onPositionChanged: m => {
                                    if (!pressed) return;
                                    const ex = Math.max(0, Math.min(1, m.x / width)), ey = Math.max(0, Math.min(1, m.y / height));
                                    root.setArea(Math.min(sx, ex), Math.min(sy, ey), Math.abs(ex - sx), Math.abs(ey - sy));
                                }
                            }
                        }
                    }
                }

                GridView {
                    id: grid
                    visible: root.tab !== "area"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    readonly property int cols: Math.max(1, Math.floor(width / cellWidth))
                    cellWidth: root.tab === "screens" ? Math.floor(width / Math.min(3, Math.max(1, root.items.length))) : Math.floor(width / 4)
                    cellHeight: Math.round(cellWidth * 0.68)
                    model: root.items
                    currentIndex: root.sel
                    onCurrentIndexChanged: positionViewAtIndex(currentIndex, GridView.Contain)
                    delegate: Item {
                        id: cell
                        required property var modelData
                        required property int index
                        width: grid.cellWidth; height: grid.cellHeight
                        readonly property bool picked: index === root.sel
                        Rectangle {
                            anchors { fill: parent; margins: 8 }
                            radius: Theme.radius
                            color: cell.picked ? Theme.panel3 : Theme.panel2
                            border.color: cell.picked ? Theme.accent : Theme.line
                            border.width: cell.picked ? 3 : 1
                            ColumnLayout {
                                anchors { fill: parent; margins: 10 }
                                spacing: 8
                                Rectangle {
                                    Layout.fillWidth: true; Layout.fillHeight: true
                                    radius: Theme.radius - 4
                                    color: Theme.bg2
                                    clip: true
                                    Image {
                                        id: thumb
                                        anchors { fill: parent; margins: 4 }
                                        source: cell.modelData.thumb ? "file://" + cell.modelData.thumb : ""
                                        sourceSize.width: 640
                                        fillMode: Image.PreserveAspectFit
                                        asynchronous: true
                                        smooth: true
                                    }
                                    // No picture (minimized, or the whole desktop): the app's icon or a screen.
                                    AppIcon {
                                        anchors.centerIn: parent
                                        visible: thumb.status !== Image.Ready && cell.modelData.kind === "window"
                                        appId: cell.modelData.app || ""
                                        size: 64
                                    }
                                    Text {
                                        anchors.centerIn: parent
                                        visible: thumb.status !== Image.Ready && cell.modelData.kind !== "window"
                                        text: cell.modelData.kind === "desktop" ? "▣ ▣" : "▭"
                                        color: Theme.muted; font.pixelSize: 54; font.family: Theme.font
                                    }
                                }
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    AppIcon {
                                        visible: cell.modelData.kind === "window"
                                        appId: cell.modelData.app || ""
                                        size: 22
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 0
                                        Text {
                                            Layout.fillWidth: true
                                            text: cell.modelData.label || ""
                                            elide: Text.ElideRight
                                            color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium
                                        }
                                        Text {
                                            Layout.fillWidth: true
                                            text: cell.modelData.minimized ? "Minimized" : (cell.modelData.sub || "")
                                            elide: Text.ElideRight
                                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                        }
                                    }
                                }
                            }
                            MouseArea {
                                anchors.fill: parent
                                onClicked: root.sel = cell.index
                                onDoubleClicked: { root.sel = cell.index; root.share(); }
                            }
                        }
                    }
                    Text {
                        anchors.centerIn: parent
                        visible: root.items.length === 0
                        text: root.tab === "windows" ? "No windows are open" : "KWin didn't list any screens"
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 17
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Text {
                        Layout.fillWidth: true
                        text: root.tab === "area" ? "Drag on the picture, or 1–6: a ready-made part  ·  D-pad: move  ·  Shift+arrows: size  ·  A: share  ·  B: cancel"
                            : "Arrows or D-pad: choose  ·  Tab: screens / windows" + (root.hasArea ? " / area" : "") + "  ·  Enter or A: " + (root.isShot ? "take" : "share") + "  ·  Esc or B: cancel"
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        elide: Text.ElideRight
                    }
                    Rectangle {
                        Layout.preferredWidth: 130; Layout.preferredHeight: 44; radius: 22
                        color: Theme.panel2; border.color: Theme.line
                        Text { anchors.centerIn: parent; text: "Cancel"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 }
                        MouseArea { anchors.fill: parent; onClicked: root.finish({ cancel: true }) }
                    }
                    Rectangle {
                        Layout.preferredWidth: 150; Layout.preferredHeight: 44; radius: 22
                        color: root.items.length ? Theme.accent : Theme.panel3
                        Text { anchors.centerIn: parent; text: root.isShot ? "Take" : "Share"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                        MouseArea { anchors.fill: parent; onClicked: root.share() }
                    }
                }
            }
        }
    }
}
