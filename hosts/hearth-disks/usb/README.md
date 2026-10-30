# Make a bootable USB: the catalog

`catalog.json` is what Hearth Disks offers to download (Disks › Make a bootable USB, or the "Make a Bootable USB"
menu entry, `hearth-disks --usb`). It's built into Disks; change it here and the next build has it.

## An entry

```json
{
  "id": "debian-netinst", "group": "linux", "family": "Debian", "name": "Debian", "flavour": "Network installer",
  "homepage": "https://www.debian.org/", "sizeHint": "800 MB", "about": "One line for the page.",
  "resolve":   { "kind": "dir", "dir": "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/" },
  "checksums": { "file": "{dir}SHA256SUMS", "pick": "^debian-(\\d+\\.\\d+\\.\\d+)-amd64-netinst\\.iso$" },
  "signature": { "kind": "detached", "of": "checksums", "file": "{dir}SHA256SUMS.sign", "key": "debian" }
}
```

- `resolve.kind` says how the newest version is found from the project's own index: `fedora` (releases.json),
  `arch` (releng JSON), `pop` (System76's build service), `dir-index` (a folder of versions; `lts` keeps even
  years), `dir` (a fixed folder), `single` (a fixed address; `followName` for a moving "Current" link), `page` (the
  project's download page), `microsoft-page` (Windows: opened in the browser). Details: `src/usb/catalog.rs`.
- `checksums.file` and `signature.file` are templates: `{url}` is the image's address, `{dir}` its folder.
  `checksums.pick` finds the image in a checksum file (its first group is the version); `checksums.listing` finds
  the checksum file in the image's folder (Fedora).
- `signature.of` is `checksums` (the usual) or `image` (Arch, CachyOS, Manjaro sign the ISO itself);
  `signature.kind` is `detached` or `clearsigned` (Fedora). Without a signature, say so in `note`; a fixed address
  says what's pinned in `pinned`.
- Only https addresses on the project's own servers or mirror network.

## Keys

`keys` names each signing key: its file in `keys/` (also listed in `src/usb/keys.rs`), the fingerprints Disks
accepts, where it was fetched from and where the project documents it. See `keys/PROVENANCE.md`.

## Checking a change

The unit tests (`cargo test --lib`, in CI: `.github/workflows/disks.yml`) validate the catalog. The same workflow
runs `hearth-disks --usb-catalog-check`: every entry is resolved against the real servers, its signature checked
with the built-in key, and its image asked for (HEAD). A project that changes its layout or signing key shows up
there.

## Several ISOs (Ventoy)

Make a bootable USB › **Several ISOs** puts [Ventoy](https://www.ventoy.net/) on the drive and copies the ISOs you
pick onto it: Hearth OS editions and Hearth Recovery (one button), anything from this catalog, and your own ISOs
(Windows: download it from Microsoft's page and add it as your own). At start-up Ventoy shows a menu of them.

- Ventoy is the upstream release, pinned: `VERSION` and `SHA256` in `src/usb/ventoy.rs` and `VENTOY_VERSION` /
  `VENTOY_SHA256` in `os/system_files/usr/libexec/nimbus/hearth-usb` (a test checks they agree). The SHA-256 is
  Ventoy's own (`sha256.txt` on the GitHub release). To move to a newer Ventoy: change both, from that release's
  `sha256.txt`, after checking the download yourself.
- Disks downloads the tarball into ~/Downloads/Hearth USB and checks it; the root helper (`hearth-admin usb-ventoy`)
  reads it itself (no links), checks the SHA-256 against its own pinned copy, unpacks only plain files under
  `ventoy-<version>/` into a private folder, and runs Ventoy's `Ventoy2Disk.sh -I` on the drive. The drive is checked
  exactly as for every other write (a whole USB drive or SD card, never the system's, the same size and serial you
  confirmed, nothing mounted; internal drives only with "Show all drives", ERASE typed and an administrator's password).
- Every ISO is downloaded and checked (or, your own, matched against a checksum file beside it or trusted by you)
  before the drive is touched; each copy is read back and its SHA-256 compared. MBR by default (Ventoy's usual), GPT
  as an option.
- CI (`disks.yml`, `tests/usb_write.rs` `g_ventoy_with_two_isos`) puts the real pinned Ventoy on a disk image with
  two ISOs and checks the partitions and files.
