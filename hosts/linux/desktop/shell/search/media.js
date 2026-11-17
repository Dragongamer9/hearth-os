.pragma library
// Search plugin: media control (like KRunner's MPRIS runner). "play", "pause", "next", "previous" (or "next song",
// "skip", "prev", "stop", "play pause") for what's playing (MPRIS players: SearchSources.qml's media
// {title, artist, player, playing}); done through MediaKeys.qml (quickshell ipc media …). Nothing when nothing plays.

var ACTS = [
    ["playPause", ["play", "pause", "resume", "play pause", "toggle play"]],
    ["next", ["next", "next song", "next track", "skip"]],
    ["previous", ["previous", "prev", "previous song", "previous track", "back a song"]],
    ["stop", ["stop", "stop playing", "stop music"]]
];

function results(qRaw, media) {
    var q = String(qRaw || "").trim().toLowerCase().replace(/\s+/g, " ");
    if (q.length < 3 || !media || !media.player) return [];
    var now = (media.title || "Nothing named") + (media.artist ? " · " + media.artist : "") + " · " + media.player;
    for (var i = 0; i < ACTS.length; i++) {
        if (ACTS[i][1].indexOf(q) < 0) continue;
        var v = ACTS[i][0], title = v === "playPause" ? (media.playing ? "Pause" : "Play") : v === "next" ? "Next track" : v === "previous" ? "Previous track" : "Stop";
        return [{ plugin: "media", group: "Media", title: title, sub: now, icon: v === "playPause" ? (media.playing ? "pause" : "play") : v === "stop" ? "close" : "forward",
                  score: 1835, top: true, action: { type: "media", value: v } }];
    }
    return [];
}
