// Hearth Settings › Bluetooth & devices › Drawing tablet & touchscreen (Mantel on KWin, docs/KWIN-SETTINGS.md): a pen
// tablet's screen and the part of it the tablet covers, which part of the tablet is used, left-handed, the pressure
// curve and range, and what the pen's buttons do; a touchscreen's screen, rotation and calibration. Each device's
// settings go to KWin over D-Bus (org.kde.KWin.InputDevice), which keeps them in kcminputrc; the pen buttons are KWin's
// button rebinds (kcminputrc [ButtonRebinds][TabletTool]).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Drawing tablet & touchscreen"
    KwinSettings { id: kw; onLoaded: page.load() }
    property var devices: []
    property var outs: []
    readonly property var pens: devices.filter(d => d.tabletTool)
    readonly property var pads: devices.filter(d => d.tabletPad)
    readonly property var touches: devices.filter(d => d.touch)
    function load() {
        kw.job.run(["input"], r => page.devices = r.devices || [], e => kw.say(e, true));
        kw.job.run(["outputs"], r => page.outs = (r.outputs || []).filter(o => o.name), () => {});
    }
    function setDev(d, props) {
        // Shown at once; KWin keeps it.
        page.devices = page.devices.map(x => x.sysName === d.sysName ? Object.assign({}, x, props) : x);
        kw.job.run(["input-set", d.sysName], () => {}, e => { kw.say(e, true); page.load(); }, JSON.stringify(props), "save");
    }
    function screenOptions() {
        return [{ value: "", label: "Every screen" }].concat(page.outs.map(o => ({ value: o.name, label: o.name + (o.model ? " · " + o.model : "") })));
    }
    readonly property var areas: [
        { value: "all", label: "All of it", rect: [0, 0, 1, 1] }, { value: "left", label: "Left half", rect: [0, 0, 0.5, 1] },
        { value: "right", label: "Right half", rect: [0.5, 0, 0.5, 1] }, { value: "middle", label: "The middle", rect: [0.25, 0.25, 0.5, 0.5] }]
    function areaOf(r) {
        if (!r || r.length !== 4) return "all";
        const a = page.areas.find(x => x.rect.every((v, i) => Math.abs(v - r[i]) < 0.01));
        return a ? a.value : "custom";
    }
    readonly property var curves: [
        { value: "0,0;1,1;", label: "Even" }, { value: "0,0.35;0.4,1;", label: "Soft (light strokes count more)" },
        { value: "0.35,0;1,0.6;", label: "Firm (press harder)" }]
    readonly property var buttonActions: [
        { value: "", label: "As it is" }, { value: "MouseButton,2", label: "Right-click" }, { value: "MouseButton,4", label: "Middle-click" },
        { value: "MouseButton,8", label: "Back" }, { value: "Key,Ctrl+Z", label: "Undo (Ctrl+Z)" }, { value: "Key,Ctrl+Shift+Z", label: "Redo" },
        { value: "Key,E", label: "Eraser key (E)" }, { value: "Disabled", label: "Nothing" }]
    function rebind(pen, button) {
        const g = (kw.files.kcminputrc || {})["ButtonRebinds][TabletTool][" + pen.name] || {};
        return g[button] || "";
    }
    property var calibrating: null

    KwinGate {
        kw: kw
        what: "Drawing tablet and touchscreen settings"

        // ------------------------------------------------ pen tablets
        SetHeader { text: "Drawing tablet" + (kw.job.what === "save" ? " · saving…" : "") }
        Text {
            visible: page.pens.length === 0
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: kw.running || kw.preview ? "No drawing tablet is connected. Its settings show here when one is." : "Connect a drawing tablet to set it up."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Repeater {
            model: page.pens
            delegate: ColumnLayout {
                id: pen
                required property var modelData
                readonly property var d: modelData
                Layout.fillWidth: true
                spacing: 3
                SetHeader { text: pen.d.name; Layout.topMargin: 8 }
                SetDropdown {
                    icon: "display"; title: "Screen it draws on"
                    options: page.screenOptions()
                    current: pen.d.mapToWorkspace ? "" : (pen.d.outputName || "")
                    onChosen: v => page.setDev(pen.d, v === "" ? { mapToWorkspace: true } : { mapToWorkspace: false, outputName: v })
                }
                SetChoice {
                    visible: pen.d.supportsOutputArea !== false
                    icon: "image"; title: "Part of the screen"
                    options: page.areas.map(a => ({ value: a.value, label: a.label }))
                    current: page.areaOf(pen.d.outputArea)
                    onChosen: v => page.setDev(pen.d, { outputArea: page.areas.find(a => a.value === v).rect })
                }
                SetChoice {
                    visible: !!pen.d.supportsInputArea
                    icon: "edit"; title: "Part of the tablet used"
                    sub: "A smaller part means less arm movement"
                    options: page.areas.map(a => ({ value: a.value, label: a.label }))
                    current: page.areaOf(pen.d.inputArea)
                    onChosen: v => page.setDev(pen.d, { inputArea: page.areas.find(a => a.value === v).rect })
                }
                SetSwitch {
                    visible: !!pen.d.supportsLeftHanded
                    icon: "mouse"; title: "Left-handed (tablet turned round)"
                    checked: !!pen.d.leftHanded
                    onToggled: v => page.setDev(pen.d, { leftHanded: v })
                }
                SetChoice {
                    icon: "edit"; title: "Pressure"
                    options: page.curves
                    current: (page.curves.find(c => c.value === pen.d.pressureCurve) || {}).value || "0,0;1,1;"
                    onChosen: v => page.setDev(pen.d, { pressureCurve: v })
                }
                SetSlider {
                    visible: !!pen.d.supportsPressureRange
                    icon: "edit"; title: "Lightest touch that counts"
                    value: pen.d.pressureRangeMin || 0
                    onMoved: v => page.setDev(pen.d, { pressureRangeMin: Math.min(v, (pen.d.pressureRangeMax || 1) - 0.1) })
                }
                SetSlider {
                    visible: !!pen.d.supportsPressureRange
                    icon: "edit"; title: "Full pressure at"
                    value: pen.d.pressureRangeMax === undefined ? 1 : pen.d.pressureRangeMax
                    onMoved: v => page.setDev(pen.d, { pressureRangeMax: Math.max(v, (pen.d.pressureRangeMin || 0) + 0.1) })
                }
                Repeater {
                    // BTN_STYLUS and BTN_STYLUS2 (linux/input-event-codes.h): the pen's lower and upper buttons.
                    model: [{ code: "331", label: "Pen's lower button" }, { code: "332", label: "Pen's upper button" }]
                    delegate: SetDropdown {
                        required property var modelData
                        icon: "edit"; title: modelData.label
                        options: page.buttonActions
                        current: page.rebind(pen.d, modelData.code)
                        onChosen: v => kw.job.run(["rebind", pen.d.name, modelData.code], () => kw.refresh(), e => kw.say(e, true), v || " ", "save")
                    }
                }
            }
        }
        Text {
            visible: page.pads.length > 0
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "The tablet's own buttons (" + page.pads.map(p => p.name + ": " + (p.tabletPadButtonCount || 0)).join(", ") + ") work as the tablet's maker set them."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }

        // ------------------------------------------------ touchscreens
        SetHeader { text: "Touchscreen" }
        Text {
            visible: page.touches.length === 0
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "No touchscreen found."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }
        Repeater {
            model: page.touches
            delegate: ColumnLayout {
                id: tc
                required property var modelData
                readonly property var d: modelData
                Layout.fillWidth: true
                spacing: 3
                SetHeader { text: tc.d.name; Layout.topMargin: 8 }
                SetSwitch {
                    visible: !!tc.d.supportsDisableEvents
                    icon: "pad"; title: "Use this touchscreen"
                    checked: tc.d.enabled !== false
                    onToggled: v => page.setDev(tc.d, { enabled: v })
                }
                SetDropdown {
                    icon: "display"; title: "Screen it's on"
                    sub: "With more than one screen, touches go to this one"
                    options: page.screenOptions().slice(1)
                    current: tc.d.outputName || ""
                    onChosen: v => page.setDev(tc.d, { outputName: v })
                }
                SetChoice {
                    icon: "sync"; title: "Turned"
                    sub: "If touches land rotated from your finger"
                    options: [{ value: 0, label: "No" }, { value: 1, label: "90° right" }, { value: 2, label: "Upside down" }, { value: 3, label: "90° left" }]
                    current: tc.d.orientationDBus || 0
                    onChosen: v => page.setDev(tc.d, { orientationDBus: v })
                }
                SetRow {
                    visible: !!tc.d.supportsCalibrationMatrix
                    icon: "search"; title: "Calibrate"
                    sub: kw.running ? "Touch four crosses so touches land under your finger" : "Needs KWin running"
                    enabled: kw.running
                    onActivated: {
                        page.setDev(tc.d, { calibrationMatrix: "1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1" });
                        page.calibrating = tc.d;
                    }
                }
                SetRow {
                    visible: !!tc.d.supportsCalibrationMatrix && !!tc.d.calibrationMatrix && tc.d.calibrationMatrix !== tc.d.defaultCalibrationMatrix
                    icon: "undo"; title: "Undo the calibration"
                    onActivated: page.setDev(tc.d, { calibrationMatrix: tc.d.defaultCalibrationMatrix || "1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1" })
                }
            }
        }
    }
    LazyLoader {
        active: page.calibrating !== null
        KwinCalibrate {
            target: Quickshell.screens.find(s => s.name === (page.calibrating || {}).outputName) || Quickshell.screens[0]
            property string before: (page.calibrating || {}).calibrationMatrix || ""
            onDone: m => {
                const d = page.calibrating;
                page.calibrating = null;
                if (d) page.setDev(d, { calibrationMatrix: m || before || d.defaultCalibrationMatrix || "1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1" });
            }
        }
    }
}
