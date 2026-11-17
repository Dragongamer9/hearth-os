.pragma library
// Search plugin: spell check and dictionary (like KRunner's spell checker and dictionary runners).
//   "spell recieve"   is it right, and what hunspell suggests (hearth-search-sources spell, looked up as you type)
//   "define hearth"   the word checked, and its meaning on Wiktionary (opens in the browser)
// spell: {word: {correct, suggestions, missing}} filled by SearchSources.qml. Enter copies the word or a suggestion.

function asked(qRaw) {
    var m = /^\s*(spell|define|meaning of|dict)\s+([^\s].*?)\s*$/i.exec(String(qRaw || ""));
    return m ? { kind: m[1].toLowerCase() === "spell" ? "spell" : "define", word: m[2] } : null;
}

function results(qRaw, spell) {
    var a = asked(qRaw);
    if (!a) return [];
    var w = a.word, info = spell ? spell[w] : null, out = [];
    function r(title, sub, icon, score, action, top) { return { plugin: "spell", group: a.kind === "spell" ? "Spelling" : "Dictionary", title: title, sub: sub, icon: icon, score: score, top: !!top, action: action }; }
    if (a.kind === "define")
        out.push(r("What “" + w + "” means", "Wiktionary, in your browser", "globe", 2060,
                   { type: "url", value: "https://en.wiktionary.org/wiki/Special:Search?search=" + encodeURIComponent(w) }, true));
    if (!info) out.push(r("Checking “" + w + "”…", "Spell check (hunspell)", "check", 2050, null));
    else if (info.missing) out.push(r("Spell check isn't installed", "hunspell and a dictionary are needed", "warn", 2050, null));
    else if (info.correct) out.push(r("“" + w + "” is spelled right", "Spell check · Enter copies it", "check", 2050, { type: "copy", value: w }, a.kind === "spell"));
    else {
        var s = info.suggestions || [];
        if (!s.length) out.push(r("“" + w + "” isn't a word the dictionary knows", "No suggestions", "warn", 2050, null));
        for (var i = 0; i < s.length && i < 4; i++)
            out.push(r(s[i], i === 0 ? "Did you mean · Enter copies it" : "Or", "check", 2050 - i, { type: "copy", value: s[i] }, i === 0 && a.kind === "spell"));
    }
    return out;
}
