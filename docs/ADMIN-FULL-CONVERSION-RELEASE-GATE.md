# Hearth OS Admin Edition — full-conversion release gate

This branch is the integration branch for **full Hearth OS parity on a writable Linux Mint base**. It is not a declaration that parity has been achieved.

## Non-negotiable requirements
1. No OSTree, rpm-ostree, immutable-root conversion, forced read-only mounts or image-only updates. Keep APT, dpkg, sudo, mutable /usr and /etc, kernel and driver management.
2. Hearth desktop experience must be the primary login choice, with a selectable, tested Cinnamon fallback and safe rollback.
3. Existing Hearth gaming host must work, with controller support and game library preserved.
4. AI Command Center must be packaged and installed, not merely linked. It needs an explicit connection and consent model.
5. Admin Center, reports, recovery guidance and settings must work from the Hearth interface; risky actions require approval.
6. Identity should be coherent across session, menu, launchers, background, and login; do not overwrite /etc/os-release to fake distribution identity.
7. A single tested in-place upgrade path from the V1 Mint test ISO and V2.x/V3.0 .deb packages; preserve user files and third-party packages.
8. Offline-capable recovery route, package rollback, and instructions to switch back to Cinnamon.
9. Validation: static checks, Debian package installation, clean Mint 22.3 VM install, login/logout, graphical app launch, AI integration, GPU/controller and upgrade/rollback smoke tests on real hardware.

## Release blocker
Do **not** advertise the next .deb as '100% Hearth OS' until every requirement above is implemented and validated. The current V3.0 build is only a foundation. This branch is isolated from stable main.
