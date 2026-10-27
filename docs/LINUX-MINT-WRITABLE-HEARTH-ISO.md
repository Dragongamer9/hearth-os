# Linux Mint + Hearth writable test ISO

This branch replaces the earlier Debian/Peppermint experiment with a **real Linux Mint Cinnamon ISO remaster**. It preserves Linux Mint's standard APT/dpkg package management and conventional writable installed filesystem; it does not use OSTree, rpm-ostree, bootc, or image-based deployment.

## Build

Open Actions → **Linux Mint writable Hearth test ISO**, select branch `test/linux-mint-writable-hearth-iso`, and run it with:
- the direct HTTPS download URL for an official Linux Mint Cinnamon amd64 ISO;
- the official SHA-256 hash of that exact ISO, from the Linux Mint download verification page.

The workflow checks the hash before changing the ISO, compiles the Rust Hearth application from this repository, adds it to the Mint live filesystem, and uploads a private-to-Actions build artifact for 14 days. It does not publish a release or modify main.

## Scope and limitations

This is an **initial Mint integration**, not yet a complete replacement for the existing Hearth OS image. The Hearth executable and desktop launcher are included, but the original Hearth Game Mode, autologin, Quickshell desktop, hardware-specific gaming stack, and integrated update UI are **not** yet installed or validated on Mint. The launcher depends on compatible runtime libraries in the selected Mint release.

The ISO remaster and installation must be tested in a VM before writing it to a real device. The live session itself uses Mint's normal squashfs/overlay arrangement; that is distinct from an installed Mint filesystem, which remains writable. Do not interpret a read-only live ISO as an immutable installed system.

## Validation required

1. Build completes and the ISO boots in BIOS and UEFI VMs.
2. Mint's installer completes on a disposable virtual disk.
3. Reboot into the installed system and verify `sudo apt update`, `sudo apt install`, `dpkg -i`, and persistence of edits under `/etc`.
4. Launch Hearth from the application menu and confirm rendering.
5. Only then integrate full Hearth sessions, branding, desktop components, and game-mode dependencies.

No ISO or successful boot is claimed until those checks pass.
