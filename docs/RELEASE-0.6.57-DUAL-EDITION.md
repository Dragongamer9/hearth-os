# Hearth OS 0.6.57 — dual-edition in-place update release gate

**Status: preparation, NOT published.** Version numbers have been aligned on this test branch. Do not tag or promote until verified.

## Update from 0.6.56 within Hearth Game Mode / Settings
The existing UI update action must detect a newer system image and invoke the edition-specific update mechanism, show download/install progress, and offer a restart when staged. It must not reinstall the OS or erase home data.

### Anatase (Fedora/bootc)
- Keep the current bootc-based upgrade and rollback pathway for installed 0.6.56 systems.
- Build 0.6.57 as an update to the **same image reference** already tracked by those installations.
- Verify installed image digest changes, new version appears in the running system after reboot, and rollback works.
- Publish only after VM/device validation and explicit release approval.

### Debian / Peppermint
- The existing 0.6.56 Peppermint build is ALSO a bootc/OSTree image; it cannot directly become a normal APT-managed writable installation via an image update.
- To update **existing 0.6.56 Peppermint bootc installations in-place**, 0.6.57 must retain the bootc image update pathway. This can be an interim compatibility update, but **does not remove immutability**.
- The future writable Debian/Peppermint edition requires its own installer and migration plan. Do not advertise an in-place bootc→APT conversion until separately tested.
- Do not use the Anatase image reference for Peppermint. Verify the existing Peppermint image reference and publish the new image to that exact update channel.

## Before releasing
1. Confirm the Game Mode Settings > Updates bridge actually checks for updates, starts them with authorization, handles errors, and surfaces restart readiness for both editions.
2. Build both edition images on CI from the same 0.6.57 commit. Keep testing tags separate from stable tags.
3. Test an actual 0.6.56→0.6.57 upgrade and rollback in each edition, not merely a clean 0.6.57 image boot.
4. Validate no unintended changes to user's files, Steam libraries, controller configuration, and session selection.
5. Do not change `main`, push release tags, or promote `:latest` without approval.

## Version source
The image workflows read `hosts/windows/Nimbus.Host/Nimbus.Host.csproj` for the release version; Rust host/core package manifests must match it. Changing the version alone does not create a working in-app update.
