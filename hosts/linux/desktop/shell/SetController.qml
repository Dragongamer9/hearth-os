// Hearth Settings › Controller: the controller as a mouse, pointer speed, button remapping and the controller
// type games see (through InputPlumber, desktop/shell/hearth-controller; the same settings as Hearth's
// Game Mode), and the button guide.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Controller"
    readonly property var pad: SettingsNav.pad
    property var ctl: null             // hearth-controller status
    property string ctlError: ""
    property bool busy: false
    property string editing: ""        // a button id, "@type", or "" for the main list
    property string pickKind: ""       // "", "button" or "key" while choosing what a button does
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-controller").toString().replace(/^file:\/\//, ""))
    readonly property var editButton: ctl && editing && editing !== "@type" ? ctl.buttons.find(b => b.id === editing) : null

    function label(id) { const t = ctl ? ctl.targets.find(x => x.id === id) : null; return t ? t.label : id; }
    function mapped(id) { return ctl && ctl.config && ctl.config.map ? (ctl.config.map[id] || "") : ""; }
    function typeLabel() { const t = ctl ? ctl.types.find(x => x.id === (ctl.config.target || "")) : null; return t ? t.label : "Automatic"; }
    // One at a time; changes made while one runs (or while the page is still reading the status) wait their turn.
    // They were dropped: a button or controller type picked in the first moments, or two quick changes, didn't stick.
    property var queue: []
    function run(args) {
        if (proc.running) { queue = queue.concat([args]); return; }
        busy = args[0] !== "status";
        proc.command = ["python3", helper].concat(args);
        proc.running = true;
    }
    function edit(id, kind) { editing = id; pickKind = kind || ""; Qt.callLater(page.focusFirst); }
    function done() { const back = editing; editing = ""; pickKind = ""; Qt.callLater(page.focusFirst); }
    Component.onCompleted: run(["status"])
    Process {
        id: proc
        onExited: if (page.queue.length) { const a = page.queue[0]; page.queue = page.queue.slice(1); Qt.callLater(() => page.run(a)); }
        stdout: StdioCollector {
            onStreamFinished: {
                const line = text.trim().split("\n").pop() || "";
                page.busy = false;
                if (line.startsWith("error: ")) { page.ctlError = line.slice(7); if (!page.ctl) page.ctl = null; return; }
                try { page.ctl = JSON.parse(line); page.ctlError = page.ctl.available ? "" : (page.ctl.error || ""); } catch (e) {}
            }
        }
    }

    // ------------------------------------------------------------ main list
    ColumnLayout {
        visible: page.editing === ""
        Layout.fillWidth: true
        spacing: 8

        Text {
            visible: !!(page.pad && page.pad.problem)
            Layout.fillWidth: true
            text: page.pad ? page.pad.problem : ""
            color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        SetSwitch {
            icon: "mouse"; title: "Controller mouse"; sub: "Right stick moves the pointer, R2 clicks. Turns off by itself while a game is full screen."
            checked: !!(page.pad && page.pad.mouseOn)
            onToggled: v => { if (page.pad) page.pad.setMouse(v); }
        }
        SetSlider {
            icon: "mouse"; title: "Pointer speed"
            value: (Settings.pointerSpeed - 0.4) / 1.6         // 0.4 … 2.0 on a 0..1 slider
            valueText: Settings.pointerSpeed.toFixed(1) + "×"
            step: 0.0625
            onMoved: v => Settings.pointerSpeed = Math.round((0.4 + v * 1.6) * 10) / 10
        }

        SetHeader { text: page.busy ? "Button mapping · applying…" : "Button mapping" }
        Text {
            Layout.fillWidth: true
            text: !page.ctl ? (page.ctlError || "Loading…")
                : !page.ctl.available ? "Remapping works on Hearth, the OS, with the built-in controller (InputPlumber)" + (page.ctlError ? ": " + page.ctlError : ".")
                : "Changes apply everywhere, in games too" + (page.ctl.devices.length ? " (" + page.ctl.devices.map(d => d.name).join(", ") + ")." : ".")
                  + (page.ctlError ? "\n" + page.ctlError : "")
            color: page.ctlError ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        SetRow {
            visible: !!(page.ctl && page.ctl.available)
            icon: "pad"; title: "Controller type games see"; sub: "Some games show button prompts to match"
            value: page.typeLabel()
            onActivated: page.edit("@type")
        }
        Repeater {
            model: page.ctl && page.ctl.available ? page.ctl.buttons : []
            delegate: SetRow {
                required property var modelData
                title: modelData.label
                value: { const m = page.mapped(modelData.id); return !m ? "Default" : m === "none" ? "Off" : page.label(m); }
                onActivated: page.edit(modelData.id)
            }
        }
        SetRow {
            visible: !!(page.ctl && page.ctl.available)
            icon: "pad"; title: "Nintendo layout"; sub: "Swaps A with B, and X with Y"; chevron: false
            onActivated: page.run(["preset", "nintendo"])
        }
        SetRow {
            visible: !!(page.ctl && page.ctl.available)
            icon: "restart"; title: "Reset all buttons"; sub: "Back to how the controller came"; chevron: false
            onActivated: page.run(["reset"])
        }

        // Handheld Daemon's own settings (fan curve, power modes, lights, stick ranges...): Hearth's page for them,
        // in a window (the same page as Game Mode's Settings › Controls › Handheld Daemon).
        SetHeader { visible: !!(page.ctl && page.ctl.backend === "hhd"); text: "Handheld Daemon" }
        SetRow {
            visible: !!(page.ctl && page.ctl.backend === "hhd")
            icon: "gear"; title: "Handheld Daemon settings"; sub: "Everything for the built-in controller, power limit, fan and lights. Opens in Hearth."
            onActivated: { Quickshell.execDetached(["nimbus", "--windowed", "--page", "hhd"]); SettingsNav.toast("Opening Handheld Daemon settings"); }
        }

        SetHeader { text: "Help" }
        SetRow { icon: "help"; title: "Controller buttons"; sub: "What each button does in Desktop Mode"; onActivated: { if (SettingsNav.help) SettingsNav.help.shown = true; } }
        SetRow { icon: "kbd"; title: "On-screen keyboard"; sub: "Y on the controller, or here"; onActivated: { if (page.pad) page.pad.openKeyboard(); } }
    }

    // ------------------------------------------------------------ picking (a button's new job, or the type)
    ColumnLayout {
        visible: page.editing !== ""
        Layout.fillWidth: true
        spacing: 8
        SetRow { icon: "back"; title: "Back"; chevron: false; onActivated: page.pickKind ? page.edit(page.editing, "") : page.done() }
        SetHeader { text: page.editing === "@type" ? "Controller type games see" : page.editButton ? page.editButton.label + (page.pickKind === "key" ? " types" : page.pickKind === "button" ? " acts as" : "") : "" }

        // the type
        Repeater {
            model: page.editing === "@type" && page.ctl ? page.ctl.types : []
            delegate: SetRow {
                required property var modelData
                title: modelData.label; sub: modelData.sub; chevron: false
                value: (page.ctl.config.target || "") === modelData.id ? "In use" : ""
                onActivated: { page.run(["type", modelData.id]); page.done(); }
            }
        }
        // what kind of job
        Repeater {
            model: page.editButton && !page.pickKind ? [
                { k: "", icon: "restart", label: "Default (" + page.editButton.label + ")" },
                { k: "button", icon: "pad", label: "Another controller button…" },
                { k: "key", icon: "kbd", label: "A keyboard key…" },
                { k: "none", icon: "close", label: "Nothing (turn it off)" }] : []
            delegate: SetRow {
                required property var modelData
                icon: modelData.icon; title: modelData.label; chevron: modelData.k === "button" || modelData.k === "key"
                onActivated: {
                    if (modelData.k === "button" || modelData.k === "key") page.edit(page.editing, modelData.k);
                    else { page.run(["set", page.editing, modelData.k]); page.done(); }
                }
            }
        }
        // the list
        Repeater {
            model: page.editButton && page.pickKind ? page.ctl.targets.filter(t => t.kind === page.pickKind && t.id !== page.editing) : []
            delegate: SetRow {
                required property var modelData
                title: modelData.label; chevron: false
                value: page.mapped(page.editing) === modelData.id ? "In use" : ""
                onActivated: { page.run(["set", page.editing, modelData.id]); page.done(); }
            }
        }
    }
}
