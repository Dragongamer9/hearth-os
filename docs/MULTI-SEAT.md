# Multi-seat (experimental)

One PC runs two to four completely separate Hearth Game Mode sessions at the same time. Each player has their
own controllers and their own Linux account, so their own Steam account, Hearth profile and saves. It works like
several consoles in one box. Each extra player plays in one of four ways (they can be mixed, see "Mixing"):

| Way to play (Settings wording) | `mode` | Beta Experimental |
|---|---|---|
| Their own graphics card | `gpu` | (the whole feature is) |
| Another screen on this graphics card | `nested` | |
| Part of this screen (split screen) | `split` | yes, on top |
| Streamed to another device (Moonlight) | `stream` | yes, on top |

This is a different feature from Couch Co-op (`hearth-coop`), which splits one screen so several people can
play one game together. Multi-seat gives each player a whole Hearth of their own.

| Piece | Where |
|---|---|
| User-side helper (scan, plan, setup, start/stop, status, move a controller, remove, session, host) | `os/system_files/usr/libexec/nimbus/hearth-seats` |
| Root side (`seats-setup`, `seats-remove`, `seats-acl`, `seats-start`, `seats-stop`) | "multi-seat" section of `os/system_files/usr/libexec/nimbus/hearth-admin` |
| An extra player's session | `os/system_files/usr/lib/systemd/system/hearth-seat@.service` (instance = `hearth-p2`, `hearth-p3`, `hearth-p4`) |
| Start it when player 1 opens that player's screen | `os/system_files/usr/lib/systemd/system/hearth-seat@.path` |
| PAM service for those sessions | `os/system_files/etc/pam.d/hearth-seat` |
| A streamed player's Sunshine (user service) | `os/system_files/usr/lib/systemd/user/hearth-seat-sunshine.service` |
| Whose virtual controller is it (udev helper, streamed players) | `os/system_files/usr/libexec/nimbus/hearth-seat-input` |
| Settings › Games › Multi-seat (Beta Experimental) | `ui/index.html` (`seats` tab), bridge `seats.*` in `hosts/hearth-core/src/bridge.rs` |
| Tests | `hosts/linux/test/test_hearth_seats.py`, the Multi-seat steps in `tools/ui-check.js` |

