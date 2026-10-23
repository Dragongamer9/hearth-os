// Hearth Settings › Mouse & touchpad: pointer speed, acceleration, natural scrolling, left-handed buttons, and for a
// touchpad tap to click, scrolling with two fingers or the edge, and ignoring it while you type. Saved by
// hearth-system to ~/.config/nimbus/pointer.json and put in Desktop Mode's labwc settings (labwc reloads them).
// The controller's pointer speed is on the Controller page.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Mouse & touchpad"
    property var st: null                // hearth-system pointer
    property var mouse: ({})
    property var pad: ({})
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    Component.onCompleted: job.run(["pointer"], r => { page.st = r; page.mouse = r.mouse || {}; page.pad = r.touchpad || {}; })
    SetJob { id: job; helper: "hearth-system" }
    function change(dev, key, value) {
        const o = Object.assign({}, dev === "mouse" ? mouse : pad);
        o[key] = value;
        if (dev === "mouse") mouse = o; else pad = o;
        later.restart();
    }
    // Sliders move in small steps: save once they stop.
    Timer {
        id: later
        interval: 600
        onTriggered: job.run(["pointer-set"], r => page.say(r.reloaded ? "" : "Saved. It's used from the next time you sign in to Desktop Mode."),
            e => page.say(e, true), JSON.stringify({ mouse: page.mouse, touchpad: page.pad }), "save")
    }

    SetHeader { text: "Mouse" + (job.what === "save" ? " · saving…" : "") }
    SetSlider {
        icon: "mouse"; title: "Pointer speed"
        value: ((page.mouse.speed || 0) + 1) / 2
        valueText: Math.round(value * 100) + "%"
        onMoved: v => page.change("mouse", "speed", Math.round((v * 2 - 1) * 100) / 100)
    }
    SetSwitch {
        icon: "mouse"; title: "Mouse acceleration"
        sub: "The pointer goes further when you move the mouse quickly. Off is steadier for games"
        checked: page.mouse.accel !== "flat"
        onToggled: v => page.change("mouse", "accel", v ? "adaptive" : "flat")
    }
    SetSwitch {
        icon: "mouse"; title: "Natural scrolling"
        sub: "The page moves the way the wheel turns, like on a phone"
        checked: !!page.mouse.natural
        onToggled: v => page.change("mouse", "natural", v)
    }
    SetSwitch {
        icon: "mouse"; title: "Left-handed"
        sub: "Swaps the left and right buttons"
        checked: !!page.mouse.leftHanded
        onToggled: v => { page.change("mouse", "leftHanded", v); page.change("touchpad", "leftHanded", v); }
    }

    SetHeader { text: "Touchpad"; visible: !!page.st }
    Text {
        visible: !!page.st && !page.st.hasTouchpad
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "No touchpad is connected. These are used when one is."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetSlider {
        visible: !!page.st
        icon: "mouse"; title: "Touchpad speed"
        value: ((page.pad.speed || 0) + 1) / 2
        valueText: Math.round(value * 100) + "%"
        onMoved: v => page.change("touchpad", "speed", Math.round((v * 2 - 1) * 100) / 100)
    }
    SetSwitch {
        visible: !!page.st
        icon: "mouse"; title: "Tap to click"
        sub: "Tap with one finger to click, two fingers to right-click"
        checked: page.pad.tap !== false
        onToggled: v => page.change("touchpad", "tap", v)
    }
    SetSwitch {
        visible: !!page.st
        icon: "mouse"; title: "Natural scrolling"
        sub: "Content moves with your fingers"
        checked: !!page.pad.natural
        onToggled: v => page.change("touchpad", "natural", v)
    }
    SetChoice {
        visible: !!page.st
        title: "Scroll with"
        options: [{ value: "twofinger", label: "Two fingers" }, { value: "edge", label: "The right edge" }]
        current: page.pad.scroll || "twofinger"
        onChosen: v => page.change("touchpad", "scroll", v)
    }
    SetSwitch {
        visible: !!page.st
        icon: "kbd"; title: "Ignore the touchpad while typing"
        checked: page.pad.dwt !== false
        onToggled: v => page.change("touchpad", "dwt", v)
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
