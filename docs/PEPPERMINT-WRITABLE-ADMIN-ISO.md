# Peppermint writable admin test ISO

This branch is an isolated test path for the request to test Peppermint on a conventional, writable Debian system. It does not modify the stable edition, the existing bootc image, or the existing installer workflows.

## What this build is

The GitHub Actions workflow builds an amd64 Debian 13 (trixie) live ISO using Debian live-build, with XFCE and administrator/development/disk tools. It enables the standard Debian live installer path so a test machine can install a conventional Debian-based system rather than deploying the existing bootc image. The installed system uses normal dpkg/APT and a writable conventional root filesystem.

APT repository signature checking remains enabled. This is an admin/test image, not a production release.

## Important scope note

This is the **writable admin base ISO**, not yet a complete Hearth-session replacement for the existing OSTree/bootc Peppermint edition. The existing Hearth image build assumes bootc, OSTree boot entries, dracut and bootupd; those components cannot simply be deleted without replacing their boot and update responsibilities. This first ISO is intended to provide a fast, conventional Debian test environment while that integration is handled separately.

The build workflow uploads an ISO and SHA-256 checksum as a GitHub Actions artifact for 14 days. It does not publish a GitHub release, update the download website, or push an image to a registry.

## Build it

1. Open https://github.com/Dragongamer9/hearth-os/actions/workflows/peppermint-writable-admin-iso.yml.
2. Select **Run workflow** on branch `test/peppermint-writable-admin-iso`.
3. When the workflow completes, download the artifact named `peppermint-writable-admin-test-iso`.
4. Verify the ISO with the included `SHA256SUMS` before booting it.

## Test checklist

- [ ] ISO builds and its checksum verifies.
- [ ] Live environment boots in a UEFI virtual machine and on the target PC.
- [ ] Root filesystem in the live session is writable (live-session changes are not expected to persist after reboot).
- [ ] Network works and APT can refresh Debian indexes with signature verification enabled.
- [ ] Install a harmless package in the live session, e.g. `hello`, and run it.
- [ ] Use the included Debian installer to install to a disposable VM disk.
- [ ] After installation and reboot, confirm `findmnt /` shows a conventional writable filesystem and `apt install hello` persists after reboot.

Do not use the installer on a drive containing data you need to keep. Test in a VM or on a spare drive first.
