# Controller apps and the desktop controls

The problem the owner reported for 1.0: in Desktop Mode (Mantel), **apps with their own controller controls only work
sometimes, or lose attention because of the desktop controls**. This page explains why, and how 1.1 decides who gets the
controller.

## How it worked in 1.0

### The desktop controller helper (hearth-pad)

`hosts/linux/desktop/shell/hearth-pad` (started by `Pad.qml`) reads every controller in `/dev/input` with python3-evdev
and acts as a virtual mouse and keyboard through `/dev/uinput`: right stick pointer, A / RT click, X / LT right-click,
B Esc, D-pad arrow keys, LB / RB Alt+Tab, Y the on-screen keyboard, Menu the launcher, View the controls card (View + D-pad
snaps, View + B closes), Guide the Hearth menu (radial), Guide + X the keyboard.

It knew only three things about the screen, all sent by `Pad.qml`:

- `pause {on}`: the active window is **fullscreen** (`ToplevelManager.activeToplevel.fullscreen`). Then it sent nothing
  but kept Guide.
- `keys {on}`: one of Hearth's own menus or windows is in front (launcher, Quick Access, power menu, Hearth Settings and
  Hearth Files by window title, Dolphin's app id): A is Enter, B Esc, D-pad arrows, still no grab.
- `mouse {on}`: the Hearth menu's "Controller mouse" switch.

There was **no notion of "an app that reads the controller itself is in front"**: no app id list, no `.desktop` key, no
socket. The only Hearth app that stood aside was one running fullscreen.

It never held (EVIOCGRAB) the controller in Hearth Classic: the grab was only used by Hearth Plasma's keyboard
(`--osk` / `--plasma`). It did a one-off grab-and-release on every newly found device to tell whether someone else
(Steam) holds it, and looked for new devices every 3 s.

### How the apps read the controller

- **WebKitGTK apps (Tauri)**: Mods, Saves, Controller Lab (`hosts/hearth-gaming/ui/kit.js`), Hearth's tools (Logs,
  Network, Passwords, Printers, System Info, Benchmark, Disk Usage, Welcome: `hosts/hearth-tools/ui/common.js`), Clock /
  Calendar / Notes / Weather, Photos, Player, Paint, Emoji, Fonts, Recorder, Camera, Share Screen, Documents, Contacts,
  Maps, Calculator, Disks, Nearby, Connect, Office. Each had its own copy of the same loop:
  `navigator.getGamepads()` in a `requestAnimationFrame` callback that scheduled itself again **at its end**.
  WebKitGTK reads the controllers itself (libmanette, evdev) and gives a page the Gamepad API state **only while its
  window is the active one**, and only after a button was pressed on a controller while the page was open.
- **Theater** (Chromium/Electron): its own page reads the Gamepad API (`viewer-pad.js`) and moves **its own uinput
  pointer** (`hosts/hearth-theater/src/pointer.rs`).
- **Hearth Files from Dolphin** (`hearthgamepad.cpp`): evdev itself while its window is active, only LB/RB, LT/RT and
  View; hearth-pad's keys context does the rest.
- **Mantel's own windows** (Hearth Settings, Hearth Files, Task Manager, the menus): don't read the controller; they get
  hearth-pad's keys (the `keys` context).
- **Game Mode** (`hearth-pad --game`, hearth-core `padmouse.rs`): only the controller mouse; it pauses while Hearth or its
  Quick Access has the controller. Hearth Input (`hearth-input`) only runs in Game Mode.
- **Handheld Daemon** (ROG Ally): hides the built-in pad and makes a virtual "Xbox Elite" (and Hearth's sign-in screen
  its own "Hearth login controller"); everything above reads the virtual one.

### What went wrong (root causes)

