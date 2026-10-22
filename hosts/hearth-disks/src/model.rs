//! What Disks shows, the same on Linux and Windows: drives, and on each a map of its partitions and free space.
//! udisks.rs (Linux) and windows.rs build it; the page draws it as it is. Sizes are bytes.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// good, warn, bad or unknown
    pub state: String,
    /// One short line for the page ("Healthy", "Wearing out: replace it soon").
    pub text: String,
    pub temp_c: Option<f64>,
    pub power_on_hours: Option<u64>,
    /// Percent of its rated life used (NVMe, or Windows' wear counter).
    pub wear_pct: Option<u64>,
    pub details: Vec<String>,
}

impl Health {
    pub fn unknown() -> Health { Health { state: "unknown".into(), text: "Health not reported".into(), ..Default::default() } }
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Partition {
    /// What actions refer to it by: a udisks block object path, "disk:N:part:M" on Windows, or "free:<drive id>:<offset>".
    pub id: String,
    /// /dev/nvme0n1p2, or "E:" on Windows.
    pub device: String,
    pub number: u32,
    pub offset: u64,
    pub size: u64,
    /// File system as the system names it (ext4, btrfs, vfat, exfat, ntfs, swap, crypto_LUKS, NTFS, FAT32 ...).
    pub fs: String,
    pub label: String,
    pub uuid: String,
    pub used: Option<u64>,
    pub free: Option<u64>,
    pub mount_points: Vec<String>,
    /// The partition table's type for it (a GPT type GUID or MBR code; Windows' partition Type).
    pub part_type: String,
    pub part_name: String,
    /// hearth, windows, efi, boot, msr, recovery, swap, steam, steamos, linux, data, encrypted, extended, free, unknown
    pub kind: String,
    /// What it is, in plain words ("Hearth OS", "EFI boot", "Steam library", "Free space").
    pub what: String,
    /// Part of the running system (its root, boot or EFI partition, swap in use): never changed.
    pub system: bool,
    /// Start-up and recovery partitions (EFI, Microsoft reserved, recovery): nothing offered that erases them
    /// on the system's drive, and only with a typed confirmation elsewhere.
    pub protected: bool,
    pub mounted: bool,
    /// Has a file system that can be mounted (or given a drive letter on Windows).
    pub mountable: bool,
    pub steam_library: bool,
    /// Windows: its drive letter ("E"), empty without one.
    pub letter: String,
    /// What can be done with it (safety.rs), in the order the page offers them.
    pub actions: Vec<Action>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// A udisks drive object path, a loop device's block path, or "disk:N" on Windows.
    pub id: String,
    /// The whole drive's block object (udisks), or the disk number (Windows): what formatting the drive acts on.
    pub block: String,
    pub device: String,
    pub vendor: String,
    pub model: String,
    pub serial: String,
    pub size: u64,
    /// nvme, ssd, hdd, usb, sd, optical, image, other
    pub kind: String,
    /// How it's connected, in plain words ("Inside, NVMe", "USB", "SD card slot").
    pub connection: String,
    pub removable: bool,
    pub can_eject: bool,
    pub can_power_off: bool,
    pub read_only: bool,
    /// gpt, dos (MBR) or empty (no partition table, or a file system on the whole drive).
    pub table: String,
    pub health: Health,
    /// The running system (Hearth OS or Windows) is on this drive.
    pub system: bool,
    /// Partitions and free space, in order on the drive.
    pub partitions: Vec<Partition>,
    /// What can be done with the whole drive (safety.rs).
    pub actions: Vec<Action>,
}

/// One thing the page offers for a drive or partition.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub id: String,
    pub label: String,
    /// Erases or changes something that can't be undone: drawn in the warning colour.
    pub danger: bool,
}

impl Drive {
    /// A name for the page: the model, else the vendor, else what kind of drive it is.
    pub fn name(&self) -> String {
        let m = self.model.trim();
        let v = self.vendor.trim();
        if !m.is_empty() && !v.is_empty() && !m.to_lowercase().starts_with(&v.to_lowercase()) { return format!("{v} {m}"); }
        if !m.is_empty() { return m.to_string(); }
        if !v.is_empty() { return v.to_string(); }
        match self.kind.as_str() { "usb" => "USB drive", "sd" => "SD card", "image" => "Disk image", "optical" => "Disc drive", _ => "Drive" }.into()
    }
}

pub const MIB: u64 = 1024 * 1024;
pub const GIB: u64 = 1024 * MIB;
/// Gaps smaller than this aren't shown as free space (alignment slack between partitions).
pub const MIN_FREE: u64 = 16 * MIB;

