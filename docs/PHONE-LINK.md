# Phone link

Your phone linked to your Hearth PC, the way KDE Connect does it on KDE Plasma, but built into Hearth: install the
free **KDE Connect** app on your Android phone (Play Store or F-Droid) or iPhone (App Store), keep both on the same
Wi-Fi, and pair them. Everything below then works in Desktop Mode (Mantel, on labwc or KWin) and, for most of it, in
Game Mode too.

It's Hearth's own code, not KDE's apps: the service is **Hearth Connect** (`hosts/hearth-connect`, the
`hearth-connect.service` user service), which speaks KDE Connect's protocol itself. Settings › Bluetooth & devices ›
**Phone** is its page in Mantel, Quick Access has a **Phone** tile, and the Hearth Connect window still works as before.

## Turning it on

Settings › Phone › **Phone link** (or tap the Phone tile in Quick Access). Off until you turn it on, so nothing listens
on the network before you ask. While it's on, the firewall lets phones on your own network in (ports 1714–1764, only
from local addresses, through `hearth-link-firewall`) and closes again when it's off. Your choice is remembered
("enabled" in `~/.config/hearth/connect/settings.json`) and never changes by itself.

## Pairing

1. On the phone, open KDE Connect: your PC shows up by its name. Tap it, then **Request pairing**.
2. On the PC, Settings › Phone shows "**Pixel 8 wants to pair**" with an 8-character key. Check the phone shows the
   same key, then **Accept**. (Or start it from the PC: pick the phone under **Pair a new phone**.)

Nothing but pairing happens with a phone until you accept. A paired phone must show the same certificate every time;
a phone that pretends to be it is refused. Phone not listed? Type its address under Pair a new phone (guest Wi-Fi
networks often keep devices apart).

## What it does

| Feature | From the phone | From the PC |
| --- | --- | --- |
| Notifications | Your phone's notifications pop up in Mantel, with the app's own buttons and a **Reply** box for messages. Dismissing one here dismisses it on the phone. | Settings › Phone lists them, with Reply, buttons and Dismiss |
| Messages (SMS, Android) | | Settings › Phone › **Messages**: every conversation with names from your contacts, read one, reply, or start a new text |
| Calls | "Ada is calling" pops up with **Mute ringer**; what's playing here pauses and plays again when the call ends (only what Hearth paused) | |
| Clipboard | Copy on the phone, paste on the PC | And the other way round (Send clipboard too) |
| Files, links, text | Share to "Hearth PC": saved in Downloads › Hearth Connect after you accept (or without asking, if you choose) | **Send files…** (Hearth Files' picker) |
| Browse the phone's files | | **Browse its files**: the phone's storage opens in Hearth Files |
| Take a photo | | **Take a photo**: the phone's camera opens, the picture comes to Downloads |
| Ring / ping | Find my PC: it rings | **Ring** (find my phone) and **Ping** |
| Media | Control the PC's music and video players (and the volume) | Control what plays on the phone |
| Touchpad and keyboard | The phone moves the pointer, clicks, scrolls and types | |
| Presentation remote | Next and previous slide, and the phone as a pointer | |
| Run commands | Start the commands you added (Lock the screen, Start Steam...) | Add or remove them in Settings › Phone › Run commands |
| Battery and signal | The phone's battery and signal show in Settings › Phone and on the Quick Access tile | The PC's battery (laptops) shows on the phone |

Each phone has its own switches (What *phone* can do): show its notifications, share the clipboard, touchpad /
keyboard / presentation remote, pause media during calls, run commands, save its files without asking.

## For other parts of Hearth

- **Starting it at sign-in.** The service has no `[Install]`: Mantel's autostart starts it, and only when Phone link
  is on. The line (Mantel's autostart, `hosts/linux/desktop/labwc/autostart` or its KWin counterpart):

  ```sh
  command -v hearth-connect >/dev/null 2>&1 && hearth-connect --autostart >/dev/null 2>&1 &
  ```

  Game Mode's session can add the same line so the phone stays linked there too. Turning it on in Settings or the
  window also starts it right away, in either mode. PCs where it was turned on before (the old enabled unit) still count
  as on.
- `hearth-connect --turn-on` / `--turn-off` / `--is-enabled`, and `hearth-connect --call <request> '<json>'` for
  everything else (Settings › Phone uses them through `hosts/linux/desktop/shell/hearth-phone`).
- `hearth-settings phone:notifications`, `phone:commands`, `phone:pair` open the page at that part;
  `phone-messages:<conversation>` opens a conversation.
- sshfs (in the image) opens the phone's files; wpctl the volume; playerctl the players; wl-clipboard / xclip the
  clipboard; uinput the touchpad and keyboard.

## How it's tested

`.github/workflows/phone-link.yml`: the service against a pretend phone (`hosts/hearth-connect/tests/fake_phone.py`,
KDE Connect's protocol in plain Python with its own certificate) over TLS: finding each other, pairing with the same
key on both sides, and every feature above packet by packet, with stand-ins for the notification service, the media
player, the volume and sshfs. Once on Ubuntu with X, once inside KWin (`HEARTH_WM=kwin`). Then Desktop Mode at
1920×1080 with the pretend phone paired, for pictures of every new page, the tile and the Reply box. The real KDE
Connect daemon is checked in `link-apps.yml`.

What only a real phone can show: the KDE Connect app's own screens and permissions (notifications access, SMS,
contacts, the camera, its SFTP server for Browse), iPhone's limits (no SMS or call info on iOS), and the touchpad on a
real screen.
