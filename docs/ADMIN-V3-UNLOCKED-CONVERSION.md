# Hearth OS Admin Edition V3.0 — unlocked conversion (experimental)

## Design rules
- **Writable system**: retain Linux Mint's APT/dpkg packaging, mutable root filesystem, user-controlled settings and normal kernel/driver management.
- **No OSTree**, rpm-ostree, immutable root, forced read-only partitions or mandatory image-based updates.
- **No silent system takeover**: keep Cinnamon, the display manager, bootloader, /etc/os-release, repositories and existing users unchanged.
- **In-place .deb**: versioned hearth-os-app package updates the Hearth UI and tools. Reboot is not normally required.
- **Reversible desktop activation**: as the regular desktop user run `hearth-convert enable`, then log out and back in to start Hearth Welcome automatically. Run `hearth-convert disable` to reverse. `hearth-convert status` inspects the installation.
- Optional wallpaper: `hearth-admin-desktop apply`; reverse with `hearth-admin-desktop restore`.
- System diagnostics remain read-only; no automated repair without explicit design and testing.
- The separate Hearth AI Command Center is **not bundled** in this package. Its Welcome shortcut works only if a compatible app is installed.

## Installation
Download the CI artifact containing `hearth-os-app_0.6.56-6_amd64.deb` and `SHA256SUMS`.
Back up your personal data and verify the checksum. On the original writable Mint test install:
```sh
sha256sum -c SHA256SUMS
sudo apt install ./hearth-os-app_0.6.56-6_amd64.deb
hearth-convert status
hearth-convert enable
```
No `sudo` for `hearth-convert` or wallpaper settings.

## Current scope / not yet completed
This V3.0 package provides the existing Hearth app, Welcome Center, diagnostic tools, branding and opt-in login launch. It is **not yet 100% functional parity** with the independent Hearth OS project: desktop shell replacement, AI bundling, unified settings, full game compatibility and physical hardware validation remain future work. Do not call the conversion complete until those features are built and tested.

CI tests packaging and simulated Ubuntu 24.04 installation, **not** an end-to-end Mint 22.3 laptop conversion.
