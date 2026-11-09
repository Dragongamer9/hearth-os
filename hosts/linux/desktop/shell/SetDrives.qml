// Hearth Settings › System › Drives & folders (docs/HEARTH-FILES.md "Hearth (C:) the Windows way", the owner's mockup):
// how Hearth Files shows Hearth's drive (Windows folder names, the Linux folders too, what the address bar shows,
// what Copy path gives) and the drive letters (C: is always Hearth; Change letter… for the others, the same as This
// PC's menu: hearth-files letter-set).
// These are Hearth Files' own settings (~/.config/nimbus/files.json keys cdrive, linuxFolders, addressBar, copyPath),
// read here and written only when you change one: hearth-files prefs-set merges that one key under the lock
// (hearth_settings_io: atomic, a .bak, never defaults over a file it couldn't read). Opening this page writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Drives & folders"
    SetJob { id: filesJob; helper: "hearth-files" }
    SetJob { id: pcJob; helper: "hearth-files" }
    // What files.json says (defaults filled in here only: Windows names on, the Linux folders off).
    property bool cdrive: true
    property bool linuxFolders: false
    property string addressBar: "windows"
    property string copyPath: "windows"
    property bool loaded: false
    property var drives: []
    property var taken: ({})
    property string user: ""
    property string homeDir: Quickshell.env("HOME") || "/home/you"
    property string opened: ""              // the drive whose letters are shown

    function load() {
        filesJob.run(["prefs"], r => {
            r = r || {};
            page.cdrive = r.cdrive !== false;
            page.linuxFolders = r.linuxFolders === true;
            page.addressBar = r.addressBar === "linux" ? "linux" : "windows";
            page.copyPath = r.copyPath === "linux" ? "linux" : "windows";
            page.loaded = true;
        }, e => SettingsNav.toast(e));
        loadDrives();
    }
    function loadDrives() {
        pcJob.run(["this-pc"], r => {
            page.user = (r && r.user) || "";
            page.taken = (r && r.taken) || {};
            page.replaceList("drives", ((r && r.drives) || []).concat((r && r.network) || []).filter(d => d.letter));
        }, e => SettingsNav.toast(e));
    }
    Component.onCompleted: load()
    // Only the key you changed is merged into files.json (and only when it really changed).
    function save(k, v) {
        if (!page.loaded || page[k] === v) return;
        page[k] = v;
        const o = {};
        o[k] = v;
        filesJob.run(["prefs-set", JSON.stringify(o)], () => {}, e => { SettingsNav.toast(e); page.load(); });
    }
    function freeLetters(d) {
        const used = {};
        for (const k in page.taken) used[k] = true;
        for (const q of page.drives) if (q.letter && q.key !== d.key) used[q.letter] = true;
        return "DEFGHIJKLMNOPQRSTUVWXY".split("").filter(l => l === d.letter || !used[l]);
    }
    function setLetter(d, L) {
        if (!d || L === d.letter) return;
        pcJob.run(["letter-set", d.key, L], () => { SettingsNav.toast("“" + d.name + "” is " + L + ": now (Windows apps see it there from their next start)"); page.loadDrives(); },
                  e => SettingsNav.toast(e));
    }
    function info(d) {
        if (d.mounted === false) return (d.sub || "Not plugged in") + " · keeps " + d.letter + ": for when it’s back";
        if (d.total && d.free !== undefined && d.free !== null) return gb(d.free) + " free of " + gb(d.total);
        return d.sub || "";
    }
    function gb(n) {
        if (!n && n !== 0) return "";
        const u = ["bytes", "KB", "MB", "GB", "TB"];
        let v = n, i = 0;
        while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
        return (v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)) + " " + u[i];
    }

    SetCardGroup {
        title: "How Files shows Hearth’s drive"
        SetSwitch {
            icon: "drive"; title: "Show Hearth (C:) with Windows folder names"
            sub: "Program Files, Users, Windows… Only names: nothing on your drive moves."
            checked: page.cdrive
            onToggled: v => page.save("cdrive", v)
        }
        SetSwitch {
            icon: "folder"; title: "Show the Linux folders too"
            sub: "etc, usr, var and the rest, under their own heading on Hearth (C:)"
            checked: page.linuxFolders
            enabled: page.cdrive
            opacity: page.cdrive ? 1 : 0.5
            onToggled: v => page.save("linuxFolders", v)
        }
        SetChoice {
            icon: "edit"; title: "The address bar shows"
            sub: "Typing either kind always works"
            options: [{ value: "windows", label: "Windows names (C:\\…)" }, { value: "linux", label: "Linux paths (/…)" }]
            current: page.addressBar
            enabled: page.cdrive
            opacity: page.cdrive ? 1 : 0.5
            onChosen: v => page.save("addressBar", v)
        }
        SetChoice {
            icon: "link"; title: "Copy path gives"
            sub: "The other one is always in the right-click menu"
            options: [{ value: "windows", label: "C:\\Users\\" + (page.user || "you") + "\\Documents" },
                      { value: "linux", label: page.homeDir + "/Documents" }]
            current: page.copyPath
            onChosen: v => page.save("copyPath", v)
        }
    }

    SetCardGroup {
        title: "Drive letters"
        SetRow {
            icon: "drive"; title: "C:  Hearth"
            sub: "The drive Hearth runs from · always C:"
            value: "Always C:"
            interactive: false
        }
        Repeater {
            model: page.drives.filter(d => d.letter !== "C")
            delegate: SetExpander {
                id: dx
                required property var modelData
                icon: modelData.kind === "network" ? "server" : "drive"
                title: modelData.letter + ":  " + modelData.name
                sub: page.info(modelData)
                value: "Change letter…"
                SetChoice {
                    title: "Letter for “" + dx.modelData.name + "”"
                    sub: "Only free letters. Z: is how Windows apps see the whole system, so it's left alone."
                    options: page.freeLetters(dx.modelData).map(l => ({ value: l, label: l + ":" }))
                    current: dx.modelData.letter
                    onChosen: v => page.setLetter(dx.modelData, v)
                }
            }
        }
        Text {
            visible: page.drives.filter(d => d.letter !== "C").length === 0
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 6
            wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            text: "Other drives get the next free letter (D: to Y:) the first time they’re plugged in, and keep it."
        }
    }

    SetCardGroup {
        title: ""
        SetRow {
            icon: "info"
            title: "Only names"
            sub: "These only change how Hearth names things. Your files, apps and games stay exactly where they are, and Windows apps keep their own C: inside them."
            interactive: false
        }
    }
}
