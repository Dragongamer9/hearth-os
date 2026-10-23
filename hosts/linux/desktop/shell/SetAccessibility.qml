// Hearth Settings › Accessibility: the magnifier (labwc's own: Super+= / Super+- / Super+Esc), colour filters,
// the screen reader (Orca, Super+Alt+S), and bigger text, high contrast and a bigger pointer. Saved by hearth-access
// in ~/.config/nimbus/accessibility.json, which Game Mode's Settings › Accessibility shares (its zoom and colour
// filters are gamescope's; Desktop Mode can only invert colours, see hearth-access).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Accessibility"
    property var st: null                // hearth-access status
    readonly property var s: st ? st.settings : ({})
    readonly property var av: st ? st.available : ({})
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    Component.onCompleted: job.run(["status"], r => page.st = r, e => page.say(e, true))
    SetJob { id: job; helper: "hearth-access" }

    // Changes one or more settings: shown at once, saved (and applied) by hearth-access.
    property var pending: ({})
    function change(o, now) {
        const next = Object.assign({}, page.s, o);
        page.st = Object.assign({}, page.st, { settings: next });
        page.pending = Object.assign({}, page.pending, o);
        if (now) save.triggered(); else save.restart();
    }
    Timer {
        id: save
        interval: 500
        onTriggered: {
            const o = page.pending;
            page.pending = ({});
            if (!Object.keys(o).length) return;
            job.run(["set", JSON.stringify(o)], r => {
                page.st = r;
                const p = r.problems || {};
                const keys = Object.keys(p);
                page.say(keys.length ? p[keys[0]] : "", keys.length > 0);
            }, e => page.say(e, true), "", "save");
        }
    }
    readonly property var filterNames: ({ none: "Off", grayscale: "Greyscale", invert: "Inverted colours",
        "invert-brightness": "Inverted brightness", protanopia: "Red-green (protanopia)",
        deuteranopia: "Green-red (deuteranopia)", tritanopia: "Blue-yellow (tritanopia)" })
    readonly property var filterSubs: ({ grayscale: "Shows everything in shades of grey", invert: "Light becomes dark and colours flip",
        "invert-brightness": "Light becomes dark; colours stay much the same",
        protanopia: "Moves reds where they're easier to tell apart", deuteranopia: "Moves greens where they're easier to tell apart",
        tritanopia: "Moves blues and yellows where they're easier to tell apart" })

    // ------------------------------------------------ magnifier
    SetHeader { text: "Magnifier" + (job.what === "save" ? " · saving…" : "") }
    Text {
        visible: !!page.st && !page.av.labwcMagnifier
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "This labwc (" + (page.av.labwc || "unknown version") + ") has no magnifier; it came in labwc 0.7.3."
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "search"; title: "Zoom in now"
        sub: "Super+= zooms in, Super+- zooms out (and off), Super+Esc turns it off or back on. The view follows the pointer"
        chevron: false
        onActivated: job.run(["magnifier", "in"], () => {}, e => page.say(e, true))
    }
    SetChoice {
        title: "What it magnifies"
        options: [{ value: "full", label: "The whole screen" }, { value: "lens", label: "A box around the pointer" }]
        current: page.s.magnifierSize || "full"
        onChosen: v => page.change({ magnifierSize: v })
    }
    SetSlider {
        icon: "search"; title: "Zoom when Super+Esc turns it on"
        readonly property real lo: 1.25
        readonly property real hi: 8
        value: ((page.s.magnifierZoom || 2) - lo) / (hi - lo)
        step: 0.25 / (hi - lo)
        valueText: (page.s.magnifierZoom || 2).toFixed(2).replace(/0$/, "") + "×"
        onMoved: v => page.change({ magnifierZoom: Math.round((lo + v * (hi - lo)) * 4) / 4 })
    }
    SetChoice {
        title: "Each Super+= or Super+- step"
        options: [{ value: 0.25, label: "Small" }, { value: 0.5, label: "Medium" }, { value: 1, label: "Big" }]
        current: page.s.magnifierStep || 0.5
        onChosen: v => page.change({ magnifierStep: v })
    }

    // ------------------------------------------------ colour filters
    SetHeader { text: "Colour filters" }
    Repeater {
        model: page.av.filters || ["none", "grayscale", "invert", "invert-brightness", "protanopia", "deuteranopia", "tritanopia"]
        delegate: SetSwitch {
            required property var modelData
            visible: modelData !== "none"
            readonly property bool here: (page.av.desktopFilters || ["none", "invert"]).indexOf(modelData) >= 0
            icon: "palette"; title: page.filterNames[modelData] || modelData
            sub: (page.filterSubs[modelData] || "") + (here ? "" : ". Game Mode only: Desktop Mode's window manager can't show it yet")
            checked: page.s.filter === modelData
            onToggled: v => page.change({ filter: v ? modelData : "none" }, true)
        }
    }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "In Desktop Mode colours are inverted with the screen's colour ramp, so Night light has to be off for it. Every filter works in Game Mode."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    // ------------------------------------------------ screen reader
    SetHeader { text: "Screen reader" }
    SetSwitch {
        icon: "vol"; title: "Screen reader (Orca)"
        sub: !page.st ? "Checking…" : !page.av.orca ? "Orca isn't installed" : "Reads out what's on the screen and what you type. Super+Alt+S turns it on or off"
        checked: !!page.s.screenReader
        onToggled: v => page.change({ screenReader: v }, true)
    }

    // ------------------------------------------------ text, contrast, pointer
    SetHeader { text: "Text and pointer" }
    SetChoice {
        title: "Text size"
        sub: "For apps and window titles. Apps open now use it after you sign out and in again"
        options: [{ value: 1, label: "Default" }, { value: 1.15, label: "Larger" }, { value: 1.3, label: "Large" }, { value: 1.5, label: "Largest" }]
        current: page.s.textScale || 1
        onChosen: v => page.change({ textScale: v }, true)
    }
    SetSwitch {
        icon: "sun"; title: "High contrast"
        sub: "Apps with GTK use their high-contrast look (after signing out and in)"
        checked: !!page.s.highContrast
        onToggled: v => page.change({ highContrast: v }, true)
    }
    SetSwitch {
        icon: "mouse"; title: "Large pointer"
        sub: "A pointer twice the size (after signing out and in)"
        checked: !!page.s.largePointer
        onToggled: v => page.change({ largePointer: v }, true)
    }

    // ------------------------------------------------ Game Mode
    SetHeader { text: "In Game Mode" }
    SetSlider {
        icon: "pad"; title: "Zoom"
        value: ((page.s.gameZoom || 1) - 1) / 3
        step: 0.25 / 3
        valueText: (page.s.gameZoom || 1) <= 1 ? "Off" : (page.s.gameZoom || 1).toFixed(2).replace(/0$/, "") + "×"
        onMoved: v => page.change({ gameZoom: Math.round((1 + v * 3) * 4) / 4 })
    }
    SetRow {
        icon: "kbd"; title: "Emoji and symbols"; sub: "Super+. opens the picker, in any app"; chevron: false
        onActivated: Quickshell.execDetached(["hearth-emoji"])
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
