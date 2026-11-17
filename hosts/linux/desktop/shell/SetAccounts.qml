// Hearth Settings › Accounts › Online accounts (1.14, docs/ONLINE-ACCOUNTS.md): sign in once to Nextcloud, Google,
// Microsoft, or a plain CalDAV/CardDAV, WebDAV or IMAP server, and Hearth's apps use the account (the calendar widget
// and Hearth Time its calendars, Hearth Contacts its address books, Hearth Files its files, mail apps its IMAP).
// Everything goes through hearth_accounts.py: passwords reach it on its stdin (SetJob's input), never on a command
// line, and it keeps them only in the system keyring. This page writes no settings file itself.
// Opened at a part: `settings page online-accounts:add-nextcloud` (also add-google, add-microsoft, add-dav,
// add-webdav, add-imap) opens that form.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Online accounts"
    property var accounts: []
    property var prov: ({})
    property bool loaded: false
    property string adding: ""          // the open form: nextcloud | google | microsoft | dav | webdav | imap
    property string message: ""
    property bool messageBad: false
    property string flowLine: ""        // the browser sign-in's progress (Nextcloud, Google, Microsoft)
    property string flowLogin: ""       // the sign-in page's address, if the browser didn't open
    readonly property bool flowing: flow.running
    readonly property string helperPath: decodeURIComponent(Qt.resolvedUrl("hearth_accounts.py").toString().replace(/^file:\/\//, ""))

    readonly property var kinds: [
        { id: "nextcloud", icon: "cloud", title: "Nextcloud", sub: "Calendar, contacts and files · sign in in your browser" },
        { id: "google", icon: "globe", title: "Google", sub: "Calendar, contacts and Gmail · sign in in your browser" },
        { id: "microsoft", icon: "window", title: "Microsoft", sub: "Outlook calendar and mail · sign in in your browser" },
        { id: "dav", icon: "calendar", title: "CalDAV / CardDAV", sub: "Any calendar and contacts server (Fastmail, iCloud, Radicale, Baïkal…)" },
        { id: "webdav", icon: "folder", title: "WebDAV files", sub: "Files on a WebDAV server, in Hearth Files" },
        { id: "imap", icon: "chat", title: "IMAP mail", sub: "Any mail server, for Hearth's mail apps" }
    ]
    readonly property var uses: [
        { id: "calendar", icon: "calendar", title: "Calendar", sub: "Events in the calendar widget, the clock and Hearth Time" },
        { id: "contacts", icon: "users", title: "Contacts", sub: "Address books in Hearth Contacts" },
        { id: "files", icon: "folder", title: "Files", sub: "Opens in Hearth Files' left menu" },
        { id: "mail", icon: "chat", title: "Mail", sub: "Mail apps can use it (IMAP)" }
    ]
    function kindIcon(k) { const x = kinds.find(o => o.id === k); return x ? x.icon : "user"; }

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        job.run(["list"], r => { page.accounts = r.accounts || []; page.loaded = true; }, e => { page.loaded = true; page.say(e, true); });
        job.run(["providers"], r => page.prov = r || {});
    }
    // Opened at a form (`online-accounts:add-nextcloud`), now or when Settings is opened here again.
    function takeAnchor() {
        const a = SettingsNav.pageAnchor;
        if (a.startsWith("add-") && kinds.find(o => o.id === a.slice(4))) { SettingsNav.pageAnchor = ""; adding = ""; openForm(a.slice(4)); }
    }
    Component.onCompleted: { load(); takeAnchor(); }
    onVisibleChanged: if (visible) { load(); takeAnchor(); }
    Connections { target: SettingsNav; function onPageAnchorChanged() { page.takeAnchor(); } }
    function openForm(k) {
        if (flowing) return;
        adding = adding === k ? "" : k;
        say("");
        if (adding === "nextcloud") Qt.callLater(() => ncServer.focusInput());
        if (adding === "dav" || adding === "webdav") Qt.callLater(() => davServer.focusInput());
        if (adding === "imap") Qt.callLater(() => imapHost.focusInput());
        if (adding !== "") toForm.restart();
    }
    // The form in view: it opens under the list of kinds.
    Timer { id: toForm; interval: 120; onTriggered: page.contentY = Math.max(0, Math.min(page.contentHeight - page.height, form.y - 24)) }
    function added(r, what) {
        adding = ""; flowLine = ""; flowLogin = "";
        davServer.text = davUser.text = davPw.text = imapHost.text = imapUser.text = imapPw.text = ncServer.text = "";
        imapPort.text = "993";
        const a = r.account || {};
        say((a.name || what) + " is added. Pick below what Hearth uses it for.");
        load();
    }

    SetJob { id: job; helper: "hearth_accounts.py" }

    // The browser sign-ins run until you finish there (or Cancel): their own process, so the list still loads.
    Process {
        id: flow
        property string what: ""
        property var result: null
        property string err: ""
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: l => {
                const m = l.match(/^progress: ([\d.]+) ?(.*)$/);
                if (m) { page.flowLine = m[2]; return; }
                if (l.startsWith("error: ")) { flow.err = l.slice(7); return; }
                if (l.startsWith("{")) {
                    try { const v = JSON.parse(l); if (v.waiting) page.flowLogin = v.login || ""; else flow.result = v; } catch (e) {}
                }
            }
        }
        onExited: code => {
            if (flow.result && flow.result.account) page.added(flow.result, flow.what);
            else if (flow.err) { page.flowLine = ""; page.say(flow.err, true); }
            else if (code !== 0) { page.flowLine = ""; page.say("Sign-in stopped.", false); }
            flow.result = null; flow.err = "";
        }
    }
    function startFlow(args, what) {
        if (flow.running) return;
        say(""); flowLine = "Starting…"; flowLogin = "";
        flow.what = what;
        flow.command = ["python3", helperPath].concat(args);
        flow.running = true;
    }
    function cancelFlow() { flow.running = false; flowLine = ""; flowLogin = ""; say("Sign-in cancelled."); }

    function addPassword(cmd, details, what) {
        job.run([cmd], r => page.added(r, what), e => page.say(e, true), JSON.stringify(details) + "\n", "adding");
    }
    function setUse(acct, what, on) {
        job.run(["use", acct.id, what, on ? "on" : "off"], r => page.load(), e => page.say(e, true));
    }
    function remove(acct) {
        job.run(["remove", acct.id], r => { page.say(acct.name + " is removed from this device (and its password from the keyring)."); page.load(); }, e => page.say(e, true));
    }
    function openFiles(acct) {
        say("Opening " + acct.name + "'s files…");
        job.run(["mount", acct.id], r => { page.say(""); if (r.path) Quickshell.execDetached(["hearth-filemanager", r.path]); }, e => page.say(e, true), "", "mount");
    }

    SetBanner {
        visible: page.loaded && page.prov.secretService === false
        kind: "bad"; icon: "lock"
        title: "No keyring on this device"
        sub: "Hearth keeps online accounts' passwords only in the system keyring (GNOME Keyring), and it isn't running. Accounts can't be added until it is."
    }

    // ---------------------------------------------------------------- your accounts
    SetHeader { text: "Your accounts"; Layout.topMargin: 0 }
    SetRow {
        visible: page.loaded && page.accounts.length === 0
        interactive: false
        icon: "cloud"; title: "No online accounts yet"
        sub: "Sign in once below, and Hearth's calendar, contacts, files and mail use the account. Passwords stay in this device's keyring."
    }
    Repeater {
        model: page.accounts
        delegate: SetExpander {
            id: acctCard
            required property var modelData
            readonly property var acct: modelData
            Layout.fillWidth: true
            open: page.accounts.length === 1
            icon: page.kindIcon(acct.kind)
            title: acct.name
            sub: acct.kindLabel + (acct.user && acct.name.indexOf(acct.user) < 0 ? " · " + acct.user : "")
                 + " · used for " + (page.uses.filter(u => acct.use[u.id]).map(u => u.title.toLowerCase()).join(", ") || "nothing yet")
            chip: "Signed in"; chipKind: "ok"
            Repeater {
                model: page.uses.filter(u => acctCard.acct.can[u.id])
                delegate: SetSwitch {
                    required property var modelData
                    Layout.fillWidth: true
                    inset: true
                    icon: modelData.icon; title: modelData.title; sub: modelData.sub
                    checked: !!acctCard.acct.use[modelData.id]
                    onToggled: v => page.setUse(acctCard.acct, modelData.id, v)
                }
            }
            SetRow {
                visible: !!acctCard.acct.use.files
                inset: true
                icon: "open"; title: "Open in Hearth Files"; sub: "Mounted with gvfs (davs://), like a drive"
                onActivated: page.openFiles(acctCard.acct)
            }
            SetRow {
                inset: true; danger: true
                icon: "trash"; title: "Remove this account"
                sub: "Signs this device out: its password or token is taken out of the keyring"
                onActivated: page.remove(acctCard.acct)
            }
        }
    }

    // ---------------------------------------------------------------- adding
    SetHeader { text: "Add an account" }
    Repeater {
        model: page.kinds
        delegate: SetRow {
            required property var modelData
            Layout.fillWidth: true
            icon: modelData.icon; title: modelData.title; sub: modelData.sub
            chevron: true; chevronIcon: page.adding === modelData.id ? "chevu" : "chevd"
            chip: (modelData.id === "google" || modelData.id === "microsoft") && page.prov[modelData.id] && !page.prov[modelData.id].ready ? "Needs setting up" : ""
            chipKind: "off"
            onActivated: page.openForm(modelData.id)
        }
    }

    // The open form (one at a time), under the list of kinds.
    ColumnLayout {
        id: form
        visible: page.adding !== ""
        Layout.fillWidth: true
        Layout.topMargin: 6
        spacing: 8

        SetHeader { text: { const k = page.kinds.find(o => o.id === page.adding); return k ? "Add " + k.title : ""; } }

        // Nextcloud: Login Flow v2 (your Nextcloud's own sign-in page; it gives this device an app password).
        ColumnLayout {
            visible: page.adding === "nextcloud"
            Layout.fillWidth: true
            spacing: 8
            SetField { id: ncServer; placeholder: "Your Nextcloud's address (cloud.example.com)"; onSubmitted: t => ncGo.activated() }
            Text {
                Layout.fillWidth: true
                text: "Your browser opens Nextcloud's own sign-in page (two-step sign-in works there). Nextcloud then gives this device its own app password, kept in the keyring; you can revoke it any time in Nextcloud › Settings › Security."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
            }
            RowLayout {
                spacing: 8
                SetButton { id: ncGo; visible: !page.flowing; primary: true; icon: "globe"; text: "Sign in in the browser"
                    onActivated: { if (ncServer.text.trim() === "") return page.say("Type your Nextcloud's address.", true); page.startFlow(["add-nextcloud", ncServer.text.trim()], "Nextcloud"); } }
                SetButton { visible: page.flowing; ghost: true; icon: "close"; text: "Cancel"; onActivated: page.cancelFlow() }
            }
        }

        // Google and Microsoft: OAuth with PKCE in the browser, or what the owner still has to set up.
        ColumnLayout {
            visible: page.adding === "google" || page.adding === "microsoft"
            Layout.fillWidth: true
            spacing: 8
            readonly property var p: page.prov[page.adding] || ({})
            Text {
                Layout.fillWidth: true
                text: parent.p.ready
                    ? "Your browser opens " + parent.p.label + "'s sign-in page. Hearth never sees your password: " + parent.p.label + " gives this device a sign-in token, kept in the keyring. You can remove its access any time in your " + parent.p.label + " account's security settings."
                    : (parent.p.why || "This sign-in isn't set up on this device yet.") + " Until then, " + (page.adding === "google" ? "Google Calendar works as CalDAV with an app password." : "Outlook mail works as IMAP if your account allows app passwords.")
                color: parent.p.ready ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
            }
            RowLayout {
                spacing: 8
                SetButton { visible: !page.flowing && !!parent.parent.p.ready; primary: true; icon: "globe"; text: "Sign in in the browser"
                    onActivated: page.startFlow(["add-oauth", page.adding], page.adding === "google" ? "Google" : "Microsoft") }
                SetButton { visible: page.flowing; ghost: true; icon: "close"; text: "Cancel"; onActivated: page.cancelFlow() }
            }
        }

        // The browser sign-ins' progress.
        SetRow {
            visible: page.flowing && page.flowLine !== ""
            interactive: false
            icon: "sync"; title: page.flowLine
            sub: page.flowLogin !== "" ? "If the browser didn't open, go to " + page.flowLogin : "Waiting for the browser…"
        }

        // CalDAV / CardDAV and WebDAV: the address, user name and password.
        ColumnLayout {
            visible: page.adding === "dav" || page.adding === "webdav"
            Layout.fillWidth: true
            spacing: 8
            SetField { id: davServer; placeholder: page.adding === "webdav" ? "WebDAV address (https://…)" : "Server address (https://dav.example.com)"; onSubmitted: t => davUser.focusInput() }
            SetField { id: davUser; placeholder: "User name"; onSubmitted: t => davPw.focusInput() }
            SetField { id: davPw; placeholder: "Password (an app password if your provider has them)"; secret: true; onSubmitted: t => davGo.activated() }
            SetButton {
                id: davGo; primary: true; icon: "key"
                text: job.what === "adding" ? "Signing in…" : "Sign in"
                onActivated: page.addPassword(page.adding === "webdav" ? "add-webdav" : "add-dav",
                                              { server: davServer.text.trim(), user: davUser.text.trim(), password: davPw.text },
                                              page.adding === "webdav" ? "WebDAV" : "The account")
            }
        }

        // IMAP mail.
        ColumnLayout {
            visible: page.adding === "imap"
            Layout.fillWidth: true
            spacing: 8
            SetField { id: imapHost; placeholder: "Mail server (imap.example.com)"; onSubmitted: t => imapUser.focusInput() }
            SetField { id: imapPort; placeholder: "Port"; text: "993" }
            SetField { id: imapUser; placeholder: "User name (often your e-mail address)"; onSubmitted: t => imapPw.focusInput() }
            SetField { id: imapPw; placeholder: "Password"; secret: true; onSubmitted: t => imapGo.activated() }
            SetButton {
                id: imapGo; primary: true; icon: "key"
                text: job.what === "adding" ? "Signing in…" : "Sign in"
                onActivated: page.addPassword("add-imap", { host: imapHost.text.trim(), port: parseInt(imapPort.text, 10) || 993, user: imapUser.text.trim(), password: imapPw.text, ssl: true }, "Mail")
            }
        }
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.topMargin: 6
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
