// The Desk bar's standard menus (mockup firehold-plasma 12): the app's own name menu, Window, Help, for an app with no
// menus of its own; one list for Firehold on Plasma (app/main.qml) and Firehold on labwc (desktop/shell/FhpDeskBar.qml).
// Rows: [label, shortcut or note on the right, action, kind (head | red | sep | sub | pair), what it does, icon].
.pragma library

function modeName(m) { return m === "couch" ? "Couch" : m === "away" ? "Away" : "Desk"; }

function appItems(appName) {
    var up = (appName || "This app").toUpperCase();
    return [["THE “" + up + "” MENU · STANDARD FOR EVERY APP", "", "", "head"],
        ["About " + (appName || "this app"), "", "about", "", "From its desktop entry"],
        ["Settings for this app", "", "settings", "", "Proton version, controller layout, which screen it opens on"],
        ["Open its folder", "", "folder", "", "Where it's installed"],
        ["Send logs", "", "logs", "", "When it misbehaves"], ["Quit", "Ctrl+Q", "close"]];
}

function windowItems(appName) {
    var up = (appName || "This app").toUpperCase();
    return [[up + " HAS NO MENUS OF ITS OWN · THESE ARE HEARTH'S", "", "", "head"],
        ["Full screen", "F11", "fullscreen"], ["Move to screen ›", "Super+Shift+← →", "", "sub"],
        ["Keep above others", "", "above"], ["Snap left · right", "Super+← →", "snap-left|snap-right", "pair"],
        ["Maximize", "Super+↑", "maximize"], ["Minimize", "Super+↓", "minimize"], ["", "", "", "sep"],
        ["Take a screenshot of it", "Alt+Print", "screenshot"], ["Record it", "Super+Alt+R", "record"], ["", "", "", "sep"],
        ["Close", "Alt+F4", "close"], ["Force quit (not answering)", "", "kill", "red"]];
}

// Move to screen ›: each screen with its mode; where the app would go full screen (a Couch screen), where it is.
function moveItems(screens, frontScreen) {
    return (screens || []).map(function (s) {
        var here = s.name === frontScreen;
        return [(s.label || s.name) + " · " + modeName(s.mode) + (here ? " ✓" : ""),
                here ? "here now" : s.mode === "couch" ? "goes full screen" : s.mode === "away" ? "waits there" : "in a window", "move:" + s.name,
                "", "", s.tv ? "video-television" : "video-display"];
    });
}

function helpItems() {
    return [["HELP", "", "", "head"], ["Search every command", "Alt+Space", "search"],
        ["Keyboard shortcuts", "", "shortcuts", "", "Hearth's and KDE's in one list"]];
}