// GPT partition types (lowercase) that matter for what a partition is.
pub const ESP_TYPE: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
pub const MSR_TYPE: &str = "e3c9e316-0b5c-4db8-817d-f92df00215ae";
pub const WINRE_TYPE: &str = "de94bba4-06d1-4d40-a16a-bfd50179d6ac";
pub const XBOOTLDR_TYPE: &str = "bc13c2ff-59e6-4262-a352-b275fd6f7172";
pub const SWAP_TYPE: &str = "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f";
pub const LINUX_TYPE: &str = "0fc63daf-8483-4772-8e79-3d69d8477de4";
pub const BASIC_DATA_TYPE: &str = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";
const STEAMOS_SYSTEM: &[&str] = &["rootfs-A", "rootfs-B", "var-A", "var-B", "efi-A", "efi-B", "esp"];

/// Where the running Linux system's own file systems are mounted. A partition mounted on one of these (or under
/// /sysroot, ostree's view of the real root) is the running system.
pub const SYSTEM_MOUNTS: &[&str] = &["/", "/sysroot", "/boot", "/boot/efi", "/efi", "/usr", "/var", "/etc", "/home", "/var/home", "/opt", "/srv", "/root"];

pub fn is_system_mount(m: &str) -> bool {
    let m = m.trim_end_matches('/');
    let m = if m.is_empty() { "/" } else { m };
    SYSTEM_MOUNTS.contains(&m) || m.starts_with("/sysroot/") || m.starts_with("/ostree/") || m.starts_with("/boot/")
}

/// Facts about one partition, from either system, for working out what it is.
#[derive(Clone, Debug, Default)]
pub struct Facts<'a> {
    pub fs: &'a str,
    pub label: &'a str,
    pub part_type: &'a str,
    pub part_name: &'a str,
    pub size: u64,
    pub mount_points: &'a [String],
    /// Part of the running system's root file system (also a second device of Hearth's btrfs).
    pub root: bool,
    /// The drive also has a Microsoft reserved partition (so its big NTFS partition is Windows).
    pub drive_has_msr: bool,
    pub steam: bool,
    /// The drive the running system is on.
    pub on_system_drive: bool,
    /// Which system is running: "linux" (Hearth OS) or "windows".
    pub host: &'a str,
}

/// What a partition is: (kind, what). Follows hearth-space's classify() (os/system_files/usr/libexec/nimbus/
/// hearth-space), so Disks and Make Hearth bigger name things the same way.
pub fn classify(f: &Facts) -> (&'static str, String) {
    let ptype = f.part_type.to_lowercase();
    let fs = f.fs.to_lowercase();
    let label = if f.label.is_empty() { f.part_name } else { f.label };
    let mps = f.mount_points;
    let has = |m: &str| mps.iter().any(|x| x.trim_end_matches('/') == m);
    if f.root {
        return if f.host == "windows" { ("windows", "Windows".into()) } else { ("hearth", "Hearth OS".into()) };
    }
    if ptype == ESP_TYPE || ptype == "0xef" || ptype == "system" || (fs == "vfat" && (has("/boot/efi") || has("/efi"))) {
        return ("efi", "EFI boot".into());
    }
    if ptype == XBOOTLDR_TYPE || has("/boot") {
        return ("boot", if f.on_system_drive && f.host == "linux" { "Hearth OS start-up files".into() } else { "Start-up files".into() });
    }
    if ptype == MSR_TYPE || ptype == "reserved" {
        return ("msr", "Microsoft reserved".into());
    }
    let lower_label = label.to_lowercase();
    if ptype == WINRE_TYPE || ptype == "recovery" || ptype == "0x27"
        || (fs == "ntfs" && (lower_label.contains("recovery") || lower_label.contains("winre") || (f.size > 0 && f.size < 3 * GIB && f.drive_has_msr))) {
        return ("recovery", "Windows recovery".into());
    }
    if STEAMOS_SYSTEM.contains(&f.part_name) {
        return ("steamos", "SteamOS system".into());
    }
    if fs == "swap" || ptype == SWAP_TYPE || ptype == "0x82" {
        return ("swap", "Swap (extra memory)".into());
    }
    if f.steam {
        return ("steam", "Steam library".into());
    }
    if fs == "crypto_luks" || fs == "bitlocker" {
        return ("encrypted", if fs == "bitlocker" { "Locked (BitLocker)".into() } else { "Locked (encrypted)".into() });
    }
    if fs == "ntfs" || fs == "refs" {
        return if f.drive_has_msr { ("windows", "Windows".into()) } else { ("data", "Files (Windows format)".into()) };
    }
    if matches!(fs.as_str(), "ext4" | "ext3" | "ext2" | "btrfs" | "xfs" | "f2fs" | "lvm2_member") {
        return ("linux", if label.eq_ignore_ascii_case("home") { "Linux home".into() } else { "Linux".into() });
    }
    if matches!(fs.as_str(), "exfat" | "vfat" | "fat32" | "fat" | "fat16" | "udf" | "iso9660") {
        return ("data", "Files".into());
    }
    if ptype == "0x05" || ptype == "0x0f" || ptype == "0x85" {
        return ("extended", "Extended (holds more partitions)".into());
    }
    if fs.is_empty() { return ("unknown", "Empty or unknown".into()); }
    ("unknown", format!("Other ({})", f.fs))
}

