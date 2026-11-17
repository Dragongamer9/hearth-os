# Hearth TV

Hearth TV is Hearth OS's home screen for a Google TV or Android TV set: a Game Mode-style home for shows, with your apps,
your PC's library (Jellyfin or Plex), your saved links (Theater), the TV's inputs, and TV Cleanup, which switches off
the ads and tracking the TV ships with. First made for the owner's TCL 55" Google TV. Mockups: docs/mockups/hearth-tv,
checklist docs/mockups/checklists/hearth-tv.md.

- Code: `hosts/android-tv` (app id `dev.hearthos.tv`). A small Kotlin host (like Hearth for Android) with the page in
  `app/src/main/assets/tv/`; the page talks to the TV through `HearthTVNative` (Bridge.kt).
- Version: `hosts/android-tv/VERSION` (the owner picks it; 1.0 first).
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

## Still to come

- Send to TV from a Hearth PC (right-click a link) and from a phone (share a video), over Hearth Link: needs a Hearth
  Link request for it and the PC and phone sides.
