# Hearth TV

Hearth TV is Hearth OS's home screen for a Google TV or Android TV set: a Game Mode-style home for shows, with your apps,
your PC's library (Jellyfin or Plex), your saved links (Theater), the TV's inputs, and TV Cleanup, which switches off
the ads and tracking the TV ships with. First made for the owner's TCL 55" Google TV. Mockups: docs/mockups/hearth-tv,
checklist docs/mockups/checklists/hearth-tv.md.

- Code: `hosts/android-tv` (app id `dev.hearthos.tv`). A small Kotlin host (like Hearth for Android) with the page in
  `app/src/main/assets/tv/`; the page talks to the TV through `HearthTVNative` (Bridge.kt).
- Version: `hosts/android-tv/VERSION` (the owner picks it; 1.0 first, 1.1 with Send to TV, 1.2 with the remote's pointer on websites).
- Built and tested by `.github/workflows/android-tv.yml`: Hearth Link for Android (arm64 and 32-bit arm), unit tests,
  the APK as the `hearth-tv-<version>` artifact, every screen photographed by `tools/tv-ui-check.js` onto the branch
  `ci/tv-shots-<branch>`. On main the APK goes on the newest release as `hearth-tv-<version>.apk`, where Hearth TV's
  own updates (Settings › About) find it.

## Looks

Hearth TV has every Hearth OS look: `hosts/linux/desktop/theme/palettes.json` (looks, sub-themes, the Black set and the
accents) and Hearth OS's wallpapers are copied into the APK when it's built (`CopyHearthLooks` in
`app/build.gradle.kts`). A look added to palettes.json (the Gold look, when it lands) is in the next Hearth TV build
without any change here.

## Installing it (from a computer on the same network)

On the TV: Settings › System › About › Android TV OS build, press OK seven times (Developer options), then Settings ›
System › Developer options: turn on USB debugging (and Network debugging, or Wireless debugging on newer sets). Note
the TV's address (Settings › Network & Internet › your network).

On the computer (adb from Android's platform-tools):

    tools/hearth-tv/install.sh <TV address> [hearth-tv-1.0.apk]

It connects (the TV asks "Allow debugging?": tick Always allow), saves the TV's package list and settings to
`~/hearth-tv-backup/<date>/`, installs Hearth TV, makes it the home screen and keeps network debugging on so TV Cleanup
can work. Nothing is switched off by the script: Hearth TV's setup does TV Cleanup and asks first. The TV asks once
more to allow debugging, this time for Hearth TV itself.

Put everything back: `tools/hearth-tv/restore.sh <TV address> [--uninstall]`.

## TV Cleanup

Groups in `assets/tv/cleanup.json`: ads and sponsored rows, screen tracking (ACR), TCL's channel and extras, Google
TV's home (kept as a backup), voice search. Each package is switched off with `pm disable-user --user 0` (never
uninstalled; `pm enable` brings it back) through the TV's own adb on 127.0.0.1:5555, with Hearth TV's key. The `never`
list is refused even if a group names it: inputs (`com.tcl.suspension`, `com.tcl.tvinput`), system updates
(`com.tcl.ttvs`), first setup and Wi-Fi (`com.tcl.initsetup`), the remote, Google services and the location provider
(switching that one off can stop the TV starting). System updates are never switched off: Hearth TV checks after each
one and switches off again anything an update turned back on, and tells you.

If the TV restarts, network debugging may go off; TV Cleanup then says so, and what's already switched off stays off.

## Send to TV

Something you're looking at on a Hearth PC or your phone plays on the TV, in Hearth TV (it comes to the front from
whatever app is on).

