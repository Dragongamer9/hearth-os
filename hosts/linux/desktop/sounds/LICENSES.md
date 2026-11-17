# Hearth's system sounds and their licences (checked Nov 16, 2026)

| Set | Where | Licence |
|---|---|---|
| **Hearth** (the default) | `hearth/stereo/*.wav`, made from sine waves by `tools/make-hearth-sounds.py`: nothing recorded, sampled or taken from anyone | Hearth OS's own: GPL-3.0 (the repo's `LICENSE`) |
| **Freedesktop** (if installed) | `/usr/share/sounds/freedesktop/stereo/*.oga`, Debian's `sound-theme-freedesktop` package (an optional package in `os-bases/peppermint/build.sh`). Hearth only plays the files the package installs; it doesn't copy or ship them | Per the package's `copyright` file: GPL-2+ for most sounds, some CC-BY-3.0 / CC-BY-SA-3.0 (their authors are named there). Playing them needs nothing from us; shipping copies would mean keeping those notices, so we don't |

The names follow the freedesktop sound naming spec, so `hearth/` is also a sound theme (`index.theme`, falling back to
`freedesktop`) other apps can use: the image links it as `/usr/share/sounds/hearth`.
