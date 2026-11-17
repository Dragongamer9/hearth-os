// Hearth's password prompt (PolkitPrompt.qml): what an admin request is, in plain words, from polkit's action id and
// message. polkit's own message stays underneath (it names the program); this is the headline. No Qt here, so node
// can check it (test_hearth_polkit_mantel.py).
.pragma library

// Most specific first: [prefix, who's asking, what it wants to do].
var ACTIONS = [
    ["org.hearth.", "Hearth", "Change a system setting"],
    ["io.github.dragongamer9.", "Hearth", "Change a system setting"],
    ["org.freedesktop.policykit.exec", "An app", "Run something as an administrator"],
    ["org.freedesktop.udisks2.filesystem-mount", "Disks", "Open a drive"],
    ["org.freedesktop.udisks2.encrypted-unlock", "Disks", "Unlock an encrypted drive"],
    ["org.freedesktop.udisks2.modify-device", "Disks", "Change a drive"],
    ["org.freedesktop.udisks2.", "Disks", "Change a drive"],
    ["org.freedesktop.Flatpak.", "Bazaar", "Install or update apps for everyone"],
    ["org.freedesktop.packagekit.", "Software", "Install or update software"],
    ["org.freedesktop.NetworkManager.", "Network", "Change the network settings"],
    ["org.freedesktop.systemd1.manage-units", "System", "Start or stop a system service"],
    ["org.freedesktop.systemd1.", "System", "Change the system"],
    ["org.freedesktop.login1.reboot", "Power", "Restart this PC"],
    ["org.freedesktop.login1.power-off", "Power", "Turn this PC off"],
    ["org.freedesktop.login1.", "Power", "Change power or sign-in settings"],
    ["org.freedesktop.timedate1.", "Date & time", "Change the date, time or time zone"],
    ["org.freedesktop.hostname1.", "System", "Rename this PC"],
    ["org.freedesktop.locale1.", "Language", "Change the system language or keyboard"],
    ["org.freedesktop.accounts.", "Accounts", "Change a user account"],
    ["org.freedesktop.fwupd.", "Firmware", "Update this PC's firmware"],
    ["org.bluez.", "Bluetooth", "Change Bluetooth"],
    ["org.opensuse.cupspkhelper.", "Printers", "Change a printer"],
    ["org.fedoraproject.config.printing", "Printers", "Change a printer"],
    ["com.ubuntu.pkexec.", "An app", "Run something as an administrator"],
    ["org.debian.", "System", "Change the system"]
];

// {who, what} for an action id; a request Hearth doesn't know is "An app" asking for "Administrator access".
function describe(actionId) {
    var id = String(actionId || "");
    for (var i = 0; i < ACTIONS.length; i++) {
        var a = ACTIONS[i];
        if (id.indexOf(a[0]) === 0) return { who: a[1], what: a[2] };
    }
    return { who: "An app", what: "Administrator access" };
}

// The program pkexec is asked to run, from polkit's message ("Authentication is needed to run `/usr/bin/foo' as the
// super user"): "foo", or "" when the message doesn't say.
function program(message) {
    var m = /`([^'`]+)'/.exec(String(message || "")) || /"([^"]+)"/.exec(String(message || ""));
    if (!m) return "";
    var p = m[1].split(" ")[0];
    return p.split("/").pop();
}

// The person whose password is asked: polkit's identities are "unix-user:NAME" or a group; Quickshell gives each
// as {id, string, displayName, isGroup}. A short label and the initial for the picture when there's no picture.
function personLabel(ident) {
    if (!ident) return "";
    var name = String(ident.string || ident.name || "");
    name = name.replace(/^unix-(user|group):/, "");
    var shown = String(ident.displayName || "").replace(/^unix-(user|group):/, "") || name;
    return ident.isGroup ? "Someone in " + shown : shown;
}
function userName(ident) { return ident ? String(ident.string || ident.name || "").replace(/^unix-(user|group):/, "") : ""; }
function initial(label) { var t = String(label || "?").replace(/^Someone in /, ""); return t ? t.charAt(0).toUpperCase() : "?"; }
