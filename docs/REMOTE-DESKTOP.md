# Use this PC from anywhere (remote desktop)

Settings › Remote play › **Use this PC from anywhere**: this PC's screen on your work computer, laptop or phone with the
free **Moonlight** app, through **Tailscale**. It uses Sunshine, which comes with Hearth OS, with settings tuned for
low latency. Off until you turn it on; also linked from Home server › Gaming and Services.

## From work: what to do

1. **At home, once:** Settings › Remote play › turn on **Use this PC from anywhere**. Make sure Tailscale is signed in
   (Settings › Home server › Tailscale). The card shows the line to type in Moonlight, like
   "In Moonlight, add chris-hearth or 100.72.99.97".
2. **Install Moonlight** on the other device: moonlight-stream.org for Windows, Mac and Linux; the App Store, Google Play
   or the Fire TV store for phones, tablets and TVs.
3. **Sign in to Tailscale** on that device (tailscale.com/download) with the same account as the PC.
4. **Add the PC in Moonlight:** Add PC (the + button) and type the name or the address from the card.
5. **Enter the PIN:** Moonlight shows 4 numbers. On the PC, Settings › Remote play › **Type Moonlight's PIN**, type them
   and a name (like "Work laptop"). You only do this once per device. (Pair at home before you need it from work.)
6. **Pick Desktop.** That's the whole screen. Moonlight's menu: Ctrl+Alt+Shift+Q quits, Ctrl+Alt+Shift+Z frees the
   mouse. In Moonlight's settings, turn on **Optimize mouse for remote desktop instead of games** for desktop work.

**Work computers:** many don't allow installing apps (Moonlight, Tailscale) or remote access, by policy. Ask your IT
department before you do; a phone or your own laptop on the work Wi-Fi is often the easier way.

## How fast it is

The card shows who is connected, the resolution and frame rate, the bitrate, and how the device reaches the PC:

- **Direct**: the best case. Try 1080p at 60 fps and 20 Mbps (1440p and 30 to 40 Mbps if it stays smooth).
- **Through a relay** (Tailscale's DERP servers): "Your work network is blocking direct connections; it will feel
  laggier". Use 720p at 60 fps and 5 Mbps, or 1080p at 30 fps. **Check the connection** pings the device once through
  Tailscale and says direct or relay and how many milliseconds.
- When nobody is streaming, the card lists your online Tailscale devices and how each one reaches the PC right now.

How it's worked out: `tailscale status --json` for the device that is sending this PC the most (a stream is megabytes a
second): `CurAddr` set means direct, otherwise relay (`Relay` names the DERP region). Streaming or not comes from
Sunshine's log (`CLIENT CONNECTED` / `CLIENT DISCONNECTED`); the device's name, resolution and frame rate from Sunshine's
prep command (its `SUNSHINE_CLIENT_*` values), kept only in your runtime folder.

## What it does to the PC

- **Sunshine with Hearth's settings** (merged into `~/.config/sunshine/sunshine.conf`, with a `.bak`; what was there
  before is remembered and put back when you turn it off, unless you changed it since): `encoder = nvenc` and
  `nvenc_preset = 1` (the fastest) on Nvidia, `encoder = vaapi` on AMD and Intel, `upnp = disabled` (nothing opened to
  the internet), `origin_web_ui_allowed = pc` (Sunshine's own page only on this PC), `stream_audio = enabled`, and a
  `global_prep_cmd` entry (your own prep commands stay). The **Desktop** app is added to `apps.json` if it's missing.
- **Only through Tailscale** (or "Tailscale and home network" if you choose; never the internet): Hearth's own nftables
  table `hearth_remote_desktop` (`/etc/hearth/remote-desktop.nft`) drops Sunshine's ports on every interface except
  `lo` and `tailscale0`: TCP 47984, 47989, 47990, 48010 and UDP 47998, 47999, 48000, 48002, 48010 (from Sunshine's
  `port`, 47989 unless you changed it). `hearth-remote-desktop-firewall.service` loads it again at boot (enabled only
  while this is on; it isn't ordered before anything and can't fail the boot). Sunshine's drop-in waits for the rule's
  mark (`/run/hearth/remote-desktop-fw`): **no rule, no Sunshine**.
- **Starts with your session** (a link in `~/.config/systemd/user/default.target.wants`), in Desktop Mode and Game Mode.
- **Stay awake while connected** (on by default when the feature is on): a logind block inhibitor (idle and sleep,
  which Mantel's own idle timer respects) while a device streams, let go 2 minutes after it disconnects.
- **Wake from away**: an asleep PC can't answer Moonlight. Another device at home on Tailscale can wake it
  (Home server › Alerts & power › Wake from the network).
- **Sunshine's login for Hearth** is made on this PC the first time (random, 0600 in `~/.local/share/nimbus/remote-play.json`,
  the same one Settings › Remote play uses), used only on 127.0.0.1 and never shown or logged. The PIN goes to
  Sunshine's local API (`/api/pin`; newer Sunshine also wants the waiting device's `pairing_id`, which Hearth reads from
  `GET /api/pin`).
