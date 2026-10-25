/* Hearth's on-screen keyboard: one keyboard for Game Mode (and its Quick Access over games), Mantel (Desktop Mode,
 * typing into any Linux app), Hearth's own apps and Hearth Browser. docs/OSK.md describes how it works, the
 * research behind it and the public API (HearthOSK.*), which other parts of Hearth call: keep it stable.
 *
 * One file, three places it runs:
 *   - a web page (<script src="osk/hearth-osk.js">): the engine and the page's keyboard (HearthOSK.mount / open);
 *   - Qt's QML engine (Mantel: import "osk/hearth-osk.js" as Osk; Osk.HearthOSK.createEngine(...)): the engine only,
 *     drawn by OskBoard.qml from engine.view();
 *   - Firefox's chrome (Hearth Browser, Services.scriptloader.loadSubScript): the engine and the page keyboard.
 * So it's plain ES2017 (no ?. or ??, no class fields), and the drawing part only runs when asked to.
 *
 * The copy in ui/osk is the one to edit. tools/osk-sync.py copies it (and the word lists) to the apps and
 * Desktop Mode; a test (hosts/linux/test/test_hearth_osk.py) fails when a copy differs.
 */
var HearthOSK = (function () {
  'use strict';
  var API_VERSION = 1;

  // ------------------------------------------------------------------------------------------------ shared state
  // Words and what was learned are shared by every keyboard on the page.
  var STATE = {
    dict: {},                 // lang → {list: [words, most used first], folded: [...]}
    learned: {},              // lang → {word: count}
    next: {},                 // lang → {word: {next word: count}}
    recentEmoji: [],
    clipboard: [],
    systemLayouts: [],        // the system's XKB layouts, in order
    settings: null,
    save: null,               // function(data) called (soon after) when learned words change
    loadWords: null,          // function(lang) to fetch a language's words (set for web pages below)
    base: '',
    listeners: []
  };
  // ------------------------------------------------------------------------------------------------ layouts
  // Letters only: the number row, punctuation and the bottom row are the same everywhere. `xkb`: the system
  // keyboard layouts (Settings › Keyboard) that pick it when the layout is Automatic.
  var LAYOUTS = [
    { id: 'en-us', name: 'English (QWERTY)', short: 'EN', lang: 'en', rows: ['qwertyuiop', 'asdfghjkl', 'zxcvbnm'], tail: [',', '.'], xkb: ['us', 'gb', 'ie', 'au', 'ca', 'in', 'za', 'nz', 'en'] },
    { id: 'fr', name: 'Français (AZERTY)', short: 'FR', lang: 'fr', rows: ['azertyuiop', 'qsdfghjklm', 'wxcvbn'], tail: ["'", ',', '.'], xkb: ['fr', 'be'] },
    { id: 'de', name: 'Deutsch (QWERTZ)', short: 'DE', lang: 'de', rows: ['qwertzuiopü', 'asdfghjklöä', 'yxcvbnmß'], tail: [',', '.'], xkb: ['de', 'at', 'ch'] },
    { id: 'es', name: 'Español (QWERTY)', short: 'ES', lang: 'es', rows: ['qwertyuiop', 'asdfghjklñ', 'zxcvbnm'], tail: [',', '.'], xkb: ['es', 'latam', 'mx', 'ar'] },
    { id: 'sv', name: 'Svenska (QWERTY)', short: 'SV', lang: 'sv', rows: ['qwertyuiopå', 'asdfghjklöä', 'zxcvbnm'], tail: [',', '.'], xkb: ['se'] },
    { id: 'fi', name: 'Suomi (QWERTY)', short: 'FI', lang: 'fi', rows: ['qwertyuiopå', 'asdfghjklöä', 'zxcvbnm'], tail: [',', '.'], xkb: ['fi'] },
    { id: 'da', name: 'Dansk (QWERTY)', short: 'DA', lang: 'da', rows: ['qwertyuiopå', 'asdfghjklæø', 'zxcvbnm'], tail: [',', '.'], xkb: ['dk'] },
    { id: 'nb', name: 'Norsk (QWERTY)', short: 'NO', lang: 'nb', rows: ['qwertyuiopå', 'asdfghjkløæ', 'zxcvbnm'], tail: [',', '.'], xkb: ['no'] },
    { id: 'pt', name: 'Português (QWERTY)', short: 'PT', lang: 'pt', rows: ['qwertyuiop', 'asdfghjklç', 'zxcvbnm'], tail: [',', '.'], xkb: ['pt', 'br'] },
    { id: 'it', name: 'Italiano (QWERTY)', short: 'IT', lang: 'it', rows: ['qwertyuiop', 'asdfghjkl', 'zxcvbnm'], tail: [',', '.'], xkb: ['it'] }
  ];
  function layoutById(id) {
    for (var i = 0; i < LAYOUTS.length; i++) if (LAYOUTS[i].id === id) return LAYOUTS[i];
    return null;
  }
  /** The keyboard layout for a system (XKB) layout code ("us", "de", "fr(azerty)"...), or null. */
  function layoutForXkb(code) {
    var c = String(code || '').toLowerCase().split(/[(:+]/)[0].trim();
    for (var i = 0; i < LAYOUTS.length; i++) if (LAYOUTS[i].xkb.indexOf(c) >= 0) return LAYOUTS[i].id;
    return null;
  }
  /** The keyboard layout for a language tag ("de-AT", "pt_BR"), or null. */
  function layoutForLang(tag) {
    var l = String(tag || '').toLowerCase().split(/[-_.]/)[0];
    if (l === 'no' || l === 'nn') l = 'nb';
    for (var i = 0; i < LAYOUTS.length; i++) if (LAYOUTS[i].lang === l) return LAYOUTS[i].id;
    return null;
  }

  // Long-press (hold A, or View) alternates. Each language puts its own letters first (PREF).
  var ALTS = {
    a: 'àáâäæãåāą', c: 'çćč', d: 'ðď', e: 'éèêëēęėě', g: 'ğ', i: 'íìîïīįı', l: 'łľ', n: 'ñńň', o: 'óòôöõøœō',
    r: 'řŕ', s: 'ßśšş', t: 'þťț', u: 'úùûüūůű', y: 'ýÿ', z: 'žźż',
    '.': '…·•', ',': ';', '-': '–—_', '?': '¿', '!': '¡', "'": '‘’‚`', '"': '“”„«»', '$': '€£¥¢₹',
    '0': '°∅', '1': '¹½¼', '2': '²', '3': '³¾', '%': '‰', '&': '§¶', '=': '≠≈±', '<': '≤«‹', '>': '≥»›',
    '/': '\\÷', '*': '×•★', '+': '±', '(': '[{<', ')': ']}>', '@': '©®™', '#': '№', '~': '≈'
  };
  var PREF = { fr: 'éèêàçùâîôûëïœæ', de: 'äöüß', es: 'áéíóúñü¿¡', pt: 'ãáàâçéêíóôõú', it: 'àèéìòù', sv: 'åäöé', fi: 'äöå', da: 'æøåé', nb: 'æøåé' };

  // The Ember Wheel (fast mode): eight petals of four, one picked with the left stick, its four typed with A B X Y
  // (the order here is the buttons': Y top, X left, B right, A bottom). Steam Big Picture's old "Daisywheel", Hearth's way.
  var WHEEL = {
    letters: ['abcd', 'efgh', 'ijkl', 'mnop', 'qrst', 'uvwx', 'yz,.', "?!'-"],
    symbols: ['1234', '5678', '90@#', '$%&*', '()-_', '=+/\\', ':;"\'', '<>[]']
  };
  var WHEEL_BTN = ['y', 'x', 'b', 'a'];

  // Emoji: a pick from Hearth's Emoji & symbols data (hosts/hearth-emoji/ui/emoji-data.js), by group.
  var EMOJI = {
    smileys: "😀 😃 😄 😁 😆 😅 🤣 😂 🙂 🙃 🫠 😉 😊 😇 🥰 😍 🤩 😘 😗 ☺️ 😚 😙 🥲 😋 😛 😜 🤪 😝 🤑 🤗 🤭 🫢 🫣 🤫 🤔 🫡 🤐 🤨 😐 😑 😶 🫥 😏 😒 🙄 😬 🤥 🫨 😌 😔 😪 🤤 😴 😷 🤒 🤕 🤢 🤮 🤧 🥵 🥶 🥴 😵 🤯 🤠 🥳 🥸 😎 🤓 🧐 😕 🫤 😟 🙁 ☹️ 😮 😯 😲 😳 🥺",
    people: "👋 🤚 🖐️ ✋ 🖖 🫱 🫲 🫳 🫴 🫷 🫸 👌 🤌 🤏 ✌️ 🤞 🫰 🤟 🤘 🤙 👈 👉 👆 🖕 👇 ☝️ 🫵 👍 👎 ✊ 👊 🤛 🤜 👏 🙌 🫶 👐 🤲 🤝 🙏 ✍️ 💅 🤳 💪 🦾 🦿 🦵 🦶 👂 🦻",
    nature: "🐵 🐒 🦍 🦧 🐶 🐕 🦮 🐩 🐺 🦊 🦝 🐱 🐈 🦁 🐯 🐅 🐆 🐴 🫎 🫏 🐎 🦄 🦓 🦌 🦬 🐮 🐂 🐃 🐄 🐷 🐖 🐗 🐽 🐏 🐑 🐐 🐪 🐫 🦙 🦒 🐘 🦣 🦏 🦛 🐭 🐁 🐀 🐹 🐰 🐇 🐿️ 🦫 🦔 🦇 🐻 🐨 🐼 🦥 🦦 🦨",
    food: "🍇 🍈 🍉 🍊 🍋 🍌 🍍 🥭 🍎 🍏 🍐 🍑 🍒 🍓 🫐 🥝 🍅 🫒 🥥 🥑 🍆 🥔 🥕 🌽 🌶️ 🫑 🥒 🥬 🥦 🧄 🧅 🥜 🫘 🌰 🫚 🫛 🍞 🥐 🥖 🫓 🥨 🥯 🥞 🧇 🧀 🍖 🍗 🥩 🥓 🍔 🍟 🍕 🌭 🥪 🌮 🌯 🫔 🥙 🧆 🥚",
    activities: "🎃 🎄 🎆 🎇 🧨 ✨ 🎈 🎉 🎊 🎋 🎍 🎎 🎏 🎐 🎑 🧧 🎀 🎁 🎗️ 🎟️ 🎫 🎖️ 🏆 🏅 🥇 🥈 🥉 ⚽ ⚾ 🥎 🏀 🏐 🏈 🏉 🎾 🥏 🎳 🏏 🏑 🏒 🥍 🏓 🏸 🥊 🥋 🥅 ⛳ ⛸️ 🎣 🤿",
    travel: "🌍 🌎 🌏 🌐 🗺️ 🗾 🧭 🏔️ ⛰️ 🌋 🗻 🏕️ 🏖️ 🏜️ 🏝️ 🏞️ 🏟️ 🏛️ 🏗️ 🧱 🪨 🪵 🛖 🏘️ 🏚️ 🏠 🏡 🏢 🏣 🏤 🏥 🏦 🏨 🏩 🏪 🏫 🏬 🏭 🏯 🏰 💒 🗼 🗽 ⛪ 🕌 🛕 🕍 ⛩️ 🕋 ⛲",
    objects: "👓 🕶️ 🥽 🥼 🦺 👔 👕 👖 🧣 🧤 🧥 🧦 👗 👘 🥻 🩱 🩲 🩳 👙 👚 🪭 👛 👜 👝 🛍️ 🎒 🩴 👞 👟 🥾 🥿 👠 👡 🩰 👢 🪮 👑 👒 🎩 🎓 🧢 🪖 ⛑️ 📿 💄 💍 💎 🔇 🔈 🔉 🔊 📢 📣 📯 🔔 🔕 🎼 🎵 🎶 🎙️",
    symbols: "🏧 🚮 🚰 ♿ 🚹 🚺 🚻 🚼 🚾 🛂 🛃 🛄 🛅 ⚠️ 🚸 ⛔ 🚫 🚳 🚭 🚯 🚱 🚷 📵 🔞 ☢️ ☣️ ⬆️ ↗️ ➡️ ↘️ ⬇️ ↙️ ⬅️ ↖️ ↕️ ↔️ ↩️ ↪️ ⤴️ ⤵️ 🔃 🔄 🔙 🔚 🔛 🔜 🔝 🛐 ⚛️ 🕉️",
    kaomoji: "(＾▽＾)\t(◕‿◕)\t٩(◕‿◕)۶\t(ﾉ◕ヮ◕)ﾉ*:･ﾟ✧\t＼(＾▽＾)／\t(*^‿^*)\t(≧◡≦)\t(ᵔᴥᵔ)\tʕ•ᴥ•ʔ\t(=^･ω･^=)\t(^・ω・^ )\tU・ᴥ・U\t(╥﹏╥)\t(ಥ﹏ಥ)\t(._.)\t(｡•́︿•̀｡)\t(╯°□°)╯︵ ┻━┻\t┬─┬ノ( º _ ºノ)\t(ノಠ益ಠ)ノ彡┻━┻\tಠ_ಠ\t(¬_¬)\t(￣ヘ￣)\t(︶︹︶)\t¯\\_(ツ)_/¯\t(・_・;)\t(⊙_⊙)\t(°ロ°)\t(￣▽￣)ノ\t(^_^)/\t(｡･∀･)ﾉﾞ"
  };
  var EMOJI_CATS = [['recent', '🕘', 'Recent'], ['smileys', '😀', 'Smileys'], ['people', '👋', 'People'], ['nature', '🐶', 'Nature'], ['food', '🍔', 'Food'],
    ['activities', '⚽', 'Activities'], ['travel', '🚗', 'Travel'], ['objects', '💡', 'Objects'], ['symbols', '❤️', 'Symbols'], ['kaomoji', '(^‿^)', 'Kaomoji']];
  function emojiList(cat) {
    if (cat === 'recent') return STATE.recentEmoji.slice();
    var s = EMOJI[cat] || '';
    return s ? s.split(cat === 'kaomoji' ? '\t' : ' ') : [];
  }

  var THEMES = [['hearth', 'Hearth'], ['ember', 'Ember glow'], ['slate', 'Slate'], ['daylight', 'Daylight']];
  var DEFAULTS = {
    layouts: ['auto'],        // 'auto' (the system's keyboard layouts) or layout ids, in order (the globe key cycles)
    suggestions: true,        // word suggestions above the keys
    learn: true,              // remember words you type (only on this device; Clear learned words forgets them)
    autocap: true,            // a capital at the start of a sentence
    doubleSpace: true,        // two spaces type ". "
    fast: 'wheel',            // fast typing: 'wheel' (Ember Wheel on the left stick), 'split' (split sticks), 'off'
    shape: 'full',            // 'full', or 'split' (halves at the edges, for thumbs on a handheld's touchscreen)
    size: 'm',                // 's', 'm', 'l'
    position: 'bottom',       // 'bottom' or 'top'
    theme: 'hearth',
    haptics: 'light',         // 'off', 'light', 'strong'
    trail: true               // the ember glow follows the selection with a short trail
  };

  function settings() {
    var s = {}, k;
    for (k in DEFAULTS) s[k] = DEFAULTS[k];
    if (STATE.settings) for (k in STATE.settings) if (STATE.settings[k] !== undefined && k in DEFAULTS) s[k] = STATE.settings[k];
    if (!Array.isArray(s.layouts) || !s.layouts.length) s.layouts = ['auto'];
    return s;
  }
  function notifyAll() { for (var i = 0; i < STATE.listeners.length; i++) try { STATE.listeners[i](); } catch (e) { /* one view's problem */ } }

  /** The layouts in use, in order (resolving 'auto' from the system's layouts, then the language). */
  function activeLayouts() {
    var s = settings(), out = [];
    for (var i = 0; i < s.layouts.length; i++) {
      var id = s.layouts[i];
      if (id === 'auto') {
        for (var j = 0; j < STATE.systemLayouts.length; j++) { var x = layoutForXkb(STATE.systemLayouts[j]); if (x && out.indexOf(x) < 0) out.push(x); }
        if (!out.length) {
          var nav = (typeof navigator !== 'undefined' && navigator.language) ? navigator.language : '';
          var y = layoutForLang(nav); if (y) out.push(y);
        }
      } else if (layoutById(id) && out.indexOf(id) < 0) out.push(id);
    }
    if (!out.length) out.push('en-us');
    return out;
  }

  // ------------------------------------------------------------------------------------------------ text helpers
  function isLetter(c) { return !!c && (c.toLowerCase() !== c.toUpperCase() || c === 'ß' || c === 'ı'); }
  function isWordChar(c) { return isLetter(c) || c === "'" || c === '’'; }
  function fold(w) {
    var s = String(w).toLowerCase();
    if (s.normalize) s = s.normalize('NFD').replace(/[̀-ͯ]/g, '');
    return s;
  }
  /** The part of a word right before the caret ("hel" in "say hel"). */
  function wordBefore(text) {
    var i = text.length;
    while (i > 0 && isWordChar(text.charAt(i - 1))) i--;
    var w = text.slice(i);
    while (w && (w.charAt(0) === "'" || w.charAt(0) === '’')) w = w.slice(1);
    return w;
  }
  /** Characters as a person sees them: an emoji with its variation selector or skin tone is one. */
  function graphemes(s) {
    var cps = Array.from ? Array.from(s) : s.split(''), out = [];
    for (var i = 0; i < cps.length; i++) {
      var c = cps[i], cp = c.codePointAt ? c.codePointAt(0) : c.charCodeAt(0);
      var joins = cp === 0xFE0F || cp === 0x200D || (cp >= 0x1F3FB && cp <= 0x1F3FF) || (cp >= 0x0300 && cp <= 0x036F) || cp === 0x20E3;
      if (out.length && (joins || out[out.length - 1].slice(-1) === '‍')) out[out.length - 1] += c;
      else out.push(c);
    }
    return out;
  }
  function dropLast(s, n) { var g = graphemes(s); return g.slice(0, Math.max(0, g.length - n)).join(''); }
  function upper(c) { return c === 'ß' ? c : c.toUpperCase(); }
  function caseLike(word, typed) {
    if (!typed) return word;
    if (typed.length > 1 && typed === typed.toUpperCase() && typed !== typed.toLowerCase()) return word.toUpperCase();
    if (typed.charAt(0) !== typed.charAt(0).toLowerCase()) return word.charAt(0).toUpperCase() + word.slice(1);
    return word;
  }

  // ------------------------------------------------------------------------------------------------ words
  /** A language's word list: words separated by spaces, most used first (ui/osk/words/<lang>.js calls this). */
  function addWords(lang, text) {
    var list = String(text || '').split(/\s+/).filter(Boolean);
    var folded = new Array(list.length), has = {};
    for (var i = 0; i < list.length; i++) { folded[i] = fold(list[i]); has[folded[i]] = true; }
    STATE.dict[lang] = { list: list, folded: folded, has: has };
    notifyAll();
  }
  var wordsAsked = {};
  function needWords(lang) {
    if (STATE.dict[lang] || wordsAsked[lang]) return;
    wordsAsked[lang] = true;
    if (STATE.loadWords) try { STATE.loadWords(lang); } catch (e) { /* no suggestions for it */ }
  }
  var saveTimer = null;
  function learnedChanged() {
    if (!STATE.save) return;
    var later = typeof setTimeout === 'function' ? setTimeout : null;
    if (!later) { STATE.save(exportLearned()); return; }
    if (saveTimer) return;
    saveTimer = later(function () { saveTimer = null; try { STATE.save(exportLearned()); } catch (e) { /* kept for next time */ } }, 1500);
  }
  function learnWord(lang, word, prev) {
    if (!word || word.length < 2 || word.length > 40 || !isLetter(word.charAt(0))) return;
    // "Hello" at the start of a sentence is "hello" (a name typed mid-sentence too: suggestions take your case).
    var key = word.charAt(0) !== word.charAt(0).toLowerCase() && word.slice(1) === word.slice(1).toLowerCase() ? word.toLowerCase() : word;
    var L = STATE.learned[lang] || (STATE.learned[lang] = {});
    L[key] = Math.min(9999, (L[key] || 0) + 1);
    if (prev) {
      var N = STATE.next[lang] || (STATE.next[lang] = {}), p = prev.toLowerCase();
      var m = N[p] || (N[p] = {});
      m[key] = Math.min(9999, (m[key] || 0) + 1);
    }
    learnedChanged();
  }
  function exportLearned() { return { version: 1, learned: STATE.learned, next: STATE.next, recentEmoji: STATE.recentEmoji }; }
  function importLearned(d) {
    if (!d || typeof d !== 'object') return;
    if (d.learned && typeof d.learned === 'object') STATE.learned = d.learned;
    if (d.next && typeof d.next === 'object') STATE.next = d.next;
    if (Array.isArray(d.recentEmoji)) STATE.recentEmoji = d.recentEmoji.slice(0, 30);
    notifyAll();
  }
  function clearLearned() {
    STATE.learned = {}; STATE.next = {}; STATE.recentEmoji = [];
    if (STATE.save) try { STATE.save(exportLearned()); } catch (e) { /* tried */ }
    notifyAll();
  }
  function learnedCount() {
    var n = 0;
    for (var l in STATE.learned) for (var w in STATE.learned[l]) if (STATE.learned[l].hasOwnProperty(w)) n++;
    return n;
  }
  /** Up to `max` words for what's typed (`typed`, may be empty: then the words that usually come after `prev`). */
  function suggest(lang, typed, prev, max) {
    max = max || 3;
    var out = [], seen = {};
    function add(w) { var k = w.toLowerCase(); if (seen[k] || k === typed.toLowerCase()) return; seen[k] = true; out.push(caseLike(w, typed)); }
    var L = STATE.learned[lang] || {};
    if (!typed) {
      var N = STATE.next[lang] && prev ? STATE.next[lang][prev.toLowerCase()] : null;
      if (!N) return [];
      Object.keys(N).sort(function (a, b) { return N[b] - N[a]; }).slice(0, max).forEach(add);
      return out;
    }
    var f = fold(typed), cands = [], D = STATE.dict[lang];
    // A word you typed once could be a typo: it's suggested once it's typed twice (or it's a known word).
    for (var w in L) if (L.hasOwnProperty(w) && (L[w] >= 2 || (D && D.has[fold(w)])) && fold(w).indexOf(f) === 0) cands.push([w, 1e7 + L[w] * 1000]);
    if (D) {
      var n = D.list.length;
      for (var i = 0; i < n && cands.length < 400; i++) if (D.folded[i].indexOf(f) === 0) cands.push([D.list[i], n - i]);
    }
    cands.sort(function (a, b) { return b[1] - a[1]; });
    for (var j = 0; j < cands.length && out.length < max; j++) add(cands[j][0]);
    return out;
  }

  // ------------------------------------------------------------------------------------------------ keys
  function K(t, v, label, w, extra) { var k = { t: t, v: v === undefined ? '' : v, label: label === undefined ? String(v) : label, w: w || 1 }; if (extra) for (var x in extra) k[x] = extra[x]; return k; }
  function chars(s) { var out = [], a = Array.from ? Array.from(s) : s.split(''); for (var i = 0; i < a.length; i++) out.push(K('ch', a[i])); return out; }
  var SYM1 = ['!@#$%^&*()', '-_=+[]{};', "'\"/\\|~`:?"];
  var SYM2 = ['€£¥¢§°•¶©®', '±×÷≠≈<>«»', '¡¿…“”‘’№™'];
  var LABELS = { bksp: '⌫', enter: '↵', shift: '⇧', sym: '&123', abc: 'ABC', sym2: '=\\<', sym1: '?123', emoji: '☺', clip: '📋', layout: '🌐',
    space: '', left: '‹', right: '›', up: '⌃', down: '⌄', hide: '⌨', tab: 'Tab', esc: 'Esc', ctrl: 'Ctrl', alt: 'Alt', home: 'Home', end: 'End', paste: 'Paste' };
  // Controller shortcuts, shown on the keys while a controller is used.
  var GLYPH = { bksp: 'X', space: 'Y', shift: 'LT', sym: 'RT', abc: 'RT', enter: '☰', hide: 'B', left: 'LB', right: 'RB', emoji: 'L3', layout: 'R3' };

  // ------------------------------------------------------------------------------------------------ the engine
  /**
   * One keyboard. opts:
   *   adapter: where typing goes: {insert(text), deleteBack(n), key(name, mods), context()?}
   *            key names: enter, tab, esc, left, right, up, down, home, end, paste; mods {ctrl, alt, shift}.
   *            context() gives the text before the caret ('' or undefined when unknown: then the keyboard keeps its own).
   *   mode: 'text' (default), 'search', 'url', 'email', 'password', 'number', 'terminal'
   *   enterLabel: the Enter key's label ('Done', 'Search', 'Go'...)
   *   onChange(): redraw; onHide(): B or the hide key; onHaptic(strength, ms); onSound(kind); onLeave(dir): the
   *   selection went past the top (up): move it to what's above.
   *   host: 'web' | 'mantel' | 'browser' (a few buttons differ: in Mantel the right stick and RT are the pointer's)
   *   suggest(typed, previousWord): your own suggestions instead of the word list's (Game Mode's search: game titles)
   */
  function createEngine(opts) {
    opts = opts || {};
    var E = {
      mode: opts.mode || 'text', enterLabel: opts.enterLabel || '', panel: 'letters', layoutIx: 0,
      shift: false, caps: false, autoShift: false, ctrl: false, alt: false,
      sel: { r: 1, c: 0 }, active: true, hints: false,
      popup: null,           // {key, alts, i, replace}
      emojiCat: 'smileys', emojiPage: 0, symPage: 1,
      sugg: [], wheel: null, split: { l: null, r: null },
      shadow: '', shadowKnown: false, autoSpace: false, lastTyped: '',
      held: {}, stick: { lx: 0, ly: 0, rx: 0, ry: 0 }, stickNav: null,
      ltAt: 0, ltHeld: false, ltUsed: false, changed: []
    };
    var host = opts.host || 'web';
    var listeners = [];
    var cachedRows = null;   // the keys on screen, worked out again after each change
    if (opts.onChange) listeners.push(opts.onChange);
    function changed() { cachedRows = null; for (var i = 0; i < listeners.length; i++) try { listeners[i](); } catch (e) { /* a view */ } }
    function now() { return Date.now(); }
    function S() { return settings(); }
    function layout() { var ids = activeLayouts(); return layoutById(ids[E.layoutIx % ids.length]) || LAYOUTS[0]; }
    function lang() { return layout().lang; }
    function wordsOn() { return S().suggestions && (E.mode === 'text' || E.mode === 'search'); }
    function learnOn() { return S().learn && (E.mode === 'text' || E.mode === 'search'); }
    function autocapOn() { return S().autocap && E.mode === 'text'; }

    // ---- the text around the caret
    function ctx() {
      var a = opts.adapter;
      if (a && typeof a.context === 'function') { var c = a.context(); if (typeof c === 'string') return c; }
      return E.shadow;
    }
    function ctxKnown() { var a = opts.adapter; return !!(a && typeof a.context === 'function' && typeof a.context() === 'string') || E.shadowKnown; }
    function insert(text) {
      if (!text) return;
      var a = opts.adapter;
      if (a && a.insert) a.insert(text, (E.ctrl || E.alt) ? { ctrl: E.ctrl, alt: E.alt } : null);
      E.shadow = (E.shadow + text).slice(-200);
      if (E.ctrl || E.alt) { E.ctrl = false; E.alt = false; E.shadow = ''; E.shadowKnown = false; }
    }
    function deleteBack(n) {
      n = n || 1;
      var a = opts.adapter;
      if (a && a.deleteBack) a.deleteBack(n);
      E.shadow = dropLast(E.shadow, n);
    }
    function sendKey(name) {
      E.lastTyped = '';
      var a = opts.adapter, mods = (E.ctrl || E.alt) ? { ctrl: E.ctrl, alt: E.alt } : null;
      if (a && a.key) a.key(name, mods);
      E.ctrl = false; E.alt = false;
      if (name !== 'paste') { E.shadow = ''; E.shadowKnown = name === 'enter'; } else { E.shadowKnown = false; }
    }

    // ---- after every edit: capitals and suggestions
    function refresh() {
      var before = ctx();
      if (autocapOn() && !E.caps && ctxKnown()) {
        var start = before === '' || /[.!?…]\s+$/.test(before) || /\n\s*$/.test(before);
        if (start && !E.shift) { E.shift = true; E.autoShift = true; }
        else if (!start && E.autoShift) { E.shift = false; E.autoShift = false; }
      }
      if (wordsOn() && E.panel === 'letters') {
        needWords(lang());
        var w = wordBefore(before), prevWord = '';
        if (!w && /[ \t]$/.test(before)) prevWord = wordBefore(before.replace(/\s+$/, ''));
        var own = opts.suggest ? opts.suggest(w, prevWord) : null;
        E.sugg = own ? own.slice(0, 3) : suggest(lang(), w, prevWord, 3);
      } else E.sugg = [];
      if (E.sel.r === -1 && !E.sugg.length) E.sel = { r: 0, c: 0 };
      if (E.sel.r === -1 && E.sel.c >= E.sugg.length) E.sel.c = E.sugg.length - 1;
    }
    function haptic(kind) {
      var h = S().haptics; if (h === 'off' || !opts.onHaptic) return;
      var strong = h === 'strong';
      try { opts.onHaptic(kind === 'big' ? (strong ? 0.9 : 0.55) : (strong ? 0.5 : 0.25), kind === 'big' ? 40 : 18); } catch (e) { /* no rumble */ }
    }
    function sound(kind) { if (opts.onSound) try { opts.onSound(kind); } catch (e) { /* quiet */ } }

    // ---- typing a character (shift, learning, double space, the space after a picked word)
    function typeChar(c) {
      var before = ctx();
      if (E.shift || E.caps) c = upper(c);
      if (E.ltHeld) E.ltUsed = true;
      // A punctuation mark right after a picked word: before its space ("word. ", not "word .").
      if (E.autoSpace && /^[.,!?;:)]$/.test(c) && / $/.test(before)) { deleteBack(1); insert(c + (/[.,!?;:]/.test(c) ? ' ' : '')); E.autoSpace = false; E.lastTyped = ''; after(); return; }
      E.autoSpace = false;
      if (!isWordChar(c)) finishWord(before);
      insert(c); E.lastTyped = c;
      if (E.shift && !E.caps && !E.ltHeld) { E.shift = false; E.autoShift = false; }
      after();
    }
    function typeSpace() {
      var before = ctx();
      if (E.ltHeld) E.ltUsed = true;
      // Space after a picked word (which brought its own space): ". " with double-space on, else nothing more.
      if (E.autoSpace && / $/.test(before)) {
        E.autoSpace = false;
        if (S().doubleSpace && E.mode === 'text') { deleteBack(1); insert('. '); E.lastTyped = '.'; }
        after(); return;
      }
      if (S().doubleSpace && E.mode === 'text' && / $/.test(before) && /[^\s.!?,;:]$/.test(before.slice(0, -1)) && !E.autoSpace && E.lastTyped === ' ') {
        deleteBack(1); insert('. '); E.lastTyped = '.'; after(); return;
      }
      E.autoSpace = false;
      finishWord(before);
      insert(' '); E.lastTyped = ' ';
      after();
    }
    function finishWord(before) {
      if (!learnOn()) return;
      var w = wordBefore(before);
      if (!w) return;
      var rest = before.slice(0, before.length - w.length).replace(/\s+$/, '');
      learnWord(lang(), w.replace(/[’']+$/, ''), wordBefore(rest));
    }
    function after() { haptic('tap'); sound('key'); refresh(); changed(); }

    function pickSuggestion(i) {
      var w = E.sugg[i]; if (!w) return;
      var before = ctx(), part = wordBefore(before);
      if (part) deleteBack(graphemes(part).length);
      insert(w + ' ');
      if (learnOn()) learnWord(lang(), w, wordBefore(before.slice(0, before.length - part.length).replace(/\s+$/, '')));
      E.autoSpace = true; E.lastTyped = '';
      if (E.shift && !E.caps) { E.shift = false; E.autoShift = false; }
      haptic('tap'); sound('key'); refresh(); changed();
    }

    // ---- the keys on screen
    function letterRows() {
      var L = layout(), s = S(), rows = [];
      var term = E.mode === 'terminal';
      if (term) rows.push([K('esc', '', LABELS.esc, 1.2), K('tab', '', LABELS.tab, 1.2), K('ctrl', '', LABELS.ctrl, 1.2, { on: E.ctrl }), K('alt', '', LABELS.alt, 1.2, { on: E.alt }),
        K('ch', '|'), K('ch', '~'), K('ch', '/'), K('ch', '-'), K('home', '', LABELS.home, 1.1), K('end', '', LABELS.end, 1.1), K('up', '', '↑'), K('down', '', '↓')]);
      rows.push(chars('1234567890').concat([K('bksp', '', LABELS.bksp, 1.5)]));
      rows.push(chars(L.rows[0]));
      rows.push(chars(L.rows[1]).concat([K('enter', '', E.enterLabel || LABELS.enter, 1.6, { go: true })]));
      var third = [K('shift', '', LABELS.shift, 1.4, { on: E.shift || E.caps, lock: E.caps })].concat(chars(L.rows[2]));
      var tail = L.tail;
      if (E.mode === 'email') tail = ['@', '.'];
      else if (E.mode === 'url') tail = ['/', '.'];
      third = third.concat(chars(tail.join(''))).concat([K('shift', 'r', LABELS.shift, 1.2, { on: E.shift || E.caps, lock: E.caps })]);
      rows.push(third);
      rows.push(bottomRow());
      return rows;
    }
    function bottomRow() {
      var many = activeLayouts().length > 1, r = [K('sym', '', LABELS.sym, 1.5)];
      if (many) r.push(K('layout', '', layout().short, 1.1));
      if (E.mode === 'url') r.push(K('txt', '.com', '.com', 1.3), K('ch', ':'));
      else if (E.mode === 'email') r.push(K('txt', '.com', '.com', 1.3), K('ch', '-'));
      else if (E.mode !== 'password' && E.mode !== 'number') r.push(K('emoji', '', LABELS.emoji, 1.1));
      r.push(K('space', ' ', layout().name, many ? 4.4 : 5.4), K('left', '', LABELS.left), K('right', '', LABELS.right), K('hide', '', LABELS.hide, 1.3));
      return r;
    }
    function symRows() {
      var P = E.symPage === 2 ? SYM2 : SYM1, rows = [];
      rows.push(chars('1234567890').concat([K('bksp', '', LABELS.bksp, 1.5)]));
      rows.push(chars(P[0]));
      rows.push(chars(P[1]).concat([K('enter', '', E.enterLabel || LABELS.enter, 1.6, { go: true })]));
      rows.push([K('sym2', '', E.symPage === 2 ? LABELS.sym1 : LABELS.sym2, 1.4)].concat(chars(P[2])).concat([K('clip', '', LABELS.clip, 1.2)]));
      var b = [K('abc', '', LABELS.abc, 1.5)];
      if (E.mode !== 'password') b.push(K('emoji', '', LABELS.emoji, 1.1));
      if (host !== 'web') b.push(K('paste', '', LABELS.paste, 1.3));
      b.push(K('space', ' ', '', 4.6), K('left', '', LABELS.left), K('right', '', LABELS.right), K('hide', '', LABELS.hide, 1.3));
      rows.push(b);
      return rows;
    }
    var EMOJI_PER = 30;
    function emojiRows() {
      var rows = [], cats = [];
      for (var i = 0; i < EMOJI_CATS.length; i++) cats.push(K('cat', EMOJI_CATS[i][0], EMOJI_CATS[i][1], EMOJI_CATS[i][0] === 'kaomoji' ? 1.4 : 1, { on: EMOJI_CATS[i][0] === E.emojiCat, title: EMOJI_CATS[i][2] }));
      rows.push(cats.concat([K('bksp', '', LABELS.bksp, 1.5)]));
      var list = emojiList(E.emojiCat), pages = Math.max(1, Math.ceil(list.length / EMOJI_PER));
      if (E.emojiPage >= pages) E.emojiPage = pages - 1;
      var page = list.slice(E.emojiPage * EMOJI_PER, (E.emojiPage + 1) * EMOJI_PER);
      var per = E.emojiCat === 'kaomoji' ? 5 : 10;
      for (var r = 0; r < 3; r++) {
        var row = [];
        for (var c = 0; c < per; c++) { var e = page[r * per + c]; row.push(e ? K('txt', e, e, 1, { emoji: true }) : K('none', '', '', 1)); }
        rows.push(row);
      }
      if (!list.length) rows[1] = [K('none', '', E.emojiCat === 'recent' ? 'Emoji you pick show here' : '', 10)];
      rows.push([K('abc', '', LABELS.abc, 1.5), K('prev', '', '◀', 1, { off: E.emojiPage === 0 }), K('page', '', (E.emojiPage + 1) + ' / ' + pages, 1.4),
        K('next', '', '▶', 1, { off: E.emojiPage >= pages - 1 }), K('space', ' ', '', 3.6), K('left', '', LABELS.left), K('right', '', LABELS.right), K('hide', '', LABELS.hide, 1.3)]);
      return rows;
    }
    function clipRows() {
      var items = STATE.clipboard.slice(0, 6), rows = [];
      if (opts.clipboard) try { var got = opts.clipboard(); if (Array.isArray(got)) items = got.slice(0, 6); } catch (e) { /* none */ }
      for (var r = 0; r < 3; r++) {
        var row = [];
        for (var c = 0; c < 2; c++) {
          var it = items[r * 2 + c];
          row.push(it ? K('txt', it, it.replace(/\s+/g, ' ').slice(0, 40) + (it.length > 40 ? '…' : ''), 1, { clip: true }) : K('none', '', r === 0 && c === 0 && !items.length ? 'Nothing copied yet' : '', 1));
        }
        rows.push(row);
      }
      var b = [K('abc', '', LABELS.abc, 1.5)];
      if (host !== 'web') b.push(K('paste', '', LABELS.paste, 1.3));
      b.push(K('space', ' ', '', 4), K('bksp', '', LABELS.bksp, 1.5), K('hide', '', LABELS.hide, 1.3));
      rows.push(b);
      return rows;
    }
    function numRows() {
      return [chars('123').concat([K('bksp', '', LABELS.bksp, 1.2)]), chars('456').concat([K('ch', '+', '+', 1.2)]),
        chars('789').concat([K('enter', '', E.enterLabel || LABELS.enter, 1.2, { go: true })]),
        [K('ch', '-', '−'), K('ch', '0'), K('ch', '.'), K('hide', '', LABELS.hide, 1.2)]];
    }
    function rows() {
      if (cachedRows) return cachedRows;
      var r = E.panel === 'sym' ? symRows() : E.panel === 'emoji' ? emojiRows() : E.panel === 'clip' ? clipRows() : E.panel === 'num' ? numRows() : letterRows();
      if (S().shape === 'split' && E.panel !== 'num') r = splitRows(r);
      for (var i = 0; i < r.length; i++) for (var j = 0; j < r[i].length; j++) { r[i][j].row = i; r[i][j].col = j; }
      cachedRows = r;
      return r;
    }
    // Split shape: space in both halves, the rest where it falls.
    function splitRows(R) {
      return R.map(function (row) {
        var tot = 0, x = 0, out = [];
        row.forEach(function (k) { tot += k.w; });
        row.forEach(function (k) {
          var mid = (x + k.w / 2) / tot; x += k.w;
          if (k.t === 'space') { out.push(K('space', ' ', '', k.w / 2, { half: 'l' })); out.push(K('space', ' ', '', k.w / 2, { half: 'r' })); }
          else { var c = {}; for (var p in k) c[p] = k[p]; c.half = mid < 0.5 ? 'l' : 'r'; out.push(c); }
        });
        return out;
      });
    }
    function dirty() { cachedRows = null; }
    function keyAt(r, c) { var R = rows(); return R[r] && R[r][c] ? R[r][c] : null; }
    function clampSel() {
      var R = rows();
      if (E.sel.r === -1) { if (!E.sugg.length) E.sel = { r: 0, c: 0 }; else return; }
      if (E.sel.r >= R.length) E.sel.r = R.length - 1;
      if (E.sel.r < 0) E.sel.r = 0;
      if (E.sel.c >= R[E.sel.r].length) E.sel.c = R[E.sel.r].length - 1;
      if (E.sel.c < 0) E.sel.c = 0;
    }
    function center(row, c) { var tot = 0, x = 0; for (var i = 0; i < row.length; i++) { if (i < c) x += row[i].w; tot += row[i].w; } return (x + row[c].w / 2) / tot; }
    function nearest(row, at) { var best = 0, d = 9; for (var i = 0; i < row.length; i++) { var dd = Math.abs(center(row, i) - at); if (dd < d) { d = dd; best = i; } } return best; }

    // ---- moving the selection
    function move(dir) {
      if (E.popup) {
        if (dir === 'left' || dir === 'right') { E.popup.i = (E.popup.i + (dir === 'right' ? 1 : -1) + E.popup.alts.length) % E.popup.alts.length; haptic('tap'); changed(); return true; }
        if (dir === 'down') { E.popup = null; changed(); return true; }
        return true;
      }
      var R = rows();
      clampSel();
      if (E.sel.r === -1) {
        var strip = E.sugg.map(function () { return { w: 1 }; });
        if (dir === 'left' || dir === 'right') { E.sel.c = (E.sel.c + (dir === 'right' ? 1 : -1) + E.sugg.length) % E.sugg.length; }
        else if (dir === 'down') { E.sel = { r: 0, c: nearest(R[0], center(strip, E.sel.c)) }; }
        else { if (opts.onLeave) { E.active = false; changed(); opts.onLeave('up'); return true; } return false; }
      } else if (dir === 'left' || dir === 'right') {
        var row = R[E.sel.r], c = E.sel.c, d = dir === 'right' ? 1 : -1;
        do { c = (c + d + row.length) % row.length; } while (row[c].t === 'none' && c !== E.sel.c);
        E.sel.c = c;
      } else {
        var at = center(R[E.sel.r], E.sel.c), nr = E.sel.r + (dir === 'down' ? 1 : -1);
        if (nr < 0) {
          if (E.sugg.length && E.panel === 'letters') { E.sel = { r: -1, c: Math.min(E.sugg.length - 1, Math.floor(at * E.sugg.length)) }; }
          else { if (opts.onLeave) { E.active = false; changed(); opts.onLeave('up'); return true; } return false; }
        } else if (nr >= R.length) { nr = 0; E.sel = { r: nr, c: nearest(R[nr], at) }; }
        else E.sel = { r: nr, c: nearest(R[nr], at) };
      }
      sound('move');
      changed();
      return true;
    }

    // ---- pressing a key
    function press(k, how) {
      if (!k) return;
      switch (k.t) {
        case 'ch': typeChar(k.v); return;
        case 'txt':
          if (E.ltHeld) E.ltUsed = true;
          E.autoSpace = false;
          if (k.emoji) { STATE.recentEmoji = [k.v].concat(STATE.recentEmoji.filter(function (x) { return x !== k.v; })).slice(0, 30); learnedChanged(); }
          insert(k.v); E.lastTyped = ''; after(); return;
        case 'space': typeSpace(); return;
        case 'bksp': backspace(); return;
        case 'enter': finishWord(ctx()); E.autoSpace = false; sendKey('enter'); haptic('tap'); sound('key'); dirty(); refresh(); changed(); return;
        case 'shift': shiftTap(); return;
        case 'sym': setPanel('sym'); return;
        case 'abc': setPanel('letters'); return;
        case 'sym2': E.symPage = E.symPage === 2 ? 1 : 2; dirty(); changed(); return;
        case 'emoji': setPanel(E.panel === 'emoji' ? 'letters' : 'emoji'); return;
        case 'clip': setPanel('clip'); return;
        case 'layout': nextLayout(); return;
        case 'left': case 'right': case 'up': case 'down': case 'home': case 'end': case 'tab': case 'esc':
          E.autoSpace = false; sendKey(k.t); haptic('tap'); refresh(); changed(); return;
        case 'paste': sendKey('paste'); refresh(); changed(); return;
        case 'ctrl': E.ctrl = !E.ctrl; dirty(); changed(); return;
        case 'alt': E.alt = !E.alt; dirty(); changed(); return;
        case 'hide': hide(); return;
        case 'cat': E.emojiCat = k.v; E.emojiPage = 0; dirty(); haptic('tap'); changed(); return;
        case 'prev': if (E.emojiPage > 0) { E.emojiPage--; dirty(); changed(); } return;
        case 'next': E.emojiPage++; dirty(); changed(); return;
        case 'sugg': pickSuggestion(k.v); return;
      }
    }
    function backspace() {
      var before = ctx();
      E.autoSpace = false; E.lastTyped = '';
      if (before === '' && ctxKnown() && opts.adapter && typeof opts.adapter.context === 'function') { haptic('tap'); changed(); return; }
      deleteBack(1);
      haptic('tap'); sound('key'); refresh(); changed();
    }
    function shiftTap() {
      var t = now();
      if (E.caps) { E.caps = false; E.shift = false; }
      else if (E.shift && t - E.ltAt < 450) { E.caps = true; E.shift = true; }
      else E.shift = !E.shift;
      E.autoShift = false; E.ltAt = t;
      dirty(); haptic('tap'); changed();
    }
    function setPanel(p) {
      if (E.panel === p) p = 'letters';
      E.panel = p; E.popup = null; dirty();
      var R = rows();
      if (E.sel.r === -1) E.sel = { r: 0, c: 0 };
      E.sel.r = Math.min(E.sel.r, R.length - 1);
      E.sel.c = Math.min(E.sel.c, R[E.sel.r].length - 1);
      if (p === 'emoji' && E.emojiCat === 'recent' && !STATE.recentEmoji.length) E.emojiCat = 'smileys';
      haptic('big'); sound('panel'); refresh(); changed();
    }
    function nextLayout() {
      var n = activeLayouts().length; if (n < 2) return;
      E.layoutIx = (E.layoutIx + 1) % n; dirty(); haptic('big'); refresh(); changed();
    }
    function hide() { E.popup = null; changed(); if (opts.onHide) opts.onHide(); }

    // ---- alternates (hold A, View, or a long touch)
    function altsFor(k) {
      if (!k || k.t !== 'ch') return null;
      var base = k.v.toLowerCase(), a = ALTS[base];
      if (!a) return null;
      var list = Array.from ? Array.from(a) : a.split(''), pref = PREF[lang()] || '';
      var order = list.slice();   // a stable sort: the language's own letters first, the rest as listed
      list.sort(function (x, y) { var i = pref.indexOf(x), j = pref.indexOf(y); return ((i < 0 ? 99 : i) - (j < 0 ? 99 : j)) || (order.indexOf(x) - order.indexOf(y)); });
      if (E.shift || E.caps) list = list.map(upper);
      return list;
    }
    function openPopup(k, replace, list) {
      var alts = list || altsFor(k); if (!alts) return false;
      E.popup = { key: k, alts: alts, i: 0, replace: !!replace };
      haptic('big'); changed();
      return true;
    }
    function pickAlt(i) {
      var p = E.popup; if (!p) return;
      var ch = p.alts[i === undefined ? p.i : i]; E.popup = null;
      if (p.replace) deleteBack(1);
      var keepShift = E.shift; if (p.replace) { E.shift = false; }
      var wasCaps = E.caps; E.caps = false;
      typeChar(ch);
      E.caps = wasCaps; if (p.replace && keepShift && wasCaps) E.shift = true;
    }

    // ---- controller buttons
    var REPEAT_FIRST = 380, REPEAT_MIN = 40;
    function repeatEvery(held) { return Math.max(REPEAT_MIN, 85 - Math.min(45, (held - REPEAT_FIRST) / 25)); }
    function act(btn, t) {
      // the Ember Wheel: with the left stick tilted, A B X Y type the petal's letters
      if (E.wheel && E.wheel.petal >= 0 && WHEEL_BTN.indexOf(btn) >= 0) {
        var set = E.panel === 'sym' ? WHEEL.symbols : WHEEL.letters;
        var ch = set[E.wheel.petal].charAt(WHEEL_BTN.indexOf(btn));
        if (ch) typeChar(ch);
        return true;
      }
      if (E.popup) {
        if (btn === 'a') { pickAlt(); return true; }
        if (btn === 'b' || btn === 'view') { E.popup = null; changed(); return true; }
      }
      switch (btn) {
        case 'up': case 'down': case 'left': case 'right': return move(btn);
        case 'a':
          if (!E.active) return false;
          if (E.sel.r === -1) { pickSuggestion(E.sel.c); return true; }
          press(keyAt(E.sel.r, E.sel.c), 'pad'); return true;
        case 'b': hide(); return true;
        case 'x': backspace(); return true;
        case 'y': typeSpace(); return true;
        case 'lb': press({ t: 'left' }); return true;
        case 'rb': press({ t: 'right' }); return true;
        case 'lt': E.ltHeld = true; E.ltUsed = false; shiftTap(); return true;
        case 'rt': if (host === 'mantel') return false; setPanel(E.panel === 'letters' ? 'sym' : 'letters'); return true;
        case 'l3': setPanel(E.panel === 'emoji' ? 'letters' : 'emoji'); return true;
        case 'r3': if (host === 'mantel') return false; nextLayout(); return true;
        case 'view': if (E.sel.r >= 0) openPopup(keyAt(E.sel.r, E.sel.c), false); return true;
        case 'menu': case 'start': press({ t: 'enter' }); return true;
      }
      return false;
    }
    /** A controller button went down (down=true) or up. t: milliseconds (Date.now() when left out). Returns
     *  whether the keyboard used it. Pages that only know presses can call handle(btn) alone: it's a press. */
    function handle(btn, down, t) {
      if (down === undefined) down = 'press';
      t = t || now();
      if (btn === 'start') btn = 'menu';
      if (btn === 'select') btn = 'view';
      // From a source that only says "pressed" (no release): act, nothing is held.
      if (down === 'press') {
        if (S().fast === 'split' && (btn === 'lb' || btn === 'rb')) { if (btn === 'lb') shiftTap(); else setPanel(E.panel === 'letters' ? 'sym' : 'letters'); return true; }
        if (btn === 'lt') { var u = act('lt', t); E.ltHeld = false; return u; }
        return act(btn, t);
      }
      // split sticks: the triggers type what each stick points at, LB/RB are shift and symbols
      if (S().fast === 'split' && (btn === 'lt' || btn === 'rt' || btn === 'lb' || btn === 'rb')) {
        if (!down) return true;
        if (btn === 'lb') { shiftTap(); return true; }
        if (btn === 'rb') { setPanel(E.panel === 'letters' ? 'sym' : 'letters'); return true; }
        var at = E.split[btn === 'lt' ? 'l' : 'r'];
        if (at) { E.sel = { r: at.r, c: at.c }; press(keyAt(at.r, at.c), 'pad'); }
        return true;
      }
      if (!down) {
        var h = E.held[btn]; delete E.held[btn];
        if (btn === 'lt') { E.ltHeld = false; if (E.ltUsed && E.shift && !E.caps) { E.shift = false; dirty(); changed(); } }
        return !!h || ['a', 'b', 'x', 'y', 'lb', 'rb', 'lt', 'rt', 'l3', 'r3', 'view', 'menu', 'up', 'down', 'left', 'right'].indexOf(btn) >= 0;
      }
      if (E.held[btn]) return true;
      var wheelPress = !!(E.wheel && E.wheel.petal >= 0 && WHEEL_BTN.indexOf(btn) >= 0);
      var key = btn === 'a' && E.sel.r >= 0 && !E.popup && !wheelPress ? keyAt(E.sel.r, E.sel.c) : null;
      var altList = key ? altsFor(key) : null;   // as the key was when pressed (a capital, before shift lets go)
      var used = act(btn, t);
      if (used) E.held[btn] = { t0: t, next: t + REPEAT_FIRST, key: key, alts: !!altList, altList: altList, wheel: wheelPress };
      return used;
    }
    /** Sticks, -1..1 (left stick: navigation, the Ember Wheel or the left half; right stick: the right half). */
    function axes(lx, ly, rx, ry, t) {
      t = t || now();
      E.stick = { lx: lx || 0, ly: ly || 0, rx: rx || 0, ry: ry || 0 };
      var fast = S().fast, mag = Math.sqrt(E.stick.lx * E.stick.lx + E.stick.ly * E.stick.ly);
      if (fast === 'wheel' && E.panel !== 'emoji' && E.panel !== 'clip' && E.panel !== 'num') {
        var was = E.wheel ? E.wheel.petal : -1, petal = -1;
        if (mag > 0.5) { var ang = Math.atan2(E.stick.lx, -E.stick.ly); if (ang < 0) ang += Math.PI * 2; petal = Math.round(ang / (Math.PI / 4)) % 8; }
        if (petal !== was) { E.wheel = petal >= 0 ? { petal: petal } : null; if (petal >= 0) haptic('tap'); changed(); }
        return;
      }
      if (fast === 'split') { splitAim(t); return; }
      // navigation with acceleration: further and longer tilts move faster
      var dir = null;
      if (mag > 0.55) dir = Math.abs(E.stick.lx) > Math.abs(E.stick.ly) ? (E.stick.lx < 0 ? 'left' : 'right') : (E.stick.ly < 0 ? 'up' : 'down');
      if (!dir) { E.stickNav = null; return; }
      if (!E.stickNav || E.stickNav.dir !== dir) { E.stickNav = { dir: dir, t0: t, next: t + 300, mag: mag }; move(dir); }
      else E.stickNav.mag = mag;
    }
    function splitAim(t) {
      var R = rows(), any = false;
      ['l', 'r'].forEach(function (side) {
        var x = side === 'l' ? E.stick.lx : E.stick.rx, y = side === 'l' ? E.stick.ly : E.stick.ry;
        if (Math.sqrt(x * x + y * y) < 0.22) { if (E.split[side]) { E.split[side] = null; any = true; } return; }
        var fx = Math.max(0, Math.min(1, (x * 1.15 + 1) / 2)), fy = Math.max(0, Math.min(1, (y * 1.15 + 1) / 2));
        var hx = side === 'l' ? fx * 0.5 : 0.5 + fx * 0.5;
        var r = Math.min(R.length - 1, Math.floor(fy * R.length)), row = R[r], best = -1, bd = 9;
        for (var c = 0; c < row.length; c++) {
          if (row[c].t === 'none') continue;
          var cx = center(row, c); if ((side === 'l') !== (cx < 0.5)) continue;
          var d = Math.abs(cx - hx); if (d < bd) { bd = d; best = c; }
        }
        var cur = E.split[side];
        if (best >= 0 && (!cur || cur.r !== r || cur.c !== best)) { E.split[side] = { r: r, c: best }; any = true; }
      });
      if (any) changed();
    }
    /** Call often (every frame while open): held buttons repeat, a held A opens the alternates. */
    function tick(t) {
      t = t || now();
      for (var b in E.held) {
        var h = E.held[b];
        if (b === 'a' && h.alts && !h.popped && t - h.t0 > 450) { h.popped = true; openPopup(h.key, true, h.altList); continue; }
        if (h.wheel) continue;
        var rep = ['x', 'y', 'lb', 'rb', 'up', 'down', 'left', 'right'].indexOf(b) >= 0 ||
          (b === 'a' && h.key && ['bksp', 'left', 'right', 'space', 'up', 'down'].indexOf(h.key.t) >= 0);
        if (!rep || t < h.next) continue;
        h.next = t + repeatEvery(t - h.t0);
        if (b === 'a') press(h.key, 'pad'); else act(b, t);
      }
      if (E.stickNav && t >= E.stickNav.next) {
        var held = t - E.stickNav.t0;
        E.stickNav.next = t + Math.max(45, (190 - Math.min(130, held / 9)) * (1.45 - 0.6 * Math.min(1, E.stickNav.mag)));
        move(E.stickNav.dir);
      }
    }

    // ---- touch and the mouse
    function tap(r, c) {
      if (r === -1) return pickSuggestion(c);
      var k = keyAt(r, c); if (!k || k.t === 'none') return;
      E.sel = { r: r, c: c };
      if (E.popup) E.popup = null;
      press(k, 'touch');
    }
    function longPress(r, c) { var k = keyAt(r, c); if (!k) return false; E.sel = { r: r, c: c }; return openPopup(k, false); }

    // ---- what to draw
    function view() {
      clampSel();
      var R = rows(), s = S();
      return {
        apiVersion: API_VERSION, panel: E.panel, mode: E.mode, rows: R, sel: { r: E.sel.r, c: E.sel.c }, active: E.active,
        suggestions: E.sugg.slice(), shift: E.shift, caps: E.caps, ctrl: E.ctrl, alt: E.alt,
        popup: E.popup ? { alts: E.popup.alts, i: E.popup.i, r: E.sel.r, c: E.sel.c } : null,
        wheel: s.fast === 'wheel' ? { active: !!(E.wheel && E.wheel.petal >= 0), petal: E.wheel ? E.wheel.petal : -1, petals: E.panel === 'sym' ? WHEEL.symbols : WHEEL.letters, buttons: WHEEL_BTN } : null,
        split: s.fast === 'split' ? { l: E.split.l, r: E.split.r } : null,
        layout: layout().id, layoutName: layout().name, layoutShort: layout().short, layouts: activeLayouts(),
        hints: E.hints, glyphs: GLYPH, settings: s, emojiCat: E.emojiCat, enterLabel: E.enterLabel
      };
    }
    function reset(mode, enterLabel) {
      if (mode) E.mode = mode;
      if (enterLabel !== undefined) E.enterLabel = enterLabel;
      E.panel = E.mode === 'number' ? 'num' : 'letters';
      E.shift = false; E.caps = false; E.autoShift = false; E.ctrl = false; E.alt = false; E.popup = null; E.wheel = null;
      E.split = { l: null, r: null };
      E.shadow = ''; E.shadowKnown = false; E.autoSpace = false; E.held = {}; E.stickNav = null; E.lastTyped = '';
      dirty();
      E.sel = { r: E.mode === 'terminal' ? 2 : 1, c: 0 };
      if (E.panel === 'num') E.sel = { r: 0, c: 0 };
      E.active = true;
      refresh(); changed();
    }
    function onGlobalChange() { dirty(); refresh(); changed(); }
    STATE.listeners.push(onGlobalChange);

    var api = {
      handle: handle, axes: axes, tick: tick, tap: tap, longPress: longPress, view: view, reset: reset,
      pickSuggestion: pickSuggestion, pickAlt: function (i) { pickAlt(i); }, closePopup: function () { E.popup = null; changed(); },
      move: move, press: function (r, c) { tap(r, c); },
      /** The text changed outside the keyboard (typed on a real keyboard, the caret moved): redo capitals and suggestions. */
      refresh: function () { E.shadow = ''; E.shadowKnown = false; refresh(); changed(); },
      setMode: function (m, label) { reset(m, label); },
      setActive: function (on) { E.active = !!on; if (on && E.sel.r < -1) E.sel = { r: 1, c: 0 }; changed(); },
      isActive: function () { return E.active; },
      setHints: function (on) { if (E.hints !== !!on) { E.hints = !!on; changed(); } },
      setAdapter: function (a) { opts.adapter = a; },
      setPanel: setPanel,
      onChange: function (f) { listeners.push(f); return function () { var i = listeners.indexOf(f); if (i >= 0) listeners.splice(i, 1); }; },
      destroy: function () { var i = STATE.listeners.indexOf(onGlobalChange); if (i >= 0) STATE.listeners.splice(i, 1); listeners.length = 0; },
      state: E
    };
    reset(E.mode, E.enterLabel);
    return api;
  }

  // ------------------------------------------------------------------------------------------------ adapters
  /** Typing into an <input> or <textarea> (Hearth's apps, web pages). */
  function inputAdapter(el, onEnter) {
    function fire() { try { el.dispatchEvent(new Event('input', { bubbles: true })); } catch (e) { /* old engine */ } }
    function sel() { var s = el.selectionStart, e = el.selectionEnd; if (typeof s !== 'number') { s = e = el.value.length; } return [s, e]; }
    return {
      context: function () { try { return el.value.slice(0, sel()[0]); } catch (e) { return ''; } },
      insert: function (text) {
        var se = sel(), v = el.value;
        if (el.maxLength > 0 && v.length - (se[1] - se[0]) + text.length > el.maxLength) return;
        el.value = v.slice(0, se[0]) + text + v.slice(se[1]);
        var at = se[0] + text.length; try { el.setSelectionRange(at, at); } catch (e) { /* number fields */ }
        fire();
      },
      deleteBack: function (n) {
        var se = sel(), v = el.value;
        if (se[0] !== se[1]) { el.value = v.slice(0, se[0]) + v.slice(se[1]); try { el.setSelectionRange(se[0], se[0]); } catch (e) { /* */ } fire(); return; }
        var head = dropLast(v.slice(0, se[0]), n);
        el.value = head + v.slice(se[0]); try { el.setSelectionRange(head.length, head.length); } catch (e) { /* */ } fire();
      },
      key: function (name) {
        var se = sel();
        if (name === 'left' || name === 'right') { var at = Math.max(0, Math.min(el.value.length, se[0] + (name === 'left' ? -1 : 1))); try { el.setSelectionRange(at, at); } catch (e) { /* */ } return; }
        if (name === 'home' || name === 'end') { var p = name === 'home' ? 0 : el.value.length; try { el.setSelectionRange(p, p); } catch (e) { /* */ } return; }
        if (name === 'enter') {
          if (el.tagName === 'TEXTAREA') { this.insert('\n'); return; }
          if (onEnter) { onEnter(); return; }
          try { el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })); } catch (e) { /* */ }
          if (el.form && el.form.requestSubmit) try { el.form.requestSubmit(); } catch (e) { /* */ }
        }
      }
    };
  }
  /** Typing into a plain {text, caret} object (Game Mode's search and text prompts). onChange redraws the field. */
  function modelAdapter(get, onChange, onKey) {
    function t() { var o = get(); if (o.caret == null || o.caret > o.text.length) o.caret = o.text.length; return o; }
    return {
      context: function () { var o = t(); return o.text.slice(0, o.caret); },
      insert: function (s) { var o = t(); o.text = o.text.slice(0, o.caret) + s + o.text.slice(o.caret); o.caret += s.length; onChange(); },
      deleteBack: function (n) { var o = t(); var head = dropLast(o.text.slice(0, o.caret), n); o.text = head + o.text.slice(o.caret); o.caret = head.length; onChange(); },
      key: function (name, mods) {
        var o = t();
        if (name === 'left') { o.caret = Math.max(0, o.caret - 1); onChange(); return; }
        if (name === 'right') { o.caret = Math.min(o.text.length, o.caret + 1); onChange(); return; }
        if (name === 'home') { o.caret = 0; onChange(); return; }
        if (name === 'end') { o.caret = o.text.length; onChange(); return; }
        if (onKey) onKey(name, mods);
      }
    };
  }

  // ------------------------------------------------------------------------------------------------ settings
  function configure(c) {
    if (!c) return;
    if (c.settings) { STATE.settings = {}; for (var k in c.settings) STATE.settings[k] = c.settings[k]; }
    if (c.systemLayouts) STATE.systemLayouts = c.systemLayouts.slice();
    if (c.learned) importLearned(c.learned);
    if (c.save !== undefined) STATE.save = c.save;
    if (c.loadWords !== undefined) STATE.loadWords = c.loadWords;
    if (c.base !== undefined) STATE.base = c.base;
    if (c.clipboard) STATE.clipboard = c.clipboard.slice(0, 12);
    notifyAll();
  }
  function setSetting(key, value) { if (!(key in DEFAULTS)) return; STATE.settings = STATE.settings || {}; STATE.settings[key] = value; notifyAll(); }

  // =========================================================================================== the page keyboard
  // Everything below draws the keyboard on a web page (it only runs when asked: mount() or open()).
  function el(doc, tag, cls, text) { var e = doc.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; }
  function keyLabel(k, v) {
    if (k.t === 'ch') return v.shift || v.caps ? upper(k.v) : k.v;
    if (k.t === 'space') return k.label || '';
    if (k.t === 'shift' && v.caps) return '⇪';
    return k.label;
  }
  /**
   * Draws `engine` inside `box` (an element; its classes and size are its own) and keeps it drawn.
   * opts: {overGame: see-through over a game, doc: the document (Hearth Browser's chrome)}
   * Returns {render(), destroy(), el}.
   */
  function mount(box, engine, mopts) {
    mopts = mopts || {};
    var doc = mopts.doc || box.ownerDocument || document;
    var win = doc.defaultView || (typeof window !== 'undefined' ? window : null);
    var root = el(doc, 'div', 'hosk');
    box.appendChild(root);
    var top = el(doc, 'div', 'hosk-top'), keys = el(doc, 'div', 'hosk-keys'), grid = el(doc, 'div', 'hosk-grid'), ember = el(doc, 'div', 'hosk-ember');
    root.appendChild(top); root.appendChild(keys); keys.appendChild(grid); keys.appendChild(ember);
    var extra = [];
    var lastRect = null, longTimer = null, pressed = null, sparks = 0;

    function cls(v) {
      var s = v.settings;
      root.className = 'hosk theme-' + (s.theme || 'hearth') + ' size-' + (s.size || 'm') + ' shape-' + (s.shape || 'full') + ' panel-' + v.panel +
        (v.active ? ' active' : '') + (v.hints ? ' hints' : '') + (mopts.overGame ? ' over-game' : '') + (s.trail === false ? ' no-trail' : '') +
        (v.shift ? ' shifted' : '') + (v.caps ? ' caps' : '');
    }
    function render() {
      var v = engine.view();
      cls(v);
      // the strip: suggestions, or what the controller does
      top.textContent = '';
      var strip = el(doc, 'div', 'hosk-sugg');
      if (v.suggestions.length) v.suggestions.forEach(function (w, i) {
        var b = el(doc, 'button', 'hosk-s' + (v.active && v.sel.r === -1 && v.sel.c === i ? ' sel' : ''), w);
        b.setAttribute('data-r', '-1'); b.setAttribute('data-c', String(i)); b.type = 'button';
        strip.appendChild(b);
      });
      else {
        var tip = v.wheel && v.hints ? 'Left stick: Ember Wheel · A B X Y type its letters' : v.split && v.hints ? 'Point with both sticks · LT / RT type' :
          v.hints ? 'A type · X ⌫ · Y space · LT ⇧ · RT &123 · LB RB move · ☰ Enter · B hide · hold A for accents' : '';
        strip.appendChild(el(doc, 'span', 'hosk-tip', tip));
      }
      top.appendChild(strip);
      top.appendChild(el(doc, 'span', 'hosk-lay', v.panel === 'emoji' ? 'Emoji' : v.panel === 'clip' ? 'Clipboard' : v.layoutShort));
      // the keys
      grid.textContent = '';
      while (extra.length) { var x = extra.pop(); if (x.parentNode) x.parentNode.removeChild(x); }
      v.rows.forEach(function (row, r) {
        var line = el(doc, 'div', 'hosk-row'), half = null, left = null, right = null;
        if (v.settings.shape === 'split' && v.panel !== 'num') { left = el(doc, 'div', 'hosk-half'); right = el(doc, 'div', 'hosk-half'); line.appendChild(left); line.appendChild(el(doc, 'div', 'hosk-gap')); line.appendChild(right); }
        row.forEach(function (k, c) {
          var b = el(doc, 'button', 'hosk-key k-' + k.t + (k.go ? ' go' : '') + (k.on ? ' on' : '') + (k.lock ? ' lock' : '') + (k.off ? ' off' : '') + (k.emoji ? ' emo' : '') + (k.clip ? ' clipk' : '') +
            (v.active && v.sel.r === r && v.sel.c === c ? ' sel' : '') + (v.split && v.split.l && v.split.l.r === r && v.split.l.c === c ? ' aim-l' : '') + (v.split && v.split.r && v.split.r.r === r && v.split.r.c === c ? ' aim-r' : ''));
          b.type = 'button';
          b.style.flexGrow = String(k.w);
          b.setAttribute('data-r', String(r)); b.setAttribute('data-c', String(c));
          if (k.title) b.title = k.title;
          b.appendChild(el(doc, 'span', 'hosk-l', keyLabel(k, v)));
          if (k.t === 'ch' && ALTS[k.v.toLowerCase()]) b.appendChild(el(doc, 'i', 'hosk-alt'));
          var g = v.hints && GLYPH[k.t] && !(k.t === 'shift' && k.v === 'r') ? GLYPH[k.t] : '';
          if (g && v.settings.fast === 'split' && (k.t === 'shift' || k.t === 'sym' || k.t === 'abc' || k.t === 'left' || k.t === 'right')) g = k.t === 'shift' ? 'LB' : (k.t === 'sym' || k.t === 'abc') ? 'RB' : '';
          if (g) b.appendChild(el(doc, 'small', 'hosk-g', g));
          (k.half === 'l' ? left : k.half === 'r' ? right : line).appendChild(b);
        });
        grid.appendChild(line);
      });
      // alternates
      if (v.popup) {
        var pop = el(doc, 'div', 'hosk-pop');
        v.popup.alts.forEach(function (a, i) { var b = el(doc, 'button', 'hosk-key k-alt' + (i === v.popup.i ? ' sel' : ''), a); b.type = 'button'; b.setAttribute('data-alt', String(i)); pop.appendChild(b); });
        keys.appendChild(pop); extra.push(pop);
        place(pop, v.popup.r, v.popup.c);
      }
      // the Ember Wheel
      if (v.wheel && (v.wheel.active || (v.hints && mopts.showWheel))) { var wh = wheel(v); keys.appendChild(wh); extra.push(wh); }
      glow(v);
    }
    function keyEl(r, c) { return keys.querySelector('[data-r="' + r + '"][data-c="' + c + '"]') || top.querySelector('[data-r="' + r + '"][data-c="' + c + '"]'); }
    function rel(e) { var a = e.getBoundingClientRect(), b = keys.getBoundingClientRect(); return { x: a.left - b.left, y: a.top - b.top, w: a.width, h: a.height }; }
    function place(pop, r, c) {
      var k = keyEl(r, c); if (!k) return;
      var p = rel(k), kw = keys.clientWidth || 600;
      var w = Math.min(kw, (p.w + 6) * pop.children.length);
      pop.style.left = Math.max(0, Math.min(kw - w, p.x + p.w / 2 - w / 2)) + 'px';
      pop.style.top = Math.max(-p.h * 1.2, p.y - p.h - 10) + 'px';
      pop.style.width = w + 'px';
    }
    function glow(v) {
      if (!v.active || v.sel.r === -1 || (v.wheel && v.wheel.active)) { ember.style.opacity = '0'; lastRect = null; return; }
      var k = keyEl(v.sel.r, v.sel.c);
      if (!k) { ember.style.opacity = '0'; return; }
      var p = rel(k);
      if (lastRect && (Math.abs(lastRect.x - p.x) > 2 || Math.abs(lastRect.y - p.y) > 2) && v.settings.trail !== false && sparks < 4) {
        var sp = el(doc, 'div', 'hosk-spark');
        sp.style.left = (lastRect.x + lastRect.w / 2) + 'px'; sp.style.top = (lastRect.y + lastRect.h / 2) + 'px';
        keys.appendChild(sp); sparks++;
        if (win) win.setTimeout(function () { if (sp.parentNode) sp.parentNode.removeChild(sp); sparks--; }, 460);
      }
      ember.style.opacity = '1';
      ember.style.transform = 'translate(' + p.x + 'px,' + p.y + 'px)';
      ember.style.width = p.w + 'px'; ember.style.height = p.h + 'px';
      lastRect = p;
    }
    function wheel(v) {
      var w = el(doc, 'div', 'hosk-wheel' + (v.wheel.active ? ' on' : ''));
      var names = { y: 'Y', x: 'X', b: 'B', a: 'A' }, pos = ['t', 'l', 'r', 'b'];
      v.wheel.petals.forEach(function (p, i) {
        var pe = el(doc, 'div', 'hosk-petal' + (i === v.wheel.petal ? ' on' : ''));
        var ang = i * 45, rad = ang * Math.PI / 180;
        pe.style.left = (50 + Math.sin(rad) * 36) + '%'; pe.style.top = (50 - Math.cos(rad) * 36) + '%';
        for (var j = 0; j < 4; j++) {
          var ch = p.charAt(j); if (!ch) continue;
          var s = el(doc, 'span', 'p-' + pos[j], v.shift || v.caps ? upper(ch) : ch);
          if (i === v.wheel.petal) s.setAttribute('data-btn', names[v.wheel.buttons[j]]);
          pe.appendChild(s);
        }
        w.appendChild(pe);
      });
      w.appendChild(el(doc, 'div', 'hosk-hub'));
      return w;
    }

    // touch and the mouse: type on release, a long touch opens the alternates
    function at(e) { var t = e.target; while (t && t !== root && !(t.getAttribute && (t.getAttribute('data-r') !== null || t.getAttribute('data-alt') !== null))) t = t.parentNode; return t && t !== root ? t : null; }
    function down(e) {
      var t = at(e); if (!t) return;
      if (e.preventDefault) e.preventDefault();        // keeps the focus (and the caret) in the text field
      pressed = t; t.classList.add('down');
      if (longTimer && win) win.clearTimeout(longTimer);
      if (t.getAttribute('data-r') !== null && win) {
        var r = +t.getAttribute('data-r'), c = +t.getAttribute('data-c');
        longTimer = win.setTimeout(function () { longTimer = null; if (pressed === t && engine.longPress(r, c)) { pressed = null; t.classList.remove('down'); } }, 480);
      }
    }
    function up(e) {
      var t = pressed; pressed = null;
      if (longTimer && win) { win.clearTimeout(longTimer); longTimer = null; }
      if (!t) return;
      t.classList.remove('down');
      if (e.type === 'pointercancel' || e.type === 'pointerleave') return;
      var over = at(e); if (over && over !== t) t = over;
      if (t.getAttribute('data-alt') !== null) return engine.pickAlt(+t.getAttribute('data-alt'));
      engine.tap(+t.getAttribute('data-r'), +t.getAttribute('data-c'));
    }
    root.addEventListener('pointerdown', down);
    root.addEventListener('pointerup', up);
    root.addEventListener('pointercancel', up);
    root.addEventListener('mousedown', function (e) { if (at(e)) e.preventDefault(); });
    root.addEventListener('click', function (e) { if (at(e) || e.target === root) e.stopPropagation(); });
    var off = engine.onChange(render);
    var onResize = function () { lastRect = null; render(); };
    if (win) win.addEventListener('resize', onResize);
    render();
    return {
      el: root, render: render,
      destroy: function () { off(); if (win) win.removeEventListener('resize', onResize); if (root.parentNode) root.parentNode.removeChild(root); }
    };
  }

  // ------------------------------------------------------------------------------------------------ open()
  // For Hearth's apps and simple pages: a keyboard docked to the bottom of the window for one text field, reading
  // the controller itself. HearthOSK.open({target: input, mode, enterLabel, onEnter, onClose})
  var DOCK = null;
  function guessMode(t) {
    if (!t || !t.getAttribute) return 'text';
    var type = (t.getAttribute('type') || '').toLowerCase(), im = (t.getAttribute('inputmode') || '').toLowerCase();
    if (type === 'password') return 'password';
    if (type === 'email' || im === 'email') return 'email';
    if (type === 'url' || im === 'url') return 'url';
    if (type === 'number' || type === 'tel' || im === 'numeric' || im === 'decimal' || im === 'tel') return 'number';
    if (type === 'search' || im === 'search') return 'search';
    return 'text';
  }
  function padLoop() {
    if (!DOCK || !DOCK.ownPad) return;
    var pads = typeof navigator !== 'undefined' && navigator.getGamepads ? navigator.getGamepads() : [];
    var st = {}, ax = { lx: 0, ly: 0, rx: 0, ry: 0 }, any = false;
    var MAP = { a: 0, b: 1, x: 2, y: 3, lb: 4, rb: 5, lt: 6, rt: 7, view: 8, menu: 9, l3: 10, r3: 11, up: 12, down: 13, left: 14, right: 15 };
    for (var i = 0; i < pads.length; i++) {
      var p = pads[i]; if (!p) continue; any = true;
      for (var k in MAP) { var b = p.buttons[MAP[k]]; if (b && (b.pressed || (k === 'lt' || k === 'rt') && b.value > 0.5)) st[k] = true; }
      ['lx', 'ly', 'rx', 'ry'].forEach(function (n, j) { var v = p.axes[j] || 0; if (Math.abs(v) > Math.abs(ax[n])) ax[n] = Math.abs(v) < 0.12 ? 0 : v; });
    }
    var t = Date.now();
    if (any) {
      for (var n in MAP) {
        var on = !!st[n];
        if (on !== !!DOCK.padHeld[n]) { DOCK.padHeld[n] = on; if (DOCK.armed[n] && on) continue; if (!on) DOCK.armed[n] = false; DOCK.engine.setHints(true); DOCK.engine.handle(n, on, t); if (!DOCK) return; }
      }
      DOCK.engine.axes(ax.lx, ax.ly, ax.rx, ax.ry, t);
    }
    if (!DOCK) return;
    DOCK.engine.tick(t);
    DOCK.raf = win().requestAnimationFrame(padLoop);
  }
  function win() { return typeof window !== 'undefined' ? window : null; }
  // Key-press rumble. Some controllers (Handheld Daemon's virtual pad on the ROG Ally, some Bluetooth pads) ignore
  // the effect's duration and keep rumbling until told to stop, so every rumble is followed by an explicit stop
  // (reset) shortly after it should have ended; quick presses push the stop back instead of stacking.
  var RUMBLE_STOP = null;
  function padsWithRumble() {
    var pads = typeof navigator !== 'undefined' && navigator.getGamepads ? navigator.getGamepads() : [], out = [];
    for (var i = 0; i < pads.length; i++) { var p = pads[i]; if (p && p.vibrationActuator) out.push(p.vibrationActuator); }
    return out;
  }
  function rumbleStop() {
    if (RUMBLE_STOP) { clearTimeout(RUMBLE_STOP); RUMBLE_STOP = null; }
    var acts = padsWithRumble();
    for (var i = 0; i < acts.length; i++) {
      var a = acts[i];
      try { if (a.reset) { var r = a.reset(); if (r && r.catch) r.catch(function () { }); } else if (a.playEffect) { a.playEffect('dual-rumble', { duration: 0, weakMagnitude: 0, strongMagnitude: 0 }).catch(function () { }); } } catch (e) { /* nothing to stop */ }
    }
  }
  function rumble(strength, ms) {
    var acts = padsWithRumble(); if (!acts.length) return;
    ms = Math.max(1, Math.min(200, ms || 20));
    for (var i = 0; i < acts.length; i++) {
      var a = acts[i]; if (!a.playEffect) continue;
      try { a.playEffect('dual-rumble', { duration: ms, weakMagnitude: strength, strongMagnitude: strength * 0.4 }).catch(function () { }); } catch (e) { /* no rumble */ }
    }
    if (RUMBLE_STOP) clearTimeout(RUMBLE_STOP);
    RUMBLE_STOP = setTimeout(rumbleStop, ms + 40);
  }
  function open(o) {
    o = o || {};
    close(true);
    var w = win(); if (!w) return null;
    var doc = w.document, target = o.target || doc.activeElement;
    var box = el(doc, 'div', 'hosk-dock' + (settings().position === 'top' ? ' at-top' : ''));
    (o.container || doc.body).appendChild(box);
    var adapter = o.adapter || (target && 'value' in target ? inputAdapter(target, o.onEnter) : null);
    var engine = createEngine({
      adapter: adapter, mode: o.mode || guessMode(target), enterLabel: o.enterLabel || (target && target.tagName === 'TEXTAREA' ? '' : 'Done'), host: 'web',
      onHide: function () { close(); }, onHaptic: rumble, onSound: o.onSound,
      onLeave: function () { engine.setActive(true); }
    });
    var view = mount(box, engine, { overGame: !!o.overGame });
    DOCK = { box: box, engine: engine, view: view, target: target, onClose: o.onClose, ownPad: o.pad !== false, padHeld: {}, armed: {}, raf: 0 };
    // Buttons already held when it opens (the A that opened it) wait for their release.
    var pads = typeof navigator !== 'undefined' && navigator.getGamepads ? navigator.getGamepads() : [];
    for (var i = 0; i < pads.length; i++) if (pads[i]) pads[i].buttons.forEach(function (b, j) { if (b && b.pressed) { var n = ['a', 'b', 'x', 'y', 'lb', 'rb', 'lt', 'rt', 'view', 'menu', 'l3', 'r3', 'up', 'down', 'left', 'right'][j]; if (n) { DOCK.armed[n] = true; DOCK.padHeld[n] = true; } } });
    engine.setHints(o.hints !== undefined ? !!o.hints : true);
    if (target && target.scrollIntoView) try { target.scrollIntoView({ block: 'nearest' }); } catch (e) { /* */ }
    if (DOCK.ownPad) DOCK.raf = w.requestAnimationFrame(padLoop);
    doc.addEventListener('keydown', onRealKey, true);
    return { engine: engine, close: close, el: box };
  }
  // A real keyboard typing: it takes over, the on-screen keyboard goes away.
  function onRealKey(e) {
    if (!DOCK || e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key && e.key.length === 1) close();
  }
  function close(quiet) {
    rumbleStop();
    if (!DOCK) return;
    var d = DOCK; DOCK = null;
    var w = win();
    if (w && d.raf) w.cancelAnimationFrame(d.raf);
    if (w) w.document.removeEventListener('keydown', onRealKey, true);
    d.view.destroy(); d.engine.destroy();
    if (d.box.parentNode) d.box.parentNode.removeChild(d.box);
    if (!quiet && d.target && d.target.focus) try { d.target.focus({ preventScroll: true }); } catch (e) { /* */ }
    if (d.onClose) try { d.onClose(); } catch (e) { /* */ }
  }
  /** For pages that read the controller themselves: give the open keyboard a button (down/up) or the sticks. */
  function input(btn, down) { if (!DOCK) return false; DOCK.engine.setHints(true); return DOCK.engine.handle(btn, down); }

  // Web pages: words come from words/<lang>.js next to this file; settings and learned words from localStorage
  // until the page says otherwise (HearthOSK.configure).
  (function webDefaults() {
    var w = win(); if (!w || !w.document) return;
    try {
      var cs = w.document.currentScript;
      if (cs && cs.src) STATE.base = cs.src.replace(/[^/]*$/, '');
    } catch (e) { /* */ }
    STATE.loadWords = function (lang) {
      if (!/^[a-z]{2}$/.test(lang)) return;
      var s = w.document.createElement('script');
      s.src = (STATE.base || 'osk/') + 'words/' + lang + '.js';
      s.async = true;
      (w.document.head || w.document.documentElement).appendChild(s);
    };
    try {
      var saved = w.localStorage.getItem('hearth.osk');
      if (saved) STATE.settings = JSON.parse(saved);
      var learned = w.localStorage.getItem('hearth.osk.learned');
      if (learned) importLearned(JSON.parse(learned));
    } catch (e) { /* private window */ }
    STATE.save = function (d) { try { w.localStorage.setItem('hearth.osk.learned', JSON.stringify(d)); } catch (e) { /* full */ } };
  })();

  return {
    version: API_VERSION,
    LAYOUTS: LAYOUTS, THEMES: THEMES, DEFAULTS: DEFAULTS, EMOJI_CATS: EMOJI_CATS, WHEEL: WHEEL,
    createEngine: createEngine, mount: mount, open: open, close: close, input: input,
    rumble: rumble, rumbleStop: rumbleStop,
    isOpen: function () { return !!DOCK; },
    inputAdapter: inputAdapter, modelAdapter: modelAdapter,
    configure: configure, setSetting: setSetting, settings: settings, activeLayouts: activeLayouts,
    layoutForXkb: layoutForXkb, layoutForLang: layoutForLang,
    words: addWords, hasWords: function (lang) { return !!STATE.dict[lang]; }, suggest: suggest,
    exportLearned: exportLearned, importLearned: importLearned, clearLearned: clearLearned, learnedCount: learnedCount,
    fold: fold, graphemes: graphemes, guessMode: guessMode
  };
})();
if (typeof module !== 'undefined' && module.exports) module.exports = HearthOSK;
