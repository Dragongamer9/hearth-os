# Moving Hearth off Electron: Rust + Tauri

Hearth's screens (ui/index.html) stay the same; they're shared with Ember on Windows. What changes is the
host around them on Linux: Electron + Node.js (hosts/linux) becomes a Rust app (hosts/hearth-rs, Tauri,
drawn by the system's WebKit) with the logic in a plain Rust library (hosts/hearth-core).

Work happens on the `rust-host` branch. Releases keep shipping the Electron host until the Rust one does
everything it does; then Hearth OS switches over in one release.

## Why
- Size: ~15 MB instead of ~250 MB (the installer USB and every update get smaller)
- Memory: a fraction of Electron's 300-400 MB while games run
- Faster start, less battery on handhelds

## The plan (each step is usable on its own and tested in CI: .github/workflows/rust.yml)
1. [x] The window, the message channel the screens already use, game art from the disk
2. [x] Steam library (installed games, play time, art), favorites, hidden, launching Steam games
3. [x] Status bar and Quick Access: battery, Wi-Fi, Bluetooth, volume, brightness, power modes
4. [x] Settings: Wi-Fi, Bluetooth, sound, display modes, power button, sign-in, Hearth OS updates, app updates, device info
5. [x] Manual games, emulators, game settings, launching and watching games, ProtonDB and achievements
6. [x] Epic, GOG and Xbox: sign-in windows, game lists, Hearth's own downloads (queue, pause, resume),
       Proton launching through umu, Epic cloud saves, Xbox Cloud Gaming (catalog and stream window)
7. [x] Game Mode: gamescope focus, the overlay over games, Guide button, power and volume buttons,
       frame-rate limit, performance overlay, screenshots, open apps
8. [x] Profiles, parental controls, collections, backup, remote play, wallpaper, importing from other drives
9. [x] The installer screens (live USB) and the first-start setup
10. [ ] Try it on real hardware (Game Mode, the overlay, store sign-ins, xCloud in WebKit)
11. [ ] Switch Hearth OS to the Rust app; remove Electron

Every method the Electron host's bridge answers has an answer in the Rust one (a test checks it).

## Not done yet in the Rust host
- The Xbox sign-in saved by the Electron host is locked with the desktop keyring; the Rust host keeps its
  own (sign in to Xbox once more after switching)
- App updates: the Rust app updates from a release file named Hearth-linux-x86_64 (+ .sha256), which
  releases start carrying when Hearth switches over. On Hearth OS the app updates with the system.

## Tested in CI so far
- hearth-core's tests, the app built and run in a virtual screen (home, Quick Access, menu, Library,
  Downloads, Settings screenshots: ci/rust-shots)
- Hearth OS built with the Rust app instead of Electron (.github/workflows/rust-os.yml): every library the
  app needs is on the system, and it runs there (ci/rust-os-shots)
- The installer USB built with the Rust app (installer-iso.yml on rust-host): boots to "Connect a controller"
  like the Electron one, and the ISO is 1.6 GB (ci/installer-shots-rust)

## Things that need Chromium today, and what happens to them
- Xbox Cloud Gaming streams in Chromium. The Rust host turns WebKit's WebRTC on for its window; whether
  that's enough for xCloud has to be tried on a real device. If not, the stream opens in a small
  Chromium-based player only while streaming.
- Store sign-in pages (Epic, GOG, Microsoft) open in web view windows with the same controller helper
  (pointer, on-screen keyboard); clicks are made in the page rather than as real input events.
- Hearth Browser stays a separate app (Electron for now); moving it to WebKit is its own project.

## Known WebKit issue
- Some Nvidia drivers draw a black window with WebKit's DMA-BUF renderer: the app turns that renderer off
  (WEBKIT_DISABLE_DMABUF_RENDERER=1).
