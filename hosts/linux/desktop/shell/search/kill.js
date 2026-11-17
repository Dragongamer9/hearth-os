.pragma library
// Search plugin: kill an app (like KRunner's kill runner). "kill firefox" lists your own running programs whose name
// matches (hearth-search-sources procs: [{name, pids, cmd}], looked up when you type "kill …"); picking one asks
// first (the confirm dialog), then asks it to close (SIGTERM, hearth-search-sources kill). Programs Hearth itself
// needs (the shell, the window manager, Steam's Game Mode, the session) are never offered.

var KEEP = ["quickshell", "labwc", "kwin_wayland", "kwin_x11", "gamescope", "gamescope-wl", "systemd", "dbus-daemon", "dbus-broker",
            "pipewire", "wireplumber", "nimbus-session", "hearth-session", "Xwayland", "greetd", "sddm"];

function target(qRaw) {
    var m = /^\s*kill\s+(.+?)\s*$/i.exec(String(qRaw || ""));
    return m ? m[1] : null;
}

function results(qRaw, procs) {
    var t = target(qRaw);
    if (!t) return [];
    var q = t.toLowerCase(), out = [];
    var list = procs && procs.query && procs.query.toLowerCase() === q ? procs.list || [] : null;
    if (!list) return [{ plugin: "kill", group: "Kill an app", title: "Looking for “" + t + "”…", sub: "Your own running programs", icon: "close", score: 2080, action: null }];
    for (var i = 0; i < list.length && out.length < 5; i++) {
        var p = list[i], n = String(p.name || "");
        if (KEEP.indexOf(n) >= 0 || n.toLowerCase().indexOf(q) < 0 || !p.pids || !p.pids.length) continue;
        var many = p.pids.length > 1 ? " (" + p.pids.length + " processes)" : "";
        out.push({ plugin: "kill", group: "Kill an app", title: "Kill " + n + many, sub: "Asks first · " + (p.cmd || n), icon: "close",
                   score: 2080 - i, top: out.length === 0,
                   action: { type: "kill", value: p.pids, name: n,
                             confirm: { title: "Kill " + n + "?", message: "Anything not saved in it is lost.", ok: "Kill" } } });
    }
    if (!out.length) out.push({ plugin: "kill", group: "Kill an app", title: "Nothing called “" + t + "” is running", sub: "Only your own programs can be killed here", icon: "close", score: 2080, action: null });
    return out;
}
