// The keyboard layout and copies and downloads for NotifyHub.qml (plasma-hearth), from Plasma's own: KDE's keyboard
// layout service (org.kde.plasma.workspace.keyboardlayout) and the job progress Plasma's notification service keeps
// (org.kde.notificationmanager: KDE's copies and downloads). Told when they change; nothing polls.
import QtQuick
import org.kde.plasma.workspace.keyboardlayout as KB
import org.kde.notificationmanager as NM

Item {
    id: ph
    KB.KeyboardLayout { id: kb }
    readonly property var layout: ({
        layouts: Array.from(kb.layoutsList || []).map(l => ({ short: l.shortName || "", name: l.longName || l.shortName || "" })),
        index: kb.layout, canSwitch: true
    })
    function setLayout(i) { kb.layout = i; }
    NM.Notifications {
        id: jm
        showNotifications: false
        showJobs: true
        showExpired: false
        showDismissed: true
        groupMode: NM.Notifications.GroupDisabled
    }
    property var jobs: []
    function role(i, r) { return jm.data(jm.index(i, 0), r); }
    function rebuild() {
        const out = [];
        for (let i = 0; i < jm.count; i++) {
            if (role(i, NM.Notifications.TypeRole) !== NM.Notifications.JobType) continue;
            const st = role(i, NM.Notifications.JobStateRole);
            out.push({ id: role(i, NM.Notifications.IdRole), title: String(role(i, NM.Notifications.SummaryRole) || role(i, NM.Notifications.ApplicationNameRole) || ""),
                percent: Number(role(i, NM.Notifications.PercentageRole) ?? -1), state: st === NM.Notifications.JobStateSuspended ? "suspended" : st === NM.Notifications.JobStateStopped ? "done" : "running",
                suspended: st === NM.Notifications.JobStateSuspended, suspendable: role(i, NM.Notifications.SuspendableRole) === true, killable: role(i, NM.Notifications.KillableRole) === true,
                entry: String(role(i, NM.Notifications.DesktopEntryRole) || ""), app: String(role(i, NM.Notifications.ApplicationNameRole) || "") });
        }
        jobs = out;
    }
    Connections { target: jm; function onRowsInserted() { ph.rebuild(); } function onRowsRemoved() { ph.rebuild(); } function onDataChanged() { ph.rebuild(); } }
    function jobAction(id, op) {
        for (let i = 0; i < jm.count; i++) if (role(i, NM.Notifications.IdRole) === id) {
            const ix = jm.index(i, 0);
            if (op === "suspend") jm.suspendJob(ix); else if (op === "resume") jm.resumeJob(ix); else if (op === "cancel") jm.killJob(ix);
        }
    }
}
