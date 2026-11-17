pragma Singleton

// 1.14: Log out, the one way Mantel's menus do it (the app menus, Super+X, the desktop's right-click menu). With
// Settings › Startup › "Confirm before logging out" on, it asks first (LeaveConfirm.qml); off (the default) it logs out
// at once, as before. If the question can't be shown (LeaveConfirm.qml didn't load), it logs out as before too.
import QtQuick
import Quickshell

Singleton {
    id: root
    property bool canAsk: false          // LeaveConfirm.qml is loaded and listening
    signal confirmRequested()
    function logout() {
        if (Settings.confirmLogout && canAsk) { confirmRequested(); return; }
        now();
    }
    function now() { Quickshell.execDetached(["nimbus-mode", "logout"]); }
}
