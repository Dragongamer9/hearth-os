// Hearth Files' 1.14 extras (Dolphin's features), kept apart from FileManager.qml, which calls in here:
//   tags (Tags ›, a Tags list in the left menu, finding what carries one; user.xdg.tags, the same as Dolphin's),
//   git status badges (on items in a git repository, and the branch in the status bar),
//   Act as administrator (changes outside your home folder through pkexec hearth-files-admin, its own polkit action: polkit asks for an
//     administrator's password; never the parts that start the PC; this window only, never saved, a red bar while on),
//   disc images (Open as a drive / Close the drive), phones (MTP) and cameras (PTP) under This PC,
//   apps' right-click actions (KDE service menus), Share on your network… (Properties › Sharing),
//   Send to TV (a video or a song plays on your Hearth TV, through Hearth Link).
// The work is hearth-files' (hearth_files_extras.py); this only asks it and shows what it said.
import QtQuick
import Quickshell

Scope {
    id: x
    property var app: null

    // ------------------------------------------------ Act as administrator
    property bool admin: false
    readonly property string adminHelper: "/usr/libexec/nimbus/hearth-files-admin"   // its own polkit action
    readonly property var adminOps: ["mkdir", "new-file", "rename", "delete", "copy", "move", "chmod"]
    function inHome(p) { const h = app ? app.home : ""; return !!p && !!h && (p === h || p.indexOf(h + "/") === 0); }
    function systemPath(p) { return !!p && p.indexOf("/") === 0 && !inHome(p) && p.indexOf("/run/media/") !== 0 && p.indexOf("/media/") !== 0 && p.indexOf("/tmp") !== 0 && p.indexOf("/run/user/") !== 0; }
    readonly property bool canAdmin: !!app && !!app.listing && !app.onHome && !app.onSpecial && !app.inTrash && systemPath(app.listing.path)
    readonly property bool adminHere: admin && canAdmin
    function setAdmin(on) {
        if (!on) { admin = false; app.say("Not acting as administrator any more"); return app.reload(); }
        app.confirm("Act as administrator?",
            "You can change system files here, and system files can break apps or Hearth itself. Every change asks for an administrator's password (remembered for a few minutes). Hearth never lets Files change what starts the PC.",
            "Act as administrator", () => { x.admin = true; app.say("Acting as administrator in system folders"); app.reload(); });
    }
    // The command for hearth-files ARGS, or null: as administrator, a change outside your home goes through pkexec.
    function route(args) {
        if (!admin || !args || !args.length || adminOps.indexOf(args[0]) < 0) return null;
        const paths = args.slice(1).filter(a => typeof a === "string" && a.indexOf("/") === 0);
        if (!paths.some(p => systemPath(p))) return null;
        return ["pkexec", adminHelper, "files-" + args[0]].concat(args.slice(1));
    }
    // Move to Trash, as administrator in a system folder: deleted for good (there's no Trash for system files).
    function adminDelete(t) {
        app.confirm(t.length === 1 ? "Delete “" + app.base(t[0]) + "” for good?" : "Delete " + t.length + " items for good?",
            "As administrator, system files have no Trash: this can't be undone.", "Delete", () => {
                app.call(["delete"].concat(t), (r, err) => {
                    if (err) return app.say(err, true);
                    app.say(t.length === 1 ? "Deleted “" + app.base(t[0]) + "”" : "Deleted " + t.length + " items");
                    app.marked = ({});
                    app.reload();
                });
            });
    }

    // ------------------------------------------------ git status
    property var git: ({ repo: null, status: {} })
    property string gitFor: ""
    readonly property bool gitOn: !(app && app.prefs && app.prefs.gitBadges === false)
    function gitOf(e) { return e && git.repo && e.path && app.parentOf(e.path) === gitFor ? (git.status || {})[e.name] || "" : ""; }
    readonly property var gitLook: ({
        modified: { mark: "M", tip: "Changed" }, added: { mark: "+", tip: "Added" }, untracked: { mark: "?", tip: "Not in git yet" },
        deleted: { mark: "−", tip: "Deleted" }, conflict: { mark: "!", tip: "In conflict" }, ignored: { mark: "·", tip: "Ignored by git" },
        clean: { mark: "✓", tip: "Up to date" } })
    function gitColor(s) {
        return s === "conflict" ? Theme.danger : s === "modified" || s === "deleted" ? "#e0a23a" : s === "added" ? "#4fbf7f"
             : s === "untracked" ? "#5aa7e6" : s === "clean" ? Qt.rgba(0.4, 0.75, 0.5, 0.85) : Theme.muted;
    }
    readonly property string gitLine: {
        if (!git.repo || gitFor !== (app && app.listing ? app.listing.path : "")) return "";
        const st = git.status || {};
        const changed = Object.keys(st).filter(k => ["modified", "added", "deleted", "conflict", "untracked"].indexOf(st[k]) >= 0).length;
        return "⎇ " + (git.branch || "git") + (changed ? " · " + changed + (changed === 1 ? " change here" : " changes here") : " · up to date") + (git.ahead ? " · " + git.ahead + " to push" : "");
    }

    // ------------------------------------------------ tags
    property var tags: []                 // [{name, count}] in your home folder
    property string pendingTag: ""
    readonly property var tagColors: ["#e5484d", "#f5a524", "#46a758", "#3e9bdb", "#8e4ec6", "#d6409f", "#12a594", "#978365"]
    function tagColor(t) { let h = 0; for (let i = 0; i < t.length; i++) h = (h * 31 + t.charCodeAt(i)) >>> 0; return tagColors[h % tagColors.length]; }
    function refreshTags() { app.call(["tag-list"], (r, err) => { if (r && r.tags) { x.tags = r.tags; app.places = app.buildNav(app.rawPlaces); } }); }
    function openTag(t) {
        if (!app.listing || app.onSpecial || app.onHome) { pendingTag = t; return app.navigate(app.home, ""); }
        app.runTagSearch(t);
    }
    function tagMenu(t) {
        const ents = t.map(p => app.entries.find(e => e.path === p)).filter(e => e);
        const all = name => ents.length && ents.every(e => (e.tags || []).indexOf(name) >= 0);
        const names = tags.map(g => g.name);
        for (const e of ents) for (const n of e.tags || []) if (names.indexOf(n) < 0) names.push(n);
        const items = names.slice(0, 16).map(n => ({ id: (all(n) ? "x-untag:" : "x-tag:") + n, label: (all(n) ? "✓  " : "     ") + n, dot: tagColor(n) }));
        items.push({ sep: true }, { id: "x-newtag", label: "New tag…", icon: "plus" });
        if (ents.some(e => (e.tags || []).length)) items.push({ id: "x-cleartags", label: "Remove all tags", icon: "close" });
        app.showMenu("Tags", items, app.lastMenuX, app.lastMenuY);
    }
    function setTags(t, add, remove) {
        const args = ["tag-set"];
        for (const a of add) args.push("--add", a);
        for (const r of remove) args.push("--remove", r);
        app.call(args.concat(t), (r, err) => {
            if (err) return app.say(err, true);
            app.say(add.length ? "Tagged “" + add.join(", ") + "”" : "Tags removed");
            app.reload();
            refreshTags();
        });
    }

    // ------------------------------------------------ phones and cameras, disc images
    property var portable: []
    function refreshPortable() {
        app.call(["portable"], (r, err) => {
            const n = r && r.devices ? r.devices : [];
            if (JSON.stringify(n) !== JSON.stringify(x.portable)) { x.portable = n; app.places = app.buildNav(app.rawPlaces); }
        });
    }
    function openPortable(p) {
        const d = p.dev || {};
        if (d.path) return app.navigate(d.path, "");
        app.say("Opening “" + d.label + "”… on a phone, unlock it and choose File transfer");
        app.call(["portable-mount", d.uri], (r, err) => {
            if (err) return app.ask("Couldn't open “" + d.label + "”", err, "OK", () => {});
            refreshPortable();
            if (r && r.path) app.navigate(r.path, "");
        });
    }
    function isImage(e) { return !!e && !e.dir && /\.(iso|img)$/i.test(e.name); }

    // ------------------------------------------------ apps' right-click actions (service menus)
    property var svc: ({ key: "", items: [] })
    function svcKey(t) { return t.join("\n"); }
    Timer {
        id: svcLater; interval: 250
        onTriggered: {
            if (!x.app || !x.app.shown || x.app.onSpecial || x.app.onHome) return;
            const t = x.app.targets();
            const k = x.svcKey(t);
            if (!t.length || k === x.svc.key) return;
            x.app.call(["service-menus"].concat(t.slice(0, 50)), (r, err) => { x.svc = { key: k, items: r && r.actions ? r.actions : [] }; });
        }
    }
    function selectionChanged() { svcLater.restart(); refreshTvs(); }

    // ------------------------------------------------ Send to TV (a video or a song plays on your Hearth TV; docs/HEARTH-TV.md)
    property var tvs: []                  // [{id, name, online}]: Hearth TVs paired with this PC (Hearth Link)
    property double tvsAt: 0
    function refreshTvs() {
        if (!app || Date.now() - tvsAt < 30000) return;
        tvsAt = Date.now();
        app.call(["tv-list"], (r, err) => { x.tvs = r && r.tvs ? r.tvs : []; });
    }
    function isMedia(e) { return !!e && !e.dir && !e.remote && e.path.indexOf("/") === 0 && /\.(mp4|m4v|mkv|webm|mov|avi|ts|mp3|flac|ogg|oga|opus|m4a|wav|aac)$/i.test(e.name); }
    function sendToTv(e, tv) {
        const name = (tvs.find(t => t.id === tv) || tvs[0] || { name: "the TV" }).name;
        app.say("Sending “" + e.name + "” to " + name + "…");
        app.call(["tv-send", e.path].concat(tv ? [tv] : []), (r, err) => err ? app.say(err, true) : app.say("Playing “" + e.name + "” on " + (r && r.tv || name)));
    }

    // ------------------------------------------------ the folder shown, the left menu
    function folderLoaded(r) {
        if (!r || !r.path || r.error) { git = { repo: null, status: {} }; gitFor = ""; return; }
        if (gitOn) app.call(["git-status", r.path], (g, err) => { if (g && x.app.listing && x.app.listing.path === r.path) { x.git = g; x.gitFor = r.path; } });
        selectionChanged();
        if (pendingTag !== "") { const t = pendingTag; pendingTag = ""; Qt.callLater(() => x.app.runTagSearch(t)); }
    }
    function refresh() { refreshTags(); refreshPortable(); }
    // Tags after your folders; phones and cameras under This PC.
    function tagPlaces() { return tags.slice(0, 12).map(g => ({ id: "tag:" + g.name, label: g.name, path: "", kind: "tag", tag: g.name, section: "folders", sub: g.count + "", dot: tagColor(g.name) })); }
    function portablePlaces() {
        return portable.map(d => ({ id: "portable:" + d.uri, label: d.label, path: d.path || "", kind: d.kind === "camera" ? "camera" : "phone", section: "pc",
                                     child: true, depth: 1, dev: d, sub: d.mounted ? "" : "Select to open" }));
    }

    // ------------------------------------------------ the right-click menu
    function menuItems(e, n, t) {
        const out = [];
        if (!app || app.picking || app.inTrash) return out;
        if (n === 1 && isImage(e)) out.push({ id: "x-isomount", label: "Open as a drive", icon: "drive" });
        if (n === 1 && e && e.dir && e.path.indexOf("/run/media/") === 0 && /\/loop|iso/i.test(e.path)) out.push({ id: "x-isoclose", label: "Close the drive", icon: "close" });
        if (n && app.writable && !e.remote) out.push({ id: "x-tags", label: "Tags  ›", icon: "pin" });
        if (n === 1 && e && e.dir && !e.remote && e.path.indexOf("/") === 0) out.push({ id: "x-share", label: "Share on your network…", icon: "share" });
        if (n === 1 && isMedia(e)) {
            if (tvs.length === 1) out.push({ id: "x-tv:" + tvs[0].id, label: "Send to TV (" + tvs[0].name + ")", icon: "display" });
            else for (let i = 0; i < Math.min(6, tvs.length); i++) out.push({ id: "x-tv:" + tvs[i].id, label: "Send to " + tvs[i].name, icon: "display", group: "Send to TV" });
        }
        if (svc.key === svcKey(t)) for (let i = 0; i < Math.min(8, svc.items.length); i++) {
            const a = svc.items[i];
            out.push({ id: "x-svc:" + i, label: (a.submenu ? a.submenu + ": " : "") + a.label, icon: "bolt", group: "Actions" });
        }
        return out;
    }
    function folderMenuItems() {
        const out = [];
        if (canAdmin && !app.picking) out.push({ id: "x-admin", label: admin ? "Stop acting as administrator" : "Act as administrator", icon: "shield", danger: !admin });
        if (git.repo && gitFor === (app.listing ? app.listing.path : "")) out.push({ id: "x-gitbadges", label: (gitOn ? "✓  " : "     ") + "Show git status", icon: "check" });
        return out;
    }
    function run(id, t, e) {
        if (id === "x-admin") return setAdmin(!admin);
        if (id === "x-tags") return tagMenu(t);
        if (id.startsWith("x-tag:")) return setTags(t, [id.slice(6)], []);
        if (id.startsWith("x-untag:")) return setTags(t, [], [id.slice(8)]);
        if (id === "x-cleartags") {
            const names = [];
            for (const p of t) { const en = app.entries.find(q => q.path === p); for (const n of (en && en.tags) || []) if (names.indexOf(n) < 0) names.push(n); }
            return setTags(t, [], names);
        }
        if (id === "x-newtag") return app.askText("New tag", "", "Tag", 0, text => { const n = text.trim(); if (n) x.setTags(t, [n], []); });
        if (id === "x-share") { if (e) app.propsAt([e.path], "sharing"); return; }
        if (id.startsWith("x-tv:")) { if (e) sendToTv(e, id.slice(5)); return; }
        if (id === "x-gitbadges") {
            const on = !gitOn;
            app.prefs = Object.assign({}, app.prefs, { gitBadges: on });
            app.call(["prefs-set", JSON.stringify({ gitBadges: on })], (r, err) => { if (err) app.say(err, true); });
            app.say(on ? "Showing git status on items" : "Git status is hidden");
            return app.reload();
        }
        if (id === "x-isomount") {
            app.say("Opening “" + e.name + "”…");
            return app.call(["iso-mount", e.path], (r, err) => { if (err) return app.say(err, true); app.refreshPlaces(); if (r && r.path) app.navigate(r.path, ""); });
        }
        if (id === "x-isoclose") return app.call(["iso-unmount", e.path], (r, err) => { if (err) return app.say(err, true); app.say("Closed it"); app.refreshPlaces(); app.navigate(app.home, ""); });
        if (id.startsWith("x-svc:")) {
            const a = svc.items[parseInt(id.slice(6), 10)];
            if (!a) return;
            return app.call(["service-run", a.id].concat(t), (r, err) => err ? app.say(err, true) : app.say(a.label + "…"));
        }
    }
}
