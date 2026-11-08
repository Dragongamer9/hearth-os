// A one-color icon from Hearth's own set (icons/), in the light, muted, accent or dark tone.
// Takes the freedesktop names used around the shell (e.g. "input-keyboard-symbolic") or Hearth's own
// ("kbd"). The color picks the tone: Theme.accent → accent, Theme.accentInk → dark, Theme.muted → muted.
// Anything else is an app's own picture (a desktop entry's Icon=): a picture file (an absolute path or file:// URL,
// like Windows apps' icons in ~/.local/share/icons/hearth-winapps) is drawn from that file, and an icon theme name
// from the icon theme (AppIcons.fromIcon), in its own colours. It used to be looked for as icons/<it>-light.svg,
// which never exists ("Cannot open: …/icons/~/.local/share/icons/hearth-winapps/unigine-heaven.png-light.svg").
import QtQuick
import "icon-data.js" as IconData

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
    readonly property string hex: { const t = (x) => ("0" + Math.round(x * 255).toString(16)).slice(-2); return "#" + t(color.r) + t(color.g) + t(color.b); }
    readonly property string tone: Qt.colorEqual(color, Theme.accentInk) ? "ink"
        : Qt.colorEqual(color, Theme.accent) ? "accent"
        : Qt.colorEqual(color, Theme.muted) ? "muted" : "light"

    // Hearth's own icon (icon-data.js has every one of icons/), or else an app's picture: a file or a theme icon.
    readonly property bool own: !!IconData.icons[root.file]
    readonly property string picture: root.own || !root.visible ? "" : AppIcons.fromIcon(root.name)

    Image {
        anchors.fill: parent
        // Drawn in exactly its colour (your accent, light mode's dark text) from icon-data.js; an app's own picture
        // as it is; nothing for a name that is neither (it used to ask for icons/<name>-light.svg, which isn't there).
        // Nothing is loaded while it's hidden: menus keep an Icon next to each app's own picture and show one or the
        // other, and the hidden one asked for "icons/<app id>-light.svg", which doesn't exist (hundreds of "Cannot
        // open" warnings in every log).
        source: !root.visible ? ""
            : root.own ? "data:image/svg+xml;utf8," + encodeURIComponent(IconData.icons[root.file].split("{c}").join(root.hex))
            : root.picture
        sourceSize: Qt.size(Math.ceil(root.size * 2), Math.ceil(root.size * 2))
        smooth: true
        fillMode: Image.PreserveAspectFit
    }
}
