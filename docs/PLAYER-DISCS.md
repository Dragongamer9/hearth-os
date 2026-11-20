# Hearth Player: CDs and DVDs (planned for 1.16)

The owner, Nov 20, 2026: "add the ability to watch movies from a CD or DVD drive in the Hearth player". Built on
`feature/player-discs`; it needs testing with a real drive.

## What you get

- **A disc goes in**: Plasma's "a disc was inserted" pop-up (Disks & Devices) offers **Play disc in Hearth Player**
  for DVDs, Video CDs and music CDs (`os/system_files/usr/share/solid/actions/hearth-player-*.desktop`). If Hearth
  Player is already open it says so itself ("A disc was inserted: …" with **Play disc**), its start screen has a
  **Play disc** button while a disc is in, and the drives are in its Open view. Hearth Files lists the CD/DVD drive
  (with or without a disc); opening a film or music disc there starts Hearth Player, a data disc opens as a folder.
- **DVD-Video**: the disc screen lists its titles (the film is marked and is the big **Play the film** button) with
  their chapters, sound tracks and subtitles. **Carry on** picks up where you stopped (remembered per disc). Sound and
  subtitles can be picked before playing or while playing (Y on a controller, A on a keyboard). The disc's own menu
  isn't run: the backend reads the disc (libdvdread) but doesn't run menus, so this screen is the menu. Camera angles:
  the first plays.
- **Video CD and Super Video CD**: their parts, played and seekable.
- **Music CDs**: the songs, with names from the disc's CD-Text, else looked up online (MusicBrainz, then CDDB/gnudb)
  by the disc's ID ("Look up names online" asks again). Only the disc's table of contents is sent.
