// Quickshell's NotificationServer (plasma-hearth), on Plasma's own notification service (org.kde.notificationmanager:
// what KDE's notifications widget uses). It takes the notifications only when nothing else in Plasma does (KDE's own
// notifications widget, in KDE's panel, keeps them then). Each new one comes as `notification(n)`, with what Mantel's
// Notifications.qml reads: appName, summary, body, appIcon, image, urgency, actions [{identifier, text, invoke()}],
// desktopEntry, expireTimeout, hints, transient; dismiss(), expire(), and `closed` when it goes.
import QtQuick
import org.kde.notificationmanager as NM

Item {
    id: srv
    property bool keepOnReload: false
    property bool actionsSupported: true
    property bool imageSupported: true
    property bool bodyMarkupSupported: true
    property bool persistenceSupported: true
    property bool inlineReplySupported: false
    signal notification(var n)
    property var live: ({})
    // plasmashell owns the notification service itself (it registers it at start); Hearth draws what comes in.
    readonly property bool serving: NM.Server.valid === true
    onServingChanged: { console.info("plasma-hearth: Plasma's notification service " + (serving ? "is up: Hearth shows the notifications" : "isn't up")); hold(); }
    Component.onCompleted: { console.info("plasma-hearth: notifications: service " + (serving ? "up" : "not up yet")); hold(); }
    // KDE's own pop-ups pause while Hearth draws them (hearth-plasma-panel hold-popups: Plasma's inhibition, held
    // only while this runs; nothing saved). Notifications still all come in.
    // KDE's critical pop-ups too (KDE lets those through a pause): Plasma's own setting, changed in this session only
    // while Hearth shows the notifications and put back as it was when this goes (hearth-plasma-panel apply / undo
    // keep the file the same way, so the next sign-in matches).
    NM.Settings { id: nms }
    property var critWas: undefined
    function quietCritical() { if (!serving || critWas !== undefined) return; critWas = nms.criticalPopupsInDoNotDisturbMode; nms.criticalPopupsInDoNotDisturbMode = false; }
    Component.onDestruction: if (critWas !== undefined) nms.criticalPopupsInDoNotDisturbMode = critWas
    property bool holding: false
    function hold() { quietCritical(); if (!serving || holding) return; holding = true; Hearth.call(["python3", Hearth.helper, "hold-popups"], (out) => { console.info("plasma-hearth: KDE's pop-ups: " + out.trim()); srv.holding = false; }); }
    NM.Notifications {
        id: model
        showNotifications: true
        showJobs: false
        showExpired: true
        showDismissed: true
        showAddedDuringInhibition: true
        groupMode: NM.Notifications.GroupDisabled
        onRowsInserted: (parent, first, last) => { if (srv.serving) for (let i = first; i <= last; i++) srv.add(i); }
        onRowsAboutToBeRemoved: (parent, first, last) => {
            for (let i = first; i <= last; i++) { const id = srv.role(i, NM.Notifications.IdRole); const n = srv.live[id]; if (n) { delete srv.live[id]; n.closed.fire(); } }
        }
    }
    function role(i, r) { return model.data(model.index(i, 0), r); }
    function rowOf(id) { for (let i = 0; i < model.count; i++) if (role(i, NM.Notifications.IdRole) === id) return i; return -1; }
    function add(i) {
        if (role(i, NM.Notifications.TypeRole) !== NM.Notifications.NotificationType) return;
        const id = role(i, NM.Notifications.IdRole);
        const names = Array.from(role(i, NM.Notifications.ActionNamesRole) || []), labels = Array.from(role(i, NM.Notifications.ActionLabelsRole) || []);
        const acts = names.map((a, k) => ({ identifier: a, text: labels[k] || a, invoke: function () { const r = srv.rowOf(id); if (r >= 0) model.invokeAction(model.index(r, 0), a); } }));
        if (role(i, NM.Notifications.HasDefaultActionRole) === true)
            acts.unshift({ identifier: "default", text: role(i, NM.Notifications.DefaultActionLabelRole) || "Open", invoke: function () { const r = srv.rowOf(id); if (r >= 0) model.invokeDefaultAction(model.index(r, 0)); } });
        const u = role(i, NM.Notifications.UrgencyRole);
        const img = role(i, NM.Notifications.ImageRole);
        const closedFns = [];
        const n = {
            id: id, nid: id, tracked: false, resident: false, hints: ({}),
            closed: { connect: function (f) { closedFns.push(f); }, fire: function () { for (const f of closedFns) { try { f(); } catch (e) {} } } },
            dismiss: function () { const r = srv.rowOf(id); if (r >= 0) model.close(model.index(r, 0)); },
            expire: function () { const r = srv.rowOf(id); if (r >= 0) model.expire(model.index(r, 0)); }, appName: String(role(i, NM.Notifications.ApplicationNameRole) || ""), summary: String(role(i, NM.Notifications.SummaryRole) || ""),
            body: String(role(i, NM.Notifications.BodyRole) || ""), appIcon: String(role(i, NM.Notifications.ApplicationIconNameRole) || role(i, NM.Notifications.IconNameRole) || ""),
            image: typeof img === "string" ? img : "", urgency: u === NM.Notifications.CriticalUrgency ? 2 : u === NM.Notifications.LowUrgency ? 0 : 1,
            actions: acts, desktopEntry: String(role(i, NM.Notifications.DesktopEntryRole) || ""), expireTimeout: Number(role(i, NM.Notifications.TimeoutRole) || 0),
            transient: role(i, NM.Notifications.TransientRole) === true
        };
        live[id] = n;
        notification(n);
    }
}
