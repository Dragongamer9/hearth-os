//! Windows' view of the drives (the Storage module's Get-Disk, Get-Partition, Get-Volume, Get-PhysicalDisk and,
//! when allowed, Get-StorageReliabilityCounter, as JSON from winps.rs's LIST script) → Disks' model.
//! Pure, so it's tested on every system. Enum values may come as names ("NVMe") or numbers (17): both are read.

use crate::model::{self, classify, Drive, Facts, Health, Partition};
use serde_json::Value;

fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.trim_matches(char::from(0)).trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn u(v: &Value, k: &str) -> u64 { v.get(k).and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f.max(0.0) as u64)).or_else(|| x.as_str().and_then(|t| t.parse().ok()))).unwrap_or(0) }
fn b(v: &Value, k: &str) -> bool { v.get(k).and_then(Value::as_bool).unwrap_or(false) }
fn arr<'a>(v: &'a Value, k: &str) -> Vec<&'a Value> {
    match v.get(k) { Some(Value::Array(a)) => a.iter().collect(), Some(o @ Value::Object(_)) => vec![o], _ => vec![] }
}

/// MSFT_Disk / MSFT_PhysicalDisk BusType.
pub fn bus_name(v: &Value, k: &str) -> String {
    let raw = s(v, k);
    let names = ["Unknown", "SCSI", "ATAPI", "ATA", "1394", "SSA", "Fibre Channel", "USB", "RAID", "iSCSI", "SAS", "SATA", "SD", "MMC", "Virtual", "File Backed Virtual", "Storage Spaces", "NVMe"];
    match raw.parse::<usize>() { Ok(n) => names.get(n).copied().unwrap_or("Unknown").to_string(), Err(_) => raw }
}
/// HealthStatus: 0 Healthy, 1 Warning, 2 Unhealthy, 5 Unknown (or the names).
pub fn health_name(v: &Value, k: &str) -> String {
    match s(v, k).as_str() { "0" | "Healthy" => "Healthy", "1" | "Warning" => "Warning", "2" | "Unhealthy" => "Unhealthy", _ => "Unknown" }.into()
}
/// PartitionStyle: 1 MBR, 2 GPT, 3 RAW.
fn table_name(v: &Value) -> String {
    match s(v, "PartitionStyle").to_uppercase().as_str() { "1" | "MBR" => "dos", "2" | "GPT" => "gpt", _ => "" }.into()
}
/// MediaType of a physical disk: 3 HDD, 4 SSD, 5 SCM.
fn media_name(v: &Value) -> String {
    match s(v, "MediaType").as_str() { "3" | "HDD" => "HDD", "4" | "SSD" => "SSD", "5" | "SCM" => "SCM", _ => "" }.into()
}
fn guid(x: &str) -> String { x.trim_matches(|c| c == '{' || c == '}').to_lowercase() }

fn letter_of(v: &Value) -> String {
    let l = s(v, "DriveLetter");
    l.chars().next().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_uppercase().to_string()).unwrap_or_default()
}

