// The Briefcase's dialogs inside Hearth Files' window (Briefcase.qml keeps what they show): Update All's list of
// actions, Unlock / Lock this Briefcase / Change password, Customize, choosing a Briefcase, and a job's progress.
// Keyboard and controller: Up / Down move, Left / Right (or A) change a choice, A presses, B cancels; text boxes take
// the on-screen keyboard (Y, or it opens by itself when the dialog was opened with the controller).
import QtQuick
import QtQuick.Layouts

Item {
    id: dlg
    required property var bc
    readonly property var app: bc.app
    readonly property string mode: bc.mode
    visible: mode !== "" && app.overlay === "briefcase"
    anchors.fill: parent

    // Rows the keyboard / controller walks through, per dialog. Each: {t: "field"|"check"|"choice"|"button"|"row", id}
    readonly property var rows: {
        if (mode === "password") return [{ t: "field", id: "a" }, { t: "check", id: "remember" }, { t: "button", id: "cancel" }, { t: "button", id: "ok" }];
        if (mode === "setlock") return [{ t: "field", id: "a" }, { t: "field", id: "b" }, { t: "check", id: "remember" }, { t: "button", id: "cancel" }, { t: "button", id: "ok" }];
        if (mode === "passwd") return [{ t: "field", id: "a" }, { t: "field", id: "b" }, { t: "field", id: "c" }, { t: "button", id: "cancel" }, { t: "button", id: "ok" }];
        if (mode === "pick" && bc.pick) return bc.pick.list.map((x, i) => ({ t: "row", id: i })).concat([{ t: "button", id: "cancel" }]);
        if (mode === "update" && bc.plan) return bc.plan.rows.map((x, i) => ({ t: "row", id: i })).concat([{ t: "button", id: "cancel" }, { t: "button", id: "ok" }]);
        if (mode === "customize") return [{ t: "field", id: "name" }, { t: "choice", id: "color" }, { t: "choice", id: "icon" }, { t: "field", id: "badge" },
            { t: "field", id: "note" }, { t: "choice", id: "view" }, { t: "choice", id: "sort" }, { t: "choice", id: "autosync" },
            { t: "field", id: "syncmin" }, { t: "choice", id: "autolock" }, { t: "field", id: "lockmin" }, { t: "check", id: "onSleep" },
            { t: "check", id: "onEject" }, { t: "check", id: "showInNav" }, { t: "button", id: "cancel" }, { t: "button", id: "ok" }]
            .filter(r => (r.id !== "syncmin" || (bc.cust && bc.cust.syncCustom)) && (r.id !== "lockmin" || (bc.cust && bc.cust.lockCustom && bc.cust.locked)));
        if (mode === "job") return [{ t: "button", id: "cancel" }];
        if (mode === "message") return [{ t: "button", id: "ok" }];
        return [];
    }
    readonly property var at: rows[Math.min(bc.row, rows.length - 1)] || null
    function lit(id) { return !!at && at.id === id; }

    function focusRow() {
        const r = at;
        const f = r && r.t === "field" ? fieldFor(r.id) : null;
        if (f) f.focusIn();
        else if (app.ui) app.ui.refocus();
    }
    function fieldFor(id) {
        if (mode === "customize") return ({ name: custName, badge: custBadge, note: custNote, syncmin: syncMin, lockmin: lockMin })[id] || null;
        return ({ a: pwA, b: pwB, c: pwC })[id] || null;
    }
    onModeChanged: Qt.callLater(focusRow)
    Connections { target: dlg.bc; function onRowChanged() { Qt.callLater(dlg.focusRow); } }

    function move(d) { bc.row = Math.max(0, Math.min(rows.length - 1, bc.row + d)); }
    function press(r, d) {
        if (!r) return;
        if (r.t === "row" && mode === "pick") return bc.pickChoose(r.id);
        if (r.t === "row" && mode === "update") return bc.cycleAction(r.id, d || 1);
        if (r.t === "check") return toggle(r.id);
        if (r.t === "choice") return choice(r.id, d || 1);
        if (r.t === "field") { if (d === undefined) return ok(); return; }
        if (r.id === "cancel") return mode === "job" ? bc.cancelJob() : bc.close();
        if (r.id === "ok") return ok();
    }
    function ok() {
        if (mode === "password") bc.submitPassword();
        else if (mode === "setlock") bc.submitSetLock();
        else if (mode === "passwd") bc.submitPasswd();
        else if (mode === "update") bc.runUpdate();
        else if (mode === "customize") bc.saveCustomize();
        else if (mode === "message") bc.close();
        else if (mode === "pick" && at && at.t === "row") bc.pickChoose(at.id);
    }
    function toggle(id) {
        if (id === "remember") bc.pw = Object.assign({}, bc.pw, { remember: !bc.pw.remember });
        else if (id === "onSleep" || id === "onEject") { if (bc.cust.locked) bc.setCustSub("autolock", id, !bc.cust.look.autolock[id]); }
        else if (id === "showInNav") bc.setCust("showInNav", !bc.cust.look.showInNav);
    }
    function cyc(list, v, d) { const i = list.indexOf(v); return list[((i < 0 ? 0 : i) + d + list.length) % list.length]; }
    function choice(id, d) {
        const l = bc.cust.look;
        if (id === "color") bc.setCust("color", cyc(bc.colorChoices, l.color, d));
        else if (id === "icon") bc.setCust("icon", cyc(bc.iconChoices, l.icon, d));
        else if (id === "view") bc.setCust("view", cyc(bc.viewChoices, l.view, d));
        else if (id === "sort") bc.setCust("sort", cyc(bc.sortChoices, l.sort, d));
        else if (id === "autosync") bc.setSyncIndex(bc.syncIndex() + d);
        else if (id === "autolock") bc.setLockIndex(bc.lockIndex() + d);
    }
    // From FileManager's key handler (overlay "briefcase") and the text boxes. True when the key was used.
    function key(e) {
        const up = e.key === Qt.Key_Up || e.key === Qt.Key_Backtab;
        const down = e.key === Qt.Key_Down || e.key === Qt.Key_Tab;
        const okKey = e.key === Qt.Key_Return || e.key === Qt.Key_Enter || (e.key === Qt.Key_Space && !(at && at.t === "field"));
        const back = e.key === Qt.Key_Escape || (e.key === Qt.Key_Backspace && !(at && at.t === "field"));
        const lr = e.key === Qt.Key_Left ? -1 : e.key === Qt.Key_Right ? 1 : 0;
        if (up) move(-1);
        else if (down) move(1);
        else if (lr && at && at.t === "button") {
            const i = rows.indexOf(at);
            const j = i + lr;
            if (rows[j] && rows[j].t === "button") bc.row = j;
        } else if (lr && at && at.t !== "field") press(at, lr);
        else if (okKey) press(at);
        else if (back) { if (mode === "job") bc.cancelJob(); else bc.close(); }
        else return false;
        e.accepted = true;
        return true;
    }

    readonly property string title: {
        if (mode === "password") return "Unlock “" + bc.pw.name + "”";
        if (mode === "setlock") return "Lock “" + bc.pw.name + "” with a password";
        if (mode === "passwd") return "Change the password of “" + bc.pw.name + "”";
        if (mode === "pick") return bc.pick ? bc.pick.title : "";
        if (mode === "update") return bc.plan ? "Update “" + bc.plan.name + "”" : "";
        if (mode === "customize") return "Customize “" + (bc.cust ? bc.cust.look.name || app.base(bc.cust.path) : "") + "”";
        if (mode === "job" && bc.job) return ({ update: "Updating…", add: "Adding to the Briefcase…", send: "Sending the Briefcase…", lock: "Encrypting and checking…",
            unlock: "Decrypting and checking…", move: "Moving…", copy: "Copying…", swap: "Swapping…", sync: "Syncing…" })[bc.job.verb] || "Working…";
        if (mode === "message" && bc.msg) return bc.msg.title;
        return "";
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: Math.min(parent.width - 48, dlg.mode === "update" || dlg.mode === "customize" ? 760 : 560)
        height: Math.min(parent.height - 48, col.implicitHeight + 44)
        radius: Theme.radius + 2
        color: Theme.panel
        border.color: Theme.line
        MouseArea { anchors.fill: parent }
        Flickable {
            id: flick
            anchors { fill: parent; margins: 22 }
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ColumnLayout {
                id: col
                width: flick.width
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    BcIcon { ctl: dlg;
                        visible: dlg.mode === "customize" && !!dlg.bc.cust
                        look: dlg.bc.cust ? dlg.bc.cust.look : null
                        locked: !!dlg.bc.cust && dlg.bc.cust.locked
                        size: 44
                    }
                    Icon { visible: ["password", "setlock", "passwd"].indexOf(dlg.mode) >= 0; name: "lock"; size: 26; color: Theme.accent }
                    Text {
                        Layout.fillWidth: true
                        text: dlg.title
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold
                        elide: Text.ElideMiddle
                    }
                }

                // ---- passwords
                Text {
                    Layout.fillWidth: true
                    visible: dlg.mode === "setlock"
                    text: "If you forget this password, the files inside can't be recovered. Hearth can't reset it."
                    color: Theme.danger; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; wrapMode: Text.WordWrap
                }
                Text {
                    Layout.fillWidth: true
                    visible: dlg.mode === "setlock"
                    text: "The files are encrypted inside the Briefcase folder and checked; only then does the unlocked copy go to the Trash. It opens with the same password on a USB stick or another Hearth device."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }
                Text {
                    Layout.fillWidth: true
                    visible: dlg.mode === "password"
                    text: "This Briefcase is locked: its files are encrypted. Type its password to open it."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }
                PwField { ctl: dlg; id: pwA; visible: ["password", "setlock", "passwd"].indexOf(dlg.mode) >= 0; rowId: "a"
                    hint: dlg.mode === "passwd" ? "Old password" : dlg.mode === "setlock" ? "New password" : "Password"
                    value: dlg.bc.pw.a; onEdited: t => dlg.bc.pw = Object.assign({}, dlg.bc.pw, { a: t, error: "" }) }
                PwField { ctl: dlg; id: pwB; visible: dlg.mode === "setlock" || dlg.mode === "passwd"; rowId: "b"
                    hint: dlg.mode === "passwd" ? "New password" : "The same password again"
                    value: dlg.bc.pw.b; onEdited: t => dlg.bc.pw = Object.assign({}, dlg.bc.pw, { b: t, error: "" }) }
                PwField { ctl: dlg; id: pwC; visible: dlg.mode === "passwd"; rowId: "c"; hint: "The new password again"
                    value: dlg.bc.pw.c; onEdited: t => dlg.bc.pw = Object.assign({}, dlg.bc.pw, { c: t, error: "" }) }
                Check { ctl: dlg;
                    visible: dlg.mode === "password" || dlg.mode === "setlock"
                    rowId: "remember"
                    label: "Remember on this device (kept in the system keyring)"
                    on: dlg.bc.pw.remember
                }
                Text {
                    Layout.fillWidth: true
                    visible: ["password", "setlock", "passwd"].indexOf(dlg.mode) >= 0 && dlg.bc.pw.error !== ""
                    text: dlg.bc.pw.error
                    color: dlg.bc.pw.error.endsWith("…") ? Theme.muted : Theme.danger
                    font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }

                // ---- choosing a Briefcase (or an item, a drive)
                Repeater {
                    model: dlg.mode === "pick" && dlg.bc.pick ? dlg.bc.pick.list : []
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        implicitHeight: 54
                        radius: Theme.r(10)
                        color: dlg.lit(index) ? Theme.panel3 : "transparent"
                        border.color: dlg.lit(index) ? Theme.accent : "transparent"
                        border.width: 2
                        RowLayout {
                            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                            spacing: 12
                            BcIcon { ctl: dlg; look: modelData.look; locked: !!modelData.raw.locked && !modelData.raw.unlocked; size: 30; visible: !!modelData.look }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Text { Layout.fillWidth: true; text: modelData.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; elide: Text.ElideRight }
                                Text { Layout.fillWidth: true; text: modelData.sub || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideMiddle }
                            }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: dlg.bc.pickChoose(index) }
                    }
                }

                // ---- Update All: each row's action, changeable
                Text {
                    Layout.fillWidth: true
                    visible: dlg.mode === "update"
                    text: "Hearth Files will do this. Change any row (click it, or Left / Right), then press Update. Replaced versions go to the Trash."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }
                Repeater {
                    model: dlg.mode === "update" && dlg.bc.plan ? dlg.bc.plan.rows : []
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        implicitHeight: 52
                        radius: Theme.r(10)
                        color: dlg.lit(index) ? Theme.panel3 : Theme.panel2
                        border.color: dlg.lit(index) ? Theme.accent : "transparent"
                        border.width: 2
                        RowLayout {
                            anchors { fill: parent; leftMargin: 14; rightMargin: 10 }
                            spacing: 12
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Text { Layout.fillWidth: true; text: modelData.item + (modelData.rel ? " › " + modelData.rel : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideMiddle }
                                Text { Layout.fillWidth: true; text: dlg.bc.kindNames[modelData.kind] || modelData.kind
                                       color: modelData.kind.indexOf("conflict") === 0 || modelData.kind.indexOf("changed-") === 0 ? Theme.danger : Theme.muted
                                       font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                            }
                            Rectangle {
                                implicitWidth: actText.implicitWidth + 28
                                implicitHeight: 34
                                radius: Theme.r(17)
                                color: modelData.action === "skip" ? Theme.panel3 : modelData.action.indexOf("delete") === 0 ? Theme.danger : Theme.accent
                                Text {
                                    id: actText
                                    anchors.centerIn: parent
                                    text: "‹  " + (dlg.bc.actionNames[modelData.action] || modelData.action) + "  ›"
                                    color: modelData.action === "skip" ? Theme.text : modelData.action.indexOf("delete") === 0 ? "#ffffff" : Theme.accentInk
                                    font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                                }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { dlg.bc.row = index; dlg.bc.cycleAction(index, 1); } }
                            }
                        }
                    }
                }
                Repeater {
                    model: dlg.mode === "update" && dlg.bc.plan ? dlg.bc.plan.info : []
                    delegate: Text {
                        required property var modelData
                        Layout.fillWidth: true
                        text: "“" + modelData.item + "”: " + modelData.label + (modelData.status === "away" ? " · it waits until then" : "")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                    }
                }

                // ---- Customize
                GridLayout {
                    Layout.fillWidth: true
                    visible: dlg.mode === "customize" && !!dlg.bc.cust
                    columns: 2
                    columnSpacing: 16
                    rowSpacing: 8
                    Label { text: "Name" }
                    TextBox { ctl: dlg; id: custName; rowId: "name"; value: dlg.bc.cust ? dlg.bc.cust.look.name : ""; onEdited: t => dlg.bc.setCust("name", t) }
                    Label { text: "Colour" }
                    Picker { ctl: dlg; rowId: "color"
                        text: dlg.bc.cust ? ({ accent: "Your accent", ember: "Ember", red: "Red", green: "Green", blue: "Blue", purple: "Purple", gold: "Gold", slate: "Slate" })[dlg.bc.cust.look.color] || dlg.bc.cust.look.color : ""
                        swatch: dlg.bc.cust ? dlg.bc.colorOf(dlg.bc.cust.look) : "transparent" }
                    Label { text: "Icon" }
                    Picker { ctl: dlg; rowId: "icon"; text: dlg.bc.cust ? ({ briefcase: "Classic briefcase", satchel: "Satchel", suitcase: "Suitcase", backpack: "Backpack", box: "Box" })[dlg.bc.cust.look.icon] : "" }
                    Label { text: "Badge" }
                    TextBox { ctl: dlg; id: custBadge; rowId: "badge"; hint: "An emoji or up to 4 letters (optional)"; value: dlg.bc.cust ? dlg.bc.cust.look.badge : ""; onEdited: t => dlg.bc.setCust("badge", t.slice(0, 4)) }
                    Label { text: "Note" }
                    TextBox { ctl: dlg; id: custNote; rowId: "note"; hint: "Shown in its details pane (optional)"; value: dlg.bc.cust ? dlg.bc.cust.look.note : ""; onEdited: t => dlg.bc.setCust("note", t) }
                    Label { text: "View" }
                    Picker { ctl: dlg; rowId: "view"; text: dlg.bc.cust ? (dlg.bc.cust.look.view ? app.viewNames[dlg.bc.cust.look.view] : "Like other folders") : "" }
                    Label { text: "Sort" }
                    Picker { ctl: dlg; rowId: "sort"; text: dlg.bc.cust ? (({ name: "Name", date: "Modified", size: "Size", type: "Type" })[dlg.bc.cust.look.sort] || "Like other folders") : "" }
                    Label { text: "Auto-sync" }
                    Picker { ctl: dlg; rowId: "autosync"; text: dlg.bc.cust ? dlg.bc.syncChoices[dlg.bc.syncIndex()][2] : "" }
                    Label { text: "Every"; visible: !!dlg.bc.cust && dlg.bc.cust.syncCustom }
                    TextBox { ctl: dlg; id: syncMin; rowId: "syncmin"; visible: !!dlg.bc.cust && dlg.bc.cust.syncCustom; hint: "minutes"
                        value: dlg.bc.cust ? String(dlg.bc.cust.look.autosync.minutes) : ""
                        onEdited: t => { const n = parseInt(t, 10); if (n > 0) dlg.bc.setCustSub("autosync", "minutes", Math.min(1440, n)); } }
                    Label { text: "Auto-lock"; dim: !!dlg.bc.cust && !dlg.bc.cust.locked }
                    Picker { ctl: dlg; rowId: "autolock"; dim: !!dlg.bc.cust && !dlg.bc.cust.locked
                        text: dlg.bc.cust ? (dlg.bc.cust.locked ? dlg.bc.lockChoices[dlg.bc.lockIndex()][1] : "Needs a password (Lock this Briefcase…)") : "" }
                    Label { text: "After"; visible: !!dlg.bc.cust && dlg.bc.cust.lockCustom && dlg.bc.cust.locked }
                    TextBox { ctl: dlg; id: lockMin; rowId: "lockmin"; visible: !!dlg.bc.cust && dlg.bc.cust.lockCustom && dlg.bc.cust.locked; hint: "minutes"
                        value: dlg.bc.cust ? String(dlg.bc.cust.look.autolock.minutes) : ""
                        onEdited: t => { const n = parseInt(t, 10); if (n > 0) dlg.bc.setCustSub("autolock", "minutes", Math.min(10080, n)); } }
                    Item { implicitWidth: 1; implicitHeight: 1 }
                    Check { ctl: dlg; rowId: "onSleep"; label: "Lock when the PC sleeps"; dim: !!dlg.bc.cust && !dlg.bc.cust.locked; on: !!dlg.bc.cust && dlg.bc.cust.look.autolock.onSleep }
                    Item { implicitWidth: 1; implicitHeight: 1 }
                    Check { ctl: dlg; rowId: "onEject"; label: "Lock when the USB stick is ejected"; dim: !!dlg.bc.cust && !dlg.bc.cust.locked; on: !!dlg.bc.cust && dlg.bc.cust.look.autolock.onEject }
                    Item { implicitWidth: 1; implicitHeight: 1 }
                    Check { ctl: dlg; rowId: "showInNav"; label: "Show in navigation pane"; on: !!dlg.bc.cust && dlg.bc.cust.look.showInNav }
                }
                Text {
                    Layout.fillWidth: true
                    visible: dlg.mode === "customize"
                    text: "Kept in the Briefcase itself, so it looks the same on a USB stick or another Hearth device. The files inside aren't touched."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                }

                // ---- a job
                Rectangle {
                    Layout.fillWidth: true
                    visible: dlg.mode === "job"
                    implicitHeight: 8
                    radius: 4
                    color: Theme.panel3
                    Rectangle {
                        width: parent.width * (dlg.bc.job && dlg.bc.job.total > 0 ? Math.min(1, dlg.bc.job.done / dlg.bc.job.total) : 0.05)
                        height: parent.height
                        radius: 4
                        color: Theme.accent
                    }
                }
                Text {
                    Layout.fillWidth: true
                    visible: dlg.mode === "job" || dlg.mode === "message"
                    text: dlg.mode === "message" && dlg.bc.msg ? dlg.bc.msg.text
                        : dlg.bc.job ? (dlg.bc.job.cancel ? "Stopping…" : (dlg.bc.job.file || "") + (dlg.bc.job.total ? "   " + app.fmtSize(dlg.bc.job.done) + " of " + app.fmtSize(dlg.bc.job.total) : "")) : ""
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                }

                // ---- buttons
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 6
                    spacing: 10
                    Text {
                        Layout.fillWidth: true
                        text: "Ⓐ choose · ◀ ▶ change · Ⓑ " + (dlg.mode === "job" ? "stop" : "cancel") + (dlg.rows.some(r => r.t === "field") ? " · Ⓨ keyboard" : "")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        elide: Text.ElideRight
                    }
                    Btn { ctl: dlg; rowId: "cancel"; visible: dlg.mode !== "message"; label: dlg.mode === "job" ? "Stop" : "Cancel" }
                    Btn { ctl: dlg;
                        rowId: "ok"
                        visible: dlg.mode !== "pick" && dlg.mode !== "job"
                        primary: true
                        label: ({ password: "Unlock", setlock: "Lock", passwd: "Change", update: "Update", customize: "Save", message: "OK" })[dlg.mode] || "OK"
                    }
                }
            }
        }
    }

    // ------------------------------------------------ small parts
    component Label: Text {
        property bool dim: false
        Layout.preferredWidth: 110
        color: dim ? Qt.rgba(Theme.muted.r, Theme.muted.g, Theme.muted.b, 0.5) : Theme.muted
        font.family: Theme.font; font.pixelSize: 13
    }
    component Btn: Rectangle {
        id: b
        property var ctl: null
        property string rowId
        property string label
        property bool primary: false
        readonly property bool focused: ctl.lit(rowId)
        implicitWidth: bt.implicitWidth + 36
        implicitHeight: 40
        radius: Theme.r(20)
        color: primary ? Theme.accent : Theme.panel3
        border.width: 2
        border.color: focused ? Theme.text : "transparent"
        Text { id: bt; anchors.centerIn: parent; text: b.label; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const i = ctl.rows.findIndex(r => r.id === b.rowId); if (i >= 0) ctl.bc.row = i; ctl.press(ctl.rows[i]); } }
    }
    component Check: Rectangle {
        id: ch
        property var ctl: null
        property string rowId
        property string label
        property bool on: false
        property bool dim: false
        Layout.fillWidth: true
        implicitHeight: 40
        radius: Theme.r(10)
        color: ctl.lit(rowId) ? Theme.panel3 : "transparent"
        border.width: 2
        border.color: ctl.lit(rowId) ? Theme.accent : "transparent"
        opacity: dim ? 0.5 : 1
        Row {
            anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
            spacing: 10
            Rectangle {
                width: 20; height: 20; radius: 5
                color: ch.on ? Theme.accent : "transparent"
                border.width: 2; border.color: ch.on ? Theme.accent : Theme.muted
                Text { anchors.centerIn: parent; text: "✓"; visible: ch.on; color: Theme.accentInk; font.pixelSize: 13; font.weight: Font.Bold }
            }
            Text { text: ch.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter }
        }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const i = ctl.rows.findIndex(r => r.id === ch.rowId); if (i >= 0) ctl.bc.row = i; ctl.toggle(ch.rowId); } }
    }
    component Picker: Rectangle {
        id: pk
        property var ctl: null
        property string rowId
        property string text
        property color swatch: "transparent"
        property bool dim: false
        Layout.fillWidth: true
        implicitHeight: 40
        radius: Theme.r(10)
        color: Theme.panel3
        border.width: 2
        border.color: ctl.lit(rowId) ? Theme.accent : "transparent"
        opacity: dim ? 0.5 : 1
        Text {
            anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
            text: "‹"
            color: Theme.muted
            font.pixelSize: 16
        }
        Row {
            anchors.centerIn: parent
            spacing: 8
            Rectangle { visible: pk.swatch.a > 0; width: 14; height: 14; radius: 7; color: pk.swatch; anchors.verticalCenter: parent.verticalCenter }
            Text { text: pk.text; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
        }
        Text {
            anchors { right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
            text: "›"
            color: Theme.muted
            font.pixelSize: 16
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: mouse => { const i = ctl.rows.findIndex(r => r.id === pk.rowId); if (i >= 0) ctl.bc.row = i; ctl.choice(pk.rowId, mouse.x < width / 3 ? -1 : 1); }
        }
    }
    component TextBox: Rectangle {
        id: tb
        property var ctl: null
        property string rowId
        property string value
        property string hint: ""
        property bool secret: false
        signal edited(string t)
        function focusIn() { input.forceActiveFocus(); }
        Layout.fillWidth: true
        implicitHeight: 44
        radius: Theme.r(10)
        color: Theme.panel3
        border.width: 2
        border.color: input.activeFocus || ctl.lit(rowId) ? Theme.accent : "transparent"
        TextInput {
            id: input
            anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.text
            selectionColor: Theme.accent
            selectedTextColor: Theme.accentInk
            font.family: Theme.font; font.pixelSize: 15
            clip: true
            echoMode: tb.secret ? TextInput.Password : TextInput.Normal
            text: tb.value
            onTextEdited: tb.edited(text)
            onActiveFocusChanged: if (activeFocus) { const i = ctl.rows.findIndex(r => r.id === tb.rowId); if (i >= 0 && ctl.bc.row !== i) ctl.bc.row = i; }
            Keys.onPressed: e => { if (e.key !== Qt.Key_Left && e.key !== Qt.Key_Right) ctl.key(e); }
            Text { anchors.verticalCenter: parent.verticalCenter; visible: input.text === "" && !input.activeFocus; text: tb.hint; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
        }
    }
    component PwField: TextBox { secret: true; Layout.fillWidth: true }
    // A Briefcase's icon: its style, in its colour, with its badge and a padlock when locked.
    component BcIcon: Item {
        id: bi
        property var ctl: null
        property var look: null
        property bool locked: false
        property real size: 32
        implicitWidth: size; implicitHeight: size
        Icon { anchors.fill: parent; name: "bc-" + (bi.look && bi.look.icon ? bi.look.icon : "briefcase"); size: bi.size; color: ctl.bc.colorOf(bi.look) }
        Text {
            visible: !!bi.look && !!bi.look.badge
            anchors { right: parent.right; top: parent.top; margins: -2 }
            text: bi.look ? bi.look.badge || "" : ""
            color: Theme.text; font.pixelSize: Math.max(9, bi.size * 0.3); font.weight: Font.Bold
        }
        Icon {
            visible: bi.locked; anchors { right: parent.right; bottom: parent.bottom; margins: -2 }
            name: "lock"
            size: Math.max(10, bi.size * 0.4)
            color: Theme.text
        }
    }
}
