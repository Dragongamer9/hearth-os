// Install Package: the window a .deb file opens in (double-click in Files, "Install" in its right-click menu, or any
// app opening a .deb: hearth-install-package.desktop is the handler for application/vnd.debian.binary-package).
// Shows what the package is (name, version, description, size, what it needs), installs it (on Peppermint while it's
// unlocked: on the system itself, like any Debian; otherwise into Hearth's Debian box) with live progress, then offers to open the apps it brought (they're in the app menu too); Remove and Reinstall
// when it's installed already. Says plainly what the Debian box is and what can't work in it.
// The work is done by hearth-debbox (Distrobox: a Debian 13 container sharing your home folder).
// Mouse, touch, keyboard or controller like Hearth Settings: Up/Down pick, A/Enter does it, B/Esc closes.
//   quickshell ipc -p <shell folder> call debs open <file.deb> | close
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Scope {
    id: inst
    property bool shown: false
    property string file: ""
    property var info: null                // hearth-debbox inspect
    property string loadError: ""
    property string phase: ""              // "" | working | done | failed | removed
    property string doing: ""              // what's running: install | remove
    property string error: ""
    property var apps: []                  // apps the package put in the menu
    property bool confirmRemove: false

    function open(path) {
        let p = (path || "").trim();
        if (p.startsWith("file://")) p = decodeURIComponent(p.slice(7));
        if (!p) return;
        if (worker.busy && shown) { SettingsNav.toast("Wait for the package that's being installed to finish."); return; }
        file = p; info = null; loadError = ""; phase = ""; error = ""; apps = []; confirmRemove = false;
        shown = true;
        // Focus on what to do (Install, or Open/Reinstall) once the package is read, not on Close.
        reader.run(["inspect", p], r => { inst.info = r; inst.apps = r.apps || []; if (SettingsNav.debNav) Qt.callLater(() => SettingsNav.debNav.focusFirst()); },
            e => inst.loadError = e);
    }
    function close() { if (!worker.busy) shown = false; }
    function install() {
        if (!info || worker.busy) return;
        phase = "working"; doing = "install"; error = ""; confirmRemove = false;
        worker.run(["install", file], r => { inst.phase = "done"; inst.apps = r.apps || []; inst.refresh(); SettingsNav.toast(inst.title() + " is installed"); },
            e => { inst.phase = "failed"; inst.error = e; });
    }
    function remove() {
        if (!info || worker.busy) return;
        phase = "working"; doing = "remove"; error = ""; confirmRemove = false;
        worker.run(["remove", info.package], r => { inst.phase = "removed"; inst.apps = []; inst.refresh(); SettingsNav.toast(inst.title() + " was removed"); },
            e => { inst.phase = "failed"; inst.error = e; });
    }
    function refresh() { reader.run(["inspect", file], r => { inst.info = r; if (inst.phase !== "done") inst.apps = r.apps || []; }); }
    function openApp(a) { worker.run(["open", a.id], () => SettingsNav.toast("Opening " + a.name), e => SettingsNav.toast(e)); }
    function title() { return info ? (info.package || "the package") : file.replace(/^.*\//, ""); }
    onShownChanged: SettingsNav.debShown = shown
    Component.onCompleted: SettingsNav.debs = inst

    SetJob { id: reader; helper: "hearth-debbox" }
    SetJob { id: worker; helper: "hearth-debbox" }

    IpcHandler {
        target: "debs"
        function open(path: string): void { inst.open(path); }
        function close(): void { inst.close(); }
    }

    LazyLoader {
        active: inst.shown
        FloatingWindow {
            id: win
            title: "Install Package"
            implicitWidth: 760
            implicitHeight: 700
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) inst.shown = false
            Component.onCompleted: { SettingsNav.debNav = page; Qt.callLater(() => page.focusFirst()); }
            Component.onDestruction: if (SettingsNav.debNav === page) SettingsNav.debNav = null

            SetPage {
                id: page
                anchors.fill: parent
                title: ""
                function back() { inst.close(); }

                // ---- what it is
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 16
                    Rectangle {
                        Layout.preferredWidth: 72; Layout.preferredHeight: 72; radius: 18; color: Theme.panel3
                        // Hearth's Debian box icon (hearth-debbox in the icon set), the store glyph if it's not installed.
                        Image { id: boxIcon; anchors.fill: parent; source: AppIcons.themed("hearth-debbox"); sourceSize: Qt.size(144, 144); fillMode: Image.PreserveAspectFit; visible: status === Image.Ready }
                        Icon { anchors.centerIn: parent; visible: !boxIcon.visible; name: "store"; size: 38; color: Theme.accent }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text { Layout.fillWidth: true; text: inst.info ? inst.info.package : inst.file.replace(/^.*\//, ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold; wrapMode: Text.WrapAnywhere }
                        Text { Layout.fillWidth: true; visible: !!inst.info; text: inst.info ? "Version " + inst.info.version + (inst.info.installed ? " · installed" + (inst.info.installedVersion && inst.info.installedVersion !== inst.info.version ? " (" + inst.info.installedVersion + ")" : "") : "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                        Text { Layout.fillWidth: true; visible: text !== ""; text: inst.info ? inst.info.summary : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; wrapMode: Text.WordWrap }
                    }
                }
                Text { visible: !inst.info && inst.loadError === ""; text: "Reading the package…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                SetRow {
                    visible: inst.loadError !== ""
                    danger: true; chevron: false; icon: "info"
                    title: "This file can't be installed"; sub: inst.loadError
                    onActivated: inst.close()
                }
                Repeater {
                    model: inst.info ? inst.info.warnings : []
                    delegate: SetRow { required property var modelData; danger: true; chevron: false; interactive: false; icon: "info"; title: "Heads up"; sub: modelData }
                }

                // ---- progress, done, failed
                ColumnLayout {
                    visible: inst.phase === "working"
                    Layout.fillWidth: true
                    Layout.topMargin: 6
                    spacing: 6
                    Text {
                        Layout.fillWidth: true
                        visible: inst.doing === "install" && !!inst.info && !inst.info.boxReady
                        text: "First time: setting up Hearth's Debian box. That takes 1–2 minutes and downloads about 100 MB; later installs are quicker."
                        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                    }
                    Text { Layout.fillWidth: true; text: (worker.line || (inst.doing === "remove" ? "Removing…" : "Installing…")) + (worker.progress >= 0 ? "  " + Math.round(worker.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
                        Rectangle {
                            width: parent.width * Math.max(0.03, worker.progress); height: parent.height; radius: 4
                            gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: Theme.ember } GradientStop { position: 1; color: Theme.flame } }
                        }
                    }
                }
                SetRow {
                    visible: inst.phase === "done"
                    icon: "check"; chevron: false; interactive: false
                    title: inst.title() + " is installed"
                    sub: inst.apps.length ? "Its apps are in your app menu too." : "It has no app for the menu. Use it from a terminal: distrobox enter hearth-debian"
                }
                SetRow {
                    visible: inst.phase === "removed"
                    icon: "check"; chevron: false; interactive: false
                    title: inst.title() + " was removed"; sub: "Its apps are gone from your app menu."
                }
                SetRow {
                    visible: inst.phase === "failed"
                    danger: true; chevron: false; icon: "info"
                    title: inst.doing === "remove" ? "Couldn't remove it" : "Couldn't install it"
                    sub: inst.error + "  (A: try again)"
                    onActivated: inst.doing === "remove" ? inst.remove() : inst.install()
                }

                // ---- what you can do
                Repeater {
                    model: inst.phase === "working" ? [] : inst.apps
                    delegate: SetRow {
                        required property var modelData
                        icon: "open"; chevron: false
                        title: "Open " + modelData.name
                        sub: modelData.native ? "On the system" : "From Hearth's Debian box"
                        onActivated: inst.openApp(modelData)
                    }
                }
                SetRow {
                    visible: !!inst.info && inst.info.installable && !inst.info.installed && inst.phase !== "working" && inst.phase !== "done"
                    icon: "update"; chevron: false; title: "Install"
                    sub: inst.info && inst.info.native ? "On the system, for everyone (asks for an administrator's password)"
                        : "Into Hearth's Debian box, for you" + (inst.info && !inst.info.boxReady ? " (the box is set up first: 1–2 minutes)" : "")
                    onActivated: inst.install()
                }
                SetRow {
                    visible: !!inst.info && inst.info.installable && inst.info.installed && inst.phase !== "working"
                    icon: "update"; chevron: false; title: "Reinstall"
                    sub: inst.info ? "Install version " + inst.info.version + " from this file again" : ""
                    onActivated: inst.install()
                }
                SetRow {
                    visible: !!inst.info && inst.info.installed && inst.phase !== "working" && !inst.confirmRemove
                    icon: "trash"; danger: true; chevron: false; title: "Remove…"
                    onActivated: inst.confirmRemove = true
                }
                ColumnLayout {
                    visible: inst.confirmRemove
                    Layout.fillWidth: true
                    spacing: 8
                    SetRow { danger: true; chevron: false; icon: "trash"; title: "Remove " + inst.title(); sub: "Its apps leave your app menu. Your files and its settings in your home folder stay."; onActivated: inst.remove() }
                    SetRow { chevron: false; title: "Cancel"; onActivated: inst.confirmRemove = false }
                }
                SetRow {
                    visible: inst.phase !== "working"
                    icon: "back"; chevron: false; title: "Close"
                    onActivated: inst.close()
                }

                // ---- about the package and the box
                SetHeader { visible: !!inst.info; text: "About" }
                SetRow {
                    visible: !!inst.info
                    icon: "info"; chevron: false; interactive: true
                    title: inst.info && inst.info.native ? "Installs on the system" : "Hearth's Debian box"
                    sub: inst.info ? inst.info.limits : ""
                }
                SetBase {
                    visible: !!inst.info && inst.info.description !== ""
                    Layout.fillWidth: true
                    implicitHeight: desc.implicitHeight + 24
                    color: activeFocus ? Theme.panel2 : "transparent"
                    Text {
                        id: desc
                        anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 14 }
                        text: inst.info ? inst.info.description : ""
                        textFormat: Text.PlainText; wrapMode: Text.WordWrap
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 14; lineHeight: 1.2
                    }
                }
                SetRow {
                    visible: !!inst.info
                    icon: "info"; chevron: false
                    title: "Size"
                    sub: inst.info ? [inst.info.fileSizeText ? "Download: " + inst.info.fileSizeText : "", inst.info.installedSizeText ? "Installed: " + inst.info.installedSizeText : "", "Debian packages it needs are downloaded too"].filter(x => x).join(" · ") : ""
                }
                SetRow {
                    visible: !!inst.info
                    icon: "grid"; chevron: false
                    title: inst.info ? (inst.info.depends.length ? "Needs " + inst.info.depends.length + (inst.info.depends.length === 1 ? " other package" : " other packages") : "Needs no other packages") : ""
                    sub: inst.info ? inst.info.depends.slice(0, 30).join(", ") + (inst.info.depends.length > 30 ? ", …" : "") : ""
                }
                SetRow {
                    visible: !!inst.info
                    icon: "info"; chevron: false
                    title: "Details"
                    sub: inst.info ? [inst.info.architecture ? "For: " + inst.info.architecture : "", inst.info.section ? "Section: " + inst.info.section : "", inst.info.maintainer ? "Made by: " + inst.info.maintainer : "", inst.file].filter(x => x).join(" · ") : ""
                }
                SetRow {
                    visible: !!inst.info && (inst.info.homepage || "") !== ""
                    icon: "globe"; title: "Website"; sub: inst.info ? inst.info.homepage : ""
                    onActivated: Quickshell.execDetached(["xdg-open", inst.info.homepage])
                }
            }
        }
    }
}
