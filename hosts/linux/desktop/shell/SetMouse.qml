// Hearth Settings › Mouse & touchpad: pointer speed, acceleration, natural scrolling, left-handed buttons, and for a
// touchpad tap to click, scrolling with two fingers or the edge, and ignoring it while you type. Saved by
// hearth-system to ~/.config/nimbus/pointer.json and put in Desktop Mode's labwc settings (labwc reloads them).
// With Mantel on KWin the same file is put on KWin's devices (hearth-kwin-settings pointer-apply), and a few more of
// KWin's own show (scroll speed, middle-click, tap-and-drag, how the touchpad clicks, its gestures): under labwc
// they're shown greyed, as needing Mantel on KWin. The controller's pointer speed is on the Controller page.
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
    KwinSettings { id: kw }
    readonly property string kwinNote: kw.live ? "" : ". Needs Mantel on KWin"
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

    SetCardGroup {
        title: "Mouse" + (job.what === "save" ? " · saving…" : "")
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
        SetSlider {
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "mouse"; title: "Scroll speed"
            sub: "How far each turn of the wheel scrolls" + page.kwinNote
            value: ((page.mouse.scrollSpeed || 1) - 0.2) / 2.8
            valueText: (page.mouse.scrollSpeed || 1).toFixed(1) + "×"
            onMoved: v => page.change("mouse", "scrollSpeed", Math.round((0.2 + v * 2.8) * 10) / 10)
        }
        SetSwitch {
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "mouse"; title: "Middle-click with both buttons"
            sub: "Pressing left and right together is a middle click" + page.kwinNote
            checked: !!page.mouse.middle
            onToggled: v => page.change("mouse", "middle", v)
        }
    }
    SetCardGroup {
        title: "Touchpad"
        visible: !!page.st
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
            icon: "mouse"; title: "Scroll with"
            options: [{ value: "twofinger", label: "Two fingers" }, { value: "edge", label: "The right edge" }]
            current: page.pad.scroll || "twofinger"
            onChosen: v => page.change("touchpad", "scroll", v)
        }
        SetSlider {
            visible: !!page.st
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "mouse"; title: "Touchpad scroll speed"
            sub: "How far two fingers scroll" + page.kwinNote
            value: ((page.pad.scrollSpeed || 1) - 0.2) / 2.8
            valueText: (page.pad.scrollSpeed || 1).toFixed(1) + "×"
            onMoved: v => page.change("touchpad", "scrollSpeed", Math.round((0.2 + v * 2.8) * 10) / 10)
        }
        SetSwitch {
            visible: !!page.st
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "mouse"; title: "Touchpad acceleration"
            sub: "The pointer goes further when you move quickly" + page.kwinNote
            checked: page.pad.accel !== "flat"
            onToggled: v => page.change("touchpad", "accel", v ? "adaptive" : "flat")
        }
        SetSwitch {
            visible: !!page.st
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "mouse"; title: "Tap and drag"
            sub: "Tap, then keep the finger down to drag" + page.kwinNote
            checked: page.pad.tapDrag !== false
            onToggled: v => page.change("touchpad", "tapDrag", v)
        }
        SetChoice {
            visible: !!page.st
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "mouse"; title: "Pressing the touchpad down"
            sub: "Right-click with the bottom-right corner, or with two fingers anywhere" + page.kwinNote
            options: [{ value: "areas", label: "Corners" }, { value: "fingers", label: "Fingers" }]
            current: page.pad.click || "areas"
            onChosen: v => page.change("touchpad", "click", v)
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
    // KWin's touchpad gestures (virtualdesktops.cpp, the overview effect): built into KWin, so shown, not changed.
    SetCardGroup {
        title: "Touchpad gestures"
        visible: !!page.st
        SetRow {
            interactive: false
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "grid"; title: "Four fingers up: overview · down: desktop grid"
            sub: (kw.live ? "Turning the overview off in Window effects turns these off" : "Needs Mantel on KWin")
        }
        SetRow {
            interactive: false
            enabled: kw.live; opacity: kw.live ? 1 : 0.45
            icon: "desktops"; title: "Three or four fingers sideways: next or previous desktop"
            sub: kw.live ? "Three fingers up or down move between rows of desktops" : "Needs Mantel on KWin"
        }
        SetLinkCard { target: "kwin-tablet"; enabled: true }
    }
}