1. **Double input (the main cause).** With a Hearth app in front (not fullscreen), hearth-pad kept acting as mouse and
   keyboard while the app read the same presses through the Gamepad API. A pressed the app's selected button *and*
   clicked wherever the pointer was resting (often another window or the desktop: the app lost the focus); B was the
   app's Back *and* Esc; the D-pad moved the app's selection *and* sent arrow keys (two steps, or a list scrolling
   under the selection); the right stick moved the pointer over other windows; **LB / RB were the app's tabs *and*
   Alt+Tab**, which switches the focus to another window: the app "loses attention" and, because WebKitGTK only feeds the
   active window, its controls stop until it's clicked again. Y also opened Mantel's keyboard over apps that have their
   own.
   *Evidence*: the hearth-pad sections of both 2026-11-02 logs. Nvidia PC (F2FK-KFKP-DD), Desktop Mode 00:56–02:25:
   `controllers: /dev/input/event22 'Xbox Wireless Controller'`, then only `last 30 s: N controller events, pointer moved
   X px` lines (e.g. `01:57:10 last 30 s: 564 controller events, pointer moved 0 px`: buttons in use with no pointer
   movement, which fits an app being driven by its own controls while hearth-pad typed and clicked along), and no `paused` line at
   all in Desktop Mode: the helper was emulating the whole time. ROG Ally (43DC-MYM8-97): the same pattern on
   `event18 'Xbox Elite'` (Handheld Daemon's virtual pad); its only pauses are `paused: a fullscreen window is in front`.
2. **A controller loop that dies on the first error.** Every app scheduled its next frame at the end of its loop; one
   exception in a handler (a missing element, a dialog closing under it) ended the loop for good: "works sometimes",
   until the app was restarted. Hearth Office's loop also stopped whenever no pad showed and only restarted on
   `gamepadconnected`, which fires once per page.
3. **Frozen state after a focus change.** When the window loses the focus (Quick Access, a menu, Alt+Tab), WebKitGTK stops
   updating the pads; a button held at that moment stays "down" in the page. Auto-repeat on a held D-pad kept moving; and
   when the focus came back, the press that closed Quick Access or picked in the Hearth menu landed in the app too.
4. **Hearth's menus over an app**: the Hearth menu (Guide) and Mantel's keyboard don't take the window focus (by
   design: typing goes to the app), so the app went on reading the controller and acted on the same D-pad / A presses
   that drove the menu or the keyboard.
5. **Focus not handed back** after Quick Access / the launcher / the power menu: those layer-shell panels take the
   keyboard exclusively; on closing, labwc gives it back to the top window in most cases, but nothing made sure, and an
   app without the keyboard gets no Gamepad API.
6. **The first press after opening**: WebKitGTK shows a page no controller until a button is pressed while it's open
   (the Gamepad API's rule). *Evidence*: Hearth's own log, `osk prompt: … controller read here (Gamepad API) · pads seen
   1 (Xbox Wireless Controller)` then `osk prompt: first press start via gamepad`.
7. **Someone else holding the controller**: when Steam (or another program) holds the device exclusively, neither
   hearth-pad nor WebKit gets anything. *Evidence*: F2FK-KFKP-DD, 2026-11-01 19:05:10 (Game Mode): `/dev/input/event22
   'Xbox Wireless Controller' is held by another program (Device or resource busy): its buttons don't reach the desktop.
   Is Steam open?`. Not changed here (Steam's to let go); the log line stays.

Checked and not a cause: Mantel's notifications, the volume / brightness OSD and toasts never took the keyboard
(layer-shell `keyboardFocus` None, the default); the on-screen keyboard never takes it either (it types into the window
you're in); labwc has no focus-follows-mouse (`rc.xml`: click to focus), so the pointer passing over windows doesn't move
the focus by itself. Hot-plug: new controllers were found within 3 s; Handheld Daemon's virtual pad coming back after a
restart gets a new event number, which WebKit picks up by itself.

## How it works now (1.1)

### One rule: the window in front owns the controller

- **Mantel tells hearth-pad which window is in front**: `focus {app, title}` with the Wayland app id from labwc's
  foreign-toplevel list (`Pad.qml`, `ToplevelManager.activeToplevel`), whenever it changes and when the helper (re)starts.
- **hearth-pad decides whether that app reads the controller itself** (`Owners` in hearth-pad). Any one of these is
  enough:
  1. the shared list `hosts/linux/desktop/shell/controller-apps.json` (installed next to hearth-pad; every Hearth app
     with controller controls, by window app id);
  2. `X-Hearth-Controller=true` in the app's `.desktop` file (its file name and `StartupWMClass` count; looked through
     again at most every 30 s, so a newly installed app is picked up);
  3. a claim over the helper's socket `$XDG_RUNTIME_DIR/hearth/pad.sock` (`{"claim": "<app id>"}`, one JSON object per
     line, held while the connection stays open; this user only: folder 0700, socket 0600, SO_PEERCRED). For programs
     that can't be listed ahead (a QML or native app of yours). Web pages can't open sockets, so Hearth's WebKit apps are
     covered by 1 and 2.
- **While a controller app is in front, hearth-pad sends nothing**: no pointer, scrolling, clicks or keys, no Alt+Tab,
  no launcher, controls card or Mantel keyboard from Y. The app gets clean input.
  It keeps only the system chords:
  - **Guide**: the Hearth menu (Quick Access, apps, close window…);
  - **Guide + X**: the on-screen keyboard;
  - **Guide + R3**: the controller mouse *in that app* (for the odd dialog the app's controls can't reach); Guide + R3
    again, or another window coming to the front, gives the controller back to the app. The Hearth menu's
    "Controller mouse" item does the same over a controller app.
- **The controller is never held while the app has it.** It is held (EVIOCGRAB) only while the Hearth menu or Mantel's
  keyboard is open over a controller app, and while Guide + R3's pointer is on, so the app doesn't act on those presses;
  it is let go the moment they close. WebKit keeps its device open and simply sees nothing meanwhile.
- **Hearth's menus still win**: while Quick Access, the launcher or another Mantel menu has the keyboard over a
  controller app, hearth-pad types its keys as before (the app isn't focused then, so it can't double up).
- **Leaving**: another window in front (or the app closing) puts the desktop controls back at once; anything held is let
  go on every switch.
- **Hot-plug**: while no controller is connected the helper looks every second (every 3 s otherwise), so a controller
  switched on or Handheld Daemon's virtual pad coming back is used straight away; a grab that's in force applies to it.
- **The log says what it decided** (`~/.local/share/nimbus/pad.log`, in Send logs under "Desktop controller helper"):
  `controller: 'hearth-mods' in front reads the controller itself: desktop controls off (Guide: Hearth menu, Guide + R3:
  pointer)`, `controller: 'foot' in front: desktop controls on`, `controller: pointer in 'hearth-mods' on (Guide + R3)`,
  and the 30-second line adds `'hearth-mods' has the controller`.

### The apps: one shared reader

`ui/pad/hearth-gamepad.js` (copied next to each app page by `tools/pad-sync.py`; `--check` runs in the tests) is how
every Hearth WebKit app reads the controller now. Each app's change is small: the page loads it first, reads
`HearthGamepad.pads()` instead of `navigator.getGamepads()`, and runs its loop with `HearthGamepad.loop(fn)`.

- **The loop can't die**: an error in the app's handler is logged and the next frame comes anyway; frames come from
  `requestAnimationFrame`, and from a 100 ms timer when WebKitGTK stops giving frames.
- **No frozen presses**: while the window isn't focused or is hidden, `pads()` is empty (held buttons are let go in the
  app); a page that never saw a `blur` counts as focused, so a WebKit build whose `hasFocus()` is wrong can't lock the
  controller out.
