// Whether the app in front has menus of its own (KDE's global menu model, as Plasma's own app menu widget reads it).
// Loaded on its own: a Plasma without it just means Hearth's standard menus show.
import QtQuick
import org.kde.plasma.private.appmenu as AppMenuPrivate

Item {
    readonly property bool available: model.menuAvailable
    AppMenuPrivate.AppMenuModel { id: model }
}
