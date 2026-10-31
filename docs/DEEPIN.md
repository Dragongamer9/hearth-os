# Hearth Deepin

**Status: not possible on Hearth OS's editions.** Settings and the sign-in screen list "Hearth Deepin (not available
on this edition)", greyed out, and a `HEARTH_DESKTOP=deepin` line in `session.conf` simply opens Hearth Classic.

Hearth only ships a desktop when it can be had properly from the edition's own distribution, without untrusted
third-party repositories (docs/COSMIC.md, docs/PLASMA.md). Deepin (DDE) can't be, on either edition:

| Edition | What's there | |
|---|---|---|
| Anatase (Fedora 45) | Only `deepin-pdfium`. Fedora retired its Deepin desktop packages (FESCo's decision, for security concerns raised after openSUSE dropped Deepin, unreachable maintainers and no maintenance). | checked in CI with `dnf repoquery 'deepin*' 'dde*' startdde` |
| Peppermint (Debian trixie, with backports) | A few Deepin apps and libraries (`dde-calendar`, `dde-store`, `dde-qt5integration`, `dde-account-faces`, `qt5dxcb-plugin`, DTK libraries), but no desktop: no `startdde`, `dde-session`, `dde-session-shell`, `dde-dock`, `dde-launcher`, `dde-shell`, `dde-control-center`, `dde-daemon`, `deepin-kwin` or `treeland`. Debian has been removing its Deepin packages (`dde-network-utils` left unstable and testing in September 2025). | checked in CI with `apt-cache policy` |

Building DDE from source in CI isn't a proper alternative: it's dozens of repositories (DTK, the shell, the session,
Deepin's KWin, the Go daemon, control center, application manager...) that upstream builds for deepin and UOS, it's
X11-based (DDE 5) or a new Wayland compositor (Treeland) neither distribution carries, and the security findings that
made openSUSE and Fedora drop it would be Hearth's to carry.

The probe that checked this was `.github/workflows/desktop-probe.yml` (removed once answered; its results are in this
page). If Fedora or Debian bring a Deepin desktop back, Hearth Deepin can be added the way Hearth COSMIC was: a
selector entry, a session case with the fallback, a hearth-pad mode and a CI test.

App kinds (docs/APP-KINDS.md): no Deepin apps are installed. Deepin's would be `dde-file-manager`, `deepin-terminal`
and `deepin-editor`; none is installed on either edition.
