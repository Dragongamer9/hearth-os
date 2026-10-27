# Hearth Mint — in-place update path (experimental)

## What this enables

The original writable Mint test ISO installs Hearth as `/usr/local/bin/hearth` with a desktop shortcut. It is **not** managed by apt, so the first ISO does not automatically upgrade Hearth-specific code.

This branch creates a versioned `hearth-os-app_<version>_amd64.deb` that installs the executable at `/usr/bin/hearth` and replaces the desktop launcher to point to the package-managed executable. The old `/usr/local/bin/hearth` binary is intentionally preserved as a fallback. Normal Linux Mint OS updates remain available through Mint Update Manager.

## Applying the first Hearth app upgrade on an installed V1 system

1. Install and boot the **original** Mint-based Hearth test ISO (after independent boot/install testing). Back up important data.
2. Download the artifact `hearth-mint-upgrade-deb-test` from the successful GitHub Actions run for `mint-hearth-upgrade-deb-test.yml` and extract it.
3. Inspect the SHA256SUMS and verify the package:
   `sha256sum -c SHA256SUMS`
4. Install the package (replace filename with the downloaded version):
   `sudo apt install ./hearth-os-app_<version>_amd64.deb`
5. Relaunch Hearth from the desktop menu. Check `dpkg -s hearth-os-app` and `command -v hearth`.
6. A later version of the same package can be installed via `sudo apt install ./hearth-os-app_<new-version>_amd64.deb`, without reinstalling the OS.

This is currently a **manual package upgrade**, not an automatic Hearth update service. A signed APT repository and Update Manager integration are planned; do not enable a source that has not yet been signed, verified, and published. Never pipe remote scripts into sudo for updates.

## Scope and limitations

- This package updates **only the Rust Hearth app**. It does not yet install the full existing Hearth OS services, AI Command Center, Gamescope integration, shell environment, or custom system configurations.
- The Mint base has normal apt/dpkg updates, independent of this package.
- The successful V1 ISO build proves that an ISO artifact was generated, not that boot/install/upgrade testing has passed.
- The .deb build in this branch is experimental until the workflow passes and installation is checked in a Mint VM.
- Compatibility: AMD64 Linux Mint 22.3 / Ubuntu 24.04 base. This is not for other distributions.
- No stable branch or release is changed by this experiment.

## Future V2 milestones

1. Prove live ISO boot, install, reboot, and Mint software updates in a VM.
2. Prove package installation and second package upgrade preserve user configuration and launch successfully.
3. Package Hearth Game Mode, desktop integrations, and AI Command Center as separately versioned components.
4. Introduce a GPG-signed, HTTPS-hosted apt repository (with narrowly scoped signing key and apt pinning) and select stable/test release channels.
5. Add opt-in Update Manager integration and backup/recovery safety checks.
6. Validate in-place upgrade from the original V1 ISO to the fully featured V2 system before promising it as seamless.