- **Data CDs and DVDs**: their folder opens in the Open view (videos and songs on it play as usual).
- **Blu-ray**: not played. The disc screen says so plainly ("This is a Blu-ray disc. Hearth Player plays DVDs, Video
  CDs and music CDs, but not Blu-ray discs.").
- **Eject**: on the disc screen, in the player's buttons while a disc plays, X on a controller on the disc screen, and
  a remote's or keyboard's Eject key.
- **Controller and remote**: the same as the rest of the player (D-pad moves, A chooses, B goes back), plus Select (a
  remote's Menu key, or D) for the disc menu, LB/RB for chapters, Y for sound and subtitles. "Controller buttons" on
  the disc screen (or ?) shows the whole list.

## Locked DVDs (libdvdcss): asked, never added by itself

Most shop-bought DVDs are locked against copying (CSS). Unlocking them needs **libdvdcss**, which Fedora doesn't ship
for legal reasons; it's in RPM Fusion's "free-tainted" repository. **It is not in the Hearth OS image** (the Fedora
build removes it if anything ever pulls it in) and nothing adds it without you. The owner approved this plan (Nov 20,
00:36).

When a DVD's film sectors are scrambled (the PES scrambling bits libdvdcss clears after unscrambling a sector), the
disc screen shows:

> **This DVD is locked**
> This DVD is locked against copying (most films from a shop are), so it needs DVD decoding to play. Hearth OS can't
> include DVD decoding for legal reasons, but you can add it yourself: it comes from RPM Fusion, a well-known add-on
> source for Fedora. Whether it's allowed to use where you live is up to you.
> Adding it asks for an administrator's password, needs the internet and takes about a minute. Nothing is added unless
> you say yes.
> [Add DVD decoding] [Eject]

**Add DVD decoding** asks once more (what it is, where it comes from, that it needs a password) and only after
**yes** runs `pkexec hearth-admin dvd-decoding-add`, which always asks for an administrator's password
(`49-hearth-admin.rules`). `hearth-dvd-decoding add` then:

1. writes `/etc/yum.repos.d/hearth-dvd-decoding.repo`: only RPM Fusion's free-tainted part, `gpgcheck=1` with the RPM
   Fusion key the image already has (from its own switched-off `rpmfusion-free.repo`; nothing is changed without a
   key);
2. adds `libdvdcss` with `rpm-ostree install --apply-live` (layered on the image: the version before stays in the boot
   menu, updates keep it because hearth-admin updates layered systems with rpm-ostree), or dnf on a plain Fedora;
3. on failure takes the repository file out again: nothing changed.

After that the disc opens again by itself and plays. **Remove DVD decoding** (on a DVD's disc screen) takes both out.
With DVD decoding installed and a disc still locked, the note says it couldn't be unlocked (drive region, a damaged
disc). On the Debian edition the button isn't offered ("isn't offered on this edition of Hearth OS yet").

## How it works (for the next person)

- `hosts/hearth-player/src/disc.rs`: drives (`/sys/block/sr*`, udev's `ID_CDROM_MEDIA_*`), the disc's top folder
  (ISO 9660 off the disc: VIDEO_TS, MPEGAV, MPEG2, BDMV), SCSI through SG_IO (READ TOC, CD-Text, READ CD; udev gives the
  person at the computer the drive), `.cue`/`.bin` and `.iso` images, MusicBrainz and CDDB disc IDs, a music CD track as
  a WAV file and a VCD track's MPEG (Mode 2 sectors read raw), the tray, and **disc events from udev's netlink socket**
  (a thread asleep in `recv`: nothing is checked over and over; CI measures the CPU at rest on a disc screen).
- `src/dvd.rs`: the IFO files (titles, chapters, cells, sound and subtitle streams, the subtitles' colours), libdvdread
  loaded at run time (`dlopen`; without it a VIDEO_TS folder is read from its files), the scrambled-sector check, and
  seeking (the cell's share, then the next VOBU's own time from its navigation pack).
- `src/discplay.rs`: opening a disc and play plans. Music CDs play as WAV from the local server. DVD titles and VCD
  parts go through ffmpeg (`-f mpeg` in, Matroska out as it goes: the picture as it is, the sound as Opus or AAC);
  seeking starts a new stream at the right sector. DVD subtitles are pictures, so with subtitles on ffmpeg draws them
  into the picture (encoded again: H.264, MPEG-2 without it); a picture the web view won't play is encoded once more
  the same way.
- The page (`ui/app.js`, "discs" section): the disc screen, `player://disc` events, resume, and the controller and
  remote keys.
- The Fedora image (`os-bases/anatase/build.sh`): libdvdread, libdvdnav, udisks2; never libdvdcss.
- Hearth Files (`hosts/linux/desktop/shell/hearth-files`): `optical_drives`, `disc_in`; a film or music disc's "open"
  starts `hearth-player --disc /dev/srN`.

## Tests

- `cargo test` (player.yml): IFO parsing on an IFO written in the test, CD-Text, CUE sheets, disc IDs (checked against
  Python's hashlib), uevents, WAV/VCD files, ffmpeg command lines; and `tests/discs.rs` on the fixture discs CI makes
  (`.github/scripts/player-disc-fixtures.sh`: a DVD-Video ISO with two titles, chapters, two sound tracks and
  subtitles, the same DVD locked, a Video CD, a music CD with CD-Text, a Blu-ray folder, a data folder), including
  ffmpeg's Matroska from a DVD title and the VCD.
- The app on the fixtures (`.github/scripts/player-disc-drive.sh`): the DVD, VCD and music CD really play (position
  over MPRIS), a DVD seek, every disc screen pictured (run player.yml with "shots" ticked).
- `hosts/linux/test/test_player_discs.py`: libdvdcss never in an image, the repository file, hearth-admin and the
  password rule, the Plasma actions, Hearth Files' drives.

## Not done yet

- Running the disc's own DVD menus (libdvdnav's menus need a renderer of their own): the titles and chapters screen
  stands in for them.
- Hearth's own Game Mode notification for a disc (the archived web Game Mode); Steam Game Mode shows no pop-up.
- Interlaced DVDs play as they are (combing on some TV discs) unless subtitles are on (then they're smoothed out).
