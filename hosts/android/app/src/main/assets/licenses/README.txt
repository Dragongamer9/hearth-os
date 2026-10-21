Hearth for Android: third-party programs and their licenses
===========================================================

PRoot (lib/<abi>/libproot.so, libproot-loader.so, libproot-loader32.so)
  Hearth Linux runs PRoot as a separate program (its own process, started with
  exec); Hearth is not linked with it.
  License: GNU General Public License, version 2 or later (proot-COPYING.txt,
  GPL-2.0.txt). Copyright STMicroelectronics and the PRoot authors.
  Build: green-green-avk/build-proot-android, commit
    01f83b8841358450c78333d1b33ab30d4943bec4, packages/proot-android-<arch>.tar.gz
  Exact source: https://github.com/green-green-avk/proot tag v0.15_release
    (commit b40accf05dfddbaf44758aa666ade79c57255005)
    https://github.com/green-green-avk/proot/archive/refs/tags/v0.15_release.tar.gz
  Build scripts: https://github.com/green-green-avk/build-proot-android/tree/01f83b8841358450c78333d1b33ab30d4943bec4

talloc 2.1.14 (statically linked into PRoot)
  License: GNU Lesser General Public License, version 3 or later (LGPL-3.0.txt,
  GPL-3.0.txt). Source: https://download.samba.org/pub/talloc/talloc-2.1.14.tar.gz

noVNC 1.7.0 (assets/ui/novnc/core, the Linux desktop inside Hearth)
  The VNC client Hearth's page shows the Linux desktop with. Used as is,
  unmodified (fetched by fetch-novnc.sh: tag v1.7.0, commit
  63107bd06d9e1f6136ff21aeda8cd62cbf0d433e).
  License: Mozilla Public License 2.0 (MPL-2.0.txt; noVNC-LICENSE.txt lists
  what's under which license). core/crypto/des.js keeps its own BSD-style
  notice in the file. Copyright The noVNC authors.
  Source: https://github.com/novnc/noVNC/tree/v1.7.0

pako (assets/ui/novnc/vendor/pako, part of noVNC: zlib for VNC's encodings)
  License: MIT (pako-LICENSE.txt). Copyright (C) 2014-2016 Vitaly Puzrin.

XZ for Java (org.tukaani:xz), used to unpack the Linux download
  License: BSD Zero Clause (0BSD) / public domain. https://tukaani.org/xz/java.html

The Linux system itself (Debian 13 "trixie", downloaded when you install it)
  is not part of this app. Each Debian package carries its own license in
  /usr/share/doc/<package>/copyright inside the Linux.
