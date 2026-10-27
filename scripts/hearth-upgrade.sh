#!/usr/bin/env bash
# Hearth OS upgrade helper: offline, owner-approved package updates only.
set -euo pipefail
usage() {
  cat <<'HELP'
Hearth OS upgrade helper (test)
  hearth-upgrade status
  hearth-upgrade install /path/to/hearth-os-app_VERSION_amd64.deb

Updates Linux Mint system packages via Mint Update Manager.
This helper does not fetch packages, add repositories or run unattended.
HELP
}
case "${1:-}" in
  status)
    echo "Hearth package:"
    dpkg-query -W -f='  ${Package} ${Version} (${Status})\n' hearth-os-app 2>/dev/null || echo "  Not managed by dpkg yet (original V1 installation)."
    echo "Hearth executable:"
    if [ -x /usr/bin/hearth ]; then echo "  /usr/bin/hearth (package-managed)"; elif [ -x /usr/local/bin/hearth ]; then echo "  /usr/local/bin/hearth (original V1)"; else echo "  Missing"; fi
    ;;
  install)
    if [ "$#" -ne 2 ]; then usage; exit 2; fi
    pkg="$(realpath -- "$2")"
    test -f "$pkg" || { echo "Package does not exist" >&2; exit 1; }
    package_name="$(dpkg-deb --field "$pkg" Package)"
    package_arch="$(dpkg-deb --field "$pkg" Architecture)"
    package_version="$(dpkg-deb --field "$pkg" Version)"
    if [ "$package_name" != hearth-os-app ] || [ "$package_arch" != amd64 ] || ! [[ "$package_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
      echo "Refusing unexpected package: $package_name ($package_arch, $package_version)" >&2
      exit 1
    fi
    installed="$(dpkg-query -W -f='${Version}' hearth-os-app 2>/dev/null || true)"
    if [ -n "$installed" ] && ! dpkg --compare-versions "$package_version" gt "$installed"; then
      echo "Refusing same-version/downgrade: installed $installed, candidate $package_version" >&2
      exit 1
    fi
    echo "Candidate: $package_name $package_version ($package_arch)"
    echo "SHA256: $(sha256sum "$pkg" | cut -d' ' -f1)"
    echo "Only install packages downloaded from a source you trust and verify its checksum first."
    printf 'Install this exact package? Type INSTALL to continue: '
    read -r answer
    [ "$answer" = INSTALL ] || { echo 'Canceled'; exit 1; }
    exec sudo apt install -- "$pkg"
    ;;
  *) usage; [ "$#" -eq 0 ] && exit 0 || exit 2 ;;
esac
