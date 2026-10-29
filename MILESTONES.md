# Hearth OS milestones and rules

## RED FLAG: version numbers
The owner chooses every release/update version number. Never pick, bump, or change a version number
(Nimbus.Host.csproj, hosts/hearth-core/Cargo.toml, hosts/hearth-rs/Cargo.toml, hosts/electron/package.json,
hosts/linux/package.json, site/index.html, docs/ROADMAP.md headings, Beacon's Cargo.toml/tauri.conf.json)
without the owner naming the number first. If a release is ready and no number was given, ask.

## RED FLAG: an update must never stop a device from starting
A Hearth update left the owner's ROG Ally at a black screen with no way back (Peppermint's GRUB menu pointed at a
removed /ostree/boot.N). Anything that touches booting (bootloader/GRUB menu, hearth-bls-grub, kernel arguments,
initramfs, ostree/bootc deployment handling, the display manager/session start) must: keep the previous version
startable, be checked against a real boot-entry layout, and never ship without the owner's go. Devices must always be
recoverable without a keyboard (Hearth Recovery USB; automatic fallback to the previous version).
