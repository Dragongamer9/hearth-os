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
on the network before you ask. While it's on, it starts by itself every time you sign in, in Game Mode and Desktop
Mode, and the firewall lets phones on your own network in (ports 1714–1764, only from local addresses, through
`hearth-link-firewall`), closing again when it's off. Your choice is remembered ("enabled" in
`~/.config/hearth/connect/settings.json`, which only you can read) and never changes by itself. Off stays off: the
service won't run at all while it's off, however it's started.

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
| Calls | "Ada is calling" pops up with **Mute ringer**; what's playing here pauses and plays again when the call ends (only what Hearth paused); with **Mute this PC's sound during calls** the sound goes off too (on again after, only if Hearth muted it); **Only once I answer** waits for the answer instead of the ringing | |
| Lock | The phone's **Lock** button locks this PC (KWin's locker, or the session's lock on labwc) and shows whether it's locked. A phone can never unlock it: that always takes your password here | |
| This PC's notifications | | With **Send this PC's notifications** on (off until you turn it on), what apps here show in their pop-ups shows on the phone; dismiss it there and it goes here. Choose whether the text goes, only ones that stay, how urgent, and apps left out |
| Keyboard and mouse for the phone | | **This PC's keyboard and mouse on the phone**: a touchpad on the page moves a pointer on the phone (tap, long press, scroll) and **Type on the phone** sends what you type to the field it has open (the phone's KDE Connect Remote Keyboard) |
| Stay awake | | **Keep this PC awake while it's connected** (off until turned on): no screen-off or sleep while that phone is linked |
| Clipboard | Copy on the phone, paste on the PC | And the other way round (Send clipboard too) |
| Files, links, text | Share to "Hearth PC": files are saved in Downloads › Hearth Connect after you accept (or without asking, if you choose); text and links pop up with **Copy** (and **Open** for a web link: nothing opens by itself) and stay listed on the page | **Send files…** (Hearth Files' picker), and **Send text or a link** (a link goes as a link, so the phone offers to open it) |
| Browse the phone's files | | **Browse its files**: the phone's storage opens in Hearth Files |
| Take a photo | | **Take a photo**: the phone's camera opens, the picture comes to Downloads (only a photo you asked for in the last 5 minutes is kept without asking; any other waits for you like a file) |
| Ring / ping | Find my PC: it rings | **Ring** (find my phone) and **Ping** |
| Media | Control the PC's music and video players, and its volume | Control what plays on the phone |
| Touchpad and keyboard | The phone moves the pointer, clicks, scrolls and types | |
| Presentation remote | Next and previous slide, and the phone as a pointer | |
| Run commands | Start the commands you added (Lock the screen, Start Steam...) | Add or remove them in Settings › Phone › Run commands |
| Battery and signal | The phone's battery and signal show in Settings › Phone and on the Quick Access tile | The PC's battery (laptops) shows on the phone |

Each phone has its own switches (What *phone* can do): show its notifications, share the clipboard, touchpad /
keyboard / presentation remote, pause media during calls (and only once answered, and mute this PC's sound), run
commands, control this PC's volume, lock this PC, keep this PC awake, save its files without asking; and "Send this
PC's notifications" under Use this PC on the phone. Lock and the round-1 switches are on for a newly paired phone;
saving files without asking, sending this PC's notifications, staying awake, muting and "only once I answer" are off
until you turn them on (for phones paired before they existed too).

Not yet (later): the phone as a second screen (KDE Connect's virtual monitor) and the link over Bluetooth.

**Browsing the phone's files** works the way KDE Connect does it: the phone starts its file server with a one-time
password and sends it over the paired, encrypted link, and Hearth opens it without checking the file server's own key
(it's made new each time). So it relies on that link: only your paired phone can send it, and only to the phone's own
address on your network.

## For other parts of Hearth

- **Starting it at sign-in.** `hearth-connect.service` is on for every account in the image (`default.target`, so
  Game Mode and Desktop Mode, labwc or KWin alike) with `ExecCondition=hearth-connect --enabled-check`: while Phone link
  is off the unit is skipped (not failed), and `--daemon` itself exits at once. Nothing else needs to start it; a
  session that wants to can run `hearth-connect --autostart` (it does nothing while it's off). PCs where it was
  turned on before (the person's own enabled unit) still count as on until the choice is saved; turning it off removes
  that unit.
- `hearth-connect --turn-on` / `--turn-off` / `--is-enabled`, and `hearth-connect --call <request> '<json>'` for
  everything else (Settings › Phone uses them through `hosts/linux/desktop/shell/hearth-phone`).
- How round 2 works: `kdeconnect.lock` (plugins/lock.rs: ScreenSaver.Lock on KWin, `loginctl lock-session` on labwc,
  whose lock screen sets logind's LockedHint in Lock.qml), this PC's notifications watched on the session bus with
  org.freedesktop.DBus.Monitoring (plugins/sendnotify.rs; its choices are "notifyOut" in settings.json), the outgoing
  `kdeconnect.mousepad.request` (plugins/remote.rs; Settings' touchpad writes straight to the service's socket so the
  pointer keeps up), staying awake as a logind "block" inhibitor for idle and sleep (`systemd-inhibit` reading a pipe
  from the service, so it lets go the moment the phone or the service goes).
- `hearth-settings phone:notifications`, `phone:commands`, `phone:pair` (and `phone:shared`, `phone:remote`,
  `phone:sendnotify`, `phone:options`) open the page at that part;
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
