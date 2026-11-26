// The desktop you're on and how many there are (DeskState.deskNow, deskCount), for blocks that show only on some
// desktops (drag and drop board 2, Place › which desktops; BlockLook.qml). The same list as DeskSwitcher.qml: KWin's
// and Firehold's from WinList, else ext-workspace. Loaded through a Loader by the main bar (Islands.qml), so a system
// without Quickshell's WindowManager module only loses this (blocks then show on every desktop).
import QtQuick
import Quickshell
import Quickshell.WindowManager

Item {
    readonly property var sets: {
        if (WinList.wmDesktops) return WinList.desktops.map(d => ({ name: d.name || "", active: d.active === true }));
        const all = Array.from(WindowManager.windowsets || []).filter(w => w && w.shouldDisplay !== false);
        all.sort((a, b) => {
            const ca = a.coordinates || [], cb = b.coordinates || [];
            for (let i = 0; i < Math.max(ca.length, cb.length); i++) if ((ca[i] || 0) !== (cb[i] || 0)) return (ca[i] || 0) - (cb[i] || 0);
            return String(a.name || "").localeCompare(String(b.name || ""), undefined, { numeric: true });
        });
        return all.map(w => ({ name: w.name || "", active: w.active }));
    }
    readonly property int now: { for (let i = 0; i < sets.length; i++) if (sets[i].active) return i + 1; return 0; }
    onNowChanged: DeskState.deskNow = now
    onSetsChanged: if (sets.length > 0) DeskState.deskCount = sets.length
    Component.onCompleted: { DeskState.deskNow = now; if (sets.length > 0) DeskState.deskCount = sets.length; }
}