/// The gaps between partitions worth showing: [(offset, size)]. Partitions are (offset, size); the first MiB
/// (the partition table) and, on GPT, the last MiB (its backup) aren't usable.
pub fn free_regions(drive_size: u64, parts: &[(u64, u64)], table: &str) -> Vec<(u64, u64)> {
    if table.is_empty() || drive_size == 0 { return vec![]; }
    let mut sorted: Vec<(u64, u64)> = parts.to_vec();
    sorted.sort();
    let end = if table == "gpt" { drive_size.saturating_sub(MIB) } else { drive_size };
    let align = |n: u64| n.div_ceil(MIB) * MIB;
    let mut cursor = MIB;
    let mut out = vec![];
    for (off, size) in sorted {
        let start = align(cursor);
        if off > start && off - start >= MIN_FREE { out.push((start, off - start)); }
        cursor = cursor.max(off + size);
    }
    let start = align(cursor);
    if end > start && end - start >= MIN_FREE { out.push((start, (end - start) / MIB * MIB)); }
    out
}

/// A free-space entry for the map.
pub fn free_partition(drive_id: &str, offset: u64, size: u64) -> Partition {
    Partition { id: format!("free:{drive_id}:{offset}"), offset, size, kind: "free".into(), what: "Free space".into(), ..Default::default() }
}

/// Puts free space in the gaps and sorts everything by where it is on the drive.
pub fn fill_free(drive: &mut Drive) {
    let spans: Vec<(u64, u64)> = drive.partitions.iter().filter(|p| p.kind != "free" && !is_logical(p)).map(|p| (p.offset, p.size)).collect();
    for (off, size) in free_regions(drive.size, &spans, &drive.table) {
        drive.partitions.push(free_partition(&drive.id, off, size));
    }
    drive.partitions.sort_by_key(|p| (p.offset, p.kind == "extended"));
}

fn is_logical(p: &Partition) -> bool { p.number > 4 && p.part_type.starts_with("0x") }

