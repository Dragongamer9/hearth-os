// Hearth Settings › Keyboard: layouts (add, remove, put in order; the first is the one you start with), the
// shortcut that switches between them, how fast a held key repeats, and Desktop Mode's keyboard shortcuts.
// Saved by hearth-system to ~/.config/labwc/environment (XKB_DEFAULT_LAYOUT etc.) and ~/.config/nimbus/keyboard.json;
// labwc reloads them (labwc --reconfigure).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

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
