# The microSD card for games (Hearth OS, next update)

The owner's Legion Go S (Nov 28, 2026): its microSD card from SteamOS (ext4, named USD00) was read by Linux but never
mounted, so Steam didn't see its games. SteamOS mounts its card by itself and tells Steam; Hearth now does the same.

- `usr/lib/udev/rules.d/70-hearth-game-card.rules`: a partition with a filesystem on the built-in mmc reader
  (mmcblk*p*) wants `hearth-game-card@<partition>.service`. USB drives stay with the desktop's own mounting.
- `hearth-game-card@.service`: no [Install], so never part of the PC's start; bound to the card, so taking it out
  unmounts it.
- `usr/libexec/nimbus/hearth-game-card`: mounts it at /run/media/<you>/<its name> (ext4 and btrfs keep their own
  owners; exFAT, FAT and NTFS mounted as you; nosuid, nodev), then asks Steam to add it
  (steam://addlibraryfolder, as SteamOS's sdcard-mount) once Steam runs, for up to 10 minutes. Anything wrong is
  logged and nothing else changes.

Until this update: `udisksctl mount -b /dev/mmcblk0p1`, then Steam › Settings › Storage › Add Drive.
