# Hearth Linux (Android, Beta Experimental)

A real ARM64 Linux (Debian 13 "trixie") inside Hearth for Android, without root. Settings › System › **Linux (Beta Experimental)**.

- **Stage 1:** install it, run commands, install packages, show Linux apps in Termux:X11. Confirmed on a real device (Oukitel RT8: MediaTek Helio G99, Mali-G57, Android 13, 1920×1200): the install works, xterm shows in Termux:X11, and `xfce4 dbus-x11` + `dbus-launch --exit-with-session startxfce4` gives an XFCE desktop there.
- **Stage 2:** the [Linux desktop](#stage-2-the-linux-desktop) with one tap, shown **inside Hearth** (TigerVNC's Xvnc in the Linux, noVNC in Hearth's page, the controller as mouse and keyboard) or in Termux:X11. Built and checked off-device; not yet run on a device. See [What's verified](#whats-verified-and-what-isnt).

## How it works

```
Hearth (the app)                       filesDir/linux/
  LinuxEnv.kt ──exec──► libproot.so ──► rootfs/            Debian, unpacked by Hearth
       │                (PRoot, ptrace)  sysdata/           stand-ins for the /proc files Android hides
       │                    │            tmp/               PRoot's own temp files
       │                    └─ runs /bin/sh -lc "<cmd>" inside rootfs, as hearth (uid 1000) or root (-0)
       └──exec──► /system/bin/app_process (Termux:X11's X server, CLASSPATH = its APK) ── socket rootfs/tmp/.X11-unix/X0
```

- **PRoot** is a user-space `chroot`/`mount --bind`: it traces the Linux programs with `ptrace` and rewrites their paths, so a whole distribution runs from a folder, as an ordinary app, with no root and no kernel support. It's how Termux's proot-distro, UserLAnd, AnLinux and Andronix run Linux on Android.
- **Why it's a "library".** Since Android 10 an app may only run programs from its native-library folder (W^X: the data folder is `noexec` for apps targeting API 29+). So PRoot ships in the APK as `lib/<abi>/libproot.so`, its loader as `libproot-loader.so` (and `libproot-loader32.so`), and the APK keeps them unpacked on the device (`packaging.jniLibs.useLegacyPackaging = true`, i.e. `extractNativeLibs=true`). Hearth starts `nativeLibraryDir/libproot.so` with `ProcessBuilder` and tells PRoot where its loader is (`PROOT_LOADER`, `PROOT_LOADER_32`). The loader is what `execve`s each Linux program: PRoot runs the loader (an allowed, native-library-folder file) and the loader maps the Linux program into memory itself, so Linux binaries in the data folder never have to be `exec`'d directly.
- **Every start** (`ProotArgs.build`, unit-tested): `--kill-on-exit --link2symlink -L --kernel-release=6.6.0-hearth`, `-0` (root) or `--change-id=1000:1000` (the user `hearth`), `--rootfs=…/rootfs`, `--cwd=` the user's home, binds `/dev`, `/proc`, `/sys`, `/dev/urandom` as `/dev/random`, `/proc/self/fd` as `/dev/fd` when missing, `rootfs/tmp` as `/dev/shm`, an empty folder over `/sys/fs/selinux`, a stand-in over each of `/proc/loadavg`, `/proc/stat`, `/proc/uptime`, `/proc/version`, `/proc/vmstat`, `/proc/sys/kernel/cap_last_cap`, `/proc/sys/fs/inotify/max_user_watches`, `/proc/sys/kernel/overflow{u,g}id` that Hearth itself can't read (Android's SELinux hides them; apt, ps, top and free read them), and Android's shared storage at `/sdcard` and `/mnt/sdcard` only when Hearth can read it (it has no storage permission, so normally not). Then `/bin/sh -lc <command>`.
- **Environment:** cleared, then `HOME`, `USER`, `LOGNAME`, `PATH` (Debian's), `TERM=dumb`, `LANG=C.UTF-8`, `TMPDIR=/tmp`, `SHELL`, the `PROOT_*` variables (`PROOT_TMP_DIR`, `PROOT_LOADER`, `PROOT_L2S_DIR=rootfs/.l2s`) and Android's `ANDROID_ROOT`-style variables. No `LD_*` from Android.
- **Hard links** inside the Linux become `--link2symlink` emulation (Android's SELinux refuses `link(2)` in app storage on many devices); while unpacking, Hearth tries a real link and copies the file when that's refused.
- **Staying alive.** While anything runs (an install, a command, the X server) `LinuxService` is a foreground service (`specialUse`, with an ongoing "Hearth Linux is running" notification), so Android doesn't freeze (cached-app freezer) or close Hearth's processes while you're in Termux:X11 or another app. It stops when nothing runs. (On Android 13+ the notification shows only if you allow Hearth's notifications; the service works either way.)

### Install

`linux.install` (one job at a time; needs about 1.5 GB free):

1. **Download** (0–45 %): https only, to `cache/<version>.tar.xz.part`, picked up where it stopped (`Range`), from the first pinned URL that works (GitHub, then the easycli.sh mirror); then **SHA-256 must match** the pinned value, or the file is deleted and nothing is unpacked. The verified file is kept so **Reset** doesn't download again (**Remove** deletes it).
2. **Unpack** (45–85 %): XZ for Java + Hearth's own tar reader (`TarReader`/`TarExtractor` in `LinuxLogic.kt`): ustar, GNU long names, PAX headers, base-256 sizes, header checksums. Safe by construction: names with `..` are refused, a leading `/` is dropped, the archive's top folder is stripped, nothing is written through a symlink (every folder on the way must be a real folder), hard links must point inside the rootfs at a regular file, device nodes and FIFOs are skipped (PRoot binds the real `/dev`), setuid bits are dropped, the owner keeps read/write on everything (folders get their own modes last, so read-only folders don't block their own files). Deleting (Reset/Remove) never follows a symlink either (`SafeFs.deleteTree`).
3. **Set up** (85–88 %), like proot-distro: `/etc/resolv.conf` (8.8.8.8, 1.1.1.1), `/etc/hosts`, the user `hearth` (uid/gid 1000) in passwd/group/shadow/gshadow with `/home/hearth` from `/etc/skel`, `en_US.UTF-8` enabled in `/etc/locale.gen`, `/tmp` and `.l2s`, and the `/proc` stand-ins (in `sysdata/`, outside the rootfs, so a Linux program can't swap them).
4. **apt** (88–100 %): `locale-gen; apt-get update` as root inside PRoot. Without internet it still finishes (apt update is tried again before the first package install).

### Bridge methods (Bridge.kt → LinuxEnv.kt)

| Method | Answer / events |
|---|---|
| `linux.status` | `{installed, sizeBytes, distro, version, running, jobs:[{job,kind}], busy:{kind,pct,text}\|null, x11App, x11Running, supported, unsupported, freeBytes, needBytes, sdcard, error}` |
| `linux.install` | `{started, job:'install'}`; events `linux.progress {kind, pct, text}`, `linux.output {job:'install', lines}`, then `linux.done {kind:'install', ok, text\|error, status}` |
| `linux.run {cmd, root?, timeout?}` | `{job}`; `cmd` (NULs stripped, at most 8000 characters) runs as `/bin/sh -lc cmd` inside the Linux (argv, never a shell on Android's side); events `linux.output {job, lines}` (stdout and stderr together, batched), `linux.exit {job, kind, code}`. At most 6 at once. |
| `linux.apt {action:'install'\|'remove'\|'upgrade', packages}` | `{job}`; names must match `[a-z0-9][a-z0-9+.-]*` (at most 64); runs `apt-get -y …` as root with `DEBIAN_FRONTEND=noninteractive`; output/exit events, then `linux.done {kind:'apt'}`. Counts as the one install/apt job. |
| `linux.stop {job}` | Stops a command (SIGTERM to PRoot, which takes everything it started with it; SIGKILL after 3 s), or cancels the install/reset (`job:'install'`/`'reset'`). |
| `linux.openX11 {cmd}` | `{ok:true, starting:true}` at once; the start (Termux:X11's server, its activity, up to 15 s for its display, then the program) runs off the bridge's thread and ends with the event `linux.x11 {ok:true, job, kind:'x11'}` or `{ok:false, error, kind}`. Or at once `{ok:false, needApp:true, url, text}` (Termux:X11 isn't installed), `{ok:false, needTrust:true, signer, changed, text}` (not allowed yet) or `{ok:false, needPackages:[…], text}` (xkb-data and/or the program aren't in the Linux yet; the page offers to install them). |
| `linux.desktop.status` | `{linux, installed, missing:[packages], packages, mode:'hearth'\|'x11' (the last one started), x11App, installing, running, method:'hearth'\|'x11'\|null, ready, width?, height?}` |
| `linux.desktop.install` | `{started, job}`: one apt job as root (`apt-get update`, then `apt-get -y --no-install-recommends -o APT::Status-Fd=1 install xfce4 xfce4-terminal dbus-x11 tigervnc-standalone-server xkb-data`); busy kind `desktop`, `linux.progress {kind:'desktop', pct, text}` from apt's status lines (lists 0–5 %, download 5–50 %, install 50–99 %), output events, then `linux.done {kind:'desktop', ok}`. `linux.stop {job:'desktop'}` cancels it. |
| `linux.desktop.start {mode, width, height}` | `mode` `'hearth'` or `'x11'` (remembered); `width`/`height` the screen in device pixels (200–16384, checked). Hearth: `{ok, mode, running, ready, port, token, password, width, height}` at once, then `linux.desktop {state:'ready', …same}` when Xvnc answers (up to 45 s), or `{state:'failed', error}`. X11: `{ok, mode:'x11', starting}` then `linux.x11 {kind:'desktop', …}`. Either: `{ok:false, needInstall, missing, text}` or Termux:X11's `need…` answers. Starting the running mode again returns the same connection (a reconnect). |
| `linux.desktop.stop` | `{ok, running:false}`: ends the session and Xvnc (SIGTERM to PRoot) and the WebSocket. Event `linux.desktop {state:'stopped', mode, code}` (also when XFCE is logged out from inside). |
| `pad.raw {on}` | The desktop's view: controller buttons as `pad {b, down, repeat?}` presses and releases, the sticks and triggers as `padAxes {lx, ly, rx, ry, lt, rt}`, the hat as d-pad buttons, Android's Back as `back`. Off again when the view closes and whenever the page (re)loads. |
| `linux.reset {confirm:true}` | Like install, from the saved download (`linux.done {kind:'reset'}`). |
| `linux.remove {confirm:true}` | Deletes `filesDir/linux` (everything); returns the new status. |
| `android.openUrl {url}` | https links only (the page's "Get Termux:X11"). |

`host.info`'s features include `linux` (the tab) and `linuxDesktop` (the desktop rows); the page shows them on Android only when they're there.

## Display: Termux:X11

Termux:X11 (`com.termux.x11`, [github.com/termux/termux-x11](https://github.com/termux/termux-x11), GPL-3.0) is an X server as an Android app. How it's split, from its source (`lorie/src/main/java/com/termux/x11/CmdEntryPoint.java`, `lorie/src/main/cpp/lorie/cmdentrypoint.cpp`, `shell-loader/scripts/termux-x11.in`, commit 5986cfc, 2026-10-20):

- The **X server** isn't started by the app: Termux's `termux-x11 :0` command runs `/system/bin/app_process / com.termux.x11.CmdEntryPoint :0` with the Termux:X11 APK on the class path. It loads `libXlorie.so` from that APK, creates its socket at `$TMPDIR/.X11-unix/X0`, finds keyboard data at `dirname($TMPDIR)/usr/share/X11/xkb` (or `$XKB_CONFIG_ROOT`), and sends the broadcast `com.termux.x11.CmdEntryPoint.ACTION_START` (with a binder) to the Termux:X11 app, whose exported receiver has no permission or caller check. The app's activity then connects and shows the screen.
- So **Termux isn't needed**: Hearth does the same as `termux-x11`: `app_process -Xnoimage-dex2oat / --nice-name=hearth-x11 com.termux.x11.CmdEntryPoint :0` with `CLASSPATH` = Termux:X11's `sourceDir` (from PackageManager; `<queries>` lists the package), `TMPDIR = rootfs/tmp` and `XKB_CONFIG_ROOT = rootfs/usr/share/X11/xkb`. The socket lands at `/tmp/.X11-unix/X0` inside the Linux, so Linux programs get `DISPLAY=:0`. Then Hearth opens Termux:X11's `MainActivity`, waits up to 15 s for the socket, and runs the program as the user `hearth`.
- The Linux needs `xkb-data` (and the program, e.g. `xterm`); the page offers to install what's missing.
- **Security note:** Termux:X11's code then runs in a process with Hearth's user id (as it does with Termux's in Termux). Hearth doesn't check the APK's signature (GitHub builds of Termux:X11 are signed with a public test key, so there's nothing reliable to pin). Install Termux:X11 only from its official releases.

If Termux:X11 isn't installed, `linux.openX11` answers with its releases link.

## Stage 2: the Linux desktop

Settings › System › Linux › **Linux desktop**. If XFCE isn't installed, the row offers one install (with progress); then it starts the desktop. **Show it** picks where (remembered, on the page and by the host): **In Hearth** (default) or **In Termux:X11** (stage 1's path, with its trust prompt). **Stop the desktop** ends it.

```
Hearth's page (WebView, https://appassets.androidplatform.net/ui/)
  noVNC RFB (ui/novnc/core/rfb.js) ──ws://127.0.0.1:<random>/<128-bit token>──► WsBridge.kt (in Hearth)
                                                                                    │ TCP 127.0.0.1:<random>
  inside the Linux (PRoot, as hearth):  Xvnc :1 -rfbport=<that> -localhost=1 -SecurityTypes=VncAuth ◄──┘
                                        DISPLAY=:1 dbus-launch --exit-with-session startxfce4
```

- **In the Linux** (`DesktopPlan.script`, one job as `hearth`, unit-tested and dry-run with stand-in programs): makes `/tmp/.X11-unix`, removes a stale `:1` lock, starts `Xvnc :1 -geometry WxH -depth 24 -rfbport=<port> -localhost=1 -SecurityTypes=VncAuth -rfbauth=/home/hearth/.vnc/hearth-passwd -AlwaysShared=1 -desktop=Hearth -nolisten tcp` (Xvnc's parameters in its documented `-name=value` form), waits for its X socket, runs the XFCE session, and stops Xvnc when the session ends. Stopping the job ends both (PRoot's `--kill-on-exit`). **Size:** the screen in device pixels / 1.5, the long side kept within 1024–1920 when the screen allows (1920×1200 → 1280×800).
- **The password file** (`VncPassword.file`): what `vncpasswd -f` writes — the password's first 8 bytes DES-encrypted with VNC's fixed key, each key byte's bits reversed (VNC's d3des convention). Hearth's own DES (`Des`, ~100 lines) is checked against the FIPS worked example, the JVM's DES for 200 random blocks, and the well-known `password` → `dbd83cfd727a1458`. A new random 8-character password per start (VNC uses only 8), written mode 600; deleted when the desktop ends.
- **The bridge** (`WsBridge.kt`, no dependencies): noVNC speaks WebSocket; Hearth runs a tiny RFC 6455 server on 127.0.0.1, a random port, per start: the request must be `GET /<token>` (compared in constant time), a WebSocket upgrade, version 13, a valid key, and `Origin: https://appassets.androidplatform.net`; anything else gets 403/400. Then client frames must be masked (≤1 MB each; reserved bits, unknown opcodes, fragmented or big control frames close it), pings are answered, a close is echoed; Xvnc's bytes go back as binary frames (16/64-bit lengths). Xvnc not up yet → the WebSocket opens and closes with 1011, and the page tries again. At most 3 connections. Stopped with the desktop.
- **In the page** (ANDROID only): a full-screen view hosting noVNC's `RFB` (`import('./novnc/core/rfb.js')`, served by the asset loader): scaled to fit, the password from the host, `shared`, no resize, best quality/no compression (it's local). It connects when the host says `ready`, and when the connection drops it asks `linux.desktop.status`: still running → connects again (backing off, up to 12 tries); stopped → "The Linux desktop stopped" with Start it again / Back to Hearth. Hearth draws its own pointer over noVNC's canvas and sends the pointer itself (noVNC has no public pointer call: `RFB.messages.pointerEvent` on its socket, framebuffer coordinates; pinned version).
- **Controller** (raw mode while the view is open; Hearth's own navigation stands still): right stick (or left, whichever is pushed further) moves the pointer, faster the further and the longer it's held; **A** left button (hold to drag), **B** right button, **X** Esc, **Y** Hearth's own keyboard (the text prompt; what you type goes in as keysyms: Latin-1 as is, the rest as Unicode keysyms), **LB/RB** scroll up/down (repeating while held), **LT** Backspace, **RT** Enter, **d-pad** arrow keys (repeating while held), **Start** Alt+F1 (XFCE's Applications menu), **Guide/View** or Android's **Back** the bar.
- **The bar** (hidden until Guide/View/Back): Keyboard, Ctrl, Alt, Super (held for the next key; Ctrl and Alt light up), Esc, Tab, Enter, Backspace, **Stop desktop**, **Back to Hearth** (the desktop keeps running: open it again and it reconnects). D-pad moves along it, A presses, B closes it. Touch works too.
- **Touch** is a touchpad: drag moves the pointer, tap clicks, tap-then-drag drags (and a double tap is a double click), two fingers scroll, a two-finger tap is a right click. A real mouse points directly; its wheel scrolls. A hardware keyboard goes to noVNC.
- **Kept running** while Hearth is in front or not: the desktop is a Linux job, so the foreground service holds (as in stage 1).

### Security

Any app on the device can connect to 127.0.0.1, so three things stand between them and the desktop: **Xvnc listens on 127.0.0.1 only and asks for a VNC password** (random per start; TigerVNC also blacklists repeated failures), **the WebSocket answers only at a random 128-bit token path** with Hearth's page origin, and both ports are random per start. The password and token are only in Hearth's memory, the page, and the 600-mode password file inside the Linux; they're never logged. (VNC's own encryption is weak, but the traffic never leaves the device.)

### Limits (stage 2)

- **Software rendering.** Xvnc draws in memory and noVNC on a 2D canvas: fine for a desktop, a terminal, a browser or an editor, not for games or 3D. That's stage 3: on **Mali** (the RT8's Mali-G57 and most MediaTek devices) GPU acceleration in PRoot means **VirGL** (virgl_test_server on Android's GLES, Mesa's virgl in the Linux) or **Vulkan passthrough** (e.g. venus / a Vulkan wrapper over Android's Mali driver) with Zink for OpenGL; on **Adreno**, Mesa's **Turnip** (Vulkan) with Zink.
- XFCE's compositor (xfwm4) costs redraws in software; turning it off (Settings › Window Manager Tweaks › Compositor) helps on slow devices.
- No sound yet; no clipboard sharing with Android yet (noVNC can do it; not wired up).
- Phones: the desktop is sized for readability (long side ≥1024), so on a small screen things are small; pinch-zoom isn't there.

## Sources (pinned)

| What | Where | SHA-256 |
|---|---|---|
| Debian 13 rootfs, aarch64 (proot-distro build 4.29.0) | `https://github.com/termux/proot-distro/releases/download/v4.29.0/debian-trixie-aarch64-pd-v4.29.0.tar.xz`, mirror `https://easycli.sh/proot-distro/debian-trixie-aarch64-pd-v4.29.0.tar.xz` | `3834a11cbc6496935760bdc20cca7e2c25724d0cd8f5e4926da8fd5ca1857918` |
| PRoot for arm64-v8a | `https://raw.githubusercontent.com/green-green-avk/build-proot-android/01f83b8841358450c78333d1b33ab30d4943bec4/packages/proot-android-aarch64.tar.gz` | `9629eb30cdf86e95c6ba681f8ab89c6fdaa9eca093d5577163513c99af5ca281` |
| … inside it: `root/bin/proot` → `libproot.so` | | `297abc237247682a84a3fd4283b28f69506502b4b852faf71fd726fb5d955d60` |
| … `root/libexec/proot/loader` → `libproot-loader.so` | | `de0ace1adc76ab0555b3dbbfbff0e78c1ac14017b255262707867aea70c49437` |
| … `root/libexec/proot/loader32` → `libproot-loader32.so` | | `1cf92eb379447ca19e13848be1e6dd57832c7d241a6fb8e2bad06df4523721ca` |
| PRoot for armeabi-v7a, x86_64, x86 (same commit: `proot-android-armv7a`, `-x86_64`, `-i686.tar.gz`) | | `3b91c720…819142cb`, `f7ecbd4e…5e06bf95`, `9b2993cf…259a84ed` (full values in `hosts/android/fetch-proot.sh`) |
| XZ for Java | Maven Central `org.tukaani:xz:1.10` (Gradle) | |
| noVNC 1.7.0 (2026-04-28) | `https://github.com/novnc/noVNC.git` tag `v1.7.0` = commit `63107bd06d9e1f6136ff21aeda8cd62cbf0d433e`; `core/**` and `vendor/pako/**` (56 files) | tree: `0e61b932064ec6ecffa0a4e4443fc2f447e7f1d1452ec2070f35c029f60bc334` (`sha256sum` of each file, sorted by path, then `sha256sum` of that list) |

- **The rootfs.** Termux's proot-distro 4.x installed Debian from release tarballs whose URL and SHA-256 are written in its plugin file (`distro-plugins/debian.sh`). The pinned pair is the one in proot-distro tags v4.30.1 through v4.34.2 (GitHub release asset), and v4.37.0 lists the same file and checksum on the easycli.sh mirror (v4.38.0, the last 4.x, moved to a newer `pd-v4.37.0` tarball, SHA-256 `9bd3b19ff7cd300c7c7bf33124b726eb199f4bab9a3b1472f34749c6d12c9195`, hosted only on easycli.sh; Hearth stays on the GitHub-hosted one) (read from the tags' git history; the release page itself couldn't be fetched from the build machine). proot-distro 5.x (2026) is a Python rewrite that pulls OCI images from Docker Hub instead, so the 4.29.0 Debian tarball is the last one its plugin files list. Tarballs have one top folder (`TARBALL_STRIP_OPT=1`), which Hearth strips.
  - Alternative, if both URLs ever disappear: Debian's own container images (debuerreotype, `github.com/debuerreotype/docker-debian-artifacts`, branch `dist-arm64v8`, a `rootfs.tar.xz` per suite; not checked here), with strip 0.
- **PRoot.** [green-green-avk/build-proot-android](https://github.com/green-green-avk/build-proot-android) (MIT build scripts; the binaries are what Another Term ships): PRoot from [green-green-avk/proot](https://github.com/green-green-avk/proot) tag `v0.15_release` (commit `b40accf05dfddbaf44758aa666ade79c57255005`; a fork of proot-me/PRoot with Termux's Android patches and more: `--link2symlink`, `--kill-on-exit`, `--change-id`, `-L`, ashmem/memfd), built with NDK 23.2 for API 21, with talloc 2.1.14 linked in statically, the loader unbundled. They need only Android's `libc.so`/`libdl.so` (`readelf -d`), so no Termux, no `libtalloc.so.2`. Prebuilt tarballs are committed in that repository, so the pin is a git commit plus SHA-256. Considered and not used: Termux's own `proot` package (dynamically linked against Termux's `libtalloc.so.2` and paths, and Termux's repository only keeps the newest version, so it can't be pinned); proot-me's releases (no Android builds with the Android patches).
- **CI** (`hosts/android/fetch-proot.sh`, run by `.github/workflows/android.yml` before the unit tests and the build): downloads the four tarballs from the pinned commit (https only), checks each SHA-256 (and the three arm64 files again after unpacking), and puts them in `app/src/main/jniLibs/<abi>/` (git-ignored). A mismatch fails the build with an annotation. Every ABI gets PRoot because an APK with native libraries for only some ABIs doesn't install on the others; Hearth Linux itself only runs on arm64-v8a. After building, CI checks the APK has `lib/arm64-v8a/libproot.so`, the loader and PRoot's license. For a local build run `./fetch-proot.sh` once in `hosts/android`.
- **noVNC** (`hosts/android/fetch-novnc.sh`, run by CI next to PRoot): clones the pinned tag, checks it's the pinned commit, takes `core/` and `vendor/pako/`, checks the tree checksum, and puts them in `app/src/main/assets/ui/novnc/` (git-ignored; served as `/ui/novnc/…` next to the page). After building, CI checks the APK has `assets/ui/novnc/core/rfb.js`, `core/websock.js`, `vendor/pako/lib/zlib/inflate.js` and the license files. For a local build run `./fetch-novnc.sh` once. (Not vendored: 750 KB of someone else's code is better pinned than copied into git.)

## Licenses (GPL compliance)

- PRoot is GNU GPL v2 (or later); talloc is GNU LGPL v3 (or later). Hearth **runs PRoot as a separate program** (its own process, started with `exec`, talking to Hearth only through its command line, environment, exit code and output): Hearth isn't linked with it.
- The APK carries the license texts in `assets/licenses/`: `proot-COPYING.txt` (PRoot's COPYING: authors and notice), `GPL-2.0.txt`, `LGPL-3.0.txt`, `GPL-3.0.txt`, and `README.txt` with where each binary comes from.
- **Exact source** of the PRoot binaries: <https://github.com/green-green-avk/proot/archive/refs/tags/v0.15_release.tar.gz> (commit `b40accf`), talloc <https://download.samba.org/pub/talloc/talloc-2.1.14.tar.gz>, built with the scripts at <https://github.com/green-green-avk/build-proot-android/tree/01f83b8841358450c78333d1b33ab30d4943bec4>. The Linux tab names the source too. Anyone distributing Hearth's APK should keep these links (or offer the source) with it.
- Debian is downloaded by the user's device from the pinned URL, not shipped in the APK; every package carries its own license in `/usr/share/doc/<package>/copyright` (XFCE, TigerVNC: GPL; installed by apt on the device, not shipped either).
- **noVNC** is MPL-2.0 (pako, in it: MIT; `core/crypto/des.js` keeps its own BSD-style notice). Hearth ships its files unmodified; MPL-2.0 is file-level copyleft, so that's allowed alongside Hearth's own code as long as the license and the source stay available: `assets/licenses/noVNC-LICENSE.txt` (noVNC's LICENSE.txt), `MPL-2.0.txt`, `pako-LICENSE.txt`, and `README.txt` with the exact source (<https://github.com/novnc/noVNC/tree/v1.7.0>). The Linux tab names it too.

## Limits (stage 1)

- **ARM64 only** (`arm64-v8a`): the pinned Debian is aarch64. Other devices see "Not available".
- **4 KB memory pages only.** The pinned PRoot and loader have 4 KB-aligned segments (`readelf -l`: `Align 0x1000`). Devices with a 16 KB-page kernel (Android 15+ option on some Pixels, and newer devices) can't run them; the tab says so (`sysconf(_SC_PAGESIZE)` check). Fix: build PRoot from source in CI with NDK r28+ (`-Wl,-z,max-page-size=16384`) — planned.
- **No GPU acceleration** yet (software rendering), **no sound** (no PulseAudio bridge), no interactive terminal in the tab (one command at a time, output only; programs that ask for input get none: use `-y`; the desktop's terminal is a real one).
- PRoot is slower than native for system-call-heavy work (it traces every call); CPU-bound programs run at full speed.
- No `systemd`, no services, no `sudo` for `hearth` (use "Run a command as root").
- 16 KB-page aside, Android 15/16's stricter rules aren't tested (see below).

## What's verified and what isn't

Verified here (no Android device or SDK on the build machine):

- The pins: the rootfs URL/SHA-256 pairs from proot-distro's git history (tags v4.29.0–v4.38.0); the PRoot tarballs' SHA-256 computed from the files in build-proot-android's git at the pinned commit, unpacked, inspected (`file`, `readelf`: aarch64 PIE linked only to `/system/bin/linker64`, `libc.so`, `libdl.so`; static loaders), and `fetch-proot.sh` run against them. The fork's options (`--kill-on-exit`, `--link2symlink`, `-L`, `-0`, `--change-id`, `--kernel-release`, `PROOT_LOADER`/`PROOT_LOADER_32`/`PROOT_TMP_DIR`/`PROOT_L2S_DIR`) read in its source.
- The Kotlin compiles (against stubs for the Android classes); the pure logic (tar safety incl. symlink/hardlink/`..` attacks, long names, PAX, damaged archives; safe delete; package names; PRoot argv and environment; /proc stand-ins; setup files) passes its unit tests (`LinuxLogicTest`) on a plain JVM, as root and as an unprivileged user.
- The page: `tools/ui-check.js` drives the Linux tab with a pretend Android host (install progress, a command and its output, Termux:X11 missing, a bad package name refused before it reaches the host, Remove asks first) and in the browser preview (`?preview=android`: a pretend Linux installs and runs a command).
- Stage 2, off-device:
  - `DesktopLogicTest` (JVM): DES (FIPS example, 200 random blocks against the JVM's DES), the VNC password file (`password` → `dbd83cfd727a1458`, random passwords against the JVM), the WebSocket accept key (RFC example), frame headers for 7/16/64-bit lengths, masked decoding (the RFC's "Hello"), round trips at 0…70000 bytes, refusals (unmasked, too big, bad control frames, reserved bits, bad 64-bit lengths, truncated), the handshake checks (token, origin, key, version, upgrade), and `WsBridge` end to end against a fake VNC server (both directions, a 300 KB burst in order, ping/pong, close, 403s, 1011 when the desktop is down, the port closed after stop); the desktop's size, script, modes, packages and apt's status lines.
  - **Real noVNC 1.7.0 in Chromium** (Playwright) → `WsBridge` on the JVM → a fake Xvnc that does RFB 3.8 with **VNC authentication checked against the password file Hearth writes** (decrypted the way Xvnc does): the handshake (with Chromium's real Origin header), auth OK, an 80 KB framebuffer update drawn on the canvas, the stick moving the pointer, A as button 1, the d-pad and typed text as keysyms — from an http and an **https** page (so `ws://127.0.0.1` from Hearth's https origin isn't blocked as mixed content, at least in Chromium).
  - The generated desktop script: `sh -n`, and run with stand-in `Xvnc`/`dbus-launch` (Xvnc first, the session once the X socket exists, Xvnc ended after).
  - `tools/ui-check.js`: the desktop rows, the install sheet and its progress, the view opening (raw controller on, start's arguments, waiting for `ready`, noVNC's URL and password), controller → keys/pointer/buttons/scroll/Enter on a stand-in RFB while Hearth's own navigation stays still, Hearth's keyboard over the view typing keysyms, the bar (Ctrl held for Tab, B closing it without clicking), touch (tap, drag, two-finger tap and scroll), reconnecting, Back to Hearth (controller back to normal), the desktop stopping by itself, Stop from the bar, Termux:X11 missing; and the browser preview (a pretend desktop installs, opens, clicks, stops).
  - The Kotlin compiles against stubs for the Android classes.

Not verified (needs a device):

- That this 2023 PRoot build runs on current Android (14, 15, 16): seccomp filters, `ptrace` rules and SELinux policy can change. Another Term, which ships these builds, is the closest reference that it works on recent Android (its target SDK wasn't checked). W^X: the loader is executed from the native library folder and maps Linux binaries from app data with `mmap(PROT_EXEC)`, which `untrusted_app` (API 29+) is still allowed to do (only `execve` of app-data files is denied) — believed right, not tested on 15/16.
- That `extractNativeLibs` puts executable files in `nativeLibraryDir` on every device (it does on stock Android; some vendor builds are stricter).
- Termux:X11 started from Hearth (`app_process` with its APK as the class path, from a non-Termux app) — read from its source, not tried. If it doesn't connect, Termux:X11's own "Preferences" and Hearth's logs (`x11:` lines in Settings › System › Save logs) are where to look. The fallback is a VNC server in the Linux (e.g. `tigervnc-standalone-server`) and a VNC viewer app.
- The rootfs's exact size (the plan assumes under 200 MB download, under 1 GB unpacked with some packages).
- The foreground service type `specialUse` passes Google Play review (it doesn't matter for GitHub builds).
- Stage 2 on a device: TigerVNC's Xvnc under PRoot (it's an ordinary X server; Termux users run it under proot-distro, but Hearth's exact command line hasn't run), XFCE on it, how fast it is on the Helio G99 in software; Android WebView allowing `ws://127.0.0.1` from the `https://appassets.androidplatform.net` page (Chromium does; WebView's `MIXED_CONTENT_NEVER_ALLOW` default is believed not to apply to loopback, which Blink treats as potentially trustworthy) and sending that origin; the WebView serving noVNC's ES modules through the asset loader (`text/javascript`); controller raw mode with real controllers (Android's axis numbering: right stick `AXIS_Z`/`AXIS_RZ`, triggers `AXIS_LTRIGGER`/`AXIS_RTRIGGER` or `BRAKE`/`GAS`). Start = Alt+F1 assumes XFCE's default shortcut.

## Next stages

1. ~~**Display in Hearth**~~ — stage 2 (VNC). Later maybe an X server drawing straight into a Hearth surface (no VNC copy).
2. **GPU (stage 3):** on **Mali** first (the user's device): **VirGL** (virglrenderer's server on Android's GLES, Mesa virgl in the Linux) or **Vulkan passthrough** (venus or a Vulkan wrapper over Android's driver) with Zink for OpenGL; on **Adreno**, Mesa **Turnip** (Vulkan) and Zink.
3. **x86 programs:** **Box64**/Box86 or **FEX** inside the Linux.
4. **Wine** (with Box64) for Windows programs, then **Steam** (x86, through Box64/FEX) and Proton.
5. **Hearth itself** inside its own Linux (the Hearth OS page and host), and PRoot built from source in CI (16 KB pages, newer fixes).
6. A real terminal (PTY) in the page, sound (PulseAudio over TCP to an Android sink), and Android storage with Hearth's own folder picker.