- **From a Hearth PC**, through Hearth Link (the TV paired with the PC: on the TV, Settings › Hearth Link › Pair a
  device). Right-click a link in Hearth Browser › **Send to TV** (on a video, the video; elsewhere, the page); in
  Hearth Files right-click a video or a song › **Send to TV**; in Mantel's clipboard, a copied link's More ›
  **Send to TV**; in Hearth Theater, a site's or a title's options › **Send to TV** (a Jellyfin title goes as its id
  and opens in the TV's own My Media). Under KDE Plasma (Anatase, no Mantel): Dolphin's right-click on a video or a
  song › **Send to TV** (`/usr/share/kio/servicemenus/hearth-send-to-tv.desktop`); Hearth Browser and Hearth Theater
  work the same there. With more than one TV, each is listed (Dolphin asks which with kdialog). All of them use
  `hearth-send-to-tv` (`os/system_files/usr/bin`), which asks Hearth Link (`tv.list`, `tv.send`;
  `hosts/hearth-link/src/tv.rs`).
- A **link** is saved in the TV's Theater and played. A **video or music file** stays on the PC: it's shared with that
  one TV for 12 hours under a random token, and the TV plays it from Hearth Link's own 127.0.0.1 address there (Hearth
  Storage's media links), each piece read from the PC over Hearth Link, with seeking. A **show or movie** of your
  library opens in My Media (the same Jellyfin or Plex on the PC).
- Only your own devices: a friend's device can't send to your TV or read a file shared with it. Hearth Link on the TV
  keeps what was sent until Hearth TV takes it (`tv.inbox`, every 2 seconds: `SendToTv.kt`); nothing plays behind
  another app.
- **From your phone**: Hearth for Android adds **Hearth TV** to the phone's Share menu (any app: a video's link, or a
  video or song file). Add the TV once: on the TV, Theater › Add a link, scan the code with the phone's camera, and
  on the page that opens tap "Add … to this phone's Share menu" (a `hearthtv://pair` link with the TV's address and
  key; kept in Hearth for Android's `hearth-tv.json`, safely saved). A link goes to the TV's add-a-link server
  (`/send`, with its key) and plays; a file plays from the phone while the "Playing on …" notification shows (Stop
  ends it; only the TV gets an answer, at an address with a random token in it).
- Tested by hearth-link.yml (`e2e.py family`: a PC sends a link, an 11 MB video with seeking and a show to a TV, and a
  handheld, a text file and a friend's TV are refused), android-tv.yml (the TV's screens for it, in tv-ui-check.js),
  android.yml (the phone's unit tests) and files-extras.yml (Hearth Files' menu).

## The remote's pointer on websites (1.2)

A website from Theater (any saved link that isn't YouTube: it opens full screen in the viewer, `ViewerActivity.kt`)
gets a mouse pointer you move with the remote's D-pad, because most websites are made for a mouse or a finger, not
arrow keys.

- **Pointer mode** (how websites start, unless Settings › Remote and controllers › Start websites in pointer mode is
  off): the pointer shows at the centre. ▲▼◀▶ move it, slowly at first for small steps and faster the longer you
  hold a button. OK clicks at the pointer, and holding OK is a right-click (a long-press, like holding a finger on
  it: the site's own menu); a ring around the tip fills while you hold it. CH +/− (or Page up/down, or the number
  keys 2 and 8; 4 and 6 sideways) scroll, and pushing the pointer past an edge scrolls too. Back leaves a video's full
  screen, goes back a page, and leaves the website from its first page. A hint at the bottom says all this for a few
  seconds when a website opens and when the mode changes.
- **Page mode** (hold Back, or press ☰ Menu, a colour key or a controller's Start; the same again brings the pointer
  back): the remote works the page the way the site built it (arrows and OK go to the site), for sites with good
  D-pad support. In a video's full screen OK plays and pauses and ◀ ▶ skip 10 s, as the viewer always did.
  Play/pause, ⏩ ⏪ and a controller's Y (full screen) work in both modes.
- **Clicks are real**: the pointer is drawn by a see-through layer over the website (`assets/tv/pointer.html`,
  `pointer.js`: Hearth's own page from the APK, never put into the site). OK is a real finger put on the website's own
  view at the pointer (pressed while OK is down, so moving meanwhile drags), hovering and scrolling real mouse events, so the site's buttons,
  menus, video players and their controls work as with a finger or a mouse, inside frames too. When a click lands on
  a text field, the TV's own keyboard opens.
- **Looks**: the pointer is in your look's accent colour with a white edge and a soft shadow, so it shows on light and
  dark pages; the hint uses the look's colours.
- **Settings** (Settings › Remote and controllers › Pointer on websites): Start websites in pointer mode (on), Pointer
  speed (Slow, Normal, Fast), Pointer size (Small, Medium, Large), saved in settings.json's `pointer` like every
  setting (merged, atomic, .bak); nothing is written until you change one.
- Nothing runs while the pointer stands still: it's only redrawn while a button is held.
- Hearth TV's own screens, Theater's YouTube player and My Media's player keep their D-pad navigation: the pointer is
  only on websites.
- Tests: `hosts/android-tv/test/pointer.test.js` (speed and acceleration, edges and scrolling, clicks and long-presses,
  dragging, Back, page mode, nothing running at rest; `node --test`), `PointerKeysTest.kt` (the remote's buttons, the wheel, the
  settings' defaults; Gradle's unit tests), and `tools/tv-ui-check.js`, which stacks the pointer over a pretend video
  site (`tools/tv-pointer/`), clicks its Play button, holds OK on it (its own menu opens), scrolls it and pictures it (`pointer-*.jpg`)
  and the settings rows (`10-pointer-settings.jpg`). The real touch and mouse events on a TV can't be run in CI (no
  TV there): they need trying on the TV.

