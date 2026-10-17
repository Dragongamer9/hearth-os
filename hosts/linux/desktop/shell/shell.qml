//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Nimbus Desktop Mode: taskbar, wallpaper, app launcher and notifications on top of labwc.
// Started by labwc's autostart: quickshell -p /usr/local/share/nimbus/desktop/shell
// The launcher can also be opened with: quickshell ipc -p <this folder> call launcher toggle
import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    // One wallpaper and one taskbar per screen.
    Variants {
        model: Quickshell.screens
        delegate: Component {
            Scope {
                required property ShellScreen modelData
                Wallpaper { screen: modelData }
                Bar {
                    screen: modelData
                    onLauncherRequested: launcher.toggle()
                }
            }
        }
    }

    Launcher { id: launcher }
    Notifications {}

    IpcHandler {
        target: "launcher"
        function toggle(): void { launcher.toggle(); }
        function open(): void { launcher.shown = true; }
        function close(): void { launcher.shown = false; }
    }
}
