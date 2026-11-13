// Task Manager's page (loaded by TaskManager.qml, which keeps the state: tm.page, tm.prefs, tm.setPref…).
// Windows 11's layout in Hearth's look: the title bar with search, the left rail (folds to icons with ☰), and the pages:
//   Processes    Apps (n) / Background processes (n), multi-process apps unfold, Name · Status · CPU · Memory · Disk ·
//                Network · GPU with the totals in the column headers and heat-map shading; Run new task, End task,
//                Efficiency mode, View (sort, unfold all); right-click / A / X: the row's menu
//   Performance  mini graphs on the left (CPU, Memory, each disk with This PC's letters, Wi-Fi / Ethernet, each graphics
//                card, the battery), a big gridded 60-second graph and the details under it
//   Startup apps your start-up apps and services, Enabled / Disabled with an impact estimate; Hearth's own are
//                "Part of Hearth" and can't be changed; Disable only writes your own ~/.config/autostart copy
//   Details      every process: PID, status, user name, CPU, memory, command line
//   Services     yours (start / stop / restart, except the ones that keep Hearth running) and the system's (read only)
//   Settings     default start page, real-time update speed, always on top, the rail; the theme follows Hearth
// One sampler (`hearth-tasks serve`) for every page: it's told how often and what the page on show needs.
import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets

