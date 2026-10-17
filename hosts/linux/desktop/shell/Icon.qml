// A one-color icon from Hearth's own set (icons/), in the light, muted, accent or dark tone.
// Takes the freedesktop names used around the shell (e.g. "input-keyboard-symbolic") or Hearth's own
// ("kbd"). The color picks the tone: Theme.accent → accent, Theme.accentInk → dark, Theme.muted → muted.
import QtQuick

Item {
    id: root
    property string name
    property color color: Theme.text
    property real size: 20
    implicitWidth: size
    implicitHeight: size

    readonly property var names: ({
        "open-menu-symbolic": "menu", "input-keyboard-symbolic": "kbd", "input-gaming-symbolic": "pad",
        "view-app-grid-symbolic": "grid", "window-close-symbolic": "close", "camera-photo-symbolic": "cam",
        "input-mouse-symbolic": "mouse", "help-about-symbolic": "help",
        "audio-volume-high-symbolic": "vol", "audio-volume-medium-symbolic": "vol", "audio-volume-low-symbolic": "vol-low",
        "audio-volume-muted-symbolic": "vol-mute",
        "network-wireless-symbolic": "wifi", "network-wireless-signal-good-symbolic": "wifi",
        "network-wired-symbolic": "wired", "network-offline-symbolic": "offline",
        "battery-good-symbolic": "batt", "battery-low-symbolic": "batt-low", "battery-caution-symbolic": "batt-low",
        "battery-good-charging-symbolic": "batt-charging",
        "display-brightness-symbolic": "sun", "night-light-symbolic": "moon", "weather-clear-night-symbolic": "sleep",
        "emblem-system-symbolic": "gear"
    })
    readonly property string file: names[name] || name || "dot"
    readonly property string tone: Qt.colorEqual(color, Theme.accentInk) ? "ink"
        : Qt.colorEqual(color, Theme.accent) ? "accent"
        : Qt.colorEqual(color, Theme.muted) ? "muted" : "light"

    Image {
        anchors.fill: parent
        source: Qt.resolvedUrl("icons/" + root.file + "-" + root.tone + ".svg")
        sourceSize: Qt.size(Math.ceil(root.size * 2), Math.ceil(root.size * 2))
        smooth: true
        fillMode: Image.PreserveAspectFit
    }
}
