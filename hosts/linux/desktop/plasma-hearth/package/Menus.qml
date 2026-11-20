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
    readonly property bool quickShown: quickW.ready && quickW.content.shown === true
    readonly property bool calendarShown: calendarW.ready && calendarW.content.shown === true
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
            // Mantel's power menu comes to Plasma with stage 2; until then KDE's own (Lock, Sleep, Restart, Shut down).
            q.powerRequested.connect(() => Hearth.run(["dbus-send", "--session", "--dest=org.kde.LogoutPrompt", "--type=method_call", "/LogoutPrompt", "org.kde.LogoutPrompt.promptAll"]));
        }
    }
    Overlay { id: calendarW; name: "hearth-calendar"; source: "ClockPanel.qml" }

    function toggleLauncher() { launcherW.wanted = true; if (launcherW.ready) launcherW.content.toggle(); else launcherW.pending = () => launcherW.content.toggle(); }
    function openMenuSearch() { launcherW.wanted = true; if (launcherW.ready) launcherW.content.openMenuSearch(); else launcherW.pending = () => launcherW.content.openMenuSearch(); }
    function toggleQuick() { quickW.wanted = true; const f = () => { const q = quickW.content; if (typeof q.toggle === "function") q.toggle(); else q.shown = !q.shown; }; if (quickW.ready) f(); else quickW.pending = f; }
    function toggleCalendar() { calendarW.wanted = true; const f = () => { const c = calendarW.content; if (typeof c.toggle === "function") c.toggle(Hearth.screen, false); else c.shown = !c.shown; }; if (calendarW.ready) f(); else calendarW.pending = f; }

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
