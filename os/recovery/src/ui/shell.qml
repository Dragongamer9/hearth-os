// Hearth Recovery: the screen of the recovery USB stick. Fullscreen on its own (labwc + Quickshell, as root),
// driven by a controller, the touchscreen, or the handheld's own buttons (Volume Down/Up move, Power selects), so
// it works on a handheld with no keyboard. Every action is a hearth-recovery command (../hearth-recovery).
//
// Buttons: D-pad / left stick / Volume move · A (or Power) selects · B goes back · X deletes (keyboard) ·
// Y switches capitals (keyboard) · Menu connects (keyboard). Destructive actions are "hold A for 3 seconds".
// From a terminal or a test: quickshell ipc -p <this folder> call recovery key <up|down|left|right|a|b|x|y|menu>
//   recovery press a / release a (hold), recovery state (JSON of what's on screen), recovery tap <index>
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
    id: app

    // ---------------------------------------------------------------- colors (Hearth's own, desktop/shell/Theme.qml)
    readonly property color bg: "#0f0b09"
    readonly property color panel: "#1c1512"
    readonly property color panel2: "#261d18"
    readonly property color panel3: "#33271f"
    readonly property color line: "#3b2e26"
    readonly property color text: "#f4ece3"
    readonly property color muted: "#a69484"
    readonly property color danger: "#ff5d5d"
    readonly property color ember: "#ff8a3d"
    readonly property color flame: "#ffd166"
    readonly property color good: "#7bd389"
    readonly property string iconDir: "file:///usr/share/nimbus/desktop/shell/icons/"

    // ---------------------------------------------------------------- state
    readonly property string bin: Quickshell.env("HEARTH_RECOVERY_BIN") || "/usr/libexec/hearth-recovery/hearth-recovery"
    readonly property string padHelper: Quickshell.env("HEARTH_RECOVERY_PAD") || "/usr/libexec/hearth-recovery/recovery-pad"
    property var systems: []
    property int sysIdx: 0
    readonly property var sys: systems.length > sysIdx ? systems[sysIdx] : null
    property bool scanned: false
    property string scanError: ""
    property string screen: "scan"        // scan home versions confirm work done wifi osk users drives power systems
    property var dialog: null             // confirm: {title, body, confirm, danger, hold, run, kind}
    property var work: ({ title: "", text: "", pct: -1, busy: false, ok: false, error: "", result: null, kind: "" })
    property var net: ({ online: false, connection: "", connectivity: "unknown" })
    property var wifi: ({ busy: false, networks: [], error: "" })
    property var drives: []
    property string pickUser: ""
    property string wifiSsid: ""
    property string pw: ""
    property bool showPw: false
    property bool shift: false
    property bool sym: false
    property int oskRow: 1
    property int oskCol: 0
    property int pads: 0
    property string padProblem: ""
    property int sel: 0
    property var history: []              // screens to go back to
    property real hold: 0                 // 0..1 while A is held on a "hold" item
    property bool holding: false
    readonly property bool busy: work.busy || wifi.busy || !scanned

    function dep(i) { return sys && sys.deployments && sys.deployments.length > i ? sys.deployments[i] : null; }
    function depName(d) {
        if (!d) return "";
        let v = d.hearthVersion ? "Hearth " + d.hearthVersion : (d.osName || "Hearth OS");
        return v;
    }
    function depWhen(d) {
        if (!d) return "";
        let parts = [];
        if (d.builtText) parts.push("built " + d.builtText);
        if (d.deployedText) parts.push("installed " + d.deployedText);
        return parts.join(" · ");
    }
    function gb(n) { return n > 0 ? (n / 1e9).toFixed(n > 1e11 ? 0 : 1) + " GB" : ""; }

    // ---------------------------------------------------------------- what each screen shows: {title, body, big, items}
    // Every screen is the same layout: a title, some text, maybe a big code, and a list of choices. Items:
    // {label, sub, icon, act, danger, hold, off}
    function build(screen, systems, sysIdx, dialog, work, wifi, net, drives, pickUser, scanned) {
        const s = systems.length > sysIdx ? systems[sysIdx] : null;
        const items = [];
        if (screen === "scan") {
            if (!scanned) return { title: "Looking for Hearth OS on this PC…", body: "Checking the drives. This takes a few seconds.", items: [] };
            items.push({ label: "Look again", sub: "Plugged in a drive, or it wasn't found?", icon: "search", act: "rescan" });
            items.push({ label: "Wi-Fi", sub: net.online ? "Connected" + (net.connection ? " to " + net.connection : "") : "Not connected", icon: "wifi", act: "wifi" });
            items.push({ label: "Restart", sub: "", icon: "restart", act: "restart" });
            items.push({ label: "Power off", sub: "", icon: "power", act: "poweroff" });
            return { title: "No Hearth OS found", body: (app.scanError ? app.scanError + "\n\n" : "") + "No drive in this PC has Hearth OS installed on it (this USB stick itself doesn't count). If Hearth OS is on a drive that's switched off or unplugged, connect it and look again.", items: items };
        }
        if (screen === "home") {
            const cur = dep(0), prev = dep(1);
            items.push({ label: "Start the previous version", icon: "back", act: "previous", off: !prev,
                         sub: prev ? depName(prev) + " (" + (prev.deployedText || prev.builtText || "older") + ") instead of " + depName(cur) : "There's no previous version on this drive" });
            items.push({ label: "Start in Desktop Mode next time", icon: "window", act: "desktop",
                         sub: "Skips Game Mode once it starts, and resets the display settings that can turn the screen black" });
            items.push({ label: "Reset Game Mode's display settings", icon: "display", act: "resetdisplay",
                         sub: "Renderer, resolution and refresh rate back to automatic. Your games and other settings stay" });
            items.push({ label: "Send logs to the developer", icon: "help", act: "logs",
                         sub: "The installed system's logs from its last starts. You get a code to share" + (net.online ? "" : " · needs Wi-Fi") });
            items.push({ label: "Update or reinstall Hearth OS", icon: "update", act: "update",
                         sub: "Puts the newest " + (s ? s.editionLabel : "Hearth OS") + " on the drive. Keeps your games, files and settings" + (net.online ? "" : " · needs Wi-Fi") });
            items.push({ label: "Copy my files to a USB drive", icon: "folder", act: "copy", sub: "Your home folder (not games), onto another USB drive" });
            items.push({ label: "Wi-Fi", icon: "wifi", act: "wifi", sub: net.online ? "Connected" + (net.connection ? " to " + net.connection : "") : "Not connected" });
            items.push({ label: "Factory reset", icon: "trash", act: "factory", danger: true, sub: "Erases the whole drive and installs Hearth OS again. Everything on it is lost" });
            if (systems.length > 1) items.push({ label: "Choose another Hearth OS", icon: "file", act: "systems", sub: systems.length + " found on this PC" });
            items.push({ label: "Restart", icon: "restart", act: "restart", sub: "Into the Hearth OS on the drive (take the USB stick out if this screen comes back)" });
            items.push({ label: "Power off", icon: "power", act: "poweroff", sub: "" });
            return { title: "What would you like to do?", body: "", items: items, card: true };
        }
        if (screen === "versions") {
            for (let i = 1; s && i < s.deployments.length; i++) {
                const d = s.deployments[i];
                items.push({ label: depName(d) + (d.osVersion ? " · " + d.osVersion : ""), sub: depWhen(d) || d.title, icon: "back", act: "version:" + i });
            }
            items.push({ label: "Back", icon: "close", act: "back", sub: "" });
            return { title: "Which version should start?", body: "The version that starts now is " + depName(dep(0)) + ". Pick one to start instead (it stays the default until the next update).", items: items };
        }
        if (screen === "confirm" && dialog) {
            items.push({ label: dialog.confirm, sub: dialog.hold ? "Hold A (or touch and hold) for 3 seconds" : "", icon: dialog.danger ? "trash" : "check", act: "go", danger: !!dialog.danger, hold: !!dialog.hold });
            items.push({ label: "Cancel", icon: "close", act: "back", sub: "" });
            return { title: dialog.title, body: dialog.body, items: items };
        }
        if (screen === "work") {
            return { title: work.title, body: work.text, items: [], progress: work.pct };
        }
        if (screen === "done") {
            const ok = work.ok;
            let big = "";
            let body = work.error;
            if (ok && work.result) {
                const r = work.result;
                if (work.kind === "logs") { big = r.code; body = "Share this code with the developer (Discord, GitHub or e-mail). The logs are kept for 14 days. A copy is on this USB stick at " + r.path + "."; }
                else if (work.kind === "previous") body = "Next time it starts " + depName(r["default"]) + (r["default"].deployedText ? " (installed " + r["default"].deployedText + ")" : "") + ". The boot menu was read back and starts that version first." + (r.grubMenu && r.grubMenu.length ? "\n\nBoot menu: " + r.grubMenu.join(" · ") : "");
                else if (work.kind === "desktop" || work.kind === "resetdisplay") {
                    const lines = [];
                    for (const u of r.users) lines.push(u.user + ": " + (u.changed.length ? u.changed.join(", ") : "nothing needed changing"));
                    body = (work.kind === "desktop" ? "Hearth starts in Desktop Mode next time. Switch back to Game Mode from the Hearth menu once you're in.\n\n" : "Game Mode's display settings are back to automatic.\n\n") + lines.join("\n") + (r.users.length && r.users[0].backup ? "\n\nThe old files are in " + r.users[0].backup : "");
                }
                else if (work.kind === "update") body = r.same ? "The newest " + r.image + " is the version you already had; it was put in place again." : "Installed " + depName(r["new"]) + (r["new"].builtText ? " (built " + r["new"].builtText + ")" : "") + ". It starts next time; the version before stays as the previous one.";
                else if (work.kind === "copy") body = "Copied to the folder " + r.folder + " on the USB drive." + (r.partial ? " Some files couldn't be copied (names the drive can't hold, or files that changed)." : "") + " Games, caches and Steam's folders were left out.";
                else if (work.kind === "factory") body = "Hearth OS is installed again. Sign in as " + r.user + " (no password: set one in Settings › Users). Take the USB stick out and restart.";
                else body = "Done.";
            }
            items.push({ label: "Back to the menu", icon: "back", act: "menu", sub: "" });
            if (ok && ["previous", "desktop", "resetdisplay", "update", "factory"].indexOf(work.kind) >= 0) items.unshift({ label: "Restart now", icon: "restart", act: "restart", sub: "Into the Hearth OS on the drive" });
            if (!ok && (work.kind === "logs" || work.kind === "update" || work.kind === "factory") && !net.online) items.unshift({ label: "Wi-Fi", icon: "wifi", act: "wifi", sub: "Not connected" });
            items.push({ label: "Power off", icon: "power", act: "poweroff", sub: "" });
            return { title: ok ? (work.doneTitle || "Done") : "That didn't work", body: body, big: big, items: items, good: ok, bad: !ok };
        }
        if (screen === "wifi") {
            if (wifi.busy) return { title: "Wi-Fi", body: "Looking for networks…", items: [], progress: -1 };
            for (const n of wifi.networks) items.push({ label: n.ssid, sub: (n.connected ? "Connected · " : "") + n.signal + "%" + (n.secure ? " · needs a password" : " · open"), icon: n.connected ? "check" : (n.secure ? "lock" : "wifi"), act: "net:" + n.ssid });
            items.push({ label: "Look again", sub: "", icon: "search", act: "wifiscan" });
            items.push({ label: "Back", sub: "", icon: "close", act: "back" });
            return { title: "Wi-Fi", body: (wifi.error ? wifi.error + "\n" : "") + (net.online ? "Connected" + (net.connection ? " to " + net.connection : "") + "." : "Not connected. Pick a network (a cable works too).") , items: items };
        }
        if (screen === "users") {
            for (const u of (s ? s.users : [])) items.push({ label: u.full || u.name, sub: u.home, icon: "user", act: "user:" + u.name });
            items.push({ label: "Back", sub: "", icon: "close", act: "back" });
            return { title: "Whose files?", body: "", items: items };
        }
        if (screen === "drives") {
            for (const d of drives) items.push({ label: d.label || d.model, sub: d.part + " · " + d.fstype + " · " + gb(d.size), icon: "folder", act: "drive:" + d.part });
            items.push({ label: "Look again", sub: "Plug the USB drive in, then look again", icon: "search", act: "drivescan" });
            items.push({ label: "Back", sub: "", icon: "close", act: "back" });
            return { title: "Copy to which USB drive?", body: drives.length ? "A folder named Hearth-files-… is made on it; nothing on it is deleted." : "No USB drive found. Plug one in (not this recovery stick).", items: items };
        }
        if (screen === "systems") {
            for (let i = 0; i < systems.length; i++) items.push({ label: systems[i].editionLabel + " · " + systems[i].model, sub: systems[i].device + " · " + (systems[i].deployments.length ? depName(systems[i].deployments[0]) : ""), icon: i === sysIdx ? "check" : "file", act: "sys:" + i });
            items.push({ label: "Back", sub: "", icon: "close", act: "back" });
            return { title: "Which Hearth OS?", body: "", items: items };
        }
        if (screen === "osk") return { title: "Password for " + app.wifiSsid, body: "", items: [], osk: true };
        return { title: "", body: "", items: [] };
    }
    readonly property var pg: build(screen, systems, sysIdx, dialog, work, wifi, net, drives, pickUser, scanned)
    onPgChanged: if (sel >= pg.items.length) sel = Math.max(0, pg.items.length - 1)

    // ---------------------------------------------------------------- navigation
    function go(to) { history = history.concat([screen]); screen = to; sel = firstOn(); hold = 0; holding = false; }
    function firstOn() { const it = build(screen, systems, sysIdx, dialog, work, wifi, net, drives, pickUser, scanned).items; for (let i = 0; i < it.length; i++) if (!it[i].off) return i; return 0; }
    function back() {
        if (work.busy) return;
        if (screen === "osk") { screen = "wifi"; sel = 0; return; }
        if (history.length === 0) return;
        const h = history.slice(); screen = h.pop(); history = h; sel = 0; hold = 0; holding = false;
    }
    function menu() { history = []; screen = systems.length ? "home" : "scan"; sel = 0; }
    function move(d) {
        const n = pg.items.length;
        if (!n) return;
        let i = sel;
        for (let k = 0; k < n; k++) { i = (i + d + n) % n; if (!pg.items[i].off) break; }
        sel = i; hold = 0; holding = false;
    }
    function key(k) {
        if (screen === "osk") return oskKey(k);
        if (k === "up" || k === "left") move(-1);
        else if (k === "down" || k === "right") move(1);
        else if (k === "a") { const it = pg.items[sel]; if (it && !it.hold) activate(sel); }
        else if (k === "b") back();
        return "ok";
    }
    function press() { const it = pg.items[sel]; if (it && it.hold && !busy) { holding = true; hold = 0; holdTimer.start(); } }
    function release() { holding = false; holdTimer.stop(); if (hold < 1) hold = 0; }
    Timer { id: holdTimer; interval: 50; repeat: true; onTriggered: { if (!app.holding) { stop(); return; } app.hold = Math.min(1, app.hold + 50 / 3000); if (app.hold >= 1) { stop(); app.holding = false; app.activate(app.sel); } } }

    function activate(i) {
        const it = pg.items[i];
        if (!it || it.off || (busy && screen !== "wifi" && screen !== "scan")) return;
        sel = i;
        const a = it.act;
        if (a === "back") back();
        else if (a === "menu") menu();
        else if (a === "rescan") rescan();
        else if (a === "wifi") { go("wifi"); wifiScan(); }
        else if (a === "wifiscan") wifiScan();
        else if (a.startsWith("net:")) {
            const ssid = a.slice(4);
            const n = wifi.networks.find(x => x.ssid === ssid);
            if (n && n.secure && !n.connected) { wifiSsid = ssid; pw = ""; shift = false; sym = false; oskRow = 1; oskCol = 0; screen = "osk"; }
            else wifiConnect(ssid, "");
        }
        else if (a === "previous") {
            if (sys.deployments.length > 2) go("versions");
            else confirmPrevious(1);
        }
        else if (a.startsWith("version:")) confirmPrevious(parseInt(a.slice(8)));
        else if (a === "desktop") ask({ kind: "desktop", title: "Start in Desktop Mode next time?", confirm: "Start in Desktop Mode", doneTitle: "Desktop Mode next time",
            body: "When this Hearth OS starts, it opens Desktop Mode instead of Game Mode. Game Mode's renderer goes back to automatic, and its saved resolution, refresh rate and screen modes are cleared (a copy of the old files is kept). Games, files, accounts and every other setting stay.",
            run: ["reset-display", sys.id, "--desktop"] });
        else if (a === "resetdisplay") ask({ kind: "resetdisplay", title: "Reset Game Mode's display settings?", confirm: "Reset display settings", doneTitle: "Display settings reset",
            body: "Renderer back to automatic; Game Mode's resolution, refresh rate and saved screen modes cleared (a copy of the old files is kept). Hearth still starts in Game Mode. Everything else stays.",
            run: ["reset-display", sys.id] });
        else if (a === "logs") ask({ kind: "logs", title: "Send logs to the developer?", confirm: "Send logs", doneTitle: "Logs sent",
            body: "Sends the installed Hearth OS's system journal from its last three starts and Hearth's own logs. Names, e-mail addresses, passwords and sign-in tokens are taken out first. You get a code to share." + (net.online ? "" : "\n\nConnect to Wi-Fi first (Wi-Fi on the main screen)."),
            run: ["send-logs", sys.id, "Sent from Hearth Recovery"] });
        else if (a === "update") ask({ kind: "update", title: "Update or reinstall Hearth OS?", confirm: "Update Hearth OS", doneTitle: "Hearth OS updated",
            body: "Downloads the newest " + sys.editionLabel + " and puts it on " + sys.model + " as the version that starts next. Your games, files, accounts and settings stay (they're kept apart from the system). The version that starts now becomes the previous one. This needs Wi-Fi and takes a while (a few GB)." + (net.online ? "" : "\n\nConnect to Wi-Fi first."),
            run: ["update", sys.id] });
        else if (a === "copy") {
            if (sys.users.length === 1) { pickUser = sys.users[0].name; go("drives"); driveScan(); }
            else go("users");
        }
        else if (a.startsWith("user:")) { pickUser = a.slice(5); go("drives"); driveScan(); }
        else if (a === "drivescan") driveScan();
        else if (a.startsWith("drive:")) {
            const part = a.slice(6);
            ask({ kind: "copy", title: "Copy " + pickUser + "'s files?", confirm: "Copy files", doneTitle: "Files copied",
                  body: "Copies the home folder of " + pickUser + " onto " + part + " (games, caches and Steam's folders are left out). Nothing on the Hearth OS drive changes.",
                  run: ["copy-home", sys.id, pickUser, part] });
        }
        else if (a === "factory") ask({ kind: "factory", title: "Erase everything and reinstall?", confirm: "Erase " + sys.model, danger: true, hold: true, doneTitle: "Hearth OS reinstalled",
            body: "This erases the WHOLE drive " + sys.model + " (" + sys.disk + "): every game, file, account and setting on it, and any other system on that drive. Then it downloads " + sys.editionLabel + " and installs it (needs Wi-Fi). The account is made again as " + (sys.users.length ? sys.users[0].name : "hearth") + " with no password.\n\nCopy your files to a USB drive first if you need them.",
            run: ["factory-reset", sys.id] });
        else if (a === "systems") go("systems");
        else if (a.startsWith("sys:")) { sysIdx = parseInt(a.slice(4)); menu(); }
        else if (a === "go") runDialog();
        else if (a === "restart") { run(["restart"].concat(sys ? [sys.id] : []), "power", "Restarting…"); }
        else if (a === "poweroff") { run(["poweroff"], "power", "Turning off…"); }
    }
    function confirmPrevious(i) {
        const d = dep(i);
        ask({ kind: "previous", title: "Start " + depName(d) + " next time?", confirm: "Start " + depName(d), doneTitle: "The previous version starts next",
              body: "The drive starts " + depName(d) + (d.deployedText ? " (installed " + d.deployedText + ")" : "") + " instead of " + depName(dep(0)) + (dep(0).deployedText ? " (installed " + dep(0).deployedText + ")" : "") + ". Nothing is deleted: the newer version stays on the drive, and the next update replaces it. Your games, files and settings stay.",
              run: ["set-default", sys.id, String(i)] });
    }
    function ask(d) { dialog = d; go("confirm"); }

    // ---------------------------------------------------------------- running hearth-recovery
    property var jobLines: []
    function run(args, kind, title) {
        work = { title: title, text: "", pct: -1, busy: true, ok: false, error: "", result: null, kind: kind, doneTitle: dialog ? dialog.doneTitle : "" };
        if (kind !== "power") { history = []; screen = "work"; }
        job.kind = kind;
        job.result = null;
        job.error = "";
        job.command = [app.bin].concat(args);
        job.running = true;
    }
    function runDialog() {
        if (!dialog) return;
        const titles = { previous: "Changing which version starts…", desktop: "Changing the settings…", resetdisplay: "Changing the settings…", logs: "Collecting the logs…",
                         update: "Updating Hearth OS…", copy: "Copying your files…", factory: "Reinstalling Hearth OS…" };
        run(dialog.run, dialog.kind, titles[dialog.kind] || "Working…");
    }
    Process {
        id: job
        property string kind: ""
        property var result: null
        property string error: ""
        stdout: SplitParser {
            onRead: data => {
                const l = data.trim();
                if (l.startsWith("stage: ")) {
                    try { const st = JSON.parse(l.slice(7)); app.work = Object.assign({}, app.work, { text: st.text, pct: st.pct }); } catch (e) {}
                } else if (l.startsWith("result: ")) {
                    try { job.result = JSON.parse(l.slice(8)); } catch (e) {}
                } else if (l.startsWith("error: ")) {
                    job.error = l.slice(7);
                } else if (l.startsWith("{")) {
                    try { job.result = JSON.parse(l); } catch (e) {}
                }
            }
        }
        stderr: SplitParser { onRead: data => console.warn("hearth-recovery:", data) }
        onExited: code => {
            const ok = code === 0 && job.result !== null && job.error === "";
            const err = job.error ? job.error.charAt(0).toUpperCase() + job.error.slice(1) + "." : (ok ? "" : "It stopped (code " + code + ").");
            app.work = Object.assign({}, app.work, { busy: false, ok: ok, error: err, result: job.result });
            if (job.kind === "power") return;
            app.screen = "done"; app.sel = 0;
            // What changed on the drive shows on the main screen afterwards.
            if (ok && ["previous", "update", "factory"].indexOf(job.kind) >= 0) app.rescan(true);
        }
    }

    function rescan(quiet) {
        if (!quiet) { scanned = false; screen = "scan"; history = []; }
        scanProc.running = true;
    }
    Process {
        id: scanProc
        command: [app.bin, "scan"]
        running: true
        property string out: ""
        stdout: SplitParser { onRead: data => scanProc.out += data }
        onRunningChanged: if (running) out = ""
        onExited: code => {
            let r = null;
            try { r = JSON.parse(scanProc.out.replace(/^result: /, "")); } catch (e) {}
            app.scanError = code !== 0 ? (scanProc.out.replace(/^error: /, "") || "Looking at the drives failed") : "";
            const list = r && r.systems ? r.systems.filter(s => s.deployments && s.deployments.length) : [];
            const wasHome = app.screen === "home" || app.screen === "scan";
            app.systems = list;
            if (app.sysIdx >= list.length) app.sysIdx = 0;
            app.scanned = true;
            if (wasHome) { app.screen = list.length ? "home" : "scan"; app.sel = app.firstOn(); }
            console.log("HEARTH-RECOVERY-UI scanned systems=" + list.length);
        }
    }
    function wifiScan() { wifi = { busy: true, networks: wifi.networks, error: "" }; wifiProc.command = [app.bin, "wifi-list"]; wifiProc.running = true; }
    function wifiConnect(ssid, password) {
        wifi = { busy: true, networks: wifi.networks, error: "" };
        screen = "wifi";
        wifiProc.command = [app.bin, "wifi-connect", ssid].concat(password ? [password] : []);
        wifiProc.running = true;
    }
    Process {
        id: wifiProc
        property string out: ""
        stdout: SplitParser { onRead: data => wifiProc.out += data }
        onRunningChanged: if (running) out = ""
        onExited: code => {
            let r = null;
            try { r = JSON.parse(wifiProc.out); } catch (e) {}
            const err = wifiProc.out.startsWith("error: ") ? wifiProc.out.slice(7) : (code !== 0 ? "Wi-Fi isn't working on this PC" : "");
            if (r && r.networks) app.wifi = { busy: false, networks: r.networks, error: err };
            else { app.wifi = { busy: false, networks: app.wifi.networks, error: err ? err.charAt(0).toUpperCase() + err.slice(1) : "" }; if (!err) app.wifiScan(); }
            if (r && r.online !== undefined) app.net = { online: r.online, connection: r.connection, connectivity: r.connectivity };
            app.sel = 0;
        }
    }
    function driveScan() { drives = []; driveProc.running = true; }
    Process {
        id: driveProc
        command: [app.bin, "usb-drives"]
        property string out: ""
        stdout: SplitParser { onRead: data => driveProc.out += data }
        onRunningChanged: if (running) out = ""
        onExited: code => { try { app.drives = JSON.parse(driveProc.out).drives; } catch (e) { app.drives = []; } }
    }
    Process {
        id: netProc
        command: [app.bin, "net-status"]
        running: true
        property string out: ""
        stdout: SplitParser { onRead: data => netProc.out += data }
        onRunningChanged: if (running) out = ""
        onExited: code => { try { const r = JSON.parse(netProc.out); app.net = r; } catch (e) {} }
    }
    Timer { interval: 8000; repeat: true; running: true; onTriggered: if (!netProc.running) netProc.running = true }

    // The controller, Volume and Power buttons (recovery-pad).
    Process {
        id: pad
        command: ["python3", app.padHelper]
        running: !Quickshell.env("HEARTH_RECOVERY_NO_PAD")
        stdout: SplitParser {
            onRead: data => {
                let m;
                try { m = JSON.parse(data); } catch (e) { return; }
                if (m.t === "key") app.key(m.k);
                else if (m.t === "press") app.press();
                else if (m.t === "release") app.release();
                else if (m.t === "pads") app.pads = m.n;
                else if (m.t === "error") app.padProblem = m.text;
            }
        }
        onExited: code => { if (code !== 2) padRestart.start(); }
    }
    Timer { id: padRestart; interval: 2000; onTriggered: pad.running = true }

    // ---------------------------------------------------------------- on-screen keyboard (Wi-Fi passwords)
    readonly property var oskRows: sym
        ? [["1","2","3","4","5","6","7","8","9","0"], ["!","@","#","$","%","^","&","*","(",")"], ["~","`","+","=","[","]","{","}",";",":"], ["SHIFT","'","\"",",","<",">","/","?","\\","|"], ["ABC","SPACE","BKSP","SHOW","OK"]]
        : [["1","2","3","4","5","6","7","8","9","0"], ["q","w","e","r","t","y","u","i","o","p"], ["a","s","d","f","g","h","j","k","l","-"], ["SHIFT","z","x","c","v","b","n","m",".","_"], ["#+=","SPACE","BKSP","SHOW","OK"]]
    function oskLabel(k) {
        const names = { SHIFT: shift ? "⇧ ON" : "⇧", SPACE: "Space", BKSP: "⌫ Delete", SHOW: showPw ? "Hide" : "Show", OK: "Connect", "#+=": "#+=", ABC: "ABC" };
        if (names[k] !== undefined) return names[k];
        return shift && k.length === 1 ? k.toUpperCase() : k;
    }
    function oskPress(k) {
        if (k === "SHIFT") shift = !shift;
        else if (k === "#+=") { sym = true; }
        else if (k === "ABC") { sym = false; }
        else if (k === "SPACE") pw += " ";
        else if (k === "BKSP") pw = pw.slice(0, -1);
        else if (k === "SHOW") showPw = !showPw;
        else if (k === "OK") { if (pw.length) wifiConnect(wifiSsid, pw); }
        else { pw += shift ? k.toUpperCase() : k; if (shift) shift = false; }
    }
    function oskKey(k) {
        const rows = oskRows;
        if (k === "up" || k === "down") {
            const from = rows[oskRow];
            const frac = (oskCol + 0.5) / from.length;
            oskRow = (oskRow + (k === "up" ? -1 : 1) + rows.length) % rows.length;
            oskCol = Math.min(rows[oskRow].length - 1, Math.floor(frac * rows[oskRow].length));
        }
        else if (k === "left") oskCol = (oskCol - 1 + rows[oskRow].length) % rows[oskRow].length;
        else if (k === "right") oskCol = (oskCol + 1) % rows[oskRow].length;
        else if (k === "a") oskPress(rows[oskRow][oskCol]);
        else if (k === "b") back();
        else if (k === "x") pw = pw.slice(0, -1);
        else if (k === "y") shift = !shift;
        else if (k === "menu" || k === "start") oskPress("OK");
        return "ok";
    }

    // ---------------------------------------------------------------- for tests and a terminal
    IpcHandler {
        target: "recovery"
        function key(k: string): string { app.key(k); return app.screen + " " + app.sel; }
        function press(k: string): string { app.press(); return "ok"; }
        function release(k: string): string { app.release(); return "ok"; }
        function tap(i: int): string { app.activate(i); return app.screen; }
        function type(t: string): string { for (let c of t) app.oskPress(c); return app.pw.length + ""; }
        function state(): string {
            return JSON.stringify({ screen: app.screen, sel: app.sel, title: app.pg.title, items: app.pg.items.map(i => i.label + (i.off ? " (off)" : "")),
                                    big: app.pg.big || "", body: app.pg.body, systems: app.systems.length, busy: app.busy, work: app.work,
                                    osk: app.screen === "osk" ? { row: app.oskRow, col: app.oskCol, key: app.oskRows[app.oskRow][app.oskCol], typed: app.pw.length } : null,
                                    net: app.net, pads: app.pads, hold: app.hold });
        }
    }

    // ---------------------------------------------------------------- the screen
    PanelWindow {
        id: win
        anchors { top: true; bottom: true; left: true; right: true }
        exclusiveZone: -1
        color: app.bg
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-recovery"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive

        // Sizes follow the screen: the Ally's 7-inch 1920×1080 and a TV both read well.
        readonly property real u: Math.max(0.55, Math.min(width / 1920, height / 1080))
        readonly property bool narrow: width < height * 1.2

        Item {
            id: keys
            anchors.fill: parent
            focus: true
            Keys.onPressed: e => {
                const map = { [Qt.Key_Up]: "up", [Qt.Key_Down]: "down", [Qt.Key_Left]: "left", [Qt.Key_Right]: "right", [Qt.Key_Escape]: "b", [Qt.Key_Backspace]: app.screen === "osk" ? "x" : "b", [Qt.Key_Tab]: "down", [Qt.Key_Backtab]: "up" };
                if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || (e.key === Qt.Key_Space && app.screen !== "osk")) {
                    if (!e.isAutoRepeat) { app.press(); app.key("a"); }
                    e.accepted = true;
                    return;
                }
                if (app.screen === "osk" && e.text && e.text.length === 1 && e.text >= " ") { app.pw += e.text; e.accepted = true; return; }
                if (map[e.key] !== undefined) { app.key(map[e.key]); e.accepted = true; }
            }
            Keys.onReleased: e => { if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) && !e.isAutoRepeat) app.release(); }
        }

        // A warm glow behind everything.
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.0; color: "#1a110c" }
                GradientStop { position: 1.0; color: app.bg }
            }
        }

        ColumnLayout {
            anchors { fill: parent; leftMargin: 56 * win.u; rightMargin: 56 * win.u; topMargin: 36 * win.u; bottomMargin: 24 * win.u }
            spacing: 20 * win.u

            // ---- header: logo, name, network and controller
            RowLayout {
                Layout.fillWidth: true
                spacing: 18 * win.u
                Image { source: "file:///usr/share/pixmaps/hearth.png"; Layout.preferredWidth: 64 * win.u; Layout.preferredHeight: 64 * win.u; fillMode: Image.PreserveAspectFit; smooth: true }
                ColumnLayout {
                    spacing: 0
                    Text { text: "Hearth Recovery"; color: app.text; font.pixelSize: 34 * win.u; font.weight: Font.DemiBold }
                    Text { text: "Repair the Hearth OS on this PC"; color: app.muted; font.pixelSize: 18 * win.u }
                }
                Item { Layout.fillWidth: true }
                Repeater {
                    model: [
                        { label: app.net.online ? (app.net.connection || "Online") : "Offline", icon: app.net.online ? "wifi" : "offline", lit: app.net.online },
                        { label: app.pads > 0 ? (app.pads === 1 ? "Controller" : app.pads + " controllers") : "No controller", icon: "pad", lit: app.pads > 0 }
                    ]
                    delegate: Rectangle {
                        id: pill
                        required property var modelData
                        radius: height / 2
                        color: modelData.lit ? Qt.rgba(1, 0.54, 0.24, 0.18) : app.panel2
                        border.color: modelData.lit ? app.ember : app.line
                        implicitHeight: 44 * win.u
                        implicitWidth: pillRow.implicitWidth + 32 * win.u
                        RowLayout {
                            id: pillRow
                            anchors.centerIn: parent
                            spacing: 8 * win.u
                            Image { source: app.iconDir + pill.modelData.icon + "-light.svg"; Layout.preferredWidth: 22 * win.u; Layout.preferredHeight: 22 * win.u; sourceSize: Qt.size(44, 44); visible: status === Image.Ready }
                            Text { text: pill.modelData.label; color: app.text; font.pixelSize: 17 * win.u }
                        }
                    }
                }
            }

            // ---- the installed system (main screen)
            Rectangle {
                visible: app.pg.card === true && app.sys !== null
                Layout.fillWidth: true
                implicitHeight: card.implicitHeight + 32 * win.u
                radius: 18 * win.u
                color: app.panel
                border.color: app.line
                ColumnLayout {
                    id: card
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 * win.u; leftMargin: 24 * win.u }
                    spacing: 6 * win.u
                    Text {
                        Layout.fillWidth: true
                        text: app.sys ? app.sys.editionLabel + " on " + app.sys.model + " · " + app.sys.device + (app.sys.size ? " · " + app.gb(app.sys.size) : "") : ""
                        color: app.text; font.pixelSize: 22 * win.u; font.weight: Font.DemiBold; elide: Text.ElideRight
                    }
                    Repeater {
                        model: app.sys ? app.sys.deployments.slice(0, 3) : []
                        delegate: RowLayout {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            spacing: 12 * win.u
                            Rectangle {
                                radius: height / 2; color: index === 0 ? app.ember : app.panel3
                                implicitWidth: roleText.implicitWidth + 20 * win.u; implicitHeight: roleText.implicitHeight + 8 * win.u
                                Text { id: roleText; anchors.centerIn: parent; text: index === 0 ? "Starts now" : index === 1 ? "Previous" : "Older"; color: index === 0 ? "#2a1206" : app.text; font.pixelSize: 15 * win.u; font.weight: Font.DemiBold }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: app.depName(modelData) + (modelData.osVersion ? " · " + modelData.osName.replace(/\s*\(.*\)$/, "") + " " + modelData.osVersion : "") + (app.depWhen(modelData) ? " · " + app.depWhen(modelData) : "")
                                color: index === 0 ? app.text : app.muted; font.pixelSize: 17 * win.u; elide: Text.ElideRight
                            }
                        }
                    }
                }
            }

            // ---- title and text
            Text {
                Layout.fillWidth: true
                text: app.pg.title
                visible: text !== ""
                color: app.pg.bad ? app.danger : app.pg.good ? app.good : app.text
                font.pixelSize: 30 * win.u; font.weight: Font.DemiBold; wrapMode: Text.WordWrap
            }
            Text {
                Layout.fillWidth: true
                text: app.pg.body || ""
                visible: text !== ""
                color: app.muted; font.pixelSize: 19 * win.u; wrapMode: Text.WordWrap; lineHeight: 1.15
                Layout.maximumHeight: app.pg.items.length ? 300 * win.u : 2000
                elide: Text.ElideRight
            }
            // The log code, big enough to read off the screen.
            Rectangle {
                visible: !!app.pg.big
                Layout.alignment: Qt.AlignHCenter
                implicitWidth: bigText.implicitWidth + 80 * win.u; implicitHeight: bigText.implicitHeight + 40 * win.u
                radius: 22 * win.u; color: app.panel2; border.color: app.ember; border.width: 3 * win.u
                Text { id: bigText; anchors.centerIn: parent; text: app.pg.big || ""; color: app.flame; font.pixelSize: 84 * win.u; font.weight: Font.Bold; font.family: "monospace"; font.letterSpacing: 4 * win.u }
            }
            // Progress.
            Rectangle {
                id: progressBox
                property real sweepPos: 0
                visible: app.screen === "work" || (app.screen === "wifi" && app.wifi.busy) || (app.screen === "scan" && !app.scanned)
                Layout.fillWidth: true
                implicitHeight: 14 * win.u; radius: height / 2; color: app.panel2
                Rectangle {
                    id: bar
                    readonly property bool known: app.screen === "work" && app.work.pct >= 0
                    height: parent.height; radius: height / 2; color: app.ember
                    width: known ? parent.width * app.work.pct / 100 : parent.width * 0.25
                    x: known ? 0 : progressBox.sweepPos * (parent.width - width)
                    Behavior on width { NumberAnimation { duration: 300 } }
                }
                NumberAnimation on sweepPos { from: 0; to: 1; duration: 1400; loops: Animation.Infinite; running: progressBox.visible && !bar.known; easing.type: Easing.InOutSine }
            }

            // ---- the choices
            Flickable {
                id: list
                visible: !app.pg.osk
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentHeight: col.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                ColumnLayout {
                    id: col
                    width: list.width
                    spacing: 10 * win.u
                    Repeater {
                        model: app.pg.items
                        delegate: Rectangle {
                            id: row
                            required property var modelData
                            required property int index
                            readonly property bool lit: app.sel === index
                            Layout.fillWidth: true
                            implicitHeight: (modelData.sub ? 86 : 64) * win.u
                            radius: 18 * win.u
                            opacity: modelData.off ? 0.45 : 1
                            color: lit ? (modelData.danger ? app.danger : app.ember) : app.panel2
                            border.color: lit ? app.flame : app.line
                            border.width: lit ? 3 * win.u : 1
                            onLitChanged: if (lit) { const y0 = row.y, y1 = row.y + row.height; if (y0 < list.contentY) list.contentY = y0; else if (y1 > list.contentY + list.height) list.contentY = y1 - list.height; }
                            // "Hold A": the bar fills over 3 seconds.
                            Rectangle {
                                visible: row.modelData.hold && row.lit
                                anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
                                width: parent.width * app.hold
                                radius: parent.radius
                                color: Qt.rgba(1, 1, 1, 0.28)
                            }
                            RowLayout {
                                anchors { fill: parent; leftMargin: 24 * win.u; rightMargin: 24 * win.u }
                                spacing: 18 * win.u
                                Image {
                                    source: app.iconDir + row.modelData.icon + (row.lit ? "-ink.svg" : "-light.svg")
                                    Layout.preferredWidth: 34 * win.u; Layout.preferredHeight: 34 * win.u
                                    sourceSize: Qt.size(68, 68); fillMode: Image.PreserveAspectFit; visible: status === Image.Ready
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 3 * win.u
                                    Text { Layout.fillWidth: true; text: row.modelData.label; color: row.lit ? "#2a1206" : (row.modelData.danger ? app.danger : app.text); font.pixelSize: 24 * win.u; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                    Text { Layout.fillWidth: true; visible: !!row.modelData.sub; text: row.modelData.sub || ""; color: row.lit ? "#3a1a08" : app.muted; font.pixelSize: 16 * win.u; elide: Text.ElideRight }
                                }
                            }
                            MouseArea {
                                anchors.fill: parent
                                onPressed: { app.sel = row.index; if (row.modelData.hold) app.press(); }
                                onReleased: { if (row.modelData.hold) app.release(); }
                                onCanceled: { if (row.modelData.hold) app.release(); }
                                onClicked: { if (!row.modelData.hold) app.activate(row.index); }
                            }
                        }
                    }
                }
            }

            // ---- on-screen keyboard
            ColumnLayout {
                visible: app.pg.osk === true
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 10 * win.u
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 72 * win.u; radius: 14 * win.u; color: app.panel2; border.color: app.ember; border.width: 2
                    Text {
                        anchors { verticalCenter: parent.verticalCenter; left: parent.left; leftMargin: 20 * win.u; right: parent.right; rightMargin: 20 * win.u }
                        text: app.pw.length ? (app.showPw ? app.pw : "•".repeat(app.pw.length)) : "Type the Wi-Fi password"
                        color: app.pw.length ? app.text : app.muted; font.pixelSize: 30 * win.u; elide: Text.ElideLeft
                    }
                }
                Repeater {
                    model: app.oskRows
                    delegate: RowLayout {
                        id: krow
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 8 * win.u
                        Repeater {
                            model: krow.modelData
                            delegate: Rectangle {
                                id: kb
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.oskRow === krow.index && app.oskCol === index
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.preferredWidth: modelData === "SPACE" ? 4 : (modelData.length > 1 ? 2 : 1)
                                radius: 12 * win.u
                                color: lit ? app.ember : (modelData === "OK" ? app.panel3 : app.panel2)
                                border.color: lit ? app.flame : app.line
                                Text { anchors.centerIn: parent; text: app.oskLabel(kb.modelData); color: kb.lit ? "#2a1206" : app.text; font.pixelSize: 26 * win.u; font.weight: Font.DemiBold }
                                MouseArea { anchors.fill: parent; onClicked: { app.oskRow = krow.index; app.oskCol = kb.index; app.oskPress(kb.modelData); } }
                            }
                        }
                    }
                }
                Text { text: "A types · X deletes · Y capitals · Menu connects · B cancels"; color: app.muted; font.pixelSize: 16 * win.u }
            }

            // ---- footer
            Text {
                Layout.fillWidth: true
                text: (app.padProblem ? app.padProblem + " · " : "") + "A  select     B  back     ·     no controller? Volume buttons move, Power selects     ·     touch works too"
                color: app.muted; font.pixelSize: 15 * win.u; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
            }
        }
    }
}