- Your choices (`on`, `reach`, `keepAwake`) are in `~/.config/nimbus/remote-desktop.json`, written only when you change
  one (hearth_settings_io). No password or runtime state is ever in it.

**Turning it off** undoes all of that: the link, the drop-in, Hearth's Sunshine values, the Desktop entry it added, the
firewall rule and the boot unit. Sunshine keeps running if Settings › Remote play's own switch has it on.

## Game Mode

Sunshine streams whatever is on the screen (it captures through KMS). In **Desktop Mode** that's your desktop. If you
switch to **Game Mode** or start a game while someone is connected, the stream goes on and shows Steam's Game Mode (and
the game); Moonlight may need to reconnect while the screen changes over. Nothing stops Game Mode: Sunshine is a separate
user service, and if it can't start, Game Mode and your session start as always. Game Mode's Quick Access (Hearth Link ›
Home server) shows "<device> is using this PC through Moonlight" while a device is connected.

## Files

- `hosts/linux/desktop/shell/hearth-remote-desktop`: the helper (status, on/off, reach, PIN, unpair, check, Sunshine's
  prep command, the stay-awake loop).
- `hosts/linux/desktop/shell/SetRemote.qml`: the card.
- `os/system_files/usr/libexec/nimbus/hearth-home-server-helper`: `remote-fw tailscale [PORT] | off | load` (root).
- `os/system_files/usr/lib/systemd/system/hearth-remote-desktop-firewall.service`: the boot unit (off in the image).
- Tests: `hosts/linux/test/test_hearth_remote_desktop.py`.

## Remote desktop with a VNC app

Settings › System › **Remote desktop** is the other way in: this PC's desktop in any **VNC** app that does VeNCrypt
(TLS): TigerVNC Viewer (Windows, Mac, Linux), Remmina and KRDC (Linux), bVNC / AVNC (Android). It's slower than
Moonlight for games, but it needs nothing on the PC but the switch, works on every desktop Hearth has (KWin and labwc),
and any VNC app can use it. Remote play above stays for games.

1. Turn on **Remote desktop** and **set a password** (at least 8 characters). It's kept in your keyring, never in a file.
2. On the other device, sign in to Tailscale and connect your VNC app to the name shown (like `gaming-pc.tail1234.ts.net`).
   Pick VeNCrypt / TLS if it asks. If it asks whether to trust this PC, check the fingerprint Settings shows.
3. The first time, this PC asks **at the PC** to allow sharing the screen (the desktop's own question). On KDE's KWin
   that answer is remembered; on the classic labwc desktop it may ask each time someone connects.

**Who can connect:** Tailscale addresses only, or your home network too when you choose it; never the internet (other
addresses are dropped before anything is sent). Every connection is a notification and a dot under the bar with
**Disconnect**. Five wrong passwords lock that address out for five minutes. **View only** lets them watch without typing
or clicking.

**Keyboard and mouse:** on KWin through the desktop's own remote control (the RemoteDesktop portal); on labwc, whose
portal can only share the screen, through a virtual keyboard and mouse (uinput, which Hearth lets the person at the PC
use; US layout); if neither works it's view only and Settings says why. The clipboard isn't shared.

**How it works** (hearth-vnc, `hearth-vnc --help`): RFB 3.8 with VeNCrypt only: X509Plain (this PC's own certificate,
made on first use in `~/.local/share/nimbus/vnc/`) for everyone, and TLSPlain (anonymous TLS) only for Tailscale
addresses, whose WireGuard tunnel already proves who they are. The screen comes from the screen-sharing portal through
GStreamer (pipewiresrc), only while someone is connected, up to 20 frames a second, and only the 64x64 tiles that
changed are sent (ZRLE, or Raw). Choices in `~/.config/nimbus/remote-vnc.json` (on, reach, viewOnly), written only when
you change one. `hearth-vnc.service` is a user service that isn't enabled: the switch starts it, and Desktop Mode
starts it again at sign-in only while the switch is on.

## Recording the screen (the Recorder panel)

The Recorder panel (`quickshell ipc ... call record open`, the dot under the bar while recording, and Super+Alt+R where
the Hearth Recorder app can't record) records a screen or a window through the same screen-sharing portal, so it
works on KWin and labwc alike: this PC's sound, the microphone or both, 30 or 60 frames a second, the pointer or not.
It encodes on the graphics card (NVENC, VA-API) when GStreamer can, else x264 (a graphics-card encoder that fails in
the first seconds is replaced by itself), into an MP4 written in fragments, so a power cut doesn't lose the recording.
Recordings go to Videos/Recordings with a notification. Its helper is hearth-record; your choices are in
`~/.config/nimbus/record.json`, written only when you change one.
