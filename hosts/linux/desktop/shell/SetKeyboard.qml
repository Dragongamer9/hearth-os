// Hearth Settings › Keyboard: layouts (add, remove, put in order; the first is the one you start with), the
// shortcut that switches between them, how fast a held key repeats, Desktop Mode's keyboard shortcuts, and the
// on-screen keyboard (Hearth's shared one, docs/OSK.md: ~/.config/nimbus/osk.json, the same file as Game Mode's
// Settings › Keyboard).
// Saved by hearth-system to ~/.config/labwc/environment (XKB_DEFAULT_LAYOUT etc.) and ~/.config/nimbus/keyboard.json;
// labwc reloads them (labwc --reconfigure).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "osk/hearth-osk.js" as Osk

SetPage {
    id: page
    title: "Keyboard"
    property var st: null                // hearth-system keyboard
    property var layouts: []             // [{layout, variant, name}] in order
    property var all: []                 // every layout and variant
    property var shortcuts: []
    property int picked: -1              // the layout whose buttons are showing
    property string switchKey: ""
    property int delay: 600
    property int rate: 25
    property string message: ""
    property bool messageBad: false
    readonly property var switchChoices: [
        { value: "", label: "None" }, { value: "grp:alt_shift_toggle", label: "Alt+Shift" },
        { value: "grp:ctrl_shift_toggle", label: "Ctrl+Shift" }, { value: "grp:alt_space_toggle", label: "Alt+Space" },
        { value: "grp:caps_toggle", label: "Caps Lock" }]
    readonly property string query: addSearch.text.trim().toLowerCase()
    readonly property var addMatches: query === "" ? [] : all.filter(l => (l.name + " " + l.layout + " " + l.variant).toLowerCase().includes(query)
        && !layouts.some(x => x.layout === l.layout && x.variant === l.variant)).slice(0, 30)

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        job.run(["keyboard"], r => {
            page.st = r;
            page.layouts = r.layouts || [];
            page.switchKey = r.switch || "";
            page.delay = r.delay || 600;
            page.rate = r.rate || 25;
        });
    }
    Component.onCompleted: {
        load();
        job.run(["keyboard-layouts"], r => page.all = r.layouts || []);
        job.run(["shortcuts"], r => page.shortcuts = r.shortcuts || []);
    }
    SetJob { id: job; helper: "hearth-system" }
    function apply() {
        const req = { layouts: layouts.map(l => ({ layout: l.layout, variant: l.variant })), switch: switchKey, delay: delay, rate: rate };
        job.run(["keyboard-set"], r => page.say(r.reloaded ? "Saved and in use." : "Saved. It's used from the next time you sign in to Desktop Mode."),
            e => page.say(e, true), JSON.stringify(req), "save");
    }
    // Sliders move in small steps: save once they stop.
    Timer { id: later; interval: 900; onTriggered: page.apply() }
    function setLayouts(list) { layouts = list; picked = -1; apply(); }
    function move(i, d) {
        const j = i + d;
        if (j < 0 || j >= layouts.length) return;
        const l = layouts.slice();
        const t = l[i]; l[i] = l[j]; l[j] = t;
        setLayouts(l);
        Qt.callLater(() => { const it = layoutList.itemAt(j); if (it) it.focusRow(); });
    }

    SetHeader { text: "Layouts" + (job.what === "save" ? " · saving…" : "") }
    Repeater {
        id: layoutList
        model: page.layouts
        delegate: ColumnLayout {
            id: lay
            required property var modelData
            required property int index
            Layout.fillWidth: true
            spacing: 8
            function focusRow() { row.forceActiveFocus(); }
            SetRow {
                id: row
                icon: "kbd"
                title: lay.modelData.name || lay.modelData.layout
                sub: lay.index === 0 ? "Used when Desktop Mode starts" : "Switch to it with " + (page.switchChoices.find(c => c.value === page.switchKey) || { label: "the shortcut below" }).label
                value: page.picked === lay.index ? "" : lay.index === 0 ? "First" : ""
                onActivated: page.picked = page.picked === lay.index ? -1 : lay.index
            }
            RowLayout {
                visible: page.picked === lay.index
                Layout.fillWidth: true
                Layout.leftMargin: 52
                spacing: 8
                SetRow { visible: lay.index > 0; title: "Move up"; chevron: false; onActivated: page.move(lay.index, -1) }
                SetRow { visible: lay.index < page.layouts.length - 1; title: "Move down"; chevron: false; onActivated: page.move(lay.index, 1) }
                SetRow {
                    visible: page.layouts.length > 1
                    title: "Remove"; danger: true; chevron: false
                    onActivated: { page.setLayouts(page.layouts.filter((x, i) => i !== lay.index)); Qt.callLater(() => page.focusFirst()); }
                }
            }
        }
    }
    SetField { id: addSearch; placeholder: "Add a layout: type a language or country" }
    Repeater {
        model: page.addMatches
        delegate: SetRow {
            required property var modelData
            icon: "kbd"; chevron: false
            title: modelData.name
            sub: modelData.layout + (modelData.variant ? " · " + modelData.variant : "")
            value: "Add"
            onActivated: {
                if (page.layouts.length >= 4) return page.say("Up to 4 layouts.", true);
                page.setLayouts(page.layouts.concat([{ layout: modelData.layout, variant: modelData.variant, name: modelData.name }]));
                addSearch.text = "";
                Qt.callLater(() => { const it = layoutList.itemAt(page.layouts.length - 1); if (it) it.focusRow(); });
            }
        }
    }
    SetChoice {
        visible: page.layouts.length > 1
        title: "Switch layouts with"; sub: "Press it to go to the next layout"
        options: page.switchChoices
        current: page.switchKey
        onChosen: v => { page.switchKey = v; page.apply(); }
    }

    SetHeader { text: "Typing" }
    SetSlider {
        icon: "kbd"; title: "Wait before a held key repeats"
        value: (page.delay - 150) / 850
        valueText: page.delay + " ms"
        onMoved: v => { page.delay = Math.round((150 + v * 850) / 10) * 10; later.restart(); }
    }
    SetSlider {
        icon: "kbd"; title: "Repeat speed"
        value: (page.rate - 5) / 55
        valueText: page.rate + " per second"
        onMoved: v => { page.rate = Math.round(5 + v * 55); later.restart(); }
    }
    SetField { placeholder: "Try typing here" }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ------------------------------------------------ the on-screen keyboard
    property var osk: ({})
    function oskGet(k) { return osk[k] !== undefined ? osk[k] : Osk.HearthOSK.DEFAULTS[k]; }
    function oskSet(k, v) { const o = Object.assign({}, osk); o[k] = v; osk = o; oskFile.setText(JSON.stringify(o, null, 2) + "\n"); }
    function oskLayouts() { const l = oskGet("layouts"); return Array.isArray(l) && l.length ? l : ["auto"]; }
    function oskToggleLayout(id) {
        let l = oskLayouts().slice();
        if (l.indexOf(id) >= 0) { if (l.length > 1) l = l.filter(x => x !== id); } else l.push(id);
        oskSet("layouts", l);
    }
    FileView {
        id: oskFile
        path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/osk.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { const o = JSON.parse(text()); if (o && typeof o === "object") page.osk = o; } catch (e) {} }
    }
    FileView { id: oskWords; path: (Quickshell.env("XDG_DATA_HOME") || (Quickshell.env("HOME") + "/.local/share")) + "/nimbus/osk-words.json" }

    SetHeader { text: "On-screen keyboard" }
    SetRow { icon: "kbd"; title: "Try the on-screen keyboard"; sub: "Y on the controller, the keyboard button on the taskbar, or a swipe in from the left edge"; onActivated: { if (SettingsNav.pad) SettingsNav.pad.openKeyboard(); } }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Its layouts (the globe key switches; Automatic follows the layouts above)"
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: [{ id: "auto", name: "Automatic" }].concat(Osk.HearthOSK.LAYOUTS)
        delegate: SetSwitch {
            required property var modelData
            title: modelData.name
            checked: page.oskLayouts().indexOf(modelData.id) >= 0
            onToggled: page.oskToggleLayout(modelData.id)
        }
    }
    SetSwitch { title: "Word suggestions"; sub: "Words to pick above the keys, from a word list on this device"; checked: page.oskGet("suggestions"); onToggled: on => page.oskSet("suggestions", on) }
    SetSwitch { title: "Learn new words"; sub: "Remembers words you type (never passwords), only on this device"; checked: page.oskGet("learn"); onToggled: on => page.oskSet("learn", on) }
    SetSwitch { title: "Capital letters"; sub: "At the start of a sentence"; checked: page.oskGet("autocap"); onToggled: on => page.oskSet("autocap", on) }
    SetSwitch { title: "Double space types a period"; checked: page.oskGet("doubleSpace"); onToggled: on => page.oskSet("doubleSpace", on) }
    SetChoice {
        title: "Fast typing"; sub: "Ember Wheel: tilt the left stick at a group of letters, A B X Y type one of its four. Split sticks: each stick points into its half, LT and RT type."
        options: [{ value: "wheel", label: "Ember Wheel" }, { value: "split", label: "Split sticks" }, { value: "off", label: "Off" }]
        current: page.oskGet("fast"); onChosen: v => page.oskSet("fast", v)
    }
    SetChoice { title: "Look"; options: Osk.HearthOSK.THEMES.map(t => ({ value: t[0], label: t[1] })); current: page.oskGet("theme"); onChosen: v => page.oskSet("theme", v) }
    SetChoice { title: "Size"; options: [{ value: "s", label: "Small" }, { value: "m", label: "Medium" }, { value: "l", label: "Large" }]; current: page.oskGet("size"); onChosen: v => page.oskSet("size", v) }
    SetChoice { title: "Shape"; sub: "Split puts the halves at the edges, under your thumbs on a handheld's touchscreen"; options: [{ value: "full", label: "Full" }, { value: "split", label: "Split" }]; current: page.oskGet("shape"); onChosen: v => page.oskSet("shape", v) }
    SetChoice { title: "Position"; options: [{ value: "bottom", label: "Bottom" }, { value: "top", label: "Top" }]; current: page.oskGet("position"); onChosen: v => page.oskSet("position", v) }
    SetSwitch { title: "Ember trail"; sub: "A warm glow follows the selection"; checked: page.oskGet("trail"); onToggled: on => page.oskSet("trail", on) }
    SetChoice { title: "Rumble on key presses"; options: [{ value: "off", label: "Off" }, { value: "light", label: "Light" }, { value: "strong", label: "Strong" }]; current: page.oskGet("haptics"); onChosen: v => page.oskSet("haptics", v) }
    SetRow { title: "Clear learned words"; sub: "Word suggestions go back to the word list (Game Mode's keyboard forgets them too)"; danger: true; chevron: false; onActivated: { oskWords.setText("{}"); page.say("Learned words cleared."); } }

    SetHeader { text: "Keyboard shortcuts"; visible: page.shortcuts.length > 0 }
    Repeater {
        model: page.shortcuts
        delegate: SetRow {
            required property var modelData
            chevron: false
            title: modelData.keys
            sub: modelData.action
        }
    }
}
