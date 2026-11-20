pragma Singleton
// Vaults in the bar (plasma-hearth): Mantel's vaults aren't on Plasma yet (KDE's own Vaults widget is).
import QtQuick
QtObject { property var vaults: []; property var opened: []; property string tip: ""; function menu(f) { return []; } }
