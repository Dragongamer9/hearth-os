// Hearth Settings › Home server › AI engine (docs/SERVER-MODE.md, "Colibri", 1.14): which AI engine Server mode runs:
// Ollama, Colibri or both. Colibri (github.com/JustVugg/colibri, Apache-2.0) runs very large mixture-of-experts models:
// the hot experts on the graphics card, warm ones in RAM, cold ones streamed from the NVMe drive. Every model in
// Colibri's own catalog is listed (nothing hidden), each with its download size, the disk it needs and an honest fit
// for this PC; the recommended one is marked. Nothing is downloaded until you press a model twice, and never without
// the free space. Opening this page writes nothing: hearth-colibri status and catalog only read.
// Settings: on / pause are Settings › Home server's (home-server.json); the model, the models folder and the engine
// Hearth's AI uses first are ai-engines.json's (hearth_ai_engines.py). Each saves only the key you change.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "AI engine"
    property var st: null
    property var cat: null
    property var plan: null
    property var dirs: null
    property string msg: ""
    property bool bad: false
    property string armed: ""            // a model pressed once: press again to download it
    property bool dirsOpen: false
    readonly property bool inst: !!st && st.installed
    readonly property var hw: cat ? cat.hardware : null
    readonly property var dl: st && st.download ? st.download : null
    readonly property var families: cat ? cat.models.filter(m => m.group === "uncensored").map(m => m.family).filter((f, i, a) => a.indexOf(f) === i) : []
    readonly property bool downloading: !!dl && dl.running === true
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    function loadCatalog() {
        if (!page.inst) return;
        cj.run(["catalog"], r => { page.cat = r; if (page.st && page.st.modelDownloaded) page.loadPlan(page.st.model); }, e => page.say(e, true));
    }
    function loadPlan(id) { cj.run(["plan", id], r => page.plan = r, () => page.plan = null); }
    function act(args, ok, after) { aj.run(args, r => { if (ok) page.say(ok); page.refresh(); if (after) after(r); }, e => { page.say(e, true); page.refresh(); }); }
    function fitKind(f) { return f === "good" ? "ok" : f === "slow" ? "warn" : f === "very-slow" ? "warn" : "bad"; }
    function size(g) { return g >= 1000 ? (g / 1000).toFixed(2) + " TB" : g >= 10 ? Math.round(g) + " GB" : g.toFixed(1) + " GB"; }
    Component.onCompleted: { refresh(); Qt.callLater(loadCatalog); }
    // CI screenshots only (colibri.yml): HEARTH_SHOT_SCROLL names a file holding "Y" or "Y open"; the page scrolls to
    // Y and opens the first Uncensored family. Never set on a real PC (reads nothing then, writes nothing ever).
    readonly property string shotFile: Quickshell.env("HEARTH_SHOT_SCROLL") || ""
    property bool shotOpen: false
    Timer { interval: 700; running: page.shotFile !== ""; repeat: true; onTriggered: shotView.reload() }
    FileView {
        id: shotView
        path: page.shotFile
        onLoaded: {
            const w = text().trim().split(/\s+/);
            const y = parseInt(w[0]);
            page.shotOpen = w[1] === "open";
            if (!isNaN(y)) page.contentY = Math.max(0, Math.min(y, page.contentHeight - page.height));
        }
    }
    onInstChanged: if (inst && !cat) loadCatalog()
    Timer { interval: 3000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !aj.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-colibri" }
    SetJob { id: cj; helper: "hearth-colibri" }
    SetJob { id: aj; helper: "hearth-colibri" }

    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 8
        text: "Server mode's AI. Ollama runs models that fit on the graphics card. Colibri runs much bigger ones too: the busiest parts on the graphics card, the rest in memory and on your fast drive (slower, but far smarter models). Both answer only this PC and your devices on Tailscale."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }

    SetHeader { text: "AI engine" }
    SetChoice {
        icon: "chat"; title: "Server mode runs"
        sub: !page.st ? "" : page.st.engine === "none" ? "Neither is on yet"
             : page.st.engine === "colibri" ? "Colibri only (Ollama is off)" : page.st.engine === "both" ? "Ollama and Colibri side by side (they share the graphics card)" : "Ollama, as before"
        options: [{ value: "ollama", label: "Ollama" }, { value: "colibri", label: "Colibri", disabled: !page.inst }, { value: "both", label: "Both", disabled: !page.inst }]
        current: page.st ? (page.st.engine === "none" ? "" : page.st.engine) : "ollama"
        onChosen: v => page.act(["set", "engine", v], v === "ollama" ? "Ollama on, Colibri off" : v === "colibri" ? "Colibri on, Ollama off" : "Ollama and Colibri on")
    }
    SetRow {
        visible: !!page.st && !page.inst
        icon: "plus"
        title: page.st && page.st.installing ? "Installing Colibri… " + (page.st.progress || 0) + "%" : "Install Colibri " + (page.st ? page.st.pinned : "")
        sub: page.st && page.st.error ? "Didn't install: " + page.st.error + ". Try again"
             : "About 14 MB from Colibri's own release on GitHub, checked against the SHA-256 it publishes before anything is unpacked. Models come separately, only the one you pick"
        value: page.st && page.st.installing ? "" : "Install"
        onActivated: if (page.st && !page.st.installing) page.act(["install"], "Downloading Colibri…")
    }
    SetRow {
        visible: page.inst
        icon: "sparkle"; chevron: false; interactive: false
        title: "Colibri " + (page.st ? page.st.version : "")
        chip: !page.st ? "" : !page.st.on ? "Off" : !page.st.active ? "Stopped" : page.st.server.health === "ready" ? "Ready" : "Starting"
        chipKind: !page.st || !page.st.on ? "off" : page.st.active && page.st.server.health === "ready" ? "ok" : "warn"
        sub: !page.st ? "" : !page.st.model ? "No model picked yet: download one below"
             : page.st.model + (page.st.server.tokensPerSecond ? " · " + page.st.server.tokensPerSecond + " tokens/s on the last answer" : "")
               + " · " + page.st.openai + " (model \"" + page.st.model + "\")"
    }
    SetChoice {
        visible: page.inst
        icon: "desktops"; title: "Hearth's own AI uses"
        sub: "What Hearth's AI pieces ask first (the AI team, Ithil when she asks Hearth). Ollama unless you change it"
        options: [{ value: "ollama", label: "Ollama" }, { value: "colibri", label: "Colibri", disabled: !page.st || !page.st.on }]
        current: page.st ? page.st.default : "ollama"
        onChosen: v => page.act(["set", "default", v], "Hearth's AI uses " + (v === "colibri" ? "Colibri" : "Ollama") + " first")
    }
    SetSwitch {
        visible: page.inst
        icon: "pad"; title: "Stop Colibri while you play"
        sub: "It holds the graphics card's memory. Stopped when Game Mode or a game starts, started again after"
        checked: !page.st || page.st.pause
        onToggled: v => page.act(["set", "pause", v ? "on" : "off"])
    }

    // ---------------------------------------------------------------- models: every one in Colibri's catalog
    SetHeader { text: "Models"; visible: page.inst }
    SetRow {
        visible: page.inst
        icon: "cpu"; chevron: false; interactive: false
        title: page.hw ? "This PC: " + Math.round(page.hw.ramGb) + " GB memory" + (page.hw.gpu ? " · " + page.hw.gpu + " " + page.hw.vramGb + " GB" : " · no graphics card found")
                         + " · " + page.size(page.hw.diskFreeGb) + " free for models" : "Reading what this PC has…"
        sub: page.cat ? "Every model in " + page.cat.source + " (" + page.cat.models.length + "), from the smallest to the largest. The fit is Hearth's estimate for this PC; the speed is what Colibri measured on other PCs" : ""
    }
    SetRow {
        visible: page.inst
        icon: "drive"; title: "Models folder"
        sub: page.st ? page.st.modelsDir : ""
        value: page.dirsOpen ? "Close" : "Change"
        onActivated: { page.dirsOpen = !page.dirsOpen; if (page.dirsOpen) aj.run(["folders"], r => page.dirs = r, e => page.say(e, true)); }
    }
    Repeater {
        model: page.dirsOpen && page.dirs ? page.dirs.folders : []
        delegate: SetRow {
            required property var modelData
            inset: true; icon: modelData.path === (page.dirs ? page.dirs.current : "") ? "check" : "drive"
            title: modelData.label + " · " + page.size(modelData.freeGb) + " free"
            sub: modelData.path
            onActivated: page.act(["set", "modelsDir", modelData.path], "Models go to " + modelData.path, () => { page.dirsOpen = false; page.loadCatalog(); })
        }
    }
    SetRow {
        visible: page.downloading
        icon: "update"
        title: page.dl ? "Downloading " + (page.dl.name || page.dl.id) + (page.dl.pct !== undefined ? "… " + page.dl.pct + "%" : "…") : ""
        sub: page.dl && page.dl.total ? page.size(page.dl.done / 1e9) + " of " + page.size(page.dl.total / 1e9) + (page.dl.rate ? " · " + (page.dl.rate / 1e6).toFixed(1) + " MB/s" : "")
             + " · every file checked; safe to stop, it continues where it stopped" : (page.dl && page.dl.phase ? "Getting ready: " + page.dl.phase : "Starting…")
        value: "Cancel"
        onActivated: page.act(["cancel"], "Stopped. What was downloaded stays")
    }
    SetRow {
        visible: !!page.dl && !page.dl.running && !!page.dl.error
        icon: "info"; chevron: false; interactive: false
        title: page.dl ? (page.dl.name || page.dl.id || "") + " didn't finish" : ""
        sub: page.dl ? (page.dl.error || "") : ""
    }
    Component {
        id: modelRow
        SetRow {
            required property var modelData
            readonly property var m: modelData
            inset: m.group === "uncensored"
            icon: m.chosen ? "check" : m.have ? "drive" : m.kind === "image" ? "image" : "plus"
            title: m.name + (m.recommended ? " · recommended" : "") + (m.kind === "image" ? " · pictures, not chat" : "")
            chip: m.fitLabel; chipKind: page.fitKind(m.fit)
            sub: m.summary + " · download " + page.size(m.downloadGb) + ", needs " + page.size(m.diskGb) + " free · " + m.why + (m.expect ? " · " + m.expect : "")
                 + " · measured elsewhere: " + m.measured + (m.license ? " · " + m.license : "")
                 + (page.armed === m.id ? " · press again to download" : "")
            value: m.chosen ? "In use" : m.have ? "Use" : m.downloading ? (page.dl && page.dl.pct !== undefined ? page.dl.pct + "%" : "…") : m.fit === "no-disk" ? "No room" : "Download"
            onActivated: {
                if (m.chosen) { page.loadPlan(m.id); return; }
                if (m.have) { page.act(["use", m.id], m.name + " is Colibri's model now", () => page.loadCatalog()); return; }
                if (m.downloading || page.downloading) { page.say("One download at a time: wait for it or cancel it", true); return; }
                if (m.fit === "no-disk") { page.say("Not enough disk space for " + m.name + ": it needs " + page.size(m.diskGb) + " free. Pick a bigger drive for your models", true); return; }
                if (page.armed !== m.id) { page.armed = m.id; page.say("Press again to download " + m.name + " (" + page.size(m.downloadGb) + ")" + (m.fit === "no-ram" ? ". It won't run on this PC's memory" : "")); return; }
                page.armed = "";
                page.act(["download", m.id], "Downloading " + m.name + "…", () => page.loadCatalog());
            }
        }
    }
    // Standard (filtered): Colibri's own catalog, every model in it. Uncensored: abliterated GGUF builds on Hugging Face of
    // models Colibri runs (Hearth's list, /usr/share/hearth/ai-models.json), each family folded, every size inside.
    SetHeader { text: "Standard (filtered)"; visible: !!page.cat }
    Repeater { model: page.cat ? page.cat.models.filter(m => m.group === "standard") : []; delegate: modelRow }
    SetHeader { text: "Uncensored"; visible: page.families.length > 0 }
    Text {
        visible: page.families.length > 0
        Layout.fillWidth: true
        text: "Builds of the same Qwen models with their refusals taken out (\"abliterated\"), from Hugging Face. They answer anything, so keep them off a PC children use. Each comes in every size its maker published: smaller is faster and fits more easily, bigger is closer to the original. Downloaded only when you press one twice, checked against Hugging Face's SHA-256, then made ready by Colibri (coli import)."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.families
        delegate: SetExpander {
            required property var modelData
            required property int index
            open: page.shotOpen && index === 0
            readonly property var rows: page.cat ? page.cat.models.filter(m => m.group === "uncensored" && m.family === modelData) : []
            readonly property var best: rows.find(m => m.recommended) || rows.find(m => m.quant === "Q4_K_M" || m.quant === "Q4_K") || rows[0]
            icon: rows.some(m => m.chosen) ? "check" : rows.some(m => m.have) ? "drive" : "lock"
            title: best ? best.familyName : modelData
            sub: rows.length + " sizes, " + (rows.length ? page.size(Math.min(...rows.map(r => r.downloadGb))) + " to " + page.size(Math.max(...rows.map(r => r.downloadGb))) : "") + (best ? " · " + best.summary : "")
            chip: best ? best.quant + ": " + best.fitLabel : ""; chipKind: best ? page.fitKind(best.fit) : "ok"
            Repeater { model: rows; delegate: modelRow }
        }
    }
    // The model in use: where Colibri's planner puts it on this PC (graphics card, memory, drive) and its doctor's word.
    SetHeader { text: "On this PC"; visible: !!page.plan && !!page.plan.id }
    SetRow {
        visible: !!page.plan && !!page.plan.id
        icon: "rocket"; chevron: false; interactive: false
        title: page.plan ? page.plan.id + ": " + (page.plan.vramGb || 0) + " GB on the graphics card · " + (page.plan.ramGb || 0) + " GB in memory · " + (page.plan.diskGb || 0) + " GB on the drive" : ""
        sub: page.plan ? (page.plan.bottleneck ? "What sets its speed: " + page.plan.bottleneck : "")
                         + (page.plan.hitRate !== undefined && page.plan.hitRate !== null ? " · experts found in memory: about " + Math.round(page.plan.hitRate * 100) + "%" : "")
                         + (page.plan.doctor ? " · check: " + page.plan.doctor : "")
                         + ((page.plan.problems || []).length ? " · " + page.plan.problems.join("; ") : "") : ""
    }

    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
