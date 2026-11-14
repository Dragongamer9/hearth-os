//@ pragma UseQApplication
//@ pragma IconTheme Adwaita

// Hearth Desktop Mode: taskbar, wallpaper, app launcher, notifications, Quick Access, the Hearth menu and
// on-screen keyboard, controller support and touch gestures, on top of labwc.
// Started by labwc's autostart: quickshell -p <share>/desktop/shell
// From a terminal or a key binding: quickshell ipc -p <this folder> call <target> <function>
//   launcher toggle|open|close|section <apps|games>   quick toggle   keyboard open|close|press <button>|tap <key>|state   menu open   settings open|page <name>|close
//   tasks open|tab <processes|performance[:cpu|memory|disk|net|gpu]|startup|details|services|settings>|close|toggle   files open <path>|home|toggle|close
//   devices open|close|toggle|scan   run open|close|toggle   winx open|close|toggle (WinTools.qml)
//   resmon open|tab <pc|overview|cpu|memory|disk|network|gpu>|select <part>|close|toggle (ResourceMonitor.qml)
//   calendar toggle|open|close   notifications dnd on|off|toggle, clear   desktop menu|close, icons on|off|toggle
//   taskbar activate <1-9>|showDesktop   brightness up|down   media playPause|next|previous|stop
import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    // Wallpaper, the top islands, the dock and edge gestures on every screen (Mantel 2026).
    // The islands (Islands.qml) and the dock (Dock.qml) are loaded by file name, so a problem in either can't stop the
    // shell: if one can't load, or the screen is upright, the classic taskbar (Bar.qml: launcher, every app, the Game
    // Mode button) comes up instead, so Desktop Mode always has a working bar, launcher and Game Mode button.
    Variants {
        model: Quickshell.screens
        delegate: Component {
            Scope {
                id: perScreen
                required property ShellScreen modelData
                readonly property bool primary: modelData === Quickshell.screens[0]
                readonly property bool portrait: modelData.height > modelData.width
                Wallpaper { screen: perScreen.modelData; primary: perScreen.primary }
                // Desktop icons (main screen) and the desktop's right-click menu.
                Desktop {
                    screen: modelData
                    primary: modelData === Quickshell.screens[0]
                    menu: popMenu
                    dialog: deskDialog
                    settingsApp: settings
                    filesApp: files
                }
                // Made from their files here (not named as types), so a problem in either is caught: both are taken
                // away again and the classic taskbar below comes up instead.
                // Bar layout (Settings › Appearance › Bar and dock): the main bar on its edge (Theme.barEdge) and the
                // floating dock only when the floating app menu is on (Theme.dockFloats); with it off, the main bar
                // holds the apps. A layout change builds both again (live, no restart), so no space stays reserved
                // on the old edge; the classic taskbar is still the fallback for any of them.
                property var islandsWin: null
                property var dockWin: null
                property bool newFailed: false
                property string builtKey: ""
                function makeNew() {
                    dropNew();
                    const wantDock = Theme.dockFloats;
                    const ci = Qt.createComponent("Islands.qml"), cd = wantDock ? Qt.createComponent("Dock.qml") : null;
                    if (ci.status !== Component.Ready || cd && cd.status !== Component.Ready) {
                        console.warn("Mantel: the top islands or the dock couldn't load; using the classic taskbar: " + (ci.errorString() + " " + (cd ? cd.errorString() : "")).trim());
                        newFailed = true;
                        return;
                    }
                    builtKey = Theme.barKey;
                    islandsWin = ci.createObject(perScreen, { screen: perScreen.modelData, primary: perScreen.primary, menu: popMenu, edge: Theme.barEdge, holdsApps: !wantDock });
                    dockWin = wantDock ? cd.createObject(perScreen, { screen: perScreen.modelData, primary: perScreen.primary, menu: popMenu }) : null;
                    if (!islandsWin || wantDock && !dockWin) {
                        console.warn("Mantel: the top islands or the dock couldn't start; using the classic taskbar");
                        dropNew();
                        newFailed = true;
                        return;
                    }
                    newFailed = false;
                    islandsWin.quickOpen = Qt.binding(() => quick.shown);
                    islandsWin.calendarOpen = Qt.binding(() => calendar.shown);
                    islandsWin.launcherRequested.connect(() => launcher.toggle());
                    islandsWin.menuSearchRequested.connect(() => launcher.openMenuSearch());
                    islandsWin.menuQuery = Qt.binding(() => launcher.menuQuery);
                    if (perScreen.primary) { const w = islandsWin; launcher.barSearch = Qt.binding(() => w.menuSearchShown); }
                    islandsWin.quickRequested.connect(() => quick.toggle());
                    islandsWin.keyboardRequested.connect(() => { if (keyboard.open) pad.closeKeyboard(); else pad.openKeyboard(); });
                    islandsWin.calendarRequested.connect(() => calendar.toggle(perScreen.modelData, false));
                    islandsWin.settingsRequested.connect(page => settings.open(page));
                    if (dockWin) {
                        dockWin.keyboardOpen = Qt.binding(() => keyboard.open === true);
                        dockWin.launcherRequested.connect(() => launcher.toggle());
                    }
                }
                function dropNew() {
                    if (perScreen.primary) launcher.barSearch = false;   // the menu shows its own search field again
                    if (islandsWin) islandsWin.destroy();
                    if (dockWin) dockWin.destroy();
                    islandsWin = null;
                    dockWin = null;
                }
                // Built once your settings are read (at most a few seconds, so the bar always comes up), then again
                // whenever the layout changes (a moment after, so a few quick changes build it once).
                property bool settingsReady: Settings.loaded
                Timer { interval: 4000; running: !perScreen.settingsReady; onTriggered: perScreen.settingsReady = true }
                onSettingsReadyChanged: if (settingsReady && !portrait) makeNew()
                Connections {
                    target: Theme
                    function onBarKeyChanged() { if (perScreen.settingsReady && !perScreen.portrait && !perScreen.newFailed && Theme.barKey !== perScreen.builtKey) rebuild.restart(); }
                }
                Timer { id: rebuild; interval: 300; onTriggered: if (!perScreen.portrait && Theme.barKey !== perScreen.builtKey) perScreen.makeNew() }
                onPortraitChanged: { if (portrait) dropNew(); else if (settingsReady) makeNew(); }
                Component.onCompleted: { makeHints(); if (!portrait && settingsReady) makeNew(); }   // the hints first: Console's strip sits at the very bottom, the dock above it
                Component.onDestruction: { dropNew(); if (hintWin) hintWin.destroy(); }
                // The classic taskbar: upright screens, and the safe fallback.
                Loader {
                    active: perScreen.portrait || perScreen.newFailed
                    sourceComponent: Component {
                        Bar {
                            screen: perScreen.modelData
                            primary: perScreen.primary
                            menu: popMenu
                            onSettingsRequested: page => settings.open(page)
                            onLauncherRequested: launcher.toggle()
                            onQuickRequested: quick.toggle()
                            onKeyboardRequested: keyboard.open ? pad.closeKeyboard() : pad.openKeyboard()
                            onCalendarRequested: calendar.toggle(perScreen.modelData, portrait)
                        }
                    }
                }
                // The controller's button hints (Console: always; other looks: after using the controller).
                property var hintWin: null
                function makeHints() {
                    if (!perScreen.primary) return;
                    const ch = Qt.createComponent("HintStrip.qml");
                    if (ch.status === Component.Ready) hintWin = ch.createObject(perScreen, { screen: perScreen.modelData });
                    else console.warn("Mantel: the controller hints couldn't load: " + ch.errorString());
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
        popMenu: popMenu
        onSettingsRequested: settings.open("")
        onPageRequested: p => settings.open(p)
        onMenuRequested: (entry, x, y) => {
            const pinned = DeskState.isPinned(entry.id);
            const list = [{ label: "Open", icon: "open", quick: true, run: () => launcher.launch(entry) }];
            // The app's own actions (New private window...), from its .desktop file.
            const acts = entry.actions ? Array.from(entry.actions).slice(0, 6) : [];
            for (const a of acts) list.push({ label: a.name, icon: "open", group: "Jump to", run: () => { a.execute(); launcher.shown = false; } });
            list.push({ separator: true });
            list.push({ label: pinned ? "Unpin from taskbar" : "Pin to taskbar", short: pinned ? "Unpin" : "Pin", icon: pinned ? "unpin" : "pin", quick: true, run: () => DeskState.togglePin(entry.id) });
            list.push({ label: "Add to desktop", short: "To desktop", icon: "plus", quick: true, run: () => DeskState.call(["desktop-shortcut", entry.id], (r, err) => SettingsNav.toast(err || entry.name + " is on the desktop")) });
            // Apps installed from Flathub: Uninstall… asks, then hearth-flatpak removes it (Bazaar can too).
            if (/\bdistrobox\b/.test(entry.execString || "") && /hearth-debian/.test(entry.execString || "")) {
                // An app from Hearth's Debian box (a .deb): removed in Settings › Apps.
                list.push({ separator: true });
                list.push({ label: "Uninstall…", icon: "trash", danger: true, run: () => { launcher.shown = false; settings.open("apps"); } });
            }
            if (/\bflatpak\b/.test(entry.execString || "")) {
                list.push({ separator: true });
                list.push({ label: "Uninstall…", icon: "trash", danger: true, run: () => {
                    const scr = launcher.screen;
                    launcher.shown = false;
                    deskDialog.confirm(scr, "Uninstall " + entry.name + "?", "Your files stay. An app installed for everyone on this PC is removed for everyone (it may ask for your password).",
                        "Uninstall", () => bazaar.uninstall(entry));
                } });
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
        onPageRequested: p => settings.open(p)
        onLockRequested: lockScreen.lock()
        onPowerRequested: powerMenu.open()
    }
    RadialMenu { id: radial; mouseOn: pad.mouseOn }
    Keyboard { id: keyboard }
    Help { id: help }
    SettingsApp { id: settings }
    // ---- Bazaar (Hearth OS's app store; Hearth's own store, AppStore.qml, is archived: docs/ARCHIVED.md) and
    // clipboard history (Super+V). The IPC target "apps" (older Apps links, keybinds) opens Bazaar; "clipboard" is inside.
    Scope {
        id: bazaar
        readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-flatpak").toString().replace(/^file:\/\//, ""))
        property string removing: ""
        function open() { if (!opener.running) opener.running = true; }
        function uninstall(entry) {
            const m = /\bflatpak\b.*?\brun\b.*?\s([A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+){2,})(\s|$)/.exec(entry.execString || "");
            const id = m ? m[1] : String(entry.id || "").replace(/\.desktop$/, "");
            if (remover.running) { SettingsNav.toast("Still removing " + removing); return; }
            removing = entry.name;
            remover.command = ["python3", helper, "remove", id];
            remover.running = true;
        }
        Process {
            id: opener
            command: ["python3", bazaar.helper, "open-bazaar"]
            stdout: StdioCollector {
                onStreamFinished: {
                    const err = text.split("\n").find(l => l.startsWith("error: "));
                    if (err) SettingsNav.toast(err.slice(7));
                }
            }
        }
        Process {
            id: remover
            stdout: StdioCollector {
                onStreamFinished: {
                    const err = text.split("\n").find(l => l.startsWith("error: "));
                    SettingsNav.toast(err ? "Couldn't uninstall " + bazaar.removing + ": " + err.slice(7) : bazaar.removing + " was uninstalled");
                }
            }
        }
        IpcHandler {
            target: "apps"
            function open(): void { bazaar.open(); }
            function page(name: string): void { bazaar.open(); }
            function app(id: string): void { bazaar.open(); }
            function toggle(): void { bazaar.open(); }
            function close(): void {}
        }
    }
    DebInstaller { id: debInstaller }
    // Clipboard history (Super+V), the volume mixer and snap layouts (Super+Z), Hearth 1.10: loaded by file name, so a
    // problem in one only loses that popup, never the shell.
    Loader { source: "Clipboard.qml" }
    Loader { source: "Mixer.qml" }
    Loader { source: "SnapLayouts.qml" }
    // Discord (Discord.qml): the bar's button, its small window and chooser; IPC targets "discord" and "friends". Loaded
    // separately like MediaKeys, so a problem in it only loses Discord, never the shell.
    Loader { source: "Discord.qml" }
    // "Pick your browser" (1.12): once after updating, Hearth Browser or Google Chrome. Loaded by file name like Discord.
    Loader { source: "BrowserPick.qml" }
    // ---- end Apps / clipboard
    TaskManager { id: tasks }
    // System Restore and Reliability history (docs/SYSTEM-RESTORE.md): loaded separately like Discord, so a problem in
    // either only loses that window. IPC targets "restore" and "reliability" (hearth-restore, hearth-reliability).
    Loader { source: "Restore.qml" }
    Loader { source: "Reliability.qml" }
    // 1.11: the shell watches its own memory and CPU (hearth-shell-health) and refreshes itself only while nobody is
    // looking and none of its own windows (Settings, Task Manager, Files) is open.
    Loader { source: "ShellHealth.qml"; onLoaded: item.inUse = Qt.binding(() => settings.shown || tasks.shown || files.shown) }
    Loader { source: "SpaceWatch.qml"; onLoaded: { item.dialog = deskDialog; item.openStorage = () => settings.open("storage"); } }   // 1.11: main drive almost full
    Loader { source: "BatteryWatch.qml" }      // 1.11: low battery warnings, then hibernate (where proven) or a clean shutdown
    // Device Manager, the Run box (Super+R) and the Super+X menu (WinTools.qml): loaded by file name, so a problem in
    // them only loses those three, never the shell.
    Loader {
        source: "WinTools.qml"
        onLoaded: item.wire({ tasks: tasks, files: files, settings: settings, launcher: launcher, lockScreen: lockScreen })
        onStatusChanged: if (status === Loader.Error) console.warn("Mantel: Device Manager, Run and the Super+X menu (WinTools.qml) couldn't load")
    }
    // Resource Monitor (docs/RESOURCE-MONITOR.md): loaded by file name, so a problem in it only loses that window, never
    // the shell. IPC target "resmon" (hearth-resmon).
    Loader { source: "ResourceMonitor.qml" }
    FileManager { id: files }
    VolumeOsd { id: osd }
    // The media keys (MPRIS) and turning the screen off / sleeping when idle: loaded separately, so a Quickshell
    // without one of these services only loses that.
    Loader { source: "MediaKeys.qml" }
    Loader { source: "IdlePower.qml" }
    PowerMenu { id: powerMenu }
    Lock { id: lockScreen }
    LiveWelcome { id: liveWelcome; settingsApp: settings }
    SteamFallback { id: steamFallback }   // "Steam Game Mode didn't start" with "Try Steam Game Mode again"
    Pad {
        id: pad
        settings: settings
        tasks: tasks
        files: files
        powerMenu: powerMenu
        liveWelcome: liveWelcome
        steamFallback: steamFallback
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
        function menu(): void { tasks.menu(); }
        function search(q: string): void { tasks.search(q); }
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
        // For checks (CI's Desktop Mode test): the ids of the apps it lists, one per line.
        function listed(): string { return launcher.apps.map(a => a.id).join("\n"); }
    }
    // App icons (AppIcons.qml), for checks: what an Icon= value and/or a window's app id resolve to ("" = nothing
    // found), and which desktop entry a window's app id belongs to.
    IpcHandler {
        target: "icons"
        function resolve(icon: string, appId: string): string { return AppIcons.find(icon, appId); }
        function entry(appId: string): string { const e = AppIcons.entryFor(appId); return e ? e.id + " " + (e.icon || "") : ""; }
    }
    // Mantel 2026: an auto-hide dock shown or hidden on purpose.
    IpcHandler {
        target: "dock"
        function show(): void { DeskState.dockPeek = true; }
        function hide(): void { DeskState.dockPeek = false; }
        function toggle(): void { DeskState.dockPeek = !DeskState.dockPeek; }
    }
    IpcHandler {
        target: "quick"
        function toggle(): void { quick.toggle(); }
    }
    IpcHandler {
        target: "keyboard"
        function open(): void { pad.openKeyboard(); }
        function close(): void { pad.closeKeyboard(); }
        // For checks (desktop-test.yml): a controller button pressed and let go (or held), a key tapped by what it
        // types ("a", "space", "bksp", "enter"...), the left stick, a setting (not saved), and what it shows.
        function press(b: string): void { keyboard.padButton(b, true); keyboard.padButton(b, false); }
        function hold(b: string, ms: int): void { keyboard.holdButton(b, ms); }
        function tap(key: string): void { keyboard.tapKey(key); }
        function select(key: string): void { keyboard.tapKey(key, true); }
        function stick(x: real, y: real): void { keyboard.padAxes(x, y, 0, 0); }
        function setting(key: string, value: string): void { keyboard.setSetting(key, value); }
        function state(): string { return keyboard.stateJson(); }
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
        // Checks only (desktop-test.yml): as if the controller was just used, so menus show their A / B hint row.
        function padseen(): void { DeskState.padSeen = Date.now(); }
    }
}
