// A card that opens another page: its one real home, when a setting shows up in two places on purpose (Tailscale in
// Home server, Clipboard history in Privacy & security), or a subpage on a category's page. Title, icon and description
// come from the page registry (settings-pages.js) unless given.
import QtQuick
import "settings-pages.js" as Reg

SetRow {
    property string target            // the page it opens (an id in settings-pages.js)
    readonly property var entry: Reg.page(target) || ({})
    icon: entry.icon || ""
    title: entry.label || target
    sub: entry.desc || ""
    chevron: true
    onActivated: SettingsNav.openPage(target)
}
