# Hearth's on-screen keyboard

One keyboard for all of Hearth OS: Game Mode (search, every text prompt, Quick Access over a game), Mantel (Desktop
Mode, typing into any Linux app), Hearth's own apps (Time, Maps, Contacts; Hearth Files' window in Game Mode) and
Hearth Browser. It is made for a controller first, and works with touch and the mouse too. It never comes up after
keyboard or mouse input: a real keyboard types straight into the field, and typing on one sends the on-screen
keyboard away.

- Code: `ui/osk/hearth-osk.js` (the engine and the web page keyboard), `ui/osk/hearth-osk.css` (its look),
  `ui/osk/words/<lang>.js` (word lists). Desktop Mode draws the same engine with `hosts/linux/desktop/shell/OskBoard.qml`.
- Copies: `python3 tools/osk-sync.py` copies `ui/osk` to Desktop Mode, the apps and Hearth Browser;
  `hosts/linux/test/test_hearth_osk.py` fails when a copy differs. Edit `ui/osk`, never a copy.

## What we learned from other keyboards

### SteamOS / Steam Deck

- **Opening**: Steam + X anywhere (Steam must be running); in Desktop Mode X opens it. B, the keyboard icon or Steam + X
  again hides it. [pimylifeup][pim], [PCGamesN][pcgn], [Tom's Guide][tg]
- **Ways to type**: the joystick, the D-pad, the touchscreen, or the trackpads; "dual trackpad typing" gives each
  trackpad its own cursor over its half of the keyboard, and a key is sent when the trackpad is pressed down (not on
  release), which helps fast typists. Trackpad typing updates at 60 Hz. [GamingOnLinux][gol-2204],
  [GSMArena][gsm], [Steam Deck HQ][sdhq]
- **Buttons**: X deletes, Y types a space (players asked for bumpers instead: "Using the bumpers for space and
  backspace is far superior"); the Deck's keyboard bindings can't be changed. [Steam discussion][steam-xy],
  [Steam discussion][steam-bind]
- **Panels**: a full emoji keyboard and Steam's emoticons. [Gardiner Bryant][gb]
- **Themes**: four or five built in, more from the Points Shop, chosen in Settings › Keyboard (with a preview);
  community keyboards from Discover. [Tom's Guide][tg], [PCGamesN][pcgn]
- **Layouts**: more languages were added in updates. [GamingOnLinux][gol-2205]
- **Word suggestions**: the old Big Picture keyboard had predictive text; players miss it in the new one (we found
  no sign it came back). [Steam discussion][steam-pred]
- **Desktop Mode**: the Game Mode keyboard (and its theme) became available in Desktop Mode; R2 / Enter no longer
  dismiss it there. [GSMArena][gsm]

### Steam Big Picture's Daisywheel (2012)

Daisywheel "divides the alphabet into eight groups of four": the left stick picks a group, the face buttons pick one
of its four letters. Slower to learn than a grid, fast once learned. [Engadget][eng]

### Xbox, PlayStation, Nintendo

- Xbox's keyboard: X is backspace, Y space, B closes it; the triggers / bumpers move the cursor and switch keyboards
  (as players asked Kodi to copy). [Kodi forum][kodi]
- PlayStation 5: no official on-screen keyboard shortcuts are listed by Sony's button guide (square and triangle are
  context actions in menus). [PlayStation][ps5]
- Controller rumble from a web page: `GamepadHapticActuator.playEffect('dual-rumble', {duration, strongMagnitude,
  weakMagnitude})`, not supported everywhere. [MDN][mdn]

### What we took

From SteamOS: X ⌫, Y space, B hides, big docked keys, emoji panel, themes, stick / D-pad / touch, the two-halves
idea (here on the two sticks), Desktop Mode parity. From Xbox: bumpers move the caret. From Daisywheel: eight groups
of four on a stick and the face buttons. From phones: suggestions, long-press accents, autocapitals, double-space
period, a number pad for numbers.

## Hearth's keyboard

### Controller

| Button | Does |
|---|---|
| D-pad / left stick | Move between keys (the stick speeds up the further and longer you tilt it). Up from the top row: the suggestions |
| A | Type the key. **Hold** on a letter: its accents (é è ê…), pick with ← →, A |
| X / Y | Backspace / Space (hold to repeat) |
| LB / RB | Move the caret left / right (hold to repeat) |
| LT | Shift for one letter; twice quickly: caps lock; held: shift while held |
| RT | Symbols and numbers (again: letters) |
| L3 | Emoji |
| R3 | Next layout (when you use more than one) |
| View | Accents for the picked key (for controllers whose A can't be held: Quick Access over a game) |
| ☰ Menu | Enter (Done / Search / Go) |
| B | Hide the keyboard (A on the text field brings it back) |

In Desktop Mode the right stick stays the pointer and RT clicks, so you can click into another field with the keyboard
open; symbols are on the &123 key there.

### Fast typing (Settings › Keyboard › Fast typing)

- **Ember Wheel** (default): tilt the left stick and a ring of eight glowing coals appears over the keys, the tilted
  coal holding four letters on the Y / X / B / A positions; press that button to type it. Center the stick and A B X Y
  are back to normal. Letters: `abcd efgh ijkl mnop qrst uvwx yz,. ?!'-`; on the symbols panel, digits and symbols.
- **Split sticks**: like the Deck's dual trackpads, on the sticks: the left stick points into the left half, the
  right stick into the right half (two cursors at once), LT types the left one's key, RT the right one's. LB is shift,
  RB symbols.

### Touch and mouse

Tap a key; a long touch on a letter shows its accents. The split shape (Settings › Keyboard › Shape) puts the halves
at the edges, under your thumbs on a handheld's touchscreen.

### Typing help

- **Word suggestions** above the keys: from a word list per language (the 6000 most used words), plus words you've
  typed (a word counts once you've typed it twice, so typos don't come back). Up to the strip with the D-pad, A
  picks; it adds its space, and a period typed right after goes before that space.
- **Next words**: after a space, the words you usually type next.
- **Learning** stays on this device: Game Mode keeps it in its page and in `~/.local/share/nimbus/osk-words.json`
  (shared with Desktop Mode); Clear learned words forgets it. Never learned: passwords, URLs, e-mail addresses.
- **Capitals** at the start of a sentence (text fields, not search, URLs or passwords).
- **Double space** types ". ".
- Game Mode's library search suggests words from your games' titles instead of the dictionary.

### Panels and kinds of fields

Letters (with a number row), symbols (two pages), emoji (Recent, Smileys, People, Nature, Food, Activities, Travel,
Objects, Symbols, Kaomoji: a pick from Hearth's Emoji & symbols data), clipboard (recent copies; in Desktop Mode and
the browser also a Paste key) and a number pad. The field's kind changes the keys: `url` (/ . :com), `email`
(@ .com), `password` (no suggestions, no emoji), `number` (the number pad, with + for phone numbers), `search`,
`terminal` (an extra row: Esc, Tab, Ctrl, Alt, |, ~, /, -, Home, End, arrows; Ctrl and Alt apply to the next key).

### Layouts

English (QWERTY), Français (AZERTY), Deutsch (QWERTZ), Español, Svenska, Suomi, Dansk, Norsk, Português, Italiano, each
with its own letters (ñ, ç, å ä ö, æ ø, ü ß…), accents in its order and its word list. **Automatic** (the default)
follows the system's keyboard layouts (Desktop Mode's Settings › Keyboard, `XKB_DEFAULT_LAYOUT`, Debian's
`/etc/default/keyboard`, Fedora's X11 keyboard config), then the language. With more than one, the globe key or R3
switches.

### Look

Hearth's look with your accent colour, and four themes: **Hearth**, **Ember glow** (warm, glowing from below),
**Slate**, **Daylight**. Small / Medium / Large, bottom or top (Desktop Mode and apps), full or split. Over a game
(Quick Access, the overlay) it's see-through with the game blurred behind it.

### Hearth's own touches

1. **The ember**: the picked key isn't just outlined: a warm glow glides from key to key, flickering softly like an
   ember, leaving a short trail of sparks behind it (Settings › Keyboard › Ember trail turns the trail off;
   reduced motion turns it off too).
2. **The Ember Wheel**: Daisywheel, reborn as a hearth: eight coals around a glowing centre.
3. **Split sticks**: the Deck's dual-trackpad typing, done with the two sticks every controller has.
4. **Split hearth** shape for handheld touchscreens.

### Rumble

A short, light rumble on every key (Settings › Keyboard: off / light / strong): Game Mode uses the controller's
`vibrationActuator` when the page reads the controller itself; Desktop Mode asks hearth-pad, which plays a force-feedback
rumble (`FF_RUMBLE`) on the controller. Controllers read by Hearth's host (Quick Access over a game) don't rumble yet.

### Settings

Game Mode: Settings › Controls & power › Keyboard. Desktop Mode: Hearth Settings › Keyboard › On-screen keyboard. Both save
`~/.config/nimbus/osk.json` (Game Mode through the `osk.set` call; it also keeps a copy in its page's storage, which
is what Ember on Windows uses). Keys: `layouts` (`["auto"]` or layout ids), `suggestions`, `learn`, `autocap`,
`doubleSpace`, `fast` (`wheel` | `split` | `off`), `shape` (`full` | `split`), `size` (`s` | `m` | `l`), `position`
(`bottom` | `top`), `theme`, `haptics` (`off` | `light` | `strong`), `trail`.

## Guide + X: the keyboard anywhere

Like Steam + X on a Steam Deck: **Guide + X** opens Hearth's on-screen keyboard wherever you are; **B** (or the hide
key), or Guide + X again, closes it. Guide alone still does what it always did.

| Where | How it opens | Where the typing goes |
|---|---|---|
| Over any game or app in Game Mode (native, Proton/Wine, emulators; with or without a Hearth Input layout) | The chord is caught by whoever holds the controller: Hearth's reader (`guide.rs`; Guide held is asked from the device itself, EVIOCGKEY, so it counts even if Hearth Input had the controller when Guide went down), Hearth Input (`{"t": "osk"}`: it keeps the controller while Guide is held, so the game sees neither press) or InputPlumber (everything after Guide). Hearth (`gamemode.rs osk_chord`) opens the overlay page as just the keyboard (`overlay.osk`): see-through, the game keeps its keyboard focus (gamescope `STEAM_INPUT_FOCUS` 2), and Hearth holds the controller for the keyboard the way it does for Quick Access (the game doesn't get the presses). Quick Access opened by that same Guide press becomes the keyboard. | `osk.type` / `osk.key` → Hearth Input's uinput keyboard: each character becomes the key presses for gamescope's keyboard layout (`hosts/linux/desktop/shell/osk-keymap.json`: us, gb, de, fr, es, it, pt, se, fi, dk, no; `XKB_DEFAULT_LAYOUT`), held 30 ms each so games that look once a frame see them. Characters the layout can't type go through `xdotool type` on gamescope's X display when xdotool is installed; otherwise the keyboard says which it couldn't type. |
| Hearth Classic (Mantel) | hearth-pad: Guide opens the Hearth menu as before; X while Guide is held closes it again and asks for the keyboard (`osk {open}`) | `Keyboard.qml` as always (wtype, or hearth-pad's uinput keyboard) |
| Hearth Plasma | `hearth-pad --osk`, started with `osk-plasma.qml` (Quickshell, `/usr/share/hearth-plasma/xdg/autostart/hearth-osk.desktop`): does nothing until Guide + X, then holds the controller for the keyboard (apps don't see the presses) | The same `Keyboard.qml` as a layer-shell window that never takes the keyboard focus, typing through hearth-pad's uinput keyboard (KWin has no wtype). Without quickshell, python3-evdev or /dev/uinput, Plasma simply runs without it. |
| Hearth's own screen in Game Mode | its page gets `osk.chord` and opens the keyboard for the open search or text prompt | the field |

The keyboard over a game uses the same engine (`ui/osk/hearth-osk.js`) with an adapter of its own (`ovoskAdapter` in
`ui/index.html`): `insert` → `osk.type`, `deleteBack(n)` → `osk.key bksp n`, `key(name)` → `osk.key` (enter, tab,
esc, arrows, home, end, paste). A strip above it shows what you've typed since the last Enter (the game's own text box
can't be read). Over a game the controller reaches the page as Hearth's presses (`pad`), so A, B, X, Y, the bumpers,
LT / RT, L3 / R3, View and ☰ all work as on the table above.

## Where it runs

| Where | How |
|---|---|
| Game Mode search and text prompts (and so Wi-Fi passwords, sign-ins, renames, "Type into Linux") | `ui/index.html`: one engine (`GOSK`), drawn into the prompt's / search's box; `openPrompt({..., mode})` picks the kind of field |
| Quick Access over a game | the same page (`?overlay=full`), see-through |
| Over any game or app (Guide + X) | the overlay page as just the keyboard (`overlay.osk`, `overlayOsk`), typing into the game through Hearth Input (above) |
| Hearth Plasma (Guide + X) | `osk-plasma.qml`: `Keyboard.qml` with `useWtype: false`, hearth-pad `--osk` (above) |
| Mantel (Desktop Mode) | `Keyboard.qml` runs the engine, `OskBoard.qml` / `OskKey.qml` draw it; hearth-pad hands it the controller (`oskpad`, `oskaxes`); typing goes through `wtype` (any character, any system layout), or hearth-pad's uinput keyboard without wtype |
| Hearth Files' window in Game Mode (the file picker) | `FileManager.qml` draws `OskBoard` inside its window; picker.qml's hearth-pad types |
| Hearth Time, Maps, Contacts | `HearthOSK.open({target})` on their text fields (their own small keyboards stay only as a fallback) |
| Hearth Browser | `hearth.sys.mjs` loads the engine and the page keyboard into Firefox's window; keys go through Firefox's text input interface (`nsITextInputProcessor`; words and emoji with `commitCompositionWith`) |

## Public API

`HearthOSK` (a global on web pages; `Osk.HearthOSK` in QML). Other parts of Hearth call these: keep them working.
`HearthOSK.version` is the API version (1).

### Simple: a keyboard for a text field

```js
const kb = HearthOSK.open({
  target: inputElement,        // <input> or <textarea>; or give `adapter` (below) instead
  mode: 'text',                // optional: guessed from type= / inputmode= (password, email, url, number, search)
  enterLabel: 'Done',          // optional: the Enter key's label
  onEnter: () => {},           // optional: Enter in an <input> (default: a keydown Enter event and the form's submit)
  onClose: () => {},           // optional: it closed (B, the hide key, a real keyboard typing, HearthOSK.close())
  pad: true,                   // optional: it reads the controller itself (false: give it presses with HearthOSK.input)
  overGame: false              // optional: see-through
});
HearthOSK.isOpen(); HearthOSK.close(); HearthOSK.input('a', true /* down */ );
```

Include `osk/hearth-osk.css` and `osk/hearth-osk.js` (the word lists are loaded from `osk/words/` next to it). While
it's open, a page that reads the controller itself should leave the controller to it (see the apps' `pad()` loops).

### Full control: an engine and a view

```js
const engine = HearthOSK.createEngine({
  adapter: {                   // where the typing goes
    insert(text, mods) {},     // mods: {ctrl, alt} or null
    deleteBack(n) {},          // n characters as people see them (an emoji is one)
    key(name, mods) {},        // enter, tab, esc, left, right, up, down, home, end, paste
    context() {}               // optional: the text before the caret ('' or undefined when unknown)
  },
  mode: 'text', enterLabel: 'Done',
  host: 'web',                 // 'web' | 'mantel' (RT and R3 are the pointer's) | 'browser'
  onChange() {}, onHide() {}, onLeave(dir) {}, onHaptic(strength, ms) {}, onSound(kind) {},
  suggest(typed, previousWord) {}   // optional: your own suggestions (return null for the word list's)
});
engine.handle('a', true);      // a button down (true) / up (false), or 'press' for sources without releases
engine.axes(lx, ly, rx, ry);   // sticks, -1..1
engine.tick();                 // every frame while open: repeats, held A
engine.tap(r, c); engine.longPress(r, c); engine.pickSuggestion(i); engine.pickAlt(i);
engine.reset(mode, enterLabel); engine.setActive(bool); engine.setHints(bool); engine.refresh(); engine.view();
const view = HearthOSK.mount(element, engine, { overGame });   // draws it in `element` (view.destroy())
```

Buttons: `a b x y lb rb lt rt l3 r3 view menu up down left right` (`start` / `select` work as `menu` / `view`).

Adapters for the usual cases: `HearthOSK.inputAdapter(element, onEnter)`, `HearthOSK.modelAdapter(get, onChange, onKey)`
(a `{text, caret}` object, as Game Mode's prompt).

### Settings, words, layouts

`HearthOSK.configure({settings, systemLayouts, learned, save(data), loadWords(lang), clipboard})`,
`HearthOSK.setSetting(key, value)`, `HearthOSK.settings()`, `HearthOSK.activeLayouts()`, `HearthOSK.LAYOUTS`,
`HearthOSK.THEMES`, `HearthOSK.DEFAULTS`, `HearthOSK.words(lang, 'space separated words')`,
`HearthOSK.suggest(lang, typed, prev, max)`, `HearthOSK.exportLearned()`, `HearthOSK.importLearned(data)`,
`HearthOSK.clearLearned()`, `HearthOSK.learnedCount()`, `HearthOSK.layoutForXkb('de')`, `HearthOSK.layoutForLang('pt-BR')`.

### Hearth's calls (hearth-core `osk.rs`)

`osk.get` → `{settings, learned, systemLayouts}`, `osk.set {settings}`, `osk.learn {data}` (merged into the file:
each word's higher count), `osk.clear`.

### Game Mode

`openPrompt({title, value, secret, mode, enterLabel, placeholder, osk})` resolves with the text (or null). `mode` is
one of the kinds above; `osk: true` shows the keyboard even after keyboard or mouse input.

The controller reaches the keyboard one of two ways: the page reads it itself (Gamepad API, `pollPadsNow`: Linux Game
Mode), or Hearth sends the presses (`onSysEvent('pad')`: the Windows host, Multi-seat pads, the overlay). Send logs
says which, once each time the keyboard gets the selection and on the first press each way:
`osk search: keyboard has the selection · controller read here (Gamepad API) · pads seen 1 (Xbox Wireless Controller)`,
`osk search: first press right via gamepad` (or `via host`). A script error in the controller loop is logged once,
then counted every 10 s (`controller: the same error N more times`).

## Tests

- `tools/ui-check.js` (release.yml): in Game Mode, the controller walks to each key with the D-pad and types, search
  finds games, autocapital, suggestions picked from the strip, double space, X, accents with A held, LT, emoji (L3),
  B / A on the field, R3 to AZERTY, the Ember Wheel, split sticks, a click, ☰ finishing, the split / Daylight look,
  a real keyboard sending it away, clearing learned words, Settings › Keyboard. Pictures as notices
  (`SHOT osk-<which>`). Then the same through a pretend controller (`navigator.getGamepads`, buttons and the left
  stick, the way Linux Game Mode reads it) in search (Y), the Library's search prompt, Send logs' note and a password
  prompt, plus Hearth's own presses (`onSysEvent('pad')`), with the Send logs lines checked; and the docked keyboard
  (`HearthOSK.open`) in hearth-time, hearth-maps and hearth-contacts with the same pretend controller.
- `hosts/linux/test/osk.test.js` (node --test): the engine on its own.
- `hosts/linux/test/test_hearth_osk.py`: copies, plain-JavaScript rules, word lists, wiring, and no name in Game
  Mode's controller loop declared again in an inner block (0.6.5's `ax` broke the controller on the keyboard).
- `hosts/linux/test/test_hearth_pad.py`: hearth-pad hands the keyboard its buttons and sticks, types without wtype;
  Guide + X in Hearth Classic and with `--osk` (Hearth Plasma: nothing else, the controller held while it's open).
- `hosts/linux/test/test_hearth_input.py` and `tools/e2e-hearth-input.py`: Guide + X from Hearth Input, and typing
  (characters to keys for us and de, Backspace) through its real uinput keyboard. `tools/ui-check.js`: the keyboard
  over a game opens alone, its presses send `osk.type` / `osk.key`, B closes it.
- `.github/workflows/desktop-test.yml`: Desktop Mode types into a real app (foot running `cat`) through the keyboard:
  taps, a picked suggestion, X / Y, an accent with A held, an emoji, the Ember Wheel, Enter; pictures `osk-*.png`.
- `tools/hearth-browser-test.py`: Hearth Browser's keyboard types into a web page.

## Word lists

`tools/osk-words.py` makes `ui/osk/words/<lang>.js` from [FrequencyWords][fw] by Hermit Dave (OpenSubtitles 2018;
content CC BY-SA 4.0, code MIT): the 6000 most used words, without words with digits or other scripts, lone letters
that aren't words, and swear words.

[pim]: https://pimylifeup.com/steam-deck-desktop-mode-keyboard/
[pcgn]: https://www.pcgamesn.com/steam-deck/keyboard-desktop-mode
[tg]: https://tomsguide.com/how-to/how-to-change-the-keyboard-theme-on-steam-deck
[gol-2204]: https://gamingonlinux.com/2022/04/steam-deck-gets-a-bunch-more-keyboard-tweaks
[gol-2205]: https://gamingonlinux.com/2022/05/steam-deck-gets-a-small-update-to-add-more-keyboard-layouts
[gsm]: https://m.gsmarena.com/valve_steam_deck_stable_beta_software_updates-news-53805.php
[sdhq]: https://steamdeckhq.com/news/steam-deck-client-update-steam-input-keyboard-and-much-more/
[steam-xy]: https://steamcommunity.com/app/1675200/discussions/2/3395175706741961527
[steam-bind]: https://steamcommunity.com/app/1675200/discussions/0/3370405530908624210
[steam-pred]: https://steamcommunity.com/app/1675200/discussions/2/7198510808235608636
[gb]: https://gardinerbryant.com/12-hidden-features-of-the-steam-deck/
[eng]: https://www.engadget.com/2012-09-11-steam-big-picture-beta-hands-on.html
[kodi]: https://forum.kodi.tv/showthread.php?tid=327109
[ps5]: https://www.playstation.com/en-gb/support/hardware/ps5-button-functions/
[mdn]: https://developer.mozilla.org/en-US/docs/Web/API/GamepadHapticActuator/playEffect
[fw]: https://github.com/hermitdave/FrequencyWords
