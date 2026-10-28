# Hearth OS Peppermint/Debian — writable APT edition

**Status: architecture migration in progress; NOT a bootable ISO yet.**

The existing `os-bases/peppermint/` implementation is an OSTree/bootc image and cannot be made writable merely by removing a package. Its bootloader, initramfs, dpkg database relocation, filesystem links, installer, updater and recovery design all depend on bootc/OSTree.

## Target
- Debian 13 trixie / Peppermint-compatible userspace, normal mutable ext4 or Btrfs root, /usr and /etc writable under root privileges.
- APT/dpkg with Debian repositories; normal Debian kernel, GRUB, initramfs-tools (or standard Debian dracut integration).
- Hearth gaming host, Mantel desktop (labwc + Quickshell), gamescope, optional Xfce fallback, system tools and controller integration.
- Offline installer using Debian live-build/Calamares or equivalent; install via normal partitioning and GRUB, NOT `ostree admin deploy` or `bootc install`.
- APT-based Hearth package repository with signed metadata; normal Debian upgrades and documented rollback/recovery.
- No OSTree, bootc, composefs immutable root, rpm-ostree, read-only root or relocated dpkg database.

## Migration sequence and blockers
1. Extract the Hearth app, Mantel desktop, sessions and assets from the current image build into independently installable Debian packages. Avoid copying OSTree-specific units or scripts.
2. Rework initramfs/kernel and signed GRUB boot path for conventional Debian; verify EFI and legacy BIOS behavior in VMs.
3. Replace image deployment installer with a conventional Debian installer and verify preserving existing data and EFI partitions.
4. Replace bootc upgrade/rollback UI with APT updates and recovery mechanisms; disable all OSTree-dependent services.
5. Build bootable live ISO, install in VM, test boot, login, gaming and network, package upgrade and recovery.
6. Only after VM and physical testing consider a stable release. Keep the existing stable image untouched until then.

## Existing bootc-specific implementation (do not reuse as-is)
- `os-bases/peppermint/Containerfile`
- `os-bases/peppermint/builder.sh`
- `os-bases/peppermint/build.sh`
- `.github/workflows/base-peppermint.yml`
- OSTree-based installer, bootloader scripts, immutable-root dracut modules and `bootc upgrade` hooks.

**Do not run `apt remove ostree bootc` on an installed image.** Use a fresh writable install or a separately engineered migration with backups.
