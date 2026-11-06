# Server mode (Hearth 1.6)

Server mode is a third mode next to **Steam Game Mode** and **Desktop Mode** (Mantel), for a PC that's also your home
server. It's the same Hearth OS image with a switch (Settings › Home server › Server mode › **Use Server mode**), no
separate edition, and it's **off until you turn it on**: Hearth still starts in Game Mode unless you pick Server mode.

In Server mode the screen shows one full-screen **dashboard**: is everything fine, the **Play** button (Game Mode), every
service as a tile (AI chat, Pictures, Media, Storage, Backups, Heat & power, Who's connected, Remote desktop) and the
latest alerts. Nothing else runs on the screen: no Steam, no desktop, no taskbar or menu. Your services (shared folders,
Jellyfin, Ollama, ComfyUI, Kokoro, Sunshine, backups, AI sharing) keep running as their own services underneath; the
dashboard only shows them and lets you restart or pause them.

## Using it

| On the dashboard | Controller | Keyboard |
|---|---|---|
| Open a tile (its details) / the Play button | A | Enter |
| Restart the selected service | X | X or the Menu key |
| Pause or resume the selected service | Y | Y |
| Menu: Game Mode, Desktop Mode, Server mode settings, Hearth Settings, Screen off, Sleep, Restart, Turn off | ☰ | M |
| Play: switch to Game Mode | Guide | G |
| Back / cancel | B | Esc |

- **Play** (or Guide) shows a **5-second countdown**: what pauses while you play (Pictures stop with their queue kept, AI
  chat switches to the small model, voices pause, backups wait) and what keeps working (shared folders, media, Tailscale,
  the web page, remote desktop). **A** switches now, **B** cancels. It then switches to Steam Game Mode the way Game Mode
  and Desktop Mode already switch (nimbus-mode), without signing out.
- **Leaving Game Mode** (Steam's "Switch to Desktop") comes back to Server mode, and **Back in Server mode** says how long
  you played and what's coming back. Desktop Mode is in the dashboard's menu.
- **Idle**: after the minutes you chose with nothing pressed, the screen **dims** to a clock and the status (dark grey,
  kind to OLED screens), then goes **black** (and off, when `wlopm` is installed). Any button wakes it.
- The PC **stays awake** in Server mode (unless you chose a Sleep time). With **Lower power limits when idle** on, after a
  while with no jobs the processor goes to power-saver and an Nvidia graphics card to the limit you chose (120 W by
  default); full power comes back the moment a job starts, always before Game Mode, and whenever Server mode ends.

## Settings

- **Settings › Home server › Server mode**: Use Server mode, Open now, Startup and schedule; power when idle (limits, drive
  spin-down, the screen dimming then off, sleep); **what pauses when you play** (Light mode's own settings, the same ones as
  Home server › Gaming: one source of truth); the **web page** (on/off, its PIN, and what it may do).
- **Settings › System › Startup** (and Gaming › Start in): start in Game Mode, Desktop Mode or Server mode. With Server mode
  on you can **follow a schedule** instead (weekdays and the weekend, each a few changes such as "Server mode from 12 AM,
  Game Mode from 6 PM"). **There is no schedule unless you make one.** A change never closes a game (it waits until the
  game ends), never switches away from Desktop Mode, and counts down 10 seconds on the dashboard first.
- **AI chat while you play**: "Switch to the small model" (the default once Server mode is on): the big model
  (qwen3:30b-a3b) leaves the graphics card and **qwen3:14b runs on the processor** (Ollama `num_gpu 0`, kept loaded), so
  your other devices still get answers; back in Server mode the small one is unloaded and the big one loads again on the
  next question. Only when qwen3:14b is downloaded; models are never downloaded or deleted by this. The other choices:
  unload the model, stop AI chat, keep it loaded.
- **Pictures (ComfyUI) while you play**: "Stop, keep the queue": ComfyUI finishes the picture it's making (up to 2½
  minutes), its queue is saved (root only, `/run/hearth/comfyui-queue.json`, 0600), and put back in order when it
  starts again.

Server mode's own choices are in `~/.config/nimbus/server-mode.json` (written through hearth_settings_io: only the key you
change, atomic, `.bak`, merged under the lock; defaults are filled in when read and never saved). Start in is
`session.conf`'s `NIMBUS_START_MODE` (only that line changes). State is never in settings: the PIN's hash and the web
page's sign-ins (`~/.local/state/nimbus/server-mode-web.json`, 0600) and `$XDG_RUNTIME_DIR/hearth-server/` (requests, the
countdown, what idle power changed, pauses you made).

## The web page (port 8460)

`http://<your PC's name>:8460` on your phone or laptop (phone and laptop layouts): status, every service with Pause and
Restart, switch modes, sleep or restart the PC.

