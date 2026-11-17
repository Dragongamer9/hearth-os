.pragma library
// Search plugin: special characters (like KRunner's special characters runner). Enter copies the character.
//   "degree", "copyright", "euro", "arrow right", "em dash"   by name (whole word, 3 letters or more)
//   "char e acute", "char →"                                  "char" first: every name that starts with it
//   "u+00e9", "U+1F525", "&eacute;", "&#233;"                 by code point or HTML name

// [character, names (space-separated words; "|" between names)]
var CHARS = [
    ["°", "degree|degrees"], ["©", "copyright"], ["®", "registered"], ["™", "trademark|tm"], ["€", "euro"], ["£", "pound|sterling"],
    ["¥", "yen|yuan"], ["¢", "cent"], ["§", "section"], ["¶", "pilcrow|paragraph"], ["†", "dagger"], ["•", "bullet"], ["…", "ellipsis"],
    ["—", "em dash|emdash"], ["–", "en dash|endash"], ["−", "minus"], ["±", "plus minus|plusminus"], ["×", "times|multiply"],
    ["÷", "divide|division"], ["≈", "approx|approximately|almost equal"], ["≠", "not equal|unequal"], ["≤", "less or equal|less than or equal"],
    ["≥", "greater or equal|greater than or equal"], ["∞", "infinity"], ["√", "square root|root"], ["π", "pi"], ["µ", "micro|mu"],
    ["Ω", "ohm|omega"], ["Δ", "delta"], ["∑", "sum|sigma"], ["½", "half|one half"], ["¼", "quarter|one quarter"], ["¾", "three quarters"],
    ["²", "squared|superscript two"], ["³", "cubed|superscript three"], ["‰", "per mille|permille"],
    ["→", "arrow right|right arrow"], ["←", "arrow left|left arrow"], ["↑", "arrow up|up arrow"], ["↓", "arrow down|down arrow"],
    ["↔", "arrow left right"], ["⇒", "double arrow right|implies"], ["✓", "check|check mark|tick"], ["✗", "cross|ballot x"],
    ["★", "star"], ["☆", "white star|empty star"], ["♥", "heart suit"], ["♪", "note|music note"], ["☀", "sun"], ["☂", "umbrella"],
    ["“", "left quote|open quote"], ["”", "right quote|close quote"], ["‘", "left single quote"], ["’", "apostrophe|right single quote"],
    ["«", "left guillemet|guillemet"], ["»", "right guillemet"], ["¿", "inverted question"], ["¡", "inverted exclamation"],
    ["é", "e acute"], ["è", "e grave"], ["ê", "e circumflex"], ["ë", "e umlaut|e diaeresis"], ["á", "a acute"], ["à", "a grave"],
    ["â", "a circumflex"], ["ä", "a umlaut"], ["å", "a ring"], ["æ", "ae|ash"], ["ç", "c cedilla|cedilla"], ["í", "i acute"],
    ["ñ", "n tilde|enye"], ["ó", "o acute"], ["ö", "o umlaut"], ["ø", "o slash"], ["ú", "u acute"], ["ü", "u umlaut"], ["ß", "eszett|sharp s"],
    ["œ", "oe"], ["þ", "thorn"], ["ð", "eth"], ["⌘", "command|cmd key"], ["⌥", "option key"], ["⇧", "shift key"], ["⏎", "return key|enter key"],
    ["␣", "space symbol"], [" ", "non breaking space|nbsp"], ["‐", "hyphen"], ["¦", "broken bar"], ["‽", "interrobang"]
];
var HTML = { amp: "&", lt: "<", gt: ">", quot: "\"", apos: "'", nbsp: " ", copy: "©", reg: "®", trade: "™", euro: "€", pound: "£",
             deg: "°", hellip: "…", mdash: "—", ndash: "–", times: "×", divide: "÷", eacute: "é", egrave: "è", aacute: "á", ntilde: "ñ",
             uuml: "ü", ouml: "ö", auml: "ä", szlig: "ß", rarr: "→", larr: "←", hearts: "♥", laquo: "«", raquo: "»", para: "¶", sect: "§" };

function hex(cp) { var h = cp.toString(16).toUpperCase(); while (h.length < 4) h = "0" + h; return "U+" + h; }
function codeOf(ch) { return ch.codePointAt ? ch.codePointAt(0) : ch.charCodeAt(0); }
function row(ch, name, score, top) {
    var shown = ch === " " ? "(space)" : ch;
    return { plugin: "chars", group: "Special characters", title: shown + "  " + name, sub: hex(codeOf(ch)) + " · Enter copies it", glyph: shown,
             icon: "kbd", score: score, top: !!top, action: { type: "copy", value: ch } };
}

function results(qRaw) {
    var q = String(qRaw || "").trim(), l = q.toLowerCase(), m, cp;
    if (q.length < 3) return [];
    if ((m = /^u\+([0-9a-f]{2,6})$/i.exec(q)) || (m = /^&#x([0-9a-f]{1,6});?$/i.exec(q)) || (m = /^&#(\d{1,7});?$/.exec(q))) {
        cp = parseInt(m[1], /^&#\d/.test(q) ? 10 : 16);
        if (!(cp > 0 && cp <= 0x10FFFF) || (cp >= 0xD800 && cp <= 0xDFFF)) return [];
        return [row(String.fromCodePoint ? String.fromCodePoint(cp) : String.fromCharCode(cp), hex(cp), 1870, true)];
    }
    if ((m = /^&([a-z]+);?$/i.exec(q)) && HTML[m[1]]) return [row(HTML[m[1]], "&" + m[1] + ";", 1870, true)];
    var prefix = /^char\s+(.+)$/i.exec(q), w = prefix ? prefix[1].toLowerCase() : l, out = [];
    for (var i = 0; i < CHARS.length && out.length < 6; i++) {
        var names = CHARS[i][1].split("|"), hit = "";
        for (var k = 0; k < names.length; k++) {
            if (names[k] === w || (prefix && (names[k].indexOf(w) === 0 || CHARS[i][0] === prefix[1]))) { hit = names[k]; break; }
        }
        if (hit) out.push(row(CHARS[i][0], hit, (prefix ? 1860 : 880) - out.length, !!prefix && out.length === 0));
    }
    return out;
}
