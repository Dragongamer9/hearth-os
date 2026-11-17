# Hearth TV 1.0, the Google TV home for shows (approved Nov 15, 2026)

Mockups: hearth-tv/setup.jpg, hearth-tv/main.jpg, hearth-tv/home-inputs.jpg, hearth-tv/home-profiles.jpg, hearth-tv/apps.jpg, hearth-tv/media.jpg, hearth-tv/show.jpg, hearth-tv/show-subtitles.jpg, hearth-tv/show-player.jpg, hearth-tv/theater.jpg, hearth-tv/search.jpg, hearth-tv/cleanup.jpg, hearth-tv/settings-home-screen.jpg, hearth-tv/settings-tv-inputs.jpg, hearth-tv/settings-profiles.jpg, hearth-tv/settings-my-media.jpg, hearth-tv/settings-hearth-link.jpg, hearth-tv/settings-screensaver.jpg, hearth-tv/settings-picture-and-sound.jpg, hearth-tv/settings-remote-and-controllers.jpg, hearth-tv/settings-about.jpg, hearth-tv/screensaver.jpg
Release: next

Hearth TV has its own version (hosts/android-tv/VERSION, 1.0, picked by the owner); "next" until it's on his TV.

- [x] Setup: Welcome, Your PC, Apps, Clean up, Done — hosts/android-tv/app/src/main/assets/tv/setup.js: "Welcome to Hearth TV"
- [x] Home: the featured banner (Pick up where you left off, Resume, All episodes, Pin) — hosts/android-tv/app/src/main/assets/tv/home.js: "Pick up where you left off"
- [x] Home: Your apps row with See all apps — hosts/android-tv/app/src/main/assets/tv/home.js: "See all apps"
- [x] Home: Keep watching (your PC and Theater), Recently added, Theater rows — hosts/android-tv/app/src/main/assets/tv/home.js: "Keep watching"
- [x] Home: Inputs picker (HDMI, antenna, AV) — hosts/android-tv/app/src/main/assets/tv/tv.js: "showInputs"
- [x] Home: who's watching (profiles, with a PIN) — hosts/android-tv/app/src/main/assets/tv/tv.js: "showProfiles"
- [x] Apps: On your Home screen (move, remove), other apps, clear an app's cache — hosts/android-tv/app/src/main/assets/tv/apps.js: "On your Home screen"
- [x] Apps: Switched off by TV Cleanup — hosts/android-tv/app/src/main/assets/tv/apps.js: "Switched off by TV Cleanup"
- [x] My Media: All, Shows, Movies, Music, the sorts, tiles without artwork — hosts/android-tv/app/src/main/assets/tv/library.js: "No artwork in your library yet"
- [x] My Media: Jellyfin and Plex on your PC — hosts/android-tv/app/src/main/assets/tv/media.js: "plexStart"
- [x] Show page: resume, start over, seasons, More like this — hosts/android-tv/app/src/main/assets/tv/library.js: "More like this"
- [x] Subtitles and audio panel — hosts/android-tv/app/src/main/assets/tv/library.js: "subtitlesPanel"
- [x] Player: night mode and dialogue boost — hosts/android-tv/app/src/main/assets/tv/library.js: "Night mode"
- [x] Theater: saved links, Videos / Live streams / Playlists / Watched — hosts/android-tv/app/src/main/assets/tv/theater.js: "Live streams"
- [x] Theater: Add a link with a QR code for your phone, or type it — hosts/android-tv/app/src/main/assets/tv/theater.js: "Type a link instead"
- [ ] Theater: On any Hearth PC, right-click a link › Send to TV (needs a Hearth Link "send to TV" request and the PC side)
- [ ] Theater: On your phone, share any video to Hearth TV (needs Hearth for Android to send it over Hearth Link)
- [x] Search: on-screen keyboard, your PC, Theater and apps at once, search in another app — hosts/android-tv/app/src/main/assets/tv/search.js: "Search for"
- [x] Search: voice search — hosts/android-tv/app/src/main/java/dev/hearthos/tv/Bridge.kt: "fun voice("
- [x] TV Cleanup: Your TV is clean, every group, Check again now — hosts/android-tv/app/src/main/assets/tv/settings.js: "Your TV is clean"
- [x] TV Cleanup: switched off, never deleted; Put everything back — hosts/android-tv/app/src/main/assets/tv/settings.js: "Put everything back"
- [x] TV Cleanup: watches for things turning back on (at start and after updates) — hosts/android-tv/app/src/main/java/dev/hearthos/tv/Cleanup.kt: "fun watch("
- [x] Settings › Home screen: every Hearth OS look, its sub-themes, dark or light, accent, background (from palettes.json, so new looks such as Gold come by themselves) — hosts/android-tv/app/src/main/assets/tv/settings.js: "Every Hearth OS look"
- [x] Hearth OS's palettes and wallpapers built into the APK — hosts/android-tv/app/build.gradle.kts: "class CopyHearthLooks"
- [x] Settings › TV inputs: turn on with, come back to Hearth, HDMI-CEC — hosts/android-tv/app/src/main/assets/tv/settings.js: "HDMI-CEC"
- [x] Settings › Profiles — hosts/android-tv/app/src/main/assets/tv/settings.js: "Profiles"
- [x] Settings › My Media: sign in, Wake the PC — hosts/android-tv/app/src/main/assets/tv/settings.js: "Wake the PC"
- [x] Settings › Hearth Link: pair a device — hosts/android-tv/app/src/main/assets/tv/settings.js: "Pair a device"
- [x] Settings › Screensaver and the screensaver itself — hosts/android-tv/app/src/main/assets/tv/screensaver.js: "Press any button to wake up"
- [x] Settings › Picture and sound: match the video's frame rate — hosts/android-tv/app/src/main/java/dev/hearthos/tv/Bridge.kt: "fun frameRate("
- [x] Settings › Remote and controllers — hosts/android-tv/app/src/main/java/dev/hearthos/tv/Bridge.kt: "fun controllers("
- [x] Settings › About: Check for updates — hosts/android-tv/app/src/main/assets/tv/settings.js: "Check for updates"
- [x] Settings are saved safely and never change by themselves (atomic, .bak, merge) — hosts/android-tv/app/src/main/java/dev/hearthos/tv/SettingsStore.kt: ".bak"
