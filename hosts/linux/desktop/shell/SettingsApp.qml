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
        { id: "sound", icon: "vol", label: "Sound" },
        { id: "notifications", icon: "bell", label: "Notifications" },
        { id: "display", icon: "display", label: "Display" },
        { id: "appearance", icon: "palette", label: "Appearance" },
        { id: "power", icon: "power", label: "Power" },
        { id: "controller", icon: "pad", label: "Controller" },
        { id: "keyboard", icon: "kbd", label: "Keyboard" },
        { id: "mouse", icon: "mouse", label: "Mouse & touchpad" },
        { id: "printers", icon: "printer", label: "Printers" },
        { id: "datetime", icon: "clock", label: "Date & time" },
        // ---- Accessibility (SetAccessibility.qml)
        { id: "accessibility", icon: "access", label: "Accessibility" },
        // ---- end Accessibility
        { id: "defaults", icon: "grid", label: "Default apps" },
        { id: "apps", icon: "store", label: "Apps" },
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
        accessibility: "SetAccessibility.qml" /* Accessibility */, link: "SetLink.qml" /* Hearth Link */ })

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
                            Layout.bottomMargin: 10; Layout.leftMargin: 6; Layout.topMargin: 6
                            spacing: 10
                            Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 30; Layout.preferredHeight: 30; fillMode: Image.PreserveAspectFit; smooth: true }
                            Text { text: "Settings"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                        }
                        ListView {
                            id: nav
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: app.pages
                            spacing: 4
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            currentIndex: Math.max(0, app.pages.findIndex(p => p.id === app.page))
                            highlightMoveDuration: Theme.quick
                            keyNavigationEnabled: false
                            // More pages than fit on a small screen: keep the picked one in view.
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            function enter() { if (loader.item && loader.item.focusFirst) loader.item.focusFirst(); }
                            Keys.onPressed: e => {
                                switch (e.key) {
                                case Qt.Key_Down: case Qt.Key_Tab: app.page = app.pages[Math.min(app.pages.length - 1, currentIndex + 1)].id; break;
                                case Qt.Key_Up: case Qt.Key_Backtab: app.page = app.pages[Math.max(0, currentIndex - 1)].id; break;
                                case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: enter(); break;
                                case Qt.Key_Escape: app.close(); break;
                                default: return;
                                }
                                e.accepted = true;
                            }
                            delegate: Rectangle {
                                id: item
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.page === modelData.id
                                width: ListView.view.width
                                height: 48
                                radius: 10
                                color: lit ? (nav.activeFocus ? Theme.accent : Theme.panel3) : mouse.containsMouse ? Theme.panel2 : "transparent"
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
                        Text {
                            Layout.fillWidth: true
                            Layout.leftMargin: 6
                            text: "Ⓐ open  Ⓑ back"
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