/// The drives. `steam`: does the drive with this letter ("E") hold a Steam library?
pub fn parse(v: &Value, steam: &dyn Fn(&str) -> bool) -> Vec<Drive> {
    let system_letter = s(v, "systemDrive").chars().next().map(|c| c.to_ascii_uppercase().to_string()).unwrap_or_else(|| "C".into());
    let disks = arr(v, "disks");
    let parts = arr(v, "partitions");
    let vols = arr(v, "volumes");
    let phys = arr(v, "physical");
    let rel = arr(v, "reliability");
    let mut out = vec![];
    for d in disks {
        let n = u(d, "Number");
        let pd = phys.iter().find(|p| s(p, "DeviceId") == n.to_string()).copied();
        let rc = rel.iter().find(|r| s(r, "DeviceId") == n.to_string()).copied();
        let bus = bus_name(d, "BusType");
        let name = { let f = s(d, "FriendlyName"); if f.is_empty() { s(d, "Model") } else { f } };
        let media = pd.map(media_name).unwrap_or_default();
        let sd_reader = name.to_lowercase().contains("sd") && (name.to_lowercase().contains("reader") || name.to_lowercase().contains("mmc") || name.to_lowercase().contains("card"));
        let (kind, connection, removable) = match bus.as_str() {
            "NVMe" => ("nvme", "Inside, NVMe", false),
            "USB" if sd_reader => ("sd", "SD card in a USB reader", true),
            "USB" => ("usb", "USB", true),
            "SD" | "MMC" => ("sd", "SD card slot", true),
            "Virtual" | "File Backed Virtual" => ("image", "Disk image file", true),
            _ if media == "HDD" => ("hdd", "Inside, hard drive", false),
            _ if media == "SSD" || media == "SCM" => ("ssd", "Inside, SSD", false),
            _ => ("other", "Inside", false),
        };
        let health = health_of(d, pd, rc);
        let table = table_name(d);
        let mut drive = Drive {
            id: format!("disk:{n}"), block: n.to_string(), device: format!("Disk {n}"),
            vendor: String::new(), model: name, serial: s(d, "SerialNumber"), size: u(d, "Size"),
            kind: kind.into(), connection: connection.into(), removable, can_eject: removable, can_power_off: false,
            read_only: b(d, "IsReadOnly") || b(d, "IsOffline"), table, health, ..Default::default()
        };
        let mine: Vec<&Value> = parts.iter().copied().filter(|p| u(p, "DiskNumber") == n).collect();
        let has_msr = mine.iter().any(|p| s(p, "Type") == "Reserved" || guid(&s(p, "GptType")) == model::MSR_TYPE);
        let disk_system = b(d, "IsSystem") || b(d, "IsBoot");
        for p in &mine {
            let num = u(p, "PartitionNumber") as u32;
            let paths: Vec<String> = arr(p, "AccessPaths").iter().filter_map(|x| x.as_str().map(str::to_string)).collect();
            let letter = letter_of(p);
            let vol = vols.iter().copied().find(|vv| { let vp = s(vv, "Path"); !vp.is_empty() && paths.iter().any(|a| a.eq_ignore_ascii_case(&vp)) })
                .or_else(|| if letter.is_empty() { None } else { vols.iter().copied().find(|vv| letter_of(vv) == letter) });
            let fs = vol.map(|vv| s(vv, "FileSystem")).unwrap_or_default();
            let label = vol.map(|vv| s(vv, "FileSystemLabel")).unwrap_or_default();
            let ptype = s(p, "Type");
            let gtype = guid(&s(p, "GptType"));
            let facts_type = if matches!(ptype.as_str(), "System" | "Reserved" | "Recovery") || gtype.is_empty() { ptype.clone() } else { gtype.clone() };
            let root = b(p, "IsBoot") || (!letter.is_empty() && letter == system_letter);
            let is_steam = !letter.is_empty() && steam(&letter);
            let mounts: Vec<String> = if letter.is_empty() { vec![] } else { vec![format!("{letter}:\\")] };
            let size = u(p, "Size");
            let facts = Facts { fs: &fs, label: &label, part_type: &facts_type, part_name: "", size, mount_points: &[], root,
                drive_has_msr: has_msr, steam: is_steam, on_system_drive: disk_system, host: "windows" };
            let (mut kind, mut what) = classify(&facts);
            if ptype == "Extended" { kind = "extended"; what = "Extended (holds more partitions)".into(); }
            // Windows on another drive (not the one running): still Windows, but not the running system.
            let system = root || b(p, "IsSystem") || (disk_system && matches!(kind, "efi" | "msr" | "recovery"));
            let (used, free) = match vol { Some(vv) if u(vv, "Size") > 0 => (Some(u(vv, "Size").saturating_sub(u(vv, "SizeRemaining"))), Some(u(vv, "SizeRemaining"))), _ => (None, None) };
            drive.partitions.push(Partition {
                id: format!("disk:{n}:part:{num}"),
                device: if letter.is_empty() { format!("Disk {n}, partition {num}") } else { format!("{letter}:") },
                number: num, offset: u(p, "Offset"), size, fs: fs.clone(), label, uuid: s(p, "Guid"), used, free,
                mount_points: mounts, part_type: if gtype.is_empty() { ptype.clone() } else { gtype }, part_name: ptype.clone(),
                kind: kind.into(), what, system, protected: matches!(kind, "efi" | "msr" | "recovery"),
                mounted: !letter.is_empty(), mountable: !fs.is_empty() || (vol.is_some() && kind != "msr"), steam_library: is_steam, letter,
                actions: vec![],
            });
        }
        model::finish(&mut drive);
        drive.system |= disk_system;
        out.push(drive);
    }
    out.sort_by_key(|d| (!d.system, d.removable, d.kind == "image", d.block.parse::<u64>().unwrap_or(0)));
    out
}

