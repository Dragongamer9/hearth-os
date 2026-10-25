> **Archived (2026-10-21).** Hearth for Android is on hold; the focus is Hearth OS and Ember. The code and these notes stay for reference; the Android workflow only runs by hand (type "archived"). The last published APK is on the v0.2.2 release.

# Hearth for Android

Hearth's library and controller interface as an Android app: the same page as Hearth OS and Ember (`ui/index.html`), in a full-screen WebView, with a small Kotlin host (`hosts/android`). It can also be your home screen, so a handheld or TV box starts straight into your games. This first version (0.2.1) is **Beta Experimental**.

## What works

| | |
|---|---|
| **Library** | Installed games: everything Android calls a game, plus known emulators, PC-game launchers and streaming apps (RetroArch, Dolphin, PPSSPP, AetherSX2/NetherSX2, Citra/Lime3DS/Azahar, yuzu forks (Eden, Citron, Sudachi), DuckStation, Vita3K, Winlator, GameHub, DroidDeck, Steam Link, Moonlight, Xbox, Game Pass, GeForce NOW and more; the list is in `GameDetect.kt`). Box art is made from each app's icon. Favorites, Hide, collections, play time and Search work as on the other Hearths. |
| **Other apps** | Settings › Games › **Add an app** puts any app in the library. **Show all apps** lists every app under Non-library. |
| **Playing** | A starts the game like its launcher icon does. Hearth counts play time from when the game comes to the front until Hearth is back in front (Home, Back out of the game, or the recent-apps switcher), and it's kept if Android closes Hearth in between. |
| **ROMs** | Settings › Games › **ROMs folder**: pick the folder with your games (Android's folder picker; Hearth only gets to read it). One folder per system inside, named like ES-DE and EmuDeck (`snes`, `gba`, `psx`, `n64`, `nds`, `psp`, `genesis`, `dreamcast`…, the same as Hearth OS); EmuDeck's `Emulation` folder (with `roms` inside) works too. Games get their titles and box art from the libretro thumbnails. They play in **RetroArch** with their system's core; without RetroArch, or for a file RetroArch can't get to, Android asks which app should open it. |
| **Controller** | USB and Bluetooth controllers: A select, B back (Back on the phone too), Start menu, View/Select and Guide Quick Access, LB/RB tabs, d-pad and both sticks (right stick jumps A–Z in the library). A keyboard and touch work as well. |
| **Device** | Battery and Wi-Fi in the top bar; volume and mute in Quick Access and Settings › Display & sound; Wi-Fi, Bluetooth, airplane mode, display, sound and battery rows open Android's own settings (apps can't switch those themselves since Android 10). |
| **Home screen** | Settings › System › **Use Hearth as your home app** opens Android's default-home choice. Home then opens Hearth's Home. |
| **Linux (Beta Experimental)** | Settings › System › **Linux**: a real Debian Linux (ARM64) inside Hearth, without root (PRoot): install it, run commands, install packages, show Linux apps in Termux:X11. See [ANDROID-LINUX.md](ANDROID-LINUX.md). |
| **Logs** | Settings › System › **Save logs** (a file in `Android/data/dev.hearthos.android/files/`) and **Send logs to the developer** (Hearth OS's bug-report service; you get a code to share). |

Not in this version (the page hides it on Android): store accounts and installs (Steam, Epic, GOG, Xbox), Downloads, Quick Resume, performance and power settings, Hearth Input layouts, Couch Co-op, Multi-seat, updates from inside Hearth, your own profile pictures, the in-game overlay. Quick Access opens over Hearth itself, not over a game.

## Install

There's no store release yet. The APK comes from GitHub Actions:

1. Open the repository's **Actions › Android** workflow and its newest green run (branch `android` or `main`).
2. Under **Artifacts**, download **hearth-android-0.2.1** and unzip it: `hearth-android-0.2.1.apk` is the one to install (`-debug` is for development and installs next to it).
3. Copy it to the device and open it (allow "Install unknown apps" for your file manager or browser when Android asks), or with a computer: `adb install hearth-android-0.2.1.apk`.

Android 8.0 (API 26) or newer. An update installs over the old version when both were signed with the same key (see Signing below); otherwise uninstall first (that deletes Hearth's favorites, collections and play time).

## Use it as your home screen

Settings › System › **Use Hearth as your home app** (or Android's Settings › Apps › Default apps › Home app) and pick **Hearth**. Pressing Home then always brings you to Hearth's Home. To change back: the same row opens the same Android page; Hearth's Power menu has **Leave Hearth** when it isn't the home app.

## Permissions

| Permission | Why |
|---|---|
| Internet | Game art (libretro thumbnails), the player counter (Settings › System › Count me in the player count turns it off), Send logs |
| Network state, Wi-Fi state | Wi-Fi on/off and signal in the top bar |
| Bluetooth (Android 11 and older only) | Bluetooth on/off in Quick Access |
| ROMs folder (chosen by you) | Reading your ROMs; nothing else on the storage |
| Foreground service (special use) | Only while Hearth Linux runs something: keeps Linux programs running while Hearth is in the background (a "Hearth Linux is running" notification) |

No location, no storage-wide access, no usage access. Because of that the Wi-Fi network's name shows as "Wi-Fi", and Hearth can't see a game still running in the background (it counts the game as over when Hearth is in front again). Hearth sees apps that have a launcher icon (Android 11 package visibility, no QUERY_ALL_PACKAGES).

## RetroArch and ROMs

- Install RetroArch from Google Play or [retroarch.com](https://www.retroarch.com/) (`com.retroarch`, `com.retroarch.aarch64` or `com.retroarch.ra32`; Hearth uses the first it finds of aarch64, then the Play build, then 32-bit).
- Get the cores in RetroArch › Main Menu › Online Updater › Core Downloader. Hearth asks for the first core of each system (`Roms.kt`, e.g. `snes9x`, `mgba`, `mupen64plus_next_gles3`, `swanstation`, `ppsspp`, `flycast`); if it isn't downloaded, RetroArch says it couldn't load the core.
- RetroArch opens ROMs by file path, so it needs its own storage access (RetroArch asks for "All files access" on Android 11+), and the ROMs folder must be on the device's storage or SD card (Hearth turns `primary:ROMs/snes/x.sfc` into `/storage/emulated/0/ROMs/snes/x.sfc`).
- How it's started (RetroArch's documented intent): activity `com.retroarch.browser.retroactivity.RetroActivityFuture` in the RetroArch package, with extras `ROM` (the file), `LIBRETRO` (`/data/data/<package>/cores/<core>_libretro_android.so`), `CONFIGFILE` (`/storage/emulated/0/Android/data/<package>/files/retroarch.cfg`) and `IME`.
- Anything else: Android's "open with" for the file, so any emulator that opens files can play it.

## How it's built

```
hosts/android/
  settings.gradle.kts, build.gradle.kts    Android Gradle Plugin 8.11, Kotlin 2.2, Gradle 8.14.3
  app/build.gradle.kts                     version from hosts/hearth-rs/Cargo.toml; copies ui/index.html into the assets
  app/src/main/java/dev/hearthos/android/
    MainActivity.kt   the WebView, the asset loader (page + icon art), the host shim, controllers, lifecycle
    Bridge.kt         the host: the page's calls and events
    Apps.kt           launchable apps; icon art
    GameDetect.kt     which apps are games
    RomFolder.kt      the ROMs folder (Storage Access Framework) and starting a ROM
    Roms.kt           systems, file types, titles, box art, RetroArch paths
    Store.kt          library.json: favorites, hidden, collections, play time, profile
    DeviceStatus.kt   battery, Wi-Fi, volume, display, device info, controllers
    Pad.kt            controller buttons and sticks to the page's button names
    Logs.kt           Hearth's log and the bug report
    LinuxEnv.kt       Hearth Linux: install, run, apt, Termux:X11 (ANDROID-LINUX.md)
    LinuxLogic.kt     its pure parts: pinned sources, tar reader, PRoot argv, /proc stand-ins (unit-tested)
    LinuxService.kt   the foreground service that keeps Linux programs running
  app/src/main/assets/licenses/   PRoot's and talloc's licenses, and where the binaries come from
  app/src/main/jniLibs/           PRoot (fetch-proot.sh; not in git)
  fetch-proot.sh                  puts the pinned, checksummed PRoot builds into jniLibs
  app/src/test/...    unit tests (game detection, ROM rules, store, controller, logs)
```

The page is served from `https://appassets.androidplatform.net/ui/index.html` by androidx.webkit's `WebViewAssetLoader`, so it's a normal https page (fetch and localStorage work). Before the page's own script, Hearth puts a shim at the top of `<head>` (like `hosts/hearth-rs`'s SHIM) that makes `window.nimbusHost` with `platform: 'android'`: its `postMessage` goes to the `HearthAndroid` JavaScript interface, and the host answers with `evaluateJavascript("window.__hearthRecv(...)")`. The messages are the same as everywhere ([bridge.md](bridge.md)). On screens under 1000 dp wide (phones, most handhelds) the page is laid out 1280 px wide and scaled to fit.

The page knows it's on Android from the shim (`ANDROID` in `ui/index.html`, next to `LINUX`): Settings shows only Profile, Appearance, Library, Wi-Fi, Bluetooth, Display & sound, Power, Controls, Games › Library, Linux (when the host has the `linux` feature), System and About, with Android's rows (`androidPage`); no Downloads; an **Android** library tab; the Power menu is just Leave Hearth.

Controller input: Hearth reads controller KeyEvents and joystick MotionEvents itself and sends the page `pad` events (as the Linux host does), with the page's repeat timing for held directions; it also tells the page `padHost` so the page doesn't read the same controller through the Gamepad API.

### Host methods

| Method | Answer |
|---|---|
| `host.info` | `{platform:'android', host:'Hearth (Android)', name, version, edition:'android', features:['android','sendLogs','linux'], deviceId, setupDone:true, isDefaultHome}` |
| `sys.status`, `audio.get`, `audio.setVolume {v}`, `audio.setMute {m}`, `display.get`, `power.get`, `system.info`, `pad.status` | The device, in the other hosts' shapes (brightness unsupported) |
| `radios.set`, `airplane.set` | Open Android's Wi-Fi / Bluetooth / airplane settings |
| `library.get {rescan}` | `{games:[dto], collections, counts:{android, manual, emu}, runningId}`; app ids `android:<package>`, ROM ids `emu:<system>:<path>` |
| `library.setFav`, `library.setHidden`, `library.collection.create/rename/remove/set`, `library.remove`, `library.addManual {id: package}`, `library.downloads` (empty) | As on Linux |
| `game.launch {id}` → `{tracked:true}`, `game.close`, `game.info` (`{}`) | Events `game.started` (Hearth went behind the game) and `game.exited` (Hearth is back, with the game's updated dto) |
| `profile.get`, `profile.set` | As on Linux (no own pictures yet) |
| `logs.send` → `{path}`, `logs.sendToDeveloper {note}` → `{path, code, url}` | POST to `https://hearth-counter.game305-cl.workers.dev/v1/logs`, `X-Hearth-Kind: android` |
| `android.status` | `{showAll, home:{isDefault}, roms:{folder, count, systems}, retroarch, pads}` |
| `android.openSettings {page, id?}` | `wifi`, `bluetooth`, `display`, `sound`, `battery`, `airplane`, `home`, `app` (App info), `settings` |
| `android.apps` | Apps not in the library, for Add an app |
| `android.setShowAll {on}`, `android.roms.pick` (waits for the folder picker), `android.roms.forget` | Settings › Games |
| `android.openUrl {url}` | Opens an https link in the browser |
| `linux.status`, `linux.install`, `linux.run`, `linux.apt`, `linux.stop`, `linux.openX11`, `linux.reset`, `linux.remove` | Hearth Linux; events `linux.progress`, `linux.output`, `linux.exit`, `linux.done`. See [ANDROID-LINUX.md](ANDROID-LINUX.md) |

Anything else answers "That isn't in Hearth for Android yet". Events: `status` (every 30 s and when Hearth is back in front), `pad`, `padHost`, `game.started`, `game.exited`, `library.changed`, `open` (Home pressed).

### Build it yourself

You need JDK 17 and the Android SDK (platform 35). The Gradle wrapper's jar isn't kept in git; make it once with an installed Gradle (8.13 or newer):

```sh
cd hosts/android
gradle wrapper --gradle-version 8.14.3
./fetch-proot.sh          # PRoot for Hearth Linux (pinned, checksummed; optional: without it the Linux tab says it's missing)
./gradlew testDebugUnitTest assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
```

Or open `hosts/android` in Android Studio. Debug builds are `dev.hearthos.android.debug` (they install next to the release build) and can be inspected from Chrome's `chrome://inspect`.

### CI

`.github/workflows/android.yml` runs on pushes to `android`, `linux-host` and `main` that touch `hosts/android/**` or `ui/**` (and by hand): JDK 17, the Android SDK (android-actions/setup-android), Gradle 8.14.3 (gradle/actions/setup-gradle, so no wrapper jar is needed), then PRoot for Hearth Linux (`fetch-proot.sh`: pinned downloads, SHA-256 checked, a mismatch fails the build), the unit tests, `assembleDebug` and `assembleRelease` (the APK must contain PRoot and its license), and lint (reported, never failing the build). The APKs are uploaded as **hearth-android-&lt;version&gt;**. A failing step puts its error lines in an annotation on the run page. There's no release or publish step yet.

**Signing.** With the repository secrets `ANDROID_KEYSTORE_B64` (the keystore, base64), `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and `ANDROID_KEY_PASSWORD`, the release APK is signed with that key; without them, with the build's debug key (it installs, but every CI machine has a different debug key, so an update may need an uninstall first). Make a key once with:

```sh
keytool -genkeypair -v -keystore hearth-release.jks -alias hearth -keyalg RSA -keysize 4096 -validity 10000
base64 -w0 hearth-release.jks   # → ANDROID_KEYSTORE_B64
```

`tools/ui-check.js` also checks the page as Hearth for Android (a pretend Android host): the Settings pages, the Android rows, the library tab, the Power menu, Add an app and the Linux tab (also in the browser preview, `ui/index.html?preview=android`, with a pretend Linux).

## What's next

- **DroidDeck and Steam:** DroidDeck (`com.droiddeck.launcher`) is already in the library as an app; next is listing the Steam games it has installed and starting one directly, and Steam Link / Moonlight hosts as library entries.
- Choosing the emulator per system (and per game), standalone emulators' own intents (Dolphin, PPSSPP, AetherSX2, DuckStation…), and checking which RetroArch cores are installed.
- Play time from Android's usage access (optional), so a game running behind Hearth still counts.
- Your own profile pictures, an Android TV layout check, a store release (Play / F-Droid / GitHub Releases) with updates.
