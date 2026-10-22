//! udisks2's view of the drives (its ObjectManager's GetManagedObjects, turned into JSON by linux.rs: byte-array
//! strings like Block.Device become plain strings) → Disks' model. Pure, so it's tested on every system.
//! Input shape: { "/org/freedesktop/UDisks2/block_devices/sda1": { "org.freedesktop.UDisks2.Block": {..}, .. }, .. }

use crate::model::{self, classify, is_system_mount, Drive, Facts, Health, Partition};
use serde_json::Value;
use std::collections::BTreeMap;

pub const BLOCK: &str = "org.freedesktop.UDisks2.Block";
pub const DRIVE: &str = "org.freedesktop.UDisks2.Drive";
pub const ATA: &str = "org.freedesktop.UDisks2.Drive.Ata";
pub const NVME: &str = "org.freedesktop.UDisks2.NVMe.Controller";
pub const PARTITION: &str = "org.freedesktop.UDisks2.Partition";
pub const TABLE: &str = "org.freedesktop.UDisks2.PartitionTable";
pub const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
pub const SWAP: &str = "org.freedesktop.UDisks2.Swapspace";
pub const ENCRYPTED: &str = "org.freedesktop.UDisks2.Encrypted";
pub const LOOP: &str = "org.freedesktop.UDisks2.Loop";

/// What the parser needs from the running system (kept out so tests can say it).
pub struct Ctx<'a> {
    /// This user's id: loop devices they set up (udisksctl loop-setup) are shown as disk images.
    pub uid: u32,
    /// Does a file system mounted at these places hold a Steam library?
    pub steam: &'a dyn Fn(&[String]) -> bool,
    /// (used, free) bytes of what's mounted at a place.
    pub usage: &'a dyn Fn(&str) -> Option<(u64, u64)>,
    /// The devices a device is made from, by kernel name ("dm-1" → ["nvme0n1p3"]: /sys/class/block/*/slaves).
    pub slaves: &'a dyn Fn(&str) -> Vec<String>,
}

type Ifaces<'a> = &'a serde_json::Map<String, Value>;

