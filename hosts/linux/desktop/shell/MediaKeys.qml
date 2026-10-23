// The media keys (play/pause, next, previous, stop) for whatever is playing: browsers, Spotify, music and video
// players (MPRIS). labwc's key bindings call `quickshell ipc ... call media playPause|next|previous|stop`.
// Loaded by shell.qml.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

Scope {
    id: root
    // The one playing, else the last one that can be controlled.
    function player() {
        const ps = Mpris.players.values;
        return ps.find(p => p.isPlaying) || ps.find(p => p.canControl) || ps[0] || null;
    }
    IpcHandler {
        target: "media"
        function playPause(): void { const p = root.player(); if (p && p.canTogglePlaying) p.togglePlaying(); }
        function next(): void { const p = root.player(); if (p && p.canGoNext) p.next(); }
        function previous(): void { const p = root.player(); if (p && p.canGoPrevious) p.previous(); }
        function stop(): void { const p = root.player(); if (p) p.stop(); }
    }
}
