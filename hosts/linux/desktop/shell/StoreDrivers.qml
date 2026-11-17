// Settings › App store › Drivers (1.14, like Ubuntu's Additional Drivers and Discover's): the PC's parts and the
// Debian packages that drive them (Nvidia's driver, firmware-*, processor microcode), found by hearth-store drivers
// (lspci, the CPU, firmware the kernel asked for and couldn't load). Each shows whether it's installed and in use;
// a missing one installs after an administrator's password (hearth-store driver-install → hearth-admin
// driver-install → Peppermint's writable system, kept across updates; the version before stays startable).
// Opening the page only reads. Loaded by SetStore.qml for its Drivers view.
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: dr
    property var page: null
    property var info: null
    property string err: ""
    property string confirm: ""
    spacing: 12
    function load() { err = ""; job.run(["drivers"], r => dr.info = r, e => dr.err = e); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-store" }

    Text {
        Layout.fillWidth: true
        text: "What your PC's parts need to work their best: graphics drivers, firmware for Wi-Fi, Bluetooth and sound, and fixes for the processor. Hearth OS comes with most of them."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text { visible: !dr.info && dr.err === ""; text: "Looking at your PC…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
    Text { visible: dr.err !== ""; Layout.fillWidth: true; text: dr.err; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
    Text {
        visible: !!dr.info && !dr.info.canInstall
        Layout.fillWidth: true
        text: dr.info ? dr.info.cantWhy : ""
        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetCardGroup {
        title: !dr.info ? "Drivers" : dr.info.missing ? "Drivers · " + dr.info.missing + " not installed" : "Drivers · all installed"
        visible: !!dr.info
        SetRow { visible: !!dr.info && dr.info.drivers.length === 0; interactive: false; icon: "check"; title: "Nothing extra is needed"; sub: "Your PC's parts work with what Hearth OS has" }
        Repeater {
            model: dr.info ? dr.info.drivers : []
            delegate: SetRow {
                required property var modelData
                readonly property bool can: !modelData.infoOnly && !modelData.installed && !!modelData.available && dr.info.canInstall
                icon: modelData.package.indexOf("nvidia") === 0 || modelData.device.indexOf("graphics") >= 0 ? "display"
                    : modelData.package.indexOf("microcode") >= 0 ? "cpu" : modelData.device.indexOf("sound") >= 0 ? "vol" : "wifi"
                title: modelData.device + "  ·  " + modelData.package
                sub: modelData.infoOnly ? modelData.note + (modelData.installed ? " · installed (" + modelData.installed + ")" : "")
                    : modelData.installed ? "Installed (" + modelData.installed + ")" + (modelData.why.length > 1 ? " · " + modelData.why.slice(1).join(" · ") : "")
                    : modelData.available ? "Not installed · " + modelData.available + " is there to install" + (modelData.why.length > 1 ? " · " + modelData.why.slice(1).join(" · ") : "")
                    : modelData.note
                chip: modelData.infoOnly ? (modelData.inUse ? "In use" : "With updates") : modelData.installed ? (modelData.inUse ? "In use" : "Installed") : modelData.available ? "Not installed" : "Not available"
                chipKind: modelData.infoOnly ? (modelData.inUse ? "ok" : "off") : modelData.installed ? "ok" : modelData.available ? "warn" : "off"
                value: can ? (dr.confirm === modelData.package ? "Install (asks for a password)" : "Install…") : ""
                danger: dr.confirm === modelData.package
                chevron: false
                interactive: can
                onActivated: {
                    if (!can || worker.busy) return;
                    if (dr.confirm !== modelData.package) { dr.confirm = modelData.package; return; }
                    dr.confirm = "";
                    worker.run(["driver-install", modelData.package], r => {
                        const t = modelData.package + " is installed: it takes effect after a restart (graphics firmware comes with Hearth OS's updates).";
                        if (dr.page) dr.page.say(t); SettingsNav.toast(t); dr.load();
                    }, e => { if (dr.page) dr.page.say(e, true); });
                }
            }
        }
    }
    ColumnLayout {
        visible: worker.busy
        Layout.fillWidth: true
        spacing: 6
        Text { Layout.fillWidth: true; text: (worker.line || "Installing…") + (worker.progress >= 0 ? "  " + Math.round(worker.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
        Rectangle { Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, worker.progress); height: parent.height; radius: 4; color: Theme.flame } }
    }
    SetJob { id: worker; helper: "hearth-store" }
    SetRow { icon: "update"; title: "Check again"; sub: "After plugging something in"; chevron: false; onActivated: dr.load() }
    SetRow { icon: "cpu"; title: "Firmware"; sub: "Updates for your PC's own firmware: Settings › Firmware"; onActivated: SettingsNav.openPage("firmware") }
}