Files written at set-up time (all listed in `/etc/hearth/seats.json` and removed by `seats-remove`):
`/etc/hearth/seats/hearth-pN.env`, `/etc/udev/rules.d/72-hearth-seats.rules`, `/etc/tmpfiles.d/hearth-seats.conf`,
`/etc/sddm.conf.d/zzz-hearth-seats.conf`, plus the `hearth-pN` accounts and the enabled units. With streamed
players also `/etc/udev/rules.d/74-hearth-seats-stream.rules`, `/etc/modules-load.d/hearth-seats-stream.conf`,
`/etc/hearth/seats/hearth-pN.stream.json` (their Sunshine's web login) and the firewall ports it opened
(`firewall_ports`). At run time: `/run/udev/hearth-seats-input.json` (hearth-seat-input's record).

## The kinds of hardware

### a. A graphics card per player (`"mode": "gpu"`)

Each extra player gets a whole graphics card with its own screen. Their session runs gamescope in DRM mode
(gamescope drives the screen directly, as in normal Game Mode) on their card:
`gamescope --prefer-vk-device VID:PID --prefer-output CONNECTOR`, with `LIBSEAT_BACKEND=noop` so gamescope
opens the card itself.

**Why this doesn't use logind seats (`loginctl attach seat1 …`)**, the textbook approach:

* SDDM supports multi-seat. As soon as `seat1` exists and has a graphics card, SDDM starts a **login screen
  on it**, and that login screen holds the card we want for player 2. SDDM only signs in automatically on
  `seat0`, and there's no setting to make it leave a seat alone.
* `loginctl attach` writes rules keyed on `ID_FOR_SEAT`, which comes from the device's path. That's fine for a
  card or a USB hub. It doesn't work for a Bluetooth controller (its path changes every time it connects), or
  for the controller's hidraw node, which Steam needs. The hidraw node sits beside the input device in sysfs,
  not under it, so it never inherits the input device's `ID_SEAT`.
* Steam Input's virtual controllers (uinput) are created under `/devices/virtual` and always land on `seat0`
  anyway, so real seats don't fix that either (see Limitations).

Instead, Hearth takes each extra player's devices **out of logind entirely** with one udev rules file, and
grants them to the player's account with an ACL (next section). Logind still starts a proper session for the
player, a session with no seat. That gives them `/run/user/UID`, their user services and D-Bus, but no
login screen and no fight over the card.

### b. One graphics card, several screens (`"mode": "nested"`, the most common case)

Neither logind nor DRM can split one card between processes. A card has one DRM master, and gamescope drives
only one screen. So player 1's Game Mode turns into the **host**:

1. `nimbus-session` (player 1) runs `hearth-seats host -- gamescope …` instead of plain gamescope.
2. `hearth-seats host` runs **labwc** on every screen, with a generated config (`$XDG_RUNTIME_DIR/hearth-seat-host/`):
   no decorations, no desktop keys, and one window rule per player.
3. Inside labwc, `hearth-seats host-run` opens a Wayland socket per player using **security-context-v1**
   (`wp_security_context_manager_v1`, a small Wayland client written with the Python standard library). Each
   socket is tagged with engine `hearth-seat` and app id `seatN`. labwc's rules match those tags
   (`<windowRule sandboxEngine="hearth-seat" sandboxAppId="seat2">`) and move that player's window to their
   screen (`MoveToOutput`). This is the only reliable way to tell the windows apart: every nested gamescope
   window has the same app id and title, and labwc can't match windows by process or user.
   * Player 1's socket: `$XDG_RUNTIME_DIR/hearth-seat1-wayland`. Player 1's gamescope runs nested on it.
   * Player N's socket: `/run/hearth-seats/hearth-pN/wayland-0`. The folder belongs to player 1 with group
     `hearth-pN`, mode 2750 (from the tmpfiles file); the socket is 0660.
4. When player N's socket appears, `hearth-seat@hearth-pN.path` starts `hearth-seat@hearth-pN.service`. That
   runs Hearth as `hearth-pN` in nested gamescope (`-W/-H` = the screen's size) on that socket.
5. When player 1's Game Mode ends (switching to Desktop Mode, signing out), host-run removes the sockets.
   The other players' windows lose their screen, their sessions end, and the path units wait for the next host.

**Why labwc and not a second gamescope, cage or sway:** gamescope can't drive several screens. cage mirrors
or extends a single kiosk app. sway would also work (`[pid=…]` rules and IPC), but it isn't in the image.
labwc is already installed for Desktop Mode, and it supports security-context-v1 and sandbox window rules.

### c. One screen split into parts (`"mode": "split"`, Beta Experimental)

One TV or monitor is divided into 2-4 parts, one whole Game Mode in each: side by side (`columns`), one above the
other (`rows`) or four corners (`grid`, 2-4 players). It's like Couch Co-op's split screen, but each part is an
independent Hearth with its own account, controllers, games and saves. It reuses the nested host:

1. Player 1 is always on the split screen (the UI puts player 1 in the first part; the parts can be swapped).
   `nimbus-session` runs `hearth-seats host -- gamescope -f …` as for nested; `host` sees that player 1 is a split
   player and rewrites the command (`player1_cmd`): `-f` goes, and `-W/-H/-w/-h` = player 1's part go last among
   gamescope's options (so they win over `session.conf`).
2. The generated `rc.xml` gets a labwc `<regions>` entry per split player, in % of the screen
   (`<region name="hearth-seat2" x="50%" y="0%" width="50%" height="100%" />`), and their window rule does
   `MoveToOutput` (the split screen) then `SnapToRegion` (their part), no decorations. Percent regions fit the screen
   wherever labwc places it among the outputs.
3. Each split player's session (`hearth-seat@hearth-pN.path` → `.service`, as nested) runs gamescope nested with
   `-W/-H/-w/-h` = their part (root works the size out from the screen's mode: `seats_split_rects`, even sizes), so
   games render at the part's size. `HEARTH_SEAT_WINDOWED=1` in their env file makes `nimbus-session` leave out `-f`.
4. Split can be mixed with `nested` (e.g. the TV split for players 1-2, a monitor for player 3): one host, some
   windows fullscreen on their screen, some snapped to a part.

**Input:** as nested: each player's controllers go to their account by the udev rules and ACLs.

**Sound:** everyone plays through the same speakers (the screen's). host-run makes a PipeWire remap sink per split
player on player 1's server (`module-remap-sink sink_name=hearth-seatN master=<the screen's sink>`), points each
player at theirs (`PULSE_SINK`/`PIPEWIRE_NODE` in `audio.env`; player 1's default sink), and sets its volume, or mutes
players 2-4 when "only player 1's sound" is on. Those settings are player 1's own (`~/.config/hearth/seats-audio.json`,
`hearth-seats audio`), applied right away while the host runs; no root needed. The sinks are unloaded and player 1's
default sink restored when the host ends.

**Lifecycle:** adding or removing players re-lays out the parts in the set-up screen (a third player switches to
four corners). Changing the set-up later is "set up again" (it replaces the old one, accounts kept); Remove
multi-seat puts everything back.

### d. Streamed to another device (`"mode": "stream"`, Beta Experimental)

An extra player plays on a TV or streaming stick with Moonlight, a phone, a tablet or a laptop. The PC streams that
player's own Game Mode to it; no extra screen or graphics card. 1-3 streamed players, mixable with any other kind.

* **The virtual screen.** The player's session (`hearth-seat@hearth-pN.service`, enabled directly: it doesn't wait
  for player 1) runs **labwc with wlroots' headless backend** (`WLR_BACKENDS=headless`, `WLR_HEADLESS_OUTPUTS=1`,
  `WLR_LIBINPUT_NO_DEVICES=1`, `WLR_RENDER_DRM_DEVICE=` the chosen card's render node). Inside it, `hearth-seats
  stream-run` sizes the output with `wlr-randr --output HEADLESS-1 --custom-mode 1920x1080@60Hz` (the player's
  preset), starts their Sunshine, then runs their Game Mode: gamescope nested at `-W/-H/-w/-h <size> -r <fps>`
  (`--prefer-vk-device` the chosen card).

  **Why not gamescope's own headless backend** (`gamescope --backend headless -W … -H … -r …`, which gamescope
  3.16 has: the output size comes from `-W/-H`, the refresh from `-r`)? Sunshine has no way to capture it: there's no
  KMS output (kms), gamescope offers no wlr-screencopy (wlr), there's no portal backend for it (portal), and Sunshine
  doesn't read gamescope's PipeWire stream (the `Video/Source` node Steam uses) directly. A headless wlroots output
  is what Sunshine's `wlr` capture is made for (Sunshine's docs mention capturing virtual/headless outputs), so the
  headless part is labwc's, with gamescope nested in it as everywhere else in multi-seat.
* **Sunshine, one per streamed player**, as that player's user service `hearth-seat-sunshine.service` (it runs
  `hearth-seats sunshine`, which refuses any account that isn't a streamed player). Its settings are written at
  every start to `~hearth-pN/.config/hearth/seat-stream/sunshine.conf`: `sunshine_name = <PC name> Player N`,
  `port = ` the player's fixed family, `capture = wlr`, `output_name = 0`, `origin_web_ui_allowed = pc`,
  `upnp = disabled`, `keyboard = disabled`, `mouse = disabled` (the virtual screen has no input devices; controllers
  only), its own `file_state`, `credentials_file`, `pkey`/`cert`, `file_apps` (one app: "Hearth") and `log_path`, and
  `adapter_name` = the render node. So each has its own pairings and certificate. Its web login comes from
  `/etc/hearth/seats/hearth-pN.stream.json` (random, made by `seats-setup`, owner player 1, group `hearth-pN`,
  mode 0640) and is set each start with `sunshine <conf> --creds`.
* **Ports.** Player N's family is `47989 + 1000 × (N-1)`: 48989, 49989, 50989. Sunshine uses port −5 to +21 of it
  (HTTPS −5, HTTP 0, web +1, video +9, control +10, audio +11, RTSP +21), so 48984-49010, 49984-50010 and
  50984-51010. Fixed, never from the request; never Remote play's 47984-48010. With firewalld running, `seats-setup`
  opens the ones Moonlight needs (TCP −5, 0, +21; UDP +9, +10, +11), runtime and permanent, records them, and
  `seats-remove` (or a failed set-up) closes them. The web page (+1) stays local (`origin_web_ui_allowed = pc`).
* **Pairing in Hearth** (player 1's Settings › Multi-seat): "Open Moonlight on the TV, phone, tablet or laptop and
  choose *<PC name> Player 2*" (Moonlight finds it through Avahi; the address `IP:port` is shown to add it by hand,
  and a QR code of it when `qrencode` is installed). Moonlight shows a 4-digit PIN; the player types it in Hearth.
  `hearth-seats stream-pair` calls that Sunshine's local API with its login: newer Sunshine lists the devices
  waiting (`GET /api/pin` → `pairings`) and wants `{"pairing_id", "pin", "name"}`; older Sunshine takes
  `{"pin", "name"}`. Paired devices are listed (`/api/clients/list`) and can be unpaired.
* **Controllers come from the client.** Sunshine makes virtual controllers: uhid devices (newer Sunshine,
  libvirtualhid: a hidraw node plus event nodes under `/devices/virtual/misc/uhid/…`) or uinput devices (older,
  inputtino: `/devices/virtual/input/inputN`). A virtual device doesn't say who made it, and udev's uaccess would
  give it to whoever is in front (player 1). So:
  * the streamed players' accounts get `rw` on `/dev/uinput` and `/dev/uhid` (74-hearth-seats-stream.rules, after
    73-seat-late.rules because udev's uaccess step resets a node's user ACLs; `uinput`/`uhid` load at boot);
  * 72-hearth-seats.rules runs `hearth-seat-input claim` (IMPORT) for every input/hidraw device under
    `/devices/virtual`. Each virtual device keeps its own open `/dev/uinput` or `/dev/uhid` file in the process that
    made it, so the streamed player whose process has such a file open that wasn't open last time made it (a
    "credit"; credits expire after 5 s). The device then gets that player's `ID_SEAT`/`HEARTH_SEAT_USER`, loses
    `uaccess`, and gets their ACL, like a real controller. One controller's nodes share one key (its uhid device or
    its `inputN`), so they all go to the same player. Two streamed players making one at the same instant: nobody
    gets it (`HEARTH_SEAT_CLAIM=nobody`, no ACL). Nobody streamed made it (player 1's Steam Input, Remote play): left
    alone. `forget` on remove;
  * the same goes for the virtual controllers the streamed player's own Steam makes (Steam Input works for them).
* **Sound** comes from the player's own PipeWire (their user session; Sunshine makes its virtual sink there).
* **Coexistence with Remote play** (Settings › Connections › Remote play, player 1's own Sunshine, `remote.rs`):
  different port families; the seat's Sunshine captures only its own virtual screen (`capture = wlr` on the
  labwc socket; it can't reach player 1's compositor); each extra player's session masks the packaged
  `sunshine.service`/`app-dev.lizardbyte.app.Sunshine.service` for itself (`--runtime`), so it never starts a
  second default-port Sunshine. `plan` warns if Remote play was moved onto a streamed player's ports.
* **Packages.** The editions (Anatase, Ubuntu, CachyOS, Peppermint) install Sunshine in their `build.sh`
  (LizardByte's COPR or the release RPM; the archive or LizardByte's .deb; CachyOS's repo or LizardByte's pacman
  repo) and `qrencode`, never failing the build. Without Sunshine (or labwc/wlr-randr/a render node) `plan` says
  `possible.stream: false` with the reason, and the UI doesn't offer it. The Flatpak Sunshine isn't used.

### Mixing

* Streamed players mix with anything (they don't use a screen here).
* `nested` and `split` mix (one host); player 1 is on the split screen when there is one.
* `gpu` doesn't mix with `nested`/`split` (a card per player vs. player 1's host), only with streamed players.
* With only streamed players, player 1's mode is `local`: Game Mode as always, no host.

The per-seat `mode` is in the layout and in `seats.json` (version 2). Version-1 set-ups (one top-level `mode`)
still work: a seat without `mode` takes the top-level one.

### Which one does `plan` choose?

* `gpu` when at least two cards each have a connected screen and the cards are different models. gamescope
  picks its card by `VID:PID`, so it can't tell two identical cards apart.
* Otherwise `nested` when at least two screens are connected in total.
* Otherwise `split` when one screen is connected (two players side by side, or one above the other on a squarer
  screen).
* Otherwise `stream` when streaming is possible, else none ("Connect a TV or monitor").

`possible` lists all four, and the set-up screen lets each extra player pick any possible way.

## Controllers and other devices

`/etc/udev/rules.d/72-hearth-seats.rules` sorts after `71-seat.rules`, which adds the seat tags, and before
`73-seat-late.rules`, which applies the `uaccess` ACLs. For every device it gives to player N it sets:

```
ENV{ID_SEAT}="seat-hearth-pN", TAG-="seat", TAG-="master-of-seat", TAG-="uaccess"
… GROUP:="root", MODE:="0660", RUN+="/usr/bin/setfacl -m u:hearth-pN:rw $devnode"
```

* `TAG-="uaccess"` means logind never gives the device to the active session (player 1). Logind only
  refreshes ACLs on devices tagged `uaccess`, so player N's ACL survives player 1's session changes.
* `TAG-="seat"`/`"master-of-seat"` keeps logind and SDDM from treating player N's card as a new seat. The
  made-up `ID_SEAT` also makes logind refuse `TakeDevice` for that card from `seat0` sessions, so player 1's
  gamescope or a KWin login screen can't grab it.
* `GROUP:=`/`MODE:=` override other packages' `0666` hidraw rules and the `input`/`audio`/`video` groups.

How a controller is identified (its "pad id"):

| id | matches | used for |
|---|---|---|
| `usb:1-2` (a USB port path) | `SUBSYSTEMS=="usb", KERNELS=="1-2", SUBSYSTEM=="input\|hidraw"` | wired controllers and dongles. Whatever is plugged into that port, or a hub on it, belongs to the player. |
| `uniq:aa:bb:cc:dd:ee:ff` (its serial or MAC) | `ATTRS{uniq}==` on input devices; `IMPORT{parent}="HID_UNIQ"` then `ENV{HID_UNIQ}==` on hidraw | Bluetooth controllers |

Controllers with neither (rare) always stay with player 1. Hearth's own controller reading (evdev, in
`guide.rs`/`gamemode.rs`) only opens nodes it may open, so each Hearth sees only its own player's controllers
with no code change.

**Moving a controller while playing** (`assign-pad` → `seats-acl`): rewrite the rules, `setfacl -b` the
controller's nodes, `udevadm trigger --action=change`, then *reconnect* it. An ACL only matters when a node is
opened, and Steam or a game may already have it open. A USB controller is reconnected by toggling its port's
`authorized`. A Bluetooth one gets synthetic remove/add uevents.

With a card per player, the card (`SUBSYSTEM=="drm", KERNELS=="<pci>"`) and its HDMI/DP sound
(`SUBSYSTEM=="sound"`, the same PCI slot, function .1) go to the player in the same way. Render nodes are
already 0666, so they aren't touched.

## Sound

* **A card per player:** player N's own PipeWire gets their card's HDMI sound through the ACL. Nothing else to do.
* **One card, several screens:** every screen's sound is on the same card. Two PipeWire servers would fight
  over it, and a player's server can't open part of a card. So players 2-4 play through **player 1's**
  PipeWire, each on the output of their own screen. host-run:
  * picks each player's sink: the layout's `audio`, or else the HDMI/DP sink whose port names the screen's
    monitor (PipeWire appends the ELD monitor name, which matches the EDID name scanned from
    `/sys/class/drm/*/edid`);
  * grants `hearth-pN` `x` on `$XDG_RUNTIME_DIR` and `$XDG_RUNTIME_DIR/pulse`, and `rw` on `pulse/native` and
    `pipewire-0`. These are revoked when the host ends;
  * writes `/run/hearth-seats/hearth-pN/audio.env` (`PULSE_SERVER`, `PIPEWIRE_REMOTE`, `PULSE_SINK`,
    `PIPEWIRE_NODE`), which the player's session loads;
  * sets player 1's default sink to player 1's screen.

  The card profile has to expose every HDMI output at once. Most HDMI profiles expose only one, so the user
  may need to choose one that has them all, or "Pro Audio", in Sound settings. This is untested.

## Accounts

`hearth-p2` to `hearth-p4` (the name must be `hearth-p<player number>`), created with `useradd --create-home
--user-group --shell /bin/bash --comment "Hearth Player N"`, home in `/var/home`. The password stays locked
(nobody signs in with one), and the accounts are hidden from SDDM (`HideUsers=`).

Supplementary groups: only `render` and `gamemode` (if they exist). **Not** `input`, `audio`, `video` or
`wheel`:
* `input`/`audio`/`video` would let a player open *every* controller, sound card or graphics card,
  which defeats the per-player ACLs. In nested mode, `audio` would also make their PipeWire grab the shared
  sound card.
* `wheel` would let the player run `hearth-admin` without a password (the polkit rule).

If player 1's own account is in `input`, `scan` warns: player 1's games could still see the other players'
controllers.

## Sessions

`hearth-seat@hearth-pN.service`: `User=%i`, `PAMName=hearth-seat`, `EnvironmentFile=/etc/hearth/seats/%i.env`,
`ConditionPathExists=` on that file, so only accounts multi-seat set up can start. `XDG_SESSION_TYPE=wayland`
and `XDG_SESSION_CLASS=user` reach `pam_systemd`. systemd passes the unit's environment into PAM before
`pam_open_session`, and there's no `XDG_SEAT`, so the session has no seat (logind counts sessions without a
seat as active).

The PAM service (`/etc/pam.d/hearth-seat`): `auth`/`password` are `pam_deny` (never used: systemd doesn't
authenticate service users), `account` is `pam_permit`, and `session` uses `pam_loginuid`, `pam_selinux`
(optional) and `system-auth` (which brings `pam_systemd`, limits and keyinit).

`hearth-seats session` (runs as the player) loads the env file, then:
* `gpu`: `LIBSEAT_BACKEND=noop`, no `WAYLAND_DISPLAY`/`DISPLAY`;
* `nested`, `split`: waits for the socket, sets `WAYLAND_DISPLAY` to it and loads `audio.env`;
* `stream`: writes the virtual screen's labwc config (`$XDG_RUNTIME_DIR/hearth-seat-stream/`) and execs headless
  labwc, whose autostart runs `hearth-seats stream-run` (size the output, start Sunshine, run Game Mode; stop
  Sunshine and close labwc when Game Mode ends).

It then sets `HEARTH_SEAT=N`, `NIMBUS_START_MODE=game` and `NIMBUS_GAMESCOPE_ARGS` (`--prefer-vk-device`/`--prefer-output`,
or `-W/-H/-w/-h`) and execs `/usr/bin/nimbus-session`. The player's Game Mode is the normal one.

## Never breaking player 1

* Player 1's devices, card and account are never changed: the rules only cover players 2-4's devices, and
  in `gpu` mode validation refuses to give player 1's card away.
* `seats-setup` validates everything before it changes anything. It then records every step (files with
  their earlier contents, accounts it created, units it enabled) and undoes them in reverse if a later step
  fails ("… Nothing was changed."). If a set-up already exists, it is replaced (accounts kept).
* The host only starts when `host-check` passes: set up, turned on, run by `main_user`, not an extra player,
  not paused, and at least two of the configured screens connected (a TV that's turned off just means normal
  Game Mode). If the host can't start (for example, labwc lacks security-context-v1), it returns exit code 3
  and `nimbus-session` runs normal Game Mode straight away. Two quick failures in a row pause multi-seat
  (`~/.local/state/hearth/seat-host.json`) until `hearth-seats start` is run again, so there's never a crash loop
  at sign-in.
* `seats-remove` works with or without `/etc/hearth/seats.json`. It disables every `hearth-seat@*` unit, deletes
  the files, reloads udev and hands the controllers back, removes players' leftover ACLs, and deletes
  `/run/hearth-seats`. Accounts (and their Steam/saves) are kept unless `--delete-users`, and even then
  only accounts multi-seat created are deleted. Nothing touches the boot configuration, so a bad set-up can't
  stop the PC from starting.

## Command line and JSON (for the Settings page and the Rust bridge)

All commands run as the signed-in user. hearth-seats calls `pkexec hearth-admin seats-*` itself; the existing
polkit rule covers it. Output is `progress: <0-100> <text>` lines (streamed live), then one JSON line. On a
problem: `error: <plain message>`, exit 1.

`hearth-seats scan`
```json
{"gpus": [{"id": "0000:03:00.0", "card": "card0", "pci": "0000:03:00.0", "vendor": "1002", "device": "73bf",
           "vk_device": "1002:73bf", "driver": "amdgpu", "boot_vga": true,
           "outputs": [{"id": "card0-HDMI-A-1", "connector": "HDMI-A-1", "connected": true, "enabled": true,
                        "mode": "3840x2160", "monitor": "LG TV", "internal": false}]}],
 "pads": [{"id": "usb:1-2", "name": "Xbox 360 Controller", "bus": "usb", "usb_port": "1-2", "uniq": null,
           "vendor": "045e", "product": "028e", "events": ["/dev/input/event5"], "hidraw": ["/dev/hidraw3"],
           "seat": 1, "assignable": true}],
 "audio": [{"card": "card1", "id": "HDMI", "pci": "0000:03:00.1", "gpu": "0000:03:00.0"}],
 "logind_seats": ["seat0"],
 "users": {"main": {"name": "bob", "uid": 1000}, "players": [{"name": "hearth-p2", "uid": 1001, "home": "/var/home/hearth-p2"}],
           "main_in_input_group": false},
 "config": null,
 "warnings": []}
```
(`config` is the saved set-up, the same as in `/etc/hearth/seats.json`, without the internal fields.)

`hearth-seats plan` (also `outputs`: the connected screens; `presets`: the stream sizes, frame rates and split
layouts; `performance`: the warning text)
```json
{"possible": {"gpu": false, "nested": true, "split": true, "stream": true}, "recommended": "nested",
 "reasons": ["There are 2 screens connected, so they can be shared out between players."],
 "warnings": [],
 "layout": {"mode": "nested", "seats": [
   {"seat": 1, "user": "bob", "gpu": "0000:03:00.0", "output": "card0-HDMI-A-1", "pads": ["usb:1-2"], "audio": null},
   {"seat": 2, "user": "hearth-p2", "gpu": "0000:03:00.0", "output": "card0-DP-1", "pads": ["uniq:aa:bb:cc:dd:ee:ff"], "audio": null}]}}
```

`hearth-seats setup --seats '<layout JSON>' [--start]`: the layout is `plan`'s `layout`, edited by the UI.
`gpu` is ignored: the root side works it out from the screen. `audio` is an optional PipeWire sink name. Each seat
has a `mode`; split seats have `region` (0-3, in the order of `seats_split_rects`) and the layout has
`"split": {"layout": "columns"|"rows"|"grid"}`; stream seats have `"stream": {"size": "1920x1080", "fps": 60}`
(sizes 1280x720, 1280x800, 1920x1080, 2560x1440, 3840x2160; 30/60/90/120 fps; optional `"gpu": "<PCI address>"`)
and no `output` or `pads`. `normalize_layout` makes player 1 and the top-level `mode` match before it's sent.
Example (TV split for two, a third player streamed):
```json
{"mode": "split", "split": {"layout": "columns"}, "seats": [
  {"seat": 1, "user": "bob", "mode": "split", "output": "card0-HDMI-A-1", "region": 0, "pads": ["usb:1-2"]},
  {"seat": 2, "user": "hearth-p2", "mode": "split", "output": "card0-HDMI-A-1", "region": 1, "pads": ["uniq:aa:bb:cc:dd:ee:ff"]},
  {"seat": 3, "user": "hearth-p3", "mode": "stream", "stream": {"size": "1920x1080", "fps": 60}}]}
```
The answer's seats then carry what root worked out: a split seat's `rect`, `size` (its part) and `screen_size`; a
stream seat's `stream.port`, `stream.name` and `stream.render`. `warnings` says if the firewall didn't open the ports.
```json
{"ok": true, "mode": "nested", "enabled": true, "created_users": ["hearth-p2"], "restart": false,
 "restart_game_mode": true,
 "seats": [{"seat": 1, "user": "bob", "output": "card0-HDMI-A-1", "card": "card0", "gpu": "0000:03:00.0",
            "vk_device": "1002:73bf", "size": "3840x2160", "pads": ["usb:1-2"], "audio": null, "audio_pci": []}, …]}
```
`restart` = a card per player: restart the PC to finish (logind only re-reads the card's tags at boot).
`restart_game_mode` = one card: player 1's Game Mode must restart to become the host (`nimbus-mode game`).

`hearth-seats start` / `stop`: `{"ok": true, "enabled": true|false, "mode": "nested", "restart_game_mode": true}`

`hearth-seats status`
```json
{"configured": true, "enabled": true, "mode": "nested",
 "seats": [{"seat": 1, "user": "bob", "output": "card0-HDMI-A-1", "gpu": "0000:03:00.0", "pads": ["usb:1-2"], "state": "active"},
           {"seat": 2, "user": "hearth-p2", "output": "card0-DP-1", "gpu": "0000:03:00.0", "pads": [], "state": "running"}],
 "host": {"running": true, "paused": false}}
```
`state`: `active` (player 1), `running`, `starting`, `waiting` (for player 1's Game Mode to open the screen),
`stopped`, `failed`. `host` is `null` with a card per player.

`hearth-seats status` also gives each seat's `mode` (and `region` / `stream`), the `split` settings and, with a
split screen, `audio` (`{"volumes": {"2": 75}, "only_p1": false}`).

`hearth-seats wait-button [--timeout S] [--exclude id,…]`: "Player N, press A": `{"pad": {…scan's pad…}}` or
`{"pad": null}` after the timeout. Only controllers player 1's account may open are listened to.

`hearth-seats stream-info`: per streamed player `{"seat", "user", "name", "port", "size", "fps", "running",
"reachable", "clients": [{"name", "uuid"}], "pending": [{"id", "name"}], "addresses": ["192.168.1.20:48989"], "qr"}`.

`hearth-seats stream-pair --seat N --pin 1234 [--name NAME]` → `{"ok": true}`; `stream-unpair --seat N --uuid U`.

`hearth-seats audio [--seat N --volume 0-100] [--only-p1 on|off]` → the split screen's sound settings.

Bridge: `seats.waitButton` (event `seats.pad.done`), `seats.streamInfo`, `seats.pair` (event `seats.pair.done`:
Sunshine answers when Moonlight finishes), `seats.unpair`, `seats.audio`.

`hearth-seats assign-pad --pad usb:1-2 --seat 2`:
`{"ok": true, "pads": {"usb:1-2": 2, "uniq:…": 1}, "changed": ["usb:1-2"]}`

`hearth-seats remove [--delete-users]`:
`{"ok": true, "removed_users": [], "kept_users": ["hearth-p2"], "restart": false}`

Root side (stdin JSON, same messages): `hearth-admin seats-setup` (`{"mode", "seats", "split", "start"}`; strict:
presets only, ports and names worked out by root, nothing free-form reaches a file or command; every step journaled
and undone on failure, firewall ports included),
`seats-remove [--delete-users]`, `seats-acl` (`{"pads": {"<pad id>": <player>}}`), `seats-start`, `seats-stop`.

## Integration still needed (outside this feature's files)

### nimbus-session (`hosts/linux/session/nimbus-session`)

In `run_game`, replace the final `gamescope …` line with:

```sh
  SEATS=/usr/libexec/nimbus/hearth-seats
  seatargs=""
  full="-f"   # an extra player on part of a split screen is a window the size of its part
  [ -n "$HEARTH_SEAT" ] && [ -n "$HEARTH_SEAT_WINDOWED" ] && full=""
  if [ -z "$HEARTH_SEAT" ] && [ -f "$SEATS" ]; then
    # Multi-seat with one card and several screens: share the screens out (falls back to normal by itself).
    if python3 "$SEATS" host-check 2>/dev/null; then
      log "multi-seat: hosting the other players' screens"
      # shellcheck disable=SC2086
      python3 "$SEATS" host -- gamescope -f $extra $NIMBUS_GAMESCOPE_ARGS -- "$WATCHDOG" "$NIMBUS_APP"
      rc=$?
      [ "$rc" -ne 3 ] && return "$rc"
      log "multi-seat host couldn't start; normal Game Mode"
    fi
    # Multi-seat with a card per player: stay on player 1's card.
    seatargs="$(python3 "$SEATS" gamescope-args 2>/dev/null)"
  fi
  # shellcheck disable=SC2086
  gamescope $full $extra $NIMBUS_GAMESCOPE_ARGS $seatargs -- "$WATCHDOG" "$NIMBUS_APP"
```
(This is in `nimbus-session` now.)

The extra players (`HEARTH_SEAT` set) must leave system-wide things to player 1:

```sh
# at the top of reset_controllers():
  [ -n "$HEARTH_SEAT" ] && return 0     # InputPlumber is shared: only player 1 resets it
# guard these three with [ -z "$HEARTH_SEAT" ] &&
  systemctl --user start steamos-manager.service …        (in run_game)
  the hearth-controller watcher block                     (if [ -z "$HEARTH_SEAT" ] && [ -x … ])
  the hearth-power restore line
# in the loop, before "case $next": extra players have Game Mode only
  [ -n "$HEARTH_SEAT" ] && [ "$next" = desktop ] && next=game
```

### Rust app (hearth-core)

* `inputplumber.rs`: do nothing when `HEARTH_SEAT` is set. Its intercept mode is system-wide, so player 2's
  Quick Access would otherwise catch player 1's built-in controller.
* `power.rs` (TDP) and anything that calls `hearth-admin`: hide them when `HEARTH_SEAT` is set (the extra
  accounts aren't in wheel, so pkexec would fail anyway).

### build.sh

Nothing is required: `cp -avf /ctx/system_files/. /` installs the helper (mode 755 from git), the units, and
the PAM file, and no unit is enabled by default. Optionally, as a safeguard:
```sh
chmod 755 /usr/libexec/nimbus/hearth-seats
install -Dm644 /ctx/../docs/MULTI-SEAT.md /usr/share/doc/hearth/MULTI-SEAT.md   # if docs are in the build context
```

## Limitations and what needs testing on real hardware

### Split screen and streamed players (Beta Experimental)

* **labwc regions.** The split parts need labwc's `<regions>` and the `SnapToRegion` action in window rules (labwc
  0.7+). If a labwc ignores them, split players' windows land at their own size wherever labwc places them.
* **Split screen focus and keyboard.** Controllers go to each player by ACL, so focus doesn't matter for them; a
  keyboard goes to the focused window (player 1's).
* **Split sound** relies on PipeWire's `module-remap-sink` (pipewire-pulse) and games honouring `PULSE_SINK`.
* **Streamed players: controllers only.** Sunshine's keyboard and mouse are off (the virtual screen has no input
  devices). Touch on phones isn't passed on either.
* **Whose virtual controller.** hearth-seat-input attributes by "who just opened /dev/uinput or /dev/uhid". If player 1's
  Steam makes a virtual controller in the same instant as a streamed player's Sunshine makes one, they can swap (rare).
  Streamed players' accounts can open /dev/uinput, so a program they run could make an input device (a keyboard)
  that player 1's session would then see (fine for a household; the same as Steam needs).
* **Sunshine binary capabilities.** Some packages give `sunshine` `cap_sys_admin` (for KMS capture). The seats'
  Sunshine is told `capture = wlr`, but a capable binary could capture any screen if its settings were changed.
* **Encoder sessions.** Each streamed player is one video encode; consumer Nvidia cards limit concurrent NVENC
  sessions, and 4K60 per player is heavy. Players share the PC's power limit: games slow down for everyone.
* **Sunshine's API** changed over versions (pairing with `pairing_id` in newer ones); both are handled, but only
  what's in the code was checked against Sunshine's source, not on a device.

What to test: (5) split on one 4K TV: columns, rows, grid with 3 and 4 players; each window in its part, no
decorations, gamescope rendering at the part's size; each player's controllers only in their part (`getfacl`);
per-player volume and "only player 1"; removing a player and setting up again; Remove multi-seat. Then split + a
nested second screen. (6) Streamed: 1 player to a Moonlight TV/stick (Android TV, Fire TV), a phone and a laptop at
720p60/1080p60/4K60; pairing with the PIN from Hearth on a newer and an older Sunshine; the controller in Moonlight
working in that player's Hearth and Steam (and not in player 1's: `getfacl /dev/input/eventN /dev/hidrawN` of the
"Wireless Controller"/virtual pad); sound on the device; 2-3 streamed players at once (each pairing separately,
controllers not crossing); Remote play turned on at the same time (both reachable, no port clash); a streamed
player plus split screen on one PC; firewalld on and off; Remove multi-seat closes the ports. Hardware: an AMD
iGPU laptop (VAAPI), an AMD dGPU, an Nvidia card (NVENC session limits).

### The original modes

* **Steam Input's virtual controllers for players 2-4.** Steam makes them through `/dev/uinput`, which only
  the active session (player 1) may open. Even if the extra players could open it, the virtual devices appear
  under `/devices/virtual` with `uaccess`, so they would go to player 1. Players 2-4's games use their
  controllers directly (SDL/hidraw/evdev), which works for most games, but Steam Input remapping isn't
  available to them.
* **Controllers InputPlumber manages** (a handheld's built-in controller) stay with player 1. InputPlumber's
  virtual device is the one games see.
* **Several controllers through one wireless dongle** (an Xbox Wireless Adapter) share a USB port, so they all go
  to one player. Use Bluetooth or one dongle per player.
* **Two identical graphics cards** can't be told apart by gamescope's `--prefer-vk-device`, so `plan` offers
  `nested` instead.
* **labwc version.** The window rules need labwc's `sandboxEngine`/`sandboxAppId` rule options and security-context-v1
  (labwc 0.8+). If they're missing, `host-run` fails and player 1 falls back to normal Game Mode. If the rules
  are ignored, players' windows can land on the wrong screen. The fallback would be
  gamescope `--backend sdl` with `SDL_APP_ID=hearth-seat-N` and `identifier=` rules (the approach `hearth-coop`
  takes).
* **Switching player 1 to Desktop Mode ends the other players' sessions** (their screens belong to player 1's
  Game Mode host). They come back when player 1 returns to Game Mode, but their running games are lost.
* **Audio in nested mode** depends on the card profile exposing every HDMI output, and on games honouring
  `PULSE_SINK`/`PIPEWIRE_NODE`. Players 2-4 can technically reach player 1's sound server, including its
  microphone, while the host runs (fine for a household, worth knowing).
* **Games are downloaded per account.** Each player's Steam has its own library folder. Sharing a library
  between accounts is a separate feature.
* **Setting it up or removing it with a card per player** needs a restart for logind to see the card's new
  tags. Controllers move live in both modes.

Hardware checklist: (1) nested: two screens on one AMD card, then Nvidia; check window placement, input
isolation (`getfacl /dev/input/eventN /dev/hidrawN`), sound on both TVs, and fallback with the second TV off.
(2) gpu: iGPU + dGPU and two dGPUs; check that SDDM shows no second login screen, that player 1 stays on its
card, and that player 2's gamescope gets DRM master. (3) `remove` from each mode, then a reboot, should be
normal single-player Hearth. (4) A Bluetooth controller moving between players while in a game.
