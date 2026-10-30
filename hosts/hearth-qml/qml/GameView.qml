// A game's page (the web page's viewGame): its picture and logo, Play / Resume / Install / the download, Exit game,
// Favorite, Uninstall, the details, its ProtonDB rating and achievements (game.info), and Game settings (Proton,
// MangoHud, GameMode, FSR, cloud saves, frame-rate limit: game.options). Launch options, mods and saves are the web
// renderer's for now.
import QtQuick

Item {
    id: gp
    readonly property var game: { App.rev; return App.g(App.gameId); }
    readonly property bool running: !!game && App.running === game.id
    readonly property bool active: App.isActive(game)
    property var info: null          // game.info: {protondb: {tier}, achievements: {done, total, list}}
    readonly property var ach: info && info.achievements && info.achievements.total ? info.achievements : null
    readonly property string pdb: info && info.protondb && info.protondb.tier ? String(info.protondb.tier) : ""
    function loadInfo() {
        info = null;
        const id = App.gameId;
        if (!id) return;
        App.call("game.info", { id: id }, 30000).then(r => { if (App.gameId === id) gp.info = r || {}; }).catch(() => { if (App.gameId === id) gp.info = {}; });
    }
    Connections { target: App; function onGameIdChanged() { if (App.view === "game") gp.loadInfo(); } }
    Component.onCompleted: loadInfo()

    Text {
        visible: !gp.game
        anchors.centerIn: parent
        text: "That game isn't in your library anymore."
        color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(16)
    }

    Scroller {
        id: page
        visible: !!gp.game
        anchors.fill: parent
        contentHeight: col.height + Theme.u(40)
        Column {
            id: col
            width: page.width
            Item {
                id: heroBox
                width: parent.width
                height: Math.max(Theme.u(220), Math.min(Theme.u(380), gp.height * 0.46))
                Rectangle { anchors.fill: parent; color: Theme.panel }
                ArtImage { id: heroArt; anchors.fill: parent; game: gp.game; kinds: ["hero", "wide", "cover"] }
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.35; color: Theme.alpha(Theme.bg, 0) }
                        GradientStop { position: 1; color: Theme.bg }
                    }
                }
                ArtImage {
                    id: logo
                    anchors { left: parent.left; leftMargin: Theme.gut; bottom: parent.bottom; bottomMargin: Theme.u(20) }
                    width: Math.min(Theme.u(420), parent.width * 0.45)
                    height: Theme.u(120)
                    game: gp.game
                    kinds: ["logo"]
                    fill: Image.PreserveAspectFit
                    hAlign: Image.AlignLeft
                    visible: ready
                }
                Text {
                    visible: !logo.ready
                    anchors { left: parent.left; leftMargin: Theme.gut; right: parent.right; rightMargin: Theme.gut; bottom: parent.bottom; bottomMargin: Theme.u(20) }
                    text: gp.game ? gp.game.title : ""
                    color: Theme.text
                    wrapMode: Text.WordWrap
                    font.family: Theme.display
                    font.pixelSize: Theme.u(40)
                    font.weight: Font.Black
                    style: Text.Raised
                    styleColor: Theme.alpha("#000000", 0.5)
                }
            }
            Flow {
                id: barRow
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                spacing: Theme.u(10)
                topPadding: Theme.u(6)
                Btn {
                    id: play
                    navKey: "g-play"
                    navFirst: true
                    kind: "primary"
                    minW: Theme.u(200)
                    height: Theme.u(52)
                    icon: gp.active && !(gp.game && gp.game.inst) ? "dl" : App.playable(gp.game) ? "play" : "dl"
                    text: !gp.game ? "" : gp.active && !gp.game.inst ? App.stateLabel(gp.game) : gp.running ? "Resume" : App.playable(gp.game) ? (gp.game.cloud && !gp.game.inst ? "Stream" : "Play") : gp.game.src === "beacon" ? "Set up" : "Install"
                    onActivated: {
                        const x = gp.game;
                        if (!x) return;
                        if (gp.active && !x.inst) return App.navigate("downloads");
                        if (gp.running) { App.closeAll(); return App.send({ type: "resume" }); }
                        if (App.playable(x)) return App.launch(x);
                        App.install(x);
                    }
                }
                Btn { visible: gp.running; navKey: "g-stop"; icon: "stop"; text: "Exit game"; height: Theme.u(52); onActivated: App.exitGame() }
                Btn { visible: gp.active; navKey: "g-cancel"; icon: "x"; text: "Cancel download"; height: Theme.u(52); onActivated: App.dlCall("game.cancelInstall", gp.game.id) }
                Btn { navKey: "g-fav"; kind: "icon"; icon: "star"; on: !!(gp.game && gp.game.fav); width: Theme.u(52); height: Theme.u(52); onActivated: App.setFav(gp.game) }
                Btn { visible: !!(gp.game && gp.game.inst && gp.game.canUninstall); navKey: "g-uninst"; icon: "trash"; text: "Uninstall"; height: Theme.u(52); onActivated: App.uninstall(gp.game) }
                Btn { visible: !!(gp.game && !gp.game.cloud); navKey: "g-more"; icon: "dots"; text: "Game settings…"; height: Theme.u(52); onActivated: gp.openSettings() }
            }
            Flow {
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                spacing: Theme.u(34)
                topPadding: Theme.u(18)
                bottomPadding: Theme.u(8)
                Repeater {
                    model: gp.game ? [
                        ["Last played", gp.game.last ? gp.ago(gp.game.last) : "Never"],
                        ["Play time", gp.game.hrs ? gp.hours(gp.game.hrs) : "—"],
                        [gp.game.system ? "System" : "Source", App.sourceName(gp.game)],
                        ["Size", gp.game.sizeBytes ? gp.size(gp.game.sizeBytes) : "—"]
                    ].concat(gp.pdb ? [["ProtonDB", gp.pdb.charAt(0).toUpperCase() + gp.pdb.slice(1)]] : []) : []
                    delegate: Column {
                        required property var modelData
                        spacing: Theme.u(3)
                        Text { text: modelData[0]; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(12); font.capitalization: Font.AllUppercase }
                        Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(16); font.weight: Font.DemiBold }
                    }
                }
            }
            Rectangle {
                visible: !!gp.ach
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                height: visible ? achCol.implicitHeight + Theme.u(32) : 0
                radius: Theme.radius
                color: Theme.panel
                Column {
                    id: achCol
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(16) }
                    spacing: Theme.u(8)
                    Text { text: gp.ach ? "Achievements · " + gp.ach.done + " of " + gp.ach.total + " unlocked" : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(15); font.weight: Font.DemiBold }
                    Bar { width: parent.width; value: gp.ach ? gp.ach.done / Math.max(1, gp.ach.total) : 0 }
                    Repeater {
                        model: gp.ach ? (gp.ach.list || []).slice(0, 6) : []
                        delegate: Text {
                            required property var modelData
                            width: achCol.width; elide: Text.ElideRight
                            text: (modelData.done ? "✓ " : "") + (modelData.hidden && !modelData.done ? "Hidden achievement" : modelData.name) + (modelData.done || modelData.hidden ? "" : " · " + (modelData.desc || ""))
                            color: modelData.done ? Theme.text : Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13)
                        }
                    }
                    Btn { visible: !!(gp.ach && (gp.ach.list || []).length > 6); navKey: "g-ach"; icon: "trophy"; text: gp.ach ? "See all " + gp.ach.total : ""; onActivated: gp.allAchievements() }
                }
            }
            Item { visible: !!gp.ach; width: 1; height: Theme.u(12) }
            Rectangle {
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                height: about.implicitHeight + Theme.u(32)
                radius: Theme.radius
                color: Theme.panel
                Column {
                    id: about
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(16) }
                    spacing: Theme.u(8)
                    Text { text: "About"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(15); font.weight: Font.DemiBold }
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14); text: gp.aboutText() }
                    Text { visible: !!(gp.game && gp.game.path); width: parent.width; wrapMode: Text.WrapAnywhere; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13); text: gp.game ? "Location: " + gp.game.path : "" }
                }
            }
        }
    }

    function allAchievements() {
        const a = ach; if (!a) return;
        App.openSheet({ title: "Achievements · " + a.done + " of " + a.total, items: (a.list || []).map(x => ({
            label: (x.done ? "✓ " : "") + (x.hidden && !x.done ? "Hidden achievement" : x.name), icon: x.done ? "check" : "trophy",
            sub: x.done ? "Unlocked" : (x.hidden ? "Keep playing to find out" : (x.desc || "")), run: null })) });
    }
    function openSettings() {
        const x = game; if (!x) return;
        App.call("game.options.get", { id: x.id }, 8000).then(r => {
            if (!r) return;
            const o = r.options || {};
            const set = patch => App.call("game.options.set", { id: x.id, options: patch }, 8000).then(() => App.later(80, () => gp.openSettings())).catch(e => App.toast(e.message));
            const onOff = v => v ? "on" : "off";
            const items = [];
            if (r.kind === "proton") {
                const cur = (r.protonChoices || []).find(c => c.id === (o.proton || "")) || { label: o.proton || "Default" };
                items.push({ label: "Proton: " + cur.label, icon: "layout", sub: "Which Proton runs this Windows game", run: () => gp.pickProton(r) });
            }
            if (r.kind !== "emu" && r.kind !== "beacon") {
                items.push({ label: "Performance overlay (MangoHud): " + onOff(o.mangohud), icon: "bolt", on: !!o.mangohud, run: () => set({ mangohud: !o.mangohud }) });
                if (r.gamemode) items.push({ label: "GameMode: " + onOff(o.gamemode), icon: "pad", on: !!o.gamemode, run: () => set({ gamemode: !o.gamemode }) });
                items.push({ label: "AMD FSR upscaling: " + onOff(o.fsr), icon: "display", on: !!o.fsr, run: () => set({ fsr: !o.fsr }) });
            }
            items.push({ label: "Frame-rate limit: " + (o.fpsLimit ? o.fpsLimit + " FPS" : "off"), icon: "display", run: () => gp.pickFps(o, set) });
            items.push({ label: "Cloud saves: " + onOff(o.cloudSaves !== false), icon: "refresh", on: o.cloudSaves !== false, run: () => set({ cloudSaves: o.cloudSaves === false }) });
            items.push({ label: "Launch options, mods and saves…", icon: "globe", sub: "In the web renderer for now", run: () => App.notHere("Launch options, environment variables, mods and saves") });
            App.openSheet({ title: x.title, sub: "Game settings. They apply the next time it starts.", items: items });
        }).catch(e => App.toast(e.message));
    }
    function pickProton(r) {
        const x = game; if (!x) return;
        const o = r.options || {};
        App.openSheet({ title: "Proton for " + x.title, items: (r.protonChoices || []).map(c => ({ label: c.label, sub: c.sub || "", icon: (o.proton || "") === c.id ? "check" : "layout", on: (o.proton || "") === c.id,
            run: () => App.call("game.options.set", { id: x.id, options: { proton: c.id } }, 8000).then(() => App.toast(x.title + " starts with " + c.label)).catch(e => App.toast(e.message)) })) });
    }
    function pickFps(o, set) {
        App.openSheet({ title: "Frame-rate limit", sub: "While this game plays.", items: [0, 30, 40, 45, 60, 90, 120].map(v => ({ label: v ? v + " FPS" : "Off", icon: (o.fpsLimit || 0) === v ? "check" : "display", on: (o.fpsLimit || 0) === v, run: () => set({ fpsLimit: v }) })) });
    }
    function aboutText() {
        const x = game; if (!x) return "";
        if (x.cloud) return "Streams with Xbox Cloud Gaming. Needs Game Pass Ultimate; sign in with your Xbox account the first time. (Streaming isn't in the Qt renderer yet: open the web renderer to play it.)";
        if (x.system) return "A " + x.system + " game from your ROMs folder, played with RetroArch.";
        if (x.inst) return "Installed through " + App.sourceName(x) + ".";
        if (App.isActive(x)) return (x.installKind === "nimbus" ? "Hearth is installing this game into your games folder." : App.sourceName(x) + " is installing this game.");
        if (x.note) return "In your " + App.sourceName(x) + " library. " + x.note + ", so Hearth can't install it.";
        if (x.installKind === "nimbus") return "In your " + App.sourceName(x) + " library. Select Install and Hearth downloads it, no launcher needed.";
        if (x.installKind === "steam") return "In your Steam library. Select Install and Steam downloads it in the background. Follow it under Downloads.";
        return "In your " + App.sourceName(x) + " library. Select Install and " + App.sourceName(x) + " will ask where to put it.";
    }
    function ago(ms) {
        const m = Math.floor((Date.now() - ms) / 60000);
        if (m < 60) return m <= 1 ? "Just now" : m + " min ago";
        const h = Math.floor(m / 60); if (h < 24) return h + " h ago";
        const d = Math.floor(h / 24); return d === 1 ? "Yesterday" : d < 30 ? d + " days ago" : new Date(ms).toLocaleDateString();
    }
    function hours(h) { return h < 1 ? Math.round(h * 60) + " min" : (h < 10 ? h.toFixed(1) : Math.round(h)) + " h"; }
    function size(b) { return b >= 1e9 ? (b / 1e9).toFixed(1) + " GB" : Math.round(b / 1e6) + " MB"; }
}