fn iface<'a>(o: Ifaces<'a>, name: &str) -> Option<&'a Value> { o.get(name).filter(|v| v.is_object()) }
fn s(v: Option<&Value>, k: &str) -> String { v.and_then(|v| v.get(k)).and_then(Value::as_str).unwrap_or("").trim_end_matches('\0').to_string() }
fn u(v: Option<&Value>, k: &str) -> u64 { v.and_then(|v| v.get(k)).and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f.max(0.0) as u64))).unwrap_or(0) }
fn i(v: Option<&Value>, k: &str) -> i64 { v.and_then(|v| v.get(k)).and_then(|x| x.as_i64()).unwrap_or(0) }
fn b(v: Option<&Value>, k: &str) -> bool { v.and_then(|v| v.get(k)).and_then(Value::as_bool).unwrap_or(false) }
fn strs(v: Option<&Value>, k: &str) -> Vec<String> {
    v.and_then(|v| v.get(k)).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(|x| x.trim_end_matches('\0').to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
}
fn obj_path(v: Option<&Value>, k: &str) -> Option<String> { Some(s(v, k)).filter(|p| !p.is_empty() && p != "/") }

/// One block device's facts, with an unlocked encrypted partition's contents folded into it.
#[derive(Default, Clone)]
struct Blk {
    path: String,
    device: String,
    /// The kernel's name for it ("nvme0n1p3", "dm-0").
    kname: String,
    size: u64,
    drive: Option<String>,
    fs: String,
    usage: String,
    label: String,
    uuid: String,
    read_only: bool,
    mounts: Vec<String>,
    has_fs: bool,
    swap_active: bool,
    part: Option<(String, u32, u64, u64, String, String, bool)>, // table, number, offset, size, type, name, container
    table: Option<String>,
    cleartext: Option<String>,
    backing: Option<String>,
    loop_uid: Option<u32>,
    loop_file: String,
    hint_ignore: bool,
}

fn blocks(objs: &serde_json::Map<String, Value>) -> BTreeMap<String, Blk> {
    let mut out = BTreeMap::new();
    for (path, o) in objs {
        let Some(o) = o.as_object() else { continue };
        let Some(bl) = iface(o, BLOCK) else { continue };
        let fs = iface(o, FILESYSTEM);
        let part = iface(o, PARTITION).map(|p| (s(Some(p), "Table"), u(Some(p), "Number") as u32, u(Some(p), "Offset"), u(Some(p), "Size"), s(Some(p), "Type").to_lowercase(), s(Some(p), "Name"), b(Some(p), "IsContainer")));
        let dev = { let p = s(Some(bl), "PreferredDevice"); if p.is_empty() { s(Some(bl), "Device") } else { p } };
        out.insert(path.clone(), Blk {
            path: path.clone(),
            device: dev,
            kname: s(Some(bl), "Device").rsplit('/').next().unwrap_or("").to_string(),
            size: u(Some(bl), "Size"),
            drive: obj_path(Some(bl), "Drive"),
            fs: s(Some(bl), "IdType"),
            usage: s(Some(bl), "IdUsage"),
            label: s(Some(bl), "IdLabel"),
            uuid: s(Some(bl), "IdUUID"),
            read_only: b(Some(bl), "ReadOnly"),
            mounts: strs(fs, "MountPoints"),
            has_fs: fs.is_some(),
            swap_active: b(iface(o, SWAP), "Active"),
            part,
            table: iface(o, TABLE).map(|t| s(Some(t), "Type")),
            cleartext: obj_path(iface(o, ENCRYPTED), "CleartextDevice"),
            backing: obj_path(Some(bl), "CryptoBackingDevice"),
            loop_uid: iface(o, LOOP).map(|l| u(Some(l), "SetupByUID") as u32),
            loop_file: s(iface(o, LOOP), "BackingFile"),
            hint_ignore: b(Some(bl), "HintIgnore"),
        });
    }
    out
}

/// The running system's root file system: the UUID of what's mounted at /sysroot (ostree/composefs: / itself is
/// an overlay, not a block device) or /. Every device of a btrfs file system has the same UUID, so a partition
/// that Make Hearth bigger added to Hearth's btrfs counts as Hearth OS too.
fn root_uuid(bl: &BTreeMap<String, Blk>) -> Option<String> {
    for want in ["/sysroot", "/"] {
        if let Some(b) = bl.values().find(|b| b.mounts.iter().any(|m| m == want)) {
            if !b.uuid.is_empty() { return Some(b.uuid.clone()); }
        }
    }
    None
}

/// Health from SMART (SATA) or the NVMe health log.
pub fn health(drive_obj: Ifaces) -> Health {
    if let Some(n) = iface(drive_obj, NVME) {
        let n = Some(n);
        if u(n, "SmartUpdated") == 0 { return Health::unknown(); }
        let warnings = strs(n, "SmartCriticalWarning");
        let temp = u(n, "SmartTemperature");
        let mut h = Health {
            state: "good".into(), text: "Healthy".into(),
            temp_c: (temp > 0).then(|| ((temp as f64 - 273.15) * 10.0).round() / 10.0),
            power_on_hours: Some(u(n, "SmartPowerOnHours")).filter(|h| *h > 0),
            ..Default::default()
        };
        let pct = u(n, "SmartPercentUsed");
        if pct > 0 { h.wear_pct = Some(pct); }
        for w in &warnings {
            h.details.push(match w.as_str() {
                "spare" => "Its spare space is running out",
                "temperature" => "It's been too hot",
                "degraded" => "Its reliability has dropped",
                "readonly" => "It has made itself read-only to protect your files",
                "volatile_mem" => "Its backup memory failed",
                "pmr_readonly" => "Part of it has made itself read-only",
                _ => "It reported a problem",
            }.into());
        }
        if warnings.iter().any(|w| w == "degraded" || w == "readonly") { h.state = "bad".into(); h.text = "Failing: copy your files off it soon".into(); }
        else if !warnings.is_empty() || pct >= 90 { h.state = "warn".into(); h.text = "Wearing out: think about replacing it".into(); }
        return h;
    }
    if let Some(a) = iface(drive_obj, ATA) {
        let a = Some(a);
        if !b(a, "SmartSupported") || !b(a, "SmartEnabled") || u(a, "SmartUpdated") == 0 { return Health::unknown(); }
        let temp = a.and_then(|v| v.get("SmartTemperature")).and_then(Value::as_f64).unwrap_or(0.0);
        let mut h = Health {
            state: "good".into(), text: "Healthy".into(),
            temp_c: (temp > 0.0).then(|| ((temp - 273.15) * 10.0).round() / 10.0),
            power_on_hours: Some(u(a, "SmartPowerOnSeconds") / 3600).filter(|h| *h > 0),
            ..Default::default()
        };
        let bad_sectors = i(a, "SmartNumBadSectors");
        let failing_attrs = i(a, "SmartNumAttributesFailing");
        if b(a, "SmartFailing") { h.state = "bad".into(); h.text = "Failing: copy your files off it soon".into(); h.details.push("The drive says it's about to fail".into()); }
        else if bad_sectors > 0 || failing_attrs > 0 { h.state = "warn".into(); h.text = "Some problems: keep a copy of what matters".into(); }
        if bad_sectors > 0 { h.details.push(format!("{bad_sectors} damaged sectors")); }
        return h;
    }
    Health::unknown()
}

/// (kind, connection, removable) for a drive.
fn kind_of(d: Option<&Value>, device: &str, nvme: bool) -> (&'static str, &'static str, bool) {
    let bus = s(d, "ConnectionBus").to_lowercase();
    let media = s(d, "Media");
    let compat = strs(d, "MediaCompatibility");
    let removable = b(d, "Removable") || b(d, "MediaRemovable") || bus == "usb";
    let sd_media = media.starts_with("flash_sd") || media.starts_with("flash_mmc") || compat.iter().any(|c| c.starts_with("flash_sd"));
    if b(d, "Optical") || media.starts_with("optical") || compat.iter().any(|c| c.starts_with("optical")) { return ("optical", "Disc drive", true); }
    if bus == "usb" {
        return if sd_media { ("sd", "SD card in a USB reader", true) } else { ("usb", "USB", true) };
    }
    if bus == "sdio" || device.starts_with("/dev/mmcblk") {
        // A soldered-in eMMC (some handhelds and small PCs) says it isn't removable; SD cards do.
        return if removable || sd_media { ("sd", "SD card slot", true) } else { ("ssd", "Inside, eMMC", false) };
    }
    if nvme || device.starts_with("/dev/nvme") { return ("nvme", "Inside, NVMe", removable); }
    match i(d, "RotationRate") {
        0 => ("ssd", if removable { "Removable SSD" } else { "Inside, SSD" }, removable),
        r if r > 0 => ("hdd", if removable { "Removable hard drive" } else { "Inside, hard drive" }, removable),
        _ => ("other", if removable { "Removable" } else { "Inside" }, removable),
    }
}

/// Builds the drives from udisks2's objects.
pub fn parse(objs: &Value, ctx: &Ctx) -> Vec<Drive> {
    let Some(objs) = objs.as_object() else { return vec![] };
    let mut bl = blocks(objs);
    let root = root_uuid(&bl);
    // Fold unlocked encrypted partitions' contents into the partition: its mounts and whether it's the root.
    let clear: Vec<(String, Blk)> = bl.values().filter_map(|b| b.cleartext.as_ref().and_then(|c| bl.get(c)).map(|c| (b.path.clone(), c.clone()))).collect();
    for (p, c) in clear {
        if let Some(b) = bl.get_mut(&p) {
            b.mounts.extend(c.mounts.iter().cloned());
            if root.is_some() && c.uuid == root.clone().unwrap_or_default() { b.uuid = c.uuid.clone(); b.fs = c.fs.clone(); }
            b.swap_active |= c.swap_active;
            if b.label.is_empty() { b.label = c.label.clone(); }
        }
    }
    let is_root = |b: &Blk| root.as_ref().map(|r| !r.is_empty() && *r == b.uuid).unwrap_or(false);
    // Partitions the running system is built on through LVM or RAID (/ on a logical volume, maybe inside LUKS):
    // nothing on the partition itself says so, so they're found from the system's own devices downwards.
    let mut held = std::collections::HashSet::new();
    let mut todo: Vec<String> = bl.values()
        .filter(|b| b.part.is_none() && (is_root(b) || b.swap_active || b.mounts.iter().any(|m| is_system_mount(m))))
        .map(|b| b.kname.clone()).filter(|k| !k.is_empty()).collect();
    while let Some(k) = todo.pop() {
        if held.len() > 256 { break; }
        for sl in (ctx.slaves)(&k) { if held.insert(sl.clone()) { todo.push(sl); } }
    }

    let mut drives = vec![];
    // Real drives.
    for (dpath, o) in objs {
        let Some(o) = o.as_object() else { continue };
        let Some(dv) = iface(o, DRIVE) else { continue };
        let d = Some(dv);
        let Some(whole) = bl.values().filter(|b| b.drive.as_deref() == Some(dpath.as_str()) && b.part.is_none() && b.backing.is_none()).min_by_key(|b| b.path.len()) else { continue };
        let (kind, connection, removable) = kind_of(d, &whole.device, iface(o, NVME).is_some());
        let mut drive = Drive {
            id: dpath.clone(), block: whole.path.clone(), device: whole.device.clone(),
            vendor: s(d, "Vendor"), model: s(d, "Model"), serial: s(d, "Serial"),
            size: if u(d, "Size") > 0 { u(d, "Size") } else { whole.size },
            kind: kind.into(), connection: connection.into(), removable,
            can_eject: b(d, "Ejectable"), can_power_off: b(d, "CanPowerOff"),
            read_only: whole.read_only, table: whole.table.clone().unwrap_or_default(),
            health: health(o), ..Default::default()
        };
        if drive.size == 0 && kind != "optical" { continue; }   // an empty card reader
        drive.partitions = partitions_of(&bl, whole, &drive, &is_root, &held, ctx);
        model::finish(&mut drive);
        drives.push(drive);
    }
    // Disk images this user set up (udisksctl loop-setup): handy for trying things safely.
    for b in bl.values().filter(|b| b.loop_uid == Some(ctx.uid) && b.size > 0 && b.part.is_none() && !b.hint_ignore) {
        let name = b.loop_file.rsplit('/').next().unwrap_or("").to_string();
        let mut drive = Drive {
            id: b.path.clone(), block: b.path.clone(), device: b.device.clone(), model: if name.is_empty() { "Disk image".into() } else { name },
            size: b.size, kind: "image".into(), connection: "Disk image file".into(), removable: true, read_only: b.read_only,
            table: b.table.clone().unwrap_or_default(), health: Health { state: "unknown".into(), text: "A file on another drive".into(), ..Default::default() },
            ..Default::default()
        };
        drive.partitions = partitions_of(&bl, b, &drive, &is_root, &held, ctx);
        model::finish(&mut drive);
        drives.push(drive);
    }
    // Internal drives first (the system's first of all), then removable ones; stable within each.
    drives.sort_by_key(|d| (!d.system, d.removable, d.kind == "image", d.device.clone()));
    drives
}

fn partitions_of(bl: &BTreeMap<String, Blk>, whole: &Blk, _drive: &Drive, is_root: &dyn Fn(&Blk) -> bool, held: &std::collections::HashSet<String>, ctx: &Ctx) -> Vec<Partition> {
    let mut members: Vec<&Blk> = bl.values().filter(|b| b.part.as_ref().map(|p| p.0 == whole.path).unwrap_or(false)).collect();
    // A file system on the whole drive (no partition table: many USB sticks): one partition that is the drive.
    if members.is_empty() && whole.table.is_none() && (whole.has_fs || !whole.fs.is_empty()) { members.push(whole); }
    members.sort_by_key(|b| b.part.as_ref().map(|p| p.2).unwrap_or(0));
    let has_msr = members.iter().any(|b| b.part.as_ref().map(|p| p.4 == model::MSR_TYPE).unwrap_or(false));
    let on_system_drive = members.iter().any(|b| is_root(b) || held.contains(&b.kname) || b.mounts.iter().any(|m| is_system_mount(m)));
    members.iter().map(|b| {
        let (number, offset, size, ptype, pname, container) = match &b.part {
            Some(p) => (p.1, p.2, p.3, p.4.clone(), p.5.clone(), p.6),
            None => (0, 0, b.size, String::new(), String::new(), false),
        };
        let root = is_root(b);
        let steam = !b.mounts.is_empty() && (ctx.steam)(&b.mounts);
        let facts = Facts { fs: &b.fs, label: &b.label, part_type: &ptype, part_name: &pname, size, mount_points: &b.mounts, root,
            drive_has_msr: has_msr, steam, on_system_drive, host: "linux" };
        let (mut kind, mut what) = classify(&facts);
        if container { kind = "extended"; what = "Extended (holds more partitions)".into(); }
        let system = root || b.swap_active || held.contains(&b.kname) || b.mounts.iter().any(|m| is_system_mount(m));
        let (used, free) = match b.mounts.first().and_then(|m| (ctx.usage)(m)) { Some((u, f)) => (Some(u), Some(f)), None => (None, None) };
        Partition {
            id: b.path.clone(), device: b.device.clone(), number, offset, size: if size > 0 { size } else { b.size },
            fs: if b.usage == "filesystem" || b.usage == "crypto" || b.fs == "swap" || !b.fs.is_empty() { b.fs.clone() } else { String::new() },
            label: b.label.clone(), uuid: b.uuid.clone(), used, free, mount_points: b.mounts.clone(),
            part_type: ptype, part_name: pname, kind: kind.into(), what, system,
            protected: matches!(kind, "efi" | "msr" | "recovery" | "boot" | "steamos"),
            mounted: !b.mounts.is_empty(), mountable: b.has_fs, steam_library: steam,
            letter: String::new(), actions: vec![],
        }
    }).collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    pub fn sample() -> Value {
        let gib: u64 = 1 << 30;
        json!({
            "/org/freedesktop/UDisks2/drives/Samsung_SSD": {
                DRIVE: {"Vendor": "", "Model": "Samsung SSD 980 PRO 1TB", "Serial": "S5", "Size": 1000 * gib, "Removable": false, "MediaRemovable": false,
                        "ConnectionBus": "", "RotationRate": 0, "Ejectable": false, "CanPowerOff": false, "Media": "", "MediaCompatibility": []},
                NVME: {"SmartUpdated": 1700000000u64, "SmartCriticalWarning": [], "SmartPowerOnHours": 1234, "SmartTemperature": 313, "SmartPercentUsed": 3}
            },
            "/org/freedesktop/UDisks2/block_devices/nvme0n1": {
                BLOCK: {"Device": "/dev/nvme0n1", "PreferredDevice": "/dev/nvme0n1", "Size": 1000 * gib, "Drive": "/org/freedesktop/UDisks2/drives/Samsung_SSD",
                        "IdType": "", "IdUsage": "", "IdLabel": "", "IdUUID": "", "ReadOnly": false, "CryptoBackingDevice": "/"},
                TABLE: {"Type": "gpt", "Partitions": []}
            },
            "/org/freedesktop/UDisks2/block_devices/nvme0n1p1": {
                BLOCK: {"Device": "/dev/nvme0n1p1", "Size": gib / 2, "Drive": "/org/freedesktop/UDisks2/drives/Samsung_SSD", "IdType": "vfat", "IdUsage": "filesystem", "IdLabel": "EFI", "IdUUID": "AAAA-BBBB", "CryptoBackingDevice": "/"},
                PARTITION: {"Table": "/org/freedesktop/UDisks2/block_devices/nvme0n1", "Number": 1, "Offset": 1u64 << 20, "Size": (gib / 2) - (1u64 << 20), "Type": "C12A7328-F81F-11D2-BA4B-00A0C93EC93B", "Name": "EFI System", "IsContainer": false},
                FILESYSTEM: {"MountPoints": []}
            },
            "/org/freedesktop/UDisks2/block_devices/nvme0n1p2": {
                BLOCK: {"Device": "/dev/nvme0n1p2", "Size": gib, "Drive": "/org/freedesktop/UDisks2/drives/Samsung_SSD", "IdType": "ext4", "IdUsage": "filesystem", "IdLabel": "boot", "IdUUID": "boot-uuid", "CryptoBackingDevice": "/"},
                PARTITION: {"Table": "/org/freedesktop/UDisks2/block_devices/nvme0n1", "Number": 2, "Offset": gib / 2, "Size": gib / 2, "Type": model::LINUX_TYPE, "Name": "boot"},
                FILESYSTEM: {"MountPoints": ["/boot"]}
            },
            "/org/freedesktop/UDisks2/block_devices/nvme0n1p3": {
                BLOCK: {"Device": "/dev/nvme0n1p3", "Size": 400 * gib, "Drive": "/org/freedesktop/UDisks2/drives/Samsung_SSD", "IdType": "btrfs", "IdUsage": "filesystem", "IdLabel": "hearth", "IdUUID": "root-uuid", "CryptoBackingDevice": "/"},
                PARTITION: {"Table": "/org/freedesktop/UDisks2/block_devices/nvme0n1", "Number": 3, "Offset": gib, "Size": 400 * gib, "Type": model::LINUX_TYPE, "Name": "root"},
                FILESYSTEM: {"MountPoints": ["/sysroot", "/var", "/var/home", "/etc"]}
            },
            // A second device Make Hearth bigger added to Hearth's btrfs: not mounted itself, same UUID.
            "/org/freedesktop/UDisks2/block_devices/nvme0n1p4": {
                BLOCK: {"Device": "/dev/nvme0n1p4", "Size": 100 * gib, "Drive": "/org/freedesktop/UDisks2/drives/Samsung_SSD", "IdType": "btrfs", "IdUsage": "filesystem", "IdLabel": "hearth", "IdUUID": "root-uuid", "CryptoBackingDevice": "/"},
                PARTITION: {"Table": "/org/freedesktop/UDisks2/block_devices/nvme0n1", "Number": 4, "Offset": 401 * gib, "Size": 100 * gib, "Type": model::LINUX_TYPE, "Name": "hearth-more"},
                FILESYSTEM: {"MountPoints": []}
            },
            "/org/freedesktop/UDisks2/block_devices/nvme0n1p5": {
                BLOCK: {"Device": "/dev/nvme0n1p5", "Size": 200 * gib, "Drive": "/org/freedesktop/UDisks2/drives/Samsung_SSD", "IdType": "ext4", "IdUsage": "filesystem", "IdLabel": "Games", "IdUUID": "games-uuid", "CryptoBackingDevice": "/"},
                PARTITION: {"Table": "/org/freedesktop/UDisks2/block_devices/nvme0n1", "Number": 5, "Offset": 600 * gib, "Size": 200 * gib, "Type": model::LINUX_TYPE, "Name": ""},
                FILESYSTEM: {"MountPoints": ["/var/mnt/games-Games"]}
            },
            "/org/freedesktop/UDisks2/drives/SanDisk_SD": {
                DRIVE: {"Vendor": "", "Model": "SD64G", "Size": 64 * gib, "Removable": true, "MediaRemovable": true, "ConnectionBus": "sdio", "RotationRate": -1, "Ejectable": false, "CanPowerOff": false, "Media": "flash_sd"}
            },
            "/org/freedesktop/UDisks2/block_devices/mmcblk0": {
                BLOCK: {"Device": "/dev/mmcblk0", "Size": 64 * gib, "Drive": "/org/freedesktop/UDisks2/drives/SanDisk_SD", "IdType": "", "IdUsage": "", "CryptoBackingDevice": "/"},
                TABLE: {"Type": "dos"}
            },
            "/org/freedesktop/UDisks2/block_devices/mmcblk0p1": {
                BLOCK: {"Device": "/dev/mmcblk0p1", "Size": 64 * gib - (1u64 << 20), "Drive": "/org/freedesktop/UDisks2/drives/SanDisk_SD", "IdType": "exfat", "IdUsage": "filesystem", "IdLabel": "SWITCH", "IdUUID": "1234-5678", "CryptoBackingDevice": "/"},
                PARTITION: {"Table": "/org/freedesktop/UDisks2/block_devices/mmcblk0", "Number": 1, "Offset": 1u64 << 20, "Size": 64 * gib - (1u64 << 20), "Type": "0x07"},
                FILESYSTEM: {"MountPoints": ["/run/media/sam/SWITCH"]}
            },
            // A USB stick with a file system on the whole drive (no partition table).
            "/org/freedesktop/UDisks2/drives/Kingston_DT": {
                DRIVE: {"Vendor": "Kingston", "Model": "DataTraveler 3.0", "Size": 32 * gib, "Removable": true, "ConnectionBus": "usb", "RotationRate": -1, "Ejectable": true, "CanPowerOff": true},
                ATA: {"SmartSupported": false}
            },
            "/org/freedesktop/UDisks2/block_devices/sda": {
                BLOCK: {"Device": "/dev/sda", "Size": 32 * gib, "Drive": "/org/freedesktop/UDisks2/drives/Kingston_DT", "IdType": "vfat", "IdUsage": "filesystem", "IdLabel": "KINGSTON", "IdUUID": "CAFE-0001", "CryptoBackingDevice": "/"},
                FILESYSTEM: {"MountPoints": []}
            },
            // An empty card reader slot: no drive shown.
            "/org/freedesktop/UDisks2/drives/Reader": {
                DRIVE: {"Model": "Card Reader", "Size": 0, "Removable": true, "ConnectionBus": "usb"}
            },
            "/org/freedesktop/UDisks2/block_devices/sdb": {
                BLOCK: {"Device": "/dev/sdb", "Size": 0, "Drive": "/org/freedesktop/UDisks2/drives/Reader"}
            },
            // A disk image this user set up, and someone else's (snap's) that isn't shown.
            "/org/freedesktop/UDisks2/block_devices/loop7": {
                BLOCK: {"Device": "/dev/loop7", "Size": 256u64 << 20, "Drive": "/", "IdType": "", "IdUsage": ""},
                LOOP: {"BackingFile": "/home/sam/test.img", "SetupByUID": 1000}
            },
            "/org/freedesktop/UDisks2/block_devices/loop0": {
                BLOCK: {"Device": "/dev/loop0", "Size": 100u64 << 20, "Drive": "/", "IdType": "squashfs", "IdUsage": "filesystem"},
                LOOP: {"BackingFile": "/var/lib/snapd/snaps/core.snap", "SetupByUID": 0},
                FILESYSTEM: {"MountPoints": ["/snap/core/1"]}
            }
        })
    }

    pub fn ctx_steam(m: &[String]) -> bool { m.iter().any(|x| x.contains("games-")) }
    pub fn ctx_usage(m: &str) -> Option<(u64, u64)> { if m == "/run/media/sam/SWITCH" { Some((10 << 30, 50 << 30)) } else { None } }

    fn drives() -> Vec<Drive> {
        let ctx = Ctx { uid: 1000, steam: &ctx_steam, usage: &ctx_usage, slaves: &|_: &str| vec![] };
        let mut d = parse(&sample(), &ctx);
        crate::safety::annotate(&mut d, "linux");
        d
    }

    #[test]
    fn reads_the_drives() {
        let d = drives();
        let names: Vec<String> = d.iter().map(|d| d.name()).collect();
        assert_eq!(names, vec!["Samsung SSD 980 PRO 1TB", "SD64G", "Kingston DataTraveler 3.0", "test.img"]);
        let nv = &d[0];
        assert_eq!((nv.kind.as_str(), nv.connection.as_str(), nv.removable, nv.system), ("nvme", "Inside, NVMe", false, true));
        assert_eq!(nv.health.state, "good");
        assert!(nv.health.temp_c.map(|t| t > 39.0 && t < 40.5).unwrap_or(false));
        assert_eq!(nv.health.power_on_hours, Some(1234));
        assert_eq!(nv.health.wear_pct, Some(3));
        let sd = &d[1];
        assert_eq!((sd.kind.as_str(), sd.connection.as_str(), sd.removable, sd.system), ("sd", "SD card slot", true, false));
        assert_eq!(sd.health.state, "unknown");
        let usb = &d[2];
        assert_eq!((usb.kind.as_str(), usb.connection.as_str()), ("usb", "USB"));
        assert_eq!(usb.partitions.len(), 1, "a file system on the whole stick is one partition");
        assert_eq!(usb.partitions[0].id, "/org/freedesktop/UDisks2/block_devices/sda");
        assert_eq!(usb.partitions[0].what, "Files");
        assert_eq!(d[3].kind, "image");
    }

    #[test]
    fn names_hearths_partitions() {
        let d = drives();
        let parts: Vec<(&str, &str, bool)> = d[0].partitions.iter().map(|p| (p.kind.as_str(), p.what.as_str(), p.system)).collect();
        assert_eq!(parts, vec![
            ("efi", "EFI boot", false),
            ("boot", "Hearth OS start-up files", true),
            ("hearth", "Hearth OS", true),
            ("hearth", "Hearth OS", true),
            ("free", "Free space", false),
            ("steam", "Steam library", false),
            ("free", "Free space", false),
        ]);
        // The EFI partition isn't mounted (bootc), but it's on the system's drive: protected, nothing offered that erases it.
        let efi = &d[0].partitions[0];
        assert!(efi.protected);
        assert!(!efi.actions.iter().any(|a| a.id == "format" || a.id == "delete"));
        assert!(crate::safety::plan("format", &d[0], Some(efi), "linux").refuse.is_some());
        // The second btrfs device is Hearth OS: never changed.
        let more = &d[0].partitions[3];
        assert!(crate::safety::plan("delete", &d[0], Some(more), "linux").refuse.is_some());
        // The games partition on the system drive: changes need the word.
        let games = &d[0].partitions[5];
        assert!(games.steam_library);
        assert_eq!(crate::safety::plan("format", &d[0], Some(games), "linux").typed.as_deref(), Some("ERASE"));
    }

    #[test]
    fn sd_card_details() {
        let d = drives();
        let p = &d[1].partitions;
        assert_eq!(p[0].kind, "data");
        assert_eq!(p[0].label, "SWITCH");
        assert_eq!(p[0].used, Some(10 << 30));
        assert!(p[0].mounted);
        let acts: Vec<&str> = p[0].actions.iter().map(|a| a.id.as_str()).collect();
        assert!(acts.contains(&"unmount") && acts.contains(&"format") && acts.contains(&"games"));
        let dacts: Vec<&str> = d[1].actions.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(dacts, vec!["eject", "games", "format_drive"]);
    }

    #[test]
    fn sata_health() {
        let o = json!({ATA: {"SmartSupported": true, "SmartEnabled": true, "SmartUpdated": 1, "SmartFailing": false, "SmartTemperature": 308.15, "SmartPowerOnSeconds": 36000, "SmartNumBadSectors": 3, "SmartNumAttributesFailing": 0}});
        let h = health(o.as_object().unwrap());
        assert_eq!(h.state, "warn");
        assert_eq!(h.temp_c, Some(35.0));
        assert_eq!(h.power_on_hours, Some(10));
        let o = json!({ATA: {"SmartSupported": true, "SmartEnabled": true, "SmartUpdated": 1, "SmartFailing": true}});
        assert_eq!(health(o.as_object().unwrap()).state, "bad");
        let o = json!({NVME: {"SmartUpdated": 1, "SmartCriticalWarning": ["spare"], "SmartTemperature": 300}});
        assert_eq!(health(o.as_object().unwrap()).state, "warn");
        let o = json!({NVME: {"SmartUpdated": 1, "SmartCriticalWarning": ["readonly"]}});
        assert_eq!(health(o.as_object().unwrap()).state, "bad");
    }

    #[test]
    fn plain_root_and_encrypted_root() {
        // A classic install: btrfs mounted at / directly, inside LUKS.
        let gib: u64 = 1 << 30;
        let v = json!({
            "/d": {DRIVE: {"Model": "WD Blue", "Size": 500 * gib, "RotationRate": 7200}},
            "/b/sda": {BLOCK: {"Device": "/dev/sda", "Size": 500 * gib, "Drive": "/d"}, TABLE: {"Type": "gpt"}},
            "/b/sda1": {BLOCK: {"Device": "/dev/sda1", "Size": 499 * gib, "Drive": "/d", "IdType": "crypto_LUKS", "IdUsage": "crypto", "IdUUID": "luks"},
                        PARTITION: {"Table": "/b/sda", "Number": 1, "Offset": 1u64 << 20, "Size": 499 * gib, "Type": model::LINUX_TYPE},
                        ENCRYPTED: {"CleartextDevice": "/b/dm0"}},
            "/b/dm0": {BLOCK: {"Device": "/dev/dm-0", "Size": 499 * gib, "Drive": "/", "IdType": "btrfs", "IdUsage": "filesystem", "IdUUID": "root", "CryptoBackingDevice": "/b/sda1"},
                       FILESYSTEM: {"MountPoints": ["/", "/home"]}}
        });
        let ctx = Ctx { uid: 1000, steam: &|_: &[String]| false, usage: &|_: &str| None, slaves: &|_: &str| vec![] };
        let d = parse(&v, &ctx);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].kind.as_str(), d[0].connection.as_str()), ("hdd", "Inside, hard drive"));
        assert!(d[0].system);
        assert_eq!(d[0].partitions[0].kind, "hearth");
        assert!(d[0].partitions[0].system);
    }

    #[test]
    fn root_on_lvm_inside_luks() {
        // / on a logical volume, in LUKS, on sda2: sda2 is the running system though nothing on it says so.
        let gib: u64 = 1 << 30;
        let v = json!({
            "/d": {DRIVE: {"Model": "WD Blue", "Size": 500 * gib, "RotationRate": 0}},
            "/b/sda": {BLOCK: {"Device": "/dev/sda", "Size": 500 * gib, "Drive": "/d"}, TABLE: {"Type": "gpt"}},
            "/b/sda1": {BLOCK: {"Device": "/dev/sda1", "Size": 100 * gib, "Drive": "/d", "IdType": "ext4", "IdUsage": "filesystem", "IdUUID": "data"},
                        PARTITION: {"Table": "/b/sda", "Number": 1, "Offset": 1u64 << 20, "Size": 100 * gib, "Type": model::LINUX_TYPE}, FILESYSTEM: {"MountPoints": []}},
            "/b/sda2": {BLOCK: {"Device": "/dev/sda2", "Size": 399 * gib, "Drive": "/d", "IdType": "crypto_LUKS", "IdUsage": "crypto", "IdUUID": "luks"},
                        PARTITION: {"Table": "/b/sda", "Number": 2, "Offset": 101 * gib, "Size": 399 * gib, "Type": model::LINUX_TYPE},
                        ENCRYPTED: {"CleartextDevice": "/b/dm0"}},
            "/b/dm0": {BLOCK: {"Device": "/dev/dm-0", "PreferredDevice": "/dev/mapper/luks-x", "Size": 399 * gib, "Drive": "/", "IdType": "LVM2_member", "IdUsage": "raid", "CryptoBackingDevice": "/b/sda2"}},
            "/b/dm1": {BLOCK: {"Device": "/dev/dm-1", "PreferredDevice": "/dev/vg/root", "Size": 300 * gib, "Drive": "/", "IdType": "ext4", "IdUsage": "filesystem", "IdUUID": "root"},
                       FILESYSTEM: {"MountPoints": ["/"]}}
        });
        let slaves = |k: &str| match k { "dm-1" => vec!["dm-0".to_string()], "dm-0" => vec!["sda2".to_string()], _ => vec![] };
        let ctx = Ctx { uid: 1000, steam: &|_: &[String]| false, usage: &|_: &str| None, slaves: &slaves };
        let mut d = parse(&v, &ctx);
        crate::safety::annotate(&mut d, "linux");
        assert!(d[0].system);
        let sda2 = d[0].partitions.iter().find(|p| p.device == "/dev/sda2").unwrap();
        assert!(sda2.system);
        for a in ["format", "delete"] { assert!(crate::safety::plan(a, &d[0], Some(sda2), "linux").refuse.is_some(), "{a}"); }
        let sda1 = d[0].partitions.iter().find(|p| p.device == "/dev/sda1").unwrap();
        assert!(!sda1.system);
        assert!(crate::safety::plan("format_drive", &d[0], None, "linux").refuse.is_some());
    }
}
