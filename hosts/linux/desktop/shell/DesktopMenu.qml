pragma Singleton

// The desktop's right-click menu under KWin: labwc's own desktop menu (labwc/menu.xml, root-menu: Apps, Terminal,
// Files, Return to Game Mode, Reconfigure, Log out) drawn by Mantel in Hearth's look. On labwc nothing changes
// (labwc's menu only shows when the shell isn't running; Mantel's desktop menu is drawn by Desktop.qml either way).
// Desktop.qml's backgroundMenu() adds these entries at the end of its own when WinList.kwin; the menu itself is the
// shared PopMenu, so the mouse, the keyboard (Menu key, Shift+F10), touch (press and hold) and the controller
// (D-pad, A, B) all work as in every other Mantel menu. `quickshell ipc ... call desktop menu` opens it.
//   quickshell ipc -p <shell folder> call deskmenu items      the entries as Mantel adds them (checks)
// Each command is menu.xml's own (test_kwin_windows.py checks they stay the same).
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: dm

    // menu.xml's root-menu, in its order, with its commands. Reconfigure is KWin's (it reloads its settings).
    readonly property var entries: [
        { id: "apps", label: "Apps", icon: "grid", group: "Hearth", command: "sh -c 'quickshell ipc -p $NIMBUS_SHARE/desktop/shell call launcher toggle'" },
        { id: "terminal", label: "Terminal", icon: "kbd", group: "Hearth", command: "sh -c 'sh $NIMBUS_SHARE/desktop/open.sh terminal'" },
        { id: "files", label: "Files", icon: "folder", group: "Hearth", command: "sh -c 'sh $NIMBUS_SHARE/desktop/open.sh files'" },
        { separator: true },
        { id: "game-mode", label: "Return to Game Mode", short: "Game Mode", icon: "pad", quick: true, group: "Hearth", command: "nimbus-mode game" },
        { id: "reconfigure", label: "Reconfigure", icon: "restart", group: "Hearth",
          command: "dbus-send --session --type=method_call --dest=org.kde.KWin /KWin org.kde.KWin.reconfigure" },
        { id: "logout", label: "Log out", icon: "logout", danger: true, group: "Hearth", command: "nimbus-mode logout" }
    ]

    function run(cmd) { Quickshell.execDetached(["sh", "-c", cmd]); }

    // The entries as PopMenu items, after a separator.
    function items() {
        const out = [{ separator: true }];
        for (const e of entries) {
            if (e.separator) { out.push({ separator: true }); continue; }
            const it = { label: e.label, icon: e.icon, group: e.group, run: () => dm.run(e.command) };
            if (e.short) it.short = e.short;
            if (e.quick) it.quick = true;
            if (e.danger) it.danger = true;
            out.push(it);
        }
        return out;
    }

    IpcHandler {
        target: "deskmenu"
        function items(): string { return JSON.stringify(dm.items().map(i => i.separator ? "-" : i.label)); }
    }
}
