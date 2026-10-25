# Where these keys came from

Make a bootable USB (Hearth Disks) checks downloads with these public keys. They're built into Disks
(`src/usb/keys.rs`), never fetched while it runs. Each was fetched on 2026-10-25 by a GitHub Actions runner from
the source below (exported minimal: the key and its own self-signatures), and its fingerprint was compared with the
one the project documents. The signer of each project's current checksum file or image was read from the signature
itself and is the same key. `catalog.json` lists the fingerprints Disks accepts for each.

| File | Key | Fingerprint | Fetched from | Documented by the project |
|---|---|---|---|---|
| fedora.asc | Fedora (43) … (46) | C6E7F081CF80E13146676E88829B606631645531, 36F612DCF27F7D1A48A835E4DBFCF71C6D9F90A6, 4F50A6114CD5C6976A7F1179655A4B02F577861E, D924B10D3E810DABDD8B56B596E7E91491211FCE | https://fedoraproject.org/fedora.gpg | https://fedoraproject.org/security/ |
| ubuntu-cdimage-2012.asc | Ubuntu CD Image Automatic Signing Key (2012) | 843938DF228D22F7B3742BC0D94AA3F0EFE21092 | keyserver.ubuntu.com | https://ubuntu.com/tutorials/how-to-verify-ubuntu |
| debian-cd-2011.asc | Debian CD signing key | DF9B9C49EAA9298432589D76DA87E80D6294BE9B | keyserver.ubuntu.com | https://www.debian.org/CD/verify |
| linuxmint.asc | Linux Mint ISO Signing Key | 27DEB15644C6B3CF3BD7D291300F846BA25BAE09 | keyserver.ubuntu.com | Linux Mint's installation guide, "Verify your ISO image" |
| archlinux-pierre-schmitz.asc | Pierre Schmitz (Arch Linux releases) | 3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C | keyserver.ubuntu.com | https://archlinux.org/download/ (0x54449A5C) and `pgp_fingerprint` in https://archlinux.org/releng/releases/json/ |
| cachyos.asc | CachyOS | 882DCFE48E2051D48E2562ABF3B607488DB35A47 | keys.openpgp.org | https://wiki.cachyos.org/cachyos_basic/download/ |
| opensuse.asc | openSUSE Project Signing Key (valid until 2030-05-27) | AD485664E901B867051AB15F35A2F86E29B700A4 | https://download.opensuse.org/tumbleweed/repo/oss/repodata/repomd.xml.key | openSUSE's signing keys page |
| manjaro-build-server.asc | Manjaro Build Server | 3B794DE6D4320FCE594F4171279E7CF5D8D56EC8 | keyserver.ubuntu.com | Manjaro forum's ISO verification guide |

Not signed (a SHA-256 from the project only, said so in the app): Pop!_OS (System76's build service), Bazzite
(the `-CHECKSUM` file next to the ISO) and Hearth OS (the `.sha256` file next to each ISO).

When a project starts signing with a new key (Fedora's next release, for one), the catalog check in CI
(`hearth-disks --usb-catalog-check`, .github/workflows/disks.yml) fails with the new signer's fingerprint: fetch the
key from the project's own page, compare the fingerprint with what the project documents, add the file here and in
`src/usb/keys.rs`, and its fingerprint in `catalog.json`.
