// The app in front's own menus for Firehold on labwc's Desk bar (1.16, mockup firehold-plasma 11): Hearth's own menu
// source where KDE's global menu isn't there (hearth-globalmenu: the registrar apps put their menus in, and the reader
// of them). Its registrar sleeps until an app registers; the menus are read when the app in front changes, when an app
// registers, and when a menu opens. Nothing runs by itself here.
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: gm
    property bool active: false
    readonly property string helper: Quickshell.shellDir + "/hearth-globalmenu"
    Process { running: gm.active; command: ["python3", gm.helper, "serve"] }
    readonly property var front: WinList.activeToplevel
    readonly property string appId: front ? String(front.appId || "") : ""
    property var menus: []
    property string service: ""
    property string path: ""
    property string readFor: ""
    function read(then) {
        if (!active || appId === "") { menus = []; return; }
        const id = appId;
        Hearth.call(["python3", helper, "layout", id], (out) => {
            let o = null;
            try { o = JSON.parse(out.trim().split("\n").pop() || "{}"); } catch (e) {}
            if (id !== gm.appId) return;
            gm.readFor = id;
            gm.menus = o && Array.isArray(o.menus) ? o.menus : [];
            gm.service = o && o.service ? o.service : "";
            gm.path = o && o.path ? o.path : "";
            if (then) then();
        });
    }
    onAppIdChanged: read()
    onActiveChanged: read()
    FileView {
        path: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-globalmenu.json"
        watchChanges: gm.active
        printErrors: false
        onFileChanged: { reload(); gm.read(); }
    }
    function click(id) { if (service !== "" && path !== "") Hearth.call(["python3", helper, "click", service, path, String(id)], () => {}); }
    // A menu's rows for MenuCard.qml: [label, shortcut, action ("id:N"), kind (sep, sub, off)], its submenus by id.
    function rows(items) {
        return (items || []).map(it => it.kind === "separator" ? ["", "", "", "sep"]
            : it.kind === "sub" ? [it.label + " ›", "", "sub:" + it.id, "sub"]
            : [(it.checked === true ? "✓ " : "") + it.label, it.shortcut || "", "id:" + it.id, it.enabled === false ? "off" : ""]);
    }
    function sub(items, id) {
        for (const it of (items || [])) {
            if (String(it.id) === String(id)) return it.items || [];
            const x = sub(it.items, id);
            if (x && x.length) return x;
        }
        return [];
    }
}
