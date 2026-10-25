//! The public keys that sign the downloads in usb/catalog.json, built into Disks (usb/keys; where each came from is in
//! the catalog's "keys" and usb/keys/PROVENANCE.md). A key file named in the catalog must be listed here.

const KEYS: &[(&str, &str)] = &[
    ("archlinux-pierre-schmitz.asc", include_str!("../../usb/keys/archlinux-pierre-schmitz.asc")),
    ("cachyos.asc", include_str!("../../usb/keys/cachyos.asc")),
    ("debian-cd-2011.asc", include_str!("../../usb/keys/debian-cd-2011.asc")),
    ("fedora.asc", include_str!("../../usb/keys/fedora.asc")),
    ("linuxmint.asc", include_str!("../../usb/keys/linuxmint.asc")),
    ("manjaro-build-server.asc", include_str!("../../usb/keys/manjaro-build-server.asc")),
    ("opensuse.asc", include_str!("../../usb/keys/opensuse.asc")),
    ("ubuntu-cdimage-2012.asc", include_str!("../../usb/keys/ubuntu-cdimage-2012.asc")),
];

/// A key file's armoured text.
pub fn text(file: &str) -> Option<&'static str> { KEYS.iter().find(|(f, _)| *f == file).map(|(_, t)| *t) }
