pragma Singleton

// What Firehold's notification cards and list share (style "firehold", mockups 09-10), none of it a setting:
//   * Do not disturb as Firehold has it (hearth-quick dnd: on by hand, its schedule, "while a game is full screen"),
//     asked when a notification comes in and when the list or Quick settings open, never in a loop.
//   * While you played: notifications that came in while a full-screen game was in front, held back so they didn't
//     cover it (they're all in the list); the line says so once the game is over, until you press Seen.
//   * Muted for an hour (the "···" menu, right-click): only for this session, nothing written; muting for good is
//     Mantel's own (DeskState.setMuted, Settings › Notifications), which you can turn back there.
import QtQuick

Item {
    id: st
    property var dnd: ({ on: false, schedule: true, from: "11 PM", to: "7 AM", games: true, family: true })
    function askDnd(then) {
        Hearth.call(["python3", Hearth.quick, "dnd"], (out) => {
            try { const o = JSON.parse(out.trim().split("\n").pop() || "{}"); if (o && o.on !== undefined) st.dnd = o; } catch (e) {}
            if (then) then();
        });
    }
    function toggleDnd() { Hearth.call(["python3", Hearth.quick, "qs-do", "dnd"], () => st.askDnd()); }

    // ---- a full-screen game in front
    readonly property var front: WinList.activeToplevel
    readonly property bool gameFront: !!front && front.fullscreen === true
    property string game: ""
    property real gameSince: 0
    property int held: 0
    property var played: null              // {game, minutes, count}: the last one, until Seen
    onGameFrontChanged: {
        if (gameFront) { game = front.title || front.appId || "a game"; gameSince = Date.now(); held = 0; }
        else if (gameSince > 0) {
            if (held > 0) played = { game: game.replace(/ — .*$/, ""), minutes: Math.max(1, Math.round((Date.now() - gameSince) / 60000)), count: held };
            gameSince = 0;
        }
    }
    function seen() { played = null; }

    // ---- muted for an hour, this session only
    property var hourMuted: ({})
    function muteHour(app) { const m = Object.assign({}, hourMuted); m[app] = Date.now() + 3600000; hourMuted = m; }
    function isHourMuted(app) { const t = hourMuted[app]; return !!t && t > Date.now(); }

    // Should a card show for this notification (it's in the list either way)?
    function cardFor(h, urgent) {
        if (urgent) return true;
        if (isHourMuted(h.app)) return false;
        if (dnd.on) return false;
        if (gameFront && dnd.games !== false) { held++; return false; }
        return true;
    }
}
