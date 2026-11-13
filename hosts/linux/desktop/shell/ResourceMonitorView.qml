// Resource Monitor's page (loaded by ResourceMonitor.qml, which keeps the tab: rm.tab). The approved mockups' layout
// in Hearth's dark blue: the tabs (Your PC, Overview, Processor, Memory, Disk, Network, Graphics), per-program tables
// with tags (Game · Proton, Steam, Hearth, Home server, System), four live graphs, and Your PC's board with the details
// of the part you pick.
// One sampler (`hearth-resmon serve`) for every tab, told once a second what the tab on show needs; it stops when the
// window closes. "Your PC" is read once when the window opens (`hearth-resmon pc`).
// Mouse, touch, keyboard and the controller (hearth-pad's keys while it's in front: D-pad arrows, A Enter, B Esc,
// X the Menu key, LB / RB Ctrl+Page Up / Down):
//   LB / RB (or Ctrl+Tab) switch tabs; in a table Up / Down pick a program, A selects it, X (or a right-click) opens its
//   actions: End task, Efficiency mode, Open file location, Search online. On Your PC the D-pad moves between parts, A
//   shows a part's details and X runs its action (Disk Management, Device Manager, the volume mixer, a Settings page).
//   B closes the actions menu, then the window.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

FocusScope {
    id: v
    focus: true
    property var rm: null
    readonly property string helper: rm ? rm.helper : decodeURIComponent(Qt.resolvedUrl("hearth-resmon").toString().replace(/^file:\/\//, ""))
    readonly property string tab: rm ? rm.tab : "pc"

    // ================================================ the look (the mockups' palette)
    readonly property var c: ({ bg: "#171d25", bar: "#1b2838", card: "#1c2b3c", head: "#213347", line: "#2f4a63", accent: "#66c0f4", text: "#e6eef5",
                                 sub: "#c9d6e2", muted: "#9fb2c3", alt: "#1f3042", sel: "#2a475e", title: "#10151b", ok: "#7fe0a4", warn: "#ffb37a" })
    readonly property var tagColors: ({ game: "#f2c14e", steam: "#66c0f4", hearth: "#ff9a4d", app: "#9a7cff", sys: "#5d7488", web: "#7ccf8a", home: "#3fb8c9" })
    readonly property var tabList: [["pc", "Your PC"], ["overview", "Overview"], ["cpu", "Processor"], ["memory", "Memory"], ["disk", "Disk"], ["network", "Network"], ["gpu", "Graphics"]]

    // ================================================ numbers in words
    function thousands(n) { return String(Math.round(n || 0)).replace(/\B(?=(\d{3})+(?!\d))/g, ","); }
    function size(b) {
        if (b === undefined || b === null) return "–";
        if (b >= 1073741824) return (b / 1073741824).toFixed(b >= 10737418240 ? 0 : 1) + " GB";
        if (b >= 1048576) return Math.round(b / 1048576) + " MB";
        if (b >= 1024) return Math.round(b / 1024) + " KB";
        return Math.round(b) + " B";
    }
    function rate(bps) {
        const b = bps || 0;
        if (b >= 1048576) return (b / 1048576).toFixed(b >= 104857600 ? 0 : 1) + " MB/s";
        if (b >= 1024) return Math.round(b / 1024) + " KB/s";
        return b > 0 ? Math.round(b) + " B/s" : "0";
    }
    function pct(n) { return n === undefined || n === null ? "–" : (n >= 10 || n === 0 ? Math.round(n) : n.toFixed(1)) + "%"; }
    function temp(t) { return t === undefined || t === null ? "" : Math.round(t) + " °C"; }
    function ghz(mhz) { return mhz ? (mhz / 1000).toFixed(2) + " GHz" : "–"; }
    function tempNote(t) { return t === null || t === undefined ? "" : t >= 90 ? "Hot" : t >= 80 ? "Warm" : "Normal"; }

    // ================================================ the sampler
    property var sample: null
    property var groups: []
    property int samples: 0
    property string sampleError: ""
    readonly property int histMax: 60
    property var hist: ({ cpu: [], disk: [], net: [], mem: [], gpu: [] })
    function push(h, k, val) { const a = (h[k] || []).slice(-(histMax - 1)); a.push(val || 0); h[k] = a; }
    function applySample(line) {
        let d;
        try { d = JSON.parse(line); } catch (e) { return; }
        if (d.error) { sampleError = d.error; return; }
        sampleError = "";
        const h = Object.assign({}, hist);
        push(h, "cpu", d.cpu ? d.cpu.total : 0);
        push(h, "disk", (d.disks || []).reduce((a, x) => a + (x.read || 0) + (x.write || 0), 0));
        push(h, "net", (d.nets || []).reduce((a, x) => a + (x.recv || 0) + (x.send || 0), 0));
        push(h, "mem", d.memBar ? d.memBar.inUse : 0);
        push(h, "gpu", (d.gpus || []).reduce((a, g) => Math.max(a, g.busy || 0), 0));
        hist = h;
        sample = d;
        if (d.procs) groups = d.procs;
        samples++;
        updateLive();
    }
    function wants() {
        switch (tab) {
        case "pc": return ["procs", "gpu", "temps", "disk"];
        case "overview": return ["procs", "disk", "net", "temps"];
        case "cpu": return ["procs", "temps"];
        case "memory": return ["procs", "mem"];
        case "disk": return ["procs", "disk"];
        case "network": return ["procs", "net"];
        case "gpu": return ["procs", "gpu", "gpuprocs", "temps"];
        }
        return ["procs"];
    }
    readonly property bool streams: typeof sampler.write === "function"
    function sendConfig() {
        const msg = JSON.stringify({ interval: 1, want: wants() }) + "\n";
        if (streams && sampler.running) sampler.write(msg);
    }
    Process {
        id: sampler
        command: ["python3", v.helper, "serve"]
        running: v.rm !== null
        stdinEnabled: true
        stdout: SplitParser { onRead: line => v.applySample(line) }
        stderr: SplitParser { onRead: line => console.warn("hearth-resmon serve:", line) }
        onRunningChanged: if (running) v.sendConfig(); else restartSampler.start()
    }
    Timer { id: restartSampler; interval: 2000; onTriggered: if (!sampler.running && v.streams && v.rm) sampler.running = true }
    // A Quickshell without stdin for processes: one sample a second from `hearth-resmon sample` instead.
    Process { id: sampleOnce; stdout: StdioCollector { onStreamFinished: v.applySample(text.trim().split("\n").pop() || "") } }
    Timer {
        interval: 1000; repeat: true; triggeredOnStart: true
        running: !v.streams && v.rm !== null
        onTriggered: if (!sampleOnce.running) { sampleOnce.command = ["python3", v.helper, "sample"].concat(v.wants()); sampleOnce.running = true; }
    }
    onTabChanged: { sendConfig(); closeMenu(); listIndex = 0; }

    // ================================================ Your PC (read once when the window opens)
    property var pc: null
    property var pcView: null
    property bool pcLoading: false
    property string pcError: ""
    property string selected: ""
    property string cursorPart: ""
    function loadPc(refresh) {
        if (pcProc.running) return;
        pcLoading = true; pcError = "";
        const args = ["python3", v.helper, "pc"];
        if (rm && rm.fixture) args.push("--fixture", rm.fixture);
        if (refresh) args.push("--refresh");
        pcProc.command = args;
        pcProc.running = true;
    }
    Process {
        id: pcProc
        stdout: StdioCollector {
            onStreamFinished: {
                v.pcLoading = false;
                const line = text.trim().split("\n").pop() || "";
                if (line.startsWith("error: ")) { v.pcError = line.slice(7); return; }
                try {
                    const d = JSON.parse(line);
                    v.pc = d.pc; v.pcView = d.view;
                    if (!v.selected || !d.view.parts.some(p => p.id === v.selected)) v.selected = d.view.def || (d.view.parts[0] || {}).id || "";
                    v.cursorPart = v.selected;
                    v.updateLive();
                } catch (e) { v.pcError = "Couldn't read this PC's parts (" + e + ")"; }
            }
        }
    }
    function selectPart(id) {
        if (!pcView || !pcView.parts.some(p => p.id === id)) return;
        selected = id; cursorPart = id;
    }
    readonly property var selPart: pcView ? (pcView.parts.find(p => p.id === selected) || null) : null

    // ---- what's live on the board: busy parts get flow, and their numbers
    property var live: ({})
    function gpuFor(key) {
        const gs = (sample && sample.gpus) || [];
        const addr = key.slice(4);
        const pg = ((pc && pc.gpus) || []).find(g => g.pci === addr);
        return gs.find(g => (g.pci && addr.endsWith(g.pci.slice(-7))) || (pg && g.card && g.card === pg.card)) || gs.find(g => g.vendor === (pg || {}).vendor) || null;
    }
    function updateLive() {
        if (!pcView) return;
        const s = sample || {};
        const out = {};
        const cpuBusy = s.cpu ? s.cpu.total > 10 : false;
        const disks = {}; (s.disks || []).forEach(d => disks[d.name] = d);
        const nets = {}; (s.nets || []).forEach(n => nets[n.name] = n);
        const fans = s.fans || {};
        let chipBusy = false;
        for (const p of pcView.parts) {
            const k = p.live || "";
            let b = false;
            if (k === "cpu") b = cpuBusy;
            else if (k === "mem") b = s.cpu ? s.cpu.total > 5 : false;
            else if (k.startsWith("gpu:")) { const g = gpuFor(k); b = !!g && (g.busy || 0) > 10; }
            else if (k.startsWith("disk:")) { const d = disks[k.slice(5)]; b = !!d && ((d.active || 0) > 3 || (d.read || 0) + (d.write || 0) > 1048576); if (b && p.hub === "chip") chipBusy = true; }
            else if (k.startsWith("net:")) { const n = nets[k.slice(4)]; b = !!n && (n.recv || 0) + (n.send || 0) > 51200; if (b && p.hub === "chip") chipBusy = true; }
            else if (k.startsWith("fan:")) b = (fans[k.slice(4)] || 0) > 0;
            else if (k === "battery") b = !!(s.battery && s.battery.watts && s.battery.watts > 1);
            out[p.id] = { busy: b };
        }
        for (const p of pcView.parts) if (p.live === "chip") out[p.id] = { busy: chipBusy };
        live = out;
    }

    // ---- the details of the selected part, live
    function topBy(key, n, fmt) {
        return groups.filter(g => (g[key] || 0) > 0).sort((a, b) => (b[key] || 0) - (a[key] || 0)).slice(0, n).map(g => ({ n: g.name, v: fmt(g[key], g) }));
    }
    function partLive(p) {
        const s = sample || {};
        const k = (p && p.live) || "";
        if (k === "cpu" && s.cpu) return { stats: [["Busy", pct(s.cpu.total)], ["Temperature", temp(s.cpu.temp) || "–"], ["Speed", ghz(s.cpu.mhz)], ["Threads", thousands(s.cpu.threads)]], users: topBy("cpu", 3, x => pct(x)) };
        if (k === "mem" && s.memBar) return { stats: [["In use", size(s.memBar.inUse)], ["Cache", size(s.memBar.cache)]], users: topBy("mem", 3, x => size(x)) };
        if (k.startsWith("gpu:")) { const g = gpuFor(k); if (g) return { stats: [["Busy", pct(g.busy)], ["Video memory", g.vramTotal ? size(g.vramUsed) + " / " + size(g.vramTotal) : "–"], ["Temperature", temp(g.temp) || "–"], ["Power", g.watts ? Math.round(g.watts) + " W" : "–"]], users: topBy("gpu", 3, x => pct(x)) }; }
        if (k.startsWith("disk:")) { const d = (s.disks || []).find(x => x.name === k.slice(5)); if (d) return { stats: [["Reading", rate(d.read)], ["Writing", rate(d.write)], ["Active", pct(d.active)], ["Response", d.response !== null && d.response !== undefined ? d.response + " ms" : "–"]], users: topBy("disk", 3, x => rate(x)) }; }
        if (k.startsWith("net:")) { const n = (s.nets || []).find(x => x.name === k.slice(4)); if (n) return { stats: [["Down", rate(n.recv)], ["Up", rate(n.send)]], users: [] }; }
        if (k.startsWith("fan:")) { const r = (s.fans || {})[k.slice(4)]; if (r !== undefined) return { stats: [["Speed", thousands(r) + " rpm"]], users: [] }; }
        if (k === "battery" && s.battery) return { stats: [["Charge", s.battery.percent + "%"], ["Power", s.battery.watts ? s.battery.watts + " W" : "–"]], users: [] };
        return { stats: [], users: [] };
    }
    readonly property var selLive: { samples; return selPart ? partLive(selPart) : { stats: [], users: [] }; }

    // ---- what a part's button does (only ones that exist here)
    property var have: ({})
    Process {
        id: haveProc
        command: ["sh", "-c", "for c in hearth-disks gnome-disks; do command -v \"$c\" >/dev/null 2>&1 && echo \"$c\"; done"]
        stdout: StdioCollector { onStreamFinished: { const h = {}; for (const l of text.split("\n")) if (l.trim()) h[l.trim()] = true; v.have = h; } }
    }
    function actionOk(a) {
        if (!a) return false;
        switch (a.kind) {
        case "tab": case "settings": case "files": return true;
        case "disks": return !!have["hearth-disks"] || !!have["gnome-disks"];
        case "devices": return !!SettingsNav.winTools;
        case "mixer": return !!SettingsNav.mixer;
        }
        return false;
    }
    function ipc(args) { Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call"].concat(args)); }
    function runAction(a) {
        if (!actionOk(a)) return;
        switch (a.kind) {
        case "tab": if (rm) rm.tab = a.tab; break;
        case "settings": ipc(["settings", "page", a.page]); break;
        case "files": ipc(["files", "open", a.path || "/"]); break;
        case "disks": Quickshell.execDetached([have["hearth-disks"] ? "hearth-disks" : "gnome-disks"]); break;
        case "devices": ipc(["devices", "open"]); break;
        case "mixer": ipc(["mixer", "open"]); break;
        }
    }

    // ================================================ the per-program tables
    function tagColor(g) { return tagColors[g.color] || tagColors.app; }
    function rowsFor(t) {
        const gs = groups.slice();
        const by = (k) => gs.sort((a, b) => (b[k] || 0) - (a[k] || 0) || (b.mem || 0) - (a.mem || 0));
        switch (t) {
        case "cpu": return by("cpu").map(g => ({ g: g, a: pct(g.cpu), b: thousands(g.threads), c: pct(g.avg) }));
        case "memory": return by("mem").map(g => ({ g: g, a: size(g.mem), b: g.shared !== undefined ? size(g.shared) : "–", c: g.swap ? size(g.swap) : "0" }));
        case "disk": return gs.sort((a, b) => ((b.read || 0) + (b.write || 0)) - ((a.read || 0) + (a.write || 0)) || (b.disk || 0) - (a.disk || 0)).map(g => ({ g: g, a: rate(g.read !== undefined ? g.read : g.disk), b: rate(g.write || 0), c: rate((g.read || 0) + (g.write || 0) || g.disk) }));
        case "network": return gs.sort((a, b) => (b.conns || 0) - (a.conns || 0) || (b.cpu || 0) - (a.cpu || 0)).map(g => ({ g: g, a: g.conns ? String(g.conns) : "0", b: g.remote || "–", c: pct(g.cpu) }));
        case "gpu": return gs.sort((a, b) => (b.gpu || 0) - (a.gpu || 0) || (b.vram || 0) - (a.vram || 0)).map(g => ({ g: g, a: pct(g.gpu), b: g.vram ? size(g.vram) : "–", c: g.gpuUse || "" }));
        }
        return [];
    }
    readonly property var colsFor: ({ cpu: ["CPU ↓", "Threads", "Average"], memory: ["In use ↓", "Shared", "Compressed"], disk: ["Read", "Write", "Total ↓"],
                                      network: ["Connections ↓", "Talking to", "CPU"], gpu: ["GPU ↓", "Video memory", "Using"] })
    readonly property var tableRows: { samples; return rowsFor(tab === "overview" ? "cpu" : tab); }
    // Overview's state: folded sections and the programs you ticked (only while it's open; nothing is saved).
    property var folded: ({ disk: true, network: true, gpu: true })
    property var ticked: ({})
    readonly property var overviewCols: ({ cpu: ["CPU ↓", "Threads", "Average"], memory: ["In use ↓", "Shared", "Compressed"] })
    // The chips on each Overview heading: [[text, tone]] (tone: main, good, warn or "").
    function overviewChips(k) {
        const s = sample || {};
        if (k === "cpu") return s.cpu ? [[pct(s.cpu.total) + " used", "main"], [ghz(s.cpu.mhz), ""]].concat(s.cpu.temp ? [[temp(s.cpu.temp), s.cpu.temp > 85 ? "warn" : ""]] : []) : [];
        if (k === "memory") {
            const mb = s.memBar || {};
            const pr = s.pressure !== undefined && s.pressure !== null ? s.pressure : null;
            return [[size(mb.inUse || 0) + " in use", "main"]].concat(mb.swapUsed ? [[size(mb.swapUsed) + " in swap", ""]] : [])
                .concat(pr === null ? [] : [["pressure: " + (pr < 1 ? "none" : pr < 10 ? "light" : pr < 30 ? "medium" : "heavy"), pr < 10 ? "good" : "warn"]]);
        }
        if (k === "disk") { const d = (s.disks || []).slice().sort((a, b) => b.active - a.active)[0]; return d ? [[d.label + " " + pct(d.active) + " busy", "main"], [rate((s.disks || []).reduce((a, x) => a + x.write, 0)) + " write", ""]] : []; }
        if (k === "network") { const n = (s.nets || []).filter(x => x.up); return n.length ? [[rate(n.reduce((a, x) => a + x.recv, 0)) + " down", "main"], [rate(n.reduce((a, x) => a + x.send, 0)) + " up", ""]] : [["Not connected", ""]]; }
        if (k === "gpu") { const g = (s.gpus || [])[0]; return g ? [[(g.vendor || "GPU") + " " + (g.busy === null || g.busy === undefined ? "" : pct(g.busy)), "main"]].concat(g.temp ? [[temp(g.temp), ""]] : []) : [["No graphics card seen", ""]]; }
        return [];
    }
    property int listIndex: 0
    readonly property var selRow: tableRows[Math.min(listIndex, tableRows.length - 1)] || null

    // ================================================ actions (End, Efficiency mode, Open file location, Search online)
    property bool menuOpen: false
    property var menuRow: null
    property int menuIndex: 0
    property real menuX: 0
    property real menuY: 0
    property string toast: ""
    readonly property var menuItems: menuRow ? [
        { id: "end", label: "End task", enabled: !menuRow.protected },
        { id: "eco", label: menuRow.eco ? "Efficiency mode: on (turn off)" : "Efficiency mode", enabled: !menuRow.protected },
        { id: "where", label: "Open file location", enabled: !!menuRow.exe },
        { id: "web", label: "Search online", enabled: true }] : []
    function openMenu(row, x, y) {
        const g = row ? row : (selRow ? selRow.g : null);
        if (!g) return;
        menuRow = g; menuIndex = Math.max(0, menuItems.findIndex(i => i.enabled));
        menuX = x === undefined ? width / 2 - 120 : x; menuY = y === undefined ? height / 2 - 80 : y;
        menuOpen = true;
    }
    function closeMenu() { menuOpen = false; }
    function pickMenu(i) {
        const it = menuItems[i], g = menuRow;
        if (!it || !it.enabled || !g) return;
        closeMenu();
        switch (it.id) {
        case "end": run(["end", String(g.pid)], g.name + " was asked to close."); break;
        case "eco": run(["eco", String(g.pid), g.eco ? "off" : "on"], g.eco ? "Efficiency mode is off for " + g.name + "." : g.name + " runs in Efficiency mode now."); break;
        case "where": openLocation(g.exe); break;
        case "web": Qt.openUrlExternally("https://duckduckgo.com/?q=" + encodeURIComponent(g.name)); break;
        }
    }
    property string okText: ""
    function run(args, ok) {
        if (actProc.running) return;
        okText = ok;
        actProc.command = ["python3", v.helper].concat(args);
        actProc.running = true;
    }
    Process {
        id: actProc
        stdout: StdioCollector {
            onStreamFinished: {
                const line = text.trim().split("\n").pop() || "";
                v.toast = line.startsWith("error: ") ? line.slice(7).replace(/^./, x => x.toUpperCase()) + "." : v.okText;
                toastTimer.restart();
            }
        }
    }
    Timer { id: toastTimer; interval: 5000; onTriggered: v.toast = "" }
    function openLocation(path) {
        if (!path) return;
        const dir = path.replace(/\/[^\/]*$/, "") || "/";
        const base = path.replace(/^.*\//, "");
        Quickshell.execDetached(["sh", "-c", "quickshell ipc -p \"$3\" call files open \"$1\" && sleep 1.5 && quickshell ipc -p \"$3\" call filemanager select \"$2\"",
            "sh", dir, base, Quickshell.shellDir]);
    }

    // ================================================ keys and the controller
    function step(t) { const i = tabList.findIndex(x => x[0] === tab); if (rm) rm.tab = tabList[(i + t + tabList.length) % tabList.length][0]; }
    function moveCursor(dx, dy) {
        if (!pcView) return;
        const parts = pcView.parts;
        const cur = parts.find(p => p.id === cursorPart) || parts[0];
        if (!cur) return;
        const cx = cur.x + cur.w / 2, cy = cur.y + cur.h / 2;
        let best = null, bestScore = 1e9;
        for (const p of parts) {
            if (p.id === cur.id) continue;
            const px = p.x + p.w / 2 - cx, py = p.y + p.h / 2 - cy;
            const along = px * dx + py * dy;
            if (along <= 2) continue;
            const across = Math.abs(px * dy - py * dx);
            const score = along + across * 2.2;
            if (score < bestScore) { bestScore = score; best = p; }
        }
        if (best) cursorPart = best.id;
    }
    function start() { haveProc.running = true; loadPc(false); forceActiveFocus(); }
    function stateLine() {
        return JSON.stringify({ tab: tab, samples: samples, rows: tableRows.length, selected: selected, parts: pcView ? pcView.parts.length : 0,
                                layout: pcView ? pcView.layout : null, error: sampleError || pcError, menu: menuOpen });
    }
    Keys.onPressed: e => {
        const ctrl = e.modifiers & Qt.ControlModifier;
        if ((ctrl && e.key === Qt.Key_PageDown) || (ctrl && e.key === Qt.Key_Tab)) { step(1); e.accepted = true; return; }
        if ((ctrl && e.key === Qt.Key_PageUp) || (ctrl && e.key === Qt.Key_Backtab)) { step(-1); e.accepted = true; return; }
        if (menuOpen) {
            switch (e.key) {
            case Qt.Key_Down: case Qt.Key_Tab: { let i = menuIndex; do i = (i + 1) % menuItems.length; while (!menuItems[i].enabled && i !== menuIndex); menuIndex = i; break; }
            case Qt.Key_Up: case Qt.Key_Backtab: { let i = menuIndex; do i = (i - 1 + menuItems.length) % menuItems.length; while (!menuItems[i].enabled && i !== menuIndex); menuIndex = i; break; }
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: pickMenu(menuIndex); break;
            case Qt.Key_Escape: case Qt.Key_Menu: case Qt.Key_Left: closeMenu(); break;
            default: return;
            }
            e.accepted = true; return;
        }
        if (tab === "pc") {
            switch (e.key) {
            case Qt.Key_Left: moveCursor(-1, 0); break;
            case Qt.Key_Right: moveCursor(1, 0); break;
            case Qt.Key_Up: moveCursor(0, -1); break;
            case Qt.Key_Down: moveCursor(0, 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: selectPart(cursorPart); break;
            case Qt.Key_Menu: if (selPart) runAction(selPart.action); break;
            case Qt.Key_Escape: if (rm) rm.close(); break;
            default: return;
            }
            e.accepted = true; return;
        }
        switch (e.key) {
        case Qt.Key_Down: listIndex = Math.min(tableRows.length - 1, listIndex + 1); break;
        case Qt.Key_Up: listIndex = Math.max(0, listIndex - 1); break;
        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: break;   // A selects (the row is selected already)
        case Qt.Key_Menu: openMenu(); break;
        case Qt.Key_Delete: if (selRow) { menuRow = selRow.g; pickMenu(0); } break;
        case Qt.Key_Escape: if (rm) rm.close(); break;
        default: return;
        }
        e.accepted = true;
    }

    // ================================================ layout
    Rectangle { anchors.fill: parent; color: v.c.bg }
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // ---- the tabs
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            color: v.c.bar
            Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: v.c.line }
            Row {
                anchors { left: parent.left; leftMargin: 12; bottom: parent.bottom }
                spacing: 4
                Repeater {
                    model: v.tabList
                    delegate: Rectangle {
                        required property var modelData
                        readonly property bool on: v.tab === modelData[0]
                        width: tabText.implicitWidth + 32; height: 34
                        radius: 6
                        color: on ? v.c.bg : (tabMouse.containsMouse ? "#22344a" : "transparent")
                        border.width: on ? 1 : 0; border.color: v.c.line
                        Rectangle { visible: parent.on; anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 1; rightMargin: 1 } height: 7; color: v.c.bg }
                        Text { id: tabText; anchors.centerIn: parent; anchors.verticalCenterOffset: 1; text: modelData[1]; color: parent.on ? v.c.text : v.c.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                        MouseArea { id: tabMouse; anchors.fill: parent; hoverEnabled: true; onClicked: if (v.rm) v.rm.tab = modelData[0] }
                    }
                }
            }
            Row {
                anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                spacing: 8
                Rectangle { width: 8; height: 8; radius: 4; anchors.verticalCenter: parent.verticalCenter; color: v.sampleError ? v.c.warn : v.c.ok }
                Text { text: v.sampleError ? "Paused: " + v.sampleError : "Live · every second"; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }

        // ---- the page
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // ------------------------------------------------ Your PC
            Item {
                id: pcPage
                visible: v.tab === "pc"
                anchors.fill: parent
                anchors.margins: 12
                readonly property real stageScale: Math.max(0.4, Math.min((width - 372) / 870, (height - 30) / 664, 1.3))
                Column {
                    id: stageCol
                    width: 870 * pcPage.stageScale
                    spacing: 8
                    Row {
                        width: parent.width
                        spacing: 10
                        Text { id: pcName; text: v.pcView ? v.pcView.name : (v.pcLoading ? "Reading this PC…" : "Your PC"); color: v.c.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight; width: Math.min(implicitWidth, parent.width * 0.45) }
                        Text { text: v.pcView ? v.pcView.sub : ""; color: v.c.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight; width: parent.width - pcName.width - legend.width - 30; anchors.baseline: pcName.baseline }
                        Row {
                            id: legend
                            spacing: 10; anchors.baseline: pcName.baseline
                            Repeater {
                                model: [["●", "#7fe0a4", "busy"], ["●", "#66c0f4", "connected"], ["○", "#5d7488", "empty"]]
                                delegate: Text { required property var modelData; text: "<font color='" + modelData[1] + "'>" + modelData[0] + "</font> " + modelData[2]; textFormat: Text.StyledText; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11 }
                            }
                        }
                    }
                    Rectangle {
                        width: 870 * pcPage.stageScale; height: 664 * pcPage.stageScale
                        radius: 10; color: "#15181c"; clip: true
                        border.width: 2; border.color: "#2a2f36"
                        ResmonBoard {
                            id: board
                            transformOrigin: Item.TopLeft
                            scale: pcPage.stageScale
                            view: v.pcView
                            live: v.live
                            selected: v.selected
                            cursor: v.cursorPart
                            animate: !Settings.reduceMotion
                            onPicked: id => { v.selectPart(id); v.forceActiveFocus(); }
                        }
                        Text {
                            anchors.centerIn: parent
                            visible: !v.pcView
                            width: parent.width - 80; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
                            text: v.pcError ? v.pcError : "Reading what this PC has inside…"
                            color: v.c.muted; font.family: Theme.font; font.pixelSize: 14
                        }
                    }
                }
                // ---- the details of the part you picked
                Rectangle {
                    id: details
                    anchors { left: stageCol.right; leftMargin: 12; right: parent.right; top: parent.top; bottom: parent.bottom }
                    radius: 10; color: v.c.card; border.width: 1; border.color: v.c.line
                    clip: true
                    readonly property var p: v.selPart
                    readonly property color col: p ? board.colorOf(p) : v.c.accent
                    readonly property var st: !p ? ["", "", ""] : (v.live[p.id] && v.live[p.id].busy) ? ["Busy now", "#1f4a32", "#7fe0a4"] : p.status === "empty" ? ["Empty", "#2a3038", "#c3ccd5"] : ["Connected", "#22405a", "#9fd6f7"]
                    Flickable {
                        anchors.fill: parent; anchors.margins: 14
                        contentHeight: detCol.implicitHeight
                        boundsBehavior: Flickable.StopAtBounds
                        Column {
                            id: detCol
                            width: parent.width
                            spacing: 11
                            visible: details.p !== null
                            Row {
                                spacing: 10; width: parent.width
                                Rectangle {
                                    width: 38; height: 38; radius: 9
                                    color: Qt.rgba(details.col.r, details.col.g, details.col.b, 0.2)
                                    Canvas {
                                        anchors.centerIn: parent; width: 20; height: 20
                                        property color cc: details.col
                                        onCcChanged: requestPaint()
                                        onPaint: { const x = getContext("2d"); x.reset(); x.scale(20 / 24, 20 / 24); x.beginPath(); x.moveTo(13, 2); x.lineTo(4, 14); x.lineTo(11, 14); x.lineTo(10, 22); x.lineTo(19, 10); x.lineTo(12, 10); x.closePath(); x.fillStyle = cc; x.fill(); }
                                    }
                                }
                                Column {
                                    width: parent.width - 48; spacing: 3
                                    Text { width: parent.width; text: details.p ? details.p.title : ""; color: v.c.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
                                    Text { width: parent.width; text: details.p ? details.p.dev : ""; color: v.c.sub; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
                                }
                            }
                            Rectangle {
                                width: chip.implicitWidth + 18; height: 20; radius: 10; color: details.st[1]
                                Text { id: chip; anchors.centerIn: parent; text: details.st[0]; color: details.st[2]; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold }
                            }
                            Grid {
                                visible: v.selLive.stats.length > 0
                                columns: 2; columnSpacing: 6; rowSpacing: 6; width: parent.width
                                Repeater {
                                    model: v.selLive.stats
                                    delegate: Rectangle {
                                        required property var modelData
                                        width: (detCol.width - 6) / 2; height: 46; radius: 7; color: v.c.head
                                        Column {
                                            anchors { left: parent.left; leftMargin: 9; verticalCenter: parent.verticalCenter } spacing: 1
                                            Text { text: modelData[0]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11 }
                                            Text { text: modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                                        }
                                    }
                                }
                            }
                            Rectangle {
                                width: parent.width; height: specCol.implicitHeight + 2
                                radius: 7; color: "transparent"; border.width: 1; border.color: "#2a3f55"; clip: true
                                visible: details.p !== null && details.p.specs.length > 0
                                Column {
                                    id: specCol
                                    x: 1; y: 1; width: parent.width - 2
                                    Repeater {
                                        model: details.p ? details.p.specs : []
                                        delegate: Rectangle {
                                            required property var modelData
                                            required property int index
                                            width: specCol.width; height: Math.max(24, specV.implicitHeight + 10)
                                            color: index % 2 ? v.c.card : v.c.head
                                            Text { x: 9; y: 5; width: 104; text: modelData[0]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
                                            Text { id: specV; x: 121; y: 5; width: parent.width - 130; text: modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.Wrap }
                                        }
                                    }
                                }
                            }
                            Column {
                                width: parent.width; spacing: 3
                                Text { text: "TALKS TO"; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.6 }
                                Text {
                                    width: parent.width; wrapMode: Text.WordWrap; color: v.c.sub; font.family: Theme.font; font.pixelSize: 12; lineHeight: 1.2
                                    text: !details.p ? "" : (details.p.hub ? "Wired to " + ((v.pcView.hubNames || {})[details.p.hub] || ((v.pcView.parts.find(x => x.id === details.p.hub) || {}).title || "").toLowerCase() || details.p.hub)
                                          + (details.p.lanes ? " · " + details.p.lanes + (details.p.lanes === 1 ? " line" : " lines") : "") + ". " : "") + details.p.link
                                }
                            }
                            Column {
                                width: parent.width; spacing: 2
                                visible: v.selLive.users.length > 0
                                Text { text: "RIGHT NOW"; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.6 }
                                Repeater {
                                    model: v.selLive.users
                                    delegate: Row {
                                        required property var modelData
                                        width: detCol.width; height: 20
                                        Text { width: parent.width - 80; text: modelData.n; color: v.c.text; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                                        Text { width: 80; horizontalAlignment: Text.AlignRight; text: modelData.v; color: v.c.sub; font.family: Theme.font; font.pixelSize: 12 }
                                    }
                                }
                            }
                            Column {   // tips that are true for this PC (the one about this part first)
                                width: parent.width; spacing: 6
                                visible: v.pcView && v.pcView.tips.length > 0
                                Text { text: "TIPS FOR THIS PC"; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.6 }
                                Repeater {
                                    model: v.pcView ? v.pcView.tips : []
                                    delegate: Rectangle {
                                        required property var modelData
                                        width: detCol.width; height: tipText.implicitHeight + 14; radius: 7
                                        color: "#2a3a2e"; border.width: 1; border.color: "#3d5a43"
                                        Text { id: tipText; x: 9; y: 7; width: parent.width - 18; text: modelData.text; color: "#d8f0dd"; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
                                    }
                                }
                            }
                            Rectangle {
                                id: actBtn
                                readonly property var a: details.p ? details.p.action : null
                                visible: v.actionOk(a)
                                width: parent.width; height: 36; radius: 6
                                color: actMouse.containsMouse ? "#86d0f8" : v.c.accent
                                Text { anchors.centerIn: parent; text: actBtn.a ? actBtn.a.label : ""; color: "#06121c"; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                                MouseArea { id: actMouse; anchors.fill: parent; hoverEnabled: true; onClicked: v.runAction(actBtn.a) }
                            }
                            Text {
                                visible: !!v.pc && v.pc.memory && v.pc.memory.source === "none" && v.pc.family === "desktop"
                                width: parent.width; wrapMode: Text.WordWrap; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11
                                text: "Each memory stick's maker, speed and slot come from the board, which Hearth reads once with the system's permission. <a href='again'>Read them now</a>"
                                textFormat: Text.StyledText; linkColor: v.c.accent
                                onLinkActivated: v.loadPc(true)
                            }
                        }
                    }
                }
            }

            // ------------------------------------------------ the other tabs: tables on the left, graphs on the right
            RowLayout {
                visible: v.tab !== "pc"
                anchors.fill: parent
                anchors.margins: 14
                spacing: 14
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 10

                    // Overview (1.11, the approved mockup resmon-1): Processor, Memory, Disk, Network and Graphics, each with its
                    // own chips and its top programs; click a heading to fold it; tick programs to show only them everywhere.
                    Flickable {
                        visible: v.tab === "overview"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        contentHeight: ovCol.implicitHeight
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ColumnLayout {
                            id: ovCol
                            width: parent.width
                            spacing: 10
                            Repeater {
                                model: v.tab === "overview" ? [["cpu", "Processor"], ["memory", "Memory"], ["disk", "Disk"], ["network", "Network"], ["gpu", "Graphics"]] : []
                                delegate: Rectangle {
                                    id: sec
                                    required property var modelData
                                    readonly property string k: modelData[0]
                                    readonly property bool open: !v.folded[k]
                                    Layout.fillWidth: true
                                    implicitHeight: 38 + (open ? 26 + Math.max(1, secRows.length) * 28 : 0)
                                    radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line; clip: true
                                    readonly property var secRows: { v.samples; v.ticked; const r = v.rowsFor(k); const t = Object.keys(v.ticked); return (t.length ? r.filter(x => v.ticked[x.g.id]) : r).slice(0, 5); }
                                    readonly property var chips: { v.samples; return v.overviewChips(k); }
                                    Column {
                                        anchors { left: parent.left; right: parent.right; top: parent.top }
                                        Rectangle {
                                            width: parent.width; height: 38; color: v.c.head
                                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const f = Object.assign({}, v.folded); f[sec.k] = !f[sec.k]; v.folded = f; } }
                                            Text { anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter } text: (sec.open ? "▾ " : "▸ ") + sec.modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                                            Row {
                                                anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                                                spacing: 6
                                                Repeater {
                                                    model: sec.chips
                                                    delegate: Rectangle {
                                                        required property var modelData
                                                        height: 24; width: chT.implicitWidth + 18; radius: 12; color: "transparent"
                                                        border.color: modelData[1] === "good" ? v.c.ok : modelData[1] === "warn" ? v.c.warn : modelData[1] === "main" ? v.c.accent : v.c.line
                                                        Text { id: chT; anchors.centerIn: parent; text: parent.modelData[0]; color: parent.modelData[1] === "good" ? v.c.ok : parent.modelData[1] === "warn" ? v.c.warn : v.c.text; font.family: Theme.font; font.pixelSize: 12 }
                                                    }
                                                }
                                            }
                                        }
                                        ProgHeader { visible: sec.open; width: parent.width; cols: v.overviewCols[sec.k] || v.colsFor[sec.k] }
                                        Repeater {
                                            model: sec.open ? sec.secRows : []
                                            delegate: ProgRow {
                                                required property var modelData; required property int index
                                                width: parent.width; row: modelData; alt: index % 2 === 0; picked: false
                                                tickable: true; ticked: !!v.ticked[modelData.g.id]
                                                onTick: { const t = Object.assign({}, v.ticked); if (t[modelData.g.id]) delete t[modelData.g.id]; else t[modelData.g.id] = true; v.ticked = t; }
                                                onMenuRow: (x, y) => { const pt = mapToItem(v, x, y); v.openMenu(modelData.g, pt.x, pt.y); }
                                            }
                                        }
                                        Text { visible: sec.open && !sec.secRows.length; leftPadding: 12; topPadding: 6; text: "Nothing here right now."; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12 }
                                    }
                                }
                            }
                            Text {
                                Layout.fillWidth: true; Layout.leftMargin: 4; wrapMode: Text.WordWrap
                                text: Object.keys(v.ticked).length ? "Showing only the programs you ticked. Untick them to see everything." : "Tick programs to show only them in every section. Click a heading to fold it."
                                color: v.c.muted; font.family: Theme.font; font.pixelSize: 12
                            }
                        }
                    }

                    // Processor, Disk, Network, Graphics: a summary on top
                    Rectangle {
                        id: summaryBox
                        visible: v.tab === "cpu" || v.tab === "disk" || v.tab === "network"
                        Layout.fillWidth: true
                        Layout.preferredHeight: 92
                        radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line
                        readonly property var cards: { v.samples; const s = v.sample || {};
                            if (v.tab === "cpu" && s.cpu) return [["Busy", pct(s.cpu.total), (s.cpu.name || "").replace(/\(R\)|\(TM\)/g, "")], ["Speed", ghz(s.cpu.mhz), s.cpu.base ? "base " + ghz(s.cpu.base) : ""],
                                ["Temperature", temp(s.cpu.temp) || "–", tempNote(s.cpu.temp)], ["Threads", thousands(s.cpu.threads), thousands(s.cpu.processes) + " processes"], ["Cores", String(s.cpu.coreCount || "–"), s.cpu.logical + " logical"]];
                            if (v.tab === "disk") return (s.disks || []).slice(0, 5).map(d => [d.label, pct(d.active) + " active", rate(d.read) + " read · " + rate(d.write) + " write"]);
                            if (v.tab === "network") return (s.nets || []).slice(0, 5).map(n => [n.kind + " (" + n.name + ")", n.up ? rate(n.recv) + " ↓" : "Not connected", n.up ? rate(n.send) + " ↑" + (n.speed ? " · " + n.speed + " Mbps link" : "") : ""]);
                            return []; }
                        Row {
                            anchors { fill: parent; margins: 12 }
                            spacing: 10
                            Repeater {
                                model: summaryBox.cards
                                delegate: Rectangle {
                                    required property var modelData
                                    width: (parent.width - 40) / 5; height: parent.height; radius: 8; color: v.c.head
                                    Column {
                                        anchors { left: parent.left; right: parent.right; leftMargin: 10; rightMargin: 8; verticalCenter: parent.verticalCenter } spacing: 3
                                        Text { width: parent.width; text: modelData[0]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                                        Text { width: parent.width; text: modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                        Text { width: parent.width; text: modelData[2]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
                                    }
                                }
                            }
                        }
                    }

                    // Memory: the bar in plain words
                    Rectangle {
                        visible: v.tab === "memory"
                        Layout.fillWidth: true
                        Layout.preferredHeight: 230
                        radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line
                        readonly property var mb: { v.samples; return (v.sample && v.sample.memBar) || null; }
                        readonly property var zr: { v.samples; return (v.sample && v.sample.zram) || null; }
                        readonly property var drive: { v.samples; return ((v.sample && v.sample.swaps) || []).filter(x => x.path.indexOf("/dev/zram") !== 0); }
                        // 1.11 (the approved mockup resmon-3): compressed memory and the swaps in numbers too
                        readonly property var facts: !mb ? [] : [
                            ["Zram holds", zr && zr.size ? v.size(zr.orig) + " of " + v.size(zr.size) + (zr.compr ? " (" + (zr.orig / zr.compr).toFixed(1) + ":1)" : "") : "off"],
                            ["Drive swap", drive.length ? v.size(drive.reduce((a, x) => a + x.used, 0)) + " of " + v.size(drive.reduce((a, x) => a + x.size, 0)) : "none"],
                            ["Compressed (real size)", zr ? v.size(zr.mem) : "–"],
                            ["Committed", v.size(mb.committed || 0)]]
                        readonly property var parts: !mb ? [] : [
                            ["Reserved", mb.reserved, "Kept by the hardware and graphics", "#5d7488"], ["In use", mb.inUse, "Programs and games", "#66c0f4"],
                            ["Compressed", zr ? zr.mem : 0, "Idle memory squeezed into zram", "#3fb8c9"],
                            ["Being saved", mb.saving, "Waiting to be written to the drive", "#ff9a4d"], ["Cache", mb.cache, "Recent files, handed back the moment a game needs it", "#9a7cff"],
                            ["Free", mb.free, "Not used at all", "#2f4a63"]]
                        Column {
                            anchors { fill: parent; margins: 14 }
                            spacing: 12
                            Row {
                                spacing: 10
                                Text { text: "Memory"; color: v.c.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                                Text { text: memBox.mb ? v.size(memBox.mb.installed) + " installed · " + v.size(memBox.mb.inUse) + " in use by programs" : ""; color: v.c.muted; font.family: Theme.font; font.pixelSize: 13 }
                            }
                            Rectangle {
                                width: parent.width; height: 28; radius: 6; clip: true; color: v.c.line
                                Row {
                                    anchors.fill: parent
                                    Repeater {
                                        model: memBox.parts
                                        delegate: Rectangle { required property var modelData; height: 28; color: modelData[3]; width: memBox.mb && memBox.mb.installed ? parent.width * (modelData[1] || 0) / memBox.mb.installed : 0 }
                                    }
                                }
                            }
                            Row {
                                width: parent.width
                                Repeater {
                                    model: memBox.parts
                                    delegate: Column {
                                        required property var modelData
                                        width: parent.width / 6; spacing: 3
                                        Row { spacing: 6; Rectangle { width: 10; height: 10; radius: 2; color: modelData[3]; anchors.verticalCenter: parent.verticalCenter } Text { text: modelData[0]; color: v.c.sub; font.family: Theme.font; font.pixelSize: 12 } }
                                        Text { text: v.size(modelData[1]); color: v.c.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                                        Text { width: parent.width - 10; text: modelData[2]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap }
                                    }
                                }
                            }
                            Grid {
                                width: parent.width; columns: 4; columnSpacing: 18; rowSpacing: 4
                                Repeater {
                                    model: memBox.facts
                                    delegate: Row {
                                        required property var modelData
                                        spacing: 8
                                        Text { text: modelData[0]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12 }
                                        Text { text: modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                                    }
                                }
                            }
                        }
                        id: memBox
                    }

                    // Graphics: each card
                    Repeater {
                        model: v.tab === "gpu" ? ((v.sample && v.sample.gpus) || []).slice(0, 2) : []
                        delegate: Rectangle {
                            required property var modelData
                            readonly property var g: modelData
                            Layout.fillWidth: true
                            Layout.preferredHeight: 158
                            radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line
                            readonly property var pg: ((v.pc && v.pc.gpus) || []).find(x => (g.pci && x.pci.endsWith(g.pci.slice(-7))) || (g.card && x.card === g.card)) || null
                            Column {
                                anchors { fill: parent; margins: 14 }
                                spacing: 12
                                Row {
                                    spacing: 10; width: parent.width
                                    Text { id: gname; text: g.name; color: v.c.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                                    Text {
                                        width: parent.width - gname.width - 10; elide: Text.ElideRight; color: v.c.muted; font.family: Theme.font; font.pixelSize: 13
                                        text: [g.driver ? "driver " + g.driver : (pg ? pg.driver : ""), pg && pg.link ? pg.link : (g.linkGen ? "PCIe " + g.linkGen + ".0 x" + g.linkWidth : ""),
                                               pg ? pg.outputs.filter(o => o.connected).map(o => o.port + (o.monitor ? " to " + o.monitor : "")).join(", ") : ""].filter(x => x).join(" · ")
                                    }
                                }
                                Row {
                                    width: parent.width; spacing: 10
                                    Repeater {
                                        model: [["Busy", pct(g.busy), ""], ["Temperature", temp(g.temp) || "–", tempNote(g.temp)], ["Power", g.watts ? Math.round(g.watts) + " W" : "–", g.wattsMax ? "of " + Math.round(g.wattsMax) + " W" : ""],
                                                ["Clock", g.mhz ? ghz(g.mhz) : "–", g.memMhz ? "Memory " + Math.round(g.memMhz) + " MHz" : ""], ["Fan", g.fan !== null && g.fan !== undefined ? Math.round(g.fan) + "%" : "–", g.fanRpm ? thousands(g.fanRpm) + " rpm" : ""]]
                                        delegate: Rectangle {
                                            required property var modelData
                                            width: (parent.width - 40) / 5; height: 70; radius: 8; color: v.c.head
                                            Column {
                                                anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } spacing: 3
                                                Text { text: modelData[0]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12 }
                                                Text { text: modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                                                Text { text: modelData[2]; color: modelData[2] === "Normal" ? v.c.ok : v.c.muted; font.family: Theme.font; font.pixelSize: 11 }
                                            }
                                        }
                                    }
                                }
                                Column {
                                    width: parent.width; spacing: 5
                                    visible: !!g.vramTotal
                                    Row { width: parent.width; Text { width: parent.width / 2; text: "Video memory"; color: v.c.sub; font.family: Theme.font; font.pixelSize: 12 } Text { width: parent.width / 2; horizontalAlignment: Text.AlignRight; text: v.size(g.vramUsed) + " of " + v.size(g.vramTotal); color: v.c.sub; font.family: Theme.font; font.pixelSize: 12 } }
                                    Rectangle { width: parent.width; height: 10; radius: 5; color: v.c.line; Rectangle { width: g.vramTotal ? parent.width * Math.min(1, (g.vramUsed || 0) / g.vramTotal) : 0; height: 10; radius: 5; color: v.c.accent } }
                                }
                            }
                        }
                    }
                    Rectangle {
                        visible: v.tab === "gpu" && !((v.sample && v.sample.gpus) || []).length
                        Layout.fillWidth: true; Layout.preferredHeight: 60; radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line
                        Text { anchors.centerIn: parent; text: v.samples ? "No graphics card reports its numbers here." : "Reading…"; color: v.c.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }

                    // The table for the tab
                    Rectangle {
                        visible: v.tab !== "overview"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line; clip: true
                        ProgHeader { id: tHead; width: parent.width; cols: v.colsFor[v.tab] || []; head: true }
                        ListView {
                            id: table
                            anchors { left: parent.left; right: parent.right; top: tHead.bottom; bottom: parent.bottom }
                            clip: true
                            model: v.tab === "overview" ? [] : v.tableRows
                            currentIndex: v.listIndex
                            highlightFollowsCurrentItem: false
                            boundsBehavior: Flickable.StopAtBounds
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            delegate: ProgRow {
                                required property var modelData
                                required property int index
                                width: table.width; row: modelData; alt: index % 2 === 0; picked: index === v.listIndex
                                onClickedRow: v.listIndex = index
                                onMenuRow: (x, y) => { v.listIndex = index; const pt = mapToItem(v, x, y); v.openMenu(modelData.g, pt.x, pt.y); }
                            }
                        }
                    }
                }

                // ---- the four live graphs
                ColumnLayout {
                    Layout.preferredWidth: 300
                    Layout.fillHeight: true
                    spacing: 10
                    Repeater {
                        model: { v.samples; const s = v.sample || {}; const mb = s.memBar || {};
                            const g4 = v.tab === "gpu" ? ["Graphics", v.pct((v.hist.gpu || []).slice(-1)[0] || 0), "#ff9a4d", v.hist.gpu, 100, ((s.gpus || [])[0] || {}).vramTotal ? v.size(((s.gpus || [])[0]).vramUsed) + " of " + v.size(((s.gpus || [])[0]).vramTotal) + " video memory" : ""]
                                     : ["Memory", v.size(mb.inUse), "#9a7cff", v.hist.mem, mb.total || 1, v.size(mb.cache) + " more is cache"];
                            const busiest = (s.disks || []).slice().sort((a, b) => b.active - a.active)[0];
                            return [["Processor", v.pct(s.cpu ? s.cpu.total : 0), "#66c0f4", v.hist.cpu, 100, s.cpu ? s.cpu.logical + " threads · " + v.ghz(s.cpu.mhz) : ""],
                                    ["Disk", v.rate((v.hist.disk || []).slice(-1)[0] || 0), "#7ccf8a", v.hist.disk, 0, busiest ? "Highest: " + busiest.label : ""],
                                    ["Network", v.rate((v.hist.net || []).slice(-1)[0] || 0), "#f2c14e", v.hist.net, 0, ((s.nets || []).find(n => n.up) || {}).kind || "Not connected"], g4]; }
                        delegate: Rectangle {
                            required property var modelData
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            radius: 8; color: v.c.card; border.width: 1; border.color: v.c.line
                            Column {
                                anchors { fill: parent; margins: 10; leftMargin: 12; rightMargin: 12 }
                                spacing: 6
                                Row {
                                    width: parent.width
                                    Text { width: parent.width - gv.implicitWidth; text: modelData[0]; color: v.c.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                                    Text { id: gv; text: modelData[1]; color: modelData[2]; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                                }
                                Canvas {
                                    id: graph
                                    width: parent.width; height: parent.height - 46
                                    property var pts: modelData[3] || []
                                    property real topVal: modelData[4]
                                    property color col: modelData[2]
                                    onPtsChanged: requestPaint()
                                    onPaint: {
                                        const ctx = getContext("2d"); ctx.reset();
                                        ctx.fillStyle = "#10151b"; ctx.fillRect(0, 0, width, height);
                                        ctx.strokeStyle = "#22344a"; ctx.lineWidth = 1; ctx.beginPath();
                                        for (const f of [0.25, 0.5, 0.75]) { ctx.moveTo(0, Math.round(height * f) + 0.5); ctx.lineTo(width, Math.round(height * f) + 0.5); }
                                        ctx.stroke();
                                        const a = pts; if (!a.length) return;
                                        const mx = topVal > 0 ? topVal : Math.max(1048576, ...a) * 1.15;
                                        const xs = i => width - (a.length - 1 - i) * (width / (v.histMax - 1));
                                        const ys = val => height - Math.min(1, (val || 0) / mx) * (height - 4) - 1;
                                        ctx.beginPath(); ctx.moveTo(xs(0), height);
                                        for (let i = 0; i < a.length; i++) ctx.lineTo(xs(i), ys(a[i]));
                                        ctx.lineTo(xs(a.length - 1), height); ctx.closePath();
                                        ctx.fillStyle = Qt.rgba(col.r, col.g, col.b, 0.18); ctx.fill();
                                        ctx.beginPath();
                                        for (let i = 0; i < a.length; i++) { if (i) ctx.lineTo(xs(i), ys(a[i])); else ctx.moveTo(xs(i), ys(a[i])); }
                                        ctx.strokeStyle = col; ctx.lineWidth = 2; ctx.stroke();
                                    }
                                }
                                Text { width: parent.width; text: modelData[5]; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
                            }
                        }
                    }
                }
            }
        }

        // ---- the hint bar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            color: v.c.bar
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: v.c.line }
            Text {
                anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                text: v.toast !== "" ? v.toast : v.tab === "pc" ? "Click any part for its details · Hearth reads these from this PC" : "Right-click a program: End, Efficiency mode, Open file location, Search online"
                color: v.toast !== "" ? v.c.text : v.c.muted; font.family: Theme.font; font.pixelSize: 12
            }
            Text {
                anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                text: v.tab === "pc" ? "LB / RB tabs · D-pad parts · A select · X action" : "LB / RB tabs · A select · X actions"
                color: v.c.muted; font.family: Theme.font; font.pixelSize: 12
            }
        }
    }

    // ---- the actions menu
    MouseArea { anchors.fill: parent; visible: v.menuOpen; onClicked: v.closeMenu(); acceptedButtons: Qt.LeftButton | Qt.RightButton }
    Rectangle {
        visible: v.menuOpen
        x: Math.min(v.menuX, v.width - width - 8); y: Math.min(v.menuY, v.height - height - 8)
        width: 260; height: menuCol.implicitHeight + 12
        radius: 8; color: "#22344a"; border.width: 1; border.color: v.c.line
        Column {
            id: menuCol
            x: 6; y: 6; width: parent.width - 12
            Text { width: parent.width; leftPadding: 8; bottomPadding: 6; topPadding: 2; text: v.menuRow ? v.menuRow.name : ""; color: v.c.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
            Repeater {
                model: v.menuItems
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: menuCol.width; height: 32; radius: 5
                    color: index === v.menuIndex && modelData.enabled ? v.c.sel : (mi.containsMouse && modelData.enabled ? "#2a3f55" : "transparent")
                    Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: modelData.label; color: modelData.enabled ? v.c.text : "#5d7488"; font.family: Theme.font; font.pixelSize: 13 }
                    MouseArea { id: mi; anchors.fill: parent; hoverEnabled: true; onClicked: v.pickMenu(index) }
                }
            }
        }
    }

    // ================================================ pieces
    component ProgHeader: Rectangle {
        readonly property var vv: ({ c: { head: "#213347", line: "#2f4a63", muted: "#9fb2c3", text: "#e6eef5", sub: "#c9d6e2", sel: "#2a475e", alt: "#1f3042" },
                                     tagColor: g => ({ game: "#f2c14e", steam: "#66c0f4", hearth: "#ff9a4d", app: "#9a7cff", sys: "#5d7488", web: "#7ccf8a", home: "#3fb8c9" })[(g || {}).color] || "#9a7cff" })
        property var cols: []
        property bool head: false
        height: 26
        color: head ? vv.c.head : "transparent"
        Rectangle { visible: !parent.head; anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: vv.c.line }
        Row {
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            Text { width: parent.width * 0.42; text: "Program"; color: vv.c.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
            Text { width: parent.width * 0.12; text: "PID"; color: vv.c.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: parent.parent.cols
                delegate: Text { required property var modelData; width: parent.width * 0.4 / 3 + (parent.width * 0.06) / 3; horizontalAlignment: Text.AlignRight; text: modelData; color: vv.c.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight }
            }
        }
    }
    component ProgRow: Rectangle {
        id: pr
        readonly property var vv: ({ c: { head: "#213347", line: "#2f4a63", muted: "#9fb2c3", text: "#e6eef5", sub: "#c9d6e2", sel: "#2a475e", alt: "#1f3042" },
                                     tagColor: g => ({ game: "#f2c14e", steam: "#66c0f4", hearth: "#ff9a4d", app: "#9a7cff", sys: "#5d7488", web: "#7ccf8a", home: "#3fb8c9" })[(g || {}).color] || "#9a7cff" })
        property var row: null
        property bool alt: false
        property bool picked: false
        property bool tickable: false
        property bool ticked: false
        signal clickedRow()
        signal menuRow(real x, real y)
        signal tick()
        readonly property var g: row ? row.g : ({})
        height: 28
        color: picked ? vv.c.sel : (alt ? vv.c.alt : "transparent")
        Row {
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            Item {
                width: parent.width * 0.42; height: parent.height
                Row {
                    anchors.verticalCenter: parent.verticalCenter; spacing: 8; width: parent.width - 6
                    Rectangle {
                        width: 16; height: 16; radius: 4; anchors.verticalCenter: parent.verticalCenter
                        color: pr.tickable ? (pr.ticked ? vv.tagColor(pr.g) : "transparent") : vv.tagColor(pr.g)
                        border.width: pr.tickable ? 1 : 0; border.color: vv.c.muted
                        Text { visible: pr.tickable && pr.ticked; anchors.centerIn: parent; text: "✓"; color: "#10151b"; font.pixelSize: 11; font.bold: true }
                        MouseArea { enabled: pr.tickable; anchors.fill: parent; anchors.margins: -4; cursorShape: Qt.PointingHandCursor; onClicked: pr.tick() }
                    }
                    Text { id: nm; text: (pr.g.name || "") + (pr.g.count > 1 ? " (" + pr.g.count + ")" : ""); color: vv.c.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight; width: Math.min(implicitWidth, parent.width - 24 - (tagPill.visible ? tagPill.width + 8 : 0)); anchors.verticalCenter: parent.verticalCenter }
                    Rectangle {
                        id: tagPill
                        visible: !!pr.g.tag
                        width: tagT.implicitWidth + 12; height: 16; radius: 8; color: "#22405a"; anchors.verticalCenter: parent.verticalCenter
                        Text { id: tagT; anchors.centerIn: parent; text: pr.g.tag || ""; color: "#9fd6f7"; font.family: Theme.font; font.pixelSize: 10; font.weight: Font.DemiBold }
                    }
                    Text { visible: !!pr.g.eco; text: "🍃"; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
                }
            }
            Text { width: parent.width * 0.12; text: String(pr.g.pid || ""); color: vv.c.muted; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
            Text { width: parent.width * 0.4 / 3 + (parent.width * 0.06) / 3; horizontalAlignment: Text.AlignRight; text: pr.row ? pr.row.a : ""; color: vv.c.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideLeft }
            Text { width: parent.width * 0.4 / 3 + (parent.width * 0.06) / 3; horizontalAlignment: Text.AlignRight; text: pr.row ? pr.row.b : ""; color: vv.c.sub; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideLeft }
            Text { width: parent.width * 0.4 / 3 + (parent.width * 0.06) / 3; horizontalAlignment: Text.AlignRight; text: pr.row ? pr.row.c : ""; color: vv.c.sub; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideLeft }
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: m => { pr.clickedRow(); if (m.button === Qt.RightButton) pr.menuRow(m.x, m.y); }
            onPressAndHold: m => pr.menuRow(m.x, m.y)
        }
    }
}