FocusScope {
    id: v
    focus: true

    // ================================================ numbers in words
    function thousands(n) { return String(Math.round(n || 0)).replace(/\B(?=(\d{3})+(?!\d))/g, ","); }
    readonly property var units: ["B", "KB", "MB", "GB", "TB"]
    function size(b) {
        if (b === undefined || b === null) return "–";
        let i = 0, x = b;
        while (x >= 1024 && i < units.length - 1) { x /= 1024; i++; }
        return (i === 0 || x >= 100 ? String(Math.round(x)) : x.toFixed(1)) + " " + units[i];
    }
    function sizeOf(used, total) {
        if (!total) return "–";
        let i = 0, x = total;
        while (x >= 1024 && i < units.length - 1) { x /= 1024; i++; }
        const f = n => { const y = n / Math.pow(1024, i); return y >= 100 ? String(Math.round(y)) : y.toFixed(1); };
        return f(used) + "/" + f(total) + " " + units[i];
    }
    function mb(b) { return thousands((b || 0) / 1048576) + " MB"; }
    function rate(bps) { const m = (bps || 0) / 1048576; return m < 0.05 ? "0 MB/s" : (m >= 100 ? Math.round(m) : m.toFixed(1)) + " MB/s"; }
    function kbps(bytes) {
        const b = (bytes || 0) * 8;
        if (b >= 1e9) return (b / 1e9).toFixed(1) + " Gbps";
        if (b >= 1e6) return (b / 1e6).toFixed(1) + " Mbps";
        return Math.round(b / 1e3) + " Kbps";
    }
    function pct(n) { return n === undefined || n === null ? "–" : (n >= 10 || n === 0 ? Math.round(n) : n.toFixed(1)) + "%"; }
    function pct1(n) { return n === undefined || n === null ? "–" : (n >= 100 ? Math.round(n) : n.toFixed(1)) + "%"; }
    function ghz(mhz) { return mhz ? (mhz / 1000).toFixed(2) + " GHz" : "–"; }
    function temp(c) { return c === undefined || c === null ? "–" : Math.round(c) + " °C"; }
    function upTime(s) {
        s = Math.max(0, Math.floor(s || 0));
        const d = Math.floor(s / 86400), h = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60), x = s % 60;
        const two = n => (n < 10 ? "0" : "") + n;
        return d + ":" + two(h) + ":" + two(m) + ":" + two(x);
    }
    function mix(a, b, t) { return Qt.rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1); }
    function plural(n, w) { return n + " " + w + (n === 1 ? "" : "s"); }

    // Each device's graph colour: the accent for the processor and graphics, Windows' hues for the others, all tinted
    // with your accent so they belong to the look.
    function deviceColor(kind) {
        const base = kind === "memory" ? "#a371f7" : kind === "disk" ? "#52b26b" : kind === "net" ? "#d9a33b" : kind === "battery" ? "#4fb7a8" : "";
        return base === "" ? Theme.accent : mix(Qt.color(base), Theme.accent, 0.2);
    }

    // ================================================ what the sampler says
    property var sample: null
    property string sampleError: ""
    property var groups: []
    property var groupMap: ({})
    property var details: []
    property var hist: ({})                 // device key → the last minute of values
    property var expanded: ({})
    property string query: ""
    readonly property int histMax: tm.interval > 0 ? Math.round(60 / tm.interval) + 1 : 61

    function push(h, k, val) {
        const a = (h[k] || []).slice(-(histMax - 1));
        a.push(val === undefined || val === null ? 0 : val);
        h[k] = a;
    }
    function applySample(line) {
        let d;
        try { d = JSON.parse(line); } catch (e) { return; }
        if (d.error) { sampleError = d.error; return; }
        sampleError = "";
        const h = Object.assign({}, hist);
        push(h, "cpu", d.cpu ? d.cpu.total : 0);
        push(h, "memory", d.mem && d.mem.total ? 100 * d.mem.used / d.mem.total : 0);
        (d.disks || []).forEach(x => push(h, "disk:" + x.name, x.active));
        (d.nets || []).forEach(x => push(h, "net:" + x.name, x.recv + x.send));
        (d.gpus || []).forEach(x => push(h, "gpu:" + x.index, x.busy));
        if (d.battery) push(h, "battery", d.battery.percent);
        hist = h;
        sample = d;
        if (d.procs) { groups = d.procs; const m = {}; d.procs.forEach(g => m[g.id] = g); groupMap = m; buildProcs(); checkEnding(); }
        if (d.details) { details = d.details; buildDetails(); }
    }
    function wants() {
        switch (tm.page) {
        case "processes": return ["procs"];
        case "performance": return ["gpu", "temps"];
        case "details": return ["details"];
        default: return [];
        }
    }
    readonly property bool streams: typeof sampler.write === "function"
    function sendConfig() {
        const msg = JSON.stringify({ interval: tm.interval, want: wants() }) + "\n";
        if (streams && sampler.running) sampler.write(msg);
        else if (!streams) oneShot.restart();
    }
    function sampleNow() { if (streams && sampler.running) sampler.write("{\"now\": true}\n"); }
    function start() { rail.forceActiveFocus(); }

    Process {
        id: sampler
        command: ["python3", tm.helper, "serve"]
        running: true
        stdinEnabled: true
        stdout: SplitParser { onRead: line => v.applySample(line) }
        stderr: SplitParser { onRead: line => console.warn("hearth-tasks serve:", line) }
        onRunningChanged: if (running) v.sendConfig(); else restartSampler.start()
    }
    Timer { id: restartSampler; interval: 2000; onTriggered: if (!sampler.running && v.streams) sampler.running = true }
    // A Quickshell without stdin for processes: one sample per tick from `hearth-tasks sample` instead.
    Process {
        id: sampleOnce
        stdout: StdioCollector { onStreamFinished: v.applySample(text.trim().split("\n").pop() || "") }
    }
    Timer {
        id: oneShot
        interval: Math.max(1000, tm.interval * 1000)
        repeat: true
        triggeredOnStart: true
        running: !v.streams && tm.interval > 0
        onTriggered: if (!sampleOnce.running) { sampleOnce.command = ["python3", tm.helper, "sample"].concat(v.wants()); sampleOnce.running = true; }
    }
    Connections {
        target: tm
        function onPageChanged() { v.sendConfig(); v.loadPageData(); v.closeMenu(); }
        function onIntervalChanged() { v.sendConfig(); }
    }
    onQueryChanged: { buildProcs(); buildDetails(); buildStartup(); buildServices(); }

    // One-shot helper runs (end, Efficiency mode, startup, services).
    function run(proc, args) {
        if (proc.running) return false;
        proc.command = ["python3", tm.helper].concat(args);
        proc.running = true;
        return true;
    }
    function lastLine(t) { return t.trim().split("\n").pop() || ""; }
    function loadPageData() {
        if (tm.page === "startup") run(startProc, ["startup"]);
        if (tm.page === "services") run(svcProc, ["services"]);
    }
    Component.onCompleted: loadPageData()

    // ================================================ Processes
    ListModel { id: procModel }
    function sortBy(list, key, desc, get) {
        const g = get || ((o, k) => o[k]);
        return list.slice().sort((a, b) => {
            const x = g(a, key), y = g(b, key);
            let c = typeof x === "string" || typeof y === "string" ? String(x).toLowerCase().localeCompare(String(y).toLowerCase()) : (x || 0) - (y || 0);
            if (c === 0) c = String(a.name || "").toLowerCase().localeCompare(String(b.name || "").toLowerCase());
            return desc ? -c : c;
        });
    }
    function sortPref(table, key, desc) { const s = tm.pref("sort_" + table, null); return s && s.key ? s : { key: key, desc: desc }; }
    function setSort(table, key, defDesc) {
        const s = sortPref(table, "", false);
        tm.setPref("sort_" + table, { key: key, desc: s.key === key ? !s.desc : defDesc });
    }
    function procMatch(g, q) {
        if (!q) return true;
        if (String(g.name).toLowerCase().indexOf(q) >= 0 || String(g.exe || "").toLowerCase().indexOf(q) >= 0
                || String(g.desktop || "").toLowerCase().indexOf(q) >= 0 || String(g.tag || "").toLowerCase().indexOf(q) >= 0) return true;
        return g.pids.some(p => String(p).indexOf(q) === 0) || (g.children || []).some(c => String(c.name).toLowerCase().indexOf(q) >= 0);
    }
    function procRow(o) {
        return Object.assign({ key: "", type: "group", gid: "", pid: 0, name: "", icon: "", tag: "", depth: 0, expandable: false,
            expanded: false, eco: false, suspended: false, locked: false, count: 0, cpu: 0, mem: 0, swap: 0, disk: 0, gpu: 0,
            hc: 0, hm: 0, hd: 0, hg: 0, label: "", power: "" }, o);
    }
    function buildProcs() {
        const q = query.trim().toLowerCase();
        const s = sortPref("procs", "", false);
        const statusOf = g => g.suspended ? "Suspended" : g.eco ? "Efficiency mode" : "";
        const list = groups.filter(g => procMatch(g, q));
        const sorted = s.key ? sortBy(list, s.key, s.desc, (o, k) => k === "status" ? statusOf(o) : o[k]) : list;
        const apps = sorted.filter(g => g.kind === "app" || g.kind === "game");
        const hearth = sorted.filter(g => g.kind === "system");                // Part of Hearth: its own group (1.11)
        const bg = sorted.filter(g => !(g.kind === "app" || g.kind === "game" || g.kind === "system"));
        const rows = [];
        const add = g => {
            const open = !!expanded[g.id] && (g.children || []).length > 1;
            rows.push(procRow({ key: g.id, gid: g.id, pid: g.pid, name: g.name || String(g.pid), icon: g.icon || "", tag: g.tag || "",
                expandable: (g.children || []).length > 1, expanded: open, eco: !!g.eco, suspended: !!g.suspended,
                locked: !!g.protected, count: g.count, cpu: g.cpu || 0, mem: g.mem || 0, swap: g.swap || 0, disk: g.disk || 0, gpu: g.gpu || 0,
                hc: g.heat.cpu, hm: g.heat.mem, hd: g.heat.disk, hg: g.heat.gpu, power: powerOf(g) }));
            if (open) {
                const kids = s.key && s.key !== "status" ? sortBy(g.children, s.key, s.desc) : g.children;
                kids.forEach(c => rows.push(procRow({ key: g.id + "/" + c.pid, type: "child", gid: g.id, pid: c.pid, name: c.name + "  ·  " + c.pid,
                    depth: 1, eco: c.nice >= 19, suspended: c.state === "T", locked: !!g.protected, count: 1, cpu: c.cpu, mem: c.mem, swap: c.swap || 0,
                    disk: c.disk, gpu: c.gpu, hc: c.heat.cpu, hm: c.heat.mem, hd: c.heat.disk, hg: c.heat.gpu, power: "" })));
            }
        };
        if (apps.length) { rows.push(procRow({ key: "s:apps", type: "section", label: "Apps (" + apps.length + ")" })); apps.forEach(add); }
        if (hearth.length) { rows.push(procRow({ key: "s:hearth", type: "section", label: "Hearth (" + hearth.length + ")" })); hearth.forEach(add); }
        if (bg.length) { rows.push(procRow({ key: "s:bg", type: "section", label: "Background processes (" + bg.length + ")" })); bg.forEach(add); }
        sync(procModel, rows);
    }
    // Updates a ListModel in place (by "key"), so rows keep their place, selection and focus while numbers change.
    function sync(model, items) {
        for (let i = 0; i < items.length; i++) {
            let j = -1;
            for (let k = i; k < model.count; k++) {
                if (model.get(k).key === items[i].key) { j = k; break; }
            }
            if (j < 0) model.insert(i, items[i]);
            else {
                if (j !== i) model.move(j, i, 1);
                model.set(i, items[i]);
            }
        }
        if (model.count > items.length) model.remove(items.length, model.count - items.length);
    }
    function toggleExpand(gid, on) {
        const e = Object.assign({}, expanded);
        if (on === undefined ? !e[gid] : on) e[gid] = true; else delete e[gid];
        expanded = e;
        buildProcs();
    }
    function heatCell(level, text) { return { t: text, heat: level }; }
    // Power usage the Windows way, from CPU and graphics use (1.11).
    function powerOf(g) {
        if (g.suspended) return "None";
        const u = (g.cpu || 0) + (g.gpu || 0) / 2;
        return u < 1 ? "Very low" : u < 5 ? "Low" : u < 15 ? "Moderate" : u < 40 ? "High" : "Very high";
    }
    function procCell(r, key) {
        switch (key) {
        case "status": return { t: r.suspended ? "Suspended" : "", leaf: r.eco };
        case "pid": return { t: r.pid ? String(r.pid) : "", muted: true };
        case "swap": return { t: r.swap > 0 ? mb(r.swap) : "", muted: r.swap < 50 * 1048576 };
        case "power": return { t: r.power, muted: r.power === "Very low" || r.power === "" };
        case "cpu": return heatCell(r.hc, pct1(r.cpu));
        case "mem": return heatCell(r.hm, mb(r.mem));
        case "disk": return heatCell(r.hd, rate(r.disk));
        case "net": return { t: "–", heat: 0, muted: true };
        case "gpu": return heatCell(r.hg, pct1(r.gpu));
        }
        return { t: "" };
    }

    // ---- ending an app (the sheet: ending → stuck: Force end)
    property string endGid: ""
    property var endPids: []
    property bool checkingEnd: false
    function endGroup(gid) {
        const g = groupMap[gid];
        if (!g) return;
        if (g.protected) {
            showSheet("notice", g.name, (g.why || "This") + " keeps Hearth running, so it can't be ended here.", [{ label: "OK", action: "close" }]);
            return;
        }
        if (run(actProc, ["end", String(g.pid)])) {
            endGid = gid;
            endPids = g.pids;
            showSheet("ending", g.name, "Asking " + g.name + " to close…", [{ label: "Hide", action: "close" }]);
            endTimer.restart();
        }
    }
    function endPid(pid, name, force) {
        if (run(actProc, [force ? "kill-pid" : "end-pid", String(pid)])) { endGid = ""; endPids = [pid]; endName = name; endTimer.restart(); }
    }
    property string endName: ""
    function checkEnding() {
        if (!checkingEnd) return;
        checkingEnd = false;
        const alive = groups.some(g => g.pids.some(p => endPids.indexOf(p) >= 0));
        if (sheetMode === "ending") {
            if (alive) showSheet("stuck", sheetTitle, sheetTitle + " hasn't closed yet. You can wait a little longer, or force it to close (anything not saved will be lost).",
                [{ label: "Force end", action: "kill", danger: true }, { label: "Wait", action: "wait" }]);
            else closeSheet();
        }
    }
    Timer {
        id: endTimer
        interval: 3000
        onTriggered: {
            if (tm.page !== "processes") { v.closeSheet(); return; }   // only Processes reads the list to check
            v.checkingEnd = true;
            v.sampleNow();
        }
    }

    function windowFor(g) {
        if (!g) return null;
        const ids = g.appIds || [];
        const desk = (g.desktop || "").toLowerCase();
        const all = ToplevelManager.toplevels.values;
        for (let i = 0; i < all.length; i++) {
            const t = all[i];
            const a = (t.appId || "").toLowerCase();
            if (!a || String(t.title).indexOf("Task Manager") === 0 || t.title === "Hearth Settings") continue;
            if (ids.indexOf(a) >= 0 || (desk && a === desk)) return t;
            const e = AppIcons.entryFor(t.appId);
            if (e && desk && (e.id || "").toLowerCase() === desk) return t;
        }
        return null;
    }
    function setEco(pid, on) { if (run(ecoProc, ["eco", String(pid), on ? "on" : "off"])) ecoName = (groupMap[groupOfPid(pid)] || {}).name || ""; }
    property string ecoName: ""
    function groupOfPid(pid) { const g = groups.find(x => x.pids.indexOf(pid) >= 0); return g ? g.id : ""; }
    function openLocation(path) {
        if (!path) return;
        const dir = path.replace(/\/[^\/]*$/, "") || "/";
        const base = path.replace(/^.*\//, "");
        Quickshell.execDetached(["sh", "-c", "quickshell ipc -p \"$3\" call files open \"$1\" && sleep 1.5 && quickshell ipc -p \"$3\" call filemanager select \"$2\"",
            "sh", dir, base, Quickshell.shellDir]);
    }
    function properties(path) { if (path) Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "fileprops", "open", path]); }
    function searchOnline(name) { Qt.openUrlExternally("https://duckduckgo.com/?q=" + encodeURIComponent(name)); }

    // A row's values as they are now (a ListModel row object follows its index, which moves as the list updates).
    function plain(r) { const o = {}; if (r) for (const k of ["key", "type", "gid", "pid", "name", "expandable", "expanded", "mine", "path", "canChange", "on", "status", "unit"]) o[k] = r[k]; return o; }
    function procMenu(r, x, y) {
        if (!r || r.type === "section") return;
        const row = plain(r);
        const g = groupMap[row.gid];
        if (!g) return;
        const child = row.type === "child";
        const w = child ? null : windowFor(g);
        const path = child ? "" : g.exe;
        const pid = child ? row.pid : g.pid;
        const procName = child ? (row.name.split("  ·  ")[0] || "") : (g.exe ? g.exe.replace(/^.*\//, "") : g.name);
        const items = [];
        if (row.expandable) items.push({ id: "expand", label: row.expanded ? "Collapse" : "Expand", icon: row.expanded ? "chevu" : "chevd" });
        items.push({ id: "end", label: "End task", icon: "close", enabled: !g.protected, key: "Del" },
                   { id: "tree", label: "End process tree", icon: "close", enabled: !g.protected },
                   { id: "eco", label: "Efficiency mode", icon: "leaf", checked: !!g.eco, enabled: !g.protected },
                   { id: "priority", label: "Set priority ▸", icon: "cpu", enabled: !g.protected },
                   { id: "cores", label: "Run on these cores…", icon: "cpu", enabled: !g.protected },
                   { id: "limit", label: "Limit CPU / memory…", icon: "cpu", enabled: !g.protected && !child },
                   { sep: true },
                   { id: "never", label: "Never close when memory runs out", icon: "shield", enabled: !g.protected && procName !== "" },
                   { sep: true });
        if (w) items.push({ id: "window", label: "Switch to", icon: "window" });
        items.push({ id: "details", label: "Go to details", icon: "list", key: "PID " + pid },
                   { id: "where", label: "Open file location", icon: "folder", enabled: !!g.exe },
                   { id: "web", label: "Search online", icon: "search" },
                   { sep: true },
                   { id: "props", label: "Properties", icon: "info", enabled: !!g.exe });
        showMenu(child ? row.name : g.name, items, x, y, id => {
            switch (id) {
            case "end": child ? endPid(row.pid, row.name, false) : endGroup(g.id); break;
            case "tree": run(actProc, ["end-tree", String(pid)]); break;
            case "eco": setEco(g.pid, !g.eco); break;
            case "priority":
                showMenu("Set priority · " + g.name, [
                    { id: "normal", label: "Normal", checked: (g.nice || 0) === 0 },
                    { id: "below", label: "Below normal", checked: (g.nice || 0) > 0 && (g.nice || 0) < 19 },
                    { id: "low", label: "Low", checked: (g.nice || 0) >= 19 }], x, y, lvl => run(actProc, ["priority", String(g.pid), lvl]));
                break;
            case "cores":
                askText("Run on these cores", "Which of this PC's cores " + g.name + " may use, until it closes (cores 0 to " + (v.coreCount - 1) + "). For example 0-3 or 0,2,4.",
                        "0-" + (v.coreCount - 1), t => run(actProc, ["affinity", String(g.pid), t.replace(/\s+/g, "")]));
                break;
            case "limit":
                askText("Limit CPU / memory", "A cap for " + g.name + " until it closes: CPU as a percent of one core, then memory in MB (0 = no limit). It's slowed down, never closed. For example: 200 4096",
                        "200 0", t => { const p = t.trim().split(/[\s,]+/); run(actProc, ["limit", String(g.pid), p[0] || "0", p[1] || "0"]); });
                break;
            case "never": Quickshell.execDetached(["pkexec", "/usr/libexec/nimbus/hearth-memory", "never", procName, "on"]);
                showSheet("notice", g.name, procName + " is on your Never close list: when memory runs out, Hearth closes something else. Change the list in Memory › Swap & settings.", [{ label: "OK", action: "close" }]);
                break;
            case "details": search.text = String(pid); tm.page = "details"; break;
            case "expand": toggleExpand(g.id); break;
            case "window": { const t = windowFor(g); if (t) { t.minimized = false; t.activate(); } break; }
            case "where": openLocation(g.exe); break;
            case "web": searchOnline(g.name); break;
            case "props": properties(g.exe); break;
            }
        });
    }
    readonly property int coreCount: v.sample && v.sample.cpu && v.sample.cpu.cores ? Math.max(1, v.sample.cpu.cores.length) : 1
    // A question with a text box (Run on these cores, Limit CPU / memory): the Run sheet's box, OK runs then().
    property var askThen: null
    function askText(title, text, dflt, then) {
        askThen = then;
        runInput.text = dflt || "";
        showSheet("ask", title, text, [{ label: "OK", action: "ask" }, { label: "Cancel", action: "close" }]);
        Qt.callLater(() => { runInput.forceActiveFocus(); runInput.selectAll(); });
    }

    // ================================================ Details
    ListModel { id: detModel }
    function buildDetails() {
        if (tm.page !== "details") return;
        const q = query.trim().toLowerCase();
        const s = sortPref("details", "pid", false);
        const list = details.filter(d => !q || String(d.pid).indexOf(q) === 0 || d.name.toLowerCase().indexOf(q) >= 0 || d.user.toLowerCase().indexOf(q) >= 0 || d.cmd.toLowerCase().indexOf(q) >= 0);
        const rows = sortBy(list, s.key, s.desc).map(d => ({ key: String(d.pid), type: "row", depth: 0, icon: "", tag: "", expandable: false,
            expanded: false, eco: false, name: d.name, pid: d.pid, status: d.status, user: d.user, cpu: d.cpu, mem: d.mem, cmd: d.cmd,
            mine: d.mine, path: d.path || "", ppid: d.ppid || 0, swap: d.swap || 0, threads: d.threads || 0, nice: d.nice || 0 }));
        sync(detModel, rows);
    }
    function detCell(r, key) {
        switch (key) {
        case "pid": return { t: String(r.pid) };
        case "status": return { t: r.status, muted: r.status === "Running" ? false : false };
        case "user": return { t: r.user, muted: !r.mine };
        case "cpu": return { t: r.cpu >= 10 ? Math.round(r.cpu) + "" : r.cpu.toFixed(1) };
        case "mem": return { t: thousands(r.mem / 1024) + " K" };
        case "cmd": return { t: r.cmd, muted: true };
        case "ppid": return { t: r.ppid ? String(r.ppid) : "0", muted: true };
        case "swap": return { t: r.swap > 0 ? mb(r.swap) : "0", muted: r.swap === 0 };
        case "threads": return { t: String(r.threads || 0) };
        case "nice": return { t: r.nice < -10 ? "High" : r.nice < 0 ? "Above normal" : r.nice === 0 ? "Normal" : r.nice < 19 ? "Below normal" : "Low" };
        }
        return { t: String(r[key] === undefined ? "" : r[key]) };
    }
    function detMenu(r0, x, y) {
        if (!r0) return;
        const r = plain(r0);
        const gid = groupOfPid(r.pid);
        const g = groupMap[gid];
        showMenu(r.name + " (" + r.pid + ")", [
            { id: "end", label: "End task", icon: "close", enabled: r.mine, key: "Del" },
            { id: "tree", label: "End process tree", icon: "close", enabled: r.mine && !!g && !g.protected },
            { id: "eco", label: "Efficiency mode", icon: "leaf", enabled: r.mine && !!g && !g.protected, checked: !!g && !!g.eco },
            { id: "priority", label: "Set priority ▸", icon: "cpu", enabled: r.mine && !!g && !g.protected },
            { id: "cores", label: "Set affinity…", icon: "cpu", enabled: r.mine && !!g && !g.protected },
            { sep: true },
            { id: "where", label: "Open file location", icon: "folder", enabled: r.path !== "" },
            { id: "web", label: "Search online", icon: "search" },
            { sep: true },
            { id: "props", label: "Properties", icon: "info", enabled: r.path !== "" }
        ], x, y, id => {
            switch (id) {
            case "end": endPid(r.pid, r.name, false); break;
            case "tree": run(actProc, ["end-tree", String(r.pid)]); break;
            case "priority":
                showMenu("Set priority · " + r.name, [{ id: "normal", label: "Normal" }, { id: "below", label: "Below normal" }, { id: "low", label: "Low" }], x, y,
                         lvl => run(actProc, ["priority", String(r.pid), lvl]));
                break;
            case "cores":
                askText("Run on these cores", "Which cores " + r.name + " may use, until it closes (0 to " + (v.coreCount - 1) + "). For example 0-3.", "0-" + (v.coreCount - 1),
                        t => run(actProc, ["affinity", String(r.pid), t.replace(/\s+/g, "")]));
                break;
            case "eco": if (g) setEco(g.pid, !g.eco); break;
            case "where": openLocation(r.path); break;
            case "web": searchOnline(r.name); break;
            case "props": properties(r.path); break;
            }
        });
    }

    // ================================================ Startup apps
    ListModel { id: startModel }
    property var startItems: []
    property var bootSeconds: null
    property bool startLoaded: false
    property string startError: ""
    function statusOfStart(i) { return i.kind === "hearth-part" ? "Part of Hearth" : i.on ? "Enabled" : "Disabled"; }
    function buildStartup() {
        const q = query.trim().toLowerCase();
        const s = sortPref("startup", "name", false);
        const list = startItems.filter(i => !q || i.name.toLowerCase().indexOf(q) >= 0 || String(i.publisher || "").toLowerCase().indexOf(q) >= 0);
        const rows = sortBy(list.map(i => ({ key: i.id, type: "row", depth: 0, icon: "", tag: "", expandable: false, expanded: false,
            eco: false, name: i.name, publisher: i.publisher || "", status: statusOfStart(i), impact: i.impact || "Not measured",
            on: !!i.on, canChange: !!i.canChange, sub: i.sub || "", kind: i.kind })), s.key, s.desc);
        sync(startModel, rows);
    }
    function startCell(r, key) {
        if (key === "status") {
            if (r.status === "Part of Hearth") return { t: r.status, badge: true };
            return { t: r.status, good: r.status === "Enabled", muted: r.status === "Disabled" };
        }
        if (key === "impact") return { t: r.impact, muted: r.impact === "Not measured" || r.impact === "None" };
        if (key === "publisher") return { t: r.publisher || "–", muted: !r.publisher };
        return { t: String(r[key] || "") };
    }
    function setStartup(key, on) {
        const i = startItems.find(x => x.id === key);
        if (!i) return;
        if (!i.canChange) {
            showSheet("notice", i.name, i.kind === "hearth-part" ? "This is part of Hearth, so it can't be switched off here: Game Mode and your devices always come up."
                : i.sub || "This can't be changed here right now.", [{ label: "OK", action: "close" }]);
            return;
        }
        startError = "";
        if (!run(setProc, ["startup-set", key, on ? "on" : "off"])) return;
        startItems = startItems.map(x => x.id === key ? Object.assign({}, x, { on: on, impact: on ? x.impact : "None" }) : x);
        buildStartup();
    }
    function startMenu(r0, x, y) {
        if (!r0) return;
        const r = plain(r0);
        showMenu(r.name, [
            { id: "on", label: "Enable", icon: "check", enabled: r.canChange && !r.on },
            { id: "off", label: "Disable", icon: "close", enabled: r.canChange && r.on },
            { sep: true },
            { id: "web", label: "Search online", icon: "search" }
        ], x, y, id => {
            if (id === "on") setStartup(r.key, true);
            else if (id === "off") setStartup(r.key, false);
            else if (id === "web") searchOnline(r.name);
        });
    }

    // ================================================ Services
    ListModel { id: svcModel }
    property var svcItems: []
    property bool svcLoaded: false
    property string svcError: ""
    function buildServices() {
        const q = query.trim().toLowerCase();
        const s = sortPref("services", "name", false);
        const pick = grp => sortBy(svcItems.filter(i => i.group === grp && (!q || i.name.toLowerCase().indexOf(q) >= 0 || i.desc.toLowerCase().indexOf(q) >= 0 || String(i.pid || "").indexOf(q) === 0)),
            s.key, s.desc, (o, k) => k === "pid" ? (o.pid || 0) : o[k]);
        const rows = [];
        const row = i => ({ key: i.id, type: "row", depth: 0, icon: "", tag: i.hearth ? "Part of Hearth" : "", expandable: false, expanded: false,
            eco: false, name: i.name, pid: i.pid || 0, desc: i.desc || "", status: i.status, group: i.group, canChange: !!i.canChange,
            unit: i.unit, label: "" });
        const sec = (k, l) => ({ key: k, type: "section", depth: 0, icon: "", tag: "", expandable: false, expanded: false, eco: false,
            name: "", pid: 0, desc: "", status: "", group: "", canChange: false, unit: "", label: l });
        const mine = pick("Yours"), sys = pick("System");
        if (mine.length) { rows.push(sec("s:yours", "Your services (" + mine.length + ")")); mine.forEach(i => rows.push(row(i))); }
        if (sys.length) { rows.push(sec("s:system", "System services (" + sys.length + ") · read only")); sys.forEach(i => rows.push(row(i))); }
        sync(svcModel, rows);
    }
    function svcCell(r, key) {
        if (key === "pid") return { t: r.pid ? String(r.pid) : "", muted: true };
        if (key === "status") return { t: r.status, good: r.status === "Running", bad: r.status === "Failed", muted: r.status === "Stopped" };
        if (key === "desc") return { t: r.desc, muted: true };
        return { t: String(r[key] || "") };
    }
    function svcAct(r, act) {
        if (!r || !r.canChange) return;
        svcError = "";
        run(svcActProc, ["service", r.unit, act]);
    }
    function svcMenu(r0, x, y) {
        if (!r0 || r0.type === "section") return;
        const r = plain(r0);
        showMenu(r.name, [
            { id: "start", label: "Start", icon: "play", enabled: r.canChange && r.status !== "Running" },
            { id: "stop", label: "Stop", icon: "pause", enabled: r.canChange && r.status === "Running" },
            { id: "restart", label: "Restart", icon: "restart", enabled: r.canChange && r.status === "Running" },
            { sep: true },
            { id: "web", label: "Search online", icon: "search" }
        ], x, y, id => { if (id === "web") searchOnline(r.name + " service"); else svcAct(r, id); });
    }

    // ---- one-shot helper processes
    Item {
        visible: false
        Process {
            id: actProc
            stdout: StdioCollector {
                onStreamFinished: {
                    const line = v.lastLine(text);
                    if (line.startsWith("error: ")) {
                        v.showSheet("error", v.sheetTitle || v.endName || "Task Manager", line.slice(7).replace(/^./, c => c.toUpperCase()) + ".", [{ label: "OK", action: "close" }]);
                        return;
                    }
                    v.sampleNow();
                }
            }
        }
        Process {
            id: ecoProc
            stdout: StdioCollector {
                onStreamFinished: {
                    const line = v.lastLine(text);
                    if (line.startsWith("error: ")) { v.showSheet("error", v.ecoName || "Efficiency mode", line.slice(7).replace(/^./, c => c.toUpperCase()) + ".", [{ label: "OK", action: "close" }]); return; }
                    try {
                        const r = JSON.parse(line);
                        if (!r.on && r.stuck) v.showSheet("notice", v.ecoName || "Efficiency mode", "Efficiency mode is off, but " + (v.ecoName || "the app") + " keeps its low priority until it's restarted: Linux only lets an administrator speed an app back up.", [{ label: "OK", action: "close" }]);
                    } catch (e) {}
                    v.sampleNow();
                }
            }
        }
        Process {
            id: startProc
            stdout: StdioCollector {
                onStreamFinished: {
                    const line = v.lastLine(text);
                    if (line.startsWith("error: ")) { v.startError = line.slice(7); return; }
                    try { const r = JSON.parse(line); v.startItems = r.items || []; v.bootSeconds = r.bootSeconds === undefined ? null : r.bootSeconds; v.startLoaded = true; v.buildStartup(); } catch (e) {}
                }
            }
        }
        Process {
            id: setProc
            stdout: StdioCollector {
                onStreamFinished: {
                    const line = v.lastLine(text);
                    if (line.startsWith("error: ")) v.startError = line.slice(7);
                    startReload.restart();
                }
            }
        }
        Timer { id: startReload; interval: 400; onTriggered: v.run(startProc, ["startup"]) }
        Process {
            id: svcProc
            stdout: StdioCollector {
                onStreamFinished: {
                    const line = v.lastLine(text);
                    if (line.startsWith("error: ")) { v.svcError = line.slice(7); return; }
                    try { v.svcItems = JSON.parse(line).services || []; v.svcLoaded = true; v.buildServices(); } catch (e) {}
                }
            }
        }
        Process {
            id: svcActProc
            stdout: StdioCollector {
                onStreamFinished: {
                    const line = v.lastLine(text);
                    if (line.startsWith("error: ")) v.svcError = line.slice(7);
                    svcReload.restart();
                }
            }
        }
        Timer { id: svcReload; interval: 500; onTriggered: v.run(svcProc, ["services"]) }
    }

    // ================================================ the menu (right-click, A, X / the Menu key)
    property bool menuOpen: false
    property string menuTitle: ""
    property var menuItems: []
    property int menuIndex: 0
    property real menuX: -1
    property real menuY: -1
    property var menuHandler: null
    property var menuReturn: null
    function showMenu(title, items, x, y, handler) {
        const out = [];
        for (const it of items) {
            if (it.sep && (!out.length || out[out.length - 1].sep)) continue;
            out.push(it);
        }
        while (out.length && out[out.length - 1].sep) out.pop();
        menuReturn = v.Window.activeFocusItem || null;
        menuTitle = title;
        menuItems = out;
        menuIndex = Math.max(0, out.findIndex(i => !i.sep && i.enabled !== false));
        menuX = x === undefined ? -1 : x;
        menuY = y === undefined ? -1 : y;
        menuHandler = handler;
        menuOpen = true;
        if (styledMenu.styled) styledMenu.item.open(title, out.filter(i => i.enabled !== false).map(i => i.sep ? i : Object.assign({}, i, { shortcut: i.key })));
        menuKeys.forceActiveFocus();
    }
    function closeMenu() {
        if (!menuOpen) return;
        menuOpen = false;
        if (menuReturn) { const r = menuReturn; menuReturn = null; Qt.callLater(() => r.forceActiveFocus()); }
    }
    function pickMenu(i) {
        const it = menuItems[i];
        if (!it || it.sep || it.enabled === false) return;
        const h = menuHandler;
        closeMenu();
        if (h) Qt.callLater(() => h(it.id));
    }

    // ================================================ the sheet (ending, stuck, notices, Run new task)
    property string sheetMode: ""
    property string sheetTitle: ""
    property string sheetText: ""
    property var sheetButtons: []
    property int sheetIndex: 0
    property var sheetReturn: null
    function showSheet(mode, t, text, buttons) {
        if (sheetMode === "") sheetReturn = v.Window.activeFocusItem || null;
        sheetMode = mode;
        sheetTitle = t;
        sheetText = text;
        sheetButtons = buttons;
        sheetIndex = 0;
        if (mode === "run") Qt.callLater(() => runInput.forceActiveFocus());
        else Qt.callLater(() => sheetCard.forceActiveFocus());
    }
    function closeSheet() {
        if (sheetMode === "") return;
        sheetMode = "";
        endTimer.stop();
        checkingEnd = false;
        if (sheetReturn) { const r = sheetReturn; sheetReturn = null; Qt.callLater(() => r.forceActiveFocus()); }
    }
    function sheetAction(action) {
        switch (action) {
        case "close": closeSheet(); break;
        case "wait":
            showSheet("ending", sheetTitle, "Waiting for " + sheetTitle + " to close…", [{ label: "Force end", action: "kill", danger: true }, { label: "Hide", action: "close" }]);
            endTimer.restart();
            break;
        case "kill": {
            const g = groupMap[endGid];
            if (g) run(actProc, ["kill", String(g.pid)]);
            else if (endPids.length) run(actProc, ["kill-pid", String(endPids[0])]);
            closeSheet();
            break;
        }
        case "run": runTask(); break;
        case "ask": { const t = runInput.text.trim(), f = askThen; askThen = null; runInput.text = ""; closeSheet(); if (f && t !== "") f(t); break; }
        }
    }
    function runTask() {
        const c = runInput.text.trim();
        if (c === "") return;
        Quickshell.execDetached(["sh", "-c", "cd \"$HOME\" && exec " + c]);
        runInput.text = "";
        closeSheet();
    }
    function askRun() {
        showSheet("run", "Run new task", "Type the name of a program, folder or command, and Hearth will open it for you.",
            [{ label: "OK", action: "run" }, { label: "Cancel", action: "close" }]);
    }

    // ================================================ the page's command bar (Up from a list's first row)
    function pageItem() { return pageLoader.item; }

    // ================================================ layout
    Rectangle { anchors.fill: parent; color: Theme.panel }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ------------------------------------------------ title bar: the Hearth mark, the name, search
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 46
            RowLayout {
                anchors { left: parent.left; verticalCenter: parent.verticalCenter; leftMargin: 14 }
                spacing: 10
                Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 18; Layout.preferredHeight: 18; fillMode: Image.PreserveAspectFit; smooth: true }
                Text { text: "Task Manager"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
            }
            Rectangle {
                id: searchBox
                anchors.centerIn: parent
                width: Math.min(560, parent.width * 0.42)
                height: 32
                radius: Theme.r(6)
                color: search.activeFocus ? Theme.bg2 : Theme.panel2
                border.width: search.activeFocus ? 2 : 1
                border.color: search.activeFocus ? Theme.accent : Theme.line
                Icon { id: searchIcon; anchors.left: parent.left; anchors.leftMargin: 10; anchors.verticalCenter: parent.verticalCenter; name: "search"; size: 15; color: Theme.muted }
                TextInput {
                    id: search
                    anchors { left: searchIcon.right; right: clearSearch.left; leftMargin: 8; rightMargin: 6; verticalCenter: parent.verticalCenter }
                    color: Theme.text
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.accentInk
                    font.family: Theme.font; font.pixelSize: 13
                    clip: true
                    onTextChanged: v.query = text
                    Keys.onPressed: e => {
                        if (e.key === Qt.Key_Down || e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { v.focusPage(); e.accepted = true; }
                        else if (e.key === Qt.Key_Escape) { if (text !== "") text = ""; else rail.forceActiveFocus(); e.accepted = true; }
                    }
                    Text {
                        anchors.fill: parent
                        verticalAlignment: Text.AlignVCenter
                        visible: search.text === ""
                        text: "Type a name, publisher or PID to search"
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        elide: Text.ElideRight
                    }
                }
                Icon {
                    id: clearSearch
                    anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                    visible: search.text !== ""
                    name: "close"; size: 13; color: Theme.muted
                    MouseArea { anchors.fill: parent; anchors.margins: -6; cursorShape: Qt.PointingHandCursor; onClicked: search.text = "" }
                }
                MouseArea { anchors.fill: parent; z: -1; onClicked: search.forceActiveFocus() }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // ------------------------------------------------ the rail
            FocusScope {
                id: rail
                Layout.fillHeight: true
                Layout.preferredWidth: tm.railFolded ? 56 : (v.width < 900 ? 196 : 228)
                Behavior on Layout.preferredWidth { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
                // -1: ☰, 0..4: the pages, 5: Settings
                readonly property var items: [
                    { id: "processes", icon: "grid", label: "Processes" },
                    { id: "performance", icon: "motion", label: "Performance" },
                    { id: "startup", icon: "play", label: "Startup apps" },
                    { id: "details", icon: "list", label: "Details" },
                    { id: "services", icon: "server", label: "Services" }
                ]
                property int cursor: Math.max(0, tm.pageIds.indexOf(tm.page))
                function step(d) {
                    cursor = Math.max(-1, Math.min(5, cursor + d));
                    if (cursor >= 0) tm.go(tm.pageIds[cursor]);
                }
                onActiveFocusChanged: if (activeFocus) cursor = Math.max(0, tm.pageIds.indexOf(tm.page))
                Keys.onPressed: e => {
                    switch (e.key) {
                    case Qt.Key_Down: case Qt.Key_Tab: step(1); break;
                    case Qt.Key_Up: case Qt.Key_Backtab: if (cursor <= -1) search.forceActiveFocus(); else step(-1); break;
                    case Qt.Key_Right: v.focusPage(); break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                        if (cursor === -1) tm.setPref("railFolded", !tm.railFolded); else v.focusPage();
                        break;
                    case Qt.Key_Escape: tm.close(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
                ColumnLayout {
                    anchors { fill: parent; leftMargin: 6; rightMargin: 6; bottomMargin: 8 }
                    spacing: 2
                    RailItem {
                        icon: "menu"; label: ""; folded: true
                        lit: false; focusRing: rail.activeFocus && rail.cursor === -1
                        tip: tm.railFolded ? "Open navigation" : "Close navigation"
                        onClicked: tm.setPref("railFolded", !tm.railFolded)
                    }
                    Repeater {
                        model: rail.items
                        delegate: RailItem {
                            required property var modelData
                            required property int index
                            icon: modelData.icon; label: modelData.label; folded: tm.railFolded
                            lit: tm.page === modelData.id
                            focusRing: rail.activeFocus && rail.cursor === index
                            onClicked: { tm.go(modelData.id); rail.forceActiveFocus(); }
                        }
                    }
                    Item { Layout.fillHeight: true }
                    RailItem {
                        icon: "gear"; label: "Settings"; folded: tm.railFolded
                        lit: tm.page === "settings"
                        focusRing: rail.activeFocus && rail.cursor === 5
                        onClicked: { tm.go("settings"); rail.forceActiveFocus(); }
                    }
                }
            }

            // ------------------------------------------------ the page, on a softer layer with a rounded corner
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                Rectangle {
                    x: 0; y: 0
                    width: parent.width + 20
                    height: parent.height + 20
                    radius: Theme.r(8)
                    color: Theme.bg2
                    border.color: Theme.line
                    border.width: 1
                }
                Loader {
                    id: pageLoader
                    anchors.fill: parent
                    anchors.leftMargin: 1; anchors.topMargin: 1
                    sourceComponent: tm.page === "performance" ? perfPage : tm.page === "startup" ? startupPage
                        : tm.page === "details" ? detailsPage : tm.page === "services" ? servicesPage
                        : tm.page === "settings" ? settingsPage : processesPage
                }
            }
        }
    }
    // For checks (TaskManager.qml menu / search): the first row's menu on the page shown, or a search.
    function ipcMenu() {
        focusPage();
        Qt.callLater(menuForPage);
    }
    function menuForPage() {
        const pick = (model, fn) => { for (let i = 0; i < model.count; i++) if (model.get(i).type !== "section") { fn(model.get(i), 360, 200 + i * 10); return; } };
        if (tm.page === "processes") pick(procModel, (r, x, y) => procMenu(r, x, y));
        else if (tm.page === "details") pick(detModel, (r, x, y) => detMenu(r, x, y));
        else if (tm.page === "startup") pick(startModel, (r, x, y) => startMenu(r, x, y));
        else if (tm.page === "services") pick(svcModel, (r, x, y) => svcMenu(r, x, y));
    }
    function ipcSearch(q) { search.text = q; }
    function focusPage() { if (pageLoader.item && pageLoader.item.focusFirst) pageLoader.item.focusFirst(); }
    function toRail() { rail.forceActiveFocus(); }

    // Keys anywhere: Ctrl+F (or F3) searches, Ctrl+E too (Windows).
    Keys.onPressed: e => {
        if ((e.modifiers & Qt.ControlModifier) && (e.key === Qt.Key_F || e.key === Qt.Key_E)) { search.forceActiveFocus(); search.selectAll(); e.accepted = true; }
        else if (e.key === Qt.Key_F3) { search.forceActiveFocus(); e.accepted = true; }
        else if (e.key === Qt.Key_F5) { v.sampleNow(); v.loadPageData(); e.accepted = true; }
    }

    // ================================================ pages
    Component {
        id: processesPage
        PageFrame {
            id: pp
            onLeave: v.toRail()
            onUpOut: search.forceActiveFocus()
            title: "Processes"
            buttons: [
                { id: "run", label: "Run new task", icon: "plus", primary: true },
                { id: "end", label: "End task", icon: "close", enabled: pp.canEnd },
                { id: "eco", label: "Efficiency mode", icon: "leaf", enabled: pp.canEnd, checked: pp.isEco },
                { id: "view", label: "View", icon: "chevd" }
            ]
            readonly property var selRow: table.currentIndex >= 0 && table.currentIndex < procModel.count ? procModel.get(table.currentIndex) : null
            readonly property var selGroup: selRow && selRow.type !== "section" ? (v.groupMap[selRow.gid] || null) : null
            readonly property bool canEnd: selGroup !== null && !selGroup.protected
            readonly property bool isEco: selGroup !== null && !!selGroup.eco
            function focusFirst() { table.focusList(); }
            onButton: (id, x, y) => {
                switch (id) {
                case "run": v.askRun(); break;
                case "end": if (selRow && selRow.type === "child") v.endPid(selRow.pid, selRow.name, false); else if (selGroup) v.endGroup(selGroup.id); break;
                case "eco": if (selGroup) v.setEco(selGroup.pid, !selGroup.eco); break;
                case "view": {
                    const s = v.sortPref("procs", "", false);
                    const by = (k, l) => ({ id: "sort:" + k, label: l, checked: s.key === k });
                    v.showMenu("View", [
                        by("name", "Sort by name"), by("cpu", "Sort by CPU"), by("mem", "Sort by memory"), by("disk", "Sort by disk"), by("gpu", "Sort by GPU"),
                        { id: "sort:", label: "Grouped by type (as found)", checked: s.key === "" },
                        { sep: true },
                        { id: "expand", label: "Expand all", icon: "chevd" },
                        { id: "collapse", label: "Collapse all", icon: "chevu" },
                        { sep: true },
                        { id: "refresh", label: "Refresh now", icon: "sync", key: "F5" }
                    ], x, y, id => {
                        if (id.indexOf("sort:") === 0) { const k = id.slice(5); tm.setPref("sort_procs", { key: k, desc: k !== "name" && k !== "" }); v.buildProcs(); }
                        else if (id === "expand") { const e = {}; v.groups.forEach(g => { if ((g.children || []).length > 1) e[g.id] = true; }); v.expanded = e; v.buildProcs(); }
                        else if (id === "collapse") { v.expanded = ({}); v.buildProcs(); }
                        else if (id === "refresh") v.sampleNow();
                    });
                    break;
                }
                }
            }
            DataTable {
                id: table
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: procModel
                tableId: "procs"
                rowHeight: 34
                widths: tm.pref("cols_procs", ({}))
                sortKey: v.sortPref("procs", "", false).key
                sortDesc: v.sortPref("procs", "", false).desc
                columns: [
                    { key: "name", title: "Name", flex: true },
                    { key: "pid", title: "PID", width: 76, right: true },
                    { key: "status", title: "Status", width: 120 },
                    { key: "cpu", title: "CPU", width: 80, right: true, hasTotal: true },
                    { key: "mem", title: "Memory", width: 96, right: true, hasTotal: true },
                    { key: "swap", title: "Compressed", width: 104, right: true },
                    { key: "disk", title: "Disk", width: 86, right: true, hasTotal: true },
                    { key: "net", title: "Network", width: 86, right: true, hasTotal: true },
                    { key: "gpu", title: "GPU", width: 76, right: true, hasTotal: true },
                    { key: "power", title: "Power", width: 84, right: true }
                ]
                showIcons: true
                totals: {
                    const t = v.sample ? v.sample.totals : null;
                    if (!t) return ({});
                    return { cpu: v.pct(t.cpu), mem: v.pct(t.mem), disk: v.pct(t.disk),
                             net: t.net !== null && t.net !== undefined ? v.pct(t.net) : t.netMbps + " Mbps",
                             gpu: t.gpu === null || t.gpu === undefined ? "–" : v.pct(t.gpu) };
                }
                cell: (r, k) => v.procCell(r, k)
                empty: v.sampleError !== "" ? v.sampleError : v.groups.length === 0 ? (tm.interval === 0 && !v.sample ? "Updates are paused (Settings › Real-time update speed)." : "Looking at what's running…") : "Nothing matches “" + v.query + "”."
                onSortClicked: k => { if (k !== "net" && k !== "power") { v.setSort("procs", k, k !== "name" && k !== "status" && k !== "pid"); v.buildProcs(); } }
                onResized: w => tm.setPref("cols_procs", w)
                onUp: pp.toBar()
                onBack: v.toRail()
                onActivated: i => { const r = procModel.get(i); const p = table.rowPoint(i); v.procMenu(r, p.x, p.y); }
                onMenuAt: (i, x, y) => v.procMenu(procModel.get(i), x, y)
                onToggled: (i, on) => { const r = procModel.get(i); if (r.expandable) v.toggleExpand(r.gid, on); else if (r.type === "child" && !on) { const gi = table.indexOfKey(r.gid); if (gi >= 0) { v.toggleExpand(r.gid, false); table.currentIndex = gi; } } }
                onDeleteKey: i => { const r = procModel.get(i); if (r.type === "child") v.endPid(r.pid, r.name, false); else if (r.type === "group") v.endGroup(r.gid); }
            }
        }
    }

    Component {
        id: perfPage
        PageFrame {
            id: pf
            onLeave: v.toRail()
            onUpOut: search.forceActiveFocus()
            title: "Performance"
            // Open Resource Monitor (ResourceMonitor.qml, docs/RESOURCE-MONITOR.md), like Windows' Task Manager.
            buttons: [{ id: "run", label: "Run new task", icon: "plus", primary: true }, { id: "resmon", label: "Open Resource Monitor", icon: "cpu" }]
            onButton: id => { if (id === "run") v.askRun(); else if (id === "resmon") Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "resmon", "open"]); }
            readonly property var s: v.sample
            // Overview | <device> & settings (1.11: Memory's Swap & settings, CPU's Power & settings, …)
            property string memTab: "overview"
            readonly property bool memSettings: !!dev && memTab === "settings"
            readonly property var settingsLabel: ({ memory: "Swap & settings", cpu: "Power & settings", disk: "Health & settings", net: "Connection & hotspot", gpu: "Tuning", battery: "Health & settings" })
            readonly property var devices: {
                const d = [];
                if (!s) return d;
                d.push({ key: "cpu", kind: "cpu", title: "CPU", sub: v.pct(s.cpu.total) + "  " + v.ghz(s.cpu.mhz), max: 100 });
                d.push({ key: "memory", kind: "memory", title: "Memory", sub: v.sizeOf(s.mem.used, s.mem.total) + " (" + v.pct(100 * s.mem.used / Math.max(1, s.mem.total)) + ")", max: 100 });
                (s.disks || []).forEach(x => d.push({ key: "disk:" + x.name, kind: "disk", title: x.label, sub: x.type + "  " + v.pct(x.active), max: 100, disk: x }));
                (s.nets || []).forEach(x => d.push({ key: "net:" + x.name, kind: "net", title: x.kind, sub: x.up ? "S: " + v.kbps(x.send) + "  R: " + v.kbps(x.recv) : "Not connected", max: 0, net: x }));
                (s.gpus || []).forEach(x => d.push({ key: "gpu:" + x.index, kind: "gpu", title: "GPU " + x.index, sub: x.name + "  " + (x.busy === null || x.busy === undefined ? "" : v.pct(x.busy)) + (x.temp !== null && x.temp !== undefined ? " (" + v.temp(x.temp) + ")" : ""), max: 100, gpu: x }));
                if (s.battery) d.push({ key: "battery", kind: "battery", title: "Battery", sub: s.battery.percent + "% · " + s.battery.status, max: 100, battery: s.battery });
                return d;
            }
            readonly property int selIndex: {
                const want = tm.perfDevice;
                let i = devices.findIndex(x => x.key === want);
                if (i < 0) i = devices.findIndex(x => x.kind === want || x.key.indexOf(want + ":") === 0);
                return Math.max(0, i);
            }
            readonly property var dev: devices.length ? devices[selIndex] : null
            function pick(i) { if (devices[i]) { tm.perfDevice = devices[i].key; tm.setPref("perfDevice", devices[i].key); } }
            Component.onCompleted: { const p = tm.pref("perfDevice", ""); if (p !== "" && tm.perfDevice === "cpu") tm.perfDevice = p; }
            function focusFirst() { tiles.forceActiveFocus(); }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0
                // the tiles
                ListView {
                    id: tiles
                    Layout.preferredWidth: 228
                    Layout.fillHeight: true
                    model: pf.devices
                    clip: true
                    spacing: 4
                    boundsBehavior: Flickable.StopAtBounds
                    currentIndex: pf.selIndex
                    keyNavigationEnabled: false
                    Keys.onPressed: e => {
                        switch (e.key) {
                        case Qt.Key_Down: case Qt.Key_Tab: pf.pick(Math.min(pf.devices.length - 1, pf.selIndex + 1)); break;
                        case Qt.Key_Up: case Qt.Key_Backtab: if (pf.selIndex === 0) pf.toBar(); else pf.pick(pf.selIndex - 1); break;
                        case Qt.Key_Left: case Qt.Key_Escape: v.toRail(); break;
                        default: return;
                        }
                        positionViewAtIndex(pf.selIndex, ListView.Contain);
                        e.accepted = true;
                    }
                    delegate: Rectangle {
                        id: tile
                        required property var modelData
                        required property int index
                        readonly property bool lit: index === pf.selIndex
                        width: ListView.view.width - 8
                        x: 4
                        height: 64
                        radius: Theme.r(6)
                        color: lit ? Theme.panel2 : tileMouse.containsMouse ? Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.6) : "transparent"
                        border.width: lit && tiles.activeFocus ? 2 : 0
                        border.color: Theme.accent
                        Rectangle { visible: tile.lit; width: 3; height: 18; radius: 1.5; color: Theme.accent; anchors { left: parent.left; leftMargin: 1; verticalCenter: parent.verticalCenter } }
                        RowLayout {
                            anchors { fill: parent; leftMargin: 10; rightMargin: 8 }
                            spacing: 10
                            Graph {
                                Layout.preferredWidth: 64; Layout.preferredHeight: 42
                                values: v.hist[tile.modelData.key] || []
                                capacity: v.histMax
                                maxValue: tile.modelData.max
                                lineColor: v.deviceColor(tile.modelData.kind)
                                grid: false
                                framed: true
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Text { Layout.fillWidth: true; text: tile.modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                                Text { Layout.fillWidth: true; text: tile.modelData.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
                            }
                        }
                        MouseArea { id: tileMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { pf.pick(tile.index); tiles.forceActiveFocus(); } }
                    }
                }
                Rectangle { Layout.fillHeight: true; Layout.preferredWidth: 1; color: Theme.line }
                // the device
                Flickable {
                    id: detail
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    contentHeight: dcol.implicitHeight + 32
                    boundsBehavior: Flickable.StopAtBounds
                    ColumnLayout {
                        id: dcol
                        x: 22; y: 12
                        width: detail.width - 44
                        spacing: 6
                        Text { visible: !pf.dev; text: v.sampleError !== "" ? v.sampleError : tm.interval === 0 ? "Updates are paused (Settings › Real-time update speed)." : "Measuring…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                        RowLayout {
                            visible: !!pf.dev
                            Layout.fillWidth: true
                            Text { text: pf.dev ? pf.dev.title : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 28; font.weight: Font.DemiBold }
                            Item { Layout.fillWidth: true }
                            Text {
                                Layout.maximumWidth: dcol.width * 0.6
                                horizontalAlignment: Text.AlignRight
                                elide: Text.ElideRight
                                text: !pf.dev ? "" : pf.dev.kind === "cpu" ? pf.s.cpu.name : pf.dev.kind === "memory" ? v.size(pf.s.mem.total) : pf.dev.kind === "disk" ? (pf.dev.disk.model || pf.dev.disk.name)
                                    : pf.dev.kind === "net" ? pf.dev.net.name : pf.dev.kind === "gpu" ? pf.dev.gpu.name : pf.dev.kind === "battery" ? pf.dev.battery.name : ""
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 15
                            }
                        }
                        // Overview | … & settings (1.11, the approved mockups; MemorySettings.qml, DeviceSettings.qml)
                        Row {
                            visible: !!pf.dev && !!pf.settingsLabel[pf.dev.kind]
                            spacing: 6
                            Repeater {
                                model: pf.dev ? [["overview", "Overview"], ["settings", pf.settingsLabel[pf.dev.kind] || "Settings"]] : []
                                delegate: Rectangle {
                                    required property var modelData
                                    readonly property bool on: pf.memTab === modelData[0]
                                    width: mt.implicitWidth + 26; height: 32; radius: Theme.r(8)
                                    color: on ? Theme.panel2 : mtm.containsMouse ? Theme.panel3 : "transparent"
                                    border.color: on ? Theme.line : "transparent"
                                    Text { id: mt; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.text : Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                                    MouseArea { id: mtm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: pf.memTab = parent.modelData[0] }
                                }
                            }
                        }
                        MemorySettings {
                            visible: pf.memSettings && pf.dev.kind === "memory"
                            Layout.fillWidth: true
                            Layout.topMargin: 6
                        }
                        Loader {
                            active: pf.memSettings && pf.dev.kind !== "memory"
                            visible: active
                            Layout.fillWidth: true
                            Layout.topMargin: 6
                            sourceComponent: DeviceSettings { kind: pf.dev ? pf.dev.kind : "cpu"; dev: pf.dev; sample: pf.s; width: parent ? parent.width : 600 }
                        }
                        RowLayout {
                            visible: !!pf.dev && !pf.memSettings
                            Layout.fillWidth: true
                            Text {
                                text: !pf.dev ? "" : pf.dev.kind === "memory" ? "Memory usage" : pf.dev.kind === "disk" ? "Active time" : pf.dev.kind === "net" ? "Throughput" : pf.dev.kind === "battery" ? "Charge" : "% Utilization"
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                            }
                            Item { Layout.fillWidth: true }
                            Text { text: !pf.dev ? "" : pf.dev.max > 0 ? "100%" : v.kbps(big.scaleTop); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                        }
                        Graph {
                            id: big
                            visible: !!pf.dev && !pf.memSettings
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.max(160, Math.min(330, detail.height * 0.45))
                            values: pf.dev ? (v.hist[pf.dev.key] || []) : []
                            capacity: v.histMax
                            maxValue: pf.dev ? pf.dev.max : 100
                            lineColor: v.deviceColor(pf.dev ? pf.dev.kind : "cpu")
                            grid: true
                            framed: true
                        }
                        RowLayout {
                            visible: !!pf.dev && !pf.memSettings
                            Layout.fillWidth: true
                            Text { text: "60 seconds"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                            Item { Layout.fillWidth: true }
                            Text { text: "0"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                        }
                        // the classic numbers
                        RowLayout {
                            visible: !!pf.dev && !pf.memSettings
                            Layout.fillWidth: true
                            Layout.topMargin: 8
                            spacing: 24
                            Flow {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 22
                                Repeater {
                                    model: pf.bigStats
                                    delegate: Column {
                                        required property var modelData
                                        spacing: 1
                                        Text { text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                                        Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 20 }
                                    }
                                }
                            }
                            GridLayout {
                                Layout.alignment: Qt.AlignTop
                                columns: 2
                                columnSpacing: 12
                                rowSpacing: 4
                                Repeater {
                                    model: pf.smallStats.length * 2
                                    delegate: Text {
                                        required property int index
                                        readonly property var pair: pf.smallStats[Math.floor(index / 2)]
                                        text: pair ? pair[index % 2] + (index % 2 === 0 ? ":" : "") : ""
                                        color: index % 2 === 0 ? Theme.muted : Theme.text
                                        font.family: Theme.font; font.pixelSize: 12
                                    }
                                }
                            }
                        }
                        // CPU: each core, and the sensors (temperatures, fans) Task Manager has always shown
                        Flow {
                            visible: !!pf.dev && !pf.memSettings && pf.dev.kind === "cpu" && pf.s.cpu.cores.length > 1
                            Layout.fillWidth: true
                            Layout.topMargin: 10
                            spacing: 4
                            Repeater {
                                model: pf.dev && pf.dev.kind === "cpu" ? pf.s.cpu.cores : []
                                delegate: Rectangle {
                                    required property real modelData
                                    width: 12; height: 30; radius: 2
                                    color: Theme.panel2
                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        width: parent.width
                                        height: Math.max(2, parent.height * Math.min(100, parent.modelData) / 100)
                                        radius: 2
                                        color: parent.modelData >= 85 ? Theme.flame : Theme.accent
                                    }
                                }
                            }
                        }
                        Text {
                            visible: !!pf.dev && !pf.memSettings && pf.dev.kind === "cpu" && !!pf.s.sensors && (pf.s.sensors.temps.length > 0 || pf.s.sensors.fans.length > 0)
                            Layout.fillWidth: true
                            Layout.topMargin: 8
                            wrapMode: Text.WordWrap
                            text: !pf.s || !pf.s.sensors ? "" : pf.s.sensors.temps.map(t => (t.label === t.sensor ? t.sensor : t.sensor + " " + t.label) + " " + v.temp(t.c))
                                .concat(pf.s.sensors.fans.map(f => f.label + " " + (f.rpm > 0 ? f.rpm + " rpm" : "stopped"))).join("   ·   ")
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                        }
                    }
                }
            }
            readonly property var bigStats: {
                if (!dev || !s) return [];
                switch (dev.kind) {
                case "cpu": return [["Utilization", v.pct(s.cpu.total)], ["Speed", v.ghz(s.cpu.mhz)], ["Processes", v.thousands(s.cpu.processes)],
                    ["Threads", s.cpu.threads ? v.thousands(s.cpu.threads) : "–"], ["Up time", v.upTime(s.uptime)]].concat(s.cpu.temp !== null && s.cpu.temp !== undefined ? [["Temperature", v.temp(s.cpu.temp)]] : []);
                case "memory": return [["In use", v.size(s.mem.used)], ["Available", v.size(s.mem.available)], ["Committed", v.sizeOf(s.mem.committed, s.mem.commitLimit)], ["Cached", v.size(s.mem.cached)]];
                case "disk": return [["Active time", v.pct(dev.disk.active)], ["Average response time", dev.disk.response === null ? "–" : dev.disk.response + " ms"], ["Read speed", v.rate(dev.disk.read)], ["Write speed", v.rate(dev.disk.write)]];
                case "net": return [["Send", v.kbps(dev.net.send)], ["Receive", v.kbps(dev.net.recv)]];
                case "gpu": return [["Utilization", dev.gpu.busy === null || dev.gpu.busy === undefined ? "–" : v.pct(dev.gpu.busy)]]
                    .concat(dev.gpu.vramTotal ? [["Dedicated GPU memory", v.sizeOf(dev.gpu.vramUsed || 0, dev.gpu.vramTotal)]] : [])
                    .concat(dev.gpu.gttTotal ? [["Shared GPU memory", v.sizeOf(dev.gpu.gttUsed || 0, dev.gpu.gttTotal)]] : [])
                    .concat([["Temperature", v.temp(dev.gpu.temp)]]);
                case "battery": return [["Charge", dev.battery.percent + "%"], ["State", dev.battery.status]].concat(dev.battery.watts ? [[dev.battery.status === "Charging" ? "Charging at" : "Power draw", dev.battery.watts.toFixed(1) + " W"]] : []);
                }
                return [];
            }
            readonly property var smallStats: {
                if (!dev || !s) return [];
                switch (dev.kind) {
                case "cpu": return [["Base speed", s.cpu.base ? v.ghz(s.cpu.base) : "–"], ["Cores", String(s.cpu.coreCount || "–")], ["Logical processors", String(s.cpu.logical || "–")],
                    ["Virtualization", s.cpu.virt ? "Enabled" : "Not available"], ["L2 cache", s.cpu.l2 ? v.size(s.cpu.l2) : "–"], ["L3 cache", s.cpu.l3 ? v.size(s.cpu.l3) : "–"]];
                case "memory": return [["Total", v.size(s.mem.total)], ["Free", v.size(s.mem.free)], ["Swap", s.mem.swapTotal ? v.sizeOf(s.mem.swapUsed, s.mem.swapTotal) : "None"]];
                case "disk": return [["Capacity", v.size(dev.disk.size)], ["Type", dev.disk.type], ["System disk", dev.disk.system ? "Yes" : "No"]]
                    .concat(dev.disk.mounts.length ? [["Mounted at", dev.disk.mounts.slice(0, 3).join(", ")]] : []);
                case "net": return [["Adapter name", dev.net.name], ["Connection type", dev.net.kind], ["Link speed", dev.net.speed ? dev.net.speed + " Mbps" : "–"], ["State", dev.net.up ? "Connected" : "Not connected"]];
                case "gpu": return [["Maker", dev.gpu.vendor], ["Clock", dev.gpu.mhz ? dev.gpu.mhz + " MHz" : "–"]];
                case "battery": return dev.battery.hours ? [[dev.battery.status === "Charging" ? "Full in" : "Time left", Math.floor(dev.battery.hours) + " h " + Math.round((dev.battery.hours % 1) * 60) + " min"]] : [];
                }
                return [];
            }
        }
    }

    Component {
        id: startupPage
        PageFrame {
            id: sp
            onLeave: v.toRail()
            onUpOut: search.forceActiveFocus()
            title: "Startup apps"
            subtitle: v.bootSeconds !== null && v.bootSeconds !== undefined ? "Last start-up time: " + v.bootSeconds + " seconds" : ""
            readonly property var sel: table.currentIndex >= 0 && table.currentIndex < startModel.count ? startModel.get(table.currentIndex) : null
            buttons: [
                { id: "on", label: "Enable", enabled: !!sp.sel && sp.sel.canChange && !sp.sel.on, primary: true },
                { id: "off", label: "Disable", enabled: !!sp.sel && sp.sel.canChange && sp.sel.on }
            ]
            onButton: id => { if (sel) v.setStartup(sel.key, id === "on"); }
            function focusFirst() { table.focusList(); }
            Text {
                Layout.fillWidth: true; Layout.leftMargin: 20
                visible: v.startError !== ""
                text: v.startError
                color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
            }
            DataTable {
                id: table
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: startModel
                tableId: "startup"
                rowHeight: 40
                widths: tm.pref("cols_startup", ({}))
                sortKey: v.sortPref("startup", "name", false).key
                sortDesc: v.sortPref("startup", "name", false).desc
                columns: [
                    { key: "name", title: "Name", flex: true },
                    { key: "publisher", title: "Publisher", width: 200 },
                    { key: "status", title: "Status", width: 150 },
                    { key: "impact", title: "Start-up impact", width: 150 }
                ]
                cell: (r, k) => v.startCell(r, k)
                empty: v.startLoaded ? "Nothing starts on its own." : "Looking for things that start on their own…"
                onSortClicked: k => { v.setSort("startup", k, false); v.buildStartup(); }
                onResized: w => tm.setPref("cols_startup", w)
                onUp: sp.toBar()
                onBack: v.toRail()
                onActivated: i => { const r = startModel.get(i); const p = table.rowPoint(i); v.startMenu(r, p.x, p.y); }
                onMenuAt: (i, x, y) => v.startMenu(startModel.get(i), x, y)
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.margins: 16
                Layout.maximumWidth: 640
                implicitHeight: noteText.implicitHeight + 24
                radius: Theme.r(8)
                color: Theme.panel
                border.color: Theme.accent
                border.width: 1
                Text {
                    id: noteText
                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 }
                    wrapMode: Text.WordWrap
                    textFormat: Text.StyledText
                    text: "<b>Part of Hearth</b> items can't be switched off here, so Game Mode and your devices always come up. Disabling an app only changes your own copy of its start-up entry; Steam is left to Hearth, which starts it when a game needs it."
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 12
                }
            }
        }
    }

    Component {
        id: detailsPage
        PageFrame {
            id: dp
            onLeave: v.toRail()
            onUpOut: search.forceActiveFocus()
            title: "Details"
            readonly property var sel: table.currentIndex >= 0 && table.currentIndex < detModel.count ? detModel.get(table.currentIndex) : null
            buttons: [
                { id: "run", label: "Run new task", icon: "plus", primary: true },
                { id: "end", label: "End task", icon: "close", enabled: !!dp.sel && dp.sel.mine }
            ]
            onButton: id => { if (id === "run") v.askRun(); else if (id === "end" && sel) v.endPid(sel.pid, sel.name, false); }
            function focusFirst() { table.focusList(); }
            Component.onCompleted: v.buildDetails()
            DataTable {
                id: table
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: detModel
                tableId: "details"
                rowHeight: 30
                widths: tm.pref("cols_details", ({}))
                sortKey: v.sortPref("details", "pid", false).key
                sortDesc: v.sortPref("details", "pid", false).desc
                columns: [
                    { key: "name", title: "Name", width: 200 },
                    { key: "pid", title: "PID", width: 72, right: true },
                    { key: "ppid", title: "Parent", width: 72, right: true },
                    { key: "status", title: "Status", width: 96 },
                    { key: "user", title: "User name", width: 110 },
                    { key: "cpu", title: "CPU", width: 60, right: true },
                    { key: "mem", title: "Memory", width: 100, right: true },
                    { key: "swap", title: "Swapped", width: 90, right: true },
                    { key: "threads", title: "Threads", width: 70, right: true },
                    { key: "nice", title: "Priority", width: 100, right: true },
                    { key: "cmd", title: "Command line", flex: true }
                ]
                cell: (r, k) => v.detCell(r, k)
                empty: v.sampleError !== "" ? v.sampleError : v.details.length === 0 ? (tm.interval === 0 ? "Updates are paused (Settings › Real-time update speed)." : "Reading the process list…") : "Nothing matches “" + v.query + "”."
                onSortClicked: k => { v.setSort("details", k, k === "cpu" || k === "mem" || k === "swap" || k === "threads"); v.buildDetails(); }
                onResized: w => tm.setPref("cols_details", w)
                onUp: dp.toBar()
                onBack: v.toRail()
                onActivated: i => { const p = table.rowPoint(i); v.detMenu(detModel.get(i), p.x, p.y); }
                onMenuAt: (i, x, y) => v.detMenu(detModel.get(i), x, y)
                onDeleteKey: i => { const r = detModel.get(i); if (r.mine) v.endPid(r.pid, r.name, false); }
            }
        }
    }

    Component {
        id: servicesPage
        PageFrame {
            id: sv
            onLeave: v.toRail()
            onUpOut: search.forceActiveFocus()
            title: "Services"
            readonly property var sel: table.currentIndex >= 0 && table.currentIndex < svcModel.count ? svcModel.get(table.currentIndex) : null
            readonly property bool can: !!sel && sel.type !== "section" && sel.canChange
            buttons: [
                { id: "start", label: "Start", icon: "play", enabled: sv.can && sv.sel.status !== "Running" },
                { id: "stop", label: "Stop", icon: "pause", enabled: sv.can && sv.sel.status === "Running" },
                { id: "restart", label: "Restart", icon: "restart", enabled: sv.can && sv.sel.status === "Running" }
            ]
            onButton: id => v.svcAct(sel, id)
            function focusFirst() { table.focusList(); }
            Text {
                Layout.fillWidth: true; Layout.leftMargin: 20
                visible: v.svcError !== ""
                text: v.svcError
                color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
            }
            DataTable {
                id: table
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: svcModel
                tableId: "services"
                rowHeight: 30
                widths: tm.pref("cols_services", ({}))
                sortKey: v.sortPref("services", "name", false).key
                sortDesc: v.sortPref("services", "name", false).desc
                columns: [
                    { key: "name", title: "Name", width: 230 },
                    { key: "pid", title: "PID", width: 72, right: true },
                    { key: "desc", title: "Description", flex: true },
                    { key: "status", title: "Status", width: 96 },
                    { key: "group", title: "Group", width: 80 }
                ]
                cell: (r, k) => v.svcCell(r, k)
                empty: v.svcLoaded ? "No services found." : "Reading the services…"
                onSortClicked: k => { v.setSort("services", k, false); v.buildServices(); }
                onResized: w => tm.setPref("cols_services", w)
                onUp: sv.toBar()
                onBack: v.toRail()
                onActivated: i => { const p = table.rowPoint(i); v.svcMenu(svcModel.get(i), p.x, p.y); }
                onMenuAt: (i, x, y) => v.svcMenu(svcModel.get(i), x, y)
            }
            Text {
                Layout.fillWidth: true; Layout.margins: 14
                wrapMode: Text.WordWrap
                text: "Your own services can be started and stopped here, except the ones that keep Hearth, sound and the desktop working (marked Part of Hearth). The system's services are shown read only."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
        }
    }

    Component {
        id: settingsPage
        PageFrame {
            id: st
            onLeave: v.toRail()
            onUpOut: search.forceActiveFocus()
            title: "Settings"
            property int cursor: 0
            readonly property int rows: 5
            function focusFirst() { setBody.forceActiveFocus(); }
            FocusScope {
                id: setBody
                Layout.fillWidth: true
                Layout.fillHeight: true
                Keys.onPressed: e => {
                    const row = setCol.children[st.cursor];
                    switch (e.key) {
                    case Qt.Key_Down: case Qt.Key_Tab: st.cursor = Math.min(st.rows - 1, st.cursor + 1); break;
                    case Qt.Key_Up: case Qt.Key_Backtab: st.cursor = Math.max(0, st.cursor - 1); break;
                    case Qt.Key_Left: if (!(row && row.stepBy && row.stepBy(-1))) v.toRail(); break;
                    case Qt.Key_Right: if (row && row.stepBy) row.stepBy(1); break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (row && row.press) row.press(); break;
                    case Qt.Key_Escape: v.toRail(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
                Flickable {
                    anchors.fill: parent
                    contentHeight: setCol.implicitHeight + 40
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ColumnLayout {
                        id: setCol
                        x: 20; y: 4
                        width: Math.min(parent.width - 40, 760)
                        spacing: 8
                        ChoiceRow {
                            title: "Default start page"
                            sub: "The page Task Manager opens on."
                            lit: setBody.activeFocus && st.cursor === 0
                            options: [["processes", "Processes"], ["performance", "Performance"], ["startup", "Startup apps"], ["details", "Details"], ["services", "Services"], ["last", "Last page used"]]
                            value: tm.pref("startPage", "processes")
                            onChose: val => tm.setPref("startPage", val)
                            onClickedRow: { st.cursor = 0; setBody.forceActiveFocus(); }
                        }
                        ChoiceRow {
                            title: "Real-time update speed"
                            sub: "How often the numbers and graphs update. Slower uses less power; Paused stops until you choose a speed again (F5 updates once)."
                            lit: setBody.activeFocus && st.cursor === 1
                            options: [["high", "High (0.5 s)"], ["normal", "Normal (1 s)"], ["low", "Low (4 s)"], ["paused", "Paused"]]
                            value: tm.speed
                            onChose: val => tm.setPref("speed", val)
                            onClickedRow: { st.cursor = 1; setBody.forceActiveFocus(); }
                        }
                        SwitchRow {
                            title: "Always on top"
                            sub: "Keep Task Manager above other windows. The window opens again to apply it."
                            lit: setBody.activeFocus && st.cursor === 2
                            checked: tm.onTop
                            onFlipped: { tm.setPref("onTop", !tm.onTop); tm.reopen(); }
                            onClickedRow: { st.cursor = 2; setBody.forceActiveFocus(); }
                        }
                        SwitchRow {
                            title: "Fold the navigation rail"
                            sub: "Show only the icons on the left (☰ does the same)."
                            lit: setBody.activeFocus && st.cursor === 3
                            checked: tm.railFolded
                            onFlipped: tm.setPref("railFolded", !tm.railFolded)
                            onClickedRow: { st.cursor = 3; setBody.forceActiveFocus(); }
                        }
                        SwitchRow {
                            title: "Theme"
                            sub: "Task Manager follows Hearth's look, dark or light and your accent. Change them in Settings › Personalization."
                            lit: setBody.activeFocus && st.cursor === 4
                            link: "Personalization"
                            onFlipped: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "settings", "page", "appearance"])
                            onClickedRow: { st.cursor = 4; setBody.forceActiveFocus(); }
                        }
                    }
                }
            }
        }
    }

    // ================================================ overlays: the menu and the sheet
    MouseArea {
        anchors.fill: parent
        visible: v.menuOpen
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: v.closeMenu()
    }
    FocusScope {
        id: menuKeys
        visible: v.menuOpen
        x: v.menuX < 0 ? (v.width - width) / 2 : Math.max(8, Math.min(v.width - width - 8, v.menuX))
        y: v.menuY < 0 ? (v.height - height) / 2 : (v.menuY + height > v.height - 8 ? Math.max(8, v.menuY - height) : v.menuY)
        width: styledMenu.styled ? styledMenu.width : menuCard.width
        height: styledMenu.styled ? styledMenu.height : menuCard.height
        Keys.onPressed: e => {
            if (styledMenu.styled) { if (styledMenu.item.key(e)) e.accepted = true; return; }
            const n = v.menuItems.length;
            const step = d => {
                let i = v.menuIndex;
                for (let k = 0; k < n; k++) {
                    i = (i + d + n) % n;
                    const it = v.menuItems[i];
                    if (!it.sep && it.enabled !== false) { v.menuIndex = i; return; }
                }
            };
            switch (e.key) {
            case Qt.Key_Down: case Qt.Key_Tab: step(1); break;
            case Qt.Key_Up: case Qt.Key_Backtab: step(-1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: v.pickMenu(v.menuIndex); break;
            case Qt.Key_Escape: case Qt.Key_Left: case Qt.Key_Menu: v.closeMenu(); break;
            default: return;
            }
            e.accepted = true;
        }
        // Another right-click menu look (Settings › Personalization › Bar and dock › Right-click menu), when Hearth has it:
        // MenuStyles.qml by file name; today's menu below otherwise.
        Loader {
            id: styledMenu
            readonly property bool styled: (Theme.menuStyle || "today") !== "today" && status === Loader.Ready && item !== null
            active: (Theme.menuStyle || "today") !== "today"
            source: active ? "MenuStyles.qml" : ""
            visible: styled
            width: item ? item.implicitWidth : 0
            height: item ? item.implicitHeight : 0
            onLoaded: { if (v.menuOpen) item.open(v.menuTitle, v.menuItems.filter(i => i.enabled !== false)); }
            Connections {
                target: styledMenu.item
                ignoreUnknownSignals: true
                function onChosen(it) { const i = v.menuItems.findIndex(x => x.id === it.id); v.pickMenu(i); }
                function onClosed() { v.closeMenu(); }
            }
        }
        Rectangle {
            id: menuCard
            visible: !styledMenu.styled
            width: 248
            height: menuCol.implicitHeight + 12
            radius: Theme.r(8)
            color: Theme.panel
            border.color: Theme.line
            border.width: 1
            Column {
                id: menuCol
                x: 6; y: 6
                width: parent.width - 12
                Text {
                    width: parent.width
                    visible: v.menuTitle !== ""
                    leftPadding: 10; topPadding: 4; bottomPadding: 6
                    text: v.menuTitle
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                    elide: Text.ElideRight
                }
                Repeater {
                    model: v.menuOpen ? v.menuItems : []
                    delegate: Item {
                        id: mi
                        required property var modelData
                        required property int index
                        readonly property bool on: modelData.enabled !== false
                        width: menuCol.width
                        height: modelData.sep ? 9 : 32
                        Rectangle { visible: !!mi.modelData.sep; anchors.centerIn: parent; width: parent.width - 8; height: 1; color: Theme.line }
                        Rectangle {
                            visible: !mi.modelData.sep
                            anchors.fill: parent
                            radius: Theme.r(4)
                            color: v.menuIndex === mi.index && mi.on ? Theme.panel2 : miMouse.containsMouse && mi.on ? Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.6) : "transparent"
                            Rectangle { visible: v.menuIndex === mi.index && mi.on; width: 3; height: 16; radius: 1.5; color: Theme.accent; anchors { left: parent.left; verticalCenter: parent.verticalCenter } }
                            RowLayout {
                                anchors { fill: parent; leftMargin: 12; rightMargin: 10 }
                                spacing: 10
                                Item {
                                    Layout.preferredWidth: 16; Layout.preferredHeight: 16
                                    Icon { anchors.fill: parent; visible: !!mi.modelData.icon && !mi.modelData.checked; name: mi.modelData.icon || "dot"; size: 16; color: mi.on ? Theme.text : Theme.muted }
                                    Icon { anchors.fill: parent; visible: !!mi.modelData.checked; name: "check"; size: 16; color: Theme.accent }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: mi.modelData.label || ""
                                    color: !mi.on ? Theme.muted : mi.modelData.danger ? Theme.danger : Theme.text
                                    opacity: mi.on ? 1 : 0.6
                                    font.family: Theme.font; font.pixelSize: 13
                                    elide: Text.ElideRight
                                }
                                Text { visible: !!mi.modelData.key; text: mi.modelData.key || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                            }
                            MouseArea {
                                id: miMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                enabled: mi.on
                                onEntered: v.menuIndex = mi.index
                                onClicked: v.pickMenu(mi.index)
                            }
                        }
                    }
                }
            }
        }
    }
    Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "look"; value: Theme.menuStyle || "windows" }
    Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "maxHeight"; value: v.height - 16 }

    Rectangle {
        anchors.fill: parent
        visible: v.sheetMode !== ""
        color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.7)
        MouseArea { anchors.fill: parent; onClicked: v.closeSheet() }
        Rectangle {
            id: sheetCard
            anchors.centerIn: parent
            width: Math.min(parent.width - 48, 480)
            height: sheetCol.implicitHeight + 44
            radius: Theme.r(10)
            color: Theme.panel
            border.color: Theme.line
            MouseArea { anchors.fill: parent }
            Keys.onPressed: e => {
                const n = v.sheetButtons.length;
                switch (e.key) {
                case Qt.Key_Left: case Qt.Key_Backtab: v.sheetIndex = Math.max(0, v.sheetIndex - 1); break;
                case Qt.Key_Right: case Qt.Key_Tab: v.sheetIndex = Math.min(n - 1, v.sheetIndex + 1); break;
                case Qt.Key_Up: if (v.sheetMode === "run" || v.sheetMode === "ask") runInput.forceActiveFocus(); break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (n) v.sheetAction(v.sheetButtons[v.sheetIndex].action); break;
                case Qt.Key_Escape: v.closeSheet(); break;
                default: return;
                }
                e.accepted = true;
            }
            ColumnLayout {
                id: sheetCol
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 }
                spacing: 10
                Text { Layout.fillWidth: true; text: v.sheetTitle; color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: v.sheetText; color: v.sheetMode === "error" ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
                Rectangle {
                    visible: v.sheetMode === "run" || v.sheetMode === "ask"
                    Layout.fillWidth: true
                    Layout.preferredHeight: 36
                    radius: Theme.r(6)
                    color: Theme.bg2
                    border.width: runInput.activeFocus ? 2 : 1
                    border.color: runInput.activeFocus ? Theme.accent : Theme.line
                    TextInput {
                        id: runInput
                        anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.text; selectionColor: Theme.accent; selectedTextColor: Theme.accentInk
                        font.family: Theme.font; font.pixelSize: 14
                        clip: true
                        Keys.onPressed: e => {
                            if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { if (v.sheetMode === "ask") v.sheetAction("ask"); else v.runTask(); e.accepted = true; }
                            else if (e.key === Qt.Key_Escape) { v.closeSheet(); e.accepted = true; }
                            else if (e.key === Qt.Key_Down) { sheetCard.forceActiveFocus(); e.accepted = true; }
                        }
                        Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter; visible: runInput.text === "" && v.sheetMode === "run"; text: "Open: e.g. firefox, foot, ~/Documents"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 8
                    spacing: 8
                    Item { Layout.fillWidth: true }
                    Repeater {
                        model: v.sheetButtons
                        delegate: Rectangle {
                            id: btn
                            required property var modelData
                            required property int index
                            readonly property bool lit: v.sheetIndex === index && sheetCard.activeFocus
                            implicitWidth: btnText.implicitWidth + 32
                            implicitHeight: 34
                            radius: Theme.r(6)
                            color: lit ? (modelData.danger ? Theme.danger : Theme.accent) : btnMouse.containsMouse ? Theme.panel3 : Theme.panel2
                            border.color: index === 0 && !lit ? Theme.accent : "transparent"
                            Text {
                                id: btnText
                                anchors.centerIn: parent
                                text: btn.modelData.label
                                color: btn.lit ? (btn.modelData.danger ? "#ffffff" : Theme.accentInk) : btn.modelData.danger ? Theme.danger : Theme.text
                                font.family: Theme.font; font.pixelSize: 13
                            }
                            MouseArea { id: btnMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { v.sheetIndex = btn.index; v.sheetAction(btn.modelData.action); } }
                        }
                    }
                }
            }
        }
    }

    // ================================================ building blocks
    // A rail entry: icon and label, the accent bar on the page you're on.
    component RailItem: Rectangle {
        id: ri
        property string icon
        property string label
        property bool folded: false
        property bool lit: false
        property bool focusRing: false
        property string tip: ""
        signal clicked()
        Layout.fillWidth: true
        Layout.preferredHeight: 38
        radius: Theme.r(6)
        color: lit ? Theme.panel2 : riMouse.containsMouse ? Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.6) : "transparent"
        border.width: focusRing ? 2 : 0
        border.color: Theme.accent
        Rectangle { visible: ri.lit; width: 3; height: 16; radius: 1.5; color: Theme.accent; anchors { left: parent.left; leftMargin: 1; verticalCenter: parent.verticalCenter } }
        Icon { id: riIcon; anchors.left: parent.left; anchors.leftMargin: 14; anchors.verticalCenter: parent.verticalCenter; name: ri.icon; size: 17; color: ri.lit ? Theme.accent : Theme.text }
        Text {
            anchors { left: riIcon.right; leftMargin: 14; right: parent.right; rightMargin: 8; verticalCenter: parent.verticalCenter }
            visible: !ri.folded && ri.label !== ""
            text: ri.label
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: ri.lit ? Font.DemiBold : Font.Normal
            elide: Text.ElideRight
        }
        MouseArea { id: riMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ri.clicked() }
    }

    // A page: its title, a line beside it, the command buttons on the right; Up from the content reaches them.
    component PageFrame: FocusScope {
        id: pfr
        property string title
        property string subtitle: ""
        property var buttons: []
        property int barIndex: 0
        default property alias content: body.data
        signal button(string which, real x, real y)
        signal leave()          // Left / B out of the page: back to the rail
        signal upOut()          // Up from the command bar: the search box
        function toBar() {
            const i = buttons.findIndex(b => b.enabled !== false);
            if (i < 0) return;
            barIndex = i;
            bar.forceActiveFocus();
        }
        function pressAt(i) {
            const b = buttons[i];
            if (!b || b.enabled === false) return;
            const it = barRep.itemAt(i);
            const p = it ? it.mapToItem(null, 0, it.height + 4) : Qt.point(-1, -1);
            button(b.id, p.x, p.y);
        }
        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 56
                Layout.leftMargin: 20; Layout.rightMargin: 14
                spacing: 18
                Text { text: pfr.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                Text { visible: pfr.subtitle !== ""; text: pfr.subtitle; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                Item { Layout.fillWidth: true }
                FocusScope {
                    id: bar
                    Layout.preferredWidth: barRow.implicitWidth
                    Layout.preferredHeight: 34
                    Keys.onPressed: e => {
                        const n = pfr.buttons.length;
                        const step = d => { let i = pfr.barIndex; for (let k = 0; k < n; k++) { i = Math.max(0, Math.min(n - 1, i + d)); if (pfr.buttons[i].enabled !== false) { pfr.barIndex = i; return; } } };
                        switch (e.key) {
                        case Qt.Key_Left: case Qt.Key_Backtab: if (pfr.barIndex <= pfr.buttons.findIndex(b => b.enabled !== false)) pfr.leave(); else step(-1); break;
                        case Qt.Key_Right: case Qt.Key_Tab: step(1); break;
                        case Qt.Key_Down: pfr.focusFirst(); break;
                        case Qt.Key_Up: pfr.upOut(); break;
                        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: pfr.pressAt(pfr.barIndex); break;
                        case Qt.Key_Escape: pfr.focusFirst(); break;
                        default: return;
                        }
                        e.accepted = true;
                    }
                    Row {
                        id: barRow
                        spacing: 4
                        Repeater {
                            id: barRep
                            model: pfr.buttons
                            delegate: Rectangle {
                                id: cb
                                required property var modelData
                                required property int index
                                readonly property bool on: modelData.enabled !== false
                                readonly property bool ring: bar.activeFocus && pfr.barIndex === index
                                width: cbRow.implicitWidth + 24
                                height: 34
                                radius: Theme.r(6)
                                color: modelData.primary ? (cbMouse.containsMouse && on ? Theme.panel3 : Theme.panel2) : cbMouse.containsMouse && on ? Theme.panel2 : modelData.checked ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : "transparent"
                                border.width: ring ? 2 : modelData.primary ? 1 : 0
                                border.color: ring ? Theme.accent : Theme.line
                                opacity: on ? 1 : 0.45
                                Row {
                                    id: cbRow
                                    anchors.centerIn: parent
                                    spacing: 8
                                    Icon { visible: !!cb.modelData.icon && cb.modelData.icon !== "chevd"; anchors.verticalCenter: parent.verticalCenter; name: cb.modelData.icon || "dot"; size: 15; color: cb.modelData.checked ? Theme.accent : Theme.text }
                                    Text { anchors.verticalCenter: parent.verticalCenter; text: cb.modelData.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                    Icon { visible: cb.modelData.icon === "chevd"; anchors.verticalCenter: parent.verticalCenter; name: "chevd"; size: 13; color: Theme.text }
                                }
                                MouseArea {
                                    id: cbMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    enabled: cb.on
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: { pfr.barIndex = cb.index; pfr.pressAt(cb.index); }
                                }
                            }
                        }
                    }
                }
            }
            ColumnLayout {
                id: body
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0
            }
        }
    }

    // A table like Windows': resizable, sortable column headers (with totals above the names), rows with heat-map
    // cells, sections ("Apps (6)"), rows that unfold. cell(row, key) → {t, heat, muted, good, bad, badge, leaf}.
    component DataTable: FocusScope {
        id: dt
        property var model
        property var columns: []
        property var cell: (r, k) => ({ t: "" })
        property string tableId: ""
        property var widths: ({})
        property string sortKey: ""
        property bool sortDesc: false
        property int rowHeight: 32
        property string empty: ""
        property alias currentIndex: list.currentIndex
        property bool showIcons: false        // the unfold arrow and the app's picture in Name (Processes)
        property var totals: ({})             // key → the total shown above a column's name
        readonly property color goodColor: Theme.dark ? "#5fd08a" : "#1e8a46"
        // Heat-map shading: Windows' pale yellow to strong orange, tinted with your accent (stronger in light mode).
        function mixc(a, b, t) { return Qt.rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1); }
        function heatColor(level) {
            const t = Math.max(0, Math.min(5, level)) / 5;
            const base = mixc(mixc(Theme.flame, Theme.ember, t), Theme.accent, 0.22);
            const a = Theme.dark ? 0.07 + 0.6 * t : 0.12 + 0.62 * t;
            return Qt.rgba(base.r, base.g, base.b, a);
        }
        signal sortClicked(string key)
        signal resized(var widths)
        signal up()
        signal back()
        signal activated(int index)
        signal menuAt(int index, real x, real y)
        signal toggled(int index, bool on)
        signal deleteKey(int index)
        property var liveWidths: ({})
        readonly property bool hasTotals: columns.some(c => !!c.hasTotal)
        function widthOf(c) {
            if (c.flex) {
                let used = 0;
                for (const o of columns) if (!o.flex) used += widthOf(o);
                return Math.max(140, list.width - used);
            }
            const w = liveWidths[c.key] !== undefined ? liveWidths[c.key] : widths[c.key];
            return typeof w === "number" && w >= 40 ? w : c.width;
        }
        function focusList() {
            if (list.count === 0) { list.forceActiveFocus(); return; }
            if (list.currentIndex < 0 || list.currentIndex >= list.count || isSection(list.currentIndex)) list.currentIndex = firstRow();
            list.forceActiveFocus();
            list.positionViewAtIndex(list.currentIndex, ListView.Contain);
        }
        function isSection(i) { const r = dt.model.get(i); return !!r && r.type === "section"; }
        function firstRow() { for (let i = 0; i < list.count; i++) if (!isSection(i)) return i; return -1; }
        function indexOfKey(k) { for (let i = 0; i < dt.model.count; i++) if (dt.model.get(i).key === k) return i; return -1; }
        function rowPoint(i) {
            const it = list.itemAtIndex(i);
            if (!it) return Qt.point(-1, -1);
            return it.mapToItem(null, 40, it.height);
        }
        function step(d) {
            if (list.count === 0) return;
            let i = list.currentIndex < 0 ? (d > 0 ? -1 : list.count) : list.currentIndex;
            const lim = Math.abs(d) > 1 ? Math.abs(d) : 1;
            let moved = 0;
            while (moved < lim) {
                let j = i + (d > 0 ? 1 : -1);
                while (j >= 0 && j < list.count && isSection(j)) j += d > 0 ? 1 : -1;
                if (j < 0 || j >= list.count) break;
                i = j;
                moved++;
            }
            if (moved === 0 && d < 0) { dt.up(); return; }
            list.currentIndex = i;
            list.positionViewAtIndex(i, ListView.Contain);
        }

        // header
        Item {
            id: head
            anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 4; rightMargin: 4 }
            height: dt.hasTotals ? 50 : 32
            Row {
                anchors.fill: parent
                Repeater {
                    model: dt.columns
                    delegate: Item {
                        id: hc
                        required property var modelData
                        required property int index
                        width: dt.widthOf(modelData)
                        height: head.height
                        readonly property bool sorted: dt.sortKey === modelData.key
                        Rectangle { anchors.fill: parent; color: hcMouse.containsMouse ? Theme.panel2 : "transparent"; radius: Theme.r(4) }
                        Column {
                            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 10; rightMargin: 10; bottomMargin: 5 }
                            spacing: 0
                            Text {
                                width: parent.width
                                visible: !!hc.modelData.hasTotal
                                horizontalAlignment: hc.modelData.right ? Text.AlignRight : Text.AlignLeft
                                text: dt.totals[hc.modelData.key] || ""
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 15
                            }
                            Text {
                                width: parent.width
                                horizontalAlignment: hc.modelData.right ? Text.AlignRight : Text.AlignLeft
                                text: hc.modelData.title + (hc.sorted ? (dt.sortDesc ? "  ˅" : "  ˄") : "")
                                color: hc.sorted ? Theme.text : Theme.muted
                                font.family: Theme.font; font.pixelSize: 11
                                elide: Text.ElideRight
                            }
                        }
                        Rectangle { anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; anchors.topMargin: 8; anchors.bottomMargin: 8; width: 1; color: Theme.line; visible: hc.index > 0 }
                        MouseArea { id: hcMouse; anchors.fill: parent; hoverEnabled: true; onClicked: dt.sortClicked(hc.modelData.key) }
                        // dragging a column's left edge makes it wider or narrower (remembered)
                        MouseArea {
                            visible: !hc.modelData.flex
                            anchors { left: parent.left; top: parent.top; bottom: parent.bottom; leftMargin: -4 }
                            width: 8
                            cursorShape: Qt.SplitHCursor
                            property real startX: 0
                            property real startW: 0
                            onPressed: m => { startX = mapToItem(head, m.x, 0).x; startW = hc.width; }
                            onPositionChanged: m => {
                                if (!pressed) return;
                                const w = Math.max(48, Math.min(600, startW - (mapToItem(head, m.x, 0).x - startX)));
                                const lw = Object.assign({}, dt.liveWidths);
                                lw[hc.modelData.key] = Math.round(w);
                                dt.liveWidths = lw;
                            }
                            onReleased: {
                                const out = Object.assign({}, dt.widths, dt.liveWidths);
                                dt.liveWidths = ({});
                                dt.resized(out);
                            }
                        }
                    }
                }
            }
            Rectangle { anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; height: 1; color: Theme.line }
        }

        ListView {
            id: list
            anchors { left: parent.left; right: parent.right; top: head.bottom; bottom: parent.bottom; leftMargin: 4; rightMargin: 4 }
            model: dt.model
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            keyNavigationEnabled: false
            currentIndex: -1
            highlightFollowsCurrentItem: false
            Keys.onPressed: e => {
                const i = list.currentIndex;
                const r = i >= 0 && i < list.count ? dt.model.get(i) : null;
                switch (e.key) {
                case Qt.Key_Down: case Qt.Key_Tab: dt.step(1); break;
                case Qt.Key_Up: case Qt.Key_Backtab: dt.step(-1); break;
                case Qt.Key_PageDown: dt.step(Math.max(2, Math.floor(list.height / dt.rowHeight) - 1)); break;
                case Qt.Key_PageUp: dt.step(-Math.max(2, Math.floor(list.height / dt.rowHeight) - 1)); break;
                case Qt.Key_Home: list.currentIndex = dt.firstRow(); list.positionViewAtBeginning(); break;
                case Qt.Key_End: list.currentIndex = list.count - 1; list.positionViewAtEnd(); break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (r && r.type !== "section") dt.activated(i); break;
                case Qt.Key_Menu: if (r && r.type !== "section") { const p = dt.rowPoint(i); dt.menuAt(i, p.x, p.y); } break;
                case Qt.Key_Right: if (r && r.expandable && !r.expanded) dt.toggled(i, true); break;
                case Qt.Key_Left:
                    if (r && r.expandable && r.expanded) dt.toggled(i, false);
                    else if (r && r.depth > 0) dt.toggled(i, false);
                    else dt.back();
                    break;
                case Qt.Key_Escape: dt.back(); break;
                case Qt.Key_Delete: if (r && r.type !== "section") dt.deleteKey(i); break;
                default: return;
                }
                e.accepted = true;
            }
            delegate: Item {
                id: row
                required property var model
                required property int index
                readonly property bool section: model.type === "section"
                readonly property bool current: ListView.isCurrentItem
                width: ListView.view.width
                height: section ? 36 : dt.rowHeight
                Rectangle {
                    anchors.fill: parent
                    visible: !row.section
                    radius: Theme.r(4)
                    color: row.current ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, Theme.dark ? 0.16 : 0.14) : rowMouse.containsMouse ? Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.7) : "transparent"
                    border.width: row.current && list.activeFocus ? 2 : row.current ? 1 : 0
                    border.color: row.current && list.activeFocus ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.45)
                    z: 2
                    opacity: 1
                }
                Text {
                    visible: row.section
                    anchors { left: parent.left; leftMargin: 10; bottom: parent.bottom; bottomMargin: 6 }
                    text: row.model.label || ""
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                }
                Row {
                    visible: !row.section
                    anchors.fill: parent
                    Repeater {
                        model: row.section ? [] : dt.columns
                        delegate: Item {
                            id: cellItem
                            required property var modelData
                            required property int index
                            readonly property var c: dt.cell(row.model, modelData.key) || ({ t: "" })
                            width: dt.widthOf(modelData)
                            height: row.height
                            Rectangle {
                                anchors { fill: parent; topMargin: 1; bottomMargin: 1 }
                                visible: cellItem.c.heat !== undefined
                                color: dt.heatColor(cellItem.c.heat || 0)
                            }
                            // Name: unfold arrow, the app's picture, the name, how many processes, a tag (Steam, Windows app, Part of Hearth)
                            RowLayout {
                                visible: cellItem.modelData.key === "name"
                                anchors { fill: parent; leftMargin: 6 + (row.model.depth || 0) * 26; rightMargin: 8 }
                                spacing: 8
                                Item {
                                    Layout.preferredWidth: 14; Layout.preferredHeight: 14
                                    visible: dt.showIcons
                                    Icon {
                                        anchors.centerIn: parent
                                        visible: !!row.model.expandable
                                        name: row.model.expanded ? "chevd" : "chev"
                                        size: 12; color: Theme.muted
                                    }
                                    MouseArea { anchors.fill: parent; anchors.margins: -6; enabled: !!row.model.expandable; onClicked: dt.toggled(row.index, !row.model.expanded) }
                                }
                                Item {
                                    Layout.preferredWidth: 18; Layout.preferredHeight: 18
                                    visible: dt.showIcons
                                    IconImage {
                                        anchors.fill: parent
                                        visible: (row.model.icon || "") !== ""
                                        implicitSize: 18
                                        source: (row.model.icon || "") === "" ? "" : AppIcons.source(row.model.icon, "")
                                    }
                                    Icon {
                                        anchors.centerIn: parent
                                        visible: (row.model.icon || "") === ""
                                        name: row.model.locked ? "lock" : row.model.type === "child" ? "dot" : "gear"
                                        size: 15; color: Theme.muted
                                    }
                                }
                                Text {
                                    Layout.fillWidth: (row.model.tag || "") === ""
                                    Layout.maximumWidth: cellItem.width - 110
                                    text: (row.model.name || "") + (row.model.expandable ? " (" + row.model.count + ")" : "")
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                                    elide: Text.ElideRight
                                }
                                Rectangle {
                                    visible: (row.model.tag || "") !== ""
                                    Layout.preferredHeight: 18
                                    Layout.preferredWidth: tagText.implicitWidth + 12
                                    radius: Theme.r(4)
                                    color: row.model.tag === "Part of Hearth" ? Qt.rgba(Theme.ember.r, Theme.ember.g, Theme.ember.b, 0.18) : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2)
                                    Text { id: tagText; anchors.centerIn: parent; text: row.model.tag || ""; color: row.model.tag === "Part of Hearth" ? Theme.ember : Theme.accent; font.family: Theme.font; font.pixelSize: 10 }
                                }
                                Item { Layout.fillWidth: (row.model.tag || "") !== "" }
                            }
                            // everything else
                            Row {
                                visible: cellItem.modelData.key !== "name" && !cellItem.c.badge
                                anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                                layoutDirection: cellItem.modelData.right ? Qt.RightToLeft : Qt.LeftToRight
                                spacing: 6
                                Icon { visible: !!cellItem.c.leaf; anchors.verticalCenter: parent.verticalCenter; name: "leaf"; size: 14; color: Theme.accent }
                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: Math.max(0, parent.width - (cellItem.c.leaf ? 20 : 0))
                                    horizontalAlignment: cellItem.modelData.right ? Text.AlignRight : Text.AlignLeft
                                    text: cellItem.c.t || ""
                                    color: cellItem.c.bad ? Theme.danger : cellItem.c.good ? dt.goodColor : cellItem.c.muted ? Theme.muted : Theme.text
                                    font.family: Theme.font; font.pixelSize: 12
                                    elide: Text.ElideRight
                                }
                            }
                            Rectangle {
                                visible: !!cellItem.c.badge
                                anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                                height: 20
                                width: badgeText.implicitWidth + 14
                                radius: Theme.r(4)
                                color: Qt.rgba(Theme.ember.r, Theme.ember.g, Theme.ember.b, 0.18)
                                Text { id: badgeText; anchors.centerIn: parent; text: cellItem.c.t || ""; color: Theme.ember; font.family: Theme.font; font.pixelSize: 11 }
                            }
                        }
                    }
                }
                Rectangle { visible: !row.section; anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; height: 1; color: Qt.rgba(Theme.line.r, Theme.line.g, Theme.line.b, 0.5) }
                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    z: -1
                    enabled: !row.section
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: m => {
                        list.currentIndex = row.index;
                        list.forceActiveFocus();
                        if (m.button === Qt.RightButton) { const p = mapToItem(null, m.x, m.y); dt.menuAt(row.index, p.x, p.y); }
                    }
                    onDoubleClicked: m => { if (m.button === Qt.LeftButton) { if (row.model.expandable) dt.toggled(row.index, !row.model.expanded); else dt.activated(row.index); } }
                    onPressAndHold: m => { list.currentIndex = row.index; const p = mapToItem(null, m.x, m.y); dt.menuAt(row.index, p.x, p.y); }
                }
            }
        }
        Text {
            anchors.centerIn: list
            visible: list.count === 0 && dt.empty !== ""
            width: list.width - 60
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: dt.empty
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }
    }
    // A setting with chips to choose from (Left / Right with the keys or the controller).
    component ChoiceRow: Rectangle {
        id: cr
        property string title
        property string sub
        property var options: []
        property string value
        property bool lit: false
        signal chose(string val)
        signal clickedRow()
        function stepBy(d) {
            const i = options.findIndex(o => o[0] === value);
            const j = i + d;
            if (j < 0 || j >= options.length) return false;
            chose(options[j][0]);
            return true;
        }
        function press() { const i = options.findIndex(o => o[0] === value); chose(options[(i + 1) % options.length][0]); }
        Layout.fillWidth: true
        implicitHeight: crCol.implicitHeight + 26
        radius: Theme.r(8)
        color: Theme.panel
        border.width: lit ? 2 : 1
        border.color: lit ? Theme.accent : Theme.line
        MouseArea { anchors.fill: parent; onClicked: cr.clickedRow() }
        ColumnLayout {
            id: crCol
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 16 }
            spacing: 4
            Text { text: cr.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { Layout.fillWidth: true; text: cr.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
            Flow {
                Layout.fillWidth: true
                Layout.topMargin: 6
                spacing: 6
                Repeater {
                    model: cr.options
                    delegate: Rectangle {
                        id: chip
                        required property var modelData
                        readonly property bool on: cr.value === modelData[0]
                        width: chipText.implicitWidth + 24
                        height: 30
                        radius: Theme.r(6)
                        color: on ? Theme.accent : chipMouse.containsMouse ? Theme.panel3 : Theme.panel2
                        Text { id: chipText; anchors.centerIn: parent; text: chip.modelData[1]; color: chip.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                        MouseArea { id: chipMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { cr.clickedRow(); cr.chose(chip.modelData[0]); } }
                    }
                }
            }
        }
    }

    // A setting with a switch (or a link button instead).
    component SwitchRow: Rectangle {
        id: sr
        property string title
        property string sub
        property bool checked: false
        property string link: ""
        property bool lit: false
        signal flipped()
        signal clickedRow()
        function press() { flipped(); }
        Layout.fillWidth: true
        implicitHeight: srCol.implicitHeight + 26
        radius: Theme.r(8)
        color: Theme.panel
        border.width: lit ? 2 : 1
        border.color: lit ? Theme.accent : Theme.line
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { sr.clickedRow(); sr.flipped(); } }
        RowLayout {
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 16 }
            spacing: 14
            ColumnLayout {
                id: srCol
                Layout.fillWidth: true
                spacing: 4
                Text { text: sr.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                Text { Layout.fillWidth: true; text: sr.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
            }
            Rectangle {
                visible: sr.link !== ""
                Layout.preferredWidth: linkText.implicitWidth + 24
                Layout.preferredHeight: 30
                radius: Theme.r(6)
                color: Theme.panel2
                border.color: Theme.line
                Text { id: linkText; anchors.centerIn: parent; text: sr.link; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
            }
            Rectangle {
                visible: sr.link === ""
                Layout.preferredWidth: 42; Layout.preferredHeight: 22
                radius: 11
                color: sr.checked ? Theme.accent : "transparent"
                border.width: sr.checked ? 0 : 1
                border.color: Theme.muted
                Rectangle {
                    width: 14; height: 14; radius: 7
                    y: 4; x: sr.checked ? parent.width - width - 4 : 4
                    color: sr.checked ? Theme.accentInk : Theme.muted
                    Behavior on x { NumberAnimation { duration: Theme.quick } }
                }
            }
        }
    }

    // The last minute as a line with a soft fill, on Windows' grid (10 rows, a line every 10 seconds).
    // maxValue 0: scales to what was seen (network).
    component Graph: Canvas {
        id: graph
        property var values: []
        property int capacity: 61
        property real maxValue: 100
        property color lineColor: Theme.accent
        property bool grid: true
        property bool framed: true
        property real scaleTop: 1
        onValuesChanged: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onLineColorChanged: requestPaint()
        function css(c, a) { return "rgba(" + Math.round(c.r * 255) + "," + Math.round(c.g * 255) + "," + Math.round(c.b * 255) + "," + (a === undefined ? c.a : a) + ")"; }
        onPaint: {
            const ctx = getContext("2d");
            const w = width, h = height;
            ctx.reset();
            ctx.fillStyle = css(Theme.bg);
            ctx.fillRect(0, 0, w, h);
            if (grid) {
                ctx.strokeStyle = css(lineColor, 0.16);
                ctx.lineWidth = 1;
                for (let i = 1; i < 10; i++) { const y = Math.round(h * i / 10) + 0.5; ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke(); }
                for (let i = 1; i < 6; i++) { const x = Math.round(w * i / 6) + 0.5; ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke(); }
            }
            const vals = values || [];
            let mx = maxValue;
            if (mx <= 0) {
                mx = 1;
                for (const n of vals) mx = Math.max(mx, n);
                let s = 1;
                while (s < mx) s *= 2;
                mx = s;
                scaleTop = mx;
            }
            if (vals.length >= 2) {
                const stepX = w / Math.max(1, capacity - 1);
                const x0 = w - (vals.length - 1) * stepX;
                const yOf = n => h - 1 - (h - 2) * Math.max(0, Math.min(mx, n)) / mx;
                ctx.beginPath();
                ctx.moveTo(x0, h);
                for (let i = 0; i < vals.length; i++) ctx.lineTo(x0 + i * stepX, yOf(vals[i]));
                ctx.lineTo(w, h);
                ctx.closePath();
                ctx.fillStyle = css(lineColor, 0.22);
                ctx.fill();
                ctx.beginPath();
                for (let i = 0; i < vals.length; i++) {
                    if (i === 0) ctx.moveTo(x0, yOf(vals[0]));
                    else ctx.lineTo(x0 + i * stepX, yOf(vals[i]));
                }
                ctx.strokeStyle = css(lineColor);
                ctx.lineWidth = 1.5;
                ctx.lineJoin = "round";
                ctx.stroke();
            }
            if (framed) {
                ctx.strokeStyle = css(lineColor, 0.9);
                ctx.lineWidth = 1;
                ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
            }
        }
    }
}
