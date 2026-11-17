// Things plugged in (1.14, KDE's removable-storage actions and USB device notices, in Hearth's way). Listens to udev
// (udevadm monitor, read only) for:
//  - a USB drive or memory card with files on it: what Settings › Bluetooth & devices › Removable drives says
//    (Settings.removable.storage: "ask" (the default) shows Open / Not now, "open" mounts it and opens it in Files,
//    "nothing" leaves it in Files' sidebar). Mounting is udisks' (hearth-files mount), as Files does it.
//  - a USB device this account hasn't seen before: one notice with its name and kind (Settings.usbNotify), each device
//    once (~/.local/state/nimbus/usb-seen.json, state). Where USBGuard runs and has blocked it, the notice asks Allow
//    once / Allow always / Keep blocked (hearth-usbdev; Hearth itself never blocks anything).
// Nothing here runs at start-up for what's already plugged in. Loaded by shell.qml by file name: a problem here only
// loses these notices.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: dw
    property var dialog: null
    property var openFiles: null             // (path) => Files opened there
    property var settings: null
    readonly property string files: decodeURIComponent(Qt.resolvedUrl("hearth-files").toString().replace(/^file:\/\//, ""))
    readonly property string usbdev: decodeURIComponent(Qt.resolvedUrl("hearth-usbdev").toString().replace(/^file:\/\//, ""))
    readonly property string seenPath: Quickshell.env("HOME") + "/.local/state/nimbus/usb-seen.json"
    property var seen: ({})
    property bool started: false
    Timer { interval: 15000; running: true; onTriggered: dw.started = true }   // what's there at sign-in isn't news
    SettingsWriter { id: writer; path: dw.seenPath }
    FileView {
        path: dw.seenPath
        onLoaded: { try { const o = JSON.parse(text()); dw.seen = o && typeof o === "object" ? o : {}; } catch (e) { dw.seen = {}; } }
    }
    property var ev: ({})
    Process {
        id: mon
        running: true
        command: ["udevadm", "monitor", "--udev", "--property", "--subsystem-match=block", "--subsystem-match=usb"]
        stdout: SplitParser {
            onRead: l => {
                if (l.trim() === "") { const e = dw.ev; dw.ev = {}; if (e.ACTION) dw.handle(e); return; }
                const i = l.indexOf("=");
                if (i > 0 && !l.startsWith("UDEV ")) { const e = dw.ev; e[l.slice(0, i)] = l.slice(i + 1); dw.ev = e; }
            }
        }
        onExited: code => { if (code !== 0 && code !== 15) restart.start(); }
    }
    Timer { id: restart; interval: 30000; onTriggered: mon.running = true }

    function handle(e) {
        if (!started || e.ACTION !== "add") return;
        if (e.SUBSYSTEM === "block") drive(e);
        else if (e.SUBSYSTEM === "usb" && e.DEVTYPE === "usb_device") usb(e);
    }
    // ---- drives
    function drive(e) {
        if (e.ID_FS_USAGE !== "filesystem" || !e.DEVNAME) return;
        const removable = e.ID_BUS === "usb" || e.ID_DRIVE_FLASH_SD === "1" || e.ID_DRIVE_MEDIA_FLASH_SD === "1" || e.ID_DRIVE_FLASH === "1";
        if (!removable || e.UDISKS_IGNORE === "1") return;
        const what = (Settings.removable && Settings.removable.storage) || "ask";
        if (what === "nothing") return;
        const name = e.ID_FS_LABEL || e.ID_MODEL || "USB drive";
        if (what === "open") { mount(e.DEVNAME, true); return; }
        if (dialog) dialog.confirm(null, name + " is plugged in", "Open it in Files? (Settings › Removable drives says what happens next time.)", "Open", () => dw.mount(e.DEVNAME, true));
    }
    function mount(dev, open) {
        mounter.command = ["python3", dw.files, "mount", dev];
        mounter.open = open;
        if (!mounter.running) mounter.running = true;
    }
    Process {
        id: mounter
        property bool open: false
        stdout: StdioCollector {
            onStreamFinished: {
                try { const r = JSON.parse(text.trim().split("\n").pop()); if (r.path && mounter.open && dw.openFiles) dw.openFiles(r.path); }
                catch (e) { SettingsNav.toast("Couldn't open the drive"); }
            }
        }
    }
    // ---- USB devices
    function usb(e) {
        const vp = (e.ID_VENDOR_ID || "") + ":" + (e.ID_MODEL_ID || "");
        if (vp === ":" || /:09/.test(e.ID_USB_INTERFACES || "")) return;   // hubs
        const name = (e.ID_MODEL_FROM_DATABASE || e.ID_MODEL || "USB device").replace(/_/g, " ");
        const kind = /:0301/.test(e.ID_USB_INTERFACES || "") ? "keyboard" : /:0302/.test(e.ID_USB_INTERFACES || "") ? "mouse" : /:03/.test(e.ID_USB_INTERFACES || "") ? "input device"
            : /:08/.test(e.ID_USB_INTERFACES || "") ? "drive" : /:0e/.test(e.ID_USB_INTERFACES || "") ? "webcam" : /:01/.test(e.ID_USB_INTERFACES || "") ? "sound device" : /:06/.test(e.ID_USB_INTERFACES || "") ? "camera or phone" : "device";
        // USBGuard blocked it (the kernel's authorized flag is 0): ask, whatever the notice switch says.
        guardCheck.name = name; guardCheck.vp = vp;
        guardCheck.command = ["sh", "-c", 'cat "/sys$1/authorized" 2>/dev/null', "sh", e.DEVPATH || "/nonexistent"];
        if (!guardCheck.running) guardCheck.running = true;
        if (!Settings.usbNotify || dw.seen[vp]) return;
        const ch = {}; ch[vp] = name; writer.merge(ch); dw.seen = Object.assign({}, dw.seen, ch);
        tell.command = ["notify-send", "-a", "Hearth", "-i", "drive-removable-media", "-A", "open=USB devices", "New USB " + kind + ": " + name, "Plugged in for the first time on this PC."];
        if (!tell.running) tell.running = true;
    }
    Process { id: tell; stdout: SplitParser { onRead: l => { if (l.trim() === "open" && dw.settings) dw.settings.open("usb-devices"); } } }
    Process {
        id: guardCheck
        property string name: ""
        property string vp: ""
        stdout: StdioCollector {
            onStreamFinished: {
                if (text.trim() !== "0" || !dw.dialog) return;
                const nm = guardCheck.name, vp = guardCheck.vp;
                dw.dialog.confirm(null, "Allow " + nm + "?", "USBGuard blocked this USB device. Allow it if you plugged it in yourself; it can be allowed for good in Settings › USB devices.",
                    "Allow", () => { allow.vp = vp; allow.running = true; });
            }
        }
    }
    // Allow once: usbguard's id for the device, then allow-device.
    Process {
        id: allow
        property string vp: ""
        command: ["sh", "-c", 'id=$(usbguard list-devices 2>/dev/null | awk -v vp="$1" \'$2=="block" && $4==vp {sub(":","",$1); print $1; exit}\'); [ -n "$id" ] && exec python3 "$2" allow "$id"', "sh", allow.vp, dw.usbdev]
    }
    IpcHandler {
        target: "devicewatch"
        function drive(dev: string, label: string): void { dw.started = true; dw.drive({ ACTION: "add", SUBSYSTEM: "block", ID_FS_USAGE: "filesystem", ID_BUS: "usb", DEVNAME: dev, ID_FS_LABEL: label }); }
    }
}
