// Away or locked, for Mantel's moving wallpapers (Wallpaper.qml holds them still then, as on Mantel): DeskState.idle
// after 3 minutes without input, DeskState.locked while KDE's screen locker is up. hearth-plasma-panel idle-wait
// sleeps until either changes; nothing here polls. Without KDE's screen saver service, they just keep moving.
import QtQuick

Item {
    id: il
    property int fails: 0
    function wait() {
        Hearth.call(["python3", Hearth.helper, "idle-wait", DeskState.idle ? "1" : "0", DeskState.locked ? "1" : "0"], (out) => {
            let o = null;
            try { o = JSON.parse(out.trim().split("\n").pop() || "null"); } catch (e) { o = null; }
            if (o && o.ok) { DeskState.idle = o.idle === true; DeskState.locked = o.locked === true; il.fails = 0; il.wait(); }
            else if (il.fails++ < 3) again.restart();
            else console.info("plasma-hearth: away and lock aren't known here (" + (o && o.why ? o.why : "no answer") + "); moving wallpapers keep moving");
        });
    }
    Timer { id: again; interval: 15000; onTriggered: il.wait() }
    Component.onCompleted: wait()
}
