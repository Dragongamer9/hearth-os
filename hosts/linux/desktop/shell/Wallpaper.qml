// Desktop background: ~/.config/nimbus/wallpaper.jpg (or .png) if you have one,
// otherwise the Nimbus gradient. Sits on the background layer under every window.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    WlrLayershell.layer: WlrLayer.Background
    WlrLayershell.namespace: "nimbus-wallpaper"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Theme.bg

    readonly property string dir: Quickshell.env("HOME") + "/.config/nimbus/"

    // Default: soft accent glow over the dark Nimbus background.
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Vertical
            GradientStop { position: 0.0; color: Theme.bg2 }
            GradientStop { position: 1.0; color: Theme.bg }
        }
    }
    Rectangle {
        width: parent.width * 0.9
        height: width
        radius: width / 2
        x: parent.width * 0.45
        y: -height * 0.55
        color: Theme.accent
        opacity: 0.08
    }

    Image {
        id: jpg
        anchors.fill: parent
        source: "file://" + root.dir + "wallpaper.jpg"
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
    Image {
        anchors.fill: parent
        source: jpg.status === Image.Error ? "file://" + root.dir + "wallpaper.png" : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
}