- **Only over Tailscale, never the internet or your home network**: it listens only on `127.0.0.1` and this PC's
  Tailscale address, refuses any other address (only 127.0.0.0/8, ::1, 100.64.0.0/10 and fd7a:115c:a1e0::/48 get an
  answer), and the home server's nftables rule (table `inet hearth_home_server`) drops port 8460 on every interface but
  `lo` and `tailscale0` while the page is on.
- **PIN protected**: 4 to 12 digits, set in Settings; only a PBKDF2-SHA256 hash is kept. A device that typed it stays signed
  in for 30 days (an HttpOnly, SameSite=Strict cookie; only its hash is kept on the PC). Five wrong PINs in five minutes:
  wait. A new PIN signs every device out. No PIN, nobody gets in.
- **It can do everything, and each permission can be turned off** in Settings: restart and pause services, switch modes,
  sleep or restart the PC, and "show it on my other Hearth devices" (the read-only summary below, without the PIN).
- Writes need the page's own header (`X-Hearth: 1`) and its own origin, so another site can't press its buttons. The page
  never runs anything itself: it leaves a request for the session's **agent** (`hearth-server-mode agent`, running in your
  sign-in at the PC), which checks it again. A switch or sleep from the web shows a **10-second countdown on the home
  screen** (cancel it there with B); nothing switches while a game is running, and the web can't switch away from Desktop
  Mode (someone may be working there).
- Its user service, `hearth-server-mode-web.service`, is off in the image and enabled for your account only when you turn
  the page on and have set a PIN.

`GET /api/summary` is the dashboard in short (read only; JSON): Hearth Link and Quick Access on your Ally can read it from
your Tailscale devices without the PIN while "show it on my other Hearth devices" is on. On the PC itself:
`hearth-server-mode summary`.

## How the session works (and why it always ends in Game Mode)

`nimbus-session` has a `server` mode (`run_server`, between its `# >>> server mode` marks). It starts **labwc with its own
folder** (`desktop/server/labwc`: no shortcuts, no root menu) whose autostart runs only the dashboard
(`quickshell -p desktop/shell/server-mode.qml`). Every failure path ends in Game Mode:

- Server mode off, labwc / Quickshell / its files missing, an extra multi-seat player: Game Mode (the reason in
  `$XDG_RUNTIME_DIR/hearth-server-mode-failed`).
- The dashboard must say it's up (`$XDG_RUNTIME_DIR/hearth-server-dash-up`) within 60 seconds, or labwc is closed and Game
  Mode opens: a black screen never stays.
- The dashboard closing three times within 20 seconds of starting: the autostart gives up and ends labwc; Game Mode opens.
- labwc ending without a switch asked for: Game Mode.
- Two failed runs within 10 minutes: Server mode is paused (Game Mode at every start) until you open it yourself (Settings'
  Open now, `nimbus-mode server`) or a run stays up for 2 minutes.
- Server mode failing never comes back to Server mode by itself, so it never goes back and forth; Steam Game Mode failing
  after you pressed Play comes back to Server mode (it's working), as Steam Game Mode's own fallback normally goes to
  Desktop Mode.

With Server mode off, nothing in Game Mode or Desktop Mode changes: the session never even asks the helper (it's only asked
when Start in is Server mode or Server mode's settings file exists). `hearth.mode=desktop` on the kernel line, B or Guide
held at start, and Hearth Recovery's "Desktop Mode this time" all still win. Settings' Open now and `nimbus-mode server`
switch from Game Mode or Desktop Mode the usual way.

## Hearth Link and the Ally

The summary at `/api/summary` (and `hearth-server-mode summary`) is what a Hearth Link page or Quick Access panel shows (the
mockup's "ally" screen). Until that page exists, the Ally can open the web page in a browser. (Follow-up: a Server mode
page in Hearth Link's Decky plugin reading `/api/summary`, and the Hearth AI Command Center honouring the small model while
playing, e.g. from AI sharing's document.)

## Files

- `hosts/linux/session/nimbus-session` (`run_server`, `server_start_mode`, `server_return`), `nimbus-mode server`
- `hosts/linux/desktop/server/labwc/` (rc.xml, environment, autostart)
- `hosts/linux/desktop/shell/server-mode.qml`, `ServerDash.qml`, `ServerTile.qml`, `ServerDetail.qml`, `ServerBack.qml`,
  `ServerLine.qml`, `ServerButton.qml`; Settings: `SetServerMode.qml`, `SetStartup.qml`, `StartInChoice.qml`
- `hosts/linux/desktop/shell/hearth-server-mode` (the helper, the agent, the web server) and `server-mode-web.html`
- `os/system_files/usr/lib/systemd/user/hearth-server-mode-web.service`
- the root helper `hearth-home-server-helper`: `svc`, `gpu-limit`, the small model and ComfyUI's queue in `services run`,
  port 8460 in the Tailscale-only rule
- Tests: `hosts/linux/test/test_hearth_server_mode.py` (Home server workflow); screenshots in desktop-test.yml
  (`shots/server-mode/`)
