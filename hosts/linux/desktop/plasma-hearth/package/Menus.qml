// The full-screen pieces Mantel's bar opens (plasma-hearth), each in its own layer window over everything, as on
// Mantel: the app menu (Launcher.qml, with the style you picked), the right-click menus (PopMenu.qml), Quick settings
// (QuickAccess.qml) and the clock's dashboard with your notifications (ClockPanel.qml). Made when first opened, so
// nothing of them runs at rest.
import QtQuick

Item {
    id: m
    property var host: null
    property Item anchorItem: null
    readonly property bool launcherShown: launcherW.ready && launcherW.content.shown
    readonly property bool quickShown: firehold ? (fhQuickW.ready && fhQuickW.content.shown === true) : (quickW.ready && quickW.content.shown === true)
    readonly property bool calendarShown: firehold ? (fhListW.ready && fhListW.content.shown === true) : (calendarW.ready && calendarW.content.shown === true)
    readonly property string menuQuery: launcherW.ready ? launcherW.content.menuQuery : ""
    function scr() { return Hearth.screen; }

    Overlay {
        id: popW
        name: "hearth-menu"
        source: "PopMenu.qml"
        wanted: true
    }
    Overlay {
        id: launcherW
        name: "hearth-launcher"
        source: "Launcher.qml"
        onContentChanged: if (content) wire()
        function wire() {
            const l = content;
            l.popMenu = popW.content;
            l.dialog = Qt.binding(() => dialogW.content);
            l.barSearch = Qt.binding(() => Theme.appMenu === "tiles" && !!m.host && m.host.role === "bar");
            l.screen = Qt.binding(() => Hearth.screen);
            l.settingsRequested.connect(() => SettingsNav.openPage(""));
            l.pageRequested.connect(p => SettingsNav.openPage(p));
            l.menuRequested.connect((entry, x, y) => m.appMenuFor(entry, x, y));
        }
    }
    Overlay {
        id: quickW
        name: "hearth-quick"
        source: "QuickAccess.qml"
        onContentChanged: if (content) {
            const q = content;
            q.keyboardRequested.connect(() => Hearth.run(["python3", Hearth.helper, "osk"]));
            q.settingsRequested.connect(() => SettingsNav.openPage(""));
            q.pageRequested.connect(p => SettingsNav.openPage(p));
            q.lockRequested.connect(() => Hearth.run(["loginctl", "lock-session"]));
            q.powerRequested.connect(() => m.openPower());
        }
    }
    Overlay { id: calendarW; name: "hearth-calendar"; source: "ClockPanel.qml" }
    // Firehold on Plasma's style of the shared Quick settings and notification list (their "style" setting).
    Overlay {
        id: fhQuickW
        name: "hearth-firehold-quick"
        source: "FireholdQuick.qml"
        onContentChanged: if (content) content.openNotifications.connect(() => m.toggleCalendar())
    }
    Overlay { id: fhListW; name: "hearth-firehold-notifications"; source: "FireholdHistory.qml" }
    readonly property bool firehold: Hearth.style === "firehold"
    function toggleFh(w) { w.wanted = true; if (w.ready) w.content.toggle(); else w.pending = () => w.content.toggle(); }
    Overlay { id: dialogW; name: "hearth-dialog"; source: "DeskDialog.qml"; wanted: true }
    Overlay {
        id: powerW
        name: "hearth-power"
        source: "PowerMenu.qml"
        wanted: true
    }
    // The volume and brightness bars and the small messages (Caps Lock, keyboard layout): Mantel's, small layer
    // windows at the top of the screen (KDE's own on-screen display is off while Hearth's panel is in use).
    Loader { id: volOsd; active: !!m.host && m.host.role === "bar"; source: "VolumeOsd.qml" }
    Loader { id: sysOsd; active: !!m.host && m.host.role === "bar"; source: "SystemOsd.qml" }
    Connections { target: Buttons; function onBrightnessOsd(pct) { if (volOsd.item) volOsd.item.showBrightness(pct); } }
    Binding { target: Buttons; property: "quiet"; value: m.quickShown }
    function showOsd(v, mu) { if (volOsd.item) volOsd.item.show(v, mu); }
    function showBright(p) { if (volOsd.item) volOsd.item.showBrightness(p); }
    function popMenu() { return popW.content; }
    function deskDialog() { return dialogW.content; }
    // hearth-plasma-panel open close: everything Hearth opened goes away (the power and right-click menus, the app
    // menu, Quick settings, the dashboard), as clicking outside them does.
    function closeAll() {
        if (powerW.ready) powerW.content.shown = false;
        if (popW.ready) popW.content.close();
        if (launcherShown) toggleLauncher();
        if (quickShown) toggleQuick();
        if (calendarShown) toggleCalendar();
    }
    function openPower() { if (powerW.ready && typeof powerW.content.open === "function") powerW.content.open(); }

    function toggleLauncher() { launcherW.wanted = true; if (launcherW.ready) launcherW.content.toggle(); else launcherW.pending = () => launcherW.content.toggle(); }
    function openMenuSearch() { launcherW.wanted = true; if (launcherW.ready) launcherW.content.openMenuSearch(); else launcherW.pending = () => launcherW.content.openMenuSearch(); }
    function toggleQuick() { if (firehold) { toggleFh(fhQuickW); return; } quickW.wanted = true; const f = () => { const q = quickW.content; if (typeof q.toggle === "function") q.toggle(); else q.shown = !q.shown; }; if (quickW.ready) f(); else quickW.pending = f; }
    function toggleCalendar() { if (firehold) { toggleFh(fhListW); return; } calendarW.wanted = true; const f = () => { const c = calendarW.content; if (typeof c.toggle === "function") c.toggle(Hearth.screen, false); else c.shown = !c.shown; }; if (calendarW.ready) f(); else calendarW.pending = f; }

    // The bar's and dock's menus: Mantel's right-click menu (PopMenu.qml), placed on the screen.
    function popProxy(xOff) {
        return {
            get cardWidth() { return popW.ready ? popW.content.cardWidth : 268; },
            popup: function (s, x, y, opts, list) { if (popW.ready) popW.content.popup(m.scr(), x + xOff(), y, opts, list); },
            close: function () { if (popW.ready) popW.content.close(); }
        };
    }

    // An app's right-click menu in the app menu (shell.qml's, as on Mantel).
    function appMenuFor(entry, x, y) {
        const launcher = launcherW.content;
        const pinned = DeskState.isPinned(entry.id);
        const list = [{ label: "Open", icon: "open", quick: true, run: () => launcher.launch(entry) }];
        const acts = entry.actions ? Array.from(entry.actions).slice(0, 6) : [];
        for (const a of acts) list.push({ label: a.name, icon: "open", group: "Jump to", run: () => { a.execute(); launcher.shown = false; } });
        list.push({ separator: true });
        list.push({ label: pinned ? "Unpin from taskbar" : "Pin to taskbar", short: pinned ? "Unpin" : "Pin", icon: pinned ? "unpin" : "pin", quick: true, run: () => DeskState.togglePin(entry.id) });
        list.push({ label: "Add to desktop", short: "To desktop", icon: "plus", quick: true, run: () => DeskState.call(["desktop-shortcut", entry.id], (r, err) => SettingsNav.toast(err || entry.name + " is on the desktop")) });
        list.push({ label: "Edit app…", icon: "edit", run: () => { launcher.shown = false; Hearth.run(["kmenuedit"]); } });
        if (/\bflatpak\b/.test(entry.execString || "")) {
            list.push({ separator: true });
            list.push({ label: "Uninstall…", icon: "trash", danger: true, run: () => { launcher.shown = false; SettingsNav.openPage("apps"); } });
        }
        popW.content.popup(m.scr(), x, y, { title: entry.name }, list);
    }
}
