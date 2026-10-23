//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Hearth Desktop Mode: taskbar, wallpaper, app launcher, notifications, Quick Access, the Hearth menu and
// on-screen keyboard, controller support and touch gestures, on top of labwc.
// Started by labwc's autostart: quickshell -p <share>/desktop/shell
// From a terminal or a key binding: quickshell ipc -p <this folder> call <target> <function>
//   launcher toggle|open|close|section <apps|games>   quick toggle   keyboard open|close   menu open   settings open|page <name>|close
//   tasks open|tab <apps|perf|startup>|close|toggle   files open <path>|home|toggle|close
//   calendar toggle|open|close   notifications dnd on|off|toggle, clear   desktop menu|close, icons on|off|toggle
//   taskbar activate <1-9>|showDesktop   brightness up|down   media playPause|next|previous|stop
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
                // Desktop icons (main screen) and the desktop's right-click menu.
                Desktop {
                    screen: modelData
                    primary: modelData === Quickshell.screens[0]
                    menu: popMenu
                    dialog: deskDialog
                    settingsApp: settings
                    filesApp: files
                }
                Bar {
                    screen: modelData
                    primary: modelData === Quickshell.screens[0]
                    menu: popMenu
                    onSettingsRequested: page => settings.open(page)
                    onLauncherRequested: launcher.toggle()
                    onQuickRequested: quick.toggle()
                    onKeyboardRequested: keyboard.open ? pad.closeKeyboard() : pad.openKeyboard()
                    onCalendarRequested: calendar.toggle(modelData, portrait)
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

    Launcher {
        id: launcher
        onSettingsRequested: settings.open("")
        onMenuRequested: (entry, x, y) => {
            const pinned = DeskState.isPinned(entry.id);
            const list = [{ label: "Open", icon: "open", run: () => launcher.launch(entry) }];
            // The app's own actions (New private window...), from its .desktop file.
            const acts = entry.actions ? Array.from(entry.actions).slice(0, 6) : [];
            for (const a of acts) list.push({ label: a.name, icon: "open", run: () => { a.execute(); launcher.shown = false; } });
            list.push({ separator: true });
            list.push({ label: pinned ? "Unpin from taskbar" : "Pin to taskbar", icon: pinned ? "unpin" : "pin", run: () => DeskState.togglePin(entry.id) });
            list.push({ label: "Add to desktop", icon: "plus", run: () => DeskState.call(["desktop-shortcut", entry.id], (r, err) => SettingsNav.toast(err || entry.name + " is on the desktop")) });
            // Apps installed from Flathub can be removed in Apps (their page has Remove).
            if (/\bflatpak\b/.test(entry.execString || "")) {
                list.push({ separator: true });
                list.push({ label: "Uninstall…", icon: "trash", danger: true, run: () => { launcher.shown = false; appStore.open(entry.id); } });
            }
            popMenu.popup(launcher.screen, x, y, { title: entry.name }, list);
        }
    }
    // ---- Taskbar & desktop: right-click menus, the desktop's small dialogs, the clock's calendar and notifications.
    PopMenu { id: popMenu }
    DeskDialog { id: deskDialog }
    ClockPanel { id: calendar }
    Binding { target: DeskState; property: "overlayShown"; value: popMenu.shown || deskDialog.shown || calendar.shown }
    // ---- end taskbar & desktop
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
    // ---- Apps (Flathub store) and clipboard history (Super+V); their IPC targets "apps" and "clipboard" are inside.
    AppStore { id: appStore }
    Clipboard { id: clipboardHistory }
    // ---- end Apps / clipboard
    TaskManager { id: tasks }
    FileManager { id: files }
    VolumeOsd { id: osd }
    // The media keys (MPRIS) and turning the screen off / sleeping when idle: loaded separately, so a Quickshell
    // without one of these services only loses that.
    Loader { source: "MediaKeys.qml" }
    Loader { source: "IdlePower.qml" }
    PowerMenu { id: powerMenu }
    Lock { id: lockScreen }
    LiveWelcome { id: liveWelcome; settingsApp: settings }
    Pad {
        id: pad
        settings: settings
        tasks: tasks
        files: files
        powerMenu: powerMenu
        liveWelcome: liveWelcome
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
        target: "lock"
        function lock(): void { lockScreen.lock(); }
    }
    IpcHandler {
        target: "brightness"
        function up(): void { osd.brightness(true); }
        function down(): void { osd.brightness(false); }
    }
    IpcHandler {
        target: "taskbar"
        function activate(n: int): void { if (DeskState.mainBar) DeskState.mainBar.activateNth(n); }
        function showDesktop(): void { if (DeskState.mainBar) DeskState.mainBar.toggleShowDesktop(); }
    }
    IpcHandler {
        target: "live"
        function welcome(): void { liveWelcome.shown = true; }
        function install(): void { liveWelcome.choose(); }
        function close(): void { liveWelcome.shown = false; }
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
        target: "tasks"
        function open(): void { tasks.open(""); }
        function tab(name: string): void { tasks.open(name); }
        function close(): void { tasks.close(); }
        function toggle(): void { tasks.toggle(); }
    }
    IpcHandler {
        target: "files"
        function open(path: string): void { files.open(path); }
        function home(): void { files.open(Quickshell.env("HOME")); }
        function toggle(): void { files.toggle(); }
        function close(): void { files.close(); }
    }
    IpcHandler {
        target: "launcher"
        function toggle(): void { launcher.toggle(); }
        function open(): void { launcher.shown = true; }
        function close(): void { launcher.shown = false; }
        // The launcher open at a section: apps or games.
        function section(name: string): void { launcher.openTab(name === "games" ? "games" : "apps"); }
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
    // ---- taskbar & desktop
    IpcHandler {
        target: "calendar"
        function toggle(): void { calendar.toggle(Quickshell.screens[0], false); }
        function open(): void { calendar.open(Quickshell.screens[0], false); }
        function close(): void { calendar.close(); }
    }
    IpcHandler {
        target: "notifications"
        function dnd(state: string): void { DeskState.setDnd(state === "toggle" ? !DeskState.dnd : state === "on"); }
        function clear(): void { DeskState.clearHistory(); }
    }
    IpcHandler {
        target: "desktop"
        function menu(): void { if (DeskState.mainDesktop) DeskState.mainDesktop.backgroundMenu(DeskState.mainDesktop.width / 2, DeskState.mainDesktop.height / 3); }
        function icons(state: string): void { DeskState.setDesktopIcons(state === "toggle" ? !DeskState.desktopIcons : state === "on"); }
        // An icon's right-click menu (by its file name in the Desktop folder).
        function itemMenu(name: string): void {
            const d = DeskState.mainDesktop;
            const e = d ? d.entries.find(x => x.name === name) : null;
            if (e) { const c = d.cellCenter(e); d.itemMenu(e, c.x, c.y); }
        }
        function close(): void { popMenu.close(); deskDialog.cancel(); }
    }
}
