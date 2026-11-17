// Hearth Settings › Privacy & security › Firmware & device security: Secure Boot, the TPM and fwupd's Host Security ID
// (read only), and firmware for this PC's parts from the LVFS (fwupd): what's there, what has an update, and installing
// one when you press (through hearth-admin, which always asks for an administrator's password and never restarts by
// itself). System firmware (UEFI) asks twice, only on mains power, and is put in at the next restart by the PC itself.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Firmware & device security"
    property var st: null              // hearth-firmware status
    property var devs: null            // hearth-firmware devices
    property var sec: null             // hearth-firmware security
    property string picked: ""
    property string confirm: ""        // the device asked about ("" none; id once; id + "!" for system firmware twice)
    property string message: ""
    property bool messageBad: false
    property bool restartNeeded: false
    readonly property var updates: devs ? devs.devices.filter(d => !!d.update) : []

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        job.run(["status"], r => page.st = r);
        job.run(["devices"], r => page.devs = r, e => page.devs = { devices: [], error: e });
    }
    Component.onCompleted: { load(); job.run(["security"], r => page.sec = r, () => page.sec = null); }
    SetJob { id: job; helper: "hearth-firmware" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    function sbText(s) {
        return !s ? "Checking…" : s.secureBoot === "on" ? "On: only signed start-up programs run"
             : s.secureBoot === "off" ? "Off. Hearth OS starts either way; turn it on in your PC's firmware setup (UEFI) if you like"
             : s.secureBoot === "unsupported" ? (s.uefi ? "This PC's firmware doesn't have it" : "This PC starts the old way (BIOS), without Secure Boot") : "Couldn't tell";
    }
    function update(d) {
        say(""); confirm = "";
        admin.run(["firmware-update", d.id], r => { page.restartNeeded = page.restartNeeded || !!r.restart; page.say(d.name + " is updated" + (r.restart ? ". It's put in when you restart" : "") + "."); page.load(); },
                  e => { page.say(e, true); page.load(); }, "", "update");
    }

    SetCardGroup {
        title: "Device security"
        SetRow {
            interactive: false; chevron: false; icon: "lock"; title: "Secure Boot"
            sub: page.sbText(page.st)
            chip: page.st ? (page.st.secureBoot === "on" ? "On" : page.st.secureBoot === "off" ? "Off" : "") : ""
            chipKind: page.st && page.st.secureBoot === "on" ? "ok" : "warn"
        }
        SetRow {
            interactive: false; chevron: false; icon: "key"; title: "Security chip (TPM)"
            sub: !page.st ? "Checking…" : page.st.tpm.present ? "TPM " + (page.st.tpm.version || "") + " is there and working" : "None found (or it's off in the firmware setup)"
            chip: page.st ? (page.st.tpm.present ? "Found" : "None") : ""
            chipKind: page.st && page.st.tpm.present ? "ok" : "warn"
        }
        SetExpander {
            visible: !!page.sec
            icon: "shield"; title: "Platform security: " + (page.sec ? page.sec.hsi : "")
            sub: page.sec ? (page.sec.level >= 2 ? "Well protected against firmware attacks" : page.sec.level === 1 ? "Basic protection" : "Some protections are off") + " · fwupd's checks" : ""
            Repeater {
                model: page.sec ? page.sec.items.slice(0, 24) : []
                delegate: SetRow {
                    required property var modelData
                    interactive: false; chevron: false
                    icon: modelData.ok ? "check" : "warn"
                    title: modelData.name
                    sub: (modelData.level ? "Level " + modelData.level + " · " : "") + modelData.result.replace(/-/g, " ")
                }
            }
        }
    }
    SetCardGroup {
        title: "Firmware updates" + (admin.what === "update" ? " · installing…" : "")
        SetRow {
            visible: !!page.st && !page.st.fwupd
            interactive: false; chevron: false; icon: "info"; title: "Firmware updates aren't on this system"; sub: "fwupd is missing"
        }
        SetRow {
            visible: !!page.st && page.st.fwupd
            icon: "sync"; chevron: false
            title: admin.what === "refresh" ? "Looking…" : "Check for firmware updates"
            sub: page.updates.length ? page.updates.length + (page.updates.length === 1 ? " update" : " updates") + " ready" : "From the Linux Vendor Firmware Service (LVFS), the way your PC's maker publishes them"
            onActivated: { page.say(""); admin.run(["firmware-refresh"], () => { page.load(); page.say("Firmware list is up to date."); }, e => page.say(e, true), "", "refresh"); }
        }
        Repeater {
            model: page.devs ? page.devs.devices : []
            delegate: ColumnLayout {
                id: dr
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                SetRow {
                    icon: dr.modelData.system ? "cpu" : "box"
                    title: dr.modelData.name
                    sub: (dr.modelData.vendor ? dr.modelData.vendor + " · " : "") + (dr.modelData.version ? "Version " + dr.modelData.version : "") + (dr.modelData.update ? " → " + dr.modelData.update.version : dr.modelData.updatable ? " · up to date" : "")
                    chip: dr.modelData.update ? "Update" : ""
                    chipKind: "warn"
                    onActivated: { page.picked = page.picked === dr.modelData.id ? "" : dr.modelData.id; page.confirm = ""; }
                }
                ColumnLayout {
                    visible: page.picked === dr.modelData.id
                    Layout.fillWidth: true
                    Layout.leftMargin: 52
                    spacing: 8
                    Text {
                        visible: !!dr.modelData.update && dr.modelData.update.description !== ""
                        Layout.fillWidth: true
                        text: dr.modelData.update ? (dr.modelData.update.summary ? dr.modelData.update.summary + ". " : "") + dr.modelData.update.description : ""
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                    }
                    SetRow {
                        visible: !!dr.modelData.update && page.confirm === ""
                        icon: "update"; chevron: false; title: "Update to " + (dr.modelData.update ? dr.modelData.update.version : "")
                        sub: dr.modelData.system ? "System firmware: put in by the PC while it restarts. Keep it plugged in" : "Takes a minute; keep the device connected"
                        onActivated: page.confirm = dr.modelData.id
                    }
                    SetRow {
                        visible: page.confirm === dr.modelData.id
                        icon: "warn"; danger: dr.modelData.system; chevron: false
                        title: dr.modelData.system ? "This changes your PC's own firmware. Continue?" : "Yes, update " + dr.modelData.name
                        sub: dr.modelData.system ? "Hearth OS stays as it is. Don't turn the PC off while the firmware updater runs at the restart" : "Asks for an administrator's password"
                        onActivated: { if (dr.modelData.system) page.confirm = dr.modelData.id + "!"; else page.update(dr.modelData); }
                    }
                    SetRow {
                        visible: page.confirm === dr.modelData.id + "!"
                        icon: "update"; danger: true; chevron: false; title: "Yes, update the system firmware"
                        sub: page.st && page.st.power.ac === false ? "Plug in the charger first" : "Asks for an administrator's password; nothing restarts until you do"
                        onActivated: page.update(dr.modelData)
                    }
                    SetRow { visible: page.confirm !== ""; title: "Cancel"; chevron: false; onActivated: page.confirm = "" }
                }
            }
        }
        Text {
            visible: !!page.devs && page.devs.devices.length === 0
            Layout.fillWidth: true
            text: page.devs && page.devs.error ? page.devs.error : "fwupd found no parts it can update."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        SetRow {
            visible: page.restartNeeded
            icon: "restart"; chevron: false; title: "Restart to finish"; sub: "The new firmware goes in while the PC restarts"
            onActivated: Quickshell.execDetached(["systemctl", "reboot"])
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
