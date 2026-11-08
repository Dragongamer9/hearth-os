// The Briefcase in Hearth Files (docs/HEARTH-FILES.md › Briefcase): the logic behind it. Every file change runs in
// hearth-briefcase (beside this file), the way FileManager.qml runs hearth-files; this keeps what Files shows (each
// item's status, the lock, auto-sync) and opens the dialogs drawn by BriefcaseDialog.qml (Update All, passwords,
// Customize, choosing a Briefcase). Passwords go to the helper on stdin (stdinEnabled + write()), never on its
// command line or in its environment, and are forgotten as soon as they're sent.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: bc
    required property var app
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-briefcase").toString().replace(/^file:\/\//, ""))

    property var known: []            // hearth-briefcase list: the Briefcases this device knows
    property var st: null             // hearth-briefcase status of the Briefcase shown now
    readonly property bool inBriefcase: !!app.listing && !!app.listing.briefcase && !app.onHome
    readonly property bool locked: !!st && st.locked
    readonly property bool unlocked: !!st && st.unlocked
    readonly property string bcName: st ? st.name : ""
    property string mode: ""          // "" | update | password | setlock | passwd | customize | pick | job | message
    property int row: 0               // where the keyboard / controller is in the dialog
    property var plan: null           // {rows, info, name, path}
    property var pw: ({ a: "", b: "", c: "", remember: false, error: "", path: "", name: "", then: null })
    property var cust: null           // Customize's working copy: {path, look, locked}
    property var pick: null           // {title, list: [{label, sub, path, item}], then}
    property var job: null            // {verb, done, total, file, cancel}
    property var msg: null            // {title, text}
    property var jobProc: null

    // ================================================ the helper
    Component {
        id: callComp
        Process {
            id: p
            property var cb: null
            property var progress: null
            property string input: ""
            property bool finished: false
            stdinEnabled: input !== ""
            stdout: SplitParser {
                onRead: line => {
                    if (line.startsWith("progress: ")) {
                        if (p.progress) { try { p.progress(JSON.parse(line.slice(10))); } catch (e) {} }
                        return;
                    }
                    if (p.finished || !line.startsWith("{")) return;
                    let r = null;
                    try { r = JSON.parse(line); } catch (e) { return; }
                    if (r && r.ok !== undefined) { p.finished = true; if (p.cb) p.cb(r); }
                }
            }
            onStarted: {
                if (p.input !== "") { p.write(p.input); p.input = ""; }   // the password: sent, then stdin closes
            }
            onExited: (code, status) => {
                if (!p.finished) { p.finished = true; if (p.cb) p.cb({ ok: false, error: "Something went wrong", code: "error" }); }
                if (bc.jobProc === p) bc.jobProc = null;
                p.destroy();
            }
        }
    }
    // args: the helper's arguments; input: what goes on stdin (passwords, one per line); progress: for long jobs.
    function call(args, cb, input, progress) {
        const p = callComp.createObject(bc, { command: ["python3", helper].concat(args), cb: cb || null, input: input || "", progress: progress || null });
        p.running = true;
        return p;
    }
    // A job with progress and Cancel (Update, adding, sending, locking): SIGTERM stops it cleanly.
    function runJob(verb, args, then, input) {
        job = { verb: verb, done: 0, total: 0, file: "", cancel: false };
        mode = "job";
        jobProc = call(args, r => {
            const was = job;
            job = null;
            if (mode === "job") mode = "";
            if (then) then(r, was && was.cancel);
            refresh();
            app.reload();
            app.refreshPlaces();
        }, input, pr => { job = Object.assign({}, job || {}, { done: pr.done || 0, total: pr.total || 0, file: pr.file || "" }); });
    }
    function cancelJob() {
        if (jobProc && job) { job = Object.assign({}, job, { cancel: true }); jobProc.signal(15); }
    }
    // A locked Briefcase is unlocked first (asks for its password), then FN runs again.
    function needUnlock(r, again) {
        if (r && r.code === "locked") { askPassword(r.path, r.name, again); return true; }
        return false;
    }
    function done(r, okText) {
        if (!r || !r.ok) { app.say(r ? r.error : "Something went wrong", true); return false; }
        if (okText) app.say(okText);
        return true;
    }

    // ================================================ what Files shows
    function refreshKnown() { call(["list"], r => { if (r && r.ok) known = r.briefcases; }); }
    function refresh() {
        if (!inBriefcase) { st = null; return; }
        const at = app.listing.path;
        call(["status", at], r => {
            if (!app.listing || app.listing.path !== at) return;
            st = r && r.ok ? r : null;
        });
    }
    // Opening a folder: a Briefcase's status; a locked one asks for its password; an unlocked one opens what's inside.
    function opened(listing) {
        if (!listing || !listing.briefcase) { st = null; return; }
        const at = listing.path;
        call(["status", at], r => {
            if (!app.listing || app.listing.path !== at) return;
            st = r && r.ok ? r : null;
            if (!st) return;
            if (st.locked && st.unlocked && st.content && st.content !== at) return app.load(st.content, "", "none");
            if (st.locked && !st.unlocked) return askPassword(st.path, st.name, () => app.navigate(st.path, ""));
            call(["touch", at]);
        });
    }
    Timer { interval: 5000; repeat: true; running: bc.app.shown && bc.inBriefcase && bc.mode === ""; onTriggered: bc.refresh() }
    Timer { interval: 30000; repeat: true; running: bc.app.shown; triggeredOnStart: true; onTriggered: bc.refreshKnown() }

    function itemOf(e) {
        if (!st || !st.items || !e) return null;
        return st.items.find(i => i.path === e.path || i.name === e.name) || null;
    }
    readonly property var statusShort: ({ ok: "Up-to-date", "newer-b": "Newer in Briefcase", "newer-o": "Newer at original",
        both: "Changes on both sides", conflict: "Changed in both" })
    function statusLabel(e) {
        const it = itemOf(e);
        if (!it) return inBriefcase && e && !e.name.startsWith(".") ? "Not synced" : "";
        return statusShort[it.status] || it.label || "";
    }
    // A small badge on an item's icon: the colour of its status, and a glyph.
    function badgeFor(e) {
        const it = itemOf(e);
        if (!it) return null;
        const map = { ok: ["#5fb98f", "✓"], "newer-b": [Theme.accent, "→"], "newer-o": [Theme.accent, "←"], both: [Theme.accent, "⇄"],
            conflict: [Theme.danger, "!"], away: [Theme.muted, "…"], orphan: ["#7d6a5c", "×"] };
        const m = map[it.status] || [Theme.muted, "?"];
        return { color: m[0], glyph: m[1] };
    }
    readonly property var colors: ({ ember: "#ff8a3d", red: "#e06666", green: "#5fb98f", blue: "#4f9dde", purple: "#b36ad9", gold: "#e0b24f", slate: "#8a9aa8" })
    function colorOf(look) {
        const c = look && look.color ? look.color : "accent";
        if (c === "accent") return Theme.accent;
        return colors[c] || (/^#[0-9a-fA-F]{6}$/.test(c) ? c : Theme.accent);
    }
    function ago(t) {
        if (!t) return "";
        const s = Math.max(0, Date.now() / 1000 - t);
        return s < 60 ? "just now" : s < 3600 ? Math.round(s / 60) + " min ago" : s < 86400 ? Math.round(s / 3600) + " h ago" : Math.round(s / 86400) + " days ago";
    }
    // "Syncing…" / "Synced 2 min ago" / "2 files need you", for the details pane and the status bar.
    readonly property string syncLine: {
        if (!st || !st.sync) return "";
        const s = st.sync;
        const set = s.settings || {};
        if (st.locked && !st.unlocked) return set.mode === "off" ? "" : "Auto-sync waits until it's unlocked";
        if (s.syncing) return "Syncing…";
        if (s.needs) return plural(s.needs, "file") + " need" + (s.needs === 1 ? "s" : "") + " you";
        if (set.mode === "off") return "Auto-sync off · Update All to sync";
        if (s.away && !s.last) return "Waiting for originals to come back";
        return s.last ? "Synced " + ago(s.last) : "Auto-sync on";
    }
    readonly property string lockLine: {
        if (!st || !st.locked) return "";
        if (!st.unlocked) return "Locked";
        if (st.lockState === "never") return "Unlocked · locks only when you lock it";
        if (st.lockState === "open") return "Unlocked · a file in it is open";
        if (st.lockIn === null || st.lockIn === undefined) return "Unlocked";
        const m = Math.max(1, Math.round(st.lockIn / 60));
        return "Unlocked · Locks in " + (m >= 60 ? Math.round(m / 6) / 10 + " h" : m + " min");
    }
    function plural(n, w) { return n + " " + w + (n === 1 ? "" : "s"); }
    // The details pane: the Briefcase's note, lock and sync; an item's status and where its original is.
    function detailRows(e) {
        if (!inBriefcase || !st) return [];
        if (!e) {
            const r = [];
            if (st.look && st.look.note) r.push(["Note", st.look.note]);
            if (lockLine) r.push(["Lock", lockLine]);
            if (syncLine) r.push(["Sync", syncLine]);
            const c = st.counts || {};
            const n = (st.items || []).length;
            if (n) r.push(["Items", n + " · " + (c.ok || 0) + " up-to-date" + (n - (c.ok || 0) ? " · " + (n - (c.ok || 0)) + " need a look" : "")]);
            if (st.damaged) r.push(["List", "Damaged: kept aside (Start the list again)"]);
            return r;
        }
        const it = itemOf(e);
        if (!it) return e.name.startsWith(".") ? [] : [["Briefcase", "Not linked to an original"]];
        return [["Status", it.label], ["Original", it.origin]];
    }

    // ================================================ menus (the shared menu API: {id, label, icon, group})
    function itemsIn(t) { return st && st.items ? t.filter(p => st.items.some(i => i.path === p)) : []; }
    function others() { return known.filter(k => !k.missing && (!st || k.path !== st.path)); }
    function menuItems(e, n, t, onFolder) {
        const out = [];
        if (app.picking || app.inTrash || app.onHome && !e) return out;
        if (inBriefcase) {
            const mine = itemsIn(t);
            if (e && mine.length) {
                out.push({ id: "bc-update-sel", label: mine.length > 1 ? "Update " + mine.length + " items" : "Update", icon: "sync", group: "Briefcase" });
                if (mine.length === 1) out.push({ id: "bc-find", label: "Find original", icon: "folder", group: "Briefcase" });
                out.push({ id: "bc-split", label: "Split from original", icon: "link", group: "Briefcase" });
                if (others().length) {
                    out.push({ id: "bc-moveto", label: "Move to Briefcase  ›", icon: "bc-briefcase", group: "Briefcase" });
                    out.push({ id: "bc-copyto", label: "Copy to Briefcase  ›", icon: "bc-briefcase", group: "Briefcase" });
                    if (mine.length === 1) out.push({ id: "bc-swap", label: "Swap with…", icon: "bc-briefcase", group: "Briefcase" });
                }
            }
            if (!e) out.push.apply(out, folderItems(st));
            return out;
        }
        if (e && e.briefcase && n === 1) {
            out.push({ id: "bc-update-all:" + e.path, label: "Update All", icon: "sync", group: "Briefcase" });
            out.push(e.briefcase.locked ? { id: "bc-unlock:" + e.path, label: "Unlock…", icon: "lock", group: "Briefcase" }
                                        : { id: "bc-setlock:" + e.path, label: "Lock this Briefcase…", icon: "lock", group: "Briefcase" });
            out.push({ id: "bc-customize:" + e.path, label: "Customize…", icon: "bc-" + (e.briefcase.icon || "briefcase"), group: "Briefcase" });
            out.push({ id: "bc-send:" + e.path, label: "Send Briefcase to…", icon: "share", group: "Briefcase" });
            return out;
        }
        if (n && known.length) out.push({ id: "bc-addto", label: "Add to Briefcase  ›", icon: "bc-briefcase", group: "Briefcase" });
        return out;
    }
    function folderItems(s) {
        const out = [{ id: "bc-update-all", label: "Update All", icon: "sync", group: "Briefcase" }];
        if (s && s.locked) {
            out.push(s.unlocked ? { id: "bc-lock", label: "Lock", icon: "lock", group: "Briefcase" }
                                : { id: "bc-unlock", label: "Unlock…", icon: "lock", group: "Briefcase" });
            if (s.unlocked) out.push({ id: "bc-passwd", label: "Change password…", icon: "lock", group: "Briefcase" },
                                     { id: "bc-unlockremove", label: "Remove lock…", icon: "lock", group: "Briefcase" });
            out.push({ id: "bc-forget", label: "Forget remembered password", icon: "close", group: "Briefcase" });
        } else {
            out.push({ id: "bc-setlock", label: "Lock this Briefcase…", icon: "lock", group: "Briefcase" });
        }
        out.push({ id: "bc-customize", label: "Customize…", icon: "bc-" + ((s && s.look && s.look.icon) || "briefcase"), group: "Briefcase" },
                 { id: "bc-syncnow", label: "Sync now", icon: "sync", group: "Briefcase" },
                 { id: "bc-send", label: "Send Briefcase to…", icon: "share", group: "Briefcase" });
        if (s && s.damaged) out.push({ id: "bc-resetlist", label: "Start the list again…", icon: "undo", group: "Briefcase", danger: true });
        return out;
    }
    // The command bar's Briefcase buttons (shown inside a Briefcase).
    function toolShown(t) {
        if (!inBriefcase) return false;
        if (t === "bc-unlock") return locked && !unlocked;
        if (t === "bc-lock") return !locked || unlocked;
        return true;
    }
    function command(t, x, y) {
        if (t === "bc-lock" && !locked) return run("bc-setlock", [], null);
        return run(t, [], null);
    }

    // Everything the menus and buttons do. T: what's picked (paths); E: the item the menu is about.
    function run(id, t, e) {
        const colon = id.indexOf(":");
        const verb = colon > 0 ? id.slice(0, colon) : id;
        const arg = colon > 0 ? id.slice(colon + 1) : "";
        const here = st ? st.path : (app.listing ? app.listing.path : "");
        const target = arg && arg.startsWith("/") ? arg : here;
        switch (verb) {
        case "bc-new": return makeNew();
        case "bc-update-all": return openUpdate(target, []);
        case "bc-update-sel": return openUpdate(here, itemsIn(t).map(p => app.base(p)));
        case "bc-find": {
            const name = app.base(itemsIn(t)[0] || "");
            return call(["find", here, name], r => { if (done(r)) app.navigate(r.path, ""); });
        }
        case "bc-split": {
            const names = itemsIn(t).map(p => app.base(p));
            return app.confirm("Split " + (names.length === 1 ? "“" + names[0] + "”" : names.length + " items") + " from the original?",
                "The copy in the Briefcase stays, but it won't be updated any more (Orphan).", "Split",
                () => call(["split", here].concat(names), r => { if (done(r, "Split from the original")) refresh(); }));
        }
        case "bc-addto": return choose("Add to Briefcase", known.filter(k => !k.missing), k => addTo(k.path, t));
        case "bc-addto-path": return addTo(arg, t);
        case "bc-moveto": case "bc-copyto": {
            const names = itemsIn(t).map(p => app.base(p));
            const move = verb === "bc-moveto";
            return choose((move ? "Move" : "Copy") + " to Briefcase", others(), k => transfer(here, k.path, names, move));
        }
        case "bc-swap": {
            const name = app.base(itemsIn(t)[0] || "");
            return choose("Swap “" + name + "” with an item in…", others(), k => call(["status", k.path], r => {
                if (needUnlock(r && r.locked && !r.unlocked ? { code: "locked", path: k.path, name: k.name } : null, () => run(id, t, e))) return;
                if (!done(r)) return;
                choose("Swap “" + name + "” with", r.items.map(i => ({ name: i.name, path: k.path, sub: i.label })),
                       it => runJob("swap", ["swap", here, name, k.path, it.name], (res) => {
                           if (needUnlock(res, () => run(id, t, e))) return;
                           done(res, "Swapped “" + name + "” and “" + it.name + "”");
                       }));
            }));
        }
        case "bc-unlock": return askPassword(target, arg ? app.base(arg) : bcName, () => { if (!arg) app.navigate(target, ""); else refreshKnown(); });
        case "bc-lock": return call(["lock", target], r => {
            if (done(r, "Locked")) { app.navigate(target, ""); app.refreshPlaces(); }
        });
        case "bc-setlock": return openSetLock(target);
        case "bc-passwd": pw = { a: "", b: "", c: "", remember: false, error: "", path: target, name: bcName, then: null }; row = 0; mode = "passwd"; return padKeyboard();
        case "bc-unlockremove":
            return app.confirm("Remove the lock from “" + bcName + "”?",
                "Its files are decrypted into the Briefcase folder and checked; only then does the locked copy go to the Trash. Anyone with the folder can read them afterwards.",
                "Remove lock", () => runJob("unlock", ["lock-remove", target], r => { if (done(r, "The lock was removed")) app.navigate(target, ""); }));
        case "bc-forget": return call(["forget", target], r => done(r, "This device no longer remembers the password"));
        case "bc-customize": return openCustomize(target);
        case "bc-syncnow": return runJob("sync", ["sync-now", target], r => {
            if (!done(r)) return;
            app.say(r.synced ? plural(r.synced, "file") + " synced" + (r.needs ? " · " + plural(r.needs, "file") + " need you" : "")
                    : r.needs ? plural(r.needs, "file") + " need you: use Update All" : "Everything is up-to-date");
        });
        case "bc-send": {
            const drives = (app.rawPlaces || app.places).filter(p => p.section === "drives" && p.path).map(p => ({ name: p.label, path: p.path, sub: p.path }));
            drives.push({ name: "Home folder", path: app.home, sub: app.home });
            return choose("Send “" + (arg ? app.base(arg) : bcName) + "” to…", drives, d => runJob("send", ["send", target, d.path], r => {
                if (done(r, "Sent to " + d.name + ": it keeps its list and works there too")) refreshKnown();
            }));
        }
        case "bc-resetlist":
            return app.confirm("Start this Briefcase's list again?", "The damaged list is kept aside (nothing is deleted). The files stay, but they forget their originals.",
                "Start again", () => call(["reset-list", here], r => { if (done(r, "A new list was started; the old one is kept aside")) refresh(); }));
        }
    }
    function makeNew() {
        const parent = app.listing && app.writable ? app.listing.path : app.home;
        call(["new", parent, "Briefcase"], r => {
            if (!done(r, "Made “" + app.base(r.path) + "”")) return;
            refreshKnown();
            app.refreshPlaces();
            app.reload();
        });
    }
    function openPlace(p) {
        if (p.missing) return call(["new", app.home, "Briefcase", "--ensure"], r => { if (done(r)) { app.navigate(r.path, ""); app.refreshPlaces(); } });
        app.navigate(p.path, "");
    }
    function addTo(path, paths) {
        if (!paths.length) return;
        runJob("add", ["add", path].concat(paths), r => {
            if (needUnlock(r, () => addTo(path, paths))) return;
            done(r, r.added ? (r.added.length === 1 ? "“" + r.added[0].name + "” is in the Briefcase" : r.added.length + " items are in the Briefcase") : "");
        });
    }
    function transfer(from, to, names, move) {
        runJob(move ? "move" : "copy", ["transfer", from, to].concat(names).concat([move ? "--move" : "--copy"]), r => {
            if (needUnlock(r, () => transfer(from, to, names, move))) return;
            done(r, (move ? "Moved " : "Copied ") + (names.length === 1 ? "“" + names[0] + "”" : names.length + " items") + ": still linked to the original");
        });
    }
    // Dropped on a Briefcase (its folder, its icon, its place): added, or moved / copied from another Briefcase.
    function briefcaseAt(dest) {
        if (inBriefcase && st && (dest === app.listing.path || dest === st.path)) return st.path;
        const k = known.find(x => x.path === dest || x.content === dest);
        if (k) return k.path;
        const e = app.entries.find(x => x.path === dest);
        return e && e.briefcase ? e.path : "";
    }
    function drop(paths, dest, ev) {
        const to = briefcaseAt(dest);
        if (!to) return false;
        if (ev) ev.accept(Qt.CopyAction);
        const from = inBriefcase && st && st.path !== to && paths.every(p => app.parentOf(p) === app.listing.path) ? st.path : "";
        if (from && itemsIn(paths).length === paths.length) transfer(from, to, paths.map(p => app.base(p)), !(ev && ev.proposedAction === Qt.CopyAction));
        else addTo(to, paths);
        return true;
    }

    // ================================================ dialogs
    function padKeyboard() { if (app.viaPad) app.openKeyboard(); }
    function choose(title, list, then) {
        if (!list.length) return app.say("There's no other Briefcase yet. Make one with New › Briefcase");
        pick = { title: title, list: list.map(k => ({ label: k.look && k.look.name ? k.look.name : (k.name || app.base(k.path)), sub: k.sub || (k.locked ? (k.unlocked ? "Unlocked" : "Locked") : app.parentOf(k.path || "")),
                                                      look: k.look || null, raw: k })), then: then };
        row = 0;
        mode = "pick";
        app.overlay = "briefcase";
    }
    function pickChoose(i) {
        const p = pick;
        close();
        if (p && p.list[i]) p.then(p.list[i].raw);
    }
    function askPassword(path, name, then) {
        pw = { a: "", b: "", c: "", remember: false, error: "", path: path, name: name || app.base(path), then: then || null };
        row = 0;
        mode = "password";
        app.overlay = "briefcase";
        // A password this device remembers (only if you asked it to): tried first, quietly.
        call(["unlock", path, "--keyring"], r => {
            if (mode !== "password" || pw.path !== path) return;
            if (r && r.ok) { const th = pw.then; close(); app.say("“" + pw.name + "” is unlocked"); refreshKnown(); if (th) th(); }
            else padKeyboard();
        });
    }
    function submitPassword() {
        const p = pw;
        if (!p.a) return pw = Object.assign({}, p, { error: "Type the password" });
        const input = p.a + "\n";
        pw = Object.assign({}, p, { a: "", error: "Checking…" });
        call(["unlock", p.path].concat(p.remember ? ["--remember"] : []), r => {
            if (!r || !r.ok) return pw = Object.assign({}, pw, { error: r ? r.error : "Couldn't unlock" });
            close();
            app.say("“" + p.name + "” is unlocked");
            refreshKnown();
            app.refreshPlaces();
            if (p.then) p.then(); else app.navigate(r.content, "");
        }, input);
    }
    function openSetLock(path) {
        pw = { a: "", b: "", c: "", remember: false, error: "", path: path, name: path === (st ? st.path : "") ? bcName : app.base(path), then: null };
        row = 0;
        mode = "setlock";
        app.overlay = "briefcase";
        padKeyboard();
    }
    function submitSetLock() {
        const p = pw;
        if (p.a.length < 4) return pw = Object.assign({}, p, { error: "Use at least 4 characters" });
        if (p.a !== p.b) return pw = Object.assign({}, p, { error: "The two passwords aren't the same" });
        const input = p.a + "\n";
        pw = Object.assign({}, p, { a: "", b: "" });
        runJob("lock", ["lock-setup", p.path].concat(p.remember ? ["--remember"] : []), r => {
            if (!done(r)) return;
            msg = { title: "“" + p.name + "” is locked", text: "Its files are encrypted inside the folder. The unlocked copy went to the Trash" +
                    " (empty the Trash to remove it" + (p.path.indexOf("/media/") >= 0 ? ", including the drive's own Trash" : "") + ")." };
            mode = "message";
            app.overlay = "briefcase";
            app.navigate(p.path, "");
        }, input);
    }
    function submitPasswd() {
        const p = pw;
        if (!p.a || p.b.length < 4) return pw = Object.assign({}, p, { error: "Type the old password and a new one (4 characters or more)" });
        if (p.b !== p.c) return pw = Object.assign({}, p, { error: "The two new passwords aren't the same" });
        const input = p.a + "\n" + p.b + "\n";
        pw = Object.assign({}, p, { a: "", b: "", c: "", error: "Changing…" });
        call(["passwd", p.path], r => {
            if (!r || !r.ok) return pw = Object.assign({}, pw, { error: r ? r.error : "Couldn't change it" });
            close();
            app.say("The password was changed");
        }, input);
    }
    function openUpdate(path, names) {
        call(["plan", path].concat(names), r => {
            if (needUnlock(r, () => openUpdate(path, names))) return;
            if (!done(r)) return;
            if (!r.rows.length && !r.info.length) return app.say("Everything in “" + r.name + "” is up-to-date");
            plan = r;
            row = 0;
            mode = "update";
            app.overlay = "briefcase";
        });
    }
    readonly property var actionNames: ({ "to-original": "Replace →", "to-briefcase": "← Replace", skip: "Skip", "keep-both": "Keep both",
        "delete-briefcase": "Delete here too", "delete-original": "Delete there too", adopt: "Same", forget: "Forget" })
    readonly property var kindNames: ({ "newer-b": "Newer in Briefcase", "new-b": "New in Briefcase", "newer-o": "Newer at original",
        "new-o": "New at original", conflict: "Changed in both", "deleted-o": "Deleted at original: keep or delete?",
        "deleted-b": "Deleted in Briefcase: keep or delete?", "changed-b-deleted-o": "Changed here, deleted at original",
        "changed-o-deleted-b": "Changed at original, deleted here" })
    function cycleAction(i, d) {
        if (!plan || !plan.rows[i]) return;
        const r = plan.rows[i];
        const at = r.choices.indexOf(r.action);
        const next = r.choices[(at + (d || 1) + r.choices.length) % r.choices.length];
        const rows = plan.rows.slice();
        rows[i] = Object.assign({}, r, { action: next });
        plan = Object.assign({}, plan, { rows: rows });
    }
    function runUpdate() {
        const p = plan;
        close();
        runJob("update", ["apply", p.path, JSON.stringify({ rows: p.rows.map(r => ({ id: r.id, action: r.action, bsig: r.bsig, osig: r.osig, size: r.size })) })], (r, cancelled) => {
            if (!r || !r.ok) return app.say(r ? r.error : "Something went wrong", true);
            const n = (r.done || []).length;
            let t = cancelled || r.cancelled ? "Stopped after " + plural(n, "change") : n ? plural(n, "change") + " made" : "Nothing needed doing";
            if ((r.trashed || []).length) t += " · replaced versions are in the Trash";
            if ((r.replaced || []).length) t += " · replaced versions kept in .briefcase-replaced (no Trash on that shared folder)";
            const sk = (r.skipped || []).filter(s => s.why !== "kept both");
            if (sk.length) t += " · " + plural(sk.length, "item") + " skipped (" + sk[0].why + ")";
            app.say(t);
        });
    }
    // Customize: only what you change is saved, into the Briefcase's own list (so it travels with it).
    readonly property var iconChoices: ["briefcase", "satchel", "suitcase", "backpack", "box"]
    readonly property var colorChoices: ["accent", "ember", "red", "green", "blue", "purple", "gold", "slate"]
    readonly property var viewChoices: ["", "large", "grid", "small", "tiles", "compact", "details"]
    readonly property var sortChoices: ["", "name", "date", "size", "type"]
    readonly property var syncChoices: [["instant", 0, "Right away"], ["interval", 5, "Every 5 minutes"], ["interval", 15, "Every 15 minutes"],
        ["interval", 30, "Every 30 minutes"], ["interval", -1, "Every … minutes (custom)"], ["onlock", 0, "When I lock it"], ["off", 0, "Off (only Update All)"]]
    readonly property var lockChoices: [[0, "Never (only when I lock it)"], [1, "1 minute"], [5, "5 minutes"], [15, "15 minutes"], [30, "30 minutes"],
        [60, "1 hour"], [240, "4 hours"], [-1, "Custom (minutes)"]]
    function openCustomize(path) {
        call(["status", path], r => {
            if (!done(r)) return;
            const look = JSON.parse(JSON.stringify(r.look));
            cust = { path: r.path, locked: r.locked, orig: JSON.stringify(r.look), look: look,
                     syncCustom: look.autosync.mode === "interval" && [5, 15, 30].indexOf(look.autosync.minutes) < 0,
                     lockCustom: [0, 1, 5, 15, 30, 60, 240].indexOf(look.autolock.minutes) < 0 };
            row = 0;
            mode = "customize";
            app.overlay = "briefcase";
        });
    }
    function setCust(k, v) {
        const c = Object.assign({}, cust);
        c.look = Object.assign({}, c.look);
        c.look[k] = v;
        cust = c;
    }
    function setCustSub(k, sub, v) {
        const o = Object.assign({}, cust.look[k] || {});
        o[sub] = v;
        setCust(k, o);
    }
    function syncIndex() {
        const a = cust.look.autosync;
        if (a.mode === "interval") return cust.syncCustom ? 4 : Math.max(1, [0, 5, 15, 30].indexOf(a.minutes));
        return a.mode === "instant" ? 0 : a.mode === "onlock" ? 5 : 6;
    }
    function setSyncIndex(i) {
        const c = syncChoices[(i + syncChoices.length) % syncChoices.length];
        const o = Object.assign({}, cust.look.autosync, { mode: c[0] });
        if (c[1] > 0) o.minutes = c[1];
        const cc = Object.assign({}, cust, { syncCustom: c[1] < 0 });
        cc.look = Object.assign({}, cust.look, { autosync: o });
        cust = cc;
    }
    function lockIndex() {
        if (cust.lockCustom) return 7;
        return Math.max(0, [0, 1, 5, 15, 30, 60, 240].indexOf(cust.look.autolock.minutes));
    }
    function setLockIndex(i) {
        if (!cust.locked) return;
        const c = lockChoices[(i + lockChoices.length) % lockChoices.length];
        const o = Object.assign({}, cust.look.autolock);
        if (c[0] >= 0) o.minutes = c[0];
        const cc = Object.assign({}, cust, { lockCustom: c[0] < 0 });
        cc.look = Object.assign({}, cust.look, { autolock: o });
        cust = cc;
    }
    function saveCustomize() {
        const was = JSON.parse(cust.orig);
        const now = cust.look;
        const ch = {};
        for (const k of ["name", "color", "icon", "badge", "note", "view", "sort", "showInNav"])
            if (JSON.stringify(was[k]) !== JSON.stringify(now[k])) ch[k] = now[k];
        for (const k of ["autosync", "autolock"]) {
            const d = {};
            for (const s in now[k]) if (JSON.stringify((was[k] || {})[s]) !== JSON.stringify(now[k][s])) d[s] = now[k][s];
            if (Object.keys(d).length) ch[k] = d;
        }
        const path = cust.path;
        close();
        if (!Object.keys(ch).length) return;
        call(["customize", path, JSON.stringify(ch)], r => {
            if (!done(r, "Saved")) return;
            refreshKnown();
            app.refreshPlaces();
            if (r.path !== path) app.navigate(r.path, "");
            else { refresh(); app.reload(); }
        });
    }
    function close() {
        mode = "";
        plan = null;
        pick = null;
        cust = null;
        msg = null;
        pw = { a: "", b: "", c: "", remember: false, error: "", path: "", name: "", then: null };
        if (app.overlay === "briefcase") app.closeOverlay();
    }
    onModeChanged: {
        if (mode !== "" && app.overlay !== "briefcase") app.overlay = "briefcase";
        else if (mode === "" && app.overlay === "briefcase") app.closeOverlay();
    }
}
