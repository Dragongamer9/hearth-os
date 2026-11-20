# Hearth Console Remote Play

A Hearth Store app that plays the owner's own Xbox at home on his ROG Ally (Game Mode) and his laptop, over the
internet, with Xbox Remote Play, the way the open-source Greenlight app does it. Approved mockups:
`docs/mockups/console-remote-play/` (18 pages); every part is listed in `docs/mockups/checklists/console-remote-play.md`
(red flag 11). It has no version number yet: the owner picks it.

## What it's built on: Greenlight's findings (MIT)

Greenlight (github.com/unknownskl/greenlight) is MIT licensed, and so are the two libraries it wraps, both by the same
author: **xal-node** (Xbox sign-in, MIT) and **xbox-xcloud-player** (the WebRTC stream, MIT). Studied from their npm
packages (xal-node 1.1.5, xbox-xcloud-player 0.2.11; the GitHub pages can't be read from here). Nothing is copied: the
app is written anew in Rust from what they show, and their copyright and MIT notice go in the app's About and
`LICENSES/greenlight-MIT.txt` with the AppImage, as MIT asks when code is ported closely (the input packet layout is).

**It's unofficial.** Microsoft publishes no API for this. Greenlight talks to the same private services as the
Xbox app and **presents itself as Microsoft's own Xbox app** (its sign-in client id `000000004c20a908`, an "Android"
device token, and `Microsoft.GamingApp` in the stream's device info). Microsoft could change or block this at any
time, and it may go against Microsoft's terms of use (an account could in the worst case be limited). This is the README's open question 1, still
the owner's to answer before the app goes in the Store.

### Sign-in (xal-node's way)

1. Make a P-256 key pair (kept with the sign-in in the keyring) and ask for a **device token**: POST
   `device.auth.xboxlive.com/device/authenticate` (ProofOfPossession; every Xbox Live request is signed with the key:
   SHA-256 over version, Windows time, method, path, auth header and body, ECDSA P-256, `Signature` header).
2. **SISU authenticate**: POST `sisu.xboxlive.com/authenticate` with the device token, PKCE (S256) and a random state;
   the answer has `MsaOauthRedirect`, the address of **Microsoft's own sign-in page** (login.live.com).
3. That page opens in a window inside the app (WebKitGTK). The person signs in there; the app never sees what's typed.
   The page ends by going to `ms-xal-000000004c20a908://auth?code=…&state=…`; the window catches that address, never
   loads it, and closes by itself.
4. The code is traded for the **user token** (POST `login.live.com/oauth20_token.srf`, PKCE verifier), then **SISU
   authorize** (POST `sisu.xboxlive.com/authorize`) gives the user, title and device tokens and the gamertag.
5. **XSTS** for `http://gssv.xboxlive.com/` (streaming) and `http://xboxlive.com` (console list and wake), then the
   **streaming token**: POST `xhome.gssv-play-prod.xboxlive.com/v2/login/user` (`offeringId: xhome`; Cloud uses
   `xgpuweb`, falling back to `xgpuwebf2p`). Its answer names the region's `baseUri` for the stream.

Kept: only the refresh token and the key, as one item in the system keyring (Secret Service, `secret-tool`, the
secret on its stdin, attributes `application=hearth-console-remote-play`, `account=xbox`). Everything else is asked
again from the refresh token when needed and held only in memory. With no keyring, sign-in says so and keeps nothing.
Sign out deletes the keyring item. No token, code or password is ever written to a file or a log (a test checks the
app's own log lines for anything token-like).

### My consoles and Wake

- The list: GET `{baseUri}/v6/servers/home` (Bearer streaming token) for the consoles that can stream, joined with
  GET `xccs.xboxlive.com/lists/devices?queryCurrentDevice=false&includeStorageDevices=true` (XBL3.0 token,
  contract 4) for the name, model and power state (On / ConnectedStandby = Sleeping / Off). Asked only when the page
  opens or on Refresh: no polling.
- Wake: POST `xccs.xboxlive.com/commands` `{destination: Xbox, type: Power, command: WakeUp, linkedXboxId}`; then
  the state is asked again after 5, 10 and 15 s (three asks, then it stops), the "about 15 s" in the mockup.

### The stream (xbox-xcloud-player's way)

1. POST `{baseUri}/v5/sessions/home/play` (`serverId` = the console, `nanoVersion: V3;WebrtcTransport.dll`, the
   device info header), then GET `…/{sessionId}/state` every 2 s while it says `Provisioning` (the "Connecting" step,
   with a 90 s limit), and `…/configuration`.
2. WebRTC offer: one audio transceiver (send/receive), one video (receive only), and four data channels: `input`
   (ordered, protocol `1.0`), `control` (`controlV1`), `message` (`messageV1`), `chat` (`chatV1`). The offer goes to
   POST `…/sdp` with the channel versions (control 1–3, input 1–8, message 1, chat 1); the answer is read from GET
   `…/sdp` (204 = ask again in 1 s, at most 30 times). ICE candidates go to POST `…/ice` and come back from GET.
3. `message` opens with a `Handshake` (`messageV1`); on `HandshakeAck` the app sends the system UI list, the screen
   size (1920x1080, or 1280x720 at 720p), orientation and touch off. `control` sends `authorizationRequest` and
   `gamepadChanged` (index 0, added) and asks for a key frame when the picture breaks up.
4. `input`: first a client-metadata packet, then a packet each time the controller changes (at most every 4 ms, none
   while nothing changes): a 14-byte header (report type, sequence, time) and per gamepad 23 bytes (index, button
   mask, both sticks as int16, both triggers as uint16). Vibration comes back on the same channel.
5. Keepalive: POST `…/{sessionId}/keepalive` every 60 s while streaming (the only timer, and only while streaming).
   Disconnect: DELETE `…/{sessionId}`; the Xbox stays on and sleeps by itself later.

## The stack: pure Rust (hearth-ui) with GStreamer for the picture and sound

Two ways were weighed:

| | Pure-Rust hearth-ui app, GStreamer's webrtcbin + VA-API | WebKit/Chromium app with the browser's WebRTC (Greenlight's way) |
|---|---|---|
| Picture decoding | the graphics card (VA-API: `vah264dec`, AMD and Intel; NVDEC on Nvidia), CPU decoder only as a fallback | Chromium on Linux decodes WebRTC video on the CPU by default (its VA-API path is behind flags for WebRTC); WebKitGTK's WebRTC is GStreamer underneath, but the page and JS sit on top |
| Delay | the jitter buffer kept at its smallest (`latency=0`, drop late frames), frames shown the moment they're decoded | the browser's own buffering and compositor, about a frame more |
| At rest (every page but the stream) | an iced window: nothing runs, 0 % (CI measures it) | a browser engine with its timers and JS: Hearth's WebKit apps needed fixes to get under 0.3 % |
| Memory | ~40–60 MB idle | 200–400 MB (Electron) |
| Fits Hearth | the same toolkit, look, settings_file.rs and controller handling as the other Fedora apps | a second toolkit to style like the mockups |

**Chosen: the pure-Rust hearth-ui app.** Every page is iced (hearth-ui's look, settings_file.rs, the same title bar
and controller hints as the other Fedora apps). The stream uses **GStreamer's webrtcbin** through gstreamer-rs: it
does ICE, DTLS-SRTP, the SCTP data channels, the jitter buffer and NACK, it's in the Fedora image already
(gstreamer1-plugins-bad-free; RPM Fusion's freeworld VA drivers for H.264 on AMD), and it's what WebKitGTK's own
WebRTC uses. webrtc-rs (pure Rust) was set aside: it has no jitter buffer or decoder of its own, so the delay would
depend on code written here, and it's less proven against Xbox's answers than a full WebRTC stack.

The picture: webrtcbin → `rtph264depay` → `vah264dec` (falls back to `avdec_h264` / `openh264dec`) → `vapostproc`
(the graphics card scales it to the window and makes RGBA) → `appsink` (one frame kept, late ones dropped) → an iced
image the size of the window, drawn as it arrives. Sound: `opusdec` → `pipewiresink` (the volume in the bar is the
stream's own). Nothing of GStreamer is started until Stream is pressed, and it all stops at Disconnect.

The controller: on the Ally and laptop, Hearth's own controller reading (hearth-pad, evdev) through iced's keyboard
in the pages, and the stream reads the controller itself (evdev, the first gamepad, or the one chosen in Settings).
**Hold View + Menu for 1 s** opens the app's menu (keyboard Ctrl+Shift+M); while it's open the Xbox gets no buttons.

The sign-in window is a small GTK window (`hearth-console-remote-play-signin`, Python and WebKitGTK, both in the
image) opened over the dimmed app: Hearth draws its frame (lock, title, "Microsoft's own page", the read-only
address, the steps, the token note, Reload page, Cancel) and the middle is Microsoft's real page. In Game Mode
Hearth's on-screen keyboard comes up for its boxes, as on every other text box.

Where it lives: `hosts/console-remote-play` (its own Cargo workspace using hearth-ui by path, so GStreamer's libraries
are needed only here and the OS image's build is untouched), built and pictured by `.github/workflows/console-remote-play.yml`.
The Store installs it as an AppImage from the app's **own GitHub repository's releases**
(`os-bases/anatase/system_files/usr/share/hearth/store/catalog.json`, kind Install, `github: owner/repo`): **that
repository has to be made by the owner** (for example Dragongamer9/hearth-console-remote-play). Until then it isn't
in the Store's list, and nothing is released.

## Data

Counted on every network (home, work Wi-Fi, hotspot): the stream's bytes received and sent, added to
`~/.local/share/hearth/console-remote-play/data.json` (the month, bytes; written through settings_file.rs once a
minute while streaming and at the end, never at rest). The limit (default 20 GB), the warning (90 %) and the day of
the month it starts again are settings. At the warning level one toast, once a month; at the limit it asks, and never
stops a game by itself. Data saver: Off / On a hotspot (NetworkManager's metered flag, asked once at Stream) / Always.

## Settings

`~/.config/nimbus/console-remote-play.json` through settings_file.rs (atomic, .bak, merge-on-write, never defaults
over a file that didn't read). Opening the app or a page writes nothing; CI checks.

## What CI checks (it can't reach a real Xbox)

- `cargo test`: the request signing against a recorded signature layout, SISU / token / console list / play / state /
  sdp / ice parsing against recorded answers in `tests/fixtures` (made up, with no real tokens), the input packet bytes
  against xbox-xcloud-player's layout, the data counter and month rollover, the warning once, settings merge.
- A stand-in Xbox service (`HEARTH_CRP_FIXTURE`): the app reads its answers from fixture files instead of Microsoft,
  so every page is pictured with real-looking content.
- Every page pictured next to its mockup (1920x1080 for the desktop, 1280x800 for the Ally), CPU at rest under 0.3 %.
- On a device only: the real sign-in, Wake, the stream's picture, sound, delay and controller ("needs testing on a
  device" in the checklist).

## Where it stands (Nov 20, 2026)

- Built: every page of the 18 mockups (desktop and the Ally's Game Mode), the sign-in window, the sign-in protocol,
  the console list, Wake, Cloud's library, the stream's session and WebRTC through webrtcbin, the controller packets,
  View + Menu's menu, rumble, the monthly data, Data saver, screenshots, the settings.
- CI (console-remote-play.yml, run 44767047864): cargo test (19 + 3 tests), every page pictured and checked against its
  mockup, CPU at rest 0.00 % on every measured page, 32–45 MB, no settings file written by opening pages, the whole
  sign-in (stand-in page → caught redirect → tokens → keyring) passes, no token in any log or file.
- The Ally's trouble page is pictured both drawn by the CPU (tiny-skia, as on PCs without a graphics card) and by the
  graphics card; both match the mockup (run 44767047864). Its second line is a layer of its own across the page: laid
  out next to the title, iced measured it to the title's width and the CPU renderer clipped it there.
- Needs a device: a real sign-in, Wake, the picture, sound, delay, the controller and rumble with his Xbox.
- Needs the owner: the app's own GitHub repository (for the Store's AppImage releases), his okay on the unofficial
  sign-in, and its version number. Nothing is released and it isn't in the Store's list yet.
