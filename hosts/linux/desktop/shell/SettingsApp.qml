// Hearth Settings: Desktop Mode's own settings app. A normal window (labwc decorates and tiles it like any
// app) with the page list on the left and the page on the right. Works with mouse, touch, keyboard and the
// controller: Up/Down pick a page, Right/A go into it, Left/B come back, B on the list closes.
// Opened from the taskbar's Hearth menu, Quick Access, the app launcher, or:
//   quickshell ipc -p <shell folder> call settings open [page]
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Scope {
    id: app
    property bool shown: false
    property string page: "wifi"
    // On the live USB, "Install next to Windows" comes first.
    property bool live: false
    Process { running: true; command: ["test", "-e", "/etc/hearth-live"]; onExited: code => app.live = code === 0 }
    readonly property var pages: (live ? [{ id: "install", icon: "update", label: "Install Hearth" }] : []).concat([
        { id: "wifi", icon: "wifi", label: "Wi-Fi" },
        { id: "bluetooth", icon: "bt", label: "Bluetooth" },
        { id: "link", icon: "desktops", label: "Hearth Link" },   // SetLink.qml: your Hearth devices
        { id: "remote", icon: "wired", label: "Remote play" },   // SetRemote.qml: Sunshine and Moonlight
        // ---- Home server (docs/HOME-SERVER.md): this PC as a home server; the rest of the group comes in later stages
        { id: "hs-overview", icon: "grid", label: "Overview" },          // SetHomeServer.qml
        { id: "hs-shares", icon: "folder", label: "Shared folders" },    // SetHomeShares.qml
        { id: "hs-drives", icon: "restart", label: "Drives & undo" },    // SetHomeDrives.qml
        { id: "hs-tailscale", icon: "globe", label: "Tailscale" },       // SetHomeTailscale.qml
        { id: "hs-backups", icon: "update", label: "Backups & sync" },   // SetHomeBackups.qml
        { id: "hs-people", icon: "user", label: "People" },              // SetHomePeople.qml
        { id: "hs-alerts", icon: "bell", label: "Alerts & power" },      // SetHomeAlerts.qml
        // ---- end Home server
        { id: "sound", icon: "vol", label: "Sound" },
        { id: "notifications", icon: "bell", label: "Notifications" },
        { id: "display", icon: "display", label: "Display" },
        { id: "appearance", icon: "palette", label: "Appearance" },
        // ---- Boot & startup (SetBoot.qml): boot splash preset, Steam Game Mode's startup animation
        { id: "boot", icon: "power", label: "Boot & startup" },
        // ---- end Boot & startup
        { id: "power", icon: "power", label: "Power" },
        { id: "controller", icon: "pad", label: "Controller" },
        { id: "handheld", icon: "pad", label: "Handheld" },        // SetHandheld.qml: Handheld Daemon's settings
        { id: "keyboard", icon: "kbd", label: "Keyboard" },
        { id: "mouse", icon: "mouse", label: "Mouse & touchpad" },
        { id: "printers", icon: "printer", label: "Printers" },
        { id: "datetime", icon: "clock", label: "Date & time" },
        // ---- Accessibility (SetAccessibility.qml)
        { id: "accessibility", icon: "access", label: "Accessibility" },
        // ---- end Accessibility
        { id: "defaults", icon: "grid", label: "Default apps" },
        { id: "apps", icon: "store", label: "Apps" },
        // ---- moved from the archived Game Mode's Settings (docs/SETTINGS-MOVE.md)
        { id: "games", icon: "pad", label: "Games & stores" },     // SetGames.qml
        { id: "storage", icon: "folder", label: "Storage" },       // SetStorage.qml
        { id: "steam", icon: "window", label: "Steam Game Mode" }, // SetSteam.qml
        // ---- end moved
        { id: "clipboard", icon: "clipboard", label: "Clipboard" },
        { id: "users", icon: "user", label: "Users" },
        { id: "signin", icon: "lock", label: "Sign-in" },
        { id: "browser", icon: "globe", label: "Web browser" },
        { id: "claude", icon: "grid", label: "Claude" },
        { id: "updates", icon: "update", label: "Updates" },
        { id: "about", icon: "info", label: "About" }
    ])
    readonly property var files: ({ wifi: "SetWifi.qml", bluetooth: "SetBluetooth.qml", sound: "SetSound.qml", display: "SetDisplay.qml",
        appearance: "SetAppearance.qml", power: "SetPower.qml", controller: "SetController.qml", signin: "SetSignin.qml", browser: "SetBrowser.qml", claude: "SetClaude.qml",
        updates: "SetUpdates.qml", about: "SetAbout.qml", install: "SetInstall.qml",
        keyboard: "SetKeyboard.qml", apps: "SetApps.qml", printers: "SetPrinters.qml", datetime: "SetDateTime.qml", defaults: "SetDefaults.qml",
        clipboard: "SetClipboard.qml", users: "SetUsers.qml", mouse: "SetMouse.qml", notifications: "SetNotifications.qml",
        accessibility: "SetAccessibility.qml" /* Accessibility */, link: "SetLink.qml" /* Hearth Link */,
        remote: "SetRemote.qml", games: "SetGames.qml", storage: "SetStorage.qml", steam: "SetSteam.qml" /* moved from the archived Game Mode */,
        handheld: "SetHandheld.qml" /* Handheld Daemon */, boot: "SetBoot.qml" /* Boot & startup */,
        "hs-overview": "SetHomeServer.qml", "hs-shares": "SetHomeShares.qml", "hs-drives": "SetHomeDrives.qml",
        "hs-tailscale": "SetHomeTailscale.qml", "hs-setup": "SetHomeSetup.qml" /* Home server (hs-setup: its setup steps, opened from Overview) */,
        "hs-backups": "SetHomeBackups.qml", "hs-people": "SetHomePeople.qml", "hs-alerts": "SetHomeAlerts.qml" /* Home server, stage 2 */ })

    // ---- Mantel 2026: the page list in groups, with a search (also used by the launcher's search).
    readonly property var groups: ({ install: "Start", wifi: "Connections", bluetooth: "Connections", link: "Connections", remote: "Connections",
        sound: "Look and sound", notifications: "Look and sound", display: "Look and sound", appearance: "Look and sound", boot: "Look and sound",
        "hs-overview": "Home server", "hs-shares": "Home server", "hs-drives": "Home server", "hs-tailscale": "Home server", "hs-setup": "Home server",
        "hs-backups": "Home server", "hs-people": "Home server", "hs-alerts": "Home server",
        power: "Devices", controller: "Devices", handheld: "Devices", keyboard: "Devices", mouse: "Devices", printers: "Devices" })
    function groupOf(id) { return groups[id] || "System"; }
    // Other words people search for, per page.
    readonly property var keywords: ({ wifi: "wireless network internet hotspot", bluetooth: "headphones headset speaker pair devices",
        link: "phone devices sync", remote: "moonlight sunshine stream",
        "hs-overview": "nas home server light mode", "hs-shares": "nas smb samba share network windows folder", "hs-drives": "nas disk smart health snapshot undo raid mirror format",
        "hs-tailscale": "vpn remote away taildrop exit node wake", "hs-backups": "nas backup restore restic cloud b2 s3 sync syncthing photos phone",
        "hs-people": "nas family users accounts guest quota", "hs-alerts": "nas alerts smart notifications sleep wake on lan spin down ups power schedule", sound: "volume audio speakers output microphone",
        notifications: "do not disturb mute alerts", display: "screen resolution refresh scale monitor", appearance: "look theme accent colour color wallpaper dark light mode sunset bar dock transparency clock",
        boot: "splash startup animation", power: "battery sleep screen off suspend", controller: "gamepad buttons pad", handheld: "hhd tdp rgb lights",
        keyboard: "layout on-screen osk typing", mouse: "touchpad pointer cursor speed", printers: "print scanner", datetime: "clock time zone 24-hour",
        accessibility: "magnifier contrast text size", defaults: "default apps browser terminal", apps: "install uninstall packages deb flatpak",
        games: "steam stores library", storage: "disk space clean", steam: "game mode", clipboard: "copy paste history", users: "accounts",
        signin: "password login autologin", browser: "web", updates: "update upgrade version", about: "system version privacy backup" })
    property string query: ""
    readonly property var navPages: {
        const q = query.trim().toLowerCase();
        const list = q ? pages.filter(p => (p.label + " " + (keywords[p.id] || "") + " " + groupOf(p.id)).toLowerCase().includes(q)) : pages;
        return list.map(p => ({ id: p.id, icon: p.icon, label: p.label, group: groupOf(p.id) }));
    }

    Component.onCompleted: SettingsNav.settingsApp = app
    function open(p) {
        if (p && files[p]) page = p;
        shown = true;
    }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(""); }

    LazyLoader {
        active: app.shown
        FloatingWindow {
            id: win
            title: "Hearth Settings"
            implicitWidth: 1100
            implicitHeight: 720
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) app.shown = false
            Component.onCompleted: { SettingsNav.navTarget = nav; nav.forceActiveFocus(); }

            RowLayout {
                anchors.fill: parent
                spacing: 0

                // ------------------------------------------------ page list
                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: win.width < 900 ? 220 : 270
                    color: Theme.panel
                    ColumnLayout {
                        anchors { fill: parent; margins: 14 }
                        spacing: 6
                        RowLayout {
                            Layout.bottomMargin: 4; Layout.leftMargin: 6; Layout.topMargin: 6
                            spacing: 10
                            Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 30; Layout.preferredHeight: 30; fillMode: Image.PreserveAspectFit; smooth: true }
                            Text { text: "Settings"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                        }
                        // Search: type to filter the pages (Down or Enter goes to the list).
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.bottomMargin: 6
                            implicitHeight: 40
                            radius: Theme.r(20)
                            color: Theme.panel2
                            border.width: 2
                            border.color: searchField.activeFocus ? Theme.accent : "transparent"
                            Icon { x: 12; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 18; color: Theme.muted }
                            TextInput {
                                id: searchField
                                anchors { fill: parent; leftMargin: 40; rightMargin: 12 }
                                verticalAlignment: TextInput.AlignVCenter
                                color: Theme.text; selectionColor: Theme.accent
                                font.family: Theme.font; font.pixelSize: 14
                                text: app.query
                                onTextChanged: app.query = text
                                Keys.onDownPressed: nav.forceActiveFocus()
                                Keys.onReturnPressed: { if (app.navPages.length) app.page = app.navPages[0].id; nav.forceActiveFocus(); }
                                Keys.onEnterPressed: { if (app.navPages.length) app.page = app.navPages[0].id; nav.forceActiveFocus(); }
                                Keys.onEscapePressed: { if (text) text = ""; else nav.forceActiveFocus(); }
                                Text { anchors.verticalCenter: parent.verticalCenter; visible: !searchField.text; text: "Search settings"; color: Theme.muted; font: searchField.font }
                            }
                        }
                        ListView {
                            id: nav
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: app.navPages
                            spacing: 2
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            currentIndex: Math.max(0, app.navPages.findIndex(p => p.id === app.page))
                            highlightMoveDuration: Theme.quick
                            keyNavigationEnabled: false
                            // More pages than fit on a small screen: keep the picked one in view.
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            function enter() { if (loader.item && loader.item.focusFirst) loader.item.focusFirst(); }
                            Keys.onPressed: e => {
                                switch (e.key) {
                                case Qt.Key_Down: case Qt.Key_Tab: if (app.navPages.length) app.page = app.navPages[Math.min(app.navPages.length - 1, currentIndex + 1)].id; break;
                                case Qt.Key_Up: case Qt.Key_Backtab: if (currentIndex <= 0 && e.key === Qt.Key_Up && app.query !== "") searchField.forceActiveFocus(); else if (app.navPages.length) app.page = app.navPages[Math.max(0, currentIndex - 1)].id; break;
                                case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: enter(); break;
                                case Qt.Key_Escape: if (app.query !== "") app.query = ""; else app.close(); break;
                                case Qt.Key_Slash: searchField.forceActiveFocus(); break;
                                default:
                                    // Ctrl+F, or typing a letter on the list, starts a search.
                                    if (e.key === Qt.Key_F && (e.modifiers & Qt.ControlModifier)) { searchField.forceActiveFocus(); break; }
                                    if (e.text && e.text.length === 1 && !(e.modifiers & Qt.ControlModifier) && (/[0-9]/.test(e.text) || e.text.toLowerCase() !== e.text.toUpperCase())) { searchField.text += e.text; searchField.forceActiveFocus(); break; }
                                    return;
                                }
                                e.accepted = true;
                            }
                            delegate: Item {
                                id: item
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.page === modelData.id
                                // A group's name above its first page.
                                readonly property bool firstOfGroup: index === 0 || app.navPages[index - 1].group !== modelData.group
                                width: ListView.view.width
                                height: 44 + (firstOfGroup ? 30 : 0)
                                Text {
                                    visible: item.firstOfGroup
                                    x: 14; y: 10
                                    text: item.modelData.group.toUpperCase()
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 1.2
                                }
                                Rectangle {
                                anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                height: 44
                                radius: Theme.r(22)
                                color: item.lit ? (nav.activeFocus ? Theme.accent : Theme.panel3) : mouse.containsMouse ? Theme.panel2 : "transparent"
                                Behavior on color { ColorAnimation { duration: Theme.quick } }
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                                    spacing: 12
                                    Icon { name: item.modelData.icon; size: 20; color: item.lit && nav.activeFocus ? Theme.accentInk : item.lit ? Theme.accent : Theme.text }
                                    Text {
                                        Layout.fillWidth: true
                                        text: item.modelData.label
                                        color: item.lit && nav.activeFocus ? Theme.accentInk : Theme.text
                                        font.family: Theme.font; font.pixelSize: 15; font.weight: item.lit ? Font.DemiBold : Font.Normal
                                    }
                                }
                                MouseArea {
                                    id: mouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: { app.page = item.modelData.id; nav.forceActiveFocus(); }
                                }
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.leftMargin: 6
                            text: app.query ? app.navPages.length + (app.navPages.length === 1 ? " page" : " pages") + " · Esc clears" : "Ⓐ open  Ⓑ back"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }
                    }
                }
                Rectangle { Layout.fillHeight: true; Layout.preferredWidth: 1; color: Theme.line }

                // ------------------------------------------------ page
                Loader {
                    id: loader
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    source: Qt.resolvedUrl(app.files[app.page] || "SetAbout.qml")
                    onStatusChanged: if (status === Loader.Error) console.warn("Hearth Settings: couldn't load", source)
                }
            }
        }
    }
}
