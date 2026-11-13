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
    property var hist: ({ cpu: [], disk: [], net: [], mem: [], gpu: [], g3d: [], gdec: [], vram: [], gtemp: [], resp: [], ping: [] })
    function push(h, k, val) { const a = (h[k] || []).slice(-(histMax - 1)); a.push(val || 0); h[k] = a; }
    property bool paused: false            // Pause: the numbers stand still until Resume (nothing else stops)
    // Tailscale's other devices (Network tab's Tailscale graph)
    property int tsPeers: -1
    Process {
        id: tsProc
        command: ["sh", "-c", "tailscale status --json 2>/dev/null || true"]
        stdout: StdioCollector { onStreamFinished: { try { v.tsPeers = Object.keys(JSON.parse(text).Peer || {}).length; } catch (e) { v.tsPeers = -1; } } }
    }
    Timer { interval: 30000; repeat: true; triggeredOnStart: true; running: v.tab === "network" && v.visible; onTriggered: if (!tsProc.running) tsProc.running = true }
    function applySample(line) {
        if (paused) return;
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
        // 1.11: the Graphics tab's 3D, video decode, video memory and temperature, and the disks' response time
        const gp = d.procs || [];
        push(h, "g3d", Math.min(100, gp.reduce((a, g) => a + (g.g3d || 0), 0)) || (d.gpus || []).reduce((a, g) => Math.max(a, g.busy || 0), 0));
        push(h, "gdec", Math.min(100, gp.reduce((a, g) => a + (g.gdec || 0), 0)));
        push(h, "vram", ((d.gpus || [])[0] || {}).vramUsed || 0);
        push(h, "gtemp", ((d.gpus || [])[0] || {}).temp || 0);
        push(h, "resp", (d.disks || []).reduce((a, x) => Math.max(a, x.response || 0), 0));
        // 1.12 (mockups resmon-2, -3, -4, -5): each core, the processor's temperature, compressed memory, memory pressure,
        // swapping in / out, each drive's busy time and each adapter's traffic
        ((d.cpu && d.cpu.cores) || []).forEach((x, i) => push(h, "core:" + i, x));
        push(h, "ctemp", d.cpu && d.cpu.temp ? d.cpu.temp : 0);
        push(h, "memz", d.zram ? d.zram.mem : 0);
        push(h, "mempsi", d.pressure || 0);
        push(h, "swapio", d.swapIO ? d.swapIO.in + d.swapIO.out : 0);
        (d.disks || []).forEach(x => push(h, "disk:" + x.name, x.active || 0));
        (d.nets || []).forEach(x => push(h, "net:" + x.name, (x.recv || 0) + (x.send || 0)));
        hist = h;
        sample = d;
        if (d.procs) groups = d.procs;
        samples++;
        updateLive();
    }
    function wants() {
        switch (tab) {
        case "pc": return ["procs", "gpu", "temps", "disk"];
        case "overview": return ["procs", "disk", "net", "temps", "mem"];
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
        // the swapped memory on the drive: Linux doesn't say per program, so each gets the PC's zram / drive split
        const sw = (sample && sample.swaps) || [];
        const drv = sw.filter(x => x.path.indexOf("/dev/zram") !== 0).reduce((a, x) => a + x.used, 0), all = sw.reduce((a, x) => a + x.used, 0);
        const driveShare = all ? drv / all : 0;
        switch (t) {
        case "cpu": return by("cpu").map(g => ({ g: g, cells: [pct(g.cpu), thousands(g.threads), priorityName(g.nice), g.cores || "All"], ov: [pct(g.cpu), pct(g.avg)] }));
        case "memory": return by("mem").map(g => ({ g: g, cells: [size(g.mem), g.private !== undefined ? size(g.private) : "–", g.shared !== undefined ? size(g.shared) : "–",
                                                               size(Math.round((g.swap || 0) * (1 - driveShare))), driveShare ? size(Math.round((g.swap || 0) * driveShare)) : "0", String(g.waits || 0)],
                                                       ov: [size(g.mem), String(g.waits || 0)] }));
        case "disk": {
            const rows = gs.sort((a, b) => ((b.read || 0) + (b.write || 0)) - ((a.read || 0) + (a.write || 0)) || (b.disk || 0) - (a.disk || 0))
                .map(g => ({ g: g, cells: [rate(g.read !== undefined ? g.read : g.disk), rate(g.write || 0), rate((g.read || 0) + (g.write || 0) || g.disk), priorityName(g.nice)],
                             ov: [rate(g.read !== undefined ? g.read : g.disk), rate(g.write || 0), rate((g.read || 0) + (g.write || 0) || g.disk)] }));
            // the file system's own writing (btrfs: committing, compressing), what the drives wrote beyond the programs
            const dw = ((sample && sample.disks) || []).reduce((a, x) => a + (x.write || 0), 0), pw = gs.reduce((a, g) => a + (g.write || 0), 0);
            if (dw - pw > 65536) rows.splice(Math.min(rows.length, 3), 0, { g: { id: "sys:fs", name: "btrfs (system)", pid: "", color: "sys" }, cells: ["0", rate(dw - pw), rate(dw - pw), "Normal"], ov: ["0", rate(dw - pw), rate(dw - pw)] });
            return rows;
        }
        case "network": {
            // 1.11 (mockup resmon-5): each program's download and upload speed (TCP, from ss), then its connections
            const nr = g => { const r = { down: 0, up: 0 }; for (const p of (g.pids || [])) { const x = (netPorts.apps || {})[String(p)]; if (x) { r.down += x.down || 0; r.up += x.up || 0; } } return r; };
            return gs.map(g => ({ g: g, r: nr(g) })).sort((a, b) => (b.r.down + b.r.up) - (a.r.down + a.r.up) || (b.g.conns || 0) - (a.g.conns || 0))
                .map(x => ({ g: x.g, cells: [rate(x.r.down), rate(x.r.up), (x.r.down + x.r.up) ? rate(x.r.down + x.r.up) : (x.g.conns ? x.g.conns + " conn." : "0")] }))
                .map(x => Object.assign(x, { ov: x.cells }));
        }
        case "gpu": return gs.filter(g => (g.gpu || 0) > 0 || (g.vram || 0) > 0 || (g.gdec || 0) > 0 || (g.genc || 0) > 0)
            .sort((a, b) => ((b.g3d !== undefined && b.g3d !== null ? b.g3d : b.gpu) || 0) - ((a.g3d !== undefined && a.g3d !== null ? a.g3d : a.gpu) || 0) || (b.vram || 0) - (a.vram || 0))
            .map(g => ({ g: g, cells: [pct(g.g3d !== undefined && g.g3d !== null ? g.g3d : g.gpu), g.gdec !== undefined && g.gdec !== null ? pct(g.gdec) : "0", g.genc ? pct(g.genc) : "0", g.vram ? size(g.vram) : "–", g.api || g.gpuUse || ""],
                         ov: [pct(g.g3d !== undefined && g.g3d !== null ? g.g3d : g.gpu), g.gdec !== undefined && g.gdec !== null ? pct(g.gdec) : "–", g.vram ? size(g.vram) : "–"] }));
        }
        return [];
    }
    // ---- each tab's sections, as in the mockups resmon-2 … resmon-6: {key, title, chips, kind, cols, rows, facts, …}
    readonly property var focusGroup: { const t = groups.filter(g => ticked[g.id]); return t.length ? t[0] : (selRow ? selRow.g : null); }
    function upTime(sec) { const d = Math.floor(sec / 86400), h = Math.floor(sec % 86400 / 3600), m = Math.floor(sec % 3600 / 60); return d ? d + " day" + (d > 1 ? "s" : "") + " " + h + " h" : h ? h + " h " + m + " min" : m + " min"; }
    function pressWord(p) { return p === null || p === undefined ? "–" : p < 1 ? "none" : p < 10 ? "light" : p < 30 ? "medium" : "heavy"; }
    function sectionsFor(t) {
        const s = sample || {}, out = [];
        const tickedNames = groups.filter(g => ticked[g.id]);
        const progs = { key: t + ":programs", kind: "programs", cols: colsFor[t] || [], rows: tableRows, chips: [] };
        if (t === "cpu") {
            progs.title = "Programs";
            progs.chips = s.cpu ? [[pct(s.cpu.total) + " used", "main"], [thousands(s.cpu.threads) + " threads", ""], ["Up " + upTime(s.uptime || 0), ""]] : [];
            out.push(progs);
            const f = focusGroup;
            if (f) {
                out.push({ key: "cpu:banner", banner: "Filtered to <b><font color='#9a7cff'>" + f.name + " (" + f.pid + ")</font></b> · showing its services, open files and libraries" });
                out.push({ key: "cpu:services", title: "Services", kind: "table", chips: [[services.filter(x => x.status === "Running").length + " running", ""]],
                           cols: [["Service", 3], ["PID", 1, true], ["Status", 1.4], ["CPU", 1, true], ["What it is", 4]],
                           rows: services.map(x => [x.name, String(x.pid || "—"), { t: x.status, c: x.status === "Running" ? v.c.ok : "" }, pct((groups.find(g => (g.pids || []).indexOf(x.pid) >= 0) || {}).cpu || 0), x.what]),
                           empty: "It doesn't run as a service." });
                out.push({ key: "cpu:files", title: "Open files", kind: "table", chips: [[openFiles.length + " open", ""]],
                           cols: [["Program", 3], ["PID", 1, true], ["File", 8]], rows: openFiles.slice(0, 40).map(x => [f.name, String(x.pid), x.win || x.path]), empty: "No files open." });
                out.push({ key: "cpu:libs", title: "Libraries (modules)", kind: "table", chips: [[libs.length + " loaded", ""]], cols: [["Library", 3], ["Where", 8]],
                           rows: libs.map(x => [x.name, x.path]), empty: "None it shares with others." });
            }
        } else if (t === "memory") {
            const mb = s.memBar || {}, zr = s.zram || null, pr = s.pressure;
            const drive = (s.swaps || []).filter(x => x.path.indexOf("/dev/zram") !== 0);
            progs.title = "Programs";
            progs.chips = [[size(mb.inUse || 0) + " in use", "main"], ["pressure: " + pressWord(pr), pr !== null && pr !== undefined && pr >= 10 ? "warn" : "good"]];
            out.push(progs);
            const programs = Math.max(0, (mb.inUse || 0) - (zr ? zr.mem : 0));
            out.push({ key: "memory:physical", title: "Physical memory", kind: "bar", chips: [[size(mb.installed || mb.total), ""]], total: mb.installed || mb.total || 1,
                       parts: [["Kept", mb.reserved || 0, "#5d7488", "#e6eef5"], ["In use " + size(programs), programs, "#8b80f9"], ["Compr. " + size(zr ? zr.mem : 0), zr ? zr.mem : 0, "#3fd0e0"],
                               ["Cached " + size((mb.cache || 0) + (mb.saving || 0)), (mb.cache || 0) + (mb.saving || 0), "#cfc9d8"], ["Free " + size(mb.free), mb.free || 0, "#2f4a63", "#c9d6e2"]],
                       facts: [["Kept for hardware", size(mb.reserved || 0)], ["In use", size(programs)], ["Compressed (zram real size)", zr ? size(zr.mem) : "off"],
                               ["Cached", size((mb.cache || 0) + (mb.saving || 0))], ["Free", size(mb.free)], ["Available", size((s.mem && s.mem.available) || 0)],
                               ["Zram holds", zr && zr.size ? size(zr.orig) + " of " + size(zr.size) + (zr.mem ? " (" + Math.max(1, Math.round(zr.orig / zr.mem)) + ":1)" : "") : "off"],
                               ["Drive swap", drive.length ? size(drive.reduce((a, x) => a + x.used, 0)) + " of " + size(drive.reduce((a, x) => a + x.size, 0)) : "none"],
                               ["Committed", size(mb.committed || 0) + (s.mem && s.mem.commitLimit ? " of " + size(s.mem.commitLimit) : "")]] });
        } else if (t === "disk") {
            const ds = s.disks || [];
            progs.title = "Programs using the disk";
            progs.chips = [[rate(ds.reduce((a, x) => a + (x.write || 0), 0)) + " write", ""], [rate(ds.reduce((a, x) => a + (x.read || 0), 0)) + " read", ""]];
            out.push(progs);
            out.push({ key: "disk:files", title: "Files being read and written", kind: "table", cols: [["File", 6], ["Program", 2], ["PID", 1, true], ["Read", 1.2, true], ["Write", 1.2, true]],
                       rows: busyFiles.slice(0, 20).map(x => { const g = busyNames[x.pid] || {}; return [(x.writing ? "✎ " : "") + (x.win || x.path), g.name || "", String(x.pid), rate(x.read || 0), rate(x.write || 0)]; }),
                       empty: "Nothing is reading or writing files right now." });
            out.push({ key: "disk:drives", title: "Drives", kind: "table", cols: [["Drive", 2], ["Type", 2], ["Busy", 1, true], ["Response", 1.2, true], ["Free", 1.2, true], ["Total", 1.2, true], ["Health", 1, true], ["Temp", 1, true]],
                       rows: driveInfo.map(d => { const h = (driveHealth.drives || {})[String(d.disk || d.name || "").replace(/p?\d+$/, "")] || {}; const live = ds.find(x => x.name === d.disk || x.name === d.name) || {};
                           return [d.label || d.name, [d.kind, d.fs].filter(x => x).join(" · "), live.active !== undefined ? pct(live.active) : "–", live.response !== null && live.response !== undefined ? live.response + " ms" : "–",
                                   size(d.free), size(d.total), { t: h.lifeLeft !== null && h.lifeLeft !== undefined ? h.lifeLeft + "%" : h.ok === false ? "Failing" : "–", c: h.ok === false ? v.c.warn : h.lifeLeft >= 80 ? v.c.ok : "" },
                                   d.temp ? Math.round(d.temp) + "°C" : h.temp ? h.temp + "°C" : "–"]; }) });
        } else if (t === "network") {
            const ns = (s.nets || []).filter(x => x.up);
            progs.title = "Programs using the network";
            progs.chips = [[rate(ns.reduce((a, x) => a + (x.recv || 0), 0)) + " down", ""], [rate(ns.reduce((a, x) => a + (x.send || 0), 0)) + " up", ""]];
            out.push(progs);
            const pick = r => !tickedNames.length || tickedNames.some(g => (g.pids || []).indexOf(r.pid) >= 0);
            out.push({ key: "net:conns", title: "Connections (TCP)", kind: "table", cols: [["Program", 2.4], ["PID", 1, true], ["Talking to", 4.5], ["Port", 1, true], ["Ping", 1, true], ["State", 1]],
                       rows: (netPorts.connections || []).filter(pick).slice(0, 30).map(c => [c.name, c.pid ? String(c.pid) : "—", c.host || c.remote, String(c.port), c.ping !== null && c.ping !== undefined ? c.ping + " ms" : "–", c.state || "Open"]) });
            out.push({ key: "net:listen", title: "Listening ports", kind: "table", chips: [["firewall: " + (netPorts.firewall || "allowed only on your network"), "warn"]],
                       cols: [["Program", 2.4], ["PID", 1, true], ["Port", 1, true], ["Open to", 2], ["What it is", 4]],
                       rows: (netPorts.listening || []).filter(pick).slice(0, 30).map(l => [l.name, l.pid ? String(l.pid) : "—", String(l.port), l.openTo, l.what || ""]) });
        } else if (t === "gpu") {
            const g = (s.gpus || [])[0] || {}, card = s.card || {};
            progs.title = "Programs using the graphics card";
            progs.chips = g.name ? [[(g.vendor === "Intel" ? "Intel graphics" : g.vendor || "GPU") + " " + (g.busy === null || g.busy === undefined ? "" : pct(g.busy)), "main"]].concat(g.temp ? [[Math.round(g.temp) + "°C", ""]] : []) : [];
            out.push(progs);
            const games = groups.filter(x => x.kind === "game").map(x => x.name);
            out.push({ key: "gpu:card", title: "Graphics card", kind: "facts",
                       facts: [["Name", g.name || "–"], ["Driver", [card.mesa ? "Mesa " + card.mesa : "", card.driver || g.driver || ""].filter(x => x).join(" · ") || "–"],
                               ["Shared memory", g.gttTotal ? size(g.gttUsed || 0) + " of " + size(g.gttTotal) : g.vramTotal ? size(g.vramUsed || 0) + " of " + size(g.vramTotal) : "–"],
                               ["Clock", g.mhz ? Math.round(g.mhz) + (g.mhzMax ? " / " + Math.round(g.mhzMax) : "") + " MHz" : "–"], ["Temperature", g.temp ? Math.round(g.temp) + "°C" : "–"],
                               ["Power", g.watts ? (g.watts < 10 ? g.watts.toFixed(1) : Math.round(g.watts)) + " W" : "–"],
                               ["Screens", (card.screens || []).length ? card.screens.length + " · " + card.screens.join(", ") : "–"],
                               ["Vulkan · OpenGL", [card.vulkan || "–", card.opengl || "–"].join(" · ")], ["Games running", games.length ? games.join(", ") : "none"]] });
        }
        return out;
    }
    function priorityName(n) { return n === undefined || n === null ? "Normal" : n < -10 ? "High" : n < 0 ? "Above normal" : n === 0 ? "Normal" : n < 19 ? "Below normal" : "Low"; }
    readonly property var colsFor: ({ cpu: ["CPU", "Threads", "Priority", "Cores"], memory: ["In use", "Private", "Shared", "Compressed", "On drive", "Waits / s"],
                                      disk: ["Read", "Write", "Total", "Priority"], network: ["Down", "Up", "Total"], gpu: ["3D", "Video decode", "Video encode", "Video memory", "Through"] })
    readonly property var tableRows: { samples; return rowsFor(tab === "overview" ? "cpu" : tab); }
    // Overview's state: folded sections and the programs you ticked (only while it's open; nothing is saved).
    property var folded: ({ disk: true, network: true, gpu: true })
    property var ticked: ({})
    readonly property var overviewCols: ({ cpu: ["CPU", "Average"], memory: ["In use", "Waits / s"], disk: ["Read", "Write", "Total"], network: ["Down", "Up", "Total"], gpu: ["3D", "Video decode", "Video memory"] })
    // The chips on each Overview heading: [[text, tone]] (tone: main, good, warn or "").
    function overviewChips(k) {
        const s = sample || {};
        if (k === "cpu") return s.cpu ? [[pct(s.cpu.total) + " used", "main"], [ghz(s.cpu.mhz), ""]].concat(s.cpu.temp ? [[temp(s.cpu.temp), s.cpu.temp > 85 ? "warn" : ""]] : []) : [];
        if (k === "memory") {
            const mb = s.memBar || {};
            const pr = s.pressure !== undefined && s.pressure !== null ? s.pressure : null;
            return [[size(mb.inUse || 0) + " in use", "main"]].concat(s.zram && s.zram.mem ? [[size(s.zram.mem) + " compressed", ""]] : mb.swapUsed ? [[size(mb.swapUsed) + " in swap", ""]] : [])
                .concat(pr === null ? [] : [["pressure: " + (pr < 1 ? "none" : pr < 10 ? "light" : pr < 30 ? "medium" : "heavy"), pr < 10 ? "good" : "warn"]]);
        }
        if (k === "disk") { const d = (s.disks || []).slice().sort((a, b) => b.active - a.active)[0]; return d ? [[d.label + " " + pct(d.active) + " busy", "main"], [rate((s.disks || []).reduce((a, x) => a + x.write, 0)) + " write", ""]] : []; }
        if (k === "network") { const n = (s.nets || []).filter(x => x.up); return n.length ? [[rate(n.reduce((a, x) => a + x.recv, 0)) + " down", "main"], [rate(n.reduce((a, x) => a + x.send, 0)) + " up", ""]] : [["Not connected", ""]]; }
        if (k === "gpu") { const g = (s.gpus || [])[0]; return g ? [[(g.vendor || "GPU") + " " + (g.busy === null || g.busy === undefined ? "" : pct(g.busy)), "main"]].concat(g.temp ? [[temp(g.temp), ""]] : []) : [["No graphics card seen", ""]]; }
        return [];
    }
    property int listIndex: 0
    // Processor tab: the selected program's open files (1.11, mockup resmon-2; hearth-resmon files PID…).
    property var openFiles: []
    // Network tab: listening ports and open connections (1.11, mockup resmon-5; hearth-resmon ports), every 3 s.
    property var netPorts: ({ listening: [], connections: [] })
    Timer { interval: 2000; repeat: true; triggeredOnStart: true; running: v.tab === "network" && v.visible; onTriggered: if (!portsProc.running) portsProc.running = true }
    // Disk tab (1.11, mockup resmon-4): each drive's kind, space and temperature every 5 s, and the files the busiest
    // programs have open for writing every 3 s (hearth-resmon drives / writing PID…).
    property var driveInfo: []
    property var busyFiles: []
    Timer { interval: 5000; repeat: true; triggeredOnStart: true; running: v.tab === "disk" && v.visible; onTriggered: if (!drivesProc.running) drivesProc.running = true }
    Process {
        id: drivesProc
        command: ["python3", v.helper, "drives"]
        stdout: StdioCollector { onStreamFinished: { try { v.driveInfo = JSON.parse(text.trim().split("\n").pop()).drives || []; } catch (e) {} } }
    }
    Timer {
        interval: 3000; repeat: true; triggeredOnStart: true; running: v.tab === "disk" && v.visible
        onTriggered: {
            if (writingProc.running) return;
            const busy = v.groups.filter(g => (g.write || 0) + (g.read || 0) > 0).sort((a, b) => ((b.read || 0) + (b.write || 0)) - ((a.read || 0) + (a.write || 0))).slice(0, 3);
            const pids = [].concat(...busy.map(g => (g.pids || []).slice(0, 20)));
            if (!pids.length) { v.busyFiles = []; return; }
            v.busyNames = {}; const nm = {}; for (const g of busy) for (const p of (g.pids || [])) nm[p] = g;
            v.busyNames = nm;
            writingProc.command = ["python3", v.helper, "writing"].concat(pids.map(String));
            writingProc.running = true;
        }
    }
    property var busyNames: ({})
    Process {
        id: writingProc
        stdout: StdioCollector { onStreamFinished: { try { v.busyFiles = JSON.parse(text.trim().split("\n").pop()).files || []; } catch (e) { v.busyFiles = []; } } }
    }
    Process {
        id: portsProc
        command: ["python3", v.helper, "ports"]
        stdout: StdioCollector { onStreamFinished: { try {
            v.netPorts = JSON.parse(text.trim().split("\n").pop());
            const pings = (v.netPorts.connections || []).map(c => c.ping).filter(x => x !== null && x !== undefined);
            const h = Object.assign({}, v.hist); v.push(h, "ping", pings.length ? pings.reduce((a, x) => a + x, 0) / pings.length : 0); v.hist = h;
        } catch (e) {} } }
    }

    property var services: []
    property var libs: []
    property var driveHealth: ({})
    property var tempLimit: null
    readonly property string focusId: focusGroup ? String(focusGroup.id) : ""
    onFocusIdChanged: lookAtFocus()
    Timer { interval: 5000; repeat: true; running: v.tab === "cpu" && v.visible; onTriggered: v.lookAtFocus() }
    function lookAtFocus() { if (tab === "cpu" && focusGroup && focusGroup.pids) {
        if (!svcProc.running) { svcProc.command = ["python3", v.helper, "services"].concat(focusGroup.pids.slice(0, 64).map(String)); svcProc.running = true; }
        if (!libsProc.running) { libsProc.command = ["python3", v.helper, "libs", String(focusGroup.pid)]; libsProc.running = true; }
        if (!filesProc.running) { filesProc.command = ["python3", v.helper, "files"].concat(focusGroup.pids.slice(0, 64).map(String)); filesProc.running = true; }
    } }
    Process { id: svcProc; stdout: StdioCollector { onStreamFinished: { try { v.services = JSON.parse(text.trim().split("\n").pop()).services || []; } catch (e) { v.services = []; } } } }
    Process { id: libsProc; stdout: StdioCollector { onStreamFinished: { try { v.libs = JSON.parse(text.trim().split("\n").pop()).libs || []; } catch (e) { v.libs = []; } } } }
    Process { id: healthProc; command: ["pkexec", "/usr/libexec/nimbus/hearth-hwset", "disk-health"]; stdout: StdioCollector { onStreamFinished: { try { v.driveHealth = JSON.parse(text.trim().split("\n").pop()) || {}; } catch (e) {} } } }
    Process { id: limitProc; running: true; command: ["python3", "/usr/libexec/nimbus/hearth-hwset", "status"]; stdout: StdioCollector { onStreamFinished: { try { v.tempLimit = JSON.parse(text.trim().split("\n").pop()).tempLimit || null; } catch (e) {} } } }
    Timer { interval: 60000; repeat: true; triggeredOnStart: true; running: v.tab === "disk" && v.visible; onTriggered: if (!healthProc.running) healthProc.running = true }
    Process {
        id: filesProc
        stdout: StdioCollector { onStreamFinished: { try { v.openFiles = JSON.parse(text.trim().split("\n").pop()).files || []; } catch (e) { v.openFiles = []; } } }
    }
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
        // ---- the title row (1.12, mockups resmon-1 … -6): Live · every second, Pause and Open Task Manager
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            color: v.c.bar
            Row {
                anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                spacing: 14
                Text { text: "🔥 Resource Monitor"; color: v.c.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                Text { text: v.paused ? "Paused" : v.sampleError ? "Paused: " + v.sampleError : "Live · every second"; color: v.paused || v.sampleError ? v.c.warn : v.c.muted; font.family: Theme.font; font.pixelSize: 13 }
            }
            Row {
                anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                spacing: 6
                Text {
                    text: v.paused ? "Resume ▶" : "Pause ⏸"; color: pzm.containsMouse ? v.c.text : v.c.muted; font.family: Theme.font; font.pixelSize: 13
                    MouseArea { id: pzm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: v.paused = !v.paused }
                }
                Text { text: "·"; color: v.c.muted; font.family: Theme.font; font.pixelSize: 13 }
                Text {
                    text: "Open Task Manager"; color: otm.containsMouse ? v.c.text : v.c.muted; font.family: Theme.font; font.pixelSize: 13
                    MouseArea { id: otm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: v.ipc(["tasks", "open"]) }
                }
            }
        }
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
                                    readonly property var secRows: { v.samples; v.ticked; const r = v.rowsFor(k).map(x => ({ g: x.g, cells: x.ov || x.cells })); const t = Object.keys(v.ticked); return (t.length ? r.filter(x => v.ticked[x.g.id]) : r).slice(0, 5); }
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
                                text: Object.keys(v.ticked).length ? "Tick programs to filter every section to just them (here: " + v.groups.filter(g => v.ticked[g.id]).map(g => g.name).join(", ") + "). Click a heading to fold it." : "Tick programs to filter every section to just them. Click a heading to fold it."
                                color: v.c.muted; font.family: Theme.font; font.pixelSize: 12
                            }
                        }
                    }

                    // Processor, Memory, Disk, Network, Graphics (1.12, built completely to the approved mockups
                    // resmon-2 … resmon-6; red flag 11): foldable sections, each with its chips, Programs first.
                    Flickable {
                        id: tabFlick
                        visible: v.tab !== "overview"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        contentHeight: tabCol.implicitHeight
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ColumnLayout {
                            id: tabCol
                            width: parent.width
                            spacing: 10
                            Repeater {
                                model: { v.samples; v.ticked; v.folded; v.services; v.libs; v.openFiles; v.busyFiles; v.driveInfo; v.netPorts; v.driveHealth; return v.tab === "overview" || v.tab === "pc" ? [] : v.sectionsFor(v.tab); }
                                delegate: Rectangle {
                                    id: tsec
                                    required property var modelData
                                    readonly property var spec: modelData
                                    readonly property bool open: !v.folded[spec.key]
                                    Layout.fillWidth: true
                                    visible: !spec.hidden
                                    implicitHeight: spec.banner ? 38 : 38 + (open ? tbody.implicitHeight : 0)
                                    radius: 8; color: spec.banner ? Qt.rgba(0.6, 0.5, 1, 0.12) : v.c.card; border.width: spec.banner ? 0 : 1; border.color: v.c.line; clip: true
                                    // a line of words (Filtered to …)
                                    Text {
                                        visible: !!tsec.spec.banner
                                        anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                                        textFormat: Text.StyledText
                                        text: tsec.spec.banner || ""
                                        color: v.c.sub; font.family: Theme.font; font.pixelSize: 13
                                    }
                                    Column {
                                        visible: !tsec.spec.banner
                                        anchors { left: parent.left; right: parent.right; top: parent.top }
                                        Rectangle {
                                            width: parent.width; height: 38; color: v.c.head
                                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const f = Object.assign({}, v.folded); f[tsec.spec.key] = !f[tsec.spec.key]; v.folded = f; } }
                                            Text { anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter } text: (tsec.open ? "▾ " : "▸ ") + tsec.spec.title; color: v.c.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                                            Row {
                                                anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                                                spacing: 6
                                                Repeater {
                                                    model: tsec.spec.chips || []
                                                    delegate: Rectangle {
                                                        required property var modelData
                                                        height: 24; width: tchT.implicitWidth + 18; radius: 12; color: "transparent"
                                                        border.color: modelData[1] === "good" ? v.c.ok : modelData[1] === "warn" ? v.c.warn : modelData[1] === "main" ? v.c.accent : v.c.line
                                                        Text { id: tchT; anchors.centerIn: parent; text: parent.modelData[0]; color: parent.modelData[1] === "good" ? v.c.ok : parent.modelData[1] === "warn" ? v.c.warn : v.c.text; font.family: Theme.font; font.pixelSize: 12 }
                                                    }
                                                }
                                            }
                                        }
                                        Column {
                                            id: tbody
                                            visible: tsec.open
                                            width: parent.width
                                            // the programs: ticks, a picked row, right-click actions
                                            ProgHeader { visible: tsec.spec.kind === "programs"; width: parent.width; cols: tsec.spec.cols || []; head: true }
                                            Repeater {
                                                model: tsec.open && tsec.spec.kind === "programs" ? tsec.spec.rows : []
                                                delegate: ProgRow {
                                                    required property var modelData; required property int index
                                                    width: parent.width; row: modelData; alt: index % 2 === 0; picked: index === v.listIndex
                                                    tickable: true; ticked: !!v.ticked[modelData.g.id]
                                                    onTick: { const t = Object.assign({}, v.ticked); if (t[modelData.g.id]) delete t[modelData.g.id]; else t[modelData.g.id] = true; v.ticked = t; }
                                                    onClickedRow: v.listIndex = index
                                                    onMenuRow: (x, y) => { v.listIndex = index; const pt = mapToItem(v, x, y); v.openMenu(modelData.g, pt.x, pt.y); }
                                                }
                                            }
                                            // any other table
                                            GenRow { visible: tsec.spec.kind === "table"; width: parent.width; cols: tsec.spec.cols || []; cells: (tsec.spec.cols || []).map(c => c[0]); head: true }
                                            Repeater {
                                                model: tsec.open && tsec.spec.kind === "table" ? tsec.spec.rows : []
                                                delegate: GenRow { required property var modelData; required property int index; width: parent.width; cols: tsec.spec.cols; cells: modelData; alt: index % 2 === 0 }
                                            }
                                            // Memory's Physical memory bar
                                            Item {
                                                visible: tsec.spec.kind === "bar"
                                                width: parent.width; height: visible ? 50 : 0
                                                Rectangle {
                                                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 14 }
                                                    height: 30; radius: 6; clip: true; color: v.c.line
                                                    Row {
                                                        anchors.fill: parent
                                                        Repeater {
                                                            model: tsec.spec.kind === "bar" ? tsec.spec.parts : []
                                                            delegate: Rectangle {
                                                                required property var modelData
                                                                height: 30; color: modelData[2]
                                                                width: tsec.spec.total ? parent.width * (modelData[1] || 0) / tsec.spec.total : 0
                                                                Text { anchors.centerIn: parent; visible: implicitWidth + 8 < parent.width; text: modelData[0]; color: modelData[3] || "#10151b"; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // facts in three columns
                                            Grid {
                                                visible: tsec.spec.kind === "bar" || tsec.spec.kind === "facts"
                                                width: parent.width - 28; x: 14; columns: 3; columnSpacing: 24; rowSpacing: 6
                                                topPadding: tsec.spec.kind === "facts" ? 12 : 0; bottomPadding: 12
                                                Repeater {
                                                    model: tsec.spec.facts || []
                                                    delegate: Item {
                                                        required property var modelData
                                                        width: (tbody.width - 28 - 48) / 3; height: 20
                                                        Text { anchors.left: parent.left; text: modelData[0]; color: v.c.sub; font.family: Theme.font; font.pixelSize: 13 }
                                                        Text { anchors.right: parent.right; text: modelData[1]; color: v.c.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                                                    }
                                                }
                                            }
                                            Text { visible: tsec.spec.kind !== "bar" && tsec.spec.kind !== "facts" && !(tsec.spec.rows || []).length; leftPadding: 12; topPadding: 8; bottomPadding: 8; text: tsec.spec.empty || "Nothing here right now."; color: v.c.muted; font.family: Theme.font; font.pixelSize: 12 }
                                        }
                                    }
                                }
                            }
                            Text {
                                Layout.fillWidth: true; Layout.leftMargin: 4; wrapMode: Text.WordWrap
                                text: "Tick programs to show only them in every section. Click a heading to fold it."
                                color: v.c.muted; font.family: Theme.font; font.pixelSize: 12
                            }
                        }
                    }
                }

                // ---- the live graphs (Processor: every core, two to a row, and the temperature)
                GridLayout {
                    Layout.preferredWidth: v.tab === "cpu" ? 340 : 300
                    Layout.fillHeight: true
                    columns: v.tab === "cpu" ? 2 : 1
                    rowSpacing: 10; columnSpacing: 10
                    Repeater {
                        model: { v.samples; const s = v.sample || {}; const mb = s.memBar || {};
                            const gpu0 = (s.gpus || [])[0] || {};
                            const last = k => (v.hist[k] || []).slice(-1)[0] || 0;
                            // 1.12 (mockups resmon-2 … -5): each tab's own graphs
                            if (v.tab === "cpu") return ((s.cpu && s.cpu.cores) || []).slice(0, 16).map((x, i) => ["Core " + i, v.pct(x) + ((s.coreMhz || [])[i] ? " · " + ((s.coreMhz[i]) / 1000).toFixed(1) + " GHz" : ""), "#9a7cff", v.hist["core:" + i], 100, "", 1])
                                .concat([["Temperature", s.cpu && s.cpu.temp ? Math.round(s.cpu.temp) + "°C" + (v.tempLimit ? " · limit " + v.tempLimit + "°C" : "") : "–", "#ff9a4d", v.hist.ctemp, v.tempLimit || 100, "", 2]]);
                            if (v.tab === "memory") return [["Used memory", mb.total ? v.pct(100 * (mb.inUse || 0) / mb.total) : "–", "#8b80f9", v.hist.mem, mb.total || 1, ""],
                                ["Compressed (zram)", s.zram ? v.size(s.zram.orig) : "off", "#3fd0e0", v.hist.memz, 0, ""],
                                ["Memory pressure", v.pct(s.pressure || 0), "#7fe0a4", v.hist.mempsi, 100, ""],
                                ["Swapping in / out", s.swapIO ? ((s.swapIO.in / 1048576).toFixed(0) + " / " + (s.swapIO.out / 1048576).toFixed(0) + " MB/s") : "0 / 0 MB/s", "#f2c14e", v.hist.swapio, 0, ""]];
                            if (v.tab === "disk") return (s.disks || []).slice(0, 3).map(x => [x.label, v.pct(x.active) + " busy", "#f2a54e", v.hist["disk:" + x.name], 100, ""])
                                .concat([["Response time", last("resp") ? last("resp") + " ms" : "–", "#7fe0a4", v.hist.resp, -1, ""]]);
                            if (v.tab === "network") return (s.nets || []).filter(n => n.up).slice(0, 2).map(n => [n.kind === "Tailscale" ? "Tailscale" : n.kind + (n.ssid ? " " + n.ssid : ""),
                                    v.rate((n.recv || 0) + (n.send || 0)) + (n.kind === "Tailscale" ? (v.tsPeers >= 0 ? " · " + v.tsPeers + " devices" : "") : n.speed ? " · " + n.speed + " Mbps link" : ""),
                                    n.kind === "Tailscale" ? "#3fd0e0" : "#7fe0a4", v.hist["net:" + n.name], 0, ""])
                                .concat([["Ping", last("ping") ? Math.round(last("ping")) + " ms" : "–", "#f2c14e", v.hist.ping, -1, ""]]);
                            if (v.tab === "gpu") return [["3D", v.pct(last("g3d")), "#ff6fb1", v.hist.g3d, 100, gpu0.name || ""],
                                ["Video decode", v.pct(last("gdec")), "#3fb8c9", v.hist.gdec, 100, "Videos playing through the graphics card"],
                                ["Video memory", v.size(last("vram")), "#9a7cff", v.hist.vram, gpu0.vramTotal || 0, gpu0.vramTotal ? "of " + v.size(gpu0.vramTotal) : "shared with the system"],
                                ["Temperature", last("gtemp") ? Math.round(last("gtemp")) + "°C" : "–", "#ff9a4d", v.hist.gtemp, 100, gpu0.watts ? Math.round(gpu0.watts) + " W" : ""]];
                            if (v.tab === "network") return [["Network", v.rate(last("net")), "#7ccf8a", v.hist.net, 0, ((s.nets || []).find(n => n.up) || {}).kind || "Not connected"],
                                ["Ping", last("ping") ? Math.round(last("ping")) + " ms" : "–", "#f2c14e", v.hist.ping, -1, "Average of your open connections"],
                                ["Processor", v.pct(s.cpu ? s.cpu.total : 0), "#66c0f4", v.hist.cpu, 100, ""],
                                ["Memory", v.size(mb.inUse), "#9a7cff", v.hist.mem, mb.total || 1, ""]];
                            if (v.tab === "disk") return [["Disk", v.rate(last("disk")), "#7ccf8a", v.hist.disk, 0, "All drives, read and write"],
                                ["Response time", last("resp") ? last("resp") + " ms" : "–", "#3fb8c9", v.hist.resp, -1, "The slowest drive"],
                                ["Processor", v.pct(s.cpu ? s.cpu.total : 0), "#66c0f4", v.hist.cpu, 100, ""],
                                ["Memory", v.size(mb.inUse), "#9a7cff", v.hist.mem, mb.total || 1, ""]];
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
                            Layout.columnSpan: modelData[6] || 1
                            Layout.preferredWidth: 1
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
                                        const mx = topVal > 0 ? topVal : topVal < 0 ? Math.max(1, ...a) * 1.15 : Math.max(1048576, ...a) * 1.15;   // < 0: small numbers (ms)
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
    component GenRow: Rectangle {
        id: gr
        property var cols: []
        property var cells: []
        property bool head: false
        property bool alt: false
        readonly property real total: cols.reduce((a, c) => a + (c[1] || 1), 0) || 1
        height: head ? 26 : 28
        color: head ? "#213347" : (alt ? "#1f3042" : "transparent")
        Row {
            id: grRow
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            Repeater {
                model: gr.cols
                delegate: Text {
                    required property var modelData
                    required property int index
                    readonly property var cell: gr.cells[index]
                    width: grRow.width * (modelData[1] || 1) / gr.total
                    horizontalAlignment: modelData[2] ? Text.AlignRight : Text.AlignLeft
                    rightPadding: modelData[2] ? 12 : 0
                    elide: index === 0 || !modelData[2] ? Text.ElideMiddle : Text.ElideRight
                    text: cell && typeof cell === "object" ? (cell.t || "") : String(cell === null || cell === undefined ? "" : cell)
                    color: gr.head ? "#9fb2c3" : cell && typeof cell === "object" && cell.c ? cell.c : index === 0 ? "#e6eef5" : "#c9d6e2"
                    font.family: Theme.font; font.pixelSize: gr.head ? 12 : 13; anchors.verticalCenter: parent.verticalCenter
                }
            }
        }
    }
    component ProgHeader: Rectangle {
        readonly property var vv: ({ c: { head: "#213347", line: "#2f4a63", muted: "#9fb2c3", text: "#e6eef5", sub: "#c9d6e2", sel: "#2a475e", alt: "#1f3042" },
                                     tagColor: g => ({ game: "#f2c14e", steam: "#66c0f4", hearth: "#ff9a4d", app: "#9a7cff", sys: "#5d7488", web: "#7ccf8a", home: "#3fb8c9" })[(g || {}).color] || "#9a7cff" })
        property var cols: []
        property bool head: false
        height: 26
        color: head ? vv.c.head : "transparent"
        Rectangle { visible: !parent.head; anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: vv.c.line }
        readonly property real nameW: cols.length > 3 ? 0.30 : 0.42
        readonly property real pidW: cols.length > 3 ? 0.09 : 0.12
        Row {
            id: phRow
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            Text { width: parent.width * parent.parent.nameW; text: "Program"; color: vv.c.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
            Text { width: parent.width * parent.parent.pidW; horizontalAlignment: Text.AlignRight; rightPadding: 12; text: "PID"; color: vv.c.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: parent.parent.cols
                delegate: Text { required property var modelData; width: phRow.width * (1 - phRow.parent.nameW - phRow.parent.pidW) / Math.max(1, phRow.parent.cols.length); elide: Text.ElideRight; horizontalAlignment: Text.AlignRight; text: modelData; color: vv.c.muted; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight }
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
        readonly property var cells: row ? (row.cells || [row.a, row.b, row.c]) : []
        readonly property real nameW: cells.length > 3 ? 0.30 : 0.42
        readonly property real pidW: cells.length > 3 ? 0.09 : 0.12
        height: 28
        color: picked ? vv.c.sel : (alt ? vv.c.alt : "transparent")
        Row {
            id: prRow
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            Item {
                width: parent.width * pr.nameW; height: parent.height
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
            Text { width: parent.width * pr.pidW; horizontalAlignment: Text.AlignRight; rightPadding: 12; text: String(pr.g.pid || ""); color: vv.c.muted; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: pr.cells
                delegate: Text {
                    required property var modelData
                    required property int index
                    width: prRow.width * (1 - pr.nameW - pr.pidW) / Math.max(1, pr.cells.length)
                    horizontalAlignment: Text.AlignRight; elide: Text.ElideRight
                    text: String(modelData === null || modelData === undefined ? "" : modelData)
                    color: index === 0 ? vv.c.text : vv.c.sub; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter
                }
            }
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: m => { pr.clickedRow(); if (m.button === Qt.RightButton) pr.menuRow(m.x, m.y); }
            onPressAndHold: m => pr.menuRow(m.x, m.y)
        }
    }
}