fn health_of(d: &Value, pd: Option<&Value>, rc: Option<&Value>) -> Health {
    let status = pd.map(|p| health_name(p, "HealthStatus")).filter(|h| h != "Unknown").unwrap_or_else(|| health_name(d, "HealthStatus"));
    let mut h = match status.as_str() {
        "Healthy" => Health { state: "good".into(), text: "Healthy".into(), ..Default::default() },
        "Warning" => Health { state: "warn".into(), text: "Windows reports a problem: keep a copy of what matters".into(), ..Default::default() },
        "Unhealthy" => Health { state: "bad".into(), text: "Failing: copy your files off it soon".into(), ..Default::default() },
        _ => Health::unknown(),
    };
    if let Some(r) = rc {
        let t = u(r, "Temperature");
        if t > 0 { h.temp_c = Some(t as f64); }
        let hours = u(r, "PowerOnHours");
        if hours > 0 { h.power_on_hours = Some(hours); }
        let wear = u(r, "Wear");
        if wear > 0 { h.wear_pct = Some(wear); }
        if wear >= 90 && h.state == "good" { h.state = "warn".into(); h.text = "Wearing out: think about replacing it".into(); }
        let bad = u(r, "ReadErrorsUncorrected");
        if bad > 0 { h.details.push(format!("{bad} read errors it couldn't fix")); }
    }
    h
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    pub fn sample() -> Value {
        let gib: u64 = 1 << 30;
        json!({
            "systemDrive": "C:",
            "disks": [
                {"Number": 0, "FriendlyName": "WD_BLACK SN850X 2000GB", "SerialNumber": "E823_8FA6", "Size": 2000 * gib, "BusType": "NVMe", "PartitionStyle": "GPT",
                 "IsSystem": true, "IsBoot": true, "IsReadOnly": false, "IsOffline": false, "HealthStatus": "Healthy"},
                {"Number": 1, "FriendlyName": "SanDisk Cruzer", "Size": 64 * gib, "BusType": 7, "PartitionStyle": 1, "IsSystem": false, "IsBoot": false, "HealthStatus": 0},
                {"Number": 2, "FriendlyName": "ST2000DM008", "Size": 2000 * gib, "BusType": "SATA", "PartitionStyle": "GPT", "IsSystem": false, "IsBoot": false, "HealthStatus": "Healthy"}
            ],
            "physical": [
                {"DeviceId": "0", "MediaType": "SSD", "HealthStatus": "Healthy"},
                {"DeviceId": "2", "MediaType": 3, "HealthStatus": 1}
            ],
            "reliability": {"DeviceId": "0", "Temperature": 41, "Wear": 2, "PowerOnHours": 900},
            "partitions": [
                {"DiskNumber": 0, "PartitionNumber": 1, "Offset": 1u64 << 20, "Size": 100u64 << 20, "Type": "System", "GptType": "{c12a7328-f81f-11d2-ba4b-00a0c93ec93b}",
                 "DriveLetter": "\u{0}", "IsSystem": true, "IsBoot": false, "AccessPaths": ["\\\\?\\Volume{a}\\"]},
                {"DiskNumber": 0, "PartitionNumber": 2, "Offset": 101u64 << 20, "Size": 16u64 << 20, "Type": "Reserved", "GptType": "{e3c9e316-0b5c-4db8-817d-f92df00215ae}", "DriveLetter": null},
                {"DiskNumber": 0, "PartitionNumber": 3, "Offset": 117u64 << 20, "Size": 1000 * gib, "Type": "Basic", "GptType": "{ebd0a0a2-b9e5-4433-87c0-68b6b72699c7}",
                 "DriveLetter": "C", "IsBoot": true, "AccessPaths": ["C:\\", "\\\\?\\Volume{c}\\"]},
                {"DiskNumber": 0, "PartitionNumber": 4, "Offset": 1000 * gib + (117u64 << 20), "Size": 900u64 << 20, "Type": "Recovery", "GptType": "{de94bba4-06d1-4d40-a16a-bfd50179d6ac}", "DriveLetter": ""},
                {"DiskNumber": 1, "PartitionNumber": 1, "Offset": 1u64 << 20, "Size": 64 * gib - (1u64 << 20), "Type": "IFS", "DriveLetter": "E", "AccessPaths": ["E:\\", "\\\\?\\Volume{e}\\"]},
                {"DiskNumber": 2, "PartitionNumber": 1, "Offset": 1u64 << 20, "Size": 2000 * gib - (2u64 << 20), "Type": "Basic", "GptType": "{ebd0a0a2-b9e5-4433-87c0-68b6b72699c7}", "DriveLetter": "D", "AccessPaths": ["D:\\", "\\\\?\\Volume{d}\\"]}
            ],
            "volumes": [
                {"Path": "\\\\?\\Volume{a}\\", "DriveLetter": null, "FileSystem": "FAT32", "FileSystemLabel": "", "Size": 100u64 << 20, "SizeRemaining": 70u64 << 20},
                {"Path": "\\\\?\\Volume{c}\\", "DriveLetter": "C", "FileSystem": "NTFS", "FileSystemLabel": "Windows", "Size": 1000 * gib, "SizeRemaining": 400 * gib},
                {"Path": "\\\\?\\Volume{e}\\", "DriveLetter": "E", "FileSystem": "exFAT", "FileSystemLabel": "STICK", "Size": 64 * gib, "SizeRemaining": 60 * gib},
                {"Path": "\\\\?\\Volume{d}\\", "DriveLetter": "D", "FileSystem": "NTFS", "FileSystemLabel": "Games", "Size": 2000 * gib, "SizeRemaining": 1000 * gib}
            ]
        })
    }

    fn drives() -> Vec<Drive> {
        let mut d = parse(&sample(), &|l: &str| l == "D");
        crate::safety::annotate(&mut d, "windows");
        d
    }

    #[test]
    fn reads_windows_drives() {
        let d = drives();
        assert_eq!(d.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), vec!["disk:0", "disk:2", "disk:1"]);
        let sys = &d[0];
        assert!(sys.system);
        assert_eq!((sys.kind.as_str(), sys.connection.as_str()), ("nvme", "Inside, NVMe"));
        assert_eq!(sys.health.state, "good");
        assert_eq!(sys.health.temp_c, Some(41.0));
        assert_eq!(sys.health.power_on_hours, Some(900));
        let kinds: Vec<(&str, bool)> = sys.partitions.iter().map(|p| (p.kind.as_str(), p.system)).collect();
        assert_eq!(kinds, vec![("efi", true), ("msr", true), ("windows", true), ("recovery", true), ("free", false)]);
        assert_eq!(sys.partitions[2].letter, "C");
        assert_eq!(sys.partitions[2].used, Some(600 << 30));
        assert!(sys.actions.is_empty(), "nothing is offered for the drive Windows runs from");
        let hdd = &d[1];
        assert_eq!((hdd.kind.as_str(), hdd.health.state.as_str()), ("hdd", "warn"));
        assert!(hdd.partitions[0].steam_library);
        assert_eq!(hdd.partitions[0].what, "Steam library");
        let usb = &d[2];
        assert_eq!((usb.kind.as_str(), usb.table.as_str(), usb.removable), ("usb", "dos", true));
        assert_eq!(usb.partitions[0].fs, "exFAT");
        assert_eq!(usb.partitions[0].device, "E:");
    }

    #[test]
    fn refuses_windows_own_partitions() {
        let d = drives();
        let sys = &d[0];
        for i in 0..4 {
            for a in ["format", "delete", "unmount", "rename", "games"] {
                assert!(crate::safety::plan(a, sys, Some(&sys.partitions[i]), "windows").refuse.is_some(), "{a} on {i}");
            }
        }
        assert!(crate::safety::plan("format_drive", sys, None, "windows").refuse.is_some());
        // Free space on Windows' drive: a partition can be made, with the word typed.
        let free = &sys.partitions[4];
        assert_eq!(crate::safety::plan("create", sys, Some(free), "windows").typed.as_deref(), Some("CHANGE"));
    }

    #[test]
    fn numbers_or_names() {
        assert_eq!(bus_name(&json!({"BusType": 17}), "BusType"), "NVMe");
        assert_eq!(bus_name(&json!({"BusType": "USB"}), "BusType"), "USB");
        assert_eq!(health_name(&json!({"H": 2}), "H"), "Unhealthy");
        assert_eq!(table_name(&json!({"PartitionStyle": 2})), "gpt");
        assert_eq!(table_name(&json!({"PartitionStyle": "RAW"})), "");
        // One disk comes as an object instead of a list.
        let one = json!({"disks": {"Number": 3, "FriendlyName": "Generic SD/MMC Card Reader USB Device", "BusType": "USB", "Size": 1u64 << 34, "PartitionStyle": "RAW"}});
        let d = parse(&one, &|_: &str| false);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, "sd");
        assert!(d[0].partitions.is_empty(), "a blank card has no partitions or free space to list");
    }
}
