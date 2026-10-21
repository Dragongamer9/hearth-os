#!/usr/bin/env bash
# Hearth Linux: puts PRoot into the app as native libraries (app/src/main/jniLibs/<abi>/libproot.so,
# libproot-loader.so, libproot-loader32.so), from pinned, checksummed downloads. CI runs it before building
# (.github/workflows/android.yml); run it yourself before a local build. The binaries are never kept in git.
#
# Source of the binaries: green-green-avk/build-proot-android (the PRoot builds Another Term uses): PRoot from
# green-green-avk/proot v0.15_release with talloc 2.1.14 linked in statically, built with the Android NDK, so they need
# nothing but Android's own libc. Each ABI's tarball is pinned by commit and SHA-256. Licenses and the exact source:
# app/src/main/assets/licenses/README.txt and docs/ANDROID-LINUX.md.
#
# Every ABI the APK supports gets PRoot (not only arm64-v8a): an APK with native libraries for some ABIs only won't
# install on the others. Hearth Linux itself runs on arm64-v8a only (the pinned Debian is aarch64).
set -euo pipefail

COMMIT=01f83b8841358450c78333d1b33ab30d4943bec4
BASE="https://raw.githubusercontent.com/green-green-avk/build-proot-android/$COMMIT/packages"
# abi  tarball-arch  sha256
PINS="
arm64-v8a   aarch64 9629eb30cdf86e95c6ba681f8ab89c6fdaa9eca093d5577163513c99af5ca281
armeabi-v7a armv7a  3b91c7200adf60c5a61707cf1c7540f3fd16cc97357bb17fea3f4b38819142cb
x86_64      x86_64  f7ecbd4ee1041eefc2d843aa9c080a583fd6c3bb010b3bb7b7522ba05e06bf95
x86         i686    9b2993cf55b9661b192f2d67fff3e0e17f5cf1b018a3b3003bbdc974259a84ed
"
# The arm64 files inside the tarball, checked again after unpacking.
ARM64_PROOT=297abc237247682a84a3fd4283b28f69506502b4b852faf71fd726fb5d955d60
ARM64_LOADER=de0ace1adc76ab0555b3dbbfbff0e78c1ac14017b255262707867aea70c49437
ARM64_LOADER32=1cf92eb379447ca19e13848be1e6dd57832c7d241a6fb8e2bad06df4523721ca

here="$(cd "$(dirname "$0")" && pwd)"
out="$here/app/src/main/jniLibs"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

fail() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=PRoot for Hearth Linux::$1"; else echo "error: $1" >&2; fi
  exit 1
}

echo "$PINS" | while read -r abi arch sum; do
  [ -n "$abi" ] || continue
  f="$work/proot-android-$arch.tar.gz"
  curl -fsSL --retry 3 --proto '=https' --tlsv1.2 -o "$f" "$BASE/proot-android-$arch.tar.gz" || fail "couldn't download proot-android-$arch.tar.gz"
  got="$(sha256sum "$f" | cut -d' ' -f1)"
  [ "$got" = "$sum" ] || fail "proot-android-$arch.tar.gz: SHA-256 is $got, expected $sum. The pinned file changed: don't build with it."
  d="$work/$arch"; mkdir -p "$d"
  tar -xzf "$f" -C "$d" --no-same-owner root/bin/proot root/libexec/proot
  mkdir -p "$out/$abi"
  install -m 755 "$d/root/bin/proot" "$out/$abi/libproot.so"
  install -m 755 "$d/root/libexec/proot/loader" "$out/$abi/libproot-loader.so"
  if [ -f "$d/root/libexec/proot/loader32" ]; then install -m 755 "$d/root/libexec/proot/loader32" "$out/$abi/libproot-loader32.so"; fi
  echo "PRoot for $abi: ok ($sum)"
done

check() { local got; got="$(sha256sum "$1" | cut -d' ' -f1)"; [ "$got" = "$2" ] || fail "$(basename "$1"): SHA-256 is $got, expected $2"; }
check "$out/arm64-v8a/libproot.so" "$ARM64_PROOT"
check "$out/arm64-v8a/libproot-loader.so" "$ARM64_LOADER"
check "$out/arm64-v8a/libproot-loader32.so" "$ARM64_LOADER32"
echo "PRoot is in $out"
