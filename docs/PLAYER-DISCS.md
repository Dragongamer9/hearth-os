# Hearth Player: CDs and DVDs (planned for 1.16)

The owner, Nov 20, 2026: "add the ability to watch movies from a CD or DVD drive in the Hearth player". Built on
`feature/player-discs`; it needs testing with a real drive.

## What you get

- **A disc goes in**: a notice "A disc was inserted" with **Play disc** and **Eject** wherever Hearth shows notices:
  Mantel's and Firehold's notifications, Plasma's, and in Steam Game Mode a Hearth Link toast (pressing it plays the
  disc; Quick Access › Hearth Link has both buttons). It comes from `hearth-disc-notify` (a user service asleep on
  udev's netlink socket: no polling), and isn't sent while Hearth Player is open. Plasma's own "a disc was inserted"
  pop-up (Disks & Devices) also offers **Play disc in Hearth Player** for DVDs, Video CDs and music CDs
  (`os/system_files/usr/share/solid/actions/hearth-player-*.desktop`). If Hearth
  Player is already open it says so itself ("A disc was inserted: …" with **Play disc**), its start screen has a
  **Play disc** button while a disc is in, and the drives are in its Open view. Hearth Files lists the CD/DVD drive
  (with or without a disc); opening a film or music disc there starts Hearth Player, a data disc opens as a folder.
- **DVD-Video, the disc's own menu**: a DVD opens in its own menu, as on a DVD player (libdvdnav runs it: its
  buttons, highlights and jumps). The D-pad, the arrow keys or a remote move between its buttons, A / Enter / OK press
  one, the mouse points and clicks; the picked button has Hearth's ember ring. A button that plays the film hands it
  to the player (seeking, sound, subtitles, Carry on); when the film ends the menu comes back. B or Back goes to the
  titles list; the remote's Menu key brings the menu back while a film plays. The menu's picture comes as JPEG frames
  (so a press answers at once; a still menu's picture is one frame, decoded once), and its own music and button
  sounds play as they do on a DVD player (Opus, from a second ffmpeg that keeps only the sound), at the player's
  volume. The sound stops the moment a title starts or the menu closes. Where the menu can't run (no libdvdnav, a locked disc,
  a disc without menus), the titles list stands in.
- **DVD-Video, the titles list**: the disc's titles (the film is marked and is the big **Play the film** button) with
  their chapters, sound tracks and subtitles, and **The disc's own menu**. **Carry on** picks up where you stopped
  (remembered per disc). Sound and subtitles can be picked before playing or while playing (Y on a controller, A on a
  keyboard). Camera angles: the first plays.
- **Interlaced DVDs** (TV recordings) are always smoothed out (deinterlaced) while they play: the player looks at the
  title's pictures when it starts it, and only an interlaced title is encoded again (films aren't touched). The cost is
  only while it plays.
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
- `src/dvdnav.rs`: the disc's own menu: libdvdnav (`dlopen`), the buttons from each navigation pack's PCI, the moves and
  presses, still menus waiting on a condition variable (nothing runs meanwhile), and ffmpeg's JPEG frames (`-re`, the
  menu's subpicture drawn in). The menu's sound: the same sectors go to a second ffmpeg (sound
  only, Opus in Ogg, the page's <audio>), from where the menu's picture started; it ends when the menu stops or a title
  starts. A button that goes to a title ends the menu stream and tells the page which.
- `os/system_files/usr/libexec/nimbus/hearth-disc-notify` (+ `hearth-disc-notify.service`): the notice. In Steam Game
  Mode `hearth-discord notify-bridge` passes its buttons to the Hearth Link plugin and its presses back.
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

## At rest

Hearth Player costs nothing at rest (CI measures it on the start screen, a disc screen and a DVD's still menu): the
controller reader (`ui/pad/hearth-gamepad.js`, every app's) used to ask for every frame for as long as an app was open
(1.2-1.5 % of a core doing nothing); now it does only while the controller is in use (see docs/CONTROLLER-FOCUS.md).

## Not done yet

- Menus on 352-wide discs are placed as 720-wide ones.
