.pragma library
// Sub themes (Settings › Personalisation › Sub theme; theme/palettes.json, the one source of truth): a look's
// colourways. A sub theme changes only colours and the default wallpaper; the look's shape (corners, bar, frames,
// Gaming's neon edges) stays. theme/hearth-theme does the same in Python (sub_list / sub_palette); the tests run both.
//
// palettes.json: a look's "subThemes" (its own, the first one "own": true = today's colours) and "blackWallpapers"
// ({id: wallpaper}: which of the shared "blackSet" it gets, and their pictures). Settings.subTheme "" (the default for
// everyone) or an id that isn't there = the first one: the look's own colours and wallpaper. Anything missing or
// broken falls back to the look's own, never an error.

var TOKENS = ["bg", "bg2", "panel", "panel2", "panel3", "line", "text", "muted", "ember", "flame", "third", "ink"];

function isHex(c) { return typeof c === "string" && /^#[0-9a-fA-F]{6}$/.test(c); }
function isObj(o) { return o !== null && typeof o === "object" && !Array.isArray(o); }

// The look's sub themes, in order: its own colourways ("colours"), then the black set ("black"). [] for a look
// without any (Zen, Glass…).
function list(pal, look) {
    try {
        var L = isObj(pal) ? pal[look] : null;
        if (!isObj(L) || !Array.isArray(L.subThemes)) return [];
        var out = [], seen = {};
        var add = function (s, group, wall) {
            if (!isObj(s) || typeof s.id !== "string" || s.id === "" || seen[s.id]) return;
            seen[s.id] = true;
            out.push({ id: s.id, label: typeof s.label === "string" && s.label ? s.label : s.id, desc: typeof s.desc === "string" ? s.desc : "",
                       group: group, own: out.length === 0 || s.own === true, wallpaper: typeof wall === "string" ? wall : "",
                       dark: isObj(s.dark) ? s.dark : null, light: isObj(s.light) ? s.light : null });
        };
        for (var i = 0; i < L.subThemes.length; i++) add(L.subThemes[i], "colours", isObj(L.subThemes[i]) ? L.subThemes[i].wallpaper : "");
        if (out.length === 0) return [];
        if (isObj(L.blackWallpapers) && Array.isArray(pal.blackSet))
            for (var j = 0; j < pal.blackSet.length; j++) {
                var b = pal.blackSet[j];
                if (isObj(b) && typeof b.id === "string" && L.blackWallpapers[b.id] !== undefined) add(b, "black", L.blackWallpapers[b.id]);
            }
        return out;
    } catch (e) { return []; }
}

// The sub theme in use: the one with this id, else the first (the look's own). null for a look without any.
function find(pal, look, id) {
    var l = list(pal, look);
    if (l.length === 0) return null;
    for (var i = 0; i < l.length; i++) if (l[i].id === id) return l[i];
    return l[0];
}

// True when the look's own colours are in use (no sub themes, "", an unknown id, or the first one).
function isOwn(pal, look, id) { var s = find(pal, look, id); return !s || s.own === true; }

// The colours for this mode: the look's own (`base`, palettes.json look[mode]) with the sub theme's valid colours on
// top. A token the sub theme lacks, or has broken, stays the look's own.
function palette(base, pal, look, id, mode) {
    var out = {};
    var k;
    if (isObj(base)) for (k in base) out[k] = base[k];
    var s = find(pal, look, id);
    if (!s || s.own) return out;
    var c = mode === "light" ? s.light : s.dark;
    if (!isObj(c)) return out;
    for (var i = 0; i < TOKENS.length; i++) { k = TOKENS[i]; if (isHex(c[k])) out[k] = c[k].toLowerCase(); }
    return out;
}

// The wallpaper the look shows when you haven't picked one ("" in Settings.wallpaper): with Match sub theme on, the
// sub theme's own; otherwise (or when it has none) the look's own.
function ownWallpaper(pal, look, id, match) {
    var L = isObj(pal) && isObj(pal[look]) ? pal[look] : {};
    var lw = typeof L.wallpaper === "string" ? L.wallpaper : "";
    if (match === false) return lw;
    var s = find(pal, look, id);
    return s && !s.own && s.wallpaper !== "" ? s.wallpaper : lw;
}

// The wallpaper on show. A picture you chose yourself (Settings.wallpaper) always wins: a sub theme never replaces it.
function shownWallpaper(pal, look, id, chosen, match) {
    return typeof chosen === "string" && chosen !== "" ? chosen : ownWallpaper(pal, look, id, match);
}

// What Settings › Personalisation writes for each choice, and nothing else (Settings never change by themselves).
// Picking a sub theme: only the sub theme (its wallpaper shows through Match; a wallpaper you chose stays).
function changesForSub(id) { return { subTheme: typeof id === "string" ? id : "" }; }
// Changing the look (only ever by you): the look, and the sub theme back to that look's first one.
function changesForLook(look) { return { look: look, subTheme: "" }; }
// The Match sub theme switch. Off: the look's own wallpaper when you haven't chosen one. On: the sub theme's wallpaper;
// when you had chosen your own, turning it on is you asking for the sub theme's instead.
function changesForMatch(on, chosen) {
    if (!on) return { wallMatch: false };
    return typeof chosen === "string" && chosen !== "" ? { wallMatch: true, wallpaper: "" } : { wallMatch: true };
}
// The switch as shown: on only while the sub theme's wallpaper is the one on show.
function matchShown(match, chosen) { return match !== false && !(typeof chosen === "string" && chosen !== ""); }

// The list's picture for a sub theme: images/wallpapers/sub/thumbs/<look>-<id>.jpg (tools/make-sub-theme-wallpapers.py).
function thumb(look, id) { return "images/wallpapers/sub/thumbs/" + look + "-" + id + ".jpg"; }
