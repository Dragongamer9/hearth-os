// A small layer window over everything that never takes the keyboard unless it asks (wantsKeys): the window type
// Firehold's notification cards are written for (plasma-hearth/package/FireholdCards.qml; on Plasma it's
// plasma-hearth/shim/LayerPanel.qml). Under Mantel it's simply Quickshell's own PanelWindow (Firehold on labwc, 1.16).
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    property string scopeName: "hearth-notifications"
    property bool wantsKeys: false
    WlrLayershell.namespace: scopeName
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: wantsKeys ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
}
