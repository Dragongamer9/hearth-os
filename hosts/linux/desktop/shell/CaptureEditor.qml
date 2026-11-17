// Hearth Capture's markup editor (Capture.qml loads it by file name, so a problem here leaves Take and Recent working).
// Tools: arrow, box, pen, highlighter, text, blur, pixelate, numbered steps, crop; seven colours, three widths, undo
// and redo. Copy, Save (Pictures › Screenshots, and copied), Save as, Show in folder, Read text (tesseract) and Read
// QR code (zbarimg). The shapes are kept in the picture's own pixels (capture-draw.js): the view draws them scaled
// to fit, and saving draws them 1:1 on a canvas the size of the (cropped) picture, so what you save is full size.
// The picture you took is never changed: saving always makes a new file (Save as never replaces one without asking).
import QtQuick
import QtQuick.Layouts
import Quickshell
import "capture-draw.js" as Draw

Item {
    id: ed
    property var cap: null
    readonly property string src: cap ? cap.shot : ""
    readonly property string url: src !== "" ? "file://" + src : ""
    readonly property int imgW: probe.sourceSize.width
    readonly property int imgH: probe.sourceSize.height
    property var crop: null
    property var shapes: []
    property var live: null
    property var undoStack: []
    property var redoStack: []
    property string tool: "arrow"
    property string color: Draw.COLORS[0]
    property string widthKey: "m"
    property string panel: ""            // "" | text | qr | saveas
    property string ocrState: ""         // busy | done | missing | error
    property string ocrText: ""
    property var qrCodes: []
    property string qrState: ""
    property string qrError: ""
    property string saveAsPath: ""
    property bool saveAsExists: false
    property string toast: ""
    property string lastSaved: ""
    property bool busy: false
    property bool editingText: false
    property real textX: 0
    property real textY: 0

    readonly property var full: ({ x: 0, y: 0, w: Math.max(1, imgW), h: Math.max(1, imgH) })
    readonly property var view: tool === "crop" || !crop ? full : crop
    readonly property real sc: Draw.fit(stage.width - 32, stage.height - 32, view.w, view.h)
    readonly property real lw: Draw.WIDTHS[widthKey] || 6
    readonly property int textSize: widthKey === "s" ? 22 : widthKey === "l" ? 52 : 34

    function start() {
        shapes = []; live = null; undoStack = []; redoStack = []; panel = ""; toast = ""; lastSaved = "";
        crop = cap && cap.crop ? Object.assign({}, cap.crop) : null;
        tool = "arrow"; editingText = false;
        ink.requestPaint();
    }
    Connections { target: ed.cap; function onShotSeqChanged() { ed.start(); } }
    // The picture's pixels for the canvases come from probe (an Image item: drawImage takes one, loaded already).
    readonly property bool ready: probe.status === Image.Ready

    function stateObj() {
        return { tool: tool, shapes: shapes.length, ready: ready, exportError: exportError, crop: crop, width: imgW, height: imgH, panel: panel, ocr: ocrState,
                 text: ocrText.slice(0, 400), qr: qrCodes, qrState: qrState, qrError: qrError, toast: toast, saved: lastSaved, busy: busy };
    }
    function say(t) { toast = t; toastTimer.restart(); }
    Timer { id: toastTimer; interval: 5000; onTriggered: ed.toast = "" }

    // ------------------------------------------------ undo
    function snapshot() { return { shapes: shapes.slice(), crop: crop ? Object.assign({}, crop) : null }; }
    function change(fn) { undoStack = undoStack.concat([snapshot()]).slice(-100); redoStack = []; fn(); ink.requestPaint(); }
    function undo() { if (!undoStack.length) return; redoStack = redoStack.concat([snapshot()]); const s = undoStack[undoStack.length - 1]; undoStack = undoStack.slice(0, -1); shapes = s.shapes; crop = s.crop; ink.requestPaint(); }
    function redo() { if (!redoStack.length) return; undoStack = undoStack.concat([snapshot()]); const s = redoStack[redoStack.length - 1]; redoStack = redoStack.slice(0, -1); shapes = s.shapes; crop = s.crop; ink.requestPaint(); }
    function add(sh) { if (Draw.worthKeeping(sh)) change(() => shapes = shapes.concat([sh])); }

    // A sample of every tool (the screenshots in CI show the editor with them).
    function sample() {
        const W = view.w, H = view.h, X = view.x, Y = view.y;
        change(() => shapes = shapes.concat([
            { t: "box", c: "#ff4d4d", w: 6, x1: X + W * 0.08, y1: Y + H * 0.12, x2: X + W * 0.42, y2: Y + H * 0.42 },
            { t: "arrow", c: "#ff8a3d", w: 6, x1: X + W * 0.62, y1: Y + H * 0.70, x2: X + W * 0.44, y2: Y + H * 0.44 },
            { t: "hl", c: "#ffd166", w: 6, pts: [X + W * 0.5, Y + H * 0.16, X + W * 0.9, Y + H * 0.16] },
            { t: "pen", c: "#4cd97b", w: 4, pts: [X + W * 0.1, Y + H * 0.8, X + W * 0.16, Y + H * 0.72, X + W * 0.22, Y + H * 0.82, X + W * 0.3, Y + H * 0.7] },
            { t: "pix", c: "#ffffff", w: 6, x1: X + W * 0.55, y1: Y + H * 0.25, x2: X + W * 0.92, y2: Y + H * 0.42 },
            { t: "blur", c: "#ffffff", w: 6, x1: X + W * 0.68, y1: Y + H * 0.55, x2: X + W * 0.94, y2: Y + H * 0.9 },
            { t: "text", c: "#ffffff", w: 6, x1: X + W * 0.1, y1: Y + H * 0.46, text: "Click here first", size: 34 },
            { t: "step", c: "#4aa8ff", w: 6, x1: X + W * 0.07, y1: Y + H * 0.1, n: 1 },
            { t: "step", c: "#4aa8ff", w: 6, x1: X + W * 0.64, y1: Y + H * 0.72, n: 2 }
        ]));
    }

    // ------------------------------------------------ saving: the picture drawn 1:1 on outC, as a PNG
    property var exporting: null
    property string exportError: ""
    function exportTo(cb) {
        if (!ready || imgW <= 0) { cb(""); return; }
        const v = crop || full;
        exporting = { cb: cb, v: v };
        outC.width = v.w; outC.height = v.h;
        outC.canvasSize = Qt.size(v.w, v.h);
        outC.requestPaint();
    }
    function finished(cb) {
        // The picture as it is now (a PNG from the canvas), written by the helper to a new file of its own (its
        // temporary folder): the picture taken is never changed.
        busy = true;
        exportTo(d => {
            const b64 = d.indexOf("base64,") >= 0 ? d.slice(d.indexOf("base64,") + 7) : "";
            if (b64.length < 64) { busy = false; exportError = "the canvas gave no picture (" + d.length + ")"; cb(""); return; }
            cap.run(["put"], j => { busy = false; if (!j.ok) exportError = j.error || "put failed"; cb(j.ok ? j.path : ""); }, b64);
        });
    }
    function saveAndCopy() {
        finished(p => {
            if (!p) { say("Couldn't make the picture"); return; }
            cap.run(["save", p], j => {
                if (!j.ok) { say(j.error || "Couldn't save it"); return; }
                lastSaved = j.path; cap.saved(j.path);
                cap.run(["copy", j.path], c => say(c.ok ? "Saved as " + j.name + " and copied" : "Saved as " + j.name));
            });
        });
    }
    function copyOnly() { finished(p => { if (p) cap.run(["copy", p], c => say(c.ok ? "Copied: paste it into any app" : (c.error || "Couldn't copy it"))); }); }
    function openSaveAs() {
        const folder = cap && cap.tools.folder ? cap.tools.folder : (Quickshell.env("HOME") + "/Pictures/Screenshots");
        saveAsPath = folder + "/" + Draw.defaultName(new Date());
        saveAsExists = false; panel = "saveas";
        saveField.forceActiveFocus(); saveField.selectAll();
    }
    function saveAs(replace) {
        const to = saveAsPath.trim();
        if (to === "") return;
        finished(p => {
            if (!p) return;
            cap.run(["save", p, "--to", to].concat(replace ? ["--replace"] : []), j => {
                if (j.exists) { saveAsExists = true; say(j.error); return; }
                if (!j.ok) { say(j.error || "Couldn't save it"); return; }
                lastSaved = j.path; cap.saved(j.path); panel = ""; say("Saved as " + j.name);
            });
        });
    }
    function showFolder() {
        if (lastSaved !== "") { cap.run(["folder", lastSaved]); return; }
        cap.run(["folder", cap.tools.folder || ""]);
    }
    function readText() {
        panel = "text"; ocrState = "busy"; ocrText = "";
        finished(p => {
            if (!p) { ocrState = "error"; return; }
            cap.run(["ocr", p], j => {
                if (j.missing) ocrState = "missing";
                else if (!j.ok) { ocrState = "error"; ocrText = j.error || ""; }
                else { ocrState = "done"; ocrText = j.text || ""; }
            });
        });
    }
    function readQr() {
        panel = "qr"; qrState = "busy"; qrCodes = [];
        finished(p => {
            if (!p) { qrState = "error"; return; }
            cap.run(["qr", p], j => { if (j.missing) qrState = "missing"; else if (!j.ok) { qrState = "error"; qrError = j.error || ""; } else { qrState = "done"; qrCodes = j.codes || []; } });
        });
    }

    // ------------------------------------------------ pointer → picture pixels
    function px(mx) { return view.x + mx / sc; }
    function py(my) { return view.y + my / sc; }
    function commitText() {
        if (editingText && textIn.text.trim() !== "")
            add({ t: "text", c: color, w: lw, x1: textX, y1: textY, text: textIn.text, size: textSize });
        editingText = false; textIn.text = "";
        ink.requestPaint();
    }

    Image { id: probe; visible: false; source: ed.url; cache: false; asynchronous: false }

    Keys.onPressed: e => {
        const ctrl = e.modifiers & Qt.ControlModifier, shift = e.modifiers & Qt.ShiftModifier;
        if (ctrl && e.key === Qt.Key_Z && !shift) undo();
        else if (ctrl && (e.key === Qt.Key_Y || (e.key === Qt.Key_Z && shift))) redo();
        else if (ctrl && e.key === Qt.Key_C) copyOnly();
        else if (ctrl && e.key === Qt.Key_S && shift) openSaveAs();
        else if (ctrl && e.key === Qt.Key_S) saveAndCopy();
        else if (e.key === Qt.Key_Escape && panel !== "") panel = "";
        else return;
        e.accepted = true;
    }

    // ------------------------------------------------ the tools
    Rectangle {
        id: bar
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 60; color: Theme.bg2
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.line }
        Row {
            anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter }
            spacing: 6
            Repeater {
                model: [
                    { id: "arrow", g: "arrow", t: "Arrow" }, { id: "box", g: "box", t: "Box" }, { id: "pen", g: "pen", t: "Pen" },
                    { id: "hl", g: "highlighter", t: "Highlighter" }, { id: "text", g: "text", t: "Text" },
                    { id: "blur", g: "blur", t: "Blur" }, { id: "pix", g: "pixelate", t: "Pixelate" },
                    { id: "step", g: "steps", t: "Numbered steps" }, { id: "crop", g: "crop", t: "Crop" }
                ]
                CaptureBtn {
                    required property var modelData
                    glyph: modelData.g; tip: modelData.t
                    text: ed.tool === modelData.id ? modelData.t : ""
                    selected: ed.tool === modelData.id
                    onActivated: { ed.commitText(); ed.tool = modelData.id; ink.requestPaint(); }
                }
            }
            Rectangle { width: 1; height: 30; color: Theme.line; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: Draw.COLORS
                CaptureBtn {
                    required property string modelData
                    anchors.verticalCenter: parent.verticalCenter
                    swatch: modelData; tip: "Colour"; implicitHeight: 30
                    selected: ed.color === modelData
                    onActivated: ed.color = modelData
                }
            }
            Rectangle { width: 1; height: 30; color: Theme.line; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: [{ k: "s", t: "S" }, { k: "m", t: "M" }, { k: "l", t: "L" }]
                CaptureBtn { required property var modelData; text: modelData.t; tip: "Width"; selected: ed.widthKey === modelData.k; onActivated: ed.widthKey = modelData.k }
            }
            Rectangle { width: 1; height: 30; color: Theme.line; anchors.verticalCenter: parent.verticalCenter }
            CaptureBtn { icon: "undo"; tip: "Undo"; enabledLook: ed.undoStack.length > 0; onActivated: ed.undo() }
            CaptureBtn { glyph: "redo"; tip: "Redo"; enabledLook: ed.redoStack.length > 0; onActivated: ed.redo() }
        }
    }

    // ------------------------------------------------ the picture
    Rectangle {
        id: stage
        anchors { left: parent.left; right: side.left; top: bar.bottom; bottom: parent.bottom }
        color: Theme.bg
        clip: true
        Item {
            id: paper
            width: Math.max(1, ed.view.w * ed.sc); height: Math.max(1, ed.view.h * ed.sc)
            anchors.centerIn: parent
            Rectangle { anchors { fill: parent; margins: -1 } color: "transparent"; border.color: Theme.line }
            Image {
                id: pic
                anchors.fill: parent
                source: ed.url
                sourceClipRect: Qt.rect(ed.view.x, ed.view.y, ed.view.w, ed.view.h)
                smooth: true; mipmap: true; cache: false
            }
            Canvas {
                id: ink
                anchors.fill: parent
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                Connections { target: ed; function onReadyChanged() { ink.requestPaint(); } }
                onPaint: {
                    const c = getContext("2d");
                    c.reset();
                    c.clearRect(0, 0, width, height);
                    const list = ed.live ? ed.shapes.concat([ed.live]) : ed.shapes;
                    Draw.paint(c, list, ed.sc, ed.view.x, ed.view.y,
                               ed.ready ? (sx, sy, sw, sh, dx, dy, dw, dh) => c.drawImage(probe, sx, sy, sw, sh, dx, dy, dw, dh) : null,
                               width, height);
                }
            }
            Connections { target: ed; function onViewChanged() { ink.requestPaint(); } function onScChanged() { ink.requestPaint(); } }
            // Crop: outside the crop dimmed, while the crop tool is on.
            Item {
                anchors.fill: parent
                visible: ed.tool === "crop"
                readonly property var r: ed.live && ed.live.t === "crop" ? Draw.normRect(ed.live.x1, ed.live.y1, ed.live.x2, ed.live.y2) : ed.crop
                readonly property real rx: r ? r.x * ed.sc : 0
                readonly property real ry: r ? r.y * ed.sc : 0
                readonly property real rw: r ? r.w * ed.sc : width
                readonly property real rh: r ? r.h * ed.sc : height
                Rectangle { color: "#99000000"; x: 0; y: 0; width: parent.width; height: parent.ry }
                Rectangle { color: "#99000000"; x: 0; y: parent.ry + parent.rh; width: parent.width; height: parent.height - parent.ry - parent.rh }
                Rectangle { color: "#99000000"; x: 0; y: parent.ry; width: parent.rx; height: parent.rh }
                Rectangle { color: "#99000000"; x: parent.rx + parent.rw; y: parent.ry; width: parent.width - parent.rx - parent.rw; height: parent.rh }
                Rectangle { x: parent.rx; y: parent.ry; width: parent.rw; height: parent.rh; color: "transparent"; border.width: 2; border.color: Theme.accent }
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: ed.tool === "text" ? Qt.IBeamCursor : Qt.CrossCursor
                onPressed: m => {
                    ed.forceActiveFocus();
                    const x = ed.px(m.x), y = ed.py(m.y);
                    if (ed.tool === "text") { ed.commitText(); ed.textX = x; ed.textY = y; ed.editingText = true; textIn.forceActiveFocus(); return; }
                    if (ed.tool === "step") { ed.add({ t: "step", c: ed.color, w: ed.lw, x1: x, y1: y, n: Draw.nextStep(ed.shapes) }); return; }
                    if (ed.tool === "pen" || ed.tool === "hl") ed.live = { t: ed.tool, c: ed.tool === "hl" && ed.color === Draw.COLORS[0] ? "#ffd166" : ed.color, w: ed.lw, pts: [x, y] };
                    else ed.live = { t: ed.tool, c: ed.color, w: ed.lw, x1: x, y1: y, x2: x, y2: y };
                    ink.requestPaint();
                }
                onPositionChanged: m => {
                    if (!pressed || !ed.live) return;
                    const x = Math.max(ed.view.x, Math.min(ed.view.x + ed.view.w, ed.px(m.x))), y = Math.max(ed.view.y, Math.min(ed.view.y + ed.view.h, ed.py(m.y)));
                    const l = Object.assign({}, ed.live);
                    if (l.pts) l.pts = l.pts.concat([x, y]); else { l.x2 = x; l.y2 = y; }
                    ed.live = l;
                    ink.requestPaint();
                }
                onReleased: {
                    const l = ed.live; ed.live = null;
                    if (!l) return;
                    if (l.t === "crop") {
                        const r = Draw.clampRect(Draw.normRect(l.x1, l.y1, l.x2, l.y2), ed.imgW, ed.imgH);
                        if (r.w >= 8 && r.h >= 8) ed.change(() => ed.crop = r);
                    } else ed.add(l);
                    ink.requestPaint();
                }
            }
            // Typing a text: right where you clicked (the on-screen keyboard types here too).
            Rectangle {
                visible: ed.editingText
                x: (ed.textX - ed.view.x) * ed.sc - 4; y: (ed.textY - ed.view.y) * ed.sc - 4
                width: Math.max(160, textIn.contentWidth + 16); height: textIn.contentHeight + 8
                color: "#66000000"; border.color: Theme.accent; border.width: 2; radius: 4
                TextInput {
                    id: textIn
                    x: 4; y: 4
                    width: Math.max(150, contentWidth + 8)
                    color: ed.color
                    font.family: "sans-serif"; font.weight: Font.DemiBold; font.pixelSize: Math.max(10, ed.textSize * ed.sc)
                    onAccepted: ed.commitText()
                    Keys.onEscapePressed: { ed.editingText = false; text = ""; }
                }
            }
        }
        // Crop's own buttons.
        Row {
            visible: ed.tool === "crop"
            anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 14 }
            spacing: 8
            Rectangle {
                height: 38; width: cropTip.implicitWidth + 24; radius: 19; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.94)
                Text { id: cropTip; anchors.centerIn: parent; text: "Drag the part to keep" + (ed.crop ? "  ·  " + ed.crop.w + " × " + ed.crop.h : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
            }
            CaptureBtn { text: "All of it"; visible: !!ed.crop; onActivated: ed.change(() => ed.crop = null) }
            CaptureBtn { text: "Done"; primary: true; onActivated: { ed.tool = "arrow"; ink.requestPaint(); } }
        }
        Rectangle {
            visible: ed.toast !== ""
            anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: 14 }
            height: 40; width: toastT.implicitWidth + 32; radius: 20
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.96); border.color: Theme.line
            Text { id: toastT; anchors.centerIn: parent; text: ed.toast; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
        }
    }

    // ------------------------------------------------ what to do with it
    Rectangle {
        id: side
        anchors { right: parent.right; top: bar.bottom; bottom: parent.bottom }
        width: 300; color: Theme.panel
        Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: 1; color: Theme.line }
        ColumnLayout {
            anchors { fill: parent; margins: 16 }
            spacing: 8
            CaptureBtn { Layout.fillWidth: true; big: true; primary: true; icon: "check"; text: ed.busy ? "Working…" : "Save and copy"; onActivated: ed.saveAndCopy() }
            RowLayout {
                Layout.fillWidth: true; spacing: 8
                CaptureBtn { Layout.fillWidth: true; icon: "copy"; text: "Copy"; onActivated: ed.copyOnly() }
                CaptureBtn { Layout.fillWidth: true; icon: "doc"; text: "Save as…"; onActivated: ed.openSaveAs() }
            }
            CaptureBtn { Layout.fillWidth: true; icon: "folder"; text: "Show in folder"; onActivated: ed.showFolder() }
            Rectangle { Layout.fillWidth: true; Layout.topMargin: 4; height: 1; color: Theme.line }
            RowLayout {
                Layout.fillWidth: true; spacing: 8
                CaptureBtn { Layout.fillWidth: true; glyph: "scan"; text: "Read text"; selected: ed.panel === "text"; enabledLook: !!(ed.cap && ed.cap.tools.tesseract); onActivated: ed.readText() }
                CaptureBtn { Layout.fillWidth: true; glyph: "qr"; text: "QR code"; selected: ed.panel === "qr"; enabledLook: !!(ed.cap && ed.cap.tools.zbar); onActivated: ed.readQr() }
            }
            CaptureBtn { Layout.fillWidth: true; glyph: "area"; text: "New screenshot"; onActivated: ed.cap.take(ed.cap.mode) }

            // Save as: where, typed (the on-screen keyboard opens with Y, or the keyboard button).
            ColumnLayout {
                visible: ed.panel === "saveas"
                Layout.fillWidth: true; Layout.topMargin: 8; spacing: 8
                Text { text: "Save as"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Rectangle {
                    Layout.fillWidth: true; implicitHeight: 64; radius: 8
                    color: Theme.bg2; border.width: saveField.activeFocus ? 2 : 1; border.color: saveField.activeFocus ? Theme.accent : Theme.line
                    TextInput {
                        id: saveField
                        anchors { fill: parent; margins: 8 }
                        text: ed.saveAsPath; onTextEdited: { ed.saveAsPath = text; ed.saveAsExists = false; }
                        wrapMode: TextInput.WrapAnywhere; clip: true
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 13; selectByMouse: true
                        onAccepted: ed.saveAs(false)
                        Keys.onEscapePressed: ed.panel = ""
                    }
                }
                Text { visible: ed.saveAsExists; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "There's a file with that name. Replace it?"; color: Theme.danger; font.family: Theme.font; font.pixelSize: 12 }
                RowLayout {
                    Layout.fillWidth: true; spacing: 8
                    CaptureBtn { icon: "kbd"; tip: "Keyboard"; onActivated: { saveField.forceActiveFocus(); if (SettingsNav.pad) SettingsNav.pad.openKeyboard(); } }
                    Item { Layout.fillWidth: true }
                    CaptureBtn { text: "Cancel"; onActivated: ed.panel = "" }
                    CaptureBtn { primary: true; text: ed.saveAsExists ? "Replace" : "Save"; onActivated: ed.saveAs(ed.saveAsExists) }
                }
            }
            // Read text: what tesseract found.
            ColumnLayout {
                visible: ed.panel === "text"
                Layout.fillWidth: true; Layout.fillHeight: true; Layout.topMargin: 8; spacing: 8
                Text { text: "Text in the picture"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Rectangle {
                    Layout.fillWidth: true; Layout.fillHeight: true; radius: 8; color: Theme.bg2; border.color: Theme.line
                    Flickable {
                        id: ocrFlick
                        anchors { fill: parent; margins: 10 }
                        contentHeight: ocrT.contentHeight; clip: true
                        TextEdit {
                            id: ocrT
                            width: ocrFlick.width; readOnly: true; selectByMouse: true; wrapMode: TextEdit.Wrap
                            text: ed.ocrState === "busy" ? "Reading…"
                                : ed.ocrState === "missing" ? "Reading text needs tesseract (tesseract-ocr), which isn't installed on this PC."
                                : ed.ocrState === "error" ? "Couldn't read it. " + ed.ocrText
                                : ed.ocrText !== "" ? ed.ocrText : "No text found."
                            color: ed.ocrState === "done" && ed.ocrText !== "" ? Theme.text : Theme.muted
                            font.family: Theme.font; font.pixelSize: 13
                        }
                    }
                }
                CaptureBtn { Layout.fillWidth: true; icon: "copy"; text: "Copy the text"; visible: ed.ocrState === "done" && ed.ocrText !== ""; onActivated: ed.cap.run(["copy-text"], j => ed.say(j.ok ? "Text copied" : j.error), ed.ocrText) }
            }
            // QR codes.
            ColumnLayout {
                visible: ed.panel === "qr"
                Layout.fillWidth: true; Layout.topMargin: 8; spacing: 8
                Text { text: "QR codes"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Text {
                    visible: ed.qrState !== "done" || ed.qrCodes.length === 0
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: ed.qrState === "busy" ? "Looking…" : ed.qrState === "missing" ? "Reading QR codes needs zbarimg (zbar-tools), which isn't installed on this PC." : ed.qrState === "error" ? "Couldn't read it. " + ed.qrError : "No QR code in the picture."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
                Repeater {
                    model: ed.qrState === "done" ? ed.qrCodes : []
                    Rectangle {
                        required property string modelData
                        Layout.fillWidth: true; implicitHeight: qc.implicitHeight + 20; radius: 8; color: Theme.bg2; border.color: Theme.line
                        ColumnLayout {
                            id: qc; anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 } spacing: 8
                            Text { Layout.fillWidth: true; wrapMode: Text.WrapAnywhere; text: modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                            Row {
                                spacing: 8
                                CaptureBtn { icon: "copy"; text: "Copy"; onActivated: ed.cap.run(["copy-text"], j => ed.say(j.ok ? "Copied" : j.error), modelData) }
                                CaptureBtn { icon: "open"; text: "Open"; visible: /^(https?:\/\/|mailto:|tel:)/i.test(modelData); onActivated: Quickshell.execDetached(["xdg-open", modelData]) }
                            }
                        }
                    }
                }
            }
            Item { Layout.fillHeight: true; visible: ed.panel !== "text" }
            Text {
                Layout.fillWidth: true; wrapMode: Text.WordWrap
                text: (ed.imgW > 0 ? (ed.crop ? ed.crop.w + " × " + ed.crop.h : ed.imgW + " × " + ed.imgH) + " pixels" : "") + (ed.lastSaved !== "" ? "\nSaved: " + ed.lastSaved.split("/").pop() : "")
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
        }
    }

    // The saving canvas: the (cropped) picture at full size with the shapes 1:1. Off to the side and clipped away,
    // but in the scene, so it paints.
    Item {
        width: 1; height: 1; clip: true; z: -1
        Canvas {
            id: outC
            width: 1; height: 1
            onPaint: {
                const e = ed.exporting;
                if (!e) return;
                const c = getContext("2d");
                c.reset();
                const v = e.v;
                c.drawImage(probe, v.x, v.y, v.w, v.h, 0, 0, v.w, v.h);
                Draw.paint(c, ed.shapes, 1, v.x, v.y, (sx, sy, sw, sh, dx, dy, dw, dh) => c.drawImage(probe, sx, sy, sw, sh, dx, dy, dw, dh), v.w, v.h);
            }
            onPainted: {
                const e = ed.exporting;
                if (!e) return;
                ed.exporting = null;
                e.cb(toDataURL("image/png"));
            }
        }
    }
}
