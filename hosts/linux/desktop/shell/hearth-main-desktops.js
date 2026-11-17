// Hearth: virtual desktops only on the main screen (Settings › Display › Several screens; hearth-screens loads this
// into KWin, with the main screen's name put in for HEARTH_MAIN). Windows on the other screens stay on every desktop,
// so switching desktops only changes the main screen (as GNOME's "workspaces on the primary display only"). It
// remembers the windows it changed and puts only those back when it's turned off (its "Hearth main desktops off"
// shortcut, which hearth-screens calls before unloading it). KWin 6.3 has one set of desktops for all screens, so
// this is the closest it can do; nothing is written anywhere.
var MAIN = "HEARTH_MAIN";
var pinned = {};

function onMain(w) {
    return !w.output || String(w.output.name) === MAIN;
}

function follow(w) {
    if (!w || !w.normalWindow || w.skipTaskbar)
        return;
    var id = String(w.internalId);
    if (!onMain(w)) {
        if (!w.onAllDesktops) {
            w.onAllDesktops = true;
            pinned[id] = true;
        }
    } else if (pinned[id]) {
        w.onAllDesktops = false;
        delete pinned[id];
    }
}

function watch(w) {
    follow(w);
    if (w && w.outputChanged)
        w.outputChanged.connect(function () { follow(w); });
}

var ws = workspace.windowList ? workspace.windowList() : workspace.clientList();
for (var i = 0; i < ws.length; i++)
    watch(ws[i]);
(workspace.windowAdded || workspace.clientAdded).connect(watch);

registerShortcut("Hearth main desktops off", "Hearth: put back the windows on the other screens", "", function () {
    var all = workspace.windowList ? workspace.windowList() : workspace.clientList();
    for (var j = 0; j < all.length; j++) {
        var id = String(all[j].internalId);
        if (pinned[id]) {
            all[j].onAllDesktops = false;
            delete pinned[id];
        }
    }
    MAIN = "";      // nothing more to do until it's unloaded
    onMain = function () { return true; };
});
