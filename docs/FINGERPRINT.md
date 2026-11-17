# Fingerprint sign-in (1.14)

Settings › Accounts › Sign-in options › Fingerprint: add and forget fingers, and turn the finger on for the lock screen
and for administrator prompts. **RED FLAG: a fingerprint is never the only way in; your password always works.**

## How it's built so nobody can be locked out

- **The PAM stacks are set up once, when the image is built**, by `hearth-fingerprint-pam install`
  (os/system_files/usr/libexec/nimbus), and checked there (`check`) and in CI (`.github/workflows/fingerprint-pam.yml`).
  Turning the finger on or off in Settings never touches a PAM file: it only adds or removes your account name in
  `/etc/nimbus/fingerprint/lock` or `/etc/nimbus/fingerprint/polkit` (root's, through `hearth-admin fingerprint-places`).
  A missing or broken list means "off".
- **common-auth stays password-only.** libpam-fprintd's own pam-auth-update profile (which would put the reader in front
  of every password: login, sudo, SDDM) is taken out by the image build, and the build fails if common-auth mentions
  pam_fprintd.
- **The lock screen** asks for a finger with its own PAM service, `hearth-fingerprint` (KWin's locker: the same file as
  `kde-fingerprint`), at the same time as the password, which keeps its own service. Either unlocks. The fingerprint
  service has no password line at all, and a few failed fingers make the reader rest until the next lock.
- **Administrator prompts (polkit)**: one included block, `hearth-fingerprint-polkit`, at the top of polkit-1. Line 1
  skips the reader for anyone not on the list; line 2 is `[success=done default=ignore] pam_fprintd.so max-tries=1
  timeout=10`: any failure (no reader, no fprintd, a wrong finger, a timeout, pam_fprintd missing) carries on to the
  password lines below it, unchanged. Mantel's prompt shows "Touch the fingerprint reader, or wait a moment and type
  your password".
- **The login screen** keeps the password only: signing in also unlocks your saved passwords (the keyring), which a
  finger can't, and SDDM would wait for the reader before taking a typed password.

## CI

`fingerprint-pam.yml` (Debian trixie, a test account, pamtester): the right password works and a wrong one fails for
polkit-1, login, sudo and other, with the fingerprint off, on (fprintd not running) and with pam_fprintd removed; the
fingerprint services never take a password; nothing else in /etc/pam.d changed; `uninstall` gives polkit-1 back byte for
byte. `test_net_signin_114.py` checks the files and the helpers. Everything with a real reader: needs testing on a device.
