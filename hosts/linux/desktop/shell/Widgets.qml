// Desktop widgets and Activities (1.14; docs/WIDGETS.md): written for Hearth, as KDE Plasma has them.
//   * Widgets on the desktop, on every screen: each its own small surface above the wallpaper and the icons, under
//     every window (WidgetWindow.qml). Add widgets opens the gallery (WidgetGallery.qml); Arrange (WidgetArrange.qml)
//     moves, resizes and removes them with the keyboard or the controller. Places and settings: widgets.json
//     (WidgetStore.qml), remembered per screen and per activity, never changed by themselves.
//   * A few of them can sit in the bar instead (BarWidgets.qml: clock, weather, system monitor, battery, timer).
//   * Activities (Activities.qml, ActivitySwitcher.qml): sets of windows, wallpaper, widgets and pinned apps.
// Loaded by shell.qml by file name, so a problem here only loses widgets and activities, never the shell.
//   quickshell ipc -p <shell folder> call widgets gallery|close|arrange|done|add <type>|addBar <type>|list|count|configure <id>
//   quickshell ipc -p <shell folder> call activities switcher|close|next|previous|switchTo <id>|create <name>|current|list|wallpaper <id> <w>
import QtQuick
import Quickshell
import Quickshell.Io
import "widget-layout.js" as L
import "activities.js" as A

Scope {
    id: root
    property string galleryOn: ""          // the screen the gallery is open on
    readonly property string primaryName: Quickshell.screens.length ? Quickshell.screens[0].name : ""
    readonly property var screenNames: Quickshell.screens.map(s => s.name)
    function screenNamed(n) { return Quickshell.screens.find(s => s.name === n) || Quickshell.screens[0] || null; }
    // The screen you're most likely looking at: the one with the window in front, else the main one.
    function hereName() { return primaryName; }

    function openGallery(n) { galleryOn = n || hereName(); WidgetStore.arranging = ""; }
    function closeGallery() { galleryOn = ""; }
    function startArrange(n) { galleryOn = ""; WidgetStore.configuring = ""; WidgetStore.arranging = n || hereName(); }
    function stopArrange() { WidgetStore.arranging = ""; }

    // Add a widget of TYPE on screen N (desktop) or to the bar, on the activity you're on, at a free place.
    function add(type, n, toBar) {
        const t = L.info(type);
        if (!t) return "";
        const scrN = n || galleryOn || hereName();
        if (toBar) {
            if (!t.bar) return "";
            const id = WidgetStore.add(type, "", "*", "bar", null, {});   // the bar is the same on every activity
            SettingsNav.toast(t.name + " is in the bar");
            return id;
        }
        const scr = screenNamed(scrN);
        const sw = scr ? scr.width : 1920, sh = scr ? scr.height : 1080;
        const ins = { top: DeskState.topInset, bottom: DeskState.bottomInset, left: DeskState.leftInset, right: DeskState.rightInset };
        const mine = L.forScreen(WidgetStore.widgets, scrN, screenNames, primaryName, Activities.current).map(w => L.shown(w, sw, sh, ins));
        const at = L.findSpot(mine, t.w, t.h, sw, sh, ins);
        const id = WidgetStore.add(type, scrN, Activities.current, "desk", { x: at.x, y: at.y, w: t.w, h: t.h }, {});
        SettingsNav.toast(t.name + " is on the desktop");
        return id;
    }
    function openActivities() { actSwitcher.open(); }
    Component.onCompleted: { WidgetStore.ui = root; SettingsNav.widgets = root; }
    Component.onDestruction: if (SettingsNav.widgets === root) SettingsNav.widgets = null
    Binding { target: Theme; property: "activityWall"; value: Activities.wallpaper }
    Binding { target: SettingsNav; property: "activityName"; value: Activities.name }
    Binding { target: SettingsNav; property: "activityIcon"; value: Activities.cur ? Activities.cur.icon : "home" }
    Binding { target: SettingsNav; property: "activitiesMany"; value: Activities.many }

    Binding { target: SettingsNav; property: "widgetsShown"; value: root.galleryOn !== "" || WidgetStore.arranging !== "" || WidgetStore.configuring !== "" || actSwitcher.shown }

    // ---- the widgets, the gallery and Arrange on every screen
    Variants {
        model: Quickshell.screens
        delegate: Component {
            Scope {
                id: per
                required property ShellScreen modelData
                readonly property var ids: { void WidgetStore.rev; return L.forScreen(WidgetStore.widgets, per.modelData.name, root.screenNames, root.primaryName, Activities.current).map(w => w.id); }
                Variants {
                    model: per.ids
                    delegate: Component { WidgetWindow { screen: per.modelData; screenName: per.modelData.name } }
                }
                WidgetGallery {
                    screen: per.modelData
                    shown: root.galleryOn === per.modelData.name
                    onAddRequested: (type, toBar) => root.add(type, per.modelData.name, toBar)
                    onCloseRequested: root.closeGallery()
                    onArrangeRequested: root.startArrange(per.modelData.name)
                }
                WidgetArrange {
                    screen: per.modelData
                    screenName: per.modelData.name
                    shown: WidgetStore.arranging === per.modelData.name
                    ids: per.ids
                    onDoneRequested: root.stopArrange()
                    onAddRequested: root.openGallery(per.modelData.name)
                }
            }
        }
    }
    ActivitySwitcher { id: actSwitcher }

    IpcHandler {
        target: "widgets"
        function gallery(): void { root.openGallery(""); }
        function close(): void { root.closeGallery(); root.stopArrange(); WidgetStore.configuring = ""; }
        function arrange(): void { root.startArrange(""); }
        function done(): void { root.stopArrange(); }
        function add(type: string): string { return root.add(type, "", false); }
        function addBar(type: string): string { return root.add(type, "", true); }
        function configure(id: string): void { WidgetStore.configuring = id; }
        function set(id: string, key: string, json: string): void { try { WidgetStore.setCfg(id, key, JSON.parse(json)); } catch (e) { console.warn("widgets: set " + key + ": " + e); } }
        function remove(id: string): void { WidgetStore.remove(id); }
        function list(): string { return JSON.stringify(WidgetStore.widgets.map(w => ({ id: w.id, type: w.type, place: w.place, screen: w.screen, activity: w.activity, x: w.x, y: w.y, w: w.w, h: w.h }))); }
        function count(): int { return WidgetStore.widgets.length; }
    }
    IpcHandler {
        target: "activities"
        function switcher(): void { actSwitcher.open(); }
        function close(): void { actSwitcher.close(); }
        function next(): void { Activities.step(1); }
        function previous(): void { Activities.step(-1); }
        function switchTo(id: string): void { Activities.switchTo(id); }
        function create(name: string): string { return Activities.create(name, ""); }
        function current(): string { return Activities.current; }
        function wallpaper(id: string, w: string): void { Activities.setWallpaper(id, w); }
        function list(): string { return JSON.stringify(Activities.list.map(a => ({ id: a.id, name: a.name, windows: Activities.windowsOn(a.id) }))); }
    }
}