/// A size for people: "512 MB", "1.8 TB".
pub fn human(bytes: u64) -> String {
    let units = ["bytes", "KB", "MB", "GB", "TB", "PB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1000.0 && i < units.len() - 1 { v /= 1000.0; i += 1; }
    if i == 0 { format!("{bytes} bytes") } else if v >= 100.0 { format!("{v:.0} {}", units[i]) } else { format!("{v:.1} {}", units[i]).replace(".0 ", " ") }
}

/// Marks the drive as the system's when one of its partitions is.
pub fn finish(drive: &mut Drive) {
    drive.system = drive.partitions.iter().any(|p| p.system);
    fill_free(drive);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts<'a>(fs: &'a str, ptype: &'a str, mps: &'a [String]) -> Facts<'a> {
        Facts { fs, part_type: ptype, mount_points: mps, size: 100 * GIB, host: "linux", ..Default::default() }
    }

    #[test]
    fn names_partitions() {
        let none: Vec<String> = vec![];
        let mut f = facts("btrfs", LINUX_TYPE, &none);
        f.root = true;
        assert_eq!(classify(&f), ("hearth", "Hearth OS".to_string()));
        assert_eq!(classify(&facts("vfat", ESP_TYPE, &none)).0, "efi");
        assert_eq!(classify(&facts("", MSR_TYPE, &none)).0, "msr");
        assert_eq!(classify(&facts("ntfs", WINRE_TYPE, &none)).0, "recovery");
        let mut w = facts("ntfs", BASIC_DATA_TYPE, &none);
        assert_eq!(classify(&w).1, "Files (Windows format)");
        w.drive_has_msr = true;
        assert_eq!(classify(&w), ("windows", "Windows".to_string()));
        assert_eq!(classify(&facts("swap", SWAP_TYPE, &none)).0, "swap");
        let mut s = facts("ext4", LINUX_TYPE, &none);
        s.steam = true;
        assert_eq!(classify(&s), ("steam", "Steam library".to_string()));
        assert_eq!(classify(&facts("exfat", "0x07", &none)), ("data", "Files".to_string()));
        assert_eq!(classify(&facts("crypto_LUKS", LINUX_TYPE, &none)).0, "encrypted");
        let boot = vec!["/boot".to_string()];
        let mut b = facts("ext4", LINUX_TYPE, &boot);
        b.on_system_drive = true;
        assert_eq!(classify(&b), ("boot", "Hearth OS start-up files".to_string()));
        let mut sd = facts("ext4", LINUX_TYPE, &none);
        sd.part_name = "rootfs-A";
        assert_eq!(classify(&sd).0, "steamos");
        assert_eq!(classify(&facts("", "", &none)).0, "unknown");
    }

    #[test]
    fn windows_names() {
        let none: Vec<String> = vec![];
        let mut f = Facts { fs: "NTFS", part_type: "Basic", host: "windows", root: true, ..Default::default() };
        f.mount_points = &none;
        assert_eq!(classify(&f), ("windows", "Windows".to_string()));
        assert_eq!(classify(&Facts { fs: "FAT32", part_type: "System", host: "windows", ..Default::default() }).0, "efi");
        assert_eq!(classify(&Facts { part_type: "Reserved", host: "windows", ..Default::default() }).0, "msr");
        assert_eq!(classify(&Facts { fs: "NTFS", part_type: "Recovery", host: "windows", ..Default::default() }).0, "recovery");
        assert_eq!(classify(&Facts { fs: "exFAT", part_type: "Basic", host: "windows", ..Default::default() }).0, "data");
    }

    #[test]
    fn system_mounts() {
        for m in ["/", "/sysroot", "/boot/efi", "/var/home", "/sysroot/ostree/deploy/x", "/boot/"] { assert!(is_system_mount(m), "{m}"); }
        for m in ["/run/media/sam/SD", "/var/mnt/games", "/mnt/data", "/media/x"] { assert!(!is_system_mount(m), "{m}"); }
    }

    #[test]
    fn finds_free_space() {
        // 100 GiB GPT drive: EFI at 1 MiB (512 MiB), Linux from 1 GiB to 51 GiB; free: the gap before 1 GiB and the end.
        let size = 100 * GIB;
        let parts = [(MIB, 512 * MIB), (GIB, 50 * GIB)];
        let free = free_regions(size, &parts, "gpt");
        assert_eq!(free.len(), 2);
        assert_eq!(free[0], (513 * MIB, 511 * MIB));
        assert_eq!(free[1].0, 51 * GIB);
        assert_eq!(free[1].1, 49 * GIB - MIB);
        // An empty table: all of it, less the table at each end.
        assert_eq!(free_regions(size, &[], "gpt"), vec![(MIB, size - 2 * MIB)]);
        assert_eq!(free_regions(size, &[], "dos"), vec![(MIB, size - MIB)]);
        // No table (a file system on the whole drive): no free space to offer.
        assert!(free_regions(size, &[], "").is_empty());
        // A tiny alignment gap isn't free space.
        assert!(free_regions(GIB, &[(MIB, GIB - 2 * MIB)], "gpt").is_empty());
    }

    #[test]
    fn sizes_and_names() {
        assert_eq!(human(512), "512 bytes");
        assert_eq!(human(500_000_000), "500 MB");
        assert_eq!(human(1_800_000_000_000), "1.8 TB");
        assert_eq!(human(64_000_000_000), "64 GB");
        let d = Drive { vendor: "Samsung".into(), model: "SSD 980 PRO".into(), ..Default::default() };
        assert_eq!(d.name(), "Samsung SSD 980 PRO");
        let d = Drive { vendor: "Samsung".into(), model: "Samsung SSD 980".into(), ..Default::default() };
        assert_eq!(d.name(), "Samsung SSD 980");
        assert_eq!(Drive { kind: "sd".into(), ..Default::default() }.name(), "SD card");
    }

    #[test]
    fn fills_the_map() {
        let mut d = Drive { id: "d".into(), size: 32 * GIB, table: "dos".into(), ..Default::default() };
        d.partitions.push(Partition { id: "p1".into(), offset: 16 * GIB, size: 8 * GIB, kind: "data".into(), system: false, ..Default::default() });
        finish(&mut d);
        let kinds: Vec<&str> = d.partitions.iter().map(|p| p.kind.as_str()).collect();
        assert_eq!(kinds, vec!["free", "data", "free"]);
        assert!(!d.system);
        assert!(d.partitions[0].id.starts_with("free:d:"));
    }
}
