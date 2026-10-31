# Hearth Input

Hearth's own Steam Input: a game without controller support gets the controller as a keyboard and mouse while it's
in front. Off by default (the game's own controller support); Beacon's strategy games start with "Strategy (RTS)"
and launchers' windows with "Mouse only".

- The mapper: `hosts/linux/desktop/shell/hearth-input` (Python, started with Game Mode by `nimbus-session`; it ends
  with it). It grabs the controller (EVIOCGRAB) only while a game with a layout is in front, and presses keys,
  buttons and the mouse through `/dev/uinput`.
- Each game's choice, the checks and the UI's calls: `hosts/hearth-core/src/input_map.rs`. The tables (keys,
  controls, limits, templates): `hosts/linux/desktop/shell/input-layouts.json`, read by both.
- The editor: `ui/index.html` (Game settings › Controller layout › Edit, and Quick Access › Controller layout over
  the running game).

## Who holds the controller

| What's going on | Who has it |
|---|---|
| A game with a layout is in front | Hearth Input (grabbed: the game doesn't see it) |
| Quick Access, the keyboard over a game (Guide + X), Hearth on screen, the game not in front | Hearth (Hearth Input lets go first; Hearth asks and waits for the answer) |
| Hearth stops talking for 5 s, Hearth Input ends or is killed | nobody: everything is let go |
| A controller someone else holds (EBUSY: Handheld Daemon's hidden pad) | left alone |

Guide over a layout: everything is let go, and the controller stays held while Guide is down. Let go (or held
0.35 s) it's Guide as always (Quick Access); X before that is **Guide + X**, Hearth's on-screen keyboard over the game
(docs/OSK.md), and the game sees neither press.

## Layouts (version 2)

```
{"version": 2, "name": "…", "base": "<template id>",
 "buttons": {control: binding}, "sticks": {"ls"|"rs": stick}, "gyro": {…},          the default action set
 "sets":   [{"id", "name", "buttons", "sticks", "gyro"?}],                           more action sets
 "layers": [{"id", "name", "buttons", "sticks"}],                                    on top of a set
 "radial": [action, …], "hold_ms": 300, "double_ms": 250, "turbo_hz": 10}
```

- **Controls**: `a b x y lb rb lt rt view menu l3 r3 up down left right l4 r4 l5 r5`, sticks `ls rs`. Guide is
  never mapped (it always gives the controller back to Hearth).
- **Actions**: `key:A+B` (up to 4 keys at once), `mouse:left|right|middle|back|forward`, `scroll:up|down|left|right`,
  `none`, and on a set's buttons also `layer:<id>` (hold: a mode shift), `layer_toggle:<id>`, `set:<id>`
  (`set:default` back), `radial`, `gyro:toggle`, `gyro:hold`, `gyro:ratchet` (gyro off while held). A layer's
  buttons can't hold another layer or open the radial menu.
- **Bindings (activators)**: an action string, or `{"press", "hold" (long press, after hold_ms), "double" (a second
  press within double_ms), "release" (a tap when let go), "chords": [{"with": control, "do": action, "turbo"?}],
  "turbo", "hold_turbo", "double_turbo", "toggle", "threshold" (triggers)}`. Long, double, release, chord and stick
  directions only press keys, click or scroll; turbo only repeats keys and clicks; toggle holds a key or click until
  the next press.
- **Action sets** replace every control; switching resets the toggled layers. **Layers** change only the controls
  they set; held ones go on top of toggled ones; a control a layer doesn't set does what the set says.
- **Sticks**: `none`; `keys` (8 ways) and `dpad` (4 ways) with `up down left right threshold`; `mouse`
  (`sensitivity deadzone outer curve invert_x invert_y`); `scroll` (`sensitivity deadzone`); `wheel` (turn the stick
  round to scroll: `sensitivity threshold`); `flick` (flick stick: push it out and the camera turns at once to where
  it points, turn it round to keep turning; `turn` is the game's full turn in mouse pixels, `threshold`).
- **Gyro**: `when` = `button` (gyro:toggle / gyro:hold, or `start_on`, or Quick Access), `always` or `stick` (while
  the right stick is touched / moved: Hearth reads the stick's position, not capacitive touch); `axis` = `yaw` or
  `roll` for left and right; `sensitivity deadzone invert_x invert_y`. A set can have its own gyro block. A ratchet
  button turns it off while held, to re-center.
- **Strict and safe**: unknown fields, keys or actions are refused, numbers are range-checked, and nothing in a layout
  can run a program: only keys, buttons, the mouse, sets, layers, gyro and the radial menu. Both checkers
  (`validate_layout` in hearth-input, `validate` in input_map.rs) are tested against the same cases,
  `hosts/linux/test/fixtures/input-layout-cases.json`.

### Version 1

Files written before (`"version": 1`, or no version and the second set) are read and migrated: the second set
(`"shift"`) becomes the layer `shift`, and buttons set to `"shift"` hold it (`"layer:shift"`). They're saved as
version 2 the next time they change.

## Where layouts live

`~/.config/nimbus/input/<game id>.json`: `{"version": 2, "layout": "off" | <template id> | "custom" | "global",
"custom": {layout}, "prefs": {"sensitivity", "gyro"}}`, and `global.json`: `{"version": 2, "enabled", "layout"}`,
the **global layout**: any game can choose it, and with "Use for every game without its own layout" on, games with
no choice of their own (that would start Off) use it. Games with their own default (Beacon, launchers) and games you
set to Off keep theirs. Hearth Classic's desktop keeps its own controller mapping (hearth-pad).

## Across your devices (Hearth Link)

Controller layout › From your other devices: every paired device lists its own layouts (each game's custom one and
its global one; Hearth Link request `input.layouts`, UI call `link.input.peers`). Picking one checks it again here
(`input.validate`) and saves it as this game's layout (or the global one). Pull only: nothing is written on the other
device. A game copied over Hearth Link brings its layout file too.

## The editor

All with the controller: rows and sheets, the D-pad and A. In Game Mode it's its own screen; over a game it's a panel
in Quick Access (the controller is Hearth's while Quick Access is open; the layout applies when it closes).

- Rename, Try it (20 s: the controller acts as the layout right there, and what it sends shows on screen; Guide stops
  it), Overview (every control in every set and layer), Start over from a template.
- Editing: the default set, an action set or a layer; add, rename and remove sets and layers (buttons that named a
  removed one do nothing).
- Each button: an action, then "More ways to press" (long, double, release, chords, turbo, toggle). Each stick: its
  mode and its sliders. Gyro, the radial menu, timing (long press, double press, turbo).

## Calls

`input.tables`, `input.get {game}` (with `global`), `input.set {game, layout, custom, prefs}`, `input.test {layout,
seconds}` / `{stop}`, `input.status`, `input.global.get`, `input.global.set {enabled, layout}`, `input.validate
{layout}`, `osk.type {text, mods}`, `osk.key {key, n, mods}`. The mapper's socket (`<runtime dir>/hearth/input.sock`,
this user only): `state`, `overlay`, `test`, `status`, `type`, `key`; events `guide`, `osk`, `osd`.

## Tests

- `hosts/linux/test/test_hearth_input.py`: the checks (shared cases, migration, round trips), the engine (sticks,
  tap / hold, double, chords, release, toggle, turbo, sets, layers, d-pad, wheel, flick, gyro modes, radial menu),
  the grab policy, the daemon with fake devices (Guide, Guide + X, typing), the socket.
- `tools/e2e-hearth-input.py` (Hearth Input e2e workflow): the real daemon with uinput controllers (Xbox 360,
  Bluetooth Xbox, DualSense), the grab and hand-offs, Guide, **Guide + X** (Hearth gets `osk`, the game never sees the
  X), typing (us and de) and Backspace through the virtual keyboard.
- `input_map.rs` tests (rust workflow): the same shared cases, version 1 files, the global layout. `inputs.rs`
  (hearth-link): only layouts that pass the checks come in from another device.
- `tools/ui-check.js`: the keyboard over a game, and the editor in Quick Access (a layer, a double press and chord,
  a flick stick, saved, the overview).

## Needs a real device

Gyro modes and flick stick feel (the turn calibration), Bluetooth pads' Guide (KEY_HOMEPAGE), the Guide + X chord with
Handheld Daemon's overlay app and InputPlumber, and typing through gamescope (its keymap and STEAM_INPUT_FOCUS 2).
