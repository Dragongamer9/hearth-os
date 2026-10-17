// Hearth Settings › Controller: the controller as a mouse, pointer speed, and the button guide.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Controller"
    readonly property var pad: SettingsNav.pad

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
        // 0.4 … 2.0 shown as a 0..1 slider
        value: (Settings.pointerSpeed - 0.4) / 1.6
        valueText: Settings.pointerSpeed.toFixed(1) + "×"
        step: 0.0625
        onMoved: v => Settings.pointerSpeed = Math.round((0.4 + v * 1.6) * 10) / 10
    }
    SetHeader { text: "Help" }
    SetRow { icon: "help"; title: "Controller buttons"; sub: "What each button does in Desktop Mode"; onActivated: { if (SettingsNav.help) SettingsNav.help.shown = true; } }
    SetRow { icon: "kbd"; title: "On-screen keyboard"; sub: "Y on the controller, or here"; onActivated: { if (page.pad) page.pad.openKeyboard(); } }
}