- **No leaked presses**: when the focus comes back, a button or stick still held from before counts as let go until it
  really is (the A that closed Quick Access doesn't also press in the app).
- **Re-acquired every frame**: the pads are read fresh each frame (indexes and devices may change when Handheld Daemon's
  pad comes back); `gamepadconnected`, `focus`, `pageshow` and `visibilitychange` wake the loop.
- **The first press**: where a controller was used in this app before and none shows a moment after the window opened,
  a small note says "Press any button on your controller to use it here", once; it goes with the first press.

Theater is listed (hearth-pad stands aside, so its own pointer and the helper's no longer both move) but keeps its own
reader: its controller page sits under a separate viewer surface, where window focus isn't the page's.

### Focus stealing

- Notifications, the volume / brightness OSD, the toasts and the Hearth menu say so explicitly now
  (`WlrLayershell.keyboardFocus: WlrKeyboardFocus.None`): they never take the keyboard (or a WebKit app's controller).
- The on-screen keyboard never takes the keyboard focus; it only opens when you ask (Y outside controller apps,
  Guide + X anywhere, a tap).
- **Quick Access, the launcher, the power menu, the clipboard, Friends and the desktop's menus give the focus back**:
  `Pad.qml` remembers the window in front when one opens and, 180 ms after it closes, activates that window again
  (foreign-toplevel activate), unless another window came to the front meanwhile (an app you started from the launcher).

### What isn't covered

- **Game Mode**: Hearth's controller mouse (`hearth-pad --game`) doesn't know about controller apps opened in Game Mode
  (Mods from a game's menu) while you've turned the controller mouse on yourself; it turns on by itself only over Steam
  and game windows, never over Hearth's apps. Steam Game Mode windows are Steam Input's.
- A controller held exclusively by another program (Steam) reaches neither the helper nor the apps.

## Tests

- `hosts/linux/test/test_hearth_pad_focus.py`: the window in front → the helper's mode (no output while an app owns
  it, Guide / Guide + X / Guide + R3, the grab only around the menus, menus' keys over an app, switching releases
  what's held, claims over the socket and their end with the connection, the list, the `.desktop` key and its rescan,
  `tools/pad-sync.py --check`).
- `hosts/linux/test/hearth-gamepad.test.js` (`npm test` in `hosts/linux`): the shared reader with a fake Gamepad API,
  focus and frames.
- `desktop-test.yml` ("Controller apps own the controller"): real labwc + Mantel, hearth-pad with a simulated controller
  (`HEARTH_PAD_SIM`: presses from a file, what it would send to uinput in `out.log`), a stand-in controller app (a window
  with Mods' app id typing into a file) with a notification, the OSD, Quick Access and the Hearth menu over it. It checks
  that the helper sends nothing while the app is in front, that typing keeps landing in the app through all of it, that
  Guide + R3 clicks, and that another window gets the desktop controls back; pictures `pad-*.png`, `pad.log`.
