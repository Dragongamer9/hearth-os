# AppImage Maker

`hosts/hearth-appimage` (`hearth-appimage`, menu name **AppImage Maker**): turns apps packaged for other systems
into AppImages that run on Hearth OS as one file. Rust (Tauri window, the work in `src/convert/`).

## What it takes

- `.deb` (Debian/Ubuntu), `.rpm` (Fedora, openSUSE, RHEL), tarballs (`.tar`, `.tar.gz`/`.tgz`, `.tar.xz`, `.tar.zst`,
  `.tar.bz2`) and `.zip`. Not AppImages (they already are one).
- Drop them on the window, pick them (Ctrl+O, X on a controller), or `hearth-appimage FILE...` / Open with.
- On the command line: `hearth-appimage convert [-o DIR] [--menu] [--no-fetch] [--no-libs] [--json] FILE...`.

## How a conversion goes

1. **Unpack** in Rust: ar + tar for .deb, the rpm headers and its cpio payload for .rpm, every compression
   (gzip, xz, zstd, bzip2, lzma). Nothing in the package is run (no maintainer scripts, no dpkg/rpm). Paths that
   climb out (`..`), writing through a link out of the package, setuid bits and device files are refused.
2. **Fix paths**: links that pointed at `/opt/...` or `/usr/...` are made relative; launcher scripts in the bin folders
   that start `/opt/<app>/...` (the way Electron apps do) are pointed at `$APPDIR/opt/<app>/...`.
3. **Find the app**: the package's menu entry (the shown one, named like the package), else its main program guessed
   (bin folders, the top of a tarball, the biggest program). Its icon from the icon theme folders or pixmaps, else
   Hearth's plain package icon.
4. **Libraries**: every program and library in it is read as ELF (DT_NEEDED and GLIBC_ needs; never run, so not
   ldd). What the app doesn't bring is copied from this computer; what this computer doesn't have either is
   looked up in Fedora's repositories (`dnf repoquery --whatprovides`, `dnf download`: no root, nothing installed)
   and taken out of that rpm. The base system is never bundled (glibc, libgcc_s, the GPU drivers' libraries, X11 and
   Wayland, ALSA/PipeWire/Pulse, fontconfig/freetype, D-Bus, udev): those must come from the computer it runs on.
5. **AppRun** sets `LD_LIBRARY_PATH` to the library folders inside, `PATH`, `XDG_DATA_DIRS` (and GSettings/Qt
   plugin folders when the app has them), and starts the program with the menu entry's fixed arguments.
6. **Pack**: AppImage's type 2 runtime + a zstd squashfs written in Rust (backhand). The runtime is
   `/usr/share/hearth-appimage/runtime-<arch>` in the image (base-anatase.yml downloads it), else downloaded once
   into `~/.cache/hearth-appimage`.
7. **App menu** (on by default in the window): `~/.local/share/applications/hearth-appimage-<id>.desktop`, with
   `TryExec` on the AppImage so the entry hides itself if the file is deleted.

AppImages go to `~/Applications` unless you pick another folder. Without FUSE they unpack themselves to run
(`APPIMAGE_EXTRACT_AND_RUN=1`, set by Open when `/dev/fuse` or fusermount isn't there); the Fedora image installs fuse3.

## What it tells you

Each conversion reports warnings when something may not work: a package for another machine type (arm64 on a PC),
a program needing a newer glibc than this computer has, libraries found nowhere, Python/Java/Node/Perl apps (the
interpreter and its modules aren't put inside), and packages depending on systemd, dkms, udev or polkit (services
and drivers can't be set up from an AppImage).

## Saved

`~/.config/hearth-appimage/settings.json` (where AppImages go, fetch libraries, add to menu: merged on write,
atomic, .bak) and `made.json` (the AppImages made: written only by the Rust side). Icons kept for the menu entries:
`~/.local/share/hearth-appimage/icons`.

## Tested (appimage-maker.yml)

Rust tests (paths, links, cpio, control files, script rewriting, ELF reading); the command-line build inside a bare
`fedora:latest` container converting an Electron-style test .deb (libogg fetched from Fedora, absolute link,
launcher script, menu entry, icon), a tarball app with its own lib/, Ubuntu's GNU Hello .deb and Fedora's hello
.rpm, each AppImage run and its insides checked; a Fedora-made AppImage run on Ubuntu; the window converting two
packages given to it; the page in Chromium with screenshots.
