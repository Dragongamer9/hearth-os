//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Hearth Desktop Mode: taskbar, wallpaper, app launcher, notifications, Quick Access, the Hearth menu and
// on-screen keyboard, controller support and touch gestures, on top of labwc.
// Started by labwc's autostart: quickshell -p <share>/desktop/shell
// From a terminal or a key binding: quickshell ipc -p <this folder> call <target> <function>
//   launcher toggle|open|close   quick toggle   keyboard open|close   menu open   settings open|page <name>|close
import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    // Wallpaper, taskbar and edge gestures on every screen.
    Variants {
        model: Quickshell.screens
        delegate: Component {
            Scope {
                required property ShellScreen modelData
                Wallpaper { screen: modelData }
                Bar {
                    screen: modelData
                    onLauncherRequested: launcher.toggle()
                    onQuickRequested: quick.toggle()
                    onKeyboardRequested: keyboard.open ? pad.closeKeyboard() : pad.openKeyboard()
                }
                EdgeSwipes {
                    screen: modelData
                    onSwiped: edge => {
                        if (edge === "right") quick.shown = true;
                        else if (edge === "left") pad.openKeyboard();
                        else launcher.shown = true;
                    }
                }
            }
        }
    }

    Launcher { id: launcher; onSettingsRequested: settings.open("") }
    Notifications {}
    QuickAccess {
        id: quick
        mouseOn: pad.mouseOn
        nightAvailable: night.available
        onMouseToggled: enable => pad.setMouse(enable)
        onKeyboardRequested: pad.openKeyboard()
        onSettingsRequested: settings.open("")
    }
    RadialMenu { id: radial; mouseOn: pad.mouseOn }
    Keyboard { id: keyboard }
    Help { id: help }
    SettingsApp { id: settings }
    VolumeOsd {}
    PowerMenu { id: powerMenu }
    Pad {
        id: pad
        settings: settings
        powerMenu: powerMenu
        launcher: launcher
        quick: quick
        radial: radial
        keyboard: keyboard
        help: help
    }

    // Night light: warm the screen from sunset to sunrise (wlsunset, if installed).
    Scope {
        id: night
        property bool available: false
        Process {
            command: ["sh", "-c", "command -v wlsunset"]
            running: true
            onExited: code => night.available = code === 0
        }
        Process {
            running: night.available && Settings.nightLight
            command: ["wlsunset", "-t", "3700", "-T", "6500", "-S", "07:00", "-s", "19:30", "-d", "1800"]
        }
    }

    Component.onCompleted: { SettingsNav.pad = pad; SettingsNav.help = help; }
    // labwc's key bindings for the volume and power keys (a backup for what hearth-buttons can't read).
    IpcHandler {
        target: "buttons"
        function up(): void { Buttons.volume(1, false); }
        function down(): void { Buttons.volume(-1, false); }
        function mute(): void { Buttons.mute(false); }
        function power(): void { Buttons.power("short", false); }
    }
    IpcHandler {
        target: "power"
        function menu(): void { powerMenu.open(); }
        function close(): void { powerMenu.shown = false; }
    }
    IpcHandler {
        target: "settings"
        function open(): void { settings.open(""); }
        function page(name: string): void { settings.open(name); }
        function close(): void { settings.close(); }
        function toggle(): void { settings.toggle(); }
    }
    IpcHandler {
        target: "launcher"
        function toggle(): void { launcher.toggle(); }
        function open(): void { launcher.shown = true; }
        function close(): void { launcher.shown = false; }
    }
    IpcHandler {
        target: "quick"
        function toggle(): void { quick.toggle(); }
    }
    IpcHandler {
        target: "keyboard"
        function open(): void { pad.openKeyboard(); }
        function close(): void { pad.closeKeyboard(); }
    }
    IpcHandler {
        target: "help"
        function toggle(): void { help.toggle(); }
    }
    IpcHandler {
        target: "menu"
        function open(): void { pad.openMenu(); }
        function close(): void { radial.open = false; }
    }
}
