.pragma library
// Search plugin: power and session (like KRunner's power management and session runners). "sleep", "restart",
// "shut down", "lock", "log out" (and "reboot", "suspend", "power off", "sign out"…). Restart, Shut down and Log out ask
// first (they close your apps); Sleep and Lock don't. Done by Launcher.qml's power() (the menu's own power buttons).

var ACTS = [
    ["sleep", "Sleep", "Your apps stay open", "sleep", ["sleep", "suspend"], false],
    ["lock", "Lock the screen", "Your apps stay open", "lock", ["lock", "lock screen", "lock the screen"], false],
    ["restart", "Restart", "Asks first · closes your apps", "restart", ["restart", "reboot"], true],
    ["shutdown", "Shut down", "Asks first · closes your apps", "power", ["shut down", "shutdown", "power off", "poweroff", "turn off"], true],
    ["logout", "Log out", "Asks first · closes your apps", "logout", ["log out", "logout", "sign out", "log off"], true],
    ["game", "Switch to Game Mode", "Steam's Game Mode", "pad", ["game mode", "switch to game mode"], false]
];

function results(qRaw) {
    var q = String(qRaw || "").trim().toLowerCase().replace(/\s+/g, " ");
    if (q.length < 3) return [];
    var out = [];
    for (var i = 0; i < ACTS.length; i++) {
        var a = ACTS[i], exact = a[4].indexOf(q) >= 0, starts = false;
        for (var k = 0; k < a[4].length; k++) if (a[4][k].indexOf(q) === 0) starts = true;
        if (!exact && !starts) continue;
        var act = { type: "power", value: a[0] };
        if (a[5]) act.confirm = { title: a[1] + "?", message: "Your apps close; anything not saved in them is lost.", ok: a[1] };
        out.push({ plugin: "power", group: "Power and session", title: a[1], sub: a[2], icon: a[3], score: exact ? 1840 : 900, top: exact, action: act });
    }
    return out;
}
