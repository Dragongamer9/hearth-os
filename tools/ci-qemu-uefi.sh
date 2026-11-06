#!/bin/bash
# CI only (installer-test.yml): installs qemu and the UEFI firmware (OVMF) on the GitHub runner and copies the
# firmware to ./code.fd and ./vars.fd. The runner's package lists can be stale or empty (apt-get then exits 100
# with "Unable to locate package" or a 404), so the lists are refreshed first; any failure becomes an ::error
# annotation with apt's own words, since the job logs can't always be read.
set -u
err=$(mktemp)
if ! sudo apt-get update -qq >/dev/null 2>"$err"; then
  echo "::warning title=apt-get update::$(tail -n 5 "$err" | tr '\n' '|' | cut -c1-1500)"
fi
if ! sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq qemu-system-x86 ovmf >/dev/null 2>"$err"; then
  echo "::error title=qemu/OVMF install failed::$(. /etc/os-release; echo "$PRETTY_NAME"): $(tail -n 8 "$err" | tr '\n' '|' | cut -c1-2500)"
  exit 1
fi
rm -f "$err"
pick() { for f in "$@"; do [ -f "$f" ] && { echo "$f"; return; }; done; }
code=$(pick /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd /usr/share/ovmf/OVMF_CODE_4M.fd /usr/share/qemu/OVMF_CODE_4M.fd)
vars=$(pick /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd /usr/share/ovmf/OVMF_VARS_4M.fd /usr/share/qemu/OVMF_VARS_4M.fd)
if [ -z "$code" ] || [ -z "$vars" ]; then
  echo "::error title=No UEFI firmware::OVMF code/vars not found; the ovmf package has: $(dpkg -L ovmf 2>/dev/null | grep -E '\.fd$' | tr '\n' ' ' | cut -c1-1500)"
  exit 1
fi
cp "$code" code.fd && cp "$vars" vars.fd
echo "::notice title=UEFI firmware::$code + $vars ($(qemu-system-x86_64 --version | head -1); kvm: $([ -e /dev/kvm ] && echo yes || echo no))"
